//! Independent acceptance tests of `specs/0089-trace-quality-and-colour` slice M (multicolour,
//! criteria 26 to 30), written from the specification before the implementation was read.
//! Pictures are generated here; the real images are read from `tests/fixtures/trace/reference/`.
//! Colour distances are in the sRGB unit cube (bytes / 255, Euclidean). Coverage is measured by
//! rasterising the result at 4x with kurbo's winding number, independent of the tracer.
//!
//! The timing test is `#[ignore]` and measures thread CPU time in release:
//!
//! ```text
//! cargo test --release -p curvyo-vectorize-core --test acceptance_0089_sliceM_tester -- --ignored --nocapture
//! ```

#![allow(
    non_snake_case,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::similar_names,
    clippy::needless_pass_by_value,
    clippy::float_cmp,
    clippy::type_complexity
)]

mod common;

use std::ops::ControlFlow;

use curvyo_document_core::{Affine, Length};
use curvyo_vectorize_core::{
    ColourParams, ColourTrace, DEFAULT_BACKGROUND_TOLERANCE, PixelLength, Raster, Rgb8, Selection,
    Swatches, TraceParams, colour_mask, trace_colours,
};
use kurbo::{BezPath, ParamCurve, ParamCurveArclen, ParamCurveNearest, PathSeg, Point, Shape};

const MM: f64 = 0.1;

type Colour = [u8; 3];
const WHITE: Colour = [255, 255, 255];
const RED: Colour = [200, 30, 30];
const GREEN: Colour = [30, 160, 60];
const BLUE: Colour = [30, 60, 200];

struct Pic {
    w: u32,
    h: u32,
    rgba: Vec<u8>,
}

impl Pic {
    fn raster(&self) -> Raster<'_> {
        Raster {
            width: self.w,
            height: self.h,
            rgba: &self.rgba,
        }
    }
}

fn rgb(c: Colour) -> Rgb8 {
    Rgb8::new(c[0], c[1], c[2])
}

fn dist(a: Colour, b: Colour) -> f64 {
    let d = |i: usize| (f64::from(a[i]) - f64::from(b[i])).powi(2);
    (d(0) + d(1) + d(2)).sqrt() / 255.0
}

/// A scene gives the colour of a point (continuous pixel coordinates).
type Scene = Box<dyn Fn(f64, f64) -> Colour>;

/// Renders `scene` with 4 x 4 samples per pixel (anti-aliased borders) and a deterministic
/// +-`noise` per channel, a stand-in for JPEG quality 75 ringing.
fn render(w: u32, h: u32, noise: i32, scene: &Scene) -> Pic {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    let mut state = 0x2545_F491_u32;
    for y in 0..h {
        for x in 0..w {
            let mut sum = [0.0_f64; 3];
            for sy in 0..4 {
                for sx in 0..4 {
                    let c = scene(
                        f64::from(x) + (f64::from(sx) + 0.5) / 4.0,
                        f64::from(y) + (f64::from(sy) + 0.5) / 4.0,
                    );
                    for i in 0..3 {
                        sum[i] += f64::from(c[i]);
                    }
                }
            }
            for s in sum {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let n = if noise == 0 {
                    0
                } else {
                    ((state >> 16) as i32 % (2 * noise + 1)) - noise
                };
                rgba.push((s / 16.0 + f64::from(n)).round().clamp(0.0, 255.0) as u8);
            }
            rgba.push(255);
        }
    }
    Pic { w, h, rgba }
}

/// Three colours meeting at (80, 80) in a rectangle [20, 140]^2 on white.
fn rect_scene(left: Colour, top: Colour, bottom: Colour) -> Scene {
    Box::new(move |x, y| {
        if !(20.0..140.0).contains(&x) || !(20.0..140.0).contains(&y) {
            WHITE
        } else if x < 80.0 {
            left
        } else if y < 80.0 {
            top
        } else {
            bottom
        }
    })
}

/// The same split inside a disc of radius 60 around (80, 80).
fn disc_scene() -> Scene {
    Box::new(|x, y| {
        if (x - 80.0).hypot(y - 80.0) >= 60.0 {
            WHITE
        } else if x < 80.0 {
            RED
        } else if y < 80.0 {
            GREEN
        } else {
            BLUE
        }
    })
}

fn tp(selection: Selection) -> TraceParams {
    TraceParams {
        selection,
        specks: Length::from_mm(0.0),
        ..TraceParams::with_threshold(128)
    }
}

fn remove(bg: Colour) -> Selection {
    Selection::RemoveBackground {
        background: Some(rgb(bg)),
        tolerance: DEFAULT_BACKGROUND_TOLERANCE,
    }
}

fn params(bg: Colour) -> ColourParams {
    ColourParams::new(tp(remove(bg)))
}

fn run(p: &Pic, params: &ColourParams) -> ColourTrace {
    run_scaled(p, params, MM)
}

fn run_scaled(p: &Pic, params: &ColourParams, mm_per_px: f64) -> ColourTrace {
    trace_colours(
        p.raster(),
        Affine::scale(mm_per_px, mm_per_px),
        params,
        &mut |_| ControlFlow::Continue(()),
    )
    .unwrap()
}

fn layer_near(t: &ColourTrace, c: Colour, d: f64) -> Option<usize> {
    t.layers.iter().position(|l| {
        let s = l.sample;
        dist([s.r, s.g, s.b], c) < d
    })
}

fn covers(t: &ColourTrace, layer: usize, x_px: f64, y_px: f64) -> bool {
    let p = Point::new(x_px * MM, y_px * MM);
    t.layers[layer]
        .outlines
        .iter()
        .map(|o| o.winding(p))
        .sum::<i32>()
        != 0
}

fn segs(l: &curvyo_vectorize_core::ColourLayer) -> Vec<PathSeg> {
    l.outlines.iter().flat_map(BezPath::segments).collect()
}

fn pts(s: &PathSeg) -> Vec<Point> {
    match s {
        PathSeg::Line(l) => vec![l.p0, l.p1],
        PathSeg::Quad(q) => vec![q.p0, q.p1, q.p2],
        PathSeg::Cubic(c) => vec![c.p0, c.p1, c.p2, c.p3],
    }
}

fn same(a: &PathSeg, b: &PathSeg, tol: f64) -> bool {
    let (pa, pb) = (pts(a), pts(b));
    pa.len() == pb.len() && pa.iter().zip(&pb).all(|(p, q)| p.distance(*q) <= tol)
}

fn nearest(s: &PathSeg, p: Point) -> f64 {
    s.nearest(p, 1e-4).distance_sq.sqrt()
}

/// Shared segments of two layers: panics when a segment of `a` lies on `b` (its midpoint within
/// 0.02 mm of a `b` segment) without `b` holding the same segment reversed within 1e-6 mm.
/// Returns the number of shared segments and their length in mm.
fn shared(t: &ColourTrace, a: usize, b: usize) -> (usize, f64) {
    let sb = segs(&t.layers[b]);
    let (mut n, mut len) = (0, 0.0);
    for s in segs(&t.layers[a]) {
        let mid = s.eval(0.5);
        if sb.iter().any(|o| nearest(o, mid) < 0.02) {
            assert!(
                sb.iter().any(|o| same(&s.reverse(), o, 1e-6)),
                "layer {a} has a segment on the border with layer {b} that is not the same \
                 segment reversed: {s:?}"
            );
            n += 1;
            len += s.arclen(1e-4);
        }
    }
    (n, len)
}

fn nodes(t: &ColourTrace, layer: usize) -> Vec<Point> {
    segs(&t.layers[layer]).iter().map(|s| pts(s)[0]).collect()
}

fn nearest_node(t: &ColourTrace, layer: usize, to: Point) -> Point {
    nodes(t, layer)
        .into_iter()
        .min_by(|a, b| a.distance(to).total_cmp(&b.distance(to)))
        .unwrap()
}

/// Per sample of a 4x grid: how many layers cover it.
fn cover_counts(t: &ColourTrace, w: u32, h: u32) -> Vec<u8> {
    let (w4, h4) = (w * 4, h * 4);
    let mut out = vec![0_u8; (w4 * h4) as usize];
    for l in &t.layers {
        for y in 0..h4 {
            for x in 0..w4 {
                let p = Point::new(
                    (f64::from(x) + 0.5) / 4.0 * MM,
                    (f64::from(y) + 0.5) / 4.0 * MM,
                );
                if l.outlines.iter().map(|o| o.winding(p)).sum::<i32>() != 0 {
                    out[(y * w4 + x) as usize] += 1;
                }
            }
        }
    }
    out
}

/// Gaps and overlaps at 4x: `deep(x, y)` says a sample (pixel coordinates) is well inside the
/// drawn content, `far(x, y)` well outside it. Returns (uncovered, doubly covered, stray,
/// samples).
fn gaps_overlaps(
    t: &ColourTrace,
    w: u32,
    h: u32,
    deep: impl Fn(f64, f64) -> bool,
    far: impl Fn(f64, f64) -> bool,
) -> (usize, usize, usize, usize) {
    let counts = cover_counts(t, w, h);
    let w4 = w * 4;
    let (mut gap, mut over, mut stray) = (0, 0, 0);
    for (i, &c) in counts.iter().enumerate() {
        let x = (f64::from(i as u32 % w4) + 0.5) / 4.0;
        let y = (f64::from(i as u32 / w4) + 0.5) / 4.0;
        if c >= 2 {
            over += 1;
        }
        if c == 0 && deep(x, y) {
            gap += 1;
        }
        if c > 0 && far(x, y) {
            stray += 1;
        }
    }
    (gap, over, stray, counts.len())
}

fn limit(n: usize) -> usize {
    // 0.01 % of the samples.
    n / 10_000
}

// ---------------------------------------------------------------------------------------------
// Criterion 26: palette

#[test]
fn c26_logo_with_anti_aliasing_and_noise_has_exactly_three_colours() {
    let p = render(160, 160, 3, &rect_scene(RED, GREEN, BLUE));
    let t = run(&p, &params(WHITE));
    assert_eq!(
        t.layers.len(),
        3,
        "layers {:?}",
        t.layers.iter().map(|l| l.sample).collect::<Vec<_>>()
    );
    assert_eq!(t.palette.len(), 3, "palette {:?}", t.palette);
    for c in [RED, GREEN, BLUE] {
        let i = layer_near(&t, c, 0.03).unwrap_or_else(|| panic!("no layer near {c:?}"));
        assert!(t.layers[i].pixels > 0);
    }
}

#[test]
fn c26_pale_accent_of_0_2_percent_is_kept() {
    let pale: Colour = [255, 242, 168];
    // 200 x 200 px, the accent is 9 x 9 = 81 px = 0.2 %.
    let scene: Scene = Box::new(move |x, y| {
        if (10.0..19.0).contains(&x) && (10.0..19.0).contains(&y) {
            pale
        } else if (40.0..190.0).contains(&x) && (40.0..110.0).contains(&y) {
            [20, 40, 120]
        } else if (40.0..190.0).contains(&x) && (110.0..190.0).contains(&y) {
            RED
        } else {
            WHITE
        }
    });
    let p = render(200, 200, 0, &scene);
    // Ignore below off: the accent is 0.9 mm wide.
    for colours in [None, Some(4)] {
        let mut cp = params(WHITE);
        cp.colours = colours;
        let t = run(&p, &cp);
        let i = layer_near(&t, pale, 0.03).unwrap_or_else(|| {
            panic!(
                "pale accent lost with colours {colours:?}: {:?}",
                t.layers.iter().map(|l| l.sample).collect::<Vec<_>>()
            )
        });
        assert!(covers(&t, i, 14.5, 14.5));
        assert!(!covers(&t, i, 100.0, 70.0));
    }
}

#[test]
fn c26_pale_accent_survives_the_default_ignore_below_when_it_is_large_enough() {
    let pale: Colour = [255, 242, 168];
    // 400 x 400 px at 0.1 mm per px, accent 18 x 18 px = 0.2 %.
    let scene: Scene = Box::new(move |x, y| {
        if (10.0..28.0).contains(&x) && (10.0..28.0).contains(&y) {
            pale
        } else if (60.0..380.0).contains(&x) && (60.0..380.0).contains(&y) {
            [20, 40, 120]
        } else {
            WHITE
        }
    });
    let p = render(400, 400, 0, &scene);
    let mut cp = params(WHITE);
    cp.trace.specks = TraceParams::with_threshold(128).specks;
    let t = run(&p, &cp);
    assert!(
        layer_near(&t, pale, 0.03).is_some(),
        "{:?}",
        t.layers.iter().map(|l| l.sample).collect::<Vec<_>>()
    );
}

#[test]
fn c26_near_greys_043_apart_stay_apart_and_closer_than_003_merge() {
    // #D9D9D9 and #DCD6CF are 0.043 apart (the spec's own example), on black.
    let (g1, g2): (Colour, Colour) = ([0xD9; 3], [0xDC, 0xD6, 0xCF]);
    assert!(dist(g1, g2) > 0.03);
    let halves = |a: Colour, b: Colour, out: Colour| -> Scene {
        Box::new(move |x, y| {
            if !(20.0..140.0).contains(&x) || !(20.0..140.0).contains(&y) {
                out
            } else if x < 80.0 {
                a
            } else {
                b
            }
        })
    };
    let apart = run(
        &render(160, 160, 0, &halves(g1, g2, [0, 0, 0])),
        &params([0, 0, 0]),
    );
    assert_eq!(
        apart.layers.len(),
        2,
        "{:?}",
        apart.layers.iter().map(|l| l.sample).collect::<Vec<_>>()
    );

    // 0.014 apart: one colour. 0.044 apart (orange-ish): two.
    let (c1, c2, c3): (Colour, Colour, Colour) = ([128, 80, 48], [131, 82, 48], [136, 88, 48]);
    assert!(dist(c1, c2) < 0.03 && dist(c1, c3) > 0.03);
    let close = run(&render(160, 160, 0, &halves(c1, c2, WHITE)), &params(WHITE));
    assert_eq!(
        close.layers.len(),
        1,
        "{:?}",
        close.layers.iter().map(|l| l.sample).collect::<Vec<_>>()
    );
    let far = run(&render(160, 160, 0, &halves(c1, c3, WHITE)), &params(WHITE));
    assert_eq!(
        far.layers.len(),
        2,
        "{:?}",
        far.layers.iter().map(|l| l.sample).collect::<Vec<_>>()
    );
}

/// 320 x 140 at the app's 96 dpi scale: three adjacent colours, two near-greys 0.043 apart, a
/// pale accent of 0.32 % of the image, on white.
fn six_colour_picture() -> (Pic, [Colour; 6]) {
    let (grey_a, grey_b, pale): (Colour, Colour, Colour) =
        ([0xD9; 3], [0xDC, 0xD6, 0xCF], [255, 242, 168]);
    let scene: Scene = Box::new(move |x, y| {
        let band = (40.0..130.0).contains(&y);
        if (10.0..22.0).contains(&x) && (10.0..22.0).contains(&y) {
            pale
        } else if band && (20.0..100.0).contains(&x) {
            RED
        } else if band && (100.0..160.0).contains(&x) {
            if y < 85.0 { GREEN } else { BLUE }
        } else if band && (190.0..240.0).contains(&x) {
            grey_a
        } else if band && (240.0..290.0).contains(&x) {
            grey_b
        } else {
            WHITE
        }
    });
    (
        render(320, 140, 0, &scene),
        [RED, GREEN, BLUE, grey_a, grey_b, pale],
    )
}

fn six_colours_found(colours: Option<u8>) -> (usize, Vec<Rgb8>, Vec<Colour>) {
    let (p, want) = six_colour_picture();
    let mut cp = params(WHITE);
    cp.trace.specks = TraceParams::with_threshold(128).specks;
    cp.colours = colours;
    let t = run_scaled(&p, &cp, 25.4 / 96.0);
    let missing = want
        .iter()
        .copied()
        .filter(|&c| layer_near(&t, c, 0.03).is_none())
        .collect();
    (
        t.layers.len(),
        t.palette.iter().map(|e| e.colour).collect(),
        missing,
    )
}

#[test]
fn c26_with_six_colours_asked_the_pale_accent_and_both_near_greys_are_kept() {
    let (layers, palette, missing) = six_colours_found(Some(6));
    assert_eq!(
        (layers, missing.len()),
        (6, 0),
        "{palette:?} missing {missing:?}"
    );
}

#[test]
fn c26_with_the_automatic_count_the_pale_accent_and_both_near_greys_are_kept() {
    let (layers, palette, missing) = six_colours_found(None);
    assert_eq!(
        (layers, missing.len()),
        (6, 0),
        "{palette:?} missing {missing:?}"
    );
}

#[test]
fn c26_count_two_to_sixteen_and_fixed_colours_are_kept_exactly() {
    // Eight flat vertical bands of well separated colours.
    let bands: [Colour; 8] = [
        [200, 30, 30],
        [30, 160, 60],
        [30, 60, 200],
        [230, 200, 20],
        [120, 30, 150],
        [20, 20, 20],
        [30, 190, 190],
        [150, 90, 40],
    ];
    let scene: Scene = Box::new(move |x, y| {
        if !(0.0..160.0).contains(&x) || !(10.0..150.0).contains(&y) {
            WHITE
        } else {
            bands[(x / 20.0) as usize]
        }
    });
    let p = render(160, 160, 0, &scene);
    let mut cp = params(WHITE);
    cp.colours = Some(3);
    let t = run(&p, &cp);
    assert_eq!(t.palette.len(), 3, "{:?}", t.palette);
    cp.colours = Some(16);
    let t = run(&p, &cp);
    assert_eq!(
        t.layers.len(),
        8,
        "16 allowed, 8 exist, none merged: {:?}",
        t.palette
    );
    assert!(t.palette.len() <= 16);

    // A fixed colour that exists nowhere in the image is kept exactly and sorts first.
    let mut cp = params(WHITE);
    cp.colours = Some(5);
    cp.fixed = vec![Rgb8::new(1, 2, 3)];
    let t = run(&p, &cp);
    assert!(t.palette.len() <= 5);
    assert_eq!(t.palette[0].colour, Rgb8::new(1, 2, 3));
    assert!(t.palette[0].fixed);

    // A fixed colour near an existing band (0.0 on the band itself) replaces the sample exactly.
    let mut cp = params(WHITE);
    cp.fixed = vec![Rgb8::new(201, 31, 29)];
    let t = run(&p, &cp);
    assert!(
        t.layers
            .iter()
            .any(|l| l.sample == Rgb8::new(201, 31, 29) && l.fixed)
    );
}

#[test]
fn c26_pick_colours_palette_is_exactly_the_picked_colours() {
    let p = render(160, 160, 0, &rect_scene(RED, GREEN, BLUE));
    let mut cp = params(WHITE);
    cp.trace.selection = Selection::PickColours {
        swatches: Swatches::from_slice(&[rgb(RED), rgb(BLUE)]),
        background: Some(rgb(WHITE)),
        tolerance: curvyo_vectorize_core::DEFAULT_PICK_TOLERANCE,
    };
    cp.colours = Some(9);
    let t = run(&p, &cp);
    let samples: Vec<Rgb8> = t.palette.iter().map(|e| e.colour).collect();
    assert_eq!(samples.len(), 2, "{samples:?}");
    assert!(
        samples.contains(&rgb(RED)) && samples.contains(&rgb(BLUE)),
        "{samples:?}"
    );
    assert!(t.layers.len() <= 2);
    // The green region is background: nothing covers it.
    assert!(t.layers.iter().all(|l| l.sample != rgb(GREEN)));
    let counts = cover_counts(&t, 160, 160);
    let w4 = 640;
    assert_eq!(counts[(50 * 4 * w4 + 110 * 4) as usize], 0);
}

#[test]
fn c26_thin_anti_aliased_line_art_keeps_a_palette_for_its_content() {
    // Black rings 1.3 px wide on white: nearly every dark pixel is an anti-aliasing grey.
    let scene: Scene = Box::new(|x, y| {
        let r = (x - 70.0).hypot(y - 70.0);
        if (r - 50.0).abs() < 0.65 || (r - 30.0).abs() < 0.65 {
            [0, 0, 0]
        } else {
            WHITE
        }
    });
    let p = render(140, 140, 0, &scene);
    let mut cp = params(WHITE);
    for colours in [None, Some(2), Some(6)] {
        cp.colours = colours;
        let t = run(&p, &cp);
        assert!(t.foreground_pixels > 0);
        assert!(
            !t.palette.is_empty() && !t.layers.is_empty(),
            "colours {colours:?}: {} content pixels but palette {:?} and {} layers",
            t.foreground_pixels,
            t.palette,
            t.layers.len()
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Criterion 27: one region per pixel, joined regions

#[test]
fn c27_small_region_joins_the_neighbour_with_the_longest_shared_border() {
    // Red left, green right (x 80..140); a 6 x 6 px blue square across the border, 5 px of it
    // in red and 1 px in green: red has the longer shared border. Ignore below 1 mm = 10 px.
    let scene: Scene = Box::new(|x, y| {
        if !(20.0..140.0).contains(&x) || !(20.0..140.0).contains(&y) {
            WHITE
        } else if (75.0..81.0).contains(&x) && (70.0..76.0).contains(&y) {
            BLUE
        } else if x < 80.0 {
            RED
        } else {
            GREEN
        }
    });
    let p = render(160, 160, 0, &scene);
    let mut cp = params(WHITE);
    cp.trace.specks = Length::from_mm(1.0);
    let t = run(&p, &cp);
    assert_eq!(
        t.layers.len(),
        2,
        "{:?}",
        t.layers.iter().map(|l| l.sample).collect::<Vec<_>>()
    );
    assert!(
        t.diagnostics.joined_regions >= 1,
        "joined {}",
        t.diagnostics.joined_regions
    );
    let red = layer_near(&t, RED, 0.03).unwrap();
    let green = layer_near(&t, GREEN, 0.03).unwrap();
    assert!(covers(&t, red, 77.0, 73.0), "the square joined red");
    assert!(!covers(&t, green, 77.0, 73.0));
}

#[test]
fn c27_without_small_regions_nothing_is_reported_joined() {
    let p = render(160, 160, 0, &rect_scene(RED, GREEN, BLUE));
    let t = run(&p, &params(WHITE));
    assert_eq!(t.diagnostics.joined_regions, 0);
}

// ---------------------------------------------------------------------------------------------
// Criterion 28: shared boundaries, triple junctions, no gaps, no overlaps

fn heavy(mut cp: ColourParams) -> ColourParams {
    cp.trace.smooth = 100;
    cp.trace.tolerance = PixelLength::from_px(2.0);
    cp
}

fn check_shared_and_junction(p: &Pic, cp: &ColourParams) {
    let t = run(p, cp);
    assert_eq!(t.layers.len(), 3);
    let (r, g, b) = (
        layer_near(&t, RED, 0.03).unwrap(),
        layer_near(&t, GREEN, 0.03).unwrap(),
        layer_near(&t, BLUE, 0.03).unwrap(),
    );
    for (a, o) in [(r, g), (r, b), (g, b)] {
        let (n, len) = shared(&t, a, o);
        let (n2, len2) = shared(&t, o, a);
        assert!(
            n > 0 && n2 > 0,
            "no shared segments between layers {a} and {o}"
        );
        assert_eq!(n, n2);
        assert!((len - len2).abs() < 1e-6);
        // Each pair shares a 60 px = 6 mm border (within a pixel at each end).
        assert!(
            len > 5.5 && len < 6.8,
            "shared length {len} mm of layers {a}, {o}"
        );
    }
    let junction = Point::new(8.0, 8.0);
    let nr = nearest_node(&t, r, junction);
    let ng = nearest_node(&t, g, junction);
    let nb = nearest_node(&t, b, junction);
    assert!(nr.distance(junction) < 0.15, "red junction node at {nr:?}");
    assert!(
        nr.distance(ng) < 1e-6 && nr.distance(nb) < 1e-6,
        "{nr:?} {ng:?} {nb:?}"
    );
}

#[test]
fn c28_neighbours_share_segments_and_the_triple_junction_is_one_node() {
    check_shared_and_junction(
        &render(160, 160, 0, &rect_scene(RED, GREEN, BLUE)),
        &params(WHITE),
    );
}

#[test]
fn c28_also_with_noise_and_after_heavy_simplification() {
    let p = render(160, 160, 3, &rect_scene(RED, GREEN, BLUE));
    check_shared_and_junction(&p, &params(WHITE));
    check_shared_and_junction(&p, &heavy(params(WHITE)));
}

#[test]
fn c28_curved_outline_shares_segments_too() {
    for cp in [params(WHITE), heavy(params(WHITE))] {
        let t = run(&render(160, 160, 2, &disc_scene()), &cp);
        assert_eq!(t.layers.len(), 3);
        let r = layer_near(&t, RED, 0.03).unwrap();
        let g = layer_near(&t, GREEN, 0.03).unwrap();
        let b = layer_near(&t, BLUE, 0.03).unwrap();
        for (a, o) in [(r, g), (r, b), (g, b)] {
            let (n, len) = shared(&t, a, o);
            assert!(n > 0 && len > 5.0, "pair {a} {o}: {n} segments, {len} mm");
            shared(&t, o, a);
        }
        let junction = Point::new(8.0, 8.0);
        let nr = nearest_node(&t, r, junction);
        assert!(nr.distance(nearest_node(&t, g, junction)) < 1e-6);
        assert!(nr.distance(nearest_node(&t, b, junction)) < 1e-6);
    }
}

fn rect_deep(x: f64, y: f64) -> bool {
    (21.5..138.5).contains(&x) && (21.5..138.5).contains(&y)
}
fn rect_far(x: f64, y: f64) -> bool {
    !(18.5..141.5).contains(&x) || !(18.5..141.5).contains(&y)
}
fn disc_deep(x: f64, y: f64) -> bool {
    (x - 80.0).hypot(y - 80.0) < 58.5
}
fn disc_far(x: f64, y: f64) -> bool {
    (x - 80.0).hypot(y - 80.0) > 61.5
}

#[test]
fn c28_no_gap_and_no_overlap_at_4x_rectangles() {
    for (noise, cp) in [
        (0, params(WHITE)),
        (3, params(WHITE)),
        (3, heavy(params(WHITE))),
    ] {
        let t = run(&render(160, 160, noise, &rect_scene(RED, GREEN, BLUE)), &cp);
        let (gap, over, stray, n) = gaps_overlaps(&t, 160, 160, rect_deep, rect_far);
        assert!(
            gap <= limit(n),
            "uncovered samples {gap} of {n} (noise {noise})"
        );
        assert!(
            over <= limit(n),
            "doubly covered samples {over} of {n} (noise {noise})"
        );
        assert!(
            stray <= limit(n) + 40,
            "stray samples {stray} outside the content"
        );
    }
}

#[test]
fn c28_no_gap_and_no_overlap_at_4x_discs() {
    for cp in [params(WHITE), heavy(params(WHITE))] {
        let t = run(&render(160, 160, 2, &disc_scene()), &cp);
        let (gap, over, stray, n) = gaps_overlaps(&t, 160, 160, disc_deep, disc_far);
        assert!(gap <= limit(n), "uncovered samples {gap} of {n}");
        assert!(over <= limit(n), "doubly covered samples {over} of {n}");
        assert!(stray <= limit(n) + 40, "stray samples {stray}");
    }
}

#[test]
fn c28_holes_and_islands_have_no_gap_or_overlap() {
    // A red ring around a green disc around a blue dot, with a white hole in the ring's edge:
    // nested labels, holes wound against their outlines.
    let scene: Scene = Box::new(|x, y| {
        let r = (x - 80.0).hypot(y - 80.0);
        if r < 12.0 {
            BLUE
        } else if r < 35.0 {
            GREEN
        } else if r < 60.0 && (x - 125.0).hypot(y - 80.0) >= 8.0 {
            RED
        } else {
            WHITE
        }
    });
    let t = run(&render(160, 160, 2, &scene), &params(WHITE));
    assert_eq!(t.layers.len(), 3);
    let deep = |x: f64, y: f64| {
        let r = (x - 80.0).hypot(y - 80.0);
        r < 58.5 && (x - 125.0).hypot(y - 80.0) > 9.5
    };
    let far =
        |x: f64, y: f64| (x - 80.0).hypot(y - 80.0) > 61.5 || (x - 125.0).hypot(y - 80.0) < 6.5;
    let (gap, over, stray, n) = gaps_overlaps(&t, 160, 160, deep, far);
    assert!(gap <= limit(n), "uncovered {gap} of {n}");
    assert!(over <= limit(n), "doubly covered {over} of {n}");
    assert!(stray <= limit(n) + 40, "stray {stray}");
}

fn real_case(name: &str) {
    {
        let path = common::quality::real::reference_dir().join(name);
        let bytes = std::fs::read(&path).unwrap();
        let img = curvyo_image_core::decode_working(&bytes).unwrap();
        let k = (img.width.max(img.height)).div_ceil(160);
        let (w, h) = (img.width / k, img.height / k);
        let mut rgba = Vec::new();
        for y in 0..h {
            for x in 0..w {
                // Box filter over k x k source pixels.
                let mut s = [0_u32; 4];
                for dy in 0..k {
                    for dx in 0..k {
                        let i = (((y * k + dy) * img.width + x * k + dx) * 4) as usize;
                        for (acc, v) in s.iter_mut().zip(&img.pixels[i..i + 4]) {
                            *acc += u32::from(*v);
                        }
                    }
                }
                for v in s {
                    rgba.push((v / (k * k)) as u8);
                }
            }
        }
        let pic = Pic { w, h, rgba };
        let bg = curvyo_vectorize_core::suggest_background(pic.raster());
        let mut cp = ColourParams::new(tp(Selection::RemoveBackground {
            background: bg,
            tolerance: DEFAULT_BACKGROUND_TOLERANCE,
        }));
        cp.colours = Some(5);
        let t = run(&pic, &cp);
        assert!(!t.layers.is_empty(), "{name}: nothing traced");
        for a in 0..t.layers.len() {
            for b in 0..t.layers.len() {
                if a != b {
                    shared(&t, a, b);
                }
            }
        }
        let counts = cover_counts(&t, w, h);
        let over = counts.iter().filter(|&&c| c >= 2).count();
        assert!(
            over <= limit(counts.len()),
            "{name}: {over} doubly covered of {}",
            counts.len()
        );
    }
}

#[test]
fn c28_real_logos_shared_boundaries_are_exact_and_nothing_is_covered_twice() {
    for name in ["logo-ymc.png", "logo-wrt.png", "clipart-goatee.jpg"] {
        real_case(name);
    }
}

#[test]
fn c28_real_clock_clipart_traces_in_colours() {
    real_case("clipart-clock.png");
}

// ---------------------------------------------------------------------------------------------
// Criterion 29 (core part): order, edits

#[test]
fn c29_layers_largest_first_ties_by_colour_value_independent_of_position() {
    // Red 60 x 120 is the largest; green and blue are 60 x 60 each (a tie).
    let a = run(
        &render(160, 160, 0, &rect_scene(RED, GREEN, BLUE)),
        &params(WHITE),
    );
    let b = run(
        &render(160, 160, 0, &rect_scene(RED, BLUE, GREEN)),
        &params(WHITE),
    );
    let order = |t: &ColourTrace| t.layers.iter().map(|l| l.sample).collect::<Vec<_>>();
    assert_eq!(order(&a)[0], rgb(RED));
    assert_eq!(
        order(&a),
        order(&b),
        "a tie is broken by colour value, not by position"
    );
    assert!(a.layers.windows(2).all(|w| w[0].pixels >= w[1].pixels));
    // Ascending by (r, g, b): green (30,160,60) before blue (30,60,200)? 160 > 60, so blue first.
    let tie: Vec<Rgb8> = order(&a)[1..].to_vec();
    let mut sorted = tie.clone();
    sorted.sort_by_key(|c| (c.r, c.g, c.b));
    assert_eq!(tie, sorted, "ties ascend by colour value");
}

#[test]
fn c29_exclude_makes_the_colour_background_and_merge_into_reassigns_it() {
    let p = render(160, 160, 0, &rect_scene(RED, GREEN, BLUE));
    let mut cp = params(WHITE);
    cp.excluded = vec![rgb(GREEN)];
    let t = run(&p, &cp);
    assert_eq!(t.layers.len(), 2);
    assert!(layer_near(&t, GREEN, 0.03).is_none());
    let counts = cover_counts(&t, 160, 160);
    assert_eq!(
        counts[(50 * 4 * 640 + 110 * 4) as usize],
        0,
        "the green region is background"
    );
    assert_eq!(counts[(110 * 4 * 640 + 110 * 4) as usize], 1, "blue stays");

    let mut cp = params(WHITE);
    cp.merged = vec![(rgb(GREEN), rgb(BLUE))];
    let t = run(&p, &cp);
    assert_eq!(t.layers.len(), 2);
    assert!(layer_near(&t, GREEN, 0.03).is_none());
    let blue = layer_near(&t, BLUE, 0.03).unwrap();
    assert!(covers(&t, blue, 110.0, 50.0) && covers(&t, blue, 110.0, 110.0));
    // The merged colour's region now has no border to the other one: blue is one piece.
    assert_eq!(t.layers[blue].outlines.len(), 1);
}

#[test]
fn c29_the_trace_is_deterministic_and_stable_under_unrelated_edits() {
    let p = render(160, 160, 3, &rect_scene(RED, GREEN, BLUE));
    let a = run(&p, &params(WHITE));
    assert_eq!(a, run(&p, &params(WHITE)));
    // Excluding a colour that does not exist changes nothing.
    let mut cp = params(WHITE);
    cp.excluded = vec![Rgb8::new(250, 0, 250)];
    assert_eq!(a.layers, run(&p, &cp).layers);
}

#[test]
fn c29_mask_view_is_the_segmented_palette_not_a_black_union() {
    let p = render(160, 160, 0, &rect_scene(RED, GREEN, BLUE));
    let (mask, palette) = colour_mask(
        p.raster(),
        Affine::scale(MM, MM),
        &params(WHITE),
        &mut |_| ControlFlow::Continue(()),
    )
    .unwrap();
    assert_eq!(palette.len(), 3);
    let at = |x: usize, y: usize| {
        let i = (y * mask.width as usize + x) * 4;
        [
            mask.rgba[i],
            mask.rgba[i + 1],
            mask.rgba[i + 2],
            mask.rgba[i + 3],
        ]
    };
    let scale = mask.width as usize / 160;
    for (x, y, want) in [(50, 80, RED), (110, 50, GREEN), (110, 110, BLUE)] {
        let px = at(x * scale, y * scale);
        assert_eq!(px[3], 255);
        assert!(
            dist([px[0], px[1], px[2]], want) < 0.03,
            "{px:?} vs {want:?}"
        );
        assert!(
            palette
                .iter()
                .any(|e| [e.colour.r, e.colour.g, e.colour.b] == [px[0], px[1], px[2]])
        );
    }
    assert_eq!(
        at(5 * scale, 5 * scale)[3],
        0,
        "the background is transparent"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 30: time

#[cfg(unix)]
fn cpu() -> f64 {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
    t.tv_sec as f64 + t.tv_nsec as f64 * 1e-9
}

#[cfg(unix)]
#[test]
#[ignore = "timing, release only; see the module comment"]
fn c30_eight_colours_on_2000_by_2000_take_under_four_seconds_of_thread_cpu() {
    let cols: [Colour; 8] = [
        [200, 30, 30],
        [30, 160, 60],
        [30, 60, 200],
        [230, 200, 20],
        [120, 30, 150],
        [20, 20, 20],
        [30, 190, 190],
        [150, 90, 40],
    ];
    // Eight regions: 4 x 2 blocks, each with a disc of the next colour inside, on white edges.
    let scene: Scene = Box::new(move |x, y| {
        if !(100.0..1900.0).contains(&x) || !(100.0..1900.0).contains(&y) {
            return WHITE;
        }
        let (bx, by) = (
            ((x - 100.0) / 450.0) as usize,
            ((y - 100.0) / 900.0) as usize,
        );
        let i = by * 4 + bx.min(3);
        let (cx, cy) = (
            100.0 + 450.0 * (bx.min(3) as f64 + 0.5),
            100.0 + 900.0 * (by as f64 + 0.5),
        );
        if (x - cx).hypot(y - cy) < 150.0 {
            cols[(i + 1) % 8]
        } else {
            cols[i]
        }
    });
    let p = render(2000, 2000, 2, &scene);
    let mut cp = params(WHITE);
    cp.colours = Some(8);
    let t0 = cpu();
    let t = run(&p, &cp);
    let secs = cpu() - t0;
    println!(
        "8 colours, 2000 x 2000: {secs:.2} s thread CPU, {} layers",
        t.layers.len()
    );
    assert_eq!(t.layers.len(), 8);
    assert!(secs < 4.0, "{secs:.2} s thread CPU");
}

#[cfg(unix)]
#[test]
#[ignore = "timing, release only; see the module comment"]
fn c30_noisy_eight_colours_on_2000_by_2000_take_under_four_seconds_of_thread_cpu() {
    // Photo-like grain (+-14 per channel) on stripes of eight colours: many tiny regions.
    let cols: [Colour; 8] = [
        [200, 30, 30],
        [30, 160, 60],
        [30, 60, 200],
        [230, 200, 20],
        [120, 30, 150],
        [20, 20, 20],
        [30, 190, 190],
        [150, 90, 40],
    ];
    let scene: Scene = Box::new(move |x, y| {
        if !(50.0..1950.0).contains(&x) || !(50.0..1950.0).contains(&y) {
            WHITE
        } else {
            cols[(((x + y * 0.3) / 240.0) as usize) % 8]
        }
    });
    let p = render(2000, 2000, 14, &scene);
    let mut cp = params(WHITE);
    cp.colours = Some(8);
    let t0 = cpu();
    let t = run(&p, &cp);
    let secs = cpu() - t0;
    println!(
        "noisy 8 colours, 2000 x 2000: {secs:.2} s thread CPU, {} layers",
        t.layers.len()
    );
    assert!(secs < 4.0, "{secs:.2} s thread CPU");
}
