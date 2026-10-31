//! Two proper rigid rotations certify a zero-cost positional drag candidate.
use crate::{Entity, EntityId, Position, Sketch};
use std::collections::BTreeMap;

// Ill-conditioned intersections fall back to ACS; this is not a goal clamp.
const CONDITION_EPS: f64 = 1e-9;
const GOAL_TOLERANCE: f64 = 1e-6;

/// Points are ordered from the certified fixed anchor to the single dragged endpoint.
pub(super) fn candidate(sketch: &Sketch, points: &[EntityId], target: Position) -> Option<Sketch> {
    let goals = BTreeMap::from([(points.last()?.clone(), target)]);
    translated_tail(sketch, points, &goals)
}

/// The complete goal set must translate a contiguous chain suffix coherently.
pub(super) fn translated_tail(
    sketch: &Sketch,
    points: &[EntityId],
    goals: &BTreeMap<EntityId, Position>,
) -> Option<Sketch> {
    if goals
        .values()
        .any(|p| !p.x.0.is_finite() || !p.y.0.is_finite())
    {
        return None;
    }
    let first = points.iter().position(|id| goals.contains_key(id))?;
    if first < 2
        || points.len() - first != goals.len()
        || points[first..].iter().any(|id| !goals.contains_key(id))
    {
        return None;
    }
    let target = *goals.get(&points[first])?;
    let old = sketch.point(&points[first]).ok()?;
    let delta = (target.x.0 - old.x.0, target.y.0 - old.y.0);
    for id in &points[first..] {
        let old = sketch.point(id).ok()?;
        let translated = Position::mm(old.x.0 + delta.0, old.y.0 + delta.1);
        if !translated.x.0.is_finite()
            || !translated.y.0.is_finite()
            || translated.distance(*goals.get(id)?).0 > CONDITION_EPS
        {
            return None;
        }
    }
    let positions = points
        .iter()
        .map(|id| sketch.point(id))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let middle = first.div_ceil(2);
    let anchor = positions[0];
    let pivot = positions[middle];
    let end = positions[first];
    let intersections = intersections(anchor, pivot, end, target)?;
    let mut best: Option<(f64, Sketch)> = None;
    for joint in intersections {
        let mut candidate = sketch.clone();
        let mut cost = 0.0;
        for (index, (id, old)) in points.iter().zip(&positions).enumerate() {
            let position = if index > first {
                Position::mm(old.x.0 + delta.0, old.y.0 + delta.1)
            } else if index <= middle {
                rotate(*old, anchor, anchor, pivot, joint)?
            } else {
                rotate(*old, pivot, joint, end, target)?
            };
            let Entity::Point {
                position: value, ..
            } = candidate.entities.get_mut(id)?
            else {
                return None;
            };
            *value = position;
            cost += old.distance(position).0.powi(2);
        }
        if !cost.is_finite()
            || goals.iter().any(|(id, raw)| {
                candidate
                    .point(id)
                    .map_or(true, |actual| actual.distance(*raw).0 > GOAL_TOLERANCE)
            })
            || crate::validate_candidate(sketch, &candidate).is_err()
        {
            continue;
        }
        // Compare the whole pose, not just the hinge; stable first-branch tie breaking.
        if best.as_ref().is_none_or(|(previous, _)| cost < *previous) {
            best = Some((cost, candidate));
        }
    }
    best.map(|(_, candidate)| candidate)
}

fn intersections(
    anchor: Position,
    pivot: Position,
    end: Position,
    target: Position,
) -> Option<[Position; 2]> {
    let first_radius = anchor.distance(pivot).0;
    let second_radius = pivot.distance(end).0;
    let distance = anchor.distance(target).0;
    let epsilon = CONDITION_EPS * first_radius.max(second_radius).max(1.0);
    if !first_radius.is_finite()
        || !second_radius.is_finite()
        || !distance.is_finite()
        || first_radius <= epsilon
        || second_radius <= epsilon
        || distance <= epsilon
        || distance >= first_radius + second_radius - epsilon
        || distance <= (first_radius - second_radius).abs() + epsilon
    {
        return None;
    }
    let along = (first_radius * first_radius - second_radius * second_radius + distance * distance)
        / (2.0 * distance);
    let height = (first_radius * first_radius - along * along).sqrt();
    if !height.is_finite() || height <= epsilon {
        return None;
    }
    let x = (target.x.0 - anchor.x.0) / distance;
    let y = (target.y.0 - anchor.y.0) / distance;
    Some([1.0, -1.0].map(|side| {
        Position::mm(
            anchor.x.0 + along * x - side * height * y,
            anchor.y.0 + along * y + side * height * x,
        )
    }))
}

fn rotate(
    point: Position,
    from: Position,
    to: Position,
    old_tip: Position,
    new_tip: Position,
) -> Option<Position> {
    let a = (old_tip.x.0 - from.x.0, old_tip.y.0 - from.y.0);
    let b = (new_tip.x.0 - to.x.0, new_tip.y.0 - to.y.0);
    let norm = a.0.hypot(a.1) * b.0.hypot(b.1);
    if !norm.is_finite() || norm <= 0.0 {
        return None;
    }
    let cos = (a.0 * b.0 + a.1 * b.1) / norm;
    let sin = (a.0 * b.1 - a.1 * b.0) / norm;
    let (x, y) = (point.x.0 - from.x.0, point.y.0 - from.y.0);
    Some(Position::mm(
        to.x.0 + cos * x - sin * y,
        to.y.0 + sin * x + cos * y,
    ))
}

#[cfg(test)]
#[path = "drag_chords_tests.rs"]
mod tests;
