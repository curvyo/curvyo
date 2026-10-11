//! The document half of Split, Break apart and Cut (`specs/0048-split-compound-path` criteria 5,
//! 6, 8, 11 and 12, `specs/0036-cut-at-crossings` criteria 6 and 7): one commit with the caller's
//! label, pieces consecutive at their source's place with its whole style, and a refusal that
//! changes nothing.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use std::ops::ControlFlow;

use curvyo_document_core::{
    AnchorId, Angle, Color, Document, Length, NewAnchor, NodeId, ObjectEditError, PathSnapshot,
    PieceFill, Point, RectBounds, StyleEdit,
};
use loro::LoroDoc;

type Outline = (Vec<NewAnchor>, bool);

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn id(n: u64) -> AnchorId {
    AnchorId::new(9, n)
}

fn square(first: u64, x: f64, y: f64, side: f64) -> Outline {
    let corners = [(x, y), (x + side, y), (x + side, y + side), (x, y + side)];
    let anchors = corners
        .into_iter()
        .enumerate()
        .map(|(k, (cx, cy))| NewAnchor::corner(id(first + k as u64), pt(cx, cy)))
        .collect();
    (anchors, true)
}

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: pt(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn path_of(document: &Document, node: NodeId) -> PathSnapshot {
    document.path(node).expect("a path")
}

fn labels(document: &Document) -> Vec<String> {
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
    out.into_iter().map(|(_, s)| s).collect()
}

/// A compound path of two outlines made through the one replace command.
fn compound(document: &Document, first: u64) -> NodeId {
    let base = rect(document, 0.0);
    document
        .replace_with_path(
            &[base],
            base,
            &[
                square(first, 0.0, 0.0, 40.0),
                square(first + 10, 10.0, 10.0, 20.0),
            ],
            "combine_paths",
        )
        .unwrap()
}

/// Pieces replace the compound at its place, in the order given, with its whole style, in one
/// commit with the caller's label.
#[test]
fn replace_with_pieces_writes_consecutive_pieces_with_the_compounds_style_in_one_commit() {
    let document = Document::new(1);
    let below = rect(&document, 100.0);
    let whole = compound(&document, 100);
    let above = rect(&document, 200.0);
    document
        .edit_style(&[whole], &StyleEdit::StrokeWidth(Length::from_mm(3.0)))
        .unwrap();
    let before = labels(&document).len();

    let ring = vec![square(500, 0.0, 0.0, 40.0), square(510, 10.0, 10.0, 20.0)];
    let island = vec![square(520, 15.0, 15.0, 10.0)];
    let created = document
        .replace_with_pieces(
            &[whole],
            &[(whole, ring.clone()), (whole, island.clone())],
            PieceFill::Keep,
            "break_apart",
        )
        .unwrap();

    assert_eq!(created.len(), 2);
    assert_eq!(labels(&document).len(), before + 1, "one commit");
    assert_eq!(labels(&document).last().unwrap(), "break_apart");
    assert_eq!(
        document.object_ids(),
        vec![below, created[0], created[1], above],
        "consecutive at the compound's place"
    );
    assert!(document.object(whole).is_none());
    let first = path_of(&document, created[0]);
    let second = path_of(&document, created[1]);
    assert!(first.is_compound() && !second.is_compound());
    assert_eq!(first.style.stroke.width.as_mm(), 3.0);
    assert_eq!(second.style.stroke.width.as_mm(), 3.0);
    assert_eq!(second.rotation, Angle::from_radians(0.0));
    let expected: Vec<_> = island.into_iter().flat_map(|(a, _)| a).collect();
    assert_eq!(second.all_anchors().copied().collect::<Vec<_>>(), expected);
}

/// Several sources: each is replaced at its own place, one commit.
#[test]
fn several_compounds_are_written_in_one_commit() {
    let document = Document::new(1);
    let a = compound(&document, 100);
    let between = rect(&document, 50.0);
    let b = compound(&document, 200);
    let before = labels(&document).len();
    let created = document
        .replace_with_pieces(
            &[a, b],
            &[
                (a, vec![square(600, 0.0, 0.0, 5.0)]),
                (a, vec![square(610, 9.0, 9.0, 5.0)]),
                (b, vec![square(620, 0.0, 0.0, 5.0)]),
                (b, vec![square(630, 9.0, 9.0, 5.0)]),
            ],
            PieceFill::Keep,
            "split",
        )
        .unwrap();
    assert_eq!(created.len(), 4);
    assert_eq!(labels(&document).len(), before + 1);
    assert_eq!(
        document.object_ids(),
        vec![created[0], created[1], between, created[2], created[3]]
    );
}

/// A refusal writes nothing: same objects, no commit.
#[test]
fn a_refusal_changes_nothing() {
    let document = Document::new(1);
    let whole = compound(&document, 100);
    let other = rect(&document, 60.0);
    let ghost = {
        let doomed = rect(&document, 99.0);
        document.delete_objects(&[doomed]).unwrap();
        doomed
    };
    let _kept = document.create_path(&[NewAnchor::corner(id(800), pt(0.0, 0.0))], false);
    let ids = document.object_ids();
    let changes = labels(&document).len();
    let snapshot = (document.object(whole), document.object(other));
    let piece = || vec![square(700, 0.0, 0.0, 5.0)];
    let write = |removed: &[NodeId], pieces: &[(NodeId, Vec<Outline>)]| {
        document.replace_with_pieces(removed, pieces, PieceFill::Keep, "split")
    };

    let cases: Vec<(Result<Vec<NodeId>, ObjectEditError>, ObjectEditError)> = vec![
        (
            write(&[ghost], &[(whole, piece())]),
            ObjectEditError::NoSuchObject,
        ),
        (
            write(&[whole], &[(ghost, piece())]),
            ObjectEditError::NoSuchObject,
        ),
        (write(&[whole], &[]), ObjectEditError::NoOutlines),
        (
            write(&[whole], &[(whole, Vec::new())]),
            ObjectEditError::NoOutlines,
        ),
        (
            write(&[whole], &[(whole, vec![(Vec::new(), true)])]),
            ObjectEditError::NoOutlines,
        ),
        (
            write(&[whole], &[(whole, piece()), (whole, piece())]),
            ObjectEditError::AnchorIds,
        ),
        (
            write(&[whole], &[(whole, vec![square(800, 0.0, 0.0, 5.0)])]),
            ObjectEditError::AnchorIds,
        ),
    ];
    for (result, expected) in cases {
        assert_eq!(result, Err(expected));
    }
    assert_eq!(document.object_ids(), ids);
    assert_eq!(labels(&document).len(), changes, "no commit by a refusal");
    assert_eq!((document.object(whole), document.object(other)), snapshot);
}

/// The compound's own anchor ids may be reused by the pieces (the compound is deleted).
#[test]
fn pieces_may_reuse_the_deleted_compounds_anchor_ids() {
    let document = Document::new(1);
    let whole = compound(&document, 100);
    let created = document
        .replace_with_pieces(
            &[whole],
            &[(whole, vec![square(100, 0.0, 0.0, 40.0)])],
            PieceFill::Keep,
            "split",
        )
        .unwrap();
    assert_eq!(created.len(), 1);
}

/// Cut: `PieceFill::None` turns the fill off and keeps its colour and opacity; an open outline
/// is stored open; a source that is not removed stays.
#[test]
fn fill_none_keeps_the_stored_colour_and_open_pieces_stay_open() {
    let document = Document::new(1);
    let base = rect(&document, 0.0);
    let keep = rect(&document, 50.0);
    document
        .edit_style(
            &[base],
            &StyleEdit::FillColor(Color {
                r: 10,
                g: 20,
                b: 30,
            }),
        )
        .unwrap();
    document
        .edit_style(&[base], &StyleEdit::FillEnabled(true))
        .unwrap();
    let open = |first: u64| {
        let anchors = vec![
            NewAnchor::corner(id(first), pt(0.0, 0.0)),
            NewAnchor::corner(id(first + 1), pt(5.0, 5.0)),
        ];
        vec![(anchors, false)]
    };
    let created = document
        .replace_with_pieces(
            &[base],
            &[(base, open(900)), (base, open(910))],
            PieceFill::None,
            "cut_at_crossings",
        )
        .unwrap();
    assert_eq!(document.object_ids(), vec![created[0], created[1], keep]);
    assert_eq!(labels(&document).last().unwrap(), "cut_at_crossings");
    for piece in created {
        let path = path_of(&document, piece);
        assert!(!path.closed);
        assert!(!path.style.fill.enabled, "fill is None");
        assert_eq!(
            path.style.fill.color,
            Color {
                r: 10,
                g: 20,
                b: 30
            }
        );
    }
}
