//! Kerning per character in the document and the file (`specs/0079-text-layout-controls` slice 4,
//! criteria 28 to 31, 37; ADR 0021): the style config on every document, a copy keeps the kerns,
//! undo restores them per character, two peers' kerns merge, the golden file of format 21, and a
//! file of format 20 opening with none.

#![allow(clippy::unwrap_used)]

use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, CharValues, CopySource, Document, FontKind, FontRef, Length,
    NodeId, ObjectSnapshot, TextChars, TextCharsLabel, TextLayoutValues, TextSnapshot, Vec2, pack,
    unpack,
};

/// The version this slice takes; a rebase onto another bump renumbers it and the fixture.
const KERNING_FORMAT_VERSION: u32 = 21;
const GOLDEN: &str = "text_layout_v21.curvyo";

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

fn kern(x: f64, y: f64) -> CharValues {
    CharValues {
        kern_x: x,
        kern_y: y,
        rotation: 0.0,
    }
}

fn text_at(d: &Document, string: &str, x: f64) -> NodeId {
    d.create_text(
        string,
        &sans(),
        Length::from_mm(10.0),
        Affine::translation(Vec2::new(x, 20.0)),
        &TextLayoutValues::DEFAULT,
    )
    .unwrap()
}

fn chars(d: &Document, id: NodeId) -> TextChars {
    d.text(id).unwrap().chars
}

/// The per-text values of the golden file: `kern_x` on one character, `kern_y` on one, both on
/// several, the per-character rotation `rot` on two characters (one of them kerned too), none.
fn golden_chars() -> [TextChars; 5] {
    [
        TextChars::new([(1, kern(-0.08, 0.0))]),
        TextChars::new([(2, kern(0.0, 0.3))]),
        TextChars::new([
            (0, kern(0.1, -0.2)),
            (3, kern(-0.05, 0.0)),
            (4, kern(0.0, 1.5)),
        ]),
        TextChars::new([
            (
                1,
                CharValues {
                    rotation: 30.0,
                    ..CharValues::DEFAULT
                },
            ),
            (
                2,
                CharValues {
                    kern_y: 0.1,
                    rotation: -45.0,
                    ..CharValues::DEFAULT
                },
            ),
        ]),
        TextChars::default(),
    ]
}

fn golden_document() -> Document {
    let d = Document::new(1);
    for (index, values) in golden_chars().iter().enumerate() {
        let x = 20.0 + 40.0 * f64::from(u8::try_from(index).unwrap());
        let id = text_at(&d, "AVATAR\nWave", x);
        if !values.is_empty() {
            // Distinct labels per row: the history test of the text goldens.
            let label = match index {
                3 => TextCharsLabel::CharRotation,
                _ if index % 2 == 0 => TextCharsLabel::Kerning,
                _ => TextCharsLabel::ClearKerning,
            };
            d.set_text_chars(&[(id, values.clone())], label).unwrap();
        }
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

/// Criterion 37: the golden file opens to the same kerns; saving and opening again keeps them;
/// the text without kerns mirrors no `chars`.
#[test]
fn ac37_golden_file_and_round_trip() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= KERNING_FORMAT_VERSION);
    let document = golden_document();
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(GOLDEN), pack(&document, "0.1.0").unwrap()).unwrap();
    }
    let stored = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&stored), KERNING_FORMAT_VERSION);
    let opened = unpack(2, &stored).unwrap();
    let read: Vec<TextChars> = texts(&opened).into_iter().map(|t| t.chars).collect();
    assert_eq!(read, golden_chars().to_vec());
    let again = unpack(3, &pack(&opened, "t").unwrap()).unwrap();
    assert_eq!(again.export_json().unwrap(), opened.export_json().unwrap());
    let json: serde_json::Value = serde_json::from_slice(&opened.export_json().unwrap()).unwrap();
    let objects = json["objects"].as_array().unwrap();
    assert!(objects[4].to_string().find("chars").is_none());
    assert!(objects[0].to_string().contains("kern_x"));
    assert!(objects[3].to_string().contains("rotation"));
    // The reopened document can kern (the config is set on open).
    let id = texts(&opened)[4].id;
    opened
        .set_text_chars(&[(id, golden_chars()[0].clone())], TextCharsLabel::Kerning)
        .unwrap();
    assert_eq!(chars(&opened, id), golden_chars()[0]);
}

/// Criterion 37: a file of format 20 opens with no kern, and saving writes none.
#[test]
fn ac37_a_format_20_file_opens_without_kerns() {
    let old = unpack(
        4,
        &std::fs::read(fixture_path("text_layout_v20.curvyo")).unwrap(),
    )
    .unwrap();
    let read = texts(&old);
    assert_ne!(read.len(), 0);
    assert!(read.iter().all(|t| t.chars.is_empty()));
    let json = String::from_utf8(old.export_json().unwrap()).unwrap();
    assert!(!json.contains("chars"));
}

/// Criteria 27 and 28: a change is one step "Change kerning", the reset one step "Clear
/// kerning"; nothing changed writes no step.
#[test]
fn ac28_one_step_per_change_and_clear() {
    let d = Document::new(1);
    let a = text_at(&d, "AV", 0.0);
    let b = text_at(&d, "To", 40.0);
    let steps = d.own_steps().len();
    let all = TextChars::new([(0, kern(0.1, 0.0)), (1, kern(0.1, 0.0))]);
    d.set_text_chars(
        &[(a, all.clone()), (b, all.clone())],
        TextCharsLabel::Kerning,
    )
    .unwrap();
    assert_eq!(d.own_steps().len(), steps + 1);
    assert_eq!(d.own_steps().last().unwrap().label, "change_kerning");
    d.set_text_chars(&[(a, all.clone())], TextCharsLabel::Kerning)
        .unwrap();
    assert_eq!(d.own_steps().len(), steps + 1, "unchanged: no step");
    d.set_text_chars(
        &[(a, TextChars::default()), (b, TextChars::default())],
        TextCharsLabel::ClearKerning,
    )
    .unwrap();
    assert_eq!(d.own_steps().last().unwrap().label, "clear_kerning");
    assert!(chars(&d, a).is_empty() && chars(&d, b).is_empty());
}

/// Criterion 30 at the document level: an edit keeps a kern with its character, and the
/// session's values decide for typed characters.
#[test]
fn ac30_an_edit_carries_kerns_with_their_characters() {
    let d = Document::new(1);
    let id = text_at(&d, "AV", 0.0);
    d.edit_text(id, "AV", &TextChars::new([(1, kern(-0.1, 0.0))]))
        .unwrap();
    assert_eq!(d.own_steps().last().unwrap().label, "edit_text");
    d.edit_text(id, "xAV", &TextChars::new([(2, kern(-0.1, 0.0))]))
        .unwrap();
    assert_eq!(chars(&d, id).entries(), &[(2, kern(-0.1, 0.0))]);
    let steps = d.own_steps().len();
    d.edit_text(id, "xAV", &TextChars::new([(2, kern(-0.1, 0.0))]))
        .unwrap();
    assert_eq!(d.own_steps().len(), steps, "unchanged: no step");
}

/// ADR 0021 §4: duplicating a text copies its string with its kerns.
#[test]
fn a_copy_keeps_the_kerns() {
    let d = Document::new(1);
    let id = text_at(&d, "AVATAR", 0.0);
    let values = golden_chars()[2].clone();
    d.set_text_chars(&[(id, values.clone())], TextCharsLabel::Kerning)
        .unwrap();
    let copies = d
        .duplicate_objects(
            &[CopySource {
                id,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(5.0, 5.0),
        )
        .unwrap();
    assert_eq!(chars(&d, copies[0]), values);
}

/// Criterion 29 and ADR 0021 §5: undo of a kerning step restores each character's old value,
/// and leaves a kern a later step set on another character; redo writes it again.
#[test]
fn ac29_undo_restores_per_character() {
    let d = Document::new(1);
    let id = text_at(&d, "HAVE", 0.0);
    let first = TextChars::new([(1, kern(-0.1, 0.0))]);
    d.set_text_chars(&[(id, first.clone())], TextCharsLabel::Kerning)
        .unwrap();
    let second = TextChars::new([(1, kern(-0.2, 0.0)), (2, kern(0.0, 0.5))]);
    d.set_text_chars(&[(id, second.clone())], TextCharsLabel::Kerning)
        .unwrap();
    let step = d.own_steps().last().unwrap().step;
    // A later step kerns the last character.
    let later = TextChars::new([
        (1, kern(-0.2, 0.0)),
        (2, kern(0.0, 0.5)),
        (3, kern(0.3, 0.0)),
    ]);
    d.set_text_chars(&[(id, later)], TextCharsLabel::Kerning)
        .unwrap();
    d.undo_step(step).unwrap();
    assert_eq!(
        chars(&d, id),
        TextChars::new([(1, kern(-0.1, 0.0)), (3, kern(0.3, 0.0))])
    );
    d.redo_step(step).unwrap();
    assert_eq!(chars(&d, id).get(2), kern(0.0, 0.5));
}

/// Criterion 29: undo of an edit session (string and kerns) gives back both.
#[test]
fn ac29_undo_of_an_edit_restores_string_and_kerns() {
    let d = Document::new(1);
    let id = text_at(&d, "AV", 0.0);
    d.set_text_chars(
        &[(id, TextChars::new([(1, kern(-0.1, 0.0))]))],
        TextCharsLabel::Kerning,
    )
    .unwrap();
    d.edit_text(id, "WAVE", &TextChars::new([(0, kern(0.2, 0.0))]))
        .unwrap();
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
    let text = d.text(id).unwrap();
    assert_eq!(text.string, "AV");
    assert_eq!(text.chars, TextChars::new([(1, kern(-0.1, 0.0))]));
}

/// Criterion 31: kerns set by two peers on different characters both survive; the same
/// character converges on one value on both peers.
#[test]
fn ac31_two_peers_kerns_merge_per_character() {
    let a = Document::new(1);
    let id = text_at(&a, "AVATAR", 0.0);
    let b = unpack(9, &pack(&a, "t").unwrap()).unwrap();
    a.set_text_chars(
        &[(
            id,
            TextChars::new([(1, kern(-0.1, 0.0)), (4, kern(0.2, 0.0))]),
        )],
        TextCharsLabel::Kerning,
    )
    .unwrap();
    b.set_text_chars(
        &[(
            id,
            TextChars::new([(3, kern(0.0, 0.4)), (4, kern(-0.3, 0.0))]),
        )],
        TextCharsLabel::Kerning,
    )
    .unwrap();
    let shifted = TextChars::new([(4, kern(0.0, 0.4)), (5, kern(-0.3, 0.0))]);
    b.edit_text(id, "xAVATAR", &shifted).unwrap();
    for (from, to) in [(&a, &b), (&b, &a)] {
        to.import_updates(&from.export_updates(&to.version()).unwrap())
            .unwrap();
    }
    let merged = chars(&a, id);
    assert_eq!(merged, chars(&b, id));
    assert_eq!(a.text(id).unwrap().string, "xAVATAR");
    assert_eq!(merged.get(2), kern(-0.1, 0.0));
    assert_eq!(merged.get(4), kern(0.0, 0.4));
    assert!(merged.get(5) == kern(0.2, 0.0) || merged.get(5) == kern(-0.3, 0.0));
}

/// ADR 0021 §2: the config survives a history wipe.
#[test]
fn a_wiped_document_can_kern() {
    let mut d = Document::new(1);
    let id = text_at(&d, "AV", 0.0);
    d.wipe_history().unwrap();
    d.set_text_chars(
        &[(id, TextChars::new([(1, kern(-0.1, 0.0))]))],
        TextCharsLabel::Kerning,
    )
    .unwrap();
    assert_eq!(chars(&d, id).get(1), kern(-0.1, 0.0));
}
