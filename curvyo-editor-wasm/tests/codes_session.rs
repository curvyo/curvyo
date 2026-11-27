//! The QR code through the session (`specs/0144-qr-and-barcodes` criteria 5 and 13, ADR 0091): the
//! bar offers it between Gear and Rounded box, a press places it with the step "Add QR code", text
//! typed into the Content field previews and commits, a text that cannot be drawn keeps the last
//! geometry, shows its reason in the panel and draws the No solution outline.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{Document, Point, pack};
use curvyo_editor_wasm::{Session, Tool};
use serde_json::Value;

const QR: &str = "curvyo.codes/qr";

fn session() -> Session {
    let mut session = Session::open(2, &pack(&Document::new(1), "0.1.0").unwrap()).unwrap();
    session.set_tool(Tool::Generators);
    session.resize_viewport(1600.0, 900.0);
    session.set_generator_kind(QR);
    assert!(session.place_generated(200.0, 150.0));
    session.set_tool(Tool::Select);
    session
}

fn panel(session: &Session) -> Value {
    serde_json::from_str(&session.generated_panel_json()).unwrap()
}

fn row<'a>(panel: &'a Value, key: &str) -> &'a Value {
    panel["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["param"]["key"] == key)
        .unwrap_or_else(|| panic!("no row {key}"))
}

/// A marquee around the code placed at (200, 150).
fn marquee(s: &mut Session) {
    s.set_tool(Tool::Select);
    s.pointer_hover(Point::new(100.0, 50.0), false, false);
    s.pointer_down(Point::new(100.0, 50.0), false);
    s.pointer_hover(Point::new(300.0, 250.0), false, false);
    s.pointer_up(Point::new(300.0, 250.0), false, false);
    assert_eq!(s.selected_object_count(), 1);
}

fn quote(text: &str) -> String {
    serde_json::to_string(text).unwrap()
}

#[test]
fn a_press_places_a_qr_code_in_one_step_called_add_qr_code() {
    let mut s = session();
    assert_eq!(s.tool(), Tool::Select);
    let block = panel(&s);
    assert_eq!(block["label"], "QR code");
    assert_eq!(
        row(&block, "content")["value"]["value"],
        "https://example.com"
    );
    assert_eq!(row(&block, "content")["param"]["kind"]["kind"], "text");
    assert_eq!(
        row(&block, "module_fill")["param"]["kind"]["kind"],
        "percent"
    );
    assert!(row(&block, "content")["problem"].is_null());
    assert_eq!(
        block["contrast_low"], false,
        "black on the default background"
    );
    let name = s.history_target_name(curvyo_document_core::Direction::Undo);
    assert_eq!(name.as_deref(), Some("Add QR code"));
    let kinds: Value = serde_json::from_str(&s.generator_kinds_json()).unwrap();
    // One segment "Code" for the whole family, in the place of the QR code's own (slice 2, criterion 5).
    assert_eq!(kinds[2]["segment"], "Code");
    assert_eq!(kinds[2]["label"], "QR code");
    assert_eq!(kinds[2]["icon"], "QrCode");
}

#[test]
fn typed_text_previews_then_commits_as_one_step_named_after_the_field() {
    let mut s = session();
    let answer = s.preview_generated_value("content", &quote("Hello"));
    assert!(!answer.contains("error"), "{answer}");
    assert_eq!(
        row(&panel(&s), "content")["value"]["value"],
        "Hello",
        "the field follows the preview"
    );
    let answer = s.set_generated_value("content", &quote("Hello"));
    assert!(answer.contains("\"written\":true"), "{answer}");
    let name = s.history_target_name(curvyo_document_core::Direction::Undo);
    assert_eq!(
        name.as_deref(),
        Some("QR code: Content"),
        "never the text itself"
    );
}

#[test]
fn a_text_that_cannot_be_drawn_is_stored_with_its_reason_and_the_no_solution_outline() {
    let mut s = session();
    let valid = s.draw_list().triangles.len();
    let answer = s.set_generated_value("content", &quote(""));
    assert!(answer.contains("\"written\":true"), "{answer}");
    let block = panel(&s);
    assert_eq!(row(&block, "content")["problem"], "Enter a text to encode.");
    assert_eq!(row(&block, "content")["value"]["value"], "");
    assert!(
        s.draw_list().triangles.len() > valid,
        "the kept outline adds the dashed No solution look"
    );
    s.set_generated_value("content", &quote("Hello"));
    assert!(row(&panel(&s), "content")["problem"].is_null());
    s.set_generated_value("content", &quote(&"a".repeat(2332)));
    assert_eq!(
        row(&panel(&s), "content")["problem"],
        "Too long for a QR code at error correction M: 2332 bytes, at most 2331."
    );
}

#[test]
fn a_saved_and_reopened_code_keeps_its_params_and_a_refused_one_keeps_its_line_and_outline() {
    let mut s = session();
    s.set_generated_value("content", &quote("Hello"));
    s.set_generated_value("module_shape", "\"dot\"");
    let valid = s.draw_list().triangles.len();
    let bytes = s.pack("0.1.0").unwrap();
    let mut reopened = Session::open(2, &bytes).unwrap();
    marquee(&mut reopened);
    let block = panel(&reopened);
    assert_eq!(row(&block, "content")["value"]["value"], "Hello");
    assert_eq!(row(&block, "module_shape")["value"]["value"], "dot");
    assert!(row(&block, "content")["problem"].is_null());
    assert_eq!(
        reopened.draw_list().triangles.len(),
        valid,
        "drawn as stored"
    );

    s.set_generated_value("content", &quote(""));
    let refused = s.draw_list().triangles.len();
    assert!(refused > valid, "the outline is there");
    let bytes = s.pack("0.1.0").unwrap();
    let mut reopened = Session::open(2, &bytes).unwrap();
    marquee(&mut reopened);
    let block = panel(&reopened);
    assert_eq!(row(&block, "content")["problem"], "Enter a text to encode.");
    assert_eq!(row(&block, "content")["value"]["value"], "");
    assert_eq!(
        reopened.draw_list().triangles.len(),
        refused,
        "the No solution outline is drawn again after reopening"
    );
}
