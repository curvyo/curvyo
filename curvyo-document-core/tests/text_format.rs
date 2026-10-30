//! Text objects in the document and the file (`specs/0074-text-objects` criteria 13 and 28,
//! ADR 0020 §1): the golden file, save and open, damaged texts, one step per command, undo of an
//! edit, and the per-character merge of the string (ADR 0009 §3).

#![allow(clippy::unwrap_used)]

#[path = "support/pre_pages_ids.rs"]
mod pre_pages_ids;

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, Angle, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Document, FontRef,
    Length, NodeId, ObjectSnapshot, OpenError, TextChars, TextEditError, TextLayoutValues,
    TextSnapshot, pack, unpack,
};
use loro::{LoroDoc, TreeID};

fn tree_id(id: NodeId) -> TreeID {
    let json = serde_json::to_value(id).unwrap();
    TreeID::new(
        json["peer"].as_str().unwrap().parse().unwrap(),
        i32::try_from(json["counter"].as_i64().unwrap()).unwrap(),
    )
}

fn sync(a: &Document, b: &Document) {
    for (from, to) in [(a, b), (b, a)] {
        to.import_updates(&from.export_updates(&to.version()).unwrap())
            .unwrap();
    }
}

/// The version this slice takes; a rebase onto another bump renumbers it and the fixture.
const TEXT_FORMAT_VERSION: u32 = 17;
const GOLDEN: &str = "text_v17.curvyo";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn font(family: &str, style: &str, hash_digit: char) -> FontRef {
    FontRef {
        family: family.to_owned(),
        style: style.to_owned(),
        face_hash: hash_digit.to_string().repeat(64),
        face_index: 0,
        kind: curvyo_document_core::FontKind::Outline,
    }
}

/// The five texts of the golden file: one line, two lines, rotated 30°, stretched 2 × 1, and a
/// face that no computer of the test has.
fn golden_document() -> (Document, Vec<NodeId>) {
    let d = Document::new(1);
    let sans = font("Source Sans 3", "Regular", 'a');
    let (sin, cos) = 30.0_f64.to_radians().sin_cos();
    let texts = [
        (
            "Curvyo",
            sans.clone(),
            20.0,
            Affine::new(1.0, 0.0, 0.0, 1.0, 10.0, 30.0),
        ),
        (
            "Line one\nLine two",
            sans.clone(),
            10.0,
            Affine::new(1.0, 0.0, 0.0, 1.0, 10.0, 60.0),
        ),
        (
            "Turned",
            sans.clone(),
            12.0,
            Affine::new(cos, sin, -sin, cos, 100.0, 30.0),
        ),
        (
            "Wide",
            sans,
            8.0,
            Affine::new(2.0, 0.0, 0.0, 1.0, 100.0, 80.0),
        ),
        (
            "Lost face",
            font("Missing Font", "Bold", 'b'),
            15.0,
            Affine::new(1.0, 0.0, 0.0, 1.0, 10.0, 120.0),
        ),
    ];
    // Each text is made 1 mm larger and then set to its size: a file has no two adjacent steps
    // with one label (`specs/0020-undo-redo` criterion 36, the steps have no time without a clock).
    let ids = texts
        .iter()
        .map(|(string, font, size, placement)| {
            let id = d
                .create_text(
                    string,
                    font,
                    Length::from_mm(size + 1.0),
                    *placement,
                    &TextLayoutValues::DEFAULT,
                )
                .unwrap();
            d.set_text_size(&[id], Length::from_mm(*size)).unwrap();
            id
        })
        .collect();
    (d, ids)
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

/// Criterion 28: the golden file opens to the same strings, fonts, sizes, placements and style,
/// and saving and opening again keeps them exactly. Nothing measured is stored.
#[test]
fn ac28_golden_file_and_round_trip() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= TEXT_FORMAT_VERSION);
    let (document, _) = golden_document();
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(GOLDEN), pack(&document, "0.1.0").unwrap()).unwrap();
    }
    let stored = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&stored), TEXT_FORMAT_VERSION);
    let opened = unpack(2, &stored).unwrap();
    let expected_texts: Vec<_> = texts(&document)
        .into_iter()
        .map(|mut text| {
            text.id = pre_pages_ids::node(text.id);
            text
        })
        .collect();
    assert_eq!(texts(&opened), expected_texts);
    assert_eq!(texts(&opened).len(), 5);
    let again = unpack(3, &pack(&opened, "t").unwrap()).unwrap();
    assert_eq!(again.export_json().unwrap(), opened.export_json().unwrap());
    let json = String::from_utf8(opened.export_json().unwrap()).unwrap();
    assert!(json.contains("\"shape\": \"text\"") && !json.contains("advance"));
}

/// Criterion 28: a text with a damaged register is refused as damaged, not repaired.
#[test]
fn ac28_damaged_text_is_refused() {
    let (d, ids) = golden_document();
    let loro = LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    loro.set_peer_id(7).unwrap();
    loro.get_tree("paths")
        .get_meta(tree_id(ids[0]))
        .unwrap()
        .insert("text_size", 5000.0)
        .unwrap();
    loro.commit();
    let manifest = serde_json::json!({
        "format_version": CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "test",
    });
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for (name, bytes) in [
        ("manifest.json", serde_json::to_vec(&manifest).unwrap()),
        (
            "document.loro",
            loro.export(loro::ExportMode::Snapshot).unwrap(),
        ),
        ("document.json", b"{}".to_vec()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    let bytes = writer.finish().unwrap().into_inner();
    assert_eq!(unpack(2, &bytes).err(), Some(OpenError::Damaged));
}

/// Criterion 13: a font or size change on several texts is one step each; criterion 31: the
/// limit; criterion 8: an empty text is not created.
#[test]
fn ac13_ac31_commands_are_one_step_and_checked() {
    let (d, ids) = golden_document();
    let steps = d.own_steps().len();
    let serif = font("Source Serif 4", "Bold", 'c');
    d.set_text_fonts(&[(ids[0], serif.clone()), (ids[1], serif)])
        .unwrap();
    d.set_text_size(&ids[..2], Length::from_mm(25.0)).unwrap();
    assert_eq!(d.own_steps().len(), steps + 2);
    assert_eq!(d.text(ids[1]).unwrap().font.family, "Source Serif 4");
    assert_eq!(d.text(ids[0]).unwrap().size, Length::from_mm(25.0));
    assert_eq!(
        d.set_text_size(&ids[..1], Length::from_mm(2000.1)),
        Err(TextEditError::SizeOutOfRange)
    );
    let long = "x".repeat(curvyo_document_core::MAX_TEXT_CHARS + 1);
    assert_eq!(
        d.edit_text(ids[0], &long, &TextChars::default()),
        Err(TextEditError::TooLong)
    );
    let placement = Affine::IDENTITY;
    let sans = font("Source Sans 3", "Regular", 'a');
    assert_eq!(
        d.create_text(
            "",
            &sans,
            Length::from_mm(10.0),
            placement,
            &TextLayoutValues::DEFAULT
        ),
        Err(TextEditError::Empty)
    );
    assert_eq!(d.own_steps().len(), steps + 2, "refusals write nothing");
}

/// Criterion 9 at the document level and ADR 0009 §3: undo of an edit gives the old string back
/// as a text, and two peers typing at both ends of one text both keep their letters.
#[test]
fn ac09_undo_of_an_edit_and_per_character_merge() {
    let (a, ids) = golden_document();
    a.edit_text(ids[0], "Curvyo text", &TextChars::default())
        .unwrap();
    a.undo_step(a.own_steps().last().unwrap().step).unwrap();
    assert_eq!(a.text(ids[0]).unwrap().string, "Curvyo");
    let b = unpack(9, &pack(&a, "t").unwrap()).unwrap();
    a.edit_text(ids[0], "My Curvyo", &TextChars::default())
        .unwrap();
    b.edit_text(ids[0], "Curvyo!", &TextChars::default())
        .unwrap();
    for (from, to) in [(&a, &b), (&b, &a)] {
        to.import_updates(&from.export_updates(&to.version()).unwrap())
            .unwrap();
    }
    assert_eq!(a.text(ids[0]).unwrap().string, "My Curvyo!");
    assert_eq!(b.text(ids[0]).unwrap().string, "My Curvyo!");
}

/// ADR 0009 §3 and ADR 0004 §9 (architect review m2, must-fix 1): two peers typing 6,000
/// characters each into one text stay inside the typing limit; the merged 12,000-character text
/// saves and opens, and may be shortened but not lengthened.
#[test]
fn ac31_a_merge_past_the_typing_limit_opens() {
    let (a, ids) = golden_document();
    let b = unpack(9, &pack(&a, "t").unwrap()).unwrap();
    a.edit_text(
        ids[0],
        &format!("{}Curvyo", "a".repeat(6000)),
        &TextChars::default(),
    )
    .unwrap();
    b.edit_text(
        ids[0],
        &format!("Curvyo{}", "b".repeat(6000)),
        &TextChars::default(),
    )
    .unwrap();
    sync(&a, &b);
    let merged = a.text(ids[0]).unwrap().string;
    assert_eq!(merged.chars().count(), 12_006);
    let opened = unpack(3, &pack(&a, "t").unwrap()).unwrap();
    assert_eq!(opened.text(ids[0]).unwrap().string, merged);
    assert_eq!(
        opened.edit_text(ids[0], &format!("{merged}c"), &TextChars::default()),
        Err(TextEditError::TooLong)
    );
    opened
        .edit_text(ids[0], &merged[1..], &TextChars::default())
        .unwrap();
}

/// Architect review m2, must-fix 3: a turn writes the placement, never the size register.
#[test]
fn ac26_rotations_leave_the_size_register_alone() {
    let (d, ids) = golden_document();
    let peer2 = unpack(2, &pack(&d, "t").unwrap()).unwrap();
    let before = peer2.text(ids[2]).unwrap().size;
    for _ in 0..1000 {
        let object = peer2.object(ids[2]).unwrap();
        let turned = object.rotated(
            curvyo_document_core::Point::new(3.0, 4.0),
            Angle::from_radians(0.7),
        );
        peer2.rotate_object(&turned).unwrap();
    }
    assert_eq!(
        peer2.text(ids[2]).unwrap().size.as_mm().to_bits(),
        before.as_mm().to_bits()
    );
    let loro = LoroDoc::new();
    loro.import(&peer2.export_loro_snapshot().unwrap()).unwrap();
    let meta = loro.get_tree("paths").get_meta(tree_id(ids[2])).unwrap();
    assert_eq!(meta.get_last_editor("text_size"), Some(1));
    assert_eq!(meta.get_last_editor("text_placement"), Some(2));
}

/// Criterion 8 (ruling b of the architect review m2): emptying an existing text removes it in the
/// one step "Edit text"; Undo brings it back with its string.
#[test]
fn ac08_emptying_an_existing_text_removes_it_in_one_step() {
    let (d, ids) = golden_document();
    let steps = d.own_steps().len();
    d.edit_text(ids[1], "", &TextChars::default()).unwrap();
    assert!(d.text(ids[1]).is_none());
    assert_eq!(d.own_steps().len(), steps + 1);
    assert_eq!(d.own_steps().last().unwrap().label, "edit_text");
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
    assert_eq!(d.text(ids[1]).unwrap().string, "Line one\nLine two");
}

/// `specs/0078-font-style-buttons` criteria 10, 11 and 13 (`adrs.md` decision 3): a face per
/// text in one step "Change font", a text that keeps its face not rewritten, one Undo restoring
/// every face.
#[test]
fn style_press_sets_a_face_per_text_in_one_step() {
    let (d, ids) = golden_document();
    let before: Vec<FontRef> = ids.iter().map(|id| d.text(*id).unwrap().font).collect();
    let steps = d.own_steps().len();
    let bold = font("Source Sans 3", "Bold", 'b');
    let light = font("Source Sans 3", "Light Italic", 'e');
    d.set_text_fonts(&[
        (ids[0], bold.clone()),
        (ids[1], light.clone()),
        (ids[2], before[2].clone()),
    ])
    .unwrap();
    assert_eq!(d.own_steps().len(), steps + 1);
    assert_eq!(d.own_steps().last().unwrap().label, "change_font");
    assert_eq!(d.text(ids[0]).unwrap().font, bold);
    assert_eq!(d.text(ids[1]).unwrap().font, light);
    assert_eq!(d.text(ids[2]).unwrap().font, before[2]);
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
    for (id, font) in ids.iter().zip(&before) {
        assert_eq!(&d.text(*id).unwrap().font, font);
    }
    // A refused font writes nothing for any text.
    assert_eq!(
        d.set_text_fonts(&[(ids[0], bold), (ids[1], font("X", "Bold", 'Z'))]),
        Err(TextEditError::InvalidFont)
    );
    assert_eq!(&d.text(ids[0]).unwrap().font, &before[0]);
}

/// Architect review m2, must-fix 2: a face hash is 64 lower-case hex digits.
#[test]
fn ac28_upper_case_face_hash_is_refused() {
    let (d, ids) = golden_document();
    assert_eq!(
        d.set_text_fonts(&[(ids[0], font("Source Sans 3", "Regular", 'A'))]),
        Err(TextEditError::InvalidFont)
    );
}
