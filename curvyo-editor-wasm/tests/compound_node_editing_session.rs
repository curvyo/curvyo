//! Node editing of compound paths through the editor session
//! (`specs/0055-compound-path-node-editing` criteria 1 to 10): pointer and toolbar input in the
//! Node tool and the Pen, one commit per gesture, the counts for the notices, and undo.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    AnchorId, Document, NewAnchor, NodeId, PathSnapshot, Point, pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_ui_core::{JoinType, NodeSplitOutcome};

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

fn compound(document: &Document, seed: u64, outlines: &[(Vec<NewAnchor>, bool)]) -> NodeId {
    let base = document.create_path(
        &[
            NewAnchor::corner(id(seed), pt(900.0, 900.0)),
            NewAnchor::corner(id(seed + 1), pt(901.0, 900.0)),
        ],
        false,
    );
    document
        .replace_with_path(&[base], base, outlines, "combine_paths")
        .unwrap()
}

/// A 100 mm square with a 20 mm hole.
fn ring(document: &Document) -> NodeId {
    compound(
        document,
        9000,
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

fn session_of(document: &Document, tool: Tool) -> Session {
    let mut session = Session::open(2, &pack(document, "0.1.0").unwrap()).unwrap();
    session.resize_viewport(1200.0, 800.0);
    session.set_tool(tool);
    session
}

fn click(session: &mut Session, at: Point, shift: bool) {
    session.pointer_hover(at, shift, false);
    session.pointer_down(at, shift);
    session.pointer_up(at, shift, false);
}

fn drag(session: &mut Session, from: Point, to: Point) {
    session.pointer_hover(from, false, false);
    session.pointer_down(from, false);
    session.pointer_hover(to, false, false);
    session.pointer_up(to, false, false);
}

fn reread(session: &Session) -> Document {
    unpack(9, &session.pack("0.1.0").unwrap()).unwrap()
}

fn path(session: &Session, node: NodeId) -> Option<PathSnapshot> {
    reread(session).path(node)
}

fn outlines(path: &PathSnapshot) -> Vec<(Vec<AnchorId>, bool)> {
    path.subpaths()
        .map(|o| (o.anchors.iter().map(|a| a.id).collect(), o.closed))
        .collect()
}

/// Criteria 1 to 3 and 10: a click selects a node of the hole, Shift-click adds a node of the
/// outer square, one drag moves both in one undo step, and undo restores them.
#[test]
fn c1_c3_c10_nodes_of_two_outlines_move_in_one_step_and_undo_restores_them() {
    let document = Document::new(1);
    let node = ring(&document);
    let mut session = session_of(&document, Tool::Node);
    assert_eq!(session.node_toolbar_state().total_nodes, 8);
    let before = path(&session, node).unwrap();
    click(&mut session, pt(60.0, 40.0), false);
    click(&mut session, pt(100.0, 0.0), true);
    assert_eq!(session.node_toolbar_state().selected_nodes, 2);
    let steps = session.undo_len();
    drag(&mut session, pt(60.0, 40.0), pt(70.0, 45.0));
    assert_eq!(session.undo_len(), steps + 1, "one step");
    let after = path(&session, node).unwrap();
    assert_eq!(outlines(&after), outlines(&before), "ids and states kept");
    assert_eq!(after.extra_subpaths[0].anchors[1].point, pt(70.0, 45.0));
    assert_eq!(after.anchors[3].point, pt(110.0, 5.0));
    session.undo();
    assert_eq!(path(&session, node).unwrap(), before);
}

/// Criteria 2 and 3: a click on a segment of the hole selects it; Make curve and insert by
/// double-click act on the hole.
#[test]
fn c2_c3_a_segment_of_the_hole_is_selected_and_edited() {
    let document = Document::new(1);
    let node = ring(&document);
    let mut session = session_of(&document, Tool::Node);
    click(&mut session, pt(50.0, 40.0), false);
    let state = session.node_toolbar_state();
    assert!(state.segment_selected && state.can_make_curve, "{state:?}");
    session.make_curve();
    let hole = &path(&session, node).unwrap().extra_subpaths[0];
    assert_ne!(hole.anchors[0].handle_out, curvyo_document_core::Vec2::ZERO);
    session.insert_at(pt(60.0, 50.0));
    assert_eq!(
        path(&session, node).unwrap().extra_subpaths[0]
            .anchors
            .len(),
        5
    );
}

/// Criteria 4 and 10, the spec's test through the session: three open lines; Delete of one node
/// of the first leaves 2 outlines, of one of the second an ordinary path with the same id; undo
/// restores the compound path.
#[test]
fn c4_c10_delete_drops_to_an_ordinary_path_and_undo_restores() {
    let document = Document::new(1);
    let node = compound(
        &document,
        9000,
        &[
            outline(10, &[(0.0, 0.0), (50.0, 0.0)], false),
            outline(20, &[(0.0, 50.0), (50.0, 50.0)], false),
            outline(30, &[(0.0, 100.0), (50.0, 100.0)], false),
        ],
    );
    let mut session = session_of(&document, Tool::Node);
    let before = path(&session, node).unwrap();
    click(&mut session, pt(0.0, 0.0), false);
    session.delete_selected();
    assert_eq!(path(&session, node).unwrap().extra_subpaths.len(), 1);
    click(&mut session, pt(50.0, 50.0), false);
    session.delete_selected();
    let after = path(&session, node).unwrap();
    assert!(!after.is_compound());
    assert_eq!(outlines(&after), vec![(vec![id(30), id(31)], false)]);
    session.undo();
    session.undo();
    assert_eq!(path(&session, node).unwrap(), before);
}

/// Criteria 5 c and 10: an open line joined to a score line of a plate moves into the plate; the
/// session reports the moved outlines for the notice (none here, 1 in the reverse order).
#[test]
fn c5_join_reports_the_moved_outlines_and_undo_restores_both_objects() {
    let document = Document::new(1);
    let plate = compound(
        &document,
        9000,
        &[
            outline(
                10,
                &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0), (100.0, 0.0)],
                true,
            ),
            outline(20, &[(20.0, 50.0), (80.0, 50.0)], false),
        ],
    );
    let line = document.create_path(
        &[
            NewAnchor::corner(id(30), pt(150.0, 50.0)),
            NewAnchor::corner(id(31), pt(150.0, 90.0)),
        ],
        false,
    );
    let mut session = session_of(&document, Tool::Node);
    let before = reread(&session).export_json().unwrap();
    // First-selected: the line's end, so the line survives and the plate's square moves in.
    click(&mut session, pt(150.0, 50.0), false);
    click(&mut session, pt(80.0, 50.0), true);
    assert!(session.node_toolbar_state().can_join);
    let steps = session.undo_len();
    assert_eq!(session.join_selected(), Some(1));
    assert_eq!(session.undo_len(), steps + 1);
    assert!(path(&session, plate).is_none());
    let joined = path(&session, line).unwrap();
    assert_eq!(
        outlines(&joined),
        vec![
            (vec![id(20), id(30), id(31)], false),
            (vec![id(10), id(11), id(12), id(13)], true)
        ]
    );
    session.undo();
    assert_eq!(reread(&session).export_json().unwrap(), before);
}

/// Criterion 6: Split at a node of an open outline keeps one object and says so.
#[test]
fn c6_split_reports_what_happened_to_the_outline() {
    let document = Document::new(1);
    let node = compound(
        &document,
        9000,
        &[
            outline(10, &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0)], true),
            outline(20, &[(20.0, 150.0), (60.0, 150.0), (100.0, 150.0)], false),
        ],
    );
    let mut session = session_of(&document, Tool::Node);
    click(&mut session, pt(60.0, 150.0), false);
    assert_eq!(
        session.split_selected_outcome(),
        NodeSplitOutcome::OutlineInTwo
    );
    assert_eq!(path(&session, node).unwrap().extra_subpaths.len(), 2);
    click(&mut session, pt(0.0, 100.0), false);
    assert_eq!(
        session.split_selected_outcome(),
        NodeSplitOutcome::OutlineOpened
    );
    assert!(!path(&session, node).unwrap().closed);
    assert_eq!(reread(&session).object_ids(), vec![node]);
}

/// Criterion 7: with a node of one open outline selected only that outline closes; with the
/// object selected and no node, all its open outlines close; the counts are apart from paths.
#[test]
fn c7_close_path_closes_the_outlines_by_the_selected_node_rule() {
    let document = Document::new(1);
    let node = compound(
        &document,
        9000,
        &[
            outline(10, &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0)], true),
            outline(20, &[(10.0, 150.0), (50.0, 150.0), (50.0, 190.0)], false),
            outline(30, &[(110.0, 150.0), (150.0, 150.0), (150.0, 190.0)], false),
            outline(40, &[(210.0, 150.0), (250.0, 150.0)], false),
        ],
    );
    let plain = document.create_path(
        &[
            NewAnchor::corner(id(50), pt(300.0, 0.0)),
            NewAnchor::corner(id(51), pt(350.0, 0.0)),
            NewAnchor::corner(id(52), pt(350.0, 50.0)),
        ],
        false,
    );
    let mut session = session_of(&document, Tool::Node);
    click(&mut session, pt(50.0, 150.0), false);
    click(&mut session, pt(350.0, 0.0), true);
    let state = session.close_path_state();
    assert_eq!(
        (
            state.closable,
            state.closable_outlines,
            state.skipped_outlines
        ),
        (1, 1, 0)
    );
    let outcome = session.close_paths(JoinType::Sharp);
    assert_eq!((outcome.closed, outcome.closed_outlines), (1, 1));
    let states: Vec<bool> = path(&session, node)
        .unwrap()
        .subpaths()
        .map(|o| o.closed)
        .collect();
    assert_eq!(states, vec![true, true, false, false]);
    assert!(path(&session, plain).unwrap().closed);
    // The object selected, no node of it: every open outline is a candidate.
    let mut session = session_of(&document, Tool::Select);
    click(&mut session, pt(0.0, 50.0), false);
    session.set_tool(Tool::Node);
    let state = session.close_path_state();
    assert_eq!((state.closable_outlines, state.skipped_outlines), (2, 1));
}

/// Criteria 8 and 10: the Pen continues an open outline of a compound path from its end; the
/// object keeps its id and the other outlines; one `extend_path` step that undo removes.
#[test]
fn c8_the_pen_continues_an_outline_of_a_compound_path() {
    let document = Document::new(1);
    let node = compound(
        &document,
        9000,
        &[
            outline(10, &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0)], true),
            outline(20, &[(20.0, 150.0), (100.0, 150.0)], false),
        ],
    );
    let mut session = session_of(&document, Tool::Pen);
    let before = path(&session, node).unwrap();
    session.pointer_hover(pt(100.0, 150.0), false, false);
    assert!(matches!(
        session.pen_target(),
        Some(curvyo_ui_core::PenTarget::Continue(_))
    ));
    click(&mut session, pt(100.0, 150.0), false);
    click(&mut session, pt(200.0, 200.0), false);
    session.finish_pen();
    let after = path(&session, node).unwrap();
    assert_eq!(after.anchors, before.anchors, "the square is unchanged");
    assert_eq!(after.extra_subpaths[0].anchors.len(), 3);
    assert_eq!(after.extra_subpaths[0].anchors[2].point, pt(200.0, 200.0));
    session.undo();
    assert_eq!(path(&session, node).unwrap(), before);
}

/// Criterion 9: a continued path that ends on an open outline end of a compound path absorbs it,
/// the compound path's square moves in, and the session holds the notice of 1 moved outline.
#[test]
fn c9_the_pen_connects_to_an_outline_and_moves_the_others_in() {
    let document = Document::new(1);
    let plate = compound(
        &document,
        9000,
        &[
            outline(10, &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0)], true),
            outline(20, &[(20.0, 150.0), (100.0, 150.0)], false),
        ],
    );
    let line = document.create_path(
        &[
            NewAnchor::corner(id(30), pt(300.0, 300.0)),
            NewAnchor::corner(id(31), pt(300.0, 200.0)),
        ],
        false,
    );
    let mut session = session_of(&document, Tool::Pen);
    assert!(session.pen_join_notice().is_none());
    click(&mut session, pt(300.0, 200.0), false);
    click(&mut session, pt(200.0, 150.0), false);
    session.pointer_hover(pt(100.0, 150.0), false, false);
    assert!(matches!(
        session.pen_target(),
        Some(curvyo_ui_core::PenTarget::Join(_))
    ));
    click(&mut session, pt(100.0, 150.0), false);
    assert!(path(&session, plate).is_none(), "the plate was absorbed");
    let joined = path(&session, line).unwrap();
    assert_eq!(joined.extra_subpaths.len(), 1, "the square moved in");
    let notice = session.pen_join_notice().expect("a notice");
    assert_eq!(notice.moved_outlines, 1);
    session.undo();
    assert!(path(&session, plate).is_some());
}
