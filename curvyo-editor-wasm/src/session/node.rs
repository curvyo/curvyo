//! `Session`'s Node-tool glue (`specs/0002-path-node-editing`,
//! `specs/0006-path-merge-split-and-node-types`): the contextual-toolbar
//! actions, the live node/handle drag substituted into the snapshot the
//! renderer draws, and the Node tool's decoration input. Split out of
//! `session/mod.rs` the same way `session/select.rs` and `session/shapes.rs`
//! are (`docs/technical-debt.md`, "`Session` is one module past the size
//! limit").

use std::borrow::Cow;
use std::collections::HashSet;

use curvyo_document_core::{AnchorKind, NodeId, ObjectSnapshot, PathSnapshot, Point};
use curvyo_render_core::{DecorationInput, Hovered as RenderHovered};
use curvyo_ui_core::{
    BendResolution, Hit, LiveNodeDrag, NodeSplitOutcome, NodeToolbarState, segment_is_bendable,
};

use super::{Session, Tool};

impl Session {
    /// [`Session::paths`], with the node tool's in-flight drag (if any)
    /// substituted into the relevant anchor's live, not-yet-committed
    /// position/handle values — resolved by [`NodeTool::live_drag`]
    /// itself (the same helpers [`NodeTool::pointer_up`] uses to commit),
    /// so this is a pure "apply already-resolved data" step with no
    /// geometry of its own. Falls back to the committed snapshot
    /// unmodified outside the node tool, with no drag in flight, or with
    /// the pointer off the canvas (`self.pointer_position` is `None`). The
    /// Select tool's live edit is not substituted here: its blue outline is
    /// drawn over the committed paths (`specs/0009-unified-object-editing`).
    pub(super) fn live_node_drag_paths_in(
        &self,
        objects: &[ObjectSnapshot],
    ) -> Vec<curvyo_document_core::PathSnapshot> {
        let mut paths: Vec<curvyo_document_core::PathSnapshot> = objects
            .iter()
            .filter_map(|object| match object {
                ObjectSnapshot::Path(path) => Some(path.clone()),
                ObjectSnapshot::Primitive(_)
                | ObjectSnapshot::Clone(_)
                | ObjectSnapshot::Text(_)
                | ObjectSnapshot::Image(_) => None,
            })
            .collect();
        if self.tool == Tool::Node {
            self.apply_live_node_drag(&mut paths);
        }
        paths
    }

    /// The Node tool's editing set (customer 2026-10-26, amending `specs/0053-node-marquee-
    /// selection` "editing set"): the selected paths when the selection holds one, else every
    /// path [`Session::paths`] gives. The Node tool draws and hit-tests these only.
    pub(super) fn node_paths(&self) -> Vec<PathSnapshot> {
        let mut paths = self.paths();
        if self.cuts_node_set(&paths) {
            paths.retain(|path| self.selection.contains(path.id));
        }
        paths
    }

    /// `paths` cut to the editing set: those whose id is selected, or all when none is.
    pub(super) fn in_node_set<'a>(&self, paths: &'a [PathSnapshot]) -> Cow<'a, [PathSnapshot]> {
        if !self.cuts_node_set(paths) {
            return Cow::Borrowed(paths);
        }
        Cow::Owned(
            paths
                .iter()
                .filter(|path| self.selection.contains(path.id))
                .cloned()
                .collect(),
        )
    }

    /// Whether the selection holds one of `paths`, which cuts the editing set to the selected.
    fn cuts_node_set(&self, paths: &[PathSnapshot]) -> bool {
        paths.iter().any(|path| self.selection.contains(path.id))
    }

    pub(super) fn apply_live_node_drag(&self, paths: &mut [curvyo_document_core::PathSnapshot]) {
        let Some(cursor) = self.pointer_position else {
            return;
        };
        let Some(live) = self.node.live_drag(cursor, self.held.shift) else {
            return;
        };
        match live {
            LiveNodeDrag::Bend(bend) => {
                // Only the two end anchors' handles move, and only the decorations read this
                // path: the artwork stays as committed (black old, `0031` criterion 16).
                for end in bend.ends {
                    if let Some(snapshot) = paths.iter_mut().find(|p| p.id == bend.path)
                        && let Some(anchor) = snapshot.all_anchors_mut().find(|a| a.id == end.id)
                    {
                        anchor.handle_in = end.handle_in;
                        anchor.handle_out = end.handle_out;
                    }
                }
            }
            LiveNodeDrag::Nodes { positions } => {
                for (path, id, point) in positions {
                    if let Some(snapshot) = paths.iter_mut().find(|p| p.id == path)
                        && let Some(anchor) = snapshot.all_anchors_mut().find(|a| a.id == id)
                    {
                        anchor.point = point;
                    }
                }
            }
            LiveNodeDrag::Handle {
                path,
                anchor,
                handle_in,
                handle_out,
            } => {
                // `handle_in`/`handle_out` already fully resolved by
                // `NodeTool::live_drag` (which calls the exact same
                // `curvyo_document_core::resolve_handle_pair` function
                // `Document::set_handle` itself commits with) — a plain
                // assignment, no slot/mirror logic of its own to
                // independently drift from the commit.
                if let Some(snapshot) = paths.iter_mut().find(|p| p.id == path)
                    && let Some(anchor) = snapshot.all_anchors_mut().find(|a| a.id == anchor)
                {
                    anchor.handle_in = handle_in;
                    anchor.handle_out = handle_out;
                }
            }
        }
    }

    /// Acceptance criterion 11 (the contextual toolbar's convert
    /// buttons). A no-op for the pen tool.
    pub fn convert_selected(&mut self, kind: AnchorKind) {
        self.recording(|this| {
            if this.tool == Tool::Node {
                this.node.convert_selected(&this.document, kind);
            }
        });
    }

    /// Acceptance criterion 14's "make line" (the contextual toolbar).
    pub fn make_line(&mut self) {
        self.recording(|this| {
            if this.tool == Tool::Node {
                this.node.make_line(&this.document);
            }
        });
    }

    /// Acceptance criterion 14's "make curve" (the contextual toolbar).
    pub fn make_curve(&mut self) {
        self.recording(|this| {
            if this.tool == Tool::Node {
                this.node.make_curve(&this.document);
            }
        });
    }

    /// Acceptance criteria 8-11: Join (the contextual toolbar/context
    /// menu button). A no-op outside the node tool or when the current
    /// selection does not qualify. Returns how many outlines moved into the surviving object
    /// besides the joined one, for the notice (`specs/0055-compound-path-node-editing` criterion
    /// 5 c), or `None` when nothing was joined.
    pub fn join_selected(&mut self) -> Option<usize> {
        self.recording(|this| {
            if this.tool == Tool::Node {
                this.node.join_selected(&this.document)
            } else {
                None
            }
        })
    }

    /// Acceptance criteria 12-15: Split (the contextual toolbar/context
    /// menu button). A no-op outside the node tool or when the current
    /// selection does not qualify. [`Session::split_selected_outcome`] also says what was split.
    pub fn split_selected(&mut self) {
        let _ = self.split_selected_outcome();
    }

    /// [`Session::split_selected`], returning what was split, for the notice of a compound path
    /// (`0055` criterion 6).
    pub fn split_selected_outcome(&mut self) -> NodeSplitOutcome {
        self.recording(|this| {
            if this.tool == Tool::Node {
                let before: HashSet<NodeId> = this.document.object_ids().into_iter().collect();
                let outcome = this.node.split_selected(&mut this.minter, &this.document);
                // The new piece joins a selection, so the editing set keeps both halves
                // (customer 2026-10-26).
                if !this.selection.is_empty() {
                    let pieces: Vec<NodeId> = this
                        .document
                        .object_ids()
                        .into_iter()
                        .filter(|id| !before.contains(id))
                        .collect();
                    this.selection.add_all(&pieces);
                }
                outcome
            } else {
                NodeSplitOutcome::None
            }
        })
    }

    /// Acceptance criterion 12: a double-click (already recognized by
    /// the host) at `point`.
    pub fn insert_at(&mut self, point: Point) {
        self.recording(|this| {
            if this.tool == Tool::Node {
                let paths = this.node_paths();
                let tolerances = this.hit_tolerances();
                this.node
                    .insert_at(&mut this.minter, &this.document, &paths, point, tolerances);
            }
        });
    }

    /// The contextual toolbar's "Insert node" action: splits the
    /// currently selected segment at its midpoint. A no-op outside the
    /// node tool or without a segment selected.
    pub fn insert_selected(&mut self) {
        self.recording(|this| {
            if this.tool == Tool::Node {
                let paths = this.node_paths();
                this.node
                    .insert_on_selected_segment(&mut this.minter, &this.document, &paths);
            }
        });
    }

    /// Which contextual-toolbar actions apply right now. Everything is
    /// `false` when the node tool isn't active, since the toolbar itself
    /// is only shown then.
    #[must_use]
    pub fn node_toolbar_state(&self) -> NodeToolbarState {
        if self.tool != Tool::Node {
            return NodeToolbarState::default();
        }
        let paths = self.node_paths();
        let mut state = self.node.toolbar_state(&self.document, &paths);
        state.clone_only = self.node_selection_is_clones();
        state
    }

    /// The segment bend in flight past the drag threshold, as the pointer now would resolve it.
    pub(super) fn live_bend(&self) -> Option<BendResolution> {
        if self.tool != Tool::Node {
            return None;
        }
        let cursor = self.pointer_position?;
        match self.node.live_drag(cursor, self.held.shift)? {
            LiveNodeDrag::Bend(bend) => Some(bend),
            LiveNodeDrag::Nodes { .. } | LiveNodeDrag::Handle { .. } => None,
        }
    }

    /// The blue half of a bend (`0031` criterion 16): every segment the bend changes as a
    /// two-anchor open path with the bent path's own style, for the live preview outline. Empty
    /// when no bend runs. The cost is the number of changed segments (at most three), not the
    /// path's node count.
    pub(super) fn bend_preview_objects(&self, objects: &[ObjectSnapshot]) -> Vec<ObjectSnapshot> {
        let Some(bend) = self.live_bend() else {
            return Vec::new();
        };
        let Some(ObjectSnapshot::Path(path)) = objects.iter().find(|o| o.id() == bend.path) else {
            return Vec::new();
        };
        bend.changed_segments
            .iter()
            .map(|segment| {
                ObjectSnapshot::Path(PathSnapshot {
                    id: path.id,
                    closed: false,
                    style: path.style.clone(),
                    anchors: segment.to_vec(),
                    extra_subpaths: Vec::new(),
                    rotation: path.rotation,
                })
            })
            .collect()
    }

    /// The segment under the pointer if a press there would bend it: the Node tool's hit test
    /// found a segment (so the pointer is outside every node and handle radius) and it has a
    /// grab point. `None` during any drag.
    pub(super) fn hovered_segment_hit(&self, paths: &[PathSnapshot], point: Point) -> Option<Hit> {
        if self.node.drag_in_flight() {
            return None;
        }
        let hit = curvyo_ui_core::hit_test(
            paths,
            self.node.selection(),
            point,
            self.point_tolerance(),
            self.handle_tolerance(),
            self.segment_tolerance(),
        );
        match hit {
            Some(Hit::Segment { path, start, end })
                if !paths
                    .iter()
                    .find(|p| p.id == path)
                    .is_some_and(|p| segment_is_bendable(p, start, end)) =>
            {
                None
            }
            other => other,
        }
    }

    pub(super) fn decoration_input(&self) -> DecorationInput {
        if self.tool != Tool::Node {
            return DecorationInput::default();
        }
        let bend = self.live_bend();
        let selection = self.node.selection();
        // `node_pairs` directly, not `nodes()` zipped with `path()`: the
        // selection can now genuinely span several path objects
        // (`specs/0006-path-merge-split-and-node-types/specification.md`
        // acceptance criteria 6, 7, 15), and `path()` reports `None` for
        // that case — zipping against it would silently render none of
        // the selected nodes as selected instead of all of them.
        let selected_nodes = selection.node_pairs().to_vec();
        // The selected-segment overlay of the dragged segment is not drawn while the bend runs
        // (`0031` criterion 16); it returns on release and on cancel.
        let selected_segment = if bend.is_some() {
            None
        } else {
            selection.segment_with_path()
        };
        let hovered = self.hovered.map(|hit| match hit {
            Hit::Node { path, anchor } => RenderHovered::Node(path, anchor),
            Hit::Handle { path, anchor, slot } => RenderHovered::Handle(path, anchor, slot),
            Hit::Segment { path, start, end } => RenderHovered::Segment(path, start, end),
        });
        let handle_nodes = bend
            .iter()
            .flat_map(|bend| bend.ends.iter().map(|end| (bend.path, end.id)))
            .collect();
        // The nodes a marquee in flight would select get the pre-selection ring (`0053`
        // criterion 10); the hover state is empty during any drag.
        let ringed_nodes = self
            .node
            .marquee()
            .map(|marquee| marquee.inside().to_vec())
            .unwrap_or_default();
        DecorationInput {
            show_nodes: true,
            selected_nodes,
            selected_segment,
            hovered,
            handle_nodes,
            ringed_nodes,
        }
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{NodeId, Point};

    use super::super::{Session, Tool};

    /// An open two-node path from `from` to `to`, drawn with the Pen.
    fn line(session: &mut Session, from: Point, to: Point) -> NodeId {
        session.set_tool(Tool::Pen);
        for point in [from, to] {
            session.pointer_down(point, false);
            session.pointer_up(point, false, false);
        }
        session.finish_pen();
        *session.document.object_ids().last().unwrap()
    }

    fn ids(session: &Session) -> Vec<NodeId> {
        session.node_paths().iter().map(|path| path.id).collect()
    }

    /// Customer 2026-10-26: with nothing selected the Node tool shows and hits every path; with
    /// paths selected only those.
    #[test]
    fn the_editing_set_is_the_selected_paths_or_every_path() {
        let mut session = Session::new(1);
        let a = line(&mut session, Point::new(0.0, 0.0), Point::new(10.0, 0.0));
        let b = line(&mut session, Point::new(0.0, 20.0), Point::new(10.0, 20.0));
        session.selection.clear();
        session.set_tool(Tool::Node);
        assert_eq!(ids(&session), [a, b]);
        session.selection.set(&[a]);
        assert_eq!(ids(&session), [a]);
        // A press on b's node hits nothing and moves nothing.
        session.pointer_down(Point::new(0.0, 20.0), false);
        session.pointer_up(Point::new(5.0, 25.0), false, false);
        assert_eq!(
            session.document.path(b).unwrap().anchors[0].point,
            Point::new(0.0, 20.0)
        );
        // The drawn nodes are a's only.
        let all = session.live_node_drag_paths_in(&session.objects());
        let drawn: Vec<NodeId> = session.in_node_set(&all).iter().map(|p| p.id).collect();
        assert_eq!(drawn, [a]);
    }

    /// A click on another path's outline selects it, Shift adds it, a click on nothing clears
    /// the selection (customer 2026-10-26, as Inkscape's node tool).
    #[test]
    fn a_click_picks_the_path_the_node_tool_edits() {
        let mut session = Session::new(1);
        let a = line(&mut session, Point::new(0.0, 0.0), Point::new(10.0, 0.0));
        let b = line(&mut session, Point::new(0.0, 20.0), Point::new(10.0, 20.0));
        session.selection.set(&[a]);
        session.set_tool(Tool::Node);
        let click = |session: &mut Session, at: Point, shift: bool| {
            session.pointer_down(at, shift);
            session.pointer_up(at, shift, false);
        };
        click(&mut session, Point::new(5.0, 20.0), false);
        assert_eq!(session.selection.ids(), [b]);
        assert_eq!(ids(&session), [b]);
        click(&mut session, Point::new(5.0, 0.0), true);
        assert_eq!(ids(&session), [a, b]);
        click(&mut session, Point::new(50.0, 50.0), false);
        assert!(session.selection.is_empty());
        assert_eq!(ids(&session), [a, b]);
    }

    /// A split keeps both pieces in the editing set: the new one joins the selection.
    #[test]
    fn a_split_adds_the_new_piece_to_the_selection() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        for point in [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(20.0, 0.0),
        ] {
            session.pointer_down(point, false);
            session.pointer_up(point, false, false);
        }
        session.finish_pen();
        let a = session.document.object_ids()[0];
        session.selection.set(&[a]);
        session.set_tool(Tool::Node);
        session.pointer_down(Point::new(10.0, 0.0), false);
        session.pointer_up(Point::new(10.0, 0.0), false, false);
        session.split_selected();
        assert_eq!(session.document.object_ids().len(), 2, "split");
        assert_eq!(session.selection.ids().len(), 2);
        assert_eq!(session.node_paths().len(), 2);
    }
}
