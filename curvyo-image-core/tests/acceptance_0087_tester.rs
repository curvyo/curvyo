//! Tester cases for `encode_png` (`specs/0087-image-clipboard-and-drop` criteria 11 and 12): every
//! decodable fixture round-trips pixel-exact (EXIF orientation included, upright), alpha under
//! straight (non-premultiplied) storage survives, the sizes at the limits encode, output is
//! deterministic and a decode of the output does not depend on the source format.

#![allow(clippy::unwrap_used, clippy::cast_possible_truncation, missing_docs)]

use std::path::PathBuf;

use curvyo_image_core::{
    MAX_WORKING_PIXELS, MAX_WORKING_SIDE, WorkingImage, decode_working, encode_png,
};
use proptest::prelude::*;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/images")
}

fn solid(width: u32, height: u32, pixel: [u8; 4]) -> WorkingImage {
    WorkingImage {
        width,
        height,
        pixels: pixel
            .iter()
            .copied()
            .cycle()
            .take(width as usize * height as usize * 4)
            .collect(),
        original_size: (width, height),
        truncated: false,
        animated: false,
    }
}

#[test]
fn every_decodable_fixture_round_trips_pixel_exact_and_upright() {
    let mut round_tripped = 0;
    for entry in std::fs::read_dir(fixtures()).unwrap() {
        let path = entry.unwrap().path();
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(image) = decode_working(&bytes) else {
            continue;
        };
        let png = encode_png(&image).unwrap_or_else(|e| panic!("{path:?}: {e:?}"));
        let back = decode_working(&png).unwrap();
        assert_eq!(
            (back.width, back.height),
            (image.width, image.height),
            "{path:?}"
        );
        assert_eq!(back.pixels, image.pixels, "{path:?}");
        round_tripped += 1;
    }
    assert!(round_tripped >= 25, "only {round_tripped} fixtures decoded");
}

#[test]
fn an_exif_rotated_jpeg_copies_upright_with_the_marker_where_the_file_shows_it() {
    // Orientation 6: the file stores 40 x 20 and shows 20 x 40 with the red marker top-right.
    let bytes = std::fs::read(fixtures().join("jpeg_exif_orientation_6.jpg")).unwrap();
    let image = decode_working(&bytes).unwrap();
    assert_eq!((image.width, image.height), (20, 40));
    let back = decode_working(&encode_png(&image).unwrap()).unwrap();
    assert_eq!((back.width, back.height), (20, 40));
    let [r, g, b, a] = back.pixel(16, 3).unwrap();
    assert!(r > 230 && g < 40 && b < 40 && a == 255, "{r},{g},{b},{a}");
}

#[test]
fn a_fully_transparent_pixel_keeps_its_colour_channels() {
    // Straight alpha: the stored pixels are exact, no premultiplication on the way out.
    let image = solid(2, 2, [200, 100, 50, 0]);
    let back = decode_working(&encode_png(&image).unwrap()).unwrap();
    assert_eq!(back.pixels, image.pixels);
    let faint = solid(3, 1, [10, 200, 90, 1]);
    let back = decode_working(&encode_png(&faint).unwrap()).unwrap();
    assert_eq!(back.pixels, faint.pixels);
}

#[test]
fn the_smallest_and_the_largest_allowed_pictures_encode() {
    for (w, h) in [
        (1, 1),
        (MAX_WORKING_SIDE, 1),
        (1, MAX_WORKING_SIDE),
        (4000, 4000),
    ] {
        assert!(u64::from(w) * u64::from(h) <= MAX_WORKING_PIXELS);
        let image = solid(w, h, [1, 2, 3, 255]);
        let png = encode_png(&image).unwrap_or_else(|e| panic!("{w}x{h}: {e:?}"));
        let back = decode_working(&png).unwrap();
        assert_eq!((back.width, back.height), (w, h));
        assert_eq!(back.pixel(w - 1, h - 1), Some([1, 2, 3, 255]));
    }
}

#[test]
fn the_same_picture_encodes_to_the_same_bytes() {
    let image = solid(17, 9, [9, 8, 7, 200]);
    assert_eq!(encode_png(&image).unwrap(), encode_png(&image).unwrap());
}

#[test]
fn the_signature_and_the_ihdr_state_the_stored_size() {
    let png = encode_png(&solid(7, 3, [0, 0, 0, 255])).unwrap();
    assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
    assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 7);
    assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 3);
}

proptest! {
    #[test]
    fn any_small_rgba_picture_round_trips(
        w in 1u32..24,
        h in 1u32..24,
        seed in proptest::collection::vec(any::<u8>(), 4..64),
    ) {
        let pixels: Vec<u8> = seed.iter().copied().cycle().take(w as usize * h as usize * 4).collect();
        let image = WorkingImage {
            width: w, height: h, pixels, original_size: (w, h), truncated: false, animated: false,
        };
        let back = decode_working(&encode_png(&image).unwrap()).unwrap();
        prop_assert_eq!((back.width, back.height), (w, h));
        prop_assert_eq!(back.pixels, image.pixels);
    }
}
