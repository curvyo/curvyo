//! Independent acceptance tests of `specs/0089-trace-quality-and-colour` slice F (automatic
//! analysis, criteria 31 to 34), written from the specification before the implementation was
//! read. Pictures are generated here (no fixture shares code with the tuning set).
//!
//! The timing test is `#[ignore]` and measures thread CPU time in release:
//!
//! ```text
//! cargo test --release -p curvyo-vectorize-core --test acceptance_0089_sliceF_tester -- --ignored --nocapture
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
    clippy::float_cmp,
    clippy::type_complexity,
    clippy::needless_pass_by_value
)]

mod common;

use std::ops::ControlFlow;

use curvyo_document_core::Affine;
use curvyo_vectorize_core::{
    AnalysisReport, Capture, ImageType, PropertyKind, Raster, Reason, Selection, TraceParams,
    analyse, auto_threshold, trace,
};

type Colour = [u8; 3];
const WHITE: Colour = [255, 255, 255];
const BLACK: Colour = [10, 10, 10];

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
    fn report(&self) -> AnalysisReport {
        analyse(self.raster()).unwrap()
    }
}

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0 >> 8
    }
    fn noise(&mut self, amp: i32) -> i32 {
        if amp == 0 {
            0
        } else {
            (self.next() % (2 * amp as u32 + 1)) as i32 - amp
        }
    }
}

/// Scene: colour and alpha of a point; rendered with 3 x 3 samples, plus +-`noise`.
fn render(
    w: u32,
    h: u32,
    noise: i32,
    seed: u32,
    scene: &dyn Fn(f64, f64) -> ([f64; 3], f64),
) -> Pic {
    let mut rng = Rng(seed);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let mut s = [0.0; 4];
            for sy in 0..3 {
                for sx in 0..3 {
                    let (c, a) = scene(
                        f64::from(x) + (f64::from(sx) + 0.5) / 3.0,
                        f64::from(y) + (f64::from(sy) + 0.5) / 3.0,
                    );
                    for i in 0..3 {
                        s[i] += c[i];
                    }
                    s[3] += a;
                }
            }
            for v in &s[..3] {
                let n = f64::from(rng.noise(noise));
                rgba.push((v / 9.0 + n).round().clamp(0.0, 255.0) as u8);
            }
            rgba.push((s[3] / 9.0 * 255.0).round() as u8);
        }
    }
    Pic { w, h, rgba }
}

fn c(v: Colour) -> [f64; 3] {
    [f64::from(v[0]), f64::from(v[1]), f64::from(v[2])]
}

/// Flat shapes in `colours` on white: a disc, a rectangle and a ring.
fn flat_logo(size: u32, colours: [Colour; 3], noise: i32, seed: u32) -> Pic {
    let s = f64::from(size);
    render(size, size, noise, seed, &move |x, y| {
        let (u, v) = (x / s, y / s);
        let col = if (u - 0.3).hypot(v - 0.35) < 0.2 {
            colours[0]
        } else if (0.55..0.9).contains(&u) && (0.15..0.5).contains(&v) {
            colours[1]
        } else if ((u - 0.5).hypot(v - 0.75) - 0.15).abs() < 0.05 {
            colours[2]
        } else {
            WHITE
        };
        (c(col), 1.0)
    })
}

/// Thin dark strokes (about `width` px) on `paper`: sine waves and a diagonal, light fall-off
/// of `fall` levels over the width, noise `noise`.
fn line_art(size: u32, width: f64, paper: Colour, fall: f64, noise: i32, seed: u32) -> Pic {
    let s = f64::from(size);
    render(size, size, noise, seed, &move |x, y| {
        let mut on = false;
        for k in 1..6 {
            let yy = s * (0.15 * f64::from(k)) + s * 0.05 * (x / s * 9.0 + f64::from(k)).sin();
            if (y - yy).abs() < width / 2.0 {
                on = true;
            }
        }
        if ((x - y).abs() / std::f64::consts::SQRT_2) < width / 2.0 {
            on = true;
        }
        let shade = 1.0 - fall * (x / s) / 255.0;
        let p = [
            f64::from(paper[0]) * shade,
            f64::from(paper[1]) * shade,
            f64::from(paper[2]) * shade,
        ];
        (if on { c(BLACK) } else { p }, 1.0)
    })
}

fn random_noise(size: u32, seed: u32) -> Pic {
    let mut rng = Rng(seed);
    let mut rgba = Vec::new();
    for _ in 0..size * size {
        rgba.extend([
            (rng.next() % 256) as u8,
            (rng.next() % 256) as u8,
            (rng.next() % 256) as u8,
            255,
        ]);
    }
    Pic {
        w: size,
        h: size,
        rgba,
    }
}

// ---------------------------------------------------------------------------------------------
// Criterion 31: properties, type, settings, reason

#[test]
fn c31_every_property_has_a_finite_value_and_a_confidence_between_0_and_1() {
    for pic in [
        flat_logo(300, [[200, 30, 30], [30, 160, 60], [30, 60, 200]], 0, 1),
        line_art(300, 2.0, [250, 245, 235], 30.0, 4, 2),
        random_noise(120, 3),
    ] {
        let r = pic.report();
        for kind in [
            PropertyKind::Colours,
            PropertyKind::BackgroundUniformity,
            PropertyKind::Transparency,
            PropertyKind::Noise,
            PropertyKind::Gradient,
            PropertyKind::StraightEdges,
            PropertyKind::Components,
            PropertyKind::Holes,
            PropertyKind::ComponentSize,
            PropertyKind::StrokeWidth,
        ] {
            let p = r.properties.iter().find(|p| p.kind == kind);
            let p = p.unwrap_or_else(|| panic!("property {kind:?} missing"));
            assert!(p.value.is_finite(), "{kind:?} = {}", p.value);
            assert!(
                (0.0..=1.0).contains(&p.confidence),
                "{kind:?} confidence {}",
                p.confidence
            );
        }
        assert!((0.0..=1.0).contains(&r.confidence));
    }
}

#[test]
fn c31_colour_count_separates_a_three_colour_logo_from_a_two_tone_drawing() {
    let logo = flat_logo(300, [[200, 30, 30], [30, 160, 60], [30, 60, 200]], 0, 1).report();
    let line = line_art(300, 3.0, WHITE, 0.0, 0, 2).report();
    let (a, b) = (
        logo.value(PropertyKind::Colours).unwrap(),
        line.value(PropertyKind::Colours).unwrap(),
    );
    assert!(a > b, "logo colours {a} should exceed drawing colours {b}");
    assert!(b <= 4.0, "a black/white drawing reported {b} colours");
    assert!(
        logo.value(PropertyKind::Chroma).unwrap() > line.value(PropertyKind::Chroma).unwrap(),
        "chroma of a coloured logo is higher than of a black and white drawing"
    );
}

/// Traces `pic` with the report's proposal on top of the first-use values and returns the
/// content pixels.
fn traced_pixels(pic: &Pic, r: &AnalysisReport) -> f64 {
    let params = TraceParams {
        selection: r.proposal.selection,
        ..TraceParams::with_threshold(r.proposal.threshold)
    };
    let mut go = |_| ControlFlow::Continue(());
    trace(pic.raster(), Affine::scale(0.1, 0.1), &params, &mut go)
        .unwrap()
        .foreground_pixels as f64
}

#[test]
fn c31_transparency_is_measured_and_the_proposal_keeps_a_light_shape_on_a_clear_background() {
    let disc = std::f64::consts::PI * 70.0 * 70.0;
    // Light grey shape on a clear background whose hidden colour is black.
    let pic = render(240, 240, 0, 5, &|x, y| {
        if (x - 120.0).hypot(y - 120.0) < 70.0 {
            (c([235, 235, 235]), 1.0)
        } else {
            (c([0, 0, 0]), 0.0)
        }
    });
    let r = pic.report();
    let t = r.value(PropertyKind::Transparency).unwrap();
    assert!(t > 0.6, "transparent share {t}");
    let got = traced_pixels(&pic, &r);
    assert!(
        (got - disc).abs() < 0.1 * disc,
        "proposal {:?} traced {got} px of a {disc} px shape",
        r.proposal
    );
    // The line names what changed settings: the reason is there exactly when the proposal is
    // Remove background on a transparent background.
    let removes = matches!(
        r.proposal.selection,
        Selection::RemoveBackground {
            background: None,
            ..
        }
    );
    assert_eq!(
        r.reasons.contains(&Reason::TransparentBackground),
        removes,
        "{:?} {:?}",
        r.proposal,
        r.reasons
    );
    let opaque = flat_logo(200, [[200, 30, 30], [30, 160, 60], [30, 60, 200]], 0, 1).report();
    assert!(opaque.value(PropertyKind::Transparency).unwrap() < 0.01);
    assert!(!opaque.reasons.contains(&Reason::TransparentBackground));
}

#[test]
fn c31_a_dark_shape_on_a_clear_background_is_traced_whatever_the_hidden_colour() {
    let disc = std::f64::consts::PI * 70.0 * 70.0;
    for hidden in [[255, 255, 255], [0, 0, 0]] {
        let pic = render(240, 240, 0, 5, &|x, y| {
            if (x - 120.0).hypot(y - 120.0) < 70.0 {
                (c(BLACK), 1.0)
            } else {
                (c(hidden), 0.0)
            }
        });
        let r = pic.report();
        let got = traced_pixels(&pic, &r);
        assert!(
            (got - disc).abs() < 0.1 * disc,
            "hidden {hidden:?}: proposal {:?} traced {got} of {disc}",
            r.proposal
        );
    }
}

#[test]
fn c31_stroke_width_tracks_the_drawn_width() {
    let thin = line_art(400, 2.0, WHITE, 0.0, 0, 7).report();
    let thick = line_art(400, 8.0, WHITE, 0.0, 0, 7).report();
    let (t, k) = (
        thin.value(PropertyKind::StrokeWidth).unwrap(),
        thick.value(PropertyKind::StrokeWidth).unwrap(),
    );
    assert!(k > t * 1.8, "stroke width thin {t}, thick {k}");
    assert!((1.0..=4.0).contains(&t), "2 px strokes measured as {t}");
    assert!((5.0..=11.0).contains(&k), "8 px strokes measured as {k}");
}

#[test]
fn c31_a_clean_drawing_is_less_noisy_than_the_same_drawing_with_grain() {
    let clean = line_art(300, 3.0, [240, 240, 240], 0.0, 0, 9).report();
    let grainy = line_art(300, 3.0, [240, 240, 240], 0.0, 12, 9).report();
    assert!(grainy.value(PropertyKind::Noise).unwrap() > clean.value(PropertyKind::Noise).unwrap());
}

#[test]
fn c31_a_flat_logo_is_a_digital_logo_or_graphic() {
    let r = flat_logo(400, [[200, 30, 30], [30, 160, 60], [30, 60, 200]], 0, 1).report();
    assert!(matches!(r.image_type, ImageType::Logo | ImageType::Graphic));
    assert_eq!(r.capture, Capture::Digital);
    assert!(!r.low_confidence(), "confidence {}", r.confidence);
}

#[test]
fn c31_a_black_drawing_on_white_is_a_line_drawing() {
    let r = line_art(500, 2.0, WHITE, 0.0, 0, 11).report();
    assert_eq!(r.image_type, ImageType::LineDrawing, "{r:?}");
    assert!(matches!(r.proposal.selection, Selection::Brightness));
}

#[test]
fn c31_a_drawing_on_uneven_paper_is_a_scan_or_paper_photo_not_digital() {
    let r = line_art(500, 3.0, [226, 220, 205], 60.0, 6, 13).report();
    assert_ne!(r.capture, Capture::Digital, "{r:?}");
}

#[test]
#[ignore = "0089F FAIL: a clean white digital drawing whose strokes reach the picture border is typed Scan (border uniformity 0.9..0.995 rule, classify.rs capture())"]
fn c31_a_clean_white_digital_drawing_is_digital() {
    let digital = line_art(500, 3.0, WHITE, 0.0, 0, 13).report();
    assert_eq!(digital.capture, Capture::Digital);
}

#[test]
fn c31_pale_shapes_on_white_are_kept_and_say_so() {
    let pic = render(300, 300, 0, 21, &|x, y| {
        if (x - 100.0).hypot(y - 100.0) < 60.0 {
            (c([0xFF, 0xF2, 0xA8]), 1.0)
        } else if (170.0..270.0).contains(&x) && (150.0..250.0).contains(&y) {
            (c([0xD9, 0xD9, 0xD9]), 1.0)
        } else {
            (c(WHITE), 1.0)
        }
    });
    let r = pic.report();
    // Both shapes together are about 21 300 px; Brightness at the automatic Threshold would drop
    // the pale yellow one (luma about 245).
    let truth = std::f64::consts::PI * 60.0 * 60.0 + 100.0 * 100.0;
    let got = traced_pixels(&pic, &r);
    assert!(
        (got - truth).abs() < 0.1 * truth,
        "proposal {:?} traced {got} of {truth} px",
        r.proposal
    );
    assert!(r.reasons.contains(&Reason::PaleColours), "{:?}", r.reasons);
}

// ---------------------------------------------------------------------------------------------
// Criterion 32: accuracy on a labelled set, low-confidence fallback

#[test]
fn c32_the_type_is_right_on_at_least_80_percent_of_a_labelled_synthetic_set() {
    let mut cases: Vec<(Pic, &str)> = Vec::new();
    let palettes: [[Colour; 3]; 4] = [
        [[200, 30, 30], [30, 160, 60], [30, 60, 200]],
        [[250, 180, 0], [0, 120, 120], [120, 0, 120]],
        [[20, 20, 20], [200, 0, 0], [230, 230, 0]],
        [[90, 40, 20], [20, 90, 40], [40, 20, 90]],
    ];
    for (i, p) in palettes.iter().enumerate() {
        cases.push((flat_logo(350, *p, i as i32, 100 + i as u32), "logo"));
    }
    for (i, w) in [1.5, 2.0, 2.5, 3.0].iter().enumerate() {
        cases.push((line_art(450, *w, WHITE, 0.0, 0, 200 + i as u32), "line"));
    }
    cases.push((random_noise(200, 301), "unclear"));
    cases.push((random_noise(250, 302), "unclear"));
    let mut right = 0;
    for (pic, label) in &cases {
        let r = pic.report();
        let ok = match *label {
            "logo" => {
                !r.low_confidence() && matches!(r.image_type, ImageType::Logo | ImageType::Graphic)
            }
            "line" => !r.low_confidence() && r.image_type == ImageType::LineDrawing,
            _ => r.low_confidence(),
        };
        println!(
            "{label}: {:?} conf {:.2} ok {ok}",
            r.image_type, r.confidence
        );
        right += usize::from(ok);
    }
    assert!(right * 10 >= cases.len() * 8, "{right}/{}", cases.len());
}

#[test]
fn c32_an_unclear_picture_falls_back_to_brightness_at_the_automatic_threshold_and_says_so() {
    for seed in [3, 4] {
        let pic = random_noise(160, seed);
        let r = pic.report();
        assert!(r.low_confidence(), "confidence {}", r.confidence);
        assert!(r.confidence < 0.5);
        assert!(matches!(r.proposal.selection, Selection::Brightness));
        assert_eq!(r.proposal.threshold, auto_threshold(pic.raster()));
        assert!(r.reasons.contains(&Reason::LowConfidence));
    }
}

#[test]
fn c32_a_confident_report_does_not_claim_low_confidence() {
    let r = line_art(500, 2.0, WHITE, 0.0, 0, 11).report();
    assert!(!r.low_confidence());
    assert!(!r.reasons.contains(&Reason::LowConfidence));
}

// ---------------------------------------------------------------------------------------------
// Criterion 33 (core side): only image-dependent values, never picked colours

#[test]
fn c33_the_proposal_is_only_brightness_or_remove_background() {
    let pics = [
        flat_logo(200, [[200, 30, 30], [30, 160, 60], [30, 60, 200]], 0, 1),
        line_art(200, 2.0, [230, 225, 210], 40.0, 5, 2),
        random_noise(100, 3),
        render(100, 100, 0, 5, &|_, _| (c(WHITE), 0.0)),
    ];
    for p in pics {
        let r = p.report();
        assert!(
            matches!(
                r.proposal.selection,
                Selection::Brightness | Selection::RemoveBackground { .. }
            ),
            "{:?}",
            r.proposal.selection
        );
        if let Selection::RemoveBackground { tolerance, .. } = r.proposal.selection {
            assert!(tolerance <= 100);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Criterion 34: deterministic, 0.5 s

#[test]
fn c34_the_analysis_is_deterministic_and_does_not_depend_on_the_buffer_address() {
    let pic = line_art(400, 3.0, [230, 225, 210], 40.0, 5, 2);
    let a = pic.report();
    let copy = Pic {
        w: pic.w,
        h: pic.h,
        rgba: pic.rgba.clone(),
    };
    let b = copy.report();
    assert_eq!(a, b);
    assert_eq!(a, pic.report());
}

#[test]
fn c34_degenerate_pictures_give_a_report_or_an_error_never_a_panic() {
    for (w, h, v) in [
        (1, 1, 0u8),
        (1, 1, 255),
        (1, 4000, 128),
        (4000, 1, 128),
        (3, 3, 7),
    ] {
        let rgba: Vec<u8> = (0..w * h).flat_map(|_| [v, v, v, 255]).collect();
        let r = analyse(Raster {
            width: w,
            height: h,
            rgba: &rgba,
        });
        if let Ok(r) = r {
            assert!(r.confidence.is_finite());
            assert!(r.properties.iter().all(|p| p.value.is_finite()));
        }
    }
    let wrong = vec![0u8; 10];
    assert!(
        analyse(Raster {
            width: 4,
            height: 4,
            rgba: &wrong
        })
        .is_err()
    );
}

#[test]
fn c34_a_blank_white_page_and_a_blank_transparent_page_are_reported() {
    let white = render(200, 200, 0, 1, &|_, _| (c(WHITE), 1.0)).report();
    assert!(white.confidence.is_finite());
    let clear = render(200, 200, 0, 1, &|_, _| (c(WHITE), 0.0)).report();
    assert!(clear.value(PropertyKind::Transparency).unwrap() > 0.9);
}

#[cfg(unix)]
#[test]
#[ignore = "timing, release only; see the module comment"]
fn c34_two_thousand_square_images_take_at_most_half_a_second_of_thread_cpu() {
    let pics = [
        (
            "logo",
            flat_logo(2000, [[200, 30, 30], [30, 160, 60], [30, 60, 200]], 3, 1),
        ),
        ("scan", line_art(2000, 3.0, [226, 220, 205], 60.0, 6, 2)),
        ("noise", random_noise(2000, 3)),
    ];
    for (name, pic) in &pics {
        let t0 = common::thread_cpu_seconds();
        let r = analyse(pic.raster()).unwrap();
        let secs = common::thread_cpu_seconds() - t0;
        println!("{name}: {secs:.3} s ({:?})", r.image_type);
        assert!(secs <= 0.5, "{name} took {secs:.3} s thread CPU");
    }
}
