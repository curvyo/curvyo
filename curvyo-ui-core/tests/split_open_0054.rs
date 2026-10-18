//! Split and Break apart with open, touching and crossing outlines
//! (`specs/0054-combine-open-paths` criteria 15 to 19), on the plans.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation
)]

use curvyo_document_core::{AnchorId, Document, NewAnchor, NodeId, ObjectSnapshot, Point};
use curvyo_ui_core::{
    AnchorIdMinter, ObjectSelection, SplitRefusal, plan_break_apart, plan_combine, plan_split,
};

type Outlines = Vec<(Vec<NewAnchor>, bool)>;

fn outline(first: u64, points: &[(f64, f64)], closed: bool) -> (Vec<NewAnchor>, bool) {
    let anchors = points
        .iter()
        .enumerate()
        .map(|(k, &(x, y))| NewAnchor::corner(AnchorId::new(1, first + k as u64), Point::new(x, y)))
        .collect();
    (anchors, closed)
}

fn square(
    first: u64,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    reversed: bool,
) -> (Vec<NewAnchor>, bool) {
    let mut corners = vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    if reversed {
        corners.reverse();
    }
    outline(first, &corners, true)
}

fn line(first: u64, from: (f64, f64), to: (f64, f64)) -> (Vec<NewAnchor>, bool) {
    outline(first, &[from, to], false)
}

/// A document holding one compound path with `outlines`.
fn compound(outlines: &Outlines) -> Document {
    let document = Document::new(1);
    let base = document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(1.0, 0.0)),
        ],
        false,
    );
    document
        .replace_with_path(&[base], base, outlines, "test")
        .unwrap();
    document
}

fn objects(document: &Document) -> Vec<ObjectSnapshot> {
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.object(id))
        .collect()
}

fn all(document: &Document) -> ObjectSelection {
    let mut selection = ObjectSelection::new();
    for id in document.object_ids() {
        selection.add(id);
    }
    selection
}

/// The pieces of a Split as (outline count, open outline count).
fn split_pieces(document: &Document) -> Result<Vec<(usize, usize)>, SplitRefusal> {
    let plan = plan_split(
        &objects(document),
        &all(document),
        &mut AnchorIdMinter::new(9),
    )?;
    Ok(plan
        .pieces
        .iter()
        .map(|(_, piece)| (piece.len(), piece.iter().filter(|(_, c)| !c).count()))
        .collect())
}

/// Criterion 15, first test: a plate with a hole and an engraving line inside the plate body, and
/// a far line: the plate with its hole and its line, and the far line.
#[test]
fn c15_a_line_on_the_plate_body_stays_with_the_plate() {
    let document = compound(&vec![
        square(10, 0.0, 0.0, 100.0, 100.0, false),
        square(20, 40.0, 40.0, 60.0, 60.0, true),
        line(30, (10.0, 10.0), (30.0, 10.0)),
        line(40, (200.0, 0.0), (220.0, 0.0)),
    ]);
    assert_eq!(split_pieces(&document).unwrap(), vec![(3, 1), (1, 1)]);
}

/// Criterion 15, second test: two crossing squares and a far circle: the two squares and the
/// circle.
#[test]
fn c15_crossing_squares_are_one_part() {
    let document = compound(&vec![
        square(10, 0.0, 0.0, 20.0, 20.0, false),
        square(20, 10.0, 10.0, 30.0, 30.0, false),
        square(30, 100.0, 0.0, 110.0, 10.0, false),
    ]);
    assert_eq!(split_pieces(&document).unwrap(), vec![(2, 0), (1, 0)]);
}

/// Rule (a): a line touching two separate squares joins them into one part; lines touching each
/// other end in one part; a line in a hole is not on the painted area and stands alone.
#[test]
fn c15_touch_joins_and_holes_are_not_painted() {
    let bridged = compound(&vec![
        square(10, 0.0, 0.0, 10.0, 10.0, false),
        square(20, 50.0, 0.0, 60.0, 10.0, false),
        line(30, (5.0, 5.0), (55.0, 5.0)),
        square(40, 200.0, 0.0, 210.0, 10.0, false),
    ]);
    assert_eq!(split_pieces(&bridged).unwrap(), vec![(3, 1), (1, 0)]);

    let crossing_lines = compound(&vec![
        line(10, (0.0, 5.0), (10.0, 5.0)),
        line(20, (5.0, 0.0), (5.0, 10.0)),
        square(30, 100.0, 0.0, 110.0, 10.0, false),
    ]);
    assert_eq!(split_pieces(&crossing_lines).unwrap(), vec![(1, 0), (2, 2)]);

    let in_hole = compound(&vec![
        square(10, 0.0, 0.0, 100.0, 100.0, false),
        square(20, 20.0, 20.0, 80.0, 80.0, true),
        line(30, (40.0, 50.0), (60.0, 50.0)),
    ]);
    assert_eq!(split_pieces(&in_hole).unwrap(), vec![(2, 0), (1, 1)]);
}

/// Criterion 16: a part of one open outline becomes an open path; nodes and direction are kept.
#[test]
fn c16_pieces_keep_their_state_nodes_and_direction() {
    let far = line(40, (220.0, 0.0), (200.0, 0.0));
    let document = compound(&vec![square(10, 0.0, 0.0, 10.0, 10.0, false), far.clone()]);
    let plan = plan_split(
        &objects(&document),
        &all(&document),
        &mut AnchorIdMinter::new(9),
    )
    .unwrap();
    let piece = &plan.pieces[1].1;
    assert_eq!(piece.len(), 1);
    assert!(!piece[0].1);
    let points: Vec<Point> = piece[0].0.iter().map(|a| a.point).collect();
    assert_eq!(points, vec![Point::new(220.0, 0.0), Point::new(200.0, 0.0)]);
}

/// Criterion 17: one part without a hole is refused with the no-hole wording; with a hole, with
/// the `0048` wording.
#[test]
fn c17_one_piece_says_whether_it_has_holes() {
    let scored = compound(&vec![
        square(10, 0.0, 0.0, 100.0, 100.0, false),
        line(20, (-10.0, 50.0), (110.0, 50.0)),
    ]);
    assert_eq!(
        split_pieces(&scored),
        Err(SplitRefusal::OnePiece {
            compounds: 1,
            with_holes: false
        })
    );
    let ring = compound(&vec![
        square(10, 0.0, 0.0, 100.0, 100.0, false),
        square(20, 40.0, 40.0, 60.0, 60.0, true),
        line(30, (-10.0, 50.0), (30.0, 50.0)),
    ]);
    assert_eq!(
        split_pieces(&ring),
        Err(SplitRefusal::OnePiece {
            compounds: 1,
            with_holes: true
        })
    );
}

/// Criterion 18: a closed square, a square crossing it and two open lines give 2 closed and 2
/// open paths; a single-node outline is dropped and counted.
#[test]
fn c18_break_apart_gives_every_outline_back() {
    let document = compound(&vec![
        square(10, 0.0, 0.0, 20.0, 20.0, false),
        square(20, 10.0, 10.0, 30.0, 30.0, false),
        line(30, (0.0, 50.0), (10.0, 50.0)),
        line(40, (0.0, 60.0), (10.0, 60.0)),
        outline(50, &[(5.0, 5.0)], false),
    ]);
    let plan = plan_break_apart(
        &objects(&document),
        &all(&document),
        &mut AnchorIdMinter::new(9),
    )
    .unwrap();
    let closed: Vec<bool> = plan.pieces.iter().map(|(_, piece)| piece[0].1).collect();
    assert_eq!(closed, vec![true, true, false, false]);
    assert_eq!(plan.single_nodes_removed, 1);
    assert!(!plan.with_holes);
}

/// Criterion 19: Combine and then Break apart give every open path back node for node and in its
/// direction.
#[test]
fn c19_open_paths_survive_combine_and_break_apart() {
    let document = Document::new(1);
    let (a, _) = line(10, (0.0, 0.0), (10.0, 0.0));
    let (b, _) = outline(20, &[(30.0, 30.0), (20.0, 20.0), (25.0, 10.0)], false);
    let _ = document.create_path(&a, false);
    let _ = document.create_path(&b, false);
    let _ = document.create_path(&square(30, 50.0, 0.0, 60.0, 10.0, false).0, true);
    let combined = plan_combine(
        &objects(&document),
        &all(&document),
        &mut AnchorIdMinter::new(5),
    )
    .unwrap();
    let ids: Vec<NodeId> = document.object_ids();
    let result = document
        .replace_with_path(&ids, combined.base, &combined.outlines, "combine_paths")
        .unwrap();
    let mut selection = ObjectSelection::new();
    selection.add(result);
    let back =
        plan_break_apart(&objects(&document), &selection, &mut AnchorIdMinter::new(6)).unwrap();
    let points = |piece: &(Vec<NewAnchor>, bool)| -> Vec<Point> {
        piece.0.iter().map(|a| a.point).collect()
    };
    assert_eq!(
        points(&back.pieces[0].1[0]),
        a.iter().map(|x| x.point).collect::<Vec<_>>()
    );
    assert_eq!(
        points(&back.pieces[1].1[0]),
        b.iter().map(|x| x.point).collect::<Vec<_>>()
    );
    assert!(!back.pieces[0].1[0].1 && !back.pieces[1].1[0].1);
}
