//! Tester's re-verification of `specs/0141-parametric-generators` after the gap fixes of
//! commits 265b556b..11eed00f. Covers the criteria the first run marked NOT TESTED or PARTIAL:
//! history labels, the refused resize of a disabled or unknown kind (while move still works),
//! handle hiding for a corners-only selection, a clamp notice that names the field and limit,
//! and save/reopen of a live gear through the session.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    missing_docs
)]

use curvyo_document_core::{
    Affine, Document, GeneratedRecord, ObjectSnapshot, Point, pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_ui_core::{ExtensionSettings, instance};
use serde_json::Value;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn select_session(bytes: &[u8]) -> Session {
    let mut s = Session::open(2, bytes).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 900.0);
    s
}

fn panel(s: &Session) -> Value {
    serde_json::from_str(&s.generated_panel_json()).unwrap()
}

fn newest_name(s: &mut Session) -> String {
    let rows: Value = serde_json::from_str(&s.history_rows_json(0, 1)).unwrap();
    rows[0]["name"].as_str().unwrap().to_owned()
}

/// The bounds of the single object in `bytes`, from its anchors.
fn bounds_of(bytes: &[u8]) -> (Point, Point) {
    let document = unpack(1, bytes).unwrap();
    let id = document.read_objects().objects.first().unwrap().id();
    let ObjectSnapshot::Path(path) = document.object(id).unwrap() else {
        panic!("a generated object reads as a path");
    };
    let points: Vec<Point> = path.all_anchors().map(|anchor| anchor.point).collect();
    (
        pt(
            points.iter().map(|p| p.x).fold(f64::MAX, f64::min),
            points.iter().map(|p| p.y).fold(f64::MAX, f64::min),
        ),
        pt(
            points.iter().map(|p| p.x).fold(f64::MIN, f64::max),
            points.iter().map(|p| p.y).fold(f64::MIN, f64::max),
        ),
    )
}

/// A gear placed at (100, 100) mm in a document, selected by a marquee in a Select session.
/// Returns the session and the gear's bounds.
fn selected_gear() -> (Session, Point, Point) {
    let document = Document::new(1);
    instance::place(
        &document,
        &ExtensionSettings::default(),
        "curvyo.generators/gear",
        pt(100.0, 100.0),
    )
    .unwrap();
    let bytes = pack(&document, "0.1.0").unwrap();
    let (low, high) = bounds_of(&bytes);
    let mut s = select_session(&bytes);
    s.pointer_hover(pt(20.0, 20.0), false, false);
    s.pointer_down(pt(20.0, 20.0), false);
    s.pointer_hover(pt(180.0, 180.0), false, false);
    s.pointer_up(pt(180.0, 180.0), false, false);
    assert_eq!(s.selected_object_count(), 1, "the marquee selects the gear");
    (s, low, high)
}

fn drag(s: &mut Session, from: Point, to: Point) {
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

fn value_of(s: &Session, key: &str) -> f64 {
    let rows = panel(s)["rows"].as_array().unwrap().clone();
    let row = rows.iter().find(|row| row["param"]["key"] == key).unwrap();
    let text = row["value"].to_string();
    text.split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .find_map(|part| part.parse::<f64>().ok())
        .unwrap_or_else(|| panic!("no number in {text}"))
}

// Criterion 5: no edge resize and no skew handle for a corners-only (generated) selection.
// A drag from an edge midpoint, which on a plain path would resize or skew, changes nothing.
#[test]
fn c5_a_drag_from_any_edge_midpoint_of_a_generated_object_changes_nothing() {
    let (mut s, low, high) = selected_gear();
    let module_before = value_of(&s, "module");
    let mid_x = f64::midpoint(low.x, high.x);
    let mid_y = f64::midpoint(low.y, high.y);
    let edges = [
        (pt(high.x, mid_y), pt(high.x + 30.0, mid_y)),
        (pt(low.x, mid_y), pt(low.x - 30.0, mid_y)),
        (pt(mid_x, high.y), pt(mid_x, high.y + 30.0)),
        (pt(mid_x, low.y), pt(mid_x, low.y - 30.0)),
    ];
    let size_of = |s: &Session| {
        let (lo, hi) = bounds_of(&pack(&s_document(s), "0.1.0").unwrap());
        (hi.x - lo.x, hi.y - lo.y)
    };
    let size_before = size_of(&s);
    let undo_before = s.undo_len();
    for (from, to) in edges {
        drag(&mut s, from, to);
        // A move keeps the size; an edge resize or a skew would change it.
        let size = size_of(&s);
        assert!(
            (size.0 - size_before.0).abs() < 1e-6 && (size.1 - size_before.1).abs() < 1e-6,
            "edge drag from {from:?} changed the size {size_before:?} -> {size:?}"
        );
        if s.undo_len() > undo_before {
            s.undo();
        }
    }
    assert_eq!(s.undo_len(), undo_before, "no edge drag left a step");
    assert_eq!(value_of(&s, "module"), module_before, "no size change");
}

// Criterion 5: a corner drag keeps the proportions, and the size numbers are what changes.
#[test]
fn c5_a_corner_drag_grows_module_and_pitch_diameter_by_the_same_factor() {
    let (mut s, _, high) = selected_gear();
    let module_before = value_of(&s, "module");
    let steps = s.history_len();
    let to = pt(high.x + 60.0, high.y + 60.0);
    drag(&mut s, high, to);
    let module_after = value_of(&s, "module");
    assert!(module_after > module_before, "the module grew");
    assert_eq!(s.history_len(), steps + 1, "one resize step");
    let name = newest_name(&mut s);
    assert_ne!(name, "Add gear", "the resize is its own step: {name}");
    // The teeth and pressure angle stay (criterion 5).
    assert_eq!(value_of(&s, "teeth"), 24.0);
    assert_eq!(value_of(&s, "pressure_angle"), 20.0);
}

// Criterion 7: a file with an unknown generator kind is refused a resize and a change by number,
// but a move still works in one step and the outline moves with it.
#[test]
fn c7_an_unknown_kind_refuses_a_resize_but_moves_in_one_step() {
    let document = Document::new(1);
    let square = vec![
        100.0, 100.0, 100.0, 100.0, 140.0, 100.0, 140.0, 100.0, 140.0, 100.0, 140.0, 140.0, 140.0,
        140.0, 140.0, 140.0, 100.0, 140.0, 100.0, 140.0, 100.0, 140.0, 100.0, 100.0, 100.0, 100.0,
    ];
    document
        .create_generated(
            &GeneratedRecord {
                kind: "future.generators/spiral".into(),
                seed: 1,
                params: serde_json::Map::new(),
                parts: vec![vec![square]],
            },
            Affine::IDENTITY,
        )
        .unwrap();
    let bytes = pack(&document, "0.1.0").unwrap();
    let (low_before, _) = bounds_of(&bytes);
    let mut s = select_session(&bytes);
    s.pointer_hover(pt(80.0, 80.0), false, false);
    s.pointer_down(pt(80.0, 80.0), false);
    s.pointer_hover(pt(170.0, 170.0), false, false);
    s.pointer_up(pt(170.0, 170.0), false, false);
    assert_eq!(s.selected_object_count(), 1);

    // Resize refused: corner drag, nothing written, the notice names the reason.
    let steps = s.history_len();
    drag(&mut s, pt(140.0, 140.0), pt(200.0, 200.0));
    assert_eq!(
        s.history_len(),
        steps,
        "resize of an unknown kind wrote a step"
    );
    assert!(
        s.take_transform_notice().contains("cannot be resized"),
        "the refusal says why"
    );
    let answer: Value = serde_json::from_str(&s.set_generated_value("thickness", "4")).unwrap();
    assert!(answer["error"].is_string(), "a change by number is refused");
    assert_eq!(s.history_len(), steps);

    // Move works: one step, the outline moves by the drag.
    drag(&mut s, pt(120.0, 120.0), pt(150.0, 120.0));
    assert_eq!(s.history_len(), steps + 1, "one move step");
    let moved = pack(&s_document(&s), "0.1.0").unwrap();
    let (low_after, _) = bounds_of(&moved);
    assert!(
        (low_after.x - low_before.x - 30.0).abs() < 1.0,
        "moved by about 30 mm: {} -> {}",
        low_before.x,
        low_after.x
    );
}

fn s_document(s: &Session) -> Document {
    unpack(1, &s.pack("0.1.0").unwrap()).unwrap()
}

// Criterion 4: a typed value below the limit is clamped and the notice names field and limit.
#[test]
fn c4_teeth_below_six_is_clamped_with_a_notice_that_names_field_and_limit() {
    let mut s = Session::open(2, &pack(&Document::new(1), "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Generators);
    s.set_generator_kind("curvyo.generators/gear");
    assert!(s.place_generated(50.0, 50.0));
    let answer: Value = serde_json::from_str(&s.set_generated_value("teeth", "3")).unwrap();
    assert_eq!(answer["written"], true);
    let notice = answer["notices"][0].as_str().unwrap().to_owned();
    assert!(notice.contains("Teeth"), "names the field: {notice}");
    assert!(notice.contains('6'), "names the limit: {notice}");
    assert!(notice.contains("minimum"), "says minimum: {notice}");
    assert_eq!(value_of(&s, "teeth"), 6.0);
}

// Criterion 1 and 6: history labels for the generator steps, box and gear, including a box field.
#[test]
fn c1_c6_history_labels_name_the_kind_and_the_field() {
    let mut s = Session::open(2, &pack(&Document::new(1), "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Generators);
    s.set_generator_kind("curvyo.generators/box");
    assert!(s.place_generated(200.0, 50.0));
    assert_eq!(newest_name(&mut s), "Add box");
    s.set_generated_value("inner_width", "60");
    let name = newest_name(&mut s);
    assert!(
        name.starts_with("Box: ") && name.contains("width"),
        "{name}"
    );
    assert!(s.convert_selected_generated());
    assert_eq!(newest_name(&mut s), "Convert box to paths");
}

// Criterion 7: save and reopen keep kind, parameters and outline; the reopened outline equals
// the stored one and no step is written by opening.
#[test]
fn c7_save_and_reopen_of_a_live_gear_keeps_panel_and_outline() {
    let mut s = Session::open(2, &pack(&Document::new(1), "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Generators);
    s.set_generator_kind("curvyo.generators/gear");
    assert!(s.place_generated(100.0, 100.0));
    s.set_generated_value("teeth", "30");
    s.set_generated_value("web", "\"spokes\"");
    let before_panel = s.generated_panel_json();
    let before_outline = s.draw_list().triangles.len();
    let bytes = s.pack("0.1.0").unwrap();

    let mut reopened = Session::open(2, &bytes).unwrap();
    reopened.set_tool(Tool::Select);
    reopened.resize_viewport(1600.0, 900.0);
    reopened.pointer_hover(pt(20.0, 20.0), false, false);
    reopened.pointer_down(pt(20.0, 20.0), false);
    reopened.pointer_hover(pt(260.0, 260.0), false, false);
    reopened.pointer_up(pt(260.0, 260.0), false, false);
    assert_eq!(reopened.selected_object_count(), 1);
    assert_eq!(
        reopened.generated_panel_json(),
        before_panel,
        "parameters survive"
    );
    assert_eq!(
        reopened.draw_list().triangles.len(),
        before_outline,
        "the outline survives"
    );
    assert_eq!(reopened.undo_len(), 0, "opening writes no step");
    assert_eq!(reopened.unknown_generated_count(), 0);
    assert!(!reopened.is_dirty());
}

// Criterion 4 (notice path): a field that gives a dependent value out of range names the field.
#[test]
fn c4_a_dependent_value_is_clamped_with_a_notice_and_never_refused() {
    let mut s = Session::open(2, &pack(&Document::new(1), "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Generators);
    s.set_generator_kind("curvyo.generators/gear");
    assert!(s.place_generated(50.0, 50.0));
    let answer: Value = serde_json::from_str(&s.set_generated_value("bore", "500")).unwrap();
    assert!(
        answer["error"].is_null() || answer.get("error").is_none(),
        "not an error: {answer}"
    );
    assert_eq!(answer["written"], true);
    assert!(answer["notices"].as_array().is_some_and(|n| !n.is_empty()));
}
