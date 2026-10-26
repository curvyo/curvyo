//! Tester cases for `specs/0055-compound-path-node-editing` through the editor session, for what
//! merged since 2026-10-19: the node marquee (`0053`), the Pen's typed values and angle snap
//! (`0072`), primitives (`0003`, `0016` Combine) and linked clones (`0049`) next to compound
//! paths. Written from the specifications before the implementation was read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::many_single_char_names
)]

use curvyo_document_core::{
    AnchorId, Document, Length, NewAnchor, NodeId, ObjectSnapshot, PathSnapshot, Point, RectBounds,
    pack, unpack,
};
use curvyo_editor_wasm::{CombineOutcome, KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::NodeSplitOutcome;

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

fn compound(document: &Document, seed: u64, outlines: &[Spec]) -> NodeId {
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
                110,
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

fn reread(session: &Session) -> Document {
    unpack(9, &session.pack("0.1.0").unwrap()).unwrap()
}

fn path(session: &Session, node: NodeId) -> Option<PathSnapshot> {
    reread(session).path(node)
}

fn ids(p: &PathSnapshot) -> Vec<(Vec<AnchorId>, bool)> {
    p.subpaths()
        .map(|o| (o.anchors.iter().map(|a| a.id).collect(), o.closed))
        .collect()
}

fn click(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

fn drag(s: &mut Session, from: Point, to: Point) {
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(
        pt(f64::midpoint(from.x, to.x), f64::midpoint(from.y, to.y)),
        false,
        false,
    );
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

fn key(s: &mut Session, k: &str, ctrl: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        ..KeyInput::default()
    })
}

// ------------------------------------------------------------------ node marquee (0053) and Join

/// Two outlines of one compound path that touch at (50, 0): a marquee over the touching point
/// rings both end nodes (the UX notes: "which is how two coincident ends are selected for Join"),
/// and Join makes one outline in one step; undo restores both.
#[test]
fn a_marquee_over_two_touching_ends_selects_both_and_join_merges_the_outlines() {
    let d = Document::new(1);
    let plate = compound(
        &d,
        9000,
        &[
            outline(10, &[(0.0, 0.0), (25.0, 0.0), (50.0, 0.0)], false),
            outline(20, &[(50.0, 0.0), (75.0, 0.0), (100.0, 0.0)], false),
            outline(30, &[(0.0, 80.0), (100.0, 80.0), (100.0, 100.0)], false),
        ],
    );
    let mut s = session_of(&d, Tool::Node);
    drag(&mut s, pt(45.0, -10.0), pt(55.0, 10.0));
    let state = s.node_toolbar_state();
    assert_eq!(state.selected_nodes, 2, "both coincident ends are ringed");
    assert!(state.can_join, "{:?}", state.join_block);
    let before = path(&s, plate).unwrap();
    let steps = s.undo_len();
    assert!(s.join_selected().is_some());
    assert_eq!(s.undo_len(), steps + 1);
    let after = path(&s, plate).unwrap();
    assert_eq!(after.subpaths().count(), 2, "two outlines became one");
    let joined = after.subpaths().next().unwrap();
    assert_eq!(
        joined.anchors.len(),
        5,
        "the two touching ends became one node"
    );
    assert_eq!(after.subpaths().nth(1).unwrap().anchors.len(), 3);
    s.undo();
    assert_eq!(path(&s, plate).unwrap(), before);
}

/// A marquee that takes three ends cannot be joined; the reason says how many are selected.
#[test]
fn a_marquee_over_three_end_nodes_disables_join_with_the_count() {
    let d = Document::new(1);
    compound(
        &d,
        9000,
        &[
            outline(10, &[(0.0, 0.0), (50.0, 0.0)], false),
            outline(20, &[(50.0, 0.0), (100.0, 0.0)], false),
            outline(30, &[(50.0, 2.0), (60.0, 30.0)], false),
        ],
    );
    let mut s = session_of(&d, Tool::Node);
    drag(&mut s, pt(45.0, -10.0), pt(55.0, 10.0));
    let state = s.node_toolbar_state();
    assert_eq!(state.selected_nodes, 3);
    assert!(!state.can_join);
    assert_eq!(state.join_block, Some(curvyo_ui_core::JoinBlock::Count(3)));
}

/// Marquee and Shift-click mix across a compound path and an ordinary path; Split then reports
/// only for one node.
#[test]
fn a_marquee_over_a_compound_and_an_ordinary_path_moves_in_one_step() {
    let d = Document::new(1);
    let plate = ring(&d);
    let line = d.create_path(
        &[
            NewAnchor::corner(id(500), pt(110.0, 50.0)),
            NewAnchor::corner(id(501), pt(160.0, 50.0)),
        ],
        false,
    );
    let mut s = session_of(&d, Tool::Node);
    drag(&mut s, pt(95.0, -5.0), pt(115.0, 55.0));
    assert_eq!(
        s.node_toolbar_state().selected_nodes,
        2,
        "ring corner (100, 0) and (110, 50)"
    );
    let steps = s.undo_len();
    drag(&mut s, pt(100.0, 0.0), pt(103.0, 7.0));
    assert_eq!(s.undo_len(), steps + 1);
    let r = reread(&s);
    assert_eq!(r.path(plate).unwrap().anchors[3].point, pt(103.0, 7.0));
    assert_eq!(r.path(line).unwrap().anchors[0].point, pt(113.0, 57.0));
    assert_eq!(
        s.split_selected_outcome(),
        NodeSplitOutcome::None,
        "two nodes: no split"
    );
}

// ------------------------------------------------------------------------- Pen angle snap (0072)

fn place_typed(s: &mut Session, length: &str, angle: &str) -> bool {
    let _ = key(s, "Tab", false);
    s.pen_entry_edit(0, length);
    s.pen_entry_edit(1, angle);
    s.pen_entry_place()
}

/// Continuing an extra outline of a compound path from its first node: a typed length and angle
/// and a Ctrl-snapped click both start from that node (`0072` criteria 7, 31), the new nodes are
/// prepended to that outline, and the other outlines and the object's id stay; one step.
#[test]
fn the_pen_types_and_snaps_when_it_continues_an_extra_outline_from_its_first_node() {
    let d = Document::new(1);
    let plate = compound(
        &d,
        9000,
        &[
            outline(10, &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0)], true),
            outline(20, &[(100.0, 50.0), (200.0, 50.0)], false),
        ],
    );
    let mut s = session_of(&d, Tool::Pen);
    let before = path(&s, plate).unwrap();
    let steps = s.undo_len();
    click(&mut s, pt(100.0, 50.0));
    assert!(
        place_typed(&mut s, "40", "180"),
        "Tab from the first node typed nothing"
    );
    s.modifiers_changed(false, true, false);
    s.pointer_hover(pt(59.0, 80.0), false, true);
    s.pointer_down(pt(59.0, 80.0), false);
    s.pointer_up(pt(59.0, 80.0), false, true);
    s.modifiers_changed(false, false, false);
    s.finish_pen();
    let after = path(&s, plate).unwrap();
    assert_eq!(after.anchors, before.anchors, "the square is untouched");
    let extra = &after.extra_subpaths[0];
    assert_eq!(extra.anchors.len(), 4, "{:?}", extra.anchors);
    let got: Vec<(f64, f64)> = extra
        .anchors
        .iter()
        .map(|a| (a.point.x, a.point.y))
        .collect();
    let want = [(60.0, 80.017), (60.0, 50.0), (100.0, 50.0), (200.0, 50.0)];
    for ((x, y), (ex, ey)) in got.iter().zip(want) {
        assert!((x - ex).abs() < 0.01 && (y - ey).abs() < 0.01, "{got:?}");
    }
    assert_eq!(extra.anchors[2].id, id(20));
    assert_eq!(extra.anchors[3].id, id(21));
    assert_eq!(s.undo_len(), steps + 1, "one extend_path step");
    s.undo();
    assert_eq!(path(&s, plate).unwrap(), before);
}

/// Ctrl held at a click on the end node of another object's outline still connects (the join
/// target wins over the snap), `0034` criterion 7 with `0055` criterion 9.
#[test]
fn a_ctrl_click_on_another_outline_end_still_connects() {
    let d = Document::new(1);
    let plate = compound(
        &d,
        9000,
        &[
            outline(10, &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0)], true),
            outline(20, &[(20.0, 150.0), (100.0, 150.0)], false),
        ],
    );
    let line = d.create_path(
        &[
            NewAnchor::corner(id(30), pt(300.0, 300.0)),
            NewAnchor::corner(id(31), pt(300.0, 200.0)),
        ],
        false,
    );
    let mut s = session_of(&d, Tool::Pen);
    click(&mut s, pt(300.0, 200.0));
    click(&mut s, pt(200.0, 150.0));
    s.modifiers_changed(false, true, false);
    s.pointer_hover(pt(100.0, 150.0), false, true);
    s.pointer_down(pt(100.0, 150.0), false);
    s.pointer_up(pt(100.0, 150.0), false, true);
    s.modifiers_changed(false, false, false);
    assert!(path(&s, plate).is_none(), "the plate was absorbed");
    let joined = path(&s, line).unwrap();
    let outlines = ids(&joined);
    assert_eq!(outlines.len(), 2, "the joined outline and the square");
    let total: usize = outlines.iter().map(|o| o.0.len()).sum();
    assert_eq!(total, 2 + 1 + 2 + 3, "{outlines:?}");
}

// ----------------------------------------------------------------------------------- primitives

fn rect(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: pt(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

/// A primitive selected beside a compound path stays a primitive when nodes of the compound path
/// are edited, and its box is not touched.
#[test]
fn editing_compound_nodes_beside_a_primitive_leaves_the_primitive_a_primitive() {
    let d = Document::new(1);
    let plate = ring(&d);
    let shape = rect(&d, 300.0, 300.0, 40.0, 30.0);
    let mut s = session_of(&d, Tool::Select);
    assert_eq!(key(&mut s, "a", true), KeyOutcome::SelectedAll);
    s.set_tool(Tool::Node);
    let before_shape = reread(&s).read_objects().get(shape).cloned().unwrap();
    let steps = s.undo_len();
    click(&mut s, pt(60.0, 40.0));
    drag(&mut s, pt(60.0, 40.0), pt(70.0, 45.0));
    assert_eq!(s.undo_len(), steps + 1);
    let r = reread(&s);
    assert_eq!(
        r.path(plate).unwrap().extra_subpaths[0].anchors[1].point,
        pt(70.0, 45.0)
    );
    let after_shape = r.read_objects().get(shape).cloned().unwrap();
    assert!(matches!(after_shape, ObjectSnapshot::Primitive(_)));
    assert_eq!(after_shape, before_shape);
}

/// A rectangle primitive Combined with an open path becomes a compound path whose nodes the Node
/// tool edits like any (criterion 1), outline by outline.
#[test]
fn a_combined_primitive_becomes_node_editable_in_the_compound_path() {
    let d = Document::new(1);
    let shape = rect(&d, 0.0, 0.0, 100.0, 100.0);
    let open = d.create_path(
        &[
            NewAnchor::corner(id(700), pt(20.0, 50.0)),
            NewAnchor::corner(id(701), pt(80.0, 50.0)),
        ],
        false,
    );
    let mut s = session_of(&d, Tool::Select);
    assert_eq!(key(&mut s, "a", true), KeyOutcome::SelectedAll);
    assert!(matches!(s.apply_combine(), CombineOutcome::Applied { .. }));
    let objects = reread(&s).object_ids();
    assert_eq!(objects.len(), 1, "one compound path");
    let node = objects[0];
    assert!(path(&s, open).is_none(), "the operands are replaced");
    let _ = shape;
    let before = path(&s, node).unwrap();
    assert!(before.is_compound());
    s.set_tool(Tool::Node);
    assert_eq!(s.node_toolbar_state().total_nodes, 6);
    let steps = s.undo_len();
    click(&mut s, pt(80.0, 50.0));
    drag(&mut s, pt(80.0, 50.0), pt(90.0, 60.0));
    assert_eq!(s.undo_len(), steps + 1);
    let after = path(&s, node).unwrap();
    assert_eq!(ids(&after), ids(&before), "ids, order and states kept");
    let moved = after
        .subpaths()
        .flat_map(|o| o.anchors.iter())
        .any(|a| a.point == pt(90.0, 60.0));
    assert!(moved);
}

// ------------------------------------------------------------------------------ linked clones

/// With only a linked clone of a compound path selected the Node bar says so (`0049` criterion
/// 26, "linked clones only" after `0055`); the source's nodes stay editable in the same tool and
/// the clone follows the edit, outline by outline, with one step that undo takes back.
#[test]
fn the_node_tool_edits_the_source_of_a_clone_of_a_compound_path_and_the_clone_follows() {
    let d = Document::new(1);
    let source = ring(&d);
    let mut s = session_of(&d, Tool::Select);
    assert_eq!(key(&mut s, "a", true), KeyOutcome::SelectedAll);
    let made = s.linked_clone();
    assert_eq!(made.notice, "Created 1 linked clone.");
    s.set_tool(Tool::Node);
    assert!(s.node_selection_is_clones(), "only the clone is selected");
    assert!(s.node_toolbar_state().clone_only);
    let before = reread(&s);
    let steps = s.undo_len();
    click(&mut s, pt(60.0, 40.0));
    drag(&mut s, pt(60.0, 40.0), pt(70.0, 45.0));
    assert_eq!(s.undo_len(), steps + 1, "one step");
    let r = reread(&s);
    assert_eq!(
        r.path(source).unwrap().extra_subpaths[0].anchors[1].point,
        pt(70.0, 45.0)
    );
    let clone = r
        .object_ids()
        .into_iter()
        .find(|&n| matches!(r.read_objects().get(n), Some(ObjectSnapshot::Clone(_))))
        .expect("the clone is still a clone");
    match r.read_objects().get(clone) {
        Some(ObjectSnapshot::Clone(c)) => {
            assert_eq!(
                c.resolved.extra_subpaths[0].anchors[1].point,
                pt(70.0, 45.0)
            );
            assert_eq!(c.resolved.extra_subpaths[0].anchors[1].id, id(111));
        }
        other => panic!("{other:?}"),
    }
    s.undo();
    assert_eq!(path(&s, source), before.path(source));
}
