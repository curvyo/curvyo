//! The Node tool, Close path and the Pen's targets on compound paths
//! (`specs/0055-compound-path-node-editing` criteria 1 to 9): hit tests reach every outline, the
//! closing line of an open outline is no segment, Close path takes outlines by the selected-node
//! rule and counts them apart from paths, and the Pen finds open outline ends.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    AnchorId, Document, NewAnchor, NodeId, PathEnd, PathSnapshot, Point, Tolerance,
};
use curvyo_ui_core::{
    AnchorIdMinter, EndNodeIndex, Hit, JoinType, NodeSelection, NodeSplitOutcome, NodeTool,
    ObjectSelection, closable_counts, close_path_set, continuation_of, hit_test, plan_close_paths,
};

const T: Tolerance = Tolerance::from_mm(2.0);

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn id(n: u64) -> AnchorId {
    AnchorId::new(1, n)
}

fn outline(first: u64, points: &[(f64, f64)], closed: bool) -> (Vec<NewAnchor>, bool) {
    (
        points
            .iter()
            .zip(first..)
            .map(|(&(x, y), n)| NewAnchor::corner(id(n), pt(x, y)))
            .collect(),
        closed,
    )
}

fn compound(document: &Document, outlines: &[(Vec<NewAnchor>, bool)]) -> PathSnapshot {
    let base = document.create_path(
        &[
            NewAnchor::corner(id(9000), pt(900.0, 900.0)),
            NewAnchor::corner(id(9001), pt(901.0, 900.0)),
        ],
        false,
    );
    let node = document
        .replace_with_path(&[base], base, outlines, "combine_paths")
        .unwrap();
    document.path(node).unwrap()
}

/// A plate (closed square) with an open U and an open line.
fn plate(document: &Document) -> PathSnapshot {
    compound(
        document,
        &[
            outline(
                10,
                &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0), (100.0, 0.0)],
                true,
            ),
            outline(
                20,
                &[(20.0, 20.0), (20.0, 40.0), (40.0, 40.0), (40.0, 20.0)],
                false,
            ),
            outline(30, &[(60.0, 50.0), (80.0, 50.0)], false),
        ],
    )
}

/// Criteria 1 and 2: nodes and segments of every outline are hit; the closing line of the open U
/// is not a segment.
#[test]
fn c1_c2_hits_reach_every_outline_but_never_a_closing_line() {
    let document = Document::new(1);
    let path = plate(&document);
    let paths = vec![path.clone()];
    let none = NodeSelection::new();
    assert_eq!(
        hit_test(&paths, &none, pt(40.0, 40.0), T, T, T),
        Some(Hit::Node {
            path: path.id,
            anchor: id(22)
        })
    );
    assert_eq!(
        hit_test(&paths, &none, pt(70.0, 50.0), T, T, T),
        Some(Hit::Segment {
            path: path.id,
            start: id(30),
            end: id(31)
        })
    );
    // The U's closing line runs from (40, 20) to (20, 20): its midpoint hits nothing.
    assert_eq!(hit_test(&paths, &none, pt(30.0, 20.0), T, T, T), None);
}

/// Criteria 5 and 6 through the Node tool: Join of two outlines of one object reports no moved
/// outline; Split of an open outline reports two outlines.
#[test]
fn c5_c6_join_and_split_report_their_outcome() {
    let document = Document::new(1);
    let path = plate(&document);
    let paths = vec![path.clone()];
    let mut tool = NodeTool::new();
    tool.select_nodes(vec![(path.id, id(23)), (path.id, id(30))]);
    assert!(tool.join_check(&document, &paths).is_ok());
    assert_eq!(tool.join_selected(&document), Some(0));
    let joined = document.path(path.id).unwrap();
    assert_eq!(
        joined.extra_subpaths.len(),
        1,
        "the U and the line became one"
    );
    let mut minter = AnchorIdMinter::new(7);
    tool.select_nodes(vec![(path.id, id(21))]);
    assert_eq!(
        tool.split_selected(&mut minter, &document),
        NodeSplitOutcome::OutlineInTwo
    );
    tool.select_nodes(vec![(path.id, id(11))]);
    assert_eq!(
        tool.split_selected(&mut minter, &document),
        NodeSplitOutcome::OutlineOpened
    );
}

/// Criterion 7: a selected node of the U makes it the only candidate of the plate; with the
/// object selected and no node, both open outlines are; the line is skipped for having 2 nodes.
#[test]
fn c7_the_close_path_set_follows_the_selected_node_rule() {
    let document = Document::new(1);
    let path = plate(&document);
    let paths = vec![path.clone()];
    let mut nodes = NodeSelection::new();
    nodes.select_nodes(vec![(path.id, id(21))]);
    let set = close_path_set(&paths, &nodes, &ObjectSelection::new());
    assert_eq!(set.len(), 1);
    let counts = closable_counts(&set);
    assert_eq!((counts.closable, counts.closable_outlines), (0, 1));
    let plan = plan_close_paths(&set, JoinType::Sharp);
    assert_eq!(plan.growths.len(), 1);
    assert_eq!(
        (plan.growths[0].end, plan.growths[0].at),
        (PathEnd::Last, id(23))
    );

    let mut objects = ObjectSelection::new();
    objects.set(&[path.id]);
    let set = close_path_set(&paths, &NodeSelection::new(), &objects);
    let counts = closable_counts(&set);
    assert_eq!(
        (counts.closable_outlines, counts.skipped_outlines),
        (1, 1),
        "the U closes, the line is skipped"
    );
    document
        .close_paths(&plan_close_paths(&set, JoinType::Smooth).growths)
        .unwrap();
    let closed: Vec<bool> = document
        .path(path.id)
        .unwrap()
        .subpaths()
        .map(|o| o.closed)
        .collect();
    assert_eq!(closed, vec![true, true, false]);
}

/// Criteria 8 and 9: the open outlines' ends are Pen targets; a continuation reads its outline.
#[test]
fn c8_c9_open_outline_ends_are_pen_targets() {
    let document = Document::new(1);
    let path = plate(&document);
    let index = EndNodeIndex::from_paths(std::slice::from_ref(&path));
    let radius = curvyo_document_core::Length::from_mm(2.0);
    let end = index
        .nearest(pt(80.0, 50.0), radius, None)
        .expect("the line's last node");
    assert_eq!((end.anchor.id, end.end), (id(31), PathEnd::Last));
    assert!(
        index.nearest(pt(0.0, 0.0), radius, None).is_none(),
        "a node of a closed outline is no target"
    );
    let continuation = continuation_of(end, &path);
    let ids: Vec<AnchorId> = continuation.original.iter().map(|a| a.id).collect();
    assert_eq!(ids, vec![id(30), id(31)]);
    assert_eq!(continuation.end_node(), id(31));
    let other: Option<NodeId> = Some(path.id);
    assert!(index.nearest(pt(80.0, 50.0), radius, other).is_none());
}
