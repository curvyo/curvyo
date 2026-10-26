//! The decode budgets of `specs/0058-bitmap-import` criterion 46, native and in release
//! (`cargo test --release -p curvyo-image-core --test budgets -- --ignored`): a regression guard,
//! not what the maker sees in the webview.

#![allow(clippy::unwrap_used, clippy::cast_possible_truncation, missing_docs)]

use std::time::{Duration, Instant};

use image::ImageEncoder;

fn picture(width: u32, height: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((width * height * 3) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.extend([(x % 251) as u8, (y % 241) as u8, ((x ^ y) % 239) as u8]);
        }
    }
    pixels
}

fn jpeg(width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85)
        .write_image(
            &picture(width, height),
            width,
            height,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    out
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(
            &picture(width, height),
            width,
            height,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    out
}

fn timed(name: &str, bytes: &[u8], budget: Duration) {
    let started = Instant::now();
    let image = curvyo_image_core::decode_working(bytes).unwrap();
    let took = started.elapsed();
    println!("{name}: {took:?} -> {}x{}", image.width, image.height);
    assert!(took < budget, "{name} took {took:?}, budget {budget:?}");
}

#[test]
#[ignore = "budget benchmark, run in release"]
fn criterion_46_decode_budgets() {
    timed(
        "12 Mpx JPEG",
        &jpeg(4000, 3000),
        Duration::from_millis(2000),
    );
    timed("16 Mpx PNG", &png(4000, 4000), Duration::from_millis(2500));
    timed(
        "50 Mpx JPEG",
        &jpeg(8660, 5770),
        Duration::from_millis(8000),
    );
}
