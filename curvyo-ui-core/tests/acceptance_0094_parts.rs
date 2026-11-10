//! `specs/0094-masked-composite-trace` slice 2 in the core: what each part keeps after the later
//! parts are taken out, and the join of the parts' traces into one result (criteria 20 to 23 and
//! 30), on the fixtures M1 and M2 with two and three parts.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::OnceLock;

use curvyo_document_core::{Point, Tolerance, Vec2};
use curvyo_geometry_core::{BooleanOp, Outline, OutlineTriple, boolean, signed_area_mm2};
use curvyo_ui_core::{
    TracePartArea, TracePartClip, TraceRegion, join_trace_parts, read_trace_wire, trace_part_clips,
};
use kurbo::{BezPath, PathEl, Shape};
use vectrace::Output;
use vectrace::job::{Job as TraceJob, Mode as TraceMode, run as run_trace_job};
use vectrace::mask::Mask;
use vectrace::ocr::OcrLine;
use vectrace::{
    ColourParams as TraceColourParams, PixelMap as TracePixelMap, Raster as TraceRaster,
    Rgb8 as TraceRgb, TraceParams,
};

const SIDE: u32 = 1000;
/// 1000 px placed 100 mm wide: 10 px per mm.
const MAP: [f64; 6] = [0.1, 0.0, 0.0, 0.1, 0.0, 0.0];
const QUAD: [Point; 4] = [
    Point::new(0.0, 0.0),
    Point::new(100.0, 0.0),
    Point::new(100.0, 100.0),
    Point::new(0.0, 100.0),
];
const TOLERANCE_MM: f64 = 0.01;
/// The kernel tolerance of the mask region, as `TraceRegion` clips with it.
const REGION_TOLERANCE: Tolerance = Tolerance::from_mm(0.005);

/// M1 (spec, "Reference fixtures"); with `paper_right` the right half's white is #D8D8D8 (M2);
/// with `colour` the disc red, the square blue, the bar green and the small square grey.
fn picture(paper_right: bool, colour: bool) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let (cx, cy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            let disc = (cx - 250.0).hypot(cy - 500.0) <= 150.0;
            let square = (600..900).contains(&x) && (350..650).contains(&y);
            let bar = (480..520).contains(&y);
            let grey = (850..950).contains(&x) && (50..150).contains(&y);
            let rgb = if bar {
                if colour { [30, 160, 60] } else { [0, 0, 0] }
            } else if disc {
                if colour { [200, 30, 40] } else { [0, 0, 0] }
            } else if square {
                if colour { [30, 70, 190] } else { [0, 0, 0] }
            } else if grey {
                [0x80, 0x80, 0x80]
            } else if paper_right && x >= 500 {
                [0xD8, 0xD8, 0xD8]
            } else {
                [255, 255, 255]
            };
            rgba.extend([rgb[0], rgb[1], rgb[2], 255]);
        }
    }
    rgba
}

fn m1() -> &'static [u8] {
    static M1: OnceLock<Vec<u8>> = OnceLock::new();
    M1.get_or_init(|| picture(false, false))
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

/// A mask rectangle x0..x1 by y0..y1 mm, Inside or `outside`.
fn mask(x0: f64, y0: f64, x1: f64, y1: f64, outside: bool) -> TracePartArea {
    TracePartArea::Mask(
        TraceRegion::of_operands(&[vec![(rect(x0, y0, x1, y1), true)]], outside).unwrap(),
    )
}

/// R1: x = 0 to 50 mm, y = 0 to 100 mm.
fn r1() -> TracePartArea {
    mask(0.0, 0.0, 50.0, 100.0, false)
}

fn params(threshold: u8, smooth: u8) -> TraceParams {
    TraceParams {
        smooth,
        ..TraceParams::with_threshold(threshold)
    }
}

fn trace(picture: &[u8], p: &TraceParams, clip: &TracePartClip) -> Output {
    read_trace_wire(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: SIDE,
                height: SIDE,
                rgba: picture,
            },
            map: TracePixelMap::new(MAP),
            mode: TraceMode::Mono(p),
            lines: &[],
            mask: clip.region().map(|r| r as &dyn Mask),
            shapes: false,
        },
        &mut |_| {},
    ))
    .unwrap()
}

/// Every part traced with its own parameters through its clip, then joined.
fn join(picture: &[u8], parts: &[(TracePartArea, TraceParams)]) -> (Output, Vec<Output>) {
    let areas: Vec<TracePartArea> = parts.iter().map(|(a, _)| a.clone()).collect();
    let clips = trace_part_clips(&areas, &QUAD);
    let wires: Vec<Output> = parts
        .iter()
        .zip(&clips)
        .map(|((_, p), clip)| trace(picture, p, clip))
        .collect();
    let pairs: Vec<(&Output, &TracePartClip)> = wires.iter().zip(&clips).collect();
    (join_trace_parts(&pairs, 0), wires)
}

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

/// `op` over the painted regions of `sets` (each one operand), as polylines.
fn region_of(op: BooleanOp, sets: &[&[BezPath]]) -> Vec<BezPath> {
    let owned: Vec<Vec<Vec<OutlineTriple>>> = sets.iter().map(|s| kernel_outlines(s)).collect();
    let operands: Vec<Vec<Outline<'_>>> = owned
        .iter()
        .map(|s| s.iter().map(|a| Outline::new(a, true)).collect())
        .collect();
    let refs: Vec<&[Outline<'_>]> = operands.iter().map(Vec::as_slice).collect();
    boolean(op, &refs, REGION_TOLERANCE).map_or_else(
        |_| Vec::new(),
        |r| {
            r.outlines()
                .iter()
                .map(|polygon| {
                    let mut path = BezPath::new();
                    for (i, p) in polygon.iter().enumerate() {
                        if i == 0 {
                            path.move_to((p.x, p.y));
                        } else {
                            path.line_to((p.x, p.y));
                        }
                    }
                    path.close_path();
                    path
                })
                .collect()
        },
    )
}

fn area(outlines: &[BezPath]) -> f64 {
    let owned = kernel_outlines(outlines);
    let operand: Vec<Outline<'_>> = owned.iter().map(|a| Outline::new(a, true)).collect();
    boolean(BooleanOp::Union, &[&operand], REGION_TOLERANCE).map_or(0.0, |r| {
        r.outlines().iter().map(|o| signed_area_mm2(o)).sum()
    })
}

fn rect_path(x0: f64, y0: f64, x1: f64, y1: f64) -> BezPath {
    let mut p = BezPath::new();
    p.move_to((x0, y0));
    p.line_to((x1, y0));
    p.line_to((x1, y1));
    p.line_to((x0, y1));
    p.close_path();
    p
}

/// The symmetric difference of two outline sets, mm².
fn xor(a: &[BezPath], b: &[BezPath]) -> f64 {
    area(&region_of(BooleanOp::Exclusion, &[a, b]))
}

#[test]
fn a_single_part_keeps_its_own_area_and_the_join_of_one_is_that_trace() {
    let clips = trace_part_clips(&[r1()], &QUAD);
    assert!(matches!(&clips[..], [TracePartClip::Region(r)] if !r.outside()));
    assert_eq!(
        trace_part_clips(&[TracePartArea::Whole], &QUAD),
        [TracePartClip::Whole]
    );
    let (joined, wires) = join(m1(), &[(r1(), params(50, 80))]);
    assert_eq!(joined, wires[0]);
}

#[test]
fn a_part_nothing_later_overlaps_keeps_its_region_and_a_covered_one_is_skipped() {
    let clips = trace_part_clips(
        &[
            mask(0.0, 0.0, 50.0, 100.0, false),
            mask(50.0, 0.0, 100.0, 100.0, false),
            TracePartArea::Nothing,
        ],
        &QUAD,
    );
    let TracePartClip::Region(left) = &clips[0] else {
        panic!("{clips:?}")
    };
    let TracePartClip::Region(right) = &clips[1] else {
        panic!("{clips:?}")
    };
    assert_eq!(left.polygons().len(), 1);
    assert_eq!(right.polygons().len(), 1);
    assert_eq!(clips[2], TracePartClip::Nothing);
    // A later part over all of an earlier one hides it.
    let hidden = trace_part_clips(&[r1(), mask(-10.0, -10.0, 60.0, 110.0, false)], &QUAD);
    assert_eq!(hidden[0], TracePartClip::Nothing);
    // Part 1 the whole image with a part above it: everything but that part.
    let whole = trace_part_clips(&[TracePartArea::Whole, r1()], &QUAD);
    let TracePartClip::Region(rest) = &whole[0] else {
        panic!("{whole:?}")
    };
    assert!(rest.keeps(Point::new(75.0, 50.0)));
    assert!(!rest.keeps(Point::new(25.0, 50.0)));
}

/// Criterion 20: part 1 the whole image at Threshold 40, part 2 R1 Inside at Threshold 70.
#[test]
fn c20_each_part_is_its_own_single_part_trace_within_its_area() {
    let (joined, _) = join(
        m1(),
        &[
            (TracePartArea::Whole, params(40, 80)),
            (r1(), params(70, 80)),
        ],
    );
    let r1_box = [rect_path(0.0, 0.0, 50.0, 100.0)];
    let inside_ref = trace(m1(), &params(70, 80), &trace_part_clips(&[r1()], &QUAD)[0]);
    let outside_ref = trace(
        m1(),
        &params(40, 80),
        &trace_part_clips(&[mask(0.0, 0.0, 50.0, 100.0, true)], &QUAD)[0],
    );
    let inside = region_of(BooleanOp::Intersection, &[&joined.outlines, &r1_box]);
    let outside = region_of(BooleanOp::Difference, &[&joined.outlines, &r1_box]);
    let (a_in, a_out) = (area(&inside_ref.outlines), area(&outside_ref.outlines));
    assert!(a_in > 100.0 && a_out > 100.0, "{a_in} {a_out}");
    let d_in = xor(&inside, &inside_ref.outlines);
    let d_out = xor(&outside, &outside_ref.outlines);
    assert!(d_in <= 0.001 * a_in, "inside R1: {d_in} of {a_in}");
    assert!(d_out <= 0.001 * a_out, "outside R1: {d_out} of {a_out}");
}

/// Criterion 21: three parts, the third over the overlap of the first two; inside each later
/// part's area only its shapes exist, and the earlier ones end on its stencil edge.
#[test]
fn c21_a_later_part_wins_inside_its_region_and_cuts_the_earlier_ones() {
    let (joined, wires) = join(
        m1(),
        &[
            (TracePartArea::Whole, params(50, 80)),
            (mask(0.0, 0.0, 50.0, 100.0, false), params(50, 20)),
            (mask(30.0, 0.0, 70.0, 100.0, false), params(50, 80)),
        ],
    );
    for p in anchors(&wires[1].outlines) {
        assert!(
            p.x <= 30.0 + TOLERANCE_MM,
            "part 2 ends on part 3's edge: {p:?}"
        );
    }
    for p in anchors(&wires[0].outlines) {
        assert!(
            p.x >= 70.0 - TOLERANCE_MM,
            "part 1 is cut by parts 2 and 3: {p:?}"
        );
    }
    for p in anchors(&wires[2].outlines) {
        assert!(
            (30.0 - TOLERANCE_MM..=70.0 + TOLERANCE_MM).contains(&p.x),
            "{p:?}"
        );
    }
    for (i, a) in wires.iter().enumerate() {
        for b in &wires[i + 1..] {
            let overlap = area(&region_of(
                BooleanOp::Intersection,
                &[&a.outlines, &b.outlines],
            ));
            assert!(overlap <= TOLERANCE_MM, "parts overlap by {overlap} mm²");
        }
    }
    // The join covers what the unmasked trace covers.
    let whole = trace(m1(), &params(50, 80), &TracePartClip::Whole);
    let a = area(&whole.outlines);
    assert!(xor(&joined.outlines, &whole.outlines) <= 0.001 * a);
}

/// Criterion 22: Smooth 20 on the whole image, Smooth 80 in R1; the bar crosses the seam at
/// x = 50 mm as one closed outline, with no seam line left in it.
#[test]
fn c22_the_seam_closes_into_one_outline() {
    let (joined, wires) = join(
        m1(),
        &[
            (TracePartArea::Whole, params(50, 20)),
            (r1(), params(50, 80)),
        ],
    );
    assert!(wires.iter().all(|w| !w.outlines.is_empty()));
    assert_eq!(
        joined.outlines.len(),
        1,
        "disc, bar and square are one path"
    );
    assert_eq!(joined.paths, 1);
    for p in [
        Point::new(25.0, 50.0),
        Point::new(50.0, 50.0),
        Point::new(75.0, 50.0),
    ] {
        assert!(paints(&joined.outlines, p), "{p:?}");
    }
    // No segment runs along the seam inside the bar.
    let mut seam = 0.0;
    for o in &joined.outlines {
        let mut last = kurbo::Point::ZERO;
        for e in o.elements() {
            match *e {
                PathEl::MoveTo(p) => last = p,
                PathEl::LineTo(p) | PathEl::CurveTo(_, _, p) | PathEl::QuadTo(_, p) => {
                    let on_seam = (last.x - 50.0).abs() < TOLERANCE_MM
                        && (p.x - 50.0).abs() < TOLERANCE_MM
                        && last.y.min(p.y) > 48.0 + TOLERANCE_MM
                        && last.y.max(p.y) < 52.0 - TOLERANCE_MM;
                    if on_seam {
                        seam += (p.y - last.y).abs();
                    }
                    last = p;
                }
                PathEl::ClosePath => {}
            }
        }
    }
    assert!(seam <= TOLERANCE_MM, "{seam} mm of seam line");
    // Only the pieces at the seam are polylines: the result is not one polygon of the grid
    // everywhere (the disc's far side keeps its curves).
    assert!(
        joined.outlines[0]
            .elements()
            .iter()
            .any(|e| matches!(e, PathEl::CurveTo(..)))
            || joined.nodes < 2000,
        "{} nodes",
        joined.nodes
    );
}

/// Criteria 21 and 23: Multicolour in both parts; one layer per distinct fill colour over all
/// parts, and shapes of different colours do not overlap.
#[test]
fn c23_multicolour_parts_merge_by_colour() {
    let colour = picture(false, true);
    let areas = [TracePartArea::Whole, r1()];
    let clips = trace_part_clips(&areas, &QUAD);
    let p = TraceColourParams::new(TraceParams {
        selection: vectrace::Selection::RemoveBackground {
            background: Some(TraceRgb::new(255, 255, 255)),
            tolerance: 40,
        },
        ..TraceParams::with_threshold(128)
    });
    let wires: Vec<Output> = clips
        .iter()
        .map(|clip| {
            read_trace_wire(&run_trace_job(
                &TraceJob {
                    raster: TraceRaster {
                        width: SIDE,
                        height: SIDE,
                        rgba: &colour,
                    },
                    map: TracePixelMap::new(MAP),
                    mode: TraceMode::Colours(&p),
                    lines: &[],
                    mask: clip.region().map(|r| r as &dyn Mask),
                    shapes: false,
                },
                &mut |_| {},
            ))
            .unwrap()
        })
        .collect();
    let pairs: Vec<(&Output, &TracePartClip)> = wires.iter().zip(&clips).collect();
    let joined = join_trace_parts(&pairs, 1);
    let mut colours: Vec<TraceRgb> = wires
        .iter()
        .flat_map(|w| w.layers.iter().map(|l| l.colour))
        .collect();
    colours.sort_by_key(|c| (c.r, c.g, c.b));
    colours.dedup();
    assert_eq!(joined.layers.len(), colours.len(), "{:?}", joined.layers);
    assert_eq!(
        joined.layers.iter().map(|l| l.outlines).sum::<usize>(),
        joined.outlines.len()
    );
    // The palette strip shows the active part's palette.
    assert_eq!(joined.palette, wires[1].palette);
    // Different colours overlap by at most 0.01 mm².
    let mut at = 0;
    let layers: Vec<&[BezPath]> = joined
        .layers
        .iter()
        .map(|l| {
            let s = &joined.outlines[at..at + l.outlines];
            at += l.outlines;
            s
        })
        .collect();
    for (i, a) in layers.iter().enumerate() {
        for b in &layers[i + 1..] {
            let overlap = area(&region_of(BooleanOp::Intersection, &[a, b]));
            assert!(overlap <= TOLERANCE_MM, "colours overlap by {overlap} mm²");
        }
    }
    // The green bar crosses the seam as one outline of its layer.
    let bar = joined
        .layers
        .iter()
        .position(|l| l.colour.g > l.colour.r && l.colour.g > l.colour.b)
        .unwrap_or_else(|| panic!("a green layer: {:?}", joined.layers));
    assert_eq!(joined.layers[bar].outlines, 1, "{:?}", joined.layers);
}

/// M2 with two halves at different Thresholds: the right half's paper (#D8D8D8) stays paper at
/// Threshold 200 while its grey square is traced; the left half at 50 traces only black.
#[test]
fn m2_two_halves_with_their_own_thresholds() {
    let m2 = picture(true, false);
    let (joined, _) = join(
        &m2,
        &[
            (mask(0.0, 0.0, 50.0, 100.0, false), params(50, 80)),
            (mask(50.0, 0.0, 100.0, 100.0, false), params(200, 80)),
        ],
    );
    assert!(
        paints(&joined.outlines, Point::new(90.0, 10.0)),
        "the grey square"
    );
    assert!(
        !paints(&joined.outlines, Point::new(70.0, 20.0)),
        "not the paper"
    );
    assert!(paints(&joined.outlines, Point::new(25.0, 50.0)), "the disc");
    assert!(
        paints(&joined.outlines, Point::new(50.0, 50.0)),
        "the bar at the seam"
    );
}

/// The same parts twice give the same result (determinism).
#[test]
fn the_join_is_deterministic() {
    let parts = [
        (TracePartArea::Whole, params(50, 20)),
        (r1(), params(50, 80)),
        (mask(40.0, 40.0, 60.0, 60.0, true), params(60, 50)),
    ];
    assert_eq!(join(m1(), &parts).0, join(m1(), &parts).0);
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

/// Criterion 30, and shapes: part 1 the whole picture with Recognise text and shapes on, part 2
/// the bottom half with both off. The top line becomes text and the top disc a shape; the bottom
/// line and disc stay part 2's traced paths.
#[test]
fn c30_text_and_shapes_follow_the_part_their_centre_lies_in() {
    let picture = text_picture();
    let map = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let quad = [
        Point::new(0.0, 0.0),
        Point::new(200.0, 0.0),
        Point::new(200.0, 200.0),
        Point::new(0.0, 200.0),
    ];
    let bottom = TracePartArea::Mask(
        TraceRegion::of_operands(&[vec![(rect(0.0, 100.0, 200.0, 200.0), true)]], false).unwrap(),
    );
    let clips = trace_part_clips(&[TracePartArea::Whole, bottom], &quad);
    let p = TraceParams::with_threshold(128);
    let lines = [ocr_line(40.0), ocr_line(140.0)];
    let first = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: 200,
                height: 200,
                rgba: &picture,
            },
            map: TracePixelMap::new(map),
            mode: TraceMode::Mono(&p),
            lines: &lines,
            mask: clips[0].region().map(|r| r as &dyn Mask),
            shapes: true,
        },
        &mut |_| {},
    );
    let first = read_trace_wire(&first).unwrap();
    let second = read_trace_wire(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: 200,
                height: 200,
                rgba: &picture,
            },
            map: TracePixelMap::new(map),
            mode: TraceMode::Mono(&p),
            lines: &[],
            mask: clips[1].region().map(|r| r as &dyn Mask),
            shapes: false,
        },
        &mut |_| {},
    ))
    .unwrap();
    let joined = join_trace_parts(&[(&first, &clips[0]), (&second, &clips[1])], 0);
    assert_eq!(joined.texts.lines.len(), 1, "only the top line is text");
    assert!(joined.texts.lines[0].outlines[0].control_box().y1 < 100.0);
    assert!(
        paints(&joined.outlines, Point::new(60.0, 140.0)),
        "the bottom line is part 2's traced path"
    );
    assert!(
        !paints(&joined.outlines, Point::new(60.0, 40.0)),
        "the top line is not a path"
    );
    assert!(
        paints(&joined.outlines, Point::new(160.0, 150.0)),
        "bottom disc"
    );
    let shapes: usize = joined
        .shapes
        .iter()
        .map(vectrace::shapes::ShapeRegion::primitives)
        .sum();
    assert_eq!(shapes, 1, "{:?}", joined.shapes);
    // The shape points at the top disc's outline in the joined list.
    let index = joined.shapes[0].outlines[0];
    let b = joined.outlines[index].control_box();
    assert!(b.y1 < 100.0 && b.x0 > 130.0, "{b:?}");
}
