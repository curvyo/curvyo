//! Analytic sketch selection returns stable entity identities instead of output anchors.
use curvyo_sketch_core::{Entity, EntityId, Length, Position, Sketch};
use kurbo::{ParamCurveNearest, Shape};

/// A selectable analytic body or its defining point.
#[derive(Debug, Clone, PartialEq)]
pub struct SketchHit {
    /// Current authoritative identity.
    pub entity: EntityId,
    /// True for a point handle, false for the body of a curve.
    pub point: bool,
}

/// Picks the nearest geometry, preferring points only at equal distance.
/// Current intent filters stale last-valid topology out of the editable hit set.
#[must_use]
pub fn sketch_hit(
    intent: &Sketch,
    solution: &Sketch,
    pointer: Position,
    tolerance: Length,
) -> Option<SketchHit> {
    sketch_hits(intent, solution, pointer, tolerance)
        .into_iter()
        .next()
}

/// Eligible hits ordered by geometric distance, points winning only equal-distance ties.
#[must_use]
pub fn sketch_hits(
    intent: &Sketch,
    solution: &Sketch,
    pointer: Position,
    tolerance: Length,
) -> Vec<SketchHit> {
    let internal_focus = solution
        .entities
        .values()
        .filter_map(|e| match e {
            Entity::Ellipse { focus, .. } => Some(focus),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    let mut points = Vec::new();
    let mut bodies = Vec::new();
    for (id, entity) in &solution.entities {
        if !intent.entities.contains_key(id) {
            continue;
        }
        if let Entity::Point { position, .. } = entity {
            if internal_focus.contains(id) {
                continue;
            }
            points.push((position.distance(pointer).0, id.clone()));
        } else if let Some(distance) = distance_to_entity(solution, entity, pointer, tolerance) {
            bodies.push((distance, id.clone()));
        }
    }
    let mut candidates = points
        .into_iter()
        .map(|(distance, id)| (distance, true, id))
        .chain(
            bodies
                .into_iter()
                .map(|(distance, id)| (distance, false, id)),
        )
        .filter(|(distance, _, _)| *distance <= tolerance.0)
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| {
        if (a.0 - b.0).abs() <= 1e-9 {
            b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2))
        } else {
            a.0.total_cmp(&b.0)
        }
    });
    let nearest = candidates
        .first()
        .map_or(f64::INFINITY, |candidate| candidate.0);
    candidates
        .into_iter()
        .filter(|candidate| candidate.0 <= nearest + 1e-9)
        .map(|(_, point, entity)| SketchHit { entity, point })
        .collect()
}

/// Picks selected curve axes independently of internal focus-point records.
#[must_use]
pub fn sketch_handle_hit(
    intent: &Sketch,
    solution: &Sketch,
    selected: &std::collections::BTreeSet<EntityId>,
    pointer: Position,
    tolerance: Length,
) -> Option<(EntityId, curvyo_sketch_core::SketchHandle)> {
    selected
        .iter()
        .filter(|id| intent.entities.contains_key(*id))
        .flat_map(|id| {
            curvyo_sketch_core::handle_positions(solution, id)
                .unwrap_or_default()
                .into_iter()
                .map(move |(handle, position)| (position.distance(pointer).0, id.clone(), handle))
        })
        .filter(|(distance, _, _)| *distance <= tolerance.0)
        .min_by(|a, b| {
            a.0.total_cmp(&b.0)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.cmp(&b.2))
        })
        .map(|(_, id, handle)| (id, handle))
}

fn distance_to_entity(
    sketch: &Sketch,
    entity: &Entity,
    pointer: Position,
    tolerance: Length,
) -> Option<f64> {
    let point = kurbo::Point::new(pointer.x.0, pointer.y.0);
    let center = |id| {
        sketch
            .point(id)
            .ok()
            .map(|pointer| kurbo::Point::new(pointer.x.0, pointer.y.0))
    };
    let path = match entity {
        Entity::Point { .. } => return None,
        Entity::Line { start, end } => {
            return Some(
                kurbo::Line::new(center(start)?, center(end)?)
                    .nearest(point, 1e-9)
                    .distance_sq
                    .sqrt(),
            );
        }
        Entity::Circle {
            center: centre,
            radius,
            ..
        } => return Some((center(centre)?.distance(point) - radius.0).abs()),
        Entity::Arc {
            center: centre,
            radius,
            start_angle,
            end_angle,
            ..
        } => {
            let sweep = (end_angle.0 - start_angle.0).rem_euclid(std::f64::consts::TAU);
            kurbo::Arc::new(
                center(centre)?,
                kurbo::Vec2::new(radius.0, radius.0),
                start_angle.0,
                sweep,
                0.0,
            )
            .to_path((tolerance.0 * 0.01).max(1e-9))
        }
        Entity::Ellipse {
            center: centre,
            focus,
            minor_radius,
            ..
        } => {
            let centre = center(centre)?;
            let focus_point = center(focus)?;
            let axis = focus_point - centre;
            kurbo::Ellipse::new(
                centre,
                (minor_radius.0.hypot(axis.hypot()), minor_radius.0),
                axis.y.atan2(axis.x),
            )
            .to_path((tolerance.0 * 0.01).max(1e-9))
        }
    };
    path.segments()
        .map(|segment| segment.nearest(point, 1e-9).distance_sq.sqrt())
        .min_by(f64::total_cmp)
}

/// Current analytic points and visible axis handles usable as atomic snap targets.
#[must_use]
pub fn sketch_snap_points(intent: &Sketch, solution: &Sketch) -> Vec<(EntityId, Position)> {
    let mut points = Vec::new();
    for (id, entity) in &intent.entities {
        let focus = intent
            .entities
            .values()
            .any(|e| matches!(e, Entity::Ellipse {focus,..} if focus==id));
        if matches!(entity, Entity::Point { .. }) && !focus {
            if let Ok(point) = solution.point(id) {
                points.push((id.clone(), point));
            }
        } else if let Ok(handles) = curvyo_sketch_core::handle_positions(solution, id) {
            points.extend(handles.into_values().map(|point| (id.clone(), point)));
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ellipse_body_hit_retains_analytic_identity_and_stale_ids_are_excluded() {
        let graph = curvyo_sketch_core::create_ellipse(
            &Sketch::new("origin".into()),
            "e",
            Position::mm(20.0, 20.0),
            Position::mm(30.0, 20.0),
            Length(5.0),
        )
        .unwrap();
        let hit = sketch_hit(&graph, &graph, Position::mm(20.0, 25.0), Length(0.1)).unwrap();
        assert_eq!(
            hit,
            SketchHit {
                entity: "e:curve".into(),
                point: false
            }
        );
        let mut current = graph.clone();
        current.entities.remove(&"e:curve".into());
        assert_eq!(
            sketch_hit(&current, &graph, Position::mm(20.0, 25.0), Length(0.1)),
            None
        );
    }
}
