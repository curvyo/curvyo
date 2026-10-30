//! Tangency validation checks analytic contacts independently of solver residuals.
// Local analytic formulas use conventional point and vector letters.
#![allow(clippy::many_single_char_names)]
use crate::measure::distance_to_segment;
use crate::{Entity, EntityId, Position, Sketch, SketchError};
const TOLERANCE: f64 = 1e-6;
fn failed() -> SketchError {
    SketchError::Solver("tangency contact does not satisfy analytic curves".into())
}

pub(crate) fn validate(s: &Sketch, a: &EntityId, b: &EntityId) -> Result<(), SketchError> {
    if let Ok((p, q)) = s.line(a) {
        return line_curve(s, p, q, b, crate::contact::joint(s, a, b));
    }
    if let Ok((p, q)) = s.line(b) {
        return line_curve(s, p, q, a, crate::contact::joint(s, a, b));
    }
    let (ca, ra) = circle(s, a)?;
    let (cb, rb) = circle(s, b)?;
    let d = ca.distance(cb).0;
    if (d - ra - rb).abs() > TOLERANCE || d <= 1e-12 {
        return Err(failed());
    }
    let contact = Position::mm(
        ca.x.0 + (cb.x.0 - ca.x.0) * ra / d,
        ca.y.0 + (cb.y.0 - ca.y.0) * ra / d,
    );
    on_span(s, a, contact)?;
    on_span(s, b, contact)
}
fn circle(s: &Sketch, id: &EntityId) -> Result<(Position, f64), SketchError> {
    match s.entities.get(id) {
        Some(Entity::Circle { center, radius, .. } | Entity::Arc { center, radius, .. }) => {
            Ok((s.point(center)?, radius.0))
        }
        _ => Err(failed()),
    }
}
fn line_curve(
    s: &Sketch,
    p: Position,
    q: Position,
    curve: &EntityId,
    joint: bool,
) -> Result<(), SketchError> {
    let length = p.distance(q).0;
    if length <= 1e-12 {
        return Err(failed());
    }
    let normal = (-(q.y.0 - p.y.0) / length, (q.x.0 - p.x.0) / length);
    let (center, support) = match s.entities.get(curve) {
        Some(Entity::Circle { center, radius, .. } | Entity::Arc { center, radius, .. }) => {
            let c = s.point(center)?;
            (c, (normal.0 * radius.0, normal.1 * radius.0))
        }
        Some(Entity::Ellipse {
            center,
            focus,
            minor_radius,
            ..
        }) => {
            let c = s.point(center)?;
            let f = s.point(focus)?;
            let dx = f.x.0 - c.x.0;
            let dy = f.y.0 - c.y.0;
            let b2 = minor_radius.0 * minor_radius.0;
            let dot = dx * normal.0 + dy * normal.1;
            let support = (b2 + dot * dot).sqrt();
            (
                c,
                (
                    (b2 * normal.0 + dx * dot) / support,
                    (b2 * normal.1 + dy * dot) / support,
                ),
            )
        }
        _ => return Err(failed()),
    };
    let signed = (center.x.0 - p.x.0) * normal.0 + (center.y.0 - p.y.0) * normal.1;
    let sign = if signed < 0.0 { -1.0 } else { 1.0 };
    let contact = Position::mm(center.x.0 - sign * support.0, center.y.0 - sign * support.1);
    let error = if joint {
        distance_to_segment(contact, p, q)
    } else {
        ((contact.x.0 - p.x.0) * normal.0 + (contact.y.0 - p.y.0) * normal.1).abs()
    };
    if error > TOLERANCE {
        return Err(failed());
    }
    if joint {
        on_span(s, curve, contact)
    } else {
        Ok(())
    }
}
pub(crate) fn on_span(s: &Sketch, id: &EntityId, p: Position) -> Result<(), SketchError> {
    if let Some(Entity::Arc {
        center,
        start_angle,
        end_angle,
        ..
    }) = s.entities.get(id)
    {
        let c = s.point(center)?;
        let theta = (p.y.0 - c.y.0).atan2(p.x.0 - c.x.0);
        let offset = (theta - start_angle.0).rem_euclid(std::f64::consts::TAU);
        let sweep = crate::measure::sweep(start_angle.0, end_angle.0);
        if offset > sweep + 1e-8 && std::f64::consts::TAU - offset > 1e-8 {
            return Err(failed());
        }
    }
    Ok(())
}
