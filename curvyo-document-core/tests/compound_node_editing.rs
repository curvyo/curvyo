//! Node commands on compound paths (`specs/0055-compound-path-node-editing` criteria 3 to 10 and
//! 12): every anchor command acts on the outline that holds the anchor, Delete removes outlines
//! and drops a compound path of one outline to an ordinary path, Join and Split work within and
//! across objects, and extend, connect and close grow one outline.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::ops::ControlFlow;

use curvyo_document_core::{
    AnchorId, AnchorKind, Color, Document, HandleSlot, NewAnchor, NodeId, PathEditError, PathEnd,
    PathGrowth, PathSnapshot, Point, StyleEdit, Vec2, pack, unpack,
};
use loro::LoroDoc;

fn id(n: u64) -> AnchorId {
    AnchorId::new(1, n)
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// An outline of corner nodes with ids `first`, `first + 1`, ...
fn outline(first: u64, points: &[(f64, f64)], closed: bool) -> (Vec<NewAnchor>, bool) {
    let anchors = points
        .iter()
        .zip(first..)
        .map(|(&(x, y), n)| NewAnchor::corner(id(n), pt(x, y)))
        .collect();
    (anchors, closed)
}

fn compound(document: &Document, outlines: &[(Vec<NewAnchor>, bool)]) -> NodeId {
    let base = document.create_path(
        &[
            NewAnchor::corner(id(9000 + document.object_ids().len() as u64), pt(0.0, 0.0)),
            NewAnchor::corner(id(9500 + document.object_ids().len() as u64), pt(1.0, 0.0)),
        ],
        false,
    );
    document
        .replace_with_path(&[base], base, outlines, "combine_paths")
        .unwrap()
}

/// A 100 mm square (counter-clockwise on screen) with a 20 mm square hole wound the other way.
fn ring(document: &Document) -> NodeId {
    compound(
        document,
        &[
            outline(
                100,
                &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0), (100.0, 0.0)],
                true,
            ),
            outline(
                200,
                &[(40.0, 40.0), (60.0, 40.0), (60.0, 60.0), (40.0, 60.0)],
                true,
            ),
        ],
    )
}

fn path(document: &Document, node: NodeId) -> PathSnapshot {
    document.path(node).expect("a path")
}

fn ids(anchors: &[NewAnchor]) -> Vec<AnchorId> {
    anchors.iter().map(|a| a.id).collect()
}

fn outlines(path: &PathSnapshot) -> Vec<(Vec<AnchorId>, bool)> {
    path.subpaths()
        .map(|o| (ids(o.anchors), o.closed))
        .collect()
}

fn labels(document: &Document) -> Vec<String> {
    let loro = LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    let frontiers: Vec<loro::ID> = loro.oplog_frontiers().iter().collect();
    let mut out: Vec<(u32, String)> = Vec::new();
    loro.travel_change_ancestors(&frontiers, &mut |m| {
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

/// The labels written since `before` labels were there.
fn new_labels(document: &Document, before: usize) -> Vec<String> {
    labels(document)[before..].to_vec()
}

// ============================================================ criterion 3

/// Criterion 3's test: the hole's four nodes dragged 50 mm right, together with a node of
/// another path, in one commit; the hole keeps its direction and ids, the outer square and the
/// other path's other node are unchanged.
#[test]
fn c3_a_drag_moves_nodes_of_an_outline_and_of_another_object_in_one_commit() {
    let document = Document::new(1);
    let node = ring(&document);
    let other = document.create_path(
        &[
            NewAnchor::corner(id(300), pt(200.0, 0.0)),
            NewAnchor::corner(id(301), pt(210.0, 0.0)),
        ],
        false,
    );
    let before = path(&document, node);
    let count = labels(&document).len();
    let mut moves: Vec<(NodeId, AnchorId, Point)> = before.extra_subpaths[0]
        .anchors
        .iter()
        .map(|a| (node, a.id, pt(a.point.x + 50.0, a.point.y)))
        .collect();
    moves.push((other, id(300), pt(200.0, 5.0)));
    document.move_anchors(&moves).unwrap();
    assert_eq!(new_labels(&document, count), vec!["move_anchors"]);
    let after = path(&document, node);
    assert_eq!(
        outlines(&after),
        outlines(&before),
        "ids, order and states kept"
    );
    assert_eq!(
        after.anchors, before.anchors,
        "the outer square is unchanged"
    );
    let xs: Vec<f64> = after.extra_subpaths[0]
        .anchors
        .iter()
        .map(|a| a.point.x)
        .collect();
    assert_eq!(xs, vec![90.0, 110.0, 110.0, 90.0]);
    assert_eq!(after.style, before.style);
    assert_eq!(path(&document, other).anchors[0].point, pt(200.0, 5.0));
}

/// Criterion 3: handle, insert, make line and curve, bend and kind conversion on an anchor of a
/// further outline change that outline only.
#[test]
fn c3_every_anchor_command_acts_on_the_outline_that_holds_the_anchor() {
    let document = Document::new(1);
    let node = ring(&document);
    let before = path(&document, node);

    document
        .convert_anchor_kind(&[(node, id(201))], AnchorKind::Symmetric)
        .unwrap();
    let after = path(&document, node);
    assert_eq!(
        after.extra_subpaths[0].anchors[1].kind,
        AnchorKind::Symmetric
    );
    assert_ne!(after.extra_subpaths[0].anchors[1].handle_out, Vec2::ZERO);

    document
        .set_handle(node, id(201), HandleSlot::Out, Vec2::new(0.0, 3.0))
        .unwrap();
    let hole = &path(&document, node).extra_subpaths[0].anchors[1];
    assert_eq!(hole.handle_out, Vec2::new(0.0, 3.0));
    assert_eq!(hole.handle_in, Vec2::new(0.0, -3.0), "mirrored");

    document.set_segment_line(node, id(201), id(202)).unwrap();
    assert_eq!(
        path(&document, node).extra_subpaths[0].anchors[1].handle_out,
        Vec2::ZERO
    );
    document.set_segment_curve(node, id(202), id(203)).unwrap();
    assert_ne!(
        path(&document, node).extra_subpaths[0].anchors[2].handle_out,
        Vec2::ZERO
    );
    // The closing segment of the hole: its last node to its first.
    document
        .bend_segment(
            node,
            id(203),
            id(200),
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, 1.0),
        )
        .unwrap();
    let hole = path(&document, node).extra_subpaths[0].clone();
    assert_eq!(hole.anchors[3].handle_out, Vec2::new(1.0, 1.0));
    assert_eq!(hole.anchors[0].handle_in, Vec2::new(-1.0, 1.0));

    document
        .insert_anchor(
            node,
            id(203),
            NewAnchor::corner(id(250), pt(40.0, 50.0)),
            Vec2::ZERO,
            Vec2::ZERO,
        )
        .unwrap();
    let after = path(&document, node);
    assert_eq!(
        ids(&after.extra_subpaths[0].anchors),
        vec![id(200), id(201), id(202), id(203), id(250)],
        "inserted after the last node of the closed hole"
    );
    assert_eq!(
        after.anchors, before.anchors,
        "the outer square is unchanged"
    );
}

/// A segment is two anchors of the same outline.
#[test]
fn a_segment_across_two_outlines_is_refused() {
    let document = Document::new(1);
    let node = ring(&document);
    let count = labels(&document).len();
    assert_eq!(
        document.set_segment_line(node, id(103), id(200)),
        Err(PathEditError::NotAnAdjacentSegment)
    );
    assert_eq!(
        document.bend_segment(node, id(103), id(200), Vec2::ZERO, Vec2::ZERO),
        Err(PathEditError::NotAnAdjacentSegment)
    );
    assert_eq!(labels(&document).len(), count, "nothing written");
}

// ============================================================ criterion 4

/// Criterion 4's test: three open lines of 2 nodes; one node of the first deleted leaves 2
/// outlines; one node of the second deleted leaves an ordinary open path with the same id.
#[test]
fn c4_deleting_outlines_drops_a_compound_path_to_an_ordinary_path() {
    let document = Document::new(1);
    let node = compound(
        &document,
        &[
            outline(10, &[(0.0, 0.0), (10.0, 0.0)], false),
            outline(20, &[(0.0, 10.0), (10.0, 10.0)], false),
            outline(30, &[(0.0, 20.0), (10.0, 20.0)], false),
        ],
    );
    document
        .edit_style(&[node], &StyleEdit::StrokeColor(Color { r: 9, g: 8, b: 7 }))
        .unwrap();
    let style = path(&document, node).style;
    let count = labels(&document).len();
    document.delete_anchors(&[(node, id(10))]);
    assert_eq!(new_labels(&document, count), vec!["delete_anchors"]);
    let after = path(&document, node);
    assert_eq!(
        outlines(&after),
        vec![(vec![id(20), id(21)], false), (vec![id(30), id(31)], false)],
        "the second outline is the first now, ids kept"
    );
    document.delete_anchors(&[(node, id(21))]);
    let after = path(&document, node);
    assert!(!after.is_compound(), "an ordinary path");
    assert_eq!(outlines(&after), vec![(vec![id(30), id(31)], false)]);
    assert_eq!(after.style, style);
    assert_eq!(document.object_ids(), vec![node]);
    document.delete_anchors(&[(node, id(30))]);
    assert!(
        document.path(node).is_none(),
        "the last outline gone: the object is removed"
    );
}

/// Criterion 4: a delete over several outlines and objects is one commit; a closed outline keeps
/// its state; an empty compound path is removed.
#[test]
fn c4_one_delete_over_outlines_and_objects_is_one_commit() {
    let document = Document::new(1);
    let node = ring(&document);
    let plain = document.create_path(
        &[
            NewAnchor::corner(id(300), pt(200.0, 0.0)),
            NewAnchor::corner(id(301), pt(210.0, 0.0)),
            NewAnchor::corner(id(302), pt(220.0, 0.0)),
        ],
        false,
    );
    let count = labels(&document).len();
    document.delete_anchors(&[(node, id(100)), (node, id(201)), (plain, id(301))]);
    assert_eq!(new_labels(&document, count), vec!["delete_anchors"]);
    let after = path(&document, node);
    assert_eq!(
        outlines(&after),
        vec![
            (vec![id(101), id(102), id(103)], true),
            (vec![id(200), id(202), id(203)], true)
        ]
    );
    assert_eq!(ids(&path(&document, plain).anchors), vec![id(300), id(302)]);
    let all: Vec<(NodeId, AnchorId)> = after.all_anchors().map(|a| (node, a.id)).collect();
    document.delete_anchors(&all);
    assert!(document.path(node).is_none());
}

/// Criterion 4: deleting the first outline of three promotes the second, with its closed state,
/// and keeps the third.
#[test]
fn c4_removing_the_first_outline_promotes_the_next() {
    let document = Document::new(1);
    let node = compound(
        &document,
        &[
            outline(10, &[(0.0, 0.0), (10.0, 0.0)], false),
            outline(20, &[(0.0, 10.0), (10.0, 10.0), (10.0, 20.0)], true),
            outline(30, &[(0.0, 30.0), (10.0, 30.0)], false),
        ],
    );
    document.delete_anchors(&[(node, id(10)), (node, id(11))]);
    let after = path(&document, node);
    assert_eq!(
        outlines(&after),
        vec![
            (vec![id(20), id(21), id(22)], true),
            (vec![id(30), id(31)], false)
        ]
    );
}

// ============================================================ criterion 5

/// Criterion 5 (a): both ends of one open outline with 3 nodes close it.
#[test]
fn c5a_the_two_ends_of_one_outline_close_it() {
    let document = Document::new(1);
    let node = compound(
        &document,
        &[
            outline(10, &[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0)], true),
            outline(20, &[(10.0, 10.0), (20.0, 10.0), (20.0, 20.0)], false),
        ],
    );
    assert!(document.check_join(node, id(20), node, id(22)));
    let count = labels(&document).len();
    assert_eq!(
        document.join_endpoints(node, id(20), node, id(22)),
        Ok((node, id(20)))
    );
    assert_eq!(new_labels(&document, count), vec!["join_endpoints"]);
    assert_eq!(
        outlines(&path(&document, node))[1],
        (vec![id(20), id(21)], true)
    );
    // Two ends of a 2-node outline are not offered.
    let lines = compound(
        &document,
        &[
            outline(40, &[(0.0, 0.0), (1.0, 0.0)], false),
            outline(50, &[(0.0, 1.0), (1.0, 1.0)], false),
        ],
    );
    assert!(!document.check_join(lines, id(50), lines, id(51)));
}

/// Criterion 5 (b): ends of two outlines of one object become one outline in the place of the
/// first-selected node's outline, by the order rule of `0006` criterion 9.
#[test]
fn c5b_two_outlines_of_one_object_become_one() {
    let document = Document::new(1);
    let node = compound(
        &document,
        &[
            outline(10, &[(0.0, 0.0), (10.0, 0.0)], false),
            outline(20, &[(0.0, 50.0), (50.0, 50.0), (50.0, 0.0)], true),
            outline(30, &[(20.0, 0.0), (30.0, 0.0)], false),
        ],
    );
    // The last node of the third outline joined to the first node of the first: the first-selected
    // outline is the third, so the joined outline stands in the third place.
    assert!(document.check_join(node, id(31), node, id(10)));
    let count = labels(&document).len();
    assert_eq!(
        document.join_endpoints(node, id(31), node, id(10)),
        Ok((node, id(31)))
    );
    assert_eq!(new_labels(&document, count), vec!["join_endpoints"]);
    let after = path(&document, node);
    assert_eq!(
        outlines(&after),
        vec![
            (vec![id(20), id(21), id(22)], true),
            (vec![id(30), id(31), id(11)], false)
        ],
        "the first outline left, the next one moved up, the joined one kept its order"
    );
}

/// Criterion 5 (c)'s test: a plate with a score line and a separate open line whose end the maker
/// joins to the score line's end give one compound path of 2 outlines with the plate's style.
#[test]
fn c5c_an_open_line_joined_to_a_score_line_moves_into_the_plate() {
    let document = Document::new(1);
    let plate = compound(
        &document,
        &[
            outline(
                10,
                &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0), (100.0, 0.0)],
                true,
            ),
            outline(20, &[(20.0, 50.0), (80.0, 50.0)], false),
        ],
    );
    document
        .edit_style(
            &[plate],
            &StyleEdit::StrokeColor(Color { r: 1, g: 2, b: 3 }),
        )
        .unwrap();
    let style = path(&document, plate).style;
    let line = document.create_path(
        &[
            NewAnchor::corner(id(30), pt(80.0, 50.0)),
            NewAnchor::corner(id(31), pt(80.0, 90.0)),
        ],
        false,
    );
    assert!(document.check_join(plate, id(21), line, id(30)));
    let count = labels(&document).len();
    document
        .join_endpoints(plate, id(21), line, id(30))
        .unwrap();
    assert_eq!(new_labels(&document, count), vec!["join_endpoints"]);
    assert!(document.path(line).is_none(), "the other object is removed");
    let after = path(&document, plate);
    assert_eq!(after.style, style);
    assert_eq!(
        outlines(&after),
        vec![
            (vec![id(10), id(11), id(12), id(13)], true),
            (vec![id(20), id(21), id(31)], false)
        ]
    );
}

/// Criterion 5 (c): when the first-selected node is on the ordinary path, that object survives
/// and the plate's other outlines are appended in their order, the survivor a compound path now.
#[test]
fn c5c_the_first_selected_object_survives_and_takes_the_other_outlines() {
    let document = Document::new(1);
    let plate = compound(
        &document,
        &[
            outline(
                10,
                &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0), (100.0, 0.0)],
                true,
            ),
            outline(20, &[(20.0, 50.0), (80.0, 50.0)], false),
            outline(25, &[(20.0, 70.0), (80.0, 70.0)], false),
        ],
    );
    let line = document.create_path(
        &[
            NewAnchor::corner(id(30), pt(80.0, 90.0)),
            NewAnchor::corner(id(31), pt(80.0, 50.0)),
        ],
        false,
    );
    document
        .join_endpoints(line, id(31), plate, id(21))
        .unwrap();
    assert!(document.path(plate).is_none());
    let after = path(&document, line);
    assert_eq!(
        outlines(&after),
        vec![
            (vec![id(30), id(31), id(20)], false),
            (vec![id(10), id(11), id(12), id(13)], true),
            (vec![id(25), id(26)], false)
        ]
    );
}

// ============================================================ criterion 6

/// Criterion 6: an interior node of an open outline splits it into two open outlines of the same
/// object, in its place, first part first; only the copy gets a fresh id.
#[test]
fn c6_an_open_outline_splits_into_two_outlines_of_the_object() {
    let document = Document::new(1);
    let node = compound(
        &document,
        &[
            outline(10, &[(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)], false),
            outline(20, &[(0.0, 50.0), (50.0, 50.0), (50.0, 0.0)], true),
        ],
    );
    assert!(document.check_split(node, id(11)));
    assert!(
        !document.check_split(node, id(10)),
        "an end node of an open outline"
    );
    let count = labels(&document).len();
    let (first, second) = document.split_at_anchor(node, id(11), id(99)).unwrap();
    assert_eq!(new_labels(&document, count), vec!["split_at_anchor"]);
    assert_eq!((first, second), ((node, id(11)), (node, id(99))));
    assert_eq!(document.object_ids(), vec![node], "one object");
    let after = path(&document, node);
    assert_eq!(
        outlines(&after),
        vec![
            (vec![id(10), id(11)], false),
            (vec![id(99), id(12)], false),
            (vec![id(20), id(21), id(22)], true)
        ]
    );
}

/// Criterion 6: a node of a closed outline opens that outline in place, as `0006` criterion 14.
#[test]
fn c6_a_node_of_a_closed_outline_opens_it_in_place() {
    let document = Document::new(1);
    let node = ring(&document);
    assert!(document.check_split(node, id(201)));
    document.split_at_anchor(node, id(201), id(99)).unwrap();
    let after = path(&document, node);
    assert_eq!(
        outlines(&after),
        vec![
            (vec![id(100), id(101), id(102), id(103)], true),
            (vec![id(99), id(202), id(203), id(200), id(201)], false)
        ]
    );
    assert_eq!(after.extra_subpaths[0].anchors[0].point, pt(60.0, 40.0));
}

/// Criterion 6 with the split outline not the first: the second part follows it.
#[test]
fn c6_the_second_part_of_a_further_outline_follows_it() {
    let document = Document::new(1);
    let node = compound(
        &document,
        &[
            outline(10, &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0)], true),
            outline(20, &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)], false),
            outline(30, &[(5.0, 5.0), (6.0, 5.0)], false),
        ],
    );
    document.split_at_anchor(node, id(21), id(99)).unwrap();
    assert_eq!(
        outlines(&path(&document, node)),
        vec![
            (vec![id(10), id(11), id(12)], true),
            (vec![id(20), id(21)], false),
            (vec![id(99), id(22)], false),
            (vec![id(30), id(31)], false)
        ]
    );
}

// ============================================================ criteria 7 to 9

fn growth(path: NodeId, end: PathEnd, at: AnchorId) -> PathGrowth {
    PathGrowth {
        path,
        end,
        at,
        added: Vec::new(),
        absorb: None,
        replace: Vec::new(),
        drop: Vec::new(),
    }
}

/// Criterion 7: two outlines of one compound path closed in one commit; the others unchanged.
#[test]
fn c7_close_paths_closes_outlines_of_a_compound_path_in_one_commit() {
    let document = Document::new(1);
    let node = compound(
        &document,
        &[
            outline(10, &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0)], true),
            outline(20, &[(10.0, 10.0), (20.0, 10.0), (20.0, 20.0)], false),
            outline(30, &[(30.0, 10.0), (40.0, 10.0), (40.0, 20.0)], false),
            outline(40, &[(50.0, 10.0), (60.0, 10.0), (60.0, 20.0)], false),
        ],
    );
    let count = labels(&document).len();
    document
        .close_paths(&[
            growth(node, PathEnd::Last, id(22)),
            growth(node, PathEnd::Last, id(42)),
        ])
        .unwrap();
    assert_eq!(new_labels(&document, count), vec!["close_path"]);
    let states: Vec<bool> = path(&document, node).subpaths().map(|o| o.closed).collect();
    assert_eq!(states, vec![true, true, false, true]);
    // Too few nodes: refused, nothing written.
    let lines = compound(
        &document,
        &[
            outline(50, &[(0.0, 0.0), (1.0, 0.0)], false),
            outline(60, &[(0.0, 1.0), (1.0, 1.0)], false),
        ],
    );
    let count = labels(&document).len();
    assert_eq!(
        document.close_paths(&[growth(lines, PathEnd::Last, id(61))]),
        Err(PathEditError::TooFewToClose)
    );
    assert_eq!(labels(&document).len(), count);
}

/// Criteria 7 and 10: two growths of one call that name the same outline are refused before the
/// first write, also when its ends coincide and the first growth drops the node the second names.
#[test]
fn c7_close_paths_refuses_the_same_outline_twice() {
    let document = Document::new(1);
    let node = compound(
        &document,
        &[
            outline(10, &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0)], true),
            outline(
                20,
                &[(10.0, 10.0), (20.0, 10.0), (20.0, 20.0), (10.0, 10.0)],
                false,
            ),
        ],
    );
    let mut merge = growth(node, PathEnd::Last, id(23));
    merge.drop = vec![id(23)];
    let count = labels(&document).len();
    assert_eq!(
        document.close_paths(&[merge.clone(), merge]),
        Err(PathEditError::NotExtendable)
    );
    assert_eq!(labels(&document).len(), count, "nothing written");
    assert!(!path(&document, node).extra_subpaths[0].closed);
}

/// Criterion 8: the Pen extends an open outline of a compound path at either end; the object keeps
/// its id and every other outline. `at` must be the end node at `end` of an open outline.
#[test]
fn c8_extending_an_outline_keeps_the_object_and_the_other_outlines() {
    let document = Document::new(1);
    let node = compound(
        &document,
        &[
            outline(10, &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0)], true),
            outline(20, &[(10.0, 10.0), (20.0, 10.0)], false),
        ],
    );
    let mut last = growth(node, PathEnd::Last, id(21));
    last.added = vec![NewAnchor::corner(id(90), pt(30.0, 10.0))];
    let count = labels(&document).len();
    document.extend_path(&last).unwrap();
    assert_eq!(new_labels(&document, count), vec!["extend_path"]);
    let mut first = growth(node, PathEnd::First, id(20));
    first.added = vec![NewAnchor::corner(id(91), pt(0.0, 10.0))];
    document.extend_path(&first).unwrap();
    assert_eq!(
        outlines(&path(&document, node)),
        vec![
            (vec![id(10), id(11), id(12)], true),
            (vec![id(91), id(20), id(21), id(90)], false)
        ]
    );
    // Not an end node, a closed outline, or the wrong end: refused.
    for (end, at) in [
        (PathEnd::Last, id(20)),
        (PathEnd::Last, id(12)),
        (PathEnd::First, id(90)),
    ] {
        let mut bad = growth(node, end, at);
        bad.added = vec![NewAnchor::corner(id(92), pt(5.0, 5.0))];
        assert_eq!(
            document.extend_path(&bad),
            Err(PathEditError::NotExtendable)
        );
    }
    let mut gone = growth(node, PathEnd::Last, id(777));
    gone.added = vec![NewAnchor::corner(id(92), pt(5.0, 5.0))];
    assert_eq!(
        document.extend_path(&gone),
        Err(PathEditError::NoSuchAnchor)
    );
}

/// Criterion 9: a continued ordinary path that ends on an open outline of a compound path absorbs
/// that outline, and the compound path's other outlines move into it in their order.
#[test]
fn c9_connecting_to_an_outline_moves_the_other_outlines_into_the_survivor() {
    let document = Document::new(1);
    let line = document.create_path(
        &[
            NewAnchor::corner(id(1), pt(0.0, 200.0)),
            NewAnchor::corner(id(2), pt(10.0, 200.0)),
        ],
        false,
    );
    let plate = compound(
        &document,
        &[
            outline(10, &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0)], true),
            outline(20, &[(20.0, 150.0), (30.0, 150.0)], false),
            outline(30, &[(40.0, 150.0), (50.0, 150.0)], false),
        ],
    );
    let mut connect = growth(line, PathEnd::Last, id(2));
    connect.added = vec![NewAnchor::corner(id(3), pt(20.0, 180.0))];
    connect.absorb = Some((plate, PathEnd::First, id(20)));
    let count = labels(&document).len();
    document.connect_paths(&connect).unwrap();
    assert_eq!(new_labels(&document, count), vec!["connect_paths"]);
    assert!(document.path(plate).is_none());
    assert_eq!(
        outlines(&path(&document, line)),
        vec![
            (vec![id(1), id(2), id(3), id(20), id(21)], false),
            (vec![id(10), id(11), id(12)], true),
            (vec![id(30), id(31)], false)
        ]
    );
    // A closed outline is no connect target.
    let other = compound(
        &document,
        &[
            outline(40, &[(0.0, 0.0), (0.0, 1.0), (1.0, 1.0)], true),
            outline(50, &[(5.0, 5.0), (6.0, 5.0)], false),
        ],
    );
    let mut bad = growth(line, PathEnd::Last, id(21));
    bad.absorb = Some((other, PathEnd::First, id(40)));
    assert_eq!(
        document.connect_paths(&bad),
        Err(PathEditError::NotExtendable)
    );
}

// ============================================================ criterion 12

/// Criterion 12: a compound path edited by every command reopens with the same outlines, states,
/// directions and anchor ids.
#[test]
fn c12_an_edited_compound_path_survives_save_and_reopen() {
    let document = Document::new(1);
    let node = ring(&document);
    let line = compound(
        &document,
        &[
            outline(10, &[(200.0, 0.0), (210.0, 0.0), (220.0, 0.0)], false),
            outline(20, &[(200.0, 10.0), (210.0, 10.0)], false),
        ],
    );
    document
        .move_anchors(&[(node, id(200), pt(41.0, 41.0))])
        .unwrap();
    document.split_at_anchor(node, id(202), id(99)).unwrap();
    document.split_at_anchor(line, id(11), id(98)).unwrap();
    document.delete_anchors(&[(line, id(20))]);
    let before = path(&document, node);
    let before_line = path(&document, line);
    let reopened = unpack(2, &pack(&document, "0.1.0").unwrap()).unwrap();
    assert_eq!(reopened.path(node).unwrap(), before);
    assert_eq!(reopened.path(line).unwrap(), before_line);
}

fn text_on(document: &Document, carrier: NodeId, outline: u32) -> NodeId {
    let font = curvyo_document_core::FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: curvyo_document_core::FontKind::Outline,
    };
    let text = document
        .create_text(
            "On",
            &font,
            curvyo_document_core::Length::from_mm(10.0),
            curvyo_document_core::Affine::IDENTITY,
            &curvyo_document_core::TextLayoutValues::DEFAULT,
        )
        .unwrap();
    document
        .put_text_on_path(
            text,
            curvyo_document_core::CarrierRef {
                node: carrier,
                outline,
            },
            &curvyo_document_core::OnPathValues::at(curvyo_document_core::Length::from_mm(0.0)),
            false,
        )
        .unwrap();
    text
}

fn carried_outline(document: &Document, text: NodeId) -> u32 {
    match document.read_objects().get(text) {
        Some(curvyo_document_core::ObjectSnapshot::Text(text)) => {
            text.on_path.as_ref().unwrap().carrier.outline
        }
        other => panic!("not a text: {other:?}"),
    }
}

/// A text on an outline of a compound path stays on that outline when node edits remove, insert
/// or merge other outlines before it (`specs/0080-text-on-path` carriers; `0055` criteria 4 to 6).
#[test]
fn a_text_stays_on_its_outline_when_outlines_before_it_change() {
    let document = Document::new(1);
    let plate = compound(
        &document,
        &[
            outline(100, &[(0.0, 0.0), (50.0, 0.0), (90.0, 0.0)], false),
            outline(200, &[(0.0, 10.0), (50.0, 10.0)], false),
            outline(300, &[(0.0, 20.0), (50.0, 20.0)], false),
            outline(400, &[(0.0, 30.0), (80.0, 30.0)], false),
        ],
    );
    let text = text_on(&document, plate, 3);
    // Split of outline 0 at its middle node inserts an outline before the text's: 3 -> 4.
    document.split_at_anchor(plate, id(101), id(150)).unwrap();
    assert_eq!(carried_outline(&document, text), 4);
    // Delete removes outline 2 (the old outline 1): 4 -> 3.
    document.delete_anchors(&[(plate, id(200))]);
    assert_eq!(carried_outline(&document, text), 3);
    // Join of outlines 0 and 2 (the old outline 2) in one object merges them: 3 -> 2.
    document
        .join_endpoints(plate, id(100), plate, id(300))
        .unwrap();
    assert_eq!(carried_outline(&document, text), 2);
    let path = path(&document, plate);
    let carried = path.subpaths().nth(2).unwrap();
    assert_eq!(carried.anchors[0].id, id(400));
}
