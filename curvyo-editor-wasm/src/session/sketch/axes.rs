//! The local axes: inferred relationships for confirmed points, and pickable constraint targets.
use super::Session;
use curvyo_sketch_core::{
    Axis, Constraint, ConstructionResult, Entity, Position, Relation, Sketch,
};

/// Screen pixels within which a press on empty canvas picks a local axis (the geometry pick
/// tolerance).
const AXIS_PICK_PX: f64 = 6.0;

impl Session {
    /// A press near the local X or Y axis selects that axis as a constraint target (AC37, AC40):
    /// toggled into the picks of an armed constraint tool, else the selection (Shift adds).
    pub(super) fn sketch_axis_press(&mut self, local: Position) -> bool {
        let Some(id) = self.sketch_axes_at(local).into_iter().next() else {
            return false;
        };
        if self.sketch.tool == super::SketchTool::Constraints {
            self.sketch_armed_pick(&id);
        } else if self.held.shift {
            if !self.sketch.entities.insert(id.clone()) {
                self.sketch.entities.remove(&id);
            }
        } else {
            self.sketch.entities = [id].into();
        }
        true
    }

    /// The local axes within pick reach of `local`, X first: both at the origin.
    pub(super) fn sketch_axes_at(&self, local: Position) -> Vec<curvyo_sketch_core::EntityId> {
        let tolerance = AXIS_PICK_PX / self.view().scale();
        [
            (local.y.0, curvyo_ui_core::SKETCH_AXIS_X),
            (local.x.0, curvyo_ui_core::SKETCH_AXIS_Y),
        ]
        .into_iter()
        .filter(|(across, _)| across.abs() <= tolerance)
        .map(|(_, axis)| axis.into())
        .collect()
    }

    pub(super) fn infer_sketch_axes(&self, mut point: Position) -> Position {
        // Proposal: a six-screen-pixel band, independent of Vector snap settings.
        let tolerance = curvyo_ui_core::SKETCH_EDGE_PICK_PX / self.view().scale();
        if point.x.0.abs() <= tolerance {
            point.x.0 = 0.0;
        }
        if point.y.0.abs() <= tolerance {
            point.y.0 = 0.0;
        }
        point
    }

    pub(super) fn constrain_inferred_axes(
        &self,
        base: &Sketch,
        operation: &mut ConstructionResult,
    ) -> Result<(), curvyo_sketch_core::SketchError> {
        for (id, entity) in &operation.intent.entities {
            if base.entities.contains_key(id) {
                continue;
            }
            let Entity::Point { position, .. } = entity else {
                continue;
            };
            // Only actual picked anchors infer axes, never hidden construction helpers.
            if !self
                .sketch
                .picks
                .iter()
                .chain(self.sketch.hover.iter())
                .any(|picked| picked.distance(*position).0 <= 1e-9)
            {
                continue;
            }
            for (axis, coordinate, suffix) in
                [(Axis::X, position.y.0, "x"), (Axis::Y, position.x.0, "y")]
            {
                if coordinate.abs() > 1e-9
                    || curvyo_sketch_core::axis_is_implied(&operation.intent, id, axis)
                {
                    continue;
                }
                let key: curvyo_sketch_core::ConstraintId =
                    format!("{}:axis-{suffix}", id.0).into();
                let constraint = Constraint::driving(Relation::OnAxis {
                    point: id.clone(),
                    axis,
                });
                operation
                    .intent
                    .constraints
                    .insert(key.clone(), constraint.clone());
                operation.candidate.constraints.insert(key, constraint);
            }
        }
        curvyo_sketch_core::validate_candidate(&operation.intent, &operation.candidate)
    }
}
