//! Relation tools collect analytic references without starting geometry drags.
use super::{Session, SketchTool};
use curvyo_sketch_core::Entity;

/// What the dimension tool measures, shown on the canvas when the picks fit no dimension
/// (UX notes revision 2 review, change 2).
pub(super) const DIMENSION_NEEDS: &str = "Dimension: pick a segment, a circle or arc, two points, a point and a segment, or two segments";

impl Session {
    /// Activates an existing constraint or the context-sensitive dimension tool.
    pub fn set_sketch_constraint_tool(&mut self, action: &str) -> bool {
        if action != "dimension" && !curvyo_ui_core::SKETCH_ACTIONS.contains(&action) {
            return false;
        }
        self.set_sketch_tool(SketchTool::Constraints);
        self.sketch.constraint_tool = Some(action.to_owned());
        // Each arming starts from the kind the picked curve suggests (change 3).
        self.sketch.circle_dimension_switched = false;
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
            // An arc is dimensioned by its radius, a full circle by its diameter; Alt switches
            // (UX notes revision 2 review, change 3).
            [round @ (Entity::Circle { .. } | Entity::Arc { .. })] => {
                let radius = matches!(round, Entity::Arc { .. });
                Some(if radius == self.sketch.circle_dimension_switched {
                    "diameter"
                } else {
                    "radius"
                })
            }
            [
                Entity::Point { .. },
                Entity::Point { .. } | Entity::Line { .. },
            ]
            | [Entity::Line { .. }, Entity::Point { .. }] => Some("distance"),
            [Entity::Line { .. }, Entity::Line { .. }] => Some("angle"),
            _ => None,
        }
    }
}
