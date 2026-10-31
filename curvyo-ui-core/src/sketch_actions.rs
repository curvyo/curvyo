//! Sketch action applicability is derived from current analytic selection types.
use curvyo_sketch_core::{Entity, EntityId, Expression, Relation, Sketch};

/// User-visible geometric constraints and dimensions, in toolbar order.
pub const SKETCH_ACTIONS: &[&str] = &[
    "coincident",
    "horizontal",
    "vertical",
    "parallel",
    "perpendicular",
    "tangent",
    "equal",
    "fixed",
    "symmetric",
    "midpoint",
    "distance",
    "length",
    "radius",
    "diameter",
    "angle",
];

/// Forms a relationship only when the current selected entity kinds support it.
/// # Errors
/// Returns a concise selection reason suitable for a focusable disabled action.
pub fn sketch_relation(
    action: &str,
    graph: &Sketch,
    selected: &[EntityId],
    value: Option<&Expression>,
) -> Result<Relation, String> {
    let entries = selected
        .iter()
        .map(|id| graph.entities.get(id))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| "Selection contains an entity that no longer exists".to_owned())?;
    let result = match entries.as_slice() {
        [entity] => unary(action, &selected[0], entity, value),
        [first, second] => binary(action, &selected[0], first, &selected[1], second, value),
        [_, _, _] if action == "symmetric" => symmetry(selected, &entries),
        _ => None,
    };
    result.ok_or_else(|| reason(action).to_owned())
}
fn unary(
    action: &str,
    id: &EntityId,
    entity: &Entity,
    value: Option<&Expression>,
) -> Option<Relation> {
    let curve = id.clone();
    Some(match (action, entity) {
        ("fixed", _) => Relation::Fixed(curve),
        ("horizontal", Entity::Line { .. }) => Relation::Horizontal(curve),
        ("vertical", Entity::Line { .. }) => Relation::Vertical(curve),
        ("length", Entity::Line { .. } | Entity::Arc { .. }) => Relation::Length {
            curve,
            value: value?.clone(),
        },
        ("radius", Entity::Circle { .. } | Entity::Arc { .. }) => Relation::Radius {
            curve,
            value: value?.clone(),
        },
        ("diameter", Entity::Circle { .. } | Entity::Arc { .. }) => Relation::Diameter {
            curve,
            value: value?.clone(),
        },
        _ => return None,
    })
}
fn binary(
    action: &str,
    first: &EntityId,
    first_entity: &Entity,
    second: &EntityId,
    second_entity: &Entity,
    value: Option<&Expression>,
) -> Option<Relation> {
    let a = first.clone();
    let b = second.clone();
    let lines = matches!(
        (first_entity, second_entity),
        (Entity::Line { .. }, Entity::Line { .. })
    );
    let points = matches!(
        (first_entity, second_entity),
        (Entity::Point { .. }, Entity::Point { .. })
    );
    let round = |entity: &Entity| matches!(entity, Entity::Circle { .. } | Entity::Arc { .. });
    Some(match action {
        "coincident" if points => Relation::Coincident(a, b),
        "parallel" if lines => Relation::Parallel(a, b),
        "perpendicular" if lines => Relation::Perpendicular(a, b),
        "angle" if lines => Relation::Angle {
            a,
            b,
            value: value?.clone(),
        },
        "equal" if lines || (round(first_entity) && round(second_entity)) => Relation::Equal(a, b),
        "tangent"
            if !lines
                && !matches!(first_entity, Entity::Point { .. })
                && !matches!(second_entity, Entity::Point { .. }) =>
        {
            Relation::Tangent(a, b)
        }
        "distance" if points => Relation::Distance {
            a,
            b,
            value: value?.clone(),
        },
        "distance" | "midpoint" => {
            let (point, line) = match (first_entity, second_entity) {
                (Entity::Point { .. }, Entity::Line { .. }) => (a, b),
                (Entity::Line { .. }, Entity::Point { .. }) => (b, a),
                _ => return None,
            };
            if action == "midpoint" {
                Relation::Midpoint { point, line }
            } else {
                Relation::Distance {
                    a: point,
                    b: line,
                    value: value?.clone(),
                }
            }
        }
        _ => return None,
    })
}
fn symmetry(selected: &[EntityId], entries: &[&Entity]) -> Option<Relation> {
    let mut points = Vec::new();
    let mut axis = None;
    for (id, entity) in selected.iter().zip(entries) {
        match entity {
            Entity::Point { .. } => points.push(id.clone()),
            Entity::Line { .. } => axis = Some(id.clone()),
            _ => return None,
        }
    }
    if points.len() != 2 {
        return None;
    }
    Some(Relation::Symmetric {
        source: points[0].clone(),
        image: points[1].clone(),
        axis: axis?,
    })
}

fn reason(action: &str) -> &str {
    match action {
        "coincident" => "Select two points",
        "horizontal" | "vertical" => "Select one segment",
        "parallel" | "perpendicular" | "angle" => "Select two segments",
        "equal" => "Select two segments or two circles/arcs",
        "tangent" => "Select two tangent-capable curves",
        "fixed" => "Select one entity",
        "symmetric" => "Select two points and an axis segment",
        "midpoint" => "Select one point and one segment",
        "distance" => "Select two points or a point and segment",
        "length" => "Select one segment or circular arc",
        "radius" | "diameter" => "Select one circle or circular arc",
        _ => "Unknown sketch action",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_sketch_core::{Constraint, Length, Position, Quantity};
    #[test]
    fn all_fifteen_actions_resolve_typed_selection_without_solving() {
        let graph = curvyo_sketch_core::create_rectangle(
            &Sketch::new("origin".into()),
            "r",
            Position::mm(10.0, 10.0),
            Position::mm(30.0, 20.0),
        )
        .unwrap();
        let graph =
            curvyo_sketch_core::create_circle(&graph, "c", Position::mm(40.0, 40.0), Length(5.0))
                .unwrap();
        let graph =
            curvyo_sketch_core::create_circle(&graph, "d", Position::mm(50.0, 40.0), Length(5.0))
                .unwrap();
        for action in SKETCH_ACTIONS {
            let ids = match *action {
                "coincident" | "distance" => vec!["r:p0", "r:p1"],
                "horizontal" | "vertical" | "fixed" | "length" => vec!["r:line0"],
                "parallel" | "perpendicular" | "angle" => vec!["r:line0", "r:line1"],
                "equal" | "tangent" => vec!["c:curve", "d:curve"],
                "radius" | "diameter" => vec!["c:curve"],
                "symmetric" => vec!["r:p0", "r:p1", "r:line2"],
                "midpoint" => vec!["r:p0", "r:line2"],
                _ => panic!("uncovered action"),
            }
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();
            let value = if *action == "angle" {
                Quantity::angle(0.5)
            } else {
                Quantity::length(5.0)
            }
            .unwrap();
            let relation =
                sketch_relation(action, &graph, &ids, Some(&Expression::Literal(value))).unwrap();
            let mut candidate = graph.clone();
            candidate
                .constraints
                .insert("test".into(), Constraint::driving(relation));
            candidate.validate().unwrap();
            assert!(sketch_relation(action, &graph, &[], None).is_err());
        }
    }
}
