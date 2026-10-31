//! Connected drawing modes construct analytic segments from an existing endpoint tangent.
use crate::{Angle, Entity, EntityId, Length, Position, Relation, Sketch, SketchError};

/// Explicit requested behavior for the next connected segment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ContinuationMode {
    /// An unconstrained line to the next point.
    #[default]
    FreeLine,
    /// A line perpendicular to the preceding endpoint tangent.
    PerpendicularLine,
    /// A line tangent to the preceding entity.
    TangentLine,
    /// An arc tangent to the preceding entity.
    TangentArc,
    /// An arc initially turning left from the perpendicular endpoint tangent.
    PerpendicularArcLeft,
    /// The opposite perpendicular arc branch.
    PerpendicularArcRight,
}

/// Outgoing unit tangent at a current endpoint, following the preceding entity into it.
/// # Errors
/// Rejects non-endpoints, unsupported entities and degenerate directions.
pub fn endpoint_tangent(
    s: &Sketch,
    previous: &EntityId,
    endpoint: &EntityId,
) -> Result<(f64, f64), SketchError> {
    let p = s.point(endpoint)?;
    let vector = match s.entities.get(previous) {
        Some(Entity::Line { start, end }) if start == endpoint || end == endpoint => {
            let other = s.point(if start == endpoint { end } else { start })?;
            (p.x.0 - other.x.0, p.y.0 - other.y.0)
        }
        Some(Entity::Arc {
            center, start, end, ..
        }) if start == endpoint || end == endpoint => {
            let c = s.point(center)?;
            let sign = if end == endpoint { 1.0 } else { -1.0 };
            (-(p.y.0 - c.y.0) * sign, (p.x.0 - c.x.0) * sign)
        }
        _ => {
            return Err(SketchError::Invalid(
                "Choose an unambiguous line or arc endpoint".into(),
            ));
        }
    };
    let length = vector.0.hypot(vector.1);
    if length <= 1e-9 {
        return Err(SketchError::Invalid(
            "Previous endpoint has no direction".into(),
        ));
    }
    Ok((vector.0 / length, vector.1 / length))
}

pub(crate) fn build(
    base: &Sketch,
    prefix: &str,
    previous: &EntityId,
    start: &EntityId,
    end: &EntityId,
    position: Position,
    mode: ContinuationMode,
) -> Result<Sketch, SketchError> {
    let a = base.point(start)?;
    let incoming = endpoint_tangent(base, previous, start)?;
    let perpendicular = matches!(
        mode,
        ContinuationMode::PerpendicularLine
            | ContinuationMode::PerpendicularArcLeft
            | ContinuationMode::PerpendicularArcRight
    );
    let direction = match mode {
        ContinuationMode::PerpendicularArcRight => (incoming.1, -incoming.0),
        _ if perpendicular => (-incoming.1, incoming.0),
        _ => incoming,
    };
    let arc = matches!(
        mode,
        ContinuationMode::TangentArc
            | ContinuationMode::PerpendicularArcLeft
            | ContinuationMode::PerpendicularArcRight
    );
    let existing = base.entities.contains_key(end);
    let b = if existing {
        base.point(end)?
    } else if !arc && mode != ContinuationMode::FreeLine {
        let distance = (position.x.0 - a.x.0) * direction.0 + (position.y.0 - a.y.0) * direction.1;
        Position::mm(
            a.x.0 + distance * direction.0,
            a.y.0 + distance * direction.1,
        )
    } else {
        position
    };
    if a.distance(b).0 <= 1e-9 {
        return Err(SketchError::Invalid("Choose a distinct endpoint".into()));
    }
    let mut s = base.clone();
    if !existing {
        crate::construction::insert(
            &mut s,
            end.clone(),
            Entity::Point {
                position: b,
                fixed: false,
            },
        )?;
    }
    let curve: EntityId = format!("{prefix}:{}", if arc { "arc" } else { "line" }).into();
    if arc {
        append_arc(&mut s, prefix, &curve, start, end, a, b, direction)?;
    } else {
        crate::construction::insert(
            &mut s,
            curve.clone(),
            Entity::Line {
                start: start.clone(),
                end: end.clone(),
            },
        )?;
    }
    if mode != ContinuationMode::FreeLine {
        let relation = if perpendicular
            && (arc || matches!(base.entities.get(previous), Some(Entity::Arc { .. })))
        {
            Relation::PerpendicularAt {
                a: previous.clone(),
                b: curve,
                point: start.clone(),
            }
        } else if perpendicular {
            Relation::Perpendicular(previous.clone(), curve)
        } else if !arc && matches!(base.entities.get(previous), Some(Entity::Line { .. })) {
            Relation::Parallel(previous.clone(), curve)
        } else {
            Relation::Tangent(previous.clone(), curve)
        };
        crate::construction::relation(&mut s, format!("{prefix}:direction"), relation)?;
    }
    Ok(s)
}

#[allow(clippy::too_many_arguments)] // Concrete construction data, not a reusable abstraction.
fn append_arc(
    graph: &mut Sketch,
    prefix: &str,
    curve: &EntityId,
    start: &EntityId,
    end: &EntityId,
    start_position: Position,
    end_position: Position,
    tangent: (f64, f64),
) -> Result<(), SketchError> {
    let dx = end_position.x.0 - start_position.x.0;
    let dy = end_position.y.0 - start_position.y.0;
    let denominator = 2.0 * (-dx * tangent.1 + dy * tangent.0);
    if denominator.abs() <= 1e-9 {
        return Err(SketchError::Invalid(
            "Arc endpoint lies on its start tangent".into(),
        ));
    }
    let signed = (dx * dx + dy * dy) / denominator;
    let center_position = Position::mm(
        start_position.x.0 - tangent.1 * signed,
        start_position.y.0 + tangent.0 * signed,
    );
    let center: EntityId = format!("{prefix}:center").into();
    crate::construction::insert(
        graph,
        center.clone(),
        Entity::Point {
            position: center_position,
            fixed: false,
        },
    )?;
    let (start, end, start_position, end_position) = if signed > 0.0 {
        (start, end, start_position, end_position)
    } else {
        (end, start, end_position, start_position)
    };
    let start_angle =
        (start_position.y.0 - center_position.y.0).atan2(start_position.x.0 - center_position.x.0);
    let sweep = ((end_position.y.0 - center_position.y.0)
        .atan2(end_position.x.0 - center_position.x.0)
        - start_angle)
        .rem_euclid(std::f64::consts::TAU);
    crate::construction::insert(
        graph,
        curve.clone(),
        Entity::Arc {
            center,
            start: start.clone(),
            end: end.clone(),
            radius: Length(signed.abs()),
            start_angle: Angle(start_angle),
            end_angle: Angle(start_angle + sweep),
            fixed_shape: false,
        },
    )
}

/// Finds a circular endpoint for a typed radius while retaining the current tangent branch.
/// The pointer chooses the radial direction; the start point remains fixed.
/// # Errors
/// Rejects nonpositive radii, degenerate directions or a pointer at the chosen circle centre.
pub fn arc_radius_endpoint(
    start: Position,
    tangent: (f64, f64),
    radius: Length,
    towards: Position,
) -> Result<Position, SketchError> {
    let norm = tangent.0.hypot(tangent.1);
    if !radius.0.is_finite() || radius.0 <= 1e-9 || !norm.is_finite() || norm <= 1e-9 {
        return Err(SketchError::Invalid(
            "Radius and tangent must be nonzero and finite".into(),
        ));
    }
    let tangent = (tangent.0 / norm, tangent.1 / norm);
    let side = (towards.y.0 - start.y.0) * tangent.0 - (towards.x.0 - start.x.0) * tangent.1;
    if side.abs() <= 1e-9 {
        return Err(SketchError::Invalid(
            "Choose an arc endpoint away from its tangent".into(),
        ));
    }
    let signed = radius.0.copysign(side);
    let center = Position::mm(
        start.x.0 - tangent.1 * signed,
        start.y.0 + tangent.0 * signed,
    );
    let dx = towards.x.0 - center.x.0;
    let dy = towards.y.0 - center.y.0;
    let distance = dx.hypot(dy);
    if !distance.is_finite() || distance <= 1e-9 {
        return Err(SketchError::Invalid(
            "Choose an arc endpoint away from its centre".into(),
        ));
    }
    let end = Position::mm(
        center.x.0 + radius.0 * dx / distance,
        center.y.0 + radius.0 * dy / distance,
    );
    if end.distance(start).0 <= 1e-9 {
        return Err(SketchError::Invalid(
            "Choose a distinct arc endpoint".into(),
        ));
    }
    Ok(end)
}
