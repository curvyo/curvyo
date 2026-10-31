//! Sketch action applicability is derived from current analytic selection types.
use curvyo_sketch_core::{Axis, Entity, EntityId, Expression, Relation, Sketch};

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

/// The selection identity of the sketch's local X axis; never a document record.
pub const SKETCH_AXIS_X: &str = "@axis-x";
/// The selection identity of the sketch's local Y axis; never a document record.
pub const SKETCH_AXIS_Y: &str = "@axis-y";

/// The local axis a selection identity names, if it names one.
#[must_use]
pub fn sketch_axis(id: &EntityId) -> Option<Axis> {
    match id.0.as_str() {
        SKETCH_AXIS_X => Some(Axis::X),
        SKETCH_AXIS_Y => Some(Axis::Y),
        _ => None,
    }
}

/// A selected target: a geometry record or one of the local axes.
#[derive(Clone, Copy)]
enum Target<'a> {
    Entity(&'a Entity),
    Axis(Axis),
}

/// Forms the relationships one press of `action` adds for the selection, as one edit:
/// Parallel and Equal relate every further member to the first, Fixed fixes each member;
/// every other action forms exactly one relationship.
/// # Errors
/// Returns a concise selection reason suitable for a focusable disabled action.
pub fn sketch_relations(
    action: &str,
    graph: &Sketch,
    selected: &[EntityId],
    value: Option<&Expression>,
) -> Result<Vec<Relation>, String> {
    let many = match (action, selected) {
        ("fixed", [_, _, ..]) => selected
            .iter()
            .map(|id| sketch_relation(action, graph, std::slice::from_ref(id), value))
            .collect::<Result<Vec<_>, _>>()
            .ok(),
        ("parallel" | "equal", [first, _, _, ..]) => selected[1..]
            .iter()
            .map(|id| sketch_relation(action, graph, &[first.clone(), id.clone()], value))
            .collect::<Result<Vec<_>, _>>()
            .ok(),
        _ => return sketch_relation(action, graph, selected, value).map(|r| vec![r]),
    };
    many.ok_or_else(|| reason(action).to_owned())
}

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
        .map(|id| {
            sketch_axis(id)
                .map(Target::Axis)
                .or_else(|| graph.entities.get(id).map(Target::Entity))
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| "Selection contains an entity that no longer exists".to_owned())?;
    let entities = entries
        .iter()
        .map(|target| match target {
            Target::Entity(entity) => Some(*entity),
            Target::Axis(_) => None,
        })
        .collect::<Option<Vec<_>>>();
    let result = match (entities.as_deref(), entries.as_slice()) {
        (Some([entity]), _) => unary(action, &selected[0], entity, value),
        (Some([first, second]), _) => {
            binary(action, &selected[0], first, &selected[1], second, value)
        }
        (None, [_, _]) if action == "coincident" => on_axis(selected, &entries),
        (_, [_, _, _]) if action == "symmetric" => symmetry(selected, &entries),
        _ => None,
    };
    result.ok_or_else(|| reason(action).to_owned())
}
fn on_axis(selected: &[EntityId], entries: &[Target]) -> Option<Relation> {
    match entries {
        [Target::Entity(Entity::Point { .. }), Target::Axis(axis)] => Some(Relation::OnAxis {
            point: selected[0].clone(),
            axis: *axis,
        }),
        [Target::Axis(axis), Target::Entity(Entity::Point { .. })] => Some(Relation::OnAxis {
            point: selected[1].clone(),
            axis: *axis,
        }),
        _ => None,
    }
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
    if let Some(relation) = point_pair(action, first_entity, &a, second_entity, &b) {
        return Some(relation);
    }
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
/// Coincident of a point with a line, circle, arc or ellipse (on the full curve), and
/// Horizontal or Vertical of two points.
fn point_pair(
    action: &str,
    first: &Entity,
    a: &EntityId,
    second: &Entity,
    b: &EntityId,
) -> Option<Relation> {
    let curve = |entity: &Entity| !matches!(entity, Entity::Point { .. });
    match (action, first, second) {
        ("coincident", Entity::Point { .. }, other) if curve(other) => Some(Relation::PointOn {
            point: a.clone(),
            curve: b.clone(),
        }),
        ("coincident", other, Entity::Point { .. }) if curve(other) => Some(Relation::PointOn {
            point: b.clone(),
            curve: a.clone(),
        }),
        ("horizontal", Entity::Point { .. }, Entity::Point { .. }) => {
            Some(Relation::HorizontalPoints(a.clone(), b.clone()))
        }
        ("vertical", Entity::Point { .. }, Entity::Point { .. }) => {
            Some(Relation::VerticalPoints(a.clone(), b.clone()))
        }
        _ => None,
    }
}

/// Two points mirrored about a third point (its centre), a line or an axis, in any order;
/// with three points the last picked is the centre.
fn symmetry(selected: &[EntityId], entries: &[Target]) -> Option<Relation> {
    let mut points = Vec::new();
    let mut mirror = None;
    for (id, entry) in selected.iter().zip(entries) {
        match entry {
            Target::Entity(Entity::Point { .. }) => points.push(id.clone()),
            Target::Entity(Entity::Line { .. }) | Target::Axis(_) if mirror.is_none() => {
                mirror = Some((id.clone(), *entry));
            }
            _ => return None,
        }
    }
    if mirror.is_none() && points.len() == 3 {
        let center = points.pop()?;
        return Some(Relation::SymmetricPoint {
            source: points[0].clone(),
            image: points[1].clone(),
            center,
        });
    }
    let [source, image] = points.as_slice() else {
        return None;
    };
    let (axis_id, entry) = mirror?;
    Some(match entry {
        Target::Axis(axis) => Relation::SymmetricAxis {
            source: source.clone(),
            image: image.clone(),
            axis,
        },
        Target::Entity(_) => Relation::Symmetric {
            source: source.clone(),
            image: image.clone(),
            axis: axis_id,
        },
    })
}

fn reason(action: &str) -> &str {
    match action {
        "coincident" => "Select two points, or a point and a line, circle, arc, ellipse or axis",
        "horizontal" | "vertical" => "Select one segment or two points",
        "parallel" => "Select two or more segments",
        "perpendicular" | "angle" => "Select two segments",
        "equal" => "Select two or more segments or circles/arcs",
        "tangent" => "Select two tangent-capable curves",
        "fixed" => "Select one or more entities",
        "symmetric" => "Select two points and a centre point, a line or an axis",
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
