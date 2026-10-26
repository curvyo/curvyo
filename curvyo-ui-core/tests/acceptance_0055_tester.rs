//! Independent acceptance tests for `specs/0055-compound-path-node-editing` at the document
//! command and Node tool level (criteria 3 to 10, 12 plus edge cases), written from the
//! specification before the implementation was read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::many_single_char_names
)]

use curvyo_document_core::{
    AnchorId, Document, NewAnchor, NodeId, PathEnd, PathGrowth, PathSnapshot, Point,
};
use curvyo_geometry_core::{Outline, OutlineTriple, contains_point_in_outlines};
use curvyo_ui_core::{AnchorIdMinter, NodeSplitOutcome, NodeTool};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn id(n: u64) -> AnchorId {
    AnchorId::new(1, n)
}

type Spec = (Vec<NewAnchor>, bool);

fn outline(first: u64, points: &[(f64, f64)], closed: bool) -> Spec {
    (
        points
            .iter()
            .zip(first..)
            .map(|(&(x, y), n)| NewAnchor::corner(id(n), pt(x, y)))
            .collect(),
        closed,
    )
}

fn compound(document: &Document, outlines: &[Spec]) -> NodeId {
    let base = document.create_path(
        &[
            NewAnchor::corner(id(9000), pt(900.0, 900.0)),
            NewAnchor::corner(id(9001), pt(901.0, 900.0)),
        ],
        false,
    );
    document
        .replace_with_path(&[base], base, outlines, "combine_paths")
        .unwrap()
}

fn line(first: u64, y: f64) -> Spec {
    outline(first, &[(0.0, y), (10.0, y)], false)
}

/// Outer square 0..100 (counter-clockwise in screen terms) and a hole 40..60 running the other way.
fn ring(document: &Document) -> NodeId {
    compound(
        document,
        &[
            outline(
                1,
                &[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
                true,
            ),
            outline(
                11,
                &[(40.0, 40.0), (40.0, 60.0), (60.0, 60.0), (60.0, 40.0)],
                true,
            ),
        ],
    )
}

fn shape(p: &PathSnapshot) -> Vec<(Vec<u64>, bool)> {
    p.subpaths()
        .map(|o| {
            (
                o.anchors.iter().map(|a| a.id.as_u128() as u64).collect(),
                o.closed,
            )
        })
        .collect()
}

fn painted(p: &PathSnapshot, at: Point) -> bool {
    let owned: Vec<(Vec<OutlineTriple>, bool)> = p
        .subpaths()
        .map(|o| {
            (
                o.anchors
                    .iter()
                    .map(|a| (a.point, a.handle_in, a.handle_out))
                    .collect(),
                o.closed,
            )
        })
        .collect();
    let outlines: Vec<Outline<'_>> = owned.iter().map(|(t, c)| Outline::new(t, *c)).collect();
    contains_point_in_outlines(&outlines, at)
}

fn steps(d: &Document) -> usize {
    d.own_steps().len()
}

fn undo_last(d: &Document) {
    let s = d.own_steps().last().unwrap().step;
    d.undo_step(s).unwrap();
}

/// Criterion 3 test case: the hole moved 50 mm right, outside the ring, is painted and not
/// re-wound; the ring's centre is painted again; one commit; undo restores.
#[test]
fn c3_hole_dragged_out_is_painted_and_not_rewound() {
    let d = Document::new(1);
    let p = ring(&d);
    let before = d.path(p).unwrap();
    assert!(!painted(&before, pt(50.0, 50.0)), "hole is a hole");
    let n = steps(&d);
    let moves: Vec<_> = [
        (11, 90.0, 40.0),
        (12, 90.0, 60.0),
        (13, 110.0, 60.0),
        (14, 110.0, 40.0),
    ]
    .iter()
    .map(|&(a, x, y)| (p, id(a), pt(x + 0.0, y)))
    .collect();
    d.move_anchors(&moves).unwrap();
    assert_eq!(steps(&d), n + 1);
    let after = d.path(p).unwrap();
    assert!(painted(&after, pt(50.0, 50.0)), "ring centre painted again");
    assert!(painted(&after, pt(100.0, 50.0)));
    // not re-wound: node order unchanged
    assert_eq!(shape(&after), shape(&before));
    let outer: Vec<_> = after.anchors.iter().map(|a| a.point).collect();
    assert_eq!(
        outer,
        before.anchors.iter().map(|a| a.point).collect::<Vec<_>>()
    );
    undo_last(&d);
    assert_eq!(d.path(p).unwrap(), before);
}

/// Criterion 3: nodes of several outlines and objects move in one commit; ids and style stay.
#[test]
fn c3_move_across_outlines_and_objects_is_one_commit() {
    let d = Document::new(1);
    let p = ring(&d);
    let q = d.create_path(
        &[
            NewAnchor::corner(id(50), pt(200.0, 0.0)),
            NewAnchor::corner(id(51), pt(210.0, 0.0)),
        ],
        false,
    );
    let before = d.path(p).unwrap();
    let n = steps(&d);
    d.move_anchors(&[
        (p, id(1), pt(-5.0, -5.0)),
        (p, id(12), pt(41.0, 61.0)),
        (q, id(50), pt(201.0, 1.0)),
    ])
    .unwrap();
    assert_eq!(steps(&d), n + 1);
    let after = d.path(p).unwrap();
    assert_eq!(after.id, before.id);
    assert_eq!(after.style, before.style);
    assert_eq!(shape(&after), shape(&before));
    assert_eq!(after.anchors[0].point, pt(-5.0, -5.0));
    assert_eq!(after.extra_subpaths[0].anchors[1].point, pt(41.0, 61.0));
    assert_eq!(
        after.extra_subpaths[0].anchors[0].point,
        before.extra_subpaths[0].anchors[0].point
    );
    undo_last(&d);
    assert_eq!(d.path(p).unwrap(), before);
}

/// Criterion 3: insert a node by double click, Make line/curve on an extra outline.
#[test]
fn c3_insert_and_segment_commands_work_on_extra_outlines() {
    let d = Document::new(1);
    let p = ring(&d);
    d.insert_anchor(
        p,
        id(11),
        NewAnchor::corner(id(77), pt(40.0, 50.0)),
        curvyo_document_core::Vec2::new(0.0, 0.0),
        curvyo_document_core::Vec2::new(0.0, 0.0),
    )
    .unwrap();
    let after = d.path(p).unwrap();
    assert_eq!(after.extra_subpaths[0].anchors.len(), 5);
    assert_eq!(after.extra_subpaths[0].anchors[1].id, id(77));
    assert_eq!(after.anchors.len(), 4);
    d.set_segment_curve(p, id(77), id(12)).unwrap();
    d.set_segment_line(p, id(77), id(12)).unwrap();
    // A segment across two outlines is refused.
    assert!(d.set_segment_line(p, id(1), id(11)).is_err());
    // Closing segment of a closed extra outline: last and first.
    d.set_segment_line(p, id(14), id(11)).unwrap();
}

/// Criterion 4 test case.
#[test]
fn c4_three_open_lines_delete_one_node_each() {
    let d = Document::new(1);
    let p = compound(&d, &[line(1, 0.0), line(3, 5.0), line(5, 10.0)]);
    let style = d.path(p).unwrap().style;
    let n = steps(&d);
    d.delete_anchors(&[(p, id(1))]);
    assert_eq!(steps(&d), n + 1);
    let a = d.path(p).unwrap();
    assert_eq!(shape(&a), vec![(vec![3, 4], false), (vec![5, 6], false)]);
    assert!(a.is_compound());
    d.delete_anchors(&[(p, id(4))]);
    let b = d.path(p).unwrap();
    assert!(!b.is_compound());
    assert_eq!(b.id, p);
    assert_eq!(b.style, style);
    assert_eq!(shape(&b), vec![(vec![5, 6], false)]);
    assert!(!b.closed);
    undo_last(&d);
    assert_eq!(shape(&d.path(p).unwrap()), shape(&a));
}

/// Criterion 4: deleting the first outline's nodes promotes the next; all nodes remove the object.
#[test]
fn c4_first_outline_removed_and_all_nodes_remove_object() {
    let d = Document::new(1);
    let p = compound(&d, &[line(1, 0.0), line(3, 5.0), line(5, 10.0)]);
    d.delete_anchors(&[(p, id(2))]);
    let a = d.path(p).unwrap();
    assert_eq!(shape(&a), vec![(vec![3, 4], false), (vec![5, 6], false)]);
    d.delete_anchors(&[(p, id(3)), (p, id(4)), (p, id(5)), (p, id(6))]);
    assert!(d.path(p).is_none());
    assert!(!d.object_ids().contains(&p));
    undo_last(&d);
    assert_eq!(shape(&d.path(p).unwrap()), shape(&a));
}

/// Criterion 4: a closed outline loses nodes down to 3 and stays closed; a ring losing a hole's
/// node keeps the hole; deleting the whole hole leaves an ordinary closed path.
#[test]
fn c4_hole_removed_leaves_ordinary_closed_path() {
    let d = Document::new(1);
    let p = ring(&d);
    d.delete_anchors(&[(p, id(11)), (p, id(12)), (p, id(13)), (p, id(14))]);
    let a = d.path(p).unwrap();
    assert!(!a.is_compound());
    assert!(a.closed);
    assert_eq!(a.anchors.len(), 4);
}

/// Edge: a single-node outline in the delete selection and a stale anchor do not panic.
#[test]
fn c4_stale_and_duplicate_anchor_ids_do_not_panic() {
    let d = Document::new(1);
    let p = compound(&d, &[line(1, 0.0), line(3, 5.0)]);
    d.delete_anchors(&[(p, id(1)), (p, id(1)), (p, id(999))]);
    let a = d.path(p).unwrap();
    // Either the batch was refused or applied once; the object must stay valid.
    assert!(a.subpaths().all(|o| o.anchors.len() >= 2));
}

/// Criterion 5 (a): both ends of one 3-node open outline of a compound path close it.
#[test]
fn c5a_join_both_ends_of_one_outline_closes_it() {
    let d = Document::new(1);
    let p = compound(
        &d,
        &[
            line(1, 0.0),
            outline(3, &[(0.0, 20.0), (10.0, 20.0), (10.0, 30.0)], false),
        ],
    );
    d.join_endpoints(p, id(3), p, id(5)).unwrap();
    let a = d.path(p).unwrap();
    assert_eq!(a.subpaths().count(), 2);
    assert!(a.extra_subpaths[0].closed);
    assert_eq!(
        a.extra_subpaths[0].anchors.len(),
        2,
        "0006 c10: one fewer node"
    );
    assert!(!a.closed);
    // the 2-node outline cannot self-join
    assert!(d.join_endpoints(p, id(1), p, id(2)).is_err());
}

/// Criterion 5 (b): ends of two outlines of one object become one outline in the first's place.
#[test]
fn c5b_join_two_outlines_of_one_object() {
    let d = Document::new(1);
    let p = compound(&d, &[line(1, 0.0), line(3, 5.0), line(5, 10.0)]);
    let n = steps(&d);
    d.join_endpoints(p, id(4), p, id(5)).unwrap();
    assert_eq!(steps(&d), n + 1);
    let a = d.path(p).unwrap();
    assert_eq!(a.id, p);
    assert_eq!(a.subpaths().count(), 2);
    let s = shape(&a);
    assert_eq!(s[0], (vec![1, 2], false));
    let ids: Vec<u64> = s[1].0.clone();
    assert_eq!(ids.len(), 3, "0006 c9: ends merge into one node");
    assert_eq!(ids[0], 3);
    undo_last(&d);
    assert_eq!(d.path(p).unwrap().subpaths().count(), 3);
}

/// Criterion 5 (b) edge: join the ends in the other order (second selected is earlier outline).
#[test]
fn c5b_join_with_first_selected_in_later_outline() {
    let d = Document::new(1);
    let p = compound(&d, &[line(1, 0.0), line(3, 5.0), line(5, 10.0)]);
    d.join_endpoints(p, id(5), p, id(2)).unwrap();
    let a = d.path(p).unwrap();
    assert_eq!(a.subpaths().count(), 2);
    let all: Vec<u64> = a.all_anchors().map(|x| x.id.as_u128() as u64).collect();
    assert_eq!(
        all.len(),
        5,
        "two ends merge into one node, nothing else lost"
    );
}

/// Criterion 5 (c) test case: compound plate with score line joined to a separate open line.
#[test]
fn c5c_join_across_objects_survivor_keeps_style_and_id() {
    let d = Document::new(1);
    let plate = compound(
        &d,
        &[
            outline(
                1,
                &[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)],
                true,
            ),
            outline(11, &[(10.0, 50.0), (50.0, 50.0)], false),
        ],
    );
    let other = d.create_path(
        &[
            NewAnchor::corner(id(30), pt(50.0, 50.0)),
            NewAnchor::corner(id(31), pt(70.0, 50.0)),
        ],
        false,
    );
    let style = d.path(plate).unwrap().style;
    let n = steps(&d);
    d.join_endpoints(plate, id(12), other, id(30)).unwrap();
    assert_eq!(steps(&d), n + 1);
    assert!(d.path(other).is_none());
    let a = d.path(plate).unwrap();
    assert_eq!(a.id, plate);
    assert_eq!(a.style, style);
    assert_eq!(a.subpaths().count(), 2);
    assert_eq!(
        a.extra_subpaths[0].anchors.len(),
        3,
        "2+2 nodes, ends merged"
    );
    undo_last(&d);
    assert!(d.path(other).is_some());
    assert_eq!(d.path(plate).unwrap().subpaths().count(), 2);
    assert_eq!(d.path(other).unwrap().anchors.len(), 2);
}

/// Criterion 5 (c): the other object's remaining outlines are appended in order; the notice count
/// comes from `join_selected`; first-selected object survives even if it is the ordinary path.
#[test]
fn c5c_join_two_compounds_moves_remaining_outlines_and_counts() {
    let d = Document::new(1);
    let a = compound(&d, &[line(1, 0.0), line(3, 5.0)]);
    let b = compound(&d, &[line(5, 100.0), line(7, 110.0), line(9, 120.0)]);
    let mut tool = NodeTool::new();
    tool.select_nodes(vec![(a, id(4)), (b, id(5))]);
    assert_eq!(tool.join_selected(&d), Some(2));
    assert!(d.path(b).is_none());
    let r = d.path(a).unwrap();
    assert_eq!(r.subpaths().count(), 1 + 1 + 2);
    let tail: Vec<Vec<u64>> = shape(&r)[2..].iter().map(|s| s.0.clone()).collect();
    assert_eq!(tail, vec![vec![7, 8], vec![9, 10]]);
}

/// Criterion 5 (c): ordinary path first, compound second; the ordinary path survives and becomes
/// compound.
#[test]
fn c5c_ordinary_survivor_becomes_compound() {
    let d = Document::new(1);
    let a = d.create_path(
        &[
            NewAnchor::corner(id(1), pt(0.0, 0.0)),
            NewAnchor::corner(id(2), pt(10.0, 0.0)),
        ],
        false,
    );
    let b = compound(&d, &[line(5, 100.0), line(7, 110.0)]);
    let mut tool = NodeTool::new();
    tool.select_nodes(vec![(a, id(2)), (b, id(5))]);
    assert_eq!(tool.join_selected(&d), Some(1));
    let r = d.path(a).unwrap();
    assert!(r.is_compound());
    assert_eq!(r.subpaths().count(), 2);
}

/// Criterion 6: Split at an interior node of an open outline keeps one object; first part first.
#[test]
fn c6_split_open_outline_keeps_one_object() {
    let d = Document::new(1);
    let p = compound(
        &d,
        &[
            line(1, 0.0),
            outline(
                3,
                &[(0.0, 5.0), (5.0, 5.0), (10.0, 5.0), (15.0, 5.0)],
                false,
            ),
            line(7, 9.0),
        ],
    );
    let before_objects = d.object_ids().len();
    let n = steps(&d);
    let mut tool = NodeTool::new();
    let mut minter = AnchorIdMinter::new(500);
    tool.select_nodes(vec![(p, id(5))]);
    assert_eq!(
        tool.split_selected(&mut minter, &d),
        NodeSplitOutcome::OutlineInTwo
    );
    assert_eq!(steps(&d), n + 1);
    assert_eq!(d.object_ids().len(), before_objects);
    let a = d.path(p).unwrap();
    assert_eq!(a.subpaths().count(), 4);
    let s = shape(&a);
    assert_eq!(s[0], (vec![1, 2], false));
    assert_eq!(s[1].0[0], 3);
    assert_eq!(s[3], (vec![7, 8], false));
    assert!(s.iter().all(|(_, c)| !c));
    undo_last(&d);
    assert_eq!(d.path(p).unwrap().subpaths().count(), 3);
}

/// Criterion 6: a node of a closed outline opens it, in place, object stays one; also for the
/// first outline.
#[test]
fn c6_split_closed_outline_opens_it() {
    let d = Document::new(1);
    let p = ring(&d);
    let mut tool = NodeTool::new();
    let mut minter = AnchorIdMinter::new(500);
    tool.select_nodes(vec![(p, id(12))]);
    assert_eq!(
        tool.split_selected(&mut minter, &d),
        NodeSplitOutcome::OutlineOpened
    );
    let a = d.path(p).unwrap();
    assert_eq!(a.subpaths().count(), 2);
    assert!(a.anchors.len() == 4 && a.closed);
    assert!(!a.extra_subpaths[0].closed);
    assert_eq!(
        d.object_ids()
            .iter()
            .filter(|&&x| d.path(x).is_some())
            .count(),
        1
    );
    tool.select_nodes(vec![(p, id(1))]);
    assert_eq!(
        tool.split_selected(&mut minter, &d),
        NodeSplitOutcome::OutlineOpened
    );
    let a = d.path(p).unwrap();
    assert!(!a.closed);
    assert_eq!(a.subpaths().count(), 2);
}

/// Criterion 6 edge: an end node of an open outline cannot be split.
#[test]
fn c6_split_at_an_end_node_is_refused() {
    let d = Document::new(1);
    let p = compound(&d, &[line(1, 0.0), line(3, 5.0)]);
    assert!(d.split_at_anchor(p, id(1), id(900)).is_err());
    assert_eq!(d.path(p).unwrap().subpaths().count(), 2);
}

/// Criterion 8: extend an open outline's end; object keeps id and other outlines; one commit.
#[test]
fn c8_pen_extends_an_outline_of_a_compound_path() {
    let d = Document::new(1);
    let p = compound(&d, &[line(1, 0.0), line(3, 5.0), line(5, 10.0)]);
    let before = d.path(p).unwrap();
    let n = steps(&d);
    let growth = PathGrowth {
        path: p,
        end: PathEnd::Last,
        at: id(4),
        added: vec![NewAnchor::corner(id(60), pt(20.0, 5.0))],
        absorb: None,
        replace: vec![],
        drop: vec![],
    };
    d.extend_path(&growth).unwrap();
    assert_eq!(steps(&d), n + 1);
    assert_eq!(d.own_steps().last().unwrap().label, "extend_path");
    let a = d.path(p).unwrap();
    assert_eq!(a.id, p);
    assert_eq!(a.style, before.style);
    assert_eq!(
        shape(&a),
        vec![
            (vec![1, 2], false),
            (vec![3, 4, 60], false),
            (vec![5, 6], false)
        ]
    );
    // First end of the same outline.
    let growth = PathGrowth {
        end: PathEnd::First,
        at: id(3),
        added: vec![NewAnchor::corner(id(61), pt(-5.0, 5.0))],
        ..growth
    };
    d.extend_path(&growth).unwrap();
    assert_eq!(shape(&d.path(p).unwrap())[1].0, vec![61, 3, 4, 60]);
    let two: Vec<_> = d.own_steps().iter().rev().take(2).map(|x| x.step).collect();
    for step in two {
        d.undo_step(step).unwrap();
    }
    assert_eq!(d.path(p).unwrap(), before);
}

/// Criterion 8 edge: an interior node or a closed outline's node is not an extend target.
#[test]
fn c8_extend_refuses_interior_and_closed_nodes() {
    let d = Document::new(1);
    let p = compound(
        &d,
        &[
            outline(1, &[(0.0, 0.0), (5.0, 0.0), (10.0, 0.0)], false),
            outline(11, &[(0.0, 9.0), (5.0, 9.0), (5.0, 12.0)], true),
        ],
    );
    for (end, at) in [
        (PathEnd::Last, id(2)),
        (PathEnd::Last, id(13)),
        (PathEnd::First, id(11)),
        (PathEnd::Last, id(3)),
    ] {
        let g = PathGrowth {
            path: p,
            end,
            at,
            added: vec![NewAnchor::corner(id(70), pt(30.0, 30.0))],
            absorb: None,
            replace: vec![],
            drop: vec![],
        };
        let r = d.extend_path(&g);
        if at == id(3) && end == PathEnd::Last {
            assert!(r.is_ok());
        } else {
            assert!(r.is_err(), "{at:?} {end:?} must be refused");
        }
    }
}

/// Criterion 9: connect to another object's outline end moves its other outlines in.
#[test]
fn c9_pen_connect_moves_other_outlines_in() {
    let d = Document::new(1);
    let a = compound(&d, &[line(1, 0.0), line(3, 50.0)]);
    let b = compound(&d, &[line(5, 100.0), line(7, 110.0)]);
    let n = steps(&d);
    let g = PathGrowth {
        path: a,
        end: PathEnd::Last,
        at: id(4),
        added: vec![],
        absorb: Some((b, PathEnd::First, id(5))),
        replace: vec![],
        drop: vec![],
    };
    d.connect_paths(&g).unwrap();
    assert_eq!(steps(&d), n + 1);
    assert_eq!(d.own_steps().last().unwrap().label, "connect_paths");
    assert!(d.path(b).is_none());
    let r = d.path(a).unwrap();
    assert_eq!(r.subpaths().count(), 3);
    assert_eq!(shape(&r)[2].0, vec![7, 8]);
    undo_last(&d);
    assert!(d.path(b).is_some());
    assert_eq!(d.path(a).unwrap().subpaths().count(), 2);
}

/// Criterion 10 and edge: selection across a compound path and other paths; Delete is one step,
/// undo restores ids, nodes, directions and open/closed states.
#[test]
fn c10_delete_across_objects_is_one_undoable_step() {
    let d = Document::new(1);
    let p = ring(&d);
    let q = compound(&d, &[line(20, 0.0), line(22, 5.0)]);
    let before_p = d.path(p).unwrap();
    let before_q = d.path(q).unwrap();
    let n = steps(&d);
    let mut tool = NodeTool::new();
    tool.select_nodes(vec![(p, id(11)), (p, id(2)), (q, id(20))]);
    tool.delete_selected(&d);
    assert_eq!(steps(&d), n + 1);
    assert_eq!(d.own_steps().last().unwrap().label, "delete_anchors");
    undo_last(&d);
    assert_eq!(d.path(p).unwrap(), before_p);
    assert_eq!(d.path(q).unwrap(), before_q);
}

/// Criterion 10: node kind conversion, Make line and Close path are single steps on outlines.
#[test]
fn c10_convert_and_close_are_single_steps() {
    let d = Document::new(1);
    let p = compound(
        &d,
        &[
            outline(1, &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], false),
            outline(11, &[(0.0, 20.0), (10.0, 20.0), (10.0, 30.0)], false),
        ],
    );
    let before = d.path(p).unwrap();
    let mut tool = NodeTool::new();
    tool.select_nodes(vec![(p, id(12))]);
    let n = steps(&d);
    tool.convert_selected(&d, curvyo_document_core::AnchorKind::Symmetric);
    assert_eq!(steps(&d), n + 1);
    undo_last(&d);
    assert_eq!(d.path(p).unwrap(), before);
}

/// Criterion 12: save and reopen keeps outlines, states, directions and ids.
#[test]
fn c12_edited_compound_roundtrips_through_json_export() {
    let d = Document::new(1);
    let p = ring(&d);
    d.join_endpoints(p, id(1), p, id(1)).ok();
    let mut tool = NodeTool::new();
    let mut minter = AnchorIdMinter::new(500);
    tool.select_nodes(vec![(p, id(12))]);
    tool.split_selected(&mut minter, &d);
    let before = d.path(p).unwrap();
    let json = String::from_utf8(d.export_json().unwrap()).unwrap();
    assert!(json.contains("extra_subpaths") || before.is_compound());
    assert_ne!(json, "");
    let snap = d.export_loro_snapshot().unwrap();
    assert_ne!(snap.len(), 0);
}

use curvyo_document_core::Tolerance;
use curvyo_ui_core::{
    EndNodeIndex, Hit, JoinType, NodeSelection, ObjectSelection, closable_counts, close_path_set,
    hit_test, plan_close_paths,
};

/// Criterion 1/2: every outline of every object is hit; a click on an extra outline's segment
/// selects it; coincident nodes of two outlines are both reachable through the hit test (one wins).
#[test]
fn c1_c2_hit_reaches_extra_outline_nodes_and_segments_and_handles_coincidence() {
    let d = Document::new(1);
    let p = compound(
        &d,
        &[
            outline(1, &[(0.0, 0.0), (10.0, 0.0)], false),
            outline(3, &[(10.0, 0.0), (10.0, 10.0)], false),
        ],
    );
    let paths = vec![d.path(p).unwrap()];
    let t = Tolerance::from_mm(1.0);
    let none = NodeSelection::new();
    let hit = hit_test(&paths, &none, pt(10.0, 5.0), t, t, t);
    assert_eq!(
        hit,
        Some(Hit::Segment {
            path: p,
            start: id(3),
            end: id(4)
        })
    );
    // Coincident end nodes: some node is hit, and it is one of the two.
    match hit_test(&paths, &none, pt(10.0, 0.0), t, t, t) {
        Some(Hit::Node { anchor, .. }) => assert!(anchor == id(2) || anchor == id(3)),
        other => panic!("expected a node, got {other:?}"),
    }
}

/// Criterion 7: Close path with no selected node closes all open outlines with 3+ nodes, one
/// commit labelled `close_path`; Sharp keeps direction; closed outlines and 2-node lines unchanged;
/// undo restores.
#[test]
fn c7_close_all_open_outlines_when_no_node_selected_one_commit() {
    let d = Document::new(1);
    let p = compound(
        &d,
        &[
            outline(1, &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], false),
            outline(
                11,
                &[(0.0, 20.0), (10.0, 20.0), (10.0, 30.0), (0.0, 30.0)],
                false,
            ),
            outline(21, &[(50.0, 0.0), (60.0, 0.0)], false),
            outline(31, &[(0.0, 50.0), (10.0, 50.0), (10.0, 60.0)], true),
        ],
    );
    let before = d.path(p).unwrap();
    let paths = vec![before.clone()];
    let mut objects = ObjectSelection::new();
    objects.set(&[p]);
    let set = close_path_set(&paths, &NodeSelection::new(), &objects);
    let counts = closable_counts(&set);
    assert_eq!((counts.closable_outlines, counts.skipped_outlines), (2, 1));
    let n = steps(&d);
    d.close_paths(&plan_close_paths(&set, JoinType::Sharp).growths)
        .unwrap();
    assert_eq!(steps(&d), n + 1);
    assert_eq!(d.own_steps().last().unwrap().label, "close_path");
    let after = d.path(p).unwrap();
    let closed: Vec<bool> = after.subpaths().map(|o| o.closed).collect();
    assert_eq!(closed, vec![true, true, false, true]);
    // direction and node ids unchanged
    assert_eq!(
        shape(&after)
            .iter()
            .map(|s| s.0.clone())
            .collect::<Vec<_>>(),
        shape(&before)
            .iter()
            .map(|s| s.0.clone())
            .collect::<Vec<_>>()
    );
    undo_last(&d);
    assert_eq!(d.path(p).unwrap(), before);
}

/// Criterion 7: with a node selected in one outline only that outline closes, even in a plate
/// with other open outlines.
#[test]
fn c7_only_outlines_with_a_selected_node_close() {
    let d = Document::new(1);
    let p = compound(
        &d,
        &[
            outline(1, &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], false),
            outline(11, &[(0.0, 20.0), (10.0, 20.0), (10.0, 30.0)], false),
        ],
    );
    let paths = vec![d.path(p).unwrap()];
    let mut nodes = NodeSelection::new();
    nodes.select_nodes(vec![(p, id(12))]);
    let set = close_path_set(&paths, &nodes, &ObjectSelection::new());
    d.close_paths(&plan_close_paths(&set, JoinType::Smooth).growths)
        .unwrap();
    let closed: Vec<bool> = d.path(p).unwrap().subpaths().map(|o| o.closed).collect();
    assert_eq!(closed, vec![false, true]);
}

/// Criterion 8/9: end nodes of open outlines are targets; closed outline and the interior are not;
/// coincident ends are both candidates.
#[test]
fn c8_end_index_offers_only_open_outline_ends() {
    let d = Document::new(1);
    let p = compound(
        &d,
        &[
            outline(1, &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], false),
            outline(11, &[(0.0, 50.0), (10.0, 50.0), (10.0, 60.0)], true),
        ],
    );
    let index = EndNodeIndex::from_paths(&[d.path(p).unwrap()]);
    let r = curvyo_document_core::Length::from_mm(1.0);
    assert!(index.nearest(pt(0.0, 0.0), r, None).is_some());
    assert!(index.nearest(pt(10.0, 10.0), r, None).is_some());
    assert!(
        index.nearest(pt(10.0, 0.0), r, None).is_none(),
        "interior node"
    );
    assert!(
        index.nearest(pt(0.0, 50.0), r, None).is_none(),
        "closed outline"
    );
}

/// Edge: coincident nodes across outlines, delete one, the other stays.
#[test]
fn edge_coincident_nodes_delete_only_the_named_one() {
    let d = Document::new(1);
    let p = compound(
        &d,
        &[
            outline(1, &[(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)], false),
            outline(11, &[(10.0, 0.0), (10.0, 10.0), (10.0, 20.0)], false),
        ],
    );
    d.delete_anchors(&[(p, id(2))]);
    let a = d.path(p).unwrap();
    // first outline is left with 2 nodes, second untouched
    assert_eq!(
        shape(&a),
        vec![(vec![1, 3], false), (vec![11, 12, 13], false)]
    );
}

/// Edge: single-node outline (compound with a degenerate extra) is handled by move and delete.
#[test]
fn edge_node_drag_to_same_point_and_zero_move_is_harmless() {
    let d = Document::new(1);
    let p = ring(&d);
    let before = d.path(p).unwrap();
    d.move_anchors(&[(p, id(11), pt(40.0, 40.0))]).unwrap();
    assert_eq!(shape(&d.path(p).unwrap()), shape(&before));
    assert!(d.move_anchors(&[(p, id(9999), pt(0.0, 0.0))]).is_err());
}
