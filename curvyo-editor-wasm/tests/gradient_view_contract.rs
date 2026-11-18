//! The shared Rust/TypeScript fixtures of the gradient views (`specs/0124-gradient-fill`
//! criteria 3, 6 to 8): each state is built through the public session and its
//! `gradient_view` and `gradient_handles` must equal the fixture in
//! `tests/fixtures/gradient_view/`; the frontend decoders read the same files
//! (`frontend/tests/gradientView.test.ts`). `CURVYO_WRITE_FIXTURES=1` rewrites them.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use curvyo_document_core::Point;
use curvyo_editor_wasm::{KeyInput, Session, Tool};
use serde_json::{Value, json};

/// Compares `value` with the fixture `name`, or writes it with `CURVYO_WRITE_FIXTURES`.
fn pin(name: &str, value: &Value) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gradient_view")
        .join(format!("{name}.json"));
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut text = serde_json::to_string_pretty(value).unwrap();
        text.push('\n');
        std::fs::write(&path, text).unwrap();
    }
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "{} is missing; run with CURVYO_WRITE_FIXTURES=1",
            path.display()
        )
    });
    let fixture: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(&fixture, value, "{name}");
}

fn both(session: &Session) -> Value {
    json!({
        "view": serde_json::to_value(session.gradient_view()).unwrap(),
        "handles": serde_json::to_value(session.gradient_handles()).unwrap(),
    })
}

fn rect(session: &mut Session, from: (f64, f64), to: (f64, f64)) {
    session.set_tool(Tool::Rectangle);
    session.pointer_down(Point::new(from.0, from.1), false);
    session.pointer_up(Point::new(to.0, to.1), false, false);
}

fn select_all(session: &mut Session) {
    session.set_tool(Tool::Select);
    session.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
}

/// A 1200 x 800 canvas at 1 px per mm with a 100 x 40 mm rectangle filled with a gradient.
fn shaded() -> Session {
    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    rect(&mut session, (10.0, 10.0), (110.0, 50.0));
    select_all(&mut session);
    session.set_fill_paint(true);
    assert!(session.set_fill_kind("gradient"));
    session
}

#[test]
fn nothing_selected() {
    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    pin("nothing_selected", &both(&session));
    rect(&mut session, (10.0, 10.0), (110.0, 50.0));
    select_all(&mut session);
    pin("solid_offered", &both(&session));
}

#[test]
fn linear_default() {
    pin("linear_default", &both(&shaded()));
}

#[test]
fn radial_with_an_inner_stop_and_its_entry_box() {
    let mut session = shaded();
    assert!(session.set_gradient_kind("radial"));
    assert!(session.add_gradient_stop());
    assert_eq!(session.set_gradient_stop_hex(1, "#3366CC80"), Ok(true));
    let handles = session.gradient_handles();
    let radius = &handles.handles[1];
    let at = session.screen_to_document(radius.x, radius.y);
    session.double_click(at, false, false);
    pin("radial_entry", &both(&session));
}

#[test]
fn an_end_drag_shows_its_readout() {
    let mut session = shaded();
    let end = Point::new(110.0, 30.0);
    session.pointer_down(end, false);
    session.pointer_hover(Point::new(100.0, 20.0), false, false);
    pin("end_drag_readout", &both(&session));
}

#[test]
fn mixed_gradients() {
    let mut session = shaded();
    rect(&mut session, (200.0, 10.0), (260.0, 50.0));
    session.set_tool(Tool::Select);
    session.pointer_down(Point::new(230.0, 10.0), false);
    session.pointer_up(Point::new(230.0, 10.0), false, false);
    session.set_fill_paint(true);
    assert!(session.set_fill_kind("gradient"));
    assert!(session.add_gradient_stop());
    select_all(&mut session);
    pin("mixed_gradients", &both(&session));
}
