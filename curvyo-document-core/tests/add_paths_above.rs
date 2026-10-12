//! The document half of Offset (`specs/0038-path-offset` criteria 10 and 13): every result is a
//! new path directly above its own operand with a copy of the operand's complete style and no
//! rotation, nothing is deleted, everything lands in one commit, and a refusal changes nothing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::ops::ControlFlow;

use curvyo_document_core::{
    AnchorId, Angle, Document, Length, NewAnchor, NodeId, ObjectEditError, ObjectSnapshot, Point,
    RectBounds, StyleEdit,
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

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
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

fn style_width(document: &Document, node: NodeId) -> f64 {
    match document.object(node).unwrap() {
        ObjectSnapshot::Path(path) => path.style.stroke.width.as_mm(),
        ObjectSnapshot::Primitive(primitive) => primitive.style.stroke.width.as_mm(),
    }
}

#[test]
fn each_result_goes_directly_above_its_operand_with_its_style_in_one_commit() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    let between = rect(&document, 50.0);
    let b = rect(&document, 100.0);
    document
        .edit_style(&[a], &StyleEdit::StrokeWidth(Length::from_mm(3.0)))
        .unwrap();
    let (before, _) = change_count(&document);

    let created = document
        .add_paths_above(
            &[
                (a, vec![square(100, -1.0, 12.0)]),
                (b, vec![square(200, 99.0, 12.0), square(210, 101.0, 2.0)]),
            ],
            "offset_path",
        )
        .unwrap();

    assert_eq!(created.len(), 2);
    assert_eq!(
        document.object_ids(),
        vec![a, created[0], between, b, created[1]],
        "each result directly above its own operand; nothing deleted"
    );
    let (after, label) = change_count(&document);
    assert_eq!(after, before + 1, "one commit");
    assert_eq!(label, "offset_path");
    assert_eq!(style_width(&document, created[0]), 3.0);
    let ObjectSnapshot::Path(second) = document.object(created[1]).unwrap() else {
        panic!("a path");
    };
    assert!(second.is_compound());
    assert_eq!(second.rotation, Angle::from_radians(0.0));
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
    let cases = [
        (
            document.add_paths_above(&[(ghost, vec![square(1, 0.0, 1.0)])], "offset_path"),
            ObjectEditError::NoSuchObject,
        ),
        (
            document.add_paths_above(&[], "offset_path"),
            ObjectEditError::NoOutlines,
        ),
        (
            document.add_paths_above(&[(a, Vec::new())], "offset_path"),
            ObjectEditError::NoOutlines,
        ),
        (
            document.add_paths_above(&[(a, vec![square(900, 0.0, 1.0)])], "offset_path"),
            ObjectEditError::AnchorIds,
        ),
        (
            document.add_paths_above(
                &[
                    (a, vec![square(1, 0.0, 1.0)]),
                    (a, vec![square(1, 5.0, 1.0)]),
                ],
                "offset_path",
            ),
            ObjectEditError::AnchorIds,
        ),
    ];
    for (result, expected) in cases {
        assert_eq!(result, Err(expected));
    }
    assert_eq!(document.object_ids(), ids);
    assert_eq!(change_count(&document).0, changes);
    assert!(document.object(kept).is_some());
}
