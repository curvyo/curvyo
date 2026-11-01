//! Customer feedback on PR #243: the edges, vertices, centres and axis ends of primitives as
//! Measure picks.
use super::*;
use curvyo_document_core::{
    CornerRadii, Curve, EllipseFrame, InnerRatio, PointCount, RectBounds, StarFrame, Style, Vec2,
};

fn node(n: u64) -> NodeId {
    let document = curvyo_document_core::Document::new(n);
    document.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(1.0, 1.0),
    ))
}

fn primitive(id: NodeId, shape: Shape) -> ObjectSnapshot {
    ObjectSnapshot::Primitive(PrimitiveSnapshot {
        id,
        shape,
        style: Style::default(),
        rotation: Angle::from_radians(0.0),
    })
}

fn rect(radius: f64) -> ObjectSnapshot {
    primitive(
        node(1),
        Shape::Rect {
            bounds: RectBounds {
                origin: Point::new(0.0, 0.0),
                width: Length::from_mm(40.0),
                height: Length::from_mm(20.0),
            },
            corner_radii: CornerRadii::uniform(Length::from_mm(radius)),
        },
    )
}

fn hexagon() -> ObjectSnapshot {
    primitive(
        node(2),
        Shape::Polygon {
            frame: StarFrame {
                center: Point::new(100.0, 100.0),
                radius: Length::from_mm(20.0),
                angle: Angle::from_radians(0.0),
            },
            point_count: PointCount::new(6).unwrap(),
            curve: Curve::POLYGON_DEFAULT,
        },
    )
}

fn star() -> ObjectSnapshot {
    primitive(
        node(3),
        Shape::Star {
            frame: StarFrame {
                center: Point::new(100.0, 100.0),
                radius: Length::from_mm(20.0),
                angle: Angle::from_radians(0.0),
            },
            point_count: PointCount::new(5).unwrap(),
            inner_ratio: InnerRatio::new(0.5).unwrap(),
            curve: Curve::POLYGON_DEFAULT,
        },
    )
}

fn ellipse() -> ObjectSnapshot {
    primitive(
        node(4),
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(50.0, 50.0),
                rx: Length::from_mm(30.0),
                ry: Length::from_mm(15.0),
            },
            arc: None,
            curve: Curve::ELLIPSE_DEFAULT,
        },
    )
}

const TOL: (Length, Length) = (Length::from_mm(2.0), Length::from_mm(2.0));

fn pick(object: ObjectSnapshot, at: Point) -> MeasurePick {
    measure_pick(&[object], at, TOL)
}

fn feature(pick: (PointSlot, Point)) -> Option<ShapeFeature> {
    match pick.0 {
        PointSlot::ShapePoint { feature, .. } => Some(feature),
        _ => None,
    }
}

fn length(start: Point, end: Point) -> f64 {
    start.vector_to(end).length()
}

#[test]
fn a_rectangle_edge_is_a_segment_between_its_corners() {
    let MeasurePick::Segment { start, end } = pick(rect(0.0), Point::new(8.0, 0.5)) else {
        panic!("the top edge is a segment");
    };
    assert_eq!(feature(start), Some(ShapeFeature::Corner(0)));
    assert_eq!(feature(end), Some(ShapeFeature::Corner(1)));
    assert!((length(start.1, end.1) - 40.0).abs() < 1e-9);
    let MeasurePick::Segment { start, end } = pick(rect(0.0), Point::new(40.4, 14.0)) else {
        panic!("the right edge is a segment");
    };
    assert!((length(start.1, end.1) - 20.0).abs() < 1e-9);
}

#[test]
fn a_rectangle_corner_is_a_point_and_a_rounded_corner_a_radius() {
    assert!(matches!(
        pick(rect(0.0), Point::new(0.5, 0.5)),
        MeasurePick::Point {
            slot: PointSlot::ShapePoint {
                feature: ShapeFeature::Corner(0),
                ..
            },
            ..
        }
    ));
    let on_arc = Point::new(6.0, 6.0).translated(Vec2::new(-1.0, -1.0).normalized_to(6.0));
    assert!(matches!(
        pick(rect(6.0), on_arc),
        MeasurePick::Curve {
            slot: CurveSlot::RoundCorner { corner: 0, .. },
            ..
        }
    ));
    // The straight part of a rounded rectangle's edge still measures the side.
    let MeasurePick::Segment { start, end } = pick(rect(6.0), Point::new(14.0, 0.3)) else {
        panic!("the straight part is a segment");
    };
    assert!((length(start.1, end.1) - 40.0).abs() < 1e-9);
}

#[test]
fn a_polygon_edge_is_a_segment_between_its_vertices_and_its_centre_a_point() {
    let shape = hexagon();
    let ObjectSnapshot::Primitive(p) = &shape else {
        unreachable!()
    };
    let a = shape_point(p, ShapeFeature::Vertex(0)).unwrap();
    let b = shape_point(p, ShapeFeature::Vertex(1)).unwrap();
    // A quarter along the edge: off the midpoint and the vertices.
    let at = a.translated(a.vector_to(b).scaled(0.25));
    let MeasurePick::Segment { start, end } = pick(shape.clone(), at) else {
        panic!("a polygon edge is a segment");
    };
    assert_eq!(feature(start), Some(ShapeFeature::Vertex(0)));
    assert_eq!(feature(end), Some(ShapeFeature::Vertex(1)));
    assert!(
        (length(start.1, end.1) - 20.0).abs() < 1e-9,
        "a hexagon side is its radius"
    );
    assert!(matches!(
        pick(shape, Point::new(100.5, 100.5)),
        MeasurePick::Point {
            slot: PointSlot::ShapePoint {
                feature: ShapeFeature::Center,
                ..
            },
            ..
        }
    ));
}

#[test]
fn a_star_edge_is_a_segment_and_its_vertices_points() {
    let shape = star();
    let ObjectSnapshot::Primitive(p) = &shape else {
        unreachable!()
    };
    let outer = shape_point(p, ShapeFeature::Vertex(0)).unwrap();
    let inner = shape_point(p, ShapeFeature::Vertex(1)).unwrap();
    let at = outer.translated(outer.vector_to(inner).scaled(0.3));
    let MeasurePick::Segment { start, end } = pick(shape.clone(), at) else {
        panic!("a star edge is a segment");
    };
    assert_eq!(feature(start), Some(ShapeFeature::Vertex(0)));
    assert_eq!(feature(end), Some(ShapeFeature::Vertex(1)));
    assert!(matches!(
        pick(shape, inner.translated(Vec2::new(0.4, 0.0))),
        MeasurePick::Point {
            slot: PointSlot::ShapePoint {
                feature: ShapeFeature::Vertex(1),
                ..
            },
            ..
        }
    ));
}

#[test]
fn an_ellipse_offers_its_centre_and_its_four_axis_ends_as_points() {
    let ends = [
        (Point::new(80.0, 50.0), 0),
        (Point::new(50.0, 65.0), 1),
        (Point::new(20.0, 50.0), 2),
        (Point::new(50.0, 35.0), 3),
    ];
    for (end, index) in ends {
        let MeasurePick::Point { slot, at } = pick(ellipse(), end.translated(Vec2::new(0.5, 0.5)))
        else {
            panic!("an axis end is a point");
        };
        assert_eq!(feature((slot, at)), Some(ShapeFeature::Vertex(index)));
        assert!(length(at, end) < 1e-9);
    }
    assert!(matches!(
        pick(ellipse(), Point::new(50.4, 50.4)),
        MeasurePick::Point {
            slot: PointSlot::ShapePoint {
                feature: ShapeFeature::Center,
                ..
            },
            ..
        }
    ));
    // Its rim between the ends has no radius: no curve pick (Proposal).
    let rim = Point::new(50.0 + 30.0 * 0.6_f64.cos(), 50.0 + 15.0 * 0.6_f64.sin());
    assert!(!matches!(pick(ellipse(), rim), MeasurePick::Curve { .. }));
}

#[test]
fn a_circle_keeps_its_radius_pick_without_axis_ends() {
    let circle = primitive(
        node(5),
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(50.0, 50.0),
                rx: Length::from_mm(20.0),
                ry: Length::from_mm(20.0),
            },
            arc: None,
            curve: Curve::ELLIPSE_DEFAULT,
        },
    );
    assert!(matches!(
        pick(circle, Point::new(70.5, 50.0)),
        MeasurePick::Curve {
            slot: CurveSlot::Circle { .. },
            ..
        }
    ));
}
