//! Independent session tests for `0068-laserable-lettering` (Weld), written from the
//! specification and the public names before the implementation was read: criterion 10 (script
//! letters that overlap are cut apart, inner lines kept), 11 and 20 (one text welds in one step;
//! one undo brings the editable text back with its id), 12 and 19 (one selected object welds),
//! 21 (the length in the outcome).

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::path::PathBuf;

use curvyo_document_core::{
    Document, Length, NodeId, ObjectSnapshot, Point, RectBounds, pack, unpack,
};
use curvyo_editor_wasm::{EscapeStep, Session, Tool, WeldOutcome};
use curvyo_ui_core::{FontBlock, TextInput};
use serde_json::Value;

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn load(s: &mut Session) {
    for file in s.wanted_font_files() {
        assert!(s.add_font_file(&file, &std::fs::read(fonts_dir().join(&file)).unwrap()));
    }
}

/// A session holding one text `string` in Great Vibes at `size_mm`, selected, tool Select.
fn script_text(string: &str, size_mm: f64) -> Session {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            let text = |key: &str| f[key].as_str().unwrap().to_owned();
            (text("file"), text("family"), text("style"), 0)
        })
        .collect();
    let mut s = Session::new(68);
    s.resize_viewport(1200.0, 800.0);
    s.set_font_list(FontBlock::Included, faces, Vec::new());
    load(&mut s);
    let tab: Value = serde_json::from_str(&s.fonts_tab_json()).unwrap();
    let vibes = tab["faces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["family"] == "Great Vibes")
        .unwrap()["index"]
        .as_u64()
        .unwrap();
    s.pick_font(usize::try_from(vibes).unwrap());
    load(&mut s);
    s.set_tool(Tool::Text);
    s.pointer_down(Point::new(10.0, 40.0), false);
    s.pointer_up(Point::new(10.0, 40.0), false, false);
    s.text_input(TextInput::Insert(string.to_owned()));
    load(&mut s);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    assert_eq!(s.escape(), EscapeStep::LeftTool, "the text is selected");
    s.set_text_size(size_mm);
    load(&mut s);
    s.set_tool(Tool::Select);
    s
}

fn reread(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

/// The only object's outlines as (node count, closed).
fn only_path(s: &Session) -> Vec<(usize, bool)> {
    let d = reread(s);
    let ids = d.object_ids();
    assert_eq!(ids.len(), 1, "one object");
    let Some(ObjectSnapshot::Path(path)) = d.object(ids[0]) else {
        panic!("a path");
    };
    path.subpaths()
        .map(|sp| (sp.anchors.len(), sp.closed))
        .collect()
}

/// Criterion 10 (and the customer's Frage 15): "Annabelle" in Great Vibes, whose letters overlap
/// each other, welds to one compound path that keeps inner open lines where letters overlap
/// (the letters are cut apart; one-piece plates are Union's job).
#[test]
fn ac10_overlapping_script_letters_are_cut_apart() {
    let mut s = script_text("Annabelle", 20.0);
    let outcome = s.apply_weld();
    let WeldOutcome::Applied {
        compound,
        length_mm,
        texts_only,
        objects,
        ..
    } = outcome
    else {
        panic!("{outcome:?}");
    };
    assert!(compound && texts_only);
    assert_eq!(objects, 1);
    assert!(length_mm > 0.0);
    let outlines = only_path(&s);
    let open = outlines.iter().filter(|o| !o.1).count();
    let closed = outlines.iter().filter(|o| o.1).count();
    assert!(open >= 1, "inner lines where letters overlap: {outlines:?}");
    assert!(closed >= 1, "the union's outlines are closed");
    assert!(outlines.iter().all(|&(n, c)| n >= if c { 3 } else { 2 }));
}

/// Criteria 11 and 20: one selected text is converted and welded in one step; one Ctrl+Z
/// restores the text, editable, with its id, string and size.
#[test]
fn ac11_ac20_one_undo_restores_the_editable_text() {
    let mut s = script_text("Weld me", 15.0);
    let before = reread(&s);
    let ids = before.object_ids();
    let Some(ObjectSnapshot::Text(text)) = before.object(ids[0]) else {
        panic!("a text");
    };
    let steps = s.history_len();
    assert!(matches!(s.apply_weld(), WeldOutcome::Applied { .. }));
    assert_eq!(s.history_len(), steps + 1, "one history step");
    assert!(matches!(
        reread(&s).object(reread(&s).object_ids()[0]),
        Some(ObjectSnapshot::Path(_))
    ));

    s.undo();
    let after = reread(&s);
    assert_eq!(after.object_ids(), ids, "the text is back with its id");
    let Some(ObjectSnapshot::Text(back)) = after.object(ids[0]) else {
        panic!("the text is a text again, not a path");
    };
    assert_eq!(back.string, text.string);
    assert_eq!(back.size, text.size);
    assert_eq!(back.font, text.font);
    assert_eq!(after.export_json().unwrap(), before.export_json().unwrap());
}

/// Criteria 12 and 19: one selected rectangle that overlaps nothing welds to one ordinary closed
/// path of 80 mm, selected alone; the Select tool stays.
#[test]
fn ac12_a_single_selected_object_welds() {
    let d = Document::new(68);
    let id: NodeId = d.create_rect(RectBounds {
        origin: Point::new(0.0, 0.0),
        width: Length::from_mm(20.0),
        height: Length::from_mm(20.0),
    });
    let mut s = Session::open(2, &pack(&d, "0.1.0").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    let at = Point::new(0.0, 10.0);
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    assert_eq!(s.style_scope().ids, vec![id], "the rectangle is selected");

    let outcome = s.apply_weld();
    let WeldOutcome::Applied {
        objects,
        compound,
        length_mm,
        length_text,
        hidden,
        ..
    } = outcome
    else {
        panic!("{outcome:?}");
    };
    assert_eq!((objects, compound, hidden), (1, false, 0));
    assert!((length_mm - 80.0).abs() < 1e-6, "{length_mm}");
    assert_eq!(length_text, "80.0 mm");
    assert_eq!(only_path(&s), vec![(4, true)]);
    let result = reread(&s).object_ids()[0];
    assert_eq!(
        s.style_scope().ids,
        vec![result],
        "the result is selected alone"
    );
    assert_eq!(s.tool(), Tool::Select);
}
