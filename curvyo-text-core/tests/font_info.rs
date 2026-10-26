//! The info fields of a face (`specs/0083-font-info-and-zip` criterion 2): each `fsType` value, a
//! face with every name ID, one with none, an SVG font and a collection. The variants are made in
//! memory from the shipped fonts (no modified font is checked in).
#![allow(clippy::unwrap_used)]
#![cfg(not(target_arch = "wasm32"))]

mod common;

use common::{SANS, SERIF, font_bytes};
use curvyo_text_core::{FaceStore, FontFormat, FontInfo, font_info};

fn info_of(bytes: Vec<u8>, index: u32) -> FontInfo {
    let mut store = FaceStore::new();
    let key = store.add(bytes, index).unwrap();
    font_info(store.get(&key).unwrap())
}

/// The offset of table `tag` in a single-face file.
fn table_at(bytes: &[u8], tag: [u8; 4]) -> Option<(usize, usize)> {
    let tables = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    (0..tables).map(|r| 12 + 16 * r).find_map(|record| {
        (bytes[record..record + 4] == tag).then(|| {
            let offset = u32::from_be_bytes(bytes[record + 8..record + 12].try_into().unwrap());
            (record, usize::try_from(offset).unwrap())
        })
    })
}

fn with_fs_type(path: &str, fs_type: u16) -> Vec<u8> {
    let mut bytes = font_bytes(path);
    let (_, os2) = table_at(&bytes, *b"OS/2").unwrap();
    bytes[os2 + 8..os2 + 10].copy_from_slice(&fs_type.to_be_bytes());
    bytes
}

/// Criterion 2, Embedding: the raw `fsType` of each value comes through.
#[test]
fn ac02_fs_type_values() {
    for fs_type in [0_u16, 0x0002, 0x0004, 0x0008, 0x0108, 0x0204] {
        assert_eq!(
            info_of(with_fs_type(SERIF, fs_type), 0).fs_type,
            Some(fs_type)
        );
    }
}

/// Criterion 2: a face with the name IDs filled, its format and glyph count.
#[test]
fn ac02_every_name_id() {
    let sans = info_of(font_bytes(SANS), 0);
    assert_eq!(sans.format, FontFormat::OpenTypeCff);
    assert_eq!(sans.full_name.as_deref(), Some("Source Sans 3"));
    assert!(sans.version.unwrap().starts_with("Version 3."));
    assert!(sans.designer.unwrap().contains("Paul D. Hunt"));
    assert!(sans.copyright.unwrap().contains("Adobe"));
    assert!(sans.trademark.unwrap().contains("Source"));
    assert!(sans.licence.unwrap().contains("SIL Open Font License"));
    assert!(sans.licence_url.unwrap().starts_with("http"));
    assert!(sans.vendor.is_some());
    assert!(sans.glyphs > 1000);
    let serif = info_of(font_bytes(SERIF), 0);
    assert_eq!(serif.format, FontFormat::TrueType);
}

/// Criterion 2, "Not stated": without a name table every name is `None`, the vendor comes from
/// `achVendID`.
#[test]
fn ac02_no_name_ids() {
    let mut bytes = font_bytes(SERIF);
    let (record, _) = table_at(&bytes, *b"name").unwrap();
    bytes[record..record + 4].copy_from_slice(b"xame");
    let info = info_of(bytes, 0);
    assert_eq!(
        (
            info.full_name,
            info.version,
            info.designer,
            info.copyright,
            info.trademark
        ),
        (None, None, None, None, None)
    );
    assert_eq!((info.licence, info.licence_url), (None, None));
    assert_eq!(info.vendor.as_deref(), Some("ADBO"));
}

/// Criterion 2: an SVG font states only its glyph count.
#[test]
fn ac02_svg_font() {
    let info = info_of(font_bytes("hershey-sans-1/HersheySans1.svg"), 0);
    assert_eq!(info.format, FontFormat::SvgFont);
    assert_eq!(
        (info.fs_type, info.full_name, info.vendor),
        (None, None, None)
    );
    let glyphs = String::from_utf8(font_bytes("hershey-sans-1/HersheySans1.svg"))
        .unwrap()
        .matches("<glyph ")
        .count();
    assert_eq!(info.glyphs, u32::try_from(glyphs).unwrap());
}

/// Criterion 2, Format: "face 2 of 2" of a collection.
#[test]
fn ac02_collection_face_number() {
    let face = font_bytes("great-vibes/GreatVibes-Regular.ttf");
    let shift = 12 + 4 * 2_u32;
    let mut font = face.clone();
    let tables = usize::from(u16::from_be_bytes([font[4], font[5]]));
    for record in 0..tables {
        let at = 12 + 16 * record + 8;
        let offset = u32::from_be_bytes(font[at..at + 4].try_into().unwrap()) + shift;
        font[at..at + 4].copy_from_slice(&offset.to_be_bytes());
    }
    let mut ttc = b"ttcf".to_vec();
    for word in [0x0001_0000_u32, 2, shift, shift] {
        ttc.extend_from_slice(&word.to_be_bytes());
    }
    ttc.extend_from_slice(&font);
    assert_eq!(
        info_of(ttc, 1).format,
        FontFormat::Collection { face: 2, faces: 2 }
    );
}

/// Criterion 5: the fields come from the held face's tables within 100 ms (asserted only in
/// release and off CI, printed always).
#[test]
fn ac05_info_fields_within_budget() {
    let mut store = FaceStore::new();
    let key = store.add(font_bytes(SANS), 0).unwrap();
    let face = store.get(&key).unwrap();
    let started = std::time::Instant::now();
    let info = font_info(face);
    let took = started.elapsed();
    assert!(info.glyphs > 0);
    let budget = std::time::Duration::from_millis(100);
    println!("font_info: {took:?} (budget {budget:?})");
    if !cfg!(debug_assertions) && std::env::var_os("CI").is_none() {
        assert!(took <= budget, "{took:?}");
    }
}
