//! Tests of the session's click-to-create placement, with the examples of
//! `specs/0052-click-to-create-shapes/` as given, at 100 % zoom.

use curvyo_document_core::{Point, Shape};
use curvyo_ui_core::{PlacementField, Viewport};

use super::super::keys::{EscapeStep, KeyInput, KeyOutcome};
use super::super::{Session, Tool};

const TOL: f64 = 1e-9;

/// A session at 100 % zoom with `tool` active and the pointer at `pointer`.
fn session(tool: Tool) -> Session {
    let mut session = Session::new(1);
    session.viewport = Viewport::new();
    session.set_tool(tool);
    session
}

fn click(session: &mut Session, at: Point) {
    session.pointer_hover(at, false, false);
    session.pointer_down(at, false);
    session.pointer_up(at, false, false);
}

fn key(session: &mut Session, name: &str) -> KeyOutcome {
    session.key_down(KeyInput {
        key: name,
        ..KeyInput::default()
    })
}

fn only_shape(session: &Session) -> (Shape, f64) {
    let ids = session.document.object_ids();
    assert_eq!(ids.len(), 1, "one object");
    let primitive = session.document.primitive(ids[0]).expect("a primitive");
    (primitive.shape, primitive.rotation.as_radians())
}

fn rect_numbers(shape: Shape) -> (f64, f64, f64, f64) {
    let Shape::Rect { bounds, .. } = shape else {
        panic!("a rectangle");
    };
    (
        bounds.origin.x,
        bounds.origin.y,
        bounds.width.as_mm(),
        bounds.height.as_mm(),
    )
}

fn assert_rect(shape: Shape, want: (f64, f64, f64, f64)) {
    let got = rect_numbers(shape);
    assert!(
        (got.0 - want.0).abs() < TOL
            && (got.1 - want.1).abs() < TOL
            && (got.2 - want.2).abs() < TOL
            && (got.3 - want.3).abs() < TOL,
        "{got:?} != {want:?}"
    );
}

const A: Point = Point::new(100.0, 50.0);

/// Criterion 1: a release about 1.9 px from the press starts a placement; 3.8 px is a drag.
#[test]
fn criterion_1_a_click_starts_a_placement_and_a_drag_still_creates() {
    let mut s = session(Tool::Rectangle);
    s.pointer_down(A, false);
    s.pointer_up(Point::new(100.5, 50.0), false, false);
    assert!(s.placement().is_some());
    assert!(s.document.object_ids().is_empty(), "nothing created");

    let mut s = session(Tool::Rectangle);
    s.pointer_down(A, false);
    s.pointer_hover(Point::new(101.0, 50.0), false, false);
    s.pointer_up(Point::new(101.0, 50.0), false, false);
    assert!(s.placement().is_none());
    assert_rect(only_shape(&s).0, (100.0, 50.0, 1.0, 0.0));
}

/// Criteria 2, 4 and 6: the preview follows with no button; a second press 3 px or more from
/// A commits the live shape, one step named as a drag, the Select tool with it selected.
#[test]
fn criteria_2_4_6_click_follow_click_is_one_create_step() {
    let mut s = session(Tool::Rectangle);
    click(&mut s, A);
    let empty = s.draw_list().triangle_count();
    s.pointer_hover(Point::new(130.0, 40.0), false, false);
    assert!(s.draw_list().triangle_count() > empty, "the preview draws");
    assert_eq!(s.live_readout(), None, "the chip replaces the readout");
    assert_eq!(
        s.placement_field_text(PlacementField::Width).as_deref(),
        Some("30.0")
    );
    s.pointer_down(Point::new(130.0, 40.0), false);
    s.pointer_up(Point::new(130.0, 40.0), false, false);
    let (shape, rotation) = only_shape(&s);
    assert_rect(shape, (100.0, 40.0, 30.0, 10.0));
    assert!(rotation.abs() < TOL);
    assert!(s.placement().is_none());
    assert_eq!(s.tool(), Tool::Select);
    assert_eq!(s.selection.ids(), &[s.document.object_ids()[0]]);
    let steps = s.document.own_steps();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].label, "create_rect");
    assert_eq!(
        s.take_placement_announcement().as_deref(),
        Some("Rectangle placed, 30 by 10 millimetres.")
    );
}

/// Criterion 3: the modifiers are read live, also with the pointer still.
#[test]
fn criterion_3_modifiers_change_an_unfrozen_placement_with_the_pointer_still() {
    let mut s = session(Tool::Rectangle);
    click(&mut s, A);
    s.pointer_hover(Point::new(130.0, 40.0), false, false);
    s.modifiers_changed(false, true, false);
    assert_eq!(
        s.placement_field_text(PlacementField::Height).as_deref(),
        Some("30.0")
    );
    s.modifiers_changed(false, false, false);
    assert_eq!(
        s.placement_field_text(PlacementField::Height).as_deref(),
        Some("10.0")
    );
}

/// Criterion 5: a press within 3 px of A is ignored; a double-click creates nothing.
#[test]
fn criterion_5_a_press_near_a_is_ignored() {
    let mut s = session(Tool::Ellipse);
    click(&mut s, A);
    click(&mut s, Point::new(100.3, 50.2));
    assert!(s.placement().is_some());
    assert_eq!(
        s.document.object_ids(),
        [] as [curvyo_document_core::NodeId; 0]
    );
    assert!(!s.double_click(A, false, false));
    assert_eq!(
        s.document.object_ids(),
        [] as [curvyo_document_core::NodeId; 0]
    );
}

/// Criterion 7: Escape cancels, the second Escape leaves the tool; a tool switch cancels; a
/// window blur does not.
#[test]
fn criterion_7_escape_and_tool_switch_cancel_and_blur_does_not() {
    let mut s = session(Tool::PolygonStar);
    click(&mut s, A);
    s.pointer_cancelled();
    s.modifiers_changed(false, false, false);
    assert!(s.placement().is_some(), "a blur does not cancel");
    let _ = s.take_placement_announcement();
    assert_eq!(s.escape(), EscapeStep::CancelledDrag);
    assert!(s.placement().is_none());
    assert_eq!(s.tool(), Tool::PolygonStar);
    assert_eq!(
        s.take_placement_announcement().as_deref(),
        Some("Cancelled.")
    );
    assert_eq!(s.escape(), EscapeStep::LeftTool);

    let mut s = session(Tool::Rectangle);
    click(&mut s, A);
    s.set_tool(Tool::Ellipse);
    assert!(s.placement().is_none());
    assert_eq!(
        s.document.object_ids(),
        [] as [curvyo_document_core::NodeId; 0]
    );
}

/// Criterion 8: the key gate treats a placement as an operation in flight.
#[test]
fn criterion_8_the_key_gate_is_closed_during_a_placement() {
    let mut s = session(Tool::Rectangle);
    click(&mut s, A);
    for name in ["v", "e", "Delete", "Enter"] {
        assert_eq!(key(&mut s, name), KeyOutcome::Ignored, "{name}");
    }
    assert_eq!(s.tool(), Tool::Rectangle);
    assert!(s.placement().is_some());
}

/// Criteria 14, 16 (b), 22: typing freezes; a press anywhere places the typed shape turned
/// about A, in one step.
#[test]
fn criteria_16_22_typed_values_place_at_a_press_anywhere() {
    let mut s = session(Tool::Rectangle);
    click(&mut s, A);
    s.pointer_hover(Point::new(130.0, 40.0), false, false);
    s.placement_edit(0, "40");
    s.placement_edit(1, "20");
    s.placement_edit(2, "90");
    s.pointer_hover(Point::new(0.0, 0.0), true, true);
    click(&mut s, Point::new(300.0, 300.0));
    let (shape, rotation) = only_shape(&s);
    assert_rect(shape, (90.0, 60.0, 40.0, 20.0));
    assert!((rotation - 90_f64.to_radians()).abs() < TOL);
    assert_eq!(s.document.own_steps().len(), 1);
}

/// Criteria 21, 23: Enter with an invalid field writes nothing and names the field.
#[test]
fn criteria_21_23_enter_refuses_an_invalid_field() {
    let mut s = session(Tool::Ellipse);
    click(&mut s, Point::new(50.0, 50.0));
    s.pointer_hover(Point::new(70.0, 80.0), false, false);
    s.placement_edit(0, "abc");
    assert!(!s.place_placement());
    assert_eq!(
        s.placement_chip().and_then(|chip| chip.error),
        Some((0, "Enter a number"))
    );
    assert_eq!(
        s.document.object_ids(),
        [] as [curvyo_document_core::NodeId; 0]
    );
    s.placement_edit(0, "10");
    assert_eq!(s.placement_chip().and_then(|chip| chip.error), None);
    assert!(s.place_placement());
    let Shape::Ellipse { frame, .. } = only_shape(&s).0 else {
        panic!("an ellipse");
    };
    assert!((frame.center.x - 60.0).abs() < TOL && (frame.center.y - 60.0).abs() < TOL);
    assert_eq!(
        s.take_placement_announcement().as_deref(),
        Some("Circle placed, radius 10 millimetres.")
    );
}

/// Criterion 20: Space toggles diameter.
#[test]
fn criterion_20_space_gives_a_diameter() {
    let mut s = session(Tool::Ellipse);
    click(&mut s, Point::new(50.0, 50.0));
    s.pointer_hover(Point::new(70.0, 80.0), false, false);
    s.placement_edit(0, "20");
    s.placement_toggle_diameter();
    assert!(s.place_placement());
    let Shape::Ellipse { frame, .. } = only_shape(&s).0 else {
        panic!("an ellipse");
    };
    assert!((frame.rx.as_mm() - 10.0).abs() < TOL);
}

/// Criterion 25: a bar change reaches the running placement.
#[test]
fn criterion_25_the_bar_reaches_the_placement() {
    let mut s = session(Tool::PolygonStar);
    click(&mut s, Point::new(100.0, 100.0));
    s.placement_edit(0, "25");
    s.set_poly_star_mode(curvyo_ui_core::PolyStarMode::Star);
    let shown = s
        .placement()
        .and_then(curvyo_ui_core::ShapePlacement::shown)
        .expect("a preview");
    assert!(matches!(shown.shape, Shape::Star { .. }));
    assert!(s.place_placement());
    assert_eq!(s.document.own_steps()[0].label, "create_star");
}

/// Criteria 26, 27: Enter on the canvas starts a placement at the viewport centre when the
/// pointer is outside; after placing, focus logic is the host's, the Select tool is active.
#[test]
fn criterion_26_enter_starts_a_keyboard_placement() {
    let mut s = session(Tool::Rectangle);
    s.resize_viewport(800.0, 600.0);
    s.pointer_leave();
    assert_eq!(key(&mut s, "Enter"), KeyOutcome::PlacementStarted);
    let a = s.placement().expect("a placement").a();
    let centre = s.screen_to_document(400.0, 300.0);
    assert!((a.x - centre.x).abs() < TOL && (a.y - centre.y).abs() < TOL);
    assert!(!s.place_placement(), "an untouched size places nothing");
    assert_eq!(
        s.placement_chip().and_then(|chip| chip.error),
        Some((0, "Must be more than 0"))
    );
    s.placement_edit(0, "30");
    s.placement_edit(1, "20");
    assert!(s.place_placement());
    assert_rect(only_shape(&s).0, (centre.x, centre.y, 30.0, 20.0));
    assert_eq!(s.tool(), Tool::Select);
}

/// UX notes 5: a locked active layer starts no placement, and a lock meanwhile ends it.
#[test]
fn a_locked_active_layer_refuses_the_placement() {
    let mut s = session(Tool::Rectangle);
    let layer = s.document.new_layer(None);
    s.document.set_node_locked(&[layer], true).expect("a layer");
    click(&mut s, A);
    assert!(s.placement().is_none());
    assert_ne!(s.take_layer_notice(), "");

    s.document
        .set_node_locked(&[layer], false)
        .expect("a layer");
    click(&mut s, A);
    s.placement_edit(0, "10");
    s.placement_edit(1, "10");
    s.document.set_node_locked(&[layer], true).expect("a layer");
    assert!(!s.place_placement());
    assert!(s.placement().is_none());
    assert!(s.document.object_ids().iter().all(|&id| id == layer));
}

/// Criterion 31: on touch a tap is 8 px.
#[test]
fn criterion_31_a_touch_tap_is_eight_pixels() {
    let mut s = session(Tool::Rectangle);
    s.set_touch_input(true);
    s.pointer_down(A, false);
    s.pointer_up(Point::new(101.5, 50.0), false, false);
    assert!(s.placement().is_some(), "5.7 px is a tap");
}

/// Criterion 35: Ctrl+Z after a placement removes the shape.
#[test]
fn criterion_35_undo_removes_the_placed_shape() {
    let mut s = session(Tool::Rectangle);
    click(&mut s, A);
    s.placement_edit(0, "10");
    s.placement_edit(1, "10");
    assert!(s.place_placement());
    let _ = s.undo();
    assert_eq!(
        s.document.object_ids(),
        [] as [curvyo_document_core::NodeId; 0]
    );
    assert_eq!(s.selection.ids(), []);
}

fn tap(session: &mut Session, name: &str) -> bool {
    session.placement_key_down(name, false, false, false)
}

/// A rectangle placement from A = (100, 50) with the pointer at (130, 40), no key held.
fn following() -> Session {
    let mut s = session(Tool::Rectangle);
    click(&mut s, Point::new(100.0, 50.0));
    s.pointer_hover(Point::new(130.0, 40.0), false, false);
    s
}

fn live_size(s: &Session) -> (String, String) {
    let chip = s.placement_chip().expect("a chip");
    (chip.fields[0].text.clone(), chip.fields[1].text.clone())
}

/// A Ctrl tap squares the preview and a second tap undoes it, with no pointer move.
#[test]
fn a_ctrl_tap_toggles_the_square_without_a_pointer_move() {
    let mut s = following();
    assert_eq!(live_size(&s), ("30.0".into(), "10.0".into()));
    assert!(s.placement_key_down("Control", false, true, false));
    assert_eq!(live_size(&s), ("30.0".into(), "30.0".into()));
    assert!(s.placement_chip().expect("a chip").checks[1].on);
    assert!(s.placement_key_down("Control", false, true, false));
    assert_eq!(live_size(&s), ("30.0".into(), "10.0".into()));
}

/// A Shift tap centres the box on A.
#[test]
fn a_shift_tap_centres_the_preview_on_a() {
    let mut s = following();
    assert!(tap(&mut s, "Shift"));
    assert_eq!(live_size(&s), ("60.0".into(), "20.0".into()));
    assert!(s.placement_chip().expect("a chip").checks[0].on);
}

/// Auto-repeat of the key does not toggle again.
#[test]
fn auto_repeat_does_not_toggle() {
    let mut s = following();
    assert!(tap(&mut s, "Shift"));
    assert!(!s.placement_key_down("Shift", true, false, true));
    assert_eq!(live_size(&s), ("60.0".into(), "20.0".into()));
}

/// Shift and then a letter with Shift still down is a chord: the toggle is taken back.
#[test]
fn shift_plus_a_letter_does_not_toggle() {
    let mut s = following();
    assert!(tap(&mut s, "Shift"));
    assert!(s.placement_key_down("A", true, false, false));
    assert_eq!(live_size(&s), ("30.0".into(), "10.0".into()));
    assert!(!s.placement_chip().expect("a chip").checks[0].on);
}

/// A letter typed after the tap, with Shift released, leaves the toggle.
#[test]
fn a_later_key_without_the_modifier_keeps_the_toggle() {
    let mut s = following();
    assert!(tap(&mut s, "Shift"));
    assert!(!s.placement_key_down("a", false, false, false));
    assert_eq!(live_size(&s), ("60.0".into(), "20.0".into()));
}

/// Once the maker has typed in a field, a tap toggles nothing.
#[test]
fn no_toggle_while_typing() {
    let mut s = following();
    s.placement_edit(0, "40");
    assert!(!tap(&mut s, "Shift"));
    assert!(!s.placement_toggle_modifier(false));
    assert_eq!(live_size(&s), ("40".into(), "10.0".into()));
}

/// The chip's checks flip the same state; a polygon has the Ctrl check only.
#[test]
fn the_checks_toggle_the_same_state() {
    let mut s = following();
    assert!(s.placement_toggle_modifier(false));
    assert_eq!(live_size(&s), ("30.0".into(), "30.0".into()));
    let mut p = session(Tool::PolygonStar);
    click(&mut p, Point::new(100.0, 50.0));
    p.pointer_hover(Point::new(130.0, 40.0), false, false);
    assert_eq!(p.placement_chip().expect("a chip").checks.len(), 1);
    assert!(!p.placement_toggle_modifier(true));
}

/// A held key and a toggle together invert each other (exclusive or).
#[test]
fn a_held_key_and_a_toggle_are_exclusive_or() {
    let mut s = following();
    assert!(tap(&mut s, "Control"));
    s.pointer_hover(Point::new(130.0, 40.0), false, true);
    assert_eq!(live_size(&s), ("30.0".into(), "10.0".into()));
    s.pointer_hover(Point::new(130.0, 40.0), false, false);
    assert_eq!(live_size(&s), ("30.0".into(), "30.0".into()));
}

/// A new placement starts with no toggle.
#[test]
fn a_new_placement_starts_without_toggles() {
    let mut s = following();
    assert!(tap(&mut s, "Control"));
    assert!(s.cancel_placement());
    s.set_tool(Tool::Rectangle);
    click(&mut s, Point::new(100.0, 50.0));
    s.pointer_hover(Point::new(130.0, 40.0), false, false);
    assert_eq!(live_size(&s), ("30.0".into(), "10.0".into()));
}

/// A click with Ctrl held latches the square on: it stays after the key is released, and a tap
/// switches it off.
#[test]
fn a_modifier_held_at_the_click_latches_on() {
    let mut s = session(Tool::Rectangle);
    let a = Point::new(100.0, 50.0);
    s.modifiers_changed(false, true, false);
    s.pointer_hover(a, false, true);
    s.pointer_down(a, false);
    s.pointer_up(a, false, true);
    s.pointer_hover(Point::new(130.0, 40.0), false, true);
    assert_eq!(live_size(&s), ("30.0".into(), "30.0".into()));
    s.modifiers_changed(false, false, false);
    s.pointer_hover(Point::new(130.0, 40.0), false, false);
    assert_eq!(live_size(&s), ("30.0".into(), "30.0".into()));
    assert!(s.placement_chip().expect("a chip").checks[1].on);
    assert!(tap(&mut s, "Control"));
    assert_eq!(live_size(&s), ("30.0".into(), "10.0".into()));
}

/// Shift held at the click latches the centre on.
#[test]
fn shift_held_at_the_click_latches_the_centre() {
    let mut s = session(Tool::Rectangle);
    let a = Point::new(100.0, 50.0);
    s.modifiers_changed(true, false, false);
    s.pointer_hover(a, true, false);
    s.pointer_down(a, true);
    s.pointer_up(a, true, false);
    s.pointer_hover(Point::new(130.0, 40.0), false, false);
    assert_eq!(live_size(&s), ("60.0".into(), "20.0".into()));
    assert!(s.placement_chip().expect("a chip").checks[0].on);
}
