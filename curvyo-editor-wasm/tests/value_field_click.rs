//! A click on a value field sets the value at the pointer (`specs/0047-transform-polish/` Part B,
//! criteria 33 to 36): the host sends the pointer's position `p` on the field as one preview tick
//! and commits it. The mapping to a value is Rust's; this checks that one such click is one
//! history step, with the value the scale gives at `p`, and that undo and redo are byte-equal.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::manual_assert_eq
)]

use curvyo_document_core::{Direction, Point, unpack};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::{Grid, ValueField};

fn session_with_selected_rectangle() -> Session {
    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    session.set_tool(Tool::Rectangle);
    session.pointer_down(Point::new(0.0, 0.0), false);
    session.pointer_up(Point::new(10.0, 10.0), false, false);
    session.set_tool(Tool::Select);
    let edge = Point::new(0.0, 5.0);
    session.pointer_hover(edge, false, false);
    session.pointer_down(edge, false);
    session.pointer_up(edge, false, false);
    assert_eq!(session.selected_object_count(), 1);
    session
}

/// The document as its JSON export: what undo and redo must restore byte for byte.
fn bytes(session: &Session) -> Vec<u8> {
    unpack(9, &session.pack("0.1.0").unwrap())
        .unwrap()
        .export_json()
        .unwrap()
}

fn key(session: &mut Session, key: &str, shift: bool) -> KeyOutcome {
    session.key_down(KeyInput {
        key,
        shift,
        ctrl: true,
        ..KeyInput::default()
    })
}

/// A click at position `p`: one preview tick, then the commit of the release.
fn click_at(session: &mut Session, field: ValueField, p: f64) {
    session.preview_value_field(field, p, Grid::Normal);
    session.commit_style_preview();
}

#[test]
fn a_click_sets_the_value_at_the_pointer_as_one_step() {
    for field in [
        ValueField::StrokeWidth,
        ValueField::StrokeOpacity,
        ValueField::FillOpacity,
    ] {
        for p in [0.0, 0.25, 0.5, 0.9, 1.0] {
            // A width of 0 is not written (the drag refuses it too): the click at the very
            // start of the width scale leaves the width as it was.
            if field == ValueField::StrokeWidth && p == 0.0 {
                continue;
            }
            let mut session = session_with_selected_rectangle();
            session.warm_history();
            let before = bytes(&session);
            let steps = session.undo_len();
            click_at(&mut session, field, p);
            let scale = field.scale();
            let expected = scale.round(scale.value_at(p), Grid::Normal);
            let shown = session
                .style_panel_state()
                .and_then(|state| state.value_of(field))
                .unwrap();
            assert!(
                (shown - expected).abs() < 1e-9,
                "{field:?} at {p}: {shown} instead of {expected}"
            );
            let after = bytes(&session);
            if after == before {
                // The click landed on the value the object already had: nothing is written.
                assert_eq!(session.undo_len(), steps, "{field:?} at {p}: no step");
                continue;
            }
            assert_eq!(session.undo_len(), steps + 1, "{field:?} at {p}: one step");
            assert_eq!(
                session.history_target_name(Direction::Undo),
                Some("Change style")
            );
            assert_eq!(key(&mut session, "z", false), KeyOutcome::Undo);
            let _ = session.take_history_notice();
            assert!(bytes(&session) == before, "{field:?} at {p}: undo");
            assert_eq!(key(&mut session, "z", true), KeyOutcome::Redo);
            let _ = session.take_history_notice();
            assert!(bytes(&session) == after, "{field:?} at {p}: redo");
        }
    }
}

/// Escape between the press and the release drops the preview: nothing is written.
#[test]
fn escape_before_the_release_writes_nothing() {
    let mut session = session_with_selected_rectangle();
    let before = bytes(&session);
    session.preview_value_field(ValueField::StrokeWidth, 0.7, Grid::Normal);
    session.cancel_style_preview();
    session.commit_style_preview();
    assert!(bytes(&session) == before);
}

/// A position that is not a number writes nothing (the host also clamps it).
#[test]
fn a_position_that_is_not_a_number_writes_nothing() {
    let mut session = session_with_selected_rectangle();
    let before = bytes(&session);
    session.preview_value_field(ValueField::StrokeOpacity, f64::NAN, Grid::Normal);
    session.commit_style_preview();
    assert!(bytes(&session) == before);
}

/// A plain click never sets an opacity to 0 %: the host clamps the position to 0.0214, which the
/// scale rounds to 1 %, so the object stays visible (`0047` criterion 51). The left end of a drag
/// (position 0) still gives 0 %.
#[test]
fn the_smallest_click_position_of_an_opacity_gives_one_percent() {
    for field in [ValueField::StrokeOpacity, ValueField::FillOpacity] {
        let mut session = session_with_selected_rectangle();
        session.preview_value_field(field, 0.0214, Grid::Normal);
        session.commit_style_preview();
        let shown = session
            .style_panel_state()
            .and_then(|state| state.value_of(field))
            .unwrap();
        assert!((shown - 1.0).abs() < 1e-9, "{field:?}: {shown}");
        session.preview_value_field(field, 0.0, Grid::Normal);
        session.commit_style_preview();
        let shown = session
            .style_panel_state()
            .and_then(|state| state.value_of(field))
            .unwrap();
        assert!(
            shown.abs() < 1e-9,
            "{field:?} at the left end of a drag: {shown}"
        );
    }
}
