//! The gate of the file chords in the editor session (`specs/0051-sidebar-and-no-menu/`
//! criterion 33): ignored during a key repeat, a blocked page, a pointer drag and an open entry
//! chip; an unfinished Pen path does not block. `key_down` keeps ignoring Ctrl+S.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::Point;
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::{FileCommand, Platform};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fresh() -> Session {
    let mut session = Session::new(2);
    session.resize_viewport(1200.0, 800.0);
    session
}

fn chord(key: &str, shift: bool) -> KeyInput<'_> {
    KeyInput {
        key,
        shift,
        ctrl: true,
        ..KeyInput::default()
    }
}

fn square(session: &mut Session) {
    session.set_tool(Tool::Rectangle);
    session.pointer_hover(pt(10.0, 10.0), false, false);
    session.pointer_down(pt(10.0, 10.0), false);
    session.pointer_hover(pt(60.0, 50.0), false, false);
    session.pointer_up(pt(60.0, 50.0), false, false);
    session.set_tool(Tool::Select);
}

#[test]
fn a_chord_that_may_act_is_named() {
    let session = fresh();
    for (key, shift, want) in [
        ("n", false, FileCommand::New),
        ("o", false, FileCommand::Open),
        ("s", false, FileCommand::Save),
        ("S", true, FileCommand::SaveAs),
        ("w", false, FileCommand::Close),
        ("q", false, FileCommand::Quit),
    ] {
        assert_eq!(
            session.file_chord(&chord(key, shift), Platform::Linux),
            Some(want),
            "{key}"
        );
    }
    assert_eq!(
        session.file_chord(&chord("n", false), Platform::Browser),
        None
    );
}

#[test]
fn the_editors_own_key_table_still_ignores_the_file_chords() {
    let mut session = fresh();
    for (key, shift) in [
        ("s", false),
        ("s", true),
        ("n", false),
        ("o", false),
        ("w", false),
    ] {
        assert_eq!(
            session.key_down(chord(key, shift)),
            KeyOutcome::Ignored,
            "{key}"
        );
    }
}

#[test]
fn a_key_repeat_and_a_blocked_page_ignore_the_chord() {
    let session = fresh();
    let repeat = KeyInput {
        repeat: true,
        ..chord("s", false)
    };
    let blocked = KeyInput {
        dom_blocked: true,
        ..chord("s", false)
    };
    assert_eq!(session.file_chord(&repeat, Platform::Linux), None);
    assert_eq!(session.file_chord(&blocked, Platform::Linux), None);
}

#[test]
fn a_pointer_drag_ignores_the_chord_until_the_release() {
    let mut session = fresh();
    square(&mut session);
    let id_point = pt(30.0, 30.0);
    session.pointer_hover(id_point, false, false);
    session.pointer_down(id_point, false);
    session.pointer_hover(pt(80.0, 90.0), false, false);
    assert_eq!(
        session.file_chord(&chord("s", false), Platform::Linux),
        None
    );
    session.pointer_up(pt(80.0, 90.0), false, false);
    assert_eq!(
        session.file_chord(&chord("s", false), Platform::Linux),
        Some(FileCommand::Save)
    );
}

#[test]
fn a_drag_in_every_creation_tool_ignores_the_chord() {
    for tool in [Tool::Rectangle, Tool::Ellipse, Tool::PolygonStar] {
        let mut session = fresh();
        session.set_tool(tool);
        session.pointer_hover(pt(10.0, 10.0), false, false);
        session.pointer_down(pt(10.0, 10.0), false);
        session.pointer_hover(pt(60.0, 50.0), false, false);
        assert_eq!(
            session.file_chord(&chord("s", false), Platform::Linux),
            None,
            "{tool:?}"
        );
        session.pointer_up(pt(60.0, 50.0), false, false);
        assert!(
            session
                .file_chord(&chord("s", false), Platform::Linux)
                .is_some()
        );
    }
}

#[test]
fn an_open_entry_chip_ignores_the_chord() {
    let mut session = fresh();
    square(&mut session);
    assert_eq!(
        session.key_down(KeyInput {
            key: "m",
            ..KeyInput::default()
        }),
        KeyOutcome::EntryOpened
    );
    assert_eq!(
        session.file_chord(&chord("s", false), Platform::Linux),
        None
    );
    session.key_down(KeyInput {
        key: "Escape",
        ..KeyInput::default()
    });
    assert!(
        session
            .file_chord(&chord("s", false), Platform::Linux)
            .is_some()
    );
}

#[test]
fn an_unfinished_pen_path_does_not_block_the_chord() {
    let mut session = fresh();
    session.set_tool(Tool::Pen);
    for point in [pt(10.0, 10.0), pt(50.0, 40.0)] {
        session.pointer_down(point, false);
        session.pointer_up(point, false, false);
    }
    assert!(session.pen_in_progress().is_some());
    assert_eq!(
        session.file_chord(&chord("s", false), Platform::Linux),
        Some(FileCommand::Save)
    );
}
