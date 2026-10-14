//! Acceptance tests for the round-3 customer feedback (2026-10-16) of
//! `specs/0052-click-to-create-shapes/specification.md`, criteria 29 and 37 and the UX notes,
//! written from the specification before the implementation was read.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use curvyo_document_core::{Point, Shape, unpack};
use curvyo_editor_wasm::{KeyInput, Session, Tool};
use curvyo_ui_core::PlacementField;

const A: Point = Point { x: 100.0, y: 50.0 };

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// Starts a placement with the given keys held at the click, releases them and moves on.
fn start_held(tool: Tool, shift: bool, ctrl: bool) -> Session {
    let mut s = Session::new(1);
    s.set_tool(tool);
    s.pointer_hover(A, shift, ctrl);
    s.pointer_down(A, shift);
    s.pointer_up(A, shift, ctrl);
    assert!(s.placement().is_some());
    s.pointer_hover(p(110.0, 50.0), false, false);
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

fn labels(tool: Tool) -> Vec<&'static str> {
    start_held(tool, false, false)
        .placement_chip()
        .unwrap()
        .checks
        .iter()
        .map(|c| c.label)
        .collect()
}

#[test]
fn ac37_ctrl_held_at_the_click_latches_the_square_after_release() {
    let s = start_held(Tool::Rectangle, false, true);
    assert_eq!(on(&s, false), Some(true));
    assert_eq!(on(&s, true), Some(false));
    assert_eq!(
        s.placement_field_text(PlacementField::Width).unwrap(),
        "30.0"
    );
    assert_eq!(
        s.placement_field_text(PlacementField::Height).unwrap(),
        "30.0"
    );
}

#[test]
fn ac37_shift_held_at_the_click_latches_from_centre_after_release() {
    let plain = start_held(Tool::Rectangle, false, false);
    let s = start_held(Tool::Rectangle, true, false);
    assert_eq!(on(&s, true), Some(true));
    assert_ne!(fields(&s), fields(&plain));
    assert_eq!(
        s.placement_field_text(PlacementField::Width).unwrap(),
        "60.0"
    );
    assert_eq!(
        s.placement_field_text(PlacementField::Height).unwrap(),
        "20.0"
    );
}

#[test]
fn ac37_latch_works_for_ellipse_and_polygon_ctrl() {
    for tool in [Tool::Ellipse, Tool::PolygonStar] {
        let plain = start_held(tool, false, false);
        let s = start_held(tool, false, true);
        assert_eq!(on(&s, false), Some(true), "{tool:?}");
        assert_ne!(fields(&s), fields(&plain), "{tool:?}");
    }
}

#[test]
fn ac37_a_tap_switches_the_latch_off_and_the_released_key_is_not_counted_again() {
    let mut plain = start_held(Tool::Rectangle, false, false);
    let mut s = start_held(Tool::Rectangle, false, true);
    assert!(s.placement_key_down("Control", false, true, false));
    assert_eq!(on(&s, false), Some(false));
    assert_eq!(fields(&s), fields(&plain));
    // And on again with a second tap.
    assert!(s.placement_key_down("Control", false, true, false));
    assert_eq!(on(&s, false), Some(true));
    assert!(plain.placement_toggle_modifier(false));
    assert_eq!(fields(&s), fields(&plain));
}

#[test]
fn ac37_the_check_switches_the_latch_off() {
    let plain = start_held(Tool::Rectangle, false, false);
    let mut s = start_held(Tool::Rectangle, false, true);
    assert!(s.placement_toggle_modifier(false));
    assert_eq!(on(&s, false), Some(false));
    assert_eq!(fields(&s), fields(&plain));
}

#[test]
fn ac37_the_latch_does_not_leak_into_the_next_placement() {
    let mut s = start_held(Tool::Rectangle, false, true);
    s.key_down(KeyInput {
        key: "Escape",
        ..KeyInput::default()
    });
    assert!(s.placement().is_none());
    s.pointer_hover(A, false, false);
    s.pointer_down(A, false);
    s.pointer_up(A, false, false);
    s.pointer_hover(p(130.0, 40.0), false, false);
    assert_eq!(on(&s, false), Some(false));
    assert_eq!(
        s.placement_field_text(PlacementField::Height).unwrap(),
        "10.0"
    );
}

#[test]
fn ac37_chip_labels() {
    assert_eq!(
        labels(Tool::Rectangle),
        ["From centre (Shift)", "Square (Ctrl)"]
    );
    assert_eq!(
        labels(Tool::Ellipse),
        ["From centre (Shift)", "Circle (Ctrl)"]
    );
    assert_eq!(labels(Tool::PolygonStar), ["Snap angle (Ctrl)"]);
}

/// The radius of the one ellipse in `s`, in millimetres.
fn placed_radius(s: &Session) -> f64 {
    let d = unpack(99, &s.pack("0.1.0").unwrap()).unwrap();
    match d.primitive(d.object_ids()[0]).unwrap().shape {
        Shape::Ellipse { frame } => frame.rx.as_mm(),
        other => panic!("ellipse expected, got {other:?}"),
    }
}

#[test]
fn ac20_space_still_switches_radius_and_diameter() {
    let mut radius = start_held(Tool::Ellipse, false, false);
    radius.placement_edit(0, "10");
    radius.pointer_down(p(130.0, 40.0), false);
    assert!((placed_radius(&radius) - 10.0).abs() < 1e-6);

    let mut diameter = start_held(Tool::Ellipse, false, false);
    diameter.placement_toggle_diameter();
    diameter.placement_edit(0, "10");
    diameter.pointer_down(p(130.0, 40.0), false);
    assert!((placed_radius(&diameter) - 5.0).abs() < 1e-6);
}
