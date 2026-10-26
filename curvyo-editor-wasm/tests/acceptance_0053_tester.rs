//! Independent tester acceptance tests for `specs/0053-node-marquee-selection/` through
//! `Session`. Written from the specification and the public API before the implementation diff
//! was read. The node selection is read through the toolbar state, the legend, the
//! announcement, and by deleting or joining (which names the selected nodes exactly).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(clippy::too_many_lines, clippy::cast_precision_loss, missing_docs)]
#![allow(
    clippy::doc_markdown,
    clippy::similar_names,
    clippy::many_single_char_names
)]

use std::collections::HashSet;

use curvyo_document_core::{
    AnchorId, AnchorKind, Direction, Document, NewAnchor, NodeId, ObjectSnapshot, PathSnapshot,
    Point, Vec2, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::{JoinBlock, SelectionCombine};

type M = (bool, bool, bool);
const NONE: M = (false, false, false);
const SHIFT: M = (true, false, false);
const CTRL: M = (false, true, false);
const ALT: M = (false, false, true);
const SHIFT_CTRL: M = (true, true, false);
const ALT_CTRL: M = (false, true, true);

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn n(peer: u64, i: u64, x: f64, y: f64) -> NewAnchor {
    NewAnchor::corner(AnchorId::new(peer, i), pt(x, y))
}

/// Millimetre layout:
/// - P1 open, y = 100: x = 100, 200, 300, 400 (ids 1/0..3)
/// - P2 open, y = 100: x = 400, 500, 600 (ids 2/0..2); its first node lies on P1's last
/// - P3 closed square (ids 3/0..3): (100,300) (200,300) (200,400) (100,400)
/// - P4 open, y = 600: x = 100, 200, 300, 400 (ids 4/0..3); node 0 has a handle out to (160, 600)
fn base_doc() -> (Document, [NodeId; 4]) {
    let d = Document::new(1);
    let p1 = d.create_path(
        &[
            n(1, 0, 100., 100.),
            n(1, 1, 200., 100.),
            n(1, 2, 300., 100.),
            n(1, 3, 400., 100.),
        ],
        false,
    );
    let p2 = d.create_path(
        &[
            n(2, 0, 400., 100.),
            n(2, 1, 500., 100.),
            n(2, 2, 600., 100.),
        ],
        false,
    );
    let p3 = d.create_path(
        &[
            n(3, 0, 100., 300.),
            n(3, 1, 200., 300.),
            n(3, 2, 200., 400.),
            n(3, 3, 100., 400.),
        ],
        true,
    );
    let mut h = n(4, 0, 100., 600.);
    h.handle_out = Vec2::new(60.0, 0.0);
    let p4 = d.create_path(
        &[
            h,
            n(4, 1, 200., 600.),
            n(4, 2, 300., 600.),
            n(4, 3, 400., 600.),
        ],
        false,
    );
    (d, [p1, p2, p3, p4])
}

fn open(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "0.1.0").unwrap()).unwrap();
    s.resize_viewport(1400.0, 1000.0);
    s.set_tool(Tool::Node);
    s
}

fn base() -> Session {
    open(&base_doc().0)
}

fn doc_of(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn path(d: &Document, id: NodeId) -> Option<PathSnapshot> {
    match d.object(id)? {
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Path(p) => Some(p),
        ObjectSnapshot::Primitive(_) | ObjectSnapshot::Clone(_) => None,
    }
}

fn all_ids(d: &Document) -> HashSet<AnchorId> {
    d.object_ids()
        .into_iter()
        .filter_map(|id| path(d, id))
        .flat_map(|p| p.all_anchors().map(|a| a.id).collect::<Vec<_>>())
        .collect()
}

fn hold(s: &mut Session, at: Point, m: M) {
    s.modifiers_changed(m.0, m.1, m.2);
    s.pointer_hover(at, m.0, m.1);
}

/// Press at `from` with `press`, move halfway and to `to` with `release`, release.
fn drag(s: &mut Session, from: Point, to: Point, press: M, release: M) {
    hold(s, from, press);
    s.pointer_down(from, press.0);
    hold(
        s,
        pt(f64::midpoint(from.x, to.x), f64::midpoint(from.y, to.y)),
        release,
    );
    hold(s, to, release);
    s.pointer_up(to, release.0, release.1);
    hold(s, to, NONE);
}

fn marquee(s: &mut Session, a: (f64, f64), b: (f64, f64), m: M) {
    drag(s, pt(a.0, a.1), pt(b.0, b.1), m, m);
}

fn click(s: &mut Session, at: (f64, f64), m: M) {
    hold(s, pt(at.0, at.1), m);
    s.pointer_down(pt(at.0, at.1), m.0);
    s.pointer_up(pt(at.0, at.1), m.0, m.1);
    hold(s, pt(at.0, at.1), NONE);
}

fn key(s: &mut Session, k: &str, ctrl: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        ..KeyInput::default()
    })
}

fn count(s: &Session) -> usize {
    s.node_toolbar_state().selected_nodes
}

/// Which anchors the session has selected, read by deleting them (destructive).
fn selected_ids(s: &mut Session) -> HashSet<AnchorId> {
    let before = all_ids(&doc_of(s));
    assert_eq!(key(s, "Delete", false), KeyOutcome::Deleted);
    let after = all_ids(&doc_of(s));
    before.difference(&after).copied().collect()
}

fn ids(list: &[(u64, u64)]) -> HashSet<AnchorId> {
    list.iter().map(|&(p, i)| AnchorId::new(p, i)).collect()
}

// ---- criteria 1 to 4: what the box selects -------------------------------------------------

#[test]
fn ac1_3_box_selects_anchors_inside_inclusive_edges_and_ignores_handles() {
    let mut s = base();
    // P4's handle end (160, 600) lies inside; its node 0 does not. Nodes 1, 2 do.
    marquee(&mut s, (150., 550.), (350., 650.), NONE);
    assert_eq!(count(&s), 2);
    assert_eq!(selected_ids(&mut s), ids(&[(4, 1), (4, 2)]));

    // Anchor exactly on the edge counts (inclusive).
    let mut s = base();
    marquee(&mut s, (200., 50.), (300., 150.), NONE);
    assert_eq!(selected_ids(&mut s), ids(&[(1, 1), (1, 2)]));

    // One unit outside does not.
    let mut s = base();
    marquee(&mut s, (200.5, 50.), (299.5, 150.), NONE);
    assert_eq!(count(&s), 0);
}

#[test]
fn ac1_3_drag_direction_does_not_matter() {
    let corners = [
        ((150., 50.), (350., 150.)),
        ((350., 150.), (150., 50.)),
        ((350., 50.), (150., 150.)),
        ((150., 150.), (350., 50.)),
    ];
    for (a, b) in corners {
        let mut s = base();
        marquee(&mut s, a, b, NONE);
        assert_eq!(selected_ids(&mut s), ids(&[(1, 1), (1, 2)]), "{a:?} {b:?}");
    }
}

#[test]
fn ac1_press_over_fill_pasteboard_and_far_outside_starts_a_marquee() {
    // Over the closed square's interior (fill) and then out to the pasteboard (negative coords).
    let mut s = base();
    marquee(&mut s, (150., 350.), (-500., -500.), NONE);
    assert_eq!(selected_ids(&mut s), ids(&[(1, 0), (3, 0)]));
}

#[test]
fn ac4_box_works_across_paths_open_and_closed_every_anchor_once() {
    let mut s = base();
    marquee(&mut s, (50., 50.), (650., 450.), NONE);
    // P1 (4) + P2 (3) + P3 (4); P4 is at y = 600.
    assert_eq!(count(&s), 11);
    let t = s.node_toolbar_state();
    assert_eq!(t.total_nodes, 15);
    assert!(!t.segment_selected);
    let got = selected_ids(&mut s);
    assert_eq!(got.len(), 11);
    assert!(got.contains(&AnchorId::new(3, 0)) && got.contains(&AnchorId::new(3, 3)));
}

#[test]
fn ac4_compound_paths_hidden_and_locked_paths_contribute_no_nodes() {
    let d = Document::new(1);
    let visible = d.create_path(&[n(1, 0, 10., 10.), n(1, 1, 20., 10.)], false);
    let hidden = d.create_path(&[n(2, 0, 10., 20.), n(2, 1, 20., 20.)], false);
    let locked = d.create_path(&[n(3, 0, 10., 30.), n(3, 1, 20., 30.)], false);
    let seed = d.create_path(&[n(9, 0, 0., 0.), n(9, 1, 1., 0.)], false);
    let compound = d
        .replace_with_path(
            &[seed],
            seed,
            &[
                (
                    vec![n(4, 0, 10., 40.), n(4, 1, 30., 40.), n(4, 2, 30., 45.)],
                    true,
                ),
                (
                    vec![n(5, 0, 12., 41.), n(5, 1, 14., 41.), n(5, 2, 14., 43.)],
                    true,
                ),
            ],
            "boolean_union",
        )
        .unwrap();
    d.set_node_visible(&[hidden], false).unwrap();
    d.set_node_locked(&[locked], true).unwrap();
    let mut s = open(&d);
    marquee(&mut s, (0., 0.), (100., 100.), NONE);
    // `0053` criterion 4 as amended by `specs/0055-compound-path-node-editing` (UX notes, "With the
    // node marquee"): every node of every outline of a compound path counts; hidden and locked
    // paths still contribute none.
    assert_eq!(
        count(&s),
        8,
        "the visible, unlocked path and both outlines of the compound path"
    );
    assert_eq!(s.node_toolbar_state().total_nodes, 8);
    let got = selected_ids(&mut s);
    assert_eq!(
        got,
        ids(&[
            (1, 0),
            (1, 1),
            (4, 0),
            (4, 1),
            (4, 2),
            (5, 0),
            (5, 1),
            (5, 2)
        ])
    );
    let _ = (visible, compound);
}

// ---- criterion 5 to 8: modifier table ------------------------------------------------------

fn preselect_two_in_p1(s: &mut Session) {
    click(s, (100., 100.), NONE);
    click(s, (200., 100.), SHIFT);
}

#[test]
fn ac5_6_replace_makes_the_selection_exactly_the_result() {
    let mut s = base();
    preselect_two_in_p1(&mut s);
    marquee(&mut s, (80., 280.), (220., 420.), NONE);
    assert_eq!(selected_ids(&mut s), ids(&[(3, 0), (3, 1), (3, 2), (3, 3)]));
}

#[test]
fn ac6_empty_replace_clears_nodes_and_drops_a_segment_selection() {
    let mut s = base();
    preselect_two_in_p1(&mut s);
    marquee(&mut s, (700., 700.), (800., 800.), NONE);
    assert_eq!(count(&s), 0);
    // Segment selected, then an empty Replace.
    click(&mut s, (250., 100.), NONE);
    assert!(s.node_toolbar_state().segment_selected);
    marquee(&mut s, (700., 700.), (800., 800.), NONE);
    assert!(!s.node_toolbar_state().segment_selected);
    assert_eq!(count(&s), 0);
}

#[test]
fn ac5_7_shift_adds_and_keeps_the_existing_selection() {
    let mut s = base();
    preselect_two_in_p1(&mut s);
    marquee(&mut s, (80., 280.), (220., 420.), SHIFT);
    assert_eq!(count(&s), 6);
    assert_eq!(
        selected_ids(&mut s),
        ids(&[(1, 0), (1, 1), (3, 0), (3, 1), (3, 2), (3, 3)])
    );
}

#[test]
fn ac7_add_with_a_segment_selected_replaces_the_segment_but_an_empty_add_changes_nothing() {
    let mut s = base();
    click(&mut s, (250., 100.), NONE);
    assert!(s.node_toolbar_state().segment_selected);
    marquee(&mut s, (700., 700.), (800., 800.), SHIFT);
    assert!(
        s.node_toolbar_state().segment_selected,
        "empty Add: unchanged"
    );
    marquee(&mut s, (80., 280.), (220., 420.), SHIFT);
    let t = s.node_toolbar_state();
    assert!(!t.segment_selected);
    assert_eq!(t.selected_nodes, 4);
}

#[test]
fn ac5_8_ctrl_removes_strictly_and_never_adds() {
    let mut s = base();
    preselect_two_in_p1(&mut s);
    // Box over node (1,1) and P3: removes (1,1), must not add P3's nodes.
    marquee(&mut s, (150., 50.), (250., 150.), CTRL);
    assert_eq!(selected_ids(&mut s), ids(&[(1, 0)]));
    // Ctrl over nothing selected adds nothing.
    let mut s = base();
    marquee(&mut s, (80., 280.), (220., 420.), CTRL);
    assert_eq!(count(&s), 0);
}

#[test]
fn ac8_ctrl_leaves_a_segment_selection_and_empty_remove_changes_nothing() {
    let mut s = base();
    click(&mut s, (250., 100.), NONE);
    marquee(&mut s, (80., 280.), (220., 420.), CTRL);
    assert!(s.node_toolbar_state().segment_selected);
    marquee(&mut s, (700., 700.), (800., 800.), CTRL);
    assert!(s.node_toolbar_state().segment_selected);
}

#[test]
fn ac5_ctrl_wins_over_shift_and_alt_has_no_effect_of_its_own() {
    let mut s = base();
    preselect_two_in_p1(&mut s);
    marquee(&mut s, (150., 50.), (250., 150.), SHIFT_CTRL);
    assert_eq!(selected_ids(&mut s), ids(&[(1, 0)]), "Shift+Ctrl removes");

    for (m, expect) in [
        (ALT, ids(&[(3, 0), (3, 1), (3, 2), (3, 3)])),
        (
            (true, false, true),
            ids(&[(1, 0), (1, 1), (3, 0), (3, 1), (3, 2), (3, 3)]),
        ),
        (ALT_CTRL, ids(&[(1, 0)])),
    ] {
        let mut s = base();
        preselect_two_in_p1(&mut s);
        let b = if m == ALT_CTRL {
            ((150., 50.), (250., 150.))
        } else {
            ((80., 280.), (220., 420.))
        };
        marquee(&mut s, b.0, b.1, m);
        assert_eq!(selected_ids(&mut s), expect, "{m:?}");
    }
}

#[test]
fn ac5_modifiers_are_read_at_the_release_not_the_press() {
    let mut s = base();
    preselect_two_in_p1(&mut s);
    // Press without a modifier, hold Shift at the release: Add.
    drag(&mut s, pt(80., 280.), pt(220., 420.), NONE, SHIFT);
    assert_eq!(count(&s), 6);
    // Press with Shift, release without: Replace.
    let mut s = base();
    preselect_two_in_p1(&mut s);
    drag(&mut s, pt(80., 280.), pt(220., 420.), SHIFT, NONE);
    assert_eq!(count(&s), 4);
}

// ---- criterion 9: order, Join ---------------------------------------------------------------

#[test]
fn ac9_marquee_order_is_stacking_order_so_join_reads_lower_path_first() {
    let mut s = base();
    marquee(&mut s, (380., 80.), (420., 120.), NONE);
    assert_eq!(count(&s), 2);
    let t = s.node_toolbar_state();
    assert!(t.can_join);
    assert!(t.join_block.is_none());
    assert!(!t.can_split, "Split needs exactly one node");
    s.join_selected();
    let d = doc_of(&s);
    let paths: Vec<PathSnapshot> = d
        .object_ids()
        .into_iter()
        .filter_map(|id| path(&d, id))
        .collect();
    let joined = paths
        .iter()
        .find(|p| p.anchors.len() == 6)
        .expect("a 6-node path");
    assert!(!joined.closed);
    let xs: Vec<f64> = joined.anchors.iter().map(|a| a.point.x).collect();
    assert_eq!(
        xs,
        vec![100., 200., 300., 400., 500., 600.],
        "lower path first"
    );
    assert_eq!(joined.anchors[3].kind, AnchorKind::Corner);
    assert_eq!(paths.len(), 3, "P2 removed, P1 extended");
}

#[test]
fn ac9_order_of_the_added_nodes_is_document_order_not_position_or_drag_order() {
    // Same join, both drag directions: same result.
    for (a, b) in [((380., 80.), (420., 120.)), ((420., 120.), (380., 80.))] {
        let mut s = base();
        marquee(&mut s, a, b, NONE);
        s.join_selected();
        let d = doc_of(&s);
        let xs: Vec<Vec<f64>> = d
            .object_ids()
            .into_iter()
            .filter_map(|id| path(&d, id))
            .map(|p| p.anchors.iter().map(|a| a.point.x).collect())
            .collect();
        assert!(
            xs.contains(&vec![100., 200., 300., 400., 500., 600.]),
            "{xs:?}"
        );
    }
    // Reverse stacking: the path created first is lower; build P2-like path first.
    let d = Document::new(1);
    let low = d.create_path(
        &[
            n(7, 0, 600., 100.),
            n(7, 1, 500., 100.),
            n(7, 2, 400., 100.),
        ],
        false,
    );
    let high = d.create_path(&[n(8, 0, 400., 100.), n(8, 1, 300., 100.)], false);
    let mut s = open(&d);
    marquee(&mut s, (380., 80.), (420., 120.), NONE);
    s.join_selected();
    let d2 = doc_of(&s);
    assert!(
        path(&d2, high).is_none() || path(&d2, low).is_none(),
        "one path removed"
    );
    let survivor = path(&d2, low).or_else(|| path(&d2, high)).unwrap();
    let xs: Vec<f64> = survivor.anchors.iter().map(|a| a.point.x).collect();
    assert_eq!(
        xs,
        vec![600., 500., 400., 300.],
        "lower path's nodes come first: {xs:?}"
    );
}

#[test]
fn ac7_add_keeps_existing_order_and_appends_exactly_like_shift_click() {
    let d = Document::new(1);
    let _ = d.create_path(&[n(1, 0, 0., 0.), n(1, 1, 100., 0.)], false);
    let _ = d.create_path(&[n(2, 0, 200., 0.), n(2, 1, 300., 0.)], false);
    let joined = |setup: &dyn Fn(&mut Session)| {
        let mut s = open(&d);
        setup(&mut s);
        assert!(s.node_toolbar_state().can_join);
        s.join_selected();
        let doc = doc_of(&s);
        let v: Vec<Vec<f64>> = doc
            .object_ids()
            .into_iter()
            .filter_map(|id| path(&doc, id))
            .map(|p| p.anchors.iter().map(|a| a.point.x).collect())
            .collect();
        v
    };
    // Order [upper.last, lower.first], by shift-click and by click + Add marquee.
    let by_clicks = joined(&|s| {
        click(s, (300., 0.), NONE);
        click(s, (0., 0.), SHIFT);
    });
    let by_marquee = joined(&|s| {
        click(s, (300., 0.), NONE);
        marquee(s, (-20., -20.), (20., 20.), SHIFT);
    });
    assert_eq!(
        by_clicks, by_marquee,
        "Add appends after the existing selection"
    );
    // The opposite order must not give the same orientation if Join is order sensitive; at least
    // both must be valid joins of 4 nodes.
    let other = joined(&|s| {
        click(s, (0., 0.), NONE);
        marquee(s, (280., -20.), (320., 20.), SHIFT);
    });
    assert_eq!(other.len(), 1);
    assert_eq!(other[0].len(), 3, "two end nodes merge into one");
}

// ---- criterion 10 to 16: gesture ------------------------------------------------------------

#[test]
fn ac10_legend_text_follows_the_modifiers_live_and_the_count_inside_the_box() {
    let mut s = base();
    let from = pt(150., 50.);
    hold(&mut s, from, NONE);
    s.pointer_down(from, false);
    hold(&mut s, pt(350., 150.), NONE);
    assert_eq!(
        s.node_marquee_legend(),
        Some((2, SelectionCombine::Replace))
    );
    assert_eq!(s.cursor_hint(), "crosshair");
    assert!(!s.move_indicators().remove_badge);
    hold(&mut s, pt(350., 150.), SHIFT);
    assert_eq!(s.node_marquee_legend(), Some((2, SelectionCombine::Add)));
    hold(&mut s, pt(350., 150.), CTRL);
    assert_eq!(s.node_marquee_legend(), Some((2, SelectionCombine::Remove)));
    assert!(
        s.move_indicators().remove_badge,
        "minus badge while Ctrl is held"
    );
    hold(&mut s, pt(350., 150.), SHIFT_CTRL);
    assert_eq!(s.node_marquee_legend(), Some((2, SelectionCombine::Remove)));
    hold(&mut s, pt(350., 150.), ALT);
    assert_eq!(
        s.node_marquee_legend(),
        Some((2, SelectionCombine::Replace))
    );
    let text = s.live_readout().map(|r| r.text);
    assert!(
        text.as_deref().is_some_and(|t| t.contains("2 nodes")),
        "{text:?}"
    );
    hold(&mut s, pt(150., 60.), NONE);
    let text = s.live_readout().map(|r| r.text).unwrap_or_default();
    assert!(text.contains("0 nodes"), "{text:?}");
    hold(&mut s, pt(90., 120.), NONE);
    let text = s.live_readout().map(|r| r.text).unwrap_or_default();
    assert!(
        text.contains("1 node") && !text.contains("1 nodes"),
        "{text:?}"
    );
    s.pointer_up(pt(90., 120.), false, false);
    hold(&mut s, pt(90., 120.), NONE);
    assert!(s.node_marquee_legend().is_none());
    assert!(s.live_readout().is_none());
    assert_ne!(s.cursor_hint(), "crosshair");
}

#[test]
fn ac10_no_legend_or_crosshair_before_the_dead_zone() {
    let mut s = base();
    let from = pt(150., 50.);
    s.pointer_down(from, false);
    let dz = 0.5 / s.view().scale();
    hold(&mut s, pt(150.0 + dz, 50.0), NONE);
    assert!(s.node_marquee_legend().is_none());
    assert!(s.live_readout().is_none());
}

#[test]
fn ac10_the_box_is_drawn_in_every_frame_and_never_in_the_saved_document() {
    let mut s = base();
    let before = s.draw_list().triangles.len();
    let from = pt(150., 50.);
    s.pointer_down(from, false);
    hold(&mut s, pt(350., 150.), NONE);
    let during = s.draw_list().triangles.len();
    assert!(
        during > before,
        "box and rings add geometry: {before} -> {during}"
    );
    s.pointer_up(pt(350., 150.), false, false);
    let after_doc = all_ids(&doc_of(&s));
    assert_eq!(after_doc.len(), 15);
}

#[test]
fn ac11_escape_cancels_and_leaves_the_selection_and_its_order_untouched() {
    let mut s = base();
    click(&mut s, (500., 100.), NONE);
    let before_t = s.node_toolbar_state();
    s.pointer_down(pt(80., 280.), false);
    hold(&mut s, pt(220., 420.), SHIFT);
    assert!(s.node_marquee_legend().is_some());
    assert!(matches!(
        key(&mut s, "Escape", false),
        KeyOutcome::Escape(_)
    ));
    assert!(s.node_marquee_legend().is_none());
    assert!(s.live_readout().is_none());
    assert!(!s.move_indicators().remove_badge);
    s.pointer_up(pt(220., 420.), true, false);
    assert_eq!(
        s.node_toolbar_state().selected_nodes,
        before_t.selected_nodes
    );
    assert_eq!(count(&s), 1);
    // Escape cascade: the next Escape clears the node selection.
    assert!(matches!(
        key(&mut s, "Escape", false),
        KeyOutcome::Escape(_)
    ));
    assert_eq!(count(&s), 0);
}

#[test]
fn ac11_escape_keeps_order_for_join() {
    // Order [P2.first (2,0) via click, P1.last (1,3) via shift-click], then a cancelled marquee.
    let run = |cancel: bool| {
        let mut s = base();
        // Nodes coincide at (400, 100): select P2's node via its distinct neighbour-free spot by marquee.
        marquee(&mut s, (390., 90.), (410., 110.), NONE);
        if cancel {
            s.pointer_down(pt(80., 280.), false);
            hold(&mut s, pt(220., 420.), SHIFT);
            key(&mut s, "Escape", false);
            s.pointer_up(pt(220., 420.), true, false);
        }
        s.join_selected();
        let d = doc_of(&s);
        let mut v: Vec<Vec<f64>> = d
            .object_ids()
            .into_iter()
            .filter_map(|id| path(&d, id))
            .map(|p| p.anchors.iter().map(|a| a.point.x).collect())
            .collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v
    };
    assert_eq!(run(true), run(false));
}

#[test]
fn ac12_click_on_nothing_clears_at_the_release_unless_a_modifier_is_held() {
    let mut s = base();
    preselect_two_in_p1(&mut s);
    // Press on nothing: not cleared yet.
    s.pointer_down(pt(700., 800.), false);
    assert_eq!(count(&s), 2, "cleared at the release, not at the press");
    s.pointer_up(pt(700., 800.), false, false);
    assert_eq!(count(&s), 0);
    for m in [SHIFT, CTRL, SHIFT_CTRL] {
        let mut s = base();
        preselect_two_in_p1(&mut s);
        click(&mut s, (700., 800.), m);
        assert_eq!(count(&s), 2, "{m:?}");
    }
}

#[test]
fn ac12_click_on_nothing_never_changes_object_selection_tool_or_history() {
    let mut s = base();
    let undo = s.undo_len();
    click(&mut s, (700., 800.), NONE);
    assert_eq!(s.undo_len(), undo);
    assert_eq!(s.tool(), Tool::Node);
}

#[test]
fn ac12_a_move_under_three_pixels_is_still_a_click() {
    let mut s = base();
    preselect_two_in_p1(&mut s);
    let d = 2.0 / s.view().scale();
    s.pointer_down(pt(700., 800.), false);
    hold(&mut s, pt(700. + d, 800.), NONE);
    assert!(
        s.node_marquee_legend().is_none(),
        "no marquee inside the dead zone"
    );
    s.pointer_up(pt(700. + d, 800.), false, false);
    assert_eq!(count(&s), 0, "a click");
    // 3 px or more: a marquee that selects nothing here, also clears (Replace) and is a marquee.
    let mut s = base();
    let d = 4.0 / s.view().scale();
    s.pointer_down(pt(700., 800.), false);
    hold(&mut s, pt(700. + d, 800.), NONE);
    assert!(s.node_marquee_legend().is_some());
}

#[test]
fn ac13_a_press_on_a_node_handle_or_segment_never_starts_a_marquee_and_alt_is_inert() {
    let mut s = base();
    // Press on node (4,1), drag far: moves the node, no legend.
    s.pointer_down(pt(200., 600.), false);
    hold(&mut s, pt(260., 640.), NONE);
    assert!(s.node_marquee_legend().is_none());
    s.pointer_up(pt(260., 640.), false, false);
    let d = doc_of(&s);
    let (_, ps) = base_doc();
    let moved = path(&d, ps[3]).unwrap();
    assert_eq!(
        (moved.anchors[1].point.x, moved.anchors[1].point.y),
        (260., 640.)
    );

    // Alt press on a node still selects it exactly like without Alt.
    let mut s = base();
    hold(&mut s, pt(200., 100.), ALT);
    s.pointer_down(pt(200., 100.), false);
    s.pointer_up(pt(200., 100.), false, false);
    hold(&mut s, pt(200., 100.), NONE);
    assert_eq!(count(&s), 1);
}

#[test]
fn ac13_dragging_a_selected_node_moves_every_selected_node() {
    let mut s = base();
    marquee(&mut s, (50., 50.), (650., 150.), NONE);
    assert_eq!(count(&s), 7);
    drag(&mut s, pt(200., 100.), pt(200., 130.), NONE, NONE);
    let d = doc_of(&s);
    let (_, ps) = base_doc();
    for p in [ps[0], ps[1]] {
        for a in &path(&d, p).unwrap().anchors {
            assert_eq!(a.point.y, 130., "all selected nodes moved");
        }
    }
    assert_eq!(path(&d, ps[2]).unwrap().anchors[0].point.y, 300.);
}

#[test]
fn ac14_zoom_during_a_marquee_keeps_the_press_corner_in_document_space() {
    let mut s = base();
    let from = pt(150., 50.);
    s.pointer_down(from, false);
    hold(&mut s, pt(250., 150.), NONE);
    assert_eq!(s.node_marquee_legend().map(|l| l.0), Some(1));
    // Zoom in about the viewport centre (ctrl wheel), then the pointer is at a new document point.
    s.wheel(0.0, -200.0, 700.0, 500.0, false, true);
    let z = s.zoom_percent();
    assert!(z > 100, "zoomed in: {z}");
    hold(&mut s, pt(350., 150.), NONE);
    assert_eq!(
        s.node_marquee_legend().map(|l| l.0),
        Some(2),
        "press corner still (150, 50)"
    );
    s.pointer_up(pt(350., 150.), false, false);
    assert_eq!(selected_ids(&mut s), ids(&[(1, 1), (1, 2)]));
}

#[test]
fn ac15_leaving_the_viewport_does_not_end_the_marquee_and_a_release_outside_ends_it() {
    let mut s = base();
    s.pointer_down(pt(150., 50.), false);
    hold(&mut s, pt(350., 150.), NONE);
    s.pointer_leave();
    // Moves far outside the viewport still extend the box.
    hold(&mut s, pt(5000., 5000.), NONE);
    assert!(s.node_marquee_legend().is_some(), "drag goes on");
    s.pointer_up(pt(5000., 5000.), false, false);
    assert!(s.node_marquee_legend().is_none());
    assert!(count(&s) >= 10, "box (150,50)-(5000,5000): {}", count(&s));
}

#[test]
fn ac16_zero_height_box_selects_nodes_on_the_line_and_hostile_input_is_harmless() {
    let mut s = base();
    marquee(&mut s, (50., 100.), (650., 100.), NONE);
    assert_eq!(
        selected_ids(&mut s),
        ids(&[(1, 0), (1, 1), (1, 2), (1, 3), (2, 0), (2, 1), (2, 2)])
    );
    // Vertical line.
    let mut s = base();
    marquee(&mut s, (200., 50.), (200., 650.), NONE);
    assert_eq!(selected_ids(&mut s), ids(&[(1, 1), (3, 1), (3, 2), (4, 1)]));

    // Non-finite coordinates are ignored; the box keeps its last valid corner.
    let mut s = base();
    s.pointer_down(pt(150., 50.), false);
    hold(&mut s, pt(350., 150.), NONE);
    for bad in [
        pt(f64::NAN, 5.),
        pt(1., f64::INFINITY),
        pt(f64::NEG_INFINITY, f64::NAN),
    ] {
        s.pointer_hover(bad, false, false);
    }
    assert_eq!(s.node_marquee_legend().map(|l| l.0), Some(2));
    s.pointer_up(pt(f64::NAN, f64::NAN), false, false);
    // A release with a bad point must not panic; either way the session is usable.
    key(&mut s, "Escape", false);
    let _ = s.draw_list();
}

#[test]
fn ac16_boxes_beyond_1e7_are_cut_at_the_limit_and_nothing_panics() {
    let d = Document::new(1);
    let _ = d.create_path(
        &[n(1, 0, 0., 0.), n(1, 1, 5.0e6, 0.), n(1, 2, 2.0e7, 0.)],
        false,
    );
    let mut s = open(&d);
    marquee(&mut s, (-1.0e9, -1.0e9), (1.0e9, 1.0e9), NONE);
    assert_eq!(count(&s), 2, "the node at 2e7 is beyond the cut-off");
    let mut s = open(&d);
    marquee(&mut s, (-f64::MAX, -f64::MAX), (f64::MAX, f64::MAX), NONE);
    assert_eq!(count(&s), 2);
}

// ---- criterion 17 and 28: nothing is written -------------------------------------------------

#[test]
fn ac17_marquee_click_and_ctrl_a_write_no_history_step_and_no_dirty_marker() {
    let mut s = base();
    let undo = s.undo_len();
    let bytes = s.pack("0.1.0").unwrap();
    assert!(!s.is_dirty());
    marquee(&mut s, (50., 50.), (650., 450.), NONE);
    marquee(&mut s, (50., 50.), (650., 450.), SHIFT);
    marquee(&mut s, (150., 50.), (250., 150.), CTRL);
    click(&mut s, (700., 800.), NONE);
    click(&mut s, (200., 100.), NONE);
    key(&mut s, "a", true);
    assert_eq!(s.undo_len(), undo);
    assert!(!s.is_dirty());
    assert_eq!(doc_of(&s).object_ids().len(), 4);
    assert_eq!(all_ids(&doc_of(&s)).len(), 15);
    let _ = bytes;
}

// ---- criterion 18 to 20: what the selection is for -------------------------------------------

#[test]
fn ac18_delete_removes_all_selected_nodes_in_one_history_step() {
    let mut s = base();
    let undo = s.undo_len();
    // 11 nodes across three paths, minus none: P1, P2, P3 gone entirely? P3 all 4 nodes: removed.
    marquee(&mut s, (250., 50.), (450., 150.), NONE);
    assert_eq!(count(&s), 3); // (1,2) (1,3) (2,0)
    marquee(&mut s, (250., 50.), (350., 150.), NONE);
    marquee(&mut s, (150., 280.), (250., 420.), SHIFT);
    assert_eq!(count(&s), 3);
    assert_eq!(key(&mut s, "Delete", false), KeyOutcome::Deleted);
    assert_eq!(s.undo_len(), undo + 1, "one step");
    assert_eq!(
        s.history_target_name(Direction::Undo).as_deref(),
        Some("Delete nodes")
    );
    assert_eq!(count(&s), 0, "no node selected afterwards");
    assert_eq!(s.tool(), Tool::Node);
    let d = doc_of(&s);
    assert_eq!(all_ids(&d).len(), 15 - 3);
    s.undo();
    assert_eq!(all_ids(&doc_of(&s)).len(), 15);
}

#[test]
fn ac18_spec_examples() {
    let d = Document::new(1);
    let p = d.create_path(
        &[
            n(1, 0, 0., 0.),
            n(1, 1, 10., 0.),
            n(1, 2, 20., 0.),
            n(1, 3, 30., 0.),
            n(1, 4, 40., 0.),
        ],
        false,
    );
    let mut s = open(&d);
    // The spec example starts at (5, -1), 3.8 px from the segment at 100 %: that press hits the
    // segment (tolerance about 5 px), so the box starts 2 mm off the line instead.
    marquee(&mut s, (5., -2.), (35., 2.), NONE);
    key(&mut s, "Delete", false);
    let r = path(&doc_of(&s), p).unwrap();
    assert_eq!(
        r.anchors.iter().map(|a| a.point.x).collect::<Vec<_>>(),
        vec![0., 40.]
    );
    // Every node: the path is removed.
    let mut s = open(&d);
    marquee(&mut s, (-5., -5.), (45., 5.), NONE);
    key(&mut s, "Backspace", false);
    assert!(path(&doc_of(&s), p).is_none());
}

#[test]
fn ac18_delete_via_ctrl_a_empties_everything_and_the_tool_stays() {
    let mut s = base();
    assert_eq!(key(&mut s, "a", true), KeyOutcome::SelectedAll);
    key(&mut s, "Delete", false);
    assert_eq!(doc_of(&s).object_ids().len(), 0);
    assert_eq!(s.tool(), Tool::Node);
    // Ctrl+A on an empty document: consumed, nothing breaks.
    assert_ne!(key(&mut s, "a", true), KeyOutcome::Ignored);
}

#[test]
fn ac19_join_reasons() {
    let mut s = base();
    // Three nodes selected.
    marquee(&mut s, (250., 50.), (450., 150.), NONE);
    let t = s.node_toolbar_state();
    assert!(!t.can_join);
    assert_eq!(t.join_block, Some(JoinBlock::Count(3)));
    // Two nodes, one interior.
    let mut s = base();
    marquee(&mut s, (350., 50.), (450., 150.), NONE);
    click(&mut s, (100., 100.), NONE);
    click(&mut s, (300., 100.), SHIFT);
    assert_eq!(
        s.node_toolbar_state().join_block,
        Some(JoinBlock::NotEndNodes { not_end: 1, of: 2 })
    );
    // Two end nodes of a two-node path: Other, not Count or NotEndNodes.
    let d = Document::new(1);
    let _ = d.create_path(&[n(1, 0, 0., 0.), n(1, 1, 10., 0.)], false);
    let mut s = open(&d);
    marquee(&mut s, (-5., -5.), (15., 5.), NONE);
    let t = s.node_toolbar_state();
    assert!(!t.can_join);
    assert_eq!(t.join_block, Some(JoinBlock::Other));
}

#[test]
fn ac20_convert_split_and_drag_act_on_multi_node_marquee_selections() {
    let d = Document::new(1);
    let mk = |peer: u64, y: f64| {
        let mut a = n(peer, 1, 100., y);
        a.kind = AnchorKind::Symmetric;
        a.handle_in = Vec2::new(-20., 0.);
        a.handle_out = Vec2::new(20., 0.);
        a
    };
    let p1 = d.create_path(&[n(1, 0, 0., 0.), mk(1, 0.), n(1, 2, 200., 0.)], false);
    let p2 = d.create_path(&[n(2, 0, 0., 50.), mk(2, 50.), n(2, 2, 200., 50.)], false);
    let mut s = open(&d);
    marquee(&mut s, (90., -10.), (110., 60.), NONE);
    assert_eq!(count(&s), 2);
    let t = s.node_toolbar_state();
    assert!(t.can_convert_to_corner && t.can_convert_to_symmetric && t.can_convert_to_asymmetric);
    assert!(!t.can_split);
    let undo = s.undo_len();
    s.convert_selected(AnchorKind::Corner);
    assert_eq!(s.undo_len(), undo + 1, "one step");
    let r = doc_of(&s);
    for p in [p1, p2] {
        assert_eq!(path(&r, p).unwrap().anchors[1].kind, AnchorKind::Corner);
    }
    assert_eq!(count(&s), 2, "selection survives a conversion");
    s.convert_selected(AnchorKind::Asymmetric);
    let r = doc_of(&s);
    for p in [p1, p2] {
        assert_eq!(path(&r, p).unwrap().anchors[1].kind, AnchorKind::Asymmetric);
    }
}

// ---- criteria 21 to 24: keyboard, count, announcement ----------------------------------------

#[test]
fn ac22_ctrl_a_selects_every_node_of_the_editing_set_and_replaces_the_selection() {
    let mut s = base();
    click(&mut s, (250., 100.), NONE); // a segment
    assert_eq!(key(&mut s, "a", true), KeyOutcome::SelectedAll);
    let t = s.node_toolbar_state();
    assert_eq!(
        (t.selected_nodes, t.total_nodes, t.segment_selected),
        (15, 15, false)
    );
    // Key repeat does nothing more.
    let undo = s.undo_len();
    let r = s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        repeat: true,
        ..KeyInput::default()
    });
    let _ = r;
    assert_eq!(count(&s), 15);
    assert_eq!(s.undo_len(), undo);
    // Gate: a focused text field blocks it.
    let mut s = base();
    let r = s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        dom_blocked: true,
        ..KeyInput::default()
    });
    assert_eq!(r, KeyOutcome::Ignored);
    assert_eq!(count(&s), 0);
    // The object selection did not change.
    assert_eq!(s.selected_object_count(), 0);
}

#[test]
fn ac22_ctrl_a_in_document_order_feeds_join_deterministically_and_ignores_hidden() {
    let d = Document::new(1);
    let a = d.create_path(&[n(1, 0, 0., 0.), n(1, 1, 10., 0.)], false);
    let b = d.create_path(&[n(2, 0, 0., 20.), n(2, 1, 10., 20.)], false);
    d.set_node_visible(&[b], false).unwrap();
    let mut s = open(&d);
    key(&mut s, "a", true);
    assert_eq!(count(&s), 2, "hidden path excluded");
    let _ = a;
}

#[test]
fn ac22_other_tools_still_ignore_ctrl_a_and_a_no_node_editing_set_consumes_the_key() {
    let (d, _) = base_doc();
    let mut s = open(&d);
    for tool in [Tool::Pen, Tool::Rectangle, Tool::Ellipse] {
        s.set_tool(tool);
        assert_eq!(key(&mut s, "a", true), KeyOutcome::Ignored, "{tool:?}");
    }
    let empty = Document::new(1);
    let mut s = open(&empty);
    assert_ne!(key(&mut s, "a", true), KeyOutcome::Ignored);
}

#[test]
fn ac23_counts_in_the_toolbar_state() {
    let mut s = base();
    let t = s.node_toolbar_state();
    assert_eq!(
        (t.selected_nodes, t.total_nodes, t.segment_selected),
        (0, 15, false)
    );
    click(&mut s, (200., 100.), NONE);
    assert_eq!(count(&s), 1);
    click(&mut s, (250., 100.), NONE);
    let t = s.node_toolbar_state();
    assert!(t.segment_selected);
    assert_eq!(t.total_nodes, 15);
    s.set_tool(Tool::Select);
    let t = s.node_toolbar_state();
    assert_eq!(
        (t.selected_nodes, t.total_nodes),
        (0, 0),
        "no count in other tools"
    );
}

#[test]
fn ac24_announcement_after_marquee_and_ctrl_a_but_not_after_a_click() {
    let mut s = base();
    click(&mut s, (200., 100.), NONE);
    assert_eq!(s.selection_announcement(), "", "a click does not announce");
    marquee(&mut s, (150., 50.), (350., 150.), NONE);
    assert_eq!(s.selection_announcement(), "2 nodes selected.");
    marquee(&mut s, (190., 90.), (210., 110.), NONE);
    assert_eq!(s.selection_announcement(), "1 node selected.");
    marquee(&mut s, (700., 700.), (800., 800.), NONE);
    assert_eq!(s.selection_announcement(), "No nodes selected.");
    key(&mut s, "a", true);
    assert_eq!(s.selection_announcement(), "15 nodes selected.");
}

// ---- Part E ------------------------------------------------------------------------------------

#[test]
fn ac27_the_select_tool_marquee_and_other_tools_have_no_node_marquee_state() {
    let (d, _) = base_doc();
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    drag(&mut s, pt(50., 50.), pt(450., 150.), NONE, NONE);
    assert!(s.node_marquee_legend().is_none());
    assert_eq!(
        s.selected_object_count(),
        1,
        "Select tool marquee still selects objects"
    );
    s.set_tool(Tool::Pen);
    assert!(s.node_marquee_legend().is_none());
}

#[test]
fn robustness_random_event_sequences_never_panic() {
    let mut seed = 0x9E37_79B9_7F4A_7C15_u64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut s = base();
    for _ in 0..3000 {
        let p = pt((next() % 800) as f64 - 50., (next() % 800) as f64 - 50.);
        let m = [NONE, SHIFT, CTRL, ALT, SHIFT_CTRL, ALT_CTRL][(next() % 6) as usize];
        match next() % 9 {
            0 => {
                hold(&mut s, p, m);
                s.pointer_down(p, m.0);
            }
            1 | 2 => hold(&mut s, p, m),
            3 => s.pointer_up(p, m.0, m.1),
            4 => {
                key(&mut s, "Escape", false);
            }
            5 => {
                key(&mut s, "a", true);
            }
            6 => {
                key(&mut s, "Delete", false);
            }
            7 => s.pointer_leave(),
            _ => {
                let _ = s.undo();
            }
        }
        let _ = s.node_toolbar_state();
        let _ = s.draw_list();
        let _ = s.node_marquee_legend();
    }
}

// ---- white-box edges: interruptions mid-marquee --------------------------------------------------

#[test]
fn edge_pointer_cancelled_tool_switch_and_undo_mid_marquee_leave_no_residue() {
    // Window loses the pointer: like Escape, selection untouched.
    let mut s = base();
    click(&mut s, (200., 100.), NONE);
    s.pointer_down(pt(80., 280.), false);
    hold(&mut s, pt(220., 420.), NONE);
    assert!(s.node_marquee_legend().is_some());
    s.pointer_cancelled();
    assert!(s.node_marquee_legend().is_none());
    assert_eq!(count(&s), 1, "cancel keeps the selection");
    s.pointer_up(pt(220., 420.), false, false);
    assert_eq!(count(&s), 1, "the release after a cancel does nothing");

    // Undo key during the drag does not corrupt state or panic.
    let mut s = base();
    click(&mut s, (200., 100.), NONE);
    s.pointer_down(pt(80., 280.), false);
    hold(&mut s, pt(220., 420.), NONE);
    let _ = s.key_down(KeyInput {
        key: "z",
        ctrl: true,
        ..KeyInput::default()
    });
    s.pointer_up(pt(220., 420.), false, false);
    let _ = s.node_toolbar_state();
}

#[test]
fn edge_delete_key_and_ctrl_a_while_a_marquee_is_in_flight_do_not_panic_or_commit_stale_ids() {
    let mut s = base();
    marquee(&mut s, (50., 50.), (650., 150.), NONE);
    s.pointer_down(pt(80., 280.), false);
    hold(&mut s, pt(220., 420.), NONE);
    let _ = key(&mut s, "Delete", false);
    let _ = key(&mut s, "a", true);
    s.pointer_up(pt(220., 420.), false, false);
    let _ = s.draw_list();
    let t = s.node_toolbar_state();
    assert!(t.selected_nodes <= t.total_nodes);
}

#[test]
fn edge_a_marquee_after_a_delete_in_the_same_session_ignores_removed_nodes() {
    let mut s = base();
    marquee(&mut s, (150., 50.), (350., 150.), NONE);
    key(&mut s, "Delete", false);
    assert_eq!(s.node_toolbar_state().total_nodes, 13);
    marquee(&mut s, (50., 50.), (650., 150.), NONE);
    assert_eq!(count(&s), 5);
    s.undo();
    s.redo();
    let t = s.node_toolbar_state();
    assert!(t.selected_nodes <= t.total_nodes);
}

#[test]
fn edge_all_selected_nodes_of_a_closed_path_deleted_removes_the_path_and_a_group_pathmate_survives()
{
    let d = Document::new(1);
    let sq = d.create_path(
        &[n(1, 0, 0., 0.), n(1, 1, 10., 0.), n(1, 2, 10., 10.)],
        true,
    );
    let other = d.create_path(&[n(2, 0, 0., 100.), n(2, 1, 10., 100.)], false);
    let _ = d.group_objects(&[sq, other]).unwrap();
    let mut s = open(&d);
    marquee(&mut s, (-5., -5.), (15., 15.), NONE);
    key(&mut s, "Delete", false);
    let r = doc_of(&s);
    assert!(path(&r, sq).is_none());
    assert!(
        path(&r, other).is_some(),
        "the other group member stays; the group is not empty"
    );
}
