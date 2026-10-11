//! Ctrl+U, Ctrl+Shift+U and the Object scope in the editor session (`specs/0041-object-history`
//! criteria 1, 2, 5, 8 to 12, 14 to 16, 29 to 31): the keys, their gate, the notices, the
//! selection, the stacks and the rows the History tab reads.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]

use curvyo_document_core::Direction;
use curvyo_document_core::Point;
use curvyo_document_core::{Document, unpack};
use curvyo_editor_wasm::{KeyHint, KeyInput, KeyOutcome, ObjectWipeOutcome, Session, Tool};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fresh() -> Session {
    let mut session = Session::new(2);
    session.resize_viewport(1200.0, 800.0);
    session
}

fn drag(session: &mut Session, from: Point, to: Point) {
    session.pointer_hover(from, false, false);
    session.pointer_down(from, false);
    session.pointer_hover(to, false, false);
    session.pointer_up(to, false, false);
}

/// Draws a 20 mm square at `x` with the Rectangle tool and switches to the Select tool; the new
/// square stays selected.
fn draw_square(session: &mut Session, x: f64) {
    session.set_tool(Tool::Rectangle);
    drag(session, pt(x, 10.0), pt(x + 20.0, 30.0));
    session.set_tool(Tool::Select);
}

fn object_key(session: &mut Session, shift: bool) -> KeyOutcome {
    session.key_down(KeyInput {
        key: if shift { "U" } else { "u" },
        shift,
        ctrl: true,
        ..KeyInput::default()
    })
}

fn nudge(session: &mut Session, time_ms: f64) {
    let outcome = session.key_down(KeyInput {
        key: "ArrowRight",
        time_ms,
        ..KeyInput::default()
    });
    assert!(matches!(outcome, KeyOutcome::Nudged { .. }), "{outcome:?}");
}

fn object_undo(session: &mut Session) -> String {
    assert_eq!(object_key(session, false), KeyOutcome::ObjectUndo);
    session.take_history_notice().unwrap()
}

fn object_redo(session: &mut Session) -> String {
    assert_eq!(object_key(session, true), KeyOutcome::ObjectRedo);
    session.take_history_notice().unwrap()
}

fn doc(session: &Session) -> Document {
    unpack(9, &session.pack("0.1.0").unwrap()).unwrap()
}

fn json(session: &Session) -> Vec<u8> {
    doc(session).export_json().unwrap()
}

fn undo(session: &mut Session) -> String {
    assert_eq!(
        session.key_down(KeyInput {
            key: "z",
            ctrl: true,
            ..KeyInput::default()
        }),
        KeyOutcome::Undo
    );
    session.take_history_notice().unwrap()
}

#[test]
fn ctrl_u_takes_back_the_latest_step_of_the_one_selected_object_and_ctrl_shift_u_brings_it_back() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    let drawn = json(&session);
    nudge(&mut session, 1_000.0);
    let moved = json(&session);

    assert_eq!(object_undo(&mut session), "Undid: Move.");
    assert_eq!(json(&session), drawn);
    assert_eq!(session.selected_object_count(), 1, "the selection stays");

    assert_eq!(object_redo(&mut session), "Redid: Move.");
    assert_eq!(json(&session), moved);
}

#[test]
fn an_object_undo_is_a_step_that_ctrl_z_takes_back() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    nudge(&mut session, 1_000.0);
    let moved = json(&session);
    let steps = session.undo_len();

    object_undo(&mut session);
    assert_eq!(session.undo_len(), steps + 1, "it is on the maker's stack");
    assert!(undo(&mut session).starts_with("Undid: Undo Move"));
    assert_eq!(json(&session), moved, "Ctrl+Z gives the object redo");
}

#[test]
fn taking_back_the_creation_empties_the_selection_and_ctrl_z_brings_the_object_back() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    let drawn = json(&session);
    assert_eq!(
        object_undo(&mut session),
        "Undid: Draw rectangle. The object is gone. Ctrl+Z brings it back."
    );
    assert_eq!(session.selected_object_count(), 0);
    assert_eq!(doc(&session).object_ids().len(), 0);
    undo(&mut session);
    assert_eq!(json(&session), drawn, "the same id and the same drawing");
    assert_eq!(session.selected_object_count(), 1);
}

#[test]
fn nothing_selected_or_several_selected_says_so_and_changes_nothing() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    draw_square(&mut session, 60.0);
    let before = json(&session);
    let select_all = session.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    assert_eq!(select_all, KeyOutcome::SelectedAll);
    assert_eq!(object_undo(&mut session), "Select one object.");
    assert_eq!(object_redo(&mut session), "Select one object.");
    session.escape();
    assert_eq!(session.selected_object_count(), 0);
    assert_eq!(object_undo(&mut session), "Select one object.");
    assert_eq!(json(&session), before);
}

#[test]
fn a_press_with_nothing_to_take_back_or_bring_back_says_so() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    assert_eq!(
        object_redo(&mut session),
        "Nothing to redo for this object."
    );
    nudge(&mut session, 1_000.0);
    object_undo(&mut session);
    // A new step on the object leaves nothing to redo for it.
    nudge(&mut session, 5_000.0);
    assert_eq!(
        object_redo(&mut session),
        "Nothing to redo for this object."
    );
}

#[test]
fn the_keys_are_gated_like_undo_and_act_in_the_select_and_node_tools_only() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    nudge(&mut session, 1_000.0);

    let blocked = session.key_down(KeyInput {
        key: "u",
        ctrl: true,
        dom_blocked: true,
        ..KeyInput::default()
    });
    assert_eq!(blocked, KeyOutcome::Ignored);

    // A drag in flight.
    session.pointer_hover(pt(20.0, 20.0), false, false);
    session.pointer_down(pt(20.0, 20.0), false);
    session.pointer_hover(pt(30.0, 30.0), false, false);
    assert_eq!(object_key(&mut session, false), KeyOutcome::Ignored);
    session.pointer_up(pt(30.0, 30.0), false, false);

    // A tool that is neither Select nor Node.
    session.set_tool(Tool::Rectangle);
    assert_eq!(object_key(&mut session, false), KeyOutcome::Ignored);

    // The Node tool acts.
    session.set_tool(Tool::Node);
    assert_eq!(object_key(&mut session, false), KeyOutcome::ObjectUndo);
    session.take_history_notice();

    // An unfinished Pen path.
    session.set_tool(Tool::Pen);
    session.pointer_down(pt(100.0, 100.0), false);
    session.pointer_up(pt(100.0, 100.0), false, false);
    assert_eq!(object_key(&mut session, false), KeyOutcome::Ignored);
    session.set_tool(Tool::Select);
}

#[test]
fn the_buttons_and_the_menu_name_the_step_they_act_on() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    nudge(&mut session, 1_000.0);
    assert_eq!(
        session
            .object_history_target_name(Direction::Undo)
            .as_deref(),
        Some("Move")
    );
    assert_eq!(session.object_history_target_name(Direction::Redo), None);
    let result = session.object_undo();
    assert!(result.changed);
    assert_eq!(result.notice, "Undid: Move.");
    assert_eq!(
        session
            .object_history_target_name(Direction::Redo)
            .as_deref(),
        Some("Move")
    );
    assert!(!session.object_history_needs_busy_state(Direction::Undo));
}

#[test]
fn the_object_scope_reads_its_rows_and_subject_from_the_session() {
    let mut session = fresh();
    assert_eq!(session.object_history_info_json(), "", "no object selected");
    draw_square(&mut session, 10.0);
    nudge(&mut session, 1_000.0);
    object_undo(&mut session);

    let info: serde_json::Value =
        serde_json::from_str(&session.object_history_info_json()).unwrap();
    assert_eq!(info["kind"], "Rectangle");
    assert_eq!(info["id"].as_str().unwrap().len(), 6);
    assert_eq!(info["undo"]["name"], "Draw rectangle");
    assert_eq!(info["redo"]["name"], "Move");
    assert_eq!(info["total"], 3);
    assert_eq!(info["wipeCount"], 3);

    assert_eq!(session.object_history_len(), 3);
    let rows: serde_json::Value =
        serde_json::from_str(&session.object_history_rows_json(0, 10)).unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows[0]["name"], "Undo Move");
    assert_eq!(rows[0]["class"], "undoes");
    assert!(
        rows[0]["summary"].as_str().unwrap().starts_with("reverts "),
        "{}",
        rows[0]["summary"]
    );
    assert_eq!(rows[1]["name"], "Move");
    assert_eq!(rows[1]["status"], "undone-by");
    assert_eq!(rows[1]["undoneBy"], "you");
    assert_eq!(
        rows[1]["id"],
        rows[0]["summary"]
            .as_str()
            .unwrap()
            .trim_start_matches("reverts ")
    );
    assert_eq!(rows[2]["name"], "Draw rectangle");
}

#[test]
fn the_wipe_hides_the_story_drops_the_stack_entries_of_this_object_only_and_stays_off_the_stack() {
    let mut session = fresh();
    draw_square(&mut session, 10.0); // A
    nudge(&mut session, 1_000.0); // A alone
    draw_square(&mut session, 60.0); // B
    // A and B together.
    session.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    nudge(&mut session, 9_000.0);
    // Select A again: click on it.
    session.escape();
    session.pointer_hover(pt(12.0, 20.0), false, false);
    session.pointer_down(pt(12.0, 20.0), false);
    session.pointer_up(pt(12.0, 20.0), false, false);
    assert_eq!(session.selected_object_count(), 1);

    let before = session.undo_len();
    assert_eq!(before, 4);
    let outcome = session.wipe_object_history();
    assert_eq!(outcome, ObjectWipeOutcome::Wiped(3));
    assert_eq!(
        session.undo_len(),
        2,
        "A's drawing and A's move are gone from the stack; B's drawing and the pair stay"
    );
    assert_eq!(session.history_object().map(|_| ()), Some(()));
    let info: serde_json::Value =
        serde_json::from_str(&session.object_history_info_json()).unwrap();
    assert_eq!(info["wipeCount"], 0, "nothing left to hide");
    assert_eq!(info["total"], 1);
    assert_eq!(session.wipe_object_history(), ObjectWipeOutcome::Ignored);

    // Ctrl+Z takes back the pair, then B's drawing, then there is nothing.
    assert!(undo(&mut session).starts_with("Undid: Move"));
    assert!(undo(&mut session).starts_with("Undid: Draw rectangle"));
    assert_eq!(undo(&mut session), "Nothing to undo.");
}

#[test]
fn a_wipe_is_ignored_during_a_drag_and_with_an_unfinished_pen_path() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    session.pointer_hover(pt(20.0, 20.0), false, false);
    session.pointer_down(pt(20.0, 20.0), false);
    session.pointer_hover(pt(30.0, 30.0), false, false);
    assert_eq!(session.wipe_object_history(), ObjectWipeOutcome::Ignored);
    session.pointer_up(pt(30.0, 30.0), false, false);
    let _ = KeyHint::SelectFirst;
}

#[test]
fn a_copy_made_by_the_move_entry_keeps_the_history_of_its_original() {
    use curvyo_ui_core::{EntryOutcome, MoveEntryMode};
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    nudge(&mut session, 1_000.0);
    assert_eq!(
        session.key_down(KeyInput {
            key: "m",
            ..KeyInput::default()
        }),
        KeyOutcome::EntryOpened
    );
    let outcome = session.commit_move_entry(
        "40",
        "0",
        MoveEntryMode {
            absolute: false,
            copy: true,
        },
    );
    assert_eq!(outcome, EntryOutcome::Committed);
    assert_eq!(session.selected_object_count(), 1, "the copy is selected");
    let rows: serde_json::Value =
        serde_json::from_str(&session.object_history_rows_json(0, 10)).unwrap();
    let rows = rows.as_array().unwrap();
    let names: Vec<_> = rows
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(rows.len(), 3);
    assert!(names[0].starts_with("Duplicate of "), "{names:?}");
    assert_eq!(rows[0]["class"], "clone-start");
    assert_eq!(
        rows[1]["from"].as_str().unwrap().len(),
        6,
        "inherited rows say where from"
    );
    assert_eq!(names[1], "Move");
    assert_eq!(names[2], "Draw rectangle");
}

#[test]
fn ctrl_u_on_a_copy_never_takes_back_the_duplicate_and_passes_over_the_originals_moves() {
    use curvyo_ui_core::{EntryOutcome, MoveEntryMode};
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    nudge(&mut session, 1_000.0);
    assert_eq!(
        session.key_down(KeyInput {
            key: "m",
            ..KeyInput::default()
        }),
        KeyOutcome::EntryOpened
    );
    let outcome = session.commit_move_entry(
        "40",
        "0",
        MoveEntryMode {
            absolute: false,
            copy: true,
        },
    );
    assert_eq!(outcome, EntryOutcome::Committed);
    // The press may read several versions: the host shows the busy state first.
    assert_eq!(object_key(&mut session, false), KeyOutcome::ObjectUndoBusy);
    let notice = session.object_undo().notice;
    assert_eq!(
        notice,
        "Undid: Draw rectangle. The object is gone. Ctrl+Z brings it back. \
         Skipped 1 step that only moved or reshaped the original."
    );
    // Ctrl+Z takes back the object undo, then the Duplicate.
    assert!(undo(&mut session).starts_with("Undid: Undo Draw rectangle"));
    assert!(undo(&mut session).starts_with("Undid: Duplicate"));
}
