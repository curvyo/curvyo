//! Axis inference proposes explicit local-frame relationships for confirmed points.
use super::Session;
use curvyo_sketch_core::{
    Axis, Constraint, ConstructionResult, Entity, Position, Relation, Sketch,
};

impl Session {
    pub(super) fn infer_sketch_axes(&self, mut point: Position) -> Position {
        // Proposal: a six-screen-pixel band, independent of Vector snap settings.
        let tolerance = 6.0 / self.view().scale();
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
