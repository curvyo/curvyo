//! Live mode of the Object scope (`specs/0042-history-branches` criteria 10 to 13 and 16): a press
//! goes to the version as one step, further presses explore inside it, Restore previous state and
//! Ctrl+Z take the exploration back whole.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use curvyo_document_core::{Point, Timestamp};
use curvyo_editor_wasm::{GoToOutcome, KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::StyleField;
use serde_json::Value;

fn draw_rect(s: &mut Session, x: f64) {
    s.set_tool(Tool::Rectangle);
    let (a, b) = (Point::new(x, 20.0), Point::new(x + 60.0, 60.0));
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
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

fn live_widths() -> Session {
    let mut s = Session::new(21);
    s.set_clock(|| Timestamp::from_secs(1_900_000_000));
    s.resize_viewport(1200.0, 800.0);
    draw_rect(&mut s, 20.0);
    for mm in ["1", "2", "3", "4"] {
        assert!(s.set_style_text(StyleField::StrokeWidth, mm).unwrap());
    }
    s.set_history_live(true);
    s
}

fn doc_json(s: &Session) -> Vec<u8> {
    curvyo_document_core::unpack(99, &s.pack("0.1.0").unwrap())
        .unwrap()
        .export_json()
        .unwrap()
}

fn rows(s: &mut Session) -> Vec<Value> {
    let rows: Value = serde_json::from_str(&s.object_history_rows_json(0, 50)).unwrap();
    rows.as_array().unwrap().clone()
}

#[test]
fn c10_a_press_in_live_goes_to_the_version_as_one_step() {
    let mut s = live_widths();
    let undo = s.undo_len();
    let target = rows(&mut s)[3]["id"].as_str().unwrap().to_owned();
    let outcome = s.go_to_history_item(3);
    assert_eq!(outcome, GoToOutcome::Went(format!("Went to {target}.")));
    assert_eq!(s.undo_len(), undo + 1);
    let top = &rows(&mut s)[0];
    assert_eq!(top["name"], format!("Go to version {target}"));
    assert_eq!(top["summary"], "Change style");
}

#[test]
fn c11_c12_five_presses_are_one_step_and_restore_takes_them_back() {
    let mut s = live_widths();
    let before = doc_json(&s);
    let undo = s.undo_len();
    let len = s.history_len();
    for index in [1, 4, 3, 1, 3] {
        assert!(matches!(s.go_to_history_item(index), GoToOutcome::Went(_)));
    }
    assert_eq!(s.undo_len(), undo + 1, "one stack entry");
    assert_eq!(s.history_len(), len + 1, "one row");
    assert!(s.history_exploring());
    assert_eq!(
        s.restore_exploration().as_deref(),
        Some("Object restored to the state before the first click.")
    );
    assert_eq!(doc_json(&s), before);
    assert!(!s.history_exploring());
    assert_eq!(rows(&mut s)[0]["status"], "undone");
}

#[test]
fn c11_ctrl_z_takes_the_exploration_back_whole() {
    let mut s = live_widths();
    let before = doc_json(&s);
    for index in [1, 3, 2] {
        assert!(matches!(s.go_to_history_item(index), GoToOutcome::Went(_)));
    }
    if key(&mut s, "z", true, false) == KeyOutcome::UndoBusy {
        s.undo();
    }
    assert_eq!(doc_json(&s), before);
}

#[test]
fn c13_selecting_another_object_ends_the_exploration() {
    let mut s = live_widths();
    draw_rect(&mut s, 300.0);
    let click = |s: &mut Session, x: f64| {
        s.pointer_hover(Point::new(x, 40.0), false, false);
        s.pointer_down(Point::new(x, 40.0), false);
        s.pointer_up(Point::new(x, 40.0), false, false);
    };
    click(&mut s, 20.0);
    assert_eq!(s.selected_object_count(), 1);
    assert!(matches!(s.go_to_history_item(2), GoToOutcome::Went(_)));
    let undo = s.undo_len();
    click(&mut s, 300.0);
    let _ = s.history_preview_json();
    assert!(!s.history_exploring());
    click(&mut s, 20.0);
    // The first object again: the next press is a step of its own.
    assert!(matches!(s.go_to_history_item(4), GoToOutcome::Went(_)));
    assert_eq!(s.undo_len(), undo + 1);
}

#[test]
fn c10_the_version_the_object_has_writes_nothing_and_a_drag_ignores_the_press() {
    let mut s = live_widths();
    let undo = s.undo_len();
    assert_eq!(s.go_to_history_item(0), GoToOutcome::Same);
    assert_eq!(s.undo_len(), undo);
    s.set_tool(Tool::Rectangle);
    s.pointer_hover(Point::new(400.0, 400.0), false, false);
    s.pointer_down(Point::new(400.0, 400.0), false);
    s.pointer_hover(Point::new(450.0, 450.0), false, false);
    assert_eq!(s.go_to_history_item(2), GoToOutcome::Ignored);
}
