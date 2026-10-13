//! The document half of Offset (`specs/0038-path-offset` criteria 16, 20, 21, 22, 42): copies go
//! directly below (outset) or above (inset) their source with its complete style, a changed
//! original keeps its id and place, a primitive stays one or becomes a path in place, everything
//! lands in one commit, and a refusal changes nothing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::ops::ControlFlow;

use curvyo_document_core::{
    AnchorId, Angle, CornerRadii, Document, EllipseFrame, Length, NewAnchor, NodeId,
    ObjectEditError, ObjectSnapshot, OffsetGeometry, OffsetWrite, Point, RectBounds, Shape,
    StyleEdit,
};
use loro::LoroDoc;

type Outline = (Vec<NewAnchor>, bool);

fn id(n: u64) -> AnchorId {
    AnchorId::new(9, n)
}

fn square(first: u64, x: f64, side: f64) -> Outline {
    let corners = [(x, 0.0), (x + side, 0.0), (x + side, side), (x, side)];
    let anchors = corners
        .into_iter()
        .zip(first..)
        .map(|((cx, cy), n)| NewAnchor::corner(id(n), Point::new(cx, cy)))
        .collect();
    (anchors, true)
}

fn path(outlines: Vec<Outline>) -> OffsetGeometry {
    OffsetGeometry::Path(outlines)
}

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn rect_shape(x: f64, side: f64, radius: f64) -> Shape {
    Shape::Rect {
        bounds: RectBounds {
            origin: Point::new(x, x),
            width: Length::from_mm(side),
            height: Length::from_mm(side),
        },
        corner_radii: CornerRadii::uniform(Length::from_mm(radius)),
    }
}

fn change_count(document: &Document) -> (usize, String) {
    let loro = LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    let ids: Vec<loro::ID> = loro.oplog_frontiers().iter().collect();
    let mut out: Vec<(u32, String)> = Vec::new();
    loro.travel_change_ancestors(&ids, &mut |m| {
        out.push((
            m.lamport,
            m.message
                .map(|s| s.split(';').next().unwrap_or_default().to_string())
                .unwrap_or_default(),
        ));
        ControlFlow::Continue(())
    })
    .unwrap();
    out.sort();
    let last = out.last().map(|(_, s)| s.clone()).unwrap_or_default();
    (out.len(), last)
}

fn stroke_width(document: &Document, node: NodeId) -> f64 {
    document.object(node).unwrap().style().stroke.width.as_mm()
}

#[test]
fn ac20_copies_go_below_an_outset_and_above_an_inset_in_one_commit() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    let between = rect(&document, 50.0);
    let b = rect(&document, 100.0);
    document
        .edit_style(&[a, b], &StyleEdit::StrokeWidth(Length::from_mm(3.0)))
        .unwrap();
    let (before, _) = change_count(&document);
    let ids = document
        .write_offset(
            &[
                OffsetWrite::Add {
                    source: a,
                    below: true,
                    geometry: path(vec![square(100, -1.0, 12.0)]),
                },
                OffsetWrite::Add {
                    source: b,
                    below: true,
                    geometry: path(vec![square(200, 99.0, 12.0)]),
                },
                OffsetWrite::Add {
                    source: b,
                    below: false,
                    geometry: path(vec![square(300, 101.0, 8.0), square(310, 103.0, 2.0)]),
                },
            ],
            "offset_path",
        )
        .unwrap();
    assert_eq!(
        document.object_ids(),
        vec![ids[0], a, between, ids[1], b, ids[2]],
        "outer copy below, inner copy above, the original between"
    );
    let (after, label) = change_count(&document);
    assert_eq!(after, before + 1, "one commit");
    assert_eq!(label, "offset_path");
    assert_eq!(stroke_width(&document, ids[0]), 3.0, "the complete style");
    let ObjectSnapshot::Path(inner) = document.object(ids[2]).unwrap() else {
        panic!("a path");
    };
    assert!(inner.is_compound());
    assert_eq!(inner.rotation, Angle::from_radians(0.0));
}

#[test]
fn ac21_a_changed_path_keeps_its_id_place_and_style() {
    let document = Document::new(1);
    let below = rect(&document, 50.0);
    let (anchors, _) = square(1, 0.0, 10.0);
    let original = document.create_path(&anchors, true);
    let above = rect(&document, 80.0);
    document
        .edit_style(&[original], &StyleEdit::StrokeWidth(Length::from_mm(2.5)))
        .unwrap();
    let ids = document
        .write_offset(
            &[OffsetWrite::Change {
                target: original,
                geometry: path(vec![square(50, -2.0, 14.0), square(60, 2.0, 2.0)]),
            }],
            "offset_path",
        )
        .unwrap();
    assert_eq!(ids, vec![original]);
    assert_eq!(document.object_ids(), vec![below, original, above]);
    let ObjectSnapshot::Path(changed) = document.object(original).unwrap() else {
        panic!("a path");
    };
    assert_eq!(changed.all_anchors().count(), 8);
    assert_eq!(
        changed.all_anchors().next().unwrap().point,
        Point::new(-2.0, 0.0)
    );
    assert_eq!(changed.style.stroke.width.as_mm(), 2.5);
    // A second change replaces the extra outline too.
    document
        .write_offset(
            &[OffsetWrite::Change {
                target: original,
                geometry: path(vec![square(70, -4.0, 18.0)]),
            }],
            "offset_path",
        )
        .unwrap();
    let ObjectSnapshot::Path(again) = document.object(original).unwrap() else {
        panic!("a path");
    };
    assert!(!again.is_compound());
}

#[test]
fn ac42_a_primitive_changes_its_fields_or_becomes_a_path_in_place() {
    let document = Document::new(1);
    let r = rect(&document, 0.0);
    let ring = document.create_ellipse(EllipseFrame {
        center: Point::new(50.0, 50.0),
        rx: Length::from_mm(20.0),
        ry: Length::from_mm(10.0),
    });
    document
        .write_offset(
            &[
                OffsetWrite::Change {
                    target: r,
                    geometry: OffsetGeometry::Primitive {
                        shape: rect_shape(-2.0, 14.0, 2.0),
                        rotation: Angle::from_radians(0.0),
                    },
                },
                OffsetWrite::Change {
                    target: ring,
                    geometry: path(vec![square(1, 28.0, 44.0)]),
                },
            ],
            "offset_path",
        )
        .unwrap();
    assert_eq!(document.object_ids(), vec![r, ring]);
    let ObjectSnapshot::Primitive(changed) = document.object(r).unwrap() else {
        panic!("still a rectangle");
    };
    assert_eq!(changed.shape, rect_shape(-2.0, 14.0, 2.0));
    assert!(matches!(
        document.object(ring).unwrap(),
        ObjectSnapshot::Path(_)
    ));
}

#[test]
fn ac41_a_copied_primitive_keeps_the_given_rotation() {
    let document = Document::new(1);
    let r = rect(&document, 0.0);
    let ids = document
        .write_offset(
            &[OffsetWrite::Add {
                source: r,
                below: true,
                geometry: OffsetGeometry::Primitive {
                    shape: rect_shape(-2.0, 14.0, 2.0),
                    rotation: Angle::from_radians(0.5),
                },
            }],
            "offset_path",
        )
        .unwrap();
    let ObjectSnapshot::Primitive(copy) = document.object(ids[0]).unwrap() else {
        panic!("a rectangle");
    };
    assert_eq!(copy.rotation, Angle::from_radians(0.5));
    assert_eq!(copy.shape, rect_shape(-2.0, 14.0, 2.0));
}

#[test]
fn a_refusal_changes_nothing() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    let ghost = rect(&document, 50.0);
    document.delete_objects(&[ghost]).unwrap();
    let kept = document.create_path(&[NewAnchor::corner(id(900), Point::new(0.0, 0.0))], false);
    let ids = document.object_ids();
    let (changes, _) = change_count(&document);
    let add = |source: NodeId, outline: Outline| OffsetWrite::Add {
        source,
        below: true,
        geometry: path(vec![outline]),
    };
    let cases = [
        (
            document.write_offset(&[add(ghost, square(1, 0.0, 1.0))], "offset_path"),
            ObjectEditError::NoSuchObject,
        ),
        (
            document.write_offset(&[], "offset_path"),
            ObjectEditError::NoOutlines,
        ),
        (
            document.write_offset(
                &[OffsetWrite::Add {
                    source: a,
                    below: false,
                    geometry: path(Vec::new()),
                }],
                "offset_path",
            ),
            ObjectEditError::NoOutlines,
        ),
        (
            document.write_offset(&[add(a, square(900, 0.0, 1.0))], "offset_path"),
            ObjectEditError::AnchorIds,
        ),
        (
            document.write_offset(
                &[add(a, square(1, 0.0, 1.0)), add(a, square(1, 5.0, 1.0))],
                "offset_path",
            ),
            ObjectEditError::AnchorIds,
        ),
        (
            document.write_offset(
                &[OffsetWrite::Change {
                    target: kept,
                    geometry: OffsetGeometry::Primitive {
                        shape: rect_shape(0.0, 1.0, 0.0),
                        rotation: Angle::from_radians(0.0),
                    },
                }],
                "offset_path",
            ),
            ObjectEditError::InvalidConversion,
        ),
    ];
    for (result, expected) in cases {
        assert_eq!(result, Err(expected));
    }
    assert_eq!(document.object_ids(), ids);
    assert_eq!(change_count(&document).0, changes);
}
