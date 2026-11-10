//! `specs/0094-masked-composite-trace` slice 1 in the core: the reference fixtures M1 and M2, the
//! clip of a trace to a mask region (criteria 2 to 8 and 15), the region-only analysis (criterion 9) and the text and shape passes
//! with a mask (criterion 11, on a synthetic picture with synthetic OCR lines: the reference
//! poster needs the OCR weights).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::OnceLock;

use curvyo_document_core::{Affine, Angle, Point, Tolerance, Vec2};
use curvyo_geometry_core::{BooleanOp, Outline, OutlineTriple, boolean, signed_area_mm2};
use curvyo_ui_core::{
    Mask, OcrLine, TraceColourParams, TraceJob, TraceMode, TraceParams, TracePixelMap, TraceRaster,
    TraceRegion, TraceRegionError, TraceRgb, TraceWire, read_trace_wire, run_trace_job,
    trace_add_shapes, trace_region_defaults, trace_suggest_background, trace_text_regions,
    unclipped_notice,
};
use kurbo::{BezPath, PathEl, Shape};

const SIDE: u32 = 1000;
/// 1000 px placed 100 mm wide: 10 px per mm.
const MAP: [f64; 6] = [0.1, 0.0, 0.0, 0.1, 0.0, 0.0];
const TOLERANCE_MM: f64 = 0.01;
/// The kernel tolerance of the mask region, as `TraceRegion` clips with it.
const REGION_TOLERANCE: Tolerance = Tolerance::from_mm(0.005);

/// M1 (spec, "Reference fixtures"); with `paper_right` the right half's white is #D8D8D8 (M2).
fn picture(paper_right: bool) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let (cx, cy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let disc = (cx - 250.0).hypot(cy - 500.0) <= 150.0;
            let square = (600..900).contains(&x) && (350..650).contains(&y);
            let bar = (480..520).contains(&y);
            let grey = (850..950).contains(&x) && (50..150).contains(&y);
            let v = if disc || square || bar {
                0
            } else if grey {
                0x80
            } else if paper_right && x >= 500 {
                0xD8
            } else {
                255
            };
            rgba.extend([v, v, v, 255]);
        }
    }
    rgba
}

fn m1() -> &'static [u8] {
    static M1: OnceLock<Vec<u8>> = OnceLock::new();
    M1.get_or_init(|| picture(false))
}

fn corner(p: Point) -> OutlineTriple {
    (p, Vec2::ZERO, Vec2::ZERO)
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<OutlineTriple> {
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
        .into_iter()
        .map(|(x, y)| corner(Point::new(x, y)))
        .collect()
}

/// R1: x = 0 to 50 mm, y = 0 to 100 mm.
fn r1(outside: bool) -> TraceRegion {
    TraceRegion::of_operands(&[vec![(rect(0.0, 0.0, 50.0, 100.0), true)]], outside).unwrap()
}

fn params() -> TraceParams {
    TraceParams::with_threshold(50)
}

fn trace(map: [f64; 6], region: Option<&TraceRegion>) -> TraceWire {
    read_trace_wire(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: SIDE,
                height: SIDE,
                rgba: m1(),
            },
            map: TracePixelMap::new(map),
            mode: TraceMode::Mono(&params()),
            lines: &[],
            mask: region.map(|r| r as &dyn Mask),
            shapes: false,
        },
        &mut |_| {},
    ))
    .unwrap()
}

/// The three traces of M1 at Threshold 50: unmasked, R1 Inside, R1 Outside.
fn traces() -> &'static [TraceWire; 3] {
    static TRACES: OnceLock<[TraceWire; 3]> = OnceLock::new();
    TRACES.get_or_init(|| {
        [
            trace(MAP, None),
            trace(MAP, Some(&r1(false))),
            trace(MAP, Some(&r1(true))),
        ]
    })
}

/// Every anchor of `outlines`.
fn anchors(outlines: &[BezPath]) -> Vec<Point> {
    outlines
        .iter()
        .flat_map(|o| o.elements().iter().copied())
        .filter_map(|e| match e {
            PathEl::MoveTo(p) | PathEl::LineTo(p) | PathEl::CurveTo(_, _, p) => {
                Some(Point::new(p.x, p.y))
            }
            _ => None,
        })
        .collect()
}

/// Whether the painted area of `outlines` (nonzero) covers `p`.
fn paints(outlines: &[BezPath], p: Point) -> bool {
    let mut all = BezPath::new();
    for o in outlines {
        all.extend(o.elements().iter().copied());
    }
    all.winding(kurbo::Point::new(p.x, p.y)) != 0
}

fn kernel_outlines(outlines: &[BezPath]) -> Vec<Vec<OutlineTriple>> {
    outlines
        .iter()
        .flat_map(|path| {
            let mut out: Vec<Vec<OutlineTriple>> = Vec::new();
            for e in path.elements() {
                match *e {
                    PathEl::MoveTo(p) => out.push(vec![corner(Point::new(p.x, p.y))]),
                    PathEl::LineTo(p) => out.last_mut().unwrap().push(corner(Point::new(p.x, p.y))),
                    PathEl::CurveTo(c1, c2, p) => {
                        let o = out.last_mut().unwrap();
                        let last = o.last_mut().unwrap();
                        last.2 = Vec2::new(c1.x - last.0.x, c1.y - last.0.y);
                        o.push((
                            Point::new(p.x, p.y),
                            Vec2::new(c2.x - p.x, c2.y - p.y),
                            Vec2::ZERO,
                        ));
                    }
                    _ => {}
                }
            }
            out
        })
        .collect()
}

/// The area of `op` over the painted regions of `sets` (each one operand), in mm².
fn area_of(op: BooleanOp, sets: &[&[BezPath]]) -> f64 {
    let owned: Vec<Vec<Vec<OutlineTriple>>> = sets.iter().map(|s| kernel_outlines(s)).collect();
    let operands: Vec<Vec<Outline<'_>>> = owned
        .iter()
        .map(|s| s.iter().map(|a| Outline::new(a, true)).collect())
        .collect();
    let refs: Vec<&[Outline<'_>]> = operands.iter().map(Vec::as_slice).collect();
    boolean(op, &refs, REGION_TOLERANCE).map_or(0.0, |r| {
        r.outlines().iter().map(|o| signed_area_mm2(o)).sum()
    })
}

/// The distance of `p` outside the convex polygon `quad` (0 inside).
fn outside_by(p: Point, quad: &[Point]) -> f64 {
    let mut worst: f64 = 0.0;
    for i in 0..quad.len() {
        let (a, b) = (quad[i], quad[(i + 1) % quad.len()]);
        let (ex, ey) = (b.x - a.x, b.y - a.y);
        let len = ex.hypot(ey);
        // Clockwise on screen (y down): the inside has a negative cross product with each edge.
        let d = ((p.x - a.x) * ey - (p.y - a.y) * ex) / len;
        worst = worst.max(d);
    }
    worst
}

#[test]
fn c2_inside_keeps_the_disc_and_the_left_bar_within_r1() {
    let [_, inside, _] = traces();
    let r1 = [
        Point::new(0.0, 0.0),
        Point::new(50.0, 0.0),
        Point::new(50.0, 100.0),
        Point::new(0.0, 100.0),
    ];
    for p in anchors(&inside.outlines) {
        assert!(outside_by(p, &r1) <= TOLERANCE_MM, "{p:?} outside R1");
    }
    assert!(paints(&inside.outlines, Point::new(25.0, 50.0)), "the disc");
    assert!(
        paints(&inside.outlines, Point::new(45.0, 50.0)),
        "the left half of the bar"
    );
    assert!(
        paints(&inside.outlines, Point::new(2.0, 50.0)),
        "the bar at the picture's edge"
    );
    assert!(
        !paints(&inside.outlines, Point::new(75.0, 50.0)),
        "not the square"
    );
    assert!(
        !paints(&inside.outlines, Point::new(55.0, 50.0)),
        "not the right bar"
    );
    assert_eq!(inside.paths, inside.outlines.len());
}

#[test]
fn c3_outside_keeps_the_square_and_the_right_bar_outside_r1() {
    let [_, _, outside] = traces();
    for p in anchors(&outside.outlines) {
        assert!(p.x >= 50.0 - TOLERANCE_MM, "{p:?} inside R1");
    }
    assert!(
        paints(&outside.outlines, Point::new(75.0, 50.0)),
        "the square"
    );
    assert!(
        paints(&outside.outlines, Point::new(55.0, 50.0)),
        "the right half of the bar"
    );
    assert!(
        !paints(&outside.outlines, Point::new(25.0, 50.0)),
        "not the disc"
    );
    assert!(
        !paints(&outside.outlines, Point::new(45.0, 50.0)),
        "not the left bar"
    );
}

#[test]
fn c4_inside_and_outside_together_are_the_unmasked_trace() {
    let [whole, inside, outside] = traces();
    let both: Vec<BezPath> = inside
        .outlines
        .iter()
        .chain(&outside.outlines)
        .cloned()
        .collect();
    let unmasked = area_of(BooleanOp::Union, &[&whole.outlines]);
    let xor = area_of(BooleanOp::Exclusion, &[&both, &whole.outlines]);
    assert!(unmasked > 1000.0, "{unmasked}");
    assert!(
        xor <= 0.001 * unmasked,
        "symmetric difference {xor} of {unmasked}"
    );
    let overlap = area_of(
        BooleanOp::Intersection,
        &[&inside.outlines, &outside.outlines],
    );
    assert!(overlap <= TOLERANCE_MM, "overlap {overlap}");
}

#[test]
fn c1_without_a_region_the_numbers_are_the_unmasked_jobs() {
    let small: Vec<u8> = (0..64 * 64)
        .flat_map(|i| {
            let (x, y) = (i % 64, i / 64);
            let v = if (10..40).contains(&x) && (20..50).contains(&y) {
                0
            } else {
                255
            };
            [v, v, v, 255]
        })
        .collect();
    let map = [0.5, 0.0, 0.0, 0.5, 3.0, 4.0];
    assert_eq!(
        run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: 64,
                    height: 64,
                    rgba: &small
                },
                map: TracePixelMap::new(map),
                mode: TraceMode::Mono(&params()),
                lines: &[],
                mask: None,
                shapes: false
            },
            &mut |_| {}
        ),
        run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: 64,
                    height: 64,
                    rgba: &small
                },
                map: TracePixelMap::new(map),
                mode: TraceMode::Mono(&params()),
                lines: &[],
                mask: None,
                shapes: false
            },
            &mut |_| {}
        )
    );
}

#[test]
fn c5_a_ring_mask_leaves_its_hole_and_overlapping_masks_are_one_union() {
    // A ring over the disc (10 to 40 mm x 35 to 65 mm): outer 5..45 x 30..70, hole 15..35 x 40..60.
    let mut hole = rect(15.0, 40.0, 35.0, 60.0);
    hole.reverse();
    let ring = TraceRegion::of_operands(
        &[vec![(rect(5.0, 30.0, 45.0, 70.0), true), (hole, true)]],
        false,
    )
    .unwrap();
    let wire = trace(MAP, Some(&ring));
    assert!(
        !paints(&wire.outlines, Point::new(25.0, 50.0)),
        "the hole is not traced"
    );
    assert!(
        paints(&wire.outlines, Point::new(12.0, 50.0)),
        "the ring is"
    );
    // Two overlapping rectangles: one region, nothing covered twice.
    let two = TraceRegion::of_operands(
        &[
            vec![(rect(0.0, 30.0, 30.0, 70.0), true)],
            vec![(rect(20.0, 30.0, 60.0, 70.0), true)],
        ],
        false,
    )
    .unwrap();
    assert_eq!(two.polygons().len(), 1);
    let wire = trace(MAP, Some(&two));
    let painted = area_of(BooleanOp::Union, &[&wire.outlines]);
    let summed: f64 = wire
        .outlines
        .iter()
        .map(|o| area_of(BooleanOp::Union, &[std::slice::from_ref(o)]))
        .sum::<f64>();
    // Every outline here is an outer one (no holes): their areas add up to the painted area
    // unless two overlap.
    assert!(
        (summed - painted).abs() <= TOLERANCE_MM,
        "{summed} vs {painted}"
    );
    assert!(paints(&wire.outlines, Point::new(25.0, 50.0)));
    assert!(paints(&wire.outlines, Point::new(58.0, 50.0)));
}

#[test]
fn c6_an_open_mask_path_is_closed_by_a_straight_line() {
    // An open triangle over the left half; a straight open path encloses nothing and is dropped.
    let open = vec![(
        vec![
            corner(Point::new(0.0, 0.0)),
            corner(Point::new(50.0, 0.0)),
            corner(Point::new(50.0, 100.0)),
        ],
        false,
    )];
    let straight = vec![(
        vec![corner(Point::new(0.0, 0.0)), corner(Point::new(90.0, 90.0))],
        false,
    )];
    assert_eq!(
        TraceRegion::of_operands(std::slice::from_ref(&straight), false),
        Err(TraceRegionError::NoArea)
    );
    let region = TraceRegion::of_operands(&[straight, open], false).unwrap();
    assert!(region.keeps(Point::new(40.0, 20.0)));
    assert!(!region.keeps(Point::new(10.0, 80.0)));
}

#[test]
fn c7_a_mask_beyond_the_picture_is_clipped_and_one_outside_meets_nothing() {
    let beyond =
        TraceRegion::of_operands(&[vec![(rect(-50.0, -50.0, 50.0, 150.0), true)]], false).unwrap();
    let quad = [
        Point::new(0.0, 0.0),
        Point::new(100.0, 0.0),
        Point::new(100.0, 100.0),
        Point::new(0.0, 100.0),
    ];
    assert_ne!(beyond.quad_parts(&quad)[1].len(), 0);
    let [_, inside, _] = traces();
    let wire = trace(MAP, Some(&beyond));
    assert_eq!(wire.outlines.len(), inside.outlines.len());
    for p in anchors(&wire.outlines) {
        assert!((0.0..=100.0).contains(&p.x) && (0.0..=100.0).contains(&p.y));
    }
    let away =
        TraceRegion::of_operands(&[vec![(rect(120.0, 0.0, 150.0, 30.0), true)]], false).unwrap();
    assert!(
        away.quad_parts(&quad)[1].is_empty(),
        "meets nothing of the picture"
    );
}

#[test]
fn c8_a_turned_and_scaled_picture_clips_as_the_upright_one() {
    // 30 degrees, 60 mm wide: the picture's map and R1 under the same placement.
    let place = Affine::translation(Vec2::new(40.0, 10.0))
        .compose(Affine::rotation_about(
            Point::new(0.0, 0.0),
            Angle::from_radians(30.0_f64.to_radians()),
        ))
        .compose(Affine::scale(0.6, 0.6));
    clips_as_upright(place);
}

#[test]
fn c8_a_skewed_picture_clips_as_the_upright_one() {
    // Skewed by 0.3 along x, 60 mm wide, turned 30 degrees.
    let place = Affine::translation(Vec2::new(40.0, 10.0))
        .compose(Affine::rotation_about(
            Point::new(0.0, 0.0),
            Angle::from_radians(30.0_f64.to_radians()),
        ))
        .compose(Affine::new(0.6, 0.0, 0.18, 0.6, 0.0, 0.0));
    clips_as_upright(place);
}

/// Criteria 2 and 4 for M1 and R1 under the same placement `place`.
fn clips_as_upright(place: Affine) {
    let map = place.compose(Affine::from_array(MAP)).to_array();
    let corners: Vec<Point> = [(0.0, 0.0), (50.0, 0.0), (50.0, 100.0), (0.0, 100.0)]
        .into_iter()
        .map(|(x, y)| place.apply_point(Point::new(x, y)))
        .collect();
    let operand = vec![(corners.iter().copied().map(corner).collect(), true)];
    let inside = trace(
        map,
        Some(&TraceRegion::of_operands(std::slice::from_ref(&operand), false).unwrap()),
    );
    let outside = trace(
        map,
        Some(&TraceRegion::of_operands(&[operand], true).unwrap()),
    );
    for p in anchors(&inside.outlines) {
        assert!(outside_by(p, &corners) <= TOLERANCE_MM, "{p:?} outside R1");
    }
    let disc = place.apply_point(Point::new(25.0, 50.0));
    let square = place.apply_point(Point::new(75.0, 50.0));
    assert!(paints(&inside.outlines, disc) && !paints(&inside.outlines, square));
    let whole = trace(map, None);
    let both: Vec<BezPath> = inside
        .outlines
        .iter()
        .chain(&outside.outlines)
        .cloned()
        .collect();
    let unmasked = area_of(BooleanOp::Union, &[&whole.outlines]);
    let xor = area_of(BooleanOp::Exclusion, &[&both, &whole.outlines]);
    assert!(
        xor <= 0.001 * unmasked,
        "symmetric difference {xor} of {unmasked}"
    );
}

#[test]
fn c15_the_same_masked_job_twice_gives_the_same_numbers() {
    let region = r1(false);
    let first = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: SIDE,
                height: SIDE,
                rgba: m1(),
            },
            map: TracePixelMap::new(MAP),
            mode: TraceMode::Mono(&params()),
            lines: &[],
            mask: Some(&region),
            shapes: false,
        },
        &mut |_| {},
    );
    let second = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: SIDE,
                height: SIDE,
                rgba: m1(),
            },
            map: TracePixelMap::new(MAP),
            mode: TraceMode::Mono(&params()),
            lines: &[],
            mask: Some(&region),
            shapes: false,
        },
        &mut |_| {},
    );
    assert_eq!(first, second);
}

#[test]
fn c9_remove_background_reads_only_the_region() {
    let m2 = picture(true);
    let right =
        TraceRegion::of_operands(&[vec![(rect(50.0, 0.0, 100.0, 100.0), true)]], false).unwrap();
    let defaults = trace_region_defaults(&m2, SIDE, SIDE, MAP, &right);
    assert_eq!(defaults.background, Some(TraceRgb::new(0xD8, 0xD8, 0xD8)));
    assert!(defaults.analysis.is_some());
    // No mask: the whole picture's suggestion, as before.
    assert_eq!(
        trace_suggest_background(TraceRaster {
            width: SIDE,
            height: SIDE,
            rgba: &m2
        }),
        curvyo_vectorize_core::suggest_background(curvyo_vectorize_core::Raster {
            width: SIDE,
            height: SIDE,
            rgba: &m2
        })
    );
    let params = TraceParams {
        selection: curvyo_vectorize_core::Selection::RemoveBackground {
            background: defaults.background,
            tolerance: curvyo_ui_core::TRACE_DEFAULT_BACKGROUND_TOLERANCE,
        },
        ..TraceParams::with_threshold(128)
    };
    let wire = read_trace_wire(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: SIDE,
                height: SIDE,
                rgba: &m2,
            },
            map: TracePixelMap::new(MAP),
            mode: TraceMode::Mono(&params),
            lines: &[],
            mask: Some(&right),
            shapes: false,
        },
        &mut |_| {},
    ))
    .unwrap();
    assert!(
        !paints(&wire.outlines, Point::new(60.0, 25.0)),
        "no shape from the paper"
    );
    assert!(
        paints(&wire.outlines, Point::new(90.0, 10.0)),
        "the grey square"
    );
    assert!(
        paints(&wire.outlines, Point::new(75.0, 50.0)),
        "the black square"
    );
    assert!(
        !paints(&wire.outlines, Point::new(25.0, 50.0)),
        "nothing left of the mask"
    );
}

/// A 200 x 200 px picture, 1 px per mm: two dark "lines" (bars) at y = 40 and y = 140 and a
/// disc in each half.
fn text_picture() -> Vec<u8> {
    (0..200 * 200)
        .flat_map(|i| {
            let (x, y) = (f64::from(i % 200) + 0.5, f64::from(i / 200) + 0.5);
            let bar = (20.0..120.0).contains(&x)
                && ((36.0..44.0).contains(&y) || (136.0..144.0).contains(&y));
            let disc = (x - 160.0).hypot(y - 50.0) <= 20.0 || (x - 160.0).hypot(y - 150.0) <= 20.0;
            let v = if bar || disc { 0 } else { 255 };
            [v, v, v, 255]
        })
        .collect()
}

fn ocr_line(y: f64) -> OcrLine {
    OcrLine {
        quad: [
            [16.0, y - 8.0],
            [124.0, y - 8.0],
            [124.0, y + 8.0],
            [16.0, y + 8.0],
        ],
        text: "Hello".to_owned(),
        confidence: vec![0.99; 5],
        positions: Vec::new(),
    }
}

#[test]
fn c11_only_lines_and_shapes_whose_centre_is_in_the_region_stay() {
    let picture = text_picture();
    let map = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let top =
        TraceRegion::of_operands(&[vec![(rect(0.0, 0.0, 200.0, 100.0), true)]], false).unwrap();
    let lines = [ocr_line(40.0), ocr_line(140.0)];
    let p = TraceParams::with_threshold(128);
    let mut wire = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: 200,
                height: 200,
                rgba: &picture,
            },
            map: TracePixelMap::new(map),
            mode: TraceMode::Mono(&p),
            lines: &lines,
            mask: Some(&top),
            shapes: false,
        },
        &mut |_| {},
    );
    let read = read_trace_wire(&wire).unwrap();
    assert_eq!(read.texts.lines.len(), 1, "only the top line becomes text");
    assert!(
        read.texts.lines[0]
            .outlines
            .iter()
            .all(|o| o.bounding_box().y1 < 100.0)
    );
    for p in anchors(&read.outlines) {
        assert!(
            p.y <= 100.0 + TOLERANCE_MM,
            "{p:?}: nothing of the bottom half is traced"
        );
    }
    // The bottom line is neither text nor traced: its pixels were left out.
    assert!(!paints(&read.outlines, Point::new(60.0, 140.0)));
    // The text pass alone (the fit's lines) decides alike.
    let regions = read_trace_wire(&trace_text_regions(&TraceJob {
        raster: TraceRaster {
            width: 200,
            height: 200,
            rgba: &picture,
        },
        map: TracePixelMap::new(map),
        mode: TraceMode::Mono(&p),
        lines: &lines,
        mask: Some(&top),
        shapes: false,
    }))
    .unwrap();
    assert_eq!(regions.texts.lines.len(), 1);
    // Recognise shapes on the clipped outlines: only the top disc.
    trace_add_shapes(&mut wire, map);
    let shapes = read_trace_wire(&wire).unwrap().shapes;
    assert_eq!(
        shapes
            .iter()
            .map(curvyo_ui_core::TraceShapeRegion::primitives)
            .sum::<usize>(),
        1,
        "{shapes:?}"
    );
}

#[test]
fn c2_multicolour_layers_are_clipped_too() {
    let params = TraceColourParams::new(TraceParams::with_threshold(50));
    let region = r1(false);
    let wire = read_trace_wire(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: SIDE,
                height: SIDE,
                rgba: m1(),
            },
            map: TracePixelMap::new(MAP),
            mode: TraceMode::Colours(&params),
            lines: &[],
            mask: Some(&region),
            shapes: false,
        },
        &mut |_| {},
    ))
    .unwrap();
    assert_eq!(
        wire.layers.iter().map(|l| l.outlines).sum::<usize>(),
        wire.outlines.len()
    );
    for p in anchors(&wire.outlines) {
        assert!(p.x <= 50.0 + TOLERANCE_MM, "{p:?}");
    }
    assert!(
        !paints(&wire.outlines, Point::new(90.0, 10.0)),
        "the grey square is gone"
    );
}

/// A ring (outer radius 30 mm, hole 15 mm, about (50, 50) mm on a 100 mm picture) traced through
/// a mask whose edge at x = 50 mm crosses it, Inside and Outside (architect review 2026-11-04):
/// the hole stays a hole on both sides, the halves meet without gap or overlap.
#[test]
fn c4_a_ring_across_the_stencil_edge_keeps_its_hole_on_both_sides() {
    let ring: Vec<u8> = (0..SIDE * SIDE)
        .flat_map(|i| {
            let (x, y) = (f64::from(i % SIDE) + 0.5, f64::from(i / SIDE) + 0.5);
            let r = (x - 500.0).hypot(y - 500.0);
            let v = if (150.0..=300.0).contains(&r) { 0 } else { 255 };
            [v, v, v, 255]
        })
        .collect();
    let run = |region: Option<&TraceRegion>| {
        read_trace_wire(&run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: SIDE,
                    height: SIDE,
                    rgba: &ring,
                },
                map: TracePixelMap::new(MAP),
                mode: TraceMode::Mono(&params()),
                lines: &[],
                mask: region.map(|r| r as &dyn Mask),
                shapes: false,
            },
            &mut |_| {},
        ))
        .unwrap()
        .outlines
    };
    let whole = run(None);
    let inside = run(Some(&r1(false)));
    let outside = run(Some(&r1(true)));
    for (side, outlines, band, hole) in [
        (
            "inside",
            &inside,
            Point::new(27.0, 50.0),
            Point::new(45.0, 50.0),
        ),
        (
            "outside",
            &outside,
            Point::new(73.0, 50.0),
            Point::new(55.0, 50.0),
        ),
    ] {
        assert!(paints(outlines, band), "{side}: the band");
        assert!(!paints(outlines, hole), "{side}: the hole is not filled");
        assert!(
            !paints(outlines, Point::new(100.0 - band.x, 50.0)),
            "{side}: nothing of the other half"
        );
    }
    let both: Vec<BezPath> = inside.iter().chain(&outside).cloned().collect();
    let unmasked = area_of(BooleanOp::Union, &[&whole]);
    let xor = area_of(BooleanOp::Exclusion, &[&both, &whole]);
    assert!(
        xor <= 0.001 * unmasked,
        "symmetric difference {xor} of {unmasked}"
    );
    assert!(area_of(BooleanOp::Intersection, &[&inside, &outside]) <= TOLERANCE_MM);
}

/// A clip the kernel refuses (here: coordinates beyond its 10^7 mm) keeps the crossing shape
/// whole and says so on the wire, for the muted row "1 shape crosses the mask edge unclipped."
/// (tester 2026-11-04).
#[test]
fn a_refused_clip_is_counted_and_named() {
    let picture: Vec<u8> = (0..16 * 16)
        .flat_map(|i| {
            let (x, y) = (i % 16, i / 16);
            let v = if (4..12).contains(&x) && (4..12).contains(&y) {
                0
            } else {
                255
            };
            [v, v, v, 255]
        })
        .collect();
    let far = 2.0e7;
    let map = [1.0, 0.0, 0.0, 1.0, far, 0.0];
    // The left half of the square, far beyond the kernel's range.
    let half = TraceRegion::from_polygons(
        vec![vec![
            Point::new(far, 0.0),
            Point::new(far + 8.0, 0.0),
            Point::new(far + 8.0, 16.0),
            Point::new(far, 16.0),
        ]],
        false,
    )
    .unwrap();
    let p = TraceParams {
        specks: curvyo_vectorize_core::Mm::from_mm(0.0),
        ..TraceParams::with_threshold(128)
    };
    let read = read_trace_wire(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: 16,
                height: 16,
                rgba: &picture,
            },
            map: TracePixelMap::new(map),
            mode: TraceMode::Mono(&p),
            lines: &[],
            mask: Some(&half),
            shapes: false,
        },
        &mut |_| {},
    ))
    .unwrap();
    assert_eq!(read.unclipped, 1);
    assert_eq!(read.outlines.len(), 1, "kept whole");
    assert_eq!(
        unclipped_notice(read.unclipped),
        "1 shape crosses the mask edge unclipped."
    );
    assert_eq!(unclipped_notice(0), "");
    // An ordinary clip has none.
    assert_eq!(traces()[1].unclipped, 0);
}
