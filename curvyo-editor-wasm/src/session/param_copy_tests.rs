//! `specs/0073-parameter-handle-clone` through the session, with the key events in the order
//! the wasm layer sends them (`modifiers_changed` before every pointer event): Shift pressed
//! during a parameter handle drag shows the copy, the Copy badge and " · copy", the release's
//! Shift writes the copy and selects it, and the entry's Copy check does the same.

use curvyo_document_core::{EllipseFrame, Length, NodeId, Point, RectBounds, Shape};
use curvyo_ui_core::{EditHandle, EntryOutcome, ParamHandle, SelectTool};

use super::{Session, Tool};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn handle(session: &Session, wanted: ParamHandle) -> Point {
    SelectTool::transform_handles(
        &session.objects(),
        &session.selection,
        session.transform_handle_tolerances(),
        false,
    )
    .into_iter()
    .find(|(handle, _)| *handle == EditHandle::Param(wanted))
    .unwrap_or_else(|| panic!("no {wanted:?} handle"))
    .1
}

/// The knob of the dragged handle as drawn this frame.
fn drawn_knob(session: &Session) -> Point {
    session
        .select_transform_decoration_input()
        .handles
        .into_iter()
        .find(|glyph| glyph.dragging)
        .expect("the dragged knob")
        .position
}

/// A circle of radius 100 mm at (200, 200), selected by a click on its outline.
fn circle() -> (Session, NodeId) {
    let mut session = Session::new(1);
    let id = session.document.create_ellipse(EllipseFrame {
        center: pt(200.0, 200.0),
        rx: Length::from_mm(100.0),
        ry: Length::from_mm(100.0),
    });
    session.set_tool(Tool::Select);
    session.pointer_down(pt(300.0, 200.0), false);
    session.pointer_up(pt(300.0, 200.0), false, false);
    assert_eq!(session.selection.ids(), [id]);
    (session, id)
}

fn hover(session: &mut Session, at: Point, shift: bool, ctrl: bool) {
    session.modifiers_changed(shift, ctrl, false);
    session.pointer_hover(at, shift, ctrl);
}

fn release(session: &mut Session, at: Point, shift: bool) {
    session.modifiers_changed(shift, false, false);
    session.pointer_up(at, shift, false);
}

fn curve_of(session: &Session, id: NodeId) -> f64 {
    match session.document.primitive(id).expect("a primitive").shape {
        Shape::Ellipse { curve, .. } => curve.get(),
        _ => panic!("an ellipse"),
    }
}

/// Criteria 1, 3 to 5 and 12: Shift pressed mid-drag (pointer at rest) turns the drag into a
/// copy drag in that frame: the original drawn as committed and boxed, the knob at the copy's
/// value, the Copy badge, " · copy"; the release with Shift writes one selected copy in one
/// step "Copy with curve", one undo takes it back and selects the original.
#[test]
fn a_shift_curve_drag_previews_and_writes_a_selected_copy() {
    let (mut session, id) = circle();
    let at = handle(&session, ParamHandle::Curve);
    let box_at_press = format!("{:?}", session.select_decoration_input().selected);
    hover(&mut session, at, false, false);
    session.pointer_down(at, false);
    let target = pt(250.0, 150.0);
    hover(&mut session, target, false, false);
    let plain_knob = drawn_knob(&session);
    assert!(!session.move_indicators().copy_badge);
    assert_eq!(session.live_readout().expect("readout").text, "curve 0.0 %");
    session.modifiers_changed(true, false, false);
    let live = session
        .select_live_edit_in(&session.objects())
        .expect("a live edit");
    assert!(live.copy, "a copy in the keydown frame");
    assert!(session.move_indicators().copy_badge);
    assert_eq!(
        session.live_readout().expect("readout").text,
        "curve 0.0 % \u{b7} copy"
    );
    let copy_knob = drawn_knob(&session);
    assert!(
        (copy_knob.x - plain_knob.x).abs() < 1e-9 && (copy_knob.y - plain_knob.y).abs() < 1e-9,
        "the knob sits at the copy's value, where the plain drag had it"
    );
    let objects = session.objects();
    assert_eq!(
        format!("{:?}", session.select_decoration_input().selected),
        box_at_press,
        "the box stays on the original"
    );
    session.modifiers_changed(false, false, false);
    assert!(
        !session.move_indicators().copy_badge,
        "gone in the keyup frame"
    );
    assert_eq!(session.live_readout().expect("readout").text, "curve 0.0 %");
    session.modifiers_changed(true, false, false);
    release(&mut session, target, true);
    assert_eq!(session.objects().len(), objects.len() + 1);
    let copy = *session.selection.ids().first().expect("selected");
    assert_ne!(copy, id);
    assert_eq!(curve_of(&session, copy), 0.0);
    assert!(curve_of(&session, id) > 0.0, "the original keeps its curve");
    let steps = session.document.own_steps();
    assert_eq!(steps.last().expect("a step").label, "copy_with_curve");
    assert_eq!(
        curvyo_ui_core::operation_name("copy_with_curve"),
        "Copy with curve"
    );
    session.undo();
    assert_eq!(session.document.object_ids(), vec![id]);
    assert_eq!(
        session.selection.ids(),
        [id],
        "the original is selected again"
    );
}

/// Criteria 1 and 6: Shift pressed mid-drag and let go before the release writes the change,
/// no copy; Ctrl alone shows no badge and no "copy".
#[test]
fn a_shift_let_go_before_the_release_writes_a_change() {
    let (mut session, id) = circle();
    let at = handle(&session, ParamHandle::Curve);
    hover(&mut session, at, false, false);
    session.pointer_down(at, false);
    let target = pt(250.0, 150.0);
    hover(&mut session, target, false, true);
    assert!(!session.move_indicators().copy_badge, "Ctrl is no copy key");
    assert!(
        !session
            .live_readout()
            .expect("readout")
            .text
            .contains("copy")
    );
    hover(&mut session, target, true, false);
    hover(&mut session, target, false, false);
    release(&mut session, target, false);
    assert_eq!(session.document.object_ids(), vec![id]);
    assert_eq!(curve_of(&session, id), 0.0);
}

/// Criterion 11: the entry with Copy set writes a limited value to the selected copy, and the
/// limit notice shows the copy's value.
#[test]
fn an_entry_copy_selects_the_copy_and_its_limit_notice_names_it() {
    let mut session = Session::new(1);
    let id = session.document.create_rect(RectBounds {
        origin: pt(10.0, 20.0),
        width: Length::from_mm(100.0),
        height: Length::from_mm(60.0),
    });
    session.set_tool(Tool::Select);
    session.pointer_down(pt(10.0, 50.0), false);
    session.pointer_up(pt(10.0, 50.0), false, false);
    let at = handle(
        &session,
        ParamHandle::CornerRadius(curvyo_ui_core::Corner::Tl),
    );
    session.pointer_down(at, false);
    session.pointer_up(at, false, false);
    let _ = session.double_click_hint(at, false, false);
    assert!(session.transform_entry().is_some(), "the entry is open");
    let outcome = session.commit_transform_entry_fields(&["99"], 0, true);
    assert_eq!(outcome, EntryOutcome::Committed);
    let copy = *session.selection.ids().first().expect("selected");
    assert_ne!(copy, id);
    let radius = |id| match session.document.primitive(id).expect("a primitive").shape {
        Shape::Rect { corner_radii, .. } => corner_radii.tl.as_mm(),
        _ => panic!("a rectangle"),
    };
    assert_eq!(radius(copy), 30.0);
    assert_eq!(radius(id), 0.0);
    assert_eq!(
        session.live_readout().expect("the limit notice").text,
        "r 30.0 mm max"
    );
}
