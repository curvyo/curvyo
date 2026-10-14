//! Acceptance tests for criterion 37 of `specs/0052-click-to-create-shapes/specification.md`
//! (Shift and Ctrl tap toggles), written from the specification before the implementation
//! was read. Everything goes through `Session`'s public API.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]

use curvyo_document_core::{Point, Shape, unpack};
use curvyo_editor_wasm::{Session, Tool};

const A: Point = Point { x: 100.0, y: 50.0 };

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn placing(tool: Tool) -> Session {
    let mut s = Session::new(1);
    s.set_tool(tool);
    s.pointer_hover(A, false, false);
    s.pointer_down(A, false);
    s.pointer_up(A, false, false);
    assert!(s.placement().is_some());
    s.pointer_hover(p(130.0, 40.0), false, false);
    s
}

fn fields(s: &Session) -> Vec<String> {
    s.placement_chip()
        .unwrap()
        .fields
        .iter()
        .map(|f| f.text.clone())
        .collect()
}

fn on(s: &Session, shift: bool) -> Option<bool> {
    s.placement_chip()
        .unwrap()
        .checks
        .iter()
        .find(|c| c.shift == shift)
        .map(|c| c.on)
}

fn key(s: &mut Session, k: &str, shift: bool, ctrl: bool, repeat: bool) -> bool {
    s.placement_key_down(k, shift, ctrl, repeat)
}

#[test]
fn ac37_a_tap_updates_preview_and_chip_without_a_pointer_move() {
    let mut s = placing(Tool::Rectangle);
    let plain = fields(&s);
    assert_eq!(on(&s, true), Some(false));
    assert_eq!(on(&s, false), Some(false));
    assert!(key(&mut s, "Control", false, true, false));
    let squared = fields(&s);
    assert_ne!(squared, plain);
    assert_eq!(squared[0], squared[1], "square");
    assert_eq!(on(&s, false), Some(true));
    assert!(key(&mut s, "Shift", true, true, false));
    assert_ne!(fields(&s), squared, "from centre changes the box");
    assert_eq!(on(&s, true), Some(true));
    // Second taps undo.
    assert!(key(&mut s, "Shift", true, true, false));
    assert!(key(&mut s, "Control", true, true, false));
    assert_eq!(fields(&s), plain);
}

#[test]
fn ac37_auto_repeat_is_ignored() {
    let mut s = placing(Tool::Rectangle);
    assert!(key(&mut s, "Shift", true, false, false));
    let after = fields(&s);
    assert!(!key(&mut s, "Shift", true, false, true));
    assert!(!key(&mut s, "Shift", true, false, true));
    assert_eq!(fields(&s), after);
    assert_eq!(on(&s, true), Some(true));
}

#[test]
fn ac37_shift_plus_letter_does_not_toggle() {
    let mut s = placing(Tool::Rectangle);
    let plain = fields(&s);
    key(&mut s, "Shift", true, false, false);
    key(&mut s, "A", true, false, false);
    assert_eq!(on(&s, true), Some(false), "chord takes the tap back");
    assert_eq!(fields(&s), plain);
    // Ctrl+letter too.
    key(&mut s, "Control", false, true, false);
    key(&mut s, "z", false, true, false);
    assert_eq!(on(&s, false), Some(false));
    assert_eq!(fields(&s), plain);
}

#[test]
fn ac37_a_later_key_without_the_modifier_keeps_the_toggle() {
    let mut s = placing(Tool::Rectangle);
    key(&mut s, "Shift", true, false, false);
    key(&mut s, "a", false, false, false);
    assert_eq!(on(&s, true), Some(true));
}

#[test]
fn ac37_no_toggle_once_a_field_is_edited() {
    let mut s = placing(Tool::Rectangle);
    s.placement_edit(0, "20");
    let before = fields(&s);
    assert!(!key(&mut s, "Shift", true, false, false));
    assert!(!key(&mut s, "Control", false, true, false));
    assert!(!s.placement_toggle_modifier(true));
    assert_eq!(fields(&s), before);
    assert!(
        s.placement_chip().unwrap().checks.is_empty(),
        "checks gone after the freeze"
    );
}

#[test]
fn ac37_checks_flip_the_same_state_as_the_keys() {
    let mut s = placing(Tool::Ellipse);
    let plain = fields(&s);
    assert!(s.placement_toggle_modifier(false));
    let by_click = fields(&s);
    assert_ne!(by_click, plain);
    // The key tap now flips it back: same state.
    assert!(key(&mut s, "Control", false, true, false));
    assert_eq!(fields(&s), plain);
    assert_eq!(on(&s, false), Some(false));
    let labels: Vec<_> = placing(Tool::Ellipse)
        .placement_chip()
        .unwrap()
        .checks
        .iter()
        .map(|c| c.label)
        .collect();
    assert_eq!(labels, ["From centre (Shift)", "Circle (Ctrl)"]);
    let labels: Vec<_> = placing(Tool::PolygonStar)
        .placement_chip()
        .unwrap()
        .checks
        .iter()
        .map(|c| c.label)
        .collect();
    assert_eq!(labels, ["Snap angle (Ctrl)"]);
}

#[test]
fn ac37_shift_does_nothing_for_the_polygon() {
    let mut s = placing(Tool::PolygonStar);
    let plain = fields(&s);
    assert!(!key(&mut s, "Shift", true, false, false));
    assert_eq!(fields(&s), plain);
    assert_eq!(on(&s, true), None);
}

#[test]
fn ac37_a_new_placement_starts_with_toggles_off() {
    let mut s = placing(Tool::Rectangle);
    key(&mut s, "Control", false, true, false);
    s.cancel_placement();
    s.pointer_hover(A, false, false);
    s.pointer_down(A, false);
    s.pointer_up(A, false, false);
    s.pointer_hover(p(130.0, 40.0), false, false);
    assert_eq!(on(&s, false), Some(false));
}

#[test]
fn ac37_held_key_and_toggle_are_exclusive_or() {
    let mut s = placing(Tool::Rectangle);
    let plain = fields(&s);
    s.pointer_hover(p(130.0, 40.0), false, true); // Ctrl held
    let held = fields(&s);
    assert_eq!(held[0], held[1]);
    assert_eq!(on(&s, false), Some(true));
    key(&mut s, "Control", false, true, false); // toggle while held: off
    assert_eq!(fields(&s), plain, "held and toggled together is off");
    assert_eq!(on(&s, false), Some(false));
    s.pointer_hover(p(130.0, 40.0), false, false); // released: toggle alone is on
    assert_eq!(fields(&s), held);
    assert_eq!(on(&s, false), Some(true));
}

#[test]
fn ac37_a_plain_create_drag_has_no_toggles() {
    let mut s = Session::new(1);
    s.set_tool(Tool::Rectangle);
    s.pointer_hover(A, false, false);
    s.pointer_down(A, false);
    s.pointer_hover(p(140.0, 70.0), false, false);
    assert!(!key(&mut s, "Control", false, true, false));
    s.pointer_up(p(140.0, 70.0), false, false);
    assert!(s.placement().is_none());
    let d = unpack(99, &s.pack("0.1.0").unwrap()).unwrap();
    let snap = d.primitive(d.object_ids()[0]).unwrap();
    match snap.shape {
        Shape::Rect { bounds, .. } => {
            assert!((bounds.width.as_mm() - 40.0).abs() < 1e-6);
            assert!((bounds.height.as_mm() - 20.0).abs() < 1e-6, "not squared");
        }
        other => panic!("{other:?}"),
    }
}
