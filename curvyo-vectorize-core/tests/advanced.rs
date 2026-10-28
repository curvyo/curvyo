//! The Advanced settings of the tracer (`specs/0025-raster-trace` criteria 41 to 44): Denoise,
//! Upscale, Invert and Ignore background on small synthetic pictures.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::type_complexity,
    clippy::needless_pass_by_value,
    clippy::similar_names
)]

mod common;

use common::quality::{Noise, measure, small_text};
use common::{Picture, area, at_96_dpi, params, render, run};
use curvyo_document_core::Affine;
use curvyo_vectorize_core::{Denoise, TraceParams, Upscale, auto_threshold};
use kurbo::Shape;

/// A black rectangle at x 30 to 70, y 20 to 40 on a white 100 x 60 px picture.
fn logo() -> Picture {
    render(100, 60, 1, |x, y| {
        (30.0..70.0).contains(&x) && (20.0..40.0).contains(&y)
    })
}

fn box_px(path: &kurbo::BezPath) -> kurbo::Rect {
    (kurbo::Affine::scale(1.0 / common::MM_PER_PX_96) * path.clone()).bounding_box()
}

/// Criterion 44: with Invert the white background is the shape and the logo its hole.
#[test]
fn invert_traces_the_light_parts() {
    let p = TraceParams {
        invert: true,
        ..params(128, 80, 0.5)
    };
    let result = run(&logo(), at_96_dpi(), &p);
    assert_eq!(result.counts.paths, 2, "{:?}", result.counts);
    let (outer, hole) = (&result.outlines[0], &result.outlines[1]);
    let outer_box = box_px(outer);
    assert!(
        outer_box.x0.abs() < 0.5 && (outer_box.x1 - 100.0).abs() < 0.5,
        "{outer_box:?}"
    );
    assert!(
        outer_box.y0.abs() < 0.5 && (outer_box.y1 - 60.0).abs() < 0.5,
        "{outer_box:?}"
    );
    let hole_box = box_px(hole);
    assert!(
        (hole_box.x0 - 30.0).abs() < 0.5 && (hole_box.x1 - 70.0).abs() < 0.5,
        "{hole_box:?}"
    );
    assert!(
        (hole_box.y0 - 20.0).abs() < 0.5 && (hole_box.y1 - 40.0).abs() < 0.5,
        "{hole_box:?}"
    );
    assert_ne!(area(outer).signum(), area(hole).signum());
}

/// Criteria 41 and 44: Ignore background drops the foreground regions touching the border.
#[test]
fn ignore_background_drops_the_regions_at_the_border() {
    // A black frame along the border and the logo inside it.
    let framed = render(100, 60, 1, |x, y| {
        let frame = !(4.0..96.0).contains(&x) || !(4.0..56.0).contains(&y);
        frame || ((30.0..70.0).contains(&x) && (20.0..40.0).contains(&y))
    });
    let plain = run(&framed, at_96_dpi(), &params(128, 80, 0.5));
    assert_eq!(plain.counts.paths, 3, "frame, its hole, the logo");
    let p = TraceParams {
        ignore_background: true,
        ..params(128, 80, 0.5)
    };
    let result = run(&framed, at_96_dpi(), &p);
    assert_eq!(result.counts.paths, 1);
    let logo_box = box_px(&result.outlines[0]);
    assert!((logo_box.x0 - 30.0).abs() < 0.5 && (logo_box.y1 - 40.0).abs() < 0.5);
    // With Invert the white background touches the border and goes; the white inside the
    // frame stays, with the logo as its hole.
    let p = TraceParams { invert: true, ..p };
    let inverted = run(&framed, at_96_dpi(), &p);
    assert_eq!(inverted.counts.paths, 2);
    let inner = box_px(&inverted.outlines[0]);
    assert!(
        (inner.x0 - 4.0).abs() < 0.5 && (inner.x1 - 96.0).abs() < 0.5,
        "{inner:?}"
    );
    // Background regions are not counted as specks.
    assert_eq!(result.counts.specks_removed, plain.counts.specks_removed);
}

/// The distance from `p` to the rectangle's border.
fn to_rect_border(p: kurbo::Point, r: kurbo::Rect) -> f64 {
    let dx = (p.x - r.x0).abs().min((p.x - r.x1).abs());
    let dy = (p.y - r.y0).abs().min((p.y - r.y1).abs());
    let inside_x = (r.x0..=r.x1).contains(&p.x);
    let inside_y = (r.y0..=r.y1).contains(&p.y);
    match (inside_x, inside_y) {
        (true, true) => dx.min(dy),
        (true, false) => dy,
        (false, true) => dx,
        (false, false) => dx.hypot(dy),
    }
}

/// Criterion 42: a JPEG-like logo with ringing and noise of 20 grey levels; Denoise Low leaves
/// no outline outside the logo and keeps the border within 1 px.
#[test]
fn denoise_low_removes_ringing_and_noise() {
    let truth = kurbo::Rect::new(50.0, 30.0, 150.0, 90.0);
    let clean = render(200, 120, 4, |x, y| truth.contains(kurbo::Point::new(x, y)));
    let mut noise = Noise::new(99);
    let levels: Vec<u8> = clean
        .levels()
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let (x, y) = ((i % 200) as f64 + 0.5, (i / 200) as f64 + 0.5);
            let edge = to_rect_border(kurbo::Point::new(x, y), truth);
            // Ringing: a decaying ripple of 20 grey levels beside the edges, plus noise.
            let ripple = if edge < 6.0 {
                20.0 * (edge * 2.0).cos() * (1.0 - edge / 6.0)
            } else {
                0.0
            };
            let speckle = if noise.next() > 0.995 {
                -140.0
            } else {
                20.0 * noise.next()
            };
            (f64::from(v) + ripple + speckle).round().clamp(0.0, 255.0) as u8
        })
        .collect();
    let jpeg = Picture::grey(200, 120, &levels);
    let threshold = auto_threshold(jpeg.raster());
    let noisy = run(&jpeg, Affine::IDENTITY, &params(threshold, 80, 0.0));
    assert!(
        noisy.counts.paths > 1,
        "the fixture has specks without Denoise"
    );
    let p = TraceParams {
        denoise: Denoise::Low,
        ..params(threshold, 80, 0.0)
    };
    let result = run(&jpeg, Affine::IDENTITY, &p);
    assert_eq!(result.counts.paths, 1, "{:?}", result.counts);
    let mut max = 0.0_f64;
    for seg in result.outlines[0].segments() {
        for i in 0..50 {
            let point = kurbo::ParamCurve::eval(&seg, f64::from(i) / 50.0);
            max = max.max(to_rect_border(point, truth));
        }
    }
    assert!(max <= 1.0, "max deviation {max} px");
}

/// Criterion 42: Upscale Auto traces the 120 x 60 px text from a 2x copy, and its deviation to
/// the 4x reference is lower than without Upscale.
#[test]
fn upscale_auto_follows_the_4x_reference_closer() {
    let small = small_text(1);
    let reference = small_text(4);
    let threshold = 128;
    let mask: Vec<bool> = reference.levels().iter().map(|&v| v < threshold).collect();
    let deviation = |upscale: Upscale| {
        let p = TraceParams {
            upscale,
            ..params(threshold, 80, 0.0)
        };
        let result = run(&small, Affine::IDENTITY, &p);
        let in_4x: Vec<kurbo::BezPath> = result
            .outlines
            .iter()
            .map(|o| kurbo::Affine::scale(4.0) * o.clone())
            .collect();
        measure(&in_4x, &mask, 480, 240, 0.25).mean_deviation_px
    };
    let (off, auto) = (deviation(Upscale::Off), deviation(Upscale::Auto));
    assert!(
        auto < off,
        "Upscale Auto {auto:.3} px against Off {off:.3} px (4x pixels)"
    );
}

/// Upscale keeps the result in millimetres and the Specks size in mm².
#[test]
fn upscale_keeps_millimetres_and_specks() {
    let off = run(&logo(), at_96_dpi(), &params(128, 80, 0.5));
    let p = TraceParams {
        upscale: Upscale::Auto,
        ..params(128, 80, 0.5)
    };
    let auto = run(&logo(), at_96_dpi(), &p);
    assert_eq!(auto.counts.paths, 1);
    let (a, b) = (
        off.outlines[0].bounding_box(),
        auto.outlines[0].bounding_box(),
    );
    assert!(
        (a.x0 - b.x0).abs() < 0.07 && (a.y1 - b.y1).abs() < 0.07,
        "{a:?} {b:?}"
    );
    assert_eq!(auto.foreground_pixels, off.foreground_pixels);
    // A 1 px speck is 0.07 mm² at 96 dpi: removed at 0.5 mm in both.
    let mut levels = logo().levels();
    levels[5 * 100 + 5] = 0;
    let specked = Picture::grey(100, 60, &levels);
    assert_eq!(run(&specked, at_96_dpi(), &p).counts.paths, 1);
}

/// A noisy photograph has shapes that touch only diagonally across another region's pocket;
/// visioncortex got stuck on them (a panic in slice 1). Every preset traces it.
#[test]
fn a_noisy_photograph_traces_with_every_preset() {
    let photo = common::quality::photograph();
    let threshold = auto_threshold(photo.raster());
    for preset in curvyo_vectorize_core::PRESETS {
        let p = preset.apply(TraceParams::with_threshold(threshold));
        let result = run(&photo, at_96_dpi(), &p);
        assert!(result.counts.paths > 0, "{}", preset.key);
    }
}
