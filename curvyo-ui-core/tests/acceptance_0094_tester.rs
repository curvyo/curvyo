//! Tester cases for `specs/0094-masked-composite-trace` slice 1 in the core (criteria 1 to 11,
//! 15; ux-notes §2): the trace limited to a mask region with an exact stencil edge, Inside and
//! Outside together equal to the unmasked trace, the region-only automatic values, text and shapes decided by the region, determinism. Written from
//! the specification on the tester's own synthetic fixtures (`support_0094`), before the
//! implementation was read. The unmasked goldens in
//! `tests/fixtures/acceptance_0094_tester_unmasked.json` were written by the base commit of the
//! story (d1275c72, before `trace_job_in` existed) with `trace_job_in`,
//! `trace_colour_job_in`, `trace_text_job_in` and `trace_add_shapes`.
//!
//! M1 and M2 are drawn at their own 1000 x 1000 px over 100 x 100 mm; the variants for text,
//! shapes, Multicolour and the property test use 160 to 400 px to stay fast in debug.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::float_cmp
)]

#[path = "support_0094/mod.rs"]
mod support;

use curvyo_document_core::{Point, Tolerance, Vec2};
use curvyo_geometry_core::{
    BooleanError, BooleanOp, Outline, OutlineTriple, boolean, signed_area_mm2,
};
use curvyo_ui_core::{TraceRegion, TraceRegionError, read_trace_wire, trace_region_defaults};
use kurbo::{BezPath, PathEl};
use proptest::prelude::*;
use support::{COLOUR, M1, M2, draw, map_100mm, text_quads, text_sheet};
use vectrace::job::{Job as TraceJob, Mode as TraceMode, run as run_trace_job};
use vectrace::mask::Mask;
use vectrace::ocr::OcrLine;
use vectrace::shapes::trace_add_shapes;
use vectrace::{
    ColourParams as TraceColourParams, PixelMap as TracePixelMap, Raster as TraceRaster,
    Rgb8 as TraceRgb, Selection as TraceMethod, TraceParams, analyse as trace_analyse,
    auto_threshold as trace_auto_threshold, suggest_background as trace_suggest_background,
};

const N: u32 = 1000;
/// Criterion 2's bound for a node outside the stencil edge.
const EDGE_MM: f64 = 0.01;

// ---------------------------------------------------------------------------------------------
// Helpers: outlines, areas and distances through the geometry kernel.

fn outlines(wire: &[f64]) -> Vec<BezPath> {
    read_trace_wire(wire).expect("a readable wire").outlines
}

/// Every anchor (segment end point) of `paths`.
fn nodes(paths: &[BezPath]) -> Vec<Point> {
    let mut out = Vec::new();
    for p in paths {
        for el in p.elements() {
            let q = match *el {
                PathEl::MoveTo(q)
                | PathEl::LineTo(q)
                | PathEl::QuadTo(_, q)
                | PathEl::CurveTo(_, _, q) => q,
                PathEl::ClosePath => continue,
            };
            out.push(Point::new(q.x, q.y));
        }
    }
    out
}

/// `paths` flattened to polygons at 0.0005 mm, as kernel anchors without handles.
fn polygons(paths: &[BezPath]) -> Vec<Vec<OutlineTriple>> {
    let mut out: Vec<Vec<OutlineTriple>> = Vec::new();
    for p in paths {
        let mut current: Vec<OutlineTriple> = Vec::new();
        kurbo::flatten(p.elements().iter().copied(), 0.0005, |el| match el {
            PathEl::MoveTo(q) => {
                if current.len() >= 3 {
                    out.push(std::mem::take(&mut current));
                }
                current.clear();
                current.push((Point::new(q.x, q.y), Vec2::ZERO, Vec2::ZERO));
            }
            PathEl::LineTo(q) => current.push((Point::new(q.x, q.y), Vec2::ZERO, Vec2::ZERO)),
            _ => {}
        });
        if current.len() >= 3 {
            out.push(current);
        }
    }
    out
}

fn poly_triples(points: &[Point]) -> Vec<OutlineTriple> {
    points
        .iter()
        .map(|&p| (p, Vec2::ZERO, Vec2::ZERO))
        .collect()
}

fn tol() -> Tolerance {
    Tolerance::from_mm(0.003)
}

/// The painted area (nonzero) of one operand.
fn area_of(operand: &[Vec<OutlineTriple>]) -> f64 {
    if operand.is_empty() {
        return 0.0;
    }
    let o: Vec<Outline<'_>> = operand.iter().map(|a| Outline::new(a, true)).collect();
    match boolean(BooleanOp::Union, &[&o], tol()) {
        Ok(r) => r
            .outlines()
            .iter()
            .map(|p| signed_area_mm2(p))
            .sum::<f64>()
            .abs(),
        Err(BooleanError::EmptyResult) => 0.0,
        Err(e) => panic!("kernel refused the area: {e:?}"),
    }
}

/// The area of `op` over the operands.
fn op_area(op: BooleanOp, operands: &[&[Vec<OutlineTriple>]]) -> f64 {
    let filled: Vec<&[Vec<OutlineTriple>]> =
        operands.iter().copied().filter(|o| !o.is_empty()).collect();
    if filled.len() < operands.len() {
        return match (op, filled.as_slice()) {
            (BooleanOp::Intersection, _) | (_, []) => 0.0,
            (_, [one]) => area_of(one),
            (BooleanOp::Difference, _) if operands[0].is_empty() => 0.0,
            (_, more) => op_area(op, more),
        };
    }
    let outl: Vec<Vec<Outline<'_>>> = operands
        .iter()
        .map(|operand| operand.iter().map(|a| Outline::new(a, true)).collect())
        .collect();
    let refs: Vec<&[Outline<'_>]> = outl.iter().map(Vec::as_slice).collect();
    match boolean(op, &refs, tol()) {
        Ok(r) => r
            .outlines()
            .iter()
            .map(|p| signed_area_mm2(p))
            .sum::<f64>()
            .abs(),
        Err(BooleanError::EmptyResult) => 0.0,
        Err(e) => panic!("kernel refused {op:?}: {e:?}"),
    }
}

/// The union of `a` and `b` as one operand's outlines.
fn union_of(a: &[Vec<OutlineTriple>], b: &[Vec<OutlineTriple>]) -> Vec<Vec<OutlineTriple>> {
    if a.is_empty() || b.is_empty() {
        return if a.is_empty() { b.to_vec() } else { a.to_vec() };
    }
    let oa: Vec<Outline<'_>> = a.iter().map(|x| Outline::new(x, true)).collect();
    let ob: Vec<Outline<'_>> = b.iter().map(|x| Outline::new(x, true)).collect();
    match boolean(BooleanOp::Union, &[&oa, &ob], tol()) {
        Ok(r) => r.outlines().iter().map(|p| poly_triples(p)).collect(),
        Err(BooleanError::EmptyResult) => Vec::new(),
        Err(e) => panic!("kernel refused the union: {e:?}"),
    }
}

/// The signed distance of `p` from the convex polygon `poly`: positive outside.
fn convex_distance(poly: &[Point], p: Point) -> f64 {
    let ccw = signed_area_mm2(poly) > 0.0;
    let mut worst = f64::NEG_INFINITY;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len = dx.hypot(dy);
        // Outward normal.
        let (nx, ny) = if ccw { (dy, -dx) } else { (-dy, dx) };
        let d = ((p.x - a.x) * nx + (p.y - a.y) * ny) / len;
        worst = worst.max(d);
    }
    worst
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point> {
    vec![
        Point::new(x0, y0),
        Point::new(x1, y0),
        Point::new(x1, y1),
        Point::new(x0, y1),
    ]
}

fn r1() -> Vec<Point> {
    rect(0.0, 0.0, 50.0, 100.0)
}

fn region(poly: Vec<Point>, outside: bool) -> TraceRegion {
    TraceRegion::from_polygons(vec![poly], outside).expect("a region")
}

fn t50() -> TraceParams {
    TraceParams::with_threshold(50)
}

fn m1() -> Vec<u8> {
    draw(M1, N)
}

fn inside_r1() -> Vec<f64> {
    run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: N,
                height: N,
                rgba: &m1(),
            },
            map: TracePixelMap::new(map_100mm(N)),
            mode: TraceMode::Mono(&t50()),
            lines: &[],
            mask: Some(&region(r1(), false)),
            shapes: false,
        },
        &mut |_| {},
    )
}

fn outside_r1() -> Vec<f64> {
    run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: N,
                height: N,
                rgba: &m1(),
            },
            map: TracePixelMap::new(map_100mm(N)),
            mode: TraceMode::Mono(&t50()),
            lines: &[],
            mask: Some(&region(r1(), true)),
            shapes: false,
        },
        &mut |_| {},
    )
}

fn unmasked() -> Vec<f64> {
    run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: N,
                height: N,
                rgba: &m1(),
            },
            map: TracePixelMap::new(map_100mm(N)),
            mode: TraceMode::Mono(&t50()),
            lines: &[],
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    )
}

fn covers(paths: &[BezPath], p: Point) -> bool {
    let polys = polygons(paths);
    let o: Vec<Outline<'_>> = polys.iter().map(|a| Outline::new(a, true)).collect();
    curvyo_geometry_core::contains_point_in_outlines(&o, p)
}

/// Criterion 4's measure: the symmetric difference of `a ∪ b` and `whole`, relative to `whole`,
/// and the overlap of `a` and `b` in mm².
fn seam(a: &[BezPath], b: &[BezPath], whole: &[BezPath]) -> (f64, f64) {
    let (pa, pb, pw) = (polygons(a), polygons(b), polygons(whole));
    let joined = union_of(&pa, &pb);
    let xor = op_area(BooleanOp::Exclusion, &[&joined, &pw]);
    let overlap = op_area(BooleanOp::Intersection, &[&pa, &pb]);
    (xor / area_of(&pw), overlap)
}

// ---------------------------------------------------------------------------------------------
// Criterion 1 (regression): without a mask the trace is path for path the base commit's.

fn golden(name: &str) -> Vec<f64> {
    let text = include_str!("fixtures/acceptance_0094_tester_unmasked.json");
    let json: serde_json::Value = serde_json::from_str(text).unwrap();
    json[name]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect()
}

fn ocr_lines(n: u32) -> Vec<OcrLine> {
    text_quads(n)
        .iter()
        .map(|q| OcrLine {
            quad: *q,
            text: "Hello".into(),
            confidence: vec![0.99; 5],
            positions: Vec::new(),
        })
        .collect()
}

/// 30 degrees, 60 mm wide, skewed in x by 0.2, offset by (20, 30) mm.
fn skew_map(n: u32) -> [f64; 6] {
    let s = 60.0 / f64::from(n);
    let (c, si) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
    [
        c * s,
        si * s,
        (c * 0.2 - si) * s,
        (si * 0.2 + c) * s,
        20.0,
        30.0,
    ]
}

fn colour_params() -> TraceColourParams {
    TraceColourParams::new(TraceParams {
        selection: TraceMethod::RemoveBackground {
            background: Some(TraceRgb::new(255, 255, 255)),
            tolerance: 40,
        },
        specks: vectrace::Mm::from_mm(0.0),
        ..TraceParams::with_threshold(128)
    })
}

#[test]
fn c1_without_a_mask_every_job_is_number_for_number_the_base_commits() {
    let n = 250;
    let m1 = draw(M1, n);
    let no_mask = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: n,
                height: n,
                rgba: &m1,
            },
            map: TracePixelMap::new(map_100mm(n)),
            mode: TraceMode::Mono(&t50()),
            lines: &[],
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    );
    assert_eq!(no_mask, golden("m1_250_t50"), "trace_job_in(None)");
    assert_eq!(
        run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: n,
                    height: n,
                    rgba: &m1
                },
                map: TracePixelMap::new(map_100mm(n)),
                mode: TraceMode::Mono(&t50()),
                lines: &[],
                mask: None,
                shapes: false
            },
            &mut |_| {}
        ),
        golden("m1_250_t50"),
        "trace_job_in"
    );
    let mut shapes = no_mask.clone();
    trace_add_shapes(&mut shapes, map_100mm(n));
    assert_eq!(shapes, golden("m1_250_t50_shapes"), "Recognise shapes");
    assert_eq!(
        run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: n,
                    height: n,
                    rgba: &m1
                },
                map: TracePixelMap::new(skew_map(n)),
                mode: TraceMode::Mono(&t50()),
                lines: &[],
                mask: None,
                shapes: false
            },
            &mut |_| {}
        ),
        golden("m1_250_t50_skew"),
        "a rotated and skewed image"
    );
    let c = 200;
    let col = draw(COLOUR, c);
    let colour = colour_params();
    assert_eq!(
        run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: c,
                    height: c,
                    rgba: &col
                },
                map: TracePixelMap::new(map_100mm(c)),
                mode: TraceMode::Colours(&colour),
                lines: &[],
                mask: None,
                shapes: false
            },
            &mut |_| {}
        ),
        golden("colour_200"),
        "Multicolour"
    );
    assert_eq!(
        run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: c,
                    height: c,
                    rgba: &col
                },
                map: TracePixelMap::new(map_100mm(c)),
                mode: TraceMode::Colours(&colour),
                lines: &[],
                mask: None,
                shapes: false
            },
            &mut |_| {}
        ),
        golden("colour_200")
    );
    let t = text_sheet(c);
    let p = TraceParams::with_threshold(128);
    assert_eq!(
        run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: c,
                    height: c,
                    rgba: &t
                },
                map: TracePixelMap::new(map_100mm(c)),
                mode: TraceMode::Mono(&p),
                lines: &ocr_lines(c),
                mask: None,
                shapes: false
            },
            &mut |_| {}
        ),
        golden("text_200"),
        "Recognise text"
    );
    assert_eq!(
        run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: c,
                    height: c,
                    rgba: &t
                },
                map: TracePixelMap::new(map_100mm(c)),
                mode: TraceMode::Mono(&p),
                lines: &ocr_lines(c),
                mask: None,
                shapes: false
            },
            &mut |_| {}
        ),
        golden("text_200")
    );
}

// ---------------------------------------------------------------------------------------------
// Criteria 2 to 4: M1 with R1 Inside and Outside.

#[test]
fn c2_inside_r1_no_node_beyond_the_edge_disc_and_left_bar_without_the_square() {
    let wire = inside_r1();
    let read = read_trace_wire(&wire).unwrap();
    let paths = read.outlines;
    assert!(!paths.is_empty(), "something is traced");
    for p in nodes(&paths) {
        assert!(
            convex_distance(&r1(), p) <= EDGE_MM,
            "node {p:?} lies {} mm outside R1",
            convex_distance(&r1(), p)
        );
    }
    assert!(covers(&paths, Point::new(25.0, 50.0)), "the disc");
    assert!(
        covers(&paths, Point::new(45.0, 50.0)),
        "the left half of the bar"
    );
    assert!(
        covers(&paths, Point::new(49.9, 50.0)),
        "the bar up to the edge"
    );
    assert!(!covers(&paths, Point::new(75.0, 50.0)), "not the square");
    assert!(
        !covers(&paths, Point::new(55.0, 50.0)),
        "not the right half of the bar"
    );
    assert!(
        !covers(&paths, Point::new(90.0, 10.0)),
        "not the grey square"
    );
    // The kept area is exactly the unmasked trace within R1.
    let whole = polygons(&outlines(&unmasked()));
    let clipped = op_area(BooleanOp::Intersection, &[&whole, &[poly_triples(&r1())]]);
    let got = polygons(&paths);
    let xor = op_area(BooleanOp::Exclusion, &[&got, &whole_within(&whole, &r1())]);
    assert!(
        xor <= 0.001 * clipped,
        "inside R1 differs from the unmasked trace clipped to R1 by {xor} mm² of {clipped}"
    );
    // The counts in the wire are the counts after the clip.
    assert_eq!(
        read.paths,
        paths.len(),
        "the path count is of the clipped result"
    );
}

fn whole_within(whole: &[Vec<OutlineTriple>], poly: &[Point]) -> Vec<Vec<OutlineTriple>> {
    let o: Vec<Outline<'_>> = whole.iter().map(|a| Outline::new(a, true)).collect();
    let r = poly_triples(poly);
    let ro = [Outline::new(&r, true)];
    boolean(BooleanOp::Intersection, &[&o, &ro], tol())
        .map(|r| r.outlines().iter().map(|p| poly_triples(p)).collect())
        .unwrap_or_default()
}

#[test]
fn c3_outside_r1_no_node_inside_square_and_right_bar_without_the_disc() {
    let paths = outlines(&outside_r1());
    assert_ne!(paths.len(), 0);
    for p in nodes(&paths) {
        assert!(
            -convex_distance(&r1(), p) <= EDGE_MM,
            "node {p:?} lies {} mm inside R1",
            -convex_distance(&r1(), p)
        );
    }
    assert!(covers(&paths, Point::new(75.0, 50.0)), "the square");
    assert!(
        covers(&paths, Point::new(55.0, 50.0)),
        "the right half of the bar"
    );
    assert!(
        covers(&paths, Point::new(50.1, 50.0)),
        "the bar from the edge"
    );
    assert!(!covers(&paths, Point::new(25.0, 50.0)), "not the disc");
    assert!(
        !covers(&paths, Point::new(45.0, 50.0)),
        "not the left half of the bar"
    );
}

#[test]
fn c4_inside_and_outside_together_are_the_unmasked_trace_within_0_1_percent() {
    let (xor, overlap) = seam(
        &outlines(&inside_r1()),
        &outlines(&outside_r1()),
        &outlines(&unmasked()),
    );
    assert!(xor <= 0.001, "symmetric difference {:.5} %", xor * 100.0);
    assert!(
        overlap <= 0.01,
        "Inside and Outside overlap by {overlap} mm²"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 5: a compound mask with a hole; overlapping masks.

fn circle(cx: f64, cy: f64, r: f64, ccw: bool) -> Vec<Point> {
    (0..256)
        .map(|i| {
            let t = f64::from(i) / 256.0 * std::f64::consts::TAU * if ccw { 1.0 } else { -1.0 };
            Point::new(cx + r * t.cos(), cy + r * t.sin())
        })
        .collect()
}

#[test]
fn c5_a_ring_mask_leaves_its_hole_untraced() {
    // A ring around the disc's centre: outer radius 20 mm, hole of radius 6 mm.
    let ring = TraceRegion::from_polygons(
        vec![
            circle(25.0, 50.0, 20.0, true),
            circle(25.0, 50.0, 6.0, false),
        ],
        false,
    )
    .unwrap();
    assert!(
        !ring.keeps(Point::new(25.0, 50.0)),
        "the hole is no part of the region"
    );
    assert!(ring.keeps(Point::new(25.0, 40.0)));
    let paths = outlines(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: N,
                height: N,
                rgba: &m1(),
            },
            map: TracePixelMap::new(map_100mm(N)),
            mode: TraceMode::Mono(&t50()),
            lines: &[],
            mask: Some(&ring),
            shapes: false,
        },
        &mut |_| {},
    ));
    assert!(
        !covers(&paths, Point::new(25.0, 50.0)),
        "the content in the hole is not traced"
    );
    assert!(
        !covers(&paths, Point::new(28.0, 52.0)),
        "nor anywhere in the hole"
    );
    assert!(
        covers(&paths, Point::new(25.0, 40.0)),
        "the disc within the ring is"
    );
    for p in nodes(&paths) {
        let r = (p.x - 25.0).hypot(p.y - 50.0);
        assert!(
            (6.0 - EDGE_MM..=20.0 + EDGE_MM).contains(&r),
            "node {p:?} at radius {r} lies off the ring"
        );
    }
    // The ring's area of the disc: disc (r 15) minus the hole (r 6), plus the bar within the
    // ring outside the disc. Compared with the kernel's own clip of the unmasked trace.
    let whole = polygons(&outlines(&unmasked()));
    let ring_operand = vec![
        poly_triples(&circle(25.0, 50.0, 20.0, true)),
        poly_triples(&circle(25.0, 50.0, 6.0, false)),
    ];
    let expected = op_area(BooleanOp::Intersection, &[&whole, &ring_operand]);
    let got = area_of(&polygons(&paths));
    assert!(
        (got - expected).abs() <= 0.001 * expected,
        "ring area {got} vs {expected}"
    );
}

#[test]
fn c5_overlapping_masks_are_one_union_and_nothing_is_covered_twice() {
    let a = rect(10.0, 30.0, 40.0, 70.0);
    let b = rect(30.0, 40.0, 70.0, 60.0);
    let region = TraceRegion::from_polygons(vec![a.clone(), b.clone()], false).unwrap();
    for p in [
        Point::new(15.0, 35.0),
        Point::new(35.0, 50.0),
        Point::new(65.0, 55.0),
    ] {
        assert!(region.keeps(p), "{p:?} is in the union");
    }
    let paths = outlines(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: N,
                height: N,
                rgba: &m1(),
            },
            map: TracePixelMap::new(map_100mm(N)),
            mode: TraceMode::Mono(&t50()),
            lines: &[],
            mask: Some(&region),
            shapes: false,
        },
        &mut |_| {},
    ));
    // Pairwise overlap of the result's outer shapes (positive outlines with their holes) is
    // at most 0.01 mm².
    let shapes = outer_shapes(&paths);
    assert_ne!(shapes.len(), 0);
    for i in 0..shapes.len() {
        for j in i + 1..shapes.len() {
            let o = op_area(BooleanOp::Intersection, &[&shapes[i], &shapes[j]]);
            assert!(o <= 0.01, "shapes {i} and {j} overlap by {o} mm²");
        }
    }
    // The area equals the unmasked trace within the union.
    let whole = polygons(&outlines(&unmasked()));
    let union = vec![poly_triples(&a), poly_triples(&b)];
    let expected = op_area(BooleanOp::Intersection, &[&whole, &union]);
    let got = area_of(&polygons(&paths));
    assert!(
        (got - expected).abs() <= 0.001 * expected,
        "{got} vs {expected}"
    );
}

/// Each outer outline of `paths` with the holes inside it, as one operand.
fn outer_shapes(paths: &[BezPath]) -> Vec<Vec<Vec<OutlineTriple>>> {
    let polys = polygons(paths);
    let pts = |a: &[OutlineTriple]| a.iter().map(|t| t.0).collect::<Vec<_>>();
    let sign_outer = polys
        .iter()
        .map(|a| signed_area_mm2(&pts(a)))
        .max_by(|x, y| x.abs().total_cmp(&y.abs()))
        .map_or(1.0, f64::signum);
    let mut shapes: Vec<Vec<Vec<OutlineTriple>>> = Vec::new();
    for a in &polys {
        if signed_area_mm2(&pts(a)) * sign_outer > 0.0 {
            shapes.push(vec![a.clone()]);
        }
    }
    for a in &polys {
        if signed_area_mm2(&pts(a)) * sign_outer < 0.0 {
            let probe = a[0].0;
            if let Some(s) = shapes.iter_mut().find(|s| {
                let o = [Outline::new(&s[0], true)];
                curvyo_geometry_core::contains_point_in_outlines(&o, probe)
            }) {
                s.push(a.clone());
            }
        }
    }
    shapes
}

// ---------------------------------------------------------------------------------------------
// Criterion 6: an open path is closed by a straight line.

fn triple(x: f64, y: f64) -> OutlineTriple {
    (Point::new(x, y), Vec2::ZERO, Vec2::ZERO)
}

#[test]
fn c6_an_open_mask_path_is_closed_by_a_straight_line() {
    // An open "L" through (0,0), (50,0), (50,100): closed, it is the triangle of R1's upper right.
    let open = vec![(
        vec![triple(0.0, 0.0), triple(50.0, 0.0), triple(50.0, 100.0)],
        false,
    )];
    let region = TraceRegion::of_operands(&[open], false).expect("no refusal");
    assert!(
        region.keeps(Point::new(45.0, 20.0)),
        "inside the closed triangle"
    );
    assert!(
        !region.keeps(Point::new(5.0, 80.0)),
        "beyond the closing line"
    );
    let paths = outlines(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: N,
                height: N,
                rgba: &m1(),
            },
            map: TracePixelMap::new(map_100mm(N)),
            mode: TraceMode::Mono(&t50()),
            lines: &[],
            mask: Some(&region),
            shapes: false,
        },
        &mut |_| {},
    ));
    let triangle = vec![
        Point::new(0.0, 0.0),
        Point::new(50.0, 0.0),
        Point::new(50.0, 100.0),
    ];
    assert_ne!(paths.len(), 0);
    for p in nodes(&paths) {
        assert!(
            convex_distance(&triangle, p) <= EDGE_MM,
            "node {p:?} off the triangle"
        );
    }
}

#[test]
fn c6_a_straight_open_path_alone_encloses_nothing_and_with_another_it_is_dropped() {
    let line = vec![(vec![triple(0.0, 0.0), triple(50.0, 50.0)], false)];
    assert!(matches!(
        TraceRegion::of_operands(std::slice::from_ref(&line), false),
        Err(TraceRegionError::NoArea)
    ));
    let square = vec![(
        rect(10.0, 10.0, 20.0, 20.0)
            .into_iter()
            .map(|p| (p, Vec2::ZERO, Vec2::ZERO))
            .collect(),
        true,
    )];
    let r = TraceRegion::of_operands(&[line, square], false).expect("the square makes a region");
    assert!(r.keeps(Point::new(15.0, 15.0)));
    assert!(!r.keeps(Point::new(40.0, 41.0)));
}

// ---------------------------------------------------------------------------------------------
// Criterion 7: a mask beyond the image; a mask outside it.

#[test]
fn c7_a_mask_beyond_the_image_traces_only_the_image() {
    let big = region(rect(-30.0, -30.0, 50.0, 130.0), false);
    let paths = outlines(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: N,
                height: N,
                rgba: &m1(),
            },
            map: TracePixelMap::new(map_100mm(N)),
            mode: TraceMode::Mono(&t50()),
            lines: &[],
            mask: Some(&big),
            shapes: false,
        },
        &mut |_| {},
    ));
    let (xor, _) = seam(&paths, &[], &outlines(&inside_r1()));
    assert!(xor <= 0.001, "beyond the image is the same as R1: {xor}");
    for p in nodes(&paths) {
        assert!(convex_distance(&rect(0.0, 0.0, 100.0, 100.0), p) <= EDGE_MM);
    }
}

#[test]
fn c7_a_mask_outside_the_image_inside_traces_nothing_outside_traces_all() {
    let far = rect(200.0, 200.0, 260.0, 260.0);
    let inside = read_trace_wire(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: N,
                height: N,
                rgba: &m1(),
            },
            map: TracePixelMap::new(map_100mm(N)),
            mode: TraceMode::Mono(&t50()),
            lines: &[],
            mask: Some(&region(far.clone(), false)),
            shapes: false,
        },
        &mut |_| {},
    ))
    .unwrap();
    assert_eq!(inside.paths, 0, "0 paths");
    assert_eq!(inside.outlines.len(), 0);
    let outside = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: N,
                height: N,
                rgba: &m1(),
            },
            map: TracePixelMap::new(map_100mm(N)),
            mode: TraceMode::Mono(&t50()),
            lines: &[],
            mask: Some(&region(far, true)),
            shapes: false,
        },
        &mut |_| {},
    );
    let (xor, _) = seam(&outlines(&outside), &[], &outlines(&unmasked()));
    assert!(
        xor <= 1e-9,
        "Outside of a far mask is the whole trace: {xor}"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 8: rotated by 30 degrees and scaled to 60 mm, and skewed in addition.

fn apply(map: [f64; 6], x: f64, y: f64) -> Point {
    let [a, b, c, d, e, f] = map;
    Point::new(a * x + c * y + e, b * x + d * y + f)
}

fn c8_case(map: [f64; 6]) {
    let n = f64::from(N);
    let r1_mapped = vec![
        apply(map, 0.0, 0.0),
        apply(map, n / 2.0, 0.0),
        apply(map, n / 2.0, n),
        apply(map, 0.0, n),
    ];
    let image = vec![
        apply(map, 0.0, 0.0),
        apply(map, n, 0.0),
        apply(map, n, n),
        apply(map, 0.0, n),
    ];
    let job = |r: Option<&dyn Mask>| {
        outlines(&run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: N,
                    height: N,
                    rgba: &m1(),
                },
                map: TracePixelMap::new(map),
                mode: TraceMode::Mono(&t50()),
                lines: &[],
                mask: r,
                shapes: false,
            },
            &mut |_| {},
        ))
    };
    let whole = job(None);
    for p in nodes(&whole) {
        assert!(
            convex_distance(&image, p) <= 0.01,
            "the pixel map convention: {p:?} lies off the image"
        );
    }
    let inside = job(Some(&region(r1_mapped.clone(), false)));
    let outside = job(Some(&region(r1_mapped.clone(), true)));
    assert!(!inside.is_empty() && !outside.is_empty());
    for p in nodes(&inside) {
        let d = convex_distance(&r1_mapped, p);
        assert!(d <= EDGE_MM, "Inside: node {p:?} {d} mm outside R1");
    }
    for p in nodes(&outside) {
        let d = -convex_distance(&r1_mapped, p);
        assert!(d <= EDGE_MM, "Outside: node {p:?} {d} mm inside R1");
    }
    let disc = apply(map, n / 4.0, n / 2.0);
    let square = apply(map, n * 0.75, n / 2.0);
    assert!(covers(&inside, disc) && !covers(&inside, square));
    assert!(covers(&outside, square) && !covers(&outside, disc));
    let (xor, overlap) = seam(&inside, &outside, &whole);
    assert!(xor <= 0.001, "symmetric difference {:.5} %", xor * 100.0);
    assert!(overlap <= 0.01, "overlap {overlap} mm²");
}

#[test]
fn c8_rotated_30_degrees_and_60_mm_wide() {
    let s = 60.0 / f64::from(N);
    let (c, si) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
    c8_case([c * s, si * s, -si * s, c * s, 20.0, 30.0]);
}

#[test]
fn c8_rotated_scaled_and_skewed() {
    c8_case(skew_map(N));
}

// ---------------------------------------------------------------------------------------------
// Criterion 9 and the C9 check: the automatic values read only the region's pixels.

#[test]
fn c9_remove_background_on_the_right_half_uses_the_paper_grey_and_traces_the_grey_square() {
    let m2 = draw(M2, N);
    let right = region(rect(50.0, 0.0, 100.0, 100.0), false);
    let defaults = trace_region_defaults(&m2, N, N, map_100mm(N), &right);
    let bg = defaults.background.expect("an opaque background");
    assert_eq!(
        (bg.r, bg.g, bg.b),
        (0xD8, 0xD8, 0xD8),
        "the right half's paper"
    );
    let params = TraceParams {
        selection: TraceMethod::RemoveBackground {
            background: Some(TraceRgb::new(bg.r, bg.g, bg.b)),
            tolerance: 20,
        },
        ..TraceParams::with_threshold(128)
    };
    let paths = outlines(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: N,
                height: N,
                rgba: &m2,
            },
            map: TracePixelMap::new(map_100mm(N)),
            mode: TraceMode::Mono(&params),
            lines: &[],
            mask: Some(&right),
            shapes: false,
        },
        &mut |_| {},
    ));
    assert!(
        covers(&paths, Point::new(90.0, 10.0)),
        "the grey square is traced"
    );
    assert!(
        covers(&paths, Point::new(75.0, 50.0)),
        "the black square is traced"
    );
    assert!(
        !covers(&paths, Point::new(60.0, 85.0)),
        "no shape comes from the paper"
    );
    assert!(
        !covers(&paths, Point::new(97.0, 97.0)),
        "no shape comes from the paper"
    );
    let traced = area_of(&polygons(&paths));
    // Square 900 mm², grey square 100 mm², the bar's right half 50 x 4 minus the square's 30 x 4.
    let expected = 900.0 + 100.0 + (50.0 - 30.0) * 4.0;
    assert!(
        (traced - expected).abs() <= 0.02 * expected,
        "the paper adds no area: {traced} vs {expected}"
    );
    // Without a mask Remove background's Auto stays the whole picture's suggestion.
    let whole = trace_suggest_background(TraceRaster {
        width: N,
        height: N,
        rgba: &m2,
    });
    let (bx, by, bz) = whole.map_or((0, 0, 0), |c| (c.r, c.g, c.b));
    assert_eq!(
        trace_region_defaults(
            &m2,
            N,
            N,
            map_100mm(N),
            &region(rect(-1.0, -1.0, 101.0, 101.0), false)
        )
        .background
        .map(|c| (c.r, c.g, c.b)),
        Some((bx, by, bz)),
        "a region over the whole picture proposes what the unmasked Auto does"
    );
}

/// The picture's columns `x0..x1` as its own picture.
fn crop_columns(rgba: &[u8], width: u32, height: u32, x0: u32, x1: u32) -> Vec<u8> {
    let mut out = Vec::new();
    for y in 0..height {
        let row = (y * width) as usize * 4;
        out.extend_from_slice(&rgba[row + x0 as usize * 4..row + x1 as usize * 4]);
    }
    out
}

#[test]
fn c9_threshold_background_and_analysis_read_only_the_masks_pixels() {
    // Left half: dark grey (60) with a black disc; right half: light grey paper (200) with mid
    // grey (120) content. The whole picture's Auto values differ from either half's.
    let n = 200_u32;
    let mut rgba = Vec::new();
    for y in 0..n {
        for x in 0..n {
            let c: u8 = if x < n / 2 {
                if (i64::from(x) - 50).pow(2) + (i64::from(y) - 100).pow(2) < 900 {
                    0
                } else {
                    60
                }
            } else if (130..170).contains(&x) && (60..140).contains(&y) {
                120
            } else {
                200
            };
            rgba.extend([c, c, c, 255]);
        }
    }
    let right = region(rect(50.0, 0.0, 100.0, 100.0), false);
    let left = region(rect(50.0, 0.0, 100.0, 100.0), true);
    let crop_r = crop_columns(&rgba, n, n, n / 2, n);
    let crop_l = crop_columns(&rgba, n, n, 0, n / 2);
    let (w, h) = (n / 2, n);
    let dr = trace_region_defaults(&rgba, n, n, map_100mm(n), &right);
    let dl = trace_region_defaults(&rgba, n, n, map_100mm(n), &left);
    assert_eq!(
        dr.threshold,
        trace_auto_threshold(TraceRaster {
            width: w,
            height: h,
            rgba: &crop_r
        }),
        "Threshold of the right half"
    );
    assert_eq!(
        dl.threshold,
        trace_auto_threshold(TraceRaster {
            width: w,
            height: h,
            rgba: &crop_l
        }),
        "Threshold of the left half"
    );
    assert_ne!(
        dr.threshold, dl.threshold,
        "each half has its own Threshold"
    );
    assert_eq!(
        dr.background.map(|c| (c.r, c.g, c.b)),
        trace_suggest_background(TraceRaster {
            width: w,
            height: h,
            rgba: &crop_r
        })
        .map(|c| (c.r, c.g, c.b)),
        "background of the right half"
    );
    assert_eq!(dr.background.map(|c| c.r), Some(200));
    assert_eq!(
        dl.background.map(|c| c.r),
        Some(60),
        "background of the left half (Outside)"
    );
    assert_eq!(
        dr.analysis,
        trace_analyse(TraceRaster {
            width: w,
            height: h,
            rgba: &crop_r
        })
        .ok(),
        "analysis of the right half"
    );
    assert_eq!(
        dl.analysis,
        trace_analyse(TraceRaster {
            width: w,
            height: h,
            rgba: &crop_l
        })
        .ok(),
        "analysis of the left half"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 11: Recognise text and Recognise shapes decided by the region.

#[test]
fn c11_only_lines_whose_box_centre_is_in_the_region_become_text() {
    let n = 200;
    let sheet = text_sheet(n);
    let top = region(rect(0.0, 0.0, 100.0, 53.0), false);
    let p = TraceParams::with_threshold(128);
    let wire = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: n,
                height: n,
                rgba: &sheet,
            },
            map: TracePixelMap::new(map_100mm(n)),
            mode: TraceMode::Mono(&p),
            lines: &ocr_lines(n),
            mask: Some(&top),
            shapes: false,
        },
        &mut |_| {},
    );
    let read = read_trace_wire(&wire).unwrap();
    let unmasked = read_trace_wire(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: n,
                height: n,
                rgba: &sheet,
            },
            map: TracePixelMap::new(map_100mm(n)),
            mode: TraceMode::Mono(&p),
            lines: &ocr_lines(n),
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    ))
    .unwrap();
    assert_eq!(
        unmasked.texts.lines.len(),
        3,
        "all three lines without a mask"
    );
    // The top line (centre 20 mm) and the middle one (centre 50 mm, its glyphs crossing the
    // edge at 53 mm) are text; the bottom one (80 mm) is not.
    assert_eq!(
        read.texts.lines.len(),
        2,
        "two lines are text: {:?}",
        read.texts.lines.len()
    );
    assert!(
        !covers(&read.outlines, Point::new(30.0, 80.0)),
        "the bottom line's glyph"
    );
    // The lines that are text leave no traced shapes behind (their pixels are left out).
    assert!(
        !covers(&read.outlines, Point::new(21.0, 20.0)),
        "the top line is text, not shapes"
    );
}

/// A circle (r 10 mm at 25, 25), a square (20 mm at 75, 75) and a rectangle 35..60 x 45..55 mm
/// that crosses x = 50, on 200 px over 100 mm.
fn shapes_sheet(n: u32) -> Vec<u8> {
    let s = 100.0 / f64::from(n);
    let mut rgba = Vec::new();
    for y in 0..n {
        for x in 0..n {
            let (u, v) = ((f64::from(x) + 0.5) * s, (f64::from(y) + 0.5) * s);
            let ink = (u - 25.0).hypot(v - 25.0) <= 10.0
                || ((65.0..=85.0).contains(&u) && (65.0..=85.0).contains(&v))
                || ((35.0..=60.0).contains(&u) && (45.0..=55.0).contains(&v));
            let c = if ink { 0 } else { 255 };
            rgba.extend([c, c, c, 255]);
        }
    }
    rgba
}

#[test]
fn c11_recognise_shapes_with_a_mask_keeps_nothing_outside_the_region() {
    let n = 200;
    let sheet = shapes_sheet(n);
    let p = TraceParams::with_threshold(128);
    let mut whole = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: n,
                height: n,
                rgba: &sheet,
            },
            map: TracePixelMap::new(map_100mm(n)),
            mode: TraceMode::Mono(&p),
            lines: &[],
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    );
    trace_add_shapes(&mut whole, map_100mm(n));
    let whole = read_trace_wire(&whole).unwrap();
    let recognised = |w: &vectrace::Output| {
        w.shapes
            .iter()
            .filter(|s| s.shapes.first().is_some_and(Option::is_some))
            .count()
    };
    assert_eq!(recognised(&whole), 3, "three shapes without a mask");
    let mut masked = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: n,
                height: n,
                rgba: &sheet,
            },
            map: TracePixelMap::new(map_100mm(n)),
            mode: TraceMode::Mono(&p),
            lines: &[],
            mask: Some(&region(r1(), false)),
            shapes: false,
        },
        &mut |_| {},
    );
    trace_add_shapes(&mut masked, map_100mm(n));
    let masked = read_trace_wire(&masked).unwrap();
    for q in nodes(&masked.outlines) {
        assert!(q.x <= 50.0 + EDGE_MM, "shape node {q:?} outside R1");
    }
    assert!(recognised(&masked) >= 1, "the circle is still a shape");
    assert!(
        covers(&masked.outlines, Point::new(25.0, 25.0)),
        "the circle"
    );
    assert!(
        covers(&masked.outlines, Point::new(45.0, 50.0)),
        "the crossing rectangle's part"
    );
    assert!(
        !covers(&masked.outlines, Point::new(75.0, 75.0)),
        "not the square"
    );
}

// ---------------------------------------------------------------------------------------------
// Multicolour with a mask.

#[test]
fn multicolour_every_layer_is_clipped_and_inside_plus_outside_is_the_unmasked_layer() {
    let n = 250;
    let col = draw(COLOUR, n);
    let params = colour_params();
    let job = |r: Option<&dyn Mask>| {
        read_trace_wire(&run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: n,
                    height: n,
                    rgba: &col,
                },
                map: TracePixelMap::new(map_100mm(n)),
                mode: TraceMode::Colours(&params),
                lines: &[],
                mask: r,
                shapes: false,
            },
            &mut |_| {},
        ))
        .unwrap()
    };
    let whole = job(None);
    let inside = job(Some(&region(r1(), false)));
    let outside = job(Some(&region(r1(), true)));
    let per_layer = |w: &vectrace::Output| {
        let mut at = 0;
        let mut out = Vec::new();
        for l in &w.layers {
            out.push((
                (l.colour.r, l.colour.g, l.colour.b),
                w.outlines[at..at + l.outlines].to_vec(),
            ));
            at += l.outlines;
        }
        assert_eq!(at, w.outlines.len(), "the layers count every outline");
        out
    };
    let (lw, li, lo) = (per_layer(&whole), per_layer(&inside), per_layer(&outside));
    assert!(lw.len() >= 4, "red, blue, green and grey: {}", lw.len());
    for (_, paths) in &li {
        for q in nodes(paths) {
            assert!(q.x <= 50.0 + EDGE_MM, "Inside layer node {q:?}");
        }
    }
    for (_, paths) in &lo {
        for q in nodes(paths) {
            assert!(q.x >= 50.0 - EDGE_MM, "Outside layer node {q:?}");
        }
    }
    let colours_in: Vec<_> = li.iter().map(|(c, _)| *c).collect();
    assert!(
        !colours_in.contains(&(30, 60, 200)),
        "the blue square is outside R1"
    );
    let colours_out: Vec<_> = lo.iter().map(|(c, _)| *c).collect();
    assert!(
        !colours_out.contains(&(200, 30, 30)),
        "the red disc is inside R1"
    );
    for (colour, paths) in &lw {
        let a = li
            .iter()
            .find(|(c, _)| c == colour)
            .map(|(_, p)| p.clone())
            .unwrap_or_default();
        let b = lo
            .iter()
            .find(|(c, _)| c == colour)
            .map(|(_, p)| p.clone())
            .unwrap_or_default();
        let (xor, overlap) = seam(&a, &b, paths);
        assert!(
            xor <= 0.001,
            "colour {colour:?}: symmetric difference {:.5} %",
            xor * 100.0
        );
        assert!(overlap <= 0.01, "colour {colour:?}: overlap {overlap}");
    }
    // Shapes of different colours do not overlap after the clip.
    let shapes: Vec<_> = li.iter().map(|(_, p)| polygons(p)).collect();
    for i in 0..shapes.len() {
        for j in i + 1..shapes.len() {
            let o = op_area(BooleanOp::Intersection, &[&shapes[i], &shapes[j]]);
            assert!(o <= 0.01, "layers {i} and {j} overlap by {o}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// A mask edge crossing a ring and an "O": the hole stays a hole.

#[test]
fn a_mask_edge_across_a_ring_keeps_its_hole() {
    let n = 400_u32;
    let s = 100.0 / f64::from(n);
    let mut rgba = Vec::new();
    for y in 0..n {
        for x in 0..n {
            let (u, v) = ((f64::from(x) + 0.5) * s, (f64::from(y) + 0.5) * s);
            let r = (u - 50.0).hypot(v - 50.0);
            // A ring (r 20..30) and an "O" (ellipse ring) at the bottom left.
            let o = ((u - 20.0) / 10.0).hypot((v - 85.0) / 12.0);
            let ink = (20.0..=30.0).contains(&r) || (0.6..=1.0).contains(&o);
            let c = if ink { 0 } else { 255 };
            rgba.extend([c, c, c, 255]);
        }
    }
    let p = TraceParams::with_threshold(128);
    let whole = outlines(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: n,
                height: n,
                rgba: &rgba,
            },
            map: TracePixelMap::new(map_100mm(n)),
            mode: TraceMode::Mono(&p),
            lines: &[],
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    ));
    // A diagonal half-plane through the ring's centre and through the "O".
    let edge = vec![
        Point::new(-10.0, -10.0),
        Point::new(110.0, -10.0),
        Point::new(-10.0, 110.0),
    ];
    let a = outlines(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: n,
                height: n,
                rgba: &rgba,
            },
            map: TracePixelMap::new(map_100mm(n)),
            mode: TraceMode::Mono(&p),
            lines: &[],
            mask: Some(&region(edge.clone(), false)),
            shapes: false,
        },
        &mut |_| {},
    ));
    let b = outlines(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: n,
                height: n,
                rgba: &rgba,
            },
            map: TracePixelMap::new(map_100mm(n)),
            mode: TraceMode::Mono(&p),
            lines: &[],
            mask: Some(&region(edge.clone(), true)),
            shapes: false,
        },
        &mut |_| {},
    ));
    assert!(
        !covers(&a, Point::new(45.0, 45.0)),
        "the ring's hole stays a hole"
    );
    assert!(
        covers(
            &a,
            Point::new(
                50.0 - 25.0 * std::f64::consts::FRAC_1_SQRT_2,
                50.0 - 25.0 * std::f64::consts::FRAC_1_SQRT_2,
            )
        ),
        "the ring"
    );
    assert!(
        !covers(&a, Point::new(20.0, 85.0)),
        "the O's hole stays a hole"
    );
    for q in nodes(&a) {
        assert!(convex_distance(&edge, q) <= EDGE_MM, "node {q:?}");
    }
    let (xor, overlap) = seam(&a, &b, &whole);
    assert!(xor <= 0.001, "symmetric difference {:.5} %", xor * 100.0);
    assert!(overlap <= 0.01);
}

// ---------------------------------------------------------------------------------------------
// Criterion 15: determinism; one job equals the same job in two threads at once.

#[test]
fn c15_the_same_masked_trace_twice_and_in_two_threads_is_identical() {
    let once = inside_r1();
    assert_eq!(once, inside_r1(), "run twice");
    let threads: Vec<_> = (0..2).map(|_| std::thread::spawn(inside_r1)).collect();
    for t in threads {
        assert_eq!(t.join().unwrap(), once, "a second worker");
    }
    // The region crosses to the worker as numbers and back unchanged.
    let r = region(r1(), true);
    let back = TraceRegion::from_numbers(&r.to_numbers(), true).unwrap();
    assert_eq!(back.polygons(), r.polygons());
    assert!(back.outside());
}

// ---------------------------------------------------------------------------------------------
// Degenerate input.

#[test]
fn degenerate_regions_are_refused_not_panicked_on() {
    assert!(
        TraceRegion::from_polygons(Vec::new(), false).is_none(),
        "no polygon"
    );
    assert!(
        TraceRegion::from_polygons(
            vec![vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)]],
            false
        )
        .is_none(),
        "two points"
    );
    assert!(
        TraceRegion::from_polygons(
            vec![vec![
                Point::new(0.0, 0.0),
                Point::new(f64::NAN, 1.0),
                Point::new(1.0, 0.0)
            ]],
            false
        )
        .is_none(),
        "not finite"
    );
    for bad in [
        vec![],
        vec![1.0],
        vec![1.0, 3.0, 0.0, 0.0, 1.0],
        vec![1.0, 3.0, 0.0, 0.0, 1.0, 0.0, 1.0, f64::INFINITY],
        vec![-1.0],
        vec![1e30],
        vec![2.0, 3.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0],
        vec![1.5, 3.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0],
    ] {
        assert!(TraceRegion::from_numbers(&bad, false).is_none(), "{bad:?}");
    }
    // A zero-area mask (three points on one line) traces nothing and does not panic.
    if let Some(flat) = TraceRegion::from_polygons(
        vec![vec![
            Point::new(0.0, 0.0),
            Point::new(50.0, 50.0),
            Point::new(100.0, 100.0),
        ]],
        false,
    ) {
        let read = read_trace_wire(&run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: N,
                    height: N,
                    rgba: &m1(),
                },
                map: TracePixelMap::new(map_100mm(N)),
                mode: TraceMode::Mono(&t50()),
                lines: &[],
                mask: Some(&flat),
                shapes: false,
            },
            &mut |_| {},
        ))
        .unwrap();
        assert!(
            area_of(&polygons(&read.outlines)) <= 0.01,
            "a flat mask keeps no area"
        );
    }
    // An empty picture with a mask is "invalid", not a panic.
    let read = read_trace_wire(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: 0,
                height: 0,
                rgba: &[],
            },
            map: TracePixelMap::new(map_100mm(1)),
            mode: TraceMode::Mono(&t50()),
            lines: &[],
            mask: Some(&region(r1(), false)),
            shapes: false,
        },
        &mut |_| {},
    ));
    assert!(read.is_some_and(|r| r.outlines.is_empty()));
}

#[test]
fn a_mask_over_the_whole_image_inside_is_the_unmasked_trace() {
    let all = region(rect(-5.0, -5.0, 105.0, 105.0), false);
    let a = outlines(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: N,
                height: N,
                rgba: &m1(),
            },
            map: TracePixelMap::new(map_100mm(N)),
            mode: TraceMode::Mono(&t50()),
            lines: &[],
            mask: Some(&all),
            shapes: false,
        },
        &mut |_| {},
    ));
    let (xor, _) = seam(&a, &[], &outlines(&unmasked()));
    assert!(xor <= 1e-9, "{xor}");
}

// ---------------------------------------------------------------------------------------------
// Property: for any convex mask, Inside and Outside partition the unmasked trace.

proptest! {
    #![proptest_config(ProptestConfig { cases: 12, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn inside_and_outside_partition_the_trace(
        x0 in -20.0..60.0_f64, y0 in -20.0..60.0_f64, w in 5.0..80.0_f64, h in 5.0..80.0_f64,
        skew in -20.0..20.0_f64,
    ) {
        let n = 160_u32;
        let rgba = draw(M1, n);
        let map = map_100mm(n);
        let poly = vec![
            Point::new(x0, y0),
            Point::new(x0 + w, y0 + skew),
            Point::new(x0 + w, y0 + h + skew),
            Point::new(x0, y0 + h),
        ];
        let job = |r: Option<&dyn Mask>| outlines(&run_trace_job(&TraceJob { raster: TraceRaster { width: n, height: n, rgba: &rgba }, map: TracePixelMap::new(map), mode: TraceMode::Mono(&t50()), lines: &[], mask: r, shapes: false }, &mut |_| {}));
        let whole = job(None);
        let a = job(Some(&region(poly.clone(), false)));
        let b = job(Some(&region(poly.clone(), true)));
        for q in nodes(&a) {
            prop_assert!(convex_distance(&poly, q) <= EDGE_MM, "Inside node {:?}", q);
        }
        for q in nodes(&b) {
            prop_assert!(-convex_distance(&poly, q) <= EDGE_MM, "Outside node {:?}", q);
        }
        let (xor, overlap) = seam(&a, &b, &whole);
        prop_assert!(xor <= 0.001, "symmetric difference {}", xor);
        prop_assert!(overlap <= 0.01, "overlap {}", overlap);
    }
}

// ---------------------------------------------------------------------------------------------
// White-box: an outer outline that crosses the edge while its hole does not (the clip sends only
// crossing outlines to the kernel and keeps the others whole), and the node growth of crossing
// shapes.

#[test]
fn a_ring_cut_only_on_its_outer_outline_keeps_its_hole() {
    let n = 400_u32;
    let s = 100.0 / f64::from(n);
    let mut rgba = Vec::new();
    for y in 0..n {
        for x in 0..n {
            let (u, v) = ((f64::from(x) + 0.5) * s, (f64::from(y) + 0.5) * s);
            let r = (u - 50.0).hypot(v - 50.0);
            let c = if (15.0..=35.0).contains(&r) { 0 } else { 255 };
            rgba.extend([c, c, c, 255]);
        }
    }
    let p = TraceParams::with_threshold(128);
    let whole = outlines(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: n,
                height: n,
                rgba: &rgba,
            },
            map: TracePixelMap::new(map_100mm(n)),
            mode: TraceMode::Mono(&p),
            lines: &[],
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    ));
    // x = 75 cuts the outer circle (r 35) but not the hole (r 15).
    let left = rect(-10.0, -10.0, 75.0, 110.0);
    for outside in [false, true] {
        let got = outlines(&run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: n,
                    height: n,
                    rgba: &rgba,
                },
                map: TracePixelMap::new(map_100mm(n)),
                mode: TraceMode::Mono(&p),
                lines: &[],
                mask: Some(&region(left.clone(), outside)),
                shapes: false,
            },
            &mut |_| {},
        ));
        if !outside {
            assert!(
                !covers(&got, Point::new(50.0, 50.0)),
                "the hole stays a hole"
            );
            assert!(covers(&got, Point::new(25.0, 50.0)), "the ring");
        }
        let expected = op_area(
            if outside {
                BooleanOp::Difference
            } else {
                BooleanOp::Intersection
            },
            &[&polygons(&whole), &[poly_triples(&left)]],
        );
        let area = area_of(&polygons(&got));
        assert!(
            (area - expected).abs() <= 0.001 * expected,
            "outside {outside}: {area} vs {expected}"
        );
    }
}

#[test]
fn node_growth_of_crossing_shapes_is_reported() {
    let count = |w: &[f64]| read_trace_wire(w).unwrap().nodes;
    let (u, i, o) = (
        count(&unmasked()),
        count(&inside_r1()),
        count(&outside_r1()),
    );
    eprintln!("M1 nodes: unmasked {u}, Inside R1 {i}, Outside R1 {o}");
    // Polylines at the kernel's flattening: a bound far above a curve fit, below a pixel staircase.
    assert!(i + o < 20 * u.max(10), "nodes {u} -> {i} + {o}");
}

// ---------------------------------------------------------------------------------------------
// Re-test 2026-11-05, item 3: a clip the kernel refuses keeps crossing shapes whole and counts
// them on the wire, only when above zero; the muted row names the count. Written from the lead's
// brief before the fix was read. A refusal is forced with coordinates beyond the kernel's range.

/// Where the outlines of a result end: the header and the outlines skipped by their own numbers.
fn after_outlines(wire: &[f64]) -> usize {
    let mut at = 7;
    for _ in 0..wire[6] as usize {
        let elements = wire[at] as usize;
        at += 1;
        for _ in 0..elements {
            let tag = wire[at];
            at += 1 + match tag as i64 {
                0 | 1 => 2,
                3 => 6,
                4 => 0,
                other => panic!("unknown tag {other}"),
            };
        }
    }
    at
}

const FAR: f64 = 2.0e7;

/// 32 x 32 px at 1 mm per px, far away: three black bars crossing x = 16 px, one square wholly
/// left of it, white paper.
fn far_picture() -> Vec<u8> {
    (0..32 * 32)
        .flat_map(|i| {
            let (x, y) = (i % 32, i / 32);
            let bar = (10..22).contains(&x)
                && ((2..7).contains(&y) || (11..16).contains(&y) || (20..25).contains(&y));
            let square = (2..6).contains(&x) && (27..31).contains(&y);
            let v = if bar || square { 0 } else { 255 };
            [v, v, v, 255]
        })
        .collect()
}

fn far_map() -> [f64; 6] {
    [1.0, 0.0, 0.0, 1.0, FAR, 0.0]
}

fn far_left_half(outside: bool) -> TraceRegion {
    region(rect(FAR - 1.0, -1.0, FAR + 16.0, 33.0), outside)
}

fn far_params() -> TraceParams {
    TraceParams {
        specks: vectrace::Mm::from_mm(0.0),
        ..TraceParams::with_threshold(128)
    }
}

#[test]
fn item3_the_unclipped_notice_counts_in_singular_and_plural() {
    assert_eq!(curvyo_ui_core::unclipped_notice(0), "");
    assert_eq!(
        curvyo_ui_core::unclipped_notice(1),
        "1 shape crosses the mask edge unclipped."
    );
    assert_eq!(
        curvyo_ui_core::unclipped_notice(3),
        "3 shapes cross the mask edge unclipped."
    );
}

#[test]
fn item3_a_refused_clip_counts_every_crossing_shape_kept_whole_and_not_the_others() {
    let wire = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: 32,
                height: 32,
                rgba: &far_picture(),
            },
            map: TracePixelMap::new(far_map()),
            mode: TraceMode::Mono(&far_params()),
            lines: &[],
            mask: Some(&far_left_half(false)),
            shapes: false,
        },
        &mut |_| {},
    );
    let read = read_trace_wire(&wire).expect("a readable wire");
    assert_eq!(
        read.outlines.len(),
        4,
        "three bars kept whole and the square"
    );
    assert_eq!(read.unclipped, 3, "the square inside crossed nothing");
    assert_eq!(read.paths, 4, "the counts agree with the outlines");
    // Kept whole: a bar still reaches past the mask edge at x = 16 mm.
    let reach = nodes(&read.outlines)
        .iter()
        .map(|p| p.x - FAR)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(reach > 20.0, "the bars are not clipped ({reach})");
    // The count opens right after the outlines.
    let at = after_outlines(&wire);
    assert_eq!(&wire[at..], &[-5.0, 3.0]);
}

#[test]
fn item3_outside_a_refused_clip_counts_only_what_it_kept() {
    let read = read_trace_wire(&run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: 32,
                height: 32,
                rgba: &far_picture(),
            },
            map: TracePixelMap::new(far_map()),
            mode: TraceMode::Mono(&far_params()),
            lines: &[],
            mask: Some(&far_left_half(true)),
            shapes: false,
        },
        &mut |_| {},
    ))
    .unwrap();
    // The square lies inside the mask and goes; each bar is kept whole or dropped by its first
    // point, and the count is exactly the bars that stayed.
    let bars_kept = read.outlines.len();
    assert_eq!(read.unclipped, bars_kept);
    assert!(bars_kept <= 3);
}

#[test]
fn item3_a_refused_multicolour_clip_is_counted_and_the_palette_still_reads() {
    let wire = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: 32,
                height: 32,
                rgba: &far_picture(),
            },
            map: TracePixelMap::new(far_map()),
            mode: TraceMode::Colours(&colour_params()),
            lines: &[],
            mask: Some(&far_left_half(false)),
            shapes: false,
        },
        &mut |_| {},
    );
    let read = read_trace_wire(&wire).expect("a readable multicolour wire");
    assert_eq!(read.unclipped, 3);
    assert!(!read.palette.is_empty(), "the palette after the count");
    let in_layers: usize = read.layers.iter().map(|l| l.outlines).sum();
    assert_eq!(in_layers, read.outlines.len());
    assert_eq!(wire[after_outlines(&wire)], -5.0);
}

#[test]
fn item3_a_refused_clip_with_text_lines_and_shapes_keeps_every_section() {
    // One OCR line over the square in the left half, centre (4, 29) px.
    let line = OcrLine {
        quad: [[1.0, 26.0], [7.0, 26.0], [7.0, 32.0], [1.0, 32.0]],
        text: "Hello".into(),
        confidence: vec![0.99; 5],
        positions: Vec::new(),
    };
    let mut wire = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: 32,
                height: 32,
                rgba: &far_picture(),
            },
            map: TracePixelMap::new(far_map()),
            mode: TraceMode::Mono(&far_params()),
            lines: &[line],
            mask: Some(&far_left_half(false)),
            shapes: false,
        },
        &mut |_| {},
    );
    let read = read_trace_wire(&wire).expect("a readable text wire");
    assert_eq!(read.unclipped, 3);
    assert_eq!(read.texts.lines.len(), 1, "the line in the region is text");
    trace_add_shapes(&mut wire, far_map());
    let read = read_trace_wire(&wire).expect("readable with the shape section");
    assert_eq!(read.unclipped, 3, "the count survives the shape section");
    assert_eq!(read.texts.lines.len(), 1);
    assert_eq!(read.outlines.len(), 3, "the square is the text line now");
}

#[test]
fn item3_no_count_on_the_wire_when_every_clip_succeeds() {
    for wire in [inside_r1(), outside_r1(), unmasked()] {
        let read = read_trace_wire(&wire).unwrap();
        assert_eq!(read.unclipped, 0);
        assert_eq!(
            after_outlines(&wire),
            wire.len(),
            "nothing after the outlines"
        );
    }
    let n = 200;
    let colour = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: n,
                height: n,
                rgba: &draw(COLOUR, n),
            },
            map: TracePixelMap::new(map_100mm(n)),
            mode: TraceMode::Colours(&colour_params()),
            lines: &[],
            mask: Some(&region(r1(), false)),
            shapes: false,
        },
        &mut |_| {},
    );
    let read = read_trace_wire(&colour).unwrap();
    assert_eq!(read.unclipped, 0);
    let at = after_outlines(&colour);
    assert_ne!(
        colour[at], -5.0,
        "the palette follows the outlines directly"
    );
}

/// Re-test 2026-11-05, item 5: the node count `docs/technical-debt.md` gives for M1 Inside R1 at
/// Threshold 50 is the one the job reports.
#[test]
fn item5_the_debt_entry_gives_m1_insides_node_count() {
    let read = read_trace_wire(&inside_r1()).unwrap();
    let unmasked = read_trace_wire(&unmasked()).unwrap();
    println!(
        "inside {} nodes, unmasked {} nodes",
        read.nodes, unmasked.nodes
    );
    assert_eq!(read.nodes, 168);
}
