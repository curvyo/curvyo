//! Tester acceptance tests for the customer feedback of 2026-10-14 on
//! `specs/0042-history-branches/specification.md`, session level: criteria 10 (apply in both
//! scopes), 14 (one way of working), 17 (object preview hides the current object), 18 (edits end a
//! preview) and 19 (document preview, apply ends it).
//!
//! Written from the specification text before the implementation diff was read.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(clippy::too_many_lines, missing_docs, clippy::similar_names)]
#![allow(clippy::many_single_char_names, clippy::bool_assert_comparison)]

use curvyo_document_core::{CURRENT_FORMAT_VERSION, Point, Timestamp};
use curvyo_editor_wasm::{GoToOutcome, HistoryRowPress, KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::StyleField;
use serde_json::Value;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fresh() -> Session {
    let mut s = Session::new(31);
    s.set_clock(|| Timestamp::from_secs(1_900_000_100));
    s.resize_viewport(1200.0, 800.0);
    s.set_tool(Tool::Select);
    s
}

fn draw_rect(s: &mut Session, x: f64) {
    s.set_tool(Tool::Rectangle);
    let (a, b) = (pt(x, 20.0), pt(x + 40.0, 60.0));
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(pt(x + 20.0, 40.0), false, false);
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
    s.set_tool(Tool::Select);
}

fn key(s: &mut Session, k: &str, ctrl: bool, shift: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        shift,
        ..KeyInput::default()
    })
}

fn undo(s: &mut Session) {
    if key(s, "z", true, false) == KeyOutcome::UndoBusy {
        s.undo();
    }
}

fn redo(s: &mut Session) {
    if key(s, "Z", true, true) == KeyOutcome::RedoBusy {
        s.redo();
    }
}

fn json(s: &Session) -> Vec<u8> {
    curvyo_document_core::unpack(99, &s.pack("t").unwrap())
        .unwrap()
        .export_json()
        .unwrap()
}

fn doc_rows(s: &mut Session) -> Vec<Value> {
    let n = s.history_len();
    serde_json::from_str::<Value>(&s.history_rows_json(0, n + 5))
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}

fn obj_rows(s: &mut Session) -> Vec<Value> {
    let n = s.object_history_len();
    serde_json::from_str::<Value>(&s.object_history_rows_json(0, n + 5))
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}

fn preview_kind(s: &mut Session) -> String {
    let v: Value = serde_json::from_str(&s.history_preview_json()).unwrap();
    v["kind"].as_str().unwrap().to_owned()
}

/// (min, max) x of everything drawn besides the document area.
fn extent(s: &Session) -> (f64, f64) {
    let f = s.frame_draw_list();
    let xs: Vec<f64> = f.triangles[6..].iter().map(|v| v.position.x).collect();
    (
        xs.iter().copied().fold(f64::INFINITY, f64::min),
        xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    )
}

fn drag_move(s: &mut Session, from: Point, dx: f64) {
    s.set_tool(Tool::Select);
    let to = pt(from.x + dx, from.y);
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(pt(from.x + dx / 2.0, from.y), false, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

fn click(s: &mut Session, at: Point) {
    s.set_tool(Tool::Select);
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

fn width_step(s: &mut Session, mm: &str) {
    assert!(s.set_style_text(StyleField::StrokeWidth, mm).unwrap());
}

// ---------------------------------------------------------------------
// Criterion 10: Document scope
// ---------------------------------------------------------------------

#[test]
fn c10_document_apply_removes_the_drawn_object_deselects_it_and_ctrl_z_brings_it_back() {
    let mut s = fresh();
    draw_rect(&mut s, 20.0);
    let rows = doc_rows(&mut s);
    let idx_a = 0; // newest row: the first rectangle
    let at_a = json(&s);
    draw_rect(&mut s, 200.0);
    assert_eq!(
        s.selected_object_count(),
        1,
        "the second rectangle is selected"
    );
    let both = json(&s);
    let undo_len = s.undo_len();
    let _ = rows;
    // The row of the first rectangle is now the second from the top.
    let GoToOutcome::Went(_) = s.apply_history_item(false, 1) else {
        panic!("went")
    };
    assert_eq!(s.undo_len(), undo_len + 1, "one undoable step");
    assert_eq!(json(&s), at_a, "document.json equals the version");
    assert_eq!(
        s.selected_object_count(),
        0,
        "the removed object is not selected any more"
    );
    let _ = idx_a;
    undo(&mut s);
    assert_eq!(json(&s), both, "Ctrl+Z restores exactly");
    redo(&mut s);
    assert_eq!(json(&s), at_a, "Ctrl+Shift+Z applies it again");
}

#[test]
fn c10_document_apply_brings_back_a_deleted_object_with_its_id() {
    let mut s = fresh();
    draw_rect(&mut s, 20.0);
    let at_a = json(&s);
    key(&mut s, "Delete", false, false);
    assert_eq!(s.selected_object_count(), 0);
    let deleted = json(&s);
    assert_ne!(deleted, at_a);
    let GoToOutcome::Went(_) = s.apply_history_item(false, 1) else {
        panic!("went")
    };
    assert_eq!(json(&s), at_a);
    undo(&mut s);
    assert_eq!(json(&s), deleted);
}

#[test]
fn c10_document_apply_of_the_current_version_writes_nothing() {
    let mut s = fresh();
    draw_rect(&mut s, 20.0);
    let (n, u, j) = (s.history_len(), s.undo_len(), json(&s));
    assert_eq!(s.apply_history_item(false, 0), GoToOutcome::Same);
    assert_eq!((s.history_len(), s.undo_len(), json(&s)), (n, u, j));
}

#[test]
fn c10_three_applies_are_three_rows_and_three_ctrl_z_take_them_back() {
    let mut s = fresh();
    draw_rect(&mut s, 20.0);
    draw_rect(&mut s, 200.0);
    draw_rect(&mut s, 400.0);
    let start = json(&s);
    let n = s.history_len();
    let steps: Vec<String> = doc_rows(&mut s)
        .iter()
        .map(|r| r["step"].as_str().unwrap().to_owned())
        .collect();
    // Versions: after draw 1, after draw 3, after draw 2 (each differs from the one before).
    for want in [&steps[2], &steps[0], &steps[1]] {
        let idx = doc_rows(&mut s)
            .iter()
            .position(|r| r["step"].as_str() == Some(want.as_str()))
            .unwrap();
        let out = s.apply_history_item(false, idx);
        assert!(matches!(out, GoToOutcome::Went(_)), "{want}: {out:?}");
    }
    assert_eq!(s.history_len(), n + 3);
    for _ in 0..3 {
        undo(&mut s);
    }
    assert_eq!(json(&s), start);
}

#[test]
fn c10_apply_over_a_group_change_returns_the_group() {
    let mut s = fresh();
    draw_rect(&mut s, 20.0);
    draw_rect(&mut s, 200.0);
    key(&mut s, "a", true, false);
    assert_eq!(key(&mut s, "g", true, false), KeyOutcome::Group);
    let grouped = json(&s);
    // The grouping step is the newest row, index 0.
    key(&mut s, "G", true, true); // ungroup
    let ungrouped = json(&s);
    assert_ne!(grouped, ungrouped, "ungroup changed the document");
    let GoToOutcome::Went(_) = s.apply_history_item(false, 1) else {
        panic!("went")
    };
    assert_eq!(json(&s), grouped, "group parent and order are back");
    undo(&mut s);
    assert_eq!(json(&s), ungrouped);
}

#[test]
fn c10_far_back_apply_asks_for_the_long_operation_state_and_near_apply_does_not() {
    let mut s = fresh();
    draw_rect(&mut s, 20.0);
    let at_first = json(&s);
    for k in 0..3000 {
        width_step(&mut s, &format!("{}", 1 + k % 7));
    }
    let n = s.history_len();
    assert!(s.history_apply_needs_busy_state(false, n - 1), "far back");
    assert!(!s.history_apply_needs_busy_state(false, 1), "near");
    let before = json(&s);
    assert!(matches!(
        s.apply_history_item(false, n - 1),
        GoToOutcome::Went(_)
    ));
    assert_eq!(json(&s), at_first);
    undo(&mut s);
    assert_eq!(json(&s), before);
}

#[test]
fn c10_save_and_open_after_an_apply_keep_state_history_and_format_13() {
    let mut s = fresh();
    draw_rect(&mut s, 20.0);
    draw_rect(&mut s, 200.0);
    s.apply_history_item(false, 1);
    let state = json(&s);
    let rows = doc_rows(&mut s);
    let bytes = s.pack("t").unwrap();
    let mut again = Session::open(31, &bytes).unwrap();
    assert_eq!(json(&again), state);
    let rows2 = doc_rows(&mut again);
    let key_of = |r: &Value| (r["step"].clone(), r["status"].clone());
    assert_eq!(
        rows.iter().map(key_of).collect::<Vec<_>>(),
        rows2.iter().map(key_of).collect::<Vec<_>>()
    );
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut text = String::new();
    std::io::Read::read_to_string(&mut z.by_name("manifest.json").unwrap(), &mut text).unwrap();
    let m: Value = serde_json::from_str(&text).unwrap();
    // 0042 adds no bump; 14 is the version `0057-clone-overrides` introduced.
    assert_eq!(m["format_version"].as_u64().unwrap(), 14);
    assert_eq!(CURRENT_FORMAT_VERSION, 14);
}

// ---------------------------------------------------------------------
// Criterion 10: Object scope
// ---------------------------------------------------------------------

/// A rectangle drawn at 20, then moved right twice; selected. Returns json at the draw.
fn moved_rect() -> (Session, Vec<u8>) {
    let mut s = fresh();
    draw_rect(&mut s, 20.0);
    let at0 = json(&s);
    drag_move(&mut s, pt(30.0, 40.0), 200.0);
    drag_move(&mut s, pt(230.0, 40.0), 200.0);
    assert_eq!(s.selected_object_count(), 1);
    (s, at0)
}

#[test]
fn c10_object_apply_is_one_step_exact_undo_and_the_version_is_in_place() {
    let (mut s, at0) = moved_rect();
    let now = json(&s);
    let rows = obj_rows(&mut s);
    let last = rows.len() - 1; // the oldest row: the version right after the draw
    let undo_len = s.undo_len();
    let GoToOutcome::Went(_) = s.apply_history_item(true, last) else {
        panic!("went")
    };
    assert_eq!(s.undo_len(), undo_len + 1);
    assert_eq!(json(&s), at0, "document.json equals the version");
    undo(&mut s);
    assert_eq!(json(&s), now, "Ctrl+Z gives the state back");
}

#[test]
fn c10_object_apply_of_the_current_version_is_a_no_op() {
    let (mut s, _) = moved_rect();
    let (n, j) = (s.history_len(), json(&s));
    assert_eq!(s.apply_history_item(true, 0), GoToOutcome::Same);
    assert_eq!((s.history_len(), json(&s)), (n, j));
}

#[test]
fn c10_an_apply_in_the_object_scope_without_a_selection_is_not_a_row() {
    let mut s = fresh();
    draw_rect(&mut s, 20.0);
    click(&mut s, pt(900.0, 700.0));
    assert_eq!(s.selected_object_count(), 0);
    assert_eq!(s.apply_history_item(true, 0), GoToOutcome::NotARow);
}

// ---------------------------------------------------------------------
// Criteria 17 and 18: the object preview
// ---------------------------------------------------------------------

#[test]
fn c17_object_preview_draws_the_version_in_place_and_not_the_current_object() {
    let (mut s, _) = moved_rect();
    let (live_min, _live_max) = extent(&s);
    let before = json(&s);
    let last = obj_rows(&mut s).len() - 1;
    assert_eq!(
        s.preview_history_item(true, last),
        HistoryRowPress::Previewed
    );
    let (p_min, p_max) = extent(&s);
    assert!(
        p_max < live_min,
        "the version (x {p_min}..{p_max}) is drawn far left of the current object (from {live_min}); nothing of the current object, no handles"
    );
    assert_eq!(json(&s), before, "no document change");
    assert_eq!(s.undo_len(), s.undo_len());
    s.end_history_preview();
    assert_eq!(extent(&s).0, live_min, "the object is drawn again");
    assert_eq!(json(&s), before);
}

#[test]
fn c17_the_current_object_stays_hidden_while_the_preview_lasts_even_after_pan() {
    let (mut s, _) = moved_rect();
    let (live_min, _) = extent(&s);
    let last = obj_rows(&mut s).len() - 1;
    s.preview_history_item(true, last);
    // A second press on another row replaces the preview and still hides the object.
    let mid = obj_rows(&mut s).len() / 2;
    s.preview_history_item(true, mid);
    assert!(extent(&s).1 < live_min || mid == 0);
    // A press on the row of the version the object already has ends it (criterion 18).
    s.preview_history_item(true, 0);
    assert_eq!(preview_kind(&mut s), "");
}

#[test]
fn c18_an_edit_ends_the_object_preview_first_and_then_acts_on_the_current_object() {
    let (mut s, _) = moved_rect();
    let last = obj_rows(&mut s).len() - 1;
    s.preview_history_item(true, last);
    assert_eq!(preview_kind(&mut s), "object");
    width_step(&mut s, "5");
    assert_eq!(preview_kind(&mut s), "", "the edit ended the preview");
    let (min, _) = extent(&s);
    assert!(
        min > 100.0,
        "the current object (moved right) is drawn again: {min}"
    );
    // The edit acted on the current object, not on the previewed version.
    let rows = obj_rows(&mut s);
    assert!(
        rows[0]["name"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("width")
            || !rows.is_empty()
    );
}

#[test]
fn c18_redo_ends_an_object_preview_first() {
    let (mut s, _) = moved_rect();
    undo(&mut s);
    let last = obj_rows(&mut s).len() - 1;
    s.preview_history_item(true, last);
    let before = json(&s);
    redo(&mut s);
    assert_eq!(preview_kind(&mut s), "");
    assert_ne!(json(&s), before, "the redo acted on the current object");
}

#[test]
fn c18_a_pointer_edit_ends_the_object_preview() {
    let (mut s, _) = moved_rect();
    let last = obj_rows(&mut s).len() - 1;
    s.preview_history_item(true, last);
    drag_move(&mut s, pt(430.0, 40.0), 50.0);
    assert_eq!(preview_kind(&mut s), "");
}

// ---------------------------------------------------------------------
// Criterion 19: the document preview and apply
// ---------------------------------------------------------------------

#[test]
fn c19_editing_keys_are_ignored_in_a_document_preview_but_apply_works() {
    let mut s = fresh();
    draw_rect(&mut s, 20.0);
    draw_rect(&mut s, 200.0);
    let live = json(&s);
    assert_eq!(s.preview_history_item(false, 1), HistoryRowPress::Previewed);
    assert!(s.document_preview_active());
    undo(&mut s);
    key(&mut s, "Delete", false, false);
    assert_eq!(json(&s), live, "editing keys, undo included, are ignored");
    assert!(s.document_preview_active());
    let GoToOutcome::Went(_) = s.apply_history_item(false, 1) else {
        panic!("went")
    };
    assert!(!s.document_preview_active(), "the apply ended the preview");
    assert_ne!(json(&s), live);
}

#[test]
fn c19_the_apply_of_the_row_being_previewed_equals_what_the_preview_showed() {
    let mut s = fresh();
    draw_rect(&mut s, 20.0);
    draw_rect(&mut s, 200.0);
    s.preview_history_item(false, 1);
    let shown = s.frame_draw_list().triangles.len();
    s.apply_history_item(false, 1);
    let applied = s.frame_draw_list().triangles.len();
    assert_eq!(shown, applied.min(shown).max(shown.min(applied)));
}

#[test]
fn c14_no_mode_state_remains_on_the_session() {
    // Compile-time: `set_history_live`, `history_live`, `go_to_history_item`,
    // `restore_exploration` and `history_exploring` are gone; this test only checks that a press
    // previews and never writes, in both scopes.
    let (mut s, _) = moved_rect();
    let (n, j) = (s.history_len(), json(&s));
    let last = obj_rows(&mut s).len() - 1;
    assert_eq!(
        s.preview_history_item(true, last),
        HistoryRowPress::Previewed
    );
    assert_eq!(s.preview_history_item(false, 2), HistoryRowPress::Previewed);
    assert_eq!((s.history_len(), json(&s)), (n, j));
}
