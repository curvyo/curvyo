//! The PNG a copied image puts on the clipboard (`specs/0087-image-clipboard-and-drop` criteria 11,
//! 12 and 14, `adrs.md` decision 2): the working image's stored pixels, straight alpha, upright,
//! without metadata; nothing over the working limits; the same bytes as the golden file.
//! Regenerate the golden file with `CURVYO_UPDATE_GOLDEN=1`.

#![allow(clippy::unwrap_used, clippy::cast_possible_truncation, missing_docs)]

use std::path::PathBuf;

use curvyo_image_core::{
    ImageError, MAX_WORKING_SIDE, WorkingImage, decode_working, encode_png, probe,
};

fn picture(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> WorkingImage {
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            pixels.extend_from_slice(&pixel(x, y));
        }
    }
    WorkingImage {
        width,
        height,
        pixels,
        original_size: (width, height),
        truncated: false,
        animated: false,
    }
}

/// 40 x 20 px: a red 5 x 5 marker top-left, a half-transparent blue band, the rest a grey ramp.
fn marker() -> WorkingImage {
    picture(40, 20, |x, y| {
        if x < 5 && y < 5 {
            [255, 0, 0, 255]
        } else if y >= 15 {
            [0, 0, 255, 128]
        } else {
            let v = (x * 6) as u8;
            [v, v, v, 255]
        }
    })
}

fn chunk_types(png: &[u8]) -> Vec<String> {
    let mut at = 8;
    let mut types = Vec::new();
    while at + 8 <= png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        types.push(String::from_utf8_lossy(&png[at + 4..at + 8]).into_owned());
        at += 12 + len;
    }
    types
}

#[test]
fn the_png_decodes_to_the_same_pixels_translucent_ones_included() {
    let image = marker();
    let png = encode_png(&image).unwrap();
    let back = decode_working(&png).unwrap();
    assert_eq!((back.width, back.height), (40, 20));
    assert_eq!(back.pixels, image.pixels);
    assert_eq!(back.pixel(0, 0), Some([255, 0, 0, 255]));
    assert_eq!(back.pixel(39, 19), Some([0, 0, 255, 128]));
}

#[test]
fn the_png_has_no_metadata_chunks() {
    let png = encode_png(&marker()).unwrap();
    let types = chunk_types(&png);
    assert_eq!(types.first().map(String::as_str), Some("IHDR"));
    assert_eq!(types.last().map(String::as_str), Some("IEND"));
    assert!(
        types
            .iter()
            .all(|t| matches!(t.as_str(), "IHDR" | "IDAT" | "IEND")),
        "{types:?}"
    );
    // No density chunk: the PNG reads at the default 96 dpi.
    let info = probe(&png).unwrap();
    assert_eq!((info.stored_width, info.stored_height), (40, 20));
}

#[test]
fn the_png_is_the_golden_file() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/encoded/marker_40x20.png");
    let png = encode_png(&marker()).unwrap();
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &png).unwrap();
    }
    assert_eq!(
        png,
        std::fs::read(&path).unwrap(),
        "regenerate with CURVYO_UPDATE_GOLDEN=1"
    );
}

#[test]
fn a_picture_over_the_working_limits_is_refused() {
    let wide = WorkingImage {
        width: MAX_WORKING_SIDE + 1,
        height: 1,
        pixels: vec![0; (MAX_WORKING_SIDE as usize + 1) * 4],
        original_size: (MAX_WORKING_SIDE + 1, 1),
        truncated: false,
        animated: false,
    };
    assert_eq!(
        encode_png(&wide),
        Err(ImageError::TooManyPixels {
            width: MAX_WORKING_SIDE + 1,
            height: 1
        })
    );
    // 4001 x 4000 is under the side limit and over 16 megapixels; nothing is allocated for it.
    let many = WorkingImage {
        width: 4001,
        height: 4000,
        pixels: Vec::new(),
        original_size: (4001, 4000),
        truncated: false,
        animated: false,
    };
    assert_eq!(
        encode_png(&many),
        Err(ImageError::TooManyPixels {
            width: 4001,
            height: 4000
        })
    );
}

#[test]
fn a_buffer_that_does_not_match_its_size_or_an_empty_picture_is_refused() {
    let mut short = marker();
    short.pixels.pop();
    assert_eq!(encode_png(&short), Err(ImageError::Damaged));
    let empty = picture(0, 0, |_, _| [0; 4]);
    assert_eq!(encode_png(&empty), Err(ImageError::Damaged));
}
