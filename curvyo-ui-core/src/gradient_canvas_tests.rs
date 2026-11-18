//! `specs/0124-gradient-fill` criteria 6 to 9 in `ui-core`: the handles' places and hit rule,
//! the shape handles they cover, the drags, the Gradient block's mixed rows and the interior hit
//! where the gradient has alpha.

use curvyo_document_core::{
    BoxPoint, Color, Document, FillPaint, GradientFill, GradientGeometry, GradientKind,
    GradientStop, GradientStops, Length, NodeId, ObjectSnapshot, Point, RectBounds, StopOffset,
    StyleEdit, Tolerance,
};

use crate::gradient_drag::GradientDragStart;
use crate::gradient_handles::{GradientHandle, gradient_layout};
use crate::gradient_panel::gradient_panel;
use crate::object_selection::ObjectSelection;
use crate::select_tool::{SelectTool, TransformHandleTolerances};
use crate::transform_handle_layout::EditHandle;
use crate::{ResizeDirection, hit_test_object};

/// 2 px per mm: a 100 mm box is 200 px on screen.
const SCALE: f64 = 2.0;

fn stop(r: u8, alpha: u8, offset: f64) -> GradientStop {
    GradientStop {
        colour: Color { r, g: 0, b: 0 },
        alpha,
        offset: StopOffset::new(offset).unwrap(),
    }
}

fn shaded_rect(document: &Document, stops: Vec<GradientStop>) -> NodeId {
    let id = document.create_rect(RectBounds {
        origin: Point::new(0.0, 0.0),
        width: Length::from_mm(100.0),
        height: Length::from_mm(40.0),
    });
    document
        .edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Gradient))
        .unwrap();
    document
        .edit_style(
            &[id],
            &StyleEdit::GradientStops(GradientStops::new(stops).unwrap()),
        )
        .unwrap();
    id
}

fn three_stops() -> Vec<GradientStop> {
    vec![stop(0, 255, 0.0), stop(100, 255, 0.25), stop(255, 255, 1.0)]
}

fn gradient_of(object: &ObjectSnapshot) -> GradientFill {
    object.style().fill.gradient.clone().unwrap()
}

fn near(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
}

/// The defaults sit on the box's left and right edge middles; the inner stop on the line.
#[test]
fn the_handles_sit_on_the_line_in_the_document() {
    let document = Document::new(1);
    let id = shaded_rect(&document, three_stops());
    let object = document.object(id).unwrap();
    let layout = gradient_layout(&gradient_of(&object), &object.gradient_box().unwrap());
    let at = |h: GradientHandle| layout.handles.iter().find(|(x, _)| *x == h).unwrap().1;
    assert!(near(at(GradientHandle::Start), Point::new(0.0, 20.0)));
    assert!(near(at(GradientHandle::End), Point::new(100.0, 20.0)));
    assert!(near(at(GradientHandle::Stop(1)), Point::new(25.0, 20.0)));
    assert_eq!(layout.handles.len(), 3);
    assert!(layout.shown_at(SCALE));
    assert!(!layout.shown_at(0.2), "20 px is under the 24 px rule");
}

/// Ends win over shape handles and over inner dots; the line takes a double-click between.
#[test]
fn hit_priority_and_the_line() {
    let document = Document::new(1);
    let id = shaded_rect(&document, three_stops());
    let object = document.object(id).unwrap();
    let layout = gradient_layout(&gradient_of(&object), &object.gradient_box().unwrap());
    assert_eq!(
        layout.hit(Point::new(5.0, 20.0), SCALE),
        Some(GradientHandle::Start),
        "10 px from the Start"
    );
    assert_eq!(layout.hit(Point::new(7.0, 20.0), SCALE), None, "14 px");
    assert_eq!(
        layout.hit(Point::new(25.0, 24.0), SCALE),
        Some(GradientHandle::Stop(1))
    );
    let t = layout
        .line_offset_at(Point::new(60.0, 21.5), SCALE)
        .unwrap();
    assert!((t - 0.6).abs() < 1e-9);
    assert_eq!(
        layout.line_offset_at(Point::new(60.0, 23.0), SCALE),
        None,
        "6 px off"
    );
    assert_eq!(
        layout.line_offset_at(Point::new(25.0, 20.0), SCALE),
        None,
        "on a dot"
    );
}

/// The W and E resize handles under the default ends yield; the others stay and still hit.
#[test]
fn covered_shape_handles_yield() {
    let document = Document::new(1);
    let id = shaded_rect(&document, three_stops());
    let objects = vec![document.object(id).unwrap()];
    let mut selection = ObjectSelection::default();
    selection.set(&[id]);
    let tolerances = TransformHandleTolerances::at_scale(SCALE);
    let handles = SelectTool::transform_handles(&objects, &selection, tolerances, false);
    let layout = gradient_layout(
        &gradient_of(&objects[0]),
        &objects[0].gradient_box().unwrap(),
    );
    let yielded = layout.yielded(&handles, SCALE);
    assert!(yielded.contains(EditHandle::Resize(ResizeDirection::W)));
    assert!(yielded.contains(EditHandle::Resize(ResizeDirection::E)));
    assert!(!yielded.contains(EditHandle::Resize(ResizeDirection::Ne)));
    let with_yield = TransformHandleTolerances {
        yielded,
        ..tolerances
    };
    let left = Point::new(0.0, 20.0);
    assert!(
        SelectTool::handle_at(&objects, &selection, left, with_yield, false).is_none(),
        "the covered W handle is neither drawn nor hit"
    );
    assert!(SelectTool::handle_at(&objects, &selection, left, tolerances, false).is_some());
    let drawn = SelectTool::transform_handles(&objects, &selection, with_yield, false);
    assert!(drawn.iter().all(|(h, _)| !yielded.contains(*h)));
    assert!(
        SelectTool::handle_at(
            &objects,
            &selection,
            Point::new(100.0, 0.0),
            with_yield,
            false
        )
        .is_some()
    );
}

/// An end drag moves that end; Shift locks the angle about the other end; Ctrl moves the whole
/// line; an inner stop slides along the line.
#[test]
fn drags_of_the_handles() {
    let document = Document::new(1);
    let id = shaded_rect(&document, three_stops());
    let object = document.object(id).unwrap();
    let frame = object.gradient_box().unwrap();
    let gradient = gradient_of(&object);
    let start = |handle, press| GradientDragStart {
        handle,
        geometry: gradient.geometry,
        layout: gradient_layout(&gradient, &frame),
        press,
    };
    let end = start(GradientHandle::End, Point::new(100.0, 20.0));
    let GradientGeometry::Linear { start: s, end: e } = end
        .dragged_geometry(&frame, Point::new(90.0, 30.0), false, false)
        .unwrap()
    else {
        panic!("linear")
    };
    assert_eq!(s, BoxPoint::new(0.0, 0.5).unwrap());
    assert!((e.x.get() - 0.9).abs() < 1e-9 && (e.y.get() - 0.75).abs() < 1e-9);
    // Shift: 100 mm right and 3 mm down snaps to 0°, length kept.
    let GradientGeometry::Linear { end: e, .. } = end
        .dragged_geometry(&frame, Point::new(100.0, 23.0), true, false)
        .unwrap()
    else {
        panic!("linear")
    };
    assert!((e.y.get() - 0.5).abs() < 1e-9, "{e:?}");
    // Ctrl: both ends move.
    let GradientGeometry::Linear { start: s, end: e } = end
        .dragged_geometry(&frame, Point::new(110.0, 20.0), false, true)
        .unwrap()
    else {
        panic!("linear")
    };
    assert!((s.x.get() - 0.1).abs() < 1e-9 && (e.x.get() - 1.1).abs() < 1e-9);
    let dot = start(GradientHandle::Stop(1), Point::new(25.0, 20.0));
    let offset = dot.dragged_offset(Point::new(40.0, 35.0)).unwrap();
    assert!((offset - 0.4).abs() < 1e-9);
    assert!(
        dot.dragged_geometry(&frame, Point::new(0.0, 0.0), false, false)
            .is_none()
    );
}

/// The Radial centre moves the whole circle without a modifier.
#[test]
fn the_radial_centre_moves_the_circle() {
    let document = Document::new(1);
    let id = shaded_rect(&document, three_stops());
    document
        .edit_style(&[id], &StyleEdit::GradientKind(GradientKind::Radial))
        .unwrap();
    let object = document.object(id).unwrap();
    let frame = object.gradient_box().unwrap();
    let gradient = gradient_of(&object);
    let drag = GradientDragStart {
        handle: GradientHandle::Centre,
        geometry: gradient.geometry,
        layout: gradient_layout(&gradient, &frame),
        press: Point::new(50.0, 20.0),
    };
    let GradientGeometry::Radial { centre, radius, .. } = drag
        .dragged_geometry(&frame, Point::new(60.0, 20.0), false, false)
        .unwrap()
    else {
        panic!("radial")
    };
    assert!((centre.x.get() - 0.6).abs() < 1e-9 && (radius.x.get() - 1.1).abs() < 1e-9);
}

/// Criterion 8: equal kinds and counts give rows with "Mixed" where values differ; other counts
/// give the "Mixed gradients" line.
#[test]
fn the_block_shows_mixed_rows_or_mixed_gradients() {
    let a = GradientFill {
        geometry: GradientGeometry::DEFAULT_LINEAR,
        stops: GradientStops::new(three_stops()).unwrap(),
    };
    let mut b = a.clone();
    b.stops = GradientStops::new(vec![
        stop(0, 255, 0.0),
        stop(7, 255, 0.25),
        stop(255, 255, 1.0),
    ])
    .unwrap();
    let panel = gradient_panel(&[&a, &b]).unwrap();
    let rows = panel.rows.unwrap();
    assert!(rows[0].colour.is_some() && rows[1].colour.is_none());
    assert!(rows[1].offset.is_some());
    let mut c = a.clone();
    c.stops = GradientStops::new(vec![stop(0, 255, 0.0), stop(255, 255, 1.0)]).unwrap();
    assert!(gradient_panel(&[&a, &c]).unwrap().rows.is_none());
    c.geometry = GradientGeometry::DEFAULT_RADIAL;
    assert_eq!(gradient_panel(&[&a, &c]).unwrap().kind, None);
    assert!(gradient_panel(&[]).is_none());
}

/// Criterion 9: the interior is hit where the gradient has alpha above 0.
#[test]
fn the_interior_is_hit_where_the_shade_has_alpha() {
    let document = Document::new(1);
    let id = shaded_rect(
        &document,
        vec![
            stop(0, 255, 0.0),
            stop(0, 255, 0.5),
            stop(0, 0, 0.5),
            stop(0, 0, 1.0),
        ],
    );
    let objects = vec![document.object(id).unwrap()];
    let tolerance = Tolerance::from_mm(0.5);
    assert_eq!(
        hit_test_object(&objects, Point::new(20.0, 20.0), tolerance),
        Some(id)
    );
    assert_eq!(
        hit_test_object(&objects, Point::new(80.0, 20.0), tolerance),
        None
    );
}
