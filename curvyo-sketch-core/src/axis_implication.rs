//! Coordinate propagation avoids duplicate automatic axis relationships.
use crate::{Axis, Entity, EntityId, Relation, Sketch};
use std::collections::{BTreeMap, BTreeSet};

/// Whether existing fixed coordinates and line directions already imply this zero coordinate.
/// This conservative check never removes user constraints or guesses from free seed positions.
#[must_use]
pub fn axis_is_implied(graph: &Sketch, point: &EntityId, axis: Axis) -> bool {
    let mut fixed = graph
        .entities
        .iter()
        .filter(|(_, e)| matches!(e, Entity::Point { fixed: true, .. }))
        .map(|(id, _)| id.clone())
        .collect::<BTreeSet<_>>();
    for constraint in graph.constraints.values().filter(|c| !c.reference) {
        if let Relation::Fixed(id) = &constraint.relation {
            fixed.insert(id.clone());
            if let Some(entity) = graph.entities.get(id) {
                fixed.extend(entity.point_ids().into_iter().cloned());
            }
        }
    }
    let mut zero = [BTreeSet::new(), BTreeSet::new()];
    for id in &fixed {
        if let Ok(p) = graph.point(id) {
            if p.y.0.abs() <= 1e-9 {
                zero[0].insert(id.clone());
            }
            if p.x.0.abs() <= 1e-9 {
                zero[1].insert(id.clone());
            }
        }
    }
    let mut directions = BTreeMap::<EntityId, usize>::new();
    for (id, entity) in &graph.entities {
        if let Entity::Line { start, end } = entity
            && fixed.contains(start)
            && fixed.contains(end)
            && let (Ok(a), Ok(b)) = (graph.point(start), graph.point(end))
        {
            if (a.y.0 - b.y.0).abs() <= 1e-9 {
                directions.insert(id.clone(), 0);
            }
            if (a.x.0 - b.x.0).abs() <= 1e-9 {
                directions.insert(id.clone(), 1);
            }
        }
    }
    loop {
        let before = (zero[0].len(), zero[1].len(), directions.len());
        for constraint in graph.constraints.values().filter(|c| !c.reference) {
            match &constraint.relation {
                Relation::OnAxis { point, axis } => {
                    zero[index(*axis)].insert(point.clone());
                }
                Relation::Horizontal(id) => {
                    directions.insert(id.clone(), 0);
                }
                Relation::Vertical(id) => {
                    directions.insert(id.clone(), 1);
                }
                Relation::Parallel(a, b) | Relation::Perpendicular(a, b) => {
                    let flip =
                        usize::from(matches!(constraint.relation, Relation::Perpendicular(..)));
                    if let Some(direction) = directions.get(a).copied() {
                        directions.insert(b.clone(), direction ^ flip);
                    } else if let Some(direction) = directions.get(b).copied() {
                        directions.insert(a.clone(), direction ^ flip);
                    }
                }
                _ => {}
            }
        }
        for (id, entity) in &graph.entities {
            let Entity::Line { start, end } = entity else {
                continue;
            };
            for (direction, points) in zero.iter_mut().enumerate() {
                if points.contains(start) && points.contains(end) {
                    directions.insert(id.clone(), direction);
                }
                if directions.get(id) == Some(&direction) {
                    if points.contains(start) {
                        points.insert(end.clone());
                    }
                    if points.contains(end) {
                        points.insert(start.clone());
                    }
                }
            }
        }
        if before == (zero[0].len(), zero[1].len(), directions.len()) {
            break;
        }
    }
    zero[index(axis)].contains(point)
}
fn index(axis: Axis) -> usize {
    match axis {
        Axis::X => 0,
        Axis::Y => 1,
    }
}
