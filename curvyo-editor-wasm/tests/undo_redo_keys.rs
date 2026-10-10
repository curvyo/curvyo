//! Ctrl+Z and Ctrl+Shift+Z in the editor session (`specs/0020-undo-redo` criteria 1 to 7, 12, 22,
//! 23, 25, 26, 28): the keys, their gate, the notices, the selection and the stacks.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]

use curvyo_document_core::{Document, NodeId, Point, unpack};
use curvyo_editor_wasm::{KeyHint, KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::BooleanOp;

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

/// Draws a 20 mm square at `x` with the Rectangle tool; the new square is selected.
fn draw_square(session: &mut Session, x: f64) {
    session.set_tool(Tool::Rectangle);
    drag(session, pt(x, 10.0), pt(x + 20.0, 30.0));
}

fn key(session: &mut Session, key: &str, shift: bool) -> KeyOutcome {
    session.key_down(KeyInput {
        key,
        shift,
        ctrl: true,
        ..KeyInput::default()
    })
}

fn undo(session: &mut Session) -> String {
    assert_eq!(key(session, "z", false), KeyOutcome::Undo);
    session.take_history_notice().unwrap()
}

fn redo(session: &mut Session) -> String {
    assert_eq!(key(session, "z", true), KeyOutcome::Redo);
    session.take_history_notice().unwrap()
}

fn doc(session: &Session) -> Document {
    unpack(9, &session.pack("0.1.0").unwrap()).unwrap()
}

fn json(session: &Session) -> Vec<u8> {
    doc(session).export_json().unwrap()
}

fn ids(session: &Session) -> Vec<NodeId> {
    doc(session).object_ids()
}

#[test]
fn ctrl_z_takes_the_latest_step_back_and_ctrl_shift_z_brings_it_back() {
    let mut session = fresh();
    let empty = json(&session);
    draw_square(&mut session, 10.0);
    let drawn = json(&session);
    assert_eq!(session.undo_len(), 1);

    assert_eq!(undo(&mut session), "Undid: Draw rectangle (1 rectangle).");
    assert_eq!(json(&session), empty);
    assert_eq!(session.selected_object_count(), 0);
    assert_eq!((session.undo_len(), session.redo_len()), (0, 1));

    assert_eq!(redo(&mut session), "Redid: Draw rectangle (1 rectangle).");
    assert_eq!(json(&session), drawn);
    assert_eq!(
        session.selected_object_count(),
        1,
        "the redo selects what the step made"
    );
}

#[test]
fn an_empty_stack_says_so_and_changes_nothing() {
    let mut session = fresh();
    let before = json(&session);
    assert_eq!(undo(&mut session), "Nothing to undo.");
    assert_eq!(redo(&mut session), "Nothing to redo.");
    assert_eq!(json(&session), before);
}

#[test]
fn a_held_key_takes_one_step_back_per_repeat_event() {
    let mut session = fresh();
    for n in 0..5 {
        draw_square(&mut session, f64::from(n) * 30.0);
    }
    for _ in 0..5 {
        let outcome = session.key_down(KeyInput {
            key: "z",
            ctrl: true,
            repeat: true,
            ..KeyInput::default()
        });
        assert_eq!(outcome, KeyOutcome::Undo);
        let _ = session.take_history_notice();
    }
    assert_eq!(ids(&session).len(), 0);
    assert_eq!(undo(&mut session), "Nothing to undo.");
}

#[test]
fn the_typed_character_decides_not_the_place_of_the_key() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    // A German keyboard types "z" where a US one types "y": the character is what arrives.
    assert_eq!(key(&mut session, "z", false), KeyOutcome::Undo);
    let _ = session.take_history_notice();
    assert_eq!(session.redo_len(), 1);
    assert_eq!(key(&mut session, "Z", true), KeyOutcome::Redo);
    let _ = session.take_history_notice();
    assert_eq!(session.undo_len(), 1);
    // Ctrl+Y redoes on Linux and Windows, and not on macOS.
    assert_eq!(key(&mut session, "z", false), KeyOutcome::Undo);
    let _ = session.take_history_notice();
    let mac = session.key_down(KeyInput {
        key: "y",
        ctrl: true,
        mac: true,
        ..KeyInput::default()
    });
    assert_eq!(mac, KeyOutcome::Ignored);
    assert_eq!(key(&mut session, "y", false), KeyOutcome::Redo);
    let _ = session.take_history_notice();
    assert_eq!(session.redo_len(), 0);
}

#[test]
fn the_gate_ignores_the_key_during_a_drag_a_chip_a_text_field_and_an_open_pen_path() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    session.set_tool(Tool::Select);

    // A move drag in flight.
    session.pointer_hover(pt(20.0, 20.0), false, false);
    session.pointer_down(pt(20.0, 20.0), false);
    session.pointer_hover(pt(30.0, 30.0), false, false);
    assert_eq!(key(&mut session, "z", false), KeyOutcome::Ignored);
    session.pointer_up(pt(30.0, 30.0), false, false);
    assert_eq!(session.undo_len(), 2);

    // A typed-entry chip.
    assert_eq!(
        session.key_down(KeyInput {
            key: "m",
            ..KeyInput::default()
        }),
        KeyOutcome::EntryOpened
    );
    assert_eq!(key(&mut session, "z", false), KeyOutcome::Ignored);
    session.escape();

    // Focus in a text field.
    let blocked = session.key_down(KeyInput {
        key: "z",
        ctrl: true,
        dom_blocked: true,
        ..KeyInput::default()
    });
    assert_eq!(blocked, KeyOutcome::Ignored);
    assert_eq!(session.undo_len(), 2, "nothing was touched");

    // An unfinished Pen path.
    session.set_tool(Tool::Pen);
    session.pointer_down(pt(100.0, 100.0), false);
    session.pointer_up(pt(100.0, 100.0), false, false);
    assert_eq!(
        key(&mut session, "z", false),
        KeyOutcome::Hint(KeyHint::FinishPenFirst)
    );
    assert_eq!(session.undo_len(), 2);
}

#[test]
fn a_new_step_after_an_undo_empties_the_redo_stack() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    draw_square(&mut session, 50.0);
    undo(&mut session);
    assert_eq!(session.redo_len(), 1);
    draw_square(&mut session, 90.0);
    assert_eq!(session.redo_len(), 0);
    assert_eq!(redo(&mut session), "Nothing to redo.");
}

#[test]
fn five_hundred_and_one_steps_keep_five_hundred_undos() {
    let mut session = fresh();
    for n in 0..501 {
        session.set_tool(Tool::Rectangle);
        let x = f64::from(n % 20) * 60.0;
        let y = f64::from(n / 20) * 60.0;
        drag(&mut session, pt(x, y), pt(x + 10.0, y + 10.0));
    }
    assert_eq!(session.undo_len(), 500);
    for _ in 0..500 {
        // The bottom of a long stack lies far behind the latest version: the host would show the
        // busy state first, then call `undo`.
        match key(&mut session, "z", false) {
            KeyOutcome::Undo => {
                let _ = session.take_history_notice();
            }
            KeyOutcome::UndoBusy => {
                assert!(session.undo().changed);
            }
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(undo(&mut session), "Nothing to undo.");
    assert_eq!(
        ids(&session).len(),
        1,
        "the 501st step is out of reach, it stays"
    );
}

#[test]
fn save_does_not_touch_the_stacks_and_open_starts_with_empty_ones() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    let bytes = session.pack("0.1.0").unwrap();
    assert_eq!(session.undo_len(), 1);
    assert!(undo(&mut session).starts_with("Undid"));

    let mut reopened = Session::open(3, &bytes).unwrap();
    assert_eq!(reopened.undo_len(), 0);
    assert_eq!(reopened.redo_len(), 0);
    assert_eq!(undo(&mut reopened), "Nothing to undo.");
}

#[test]
fn undo_of_a_union_selects_the_operands_and_redo_selects_the_result() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    draw_square(&mut session, 20.0);
    session.set_tool(Tool::Select);
    session.pointer_down(pt(20.0, 20.0), false);
    session.pointer_up(pt(20.0, 20.0), false, false);
    session.pointer_down(pt(11.0, 11.0), true);
    session.pointer_up(pt(11.0, 11.0), true, false);
    // Select both by marquee instead of clicks: simpler and exact.
    session.pointer_down(pt(0.0, 0.0), false);
    session.pointer_up(pt(60.0, 60.0), false, false);
    assert_eq!(session.selected_object_count(), 2);
    let before = json(&session);
    session.apply_boolean(BooleanOp::Union);
    assert_eq!(session.selected_object_count(), 1);
    let united = json(&session);

    assert!(undo(&mut session).starts_with("Undid: Union"));
    assert_eq!(json(&session), before);
    assert_eq!(session.selected_object_count(), 2);
    assert!(redo(&mut session).starts_with("Redid: Union"));
    assert_eq!(json(&session), united);
    assert_eq!(session.selected_object_count(), 1);
}

#[test]
fn undo_of_delete_brings_the_object_back_with_its_id_and_selects_it() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    draw_square(&mut session, 50.0);
    let before = json(&session);
    let ids_before = ids(&session);
    session.set_tool(Tool::Select);
    session.delete_selected();
    assert_eq!(ids(&session).len(), 1);

    let notice = undo(&mut session);
    assert_eq!(notice, "Undid: Delete (1 rectangle).");
    assert_eq!(json(&session), before);
    assert_eq!(ids(&session), ids_before);
    assert_eq!(session.selected_object_count(), 1);

    // Save and open: the ids are the same as before the delete.
    let reopened = Session::open(3, &session.pack("0.1.0").unwrap()).unwrap();
    assert_eq!(
        unpack(9, &reopened.pack("0.1.0").unwrap())
            .unwrap()
            .object_ids(),
        ids_before
    );
}

#[test]
fn a_gesture_that_writes_nothing_adds_no_step() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    session.set_tool(Tool::Select);
    // A drag that ends where it began; a click; Escape during a drag.
    session.pointer_down(pt(20.0, 20.0), false);
    session.pointer_hover(pt(35.0, 35.0), false, false);
    session.pointer_hover(pt(20.0, 20.0), false, false);
    session.pointer_up(pt(20.0, 20.0), false, false);
    session.pointer_down(pt(20.0, 20.0), false);
    session.pointer_hover(pt(40.0, 40.0), false, false);
    session.escape();
    session.pointer_up(pt(40.0, 40.0), false, false);
    assert_eq!(session.undo_len(), 1);
}

#[test]
fn two_identical_moves_within_a_second_are_two_steps() {
    let mut session = fresh();
    session.set_clock(|| curvyo_document_core::Timestamp::from_secs(1_700_000_000));
    draw_square(&mut session, 10.0);
    session.set_tool(Tool::Select);
    for _ in 0..2 {
        assert_eq!(
            session.key_down(KeyInput {
                key: "m",
                ..KeyInput::default()
            }),
            KeyOutcome::EntryOpened
        );
        session.commit_move_entry(
            "5",
            "0",
            curvyo_ui_core::MoveEntryMode {
                absolute: false,
                copy: false,
            },
        );
    }
    assert_eq!(session.undo_len(), 3);
    assert!(undo(&mut session).starts_with("Undid: Move"));
    assert!(undo(&mut session).starts_with("Undid: Move"));
    assert_eq!(
        ids(&session).len(),
        1,
        "two presses were needed, and the square is still there"
    );
}

#[test]
fn a_restore_off_screen_says_so_and_leaves_pan_and_zoom_alone() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    session.set_tool(Tool::Select);
    session.delete_selected();
    // Pan far away from the square, then undo the delete.
    session.begin_pan(100.0, 100.0);
    session.pan_to(100.0 + 5000.0, 100.0);
    session.end_pan();
    let zoom = session.zoom_percent();
    let notice = undo(&mut session);
    assert_eq!(notice, "Undid: Delete (1 rectangle), off screen.");
    assert_eq!(session.zoom_percent(), zoom);
    assert_eq!(session.selected_object_count(), 1);
}

#[test]
fn a_style_value_drag_blocks_the_keys_like_any_drag() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    session.set_tool(Tool::Select);
    session.preview_style_hsv(curvyo_ui_core::StyleField::StrokeColor, 0.5, 0.5, 0.5);
    assert_eq!(key(&mut session, "z", false), KeyOutcome::Ignored);
    session.commit_style_preview();
    assert_eq!(session.undo_len(), 2);
}

#[test]
fn a_held_arrow_key_is_one_step_and_one_press_takes_the_whole_run_back() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    session.pointer_down(pt(-50.0, -50.0), false);
    session.pointer_up(pt(500.0, 500.0), false, false);
    let before = json(&session);
    let steps = session.undo_len();
    for n in 0..6_u32 {
        let outcome = session.key_down(KeyInput {
            key: "ArrowRight",
            repeat: n > 0,
            time_ms: 1_000.0 + f64::from(n) * 30.0,
            ..KeyInput::default()
        });
        assert!(matches!(outcome, KeyOutcome::Nudged { .. }), "{outcome:?}");
    }
    assert_ne!(json(&session), before);
    assert_eq!(session.undo_len(), steps + 1, "eleven commits, one step");
    assert!(undo(&mut session).starts_with("Undid: Move"));
    assert_eq!(json(&session), before);

    // A nudge right after Ctrl+Z is a new step and never joins the undo.
    let outcome = session.key_down(KeyInput {
        key: "ArrowRight",
        time_ms: 5_000.0,
        ..KeyInput::default()
    });
    assert!(matches!(outcome, KeyOutcome::Nudged { new_run: true }));
    assert_eq!(session.undo_len(), steps + 1);
}
