//! The Sketch canvas cursor names what a primary press at the pointer would do.
use super::{Session, SketchTool};
use curvyo_document_core::SketchSnapshot;
use curvyo_sketch_core::{Entity, EntityId, Length};

impl Session {
    /// `crosshair` for drawing tools; in Select `grabbing` during a drag of movable geometry,
    /// `grab` over movable geometry and `default` elsewhere (UX notes revision 2, section 6).
    pub(super) fn sketch_cursor(&self) -> &'static str {
        match self.sketch.tool {
            SketchTool::Select => {}
            SketchTool::Constraints => return "default",
            _ => return "crosshair",
        }
        // A dimension text is dragged with the hand; symbols are not draggable (AC30, AC31).
        if self.sketch.label_drag.is_some() {
            return "grabbing";
        }
        if self.sketch_hovering_dimension() {
            return "grab";
        }
        if let Some(drag) = &self.sketch.drag {
            let movable =
                drag.points.iter().any(|id| movable(&drag.base, id)) || drag.handle.is_some();
            return if movable { "grabbing" } else { "default" };
        }
        if self.sketch.marquee.is_some() || self.sketch.lasso.is_some() {
            return "default";
        }
        let Some(point) = self.pointer_position else {
            return "default";
        };
        let (Ok(snapshot), Some(local)) =
            (self.sketch_snapshot_for_edit(), self.sketch_local(point))
        else {
            return "default";
        };
        if snapshot.stale {
            return "default";
        }
        let tolerance = Length(6.0 / self.view().scale());
        let handle = curvyo_ui_core::sketch_handle_hit(
            &snapshot.graph,
            &snapshot.solution,
            &self.sketch.entities,
            local,
            tolerance,
        );
        if handle.is_some() {
            return "grab";
        }
        let hit = curvyo_ui_core::sketch_hit(&snapshot.graph, &snapshot.solution, local, tolerance);
        match hit {
            Some(hit) if entity_movable(&snapshot, &hit.entity) => "grab",
            _ => "default",
        }
    }
}

/// Whether a body drag of `id` moves at least one of its defining points.
fn entity_movable(snapshot: &SketchSnapshot, id: &EntityId) -> bool {
    match snapshot.graph.entities.get(id) {
        Some(Entity::Point { .. }) => movable(snapshot, id),
        Some(entity) => entity
            .point_ids()
            .into_iter()
            .any(|point| movable(snapshot, point)),
        None => false,
    }
}

/// A point moves unless it is pinned or the solver proved it fixed.
fn movable(snapshot: &SketchSnapshot, id: &EntityId) -> bool {
    if matches!(
        snapshot.graph.entities.get(id),
        Some(Entity::Point { fixed: true, .. })
    ) {
        return false;
    }
    !snapshot
        .fully_constrained
        .as_ref()
        .is_some_and(|fixed| fixed.contains(id))
}
