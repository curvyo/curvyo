//! Previews from the History list (`specs/0042-history-branches` criteria 14 and 16 to 21): a press
//! on a row shows the version and changes nothing; the ways a preview ends; a document preview
//! pauses editing.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use curvyo_document_core::{Point, Timestamp};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, RowPress, Session, Tool};
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

/// A rectangle with widths 1, 2 and 3, selected.
fn widths() -> Session {
    let mut s = Session::new(21);
    s.set_clock(|| Timestamp::from_secs(1_900_000_000));
    s.resize_viewport(1200.0, 800.0);
    draw_rect(&mut s, 20.0);
    assert_eq!(key(&mut s, "a", true, false), KeyOutcome::SelectedAll);
    for mm in ["1", "2", "3"] {
        assert!(s.set_style_text(StyleField::StrokeWidth, mm).unwrap());
    }
    s
}

fn preview(s: &mut Session) -> Value {
    serde_json::from_str(&s.history_preview_json()).unwrap()
}

fn doc_json(s: &Session) -> Vec<u8> {
    curvyo_document_core::unpack(99, &s.pack("0.1.0").unwrap())
        .unwrap()
        .export_json()
        .unwrap()
}

#[test]
fn c14_the_mode_starts_at_preview_and_stays_where_it_is_put() {
    let mut s = widths();
    assert!(!s.history_live());
    s.set_history_live(true);
    assert!(s.history_live());
}

#[test]
fn c17_c20_an_object_preview_draws_the_version_and_writes_nothing() {
    let mut s = widths();
    let before = doc_json(&s);
    let rows = s.history_len();
    let plain = s.frame_draw_list().triangle_count();
    // Rows of the Object scope: 0 is width 3, 2 is width 1.
    assert_eq!(s.preview_history_item(true, 2), RowPress::Previewed);
    let shown = preview(&mut s);
    assert_eq!(shown["kind"], "object");
    assert_eq!(shown["since"], 0);
    assert!(
        s.frame_draw_list().triangle_count() > plain,
        "ghost and outline drawn"
    );
    assert_eq!(doc_json(&s), before, "the document did not change");
    assert_eq!(s.history_len(), rows, "no step");
    assert_eq!(s.undo_len(), 4, "nothing on the stack");
}

#[test]
fn c18_a_press_on_the_canvas_ends_the_preview_and_does_nothing_else() {
    let mut s = widths();
    assert_eq!(s.preview_history_item(true, 2), RowPress::Previewed);
    let selected = s.selected_object_count();
    s.pointer_down(Point::new(500.0, 500.0), false);
    s.pointer_up(Point::new(500.0, 500.0), false, false);
    assert_eq!(preview(&mut s)["kind"], "");
    assert_eq!(
        s.selected_object_count(),
        selected,
        "the press selected nothing"
    );
}

#[test]
fn c18_the_row_of_the_current_version_selecting_another_object_and_live_end_it() {
    let mut s = widths();
    assert_eq!(s.preview_history_item(true, 0), RowPress::Ended);
    assert_eq!(s.preview_history_item(true, 2), RowPress::Previewed);
    s.set_history_live(true);
    assert_eq!(preview(&mut s)["kind"], "");
    s.set_history_live(false);
    assert_eq!(s.preview_history_item(true, 2), RowPress::Previewed);
    draw_rect(&mut s, 200.0);
    assert_eq!(preview(&mut s)["kind"], "", "another object is selected");
    assert_eq!(s.preview_history_item(false, 1), RowPress::Previewed);
    s.end_history_preview();
    assert_eq!(preview(&mut s)["kind"], "");
}

#[test]
fn c19_a_document_preview_pauses_editing_until_back_to_live() {
    let mut s = widths();
    let before = doc_json(&s);
    // The oldest row of the Document scope: the rectangle alone, before any width.
    let last = s.history_len() - 1;
    assert_eq!(s.preview_history_item(false, last), RowPress::Previewed);
    assert_eq!(preview(&mut s)["kind"], "document");
    assert_eq!(key(&mut s, "z", true, false), KeyOutcome::Paused);
    assert_eq!(key(&mut s, "Delete", false, false), KeyOutcome::Paused);
    s.set_tool(Tool::Rectangle);
    draw_rect(&mut s, 300.0);
    assert_eq!(doc_json(&s), before, "no key and no press edited");
    s.end_history_preview();
    assert_eq!(key(&mut s, "z", true, false), KeyOutcome::Undo);
}

#[test]
fn c16_a_press_during_a_drag_is_ignored() {
    let mut s = widths();
    s.set_tool(Tool::Rectangle);
    s.pointer_hover(Point::new(400.0, 400.0), false, false);
    s.pointer_down(Point::new(400.0, 400.0), false);
    s.pointer_hover(Point::new(450.0, 450.0), false, false);
    assert_eq!(s.preview_history_item(false, 1), RowPress::Ignored);
}

#[test]
fn c21_only_rows_a_thousand_back_need_the_long_operation_state() {
    let mut s = widths();
    assert!(!s.history_item_is_far(false, 1));
}
