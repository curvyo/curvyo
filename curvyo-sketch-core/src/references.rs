//! Reference traversal identifies explicit dependants for atomic sketch edits.
use crate::{Entity, EntityId, Expression, ParameterId, Relation, Sketch, SketchError};
use std::collections::BTreeSet;

impl Entity {
    /// Defining point records, excluding the entity's own identity.
    #[must_use]
    pub fn point_ids(&self) -> Vec<&EntityId> {
        match self {
            Self::Point { .. } => Vec::new(),
            Self::Line { start, end } => vec![start, end],
            Self::Circle { center, .. } => vec![center],
            Self::Arc {
                center, start, end, ..
            } => vec![center, start, end],
            Self::Ellipse { center, focus, .. } => vec![center, focus],
        }
    }
}
impl Relation {
    /// Every directly referenced geometric record, including construction helpers.
    #[must_use]
    pub fn entity_ids(&self) -> Vec<&EntityId> {
        match self {
            Self::On { point, curve } => vec![point, curve],
            Self::Sweep { arc, .. } => vec![arc],
            Self::PerpendicularAt { a, b, point } => vec![a, b, point],
            Self::OnAxis { point, .. } => vec![point],
            Self::Horizontal(id) | Self::Vertical(id) | Self::Fixed(id) => vec![id],
            Self::Coincident(a, b)
            | Self::Parallel(a, b)
            | Self::Perpendicular(a, b)
            | Self::Tangent(a, b)
            | Self::Equal(a, b)
            | Self::Distance { a, b, .. }
            | Self::Angle { a, b, .. } => vec![a, b],
            Self::Symmetric {
                source,
                image,
                axis,
            } => vec![source, image, axis],
            Self::Midpoint { point, line } => vec![point, line],
            Self::Length { curve, .. }
            | Self::Radius { curve, .. }
            | Self::Diameter { curve, .. } => vec![curve],
        }
    }

    /// The driving or reference dimension expression, when this relation has one.
    #[must_use]
    pub const fn expression(&self) -> Option<&Expression> {
        match self {
            Self::Sweep { value, .. }
            | Self::Distance { value, .. }
            | Self::Length { value, .. }
            | Self::Radius { value, .. }
            | Self::Diameter { value, .. }
            | Self::Angle { value, .. } => Some(value),
            _ => None,
        }
    }

    /// Stable parameter dependencies in a dimension's expression.
    #[must_use]
    pub fn parameter_ids(&self) -> BTreeSet<ParameterId> {
        let mut ids = Vec::new();
        if let Some(expression) = self.expression() {
            expression.references(&mut ids);
        }
        ids.into_iter().collect()
    }
}

/// Removes selected geometry plus entities/relations that directly depend on it.
/// Unreferenced backing points remain available for deliberate subsequent selection.
/// # Errors
/// Refuses deleting the sketch origin or an identity absent from current intent.
pub fn remove_entities(
    graph: &Sketch,
    selected: &BTreeSet<EntityId>,
) -> Result<Sketch, SketchError> {
    if selected.contains(&graph.origin)
        || selected.iter().any(|id| !graph.entities.contains_key(id))
    {
        return Err(SketchError::Invalid(
            "selection cannot be deleted from this sketch".into(),
        ));
    }
    let mut removed = selected.clone();
    loop {
        let before = removed.len();
        for (id, entity) in &graph.entities {
            if entity
                .point_ids()
                .iter()
                .any(|point| removed.contains(*point))
            {
                removed.insert(id.clone());
            }
        }
        if removed.len() == before {
            break;
        }
    }
    let mut result = graph.clone();
    result.entities.retain(|id, _| !removed.contains(id));
    result.construction.retain(|id| !removed.contains(id));
    result.constraints.retain(|_, constraint| {
        !constraint
            .relation
            .entity_ids()
            .iter()
            .any(|id| removed.contains(*id))
    });
    result
        .contact_seeds
        .retain(|id, _| result.constraints.contains_key(id));
    Ok(result)
}
