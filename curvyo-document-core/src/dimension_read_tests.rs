use super::*;
use crate::corner_radii::CornerRadii;
use crate::dimension_model::DimensionPlace;
use crate::path_model::NewAnchor;
use crate::primitive_model::{EllipseFrame, RectBounds};
use crate::ring_curve::Curve;
use crate::style_model::Style;

fn id(n: i32) -> NodeId {
    NodeId::from_parts(1, n)
}

fn aid(n: u64) -> AnchorId {
    AnchorId::new(7, n)
}

fn path(n: i32, points: &[(f64, f64)], closed: bool) -> ObjectSnapshot {
    ObjectSnapshot::Path(PathSnapshot {
        id: id(n),
        closed,
        style: Style::default(),
        anchors: points
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| NewAnchor::corner(aid(i as u64 + 1), Point::new(x, y)))
            .collect(),
        extra_subpaths: Vec::new(),
        rotation: Angle::from_radians(0.0),
    })
}

fn primitive(n: i32, shape: Shape, rotation: f64) -> ObjectSnapshot {
    ObjectSnapshot::Primitive(PrimitiveSnapshot {
        id: id(n),
        shape,
        style: Style::default(),
        rotation: Angle::from_radians(rotation),
    })
}

fn circle(n: i32, cx: f64, cy: f64, rx: f64, ry: f64) -> ObjectSnapshot {
    primitive(
        n,
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(cx, cy),
                rx: Length::from_mm(rx),
                ry: Length::from_mm(ry),
            },
            arc: None,
            curve: Curve::ELLIPSE_DEFAULT,
        },
        0.0,
    )
}

fn dimension(n: i32, kind: DimensionKind, slots: DimensionSlots) -> ObjectSnapshot {
    ObjectSnapshot::Dimension(Box::new(crate::DimensionSnapshot::new(
        id(n),
        kind,
        slots,
        DimensionPlace::at_offset(Length::from_mm(5.0)),
        Measured {
            points: vec![Point::new(0.0, 0.0), Point::new(1.0, 0.0)],
            value: DimensionValue::Length(Length::from_mm(1.0)),
        },
    )))
}

fn anchor(node: i32, n: u64) -> PointSlot {
    PointSlot::Anchor {
        node: id(node),
        anchor: aid(n),
    }
}

fn shown(objects: &[ObjectSnapshot], n: i32) -> (bool, f64) {
    objects
        .iter()
        .find_map(|object| match object {
            ObjectSnapshot::Dimension(d) if d.id == id(n) => Some((d.linked, d.shown.value.raw())),
            _ => None,
        })
        .unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn distances_measure_along_their_kind() {
    let a = Point::new(0.0, 0.0);
    let b = Point::new(30.0, 40.0);
    let raw = |kind| measure(kind, &[a, b], 0).unwrap().raw();
    assert!(close(raw(DimensionKind::Aligned), 50.0));
    assert!(close(raw(DimensionKind::Horizontal), 30.0));
    assert!(close(raw(DimensionKind::Vertical), 40.0));
}

#[test]
fn the_sector_picks_one_of_the_four_angles() {
    let v = Point::new(0.0, 0.0);
    let first = Point::new(10.0, 0.0);
    let second = Point::new(10.0, 10.0);
    let degrees = |sector| {
        measure(DimensionKind::Angle, &[v, first, second], sector)
            .unwrap()
            .raw()
            .to_degrees()
    };
    assert!(close(degrees(0), 45.0));
    assert!(close(degrees(1), 135.0));
    assert!(close(degrees(2), 135.0));
    assert!(close(degrees(3), 45.0));
}

#[test]
fn a_linked_aligned_dimension_follows_the_anchors_of_its_path() {
    let mut objects = vec![
        path(1, &[(0.0, 0.0), (12.5, 0.0)], false),
        dimension(
            2,
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![anchor(1, 1), anchor(1, 2)]),
        ),
    ];
    resolve_dimensions_in(&mut objects);
    assert_eq!(shown(&objects, 2), (true, 12.5));
}

#[test]
fn a_dimension_whose_node_is_gone_is_unlinked_and_shows_its_kept_value() {
    let mut objects = vec![dimension(
        2,
        DimensionKind::Aligned,
        DimensionSlots::Points(vec![anchor(1, 1), anchor(1, 2)]),
    )];
    resolve_dimensions_in(&mut objects);
    assert_eq!(shown(&objects, 2), (false, 1.0));
}

#[test]
fn a_missing_anchor_unlinks_too() {
    let mut objects = vec![
        path(1, &[(0.0, 0.0), (12.5, 0.0)], false),
        dimension(
            2,
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![anchor(1, 1), anchor(1, 9)]),
        ),
    ];
    resolve_dimensions_in(&mut objects);
    assert!(!shown(&objects, 2).0);
}

#[test]
fn a_segment_midpoint_and_a_free_point_resolve() {
    let mut objects = vec![
        path(1, &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], true),
        dimension(
            2,
            DimensionKind::Vertical,
            DimensionSlots::Points(vec![
                PointSlot::SegmentMid {
                    node: id(1),
                    anchor: aid(3),
                },
                PointSlot::Free {
                    point: Point::new(0.0, 20.0),
                },
            ]),
        ),
    ];
    resolve_dimensions_in(&mut objects);
    // The closing segment runs from (10, 10) to (0, 0): its middle is at y = 5.
    assert_eq!(shown(&objects, 2), (true, 15.0));
}

#[test]
fn shape_points_follow_the_rotation_of_the_shape() {
    let rect = primitive(
        1,
        Shape::Rect {
            bounds: RectBounds {
                origin: Point::new(0.0, 0.0),
                width: Length::from_mm(20.0),
                height: Length::from_mm(10.0),
            },
            corner_radii: CornerRadii::uniform(Length::from_mm(0.0)),
        },
        std::f64::consts::FRAC_PI_2,
    );
    let ObjectSnapshot::Primitive(p) = &rect else {
        unreachable!()
    };
    let center = shape_point(p, ShapeFeature::Center).unwrap();
    assert!(close(center.x, 10.0) && close(center.y, 5.0));
    let top_left = shape_point(p, ShapeFeature::Corner(0)).unwrap();
    // A quarter turn about (10, 5) takes (0, 0) to (15, -5).
    assert!(
        close(top_left.x, 15.0) && close(top_left.y, -5.0),
        "{top_left:?}"
    );
    let top_mid = shape_point(p, ShapeFeature::EdgeMid(0)).unwrap();
    assert!(
        close(top_mid.x, 15.0) && close(top_mid.y, 5.0),
        "{top_mid:?}"
    );
    assert!(shape_point(p, ShapeFeature::Vertex(0)).is_none());
}

#[test]
fn a_circle_measures_its_radius_until_it_is_scaled_unevenly() {
    let slots = DimensionSlots::Curve(CurveSlot::Circle { node: id(1) });
    let mut objects = vec![
        circle(1, 5.0, 5.0, 25.0, 25.0),
        dimension(2, DimensionKind::Diameter, slots.clone()),
    ];
    resolve_dimensions_in(&mut objects);
    assert_eq!(shown(&objects, 2), (true, 50.0));
    let mut objects = vec![
        circle(1, 5.0, 5.0, 25.0, 30.0),
        dimension(2, DimensionKind::Radius, slots),
    ];
    resolve_dimensions_in(&mut objects);
    assert!(!shown(&objects, 2).0, "a non-uniform scale unlinks");
}

#[test]
fn a_circular_path_segment_is_an_arc_and_a_straight_one_is_not() {
    // A quarter circle of radius 10 about the origin, as one cubic.
    let k = 0.552_284_749_8 * 10.0;
    let a = NewAnchor {
        id: aid(1),
        point: Point::new(10.0, 0.0),
        handle_in: Vec2::ZERO,
        handle_out: Vec2::new(0.0, k),
        kind: crate::AnchorKind::Corner,
    };
    let b = NewAnchor {
        id: aid(2),
        point: Point::new(0.0, 10.0),
        handle_in: Vec2::new(k, 0.0),
        handle_out: Vec2::ZERO,
        kind: crate::AnchorKind::Corner,
    };
    let (center, radius) = arc_circle(a, b).unwrap();
    assert!(center.x.abs() < 1e-6 && center.y.abs() < 1e-6);
    assert!((radius.as_mm() - 10.0).abs() < 1e-6);
    let straight = NewAnchor::corner(aid(3), Point::new(20.0, 0.0));
    assert!(arc_circle(NewAnchor::corner(aid(1), Point::new(0.0, 0.0)), straight).is_none());
}

#[test]
fn a_rounded_corner_is_a_circle_inside_its_corner() {
    let rect = primitive(
        1,
        Shape::Rect {
            bounds: RectBounds {
                origin: Point::new(0.0, 0.0),
                width: Length::from_mm(40.0),
                height: Length::from_mm(20.0),
            },
            corner_radii: CornerRadii::uniform(Length::from_mm(4.0)),
        },
        0.0,
    );
    let mut objects = vec![
        rect,
        dimension(
            2,
            DimensionKind::Radius,
            DimensionSlots::Curve(CurveSlot::RoundCorner {
                node: id(1),
                corner: 2,
            }),
        ),
    ];
    resolve_dimensions_in(&mut objects);
    assert_eq!(shown(&objects, 2), (true, 4.0));
    let ObjectSnapshot::Dimension(d) = &objects[1] else {
        unreachable!()
    };
    assert!(close(d.shown.points[0].x, 36.0) && close(d.shown.points[0].y, 16.0));
}

#[test]
fn an_angle_between_two_segments_sharing_a_node() {
    let mut objects = vec![
        path(1, &[(10.0, 0.0), (0.0, 0.0), (0.0, 10.0)], false),
        dimension(
            2,
            DimensionKind::Angle,
            DimensionSlots::Points(vec![anchor(1, 2), anchor(1, 1), anchor(1, 3)]),
        ),
    ];
    resolve_dimensions_in(&mut objects);
    let (linked, value) = shown(&objects, 2);
    assert!(linked && close(value.to_degrees(), 90.0));
}
