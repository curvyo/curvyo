//! `curvyo-ui-core` tests of `specs/0056-mirror-and-negative-scale/` Part A and D: the
//! flip of a selection about the centre of its true bounding box, kind by kind, the no-op
//! flip, clones, and the arithmetic of criterion 28 (a to c, f).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]

use curvyo_document_core::{
    AnchorId, AnchorKind, Angle, CornerRadii, Document, EllipseFrame, InnerRatio, Length,
    NewAnchor, NodeId, ObjectSnapshot, Point, PointCount, PrimitiveSnapshot, RectBounds, Shape,
    StarFrame, Vec2, outline_of_rotated,
};
use curvyo_ui_core::{
    FlipAxis, FlipOutcome, ObjectSelection, flip_selection, object_outline_bounds,
};

const H: FlipAxis = FlipAxis::Horizontal;
const V: FlipAxis = FlipAxis::Vertical;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn objects(document: &Document) -> Vec<ObjectSnapshot> {
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.object(id))
        .collect()
}

fn flip(document: &Document, ids: &[NodeId], axis: FlipAxis) -> FlipOutcome {
    let mut selection = ObjectSelection::new();
    selection.set(ids);
    flip_selection(document, &objects(document), &selection, axis)
}

fn rect(document: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: pt(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

fn set_radii(document: &Document, id: NodeId, [tl, tr, br, bl]: [f64; 4]) {
    document
        .set_corner_radii(&[(
            id,
            CornerRadii {
                tl: Length::from_mm(tl),
                tr: Length::from_mm(tr),
                br: Length::from_mm(br),
                bl: Length::from_mm(bl),
            },
        )])
        .unwrap();
}

fn rect_of(document: &Document, id: NodeId) -> (RectBounds, [f64; 4]) {
    let Some(ObjectSnapshot::Primitive(PrimitiveSnapshot {
        shape: Shape::Rect {
            bounds,
            corner_radii,
        },
        ..
    })) = document.object(id)
    else {
        panic!("a rectangle");
    };
    (
        bounds,
        [
            corner_radii.tl,
            corner_radii.tr,
            corner_radii.br,
            corner_radii.bl,
        ]
        .map(Length::as_mm),
    )
}

fn turn(document: &Document, id: NodeId, degrees: f64) {
    let object = document.object(id).unwrap();
    let (min, max) = object_outline_bounds(&object);
    let turned = object.rotated(
        pt(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y)),
        Angle::from_radians(degrees.to_radians()),
    );
    document.rotate_object(&turned).unwrap();
}

fn path(document: &Document, first: u64, points: &[(f64, f64)], closed: bool) -> NodeId {
    let anchors: Vec<NewAnchor> = points
        .iter()
        .enumerate()
        .map(|(k, &(x, y))| NewAnchor::corner(AnchorId::new(1, first + k as u64), pt(x, y)))
        .collect();
    document.create_path(&anchors, closed)
}

fn tbb(document: &Document, ids: &[NodeId]) -> (Point, Point) {
    ids.iter()
        .map(|id| object_outline_bounds(&document.object(*id).unwrap()))
        .reduce(|(a, b), (c, d)| {
            (
                pt(a.x.min(c.x), a.y.min(c.y)),
                pt(b.x.max(d.x), b.y.max(d.y)),
            )
        })
        .unwrap()
}

/// Criterion 2 example: two rectangles swap places about x = 15; B's radius goes to its
/// top-right corner; the selection's box stays.
#[test]
fn the_two_rectangles_of_criterion_2() {
    let document = Document::new(1);
    let a = rect(&document, 0.0, 0.0, 10.0, 10.0);
    let b = rect(&document, 20.0, 0.0, 10.0, 10.0);
    set_radii(&document, b, [2.0, 0.0, 0.0, 0.0]);
    let before = tbb(&document, &[a, b]);
    assert_eq!(flip(&document, &[a, b], H), FlipOutcome::Flipped);
    let (ba, _) = rect_of(&document, a);
    let (bb, rb) = rect_of(&document, b);
    assert!((ba.origin.x - 20.0).abs() < 1e-9 && ba.origin.y.abs() < 1e-9);
    assert!(bb.origin.x.abs() < 1e-9);
    assert_eq!(rb, [0.0, 2.0, 0.0, 0.0]);
    let after = tbb(&document, &[a, b]);
    for (p, q) in [(before.0, after.0), (before.1, after.1)] {
        assert!((p.x - q.x).abs() < 1e-9 && (p.y - q.y).abs() < 1e-9);
    }
}

/// Criterion 20 example: the 3 mm radius at the top-left of a 30 x 10 rectangle; flipped
/// horizontally it is at the top-right, vertically at the bottom-left, both at the
/// bottom-right; a rotated rectangle's rotation is negated.
#[test]
fn a_rectangle_keeps_its_radius_at_the_physical_corner() {
    for (axes, want) in [
        (vec![H], [0.0, 3.0, 0.0, 0.0]),
        (vec![V], [0.0, 0.0, 0.0, 3.0]),
        (vec![H, V], [0.0, 0.0, 3.0, 0.0]),
    ] {
        let document = Document::new(1);
        let id = rect(&document, 0.0, 0.0, 30.0, 10.0);
        set_radii(&document, id, [3.0, 0.0, 0.0, 0.0]);
        for axis in axes {
            assert_eq!(flip(&document, &[id], axis), FlipOutcome::Flipped);
        }
        let (bounds, radii) = rect_of(&document, id);
        assert_eq!(radii, want);
        assert!(bounds.origin.x.abs() < 1e-9 && bounds.origin.y.abs() < 1e-9);
    }
    let document = Document::new(1);
    let id = rect(&document, 0.0, 0.0, 30.0, 10.0);
    turn(&document, id, 30.0);
    flip(&document, &[id], H);
    let rotation = document.object(id).unwrap().rotation().as_radians();
    assert!((rotation + 30.0_f64.to_radians()).abs() < 1e-12);
}

/// Criterion 22 example: the triangle flipped about the centre of its box, x = 26.25.
#[test]
fn the_triangle_of_criterion_22() {
    let document = Document::new(1);
    let id = document.create_polygon(
        StarFrame {
            center: pt(25.0, 5.0),
            radius: Length::from_mm(5.0),
            angle: Angle::from_radians(0.0),
        },
        PointCount::new(3).unwrap(),
    );
    flip(&document, &[id], H);
    let Some(ObjectSnapshot::Primitive(PrimitiveSnapshot {
        shape: Shape::Polygon {
            frame, point_count, ..
        },
        ..
    })) = document.object(id)
    else {
        panic!("still a polygon");
    };
    assert_eq!(point_count.get(), 3);
    assert!((frame.radius.as_mm() - 5.0).abs() < 1e-9);
    assert!((frame.center.x - 27.5).abs() < 1e-9 && (frame.center.y - 5.0).abs() < 1e-9);
}

/// Criterion 3: a flip that changes nothing writes nothing and says so.
#[test]
fn a_flip_that_changes_nothing_writes_nothing() {
    let document = Document::new(1);
    let circle = document.create_ellipse(EllipseFrame {
        center: pt(10.0, 10.0),
        rx: Length::from_mm(5.0),
        ry: Length::from_mm(5.0),
    });
    turn(&document, circle, 30.0);
    let line = path(&document, 1, &[(0.0, 50.0), (40.0, 50.0)], false);
    let symmetric = path(
        &document,
        10,
        &[(100.0, 0.0), (110.0, 0.0), (105.0, 10.0)],
        true,
    );
    let before = objects(&document);
    for (ids, axis) in [
        (vec![circle], H),
        (vec![circle], V),
        (vec![line], V),
        (vec![symmetric], H),
    ] {
        assert_eq!(
            flip(&document, &ids, axis),
            FlipOutcome::LooksTheSame,
            "{ids:?}"
        );
    }
    assert_eq!(objects(&document), before);
    assert_eq!(flip(&document, &[symmetric], V), FlipOutcome::Flipped);
    assert_eq!(flip(&document, &[], H), FlipOutcome::NothingSelected);
}

/// Criterion 19: a ring flipped horizontally keeps its hole a hole (the two outlines keep
/// opposite windings), and the hole's centre is where the mirror puts it.
#[test]
fn a_compound_path_keeps_its_holes() {
    let document = Document::new(1);
    let square = |first: u64, x: f64, y: f64, side: f64, reversed: bool| {
        let mut corners = vec![(x, y), (x + side, y), (x + side, y + side), (x, y + side)];
        if reversed {
            corners.reverse();
        }
        let anchors: Vec<NewAnchor> = corners
            .into_iter()
            .enumerate()
            .map(|(k, (cx, cy))| NewAnchor::corner(AnchorId::new(4, first + k as u64), pt(cx, cy)))
            .collect();
        (anchors, true)
    };
    let seed = path(&document, 90, &[(900.0, 900.0), (901.0, 900.0)], false);
    let ring = document
        .replace_with_path(
            &[seed],
            seed,
            &[
                square(100, 0.0, 0.0, 40.0, false),
                square(200, 5.0, 10.0, 10.0, true),
            ],
            "boolean_difference",
        )
        .unwrap();
    let area = |anchors: &[Point]| {
        anchors
            .iter()
            .zip(anchors.iter().cycle().skip(1))
            .map(|(p, q)| p.x * q.y - q.x * p.y)
            .sum::<f64>()
    };
    flip(&document, &[ring], H);
    let Some(ObjectSnapshot::Path(flipped)) = document.object(ring) else {
        panic!("a compound path");
    };
    let outer: Vec<Point> = flipped.anchors.iter().map(|a| a.point).collect();
    let hole: Vec<Point> = flipped.extra_subpaths[0]
        .anchors
        .iter()
        .map(|a| a.point)
        .collect();
    assert!(area(&outer) * area(&hole) < 0.0, "opposite windings");
    let centre_x = hole.iter().map(|p| p.x).sum::<f64>() / 4.0;
    assert!((centre_x - 30.0).abs() < 1e-9, "{centre_x}");
}

/// Criterion 25 example: a clone alone flipped horizontally about its box centre 35: M
/// becomes (-1, 0, 0, 1, 40, 0) and the origin is untouched.
#[test]
fn a_clone_alone_takes_the_reflection_in_its_matrix() {
    let document = Document::new(1);
    let origin = path(&document, 1, &[(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)], true);
    let clone = document.linked_clone(&[origin], &[]).unwrap()[0];
    document
        .translate_objects(&[clone], Vec2::new(30.0, 0.0))
        .unwrap();
    let origin_before = document.object(origin).unwrap();
    assert_eq!(flip(&document, &[clone], H), FlipOutcome::Flipped);
    let Some(ObjectSnapshot::Clone(snapshot)) = document.object(clone) else {
        panic!("still a clone");
    };
    let m = snapshot.chain.matrix.to_array();
    for (got, want) in m.iter().zip([-1.0, 0.0, 0.0, 1.0, 40.0, 0.0]) {
        assert!((got - want).abs() < 1e-9, "{m:?}");
    }
    let points: Vec<Point> = snapshot.resolved.anchors.iter().map(|a| a.point).collect();
    for (p, (x, y)) in points.iter().zip([(40.0, 0.0), (30.0, 0.0), (40.0, 10.0)]) {
        assert!(
            (p.x - x).abs() < 1e-9 && (p.y - y).abs() < 1e-9,
            "{points:?}"
        );
    }
    assert_eq!(document.object(origin).unwrap(), origin_before);
}

/// Every kind of Part D at coordinates near 1e5 mm, rotated where it has a rotation.
fn every_kind(document: &Document) -> Vec<NodeId> {
    let far = 1.0e5;
    let r = rect(document, far, far, 30.0, 10.0);
    set_radii(document, r, [1.0, 2.0, 3.0, 4.0]);
    turn(document, r, 30.0);
    let e = document.create_ellipse(EllipseFrame {
        center: pt(far + 50.0, far),
        rx: Length::from_mm(8.0),
        ry: Length::from_mm(3.0),
    });
    turn(document, e, 20.0);
    let c = document.create_ellipse(EllipseFrame {
        center: pt(far - 20.0, far + 5.0),
        rx: Length::from_mm(4.0),
        ry: Length::from_mm(4.0),
    });
    let frame = StarFrame {
        center: pt(far + 10.0, far + 40.0),
        radius: Length::from_mm(6.0),
        angle: Angle::from_radians(0.3),
    };
    let polygon = document.create_polygon(frame, PointCount::new(5).unwrap());
    turn(document, polygon, 15.0);
    let star = document.create_star(
        StarFrame {
            center: pt(far - 30.0, far - 30.0),
            ..frame
        },
        PointCount::new(7).unwrap(),
        InnerRatio::new(0.4).unwrap(),
    );
    let curve = document.create_path(
        &[
            NewAnchor {
                id: AnchorId::new(2, 1),
                point: pt(far, far - 20.0),
                handle_in: Vec2::new(-2.0, 1.0),
                handle_out: Vec2::new(2.0, -1.0),
                kind: AnchorKind::Symmetric,
            },
            NewAnchor::corner(AnchorId::new(2, 2), pt(far + 30.0, far - 15.0)),
            NewAnchor {
                id: AnchorId::new(2, 3),
                point: pt(far + 12.0, far - 2.0),
                handle_in: Vec2::new(3.0, 0.0),
                handle_out: Vec2::new(-1.0, 4.0),
                kind: AnchorKind::Asymmetric,
            },
        ],
        true,
    );
    turn(document, curve, 10.0);
    vec![r, e, c, polygon, star, curve]
}

fn assert_same(a: &[ObjectSnapshot], b: &[ObjectSnapshot]) {
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b) {
        let (nx, ny) = (numbers(x), numbers(y));
        assert_eq!(nx.len(), ny.len());
        for (p, q) in nx.iter().zip(&ny) {
            assert!((p - q).abs() < 1e-9, "{x:?}\n{y:?}");
        }
        let turn = Angle::from_radians(x.rotation().as_radians() - y.rotation().as_radians())
            .normalized()
            .as_radians();
        assert!(turn.abs() < 1e-12, "{x:?}\n{y:?}");
    }
}

/// The drawn outline's anchor points of `object`.
fn drawn(object: &ObjectSnapshot) -> Vec<Point> {
    match object {
        ObjectSnapshot::Sketch(_) => unreachable!("no sketch in this test"),
        ObjectSnapshot::Image(_) => unreachable!("no image in this test"),
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Primitive(p) => outline_of_rotated(&p.shape, p.rotation)
            .iter()
            .map(|a| a.point)
            .collect(),
        ObjectSnapshot::Path(p) => p.all_anchors().map(|a| a.point).collect(),
        ObjectSnapshot::Clone(c) => c.resolved.all_anchors().map(|a| a.point).collect(),
    }
}

fn numbers(object: &ObjectSnapshot) -> Vec<f64> {
    drawn(object).iter().flat_map(|p| [p.x, p.y]).collect()
}

/// Criterion 28 a and c: flipping twice, either way, gives every object back, alone and in
/// a mixed selection; the box of the selection is unchanged by each flip.
#[test]
fn flipping_twice_gives_every_object_back() {
    for axis in [H, V] {
        let document = Document::new(1);
        let ids = every_kind(&document);
        let before = objects(&document);
        let box_before = tbb(&document, &ids);
        flip(&document, &ids, axis);
        let box_after = tbb(&document, &ids);
        for (p, q) in [(box_before.0, box_after.0), (box_before.1, box_after.1)] {
            assert!((p.x - q.x).abs() < 1e-9 && (p.y - q.y).abs() < 1e-9);
        }
        flip(&document, &ids, axis);
        assert_same(&objects(&document), &before);
        for id in &ids {
            let alone = objects(&document);
            flip(&document, &[*id], axis);
            flip(&document, &[*id], axis);
            assert_same(&objects(&document), &alone);
        }
    }
}

/// Criterion 28 b: a horizontal and then a vertical flip is a half turn about the centre
/// of the box, outline for outline.
#[test]
fn two_flips_are_a_half_turn() {
    let document = Document::new(1);
    let ids = every_kind(&document);
    let (min, max) = tbb(&document, &ids);
    let centre = pt(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
    let turned: Vec<ObjectSnapshot> = objects(&document)
        .iter()
        .map(|o| o.rotated(centre, Angle::from_radians(std::f64::consts::PI)))
        .collect();
    flip(&document, &ids, H);
    flip(&document, &ids, V);
    for (flipped, turned) in objects(&document).iter().zip(&turned) {
        let (a, b) = (drawn(flipped), drawn(turned));
        for p in &a {
            assert!(
                b.iter()
                    .any(|q| (p.x - q.x).abs() < 1e-6 && (p.y - q.y).abs() < 1e-6),
                "{p:?} not in {b:?}"
            );
        }
    }
}

/// Criterion 28 f: the result does not depend on the order the objects were selected in;
/// every id and anchor id stays.
#[test]
fn the_selection_order_does_not_matter() {
    let run = |reverse: bool| {
        let document = Document::new(1);
        let mut ids = every_kind(&document);
        if reverse {
            ids.reverse();
        }
        flip(&document, &ids, H);
        format!("{:?}", objects(&document))
    };
    assert_eq!(run(false), run(true));
    let document = Document::new(1);
    let ids = every_kind(&document);
    let before: Vec<(NodeId, Vec<AnchorId>)> = anchor_ids(&document);
    flip(&document, &ids, V);
    assert_eq!(anchor_ids(&document), before);
}

fn anchor_ids(document: &Document) -> Vec<(NodeId, Vec<AnchorId>)> {
    objects(document)
        .iter()
        .map(|o| {
            let ids = match o {
                ObjectSnapshot::Path(p) => p.all_anchors().map(|a| a.id).collect(),
                _ => Vec::new(),
            };
            (o.id(), ids)
        })
        .collect()
}
