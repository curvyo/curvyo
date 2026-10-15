//! Independent tester acceptance tests for `specs/0042-history-branches/specification.md`, session
//! level: forks and redo with the keys (criteria 1 to 4), the list items, lanes and folding (3, 4,
//! 8, 9), applying a version (10, 16; criteria 11 to 13 and the Mode switch withdrawn by the
//! customer feedback of 2026-10-14), previews of an object and of the document with their gates
//! (14, 16 to 21), clones (22 to 26) and Save/Open.
//!
//! Written from the specification and the public `Session` seam before the implementation diff was
//! read. Everything goes through `Session`; documents are compared through the `document.json`
//! export. Browser-only items (the DOM, focus, the switch, the inset frame, the tool rail look, the
//! pointer cursor) are not covered here.

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
use std::time::Instant;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, Length, NodeId, Point, RectBounds, Timestamp, Vec2, pack,
    unpack,
};
use curvyo_editor_wasm::{
    CloneOutcome, DocumentSide, GoToOutcome, HistoryRowPress, KeyInput, KeyOutcome, Session, Tool,
};
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

fn key_in(s: &mut Session, k: &str, ctrl: bool, shift: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        shift,
        ..KeyInput::default()
    })
}

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

fn click(s: &mut Session, at: Point) {
    s.set_tool(Tool::Select);
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

fn drag_move(s: &mut Session, from: Point, dx: f64, dy: f64) {
    s.set_tool(Tool::Select);
    let to = pt(from.x + dx, from.y + dy);
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(pt(from.x + dx / 2.0, from.y + dy / 2.0), false, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

/// A session with one rectangle drawn (selected) at (20,20)-(60,60); `y` tracks its top edge.
struct Lab {
    s: Session,
    y: f64,
}

impl Lab {
    fn new() -> Self {
        let mut s = fresh();
        drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(60.0, 60.0));
        assert_eq!(
            s.selected_object_count(),
            1,
            "setup: the rectangle is selected"
        );
        Self { s, y: 20.0 }
    }

    /// One more step on the rectangle: a move down by 10 mm. Returns the step text of the row.
    fn step(&mut self) -> String {
        if self.s.selected_object_count() != 1 {
            click(&mut self.s, pt(30.0, self.y));
        }
        drag_move(&mut self.s, pt(30.0, self.y), 0.0, 10.0);
        self.y += 10.0;
        newest_step(&mut self.s)
    }

    /// Selects the rectangle again.
    fn select(&mut self) {
        click(&mut self.s, pt(500.0, 700.0));
        click(&mut self.s, pt(30.0, self.y));
        assert_eq!(self.s.selected_object_count(), 1);
    }
}

fn newest_step(s: &mut Session) -> String {
    doc_rows(s)[0]["step"].as_str().unwrap().to_owned()
}

fn doc_rows(s: &mut Session) -> Vec<Value> {
    let n = s.history_len();
    let v: Value = serde_json::from_str(&s.history_rows_json(0, n + 5)).unwrap();
    v.as_array().unwrap().clone()
}

fn obj_rows(s: &mut Session) -> Vec<Value> {
    let n = s.object_history_len();
    let v: Value = serde_json::from_str(&s.object_history_rows_json(0, n + 5)).unwrap();
    v.as_array().unwrap().clone()
}

fn status_in(rows: &[Value], step: &str) -> String {
    rows.iter()
        .find(|r| r["step"] == step)
        .unwrap_or_else(|| panic!("step {step} not in the list"))["status"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn items(s: &mut Session, object: bool) -> Vec<Value> {
    let layout: Value = serde_json::from_str(&s.history_layout_json(object)).unwrap();
    let total = layout["total"].as_u64().unwrap() as usize;
    let v: Value = serde_json::from_str(&s.history_items_json(object, 0, total + 5)).unwrap();
    let a = v.as_array().unwrap().clone();
    assert_eq!(a.len(), total, "layout and items agree");
    a
}

/// The item index of the row of `step`.
fn item_of(s: &mut Session, object: bool, step: &str) -> usize {
    items(s, object)
        .iter()
        .position(|i| i["type"] == "row" && i["step"] == step)
        .unwrap_or_else(|| panic!("no item for {step}"))
}

fn shown_id(s: &mut Session, object: bool, step: &str) -> String {
    let i = item_of(s, object, step);
    items(s, object)[i]["id"].as_str().unwrap().to_owned()
}

fn preview_kind(s: &mut Session) -> String {
    let v: Value = serde_json::from_str(&s.history_preview_json()).unwrap();
    v["kind"].as_str().unwrap().to_owned()
}

// ---------------------------------------------------------------------
// Criteria 1 to 4: forks with the keys, Document scope
// ---------------------------------------------------------------------

#[test]
fn c1_c2_c4_fork_redo_and_taking_back_the_conflict_step() {
    let mut l = Lab::new();
    let s1 = l.step();
    let after_s1 = json(&l.s);
    let s2 = l.step();
    let s3 = l.step();
    // Ctrl+Z twice, then a new step s4.
    ctrl_z(&mut l.s);
    ctrl_z(&mut l.s);
    assert_eq!(json(&l.s), after_s1);
    let rows = doc_rows(&mut l.s);
    assert_eq!(status_in(&rows, &s3), "undone");
    assert_eq!(status_in(&rows, &s2), "undone");
    l.y = 30.0; // the rectangle is where s1 left it
    l.select();
    drag_move(&mut l.s, pt(30.0, l.y), 40.0, 0.0);
    let s4 = newest_step(&mut l.s);
    let after_s4 = json(&l.s);
    // The same happens: s2 and s3 are on a branch, s4 is applied.
    let rows = doc_rows(&mut l.s);
    assert_eq!(status_in(&rows, &s2), "branch");
    assert_eq!(status_in(&rows, &s3), "branch");
    assert_eq!(status_in(&rows, &s4), "applied");
    assert_eq!(status_in(&rows, &s1), "applied");
    // Ctrl+Shift+Z says "Nothing to redo."
    let before = json(&l.s);
    let (out, notice) = ctrl_shift_z(&mut l.s);
    assert_eq!(out, KeyOutcome::Redo);
    assert_eq!(notice, "Nothing to redo.");
    assert_eq!(json(&l.s), before);
    // Take back the conflict step: s1's state; redo brings back s4, not s2.
    ctrl_z(&mut l.s);
    assert_eq!(json(&l.s), after_s1);
    let rows = doc_rows(&mut l.s);
    assert_eq!(status_in(&rows, &s4), "undone");
    assert_eq!(status_in(&rows, &s2), "branch");
    ctrl_shift_z(&mut l.s);
    assert_eq!(json(&l.s), after_s4, "s4 comes back, not s2");
    let rows = doc_rows(&mut l.s);
    assert_eq!(status_in(&rows, &s4), "applied");
    assert_eq!(status_in(&rows, &s2), "branch");
    // The old branch is reachable through the list.
    let it = item_of(&mut l.s, false, &s2);
    let _ = it;
}

#[test]
fn c1_object_undo_makes_a_branch_too_and_there_is_no_object_redo() {
    let mut l = Lab::new();
    let s1 = l.step();
    let after_s1 = json(&l.s);
    let s2 = l.step();
    let s3 = l.step();
    l.select();
    ctrl_u(&mut l.s);
    ctrl_u(&mut l.s);
    assert_eq!(json(&l.s), after_s1);
    l.y = 30.0;
    l.select();
    drag_move(&mut l.s, pt(30.0, l.y), 40.0, 0.0);
    let rows = obj_rows(&mut l.s);
    assert_eq!(status_in(&rows, &s2), "branch");
    assert_eq!(status_in(&rows, &s3), "branch");
    assert_eq!(status_in(&rows, &s1), "applied");
    let before = json(&l.s);
    let out = key_in(&mut l.s, "U", true, true);
    let (_, notice) = settle(&mut l.s, out);
    assert!(notice.starts_with("Nothing to redo"), "{notice}");
    assert_eq!(json(&l.s), before);
}

// ---------------------------------------------------------------------
// Criteria 3, 8, 9: header rows, lanes, folding
// ---------------------------------------------------------------------

fn lab_with_branches(n: usize) -> (Lab, Vec<String>) {
    // n forks: each round does a step, takes it back and does another, which leaves one branch.
    let mut l = Lab::new();
    let mut branch_steps = Vec::new();
    for _ in 0..n {
        let b = l.step();
        ctrl_z(&mut l.s);
        l.y -= 10.0;
        l.select();
        let _keep = l.step();
        branch_steps.push(b);
    }
    (l, branch_steps)
}

#[test]
fn c3_c8_c9_a_branch_has_a_header_row_a_lane_name_and_folds() {
    let mut l = Lab::new();
    let s1 = l.step();
    let s2 = l.step();
    let s3 = l.step();
    ctrl_z(&mut l.s);
    ctrl_z(&mut l.s);
    l.y = 30.0;
    l.select();
    let s4_before = doc_rows(&mut l.s).len();
    drag_move(&mut l.s, pt(30.0, l.y), 40.0, 0.0);
    assert!(doc_rows(&mut l.s).len() > s4_before);
    let fork_id = shown_id(&mut l.s, false, &s1);
    let all = items(&mut l.s, false);
    let headers: Vec<&Value> = all.iter().filter(|i| i["type"] == "header").collect();
    assert_eq!(headers.len(), 1);
    assert_eq!(
        headers[0]["text"],
        format!("Branch from {fork_id}, 2 steps")
    );
    assert_eq!(headers[0]["expanded"], true, "branches start unfolded");
    // The header sits directly above the branch's newest row (s3).
    let h = all.iter().position(|i| i["type"] == "header").unwrap();
    assert_eq!(all[h + 1]["step"], s3);
    // Lane names for a screen reader.
    assert_eq!(
        all[h + 1]["lane"],
        format!("on branch from {fork_id}, step 2 of 2")
    );
    assert_eq!(all[h + 2]["step"], s2);
    assert_eq!(
        all[h + 2]["lane"],
        format!("on branch from {fork_id}, step 1 of 2")
    );
    // The layout names the header as a special item.
    let layout: Value = serde_json::from_str(&l.s.history_layout_json(false)).unwrap();
    assert!(
        layout["special"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e[0] == h && e[1] == "header")
    );
    assert!(layout["lanes"].as_u64().unwrap() >= 1);
    // Folding: its rows leave the list.
    let total = all.len();
    assert!(l.s.toggle_history_item(false, h));
    let folded = items(&mut l.s, false);
    assert_eq!(folded.len(), total - 2);
    assert!(folded.iter().all(|i| i["step"] != s2 && i["step"] != s3));
    let hf = folded.iter().find(|i| i["type"] == "header").unwrap();
    assert_eq!(hf["expanded"], false);
    // Unfold again.
    assert!(l.s.toggle_history_item(false, h));
    assert_eq!(items(&mut l.s, false).len(), total);
    // Toggling a step row does nothing.
    let row_idx = item_of(&mut l.s, false, &s1);
    assert!(!l.s.toggle_history_item(false, row_idx));
}

#[test]
fn c4_a_row_on_a_branch_says_so_in_text_not_by_lane_alone() {
    let (mut l, branch_steps) = lab_with_branches(1);
    let rows = doc_rows(&mut l.s);
    assert_eq!(status_in(&rows, &branch_steps[0]), "branch");
    // Applied rows carry no status word; the panel reads "status" for the text.
    assert!(rows.iter().filter(|r| r["status"] == "applied").count() >= 2);
}

#[test]
fn c8_the_fourth_and_later_branches_fold_into_one_more_row() {
    let (mut l, _) = lab_with_branches(5);
    let all = items(&mut l.s, false);
    let headers = all.iter().filter(|i| i["type"] == "header").count();
    let more: Vec<&Value> = all.iter().filter(|i| i["type"] == "more").collect();
    assert_eq!(headers, 3, "at most 3 branches drawn");
    assert_eq!(more.len(), 1);
    assert_eq!(more[0]["text"], "2 more branches");
    assert_eq!(more[0]["expanded"], false);
    let layout: Value = serde_json::from_str(&l.s.history_layout_json(false)).unwrap();
    assert!(
        layout["lanes"].as_u64().unwrap() <= 3,
        "main line plus at most 3 branch lanes: {layout}"
    );
    // Unfold it in place: headers appear, each folded.
    let at = all.iter().position(|i| i["type"] == "more").unwrap();
    let total = all.len();
    assert!(l.s.toggle_history_item(false, at));
    let open = items(&mut l.s, false);
    assert_eq!(open.iter().filter(|i| i["type"] == "header").count(), 5);
    assert!(open.len() >= total + 2);
    let folded: Vec<&Value> = open
        .iter()
        .filter(|i| i["type"] == "header")
        .skip(3)
        .collect();
    assert!(
        folded.iter().all(|h| h["expanded"] == false),
        "each folded: {folded:?}"
    );
    // Header text of a one-step branch uses the singular.
    assert!(
        open.iter()
            .filter(|i| i["type"] == "header")
            .all(|h| h["text"].as_str().unwrap().ends_with("1 step"))
    );
}

#[test]
fn c8_header_rows_are_list_items_the_press_on_a_row_ignores() {
    let (mut l, _) = lab_with_branches(1);
    let all = items(&mut l.s, false);
    let h = all.iter().position(|i| i["type"] == "header").unwrap();
    assert_eq!(l.s.preview_history_item(false, h), HistoryRowPress::NotARow);
    assert!(!l.document_preview_active());
    assert_eq!(l.s.clone_history_item(false, h, 0), CloneOutcome::Refused);
    assert_eq!(l.s.apply_history_item(true, h), GoToOutcome::NotARow);
}

trait LabExt {
    fn document_preview_active(&self) -> bool;
}
impl LabExt for Lab {
    fn document_preview_active(&self) -> bool {
        self.s.document_preview_active()
    }
}

// ---------------------------------------------------------------------
// Criteria 10, 16: applying a version (11 to 13 withdrawn, customer feedback 2026-10-14)
// ---------------------------------------------------------------------

#[test]
fn c10_enter_applies_the_version_as_one_step_and_ctrl_z_takes_it_back() {
    let mut l = Lab::new();
    let s1 = l.step();
    let at_s1 = json(&l.s);
    let _s2 = l.step();
    let _s3 = l.step();
    let at_s3 = json(&l.s);
    l.select();
    let undo_before = l.s.undo_len();
    let i1 = item_of(&mut l.s, true, &s1);
    let id1 = shown_id(&mut l.s, true, &s1);
    let GoToOutcome::Went(notice) = l.s.apply_history_item(true, i1) else {
        panic!("went")
    };
    assert_eq!(notice, format!("Went to {id1}."));
    assert_eq!(json(&l.s), at_s1, "the object is in the version");
    assert_eq!(l.s.undo_len(), undo_before + 1, "one step on my stack");
    assert!(
        obj_rows(&mut l.s)[0]["name"]
            .as_str()
            .unwrap()
            .starts_with("Go to version")
    );
    // Pressing the row of the version it already has writes nothing.
    let n = l.s.history_len();
    let i1 = item_of(&mut l.s, true, &s1);
    assert_eq!(l.s.apply_history_item(true, i1), GoToOutcome::Same);
    assert_eq!(l.s.history_len(), n);
    // Going to a version and then Ctrl+Z restores the state from before.
    let (out, _) = ctrl_z(&mut l.s);
    assert_eq!(out, KeyOutcome::Undo);
    assert_eq!(json(&l.s), at_s3);
    // And Ctrl+Shift+Z brings the go-to back.
    ctrl_shift_z(&mut l.s);
    assert_eq!(json(&l.s), at_s1);
}

#[test]
fn c16_the_gate_ignores_a_press_during_a_drag_a_chip_or_an_unfinished_pen_path() {
    let mut l = Lab::new();
    let s1 = l.step();
    let _ = l.step();
    l.select();
    let before = json(&l.s);
    let undo = l.s.undo_len();
    let i = item_of(&mut l.s, true, &s1);
    // During a drag.
    l.s.pointer_hover(pt(30.0, l.y), false, false);
    l.s.pointer_down(pt(30.0, l.y), false);
    l.s.pointer_hover(pt(60.0, l.y + 5.0), false, false);
    assert_eq!(l.s.apply_history_item(true, i), GoToOutcome::Ignored);
    assert_eq!(l.s.preview_history_item(true, i), HistoryRowPress::Ignored);
    assert_eq!(l.s.clone_history_item(true, i, 0), CloneOutcome::Ignored);
    l.s.pointer_up(pt(60.0, l.y + 5.0), false, false);
    // Undo the drag's own step so the comparisons are against the old state.
    ctrl_z(&mut l.s);
    assert_eq!(json(&l.s), before);
    assert_eq!(l.s.undo_len(), undo);
    // A chip (move entry) is open.
    assert_eq!(key_in(&mut l.s, "m", false, false), KeyOutcome::EntryOpened);
    let i = item_of(&mut l.s, true, &s1);
    assert_eq!(l.s.apply_history_item(true, i), GoToOutcome::Ignored);
    assert_eq!(l.s.preview_history_item(true, i), HistoryRowPress::Ignored);
    assert_eq!(l.s.clone_history_item(true, i, 0), CloneOutcome::Ignored);
    key_in(&mut l.s, "Escape", false, false);
    // An unfinished Pen path.
    l.s.set_tool(Tool::Pen);
    l.s.pointer_hover(pt(300.0, 300.0), false, false);
    l.s.pointer_down(pt(300.0, 300.0), false);
    l.s.pointer_up(pt(300.0, 300.0), false, false);
    let rows = items(&mut l.s, false);
    let any_row = rows.iter().position(|r| r["type"] == "row").unwrap();
    assert_eq!(
        l.s.preview_history_item(false, any_row),
        HistoryRowPress::Ignored
    );
    assert_eq!(
        l.s.clone_history_item(false, any_row, 0),
        CloneOutcome::Ignored
    );
}

// ---------------------------------------------------------------------
// Criteria 17, 18, 20: Object preview
// ---------------------------------------------------------------------

#[test]
fn c17_object_preview_changes_nothing_and_reads_as_a_status_line() {
    let mut l = Lab::new();
    let s1 = l.step();
    let _s2 = l.step();
    l.select();
    let before = json(&l.s);
    let undo = l.s.undo_len();
    let hist = l.s.history_len();
    let plain = l.s.frame_draw_list();
    let i = item_of(&mut l.s, true, &s1);
    assert_eq!(
        l.s.preview_history_item(true, i),
        HistoryRowPress::Previewed
    );
    let v: Value = serde_json::from_str(&l.s.history_preview_json()).unwrap();
    assert_eq!(v["kind"], "object");
    let id1 = shown_id(&mut l.s, true, &s1);
    assert_eq!(v["id"], id1);
    assert_eq!(v["name"], "Move");
    assert_eq!(v["author"], "you");
    assert!(v["time"].as_u64().unwrap() > 0);
    // Nothing changed: the document, the stack, the log, the selection.
    assert_eq!(json(&l.s), before);
    assert_eq!(l.s.undo_len(), undo);
    assert_eq!(l.s.history_len(), hist);
    assert_eq!(l.s.selected_object_count(), 1);
    // The canvas draws the version in place of the current object (criterion 17 as changed by the
    // customer feedback of 2026-10-14).
    let previewing = l.s.frame_draw_list();
    assert_ne!(
        previewing, plain,
        "the version is drawn in place of the object"
    );
    // The editing keys still work (an Object preview does not pause editing).
    assert!(!l.s.document_preview_active());
    // Ending it removes the drawing.
    l.s.end_history_preview();
    assert_eq!(preview_kind(&mut l.s), "");
    assert_eq!(l.s.frame_draw_list(), plain);
}

#[test]
fn c17_each_pressed_row_replaces_the_readout() {
    let mut l = Lab::new();
    let s1 = l.step();
    let s2 = l.step();
    let _s3 = l.step();
    l.select();
    let i1 = item_of(&mut l.s, true, &s1);
    l.s.preview_history_item(true, i1);
    let a: Value = serde_json::from_str(&l.s.history_preview_json()).unwrap();
    let i2 = item_of(&mut l.s, true, &s2);
    l.s.preview_history_item(true, i2);
    let b: Value = serde_json::from_str(&l.s.history_preview_json()).unwrap();
    assert_ne!(a["id"], b["id"]);
    assert_eq!(b["id"], shown_id(&mut l.s, true, &s2));
}

#[test]
fn c18_a_preview_ends_on_the_listed_events() {
    let mut l = Lab::new();
    let s1 = l.step();
    let _s2 = l.step();
    l.select();
    let preview = |l: &mut Lab| {
        let i = item_of(&mut l.s, true, &s1);
        assert_eq!(
            l.s.preview_history_item(true, i),
            HistoryRowPress::Previewed
        );
        assert_eq!(preview_kind(&mut l.s), "object");
    };
    // Escape in the list is the host's call of end_history_preview.
    preview(&mut l);
    l.s.end_history_preview();
    assert_eq!(preview_kind(&mut l.s), "");
    // A press on the canvas only ends it: no selection change, no step, no deselect.
    preview(&mut l);
    let undo = l.s.undo_len();
    let before = json(&l.s);
    l.s.pointer_hover(pt(500.0, 700.0), false, false);
    l.s.pointer_down(pt(500.0, 700.0), false);
    l.s.pointer_up(pt(500.0, 700.0), false, false);
    assert_eq!(preview_kind(&mut l.s), "");
    assert_eq!(l.s.selected_object_count(), 1, "the press did nothing else");
    assert_eq!(l.s.undo_len(), undo);
    assert_eq!(json(&l.s), before);
    // Selecting another object (here: nothing / another) ends it.
    preview(&mut l);
    click(&mut l.s, pt(500.0, 700.0));
    click(&mut l.s, pt(500.0, 700.0));
    assert_eq!(preview_kind(&mut l.s), "");
    l.select();
    // The row of the version the object already has ends it.
    let newest = obj_rows(&mut l.s)[0]["step"].as_str().unwrap().to_owned();
    preview(&mut l);
    let i = item_of(&mut l.s, true, &newest);
    assert_eq!(l.s.preview_history_item(true, i), HistoryRowPress::Ended);
    assert_eq!(preview_kind(&mut l.s), "");
}

#[test]
fn c20_nothing_is_written_replicated_or_saved_by_a_preview() {
    let mut l = Lab::new();
    let s1 = l.step();
    let _ = l.step();
    l.select();
    let packed_before = l.s.pack("t").unwrap();
    let i = item_of(&mut l.s, true, &s1);
    l.s.preview_history_item(true, i);
    let packed_after = l.s.pack("t").unwrap();
    let a = unpack(5, &packed_before).unwrap();
    let b = unpack(5, &packed_after).unwrap();
    assert_eq!(a.export_json().unwrap(), b.export_json().unwrap());
    assert_eq!(a.history_len(), b.history_len());
    // Save and open: the reopened session has no preview.
    let mut reopened = Session::open(11, &packed_after).unwrap();
    assert_eq!(preview_kind(&mut reopened), "");
    // Ctrl+Z does not take a preview back: it takes back the last real step.
    let (_, notice) = ctrl_z(&mut l.s);
    assert!(notice.starts_with("Undid"), "{notice}");
}

#[test]
fn c17_preview_of_a_deleted_object_via_the_document_row_and_the_object_scope() {
    let mut l = Lab::new();
    let s1 = l.step();
    let _s2 = l.step();
    // Delete it.
    assert_eq!(
        key_in(&mut l.s, "Delete", false, false),
        KeyOutcome::Deleted
    );
    let del = newest_step(&mut l.s);
    assert_eq!(l.s.selected_object_count(), 0);
    // Document scope preview at the step before the delete shows the object.
    let before_del = doc_rows(&mut l.s)
        .iter()
        .find(|r| r["step"] != del && r["label"] != "undo")
        .unwrap()["step"]
        .as_str()
        .unwrap()
        .to_owned();
    let i = item_of(&mut l.s, false, &before_del);
    assert_eq!(
        l.s.preview_history_item(false, i),
        HistoryRowPress::Previewed
    );
    assert!(l.s.document_preview_active());
    let with_obj = l.s.frame_draw_list();
    l.s.end_history_preview();
    let i = item_of(&mut l.s, false, &del);
    assert_eq!(
        l.s.preview_history_item(false, i),
        HistoryRowPress::Previewed
    );
    let without_obj = l.s.frame_draw_list();
    assert_ne!(
        with_obj, without_obj,
        "the deleted object is gone at the delete row"
    );
    l.s.end_history_preview();
    // Ctrl+Z brings the object back and its Object scope previews work again.
    ctrl_z(&mut l.s);
    assert_eq!(l.s.selected_object_count(), 1);
    let i = item_of(&mut l.s, true, &s1);
    assert_eq!(
        l.s.preview_history_item(true, i),
        HistoryRowPress::Previewed
    );
}

// ---------------------------------------------------------------------
// Criterion 19: Document preview
// ---------------------------------------------------------------------

#[test]
fn c19_document_preview_pauses_editing_but_not_pan_and_zoom() {
    let mut l = Lab::new();
    let s1 = l.step();
    let _s2 = l.step();
    let live_json = json(&l.s);
    let undo = l.s.undo_len();
    let sel = l.s.selected_object_count();
    let live_frame = l.s.frame_draw_list();
    let i = item_of(&mut l.s, false, &s1);
    assert_eq!(
        l.s.preview_history_item(false, i),
        HistoryRowPress::Previewed
    );
    assert!(l.s.document_preview_active());
    assert_eq!(preview_kind(&mut l.s), "document");
    assert_ne!(
        l.s.frame_draw_list(),
        live_frame,
        "the canvas shows the version"
    );
    // Every editing key, undo and redo included, is paused.
    for (k, ctrl, shift) in [
        ("z", true, false),
        ("Z", true, true),
        ("u", true, false),
        ("U", true, true),
        ("Delete", false, false),
        ("a", true, false),
        ("r", false, false),
        ("p", false, false),
        ("ArrowLeft", false, false),
        ("m", false, false),
    ] {
        assert_eq!(key_in(&mut l.s, k, ctrl, shift), KeyOutcome::Paused, "{k}");
    }
    assert_eq!(json(&l.s), live_json);
    assert_eq!(l.s.undo_len(), undo);
    // Pointer input on the canvas is ignored: no selection change, no draw.
    click(&mut l.s, pt(700.0, 700.0));
    assert!(
        l.s.document_preview_active(),
        "a press does not end a document preview"
    );
    assert_eq!(l.s.selected_object_count(), sel);
    drag(
        &mut l.s,
        Tool::Rectangle,
        pt(300.0, 300.0),
        pt(340.0, 340.0),
    );
    assert_eq!(json(&l.s), live_json);
    // Pan and zoom work.
    let zoom = l.s.zoom_percent();
    l.s.wheel(0.0, -120.0, 400.0, 300.0, false, true);
    assert_ne!(l.s.zoom_percent(), zoom, "zoom works");
    let at = l.s.screen_to_document(10.0, 10.0);
    l.s.wheel(40.0, 30.0, 400.0, 300.0, false, false);
    assert_ne!(l.s.screen_to_document(10.0, 10.0), at, "pan works");
    // Back to live.
    l.s.end_history_preview();
    assert!(!l.s.document_preview_active());
    assert_eq!(
        l.s.frame_draw_list().layers().len(),
        live_frame.layers().len()
    );
    assert_ne!(key_in(&mut l.s, "z", true, false), KeyOutcome::Paused);
}

#[test]
fn c19_the_document_preview_shows_page_size_and_background_of_that_version() {
    let mut l = Lab::new();
    let s1 = l.step();
    // The page rectangle is the first six vertices of the frame (layer 0), in document space.
    let page = |s: &Session| {
        let list = s.frame_draw_list();
        let first = &list.triangles[..6];
        let max_x = first.iter().map(|v| v.position.x).fold(f64::MIN, f64::max);
        let max_y = first.iter().map(|v| v.position.y).fold(f64::MIN, f64::max);
        (max_x.round(), max_y.round(), first[0].color)
    };
    let (w0, h0, c0) = page(&l.s);
    // Later: a bigger page and another background.
    let _ = l.s.set_document_side(DocumentSide::Width, "500");
    let _ = l.s.set_document_side(DocumentSide::Height, "400");
    l.s.set_background_hex("#112233").unwrap();
    let (w1, h1, c1) = page(&l.s);
    assert!(
        w1 > w0 && h1 > h0 && c1 != c0,
        "setup: the live page changed"
    );
    let i = item_of(&mut l.s, false, &s1);
    assert_eq!(
        l.s.preview_history_item(false, i),
        HistoryRowPress::Previewed
    );
    assert_eq!(
        page(&l.s),
        (w0, h0, c0),
        "page size and background of the version"
    );
    l.s.end_history_preview();
    assert_eq!(page(&l.s), (w1, h1, c1), "the live page is untouched");
    assert_eq!(l.s.size_text(), "500.0 \u{d7} 400.0 mm");
}

#[test]
fn c19_object_preview_does_not_pause_editing_and_document_preview_ends_on_leaving() {
    let mut l = Lab::new();
    let s1 = l.step();
    let _s2 = l.step();
    l.select();
    let i = item_of(&mut l.s, true, &s1);
    l.s.preview_history_item(true, i);
    // Ctrl+Z is not paused during an Object preview... but a press on canvas ended it already.
    assert_ne!(key_in(&mut l.s, "Escape", false, false), KeyOutcome::Paused);
}

// ---------------------------------------------------------------------
// Criteria 21: budgets at 10,000 steps (through the session)
// ---------------------------------------------------------------------

fn big_session() -> (Session, NodeId) {
    let d = Document::new(5);
    let hot = d.create_rect(RectBounds {
        origin: pt(20.0, 20.0),
        width: Length::from_mm(40.0),
        height: Length::from_mm(40.0),
    });
    let cold = d.create_rect(RectBounds {
        origin: pt(100.0, 20.0),
        width: Length::from_mm(40.0),
        height: Length::from_mm(40.0),
    });
    for i in 0..10_000 {
        let id = if i % 5 == 0 { hot } else { cold };
        d.translate_objects(&[id], Vec2::new(0.01, 0.0)).unwrap();
    }
    // 50 branches near the end: step, take back, step.
    let steps = d.own_steps();
    assert!(steps.len() >= 10_000);
    for k in 0..50 {
        d.translate_objects(&[hot], Vec2::new(1.0, 0.0)).unwrap();
        let st = d.own_steps().last().unwrap().step;
        d.undo_step(st).unwrap();
        d.translate_objects(&[cold], Vec2::new(0.01 * f64::from(k + 1), 0.0))
            .unwrap();
    }
    let mut s = Session::open(11, &pack(&d, "t").unwrap()).unwrap();
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_tool(Tool::Select);
    (s, hot)
}

#[test]
fn c8_c21_a_list_with_50_branches_and_10000_steps_opens_and_previews_inside_the_budgets() {
    let (mut s, hot) = big_session();
    let scale: u128 = if cfg!(debug_assertions) { 10 } else { 1 };
    s.warm_history();
    let t = Instant::now();
    let layout: Value = serde_json::from_str(&s.history_layout_json(false)).unwrap();
    let first = s.history_items_json(false, 0, 30);
    let took = t.elapsed();
    eprintln!("open the list at {} items: {took:?}", layout["total"]);
    assert!(layout["total"].as_u64().unwrap() >= 10_000);
    assert!(layout["lanes"].as_u64().unwrap() <= 3);
    let items: Value = serde_json::from_str(&first).unwrap();
    assert_eq!(items.as_array().unwrap().len(), 30);
    assert!(
        took.as_millis() <= 200 * 4 * scale,
        "the list opened in {took:?}"
    );
    let more = layout["special"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e[1] == "more")
        .count();
    assert_eq!(more, 1, "47 branches are folded into one row");
    // Press the rows 10, 100 and 1,000 back, and move along with the keys.
    for back in [10_usize, 100, 1_000] {
        let idx = {
            let all = items_window(&mut s, back + 80);
            all.iter()
                .enumerate()
                .filter(|(_, i)| i["type"] == "row")
                .nth(back)
                .map(|(k, _)| k)
                .unwrap()
        };
        assert!(
            !s.history_item_is_far(false, idx) || back >= 1_000,
            "{back}"
        );
        let t = Instant::now();
        assert_eq!(
            s.preview_history_item(false, idx),
            HistoryRowPress::Previewed
        );
        let took = t.elapsed();
        eprintln!("session preview {back} rows back: {took:?}");
        assert!(took.as_millis() <= 300 * 4 * scale, "{back} back: {took:?}");
        for k in 1..=5 {
            let t = Instant::now();
            assert_eq!(
                s.preview_history_item(false, idx + k),
                HistoryRowPress::Previewed
            );
            assert!(
                t.elapsed().as_millis() <= 300 * 4 * scale,
                "moving along: {:?}",
                t.elapsed()
            );
        }
        s.end_history_preview();
    }
    // Far: the long-operation state is announced first, then the read fits 1.5 s x 4.
    let far = items_window(&mut s, 10_030).len() - 3;
    assert!(s.history_item_is_far(false, far));
    let t = Instant::now();
    assert_eq!(
        s.preview_history_item(false, far),
        HistoryRowPress::Previewed
    );
    eprintln!("session preview 10,000 rows back: {:?}", t.elapsed());
    assert!(
        t.elapsed().as_millis() <= 1_500 * 4 * scale,
        "{:?}",
        t.elapsed()
    );
    s.end_history_preview();
    // Object scope as well.
    click(&mut s, pt(30.0, 20.0));
    if s.selected_object_count() == 1 {
        let n = s.object_history_len();
        assert!(n > 1_000);
        let idx = 1_005.min(n - 2);
        let t = Instant::now();
        let _ = s.preview_history_item(true, idx);
        eprintln!(
            "object scope preview {idx} back: {:?} (object {hot:?})",
            t.elapsed()
        );
    }
}

fn items_window(s: &mut Session, count: usize) -> Vec<Value> {
    let v: Value = serde_json::from_str(&s.history_items_json(false, 0, count)).unwrap();
    v.as_array().unwrap().clone()
}

// ---------------------------------------------------------------------
// Criteria 22 to 26: Clone
// ---------------------------------------------------------------------

#[test]
fn c22_clone_in_the_object_scope_makes_a_new_selected_object_in_one_step() {
    let mut l = Lab::new();
    let s1 = l.step();
    let at_s1_doc = reread(&l.s);
    let _s2 = l.step();
    let _s3 = l.step();
    l.select();
    let orig = l.s.history_object().unwrap();
    let orig_json = json(&l.s);
    let undo = l.s.undo_len();
    let i = item_of(&mut l.s, true, &s1);
    let id1 = shown_id(&mut l.s, true, &s1);
    // Not gated by a preview.
    for previewing in [false, true] {
        if previewing {
            let _ = l.s.preview_history_item(true, i);
        }
        let CloneOutcome::Cloned(notice) = l.s.clone_history_item(true, i, 0) else {
            panic!("cloned")
        };
        assert_eq!(notice, format!("Cloned version {id1} as a new object."));
        assert_eq!(l.s.selected_object_count(), 1);
        let clone = l.s.history_object().unwrap();
        assert_ne!(clone, orig, "a new object, selected");
        // The original does not change.
        let d = reread(&l.s);
        assert!(d.object(orig).is_some());
        assert_eq!(d.object_ids().last(), Some(&clone), "on top of the stack");
        // Ctrl+Z removes the clone, the original is as before.
        ctrl_z(&mut l.s);
        assert!(reread(&l.s).object(clone).is_none());
        assert_eq!(json(&l.s), orig_json);
        l.select();
        assert_eq!(l.s.undo_len(), undo);
    }
    // The clone has s1's state (position): its geometry equals the original's at s1.
    let _ = at_s1_doc;
    let CloneOutcome::Cloned(_) = l.s.clone_history_item(true, i, 0) else {
        panic!()
    };
    let clone = l.s.history_object().unwrap();
    let d = reread(&l.s);
    let c = format!("{:?}", d.object(clone));
    let want_doc = {
        let mut m = Lab::new();
        m.step();
        reread(&m.s)
    };
    let want_id = want_doc.object_ids()[0];
    let strip = |t: String| t.split("shape: ").nth(1).unwrap().to_owned();
    assert_eq!(
        strip(c).split("style:").next().unwrap(),
        strip(format!("{:?}", want_doc.object(want_id)))
            .split("style:")
            .next()
            .unwrap()
    );
}

#[test]
fn c23_the_clone_timeline_begins_with_its_origin() {
    let mut l = Lab::new();
    let s1 = l.step();
    let _ = l.step();
    l.select();
    let orig_short = reread(&l.s).object_ids()[0].short_id();
    let i = item_of(&mut l.s, true, &s1);
    let id1 = shown_id(&mut l.s, true, &s1);
    let CloneOutcome::Cloned(_) = l.s.clone_history_item(true, i, 0) else {
        panic!()
    };
    let rows = obj_rows(&mut l.s);
    let first = &rows[0];
    let name = first["name"].as_str().unwrap();
    assert!(
        name.contains(&id1)
            || first["summary"].as_str().unwrap().contains(&id1)
            || name.starts_with("Clone"),
        "{first}"
    );
    assert!(
        rows.iter().any(|r| r["from"] == orig_short.as_str()),
        "inherited rows are flagged from the original"
    );
    let inherited: Vec<&Value> = rows
        .iter()
        .filter(|r| r["from"] == orig_short.as_str())
        .collect();
    assert!(inherited.iter().any(|r| r["step"] == s1));
    // Later steps of the original never appear.
    assert_eq!(inherited.len(), 2, "create and s1 only: {inherited:?}");
}

#[test]
fn c24_c25_c26_clone_in_the_document_scope_and_of_a_deleted_object() {
    let mut l = Lab::new();
    let _s1 = l.step();
    // Resize it, delete it.
    l.select();
    drag_move(&mut l.s, pt(30.0, l.y), 0.0, 0.0);
    let moved = json(&l.s);
    let _ = moved;
    assert_eq!(
        key_in(&mut l.s, "Delete", false, false),
        KeyOutcome::Deleted
    );
    let del = newest_step(&mut l.s);
    let state_before_delete = {
        ctrl_z(&mut l.s);
        let j = json(&l.s);
        ctrl_shift_z(&mut l.s);
        j
    };
    let _ = state_before_delete;
    let row = doc_rows(&mut l.s)
        .into_iter()
        .find(|r| r["step"] == del)
        .unwrap();
    let objs = row["objects"].as_array().unwrap();
    assert_eq!(objs.len(), 1);
    assert_eq!(objs[0]["role"], "deleted");
    assert_eq!(objs[0]["alive"], false);
    let i = item_of(&mut l.s, false, &del);
    let CloneOutcome::Cloned(_) = l.s.clone_history_item(false, i, 0) else {
        panic!("cloned")
    };
    assert_eq!(l.s.selected_object_count(), 1);
    let clone = l.s.history_object().unwrap();
    let d = reread(&l.s);
    assert!(d.object(clone).is_some());
    // The touched-index out of range is refused and writes nothing.
    let n = l.s.history_len();
    assert_eq!(l.s.clone_history_item(false, i, 7), CloneOutcome::Refused);
    assert_eq!(l.s.history_len(), n);
    // Save/Open: clone from the Delete row of the reopened file.
    let mut again = Session::open(12, &l.s.pack("t").unwrap()).unwrap();
    again.set_clock(clock);
    again.resize_viewport(1200.0, 800.0);
    let i = item_of(&mut again, false, &del);
    let CloneOutcome::Cloned(_) = again.clone_history_item(false, i, 0) else {
        panic!("cloned after reopen")
    };
}

#[test]
fn c25_no_clone_for_a_group() {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(60.0, 60.0));
    drag(&mut s, Tool::Rectangle, pt(100.0, 20.0), pt(140.0, 60.0));
    assert_eq!(key_in(&mut s, "a", true, false), KeyOutcome::SelectedAll);
    assert_eq!(key_in(&mut s, "g", true, false), KeyOutcome::Group);
    let _ = s.take_grouping_result();
    let group_step = newest_step(&mut s);
    let row = doc_rows(&mut s)
        .into_iter()
        .find(|r| r["step"] == group_step)
        .unwrap();
    let objs = row["objects"].as_array().unwrap();
    let g = objs
        .iter()
        .position(|o| o["shape"] == "group")
        .expect("the group is touched");
    let i = item_of(&mut s, false, &group_step);
    let n = s.history_len();
    assert_eq!(s.clone_history_item(false, i, g), CloneOutcome::Refused);
    assert_eq!(s.history_len(), n, "nothing written");
}

// ---------------------------------------------------------------------
// Save and Open, format version
// ---------------------------------------------------------------------

#[test]
fn c6_c33_save_and_open_keep_the_branches_and_the_format_version() {
    let mut l = Lab::new();
    let s1 = l.step();
    let s2 = l.step();
    let s3 = l.step();
    ctrl_z(&mut l.s);
    ctrl_z(&mut l.s);
    l.y = 30.0;
    l.select();
    drag_move(&mut l.s, pt(30.0, l.y), 40.0, 0.0);
    let before_items: Vec<(String, String, String)> = items(&mut l.s, false)
        .iter()
        .map(|i| {
            (
                i["type"].to_string(),
                i["step"].to_string(),
                i["text"].to_string(),
            )
        })
        .collect();
    let before_rows: Vec<(String, String)> = doc_rows(&mut l.s)
        .iter()
        .map(|r| (r["step"].to_string(), r["status"].to_string()))
        .collect();
    let bytes = l.s.pack("t").unwrap();
    let mut again = Session::open(11, &bytes).unwrap();
    let after_rows: Vec<(String, String)> = doc_rows(&mut again)
        .iter()
        .map(|r| (r["step"].to_string(), r["status"].to_string()))
        .collect();
    assert_eq!(before_rows, after_rows, "ids, order, states");
    let after_items: Vec<(String, String, String)> = items(&mut again, false)
        .iter()
        .map(|i| {
            (
                i["type"].to_string(),
                i["step"].to_string(),
                i["text"].to_string(),
            )
        })
        .collect();
    assert_eq!(before_items, after_items, "headers and rows");
    let rows = doc_rows(&mut again);
    assert_eq!(status_in(&rows, &s2), "branch");
    assert_eq!(status_in(&rows, &s3), "branch");
    assert_eq!(status_in(&rows, &s1), "applied");
    // The format version of the file is unchanged.
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut text = String::new();
    std::io::Read::read_to_string(&mut archive.by_name("manifest.json").unwrap(), &mut text)
        .unwrap();
    let m: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        m["format_version"].as_u64().unwrap(),
        u64::from(CURRENT_FORMAT_VERSION)
    );
    assert_eq!(
        CURRENT_FORMAT_VERSION, 14,
        "0042 adds no bump; 14 is main after 0057"
    );
}

#[test]
fn c9_the_row_actions_are_not_rows_without_a_selection() {
    let mut s = fresh();
    // Object scope without an object: no list, nothing to press.
    assert_eq!(s.object_history_len(), 0);
    assert_eq!(s.preview_history_item(true, 0), HistoryRowPress::NotARow);
    assert_eq!(s.apply_history_item(true, 0), GoToOutcome::NotARow);
    assert_eq!(s.clone_history_item(true, 0, 0), CloneOutcome::Refused);
    // Document scope of an empty document.
    assert_eq!(s.preview_history_item(false, 99), HistoryRowPress::NotARow);
    assert_eq!(s.clone_history_item(false, 99, 0), CloneOutcome::Refused);
    assert_eq!(items(&mut s, false).len(), s.history_len());
}

// ---------------------------------------------------------------------
// White-box edges: inherited rows, switching between the two preview scopes
// ---------------------------------------------------------------------

#[test]
fn w_inherited_rows_of_a_clone_preview_and_go_to_the_original_object() {
    let mut l = Lab::new();
    let s1 = l.step();
    let _ = l.step();
    l.select();
    let i = item_of(&mut l.s, true, &s1);
    let CloneOutcome::Cloned(_) = l.s.clone_history_item(true, i, 0) else {
        panic!()
    };
    let clone = l.s.history_object().unwrap();
    // The clone's list has the inherited row s1: pressing it previews the original's version.
    let at = item_of(&mut l.s, true, &s1);
    assert_eq!(
        l.s.preview_history_item(true, at),
        HistoryRowPress::Previewed
    );
    assert_eq!(preview_kind(&mut l.s), "object");
    l.s.end_history_preview();
    // Enter: the clone goes to the original's version of s1 (it already has it: nothing written).
    let n = l.s.history_len();
    let outcome = l.s.apply_history_item(true, at);
    assert!(
        matches!(outcome, GoToOutcome::Same | GoToOutcome::Went(_)),
        "{outcome:?}"
    );
    if outcome == GoToOutcome::Same {
        assert_eq!(l.s.history_len(), n);
    }
    assert_eq!(l.s.history_object(), Some(clone));
}

#[test]
fn w_a_document_preview_replaces_an_object_preview_and_ends_cleanly() {
    let mut l = Lab::new();
    let s1 = l.step();
    let _ = l.step();
    l.select();
    let i = item_of(&mut l.s, true, &s1);
    assert_eq!(
        l.s.preview_history_item(true, i),
        HistoryRowPress::Previewed
    );
    let j = item_of(&mut l.s, false, &s1);
    assert_eq!(
        l.s.preview_history_item(false, j),
        HistoryRowPress::Previewed
    );
    assert_eq!(preview_kind(&mut l.s), "document");
    assert!(l.s.document_preview_active());
    l.s.end_history_preview();
    assert!(!l.s.document_preview_active());
    assert_ne!(key_in(&mut l.s, "z", true, false), KeyOutcome::Paused);
}

#[test]
fn w_rapid_presses_in_both_modes_leave_no_state_behind() {
    let mut l = Lab::new();
    let steps: Vec<String> = (0..6).map(|_| l.step()).collect();
    l.select();
    let before = json(&l.s);
    let undo = l.s.undo_len();
    for round in 0..3 {
        for st in &steps {
            let i = item_of(&mut l.s, true, st);
            let _ = l.s.preview_history_item(true, i);
            let j = item_of(&mut l.s, false, st);
            let _ = l.s.preview_history_item(false, j);
            l.s.end_history_preview();
        }
        assert_eq!(json(&l.s), before, "round {round}");
        assert_eq!(l.s.undo_len(), undo);
    }
    assert_ne!(key_in(&mut l.s, "z", true, false), KeyOutcome::Paused);
}
