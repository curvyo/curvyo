//! The import dialog's thumbnail (`specs/0087-image-clipboard-and-drop` criteria 26 and 28).

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::path::PathBuf;

use curvyo_image_core::{ImageError, MAX_FILE_BYTES, thumbnail};
use image::ImageEncoder;

fn png_of(width: u32, height: u32) -> Vec<u8> {
    let pixels = [40_u8, 90, 200].repeat((width * height) as usize);
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

fn fixture(folder: &str, name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(folder)
        .join(name);
    std::fs::read(path).unwrap()
}

#[test]
fn a_3000_by_2000_png_gives_a_256_px_thumbnail_with_its_aspect() {
    let picture = thumbnail(&png_of(3000, 2000), 256).unwrap();
    assert_eq!(picture.width, 256);
    assert!(picture.height.abs_diff(171) <= 1, "{}", picture.height);
    assert_eq!(picture.original_size, (3000, 2000));
    assert_eq!(picture.pixel(10, 10), Some([40, 90, 200, 255]));
}

#[test]
fn a_small_picture_is_kept_and_a_tall_one_fits_on_its_height() {
    assert_eq!(
        (thumbnail(&png_of(100, 50), 256).unwrap().width),
        100,
        "never enlarged"
    );
    let tall = thumbnail(&png_of(500, 1000), 256).unwrap();
    assert_eq!((tall.width, tall.height), (128, 256));
}

#[test]
fn a_rotated_jpeg_is_upright() {
    let picture = thumbnail(&fixture("images", "jpeg_exif_orientation_6.jpg"), 256).unwrap();
    let (width, height) = picture.original_size;
    assert_eq!(
        (picture.width > picture.height),
        (width > height),
        "the thumbnail is the upright picture"
    );
}

#[test]
fn files_over_the_ceilings_are_refused_before_a_decode() {
    assert!(matches!(
        thumbnail(&fixture("images", "png_bomb_60000x60000.png"), 256),
        Err(ImageError::TooManyPixels { .. })
    ));
    let huge = vec![0_u8; usize::try_from(MAX_FILE_BYTES).unwrap() + 1];
    assert!(matches!(
        thumbnail(&huge, 256),
        Err(ImageError::FileTooLarge { .. })
    ));
}

#[test]
fn the_iccp_bomb_and_damaged_files_never_give_a_picture_out_of_bounds() {
    // gdk-pixbuf decoded this one past our ceilings (`adrs.md` decision 6); ours reads past the
    // chunk in bounded memory and gives its 1 x 1 picture or a refusal, never an unbounded buffer.
    if let Ok(picture) = thumbnail(&fixture("tester", "png_iccp_bomb.png"), 256) {
        assert!(picture.width <= 256 && picture.height <= 256);
    }
    for name in [
        "png_truncated.png",
        "text_renamed.png",
        "png_zero_width.png",
    ] {
        assert!(thumbnail(&fixture("images", name), 256).is_err(), "{name}");
    }
}
