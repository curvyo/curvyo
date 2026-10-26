//! The taper of texts in the file (`specs/0079-text-layout-controls` slice 2, criteria 16, 37;
//! `adrs.md` decision 7): the golden file of format 24, save and open, a taper of 0 never stored,
//! and a file of format 23 opening untapered.

#![allow(clippy::unwrap_used)]

use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, Angle, CURRENT_FORMAT_VERSION, Document, FontKind, FontRef, Length, ObjectSnapshot,
    TextAlign, TextGlyphOrientation, TextLayoutLabel, TextLayoutValues, TextSnapshot,
    TextWritingMode, Vec2, pack, unpack,
};

/// The version this slice takes; a rebase onto another bump renumbers it and the fixture.
const TAPER_FORMAT_VERSION: u32 = 24;
const GOLDEN: &str = "text_layout_v24.curvyo";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn sans() -> FontRef {
    FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    }
}

/// One text with a taper, one with a negative taper, one with the taper and every other value,
/// one with none.
fn values() -> [TextLayoutValues; 4] {
    let d = TextLayoutValues::DEFAULT;
    [
        TextLayoutValues { taper: 0.2, ..d },
        TextLayoutValues { taper: -0.5, ..d },
        TextLayoutValues {
            align: TextAlign::Centre,
            letter_spacing: 0.1,
            word_spacing: 0.2,
            line_spacing: Some(1.5),
            rotation: Angle::from_radians(10.0_f64.to_radians()),
            writing_mode: TextWritingMode::VerticalRl,
            orientation: TextGlyphOrientation::Upright,
            taper: 0.125,
        },
        d,
    ]
}

fn golden_document() -> Document {
    let d = Document::new(1);
    for (index, layout) in values().iter().enumerate() {
        let x = 20.0 + 25.0 * f64::from(u8::try_from(index).unwrap());
        let id = d
            .create_text(
                "Ring",
                &sans(),
                Length::from_mm(11.0),
                Affine::translation(Vec2::new(x, 20.0)),
                layout,
            )
            .unwrap();
        d.set_text_size(&[id], Length::from_mm(10.0)).unwrap();
    }
    d
}

fn texts(d: &Document) -> Vec<TextSnapshot> {
    d.read_objects()
        .objects
        .iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Text(text) => Some((**text).clone()),
            _ => None,
        })
        .collect()
}

fn manifest_version(bytes: &[u8]) -> u32 {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Vec::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut out)
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&out).unwrap();
    u32::try_from(manifest["format_version"].as_u64().unwrap()).unwrap()
}

/// Criterion 37: the golden file opens to the same values; saving and opening again keeps them;
/// the untapered text mirrors without a taper.
#[test]
fn ac37_golden_file_and_round_trip() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= TAPER_FORMAT_VERSION);
    let document = golden_document();
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(GOLDEN), pack(&document, "0.1.0").unwrap()).unwrap();
    }
    let stored = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&stored), TAPER_FORMAT_VERSION);
    let opened = unpack(2, &stored).unwrap();
    let read: Vec<TextLayoutValues> = texts(&opened).iter().map(|t| t.layout).collect();
    assert_eq!(read.len(), 4);
    for (got, want) in read.iter().zip(values()) {
        assert!((got.taper - want.taper).abs() < 1e-12);
        assert_eq!(got.align, want.align);
        assert_eq!(got.writing_mode, want.writing_mode);
    }
    let again = unpack(3, &pack(&opened, "t").unwrap()).unwrap();
    assert_eq!(again.export_json().unwrap(), opened.export_json().unwrap());
    let json: serde_json::Value = serde_json::from_slice(&opened.export_json().unwrap()).unwrap();
    let objects = json["objects"].as_array().unwrap();
    assert!(objects[0].to_string().contains("\"taper\":0.2"));
    assert!(!objects[3].to_string().contains("taper"));
}

/// Criterion 37: a file of format 23 opens untapered, and saving writes no taper.
#[test]
fn ac37_a_format_23_file_opens_untapered() {
    let old = unpack(
        4,
        &std::fs::read(fixture_path("text_clones_v23.curvyo")).unwrap(),
    )
    .unwrap();
    let read = texts(&old);
    assert_ne!(read.len(), 0);
    assert!(read.iter().all(|t| t.layout.taper == 0.0));
    let json = String::from_utf8(old.export_json().unwrap()).unwrap();
    assert!(!json.contains("taper"));
}

/// Criterion 16: a change is one step "Change taper"; back at 0 the key goes.
#[test]
fn ac16_one_step_and_zero_deleted() {
    let d = Document::new(1);
    let id = d
        .create_text(
            "Hi",
            &sans(),
            Length::from_mm(10.0),
            Affine::IDENTITY,
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    let tapered = TextLayoutValues {
        taper: 0.3,
        ..TextLayoutValues::DEFAULT
    };
    d.set_text_layout(&[(id, tapered, Affine::IDENTITY)], TextLayoutLabel::Taper)
        .unwrap();
    assert_eq!(d.own_steps().last().unwrap().label, "change_taper");
    assert!((d.text(id).unwrap().layout.taper - 0.3).abs() < 1e-12);
    d.set_text_layout(
        &[(id, TextLayoutValues::DEFAULT, Affine::IDENTITY)],
        TextLayoutLabel::Taper,
    )
    .unwrap();
    let json = String::from_utf8(d.export_json().unwrap()).unwrap();
    assert!(!json.contains("taper"), "{json}");
}
