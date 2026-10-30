//! Independent acceptance tests of `specs/0089-trace-quality-and-colour` slice C (selection
//! methods, criteria 14 to 20), written from the specification. Pictures are generated here;
//! nothing is read from `tests/fixtures/`. Colour distances are in the sRGB unit cube.
//!
//! The timing test is `#[ignore]` and measures thread CPU time in release:
//!
//! ```text
//! cargo test --release -p curvyo-vectorize-core --test acceptance_0089_sliceC_tester -- --ignored --nocapture
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
    clippy::needless_pass_by_value
)]

use std::ops::ControlFlow;

use curvyo_document_core::Affine;
use curvyo_vectorize_core::{
    DEFAULT_PICK_TOLERANCE, MAX_SWATCHES, METHOD_PARAMS, ParamKind, Raster, Rgb8, Selection,
    Swatches, TraceParams, TraceResult, selected, suggest_background, trace,
};
use kurbo::{BezPath, PathEl, Point, Shape};

const MM_PER_PX: f64 = 25.4 / 96.0;

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

fn px_map() -> Affine {
    Affine::new(MM_PER_PX, 0.0, 0.0, MM_PER_PX, 0.0, 0.0)
}

/// A picture whose pixel (x, y) is `f(x, y)` as RGBA.
fn pic(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Pic {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            rgba.extend_from_slice(&f(x, y));
        }
    }
    Pic { w, h, rgba }
}

fn mix(bg: [u8; 3], fg: [u8; 3], t: f64) -> [u8; 4] {
    let c = |i: usize| (f64::from(bg[i]) * (1.0 - t) + f64::from(fg[i]) * t).round() as u8;
    [c(0), c(1), c(2), 255]
}

/// Coverage of the unit pixel (x, y) by `inside`, 8 x 8 samples.
fn coverage(x: u32, y: u32, inside: &impl Fn(f64, f64) -> bool) -> f64 {
    let mut hits = 0;
    for sy in 0..8 {
        for sx in 0..8 {
            if inside(
                f64::from(x) + (f64::from(sx) + 0.5) / 8.0,
                f64::from(y) + (f64::from(sy) + 0.5) / 8.0,
            ) {
                hits += 1;
            }
        }
    }
    f64::from(hits) / 64.0
}

fn disc(cx: f64, cy: f64, r: f64) -> impl Fn(f64, f64) -> bool {
    move |x, y| (x - cx).hypot(y - cy) < r
}

fn trace_with(p: &Pic, params: &TraceParams) -> TraceResult {
    trace(p.raster(), px_map(), params, &mut |_| {
        ControlFlow::Continue(())
    })
    .unwrap()
}

fn base(selection: Selection) -> TraceParams {
    TraceParams {
        selection,
        // Nothing but the test shapes is removed as a speck.
        specks: curvyo_document_core::Length::from_mm(0.0),
        ..TraceParams::with_threshold(128)
    }
}

/// Pick colours on a white ground.
fn pick(colours: &[[u8; 3]], tolerance: u8) -> Selection {
    pick_on(colours, WHITE, tolerance)
}

/// Pick colours on the ground `bg`, which the host supplies (the blends run toward it).
fn pick_on(colours: &[[u8; 3]], bg: [u8; 3], tolerance: u8) -> Selection {
    let list: Vec<Rgb8> = colours
        .iter()
        .map(|c| Rgb8::new(c[0], c[1], c[2]))
        .collect();
    Selection::PickColours {
        swatches: Swatches::from_slice(&list),
        background: Some(Rgb8::new(bg[0], bg[1], bg[2])),
        tolerance,
    }
}

fn flat_px(path: &BezPath) -> Vec<Point> {
    let mut pts = Vec::new();
    kurbo::flatten(path.elements().iter().copied(), 0.01, |el| match el {
        PathEl::MoveTo(p) | PathEl::LineTo(p) => {
            pts.push(Point::new(p.x / MM_PER_PX, p.y / MM_PER_PX));
        }
        _ => {}
    });
    pts
}

/// Filled area of the result in square pixels (holes wind against their outline).
fn area_px(r: &TraceResult) -> f64 {
    r.outlines.iter().map(Shape::area).sum::<f64>().abs() / (MM_PER_PX * MM_PER_PX)
}

/// Largest distance of any outline point from the circle (cx, cy, r), in pixels.
fn max_radial_error(r: &TraceResult, cx: f64, cy: f64, radius: f64) -> f64 {
    r.outlines
        .iter()
        .flat_map(flat_px)
        .map(|p| ((p.x - cx).hypot(p.y - cy) - radius).abs())
        .fold(0.0, f64::max)
}

const BLUE: [u8; 3] = [0x33, 0x66, 0xCC];
const PALE: [u8; 3] = [0xFF, 0xF2, 0xA8];
const WHITE: [u8; 3] = [255, 255, 255];

// ---------------------------------------------------------------- criterion 14

#[test]
fn c14_method_is_a_three_way_choice_in_the_schema() {
    let method = METHOD_PARAMS.iter().find(|p| p.name == "method").unwrap();
    assert_eq!(
        method.kind,
        ParamKind::Choice(&["Brightness", "Remove background", "Pick colours"])
    );
    assert_eq!(method.label, "Method");
    assert_eq!(Selection::default(), Selection::Brightness);
}

/// On an opaque grey picture Brightness is exactly "grey below the Threshold", and Invert is its
/// exact complement.
#[test]
fn c14_brightness_on_opaque_images_is_the_threshold_cut() {
    let p = pic(256, 4, |x, _| [x as u8, x as u8, x as u8, 255]);
    let thr = 100_u8;
    let params = TraceParams::with_threshold(thr);
    let mask = selected(p.raster(), &params);
    let inv = selected(
        p.raster(),
        &TraceParams {
            invert: true,
            ..params
        },
    );
    assert_eq!(mask.len(), 256 * 4);
    for x in 0..256_usize {
        // The value of the cut itself is not fixed by the spec (below vs. at-or-below).
        if (x as i32 - i32::from(thr)).abs() <= 1 {
            continue;
        }
        for y in 0..4 {
            assert_eq!(mask[y * 256 + x], x < usize::from(thr), "grey {x}");
            assert_eq!(inv[y * 256 + x], !mask[y * 256 + x], "inverted grey {x}");
        }
    }
}

/// An opaque picture traces to the same outlines whichever way Brightness is spelled (the default
/// params and an explicit `Selection::Brightness`): nothing of the new method layer leaks in.
#[test]
fn c14_brightness_traces_an_opaque_disc_with_the_old_geometry() {
    let inside = disc(50.0, 50.0, 30.0);
    let p = pic(100, 100, |x, y| {
        mix(WHITE, [0, 0, 0], coverage(x, y, &inside))
    });
    let r = trace_with(&p, &TraceParams::with_threshold(128));
    assert_eq!(r.outlines.len(), 1);
    assert!(max_radial_error(&r, 50.0, 50.0, 30.0) < 0.6);
    let again = trace_with(&p, &base(Selection::Brightness));
    assert_eq!(r.outlines.len(), again.outlines.len());
}

// ---------------------------------------------------------------- criterion 15

fn every_selection() -> Vec<Selection> {
    vec![
        Selection::Brightness,
        Selection::RemoveBackground {
            background: Some(Rgb8::new(255, 255, 255)),
            tolerance: 40,
        },
        Selection::RemoveBackground {
            background: None,
            tolerance: 40,
        },
        pick(&[[0, 0, 0]], 15),
        pick(&[[0, 0, 0], [255, 255, 255]], 100),
    ]
}

#[test]
fn c15_alpha_zero_is_never_content_in_any_method_also_inverted_also_black() {
    // Left half alpha 0 with black RGB, right half alpha 0 with white RGB, middle band opaque.
    let p = pic(30, 6, |x, _| match x {
        0..=9 => [0, 0, 0, 0],
        10..=19 => [255, 255, 255, 0],
        _ => [0, 0, 0, 255],
    });
    for selection in every_selection() {
        for invert in [false, true] {
            let params = TraceParams {
                invert,
                ..base(selection)
            };
            let mask = selected(p.raster(), &params);
            assert_eq!(mask.len(), 30 * 6);
            for y in 0..6 {
                for x in 0..20 {
                    assert!(!mask[y * 30 + x], "{selection:?} invert={invert} at {x}");
                }
            }
            // And the whole trace of a fully transparent picture is empty.
            let clear = pic(20, 20, |_, _| [0, 0, 0, 0]);
            let r = trace_with(&clear, &params);
            assert_eq!(r.outlines.len(), 0, "{selection:?} invert={invert}");
        }
    }
}

/// A partly transparent black pixel's alpha counts once: membership = alpha / 255, so a pixel at
/// alpha 140 (0.55) is content at the 50 % cut and one at alpha 115 (0.45) is not. Double
/// counting (composite on white, then weight by alpha again) would give 0.30 and 0.20.
#[test]
fn c15_partial_alpha_counts_once() {
    let p = pic(2, 1, |x, _| [0, 0, 0, if x == 0 { 140 } else { 115 }]);
    let mask = selected(p.raster(), &TraceParams::with_threshold(128));
    assert_eq!(mask, vec![true, false], "alpha 140 content, alpha 115 not");
}

/// The background is not assumed white: a half-transparent black edge ring on transparent
/// ground, traced with Brightness, makes the shape even though no pixel is "white".
#[test]
fn c15_background_is_not_assumed_white() {
    let inside = disc(30.0, 30.0, 20.0);
    let p = pic(60, 60, |x, y| {
        let c = coverage(x, y, &inside);
        [0, 0, 0, (c * 255.0).round() as u8]
    });
    let r = trace_with(&p, &TraceParams::with_threshold(128));
    assert_eq!(r.outlines.len(), 1);
    assert!(max_radial_error(&r, 30.0, 30.0, 20.0) < 0.75);
}

// ---------------------------------------------------------------- criterion 16

/// Logo on #3366CC: a dark disc (anti-aliased), a pale #FFF2A8 square and a black bar.
fn logo(noise: u8) -> (Pic, f64) {
    let inside = disc(60.0, 60.0, 30.0);
    let (x0, y0, x1, y1) = (110, 100, 160, 140); // pale 50 x 40
    let (bx0, by0, bx1, by1) = (20, 150, 100, 170); // black 80 x 20
    let truth = std::f64::consts::PI * 30.0 * 30.0 + 50.0 * 40.0 + 80.0 * 20.0;
    let p = pic(200, 200, |x, y| {
        let n = if noise == 0 {
            0
        } else {
            // Deterministic pseudo-noise in -noise..=noise on the background only.
            let h = (x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663))
                % (2 * u32::from(noise) + 1);
            h as i32 - i32::from(noise)
        };
        let bg = BLUE.map(|c| (i32::from(c) + n).clamp(0, 255) as u8);
        if x >= x0 && x < x1 && y >= y0 && y < y1 {
            return [PALE[0], PALE[1], PALE[2], 255];
        }
        if x >= bx0 && x < bx1 && y >= by0 && y < by1 {
            return [0, 0, 0, 255];
        }
        mix(bg, [0, 0, 0], coverage(x, y, &inside))
    });
    (p, truth)
}

fn dist_unit_cube(a: Rgb8, b: [u8; 3]) -> f64 {
    let d = |p: u8, q: u8| (f64::from(p) - f64::from(q)) / 255.0;
    (d(a.r, b[0]).powi(2) + d(a.g, b[1]).powi(2) + d(a.b, b[2]).powi(2)).sqrt()
}

#[test]
fn c16_auto_suggests_the_background_within_0_02_and_the_logo_keeps_the_pale_element() {
    let (p, truth) = logo(0);
    let bg = suggest_background(p.raster()).expect("an opaque border has a background");
    assert!(dist_unit_cube(bg, BLUE) <= 0.02, "suggested {bg:?}");
    for tolerance in [20_u8, 40, 60] {
        let r = trace_with(
            &p,
            &base(Selection::RemoveBackground {
                background: Some(bg),
                tolerance,
            }),
        );
        let err = (area_px(&r) - truth).abs() / truth;
        assert!(
            err <= 0.02,
            "tolerance {tolerance}: area error {:.1} %",
            err * 100.0
        );
    }
}

#[test]
fn c16_auto_suggestion_survives_jpeg_like_border_noise() {
    let (p, _) = logo(4);
    let bg = suggest_background(p.raster()).unwrap();
    assert!(dist_unit_cube(bg, BLUE) <= 0.02, "suggested {bg:?}");
}

#[test]
fn c16_colour_tolerance_widens_the_background() {
    let (p, _) = logo(6);
    let count = |tolerance: u8| {
        selected(
            p.raster(),
            &base(Selection::RemoveBackground {
                background: Some(Rgb8::new(BLUE[0], BLUE[1], BLUE[2])),
                tolerance,
            }),
        )
        .iter()
        .filter(|&&c| c)
        .count()
    };
    let (t0, t30, t100) = (count(0), count(30), count(100));
    assert!(t0 > t30, "noise is content at tolerance 0: {t0} vs {t30}");
    assert!(t30 >= count(60), "monotonic");
    // The spec fixes only the direction: a larger tolerance never selects more.
    assert!(t100 <= t30, "tolerance 100 selects no more than 30");
    // At a fitting tolerance the content is the logo only: far below the noisy tolerance 0.
    assert!(t30 < 200 * 200 / 3);
}

#[test]
fn c16_transparent_border_means_transparent_background() {
    let inside = disc(30.0, 30.0, 20.0);
    let p = pic(60, 60, |x, y| {
        if inside(f64::from(x) + 0.5, f64::from(y) + 0.5) {
            [200, 30, 30, 255]
        } else {
            [0, 0, 0, 0]
        }
    });
    assert_eq!(suggest_background(p.raster()), None);
}

// ---------------------------------------------------------------- criterion 17

/// White ground, a pale disc (#FFF2A8) at (50, 50) and a light grey disc (#D9D9D9) at (150, 50),
/// both r = 30 and anti-aliased.
fn two_pale_discs() -> Pic {
    let a = disc(50.0, 50.0, 30.0);
    let b = disc(150.0, 50.0, 30.0);
    pic(200, 100, |x, y| {
        let ca = coverage(x, y, &a);
        if ca > 0.0 {
            return mix(WHITE, PALE, ca);
        }
        mix(WHITE, [0xD9, 0xD9, 0xD9], coverage(x, y, &b))
    })
}

#[test]
fn c17_a_pale_colour_and_a_light_grey_are_each_traced_alone_within_0_75_px() {
    let p = two_pale_discs();
    for (colour, cx, tolerance) in [
        (PALE, 50.0, 15),
        ([0xD9, 0xD9, 0xD9], 150.0, 15),
        (PALE, 50.0, DEFAULT_PICK_TOLERANCE),
        ([0xD9, 0xD9, 0xD9], 150.0, DEFAULT_PICK_TOLERANCE),
    ] {
        let r = trace_with(&p, &base(pick(&[colour], tolerance)));
        assert_eq!(r.outlines.len(), 1, "colour {colour:?} traces one shape");
        let err = max_radial_error(&r, cx, 50.0, 30.0);
        assert!(
            err <= 0.75,
            "colour {colour:?}: boundary {err:.2} px from the truth"
        );
    }
}

#[test]
fn c17_up_to_eight_swatches() {
    let eight: Vec<Rgb8> = (0..12).map(|i| Rgb8::new(i * 20, 0, 0)).collect();
    let s = Swatches::from_slice(&eight);
    assert_eq!(MAX_SWATCHES, 8);
    assert_eq!(s.as_slice().len(), 8);
    assert_eq!(s.as_slice()[0], eight[0]);
    assert_eq!(Swatches::from_slice(&[]).as_slice().len(), 0);
}

#[test]
fn c17_near_greys_d9d9d9_and_dcd6cf_are_separated() {
    let a = [0xD9, 0xD9, 0xD9];
    let b = [0xDC, 0xD6, 0xCF];
    // Two blocks side by side on white, separated by a white gap.
    let p = pic(120, 60, |x, y| match (x, y) {
        (10..=49, 10..=49) => [a[0], a[1], a[2], 255],
        (70..=109, 10..=49) => [b[0], b[1], b[2], 255],
        _ => [255, 255, 255, 255],
    });
    let only = |colour: [u8; 3]| selected(p.raster(), &base(pick(&[colour], 15)));
    let ma = only(a);
    let mb = only(b);
    let at = |m: &[bool], x: usize, y: usize| m[y * 120 + x];
    assert!(
        at(&ma, 30, 30) && !at(&ma, 90, 30),
        "picking #D9D9D9 takes only its block"
    );
    assert!(
        at(&mb, 90, 30) && !at(&mb, 30, 30),
        "picking #DCD6CF takes only its block"
    );
    assert!(!at(&ma, 2, 2) && !at(&mb, 2, 2), "white is never picked");
}

#[test]
fn c17_empty_swatch_list_and_degenerate_pictures_do_not_panic() {
    let p = two_pale_discs();
    let r = trace_with(&p, &base(pick(&[], 15)));
    assert_eq!(r.outlines.len(), 0);
    let one = pic(1, 1, |_, _| [10, 20, 30, 255]);
    let r = trace_with(&one, &base(pick(&[[10, 20, 30]], 15)));
    assert!(r.outlines.len() <= 1);
    let r = trace_with(
        &one,
        &base(Selection::RemoveBackground {
            background: None,
            tolerance: 0,
        }),
    );
    assert!(r.outlines.len() <= 1);
}

/// A disc of `ink` on `ground`, anti-aliased, with per-channel noise of +-`amp` on every pixel.
fn noisy_disc(ink: [u8; 3], ground: [u8; 3], amp: i32) -> Pic {
    let inside = disc(100.0, 100.0, 60.0);
    pic(200, 200, |x, y| {
        let mut c = mix(ground, ink, coverage(x, y, &inside));
        let h = x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663);
        for (k, v) in c.iter_mut().take(3).enumerate() {
            let n = ((h >> (5 * k)) % (2 * amp as u32 + 1)) as i32 - amp;
            *v = (i32::from(*v) + n).clamp(0, 255) as u8;
        }
        c
    })
}

/// Criterion 17 (default Colour tolerance 19, noise filter): at least 99 % of a picked shape's
/// solid pixels with +-6 per channel of noise. (+-3 is printed; the spec fixes only +-6.)
#[test]
fn c17_the_default_colour_tolerance_takes_99_percent_of_a_noisy_shape() {
    let red = [0xCC, 0x22, 0x22];
    let inside = disc(100.0, 100.0, 60.0);
    assert_eq!(DEFAULT_PICK_TOLERANCE, 19);
    for (amp, required) in [(6, true), (3, false)] {
        let p = noisy_disc(red, BLUE, amp);
        let params = TraceParams {
            selection: pick_on(&[red], BLUE, DEFAULT_PICK_TOLERANCE),
            ..base(Selection::Brightness)
        };
        let mask = selected(p.raster(), &params);
        let (mut total, mut got) = (0_usize, 0_usize);
        for y in 0..200_u32 {
            for x in 0..200_u32 {
                if coverage(x, y, &inside) >= 1.0 {
                    total += 1;
                    got += usize::from(mask[(y * 200 + x) as usize]);
                }
            }
        }
        println!(
            "c17 +-{amp} noise at the default: {:.2} % of {total} solid pixels",
            got as f64 * 100.0 / total as f64
        );
        if required {
            assert!(
                got * 100 >= total * 99,
                "+-{amp}: selected {got} of {total}"
            );
        }
    }
}

/// Thin features in the picked colour survive the noise filter: a 1 px line and a 2 px dot are
/// still selected at the default (clean, and the clean share reported with +-3 noise).
#[test]
fn c17_thin_lines_and_dots_in_the_picked_colour_stay_selected() {
    let ink = [0xCC, 0x22, 0x22];
    let line = |x: u32, y: u32| y == 20 && (10..110).contains(&x);
    let dot = |x: u32, y: u32| (150..152).contains(&x) && (50..52).contains(&y);
    let diag = |x: u32, y: u32| x == y && (40..100).contains(&x);
    for ground in [WHITE, BLUE] {
        let p = pic(200, 120, |x, y| {
            if line(x, y) || dot(x, y) || diag(x, y) {
                [ink[0], ink[1], ink[2], 255]
            } else {
                [ground[0], ground[1], ground[2], 255]
            }
        });
        let params = TraceParams {
            selection: pick_on(&[ink], ground, DEFAULT_PICK_TOLERANCE),
            ..base(Selection::Brightness)
        };
        let mask = selected(p.raster(), &params);
        for (name, f) in [
            ("line", &line as &dyn Fn(u32, u32) -> bool),
            ("dot", &dot),
            ("diagonal", &diag),
        ] {
            let (mut total, mut got) = (0, 0);
            for y in 0..120_u32 {
                for x in 0..200_u32 {
                    if f(x, y) {
                        total += 1;
                        got += usize::from(mask[(y * 200 + x) as usize]);
                    }
                }
            }
            assert_eq!(
                got, total,
                "1-2 px {name} on {ground:?}: {got} of {total} pixels selected"
            );
        }
        // And nothing else is selected: the ground stays out.
        let extra = mask
            .iter()
            .enumerate()
            .filter(|&(i, &m)| {
                m && !(line(i as u32 % 200, i as u32 / 200)
                    || dot(i as u32 % 200, i as u32 / 200)
                    || diag(i as u32 % 200, i as u32 / 200))
            })
            .count();
        assert_eq!(extra, 0, "ground {ground:?}");
    }
}

// ---------------------------------------------------------------- criterion 18

#[test]
fn c18_anti_aliased_edges_are_not_shrunk_inward() {
    let red = [0xCC, 0x22, 0x22];
    let inside = disc(60.0, 60.0, 40.0);
    let p = pic(120, 120, |x, y| mix(WHITE, red, coverage(x, y, &inside)));
    for tolerance in [10_u8, 15, 30] {
        let r = trace_with(&p, &base(pick(&[red], tolerance)));
        assert_eq!(r.outlines.len(), 1);
        let err = max_radial_error(&r, 60.0, 60.0, 40.0);
        assert!(
            err <= 0.5,
            "tolerance {tolerance}: outline {err:.2} px from the true edge"
        );
    }
}

/// Largest distance from a point of the true circle to the traced outline, pixels: the other
/// direction of the 0.5 px bound (the outline may not miss a stretch of the edge either).
fn max_truth_to_outline(r: &TraceResult, cx: f64, cy: f64, radius: f64) -> f64 {
    let segs: Vec<(Point, Point)> = r
        .outlines
        .iter()
        .flat_map(|o| {
            let pts = flat_px(o);
            (0..pts.len())
                .map(|i| (pts[i], pts[(i + 1) % pts.len()]))
                .collect::<Vec<_>>()
        })
        .collect();
    (0..720)
        .map(|k| {
            let a = f64::from(k) * std::f64::consts::TAU / 720.0;
            let t = Point::new(cx + radius * a.cos(), cy + radius * a.sin());
            segs.iter()
                .map(|&(p, q)| {
                    let (dx, dy) = (q.x - p.x, q.y - p.y);
                    let len2 = dx * dx + dy * dy;
                    let u = if len2 == 0.0 {
                        0.0
                    } else {
                        (((t.x - p.x) * dx + (t.y - p.y) * dy) / len2).clamp(0.0, 1.0)
                    };
                    (t.x - (p.x + u * dx)).hypot(t.y - (p.y + u * dy))
                })
                .fold(f64::MAX, f64::min)
        })
        .fold(0.0, f64::max)
}

/// Criterion 18 with the noise filter on (the default Colour tolerance): the outline lies within
/// 0.5 px of the true edge measured both ways, on a clean anti-aliased disc; the noisy disc
/// (+-3) is printed only.
#[test]
fn c18_the_edge_is_within_half_a_pixel_both_ways_at_the_default() {
    let red = [0xCC, 0x22, 0x22];
    let inside = disc(60.0, 60.0, 40.0);
    let p = pic(120, 120, |x, y| mix(WHITE, red, coverage(x, y, &inside)));
    let r = trace_with(&p, &base(pick(&[red], DEFAULT_PICK_TOLERANCE)));
    assert_eq!(r.outlines.len(), 1);
    let (a, b) = (
        max_radial_error(&r, 60.0, 60.0, 40.0),
        max_truth_to_outline(&r, 60.0, 60.0, 40.0),
    );
    assert!(
        a <= 0.5 && b <= 0.5,
        "outline to edge {a:.2} px, edge to outline {b:.2} px"
    );
    let noisy = noisy_disc(red, WHITE, 3);
    let r = trace_with(
        &noisy,
        &base(pick_on(&[red], WHITE, DEFAULT_PICK_TOLERANCE)),
    );
    println!(
        "c18 +-3 noise: {} paths, outline to edge {:.2} px, edge to outline {:.2} px",
        r.outlines.len(),
        max_radial_error(&r, 100.0, 100.0, 60.0),
        max_truth_to_outline(&r, 100.0, 100.0, 60.0)
    );
}

/// The same on a coloured (non-white) ground: the blend is toward the background, wherever it is.
#[test]
fn c18_blends_toward_a_coloured_background_are_included() {
    let ink = [0x20, 0x80, 0x20];
    let inside = disc(60.0, 60.0, 35.0);
    let p = pic(120, 120, |x, y| mix(PALE, ink, coverage(x, y, &inside)));
    let r = trace_with(&p, &base(pick_on(&[ink], PALE, 15)));
    assert_eq!(r.outlines.len(), 1);
    assert!(max_radial_error(&r, 60.0, 60.0, 35.0) <= 0.5);
}

// ---------------------------------------------------------------- criterion 19

/// A red block touching a blue block, on white: hard seam.
fn touching(seam_blend: bool) -> Pic {
    let red = [0xCC, 0x22, 0x22];
    let blue = [0x22, 0x22, 0xCC];
    pic(120, 60, |x, y| {
        if !(10..50).contains(&y) {
            return [255, 255, 255, 255];
        }
        match x {
            10..=49 => [red[0], red[1], red[2], 255],
            50 if seam_blend => mix(red, blue, 0.5),
            50..=89 => [blue[0], blue[1], blue[2], 255],
            _ => [255, 255, 255, 255],
        }
    })
}

#[test]
fn c19_two_touching_picked_colours_are_one_compound_path_without_a_seam() {
    let p = touching(false);
    let r = trace_with(
        &p,
        &base(pick(&[[0xCC, 0x22, 0x22], [0x22, 0x22, 0xCC]], 15)),
    );
    assert_eq!(r.outlines.len(), 1, "one path for both colours");
    // Truth: the 80 x 40 union. No outline point lies inside it (a seam would).
    assert!((area_px(&r) - 80.0 * 40.0).abs() / 3200.0 < 0.02);
    for p in r.outlines.iter().flat_map(flat_px) {
        let inner = p.x > 10.7 && p.x < 89.3 && p.y > 10.7 && p.y < 49.3;
        assert!(!inner, "outline point {p:?} lies inside the union (a seam)");
    }
}

/// An anti-aliased seam (a pixel mixing the two picked colours) must not open a gap: the blend
/// of two picked colours belongs to the selection too.
#[test]
fn c19_a_blended_seam_pixel_between_two_picked_colours_is_content() {
    let p = touching(true);
    let r = trace_with(
        &p,
        &base(pick(&[[0xCC, 0x22, 0x22], [0x22, 0x22, 0xCC]], 15)),
    );
    assert_eq!(
        r.outlines.len(),
        1,
        "no gap along the seam: one path, no hole"
    );
}

// ---------------------------------------------------------------- criterion 20

fn thread_cpu_secs() -> f64 {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
    t.tv_sec as f64 + t.tv_nsec as f64 * 1e-9
}

#[test]
#[ignore = "timing: run in release"]
fn c20_selection_on_2000_square_costs_at_most_0_5_s_cpu_and_the_trace_stays_under_4_s() {
    let (a, b) = (disc(700.0, 1000.0, 400.0), disc(1400.0, 1000.0, 400.0));
    // Per-pixel noise of +-6 per channel (a JPEG-like picture: almost every pixel its own colour).
    let p = pic(2000, 2000, |x, y| {
        let ca = coverage(x, y, &a);
        let mut c = if ca > 0.0 {
            mix(BLUE, [0xCC, 0x22, 0x22], ca)
        } else {
            mix(BLUE, PALE, coverage(x, y, &b))
        };
        let h = x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663);
        for (k, v) in c.iter_mut().take(3).enumerate() {
            let n = ((h >> (5 * k)) % 13) as i32 - 6;
            *v = (i32::from(*v) + n).clamp(0, 255) as u8;
        }
        c
    });
    let methods = [
        Selection::RemoveBackground {
            background: Some(Rgb8::new(BLUE[0], BLUE[1], BLUE[2])),
            tolerance: 40,
        },
        Selection::RemoveBackground {
            background: None,
            tolerance: 40,
        },
        pick_on(&[[0xCC, 0x22, 0x22], PALE], BLUE, 35),
        pick_on(
            &[
                [0xCC, 0x22, 0x22],
                PALE,
                [1, 2, 3],
                [9, 9, 9],
                [50, 60, 70],
                [90, 10, 10],
                [10, 90, 10],
                [10, 10, 90],
            ],
            BLUE,
            30,
        ),
    ];
    for selection in methods {
        // The default Ignore below, as in use: noise speckle is removed as the app does.
        let params = TraceParams {
            selection,
            ..TraceParams::with_threshold(128)
        };
        let brightness = TraceParams::with_threshold(128);
        // Warm up and measure the selection stage alone.
        let t = thread_cpu_secs();
        let mask = selected(p.raster(), &params);
        let sel = thread_cpu_secs() - t;
        println!(
            "c20 {selection:?}: selection {sel:.3} s CPU, {} content px",
            mask.iter().filter(|&&c| c).count()
        );
        assert!(sel <= 0.5, "selection {sel:.3} s");
        let t = thread_cpu_secs();
        let _ = trace_with(&p, &params);
        let whole = thread_cpu_secs() - t;
        let t = thread_cpu_secs();
        let _ = trace_with(&p, &brightness);
        let whole_b = thread_cpu_secs() - t;
        println!("c20   trace {whole:.2} s CPU (Brightness on the same picture {whole_b:.2} s)");
        assert!(whole < 4.0);
    }
}
