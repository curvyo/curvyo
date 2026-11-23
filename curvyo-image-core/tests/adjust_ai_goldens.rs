//! The stored-result goldens of `specs/0135-image-optimisation` criterion 54: the raw outputs of
//! the three AI steps are fixed stand-in PNGs (`mask_stored`, `upscale_stored_2x`,
//! `denoise_ai_stored`, made from integers here, never by a model), and the pictures after the
//! integer application of ADR 0077 §3 are goldens too. The pixels are compared, not the PNG
//! bytes; the same comparison runs natively and in Node on `wasm32-unknown-unknown`.
//! `CURVYO_UPDATE_GOLDEN=1` rewrites the files natively.

#![allow(clippy::unwrap_used, clippy::missing_panics_doc, missing_docs)]

#[allow(dead_code)]
mod adjust_pictures;

use std::sync::Arc;

use curvyo_image_core::{
    Adjusted, PixelStep, StoredPixels, WorkingImage, decode_working, encode_png, run_steps_with,
    step_input,
};
use sha2::{Digest, Sha256};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

macro_rules! goldens {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_bytes!(concat!("fixtures/adjust/", $name, ".png")))),*]
    };
}

const GOLDENS: &[(&str, &[u8])] = goldens![
    "mask_stored",
    "upscale_stored_2x",
    "denoise_ai_stored",
    "mask_applied",
    "upscale_applied_2x",
    "denoise_ai_blend",
];

#[cfg(not(target_arch = "wasm32"))]
fn update(name: &str, image: &WorkingImage) -> bool {
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_none() {
        return false;
    }
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/adjust")
        .join(format!("{name}.png"));
    std::fs::write(&path, encode_png(image).unwrap()).unwrap();
    true
}

#[cfg(target_arch = "wasm32")]
fn update(_: &str, _: &WorkingImage) -> bool {
    false
}

fn stored(name: &str) -> WorkingImage {
    let bytes = GOLDENS.iter().find(|(n, _)| *n == name).unwrap().1;
    decode_working(bytes).unwrap()
}

fn check(name: &str, image: &WorkingImage) {
    if update(name, image) {
        return;
    }
    let golden = stored(name);
    assert_eq!(
        (golden.width, golden.height),
        (image.width, image.height),
        "{name}"
    );
    assert!(
        golden.pixels == image.pixels,
        "{name}.png differs from its golden"
    );
}

fn grey(width: u32, height: u32, g: impl Fn(u32, u32) -> u8) -> WorkingImage {
    WorkingImage {
        width,
        height,
        pixels: (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .flat_map(|(x, y)| {
                let v = g(x, y);
                [v, v, v, 255]
            })
            .collect(),
        original_size: (width, height),
        truncated: false,
        animated: false,
    }
}

/// The stand-in raw outputs: a soft disc mask at the model's 20 x 20 (a small stand-in for
/// 320 x 320), the photo at twice its pixels by repetition, and the photo smoothed by a 3 x 3 box.
fn stand_ins() -> [(&'static str, WorkingImage); 3] {
    let photo = adjust_pictures::photo();
    let mask = grey(20, 20, |x, y| {
        let (dx, dy) = (i64::from(x) * 2 - 19, i64::from(y) * 2 - 19);
        let d = dx * dx + dy * dy;
        u8::try_from((400 - d).clamp(0, 255)).unwrap()
    });
    let s = photo.width;
    let up = WorkingImage {
        width: 2 * s,
        height: 2 * s,
        pixels: (0..2 * s)
            .flat_map(|y| (0..2 * s).map(move |x| (x / 2, y / 2)))
            .flat_map(|(x, y)| photo.pixel(x, y).unwrap())
            .collect(),
        original_size: (2 * s, 2 * s),
        truncated: false,
        animated: false,
    };
    let boxed = WorkingImage {
        pixels: (0..s)
            .flat_map(|y| (0..s).map(move |x| (x, y)))
            .flat_map(|(x, y)| {
                let mut sum = [0_u32; 3];
                for (dx, dy) in (0..9).map(|i| (i % 3, i / 3)) {
                    let p = photo
                        .pixel(
                            (x + dx).saturating_sub(1).min(s - 1),
                            (y + dy).saturating_sub(1).min(s - 1),
                        )
                        .unwrap();
                    for c in 0..3 {
                        sum[c] += u32::from(p[c]);
                    }
                }
                let m = sum.map(|v| u8::try_from((v + 4) / 9).unwrap());
                [m[0], m[1], m[2], 255]
            })
            .collect(),
        ..photo.clone()
    };
    [
        ("mask_stored", mask),
        ("upscale_stored_2x", up),
        ("denoise_ai_stored", boxed),
    ]
}

fn hash(image: &WorkingImage) -> [u8; 32] {
    Sha256::digest(encode_png(image).unwrap()).into()
}

fn rgba(adjusted: Adjusted) -> WorkingImage {
    match adjusted {
        Adjusted::Rgba(image) => image,
        other => panic!("expected colour, got {other:?}"),
    }
}

#[test]
fn stored_results_apply_in_integers() {
    let mut results = StoredPixels::default();
    let mut hashes = Vec::new();
    for (name, image) in stand_ins() {
        check(name, &image);
        hashes.push(hash(&image));
        results.insert(hash(&image), Arc::new(image));
    }
    let photo = adjust_pictures::photo();
    let mask = PixelStep::Mask {
        result: hashes[0],
        threshold_pct: 50,
        softness_pct: 5,
    };
    check(
        "mask_applied",
        &rgba(run_steps_with(&photo, &[mask], &results)),
    );
    let up = PixelStep::Upscale { result: hashes[1] };
    let upscaled = rgba(run_steps_with(&photo, &[up], &results));
    assert_eq!((upscaled.width, upscaled.height), (128, 128));
    check("upscale_applied_2x", &upscaled);
    let denoise = PixelStep::AiDenoise {
        result: hashes[2],
        strength_pct: 50,
    };
    check(
        "denoise_ai_blend",
        &rgba(run_steps_with(&photo, &[denoise], &results)),
    );
    // Criterion 3: the mask is computed from the picture after AI denoise and applied last.
    let input = step_input(&photo, &[denoise], &results);
    assert_eq!(
        input,
        rgba(run_steps_with(&photo, &[denoise], &results)),
        "the input of a later step is the picture after the earlier ones"
    );
}

#[test]
fn a_result_that_is_not_held_changes_nothing() {
    let photo = adjust_pictures::photo();
    let none = StoredPixels::default();
    let out = rgba(run_steps_with(
        &photo,
        &[PixelStep::Upscale { result: [7; 32] }],
        &none,
    ));
    assert_eq!(out, curvyo_image_core::opaque_over_white(&photo));
}
