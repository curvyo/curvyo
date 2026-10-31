//! Construction operations create constrained analytic geometry atomically.
use crate::{
    Angle, Constraint, Entity, EntityId, Expression, Length, Position, Quantity, Relation, Sketch,
    SketchError, SolveMode, solve,
};

pub(crate) fn insert(s: &mut Sketch, id: EntityId, e: Entity) -> Result<(), SketchError> {
    if s.entities.contains_key(&id) {
        return Err(SketchError::Invalid(format!(
            "entity {} already exists",
            id.0
        )));
    }
    s.entities.insert(id, e);
    Ok(())
}
pub(crate) fn relation(s: &mut Sketch, id: String, r: Relation) -> Result<(), SketchError> {
    if s.constraints.contains_key(&id.clone().into()) {
        return Err(SketchError::Invalid(format!(
            "constraint {id} already exists"
        )));
    }
    s.constraints.insert(id.into(), Constraint::driving(r));
    Ok(())
}
pub(crate) fn finish(s: &Sketch) -> Result<Sketch, SketchError> {
    let result = solve(s, &SolveMode::Edit)?;
    result.candidate.ok_or_else(|| {
        SketchError::Invalid(
            result
                .message
                .unwrap_or_else(|| "construction conflicts with existing constraints".into()),
        )
    })
}
/// Constructs a straight slot from its cap centres and total width.
///
/// # Errors
/// Rejects coincident centres, invalid width, ID collisions or unsatisfied constraints.
pub(crate) fn create_slot(
    seed: &Sketch,
    prefix: &str,
    start: Position,
    end: Position,
    width: Length,
) -> Result<Sketch, SketchError> {
    let length = start.distance(end).0;
    if !length.is_finite() || length <= 1e-9 || !width.0.is_finite() || width.0 <= 0.0 {
        return Err(SketchError::Invalid(
            "slot needs distinct centres and positive width".into(),
        ));
    }
    let direction = (
        (end.x.0 - start.x.0) / length,
        (end.y.0 - start.y.0) / length,
    );
    let normal = (-direction.1, direction.0);
    let radius = width.0 / 2.0;
    let point = |center: Position, sign: f64| {
        Position::mm(
            center.x.0 + sign * radius * normal.0,
            center.y.0 + sign * radius * normal.1,
        )
    };
    let id = |name: &str| EntityId::from(format!("{prefix}:{name}"));
    let mut s = seed.clone();
    for (name, position) in [
        ("c0", start),
        ("c1", end),
        ("p0", point(start, 1.0)),
        ("p1", point(end, 1.0)),
        ("p2", point(end, -1.0)),
        ("p3", point(start, -1.0)),
    ] {
        insert(
            &mut s,
            id(name),
            Entity::Point {
                position,
                fixed: false,
            },
        )?;
    }
    for (name, a, b) in [("top", "p0", "p1"), ("bottom", "p2", "p3")] {
        insert(
            &mut s,
            id(name),
            Entity::Line {
                start: id(a),
                end: id(b),
            },
        )?;
    }
    slot_caps(&mut s, prefix, radius, direction.1.atan2(direction.0))?;
    slot_relations(&mut s, prefix, radius, length)?;
    Ok(s)
}

fn slot_caps(s: &mut Sketch, prefix: &str, radius: f64, angle: f64) -> Result<(), SketchError> {
    let id = |name: &str| EntityId::from(format!("{prefix}:{name}"));
    for (name, center, a, b, start_angle) in [
        (
            "left",
            "c0",
            "p0",
            "p3",
            angle + std::f64::consts::FRAC_PI_2,
        ),
        (
            "right",
            "c1",
            "p2",
            "p1",
            angle - std::f64::consts::FRAC_PI_2,
        ),
    ] {
        insert(
            s,
            id(name),
            Entity::Arc {
                center: id(center),
                start: id(a),
                end: id(b),
                radius: Length(radius),
                start_angle: Angle(start_angle),
                end_angle: Angle(start_angle + std::f64::consts::PI),
                fixed_shape: false,
            },
        )?;
        relation(
            s,
            format!("{prefix}:sweep:{name}"),
            Relation::Sweep {
                arc: id(name),
                value: Expression::Literal(Quantity::angle(std::f64::consts::PI)?),
            },
        )?;
    }
    Ok(())
}

fn slot_relations(
    s: &mut Sketch,
    prefix: &str,
    radius: f64,
    length: f64,
) -> Result<(), SketchError> {
    let id = |name: &str| EntityId::from(format!("{prefix}:{name}"));
    for (name, line, arc) in [
        ("tl", "top", "left"),
        ("tr", "top", "right"),
        ("bl", "bottom", "left"),
        ("br", "bottom", "right"),
    ] {
        relation(
            s,
            format!("{prefix}:{name}"),
            Relation::Tangent(id(line), id(arc)),
        )?;
    }
    relation(
        s,
        format!("{prefix}:radius"),
        Relation::Radius {
            curve: id("left"),
            value: Expression::Literal(Quantity::length(radius)?),
        },
    )?;
    relation(
        s,
        format!("{prefix}:equal"),
        Relation::Equal(id("left"), id("right")),
    )?;
    relation(
        s,
        format!("{prefix}:spacing"),
        Relation::Distance {
            a: id("c0"),
            b: id("c1"),
            value: Expression::Literal(Quantity::length(length)?),
        },
    )?;
    Ok(())
}
