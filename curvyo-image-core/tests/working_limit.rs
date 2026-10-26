//! The working limit and hostile mutations (`specs/0058-bitmap-import` criteria 18 to 20).

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::path::PathBuf;

use curvyo_image_core::{ImageError, decode_working, probe};
use image::ImageEncoder;
use proptest::prelude::*;

fn png_of(width: u32, height: u32, pixel: [u8; 3]) -> Vec<u8> {
    let pixels = pixel.repeat((width * height) as usize);
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new_with_quality(
        &mut out,
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::NoFilter,
    )
    .write_image(&pixels, width, height, image::ExtendedColorType::Rgb8)
    .unwrap();
    out
}

#[test]
fn a_picture_over_the_working_limit_is_reduced_with_the_box_filter() {
    let bytes = png_of(5000, 4000, [10, 200, 30]);
    let image = decode_working(&bytes).unwrap();
    assert_eq!((image.width, image.height), (4472, 3577));
    assert_eq!(image.original_size, (5000, 4000));
    assert!(image.is_reduced());
    assert_eq!(image.pixel(2000, 1500), Some([10, 200, 30, 255]));
}

#[test]
fn a_header_over_50_megapixels_is_refused_with_its_numbers() {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/images/png_bomb_60000x60000.png"),
    )
    .unwrap();
    assert_eq!(
        probe(&bytes),
        Err(ImageError::TooManyPixels {
            width: 60_000,
            height: 60_000
        })
    );
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/images")
            .join(name),
    )
    .unwrap()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// Mutated fixtures decode or fail; they never panic (a panic aborts the wasm module).
    #[test]
    fn mutated_files_never_panic(
        which in 0_usize..4,
        edits in proptest::collection::vec((any::<usize>(), any::<u8>()), 1..8),
        cut in any::<usize>(),
    ) {
        let name = ["png_rgba_8.png", "jpeg_progressive.jpg", "webp_lossy.webp",
                    "webp_exif_orientation_6.webp"][which];
        let mut bytes = fixture(name);
        for (at, value) in edits {
            let at = at % bytes.len();
            bytes[at] = value;
        }
        bytes.truncate(cut % (bytes.len() + 1));
        let _ = decode_working(&bytes);
    }
}
