//! The adjustment goldens of `specs/0135-image-optimisation` criterion 54: three small golden
//! pictures in `tests/fixtures/adjust/` (a photo, a flat graphic, a pen drawing) and the expected
//! adjusted image of each slice-1 step as a PNG. The pixels are compared, not the PNG bytes.
//! The goldens are embedded with `include_bytes!` so the same comparison runs natively and in
//! Node on `wasm32-unknown-unknown` (ADR 0073 §1, the CI wasm golden job):
//!
//! ```text
//! cargo test --target wasm32-unknown-unknown -p curvyo-image-core --test adjust_goldens
//! ```
//!
//! `CURVYO_UPDATE_GOLDEN=1` rewrites the files natively (a new golden needs an empty file first,
//! `include_bytes!` reads it at compile time); review the diff.

#![allow(clippy::unwrap_used, clippy::missing_panics_doc, missing_docs)]

mod adjust_pictures;

use curvyo_image_core::{
    Adjusted, GreyPlane, PixelStep, SubPx, WorkingImage, decode_working, run_steps,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

macro_rules! goldens {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_bytes!(concat!("fixtures/adjust/", $name, ".png")))),*]
    };
}

/// Every stored PNG, embedded at compile time.
const GOLDENS: &[(&str, &[u8])] = goldens![
    "photo",
    "graphic",
    "pen",
    "levels_stretch",
    "levels_gamma",
    "sharpen_unsharp",
    "local_contrast_clahe",
    "denoise_bilateral",
    "contours_dog",
    "curves_s",
    "channel_mix_red_filter",
    "crop_20pct",
    "resize_half",
];

/// Writes `image` as the golden `name` when `CURVYO_UPDATE_GOLDEN` is set (native only).
#[cfg(not(target_arch = "wasm32"))]
fn update(name: &str, image: &WorkingImage) -> bool {
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_none() {
        return false;
    }
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/adjust")
        .join(format!("{name}.png"));
    let bytes = curvyo_image_core::encode_png(image).unwrap();
    if std::fs::read(&path).ok().as_ref() != Some(&bytes) {
        let temp = path.with_extension(format!("png.{}", std::process::id()));
        std::fs::write(&temp, bytes).unwrap();
        std::fs::rename(&temp, &path).unwrap();
    }
    true
}

#[cfg(target_arch = "wasm32")]
fn update(_: &str, _: &WorkingImage) -> bool {
    false
}

/// `image` against the stored PNG `name` (written instead when updating).
fn check(name: &str, image: &WorkingImage) {
    if update(name, image) {
        return;
    }
    let stored = GOLDENS
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap_or_else(|| panic!("missing golden {name}.png"))
        .1;
    let stored = decode_working(stored).unwrap();
    assert_eq!(
        (stored.width, stored.height),
        (image.width, image.height),
        "{name}"
    );
    assert!(
        stored.pixels == image.pixels,
        "{name}.png differs from its golden"
    );
}

/// A grey plane as an opaque RGBA picture.
fn grey_image(plane: &GreyPlane) -> WorkingImage {
    WorkingImage {
        width: plane.width,
        height: plane.height,
        pixels: plane.grey.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        original_size: (plane.width, plane.height),
        truncated: false,
        animated: false,
    }
}

/// The golden picture `name`, checked against its stored PNG.
fn picture(name: &str) -> WorkingImage {
    let image = adjust_pictures::by_name(name);
    check(name, &image);
    image
}

fn golden(name: &str, input: &str, steps: &[PixelStep]) {
    let out = match run_steps(&picture(input), steps) {
        Adjusted::Unchanged => panic!("{name}: no step ran"),
        Adjusted::Rgba(image) => image,
        Adjusted::Grey(plane) => grey_image(&plane),
    };
    check(name, &out);
}

#[test]
fn levels_stretch() {
    golden(
        "levels_stretch",
        "photo",
        &[PixelStep::Levels {
            black_pct: 10,
            white_pct: 90,
            gamma_centi: 100,
        }],
    );
}

#[test]
fn levels_gamma() {
    golden(
        "levels_gamma",
        "photo",
        &[PixelStep::Levels {
            black_pct: 0,
            white_pct: 100,
            gamma_centi: 150,
        }],
    );
}

#[test]
fn sharpen_unsharp() {
    golden(
        "sharpen_unsharp",
        "photo",
        &[PixelStep::Sharpen {
            radius: SubPx(24),
            amount_pct: 150,
            threshold_pct: 0,
        }],
    );
}

#[test]
fn local_contrast_clahe() {
    golden(
        "local_contrast_clahe",
        "photo",
        &[PixelStep::LocalContrast {
            radius: SubPx(128),
            amount_pct: 80,
        }],
    );
}

#[test]
fn denoise_bilateral() {
    golden(
        "denoise_bilateral",
        "photo",
        &[PixelStep::Denoise {
            strength_pct: 60,
            detail_pct: 50,
        }],
    );
}

#[test]
fn contours_dog() {
    golden(
        "contours_dog",
        "pen",
        &[PixelStep::Contours {
            sigma: SubPx(16),
            sensitivity_pct: 60,
        }],
    );
}

#[test]
fn curves_s() {
    golden(
        "curves_s",
        "photo",
        &[PixelStep::Curves {
            points: [
                (0, 0),
                (25, 15),
                (75, 85),
                (100, 100),
                (0, 0),
                (0, 0),
                (0, 0),
                (0, 0),
            ],
            count: 4,
        }],
    );
}

#[test]
fn channel_mix_red_filter() {
    golden(
        "channel_mix_red_filter",
        "photo",
        &[PixelStep::ChannelMix {
            weights: [100, 0, 0],
        }],
    );
}

#[test]
fn crop_20pct() {
    golden(
        "crop_20pct",
        "photo",
        &[PixelStep::Crop {
            pct: [20, 20, 60, 60],
        }],
    );
}

#[test]
fn resize_half() {
    let photo = picture("photo");
    golden(
        "resize_half",
        "photo",
        &[PixelStep::Resize {
            width_px: photo.width / 2,
        }],
    );
}

#[test]
fn the_flat_graphic_is_a_stored_picture() {
    picture("graphic");
}

#[test]
fn two_runs_give_the_same_bytes() {
    let steps = [
        PixelStep::Denoise {
            strength_pct: 40,
            detail_pct: 60,
        },
        PixelStep::Levels {
            black_pct: 2,
            white_pct: 97,
            gamma_centi: 112,
        },
        PixelStep::LocalContrast {
            radius: SubPx(200),
            amount_pct: 60,
        },
        PixelStep::Sharpen {
            radius: SubPx(16),
            amount_pct: 120,
            threshold_pct: 0,
        },
    ];
    let image = picture("photo");
    assert_eq!(run_steps(&image, &steps), run_steps(&image, &steps));
}
