//! The History list with branches as the panel reads it (`specs/0042-history-branches` criteria
//! 1 to 4, 8 and 9): step rows, header rows and the "more" row with their lanes, in both scopes.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use curvyo_document_core::{Point, Timestamp};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::StyleField;
use serde_json::Value;

fn fresh_rect() -> Session {
    let mut s = Session::new(21);
    s.set_clock(|| Timestamp::from_secs(1_900_000_000));
    s.resize_viewport(1200.0, 800.0);
    s.set_tool(Tool::Rectangle);
    let (a, b) = (Point::new(20.0, 20.0), Point::new(80.0, 60.0));
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
    s.set_tool(Tool::Select);
    assert_eq!(key(&mut s, "a", true, false), KeyOutcome::SelectedAll);
    s
}

fn key(s: &mut Session, k: &str, ctrl: bool, shift: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        shift,
        ..KeyInput::default()
    })
}

fn width(s: &mut Session, mm: &str) {
    assert!(s.set_style_text(StyleField::StrokeWidth, mm).unwrap());
}

fn undo(s: &mut Session) {
    if key(s, "z", true, false) == KeyOutcome::UndoBusy {
        s.undo();
    }
}

fn items(s: &mut Session, object: bool) -> Vec<Value> {
    let shape: Value = serde_json::from_str(&s.history_layout_json(object)).unwrap();
    let total = usize::try_from(shape["total"].as_u64().unwrap()).unwrap();
    let items: Value = serde_json::from_str(&s.history_items_json(object, 0, total)).unwrap();
    items.as_array().unwrap().clone()
}

/// The rectangle, three widths, two undos and a fourth width: the fork of criterion 1.
fn forked() -> Session {
    let mut s = fresh_rect();
    width(&mut s, "1");
    width(&mut s, "2");
    width(&mut s, "3");
    undo(&mut s);
    undo(&mut s);
    width(&mut s, "4");
    s
}

#[test]
fn c3_c4_c8_the_branch_has_a_header_above_its_newest_row_and_its_rows_say_branch() {
    let mut s = forked();
    for object in [false, true] {
        let items = items(&mut s, object);
        let kinds: Vec<&str> = items
            .iter()
            .map(|item| item["type"].as_str().unwrap())
            .collect();
        assert_eq!(kinds, ["row", "header", "row", "row", "row", "row"]);
        let fork_id = items[4]["id"].as_str().unwrap();
        assert_eq!(
            items[1]["text"],
            format!("Branch from {fork_id}, 2 steps"),
            "object scope {object}"
        );
        assert_eq!(items[1]["expanded"], true);
        assert_eq!(items[2]["status"], "branch");
        assert_eq!(items[3]["status"], "branch");
        assert_eq!(items[0]["status"], "applied");
        assert_eq!(
            items[2]["lane"],
            format!("on branch from {fork_id}, step 2 of 2")
        );
        assert_eq!(
            items[3]["lane"],
            format!("on branch from {fork_id}, step 1 of 2")
        );
        assert!(
            items[4]["gutter"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!(["fork", 0, 1])),
            "the fork connector is drawn at the fork row"
        );
    }
}

#[test]
fn c8_c9_a_press_on_the_header_folds_the_branch_and_a_second_unfolds_it() {
    let mut s = forked();
    assert!(s.toggle_history_item(false, 1));
    let folded = items(&mut s, false);
    assert_eq!(folded.len(), 4);
    assert_eq!(folded[1]["expanded"], false);
    assert!(!s.toggle_history_item(false, 0), "a step row does not fold");
    assert!(s.toggle_history_item(false, 1));
    assert_eq!(items(&mut s, false).len(), 6);
}

#[test]
fn c2_undoing_the_new_step_keeps_one_branch_and_redo_brings_the_new_step_back() {
    let mut s = forked();
    undo(&mut s);
    let items = items(&mut s, false);
    assert_eq!(items[0]["status"], "undone");
    assert_eq!(items[1]["type"], "header");
    let notice = if key(&mut s, "Z", true, true) == KeyOutcome::RedoBusy {
        s.redo().notice
    } else {
        s.take_history_notice().unwrap_or_default()
    };
    assert!(notice.starts_with("Redid:"), "{notice}");
    let after = self::items(&mut s, false);
    assert_eq!(after[0]["status"], "applied");
}
