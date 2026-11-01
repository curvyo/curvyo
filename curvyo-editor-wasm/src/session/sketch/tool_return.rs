//! Completed shapes and applied relationships return the tool to Select with their geometry
//! selected (AC42, AC43).
use super::{Session, SketchTool};
use curvyo_sketch_core::EntityId;
use std::collections::BTreeSet;

impl Session {
    /// The entity identities of the active sketch now; empty before the sketch exists.
    pub(super) fn sketch_entity_ids(&self) -> BTreeSet<EntityId> {
        self.active_sketch()
            .and_then(|id| self.document.sketch(id))
            .map(|snapshot| snapshot.graph.entities.into_keys().collect())
            .unwrap_or_default()
    }

    /// The shapes created since `before`: new entities that are not the defining points of
    /// another new entity, without the origin a first shape materialises. A drawn line is
    /// selected as the line, as in Vector, so a rail tool applies to it at once.
    pub(super) fn sketch_created_since(&self, before: &BTreeSet<EntityId>) -> BTreeSet<EntityId> {
        let Some(graph) = self
            .active_sketch()
            .and_then(|id| self.document.sketch(id))
            .map(|snapshot| snapshot.graph)
        else {
            return BTreeSet::new();
        };
        let created: BTreeSet<&EntityId> = graph
            .entities
            .keys()
            .filter(|id| !before.contains(*id) && **id != graph.origin)
            .collect();
        let defining: BTreeSet<&EntityId> = created
            .iter()
            .filter_map(|id| graph.entities.get(*id))
            .flat_map(curvyo_sketch_core::Entity::point_ids)
            .collect();
        created
            .into_iter()
            .filter(|id| !defining.contains(id))
            .cloned()
            .collect()
    }

    /// After a committed shape: back in Select with exactly what it created selected. A
    /// Polyline whose chain stays open only collects its segments' geometry for the chain end.
    pub(super) fn sketch_shape_committed(&mut self, before: &BTreeSet<EntityId>, chain_open: bool) {
        let created = self.sketch_created_since(before);
        if self.sketch.tool == SketchTool::Polyline {
            self.sketch.chain_created.extend(created);
            if !chain_open {
                self.sketch_end_chain();
            }
            return;
        }
        self.sketch_select_new(created);
    }

    /// The Polyline chain ended (closed, Enter or Escape): with at least one committed segment
    /// the whole chain is selected in Select (AC42). `false` when no segment was committed.
    pub(in crate::session) fn sketch_end_chain(&mut self) -> bool {
        if self.sketch.tool != SketchTool::Polyline || self.sketch.chain_created.is_empty() {
            return false;
        }
        let mut created = std::mem::take(&mut self.sketch.chain_created);
        // An undo during the chain may have removed segments it collected.
        let current = self.sketch_entity_ids();
        created.retain(|id| current.contains(id));
        self.sketch_select_new(created);
        true
    }

    /// Select with `created` as the selection; neither adds a history step.
    fn sketch_select_new(&mut self, created: BTreeSet<EntityId>) {
        self.set_sketch_tool(SketchTool::Select);
        self.sketch.entities = created;
        self.sketch.constraints.clear();
    }

    /// After a constraint or dimension applied: Select with the geometry it refers to (AC43).
    pub(super) fn sketch_applied_select(&mut self, geometry: BTreeSet<EntityId>) {
        if self.sketch.tool != SketchTool::Select || self.sketch.constraint_tool.is_some() {
            self.set_sketch_tool(SketchTool::Select);
        }
        self.sketch.entities = geometry;
    }
}
