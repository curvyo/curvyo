//! Clone of any version from the History list (`specs/0042-history-branches` criteria 22, 24 and
//! 26): from a row of the Object scope and from a touched object of a Document-scope row; the clone
//! is selected and the notice names the version.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use curvyo_document_core::{Point, Timestamp};
use curvyo_editor_wasm::{CloneOutcome, KeyInput, KeyOutcome, Session, Tool};
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

fn widths() -> Session {
    let mut s = Session::new(21);
    s.set_clock(|| Timestamp::from_secs(1_900_000_000));
    s.resize_viewport(1200.0, 800.0);
    draw_rect(&mut s, 20.0);
    for mm in ["1", "2"] {
        assert!(s.set_style_text(StyleField::StrokeWidth, mm).unwrap());
    }
    s
}

fn count(s: &Session) -> usize {
    curvyo_document_core::unpack(99, &s.pack("0.1.0").unwrap())
        .unwrap()
        .object_ids()
        .len()
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

#[test]
fn c22_clone_on_an_object_row_creates_a_selected_object_and_names_the_version() {
    let mut s = widths();
    let id = rows(&mut s, true)[1]["id"].as_str().unwrap().to_owned();
    let objects = count(&s);
    let outcome = s.clone_history_item(true, 1, 0);
    assert_eq!(
        outcome,
        CloneOutcome::Cloned(format!("Cloned version {id} as a new object."))
    );
    assert_eq!(count(&s), objects + 1);
    assert_eq!(s.selected_object_count(), 1);
    let first = rows(&mut s, true)[0].clone();
    assert!(
        first["name"]
            .as_str()
            .unwrap()
            .contains(&format!("at version {id}")),
        "{first}"
    );
}

#[test]
fn c24_clone_from_a_delete_row_brings_the_deleted_object_back_as_a_clone() {
    let mut s = widths();
    s.key_down(KeyInput {
        key: "Delete",
        ..KeyInput::default()
    });
    assert_eq!(count(&s), 0);
    let delete = &rows(&mut s, false)[0];
    assert_eq!(delete["objects"][0]["role"], "deleted");
    assert!(matches!(
        s.clone_history_item(false, 0, 0),
        CloneOutcome::Cloned(_)
    ));
    assert_eq!(count(&s), 1);
    assert_eq!(
        s.key_down(KeyInput {
            key: "z",
            ctrl: true,
            ..KeyInput::default()
        }),
        KeyOutcome::Undo
    );
    assert_eq!(count(&s), 0, "Ctrl+Z removes the clone");
}

#[test]
fn c26_clone_is_ignored_during_a_drag() {
    let mut s = widths();
    s.set_tool(Tool::Rectangle);
    s.pointer_hover(Point::new(400.0, 400.0), false, false);
    s.pointer_down(Point::new(400.0, 400.0), false);
    s.pointer_hover(Point::new(450.0, 450.0), false, false);
    assert_eq!(s.clone_history_item(false, 1, 0), CloneOutcome::Ignored);
}
