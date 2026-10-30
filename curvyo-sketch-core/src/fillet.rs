//! Fillets retain original corner references as construction geometry.
use crate::construction::{insert, relation};
use crate::{
    Angle, Entity, EntityId, Expression, Length, Position, Quantity, Relation, Sketch, SketchError,
};

/// Rounds two joined segments while retaining their original dimensional intent.
///
/// Original segments become construction references; all their IDs and constraints
/// remain intact. New trimmed segments share tangent endpoints with the new arc.
///
/// # Errors
/// Rejects non-shared or straight corners, oversized radii, ID collisions and
/// constraints that cannot be preserved. The input remains unchanged on failure.
pub(crate) fn fillet(
    seed: &Sketch,
    prefix: &str,
    first: &EntityId,
    second: &EntityId,
    radius: Length,
) -> Result<Sketch, SketchError> {
    let (corner, a, b) = corner(seed, first, second)?;
    let center = seed.point(&corner)?;
    let pa = seed.point(&a)?;
    let pb = seed.point(&b)?;
    let la = center.distance(pa).0;
    let lb = center.distance(pb).0;
    if la <= 1e-9 || lb <= 1e-9 || !radius.0.is_finite() || radius.0 <= 0.0 {
        return Err(invalid());
    }
    let da = ((pa.x.0 - center.x.0) / la, (pa.y.0 - center.y.0) / la);
    let db = ((pb.x.0 - center.x.0) / lb, (pb.y.0 - center.y.0) / lb);
    let half = ((da.0 * db.0 + da.1 * db.1).clamp(-1.0, 1.0).acos()) / 2.0;
    if half.sin().abs() <= 1e-9 || half.cos().abs() <= 1e-9 {
        return Err(invalid());
    }
    let trim = radius.0 / half.tan();
    if trim >= la - 1e-9 || trim >= lb - 1e-9 {
        return Err(invalid());
    }
    let start = Position::mm(center.x.0 + trim * da.0, center.y.0 + trim * da.1);
    let end = Position::mm(center.x.0 + trim * db.0, center.y.0 + trim * db.1);
    let norm = (da.0 + db.0).hypot(da.1 + db.1);
    let offset = radius.0 / half.sin();
    let arc_center = Position::mm(
        center.x.0 + offset * (da.0 + db.0) / norm,
        center.y.0 + offset * (da.1 + db.1) / norm,
    );
    let id = |name: &str| EntityId::from(format!("{prefix}:{name}"));
    let mut s = seed.clone();
    for (name, position) in [("start", start), ("end", end), ("center", arc_center)] {
        insert(
            &mut s,
            id(name),
            Entity::Point {
                position,
                fixed: false,
            },
        )?;
    }
    insert(
        &mut s,
        id("first"),
        Entity::Line {
            start: a,
            end: id("start"),
        },
    )?;
    insert(
        &mut s,
        id("second"),
        Entity::Line {
            start: id("end"),
            end: b,
        },
    )?;
    let first_angle = (start.y.0 - arc_center.y.0).atan2(start.x.0 - arc_center.x.0);
    let second_angle = (end.y.0 - arc_center.y.0).atan2(end.x.0 - arc_center.x.0);
    let reverse =
        (second_angle - first_angle).rem_euclid(std::f64::consts::TAU) > std::f64::consts::PI;
    insert(
        &mut s,
        id("arc"),
        Entity::Arc {
            center: id("center"),
            start: id(if reverse { "end" } else { "start" }),
            end: id(if reverse { "start" } else { "end" }),
            radius,
            start_angle: Angle(if reverse { second_angle } else { first_angle }),
            end_angle: Angle(if reverse { first_angle } else { second_angle }),
            fixed_shape: false,
        },
    )?;
    for (name, point, curve) in [
        ("on_first", id("start"), first.clone()),
        ("on_second", id("end"), second.clone()),
    ] {
        relation(
            &mut s,
            format!("{prefix}:{name}"),
            Relation::On { point, curve },
        )?;
    }
    for name in ["first", "second"] {
        relation(
            &mut s,
            format!("{prefix}:tangent:{name}"),
            Relation::Tangent(id(name), id("arc")),
        )?;
    }
    relation(
        &mut s,
        format!("{prefix}:radius"),
        Relation::Radius {
            curve: id("arc"),
            value: Expression::Literal(Quantity::length(radius.0)?),
        },
    )?;
    s.construction.insert(first.clone());
    s.construction.insert(second.clone());
    Ok(s)
}
fn invalid() -> SketchError {
    SketchError::Invalid("fillet cannot preserve this corner at the requested radius".into())
}
fn corner(
    s: &Sketch,
    first: &EntityId,
    second: &EntityId,
) -> Result<(EntityId, EntityId, EntityId), SketchError> {
    let Some(Entity::Line {
        start: first_start,
        end: first_end,
    }) = s.entities.get(first)
    else {
        return Err(invalid());
    };
    let Some(Entity::Line {
        start: second_start,
        end: second_end,
    }) = s.entities.get(second)
    else {
        return Err(invalid());
    };
    for (shared, outer) in [(first_start, first_end), (first_end, first_start)] {
        if shared == second_start {
            return Ok((shared.clone(), outer.clone(), second_end.clone()));
        }
        if shared == second_end {
            return Ok((shared.clone(), outer.clone(), second_start.clone()));
        }
    }
    Err(invalid())
}
