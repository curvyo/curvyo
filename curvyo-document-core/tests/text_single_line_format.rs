//! Single-line texts in the file (`specs/0076-single-line-fonts` criteria 9, 10 and 21,
//! `adrs.md` decision 1): the golden file of format 18 with `text_font.kind`, save and open, and
//! the stroke a single-line pick switches on in the same step.

#![allow(clippy::unwrap_used)]

#[path = "support/pre_pages_ids.rs"]
mod pre_pages_ids;

use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, Color, Document, FontKind, FontRef, Length, LineCap, LineJoin,
    ObjectSnapshot, SINGLE_LINE_STROKE_WIDTH, StyleEdit, TextLayoutValues, TextSnapshot, pack,
    unpack,
};

/// The version this slice takes; a rebase onto another bump renumbers it and the fixture.
const SINGLE_LINE_FORMAT_VERSION: u32 = 18;
const GOLDEN: &str = "text_single_line_v18.curvyo";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn font(family: &str, hash_digit: char, kind: FontKind) -> FontRef {
    FontRef {
        family: family.to_owned(),
        style: "Regular".to_owned(),
        face_hash: hash_digit.to_string().repeat(64),
        face_index: 0,
        kind,
    }
}

/// The four texts of criterion 21: one line, two lines, rotated 30°, and a single-line face no
/// computer of the test has.
fn golden_document() -> Document {
    let d = Document::new(1);
    let hershey = font("Hershey Sans 1-stroke", 'a', FontKind::SingleLine);
    let (sin, cos) = 30.0_f64.to_radians().sin_cos();
    let texts = [
        (
            "Engrave",
            hershey.clone(),
            20.0,
            Affine::new(1.0, 0.0, 0.0, 1.0, 10.0, 30.0),
        ),
        (
            "Line one\nLine two",
            hershey.clone(),
            10.0,
            Affine::new(1.0, 0.0, 0.0, 1.0, 10.0, 60.0),
        ),
        (
            "Turned",
            hershey,
            12.0,
            Affine::new(cos, sin, -sin, cos, 100.0, 30.0),
        ),
        (
            "Lost face",
            font("Missing Stroke Font", 'b', FontKind::SingleLine),
            15.0,
            Affine::new(1.0, 0.0, 0.0, 1.0, 10.0, 120.0),
        ),
    ];
    for (string, font, size, placement) in texts {
        let id = d
            .create_text(
                string,
                &font,
                Length::from_mm(size + 1.0),
                placement,
                &TextLayoutValues::DEFAULT,
            )
            .unwrap();
        d.set_text_size(&[id], Length::from_mm(size)).unwrap();
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

/// Criterion 21: the golden file opens to the same strings, faces (with their kind), sizes,
/// placements and style; saving and opening again keeps them; no glyph is stored.
#[test]
fn ac21_golden_file_and_round_trip() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= SINGLE_LINE_FORMAT_VERSION);
    let document = golden_document();
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(GOLDEN), pack(&document, "0.1.0").unwrap()).unwrap();
    }
    let stored = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&stored), SINGLE_LINE_FORMAT_VERSION);
    let opened = unpack(2, &stored).unwrap();
    let expected_texts: Vec<_> = texts(&document)
        .into_iter()
        .map(|mut text| {
            text.id = pre_pages_ids::node(text.id);
            text
        })
        .collect();
    assert_eq!(texts(&opened), expected_texts);
    assert_eq!(texts(&opened).len(), 4);
    assert!(
        texts(&opened)
            .iter()
            .all(|t| t.font.kind == FontKind::SingleLine)
    );
    let again = unpack(3, &pack(&opened, "t").unwrap()).unwrap();
    assert_eq!(again.export_json().unwrap(), opened.export_json().unwrap());
    let json = String::from_utf8(opened.export_json().unwrap()).unwrap();
    assert!(json.contains("\"kind\": \"single_line\"") && !json.contains("advance"));
    // A format-17 file has no key: its texts are outline texts.
    let old = unpack(4, &std::fs::read(fixture_path("text_v17.curvyo")).unwrap()).unwrap();
    assert!(texts(&old).iter().all(|t| t.font.kind == FontKind::Outline));
}

/// Criteria 9 and 10: a new single-line text, or a pick of a single-line face, switches a stroke
/// that is off on in the fill colour at 0.25 mm with round caps and joins, in the same step; a
/// stroke that is on is kept; switching back to an outline face changes nothing else.
#[test]
fn ac09_ac10_the_stroke_of_a_single_line_pick() {
    let d = Document::new(1);
    let sans = font("Source Sans 3", 'c', FontKind::Outline);
    let id = d
        .create_text(
            "Hi",
            &sans,
            Length::from_mm(10.0),
            Affine::IDENTITY,
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    d.edit_style(&[id], &StyleEdit::StrokeEnabled(false))
        .unwrap();
    d.edit_style(&[id], &StyleEdit::FillColor(Color { r: 200, g: 0, b: 0 }))
        .unwrap();
    let steps = d.own_steps().len();
    d.set_text_fonts(&[(id, font("Hershey Sans 1-stroke", 'a', FontKind::SingleLine))])
        .unwrap();
    assert_eq!(d.own_steps().len(), steps + 1, "one step: Change font");
    let stroke = d.text(id).unwrap().style.stroke;
    assert!(stroke.enabled);
    assert_eq!(stroke.color, Color { r: 200, g: 0, b: 0 });
    assert_eq!(stroke.width, SINGLE_LINE_STROKE_WIDTH);
    assert_eq!((stroke.cap, stroke.join), (LineCap::Round, LineJoin::Round));
    let fill = d.text(id).unwrap().style.fill;
    d.set_text_fonts(&[(id, sans.clone())]).unwrap();
    let back = d.text(id).unwrap().style;
    assert_eq!(
        (back.stroke, back.fill),
        (stroke.clone(), fill),
        "criterion 10"
    );
    d.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(1.0)))
        .unwrap();
    d.set_text_fonts(&[(id, font("EMS Allure", 'd', FontKind::SingleLine))])
        .unwrap();
    assert_eq!(
        d.text(id).unwrap().style.stroke.width,
        Length::from_mm(1.0),
        "kept"
    );
    let created = d
        .create_text(
            "New",
            &font("EMS Readability", 'e', FontKind::SingleLine),
            Length::from_mm(10.0),
            Affine::IDENTITY,
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    let new_style = d.text(created).unwrap().style;
    assert!(new_style.stroke.enabled, "criterion 9 on Add text");
    assert_eq!(d.own_steps().last().unwrap().label, "add_text");
}
