//! Independent tester tests for the pure parts of `specs/0053-node-marquee-selection/` in
//! `curvyo-ui-core`: `anchors_in_box` (criteria 3, 4, 9, 16), `editing_set_anchors` (criterion 9)
//! and `NodeSelection::apply` (criteria 5 to 8).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    missing_docs,
    clippy::assert_is_empty
)]

use curvyo_document_core::{
    AnchorId, Document, NewAnchor, NodeId, ObjectSnapshot, PathSnapshot, Point, Tolerance, Vec2,
};
use curvyo_ui_core::{
    NodeSelection, SelectionCombine, anchors_in_box, editing_set_anchors, node_marquee_legend_text,
};

const TOL: Tolerance = Tolerance::from_mm(0.001);

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn make(d: &Document, peer: u64, pts: &[(f64, f64)], closed: bool) -> PathSnapshot {
    let anchors: Vec<NewAnchor> = pts
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| NewAnchor::corner(AnchorId::new(peer, i as u64), pt(x, y)))
        .collect();
    let id = d.create_path(&anchors, closed);
    match d.object(id).unwrap() {
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Path(p) => p,
        ObjectSnapshot::Primitive(_) | ObjectSnapshot::Clone(_) => unreachable!(),
    }
}

#[test]
fn box_example_from_criterion_3_with_a_handle_inside_the_box() {
    let d = Document::new(1);
    let mut first = NewAnchor::corner(AnchorId::new(1, 0), pt(0., 0.));
    first.handle_out = Vec2::new(6., 0.);
    let id = d.create_path(
        &[
            first,
            NewAnchor::corner(AnchorId::new(1, 1), pt(10., 0.)),
            NewAnchor::corner(AnchorId::new(1, 2), pt(20., 0.)),
            NewAnchor::corner(AnchorId::new(1, 3), pt(30., 0.)),
        ],
        false,
    );
    let ObjectSnapshot::Path(p) = d.object(id).unwrap() else {
        panic!()
    };
    let got = anchors_in_box(&[p], pt(5., -5.), pt(25., 5.), TOL);
    let ids: Vec<AnchorId> = got.iter().map(|g| g.1).collect();
    assert_eq!(ids, vec![AnchorId::new(1, 1), AnchorId::new(1, 2)]);
}

#[test]
fn box_edges_are_inclusive_within_tolerance_and_not_beyond() {
    let d = Document::new(1);
    let p = make(
        &d,
        1,
        &[(0., 0.), (10., 0.), (10.0005, 0.), (10.01, 0.)],
        false,
    );
    let got = anchors_in_box(&[p], pt(-1., -1.), pt(10., 1.), TOL);
    assert_eq!(
        got.len(),
        3,
        "10.0 on the edge, 10.0005 within 0.001, 10.01 outside"
    );
}

#[test]
fn result_is_in_document_order_whatever_the_corners_and_the_positions() {
    let d = Document::new(1);
    let a = make(&d, 1, &[(50., 0.), (0., 0.), (25., 0.)], false);
    let b = make(&d, 2, &[(40., 0.), (5., 0.)], true);
    let paths = vec![a.clone(), b.clone()];
    let expect: Vec<(NodeId, AnchorId)> = a
        .anchors
        .iter()
        .map(|x| (a.id, x.id))
        .chain(b.anchors.iter().map(|x| (b.id, x.id)))
        .collect();
    for (f, t) in [
        (pt(-1., -1.), pt(60., 1.)),
        (pt(60., 1.), pt(-1., -1.)),
        (pt(60., -1.), pt(-1., 1.)),
    ] {
        assert_eq!(anchors_in_box(&paths, f, t, TOL), expect);
    }
    let all: Vec<(NodeId, AnchorId)> = editing_set_anchors(&paths)
        .map(|(p, a)| (p, a.id))
        .collect();
    assert_eq!(all, expect);
}

#[test]
fn degenerate_and_hostile_boxes() {
    let d = Document::new(1);
    let p = make(
        &d,
        1,
        &[(0., 0.), (1.0e6, 0.), (2.0e7, 0.), (-2.0e7, 0.)],
        false,
    );
    let paths = vec![p];
    // Zero-size box on a node.
    assert_eq!(anchors_in_box(&paths, pt(0., 0.), pt(0., 0.), TOL).len(), 1);
    // Infinite box: cut at the limit, the nodes at +-2e7 stay out.
    let inf = f64::INFINITY;
    assert_eq!(
        anchors_in_box(&paths, pt(-inf, -inf), pt(inf, inf), TOL).len(),
        2
    );
    // NaN corner: nothing, no panic.
    assert!(anchors_in_box(&paths, pt(f64::NAN, 0.), pt(1., 1.), TOL).is_empty());
    assert!(anchors_in_box(&[], pt(0., 0.), pt(1., 1.), TOL).is_empty());
}

#[test]
fn apply_replace_add_remove_follow_the_table_and_keep_order() {
    let d = Document::new(1);
    let p = make(
        &d,
        1,
        &[(0., 0.), (1., 0.), (2., 0.), (3., 0.), (4., 0.)],
        false,
    );
    let q = make(&d, 2, &[(0., 5.), (1., 5.)], false);
    let n = |path: &PathSnapshot, i: usize| (path.id, path.anchors[i].id);

    let mut sel = NodeSelection::new();
    sel.apply(SelectionCombine::Replace, &[n(&p, 3), n(&p, 1)]);
    assert_eq!(sel.node_pairs(), &[n(&p, 3), n(&p, 1)]);

    // Add: existing keep their place; duplicates are not repeated; new ones appended.
    sel.apply(SelectionCombine::Add, &[n(&p, 1), n(&q, 0), n(&p, 0)]);
    assert_eq!(sel.node_pairs(), &[n(&p, 3), n(&p, 1), n(&q, 0), n(&p, 0)]);
    // Add of an empty result changes nothing.
    sel.apply(SelectionCombine::Add, &[]);
    assert_eq!(sel.node_pairs().len(), 4);

    // Remove: strictly; nodes not selected are not added.
    sel.apply(SelectionCombine::Remove, &[n(&p, 1), n(&p, 4)]);
    assert_eq!(sel.node_pairs(), &[n(&p, 3), n(&q, 0), n(&p, 0)]);
    sel.apply(SelectionCombine::Remove, &[]);
    assert_eq!(sel.node_pairs().len(), 3);

    // Replace with nothing clears.
    sel.apply(SelectionCombine::Replace, &[]);
    assert!(sel.is_empty());

    // Segment selection: Add replaces it, Remove keeps it, Replace drops it.
    sel.select_segment(p.id, p.anchors[0].id, p.anchors[1].id);
    sel.apply(SelectionCombine::Remove, &[n(&p, 2)]);
    assert!(!sel.is_empty(), "segment kept by Remove");
    sel.apply(SelectionCombine::Add, &[]);
    assert!(!sel.is_empty(), "segment kept by empty Add");
    sel.apply(SelectionCombine::Add, &[n(&p, 2)]);
    assert_eq!(sel.node_pairs(), &[n(&p, 2)]);
    sel.select_segment(p.id, p.anchors[0].id, p.anchors[1].id);
    sel.apply(SelectionCombine::Replace, &[]);
    assert!(sel.is_empty());
}

#[test]
fn apply_handles_duplicates_inside_one_result() {
    let d = Document::new(1);
    let p = make(&d, 1, &[(0., 0.), (1., 0.), (2., 0.)], false);
    let x = (p.id, p.anchors[1].id);
    let mut sel = NodeSelection::new();
    sel.apply(SelectionCombine::Replace, &[x, x]);
    assert_eq!(sel.node_pairs().len(), 1, "no repeats");
    sel.apply(SelectionCombine::Add, &[x, x]);
    assert_eq!(sel.node_pairs().len(), 1);
}

#[test]
fn legend_texts_match_criterion_10() {
    assert_eq!(
        node_marquee_legend_text(5, SelectionCombine::Replace),
        "5 nodes \u{b7} Replace"
    );
    assert_eq!(
        node_marquee_legend_text(1, SelectionCombine::Add),
        "1 node \u{b7} +Add"
    );
    assert_eq!(
        node_marquee_legend_text(0, SelectionCombine::Remove),
        "0 nodes \u{b7} \u{2212}Remove"
    );
}
