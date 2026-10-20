//! `specs/0075-font-library` through the session: the cuttable check gives the pieces Union
//! makes of the same converted text (criterion 19), and the Remove prompt counts the texts that
//! use a family of the Curvyo fonts block (criterion 25).

#![allow(clippy::unwrap_used, missing_docs)]

use std::path::PathBuf;

use curvyo_document_core::Point;
use curvyo_document_core::unpack;
use curvyo_editor_wasm::{
    BooleanOutcome, EscapeStep, KeyInput, ObjectToPathOutcome, Session, Tool,
};
use curvyo_ui_core::BooleanOp;
use curvyo_ui_core::{FontBlock, TextInput, cuttable_pieces};
use serde_json::Value;

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn read(path: &str) -> Vec<u8> {
    std::fs::read(fonts_dir().join(path)).unwrap()
}

/// Great Vibes with every advance at 80 %: a connected script (the fixture of criterion 17).
fn connected_script() -> Vec<u8> {
    let mut out = read("great-vibes/GreatVibes-Regular.ttf");
    let u16_at = |b: &[u8], at: usize| u16::from_be_bytes([b[at], b[at + 1]]);
    let table = |b: &[u8], tag: &[u8; 4]| {
        let tables = usize::from(u16_at(b, 4));
        let at = (0..tables)
            .map(|r| 12 + 16 * r)
            .find(|&at| &b[at..at + 4] == tag)
            .unwrap();
        u32::from_be_bytes(b[at + 8..at + 12].try_into().unwrap()) as usize
    };
    let metrics = usize::from(u16_at(&out, table(&out, b"hhea") + 34));
    let hmtx = table(&out, b"hmtx");
    for glyph in 0..metrics {
        let at = hmtx + 4 * glyph;
        let advance = u32::from(u16_at(&out, at)) * 80 / 100;
        out[at..at + 2].copy_from_slice(&u16::try_from(advance).unwrap().to_be_bytes());
    }
    out
}

/// A session with the files listed in the Curvyo block under their names, all loaded.
fn session(files: &[(&str, &str, Vec<u8>)]) -> Session {
    let mut s = Session::new(1);
    s.resize_viewport(1200.0, 800.0);
    let faces = files
        .iter()
        .map(|(id, family, _)| {
            (
                (*id).to_owned(),
                (*family).to_owned(),
                "Regular".to_owned(),
                0,
            )
        })
        .collect();
    s.set_font_list(FontBlock::Curvyo, faces, Vec::new());
    for (id, _, bytes) in files {
        assert!(s.add_font_file(id, bytes));
    }
    s
}

fn pick(s: &mut Session, family: &str) {
    let tab: Value = serde_json::from_str(&s.fonts_tab_json()).unwrap();
    let index = tab["faces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["family"] == family)
        .unwrap()["index"]
        .as_u64()
        .unwrap();
    s.pick_font(usize::try_from(index).unwrap());
}

fn add_text(s: &mut Session, at: Point, string: &str) {
    s.set_tool(Tool::Text);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    s.text_input(TextInput::Insert(string.to_owned()));
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    assert_eq!(s.escape(), EscapeStep::LeftTool);
    s.set_text_size(20.0);
}

fn select_all(s: &mut Session) {
    s.set_tool(Tool::Select);
    s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
}

/// The pieces Union (`0016`) makes of the converted text: Object to path gives one path per
/// glyph (in a group, ungrouped here), Union joins them, Split gives one object per piece (a
/// shape with its holes).
fn union_pieces_of_text(s: &mut Session) -> usize {
    select_all(s);
    assert_eq!(s.object_to_path(), ObjectToPathOutcome::Converted);
    select_all(s);
    let _ = s.ungroup_selection();
    select_all(s);
    let union = s.apply_boolean(BooleanOp::Union);
    assert!(matches!(union, BooleanOutcome::Applied { .. }), "{union:?}");
    select_all(s);
    let _ = s.apply_split();
    let document = unpack(5, &s.pack("0.1.0").unwrap()).unwrap();
    document.object_ids().len()
}

/// Criterion 19: for the included fonts and two fixtures, the check equals the number of pieces
/// Union makes of the converted text `cuttable` at 20 mm.
#[test]
fn ac19_the_check_equals_the_union_of_the_converted_text() {
    let fixtures = [
        ("vibes", read("great-vibes/GreatVibes-Regular.ttf")),
        ("sans", read("source-sans-3/SourceSans3-Regular.otf")),
        ("serif", read("source-serif-4/SourceSerif4-Regular.ttf")),
        ("connected", connected_script()),
        ("bold", read("source-serif-4/SourceSerif4-BoldIt.ttf")),
    ];
    for (name, bytes) in fixtures {
        let mut s = session(&[(name, name, bytes.clone())]);
        pick(&mut s, name);
        add_text(&mut s, Point::new(10.0, 40.0), "cuttable");
        assert_eq!(
            union_pieces_of_text(&mut s),
            cuttable_pieces(&bytes, 0).unwrap(),
            "{name}"
        );
    }
}

/// Criterion 25: the prompt counts the texts using a face of the family from the Curvyo block.
#[test]
fn ac25_texts_using_a_curvyo_family_are_counted() {
    let mut s = session(&[
        (
            "vibes",
            "Great Vibes",
            read("great-vibes/GreatVibes-Regular.ttf"),
        ),
        (
            "sans",
            "Sans",
            read("source-sans-3/SourceSans3-Regular.otf"),
        ),
    ]);
    pick(&mut s, "Great Vibes");
    add_text(&mut s, Point::new(10.0, 40.0), "One");
    add_text(&mut s, Point::new(10.0, 80.0), "Two");
    // Nothing selected, so the pick is the next text's face.
    s.set_tool(Tool::Select);
    s.escape();
    pick(&mut s, "Sans");
    add_text(&mut s, Point::new(10.0, 120.0), "Three");
    assert_eq!(s.texts_using_family("great  VIBES"), 2);
    assert_eq!(s.texts_using_family("Sans"), 1);
    assert_eq!(s.texts_using_family("Lobster"), 0);
}
