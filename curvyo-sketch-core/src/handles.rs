//! Analytic curve handles describe geometric axes rather than solver variables.
use crate::{Entity, EntityId, Position, Sketch, SketchError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A visible control of a round analytic curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SketchHandle {
    /// Positive local X radius point of a circle.
    Radius,
    /// Positive major-axis endpoint of an ellipse.
    MajorAxis,
    /// Positive minor-axis endpoint of an ellipse.
    MinorAxis,
}

/// Resolves actual curve handle positions in local sketch coordinates.
/// # Errors
/// Rejects missing or malformed curve references.
pub fn handle_positions(
    graph: &Sketch,
    id: &EntityId,
) -> Result<BTreeMap<SketchHandle, Position>, SketchError> {
    let mut result = BTreeMap::new();
    match graph.entities.get(id) {
        Some(Entity::Circle { center, radius, .. }) => {
            let center = graph.point(center)?;
            result.insert(
                SketchHandle::Radius,
                Position::mm(center.x.0 + radius.0, center.y.0),
            );
        }
        Some(Entity::Ellipse {
            center,
            focus,
            minor_radius,
            ..
        }) => {
            let center = graph.point(center)?;
            let focus = graph.point(focus)?;
            let dx = focus.x.0 - center.x.0;
            let dy = focus.y.0 - center.y.0;
            let distance = dx.hypot(dy);
            let (ux, uy) = if distance > 1e-9 {
                (dx / distance, dy / distance)
            } else {
                (1.0, 0.0)
            };
            let major = distance.hypot(minor_radius.0);
            result.insert(
                SketchHandle::MajorAxis,
                Position::mm(center.x.0 + ux * major, center.y.0 + uy * major),
            );
            result.insert(
                SketchHandle::MinorAxis,
                Position::mm(
                    center.x.0 - uy * minor_radius.0,
                    center.y.0 + ux * minor_radius.0,
                ),
            );
        }
        Some(_) => {}
        None => return Err(SketchError::Invalid("curve no longer exists".into())),
    }
    Ok(result)
}
