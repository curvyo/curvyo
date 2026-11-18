//! `specs/0124-gradient-fill` criteria 15 and 19 in the document: a transform's result writes
//! the image of its gradient in the same step, and an operation's pieces continue their source's
//! shade (ADR 0066 §5, §6).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    Affine, AnchorId, AxisRatio, BoxPoint, Document, FillPaint, GradientGeometry, Length,
    NewAnchor, NodeId, ObjectSnapshot, OffsetGeometry, OffsetWrite, PieceFill, PlacedGradient,
    Point, PointCount, StarFrame, StyleEdit, carry_gradient, outline_of_rotated,
};

fn anchors(first: u64, points: &[(f64, f64)]) -> Vec<NewAnchor> {
    points
        .iter()
        .zip(first..)
        .map(|((x, y), k)| NewAnchor::corner(AnchorId::new(1, k), Point::new(*x, *y)))
        .collect()
}

fn with_gradient(document: &Document, id: NodeId, geometry: GradientGeometry) {
    document
        .edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Gradient))
        .unwrap();
    document
        .edit_style(&[id], &StyleEdit::GradientGeometry(geometry))
        .unwrap();
}

fn placed(object: &ObjectSnapshot) -> PlacedGradient {
    let gradient = object.style().fill.gradient.clone().expect("a gradient");
    PlacedGradient::new(&gradient, &object.gradient_box().expect("a box"))
}

/// The picture of `after` at `map(p)` is the picture of `before` at `p`.
fn assert_same_picture(before: &PlacedGradient, after: &PlacedGradient, map: Affine, at: &[Point]) {
    for &p in at {
        let (tb, ta) = (before.t_at(p), after.t_at(map.apply_point(p)));
        assert!((tb - ta).abs() < 1e-9, "at {p:?}: {tb} before, {ta} after");
    }
}

fn probes() -> Vec<Point> {
    [
        (0.0, 0.0),
        (30.0, 10.0),
        (100.0, 40.0),
        (65.0, 3.0),
        (12.0, 37.0),
    ]
    .map(|(x, y)| Point::new(x, y))
    .to_vec()
}

fn slanted() -> GradientGeometry {
    GradientGeometry::Linear {
        start: BoxPoint::new(0.1, 0.2).unwrap(),
        end: BoxPoint::new(0.9, 0.7).unwrap(),
    }
}

#[test]
fn a_skew_result_writes_the_image_of_its_gradient_in_the_same_step() {
    let d = Document::new(1);
    let id = d.create_path(
        &anchors(0, &[(0.0, 0.0), (100.0, 0.0), (100.0, 40.0), (0.0, 40.0)]),
        true,
    );
    with_gradient(&d, id, slanted());
    let start = d.object(id).unwrap();
    let skew = Affine::new(1.0, 0.0, 0.6, 1.0, 0.0, 0.0);
    let ObjectSnapshot::Path(path) = &start else {
        panic!("a path")
    };
    let mut result = ObjectSnapshot::Path(path.mapped(skew));
    carry_gradient(&start, &mut result, skew);
    let steps = d.own_step_count();
    d.transform_objects(std::slice::from_ref(&result), false)
        .unwrap();
    assert_eq!(d.own_step_count(), steps + 1, "one step");
    let after = d.object(id).unwrap();
    assert_eq!(after.style().fill.gradient, result.style().fill.gradient);
    assert_ne!(after.style().fill.gradient, start.style().fill.gradient);
    assert_same_picture(&placed(&start), &placed(&after), skew, &probes());
}

#[test]
fn a_move_writes_no_gradient() {
    let d = Document::new(1);
    let id = d.create_path(&anchors(0, &[(0.0, 0.0), (50.0, 0.0), (50.0, 20.0)]), true);
    with_gradient(&d, id, slanted());
    let start = d.object(id).unwrap();
    let shift = Affine::translation(curvyo_document_core::Vec2::new(12.5, -3.25));
    let mut result = start.translated(curvyo_document_core::Vec2::new(12.5, -3.25));
    carry_gradient(&start, &mut result, shift);
    assert_eq!(result.style().fill.gradient, start.style().fill.gradient);
}

#[test]
fn break_apart_pieces_continue_one_shade() {
    let d = Document::new(1);
    let left = anchors(0, &[(0.0, 0.0), (50.0, 0.0), (50.0, 40.0), (0.0, 40.0)]);
    let right = anchors(
        10,
        &[(50.0, 0.0), (100.0, 0.0), (100.0, 40.0), (50.0, 40.0)],
    );
    let id = d.create_path(&left, true);
    let both = d
        .replace_with_path(
            &[id],
            id,
            &[
                (
                    anchors(20, &[(0.0, 0.0), (50.0, 0.0), (50.0, 40.0), (0.0, 40.0)]),
                    true,
                ),
                (
                    anchors(
                        30,
                        &[(50.0, 0.0), (100.0, 0.0), (100.0, 40.0), (50.0, 40.0)],
                    ),
                    true,
                ),
            ],
            "combine",
        )
        .unwrap();
    with_gradient(&d, both, GradientGeometry::DEFAULT_LINEAR);
    let whole = placed(&d.object(both).unwrap());
    let pieces = d
        .replace_with_pieces(
            &[both],
            &[(both, vec![(left, true)]), (both, vec![(right, true)])],
            PieceFill::Keep,
            "break_apart",
        )
        .unwrap();
    let second = d.object(pieces[1]).unwrap();
    assert_eq!(
        second.style().fill.gradient.as_ref().unwrap().geometry,
        GradientGeometry::Linear {
            start: BoxPoint::new(-1.0, 0.5).unwrap(),
            end: BoxPoint::new(1.0, 0.5).unwrap(),
        }
    );
    for piece in &pieces {
        assert_same_picture(
            &whole,
            &placed(&d.object(*piece).unwrap()),
            Affine::IDENTITY,
            &probes(),
        );
    }
}

#[test]
fn a_boolean_result_keeps_the_base_shade_where_it_was() {
    let d = Document::new(1);
    let a = d.create_path(
        &anchors(0, &[(0.0, 0.0), (100.0, 0.0), (100.0, 40.0), (0.0, 40.0)]),
        true,
    );
    with_gradient(&d, a, slanted());
    let before = placed(&d.object(a).unwrap());
    let result = d
        .replace_with_path(
            &[a],
            a,
            &[(anchors(40, &[(20.0, 5.0), (60.0, 5.0), (60.0, 30.0)]), true)],
            "boolean_union",
        )
        .unwrap();
    assert_same_picture(
        &before,
        &placed(&d.object(result).unwrap()),
        Affine::IDENTITY,
        &probes(),
    );
}

#[test]
fn object_to_path_of_a_turned_polygon_keeps_the_shade() {
    let d = Document::new(1);
    let frame = StarFrame {
        center: Point::new(40.0, 30.0),
        radius: Length::from_mm(20.0),
        angle: curvyo_document_core::Angle::from_radians(0.3),
    };
    let id = d.create_polygon(frame, PointCount::new(5).unwrap());
    with_gradient(
        &d,
        id,
        GradientGeometry::Radial {
            centre: BoxPoint::new(0.3, 0.4).unwrap(),
            radius: BoxPoint::new(0.8, 0.5).unwrap(),
            ratio: AxisRatio::new(0.5).unwrap(),
        },
    );
    let start = d.object(id).unwrap();
    let ObjectSnapshot::Primitive(primitive) = &start else {
        panic!("a primitive")
    };
    let outline = outline_of_rotated(&primitive.shape, primitive.rotation);
    let converted: Vec<NewAnchor> = outline
        .anchors
        .iter()
        .zip(100..)
        .map(|(anchor, k)| NewAnchor {
            id: AnchorId::new(1, k),
            point: anchor.point,
            handle_in: anchor.handle_in,
            handle_out: anchor.handle_out,
            kind: anchor.kind,
        })
        .collect();
    d.convert_to_paths(&[(id, converted)]).unwrap();
    let after = d.object(id).unwrap();
    assert!(matches!(after, ObjectSnapshot::Path(_)));
    assert_ne!(
        after.style().fill.gradient.as_ref().unwrap().geometry,
        start.style().fill.gradient.as_ref().unwrap().geometry,
        "the polygon's box is smaller than its frame's"
    );
    assert_same_picture(
        &placed(&start),
        &placed(&after),
        Affine::IDENTITY,
        &[
            Point::new(40.0, 30.0),
            Point::new(31.0, 22.0),
            Point::new(52.0, 40.0),
        ],
    );
}

#[test]
fn an_offset_copy_continues_its_source_shade() {
    let d = Document::new(1);
    let id = d.create_path(
        &anchors(0, &[(0.0, 0.0), (100.0, 0.0), (100.0, 40.0), (0.0, 40.0)]),
        true,
    );
    with_gradient(&d, id, slanted());
    let before = placed(&d.object(id).unwrap());
    let ids = d
        .write_offset(
            &[OffsetWrite::Add {
                source: id,
                below: true,
                geometry: OffsetGeometry::Path(vec![(
                    anchors(
                        50,
                        &[(-5.0, -5.0), (105.0, -5.0), (105.0, 45.0), (-5.0, 45.0)],
                    ),
                    true,
                )]),
            }],
            "offset",
        )
        .unwrap();
    assert_same_picture(
        &before,
        &placed(&d.object(ids[0]).unwrap()),
        Affine::IDENTITY,
        &probes(),
    );
}

/// Makes a linked clone of `root`, carries it by `m`, unlinks it and checks the unlinked object
/// shows the shade the clone showed (`specs/0124-gradient-fill` criterion 17).
fn assert_unlink_keeps_the_clone_shade(d: &Document, root: NodeId, m: Affine) {
    let clone = d.linked_clone(&[root], &[]).unwrap()[0];
    let ObjectSnapshot::Clone(snapshot) = d.object(clone).unwrap() else {
        panic!("a clone")
    };
    d.transform_objects(&[ObjectSnapshot::Clone(snapshot.mapped_by(m))], false)
        .unwrap();
    let shown = placed(&d.object(clone).unwrap());
    let anchor_count = match d.object(clone).unwrap() {
        ObjectSnapshot::Clone(c) => c.resolved.all_anchors().count(),
        _ => unreachable!(),
    };
    d.unlink_clones(&[curvyo_document_core::CopySource {
        id: clone,
        anchor_ids: (0..anchor_count as u64)
            .map(|k| AnchorId::new(7, 900 + k))
            .collect(),
    }])
    .unwrap();
    let unlinked = d.object(clone).unwrap();
    assert!(
        !matches!(unlinked, ObjectSnapshot::Clone(_)),
        "unlinked: {unlinked:?}"
    );
    assert_same_picture(&shown, &placed(&unlinked), Affine::IDENTITY, &probes());
}

fn transforms() -> [(&'static str, Affine); 3] {
    [
        ("mirrored", Affine::new(-1.0, 0.0, 0.0, 1.0, 140.0, 5.0)),
        (
            "rotated",
            Affine::rotation_about(
                Point::new(20.0, 10.0),
                curvyo_document_core::Angle::from_radians(0.7),
            ),
        ),
        ("skewed", Affine::new(1.0, 0.2, 0.5, 1.0, 3.0, -2.0)),
    ]
}

#[test]
fn unlinking_a_transformed_clone_of_a_path_keeps_its_shade() {
    for (what, m) in transforms() {
        let d = Document::new(1);
        let root = d.create_path(
            &anchors(0, &[(0.0, 0.0), (100.0, 0.0), (100.0, 40.0), (0.0, 40.0)]),
            true,
        );
        with_gradient(&d, root, slanted());
        eprintln!("{what}");
        assert_unlink_keeps_the_clone_shade(&d, root, m);
    }
}

#[test]
fn unlinking_a_transformed_clone_of_a_group_keeps_its_members_shade() {
    for (what, m) in transforms() {
        let d = Document::new(1);
        let member = d.create_path(
            &anchors(0, &[(0.0, 0.0), (100.0, 0.0), (100.0, 40.0), (0.0, 40.0)]),
            true,
        );
        let other = d.create_path(
            &anchors(10, &[(0.0, 50.0), (10.0, 50.0), (10.0, 60.0)]),
            true,
        );
        with_gradient(&d, member, slanted());
        let group = d.group_objects(&[member, other]).unwrap();
        let before = placed(&d.object(member).unwrap());
        let clone = d.linked_clone(&[group], &[]).unwrap()[0];
        let ObjectSnapshot::Clone(snapshot) = d.object(clone).unwrap() else {
            panic!("a clone")
        };
        d.transform_objects(&[ObjectSnapshot::Clone(snapshot.mapped_by(m))], false)
            .unwrap();
        d.unlink_clones(&[curvyo_document_core::CopySource {
            id: clone,
            anchor_ids: Vec::new(),
        }])
        .unwrap();
        let shaded: Vec<ObjectSnapshot> = d
            .leaves_of(clone)
            .into_iter()
            .filter_map(|leaf| d.object(leaf))
            .filter(|object| object.style().fill.gradient.is_some())
            .collect();
        assert_eq!(shaded.len(), 1, "{what}: one shaded member");
        eprintln!("{what}");
        assert_same_picture(&before, &placed(&shaded[0]), m, &probes());
    }
}
