//! Sketch action applicability is derived from current analytic selection types.
use curvyo_sketch_core::{Axis, Entity, EntityId, Expression, Length, Position, Relation, Sketch};

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
    "horizontal-distance",
    "vertical-distance",
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
    if let Some(axis) = measured_axis(action) {
        return axis_distance(axis, graph, selected, &entries, value)
            .ok_or_else(|| reason(action).to_owned());
    }
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
        (_, [_, _, _]) if action == "symmetric" => return symmetry(graph, selected, &entries),
        _ => None,
    };
    result.ok_or_else(|| reason(action).to_owned())
}
/// The local axis a Horizontal or Vertical distance measures along.
fn measured_axis(action: &str) -> Option<Axis> {
    match action {
        "horizontal-distance" => Some(Axis::X),
        "vertical-distance" => Some(Axis::Y),
        _ => None,
    }
}

/// A distance along `axis` between two points, a line's endpoints, or a point and the origin
/// (picked as the origin or as the axis the distance is measured to). The point with the smaller
/// coordinate is stored first, so the dimension keeps the side the geometry has (AC46); one point
/// twice is no distance.
fn axis_distance(
    axis: Axis,
    graph: &Sketch,
    selected: &[EntityId],
    entries: &[Target],
    value: Option<&Expression>,
) -> Option<Relation> {
    // The axis a point's distance is measured to: across the measured direction.
    let to = match axis {
        Axis::X => Axis::Y,
        Axis::Y => Axis::X,
    };
    let (a, b) = match entries {
        [Target::Entity(Entity::Line { start, end })] => (start.clone(), end.clone()),
        [
            Target::Entity(Entity::Point { .. }),
            Target::Entity(Entity::Point { .. }),
        ] => (selected[0].clone(), selected[1].clone()),
        [Target::Entity(Entity::Point { .. }), Target::Axis(picked)] if *picked == to => {
            (graph.origin.clone(), selected[0].clone())
        }
        [Target::Axis(picked), Target::Entity(Entity::Point { .. })] if *picked == to => {
            (graph.origin.clone(), selected[1].clone())
        }
        _ => return None,
    };
    // The origin picked with the matching axis would measure the origin to itself.
    if a == b {
        return None;
    }
    let coordinate = |id: &EntityId| {
        graph.point(id).ok().map(|p| match axis {
            Axis::X => p.x.0,
            Axis::Y => p.y.0,
        })
    };
    let (a, b) = if coordinate(&a)? > coordinate(&b)? {
        (b, a)
    } else {
        (a, b)
    };
    let value = value?.clone();
    Some(match axis {
        Axis::X => Relation::HorizontalDistance { a, b, value },
        Axis::Y => Relation::VerticalDistance { a, b, value },
    })
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

/// Below this difference two points are equally near the middle of the others.
const CENTRE_TIE: Length = Length(1e-6);

/// Two points mirrored about a third point (its centre), a line or an axis, in any order.
fn symmetry(graph: &Sketch, selected: &[EntityId], entries: &[Target]) -> Result<Relation, String> {
    let refused = || reason("symmetric").to_owned();
    let mut points = Vec::new();
    let mut mirror = None;
    for (id, entry) in selected.iter().zip(entries) {
        match entry {
            Target::Entity(Entity::Point { .. }) => points.push(id.clone()),
            Target::Entity(Entity::Line { .. }) | Target::Axis(_) if mirror.is_none() => {
                mirror = Some((id.clone(), *entry));
            }
            _ => return Err(refused()),
        }
    }
    if mirror.is_none() && points.len() == 3 {
        return symmetric_centre(graph, &points);
    }
    let ([source, image], Some((axis_id, entry))) = (points.as_slice(), mirror) else {
        return Err(refused());
    };
    Ok(match entry {
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

/// Three points selected first: the centre is the one nearest the midpoint of the other two,
/// whatever the selection order (0066 AC51); an equal distance is refused as ambiguous.
fn symmetric_centre(graph: &Sketch, points: &[EntityId]) -> Result<Relation, String> {
    let position = |id: &EntityId| graph.point(id).map_err(|e| e.to_string());
    let mut scored = Vec::with_capacity(3);
    for (i, centre) in points.iter().enumerate() {
        let others = points
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, id)| id)
            .collect::<Vec<_>>();
        let (a, b, c) = (
            position(others[0])?,
            position(others[1])?,
            position(centre)?,
        );
        let middle = Position::mm(f64::midpoint(a.x.0, b.x.0), f64::midpoint(a.y.0, b.y.0));
        scored.push((c.distance(middle), i));
    }
    scored.sort_by(|a, b| a.0.0.total_cmp(&b.0.0));
    if scored[1].0.0 - scored[0].0.0 <= CENTRE_TIE.0 {
        return Err(AMBIGUOUS_CENTRE.to_owned());
    }
    sketch_symmetric_about(graph, points, &points[scored[0].1])
        .ok_or_else(|| reason("symmetric").to_owned())
}

/// The one Symmetric-about-a-point rule (0066 AC51): three points, `centre` among them, the
/// other two mirrored about it (stored in id order). `None` for any other picks.
#[must_use]
pub fn sketch_symmetric_about(
    graph: &Sketch,
    picks: &[EntityId],
    centre: &EntityId,
) -> Option<Relation> {
    let point = |id: &EntityId| matches!(graph.entities.get(id), Some(Entity::Point { .. }));
    if picks.len() != 3 || !picks.iter().all(point) || !picks.contains(centre) {
        return None;
    }
    let mut others = picks
        .iter()
        .filter(|id| *id != centre)
        .cloned()
        .collect::<Vec<_>>();
    others.sort();
    let [source, image] = <[EntityId; 2]>::try_from(others).ok()?;
    Some(Relation::SymmetricPoint {
        source,
        image,
        center: centre.clone(),
    })
}

/// Why three points selected first give no Symmetric (0066 AC51).
const AMBIGUOUS_CENTRE: &str = "The centre is ambiguous: two of the points are equally near the middle of the others; arm Symmetric and pick the centre last";

fn reason(action: &str) -> &str {
    match action {
        "coincident" => "Select two points, or a point and a line, circle, arc, ellipse or axis",
        "horizontal" | "vertical" => "Select one segment or two points",
        "parallel" => "Select two or more segments",
        "perpendicular" | "angle" => "Select two segments",
        "equal" => "Select two or more segments or circles/arcs",
        "tangent" => "Select two tangent-capable curves",
        "fixed" => "Select one or more entities",
        "symmetric" => {
            "Select two points and a centre point, a line or an axis; picked one by one, the last point is the centre"
        }
        "midpoint" => "Select one point and one segment",
        "distance" => "Select two points or a point and segment",
        "horizontal-distance" => "Select two points, one line, or a point and the Y axis",
        "vertical-distance" => "Select two points, one line, or a point and the X axis",
        "length" => "Select one segment or circular arc",
        "radius" | "diameter" => "Select one circle or circular arc",
        _ => "Unknown sketch action",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_sketch_core::{Constraint, Quantity};

    /// The side at creation is kept: the left (lower) point is stored first (0066 AC46).
    #[test]
    fn axis_distances_store_the_left_or_lower_point_first() {
        let graph = curvyo_sketch_core::create_polyline(
            &Sketch::new("origin".into()),
            "l",
            &[Position::mm(30.0, 5.0), Position::mm(10.0, 25.0)],
            false,
        )
        .unwrap();
        let value = Expression::Literal(Quantity::length(1.0).unwrap());
        let ids = |names: &[&str]| names.iter().map(|n| EntityId::from(*n)).collect::<Vec<_>>();
        for selection in [
            ids(&["l:line0"]),
            ids(&["l:p0", "l:p1"]),
            ids(&["l:p1", "l:p0"]),
        ] {
            assert_eq!(
                sketch_relation("horizontal-distance", &graph, &selection, Some(&value)),
                Ok(Relation::HorizontalDistance {
                    a: "l:p1".into(),
                    b: "l:p0".into(),
                    value: value.clone(),
                })
            );
            assert_eq!(
                sketch_relation("vertical-distance", &graph, &selection, Some(&value)),
                Ok(Relation::VerticalDistance {
                    a: "l:p0".into(),
                    b: "l:p1".into(),
                    value: value.clone(),
                })
            );
        }
        // A point and the Y axis: the horizontal distance from the origin.
        assert_eq!(
            sketch_relation(
                "horizontal-distance",
                &graph,
                &ids(&[SKETCH_AXIS_Y, "l:p1"]),
                Some(&value)
            ),
            Ok(Relation::HorizontalDistance {
                a: "origin".into(),
                b: "l:p1".into(),
                value: value.clone(),
            })
        );
        // The origin and the matching axis would measure the origin to itself.
        for (kind, axis) in [
            ("horizontal-distance", SKETCH_AXIS_Y),
            ("vertical-distance", SKETCH_AXIS_X),
        ] {
            assert!(
                sketch_relation(kind, &graph, &ids(&["origin", axis]), Some(&value)).is_err(),
                "{kind}"
            );
        }
    }
    #[test]
    fn every_action_resolves_typed_selection_without_solving() {
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
                "coincident" | "distance" | "horizontal-distance" | "vertical-distance" => {
                    vec!["r:p0", "r:p2"]
                }
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

    fn three_points(c: (f64, f64)) -> Sketch {
        let mut graph = Sketch::new("origin".into());
        for (id, (x, y)) in [("a", (0.0, 0.0)), ("b", (10.0, 0.0)), ("c", c)] {
            graph.entities.insert(
                id.into(),
                Entity::Point {
                    position: Position::mm(x, y),
                    fixed: false,
                },
            );
        }
        graph
    }

    /// 0066 AC51: three selected points take the one nearest the midpoint of the other two as
    /// the centre, whatever the order.
    #[test]
    fn the_symmetric_centre_is_the_point_nearest_the_midpoint_of_the_others() {
        let graph = three_points((4.0, 0.0));
        let orders: [[&str; 3]; 6] = [
            ["a", "b", "c"],
            ["a", "c", "b"],
            ["b", "a", "c"],
            ["b", "c", "a"],
            ["c", "a", "b"],
            ["c", "b", "a"],
        ];
        for order in orders {
            let ids = order.iter().map(|n| EntityId::from(*n)).collect::<Vec<_>>();
            assert_eq!(
                sketch_relation("symmetric", &graph, &ids, None),
                Ok(Relation::SymmetricPoint {
                    source: "a".into(),
                    image: "b".into(),
                    center: "c".into(),
                }),
                "{order:?}"
            );
        }
    }

    /// 0066 AC51: picked one by one, the named centre wins whatever the geometry; anything but
    /// three points with the centre among them gives no relation.
    #[test]
    fn symmetric_about_the_named_centre_needs_three_points_including_it() {
        let mut graph = three_points((4.0, 0.0));
        let ids = ["c", "b", "a"].map(EntityId::from).to_vec();
        assert_eq!(
            sketch_symmetric_about(&graph, &ids, &"a".into()),
            Some(Relation::SymmetricPoint {
                source: "b".into(),
                image: "c".into(),
                center: "a".into(),
            })
        );
        assert_eq!(sketch_symmetric_about(&graph, &ids[..2], &"a".into()), None);
        assert_eq!(sketch_symmetric_about(&graph, &ids, &"z".into()), None);
        graph.entities.insert(
            "l".into(),
            Entity::Line {
                start: "a".into(),
                end: "b".into(),
            },
        );
        let with_line = ["a", "b", "l"].map(EntityId::from).to_vec();
        assert_eq!(
            sketch_symmetric_about(&graph, &with_line, &"l".into()),
            None
        );
    }

    /// 0066 AC51: an equal distance within 1e-6 mm names the ambiguity instead of guessing.
    #[test]
    fn an_ambiguous_symmetric_centre_is_refused_with_its_reason() {
        let graph = three_points((5.0, 75.0_f64.sqrt()));
        let ids = ["a", "b", "c"].map(EntityId::from).to_vec();
        let reason = sketch_relation("symmetric", &graph, &ids, None).unwrap_err();
        assert!(reason.contains("centre"), "{reason}");
        assert!(reason.contains("ambiguous"), "{reason}");
    }
}
