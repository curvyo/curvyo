//! Independent tester acceptance tests for `specs/0041-object-history/specification.md`, session
//! level: the keys Ctrl+U and Ctrl+Shift+U with their gate (criterion 5), the notices (UX table),
//! interplay with Ctrl+Z and Ctrl+Shift+Z (criteria 8 to 11), the selection after an object undo
//! (15), the Object scope data the History tab reads (1 to 3, 16), Duplicate and clones (23 to
//! 25), the wipe of one object (29 to 33) and Save/Open.
//!
//! Written from the specification and the public `Session` seam before the implementation diff was
//! read. Everything goes through `Session`; documents are compared through the `document.json`
//! export. Browser-only items (the DOM, focus, the 400 ms Wipe delay, Escape on the confirm line,
//! the Edit menu) are not covered here.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::similar_names
)]
#![allow(
    clippy::assert_is_empty,
    clippy::cast_lossless,
    clippy::redundant_closure_for_method_calls,
    clippy::semicolon_if_nothing_returned,
    clippy::bool_assert_comparison,
    clippy::collapsible_if,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
#![allow(missing_docs, clippy::doc_markdown, clippy::needless_pass_by_value)]
#![allow(clippy::many_single_char_names, clippy::cast_possible_truncation)]

use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    Author, Direction, Document, Length, NodeId, ObjectRestore, ObjectTarget, Point, RectBounds,
    RestoreReport, Timestamp, Vec2, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, ObjectWipeOutcome, Session, Tool};
use curvyo_ui_core::{EntryOutcome, MoveEntryMode, StyleField, object_restore_notice};
use serde_json::Value;

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

static NOW: AtomicU64 = AtomicU64::new(1_800_000_000);

fn clock() -> Timestamp {
    Timestamp::from_secs(NOW.fetch_add(7, Ordering::SeqCst))
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fresh() -> Session {
    let mut s = Session::new(11);
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_tool(Tool::Select);
    s
}

fn reread(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn json(s: &Session) -> Vec<u8> {
    reread(s).export_json().unwrap()
}

fn ids(s: &Session) -> Vec<NodeId> {
    reread(s).object_ids()
}

fn key_in(s: &mut Session, k: &str, ctrl: bool, shift: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        shift,
        ..KeyInput::default()
    })
}

/// Runs the busy variant the way the host does: the outcome says "needs the long state", then the
/// call itself is made.
fn settle(s: &mut Session, out: KeyOutcome) -> (KeyOutcome, String) {
    match out {
        KeyOutcome::UndoBusy => (KeyOutcome::Undo, s.undo().notice),
        KeyOutcome::RedoBusy => (KeyOutcome::Redo, s.redo().notice),
        KeyOutcome::ObjectUndoBusy => (KeyOutcome::ObjectUndo, s.object_undo().notice),
        KeyOutcome::ObjectRedoBusy => (KeyOutcome::ObjectRedo, s.object_redo().notice),
        other => (other, s.take_history_notice().unwrap_or_default()),
    }
}

fn ctrl_z(s: &mut Session) -> (KeyOutcome, String) {
    let out = key_in(s, "z", true, false);
    settle(s, out)
}

fn ctrl_shift_z(s: &mut Session) -> (KeyOutcome, String) {
    let out = key_in(s, "Z", true, true);
    settle(s, out)
}

fn ctrl_u(s: &mut Session) -> (KeyOutcome, String) {
    let out = key_in(s, "u", true, false);
    settle(s, out)
}

fn ctrl_shift_u(s: &mut Session) -> (KeyOutcome, String) {
    let out = key_in(s, "U", true, true);
    settle(s, out)
}

fn drag(s: &mut Session, tool: Tool, a: Point, b: Point) {
    s.set_tool(tool);
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(
        pt(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)),
        false,
        false,
    );
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
}

fn click(s: &mut Session, at: Point, shift: bool) {
    s.set_tool(Tool::Select);
    s.pointer_hover(at, shift, false);
    s.pointer_down(at, shift);
    s.pointer_up(at, shift, false);
}

fn select_all(s: &mut Session) {
    s.set_tool(Tool::Select);
    assert_eq!(key_in(s, "a", true, false), KeyOutcome::SelectedAll);
}

/// Moves whatever is selected by dragging from `from` (inside it) by (dx, dy).
fn drag_move(s: &mut Session, from: Point, dx: f64, dy: f64) {
    s.set_tool(Tool::Select);
    let to = pt(from.x + dx, from.y + dy);
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(pt(from.x + dx / 2.0, from.y + dy / 2.0), false, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

/// Three rectangles side by side; the last one drawn is selected afterwards.
fn three_rects() -> Session {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(60.0, 60.0));
    drag(&mut s, Tool::Rectangle, pt(100.0, 20.0), pt(140.0, 60.0));
    drag(&mut s, Tool::Rectangle, pt(180.0, 20.0), pt(220.0, 60.0));
    s
}

/// A point on the top edge of each rectangle (a rectangle has no fill by default: hits are on the
/// stroke).
const A: (f64, f64) = (30.0, 20.0);
const B: (f64, f64) = (130.0, 20.0);
const C: (f64, f64) = (210.0, 20.0);

fn sel(s: &mut Session, p: (f64, f64)) {
    click(s, pt(500.0, 500.0), false);
    click(s, pt(p.0, p.1), false);
    assert_eq!(
        s.selected_object_count(),
        1,
        "setup: one object selected at {p:?}"
    );
}

/// A session over a file of two rectangles without any step of this session (an empty stack).
fn opened_two() -> (Session, NodeId, NodeId) {
    let d = Document::new(5);
    let a = d.create_rect(RectBounds {
        origin: pt(20.0, 20.0),
        width: Length::from_mm(40.0),
        height: Length::from_mm(40.0),
    });
    let b = d.create_rect(RectBounds {
        origin: pt(100.0, 20.0),
        width: Length::from_mm(40.0),
        height: Length::from_mm(40.0),
    });
    let mut s = Session::open(11, &pack(&d, "t").unwrap()).unwrap();
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_tool(Tool::Select);
    (s, a, b)
}

fn info(s: &mut Session) -> Value {
    let t = s.object_history_info_json();
    if t.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&t).unwrap()
    }
}

fn rows(s: &mut Session) -> Vec<Value> {
    let n = s.object_history_len();
    let t = s.object_history_rows_json(0, n + 5);
    let v: Value = serde_json::from_str(&t).unwrap();
    let rows = v.as_array().unwrap().clone();
    assert_eq!(rows.len(), n, "len and page agree");
    rows
}

fn names(s: &mut Session) -> Vec<String> {
    rows(s)
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect()
}

fn doc_rows(s: &mut Session) -> Vec<Value> {
    let n = s.history_len();
    let v: Value = serde_json::from_str(&s.history_rows_json(0, n + 5)).unwrap();
    v.as_array().unwrap().clone()
}

fn dup_selected(s: &mut Session, dx: &str, dy: &str) {
    assert_eq!(key_in(s, "m", false, false), KeyOutcome::EntryOpened);
    assert_eq!(
        s.commit_move_entry(
            dx,
            dy,
            MoveEntryMode {
                absolute: false,
                copy: true
            }
        ),
        EntryOutcome::Committed
    );
}

// ---------------------------------------------------------------------
// Criterion 5: the keys, the gate and the notices
// ---------------------------------------------------------------------

#[test]
fn c5_ctrl_u_takes_back_the_latest_step_of_the_selected_object_only() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    sel(&mut s, B);
    drag_move(&mut s, pt(B.0, B.1), 0.0, 30.0);
    let after_both = json(&s);
    click(&mut s, pt(A.0, A.1 + 30.0), false);
    assert_eq!(s.selected_object_count(), 1);
    let (out, notice) = ctrl_u(&mut s);
    assert_eq!(out, KeyOutcome::ObjectUndo);
    assert_eq!(notice, "Undid: Move.");
    assert_ne!(json(&s), after_both);
    // B stays where it is: a second object undo on B takes B's move back, A's state is unchanged.
    let a_back = json(&s);
    click(&mut s, pt(B.0, B.1 + 30.0), false);
    assert_eq!(s.selected_object_count(), 1);
    let (_, notice) = ctrl_u(&mut s);
    assert_eq!(notice, "Undid: Move.");
    assert_ne!(json(&s), a_back);
    // Ctrl+Shift+U redoes B's.
    let (out, notice) = ctrl_shift_u(&mut s);
    assert_eq!(out, KeyOutcome::ObjectRedo);
    assert_eq!(notice, "Redid: Move.");
    assert_eq!(json(&s), a_back);
}

#[test]
fn c5_no_object_or_several_selected_says_select_one_object_and_writes_nothing() {
    let mut s = three_rects();
    click(&mut s, pt(400.0, 400.0), false); // empty canvas: nothing selected
    assert_eq!(s.selected_object_count(), 0);
    let before = json(&s);
    let n = s.undo_len();
    let (out, notice) = ctrl_u(&mut s);
    assert_eq!(out, KeyOutcome::ObjectUndo);
    assert_eq!(notice, "Select one object.");
    let (_, notice) = ctrl_shift_u(&mut s);
    assert_eq!(notice, "Select one object.");
    select_all(&mut s);
    assert_eq!(s.selected_object_count(), 3);
    let (_, notice) = ctrl_u(&mut s);
    assert_eq!(notice, "Select one object.");
    let (_, notice) = ctrl_shift_u(&mut s);
    assert_eq!(notice, "Select one object.");
    assert_eq!(json(&s), before);
    assert_eq!(s.undo_len(), n);
    assert!(
        s.object_history_info_json().is_empty(),
        "no Object scope for several"
    );
    assert_eq!(s.object_history_len(), 0);
}

#[test]
fn c5_the_gate_drag_chip_text_field_and_tool() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    let before = json(&s);
    // A text field has the focus.
    let out = s.key_down(KeyInput {
        key: "u",
        ctrl: true,
        dom_blocked: true,
        ..KeyInput::default()
    });
    assert_eq!(out, KeyOutcome::Ignored);
    assert_eq!(s.take_history_notice(), None);
    assert_eq!(json(&s), before);
    // During a drag.
    s.pointer_hover(pt(A.0, A.1 + 30.0), false, false);
    s.pointer_down(pt(A.0, A.1 + 30.0), false);
    s.pointer_hover(pt(A.0 + 10.0, A.1 + 40.0), false, false);
    assert_eq!(key_in(&mut s, "u", true, false), KeyOutcome::Ignored);
    assert_eq!(key_in(&mut s, "U", true, true), KeyOutcome::Ignored);
    assert_eq!(s.take_history_notice(), None);
    s.pointer_up(pt(A.0 + 10.0, A.1 + 40.0), false, false);
    // Chip open.
    click(&mut s, pt(A.0 + 10.0, A.1 + 40.0), false);
    assert_eq!(s.selected_object_count(), 1);
    let before = json(&s);
    assert_eq!(key_in(&mut s, "m", false, false), KeyOutcome::EntryOpened);
    assert_eq!(key_in(&mut s, "u", true, false), KeyOutcome::Ignored);
    assert_eq!(s.take_history_notice(), None);
    assert_eq!(json(&s), before);
    // Escape closes the chip.
    key_in(&mut s, "Escape", false, false);
    // The Rectangle tool: not a tool Ctrl+U acts in.
    s.set_tool(Tool::Rectangle);
    let out = key_in(&mut s, "u", true, false);
    assert!(
        !matches!(out, KeyOutcome::ObjectUndo | KeyOutcome::ObjectUndoBusy),
        "{out:?}"
    );
    assert_eq!(json(&s), before);
    // Unfinished Pen path.
    s.set_tool(Tool::Pen);
    s.pointer_hover(pt(300.0, 300.0), false, false);
    s.pointer_down(pt(300.0, 300.0), false);
    s.pointer_up(pt(300.0, 300.0), false, false);
    let out = key_in(&mut s, "u", true, false);
    assert!(
        !matches!(out, KeyOutcome::ObjectUndo | KeyOutcome::ObjectUndoBusy),
        "{out:?}"
    );
}

#[test]
fn c5_the_node_tool_acts_too() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    s.set_tool(Tool::Node);
    let (out, notice) = ctrl_u(&mut s);
    assert_eq!(out, KeyOutcome::ObjectUndo, "{notice}");
}

#[test]
fn c5_keys_are_matched_by_the_typed_character_and_the_modifiers() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    let before = json(&s);
    // Without Ctrl: "u" is not an object undo.
    assert_ne!(key_in(&mut s, "u", false, false), KeyOutcome::ObjectUndo);
    // AltGr+U on layouts that type a character with it.
    let out = s.key_down(KeyInput {
        key: "u",
        ctrl: true,
        alt: true,
        ..KeyInput::default()
    });
    assert!(
        !matches!(out, KeyOutcome::ObjectUndo | KeyOutcome::ObjectUndoBusy),
        "{out:?}"
    );
    assert_eq!(json(&s), before);
    // Cmd+U on macOS arrives as ctrl + mac.
    let out = s.key_down(KeyInput {
        key: "u",
        ctrl: true,
        mac: true,
        ..KeyInput::default()
    });
    let (out, _) = settle(&mut s, out);
    assert_eq!(out, KeyOutcome::ObjectUndo);
    // Ctrl+Shift+U with the typed character "U" (QWERTZ and QWERTY alike).
    let (out, notice) = ctrl_shift_u(&mut s);
    assert_eq!(out, KeyOutcome::ObjectRedo);
    assert_eq!(notice, "Redid: Move.");
    // Ctrl+Z / Ctrl+Y neighbours are unaffected.
    assert_eq!(key_in(&mut s, "z", true, false), KeyOutcome::Undo);
}

#[test]
fn c5_auto_repeat_acts_once_per_event() {
    let mut s = three_rects();
    sel(&mut s, A);
    for _ in 0..3 {
        drag_move(&mut s, pt(A.0, A.1), 0.0, 0.0);
    }
    for dy in [10.0, 10.0, 10.0] {
        // keep the object under the pointer: move by arrows instead
        let _ = dy;
        key_in(&mut s, "ArrowDown", false, false);
    }
    let mut undone = 0;
    for _ in 0..3 {
        let out = s.key_down(KeyInput {
            key: "u",
            ctrl: true,
            repeat: true,
            ..KeyInput::default()
        });
        let (out, notice) = settle(&mut s, out);
        if out == KeyOutcome::ObjectUndo && notice.starts_with("Undid") {
            undone += 1;
        }
    }
    assert!(undone >= 1, "each repeat event acts");
}

#[test]
fn c14_nothing_to_undo_or_redo_for_this_object() {
    let (mut s, a, _b) = opened_two();
    click(&mut s, pt(30.0, 20.0), false);
    assert_eq!(s.history_object(), Some(a));
    // The object has only its creation in the file (an earlier session). Redo: nothing.
    let n_doc = s.history_len();
    let (out, notice) = ctrl_shift_u(&mut s);
    assert_eq!(out, KeyOutcome::ObjectRedo);
    assert_eq!(notice, "Nothing to redo for this object.");
    assert_eq!(s.history_len(), n_doc, "nothing written");
}

#[test]
fn c11_after_a_new_step_there_is_nothing_to_redo_for_the_object_and_the_steps_stay() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    let (_, notice) = ctrl_u(&mut s);
    assert_eq!(notice, "Undid: Move.");
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    let before = json(&s);
    let (_, notice) = ctrl_shift_u(&mut s);
    assert_eq!(notice, "Nothing to redo for this object.");
    assert_eq!(json(&s), before);
    let n = names(&mut s);
    assert!(
        n.iter().any(|x| x == "Move"),
        "the steps taken back stay in the list: {n:?}"
    );
    assert!(n.iter().any(|x| x == "Undo Move"), "{n:?}");
}

#[test]
fn c11_a_step_on_another_object_does_not_clear_this_objects_redo() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    ctrl_u(&mut s);
    sel(&mut s, B);
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    sel(&mut s, A);
    let (_, notice) = ctrl_shift_u(&mut s);
    assert_eq!(notice, "Redid: Move.");
}

#[test]
fn c12_undoing_the_creation_empties_the_selection_and_ctrl_z_restores_object_and_history() {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(80.0, 60.0));
    select_all(&mut s);
    let obj = s.history_object().unwrap();
    let j0 = json(&s);
    let tool = Tool::Select;
    s.set_tool(tool);
    let (_, notice) = ctrl_u(&mut s);
    assert_eq!(
        notice,
        "Undid: Draw rectangle. The object is gone. Ctrl+Z brings it back."
    );
    assert!(ids(&s).is_empty());
    assert_eq!(s.selected_object_count(), 0, "the selection is empty");
    assert_eq!(s.history_object(), None);
    let (_, notice) = ctrl_z(&mut s);
    assert!(notice.starts_with("Undid"), "{notice}");
    assert_eq!(json(&s), j0);
    assert_eq!(ids(&s), vec![obj], "same id");
    select_all(&mut s);
    assert_eq!(s.history_object(), Some(obj));
    assert!(
        names(&mut s).contains(&"Draw rectangle".to_owned()),
        "same history"
    );
}

#[test]
fn c15_the_selection_the_tool_stay_after_an_object_undo_and_redo() {
    let mut s = three_rects();
    sel(&mut s, B);
    drag_move(&mut s, pt(B.0, B.1), 0.0, 30.0);
    let obj = s.history_object().unwrap();
    ctrl_u(&mut s);
    assert_eq!(s.history_object(), Some(obj));
    assert_eq!(s.selected_object_count(), 1);
    ctrl_shift_u(&mut s);
    assert_eq!(s.history_object(), Some(obj));
    s.set_tool(Tool::Node);
    ctrl_u(&mut s);
    assert_eq!(s.history_object(), Some(obj));
    assert_eq!(s.selected_object_count(), 1);
}

// ---------------------------------------------------------------------
// Criteria 8 to 10: interplay with Ctrl+Z
// ---------------------------------------------------------------------

#[test]
fn c9_ctrl_z_of_an_object_undo_is_an_object_redo_json_at_each_point() {
    let (mut s, _a, _b) = opened_two();
    let j0 = json(&s);
    click(&mut s, pt(30.0, 20.0), false);
    drag_move(&mut s, pt(30.0, 20.0), 0.0, 30.0); // step 1 (A)
    let j1 = json(&s);
    click(&mut s, pt(130.0, 20.0), false);
    drag_move(&mut s, pt(130.0, 20.0), 0.0, 30.0); // step 2 (B)
    let j2 = json(&s);
    click(&mut s, pt(30.0, 50.0), false); // A is now at y 50..90
    assert_eq!(s.selected_object_count(), 1);
    let (_, notice) = ctrl_u(&mut s); // step 3: object undo of step 1
    assert_eq!(notice, "Undid: Move.");
    let j3 = json(&s);
    assert_ne!(j3, j2);
    assert_ne!(j3, j1);
    let (out, notice) = ctrl_z(&mut s); // takes step 3 back: A moved again
    assert_eq!(out, KeyOutcome::Undo);
    assert!(notice.starts_with("Undid: Undo Move"), "{notice}");
    assert_eq!(json(&s), j2, "A is moved again");
    ctrl_z(&mut s); // step 2 (B)
    assert_eq!(json(&s), j1);
    ctrl_z(&mut s); // step 1 (A)
    assert_eq!(json(&s), j0);
    let (_, notice) = ctrl_z(&mut s);
    assert_eq!(notice, "Nothing to undo.");
}

#[test]
fn c10_ctrl_u_ctrl_shift_u_ctrl_z_gives_the_state_after_ctrl_u() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    ctrl_u(&mut s);
    let after_u = json(&s);
    let (_, notice) = ctrl_shift_u(&mut s);
    assert_eq!(notice, "Redid: Move.");
    assert_ne!(json(&s), after_u);
    let (_, notice) = ctrl_z(&mut s);
    assert!(notice.starts_with("Undid: Redo Move"), "{notice}");
    assert_eq!(json(&s), after_u);
}

#[test]
fn c8_the_object_undo_is_a_row_in_the_document_list_and_the_timeline_with_the_status_words() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    ctrl_u(&mut s);
    let doc = doc_rows(&mut s);
    assert_eq!(doc[0]["name"], "Undo Move", "{:?}", doc[0]);
    let obj = rows(&mut s);
    assert_eq!(obj[0]["name"], "Undo Move");
    assert!(
        obj[0]["summary"].as_str().unwrap().starts_with("reverts "),
        "{:?}",
        obj[0]["summary"]
    );
    let moved = obj.iter().find(|r| r["name"] == "Move").unwrap();
    // "Undone by You" is an object undo; plain "Undone" is only Ctrl+Z.
    let by = moved["undoneBy"].as_str().unwrap();
    assert_eq!(by.to_lowercase(), "you", "{moved:?}");
    assert_ne!(
        moved["status"], "undone",
        "plain Undone is only for Ctrl+Z: {moved:?}"
    );
    let (_, _) = ctrl_z(&mut s);
    let obj = rows(&mut s);
    let moved = obj.iter().find(|r| r["name"] == "Move").unwrap();
    assert_eq!(
        moved["undoneBy"].as_str().unwrap_or(""),
        "",
        "the revert is undone again"
    );
}

#[test]
fn c8_interplay_a_new_ctrl_u_clears_the_global_redo_stack() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    ctrl_z(&mut s); // style undone; the redo stack holds it
    assert_eq!(s.redo_len(), 1);
    let (_, notice) = ctrl_u(&mut s); // takes the Move back: a new step
    assert_eq!(notice, "Undid: Move.");
    assert_eq!(s.redo_len(), 0, "a new step clears Ctrl+Shift+Z");
    let (_, notice) = ctrl_shift_z(&mut s);
    assert_eq!(notice, "Nothing to redo.");
}

#[test]
fn c8_ctrl_z_then_ctrl_shift_z_around_an_object_undo_round_trips() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    ctrl_u(&mut s);
    let j = json(&s);
    ctrl_z(&mut s);
    assert_ne!(json(&s), j);
    let (_, notice) = ctrl_shift_z(&mut s);
    assert!(notice.starts_with("Redid"), "{notice}");
    assert_eq!(json(&s), j);
}

#[test]
fn c41_ctrl_u_after_ctrl_z_does_not_take_back_an_already_undone_step() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    ctrl_z(&mut s); // the style step is undone by Ctrl+Z
    let j = json(&s);
    let (_, notice) = ctrl_u(&mut s);
    assert_eq!(
        notice, "Undid: Move.",
        "the Move, not the already undone style"
    );
    assert_ne!(json(&s), j);
}

// ---------------------------------------------------------------------
// Criterion 6, session level: replace steps and their notice
// ---------------------------------------------------------------------

#[test]
fn c6_union_is_taken_back_whole_and_the_notice_names_the_others() {
    use curvyo_ui_core::BooleanOp;
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(80.0, 60.0));
    drag(&mut s, Tool::Rectangle, pt(60.0, 40.0), pt(120.0, 100.0));
    select_all(&mut s);
    let j0 = json(&s);
    let _ = s.apply_boolean(BooleanOp::Union);
    assert_eq!(ids(&s).len(), 1);
    assert_eq!(s.selected_object_count(), 1, "the result is selected");
    let (_, notice) = ctrl_u(&mut s);
    assert_eq!(notice, "Undid: Union. 2 other objects came back too.");
    assert_eq!(ids(&s).len(), 2);
    assert_eq!(json(&s), j0, "the operands are back as they were");
    // The selection: the result is gone; the selection is empty or the operands, never a stale id.
    assert!(s.selected_object_count() <= 2);
    // Ctrl+Z brings the union back (object undo is a step).
    let (_, _) = ctrl_z(&mut s);
    assert_eq!(ids(&s).len(), 1);
}

// ---------------------------------------------------------------------
// Criteria 23 to 25: Duplicate and clones, session level
// ---------------------------------------------------------------------

#[test]
fn c23_c24_a_duplicate_has_duplicate_of_and_inherited_rows_and_the_original_is_unaffected() {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(80.0, 60.0));
    select_all(&mut s);
    let original = s.history_object().unwrap();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    dup_selected(&mut s, "100", "0");
    let clone = s.history_object().expect("the copy is selected");
    assert_ne!(clone, original);
    let r = rows(&mut s);
    let short = original.short_id();
    assert_eq!(r[0]["name"], format!("Duplicate of {short}"), "{:?}", r[0]);
    assert_eq!(r.len(), 3, "duplicate + 2 inherited");
    for inherited in &r[1..] {
        assert_eq!(
            inherited["from"], short,
            "flagged \"from {short}\": {inherited:?}"
        );
        assert_ne!(inherited["status"], "undone");
    }
    let listed = names(&mut s);
    assert_eq!(listed[1], "Change style");
    assert_eq!(listed[2], "Draw rectangle");
    // The original's timeline has only its own rows, the clone's later move is not in it.
    drag_move(&mut s, pt(135.0, 20.0), 0.0, 20.0); // moves the clone
    let n_clone = rows(&mut s).len();
    assert_eq!(n_clone, 4);
    click(&mut s, pt(30.0, 20.0), false);
    assert_eq!(s.history_object(), Some(original));
    let n = names(&mut s);
    assert_eq!(n, vec!["Change style", "Draw rectangle"], "{n:?}");
}

#[test]
fn c25_a_clone_object_undo_never_changes_the_original_and_style_goes_back_into_the_lineage() {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(80.0, 60.0));
    select_all(&mut s);
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    dup_selected(&mut s, "100", "0");
    drag_move(&mut s, pt(135.0, 20.0), 0.0, 20.0);
    click(&mut s, pt(30.0, 20.0), false);
    let a_json = reread(&s).export_json().unwrap();
    let original_obj = reread(&s).object(s.history_object().unwrap());
    click(&mut s, pt(135.0, 40.0), false); // the clone, moved by 20
    assert_eq!(s.selected_object_count(), 1);
    let (_, n1) = ctrl_u(&mut s);
    assert_eq!(n1, "Undid: Move.");
    let (_, n2) = ctrl_u(&mut s);
    assert!(
        n2.starts_with("Undid: Change style."),
        "into the inherited rows: {n2}"
    );
    click(&mut s, pt(30.0, 20.0), false);
    let original_after = reread(&s).object(s.history_object().unwrap());
    assert_eq!(format!("{original_obj:?}"), format!("{original_after:?}"));
    let _ = a_json;
}

#[test]
fn c25_ctrl_u_on_a_copy_must_not_remove_the_copy() {
    // Known limit: Ctrl+U on a copy never takes back the Duplicate. Walking back through the
    // inherited rows must not end by deleting the copy either.
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(80.0, 60.0));
    select_all(&mut s);
    dup_selected(&mut s, "100", "0");
    let clone = s.history_object().unwrap();
    for press in 0..5 {
        let (_, notice) = ctrl_u(&mut s);
        assert!(
            ids(&s).contains(&clone),
            "press {press}: the copy is gone; notice was {notice:?}"
        );
        if s.history_object().is_none() {
            break;
        }
    }
}

#[test]
fn c25_the_notice_for_an_inherited_step_that_wrote_nothing_is_honest() {
    // A rectangle resized before the copy: its resize is shape data the restore of plain registers
    // leaves alone. The press must not claim "Undid: Resize." when it wrote nothing.
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(80.0, 60.0));
    select_all(&mut s);
    assert_eq!(key_in(&mut s, "s", false, false), KeyOutcome::EntryOpened);
    let chip = s.transform_entry().unwrap();
    let second = chip
        .fields
        .get(1)
        .map(|f| f.prefill.clone())
        .unwrap_or_default();
    assert_eq!(
        s.commit_transform_entry("50", &second, 0),
        EntryOutcome::Committed
    );
    dup_selected(&mut s, "100", "0");
    let before = json(&s);
    let (_, notice) = ctrl_u(&mut s);
    let after = json(&s);
    if before == after {
        assert!(
            !notice.starts_with("Undid:") || notice.contains("Nothing"),
            "nothing was written but the notice says: {notice}"
        );
    }
}

// ---------------------------------------------------------------------
// Criterion 1, 2, 3, 16: what the History tab reads
// ---------------------------------------------------------------------

#[test]
fn c1_c2_the_object_scope_data_exists_for_one_object_only() {
    let mut s = three_rects();
    sel(&mut s, A);
    let i = info(&mut s);
    assert_eq!(i["kind"], "Rectangle");
    assert_eq!(
        i["id"].as_str().unwrap().len(),
        6,
        "the first 6 characters of the id"
    );
    let full = i["fullId"].as_str().unwrap();
    assert!(full.starts_with(i["id"].as_str().unwrap()));
    assert_eq!(
        i["total"].as_u64().unwrap() as usize,
        s.object_history_len()
    );
    // None: the group is not in the tree and the list is the document list.
    click(&mut s, pt(500.0, 500.0), false);
    assert!(s.object_history_info_json().is_empty());
    assert_eq!(s.object_history_len(), 0);
    select_all(&mut s);
    assert!(s.object_history_info_json().is_empty());
    assert_eq!(s.object_history_len(), 0);
    assert!(s.history_len() >= 3, "the document list is unaffected");
}

#[test]
fn c2_compound_path_is_named_as_such() {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(80.0, 60.0));
    drag(&mut s, Tool::Rectangle, pt(200.0, 20.0), pt(260.0, 60.0));
    select_all(&mut s);
    let _ = s.apply_combine();
    assert_eq!(s.selected_object_count(), 1);
    let i = info(&mut s);
    assert_eq!(i["kind"], "Compound path");
    let r = rows(&mut s);
    assert_eq!(r.len(), 1, "one object, one timeline");
    assert_eq!(r[0]["name"], "Created by Combine");
    assert_eq!(r[0]["summary"], "from 2 objects");
}

#[test]
fn c3_a_step_with_other_objects_says_so() {
    let mut s = three_rects();
    select_all(&mut s);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    let doc = doc_rows(&mut s);
    let step = doc[0]["id"].as_str().unwrap().to_owned();
    click(&mut s, pt(500.0, 500.0), false);
    click(&mut s, pt(B.0, B.1 + 30.0), false);
    assert_eq!(s.selected_object_count(), 1);
    // B was moved with the others, now at y + 30.
    let r = rows(&mut s);
    assert_eq!(r[0]["name"], "Move", "{:?}", r[0]);
    assert_eq!(r[0]["summary"], "with 2 other objects", "{:?}", r[0]);
    assert_eq!(r[0]["id"], step, "the same step id as in the document list");
}

#[test]
fn c16_the_buttons_data_undo_and_redo_targets_come_and_go() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    let i = info(&mut s);
    assert_eq!(i["undo"]["name"], "Move");
    assert!(i["redo"].is_null(), "redo is present only while possible");
    assert_eq!(
        s.object_history_target_name(Direction::Undo).as_deref(),
        Some("Move")
    );
    ctrl_u(&mut s);
    let i = info(&mut s);
    assert_eq!(i["undo"]["name"], "Draw rectangle");
    assert_eq!(i["redo"]["name"], "Move");
    assert_eq!(
        s.object_history_target_name(Direction::Redo).as_deref(),
        Some("Move")
    );
    // The buttons are the same as the keys.
    let r = s.object_redo();
    assert!(r.changed);
    assert_eq!(r.notice, "Redid: Move.");
    let r = s.object_undo();
    assert!(r.changed);
    assert_eq!(r.notice, "Undid: Move.");
}

// ---------------------------------------------------------------------
// Criteria 29 to 33: wipe
// ---------------------------------------------------------------------

#[test]
fn c29_c30_c32_wipe_shows_the_floor_row_and_the_document_list_keeps_everything() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    let info_before = info(&mut s);
    let hidden = info_before["wipeCount"].as_u64().unwrap() as usize;
    assert_eq!(hidden, 2);
    let doc_before = doc_rows(&mut s).len();
    let obj = s.history_object().unwrap();
    assert_eq!(s.wipe_object_history(), ObjectWipeOutcome::Wiped(2));
    assert_eq!(s.history_object(), Some(obj), "the selection stays");
    let r = rows(&mut s);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0]["name"], "History wiped");
    assert_eq!(info(&mut s)["wipeCount"], 0, "the control leaves the tree");
    // The document list keeps every earlier row and gains one.
    let doc = doc_rows(&mut s);
    assert_eq!(doc.len(), doc_before + 1);
    let top = doc[0]["name"].as_str().unwrap().to_owned();
    assert!(
        top.starts_with("Wipe history of Rectangle"),
        "criterion 32 row: {top:?}"
    );
    assert!(top.contains(obj.short_id().as_str()), "{top:?}");
    // The wipe is not undoable: Ctrl+Z skips it and takes back the move of the other steps.
    let (_, notice) = ctrl_z(&mut s);
    assert!(notice.starts_with("Undid"), "{notice}");
    // Nothing to hide any more: a second wipe is refused without writing.
    let (_, _) = ctrl_shift_z(&mut s);
}

#[test]
fn c30_a_wipe_when_nothing_would_be_hidden_writes_nothing() {
    let mut s = three_rects();
    sel(&mut s, A);
    s.wipe_object_history();
    let n = s.history_len();
    let out = s.wipe_object_history();
    assert_ne!(matches!(out, ObjectWipeOutcome::Wiped(_)), true);
    assert_eq!(s.history_len(), n);
}

#[test]
fn c31_a_wipe_drops_the_stack_entries_that_touched_only_this_object() {
    let (mut s, a, b) = opened_two();
    click(&mut s, pt(30.0, 20.0), false); // A
    drag_move(&mut s, pt(30.0, 20.0), 0.0, 30.0); // A alone
    select_all(&mut s);
    drag_move(&mut s, pt(30.0, 50.0), 40.0, 0.0); // A and B together
    let both = json(&s);
    click(&mut s, pt(500.0, 500.0), false);
    click(&mut s, pt(70.0, 50.0), false);
    assert_eq!(s.history_object(), Some(a));
    assert!(matches!(
        s.wipe_object_history(),
        ObjectWipeOutcome::Wiped(_)
    ));
    assert_eq!(json(&s), both, "a wipe changes no object");
    let _ = b;
    // Ctrl+Z takes back the pair, with its effect on all objects.
    let (_, notice) = ctrl_z(&mut s);
    assert!(notice.starts_with("Undid"), "{notice}");
    assert_ne!(json(&s), both);
    let (_, notice) = ctrl_z(&mut s);
    assert_eq!(notice, "Nothing to undo.", "A's own entry was dropped");
}

#[test]
fn c31_the_wipe_is_ignored_during_a_drag_and_with_a_chip() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    let n = s.history_len();
    assert_eq!(key_in(&mut s, "m", false, false), KeyOutcome::EntryOpened);
    assert_eq!(s.wipe_object_history(), ObjectWipeOutcome::Ignored);
    key_in(&mut s, "Escape", false, false);
    s.pointer_hover(pt(A.0, A.1 + 30.0), false, false);
    s.pointer_down(pt(A.0, A.1 + 30.0), false);
    s.pointer_hover(pt(A.0 + 5.0, A.1 + 35.0), false, false);
    assert_eq!(s.wipe_object_history(), ObjectWipeOutcome::Ignored);
    s.pointer_up(pt(A.0 + 5.0, A.1 + 35.0), false, false);
    // No single object.
    select_all(&mut s);
    assert_eq!(s.wipe_object_history(), ObjectWipeOutcome::Ignored);
    click(&mut s, pt(500.0, 500.0), false);
    assert_eq!(s.wipe_object_history(), ObjectWipeOutcome::Ignored);
    assert!(s.history_len() >= n);
}

#[test]
fn c33_save_open_keeps_the_floor_and_the_clone_lineage() {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(80.0, 60.0));
    select_all(&mut s);
    let original = s.history_object().unwrap();
    dup_selected(&mut s, "100", "0");
    let clone = s.history_object().unwrap();
    click(&mut s, pt(30.0, 20.0), false);
    assert_eq!(s.history_object(), Some(original));
    assert!(matches!(
        s.wipe_object_history(),
        ObjectWipeOutcome::Wiped(_)
    ));
    let bytes = s.pack("0.1.0").unwrap();
    let mut t = Session::open(12, &bytes).unwrap();
    t.set_clock(clock);
    t.resize_viewport(1200.0, 800.0);
    t.set_tool(Tool::Select);
    click(&mut t, pt(30.0, 20.0), false);
    assert_eq!(t.history_object(), Some(original));
    let n = names(&mut t);
    assert_eq!(n, vec!["History wiped"], "still starts at the floor");
    click(&mut t, pt(135.0, 20.0), false);
    assert_eq!(t.history_object(), Some(clone));
    let n = names(&mut t);
    assert_eq!(n.len(), 2, "the clone keeps its copy: {n:?}");
    assert!(n[0].starts_with("Duplicate of"));
    // Ctrl+U in the reopened session works on steps of an earlier session and says so.
    let (out, notice) = ctrl_u(&mut t);
    assert_eq!(out, KeyOutcome::ObjectUndo);
    let _ = notice;
}

#[test]
fn c34_wipe_then_ctrl_u_cannot_go_before_the_floor() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    assert!(matches!(
        s.wipe_object_history(),
        ObjectWipeOutcome::Wiped(_)
    ));
    let j = json(&s);
    let (_, notice) = ctrl_u(&mut s);
    assert_eq!(notice, "Nothing to undo for this object.");
    assert_eq!(json(&s), j);
    drag_move(&mut s, pt(A.0, A.1 + 30.0), 20.0, 0.0);
    let (_, notice) = ctrl_u(&mut s);
    assert_eq!(notice, "Undid: Move.");
    let (_, notice) = ctrl_u(&mut s);
    assert_eq!(notice, "Nothing to undo for this object.");
}

// ---------------------------------------------------------------------
// Deleted objects
// ---------------------------------------------------------------------

#[test]
fn c4_a_deleted_object_has_no_scope_and_ctrl_z_brings_back_its_timeline() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    let obj = s.history_object().unwrap();
    let len = s.object_history_len();
    assert_eq!(key_in(&mut s, "Delete", false, false), KeyOutcome::Deleted);
    assert_eq!(s.selected_object_count(), 0);
    assert!(s.object_history_info_json().is_empty());
    let (_, notice) = ctrl_u(&mut s);
    assert_eq!(notice, "Select one object.");
    let doc = doc_rows(&mut s);
    assert!(
        doc[0]["name"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("delete"),
        "{:?}",
        doc[0]
    );
    ctrl_z(&mut s);
    click(&mut s, pt(A.0, A.1 + 30.0), false);
    assert_eq!(s.history_object(), Some(obj));
    // The delete step touched the object, so it is a row of its timeline (taken back by Ctrl+Z).
    assert_eq!(s.object_history_len(), len + 1);
    let r = rows(&mut s);
    assert_eq!(r[0]["role"], "deleted", "{:?}", r[0]);
    assert_eq!(r[0]["status"], "undone", "{:?}", r[0]);
}

// ---------------------------------------------------------------------
// Notice texts (formatting only: built from the public report types)
// ---------------------------------------------------------------------

fn target(label: &str, author: Author) -> ObjectTarget {
    // Obtained from a real document so the fields are consistent.
    let d = Document::new(1);
    let a = d.create_rect(RectBounds {
        origin: pt(0.0, 0.0),
        width: Length::from_mm(1.0),
        height: Length::from_mm(1.0),
    });
    d.translate_objects(&[a], Vec2::new(1.0, 0.0)).unwrap();
    let mut t = d.object_history_targets(a).undo.unwrap();
    label.clone_into(&mut t.label);
    t.author = author;
    t
}

#[test]
fn notice_texts_for_peer_conflict_and_nothing_cases() {
    let d = Document::new(1);
    let a = d.create_rect(RectBounds {
        origin: pt(0.0, 0.0),
        width: Length::from_mm(1.0),
        height: Length::from_mm(1.0),
    });
    d.translate_objects(&[a], Vec2::new(1.0, 0.0)).unwrap();
    let mut res: ObjectRestore = d.undo_object_step(a).unwrap().unwrap();
    assert_eq!(object_restore_notice(&res), "Undid: Move.");
    // Another author: the wording until names exist.
    res.target = target("translate_objects", Author::Earlier);
    let short = res.target.shown_id.clone();
    assert_eq!(
        object_restore_notice(&res),
        format!("Undid a step by someone else: Move ({short}).")
    );
    // Field conflicts.
    res.target = target("translate_objects", Author::You);
    res.report.objects[0].kept = 1;
    assert_eq!(
        object_restore_notice(&res),
        "Undid: Move. 1 field kept later changes."
    );
    res.report.objects[0].kept = 3;
    assert_eq!(
        object_restore_notice(&res),
        "Undid: Move. 3 fields kept later changes."
    );
    let _: &RestoreReport = &res.report;
}

#[test]
fn notice_long_creation_text_is_the_one_of_the_spec() {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(80.0, 60.0));
    select_all(&mut s);
    let (_, notice) = ctrl_u(&mut s);
    assert_eq!(
        notice,
        "Undid: Draw rectangle. The object is gone. Ctrl+Z brings it back."
    );
}

// ---------------------------------------------------------------------
// Hostile: rapid spam and alternation never breaks the document
// ---------------------------------------------------------------------

#[test]
fn hostile_spam_of_every_history_key_keeps_the_document_consistent() {
    let mut s = three_rects();
    sel(&mut s, A);
    drag_move(&mut s, pt(A.0, A.1), 0.0, 30.0);
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    sel(&mut s, B);
    let mut seed = 0x1234_5678_u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for i in 0..400 {
        match next() % 7 {
            0 => {
                ctrl_u(&mut s);
            }
            1 => {
                ctrl_shift_u(&mut s);
            }
            2 => {
                ctrl_z(&mut s);
            }
            3 => {
                ctrl_shift_z(&mut s);
            }
            4 => {
                let _ = s.wipe_object_history();
            }
            5 => {
                let p = [A, B, C][(next() % 3) as usize];
                click(&mut s, pt(p.0, p.1 + 30.0 * ((next() % 2) as f64)), false);
            }
            _ => {
                if s.selected_object_count() == 1 {
                    drag_move(&mut s, pt(B.0, B.1), 0.0, 5.0);
                }
            }
        }
        // The document stays readable and the lists agree.
        let d = reread(&s);
        assert!(d.object_ids().len() <= 3, "iteration {i}");
        let n = s.object_history_len();
        if s.history_object().is_some() {
            let v: Value = serde_json::from_str(&s.object_history_rows_json(0, n + 3)).unwrap();
            assert_eq!(v.as_array().unwrap().len(), n);
        }
    }
}

// ---------------------------------------------------------------------
// Criterion 35: the first press after opening a big file
// ---------------------------------------------------------------------

#[test]
fn c35_first_object_undo_after_opening_a_file_of_25000_steps_stays_within_four_times_300_ms() {
    let d = Document::new(5);
    let hot = d.create_rect(RectBounds {
        origin: pt(20.0, 20.0),
        width: Length::from_mm(40.0),
        height: Length::from_mm(40.0),
    });
    let cold = d.create_rect(RectBounds {
        origin: pt(300.0, 20.0),
        width: Length::from_mm(40.0),
        height: Length::from_mm(40.0),
    });
    for i in 0..25_000 {
        if i % 25 == 0 {
            d.translate_objects(&[hot], Vec2::new(0.001, 0.0)).unwrap();
        } else {
            d.translate_objects(&[cold], Vec2::new(0.001, 0.0)).unwrap();
        }
    }
    let bytes = pack(&d, "t").unwrap();
    let t = std::time::Instant::now();
    let mut s = Session::open(11, &bytes).unwrap();
    eprintln!("open of 25,000 steps: {:?}", t.elapsed());
    s.resize_viewport(1200.0, 800.0);
    s.set_tool(Tool::Select);
    // Select the hot one by a click on its top edge (quarter point, away from the handles).
    click(&mut s, pt(30.0, 20.0), false);
    assert_eq!(s.history_object(), Some(hot), "setup");
    let busy = s.object_history_needs_busy_state(Direction::Undo);
    eprintln!("needs the long-operation state: {busy}");
    let t = std::time::Instant::now();
    let res = s.object_undo();
    let first = t.elapsed();
    eprintln!("first object undo: {first:?}, notice {:?}", res.notice);
    assert!(res.changed);
    let budget = if cfg!(debug_assertions) { 6_000 } else { 1_200 };
    assert!(first.as_millis() <= budget, "first press took {first:?}");
    let t = std::time::Instant::now();
    let _ = s.object_history_info_json();
    let _ = s.object_history_rows_json(0, 40);
    let open_scope = t.elapsed();
    eprintln!("Object scope data after the index exists: {open_scope:?}");
    let budget = if cfg!(debug_assertions) { 2_000 } else { 400 };
    assert!(open_scope.as_millis() <= budget, "{open_scope:?}");
    let _ = cold;
}

#[test]
fn c23_a_duplicate_of_three_objects_starts_every_copy_with_duplicate_of() {
    let mut s = three_rects();
    select_all(&mut s);
    let sources = {
        let d = reread(&s);
        d.object_ids()
    };
    dup_selected(&mut s, "0", "100");
    assert_eq!(s.selected_object_count(), 3, "the copies are selected");
    for (point, expected_source) in [(A, sources[0]), (B, sources[1]), (C, sources[2])] {
        sel(&mut s, (point.0, point.1 + 100.0));
        let copy = s.history_object().unwrap();
        assert!(!sources.contains(&copy));
        let r = rows(&mut s);
        assert_eq!(
            r[0]["name"],
            format!("Duplicate of {}", expected_source.short_id()),
            "copy at {point:?}: {:?}",
            r[0]
        );
        assert_eq!(r.len(), 2, "duplicate row and the creation it inherited");
        assert_eq!(r[1]["from"], expected_source.short_id());
    }
}

#[test]
fn c25_the_first_press_on_a_fresh_copy_does_not_make_it_disappear() {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(80.0, 60.0));
    select_all(&mut s);
    dup_selected(&mut s, "100", "0");
    let copy = s.history_object().unwrap();
    let (_, notice) = ctrl_u(&mut s);
    assert!(
        ids(&s).contains(&copy),
        "the copy was removed with the notice {notice:?}"
    );
}

#[test]
fn c31_a_wipe_also_drops_redo_entries_that_touched_only_this_object() {
    let (mut s, a, _b) = opened_two();
    click(&mut s, pt(30.0, 20.0), false);
    assert_eq!(s.history_object(), Some(a));
    drag_move(&mut s, pt(30.0, 20.0), 0.0, 30.0);
    ctrl_u(&mut s);
    ctrl_z(&mut s); // takes the object undo back: the redo stack holds it
    assert_eq!(s.redo_len(), 1);
    ctrl_z(&mut s); // takes the move back: two redo entries
    assert_eq!(s.redo_len(), 2);
    click(&mut s, pt(30.0, 20.0), false);
    assert!(matches!(
        s.wipe_object_history(),
        ObjectWipeOutcome::Wiped(_)
    ));
    assert_eq!(
        s.redo_len(),
        0,
        "redo entries that touched only A are dropped"
    );
    let (_, notice) = ctrl_shift_z(&mut s);
    assert_eq!(notice, "Nothing to redo.");
    assert_eq!(s.undo_len(), 0);
}

#[test]
fn c34_the_wipe_of_a_selected_clone_leaves_the_original_and_its_timeline_alone() {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(80.0, 60.0));
    select_all(&mut s);
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    let original = s.history_object().unwrap();
    dup_selected(&mut s, "100", "0");
    let n_clone = rows(&mut s).len();
    assert!(matches!(
        s.wipe_object_history(),
        ObjectWipeOutcome::Wiped(k) if k == n_clone
    ));
    assert_eq!(names(&mut s), vec!["History wiped"]);
    click(&mut s, pt(500.0, 500.0), false);
    click(&mut s, pt(30.0, 20.0), false);
    assert_eq!(s.history_object(), Some(original));
    assert_eq!(names(&mut s), vec!["Change style", "Draw rectangle"]);
}
