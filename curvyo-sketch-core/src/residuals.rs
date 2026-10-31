//! Independent residual checks prevent invalid candidates from becoming geometry.
// Local analytic formulas use conventional point and vector letters.
#![allow(clippy::many_single_char_names)]
use crate::measure::{length, radius};
use crate::{Dimension, Entity, EntityId, Position, Relation, Sketch, SketchError};
const LENGTH_TOLERANCE: f64 = 1e-6;
const ANGLE_TOLERANCE: f64 = 1e-8;

pub(crate) fn validate(
    seed: &Sketch,
    solved: &Sketch,
    contacts: &std::collections::BTreeMap<crate::ConstraintId, Position>,
) -> Result<(), SketchError> {
    let values = seed.parameters.evaluate()?;
    intrinsic_geometry(seed, solved)?;
    for (id, c) in &seed.constraints {
        if c.reference {
            continue;
        }
        let result = match &c.relation {
            Relation::Distance { value, .. }
            | Relation::Length { value, .. }
            | Relation::Radius { value, .. }
            | Relation::Diameter { value, .. }
            | Relation::Angle { value, .. }
            | Relation::Sweep { value, .. } => {
                let expected = value.evaluate(&values)?;
                let actual = crate::measure::measure(solved, &c.relation)?;
                if actual.dimension() == Dimension::Angle {
                    check(wrapped(actual.value() - expected.value()), ANGLE_TOLERANCE)
                } else {
                    check(actual.value() - expected.value(), LENGTH_TOLERANCE)
                }
            }
            Relation::Tangent(a, b) if contacts.contains_key(id) => {
                let point = contacts[id];
                let (ea, na) = crate::contact::normal(solved, a, point)?;
                let (eb, nb) = crate::contact::normal(solved, b, point)?;
                check(ea, LENGTH_TOLERANCE)?;
                check(eb, LENGTH_TOLERANCE)?;

                check(na.0 * nb.1 - na.1 * nb.0, ANGLE_TOLERANCE)
            }
            other => geometric(seed, solved, other),
        };
        result.map_err(|e| SketchError::Solver(format!("constraint {}: {e}", id.0)))?;
    }
    Ok(())
}
fn intrinsic_geometry(seed: &Sketch, solved: &Sketch) -> Result<(), SketchError> {
    for (id, entity) in &seed.entities {
        match entity {
            Entity::Point {
                position,
                fixed: true,
            } => check(position.distance(solved.point(id)?).0, LENGTH_TOLERANCE)?,
            Entity::Circle {
                radius: r,
                fixed_radius: true,
                ..
            } => check(radius(solved, id)? - r.0, LENGTH_TOLERANCE)?,
            Entity::Ellipse {
                minor_radius,
                fixed_radius: true,
                ..
            } => {
                if let Some(Entity::Ellipse {
                    minor_radius: actual,
                    ..
                }) = solved.entities.get(id)
                {
                    check(actual.0 - minor_radius.0, LENGTH_TOLERANCE)?;
                } else {
                    return Err(SketchError::Solver("changed ellipse type".into()));
                }
            }
            Entity::Arc {
                radius: r,
                start_angle,
                end_angle,
                fixed_shape: true,
                ..
            } => {
                if let Some(Entity::Arc {
                    radius: actual,
                    start_angle: a,
                    end_angle: b,
                    ..
                }) = solved.entities.get(id)
                {
                    check(actual.0 - r.0, LENGTH_TOLERANCE)?;
                    check(wrapped(a.0 - start_angle.0), ANGLE_TOLERANCE)?;
                    check(wrapped(b.0 - end_angle.0), ANGLE_TOLERANCE)?;
                } else {
                    return Err(SketchError::Solver("changed arc type".into()));
                }
            }
            _ => {}
        }
    }
    for entity in solved.entities.values() {
        if let Entity::Arc {
            center,
            start,
            end,
            radius,
            start_angle,
            end_angle,
            ..
        } = entity
        {
            let center = solved.point(center)?;
            for (point, angle) in [(start, start_angle), (end, end_angle)] {
                let expected = Position::mm(
                    center.x.0 + radius.0 * angle.0.cos(),
                    center.y.0 + radius.0 * angle.0.sin(),
                );
                check(expected.distance(solved.point(point)?).0, LENGTH_TOLERANCE)?;
            }
        }
    }
    Ok(())
}
fn check(residual: f64, tolerance: f64) -> Result<(), SketchError> {
    if residual.is_finite() && residual.abs() <= tolerance {
        Ok(())
    } else {
        Err(SketchError::Solver(format!(
            "residual {residual} exceeds {tolerance}"
        )))
    }
}
fn wrapped(angle: f64) -> f64 {
    (angle + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}
fn direction(s: &Sketch, id: &EntityId) -> Result<(f64, f64), SketchError> {
    let (a, b) = s.line(id)?;
    let d = a.distance(b).0;
    if d <= 1e-12 {
        return Err(SketchError::Solver(
            "undefined zero-length direction".into(),
        ));
    }
    Ok(((b.x.0 - a.x.0) / d, (b.y.0 - a.y.0) / d))
}
fn geometric(seed: &Sketch, s: &Sketch, r: &Relation) -> Result<(), SketchError> {
    match r {
        Relation::PerpendicularAt { a, b, point } => {
            let first = crate::endpoint_tangent(s, a, point)?;
            let second = crate::endpoint_tangent(s, b, point)?;
            check(first.0 * second.0 + first.1 * second.1, ANGLE_TOLERANCE)
        }
        Relation::OnAxis { point, axis } => {
            let point = s.point(point)?;
            check(
                match axis {
                    crate::Axis::X => point.y.0,
                    crate::Axis::Y => point.x.0,
                },
                LENGTH_TOLERANCE,
            )
        }
        Relation::On { point, curve } => {
            let point = s.point(point)?;
            if let Ok((a, b)) = s.line(curve) {
                check(
                    crate::measure::distance_to_segment(point, a, b),
                    LENGTH_TOLERANCE,
                )
            } else {
                check(crate::contact::normal(s, curve, point)?.0, LENGTH_TOLERANCE)?;
                crate::tangency::on_span(s, curve, point)
            }
        }
        Relation::Coincident(a, b) => check(s.point(a)?.distance(s.point(b)?).0, LENGTH_TOLERANCE),
        Relation::Horizontal(a) => {
            let (a, b) = s.line(a)?;
            check(a.y.0 - b.y.0, LENGTH_TOLERANCE)
        }
        Relation::Vertical(a) => {
            let (a, b) = s.line(a)?;
            check(a.x.0 - b.x.0, LENGTH_TOLERANCE)
        }
        Relation::Parallel(a, b) | Relation::Perpendicular(a, b) => {
            let a = direction(s, a)?;
            let b = direction(s, b)?;
            check(
                if matches!(r, Relation::Parallel(..)) {
                    a.0 * b.1 - a.1 * b.0
                } else {
                    a.0 * b.0 + a.1 * b.1
                },
                ANGLE_TOLERANCE,
            )
        }
        Relation::Equal(a, b) => check(
            if s.line(a).is_ok() {
                length(s, a)? - length(s, b)?
            } else {
                radius(s, a)? - radius(s, b)?
            },
            LENGTH_TOLERANCE,
        ),
        Relation::Midpoint { point, line } => {
            let (a, b) = s.line(line)?;
            check(
                s.point(point)?
                    .distance(Position::mm(a.x.0.midpoint(b.x.0), a.y.0.midpoint(b.y.0)))
                    .0,
                LENGTH_TOLERANCE,
            )
        }
        Relation::Symmetric {
            source,
            image,
            axis,
        } => {
            let (a, _) = s.line(axis)?;
            let d = direction(s, axis)?;
            let p = s.point(source)?;
            let projection = (p.x.0 - a.x.0) * d.0 + (p.y.0 - a.y.0) * d.1;
            let mirror = Position::mm(
                2.0 * (a.x.0 + projection * d.0) - p.x.0,
                2.0 * (a.y.0 + projection * d.1) - p.y.0,
            );
            check(mirror.distance(s.point(image)?).0, LENGTH_TOLERANCE)
        }
        Relation::Fixed(id) => fixed(seed, s, id),
        Relation::Tangent(a, b) => crate::tangency::validate(s, a, b),
        _ => Err(SketchError::Solver("unhandled residual".into())),
    }
}
fn fixed(seed: &Sketch, s: &Sketch, id: &EntityId) -> Result<(), SketchError> {
    let entity = seed
        .entities
        .get(id)
        .ok_or_else(|| SketchError::Invalid("missing fixed entity".into()))?;
    let points = match entity {
        Entity::Point { .. } => vec![id],
        Entity::Line { start, end } => vec![start, end],
        Entity::Circle { center, .. } => vec![center],
        Entity::Arc {
            center, start, end, ..
        } => vec![center, start, end],
        Entity::Ellipse { center, focus, .. } => vec![center, focus],
    };
    for id in points {
        check(seed.point(id)?.distance(s.point(id)?).0, LENGTH_TOLERANCE)?;
    }
    match entity {
        Entity::Circle { .. } | Entity::Arc { .. } => {
            check(radius(seed, id)? - radius(s, id)?, LENGTH_TOLERANCE)
        }
        Entity::Ellipse { minor_radius, .. } => match s.entities.get(id) {
            Some(Entity::Ellipse {
                minor_radius: r, ..
            }) => check(r.0 - minor_radius.0, LENGTH_TOLERANCE),
            _ => Err(SketchError::Solver("changed ellipse type".into())),
        },
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Angle, Length};
    fn arc() -> Sketch {
        let mut s = Sketch::new("c".into());
        for (id, position) in [
            ("start", Position::mm(2.0, 0.0)),
            ("end", Position::mm(0.0, 2.0)),
        ] {
            s.entities.insert(
                id.into(),
                Entity::Point {
                    position,
                    fixed: false,
                },
            );
        }
        s.entities.insert(
            "arc".into(),
            Entity::Arc {
                center: "c".into(),
                start: "start".into(),
                end: "end".into(),
                radius: Length(2.0),
                start_angle: Angle(0.0),
                end_angle: Angle(std::f64::consts::FRAC_PI_2),
                fixed_shape: true,
            },
        );
        s
    }
    #[test]
    fn rejects_inconsistent_arc_endpoint() {
        let seed = arc();
        let mut bad = seed.clone();
        bad.entities.insert(
            "start".into(),
            Entity::Point {
                position: Position::mm(3.0, 0.0),
                fixed: false,
            },
        );
        assert!(validate(&seed, &bad, &std::collections::BTreeMap::new()).is_err());
    }
    #[test]
    fn rejects_changed_fixed_arc_shape() {
        let seed = arc();
        let mut bad = seed.clone();
        if let Some(Entity::Arc { radius, .. }) = bad.entities.get_mut(&"arc".into()) {
            *radius = Length(3.0);
        }
        assert!(validate(&seed, &bad, &std::collections::BTreeMap::new()).is_err());
    }
    #[test]
    fn rejects_changed_fixed_ellipse_radius() {
        let mut seed = arc();
        seed.entities.insert(
            "ellipse".into(),
            Entity::Ellipse {
                center: "c".into(),
                focus: "start".into(),
                minor_radius: Length(2.0),
                fixed_radius: true,
            },
        );
        let mut bad = seed.clone();
        if let Some(Entity::Ellipse { minor_radius, .. }) = bad.entities.get_mut(&"ellipse".into())
        {
            *minor_radius = Length(3.0);
        }
        assert!(validate(&seed, &bad, &std::collections::BTreeMap::new()).is_err());
    }
}
