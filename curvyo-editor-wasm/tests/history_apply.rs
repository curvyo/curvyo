//! Applying a version from the History list and the object preview that hides the current object
//! (`specs/0042-history-branches` criteria 10, 14, 15, 17 and 18, customer feedback 2026-10-14).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]

use curvyo_document_core::{Point, Timestamp};
use curvyo_editor_wasm::{GoToOutcome, HistoryRowPress, KeyInput, KeyOutcome, Session, Tool};
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

/// A rectangle with widths 1, 2 and 9, selected.
fn widths() -> Session {
    let mut s = Session::new(21);
    s.set_clock(|| Timestamp::from_secs(1_900_000_000));
    s.resize_viewport(1200.0, 800.0);
    draw_rect(&mut s, 20.0);
    for mm in ["1", "2", "9"] {
        assert!(s.set_style_text(StyleField::StrokeWidth, mm).unwrap());
    }
    s
}

fn doc_json(s: &Session) -> Vec<u8> {
    curvyo_document_core::unpack(99, &s.pack("0.1.0").unwrap())
        .unwrap()
        .export_json()
        .unwrap()
}

fn rows(s: &mut Session, object: bool) -> Vec<Value> {
    let json = if object {
        s.object_history_rows_json(0, 50)
    } else {
        s.history_rows_json(0, 50)
    };
    serde_json::from_str::<Value>(&json)
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}

fn undo(s: &mut Session) {
    if key(s, "z", true, false) == KeyOutcome::UndoBusy {
        s.undo();
    }
}

/// The x extent of everything drawn besides the document area.
fn drawn_width(s: &Session) -> f64 {
    let frame = s.frame_draw_list();
    let xs = frame.triangles[6..].iter().map(|v| v.position.x);
    xs.clone().fold(f64::NEG_INFINITY, f64::max) - xs.fold(f64::INFINITY, f64::min)
}

#[test]
fn c10_an_object_apply_is_one_step_and_ctrl_z_takes_it_back() {
    let mut s = widths();
    let before = doc_json(&s);
    let undo_len = s.undo_len();
    // Object rows newest first: 0 width 9, 1 width 2, 2 width 1, 3 the rectangle.
    let target = rows(&mut s, true)[2]["id"].as_str().unwrap().to_owned();
    assert_eq!(
        s.apply_history_item(true, 2),
        GoToOutcome::Went(format!("Went to {target}."))
    );
    assert_eq!(s.undo_len(), undo_len + 1);
    assert_eq!(
        rows(&mut s, true)[0]["name"],
        format!("Go to version {target}")
    );
    undo(&mut s);
    assert_eq!(doc_json(&s), before);
}

#[test]
fn c10_a_document_apply_removes_a_drawn_object_and_ctrl_z_brings_it_back() {
    let mut s = widths();
    let before = doc_json(&s);
    draw_rect(&mut s, 300.0);
    let with_two = doc_json(&s);
    // Document rows: 0 the second rectangle, 1 width 9.
    assert!(matches!(
        s.apply_history_item(false, 1),
        GoToOutcome::Went(_)
    ));
    assert_eq!(doc_json(&s), before, "the drawn rectangle is gone");
    undo(&mut s);
    assert_eq!(doc_json(&s), with_two);
}

#[test]
fn c10_three_applies_are_three_steps_and_an_equal_version_writes_nothing() {
    let mut s = widths();
    let undo_len = s.undo_len();
    assert_eq!(s.apply_history_item(true, 0), GoToOutcome::Same);
    for index in [2, 2, 2] {
        // Each apply adds a row on top: row 2 is a different version each time.
        assert!(matches!(
            s.apply_history_item(true, index),
            GoToOutcome::Went(_)
        ));
    }
    assert_eq!(s.undo_len(), undo_len + 3);
}

#[test]
fn c17_an_object_preview_draws_only_the_version_and_escape_brings_the_object_back() {
    let mut s = widths();
    let before = doc_json(&s);
    let live = drawn_width(&s);
    assert_eq!(s.preview_history_item(true, 2), HistoryRowPress::Previewed);
    let previewed = drawn_width(&s);
    assert!(
        live - previewed > 3.0,
        "the 9 mm stroke and the handles are not drawn: {live} vs {previewed}"
    );
    s.end_history_preview();
    assert_eq!(doc_json(&s), before, "ending the preview changes nothing");
    assert!(
        (drawn_width(&s) - live).abs() < 1e-9,
        "the object is drawn again"
    );
}

#[test]
fn c18_undo_ends_an_object_preview_first() {
    let mut s = widths();
    assert_eq!(s.preview_history_item(true, 2), HistoryRowPress::Previewed);
    undo(&mut s);
    let preview: Value = serde_json::from_str(&s.history_preview_json()).unwrap();
    assert_eq!(preview["kind"], "");
}

#[test]
fn c10_c19_an_apply_ends_a_document_preview() {
    let mut s = widths();
    let last = s.history_len() - 1;
    assert_eq!(
        s.preview_history_item(false, last),
        HistoryRowPress::Previewed
    );
    assert!(matches!(
        s.apply_history_item(false, last),
        GoToOutcome::Went(_)
    ));
    let preview: Value = serde_json::from_str(&s.history_preview_json()).unwrap();
    assert_eq!(preview["kind"], "");
}

#[test]
fn c16_an_apply_during_a_drag_is_ignored() {
    let mut s = widths();
    s.set_tool(Tool::Rectangle);
    s.pointer_hover(Point::new(400.0, 400.0), false, false);
    s.pointer_down(Point::new(400.0, 400.0), false);
    s.pointer_hover(Point::new(450.0, 450.0), false, false);
    assert_eq!(s.apply_history_item(false, 1), GoToOutcome::Ignored);
}
