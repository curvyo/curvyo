//! The document half of Stroke to path (`specs/0085-stroke-to-path` criteria 14, 17, 19, 21):
//! the fill part keeps its id and kind with Stroke off, the outline lies directly above it
//! filled in the stroke colour, a subject with Fill off becomes its outline under its own id,
//! clones are unlinked, all in one commit, and a refusal changes nothing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::ops::ControlFlow;

use curvyo_document_core::{
    AnchorId, Color, CopySource, Document, Length, NewAnchor, ObjectEditError, ObjectSnapshot,
    Point, RectBounds, STROKE_TO_PATH_COMMIT_LABEL, StrokeOutlineWrite, StrokeSubject,
    StrokeToPath, StyleEdit,
};
use loro::LoroDoc;

type Outline = (Vec<NewAnchor>, bool);

fn square(first: u64, x: f64, side: f64) -> Outline {
    let corners = [(x, 0.0), (x + side, 0.0), (x + side, side), (x, side)];
    let anchors = corners
        .into_iter()
        .zip(first..)
        .map(|((cx, cy), n)| NewAnchor::corner(AnchorId::new(9, n), Point::new(cx, cy)))
        .collect();
    (anchors, true)
}

fn commits(document: &Document) -> (usize, String) {
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

fn rect(document: &Document, x: f64) -> curvyo_document_core::NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

#[test]
fn ac14_ac19_a_filled_rectangle_keeps_its_kind_and_gets_its_outline_above() {
    let document = Document::new(1);
    let below = rect(&document, 50.0);
    let shape = rect(&document, 0.0);
    let above = rect(&document, 80.0);
    document
        .edit_style(&[shape], &StyleEdit::FillEnabled(true))
        .unwrap();
    document
        .edit_style(
            &[shape],
            &StyleEdit::StrokeColor(Color {
                r: 200,
                g: 10,
                b: 0,
            }),
        )
        .unwrap();
    let (before, _) = commits(&document);
    let ids = document
        .stroke_to_path(&StrokeToPath {
            outlines: vec![StrokeOutlineWrite {
                subject: StrokeSubject::Object(shape),
                outline: vec![square(1, -1.0, 12.0), square(10, 1.0, 8.0)],
            }],
            ..StrokeToPath::default()
        })
        .unwrap();
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0], shape);
    assert_eq!(document.object_ids(), vec![below, shape, ids[1], above]);
    let (after, label) = commits(&document);
    assert_eq!(after, before + 1, "one commit");
    assert_eq!(label, STROKE_TO_PATH_COMMIT_LABEL);
    let ObjectSnapshot::Primitive(fill_part) = document.object(shape).unwrap() else {
        panic!("the rectangle stays a rectangle");
    };
    assert!(!fill_part.style.stroke.enabled);
    assert!(fill_part.style.fill.enabled);
    let ObjectSnapshot::Path(outline) = document.object(ids[1]).unwrap() else {
        panic!("a path");
    };
    assert!(outline.is_compound());
    assert!(outline.style.fill.enabled);
    assert_eq!(
        outline.style.fill.color,
        Color {
            r: 200,
            g: 10,
            b: 0
        }
    );
    assert!(!outline.style.stroke.enabled);
}

#[test]
fn ac19_a_subject_without_fill_becomes_its_outline_under_its_id() {
    let document = Document::new(1);
    let (anchors, _) = square(1, 0.0, 10.0);
    let line = document.create_path(&anchors[..2], false);
    document
        .edit_style(&[line], &StyleEdit::StrokeWidth(Length::from_mm(2.0)))
        .unwrap();
    let ids = document
        .stroke_to_path(&StrokeToPath {
            outlines: vec![StrokeOutlineWrite {
                subject: StrokeSubject::Object(line),
                outline: vec![square(20, 0.0, 2.0)],
            }],
            ..StrokeToPath::default()
        })
        .unwrap();
    assert_eq!(ids, vec![line]);
    assert_eq!(document.object_ids(), vec![line]);
    let ObjectSnapshot::Path(path) = document.object(line).unwrap() else {
        panic!("a path");
    };
    assert!(path.closed);
    assert!(path.style.fill.enabled);
    assert!(!path.style.stroke.enabled);
    assert_eq!(path.style.stroke.width, Length::from_mm(2.0));
}

#[test]
fn ac17_a_clone_of_a_converted_original_is_unlinked_in_the_same_commit() {
    let document = Document::new(1);
    let (anchors, _) = square(1, 0.0, 10.0);
    let original = document.create_path(&anchors, true);
    let clone = document.linked_clone(&[original], &[]).unwrap()[0];
    let (before, _) = commits(&document);
    let fresh: Vec<AnchorId> = (100..104).map(|n| AnchorId::new(9, n)).collect();
    document
        .stroke_to_path(&StrokeToPath {
            unlinks: vec![CopySource {
                id: clone,
                anchor_ids: fresh,
            }],
            outlines: vec![StrokeOutlineWrite {
                subject: StrokeSubject::Object(original),
                outline: vec![square(20, -1.0, 12.0), square(30, 1.0, 8.0)],
            }],
            ..StrokeToPath::default()
        })
        .unwrap();
    assert_eq!(commits(&document).0, before + 1);
    let ObjectSnapshot::Path(unlinked) = document.object(clone).unwrap() else {
        panic!("the clone became a path");
    };
    assert!(unlinked.style.stroke.enabled, "it keeps its look");
}

#[test]
fn a_refused_write_changes_nothing() {
    let document = Document::new(1);
    let shape = rect(&document, 0.0);
    let (before, _) = commits(&document);
    let twice = square(1, 0.0, 2.0);
    let refused = document.stroke_to_path(&StrokeToPath {
        outlines: vec![StrokeOutlineWrite {
            subject: StrokeSubject::Object(shape),
            outline: vec![twice.clone(), twice],
        }],
        ..StrokeToPath::default()
    });
    assert_eq!(refused, Err(ObjectEditError::AnchorIds));
    assert_eq!(commits(&document).0, before);
    assert!(document.object(shape).unwrap().style().stroke.enabled);
}
