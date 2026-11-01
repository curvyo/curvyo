//! Customer feedback on PR #243, through the session's pointer path: primitives measure by
//! their edges, corners, vertices, centres and axis ends; the selection box's handles win over a
//! dimension; no chip shows before the first pick.
use super::*;
use curvyo_document_core::{
    Curve, DimensionPlace, EllipseFrame, NodeId, PointCount, RectBounds, Shape, ShapeFeature,
    StarFrame,
};

fn session() -> Session {
    let mut session = Session::new(1);
    let mut snapping = session.snapping();
    snapping.enabled = true;
    session.set_snapping(snapping);
    session
}

fn click(session: &mut Session, at: Point, shift: bool) {
    session.modifiers_changed(shift, false, false);
    session.pointer_hover(at, shift, false);
    session.pointer_down(at, shift);
    session.pointer_up(at, shift, false);
    session.modifiers_changed(false, false, false);
}

fn view(session: &Session) -> serde_json::Value {
    let _ = session.draw_list();
    serde_json::from_str(&session.measure_view_json()).unwrap()
}

fn dimensions(session: &Session) -> Vec<curvyo_document_core::DimensionSnapshot> {
    session
        .document
        .read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Dimension(d) => Some((**d).clone()),
            _ => None,
        })
        .collect()
}

fn rect(session: &Session) -> NodeId {
    session.document.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(40.0, 20.0),
    ))
}

fn features(dimension: &curvyo_document_core::DimensionSnapshot) -> Vec<ShapeFeature> {
    let DimensionSlots::Points(slots) = &dimension.slots else {
        panic!("a point dimension");
    };
    slots
        .iter()
        .filter_map(|slot| match slot {
            PointSlot::ShapePoint { feature, .. } => Some(*feature),
            _ => None,
        })
        .collect()
}

#[test]
fn a_click_on_a_rectangle_edge_measures_the_side_and_shift_leaves_its_linked_dimension() {
    let mut session = session();
    rect(&session);
    session.set_tool(Tool::Measure);
    click(&mut session, Point::new(12.0, 0.2), false);
    session.pointer_hover(Point::new(30.0, 30.0), false, false);
    assert_eq!(view(&session)["readout"]["lines"][0], "40 mm, 0°");
    session.measure_escape();
    click(&mut session, Point::new(39.8, 14.0), true);
    click(&mut session, Point::new(60.0, 10.0), false);
    let placed = dimensions(&session);
    assert_eq!(placed.len(), 1);
    assert_eq!(
        features(&placed[0]),
        [ShapeFeature::Corner(1), ShapeFeature::Corner(2)]
    );
    assert!(
        (placed[0].shown.points[0]
            .vector_to(placed[0].shown.points[1])
            .length()
            - 20.0)
            .abs()
            < 1e-9
    );
}

#[test]
fn a_star_edge_and_its_centre_are_picked() {
    let mut session = session();
    let star = session.document.create_primitive(
        Shape::Star {
            frame: StarFrame {
                center: Point::new(100.0, 100.0),
                radius: Length::from_mm(30.0),
                angle: Angle::from_radians(0.0),
            },
            point_count: PointCount::new(5).unwrap(),
            inner_ratio: curvyo_document_core::InnerRatio::new(0.5).unwrap(),
            curve: Curve::POLYGON_DEFAULT,
        },
        Angle::from_radians(0.0),
    );
    let ObjectSnapshot::Primitive(p) = session.document.object(star).unwrap() else {
        panic!("a primitive");
    };
    let a = curvyo_document_core::shape_point(&p, ShapeFeature::Vertex(0)).unwrap();
    let b = curvyo_document_core::shape_point(&p, ShapeFeature::Vertex(1)).unwrap();
    session.set_tool(Tool::Measure);
    click(&mut session, a.translated(a.vector_to(b).scaled(0.3)), true);
    click(&mut session, Point::new(140.0, 60.0), false);
    // Centre to a vertex: two points.
    click(&mut session, Point::new(100.3, 100.3), true);
    click(&mut session, a, true);
    click(&mut session, Point::new(100.0, 60.0), false);
    let placed = dimensions(&session);
    assert_eq!(placed.len(), 2);
    assert_eq!(
        features(&placed[0]),
        [ShapeFeature::Vertex(0), ShapeFeature::Vertex(1)]
    );
    assert_eq!(
        features(&placed[1]),
        [ShapeFeature::Center, ShapeFeature::Vertex(0)]
    );
}

#[test]
fn an_ellipse_measures_between_its_axis_ends_and_follows_a_resize() {
    let mut session = session();
    let ellipse = session.document.create_primitive(
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(50.0, 50.0),
                rx: Length::from_mm(30.0),
                ry: Length::from_mm(15.0),
            },
            arc: None,
            curve: Curve::ELLIPSE_DEFAULT,
        },
        Angle::from_radians(0.0),
    );
    session.set_tool(Tool::Measure);
    click(&mut session, Point::new(80.4, 50.3), false);
    session.pointer_hover(Point::new(20.3, 50.2), false, false);
    click(&mut session, Point::new(20.3, 50.2), false);
    assert_eq!(view(&session)["readout"]["lines"][0], "60 mm, 180°");
    session.measure_escape();
    click(&mut session, Point::new(50.3, 35.2), false);
    session.modifiers_changed(true, false, false);
    click(&mut session, Point::new(50.2, 64.8), true);
    click(&mut session, Point::new(90.0, 50.0), false);
    let placed = dimensions(&session);
    assert_eq!(placed.len(), 1);
    assert_eq!(
        features(&placed[0]),
        [ShapeFeature::Vertex(3), ShapeFeature::Vertex(1)]
    );
    assert_eq!(
        placed[0].shown.value,
        DimensionValue::Length(Length::from_mm(30.0))
    );
    let _ = ellipse;
}

#[test]
fn a_resize_handle_wins_over_a_dimension_on_it() {
    let mut session = session();
    let rect = rect(&session);
    // A dimension whose line and text lie on the rectangle's east handle (40, 10).
    let dimension = session
        .document
        .add_dimension(
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![
                PointSlot::Free {
                    point: Point::new(40.0, -20.0),
                },
                PointSlot::Free {
                    point: Point::new(40.0, 40.0),
                },
            ]),
            DimensionPlace::at_offset(Length::from_mm(0.0)),
        )
        .unwrap();
    session.set_tool(Tool::Select);
    session.selection.select_single(rect);
    let _ = view(&session);
    let handle = Point::new(40.0, 10.0);
    session.pointer_hover(handle, false, false);
    assert!(session.cursor_hint().starts_with("resize:"));
    session.pointer_down(handle, false);
    session.pointer_hover(Point::new(50.0, 10.0), false, false);
    session.pointer_up(Point::new(50.0, 10.0), false, false);
    assert_eq!(
        session.selection.ids(),
        [rect],
        "the rectangle stays selected"
    );
    let ObjectSnapshot::Primitive(p) = session.document.object(rect).unwrap() else {
        panic!("a primitive");
    };
    let Shape::Rect { bounds, .. } = p.shape else {
        panic!("a rectangle");
    };
    assert!(
        (bounds.width.as_mm() - 50.0).abs() < 1e-6,
        "scaled: {bounds:?}"
    );
    let kept = dimensions(&session);
    assert_eq!(kept[0].id, dimension);
    assert_eq!(
        kept[0].place,
        DimensionPlace::at_offset(Length::from_mm(0.0))
    );
}

#[test]
fn no_chip_shows_before_the_first_pick() {
    let mut session = session();
    rect(&session);
    session.set_tool(Tool::Measure);
    session.pointer_hover(Point::new(100.0, 100.0), false, false);
    let shown = view(&session);
    assert!(
        shown["mode"].is_null() && shown["readout"].is_null(),
        "{shown}"
    );
    session.modifiers_changed(true, false, false);
    session.pointer_hover(Point::new(100.0, 100.0), true, false);
    assert!(
        view(&session)["mode"].is_null(),
        "Shift alone shows none either"
    );
    session.modifiers_changed(false, false, false);
    click(&mut session, Point::new(100.0, 100.0), false);
    session.pointer_hover(Point::new(120.0, 100.0), false, false);
    let shown = view(&session);
    assert_eq!(shown["mode"]["lines"][0], "Aligned");
    assert!(shown["readout"].is_object());
}
