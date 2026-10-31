//! Checkpoint geometry seeds solving without replacing authoritative intent.
use crate::{Entity, EntityId, Relation, Sketch, SketchError, SolveMode, SolveResult};
use std::collections::BTreeSet;

/// Checks cached geometry against the complete authoritative graph.
///
/// # Errors
/// Rejects changed topology, intent, invalid geometry or unsatisfied hard constraints.
pub fn validate_candidate(intent: &Sketch, candidate: &Sketch) -> Result<(), SketchError> {
    intent.validate()?;
    candidate.validate()?;
    let mut metadata = candidate.clone();
    metadata.contact_seeds = intent.contact_seeds.clone();
    for (id, entity) in &mut metadata.entities {
        if let Some(original) = intent.entities.get(id) {
            copy_geometry(entity, original, false);
        }
    }
    if metadata != *intent {
        return Err(SketchError::Invalid(
            "checkpoint changes sketch intent".into(),
        ));
    }
    crate::residuals::validate(intent, candidate, &candidate.contact_seeds)
}

/// Solves authoritative intent using compatible free geometry from a checkpoint.
/// Explicit user geometry edits must also be applied to the supplied seed by the
/// caller; dependent solved positions remain checkpoint data, not intent edits.
///
/// # Errors
/// Rejects malformed intent, non-finite seeds or invalid solver responses.
pub fn solve_seeded(
    intent: &Sketch,
    seed: &Sketch,
    mode: &SolveMode,
) -> Result<SolveResult, SketchError> {
    intent.validate()?;
    let protected = fixed_entities(intent);
    let mut input = intent.clone();
    for (id, entity) in &mut input.entities {
        if !protected.contains(id)
            && let Some(value) = seed.entities.get(id)
        {
            copy_geometry(entity, value, true);
        }
    }
    input.contact_seeds = seed
        .contact_seeds
        .iter()
        .filter(|(id, _)| {
            matches!(
                intent.constraints.get(*id).map(|c| &c.relation),
                Some(Relation::Tangent(..))
            )
        })
        .map(|(id, point)| (id.clone(), *point))
        .collect();
    let mut result = crate::solve(&input, mode)?;
    if let Some(candidate) = &result.candidate
        && let Err(error) = validate_candidate(intent, candidate)
    {
        result.candidate = None;
        result.fully_constrained = None;
        result.status = crate::SolveStatus::SolverFailure;
        result.message = Some(error.to_string());
    }
    Ok(result)
}

fn fixed_entities(sketch: &Sketch) -> BTreeSet<EntityId> {
    let mut result = BTreeSet::new();
    let mut pending: Vec<_> = sketch
        .constraints
        .values()
        .filter_map(|c| match &c.relation {
            Relation::Fixed(id) if !c.reference => Some(id.clone()),
            _ => None,
        })
        .collect();
    while let Some(id) = pending.pop() {
        if !result.insert(id.clone()) {
            continue;
        }
        match sketch.entities.get(&id) {
            Some(Entity::Line { start, end }) => pending.extend([start.clone(), end.clone()]),
            Some(Entity::Circle { center, .. }) => pending.push(center.clone()),
            Some(Entity::Arc {
                center, start, end, ..
            }) => pending.extend([center.clone(), start.clone(), end.clone()]),
            Some(Entity::Ellipse { center, focus, .. }) => {
                pending.extend([center.clone(), focus.clone()]);
            }
            _ => {}
        }
    }
    result
}

// Numeric fields may change; identity, references and fixed flags never do.
fn copy_geometry(target: &mut Entity, source: &Entity, respect_fixed: bool) {
    match (target, source) {
        (
            Entity::Point { position, fixed },
            Entity::Point {
                position: value, ..
            },
        ) if !respect_fixed || !*fixed => *position = *value,
        (
            Entity::Circle {
                center,
                radius,
                fixed_radius,
            },
            Entity::Circle {
                center: other,
                radius: value,
                ..
            },
        ) if center == other && (!respect_fixed || !*fixed_radius) => *radius = *value,
        (
            Entity::Ellipse {
                center,
                focus,
                minor_radius,
                fixed_radius,
            },
            Entity::Ellipse {
                center: other,
                focus: other_focus,
                minor_radius: value,
                ..
            },
        ) if center == other && focus == other_focus && (!respect_fixed || !*fixed_radius) => {
            *minor_radius = *value;
        }
        (
            Entity::Arc {
                center,
                start,
                end,
                radius,
                start_angle,
                end_angle,
                fixed_shape,
            },
            Entity::Arc {
                center: other,
                start: other_start,
                end: other_end,
                radius: value,
                start_angle: a,
                end_angle: b,
                ..
            },
        ) if center == other
            && start == other_start
            && end == other_end
            && (!respect_fixed || !*fixed_shape) =>
        {
            *radius = *value;
            *start_angle = *a;
            *end_angle = *b;
        }
        _ => {}
    }
}
