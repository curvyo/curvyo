//! `Session`'s layer rules on the canvas: the active layer that new objects go into, the selection
//! invariant that drops hidden and locked objects, the refusal to draw into a hidden or locked
//! layer, and the stacking-order keys (`specs/0050-layers-and-objects-panel` criteria 14, 23, 33
//! and 35).

use std::collections::HashSet;

use curvyo_document_core::{MOVE_NODES_COMMIT_LABEL, MoveRefusal, NodeId, ObjectSnapshot, Point};
use curvyo_ui_core::{
    Reorder, draw_refused_notice, hit_test_object, move_refusal_text, reorder_plans, unchanged_text,
};

use super::keys::KeyOutcome;
use super::{Session, Tool};

/// Which layer the maker draws into. View state: not saved, no step (criterion 35).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ActiveLayer {
    /// Nothing chose one yet: the front-most layer, if any (at Open and New).
    #[default]
    Unchosen,
    /// The maker chose this layer (a selection in it, a layer row, New layer), or none (a
    /// selection of a loose object).
    Chosen(Option<NodeId>),
}

/// The layer state of one session.
#[derive(Debug, Default)]
pub(super) struct LayerState {
    pub(super) active: ActiveLayer,
    /// The first selected leaf when the active layer last followed the selection.
    seen_head: Option<NodeId>,
    /// A notice for the canvas notice slot and whether it is a refusal (an alert), until the
    /// host has taken it.
    pub(super) notice: Option<(String, bool)>,
}

impl Session {
    /// The active layer as the document holds it now: the chosen one while it is a live layer,
    /// else (unchosen) the front-most layer. `None` when the document has no layer to draw into.
    #[must_use]
    pub fn active_layer(&self) -> Option<NodeId> {
        match self.layers.active {
            ActiveLayer::Chosen(chosen) => chosen.filter(|&id| self.document.is_layer(id)),
            ActiveLayer::Unchosen => self.document_read().front_layer,
        }
    }

    /// The layer that holds `id`, or `None` for a loose object.
    pub(super) fn layer_of(&self, id: NodeId) -> Option<NodeId> {
        let mut top = id;
        while let Some(parent) = self.document.parent_node(top) {
            top = parent;
        }
        (top != id || self.document.is_layer(id))
            .then_some(top)
            .filter(|&top| self.document.is_layer(top))
    }

    /// Selecting an object makes its layer the active layer; a loose object makes none
    /// (criterion 35). Only a change of the selection's first object counts, so a layer chosen
    /// by its row stays while nothing is selected.
    pub(super) fn follow_selection_with_active_layer(&mut self) {
        let head = self.selection.ids().first().copied();
        if head == self.layers.seen_head {
            return;
        }
        self.layers.seen_head = head;
        if let Some(leaf) = head {
            self.layers.active = ActiveLayer::Chosen(self.layer_of(leaf));
            // The panel target is the selection again (criterion 13).
            self.panel.target_layer = false;
        }
    }

    /// Drops from the selection every object of the context that is effectively hidden or
    /// locked, whatever made it so (criterion 14). A selected group keeps its hidden and locked
    /// members (criterion 21). Reads nothing while the selection is empty.
    pub(super) fn drop_inert_selection(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        let units = self.units();
        let kept = units.without_inert(self.selection.ids());
        if kept.len() != self.selection.ids().len() {
            self.selection.set(&kept);
        }
    }

    /// The node new objects are created in: the entered group, else the active layer, else the
    /// top level (criterion 35, `0023` criterion 19).
    pub(super) fn draw_parent(&self, entered: Option<NodeId>) -> Option<NodeId> {
        entered.or_else(|| self.active_layer())
    }

    /// Whether a press of the Pen or a shape tool that would start a new object is refused
    /// because the active layer is hidden or locked (criterion 23); the notice is posted. Inside
    /// an entered group the group decides, and it cannot be hidden or locked (criterion 14).
    pub(super) fn refuse_drawing_into_layer(&mut self) -> bool {
        let starts_object = match self.tool {
            Tool::Pen => !self.pen_path_unfinished(),
            Tool::Rectangle | Tool::Ellipse | Tool::PolygonStar => true,
            Tool::Select | Tool::Node | Tool::Offset => false,
        };
        if !starts_object || self.context_group().is_some() {
            return false;
        }
        let Some(layer) = self.active_layer() else {
            return false;
        };
        let Some(registers) = self.document.node_registers(layer) else {
            return false;
        };
        if registers.visible && !registers.locked {
            return false;
        }
        let name = registers.name.unwrap_or_default();
        self.layers.notice = Some((draw_refused_notice(&name, registers.locked), true));
        true
    }

    /// Whether the front-most drawn object of the context at `point` is effectively locked, by
    /// its own lock or an ancestor's: the cursor then shows the lock where a press would select
    /// nothing (criterion 20, customer feedback 2026-10-14). A hidden object is not there.
    pub(super) fn locked_at(&self, point: Point) -> bool {
        let read = self.document_read();
        if read.inert.is_empty() {
            return false;
        }
        let shown: Vec<ObjectSnapshot> = if self.context_group().is_none() {
            read.visible.to_vec()
        } else {
            let inside: HashSet<NodeId> = self.objects().iter().map(ObjectSnapshot::id).collect();
            read.visible
                .iter()
                .filter(|object| inside.contains(&object.id()))
                .cloned()
                .collect()
        };
        hit_test_object(&shown, point, self.object_tolerance())
            .is_some_and(|id| read.inert.contains(&id))
    }

    /// Home, End, Page Up or Page Down on the canvas: the selected objects of the context change
    /// their stacking order among their siblings, in one step (criterion 33).
    pub(super) fn reorder_selection(&mut self, how: Reorder) -> KeyOutcome {
        self.recording(|this| {
            let nodes: Vec<NodeId> = this
                .units()
                .selected(this.selection.ids())
                .into_iter()
                .map(|unit| unit.id)
                .collect();
            match this.run_reorder(&nodes, how) {
                Ok(true) => {}
                // A press that changes nothing is said (criterion 33, amended 2026-10-12).
                Ok(false) => {
                    this.layers.notice = unchanged_text(how).map(|text| (text.to_owned(), false));
                }
                Err(refusal) => {
                    this.layers.notice = Some((move_refusal_text(refusal).to_owned(), true));
                }
            }
            KeyOutcome::Reordered
        })
    }

    /// Runs the moves `how` makes of `nodes` in one step (`move_nodes`), after checking every
    /// one; returns whether anything moved. Nothing is written when one is refused.
    pub(super) fn run_reorder(
        &mut self,
        nodes: &[NodeId],
        how: Reorder,
    ) -> Result<bool, MoveRefusal> {
        let plans = reorder_plans(&self.document, nodes, how);
        for plan in &plans {
            self.document.check_move(plan)?;
        }
        if plans.is_empty() {
            return Ok(false);
        }
        self.document.batch(MOVE_NODES_COMMIT_LABEL, |document| {
            for plan in &plans {
                // invariant: the plans are independent: one per parent, each among that parent's
                // own children (`reorder_plans`), so a plan checked before the first move still
                // holds after the others ran.
                let moved = document.move_nodes(plan);
                debug_assert!(moved.is_ok(), "a checked plan was refused: {moved:?}");
            }
        });
        Ok(true)
    }

    /// The layer notice for the canvas notice slot, once; empty when there is none.
    pub fn take_layer_notice(&mut self) -> String {
        self.layers
            .notice
            .take()
            .map(|(text, _)| text)
            .unwrap_or_default()
    }

    /// The layer notice and its kind as the host reads it, once: `{"kind": "alert" | "status",
    /// "text"}`, or the empty string when there is none. An empty notice is never serialised
    /// (customer feedback 2026-10-14: a press on a locked object showed `{"kind":"","text":""}`).
    pub fn take_layer_notice_json(&mut self) -> String {
        match self.layers.notice.take() {
            Some((text, alert)) if !text.is_empty() => {
                let kind = if alert { "alert" } else { "status" };
                serde_json::json!({ "kind": kind, "text": text }).to_string()
            }
            _ => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{MovePlan, MoveTarget, Point, RectBounds};

    use super::*;

    fn rect(session: &Session, x: f64) -> NodeId {
        session.document.create_rect(RectBounds::from_corners(
            Point::new(x, 0.0),
            Point::new(x + 10.0, 10.0),
        ))
    }

    fn into(session: &Session, nodes: &[NodeId], layer: NodeId) {
        session
            .document
            .move_nodes(&MovePlan {
                nodes: nodes.to_vec(),
                target: MoveTarget::FrontOf(Some(layer)),
            })
            .unwrap();
    }

    #[test]
    fn the_front_most_layer_is_active_until_the_maker_chooses() {
        let session = Session::new(1);
        assert_eq!(session.active_layer(), None);
        let back = session.document.new_layer(None);
        let front = session.document.new_layer(None);
        assert_eq!(session.active_layer(), Some(front));
        session.document.delete_layer(front).unwrap();
        assert_eq!(session.active_layer(), Some(back));
    }

    #[test]
    fn selecting_an_object_makes_its_layer_active_and_a_loose_one_none() {
        let mut session = Session::new(1);
        let cut = session.document.new_layer(None);
        let engrave = session.document.new_layer(None);
        let in_cut = rect(&session, 0.0);
        into(&session, &[in_cut], cut);
        let loose = rect(&session, 20.0);
        session.selection.select_single(in_cut);
        session.sync_context();
        assert_eq!(session.active_layer(), Some(cut));
        session.selection.clear();
        session.sync_context();
        assert_eq!(
            session.active_layer(),
            Some(cut),
            "an empty selection keeps it"
        );
        session.selection.select_single(loose);
        session.sync_context();
        assert_eq!(session.active_layer(), None);
        let _ = engrave;
    }

    #[test]
    fn new_objects_go_to_the_front_of_the_active_layer() {
        let mut session = Session::new(1);
        let cut = session.document.new_layer(None);
        let _front = session.document.new_layer(None);
        session.layers.active = ActiveLayer::Chosen(Some(cut));
        session.sync_context();
        let a = rect(&session, 0.0);
        assert_eq!(session.document.children(Some(cut)), vec![a]);
    }

    #[test]
    fn hiding_or_locking_drops_the_object_from_the_selection() {
        let mut session = Session::new(1);
        let layer = session.document.new_layer(None);
        let (a, b) = (rect(&session, 0.0), rect(&session, 20.0));
        into(&session, &[a], layer);
        session.selection.set(&[a, b]);
        session.document.set_node_locked(&[layer], true).unwrap();
        session.sync_context();
        assert_eq!(session.selection.ids(), &[b]);
        session.document.set_node_visible(&[b], false).unwrap();
        session.sync_context();
        assert!(session.selection.is_empty());
    }

    #[test]
    fn a_selected_group_keeps_its_locked_member() {
        let mut session = Session::new(1);
        let (a, b) = (rect(&session, 0.0), rect(&session, 20.0));
        let group = session.document.group_objects(&[a, b]).unwrap();
        session.document.set_node_locked(&[a], true).unwrap();
        session.selection.set(&[a, b]);
        session.sync_context();
        assert_eq!(session.selection.ids(), &[a, b]);
        session.document.set_node_locked(&[group], true).unwrap();
        session.sync_context();
        assert!(session.selection.is_empty());
    }

    #[test]
    fn the_canvas_keys_reorder_the_selection_in_one_step() {
        let mut session = Session::new(1);
        let (a, b, c) = (
            rect(&session, 0.0),
            rect(&session, 20.0),
            rect(&session, 40.0),
        );
        session.selection.set(&[a]);
        let steps = session.document.own_step_count();
        let key = |k: &'static str| super::super::keys::KeyInput {
            key: k,
            ..Default::default()
        };
        assert_eq!(session.key_down(key("Home")), KeyOutcome::Reordered);
        assert_eq!(session.document.children(None), vec![b, c, a]);
        assert_eq!(session.document.own_step_count(), steps + 1);
        session.key_down(key("PageDown"));
        assert_eq!(session.document.children(None), vec![b, a, c]);
        session.key_down(key("End"));
        assert_eq!(session.document.children(None), vec![a, b, c]);
        session.key_down(key("PageUp"));
        assert_eq!(session.document.children(None), vec![b, a, c]);
        assert_eq!(session.selection.ids(), &[a], "the selection stays");
    }

    #[test]
    fn drawing_into_a_locked_or_hidden_active_layer_is_refused_with_a_notice() {
        let mut session = Session::new(1);
        let layer = session.document.new_layer(None);
        session.document.rename_node(layer, "Cut").unwrap();
        session.document.set_node_locked(&[layer], true).unwrap();
        session.set_tool(Tool::Rectangle);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_hover(Point::new(30.0, 30.0), false, false);
        session.pointer_up(Point::new(30.0, 30.0), false, false);
        assert_eq!(session.document.object_ids(), Vec::new());
        assert_eq!(
            session.take_layer_notice(),
            "Layer Cut is locked. Unlock it or choose another layer."
        );
        assert_eq!(session.tool(), Tool::Rectangle, "the tool stays");
        session.document.set_node_locked(&[layer], false).unwrap();
        session.document.set_node_visible(&[layer], false).unwrap();
        session.set_tool(Tool::Pen);
        session.pointer_down(Point::new(0.0, 0.0), false);
        session.pointer_up(Point::new(0.0, 0.0), false, false);
        assert_eq!(
            session.take_layer_notice(),
            "Layer Cut is hidden. Show it or choose another layer."
        );
        assert!(session.pen_in_progress().is_none());
    }
}
