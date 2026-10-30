//! What `Session` shows and does for the Node tool's marquee and Ctrl+A
//! (`specs/0053-node-marquee-selection/`): the release, the box, the legend, the cursor and the
//! announcement, read from the marquee the Node tool holds.

use curvyo_document_core::{PathSnapshot, Point};
use curvyo_render_core::MarqueeOverlay;
use curvyo_ui_core::{
    Modifiers, NodePointerUpOutcome, NodeSelection, SelectionCombine, hit_test, hit_test_object,
    node_marquee_legend_text, nodes_selected_announcement,
};

use super::keys::KeyOutcome;
use super::shapes::LiveReadout;
use super::{Session, Tool};

/// One node announcement: its text, its sequence number and the node selection it describes.
/// Once the selection differs (Escape, a click, Delete, undo, another tool) it is stale and the
/// live region is emptied (UX review of 2026-10-14, finding 2).
#[derive(Debug, Clone)]
pub(super) struct NodeAnnouncement {
    text: String,
    seq: u32,
    selection: NodeSelection,
}

impl Session {
    /// The release in the Node tool: the release point reaches the drag first (a marquee's box
    /// and inside list end there), then the tool ends it. A marquee that changed the selection
    /// sets the announcement (criterion 24).
    pub(super) fn node_pointer_up(&mut self, point: Point, modifiers: Modifiers) {
        let paths = self.node_paths();
        let threshold = self.hit_tolerances().drag_threshold;
        self.node
            .pointer_moved(&paths, point, modifiers.shift, threshold);
        if self.node.marquee().is_some_and(|marquee| !marquee.armed()) {
            self.node_click_on_nothing(point, modifiers);
        }
        let correction = self.resolve_node_snapping(point);
        let outcome = self
            .node
            .pointer_up(&self.document, point.translated(correction), modifiers);
        self.end_snapping_drag();
        if outcome == NodePointerUpOutcome::NodesSelected {
            self.announce_node_selection();
        }
    }

    /// A click that hit no node, handle or segment of the editing set (customer 2026-10-26, as
    /// Inkscape's node tool): on a path of the context it selects that path, Shift adds it; on
    /// anything else a plain click clears the selection, so every path shows its nodes again.
    fn node_click_on_nothing(&mut self, point: Point, modifiers: Modifiers) {
        if modifiers.ctrl {
            return;
        }
        let paths = self.paths();
        let hit = hit_test_object(&self.pickable_objects(), point, self.object_tolerance())
            .filter(|id| paths.iter().any(|path| path.id == *id));
        match (hit, modifiers.shift) {
            (Some(id), true) => self.selection.add(id),
            (Some(id), false) => self.selection.select_single(id),
            (None, true) => {}
            (None, false) => self.selection.clear(),
        }
    }

    /// Whether a press at `point` would hit no node, handle or segment of `paths`, so it would
    /// start a node marquee.
    pub(super) fn node_hit_is_nothing(&self, paths: &[PathSnapshot], point: Point) -> bool {
        hit_test(
            paths,
            self.node.selection(),
            point,
            self.point_tolerance(),
            self.handle_tolerance(),
            self.segment_tolerance(),
        )
        .is_none()
    }

    /// Ctrl+A in the Node tool (criterion 22): every node of the editing set, in document
    /// order, replacing the selection. Consumed also when there is no node. View state only.
    pub(super) fn node_select_all(&mut self) -> KeyOutcome {
        let paths = self.node_paths();
        self.node.select_all(&paths);
        self.announce_node_selection();
        KeyOutcome::SelectedAll
    }

    fn announce_node_selection(&mut self) {
        let selection = self.node.selection().clone();
        self.node_announcement_seq = self.node_announcement_seq.wrapping_add(1);
        self.node_announcement = Some(NodeAnnouncement {
            text: nodes_selected_announcement(selection.node_pairs().len()),
            seq: self.node_announcement_seq,
            selection,
        });
    }

    /// The announcement that still describes the current node selection in the Node tool.
    fn live_node_announcement(&self) -> Option<&NodeAnnouncement> {
        if self.tool != Tool::Node {
            return None;
        }
        self.node_announcement
            .as_ref()
            .filter(|a| &a.selection == self.node.selection())
    }

    /// The polite live-region text after a node marquee or Ctrl+A, empty once the node
    /// selection has changed another way (criterion 24).
    #[must_use]
    pub fn node_selection_announcement(&self) -> String {
        self.live_node_announcement()
            .map(|a| a.text.clone())
            .unwrap_or_default()
    }

    /// The sequence number of [`Session::selection_announcement`]'s text: it changes with every
    /// node announcement, so the host speaks the same sentence again ("4 nodes selected." after
    /// a second Ctrl+A). 0 when there is none.
    #[must_use]
    pub fn selection_announcement_seq(&self) -> u32 {
        self.live_node_announcement().map_or(0, |a| a.seq)
    }

    /// The box of an armed node marquee, always in the contain colour (criterion 2).
    pub(super) fn node_marquee_overlay(&self) -> Option<MarqueeOverlay> {
        if self.tool != Tool::Node {
            return None;
        }
        let marquee = self.node.marquee().filter(|m| m.armed())?;
        let (from, to) = marquee.corners();
        Some(MarqueeOverlay::Box {
            from,
            to,
            contain: true,
        })
    }

    /// The count inside an armed node marquee and how a release now would combine it, from the
    /// held modifiers (criterion 10).
    #[must_use]
    pub fn node_marquee_legend(&self) -> Option<(usize, SelectionCombine)> {
        if self.tool != Tool::Node {
            return None;
        }
        let marquee = self.node.marquee().filter(|m| m.armed())?;
        Some((
            marquee.inside().len(),
            SelectionCombine::from_modifiers(self.held.shift, self.held.ctrl),
        ))
    }

    /// The legend at the pointer: "5 nodes · +Add".
    pub(super) fn node_marquee_readout(&self) -> Option<LiveReadout> {
        let (count, combine) = self.node_marquee_legend()?;
        Some(LiveReadout {
            text: node_marquee_legend_text(count, combine),
            anchor: self.pointer_position?,
        })
    }

    /// The crosshair from the press on nothing to the release (UX notes).
    pub(super) fn node_marquee_cursor(&self) -> Option<&'static str> {
        self.node.marquee().map(|_| "crosshair")
    }

    /// The minus badge: Ctrl held during a node marquee, or with no button down where a press
    /// would start one (UX notes).
    pub(super) fn node_remove_badge(&self) -> bool {
        self.held.ctrl
            && (self.node.marquee().is_some()
                || (!self.node.drag_in_flight() && !self.button_down && self.node_hover_on_nothing))
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{AnchorId, NewAnchor, NodeId};
    use curvyo_ui_core::NodeToolbarState;

    use super::*;
    use crate::session::{EscapeStep, KeyInput};

    fn pt(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn line(session: &Session, tag: u64, xs: &[f64]) -> NodeId {
        let anchors: Vec<NewAnchor> = xs
            .iter()
            .zip(0_u64..)
            .map(|(&x, i)| NewAnchor::corner(AnchorId::new(tag, i), pt(x, 0.0)))
            .collect();
        session.document.create_path(&anchors, false)
    }

    /// Criterion 9's example through the session, scaled by ten so the press stays clear of
    /// the node hit radius at the session's default zoom: P1 (lower) and P2 (upper) meet at
    /// (100, 0); a marquee there selects P1's last node, then P2's first, so the session passes
    /// the editing set bottom to top, and Join keeps P1.
    fn node_session() -> (Session, NodeId, NodeId) {
        let mut session = Session::new(1);
        let p1 = line(&session, 1, &[0.0, 100.0]);
        let p2 = line(&session, 2, &[100.0, 200.0]);
        session.set_tool(Tool::Node);
        (session, p1, p2)
    }

    fn drag(session: &mut Session, from: Point, to: Point, shift: bool, ctrl: bool) {
        session.modifiers_changed(shift, ctrl, false);
        session.pointer_down(from, shift);
        session.pointer_hover(to, shift, ctrl);
        session.pointer_up(to, shift, ctrl);
    }

    #[test]
    fn a_marquee_selects_in_stacking_order_and_join_keeps_the_lower_path() {
        let (mut session, p1, p2) = node_session();
        drag(&mut session, pt(80.0, -20.0), pt(120.0, 20.0), false, false);
        assert_eq!(
            session.node.selection().node_pairs(),
            &[(p1, AnchorId::new(1, 1)), (p2, AnchorId::new(2, 0))]
        );
        assert!(session.node_toolbar_state().can_join);
        session.join_selected();
        let joined = session.document.path(p1).expect("P1 survives");
        let xs: Vec<f64> = joined.anchors.iter().map(|a| a.point.x).collect();
        assert_eq!(xs, vec![0.0, 100.0, 200.0]);
        assert!(session.document.path(p2).is_none());
    }

    /// Criteria 17, 22 and 24: Ctrl+A and a marquee write nothing, select in document order
    /// and announce the count.
    #[test]
    fn ctrl_a_and_a_marquee_write_nothing_and_announce() {
        let (mut session, p1, p2) = node_session();
        let version = session.document.version();
        let dirty = session.is_dirty();
        let outcome = session.key_down(KeyInput {
            key: "a",
            ctrl: true,
            ..KeyInput::default()
        });
        assert_eq!(outcome, KeyOutcome::SelectedAll);
        assert_eq!(
            session.node.selection().node_pairs(),
            &[
                (p1, AnchorId::new(1, 0)),
                (p1, AnchorId::new(1, 1)),
                (p2, AnchorId::new(2, 0)),
                (p2, AnchorId::new(2, 1)),
            ]
        );
        assert_eq!(session.selection_announcement(), "4 nodes selected.");
        drag(&mut session, pt(-5.0, -5.0), pt(5.0, 5.0), false, true);
        assert_eq!(session.selection_announcement(), "3 nodes selected.");
        drag(
            &mut session,
            pt(100.0, 100.0),
            pt(110.0, 110.0),
            false,
            false,
        );
        assert_eq!(session.selection_announcement(), "No nodes selected.");
        assert_eq!(session.document.version(), version);
        assert_eq!(session.is_dirty(), dirty);
    }

    /// Criterion 24 (UX review finding 2): the text goes as soon as the node selection changes
    /// another way, and a repeated Ctrl+A with the same count gets a new sequence number so the
    /// live region speaks it again.
    #[test]
    fn a_repeated_announcement_speaks_again_and_a_stale_one_is_cleared() {
        let (mut session, _, _) = node_session();
        let ctrl_a = |session: &mut Session| {
            session.key_down(KeyInput {
                key: "a",
                ctrl: true,
                ..KeyInput::default()
            })
        };
        ctrl_a(&mut session);
        assert_eq!(session.selection_announcement(), "4 nodes selected.");
        let first = session.selection_announcement_seq();
        assert_eq!(session.escape(), EscapeStep::ClearedState);
        assert_eq!(session.selection_announcement(), "", "stale after Escape");
        ctrl_a(&mut session);
        assert_eq!(session.selection_announcement(), "4 nodes selected.");
        assert_ne!(session.selection_announcement_seq(), first);
        session.set_tool(Tool::Select);
        session.set_tool(Tool::Node);
        assert_eq!(session.selection_announcement(), "");
    }

    /// Criterion 4 with `0049`: a linked clone is not in the editing set, so neither Ctrl+A nor
    /// a marquee over it selects its nodes, and the count leaves it out.
    #[test]
    fn a_linked_clone_contributes_no_node() {
        let (mut session, p1, p2) = node_session();
        let clone = session.document.linked_clone(&[p1], &[]).unwrap()[0];
        assert!(session.paths().iter().all(|p| p.id != clone));
        let _ = session.key_down(KeyInput {
            key: "a",
            ctrl: true,
            ..KeyInput::default()
        });
        assert!(
            session
                .node
                .selection()
                .node_pairs()
                .iter()
                .all(|&(path, _)| path == p1 || path == p2)
        );
        assert_eq!(session.node_toolbar_state().total_nodes, 4);
        drag(&mut session, pt(-5.0, -5.0), pt(5.0, 5.0), false, false);
        assert_eq!(
            session.node.selection().node_pairs(),
            &[(p1, AnchorId::new(1, 0))],
            "the clone's node on top of P1's first node is not taken"
        );
    }

    /// Criteria 2, 10, 11: box, rings, legend, badge and cursor while the marquee runs; Escape
    /// removes them and keeps the selection; the release that follows does nothing.
    #[test]
    fn the_marquee_shows_while_it_runs_and_escape_cancels_it() {
        let (mut session, p1, _) = node_session();
        session.node.select_nodes(vec![(p1, AnchorId::new(1, 0))]);
        session.modifiers_changed(false, true, false);
        session.pointer_hover(pt(5.0, 50.0), false, true);
        assert!(session.move_indicators().remove_badge, "Ctrl over nothing");
        session.pointer_down(pt(5.0, 50.0), false);
        assert_eq!(session.cursor_hint(), "crosshair");
        session.pointer_hover(pt(250.0, -5.0), false, true);
        assert_eq!(
            session.live_readout().map(|r| r.text),
            Some("3 nodes \u{b7} \u{2212}Remove".to_string())
        );
        assert!(matches!(
            session.node_marquee_overlay(),
            Some(MarqueeOverlay::Box { contain: true, .. })
        ));
        assert_eq!(session.decoration_input().ringed_nodes.len(), 3);
        assert_eq!(session.decoration_input().hovered, None);
        assert_eq!(session.escape(), EscapeStep::CancelledDrag);
        assert_eq!(session.node_marquee_overlay(), None);
        assert_eq!(session.live_readout(), None);
        session.pointer_up(pt(250.0, -5.0), false, true);
        assert_eq!(
            session.node.selection().node_pairs(),
            &[(p1, AnchorId::new(1, 0))]
        );
    }

    /// Criterion 12: a click on nothing clears at the release, unless Shift or Ctrl is held.
    #[test]
    fn a_click_on_nothing_clears_at_the_release() {
        let (mut session, p1, _) = node_session();
        session.node.select_nodes(vec![(p1, AnchorId::new(1, 0))]);
        let far = pt(300.0, 300.0);
        session.pointer_down(far, false);
        assert!(!session.node.selection().is_empty(), "not at the press");
        session.pointer_up(far, false, true);
        assert!(!session.node.selection().is_empty(), "Ctrl keeps it");
        session.pointer_down(far, false);
        session.pointer_up(far, false, false);
        assert!(session.node.selection().is_empty());
        assert_eq!(
            session.node_toolbar_state(),
            NodeToolbarState {
                join_block: Some(curvyo_ui_core::JoinBlock::Count(0)),
                total_nodes: 4,
                ..NodeToolbarState::default()
            }
        );
    }

    /// Criterion 18: a marquee over nodes of two paths, then Delete, is one step.
    #[test]
    fn delete_after_a_marquee_across_paths_is_one_step() {
        let mut session = Session::new(1);
        let p = line(&session, 1, &[0.0, 10.0, 20.0]);
        let q = line(&session, 2, &[0.0, 10.0, 20.0]);
        session.set_tool(Tool::Node);
        drag(&mut session, pt(5.0, -5.0), pt(15.0, 5.0), false, false);
        assert_eq!(session.node.selection().node_pairs().len(), 2);
        let steps = session.document.own_step_count();
        session.delete_selected();
        assert_eq!(session.document.own_step_count(), steps + 1);
        for path in [p, q] {
            assert_eq!(session.document.path(path).unwrap().anchors.len(), 2);
        }
        assert_eq!(session.tool, Tool::Node);
    }
}
