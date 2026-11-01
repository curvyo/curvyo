//! Unit tests of the Measure tool's picks.
use super::*;

#[test]
fn an_arc_sweeps_through_its_middle() {
    let c = Point::new(0.0, 0.0);
    let (start, sweep) = arc_through(
        c,
        Point::new(1.0, 0.0),
        Point::new(0.0, 1.0),
        Point::new(-1.0, 0.0),
    );
    assert!(
        start.as_radians().abs() < 1e-12
            && (sweep.as_radians() - std::f64::consts::PI).abs() < 1e-12
    );
    let (_, back) = arc_through(
        c,
        Point::new(1.0, 0.0),
        Point::new(0.0, -1.0),
        Point::new(-1.0, 0.0),
    );
    assert!((back.as_radians() + std::f64::consts::PI).abs() < 1e-12);
}
use curvyo_document_core::{Angle, EllipseFrame, Length, NewAnchor, RectBounds, Style, Vec2};

fn id(n: i32) -> NodeId {
    // Ids come from the document in real use; the tests take one from a fresh path.
    let document = curvyo_document_core::Document::new(u64::try_from(n).unwrap());
    document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(1.0, 0.0)),
        ],
        false,
    )
}

fn line(node: NodeId, a: Point, b: Point) -> ObjectSnapshot {
    ObjectSnapshot::Path(PathSnapshot {
        id: node,
        closed: false,
        style: Style::default(),
        anchors: vec![
            NewAnchor::corner(AnchorId::new(1, 1), a),
            NewAnchor::corner(AnchorId::new(1, 2), b),
        ],
        extra_subpaths: Vec::new(),
        rotation: Angle::from_radians(0.0),
    })
}

fn circle(node: NodeId) -> ObjectSnapshot {
    ObjectSnapshot::Primitive(PrimitiveSnapshot {
        id: node,
        shape: Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(50.0, 50.0),
                rx: Length::from_mm(20.0),
                ry: Length::from_mm(20.0),
            },
            arc: None,
            curve: curvyo_document_core::Curve::ELLIPSE_DEFAULT,
        },
        style: Style::default(),
        rotation: Angle::from_radians(0.0),
    })
}

#[test]
fn a_node_wins_over_the_segment_and_alt_frees_the_point() {
    let path = line(id(1), Point::new(0.0, 0.0), Point::new(30.0, 0.0));
    let objects = [path];
    let near_node = measure_pick(
        &objects,
        Point::new(0.5, 0.5),
        (Length::from_mm(2.0), Length::from_mm(2.0)),
    );
    assert!(matches!(
        near_node,
        MeasurePick::Point {
            slot: PointSlot::Anchor { .. },
            ..
        }
    ));
    let middle = measure_pick(
        &objects,
        Point::new(15.2, 0.3),
        (Length::from_mm(2.0), Length::from_mm(2.0)),
    );
    assert!(matches!(
        middle,
        MeasurePick::Point {
            slot: PointSlot::SegmentMid { .. },
            ..
        }
    ));
    let body = measure_pick(
        &objects,
        Point::new(8.0, 0.5),
        (Length::from_mm(2.0), Length::from_mm(2.0)),
    );
    assert!(matches!(body, MeasurePick::Segment { .. }));
    let free = measure_pick(
        &objects,
        Point::new(0.5, 0.5),
        (Length::from_mm(0.0), Length::from_mm(2.0)),
    );
    assert!(
        matches!(free, MeasurePick::Segment { .. }),
        "Alt: no node snap"
    );
    let empty = measure_pick(
        &objects,
        Point::new(8.0, 9.0),
        (Length::from_mm(2.0), Length::from_mm(2.0)),
    );
    assert!(matches!(
        empty,
        MeasurePick::Point {
            slot: PointSlot::Free { .. },
            ..
        }
    ));
}

#[test]
fn a_circle_outline_is_a_curve_pick_and_its_centre_a_point() {
    let objects = [circle(id(2))];
    let outline = measure_pick(
        &objects,
        Point::new(70.5, 50.0),
        (Length::from_mm(2.0), Length::from_mm(2.0)),
    );
    assert!(
        matches!(outline, MeasurePick::Curve { radius, .. } if (radius.as_mm() - 20.0).abs() < 1e-9)
    );
    let centre = measure_pick(
        &objects,
        Point::new(50.5, 50.0),
        (Length::from_mm(2.0), Length::from_mm(2.0)),
    );
    assert!(matches!(
        centre,
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
fn a_rounded_corner_is_picked_on_its_quarter() {
    let rect = ObjectSnapshot::Primitive(PrimitiveSnapshot {
        id: id(3),
        shape: Shape::Rect {
            bounds: RectBounds {
                origin: Point::new(0.0, 0.0),
                width: Length::from_mm(40.0),
                height: Length::from_mm(20.0),
            },
            corner_radii: curvyo_document_core::CornerRadii::uniform(Length::from_mm(6.0)),
        },
        style: Style::default(),
        rotation: Angle::from_radians(0.0),
    });
    // On the top-left arc, 45° out from its centre (6, 6).
    let on_arc = Point::new(6.0, 6.0).translated(Vec2::new(-1.0, -1.0).normalized_to(6.0));
    let pick = measure_pick(
        &[rect],
        on_arc,
        (Length::from_mm(0.5), Length::from_mm(1.0)),
    );
    assert!(matches!(
        pick,
        MeasurePick::Curve {
            slot: CurveSlot::RoundCorner { corner: 0, .. },
            ..
        }
    ));
}
