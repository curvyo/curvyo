//! The writing mode and glyph orientation of texts in the file (`specs/0079-text-layout-controls`
//! slice 3, criteria 20, 21, 37; `adrs.md` decision 13): the golden file of format 20, save and
//! open, defaults never stored, and a file of format 19 opening as before.

#![allow(clippy::unwrap_used)]

use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, Document, FontKind, FontRef, Length, ObjectSnapshot,
    TextGlyphOrientation, TextLayoutLabel, TextLayoutValues, TextSnapshot, TextWritingMode, Vec2,
    pack, unpack,
};

/// The version this slice takes; a rebase onto another bump renumbers it and the fixture.
const VERTICAL_FORMAT_VERSION: u32 = 20;
const GOLDEN: &str = "text_layout_v20.curvyo";

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

fn values() -> [TextLayoutValues; 6] {
    let d = TextLayoutValues::DEFAULT;
    let mode = |writing_mode| TextLayoutValues { writing_mode, ..d };
    let orientation = |orientation| TextLayoutValues { orientation, ..d };
    [
        mode(TextWritingMode::VerticalRl),
        mode(TextWritingMode::VerticalLr),
        orientation(TextGlyphOrientation::Upright),
        orientation(TextGlyphOrientation::Sideways),
        TextLayoutValues {
            writing_mode: TextWritingMode::VerticalLr,
            orientation: TextGlyphOrientation::Upright,
            ..d
        },
        d,
    ]
}

/// Criterion 37: one text per value, one with both keys and one with none.
fn golden_document() -> Document {
    let d = Document::new(1);
    for (index, layout) in values().iter().enumerate() {
        let x = 20.0 + 25.0 * f64::from(u8::try_from(index).unwrap());
        // Created a size off and resized, so no two rows of the history share a label.
        let id = d
            .create_text(
                "Line one\nLine two",
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
/// the default text has no layout.
#[test]
fn ac37_golden_file_and_round_trip() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= VERTICAL_FORMAT_VERSION);
    let document = golden_document();
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(GOLDEN), pack(&document, "0.1.0").unwrap()).unwrap();
    }
    let stored = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&stored), VERTICAL_FORMAT_VERSION);
    let opened = unpack(2, &stored).unwrap();
    let read = texts(&opened);
    assert_eq!(
        read.iter().map(|t| t.layout).collect::<Vec<_>>(),
        values().to_vec()
    );
    let again = unpack(3, &pack(&opened, "t").unwrap()).unwrap();
    assert_eq!(again.export_json().unwrap(), opened.export_json().unwrap());
    let json: serde_json::Value = serde_json::from_slice(&opened.export_json().unwrap()).unwrap();
    let objects = json["objects"].as_array().unwrap();
    assert!(objects[5].to_string().find("layout").is_none());
    assert!(objects[0].to_string().contains("\"vertical_rl\""));
}

/// Criterion 37: a file of format 19 opens horizontal and auto, and saving writes neither key.
#[test]
fn ac37_a_format_19_file_opens_horizontal() {
    let old = unpack(
        4,
        &std::fs::read(fixture_path("text_layout_v19.curvyo")).unwrap(),
    )
    .unwrap();
    let read = texts(&old);
    assert_ne!(read.len(), 0);
    assert!(
        read.iter()
            .all(|t| t.layout.writing_mode == TextWritingMode::Horizontal
                && t.layout.orientation == TextGlyphOrientation::Auto)
    );
    let json = String::from_utf8(old.export_json().unwrap()).unwrap();
    assert!(!json.contains("writing_mode") && !json.contains("orientation"));
}

/// Criteria 20 and 21: a change is one step with its label; back at the default the key goes.
#[test]
fn ac20_one_step_and_defaults_deleted() {
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
    let vertical = TextLayoutValues {
        writing_mode: TextWritingMode::VerticalRl,
        ..TextLayoutValues::DEFAULT
    };
    let moved = Affine::translation(Vec2::new(0.0, -7.0));
    d.set_text_layout(&[(id, vertical, moved)], TextLayoutLabel::WritingMode)
        .unwrap();
    assert_eq!(d.own_steps().last().unwrap().label, "change_writing_mode");
    assert_eq!(d.text(id).unwrap().placement, moved);
    let sideways = TextLayoutValues {
        orientation: TextGlyphOrientation::Sideways,
        ..vertical
    };
    d.set_text_layout(&[(id, sideways, moved)], TextLayoutLabel::GlyphOrientation)
        .unwrap();
    assert_eq!(
        d.own_steps().last().unwrap().label,
        "change_glyph_orientation"
    );
    d.set_text_layout(
        &[(id, TextLayoutValues::DEFAULT, moved)],
        TextLayoutLabel::WritingMode,
    )
    .unwrap();
    let json = String::from_utf8(d.export_json().unwrap()).unwrap();
    assert!(!json.contains("layout"), "{json}");
}
