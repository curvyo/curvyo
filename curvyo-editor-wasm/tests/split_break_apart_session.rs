//! Split and Break apart in the editor session (`specs/0048-split-compound-path` criteria 1, 4,
//! 6, 8, 9, 10, 11, 12, 13): the new Break apart, the touch rule on a file from another program,
//! the labels, and the inverse of Combine.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    AnchorId, Document, EllipseFrame, Length, NewAnchor, NodeId, ObjectSnapshot, Point, pack,
    unpack,
};
use curvyo_editor_wasm::{BreakApartOutcome, CombineOutcome, Session, SplitOutcome, Tool};
use curvyo_ui_core::{BreakApartRefusal, SplitRefusal};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn disc(document: &Document, x: f64, y: f64, radius: f64) -> NodeId {
    document.create_ellipse(EllipseFrame {
        center: pt(x, y),
        rx: Length::from_mm(radius),
        ry: Length::from_mm(radius),
    })
}

fn session_of(document: &Document) -> Session {
    let mut session = Session::open(2, &pack(document, "0.1.0").unwrap()).unwrap();
    session.set_tool(Tool::Select);
    session.resize_viewport(1200.0, 800.0);
    session
}

fn select_all(session: &mut Session) {
    let from = pt(-200.0, -200.0);
    session.pointer_hover(from, false, false);
    session.pointer_down(from, false);
    session.pointer_hover(pt(400.0, 400.0), false, false);
    session.pointer_up(pt(400.0, 400.0), false, false);
}

fn doc(session: &Session) -> Document {
    unpack(3, &session.pack("0.1.0").unwrap()).unwrap()
}

fn paths(session: &Session) -> Vec<curvyo_document_core::PathSnapshot> {
    let document = doc(session);
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| match document.object(id) {
            Some(ObjectSnapshot::Path(path)) => Some(path),
            _ => None,
        })
        .collect()
}

fn commit_labels(session: &Session) -> Vec<String> {
    let loro = loro::LoroDoc::new();
    loro.import(&doc(session).export_loro_snapshot().unwrap())
        .unwrap();
    let ids: Vec<loro::ID> = loro.oplog_frontiers().iter().collect();
    let mut changes: Vec<(u32, String)> = Vec::new();
    loro.travel_change_ancestors(&ids, &mut |change| {
        changes.push((
            change.lamport,
            change
                .message
                .map(|m| m.split(';').next().unwrap_or_default().to_string())
                .unwrap_or_default(),
        ));
        std::ops::ControlFlow::Continue(())
    })
    .unwrap();
    changes.sort();
    changes.into_iter().map(|(_, label)| label).collect()
}

/// A ring with a disc in its hole (radii 20, 10, 5), combined into one compound path, selected.
fn ring_with_island() -> Session {
    let document = Document::new(1);
    let _ = disc(&document, 50.0, 50.0, 20.0);
    let _ = disc(&document, 50.0, 50.0, 10.0);
    let _ = disc(&document, 50.0, 50.0, 5.0);
    let mut session = session_of(&document);
    select_all(&mut session);
    assert!(matches!(
        session.apply_combine(),
        CombineOutcome::Applied { .. }
    ));
    session
}

/// Criteria 11 and 12 (E5): three closed paths, one commit labelled `break_apart`, all selected.
#[test]
fn break_apart_gives_one_closed_path_per_outline() {
    let mut session = ring_with_island();
    let before = commit_labels(&session);
    let sources = paths(&session);
    let originals: Vec<Vec<Point>> = sources[0]
        .subpaths()
        .map(|s| s.anchors.iter().map(|a| a.point).collect())
        .collect();

    let outcome = session.apply_break_apart();
    assert_eq!(
        outcome,
        BreakApartOutcome::Applied {
            compounds: 1,
            pieces: 3,
            with_holes: true,
            single_nodes_removed: 0,
        }
    );
    let labels = commit_labels(&session);
    assert_eq!(labels.len(), before.len() + 1, "one commit");
    assert_eq!(labels.last().unwrap(), "break_apart");
    assert_eq!(session.selected_object_count(), 3);
    assert_eq!(session.tool(), Tool::Select);
    let pieces = paths(&session);
    assert_eq!(pieces.len(), 3);
    for (piece, original) in pieces.iter().zip(&originals) {
        assert!(piece.closed && !piece.is_compound());
        let points: Vec<Point> = piece.anchors.iter().map(|a| a.point).collect();
        assert_eq!(&points, original, "nodes exactly as before");
    }
}

/// Criteria 7 and 11 (E1): a ring is refused by Split and broken into two by Break apart.
#[test]
fn a_ring_is_one_part_for_split_and_two_outlines_for_break_apart() {
    let document = Document::new(1);
    let _ = disc(&document, 50.0, 50.0, 20.0);
    let _ = disc(&document, 50.0, 50.0, 10.0);
    let mut session = session_of(&document);
    select_all(&mut session);
    let _ = session.apply_combine();
    let ids = doc(&session).object_ids();
    assert_eq!(
        session.apply_split(),
        SplitOutcome::Refused(SplitRefusal::OnePiece {
            compounds: 1,
            with_holes: true
        })
    );
    assert_eq!(doc(&session).object_ids(), ids, "same object, same ids");
    assert_eq!(
        session.apply_break_apart(),
        BreakApartOutcome::Applied {
            compounds: 1,
            pieces: 2,
            with_holes: true,
            single_nodes_removed: 0,
        }
    );
}

/// Criterion 13: no compound path, or a group, is refused and writes nothing.
#[test]
fn break_apart_without_a_compound_path_is_refused() {
    let document = Document::new(1);
    let _ = disc(&document, 50.0, 50.0, 20.0);
    let mut session = session_of(&document);
    select_all(&mut session);
    let labels = commit_labels(&session);
    assert_eq!(
        session.apply_break_apart(),
        BreakApartOutcome::Refused(BreakApartRefusal::NoCompound)
    );
    assert_eq!(commit_labels(&session), labels);
    session.set_tool(Tool::Node);
    assert_eq!(session.apply_break_apart(), BreakApartOutcome::Ignored);
}

/// Criterion 10: Combine, Split, Combine again: the same outlines, node for node.
#[test]
fn split_is_the_inverse_of_combine() {
    let document = Document::new(1);
    let _ = disc(&document, 20.0, 50.0, 10.0);
    let _ = disc(&document, 80.0, 50.0, 10.0);
    let _ = disc(&document, 140.0, 50.0, 10.0);
    let mut session = session_of(&document);
    select_all(&mut session);
    let _ = session.apply_combine();
    let combined: Vec<Vec<Point>> = paths(&session)[0]
        .subpaths()
        .map(|s| s.anchors.iter().map(|a| a.point).collect())
        .collect();
    assert_eq!(
        session.apply_split(),
        SplitOutcome::Applied {
            compounds: 1,
            pieces: 3,
            with_holes: false,
            left_alone: 0
        }
    );
    let split: Vec<Vec<Point>> = paths(&session)
        .iter()
        .map(|p| p.anchors.iter().map(|a| a.point).collect())
        .collect();
    assert_eq!(split, combined);
    assert!(matches!(
        session.apply_combine(),
        CombineOutcome::Applied { operands: 3, .. }
    ));
    let again: Vec<Vec<Point>> = paths(&session)[0]
        .subpaths()
        .map(|s| s.anchors.iter().map(|a| a.point).collect())
        .collect();
    assert_eq!(again, combined);
}

/// Criterion 4, golden file: a compound path of two overlapping squares, as another program
/// writes it (Combine refuses it), is one part. Regenerate with
/// `CURVYO_WRITE_FIXTURES=1 cargo test -p curvyo-editor-wasm --test split_break_apart_session`.
#[test]
fn the_golden_overlapping_squares_are_one_part_but_two_outlines() {
    const GOLDEN: &str = "compound_overlapping_squares.curvyo";
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(GOLDEN);
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        let document = Document::new(1);
        let base = disc(&document, 500.0, 500.0, 1.0);
        let square = |first: u64, x: f64, y: f64| {
            let corners = [(x, y), (x + 10.0, y), (x + 10.0, y + 10.0), (x, y + 10.0)];
            let anchors: Vec<NewAnchor> = corners
                .iter()
                .enumerate()
                .map(|(k, &(cx, cy))| {
                    NewAnchor::corner(AnchorId::new(8, first + k as u64), pt(cx, cy))
                })
                .collect();
            (anchors, true)
        };
        document
            .replace_with_path(
                &[base],
                base,
                &[square(10, 0.0, 0.0), square(20, 8.0, -5.0)],
                "combine_paths",
            )
            .unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, pack(&document, "0.1.0").unwrap()).unwrap();
    }
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut session = Session::open(2, &bytes).unwrap();
    session.set_tool(Tool::Select);
    session.resize_viewport(1200.0, 800.0);
    select_all(&mut session);
    let ids = doc(&session).object_ids();
    assert_eq!(ids.len(), 1);
    assert_eq!(
        session.apply_split(),
        SplitOutcome::Refused(SplitRefusal::OnePiece {
            compounds: 1,
            with_holes: false
        })
    );
    assert_eq!(doc(&session).object_ids(), ids);
    assert_eq!(
        session.apply_break_apart(),
        BreakApartOutcome::Applied {
            compounds: 1,
            pieces: 2,
            with_holes: false,
            single_nodes_removed: 0,
        }
    );
}
