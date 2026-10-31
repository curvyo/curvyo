//! Ellipse pair tangency expands through a single shared analytic contact.
use crate::solver_input::Input;
use crate::{ConstraintId, Entity, EntityId, Position, Sketch, SketchError};
use serde_json::{Value, json};

pub(crate) fn joint(s: &Sketch, a: &EntityId, b: &EntityId) -> bool {
    let ends = |id: &EntityId| match s.entities.get(id) {
        Some(Entity::Line { start, end } | Entity::Arc { start, end, .. }) => Some((start, end)),
        _ => None,
    };
    match (ends(a), ends(b)) {
        (Some((a0, a1)), Some((b0, b1))) => a0 == b0 || a0 == b1 || a1 == b0 || a1 == b1,
        _ => false,
    }
}
pub(crate) fn required(s: &Sketch, a: &EntityId, b: &EntityId) -> bool {
    !joint(s, a, b)
        && [a, b].iter().any(|id| {
            matches!(
                s.entities.get(*id),
                Some(Entity::Ellipse { .. } | Entity::Arc { .. })
            )
        })
}
pub(crate) fn append(
    s: &Sketch,
    input: &mut Input,
    ps: &mut Vec<Value>,
    id: &ConstraintId,
    a: &EntityId,
    b: &EntityId,
) -> Result<(), SketchError> {
    let (curve, other) = if matches!(s.entities.get(a), Some(Entity::Ellipse { .. })) {
        (a, b)
    } else if matches!(s.entities.get(b), Some(Entity::Ellipse { .. }))
        || matches!(s.entities.get(a), Some(Entity::Line { .. }))
    {
        (b, a)
    } else {
        (a, b)
    };
    let target = target(s, curve, other)?;
    let contact = s
        .contact_seeds
        .get(id)
        .copied()
        .map_or_else(|| seed_contact(s, curve, target), Ok)?;
    let normal = normal(s, curve, contact)?.1;
    let q = format!("contact{}", input.contacts.len());
    let tip = format!("tip{}", input.contacts.len());
    let line = format!("tangent{}", input.contacts.len());
    ps.push(json!({"id":q,"type":"point","x":contact.x.0,"y":contact.y.0}));
    ps.push(json!({"id":tip,"type":"point","x":contact.x.0-normal.1,"y":contact.y.0+normal.0}));
    ps.push(json!({"id":line,"type":"line","p1_id":q,"p2_id":tip}));
    for curve in [a, b] {
        let backend = carrier(s, input, ps, id, curve)?;
        if matches!(s.entities.get(curve), Some(Entity::Line { .. })) {
            input.add_constraint(
                ps,
                id,
                json!({"type":"on","point":q,"curve":backend,"extension":true}),
            );
            input.add_constraint(ps, id, json!({"type":"parallel","a":line,"b":backend}));
        } else {
            input.add_constraint(ps, id, json!({"type":"on","point":q,"curve":backend}));
            input.add_constraint(ps, id, json!({"type":"tangent","a":line,"b":backend}));
        }
    }
    input.add_constraint(ps, id, json!({"type":"length","curve":line,"value":1.0}));
    input.contacts.insert(id.clone(), q);
    Ok(())
}
fn carrier(
    s: &Sketch,
    input: &mut Input,
    ps: &mut Vec<Value>,
    id: &ConstraintId,
    curve: &EntityId,
) -> Result<String, SketchError> {
    if let Some(Entity::Arc { center, radius, .. }) = s.entities.get(curve) {
        let alias = format!("carrier{}", ps.len());
        ps.push(json!({"id":alias,"type":"circle","c_id":input.id(center),"radius":radius.0}));
        input.add_constraint(
            ps,
            id,
            json!({"type":"equal","a":alias,"b":input.id(curve)}),
        );
        Ok(alias)
    } else if matches!(
        s.entities.get(curve),
        Some(Entity::Circle { .. } | Entity::Ellipse { .. } | Entity::Line { .. })
    ) {
        Ok(input.id(curve).into())
    } else {
        Err(SketchError::Invalid("unsupported tangency pair".into()))
    }
}
fn target(s: &Sketch, curve: &EntityId, other: &EntityId) -> Result<Position, SketchError> {
    match s.entities.get(other) {
        Some(
            Entity::Circle { center, .. }
            | Entity::Arc { center, .. }
            | Entity::Ellipse { center, .. },
        ) => s.point(center),
        Some(Entity::Line { .. }) => {
            let center = match s.entities.get(curve) {
                Some(
                    Entity::Circle { center, .. }
                    | Entity::Arc { center, .. }
                    | Entity::Ellipse { center, .. },
                ) => s.point(center)?,
                _ => return Err(SketchError::Invalid("unsupported tangent".into())),
            };
            let (a, b) = s.line(other)?;
            let dx = b.x.0 - a.x.0;
            let dy = b.y.0 - a.y.0;
            let norm = dx * dx + dy * dy;
            if norm <= 1e-24 {
                return Err(SketchError::Invalid("zero-length tangent line".into()));
            }
            let t = ((center.x.0 - a.x.0) * dx + (center.y.0 - a.y.0) * dy) / norm;
            Ok(Position::mm(a.x.0 + t * dx, a.y.0 + t * dy))
        }
        _ => Err(SketchError::Invalid("unsupported tangent".into())),
    }
}
fn seed_contact(s: &Sketch, id: &EntityId, target: Position) -> Result<Position, SketchError> {
    if let Some(Entity::Circle { center, radius, .. } | Entity::Arc { center, radius, .. }) =
        s.entities.get(id)
    {
        let center = s.point(center)?;
        let dx = target.x.0 - center.x.0;
        let dy = target.y.0 - center.y.0;
        let length = dx.hypot(dy);
        if length <= 1e-12 {
            return Err(SketchError::Invalid(
                "ambiguous concentric tangency seed".into(),
            ));
        }
        return Ok(Position::mm(
            center.x.0 + radius.0 * dx / length,
            center.y.0 + radius.0 * dy / length,
        ));
    }
    let (center, axis, major, minor) = frame(s, id)?;
    let local = (
        (target.x.0 - center.x.0) * axis.0 + (target.y.0 - center.y.0) * axis.1,
        -(target.x.0 - center.x.0) * axis.1 + (target.y.0 - center.y.0) * axis.0,
    );
    let mut theta = (local.1 / minor).atan2(local.0 / major);
    // A bounded Newton seed improves convergence; it never declares satisfaction.
    for _ in 0..12 {
        let (sin, cos) = theta.sin_cos();
        let dx = major * cos - local.0;
        let dy = minor * sin - local.1;
        let gradient = -dx * major * sin + dy * minor * cos;
        let curvature = major * major * sin * sin + minor * minor * cos * cos
            - dx * major * cos
            - dy * minor * sin;
        if curvature.abs() <= 1e-12 {
            break;
        }
        theta -= (gradient / curvature).clamp(-0.5, 0.5);
    }
    Ok(Position::mm(
        center.x.0 + axis.0 * major * theta.cos() - axis.1 * minor * theta.sin(),
        center.y.0 + axis.1 * major * theta.cos() + axis.0 * minor * theta.sin(),
    ))
}
fn frame(s: &Sketch, id: &EntityId) -> Result<(Position, (f64, f64), f64, f64), SketchError> {
    match s.entities.get(id) {
        Some(Entity::Ellipse {
            center,
            focus,
            minor_radius,
            ..
        }) => {
            let center = s.point(center)?;
            let focus = s.point(focus)?;
            let distance = center.distance(focus).0;
            let axis = if distance <= 1e-12 {
                (1.0, 0.0)
            } else {
                (
                    (focus.x.0 - center.x.0) / distance,
                    (focus.y.0 - center.y.0) / distance,
                )
            };
            Ok((center, axis, minor_radius.0.hypot(distance), minor_radius.0))
        }
        _ => Err(SketchError::Invalid("expected ellipse".into())),
    }
}
/// Returns signed radial error in millimetres plus a unit analytic normal.
pub(crate) fn normal(
    s: &Sketch,
    id: &EntityId,
    point: Position,
) -> Result<(f64, (f64, f64)), SketchError> {
    let (error, normal) = match s.entities.get(id) {
        Some(Entity::Line { .. }) => {
            let (a, b) = s.line(id)?;
            let dx = b.x.0 - a.x.0;
            let dy = b.y.0 - a.y.0;
            let norm = dx.hypot(dy);
            if norm <= 1e-12 {
                return Err(SketchError::Solver("undefined line normal".into()));
            }
            (
                ((point.x.0 - a.x.0) * (-dy) + (point.y.0 - a.y.0) * dx) / norm,
                (-dy, dx),
            )
        }
        Some(Entity::Circle { center, radius, .. } | Entity::Arc { center, radius, .. }) => {
            let c = s.point(center)?;
            let dx = point.x.0 - c.x.0;
            let dy = point.y.0 - c.y.0;
            (dx.hypot(dy) - radius.0, (dx, dy))
        }
        Some(Entity::Ellipse { .. }) => {
            let (center, axis, major, minor) = frame(s, id)?;
            let dx = point.x.0 - center.x.0;
            let dy = point.y.0 - center.y.0;
            let x = dx * axis.0 + dy * axis.1;
            let y = -dx * axis.1 + dy * axis.0;
            let implicit = x * x / (major * major) + y * y / (minor * minor) - 1.0;
            let nx = x / (major * major);
            let ny = y / (minor * minor);
            (
                implicit / (2.0 * nx.hypot(ny)),
                (nx * axis.0 - ny * axis.1, nx * axis.1 + ny * axis.0),
            )
        }
        _ => {
            return Err(SketchError::Invalid(
                "expected analytic closed curve".into(),
            ));
        }
    };
    let length = normal.0.hypot(normal.1);
    if !error.is_finite() || length <= 1e-12 {
        return Err(SketchError::Solver("undefined contact normal".into()));
    }
    Ok((error, (normal.0 / length, normal.1 / length)))
}
