//! Tester cases for `specs/0094-masked-composite-trace` slice 2 in the core (criteria 20 to 23,
//! 30 and 33; determinism): parts traced each with their own settings through the clip each one
//! keeps once the later parts are taken out (`trace_part_clips`), then joined
//! (`join_trace_parts`). Written from the specification and the public API before the
//! implementation was read, on the tester's own synthetic fixtures (`support_0094`): two and
//! three parts, overlapping and not, Multicolour in both parts, Recognise text in part 1 only.
//!
//! The pictures are drawn at 200 to 500 px (M1's own 1000 px frame sampled at pixel centres) to
//! keep each case well under 20 s in debug; the bounds are relative or in mm and do not depend on
//! the resolution.

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
use curvyo_ui_core::{
    TracePartArea, TracePartClip, TraceRegion, join_trace_parts, read_trace_wire, trace_part_clips,
};
use kurbo::{BezPath, PathEl};
use proptest::prelude::*;
use support::{COLOUR, M1, draw, map_100mm, text_sheet};
use vectrace::Output;
use vectrace::job::{Job as TraceJob, Mode as TraceMode, run as run_trace_job};
use vectrace::mask::Mask;
use vectrace::ocr::OcrLine;
use vectrace::wire::Status;
use vectrace::{
    ColourParams as TraceColourParams, PixelMap as TracePixelMap, Raster as TraceRaster,
    Rgb8 as TraceRgb, Selection as TraceMethod, TraceParams,
};

// ---------------------------------------------------------------------------------------------
// Geometry helpers through the kernel (as the slice 1 tester's).

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

fn triples(points: &[Point]) -> Vec<OutlineTriple> {
    points
        .iter()
        .map(|&p| (p, Vec2::ZERO, Vec2::ZERO))
        .collect()
}

fn tol() -> Tolerance {
    Tolerance::from_mm(0.003)
}

type Polys = Vec<Vec<OutlineTriple>>;

/// `op` over two operands as kernel polygons (nonzero); empty when nothing is left.
fn op(op: BooleanOp, a: &Polys, b: &Polys) -> Polys {
    match (a.is_empty(), b.is_empty()) {
        (true, true) => return Vec::new(),
        (true, false) => {
            return match op {
                BooleanOp::Union | BooleanOp::Exclusion => b.clone(),
                _ => Vec::new(),
            };
        }
        (false, true) => {
            return match op {
                BooleanOp::Intersection => Vec::new(),
                _ => a.clone(),
            };
        }
        _ => {}
    }
    let oa: Vec<Outline<'_>> = a.iter().map(|x| Outline::new(x, true)).collect();
    let ob: Vec<Outline<'_>> = b.iter().map(|x| Outline::new(x, true)).collect();
    match boolean(op, &[&oa, &ob], tol()) {
        Ok(r) => r.outlines().iter().map(|p| triples(p)).collect(),
        Err(BooleanError::EmptyResult) => Vec::new(),
        Err(e) => panic!("kernel refused {op:?}: {e:?}"),
    }
}

/// The painted area (nonzero) of one operand.
fn area(a: &Polys) -> f64 {
    if a.is_empty() {
        return 0.0;
    }
    let o: Vec<Outline<'_>> = a.iter().map(|x| Outline::new(x, true)).collect();
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

/// The symmetric difference of `got` and `want`, relative to `want`'s area.
fn xor_ratio(got: &Polys, want: &Polys) -> f64 {
    let x = area(&op(BooleanOp::Exclusion, got, want));
    x / area(want).max(1e-9)
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Point> {
    vec![
        Point::new(x0, y0),
        Point::new(x1, y0),
        Point::new(x1, y1),
        Point::new(x0, y1),
    ]
}

fn rect_polys(x0: f64, y0: f64, x1: f64, y1: f64) -> Polys {
    vec![triples(&rect(x0, y0, x1, y1))]
}

fn inside(poly: Vec<Point>) -> TraceRegion {
    TraceRegion::from_polygons(vec![poly], false).expect("a region")
}

fn outside(poly: Vec<Point>) -> TraceRegion {
    TraceRegion::from_polygons(vec![poly], true).expect("a region")
}

/// The picture's corners: 100 x 100 mm at the origin.
fn quad() -> [Point; 4] {
    [
        Point::new(0.0, 0.0),
        Point::new(100.0, 0.0),
        Point::new(100.0, 100.0),
        Point::new(0.0, 100.0),
    ]
}

fn covers(paths: &[BezPath], p: Point) -> bool {
    let polys = polygons(paths);
    let o: Vec<Outline<'_>> = polys.iter().map(|a| Outline::new(a, true)).collect();
    curvyo_geometry_core::contains_point_in_outlines(&o, p)
}

// ---------------------------------------------------------------------------------------------
// The pipeline as the worker and the session run it: clips, one job per contributing part over
// its clip, the join.

#[derive(Clone)]
enum Job {
    Mono(TraceParams),
    Colour(TraceColourParams),
    Text(TraceParams, Vec<OcrLine>),
}

struct Picture {
    rgba: Vec<u8>,
    n: u32,
}

fn job(pic: &Picture, job: &Job, region: Option<&TraceRegion>) -> Vec<f64> {
    let map = map_100mm(pic.n);
    let region = region.map(|r| r as &dyn Mask);
    match job {
        Job::Mono(p) => run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: pic.n,
                    height: pic.n,
                    rgba: &pic.rgba,
                },
                map: TracePixelMap::new(map),
                mode: TraceMode::Mono(p),
                lines: &[],
                mask: region,
                shapes: false,
            },
            &mut |_| {},
        ),
        Job::Colour(p) => run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: pic.n,
                    height: pic.n,
                    rgba: &pic.rgba,
                },
                map: TracePixelMap::new(map),
                mode: TraceMode::Colours(p),
                lines: &[],
                mask: region,
                shapes: false,
            },
            &mut |_| {},
        ),
        Job::Text(p, lines) => run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: pic.n,
                    height: pic.n,
                    rgba: &pic.rgba,
                },
                map: TracePixelMap::new(map),
                mode: TraceMode::Mono(p),
                lines,
                mask: region,
                shapes: false,
            },
            &mut |_| {},
        ),
    }
}

/// The joined result of `parts` (stacking order, bottom first); the active part is the last.
fn joined(pic: &Picture, parts: &[(TracePartArea, Job)]) -> (Output, Vec<TracePartClip>) {
    let areas: Vec<TracePartArea> = parts.iter().map(|(a, _)| a.clone()).collect();
    let clips = trace_part_clips(&areas, &quad());
    assert_eq!(clips.len(), parts.len(), "one clip per part");
    let wires: Vec<(Output, TracePartClip)> = parts
        .iter()
        .zip(&clips)
        .filter(|(_, clip)| clip.contributes())
        .map(|((_, j), clip)| {
            let wire = job(pic, j, clip.region());
            (
                read_trace_wire(&wire).expect("a readable wire"),
                clip.clone(),
            )
        })
        .collect();
    let refs: Vec<(&Output, &TracePartClip)> = wires.iter().map(|(w, c)| (w, c)).collect();
    let palette_of = refs.len().saturating_sub(1);
    (join_trace_parts(&refs, palette_of), clips)
}

fn read(wire: &[f64]) -> Output {
    read_trace_wire(wire).expect("a readable wire")
}

fn mono(threshold: u8, smooth: u8) -> Job {
    Job::Mono(TraceParams {
        smooth,
        ..TraceParams::with_threshold(threshold)
    })
}

fn m1(n: u32) -> Picture {
    Picture {
        rgba: draw(M1, n),
        n,
    }
}

/// The grey square of M1 is at x 85..95, y 5..15 mm; its centre.
const GREY: (f64, f64) = (90.0, 10.0);

// ---------------------------------------------------------------------------------------------
// Criterion 20: part 1 whole at Threshold 40, part 2 R1 Inside at Threshold 70.

fn c20_case(pic: &Picture, t1: u8, t2: u8, r: Vec<Point>) -> (f64, f64, Output) {
    let (join, clips) = joined(
        pic,
        &[
            (TracePartArea::Whole, mono(t1, 50)),
            (TracePartArea::Mask(inside(r.clone())), mono(t2, 50)),
        ],
    );
    assert!(
        clips.iter().all(TracePartClip::contributes),
        "both parts add something: {clips:?}"
    );
    assert_eq!(join.status, Status::Traced);
    let got = polygons(&join.outlines);
    let rp = vec![triples(&r)];
    let ref_in = polygons(&read(&job(pic, &mono(t2, 50), Some(&inside(r.clone())))).outlines);
    let ref_out = polygons(&read(&job(pic, &mono(t1, 50), Some(&outside(r)))).outlines);
    let got_in = op(BooleanOp::Intersection, &got, &rp);
    let got_out = op(BooleanOp::Difference, &got, &rp);
    (
        xor_ratio(&got_in, &ref_in),
        xor_ratio(&got_out, &ref_out),
        join,
    )
}

#[test]
fn c20_inside_r1_is_part_2s_trace_outside_is_part_1s_within_0_1_percent() {
    let pic = m1(500);
    let (inner, outer, _) = c20_case(&pic, 40, 70, rect(0.0, 0.0, 50.0, 100.0));
    assert!(inner <= 0.001, "inside R1: symmetric difference {inner}");
    assert!(outer <= 0.001, "outside R1: symmetric difference {outer}");
}

#[test]
fn c20_each_part_keeps_its_own_threshold_the_grey_square_follows_the_part_over_it() {
    // M1's grey (luma 128) is content at Threshold 150 and paper at 50. The right half as part 2
    // decides the grey square; part 1 (whole image) decides the rest.
    let pic = m1(400);
    let right = rect(50.0, 0.0, 100.0, 100.0);
    let (inner, outer, join) = c20_case(&pic, 50, 150, right.clone());
    assert!(inner <= 0.001 && outer <= 0.001, "{inner} {outer}");
    assert!(
        covers(&join.outlines, Point::new(GREY.0, GREY.1)),
        "part 2 at 150 traces the grey"
    );
    let (inner, outer, join) = c20_case(&pic, 150, 50, right);
    assert!(inner <= 0.001 && outer <= 0.001, "{inner} {outer}");
    assert!(
        !covers(&join.outlines, Point::new(GREY.0, GREY.1)),
        "part 2 at 50 leaves the grey, part 1's 150 does not leak into part 2's region"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 21: overlapping regions, last on top.

#[test]
fn c21_in_the_overlap_only_the_later_part_and_the_earlier_is_cut_at_its_edge() {
    let pic = m1(400);
    // A covers x 0..95 (the grey square too), B covers x 60..100; the overlap is x 60..95.
    let a = rect(0.0, 0.0, 95.0, 100.0);
    let b = rect(60.0, 0.0, 100.0, 100.0);
    for (ta, tb, grey) in [(150_u8, 50_u8, false), (50, 150, true)] {
        let (join, clips) = joined(
            &pic,
            &[
                (TracePartArea::Mask(inside(a.clone())), mono(ta, 50)),
                (TracePartArea::Mask(inside(b.clone())), mono(tb, 50)),
            ],
        );
        assert!(clips.iter().all(TracePartClip::contributes), "{clips:?}");
        assert_eq!(
            covers(&join.outlines, Point::new(GREY.0, GREY.1)),
            grey,
            "the grey square lies in the overlap: part 2 at {tb} decides it"
        );
        let got = polygons(&join.outlines);
        let bp = vec![triples(&b)];
        let ap = vec![triples(&a)];
        let ref_b = polygons(&read(&job(&pic, &mono(tb, 50), Some(&inside(b.clone())))).outlines);
        let ref_a_minus_b = op(
            BooleanOp::Difference,
            &polygons(&read(&job(&pic, &mono(ta, 50), Some(&inside(a.clone())))).outlines),
            &bp,
        );
        let in_b = op(BooleanOp::Intersection, &got, &bp);
        let in_a_only = op(
            BooleanOp::Difference,
            &op(BooleanOp::Intersection, &got, &ap),
            &bp,
        );
        let r = xor_ratio(&in_b, &ref_b);
        assert!(r <= 0.001, "inside B only part 2's shapes: {r}");
        let r = xor_ratio(&in_a_only, &ref_a_minus_b);
        assert!(r <= 0.001, "part 1 is cut at B's stencil edge: {r}");
        // Nothing outside A ∪ B.
        let ab = op(BooleanOp::Union, &ap, &bp);
        let beyond = area(&op(BooleanOp::Difference, &got, &ab));
        assert!(beyond <= 0.01, "nothing outside the regions: {beyond} mm²");
    }
}

/// The outlines of each layer of a multicolour result, with its colour.
fn layers(w: &Output) -> Vec<(TraceRgb, Vec<BezPath>)> {
    let mut at = 0;
    w.layers
        .iter()
        .map(|l| {
            let paths = w.outlines[at..at + l.outlines].to_vec();
            at += l.outlines;
            (l.colour, paths)
        })
        .collect()
}

fn colour(tolerance: u8) -> Job {
    Job::Colour(TraceColourParams::new(TraceParams {
        selection: TraceMethod::RemoveBackground {
            background: Some(TraceRgb::new(255, 255, 255)),
            tolerance,
        },
        specks: vectrace::Mm::from_mm(0.0),
        ..TraceParams::with_threshold(128)
    }))
}

fn colour_pic(n: u32) -> Picture {
    Picture {
        rgba: draw(COLOUR, n),
        n,
    }
}

#[test]
fn c21_c23_multicolour_overlap_different_colours_overlap_at_most_0_01_mm2_and_one_layer_per_rgb() {
    let pic = colour_pic(240);
    let a = rect(0.0, 0.0, 70.0, 100.0);
    let b = rect(40.0, 20.0, 100.0, 80.0);
    let (join, _) = joined(
        &pic,
        &[
            (TracePartArea::Whole, colour(40)),
            (TracePartArea::Mask(inside(a)), colour(30)),
            (TracePartArea::Mask(inside(b)), colour(50)),
        ],
    );
    assert_eq!(join.status, Status::Traced);
    let ls = layers(&join);
    assert!(
        ls.len() >= 3,
        "red, blue, green at least: {:?}",
        join.layers
    );
    let mut seen: Vec<TraceRgb> = Vec::new();
    for (c, _) in &ls {
        assert!(!seen.contains(c), "one layer per distinct RGB: {c:?} twice");
        seen.push(*c);
    }
    let polys: Vec<Polys> = ls.iter().map(|(_, p)| polygons(p)).collect();
    for i in 0..polys.len() {
        for j in i + 1..polys.len() {
            let o = area(&op(BooleanOp::Intersection, &polys[i], &polys[j]));
            assert!(
                o <= 0.01,
                "{:?} and {:?} overlap by {o} mm²",
                ls[i].0,
                ls[j].0
            );
        }
    }
}

#[test]
fn c23_multicolour_in_both_parts_same_rgb_merges_across_the_seam() {
    let pic = colour_pic(240);
    let (join, _) = joined(
        &pic,
        &[
            (TracePartArea::Whole, colour(40)),
            (
                TracePartArea::Mask(inside(rect(0.0, 0.0, 50.0, 100.0))),
                colour(40),
            ),
        ],
    );
    let ls = layers(&join);
    let whole = read(&job(&pic, &colour(40), None));
    let want: Vec<TraceRgb> = layers(&whole).iter().map(|(c, _)| *c).collect();
    let mut got: Vec<TraceRgb> = ls.iter().map(|(c, _)| *c).collect();
    let mut want_sorted = want.clone();
    got.sort_by_key(|c| (c.r, c.g, c.b));
    want_sorted.sort_by_key(|c| (c.r, c.g, c.b));
    assert_eq!(
        got, want_sorted,
        "the same settings in both parts give the whole picture's colours, each once"
    );
    // The green bar crosses the seam at x = 50 mm: one closed outline in its layer.
    let green = ls
        .iter()
        .find(|(c, _)| c.g > c.r && c.g > c.b)
        .expect("a green layer");
    // The bar is cut by the disc and the square into three pieces in the whole picture's trace
    // too; the middle piece crosses the seam and stays one outline.
    let whole_green = layers(&whole)
        .into_iter()
        .find(|(c, _)| c.g > c.r && c.g > c.b)
        .unwrap();
    assert_eq!(
        green.1.len(),
        whole_green.1.len(),
        "the bar's pieces as in the whole picture: no extra outline at the seam"
    );
    let doubled = doubled_along_x(&green.1, 50.0);
    let seam = length_along_x(&green.1, 50.0);
    assert!(
        doubled <= 0.01 && seam < 0.5,
        "the seam closes: {doubled} {seam}"
    );
    for (c, paths) in &ls {
        let r = xor_ratio(
            &polygons(paths),
            &polygons(&layers(&whole).iter().find(|(w, _)| w == c).unwrap().1),
        );
        assert!(r <= 0.001, "{c:?} equals the whole picture's: {r}");
    }
}

#[test]
fn c23_a_different_rgb_in_a_part_stays_a_layer_of_its_own() {
    // Part 2's fixed background differs: its Remove background keeps the paper of the left half
    // white-ish, but the colours its own palette finds are rounded per part. Whatever they are,
    // the join has exactly the distinct colours of its parts' layers, each once.
    let pic = colour_pic(240);
    let r1 = rect(0.0, 0.0, 50.0, 100.0);
    let p2 = Job::Colour(TraceColourParams::new(TraceParams {
        selection: TraceMethod::RemoveBackground {
            background: Some(TraceRgb::new(255, 255, 255)),
            tolerance: 40,
        },
        specks: vectrace::Mm::from_mm(0.0),
        invert: false,
        ..TraceParams::with_threshold(128)
    }));
    let (join, clips) = joined(
        &pic,
        &[
            (TracePartArea::Whole, colour(60)),
            (TracePartArea::Mask(inside(r1)), p2.clone()),
        ],
    );
    let w1 = read(&job(&pic, &colour(60), clips[0].region()));
    let w2 = read(&job(&pic, &p2, clips[1].region()));
    let mut want: Vec<TraceRgb> = layers(&w1)
        .into_iter()
        .chain(layers(&w2))
        .filter(|(_, p)| !p.is_empty())
        .map(|(c, _)| c)
        .collect();
    want.sort_by_key(|c| (c.r, c.g, c.b));
    want.dedup();
    let mut got: Vec<TraceRgb> = layers(&join).iter().map(|(c, _)| *c).collect();
    got.sort_by_key(|c| (c.r, c.g, c.b));
    assert_eq!(got, want, "one layer per distinct RGB over all parts");
}

// ---------------------------------------------------------------------------------------------
// Criterion 22: the seam closes.

/// The segments of `paths` (flattened) that lie along the vertical line `x`, as y intervals.
fn along_x(paths: &[BezPath], x: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for poly in polygons(paths) {
        for i in 0..poly.len() {
            let (a, b) = (poly[i].0, poly[(i + 1) % poly.len()].0);
            if (a.x - x).abs() < 0.001 && (b.x - x).abs() < 0.001 {
                out.push((a.y.min(b.y), a.y.max(b.y)));
            }
        }
    }
    out
}

/// Criterion 22's measure: the length over which two result segments overlap along the seam.
fn doubled_along_x(paths: &[BezPath], x: f64) -> f64 {
    let segs = along_x(paths, x);
    let mut total = 0.0;
    for i in 0..segs.len() {
        for j in i + 1..segs.len() {
            total += (segs[i].1.min(segs[j].1) - segs[i].0.max(segs[j].0)).max(0.0);
        }
    }
    total
}

/// Any part of the outline along the seam (a jog where two smoothings meet is allowed but short).
fn length_along_x(paths: &[BezPath], x: f64) -> f64 {
    along_x(paths, x).iter().map(|(a, b)| b - a).sum()
}

#[test]
fn c22_the_bar_across_the_seam_is_one_closed_outline_and_no_stencil_edge_remains() {
    let pic = m1(500);
    let (join, _) = joined(
        &pic,
        &[
            (TracePartArea::Whole, mono(50, 20)),
            (
                TracePartArea::Mask(inside(rect(0.0, 0.0, 50.0, 100.0))),
                mono(50, 80),
            ),
        ],
    );
    assert_eq!(join.status, Status::Traced);
    assert_eq!(
        join.outlines.len(),
        1,
        "disc, bar and square are one path (the grey is paper at 50)"
    );
    assert_eq!(join.paths, 1, "the header counts one path");
    let doubled = doubled_along_x(&join.outlines, 50.0);
    assert!(
        doubled <= 0.01,
        "no two segments overlap along x = 50 mm: {doubled} mm"
    );
    // The bar is 4 mm high where it crosses the seam: no stencil edge across it either.
    let seam = length_along_x(&join.outlines, 50.0);
    assert!(
        seam < 0.5,
        "no stencil edge across the bar at x = 50 mm: {seam} mm"
    );
    assert!(
        matches!(join.outlines[0].elements().last(), Some(PathEl::ClosePath)),
        "the outline is closed"
    );
}

#[test]
fn c22_three_parts_side_by_side_still_one_outline_and_no_seams() {
    let pic = m1(400);
    let (join, _) = joined(
        &pic,
        &[
            (TracePartArea::Whole, mono(50, 10)),
            (
                TracePartArea::Mask(inside(rect(-5.0, -5.0, 35.0, 105.0))),
                mono(50, 90),
            ),
            (
                TracePartArea::Mask(inside(rect(65.0, -5.0, 105.0, 105.0))),
                mono(50, 60),
            ),
        ],
    );
    assert_eq!(join.outlines.len(), 1, "one path over three parts");
    for x in [35.0, 65.0] {
        let doubled = doubled_along_x(&join.outlines, x);
        assert!(
            doubled <= 0.01,
            "no two segments overlap along x = {x} mm: {doubled} mm"
        );
        // The seam runs through the disc and the square: only the jogs where the two smoothings
        // meet their outline may lie on it, never the stencil edge across the shape (> 20 mm).
        let seam = length_along_x(&join.outlines, x);
        assert!(
            seam < 0.5,
            "no stencil edge inside the shape at x = {x} mm: {seam} mm"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Criterion 30: Recognise text in part 1 only.

fn ocr(n: u32) -> Vec<OcrLine> {
    let nf = f64::from(n);
    ["Top", "Middle", "Bottom"]
        .iter()
        .zip([0.15, 0.45, 0.75])
        .map(|(text, top)| {
            let (x0, x1) = (0.18 * nf, 0.82 * nf);
            let (y0, y1) = ((top - 0.02) * nf, (top + 0.12) * nf);
            OcrLine {
                quad: [[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
                text: (*text).into(),
                confidence: vec![0.99; text.len()],
                positions: Vec::new(),
            }
        })
        .collect()
}

#[test]
fn c30_text_in_part_1_is_text_a_line_in_part_2_stays_paths() {
    let n = 300;
    let pic = Picture {
        rgba: text_sheet(n),
        n,
    };
    // Part 2: the bottom 40 % (the bottom line's centre at 81 mm is in it, the middle line's
    // centre at 50 mm is not).
    let bottom = rect(-1.0, 60.0, 101.0, 101.0);
    let (join, _) = joined(
        &pic,
        &[
            (
                TracePartArea::Whole,
                Job::Text(TraceParams::with_threshold(128), ocr(n)),
            ),
            (
                TracePartArea::Mask(inside(bottom.clone())),
                Job::Mono(TraceParams::with_threshold(128)),
            ),
        ],
    );
    let texts: Vec<&str> = join.texts.lines.iter().map(|l| l.text.as_str()).collect();
    assert!(texts.contains(&"Top"), "{texts:?}");
    assert!(texts.contains(&"Middle"), "{texts:?}");
    assert!(
        !texts.contains(&"Bottom"),
        "the line in part 2 is not text: {texts:?}"
    );
    // The bottom line's blocks are traced paths of part 2.
    let got = polygons(&join.outlines);
    let in_bottom = area(&op(BooleanOp::Intersection, &got, &vec![triples(&bottom)]));
    // Five blocks of 9 % x 10 % of 100 mm: about 5 x 9 x 10 = 450 mm².
    assert!(
        in_bottom > 300.0,
        "the bottom line is traced as paths: {in_bottom} mm²"
    );
    // The top and middle lines are not traced as paths (their pixels went to text).
    let upper = area(&op(
        BooleanOp::Intersection,
        &got,
        &rect_polys(0.0, 0.0, 100.0, 60.0),
    ));
    assert!(upper < 50.0, "text lines are not also paths: {upper} mm²");
}

// ---------------------------------------------------------------------------------------------
// Criterion 33 and the one-part identity: one part is slice 1, number for number.

#[test]
fn c33_one_part_is_exactly_the_slice_1_trace() {
    let pic = m1(250);
    let r1 = rect(0.0, 0.0, 50.0, 100.0);
    for (area_, region) in [
        (TracePartArea::Whole, None),
        (
            TracePartArea::Mask(inside(r1.clone())),
            Some(inside(r1.clone())),
        ),
        (
            TracePartArea::Mask(outside(r1.clone())),
            Some(outside(r1.clone())),
        ),
    ] {
        let clips = trace_part_clips(std::slice::from_ref(&area_), &quad());
        assert_eq!(clips.len(), 1);
        assert_eq!(
            clips[0].region(),
            region.as_ref(),
            "one part keeps its own area"
        );
        let (join, _) = joined(&pic, &[(area_.clone(), mono(50, 50))]);
        let single = read(&job(&pic, &mono(50, 50), region.as_ref()));
        assert_eq!(join, single, "one part's join is its trace ({area_:?})");
    }
}

#[test]
fn a_later_muted_part_changes_nothing() {
    let pic = m1(250);
    let r1 = rect(0.0, 0.0, 50.0, 100.0);
    let areas = [
        TracePartArea::Mask(inside(r1.clone())),
        TracePartArea::Nothing,
    ];
    let clips = trace_part_clips(&areas, &quad());
    assert!(
        clips[0].contributes() && !clips[1].contributes(),
        "{clips:?}"
    );
    let (join, _) = joined(
        &pic,
        &[
            (areas[0].clone(), mono(50, 50)),
            (areas[1].clone(), mono(90, 50)),
        ],
    );
    assert_eq!(join, read(&job(&pic, &mono(50, 50), Some(&inside(r1)))));
}

#[test]
fn a_part_covered_by_a_later_one_adds_nothing() {
    let areas = [
        TracePartArea::Mask(inside(rect(10.0, 10.0, 40.0, 40.0))),
        TracePartArea::Whole,
    ];
    // Whole is only for the first part, but a later whole-covering mask hides the first.
    let areas2 = [
        areas[0].clone(),
        TracePartArea::Mask(inside(rect(-10.0, -10.0, 110.0, 110.0))),
    ];
    let clips = trace_part_clips(&areas2, &quad());
    assert!(!clips[0].contributes(), "fully covered: {clips:?}");
    assert!(clips[1].contributes());
}

#[test]
fn a_mask_beyond_the_picture_does_not_hide_an_earlier_part() {
    // Part 2 lies wholly outside the picture: part 1 keeps the whole image (whether part 2 is
    // "nothing" is the session's call, `TracePartArea::Nothing`; its trace is empty either way).
    let pic = m1(200);
    let areas = [
        TracePartArea::Whole,
        TracePartArea::Mask(inside(rect(200.0, 200.0, 300.0, 300.0))),
    ];
    let clips = trace_part_clips(&areas, &quad());
    assert_eq!(clips[0], TracePartClip::Whole, "{clips:?}");
    let (join, _) = joined(
        &pic,
        &[
            (areas[0].clone(), mono(50, 50)),
            (areas[1].clone(), mono(50, 50)),
        ],
    );
    let whole = read(&job(&pic, &mono(50, 50), None));
    assert!(xor_ratio(&polygons(&join.outlines), &polygons(&whole.outlines)) <= 0.001);
}

// ---------------------------------------------------------------------------------------------
// Determinism, and a property: equal settings in every part give the whole-picture trace.

#[test]
fn the_join_is_deterministic() {
    let pic = colour_pic(200);
    let parts = [
        (TracePartArea::Whole, colour(40)),
        (
            TracePartArea::Mask(inside(rect(20.0, 20.0, 80.0, 80.0))),
            colour(55),
        ),
        (
            TracePartArea::Mask(outside(rect(0.0, 0.0, 60.0, 60.0))),
            colour(30),
        ),
    ];
    let (a, _) = joined(&pic, &parts);
    let (b, _) = joined(&pic, &parts);
    assert_eq!(a, b);
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 6, .. ProptestConfig::default() })]

    /// Two parts with the same settings split at any x: the join equals the unmasked trace
    /// within 0.1 % and no two of its shapes overlap.
    #[test]
    fn equal_settings_in_two_parts_equal_the_whole_trace(x in 12.0_f64..88.0, outside_mode in any::<bool>()) {
        let pic = m1(200);
        let r = rect(-1.0, -1.0, x, 101.0);
        let region = if outside_mode { outside(r) } else { inside(r) };
        let (join, _) = joined(&pic, &[
            (TracePartArea::Whole, mono(50, 50)),
            (TracePartArea::Mask(region), mono(50, 50)),
        ]);
        let whole = read(&job(&pic, &mono(50, 50), None));
        let got = polygons(&join.outlines);
        let r = xor_ratio(&got, &polygons(&whole.outlines));
        prop_assert!(r <= 0.001, "x = {x}: symmetric difference {r}");
        let shapes: Vec<Polys> = join.outlines.iter().map(|p| polygons(std::slice::from_ref(p))).collect();
        for i in 0..shapes.len() {
            for j in i + 1..shapes.len() {
                let o = area(&op(BooleanOp::Intersection, &shapes[i], &shapes[j]));
                prop_assert!(o <= 0.01, "shapes {i} and {j} overlap by {o} mm²");
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// White-box edges seen in the join: a failed part, and Single colour next to Multicolour.

#[test]
fn c26_a_part_that_did_not_trace_makes_the_join_say_so() {
    let pic = m1(200);
    let areas = [
        TracePartArea::Whole,
        TracePartArea::Mask(inside(rect(0.0, 0.0, 50.0, 100.0))),
    ];
    let clips = trace_part_clips(&areas, &quad());
    let good = read(&job(&pic, &mono(50, 50), clips[0].region()));
    let mut failed = read(&job(&pic, &mono(50, 50), clips[1].region()));
    for status in [Status::TooDetailed, Status::Invalid] {
        failed.status = status;
        let join = join_trace_parts(&[(&good, &clips[0]), (&failed, &clips[1])], 1);
        assert_eq!(join.status, status, "the join reports the part's failure");
        assert!(join.outlines.is_empty(), "and creates nothing");
    }
}

#[test]
fn single_colour_next_to_multicolour_parts_overlap_nowhere() {
    let pic = colour_pic(200);
    let (join, _) = joined(
        &pic,
        &[
            (TracePartArea::Whole, colour(40)),
            (
                TracePartArea::Mask(inside(rect(0.0, 0.0, 50.0, 100.0))),
                Job::Mono(TraceParams::with_threshold(200)),
            ),
        ],
    );
    assert_eq!(join.status, Status::Traced);
    let ls = layers(&join);
    assert!(!ls.is_empty(), "a multicolour result");
    let mut seen = Vec::new();
    for (c, _) in &ls {
        assert!(!seen.contains(c), "one layer per RGB: {c:?}");
        seen.push(*c);
    }
    let polys: Vec<Polys> = ls.iter().map(|(_, p)| polygons(p)).collect();
    for i in 0..polys.len() {
        for j in i + 1..polys.len() {
            let o = area(&op(BooleanOp::Intersection, &polys[i], &polys[j]));
            assert!(
                o <= 0.01,
                "{:?} and {:?} overlap by {o} mm²",
                ls[i].0,
                ls[j].0
            );
        }
    }
    assert_eq!(
        join.outlines.len(),
        ls.iter().map(|(_, p)| p.len()).sum::<usize>(),
        "every outline is in a layer"
    );
}
