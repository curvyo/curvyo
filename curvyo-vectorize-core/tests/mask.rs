//! The mask the tracer turns into outlines (`specs/0089-trace-quality-and-colour` criterion 21):
//! the Mask view shows exactly it, so it must be the mask after Specks and Drop border shapes, at
//! the traced resolution, pixel-equal with what the outlines enclose.

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
    clippy::many_single_char_names
)]

mod common;

use std::ops::ControlFlow;

use common::{Picture, at_96_dpi, params, run};
use curvyo_document_core::Affine;
use curvyo_vectorize_core::{Mask, PixelLength, TraceParams, Upscale, mask};
use kurbo::Shape;

fn mask_of(picture: &Picture, map: Affine, p: &TraceParams) -> Mask {
    mask(picture.raster(), map, p, &mut |_| ControlFlow::Continue(())).unwrap()
}

/// 60 x 40 px on white: a 20 x 16 black block with a 6 x 6 hole and a 1 px speck hole, a 2 x 2
/// speck, and a bar touching the left border.
fn scene() -> Picture {
    let (w, h) = (60_u32, 40_u32);
    let mut levels = vec![255_u8; (w * h) as usize];
    let mut ink = |x0: u32, y0: u32, x1: u32, y1: u32, v: u8| {
        for y in y0..y1 {
            for x in x0..x1 {
                levels[(y * w + x) as usize] = v;
            }
        }
    };
    ink(20, 10, 40, 26, 0);
    ink(26, 14, 32, 20, 255);
    ink(36, 22, 37, 23, 255);
    ink(50, 5, 52, 7, 0);
    ink(0, 30, 12, 34, 0);
    Picture::grey(w, h, &levels)
}

/// Content pixels of `mask` as `(x, y)`.
fn content(mask: &Mask) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for y in 0..mask.height {
        for x in 0..mask.width {
            let bit = mask.bits[(y * mask.width + x) as usize];
            assert!(bit == 0 || bit == 255, "bits are 0 or 255");
            if bit == 255 {
                out.push((x, y));
            }
        }
    }
    out
}

/// Whether the centre of every mask pixel lies inside the outlines exactly when it is content
/// (the trace at Smooth 0 and the smallest Tolerance follows the pixel border within 0.1 px).
fn assert_pixel_equal(picture: &Picture, p: &TraceParams) {
    let map = at_96_dpi();
    let m = mask_of(picture, map, p);
    let result = run(picture, map, p);
    let to_mm = kurbo::Affine::new(m.pixel_map.to_array());
    let mut differ = Vec::new();
    for y in 0..m.height {
        for x in 0..m.width {
            let centre = to_mm * kurbo::Point::new(f64::from(x) + 0.5, f64::from(y) + 0.5);
            let winding: i32 = result.outlines.iter().map(|o| o.winding(centre)).sum();
            let inside = winding != 0;
            let bit = m.bits[(y * m.width + x) as usize] == 255;
            if inside != bit {
                differ.push((x, y));
            }
        }
    }
    assert!(differ.is_empty(), "mask and outlines differ at {differ:?}");
}

fn exact(specks_mm: f64) -> TraceParams {
    TraceParams {
        tolerance: PixelLength::from_px(0.1),
        upscale: Upscale::Off,
        ..params(128, 0, specks_mm)
    }
}

/// Criterion 21: the mask is what the outlines enclose, pixel for pixel.
#[test]
fn the_mask_is_what_the_outlines_enclose() {
    let picture = scene();
    assert_pixel_equal(&picture, &exact(0.0));
    assert_pixel_equal(&picture, &exact(0.8));
    assert_pixel_equal(
        &picture,
        &TraceParams {
            ignore_background: true,
            ..exact(0.8)
        },
    );
}

/// Criterion 21: Specks are applied (the 2 x 2 dot goes, the 1 px hole is filled), Drop border
/// shapes too; at 96 dpi a pixel is 0.26 mm, so 0.8 mm drops anything under about 9 px.
#[test]
fn the_mask_is_after_specks_and_border_shapes() {
    let picture = scene();
    let all = content(&mask_of(&picture, at_96_dpi(), &exact(0.0)));
    assert!(all.contains(&(50, 5)), "the dot is content without Specks");
    assert!(
        !all.contains(&(36, 22)),
        "the small hole is a hole without Specks"
    );
    let cleaned = content(&mask_of(&picture, at_96_dpi(), &exact(0.8)));
    assert!(!cleaned.contains(&(50, 5)), "the dot is a speck");
    assert!(cleaned.contains(&(36, 22)), "the small hole is filled");
    assert!(cleaned.contains(&(0, 30)), "the border bar stays");
    let dropped = content(&mask_of(
        &picture,
        at_96_dpi(),
        &TraceParams {
            ignore_background: true,
            ..exact(0.8)
        },
    ));
    assert!(
        !dropped.contains(&(0, 30)),
        "Drop border shapes takes the bar"
    );
    assert!(dropped.contains(&(20, 10)), "the block stays");
}

/// Criterion 21: at the traced resolution: Upscale traces a 2x copy, so the mask is twice the
/// size and its pixel map half the picture's.
#[test]
fn the_mask_is_at_the_traced_resolution() {
    let picture = scene();
    let p = TraceParams {
        upscale: Upscale::Auto,
        ..exact(0.0)
    };
    let m = mask_of(&picture, at_96_dpi(), &p);
    assert_eq!((m.width, m.height), (120, 80));
    let expected = Affine::scale(0.5, 0.5).compose(at_96_dpi());
    assert_eq!(m.pixel_map, expected);
    assert_eq!(m.bits.len(), 120 * 80);
    assert_pixel_equal(&picture, &p);
    let flat = mask_of(&picture, at_96_dpi(), &exact(0.0));
    assert_eq!((flat.width, flat.height), (60, 40));
    assert_eq!(flat.pixel_map, at_96_dpi());
}

/// Criterion 21 with 15: a transparent pixel is never content in the mask, also with Invert and
/// a black stored colour.
#[test]
fn transparent_pixels_are_never_content() {
    let mut picture = scene();
    for px in picture.rgba.as_chunks_mut::<4>().0.iter_mut().take(60 * 20) {
        px.copy_from_slice(&[0, 0, 0, 0]);
    }
    for invert in [false, true] {
        let m = mask_of(
            &picture,
            at_96_dpi(),
            &TraceParams {
                invert,
                ..exact(0.0)
            },
        );
        assert!(
            m.bits[..60 * 20].iter().all(|&b| b == 0),
            "invert {invert}: the transparent half is empty"
        );
    }
}

/// The mask fails as the trace does: bytes that do not match the size, a singular map.
#[test]
fn the_mask_refuses_what_the_trace_refuses() {
    let picture = scene();
    let short = Picture {
        rgba: picture.rgba[..100].to_vec(),
        ..picture.clone()
    };
    let p = params(128, 80, 0.5);
    assert!(
        mask(short.raster(), at_96_dpi(), &p, &mut |_| {
            ControlFlow::Continue(())
        })
        .is_err()
    );
    let singular = Affine::scale(0.0, 1.0);
    assert!(
        mask(picture.raster(), singular, &p, &mut |_| {
            ControlFlow::Continue(())
        })
        .is_err()
    );
}
