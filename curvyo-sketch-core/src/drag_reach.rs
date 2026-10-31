//! Distance-chain certificates preserve the raw positional objectives.
use crate::{Entity, EntityId, Length, Position, Relation, Sketch, SolveMode};
use std::collections::{BTreeMap, BTreeSet};

struct Chain {
    points: Vec<(EntityId, Length)>,
    anchor: EntityId,
    length: Length,
}

/// Returns only a validated pose attaining a proven positional lower bound.
pub(crate) fn candidate(sketch: &Sketch, mode: &SolveMode) -> Option<Sketch> {
    if let SolveMode::DragPoints { goals } = mode
        && goals.len() > 1
    {
        return translated_tail(sketch, goals);
    }
    let (endpoint, target) = match mode {
        SolveMode::Drag { point, target } => (point, *target),
        SolveMode::DragPoints { goals } if goals.len() == 1 => {
            let (point, target) = goals.first_key_value()?;
            (point, *target)
        }
        _ => return None,
    };
    let chain = chain(sketch, endpoint)?;
    let anchor = sketch.point(&chain.anchor).ok()?;
    let distance = anchor.distance(target);
    if !chain.length.is_finite() || !distance.is_finite() {
        return None;
    }
    if distance.0 <= chain.length.0 + 1e-9 {
        let points = chain
            .points
            .iter()
            .rev()
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        return crate::drag_chords::candidate(sketch, &points, target);
    }
    let direction = (
        (target.x.0 - anchor.x.0) / distance.0,
        (target.y.0 - anchor.y.0) / distance.0,
    );
    let mut result = sketch.clone();
    for (id, from_endpoint) in chain.points {
        if id == chain.anchor {
            continue;
        }
        let Entity::Point { position, .. } = result.entities.get_mut(&id)? else {
            return None;
        };
        let along = chain.length.0 - from_endpoint.0;
        *position = Position::mm(
            anchor.x.0 + along * direction.0,
            anchor.y.0 + along * direction.1,
        );
    }
    // Extra equations, fixed geometry and disconnected entities remain authoritative.
    // The optimization is inapplicable if the straight pose violates any of them.
    crate::validate_candidate(sketch, &result).ok()?;
    let achieved = result.point(endpoint).ok()?.distance(target);
    ((achieved.0 - (distance.0 - chain.length.0)).abs() <= 1e-6).then_some(result)
}

fn translated_tail(sketch: &Sketch, goals: &BTreeMap<EntityId, Position>) -> Option<Sketch> {
    // Only a complete contiguous goal suffix of one certified chain is eligible.
    // A foreign goal or a gap cannot be dropped from the positional objective.
    for endpoint in goals.keys() {
        let Some(chain) = chain(sketch, endpoint) else {
            continue;
        };
        let points = chain
            .points
            .iter()
            .rev()
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        if let Some(result) = crate::drag_chords::translated_tail(sketch, &points, goals) {
            return Some(result);
        }
    }
    None
}

fn chain(sketch: &Sketch, endpoint: &EntityId) -> Option<Chain> {
    let edges = edges(sketch)?;
    if edges.get(endpoint)?.len() != 1 {
        return None;
    }
    let fixed = |id: &EntityId| {
        matches!(
            sketch.entities.get(id),
            Some(Entity::Point { fixed: true, .. })
        ) || sketch
            .constraints
            .values()
            .any(|c| !c.reference && matches!(&c.relation, Relation::Fixed(point) if point == id))
    };
    if fixed(endpoint) {
        return None;
    }
    let mut points = vec![(endpoint.clone(), Length(0.0))];
    let mut visited = BTreeSet::from([endpoint.clone()]);
    let mut current = endpoint;
    let mut length = Length(0.0);
    while !fixed(current) {
        let adjacent = edges.get(current)?;
        if current != endpoint && adjacent.len() != 2 {
            return None;
        }
        let mut next = adjacent.iter().filter(|(id, _)| !visited.contains(id));
        let (id, edge_length) = next.next()?;
        if next.next().is_some() {
            return None;
        }
        length.0 += edge_length.0;
        points.push((id.clone(), length));
        visited.insert(id.clone());
        current = id;
    }
    Some(Chain {
        points,
        anchor: current.clone(),
        length,
    })
}

fn edges(sketch: &Sketch) -> Option<BTreeMap<EntityId, Vec<(EntityId, Length)>>> {
    let values = sketch.parameters.evaluate().ok()?;
    let mut edges: BTreeMap<EntityId, Vec<(EntityId, Length)>> = BTreeMap::new();
    for constraint in sketch.constraints.values().filter(|c| !c.reference) {
        if let Relation::Distance { a, b, value } = &constraint.relation {
            let length = Length(value.evaluate(&values).ok()?.value());
            if !length.is_finite() || length.0 <= 0.0 {
                return None;
            }
            edges
                .entry(a.clone())
                .or_default()
                .push((b.clone(), length));
            edges
                .entry(b.clone())
                .or_default()
                .push((a.clone(), length));
        }
    }
    Some(edges)
}
