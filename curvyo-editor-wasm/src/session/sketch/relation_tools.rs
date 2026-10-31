//! Relation tools collect analytic references without starting geometry drags.
use super::{Session, SketchTool};
use curvyo_sketch_core::Entity;

impl Session {
    /// Chooses the measured circle quantity without changing document intent.
    pub fn set_sketch_circle_dimension(&mut self, radius: bool) {
        self.sketch.circle_dimension_radius = radius;
    }

    /// Activates an existing constraint or the context-sensitive dimension tool.
    pub fn set_sketch_constraint_tool(&mut self, action: &str) -> bool {
        if action != "dimension" && !curvyo_ui_core::SKETCH_ACTIONS.contains(&action) {
            return false;
        }
        self.set_sketch_tool(SketchTool::Constraints);
        self.sketch.constraint_tool = Some(action.to_owned());
        true
    }
    pub(super) fn sketch_dimension_action(&self) -> Option<&'static str> {
        let snapshot = self.sketch_snapshot_for_edit().ok()?;
        let entities = self
            .sketch
            .entities
            .iter()
            .filter_map(|id| snapshot.graph.entities.get(id))
            .collect::<Vec<_>>();
        match entities.as_slice() {
            [Entity::Line { .. }] => Some("length"),
            [Entity::Circle { .. } | Entity::Arc { .. }] => {
                Some(if self.sketch.circle_dimension_radius {
                    "radius"
                } else {
                    "diameter"
                })
            }
            [Entity::Point { .. }, Entity::Point { .. }] => Some("distance"),
            [Entity::Line { .. }, Entity::Line { .. }] => Some("angle"),
            _ => None,
        }
    }
}
