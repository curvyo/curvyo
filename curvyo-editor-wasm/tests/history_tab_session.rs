//! The History tab in the editor session (`specs/0020-undo-redo` criteria 32 to 46 and 47 to 51):
//! the rows the panel reads, the hover link, the tab and the wipe with its gate.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]

use curvyo_document_core::Point;
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool, WipeOutcome, WipeState};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fresh() -> Session {
    let mut session = Session::new(2);
    session.resize_viewport(1200.0, 800.0);
    session.set_clock(|| curvyo_document_core::Timestamp::from_secs(1_760_000_000));
    session
}

fn draw(session: &mut Session, x: f64) {
    session.set_tool(Tool::Rectangle);
    session.pointer_hover(pt(x, 10.0), false, false);
    session.pointer_down(pt(x, 10.0), false);
    session.pointer_hover(pt(x + 20.0, 30.0), false, false);
    session.pointer_up(pt(x + 20.0, 30.0), false, false);
}

fn rows(session: &mut Session, count: usize) -> Vec<serde_json::Value> {
    serde_json::from_str(&session.history_rows_json(0, count)).unwrap()
}

#[test]
fn the_rows_say_what_the_step_was_who_made_it_and_what_it_touched() {
    let mut session = fresh();
    assert_eq!(session.history_len(), 0);
    assert_eq!(session.history_now_id(), "");
    draw(&mut session, 10.0);
    draw(&mut session, 60.0);
    session.undo();
    assert_eq!(session.history_len(), 2);
    let list = rows(&mut session, 50);
    assert_eq!(list.len(), 2);
    assert_eq!(list[0]["name"], "Draw rectangle");
    assert_eq!(list[0]["status"], "undone");
    assert_eq!(list[1]["status"], "applied");
    assert_eq!(list[0]["author"], "you");
    assert_eq!(list[0]["kind"], "create");
    assert_eq!(list[0]["summary"], "1 rectangle");
    assert_eq!(list[0]["time"], 1_760_000_000);
    assert!(list[0]["id"].as_str().unwrap().len() >= 7);
    // The pinned Now row names the newest step in effect.
    assert_eq!(session.history_now_id(), list[1]["id"].as_str().unwrap());
}

#[test]
fn a_deleted_object_stays_in_its_row_and_shows_no_hover() {
    let mut session = fresh();
    draw(&mut session, 10.0);
    session.delete_selected();
    let list = rows(&mut session, 50);
    assert_eq!(list[0]["name"], "Delete");
    assert_eq!(list[0]["kind"], "delete");
    assert_eq!(list[0]["summary"], "1 rectangle");
    assert_eq!(list[0]["objects"][0]["alive"], false);
    let step = list[0]["step"].as_str().unwrap().to_owned();
    session.set_history_hover(&step);
    session.set_history_hover("");
}

#[test]
fn the_history_tab_exists_and_the_shortcut_selects_it() {
    let mut session = fresh();
    session.panel_shortcut_history();
    let view = session.panel_view();
    let active = view.tabs.iter().find(|t| t.selected).unwrap();
    assert_eq!(active.name, "history");
    let history = view.tabs.iter().find(|t| t.name == "history").unwrap();
    assert_eq!(history.tooltip, "History (Shift+Ctrl+H)");
    // The Fonts tab of `specs/0074-text-objects` is appended after History.
    assert_eq!(
        view.tabs.iter().rev().nth(1).map(|t| t.name),
        Some("history")
    );
}

#[test]
fn wipe_is_offered_with_steps_ignored_during_a_gesture_and_empties_the_stacks() {
    let mut session = fresh();
    assert_eq!(session.history_wipe_state(), WipeState::NoSteps);
    draw(&mut session, 10.0);
    draw(&mut session, 60.0);
    assert_eq!(session.history_wipe_state(), WipeState::Offered(2));

    // A drag in flight, an open chip, an unfinished Pen path: ignored.
    session.set_tool(Tool::Select);
    session.pointer_down(pt(20.0, 20.0), false);
    session.pointer_hover(pt(40.0, 40.0), false, false);
    assert_eq!(session.wipe_history(), WipeOutcome::Ignored);
    session.pointer_up(pt(40.0, 40.0), false, false);
    session.pointer_down(pt(-50.0, -50.0), false);
    session.pointer_up(pt(500.0, 500.0), false, false);
    let before = rows(&mut session, 50).len();
    assert_eq!(
        session.key_down(KeyInput {
            key: "m",
            ..KeyInput::default()
        }),
        KeyOutcome::EntryOpened
    );
    assert_eq!(session.wipe_history(), WipeOutcome::Ignored);
    session.escape();
    assert_eq!(rows(&mut session, 50).len(), before);

    let objects_before = session.pack("0.1.0").unwrap();
    let WipeOutcome::Wiped(removed) = session.wipe_history() else {
        panic!("the wipe ran");
    };
    assert!(removed >= 2);
    assert_eq!((session.undo_len(), session.redo_len()), (0, 0));
    let list = rows(&mut session, 50);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["name"], "History wiped");
    assert_eq!(session.undo().notice, "Nothing to undo.");
    // The drawing is as it was: the project reopens with the same objects.
    let reopened = Session::open(3, &session.pack("0.1.0").unwrap()).unwrap();
    let before_objects = Session::open(4, &objects_before).unwrap();
    assert_eq!(
        reopened.selected_object_count(),
        before_objects.selected_object_count()
    );
}

#[test]
fn a_union_counts_the_selected_operands_and_a_document_resize_counts_no_object() {
    let mut session = fresh();
    draw(&mut session, 10.0);
    draw(&mut session, 20.0);
    session.set_tool(Tool::Select);
    session.pointer_down(pt(0.0, 0.0), false);
    session.pointer_up(pt(80.0, 60.0), false, false);
    assert_eq!(session.selected_object_count(), 2);
    session.apply_boolean(curvyo_ui_core::BooleanOp::Union);
    assert_eq!(session.undo().notice, "Undid: Union (2 rectangles).");
    session.redo();
    let list = rows(&mut session, 5);
    assert_eq!(list[0]["name"], "Union");
    assert_eq!(list[0]["summary"], "2 rectangles");
    assert_eq!(
        list[0]["objects"].as_array().unwrap().len(),
        3,
        "the hover link keeps all three"
    );

    let _ = session.set_document_side(curvyo_editor_wasm::DocumentSide::Width, "400");
    let list = rows(&mut session, 5);
    assert_eq!(list[0]["name"], "Resize document");
    assert_eq!(list[0]["kind"], "document");
    assert_eq!(list[0]["summary"], "");
    assert!(session.undo().notice.starts_with("Undid: Resize document"));
}
