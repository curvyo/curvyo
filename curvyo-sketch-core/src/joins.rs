//! Construction picks that land on existing points hand those point identities to the new geometry.
use crate::{Construction, Entity, EntityId, Length, Position, Sketch, SketchError};

/// A new point within this distance of a pick was created at that pick.
const PICK_TOLERANCE: Length = Length(1e-6);

/// Replaces, for each `(pick, existing)` join, the new point created at `pick` by `existing`.
///
/// A join whose existing point the new geometry already references (a chain start, a closing
/// end) or whose pick created no point (a radius or through pick) changes nothing. The end of a
/// centre-first arc is projected onto the start radius; its end pick still names that end.
pub(crate) fn apply(
    base: &Sketch,
    intent: &mut Sketch,
    command: &Construction,
    joins: &[(Position, EntityId)],
) -> Result<(), SketchError> {
    // Points the command itself references (a chain start, a closing end) keep that role.
    let referenced = joins
        .iter()
        .filter(|(_, existing)| referenced_by_new(base, intent, existing))
        .map(|(_, existing)| existing.clone())
        .collect::<Vec<_>>();
    for (pick, existing) in joins {
        base.point(existing)?;
        if referenced.contains(existing) {
            continue;
        }
        let Some(new) = pick_point(base, intent, command, *pick) else {
            continue;
        };
        substitute(intent, &new, existing);
    }
    for (id, entity) in &intent.entities {
        if base.entities.contains_key(id) {
            continue;
        }
        let points = entity.point_ids();
        if points
            .iter()
            .enumerate()
            .any(|(i, a)| points[i + 1..].contains(a))
        {
            return Err(SketchError::Invalid(
                "Two picks land on the same point".into(),
            ));
        }
    }
    Ok(())
}

fn referenced_by_new(base: &Sketch, intent: &Sketch, existing: &EntityId) -> bool {
    intent
        .entities
        .iter()
        .filter(|(id, _)| !base.entities.contains_key(*id))
        .any(|(_, entity)| entity.point_ids().contains(&existing))
}

/// The new point created at `pick`: the nearest within tolerance, else a centre-first arc's
/// projected end when `pick` is that arc's end pick.
fn pick_point(
    base: &Sketch,
    intent: &Sketch,
    command: &Construction,
    pick: Position,
) -> Option<EntityId> {
    let exact = intent
        .entities
        .iter()
        .filter(|(id, _)| !base.entities.contains_key(*id))
        .filter_map(|(id, entity)| match entity {
            Entity::Point { position, .. } => Some((position.distance(pick).0, id)),
            _ => None,
        })
        .filter(|(distance, _)| *distance <= PICK_TOLERANCE.0)
        .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(b.1)))
        .map(|(_, id)| id.clone());
    if exact.is_some() {
        return exact;
    }
    let Construction::ArcCenter { end, .. } = command else {
        return None;
    };
    if end.distance(pick).0 > PICK_TOLERANCE.0 {
        return None;
    }
    intent
        .entities
        .iter()
        .filter(|(id, _)| !base.entities.contains_key(*id))
        .find_map(|(_, entity)| match entity {
            Entity::Arc { end, .. } if !base.entities.contains_key(end) => Some(end.clone()),
            _ => None,
        })
}

/// Points every reference of `new` at `existing` and drops the `new` point record.
fn substitute(intent: &mut Sketch, new: &EntityId, existing: &EntityId) {
    intent.entities.remove(new);
    intent.construction.remove(new);
    let swap = |id: &mut EntityId| {
        if id == new {
            *id = existing.clone();
        }
    };
    for entity in intent.entities.values_mut() {
        entity.point_ids_mut().into_iter().for_each(swap);
    }
    for constraint in intent.constraints.values_mut() {
        constraint
            .relation
            .entity_ids_mut()
            .into_iter()
            .for_each(swap);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)] // Test helpers build known-valid sketches.
mod tests {
    use crate::{
        Construction, Entity, EntityId, Length, Position, Sketch, construct_joined,
        construct_preview_joined,
    };

    fn line() -> Sketch {
        crate::create_polyline(
            &Sketch::new("origin".into()),
            "l",
            &[Position::mm(10.0, 10.0), Position::mm(30.0, 10.0)],
            false,
        )
        .unwrap()
    }

    fn new_points(base: &Sketch, after: &Sketch) -> usize {
        after
            .entities
            .iter()
            .filter(|(id, e)| !base.entities.contains_key(*id) && matches!(e, Entity::Point { .. }))
            .count()
    }

    #[test]
    fn a_rectangle_corner_on_an_endpoint_takes_its_identity() {
        let base = line();
        let end: EntityId = "l:p1".into();
        let result = construct_joined(
            &base,
            "r",
            &Construction::Rectangle(Position::mm(30.0, 10.0), Position::mm(50.0, 30.0)),
            &[(Position::mm(30.0, 10.0), end.clone())],
        )
        .unwrap();
        assert_eq!(new_points(&base, &result.intent), 3);
        assert!(!result.intent.entities.contains_key(&EntityId::from("r:p0")));
        let users = result
            .intent
            .entities
            .values()
            .filter(|e| e.point_ids().contains(&&end))
            .count();
        assert_eq!(
            users, 3,
            "the old line and two rectangle sides share the corner"
        );
    }

    #[test]
    fn a_centre_first_arc_end_takes_the_identity_off_its_radius() {
        let base = crate::create_polyline(
            &line(),
            "m",
            &[Position::mm(10.0, 40.0), Position::mm(0.0, 40.0)],
            false,
        )
        .unwrap();
        let (a, b): (EntityId, EntityId) = ("l:p0".into(), "m:p0".into());
        let command = Construction::ArcCenter {
            center: Position::mm(12.0, 25.0),
            start: Position::mm(10.0, 10.0),
            end: Position::mm(10.0, 40.0),
        };
        let joins = [
            (Position::mm(10.0, 10.0), a.clone()),
            (Position::mm(10.0, 40.0), b.clone()),
        ];
        for result in [
            construct_preview_joined(&base, "arc", &command, &joins).unwrap(),
            construct_joined(&base, "arc", &command, &joins).unwrap(),
        ] {
            assert_eq!(
                new_points(&base, &result.intent),
                1,
                "only the centre is new"
            );
            let Some(Entity::Arc { start, end, .. }) =
                result.candidate.entities.get(&EntityId::from("arc:curve"))
            else {
                panic!("arc");
            };
            assert_eq!((start, end), (&a, &b));
            let c = result.candidate.point(&"arc:center".into()).unwrap();
            let ra = c.distance(result.candidate.point(&a).unwrap()).0;
            let rb = c.distance(result.candidate.point(&b).unwrap()).0;
            assert!((ra - rb).abs() < 1e-6, "{ra} {rb}");
        }
    }

    #[test]
    fn a_resumed_arc_end_and_a_point_free_pick() {
        let base = crate::create_polyline(
            &line(),
            "m",
            &[Position::mm(10.0, 40.0), Position::mm(0.0, 40.0)],
            false,
        )
        .unwrap();
        let end: EntityId = "m:p0".into();
        let result = construct_joined(
            &base,
            "arc",
            &Construction::ArcFrom {
                start: "l:p1".into(),
                through: Position::mm(30.0, 40.0),
                end: Position::mm(10.0, 40.0),
            },
            &[(Position::mm(10.0, 40.0), end.clone())],
        )
        .unwrap();
        assert_eq!(
            new_points(&base, &result.intent),
            1,
            "only the centre is new"
        );
        // A radius pick creates no point: the join changes nothing.
        let circle = construct_joined(
            &base,
            "c",
            &Construction::Circle {
                center: Position::mm(50.0, 50.0),
                radius: Length(5.0),
            },
            &[(Position::mm(55.0, 50.0), end)],
        )
        .unwrap();
        assert_eq!(new_points(&base, &circle.intent), 1);
    }

    #[test]
    fn two_picks_on_one_point_are_refused() {
        let base = line();
        let end: EntityId = "l:p1".into();
        let result = construct_joined(
            &base,
            "s",
            &Construction::Polyline(vec![
                Position::mm(30.0, 10.0),
                Position::mm(30.0, 10.000_000_1),
            ]),
            &[
                (Position::mm(30.0, 10.0), end.clone()),
                (Position::mm(30.0, 10.000_000_1), end),
            ],
        );
        assert!(result.is_err());
    }
}
