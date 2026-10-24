//! The layout values of texts in the file and in one step (`specs/0079-text-layout-controls`
//! criteria 4, 12, 13, 34, 36, 37; `adrs.md` decisions 1 to 3): the golden file of format 19, save
//! and open, defaults never stored, and `Document::set_text_layout` as one commit.

#![allow(clippy::unwrap_used)]

use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, Angle, CURRENT_FORMAT_VERSION, Document, FontKind, FontRef, Length, NodeId,
    ObjectSnapshot, TextAlign, TextLayoutLabel, TextLayoutValues, TextSnapshot, Vec2, pack, unpack,
};

/// The version this slice takes; a rebase onto another bump renumbers it and the fixture.
const LAYOUT_FORMAT_VERSION: u32 = 19;
const GOLDEN: &str = "text_layout_v19.curvyo";

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

fn values() -> [TextLayoutValues; 7] {
    let d = TextLayoutValues::DEFAULT;
    [
        TextLayoutValues {
            align: TextAlign::Centre,
            ..d
        },
        TextLayoutValues {
            letter_spacing: 0.15,
            ..d
        },
        TextLayoutValues {
            word_spacing: -0.25,
            ..d
        },
        TextLayoutValues {
            line_spacing: Some(1.5),
            ..d
        },
        TextLayoutValues {
            rotation: Angle::from_radians(-12.5_f64.to_radians()),
            ..d
        },
        TextLayoutValues {
            align: TextAlign::Justify,
            letter_spacing: 0.1,
            word_spacing: 0.3,
            line_spacing: Some(0.0),
            rotation: Angle::from_radians(90.0_f64.to_radians()),
        },
        d,
    ]
}

/// Criterion 37: one text per key, one with all five and one with none.
fn golden_document() -> Document {
    let d = Document::new(1);
    for (index, layout) in values().iter().enumerate() {
        let y = 20.0 + 25.0 * f64::from(u8::try_from(index).unwrap());
        // Created a size off and resized, so no two rows of the history share a label (as the
        // other text goldens do).
        let id = d
            .create_text(
                "Line one\nLine two",
                &sans(),
                Length::from_mm(11.0),
                Affine::translation(Vec2::new(20.0, y)),
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

fn same(a: &TextLayoutValues, b: &TextLayoutValues) -> bool {
    a.align == b.align
        && (a.letter_spacing - b.letter_spacing).abs() < 1e-12
        && (a.word_spacing - b.word_spacing).abs() < 1e-12
        && a.line_spacing.map(|f| (f * 1e12).round()) == b.line_spacing.map(|f| (f * 1e12).round())
        && (a.rotation.as_radians() - b.rotation.as_radians()).abs() < 1e-12
}

/// Criterion 37: the golden file opens to the same values; saving and opening again keeps them;
/// the text with every value at its default has no layout key.
#[test]
fn ac37_golden_file_and_round_trip() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= LAYOUT_FORMAT_VERSION);
    let document = golden_document();
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(GOLDEN), pack(&document, "0.1.0").unwrap()).unwrap();
    }
    let stored = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&stored), LAYOUT_FORMAT_VERSION);
    let opened = unpack(2, &stored).unwrap();
    let read = texts(&opened);
    assert_eq!(read.len(), 7);
    for (text, expected) in read.iter().zip(values()) {
        assert!(same(&text.layout, &expected), "{:?}", text.layout);
    }
    let again = unpack(3, &pack(&opened, "t").unwrap()).unwrap();
    assert_eq!(again.export_json().unwrap(), opened.export_json().unwrap());
    let json: serde_json::Value = serde_json::from_slice(&opened.export_json().unwrap()).unwrap();
    let objects = json["objects"].as_array().unwrap();
    assert!(
        objects[6].to_string().find("layout").is_none(),
        "a default text writes no layout"
    );
    assert!(objects[0].to_string().contains("\"centre\""));
}

/// Criterion 37: a file from before this spec opens and draws as before; its texts carry the
/// default values and saving writes no layout key.
#[test]
fn ac37_older_files_open_with_the_defaults() {
    for name in ["text_v17.curvyo", "text_single_line_v18.curvyo"] {
        let old = unpack(4, &std::fs::read(fixture_path(name)).unwrap()).unwrap();
        assert_ne!(texts(&old).len(), 0);
        assert!(texts(&old).iter().all(|t| t.layout.is_default()), "{name}");
        let saved = unpack(5, &pack(&old, "t").unwrap()).unwrap();
        let json = String::from_utf8(saved.export_json().unwrap()).unwrap();
        assert!(!json.contains("layout"), "{name}");
    }
}

fn one_text(d: &Document) -> NodeId {
    d.create_text(
        "Hi there",
        &sans(),
        Length::from_mm(10.0),
        Affine::translation(Vec2::new(5.0, 5.0)),
        &TextLayoutValues::DEFAULT,
    )
    .unwrap()
}

/// Criteria 4, 12 and 13 (`adrs.md` decision 3): several texts change in one step with the
/// spec's label; the placement goes with it; an unchanged value writes no step; back at the
/// default the key is gone again.
#[test]
fn ac04_ac12_ac13_one_step_for_every_text() {
    let d = Document::new(1);
    let (a, b) = (one_text(&d), one_text(&d));
    let steps = d.own_steps().len();
    let spaced = TextLayoutValues {
        letter_spacing: 0.2,
        ..TextLayoutValues::DEFAULT
    };
    let place_a = d.text(a).unwrap().placement;
    let place_b = d.text(b).unwrap().placement;
    d.set_text_layout(
        &[(a, spaced, place_a), (b, spaced, place_b)],
        TextLayoutLabel::LetterSpacing,
    )
    .unwrap();
    assert_eq!(d.own_steps().len(), steps + 1);
    assert_eq!(d.own_steps().last().unwrap().label, "change_letter_spacing");
    assert!((d.text(b).unwrap().layout.letter_spacing - 0.2).abs() < 1e-12);

    d.set_text_layout(&[(a, spaced, place_a)], TextLayoutLabel::LetterSpacing)
        .unwrap();
    assert_eq!(d.own_steps().len(), steps + 1, "unchanged: no step");

    let centred = TextLayoutValues {
        align: TextAlign::Centre,
        ..spaced
    };
    let moved = Affine::translation(Vec2::new(-3.0, 0.0)).compose(place_a);
    d.set_text_layout(&[(a, centred, moved)], TextLayoutLabel::Alignment)
        .unwrap();
    assert_eq!(d.own_steps().last().unwrap().label, "change_text_alignment");
    assert_eq!(d.text(a).unwrap().placement, moved);
    assert_eq!(d.text(a).unwrap().layout.align, TextAlign::Centre);

    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
    assert_eq!(d.text(a).unwrap().layout.align, TextAlign::Left);
    assert_eq!(d.text(a).unwrap().placement, place_a);

    d.set_text_layout(
        &[(a, TextLayoutValues::DEFAULT, place_a)],
        TextLayoutLabel::LetterSpacing,
    )
    .unwrap();
    let json = String::from_utf8(d.export_json().unwrap()).unwrap();
    assert_eq!(
        json.matches("letter_spacing").count(),
        1,
        "only text b keeps it"
    );
}
