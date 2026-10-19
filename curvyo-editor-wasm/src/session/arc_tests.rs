//! `specs/0021-ellipse-arcs-and-shaping/` through the session's pointer: an arc-handle
//! drag on a whole circle, a curve-handle drag, the readouts, one step each, and Object to
//! path of an open arc.

use curvyo_document_core::{
    ArcKind, EllipseFrame, Length, NodeId, ObjectSnapshot, Point, PointCount, Shape, StarFrame,
};
use curvyo_ui_core::{EditHandle, ParamHandle, SelectTool};

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

fn shape_of(session: &Session, id: NodeId) -> Shape {
    session.document.primitive(id).expect("a primitive").shape
}

/// Criteria 1, 2, 3, 6 and 59: dragging the end handle of a whole circle makes an open Arc
/// from the seam of 45° to the pointer's angle, the readout names it, one step.
#[test]
fn dragging_an_arc_handle_makes_an_open_arc() {
    let (mut session, id) = circle();
    let at = handle(&session, ParamHandle::ArcEnd);
    session.pointer_down(at, false);
    // Straight up from the centre: 270° (clockwise from the right-hand axis, y down).
    let target = pt(200.0, 150.0);
    session.pointer_hover(target, false, false);
    let readout = session.live_readout().expect("a readout").text;
    assert_eq!(readout, "end 270.0\u{b0} \u{b7} sweep 225.0\u{b0}");
    session.pointer_up(target, false, false);
    let Shape::Ellipse { arc: Some(arc), .. } = shape_of(&session, id) else {
        panic!("an arc");
    };
    assert_eq!(arc.kind, ArcKind::Arc);
    assert!((arc.start().as_radians().to_degrees() - 45.0).abs() < 1e-9);
    assert!((arc.end().as_radians().to_degrees() - 270.0).abs() < 1e-9);
    assert_eq!(
        session.take_transform_announcement(),
        "Arc start 45.0\u{b0}, end 270.0\u{b0}, arc"
    );
    session.undo();
    assert!(matches!(
        shape_of(&session, id),
        Shape::Ellipse { arc: None, .. }
    ));
}

/// Criteria 22, 23 and 25: the curve handle dragged to the centre's side makes the sides
/// concave; the readout shows the value.
#[test]
fn dragging_the_curve_handle_bends_the_sides() {
    let (mut session, id) = circle();
    let at = handle(&session, ParamHandle::Curve);
    session.pointer_down(at, false);
    // The middle of the straight side E to N is at (250, 150): the curve 0 %.
    session.pointer_hover(pt(250.0, 150.0), false, false);
    assert_eq!(
        session.live_readout().expect("a readout").text,
        "curve 0.0 %"
    );
    session.pointer_up(pt(250.0, 150.0), false, false);
    let Shape::Ellipse { curve, .. } = shape_of(&session, id) else {
        panic!("an ellipse");
    };
    assert_eq!(curve.get(), 0.0);
    assert_eq!(session.take_transform_announcement(), "Curve 0.0%");
}

/// Criteria 40 and 41: a polygon has a curve handle, and dragging it outwards bulges it.
#[test]
fn a_polygon_has_a_curve_handle() {
    let mut session = Session::new(1);
    let id = session.document.create_polygon(
        StarFrame {
            center: pt(200.0, 200.0),
            radius: Length::from_mm(100.0),
            angle: curvyo_document_core::Angle::from_radians(0.0),
        },
        PointCount::new(6).expect("six"),
    );
    session.set_tool(Tool::Select);
    session.pointer_down(pt(300.0, 200.0), false);
    session.pointer_up(pt(300.0, 200.0), false, false);
    let at = handle(&session, ParamHandle::Curve);
    let centre = pt(200.0, 200.0);
    let out = centre.vector_to(at);
    let target = at.translated(out.scaled(0.05));
    session.pointer_down(at, false);
    session.pointer_hover(target, false, false);
    session.pointer_up(target, false, false);
    let Shape::Polygon { curve, .. } = shape_of(&session, id) else {
        panic!("a polygon");
    };
    assert!(curve.get() > 0.0, "{}", curve.get());
}

/// Criterion 13: Object to path of an open Arc gives an open path.
#[test]
fn object_to_path_keeps_an_arc_open() {
    let (mut session, id) = circle();
    assert_eq!(
        session.set_selected_arc_angle_text(curvyo_ui_core::ArcEnd::End, "180"),
        curvyo_ui_core::EntryOutcome::Committed
    );
    session.convert_selected_to_paths();
    let Some(ObjectSnapshot::Path(path)) = session.document.object(id) else {
        panic!("a path");
    };
    assert!(!path.closed);
    assert_eq!(path.anchors.len(), 3, "135° is two cubics");
}

fn arc_degrees(session: &Session, id: NodeId) -> (f64, f64, ArcKind) {
    let Shape::Ellipse { arc: Some(arc), .. } = shape_of(session, id) else {
        panic!("an arc");
    };
    (
        arc.start().as_radians().to_degrees(),
        arc.end().as_radians().to_degrees(),
        arc.kind,
    )
}

/// Criteria 11, 54 and 63: a move keeps the arc; a horizontal flip mirrors it
/// (`start' = 180° − end`) and keeps the ellipse an ellipse with an arc.
#[test]
fn a_move_keeps_and_a_flip_mirrors_the_arc() {
    let (mut session, id) = circle();
    session.set_selected_arc_angle_text(curvyo_ui_core::ArcEnd::Start, "20");
    session.set_selected_arc_angle_text(curvyo_ui_core::ArcEnd::End, "130");
    session.set_selected_arc_kind(ArcKind::Pie);
    // A drag of the outline (no handle there) moves the arc.
    let on_arc = pt(
        200.0 + 100.0 * 80_f64.to_radians().cos(),
        200.0 + 100.0 * 80_f64.to_radians().sin(),
    );
    session.pointer_down(on_arc, false);
    session.pointer_hover(
        on_arc.translated(curvyo_document_core::Vec2::new(10.0, 0.0)),
        false,
        false,
    );
    session.pointer_up(
        on_arc.translated(curvyo_document_core::Vec2::new(10.0, 0.0)),
        false,
        false,
    );
    let (start, end, kind) = arc_degrees(&session, id);
    assert!((start - 20.0).abs() < 1e-9 && (end - 130.0).abs() < 1e-9);
    assert_eq!(kind, ArcKind::Pie);
    session.flip_selection(curvyo_ui_core::FlipAxis::Horizontal);
    let (start, end, kind) = arc_degrees(&session, id);
    assert!((start - 50.0).abs() < 1e-6, "{start}");
    assert!((end - 160.0).abs() < 1e-6, "{end}");
    assert_eq!(kind, ArcKind::Pie);
}

/// Criterion 10: a resize by a handle keeps the angles (a half circle stays a half circle).
#[test]
fn a_resize_keeps_the_angles() {
    let (mut session, id) = circle();
    session.set_selected_arc_angle_text(curvyo_ui_core::ArcEnd::Start, "0");
    session.set_selected_arc_angle_text(curvyo_ui_core::ArcEnd::End, "180");
    let east = SelectTool::transform_handles(
        &session.objects(),
        &session.selection,
        session.transform_handle_tolerances(),
        false,
    )
    .into_iter()
    .find(|(handle, _)| *handle == EditHandle::Resize(curvyo_ui_core::ResizeDirection::E))
    .expect("an east handle")
    .1;
    session.pointer_down(east, false);
    session.pointer_hover(
        east.translated(curvyo_document_core::Vec2::new(50.0, 0.0)),
        false,
        false,
    );
    session.pointer_up(
        east.translated(curvyo_document_core::Vec2::new(50.0, 0.0)),
        false,
        false,
    );
    let (start, end, _) = arc_degrees(&session, id);
    assert!(start.abs() < 1e-9 && (end - 180.0).abs() < 1e-9);
    let Shape::Ellipse { frame, .. } = shape_of(&session, id) else {
        panic!("an ellipse");
    };
    assert!(
        (frame.rx.as_mm() - 125.0).abs() < 1e-6,
        "{}",
        frame.rx.as_mm()
    );
}
