//! Acceptance tests of `specs/0068-laserable-lettering` written from the specification by the
//! tester, through the public session: the dot of an i is its own piece (criterion 23, Terms);
//! several texts and a shape weld into one result (criterion 12); one undo gives back the
//! texts and the shape exactly, editable, and redo welds again (criterion 22).

#![allow(clippy::unwrap_used, missing_docs)]

use std::path::PathBuf;

use curvyo_document_core::{Document, ObjectSnapshot, Point, unpack};
use curvyo_editor_wasm::{EscapeStep, KeyInput, Session, Tool, WeldOutcome};
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

/// A session with Great Vibes picked and loaded.
fn session() -> Session {
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
    let mut s = Session::new(1);
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
    s
}

/// Types `string` as a new text at `at`, then leaves to the Select tool.
fn add_text(s: &mut Session, at: Point, string: &str) {
    s.set_tool(Tool::Text);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    s.text_input(TextInput::Insert(string.to_owned()));
    load(s);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    assert_eq!(s.escape(), EscapeStep::LeftTool);
    s.set_text_size(20.0);
    load(s);
}

fn select_all(s: &mut Session) {
    s.set_tool(Tool::Select);
    s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
}

fn saved(s: &Session) -> Document {
    unpack(5, &s.pack("0.1.0").unwrap()).unwrap()
}

fn snapshots(s: &Session) -> Vec<ObjectSnapshot> {
    let document = saved(s);
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.object(id))
        .collect()
}

/// Criteria 23 and Terms: the dot of an i is its own piece.
#[test]
fn ac23_the_dot_of_an_i_is_a_piece() {
    let mut s = session();
    add_text(&mut s, Point::new(10.0, 40.0), "i");
    let outcome = s.apply_weld(0.1);
    let WeldOutcome::Applied {
        objects,
        texts_only,
        compound,
        pieces,
        ..
    } = outcome
    else {
        panic!("{outcome:?}");
    };
    assert_eq!((objects, texts_only, compound, pieces), (1, true, true, 2));
}

/// Criteria 12 and 22: two texts and a rectangle weld into one path in one step; one undo
/// restores both texts and the rectangle exactly and editable; redo welds again.
#[test]
fn ac12_ac22_texts_and_a_shape_weld_and_one_undo_restores_them() {
    let mut s = session();
    add_text(&mut s, Point::new(10.0, 40.0), "Ann");
    add_text(&mut s, Point::new(10.0, 80.0), "Bel");
    s.set_tool(Tool::Rectangle);
    s.pointer_down(Point::new(0.0, 30.0), false);
    s.pointer_up(Point::new(120.0, 90.0), false, false);
    let before = snapshots(&s);
    assert_eq!(before.len(), 3);
    select_all(&mut s);
    let undo_depth = s.history_len();
    let outcome = s.apply_weld(0.1);
    let WeldOutcome::Applied {
        objects,
        texts_only,
        pieces,
        ..
    } = outcome
    else {
        panic!("{outcome:?}");
    };
    assert_eq!((objects, texts_only, pieces), (3, false, 1));
    let welded = snapshots(&s);
    assert_eq!(welded.len(), 1);
    assert!(matches!(welded[0], ObjectSnapshot::Path(_)));
    assert_eq!(s.history_len(), undo_depth + 1, "one step");

    s.undo();
    assert_eq!(
        snapshots(&s),
        before,
        "texts and the shape come back exactly"
    );
    s.redo();
    assert_eq!(snapshots(&s), welded, "redo welds again");
    s.undo();
    assert_eq!(snapshots(&s), before);
    // Editable again: the text takes typing.
    s.set_tool(Tool::Text);
    s.pointer_down(Point::new(10.5, 40.0), false);
    s.pointer_up(Point::new(10.5, 40.0), false, false);
    s.text_input(TextInput::Insert("x".to_owned()));
    load(&mut s);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    let strings: Vec<String> = snapshots(&s)
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Text(t) => Some(t.string.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(strings.len(), 2, "no new text was made: {strings:?}");
    assert!(
        strings.iter().any(|t| t.contains('x')),
        "the restored text edits: {strings:?}"
    );
}
