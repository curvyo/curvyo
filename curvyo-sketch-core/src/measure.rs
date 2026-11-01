//! Analytic measurements provide independent dimension oracles.
// Local analytic formulas use conventional point and vector letters.
#![allow(clippy::many_single_char_names)]
use crate::{
    Dimension, EllipseAxis, Entity, EntityId, Position, Quantity, Relation, Sketch, SketchError,
};

/// Measures a reference or driving dimension from solved analytic geometry.
///
/// # Errors
/// Rejects invalid references or a relation that is not a dimension.
pub fn measure(s: &Sketch, r: &Relation) -> Result<Quantity, SketchError> {
    let (value, dimension) = match r {
        Relation::Sweep { arc, .. } => match s.entities.get(arc) {
            Some(Entity::Arc {
                start_angle,
                end_angle,
                ..
            }) => (sweep(start_angle.0, end_angle.0), Dimension::Angle),
            _ => return Err(SketchError::Invalid("sweep needs arc".into())),
        },
        Relation::Distance { a, b, .. } => {
            let p = s.point(a)?;
            let d = if let Ok(q) = s.point(b) {
                p.distance(q).0
            } else {
                let (a, b) = s.line(b)?;
                distance_to_segment(p, a, b)
            };
            (d, Dimension::Length)
        }
        Relation::HorizontalDistance { a, b, .. } => {
            (s.point(b)?.x.0 - s.point(a)?.x.0, Dimension::Length)
        }
        Relation::VerticalDistance { a, b, .. } => {
            (s.point(b)?.y.0 - s.point(a)?.y.0, Dimension::Length)
        }
        Relation::Length { curve, .. } => (length(s, curve)?, Dimension::Length),
        Relation::Radius { curve, .. } => (radius(s, curve)?, Dimension::Length),
        Relation::Diameter { curve, .. } => (2.0 * radius(s, curve)?, Dimension::Length),
        Relation::Angle { a, b, .. } => {
            let (a0, a1) = s.line(a)?;
            let (b0, b1) = s.line(b)?;
            (
                (b1.y.0 - b0.y.0).atan2(b1.x.0 - b0.x.0) - (a1.y.0 - a0.y.0).atan2(a1.x.0 - a0.x.0),
                Dimension::Angle,
            )
        }
        Relation::AxisAngle { line, .. } => {
            let (a, b) = s.line(line)?;
            ((b.y.0 - a.y.0).atan2(b.x.0 - a.x.0), Dimension::Angle)
        }
        Relation::EllipseRadius { curve, axis, .. } => {
            let (major, minor) = ellipse_radii(s, curve)?;
            let value = match axis {
                EllipseAxis::Major => major,
                EllipseAxis::Minor => minor,
            };
            (value, Dimension::Length)
        }
        _ => return Err(SketchError::Invalid("not a dimension".into())),
    };
    Quantity::new(value, dimension)
}
/// An ellipse's major and minor semi-axis lengths.
pub(crate) fn ellipse_radii(s: &Sketch, id: &EntityId) -> Result<(f64, f64), SketchError> {
    match s.entities.get(id) {
        Some(Entity::Ellipse {
            center,
            focus,
            minor_radius,
            ..
        }) => {
            let focal = s.point(center)?.distance(s.point(focus)?).0;
            Ok((focal.hypot(minor_radius.0), minor_radius.0))
        }
        _ => Err(SketchError::Invalid("expected ellipse".into())),
    }
}
pub(crate) fn radius(s: &Sketch, id: &EntityId) -> Result<f64, SketchError> {
    match s.entities.get(id) {
        Some(Entity::Circle { radius, .. } | Entity::Arc { radius, .. }) => Ok(radius.0),
        _ => Err(SketchError::Invalid("expected circular curve".into())),
    }
}
pub(crate) fn length(s: &Sketch, id: &EntityId) -> Result<f64, SketchError> {
    if let Some(Entity::Arc {
        radius,
        start_angle,
        end_angle,
        ..
    }) = s.entities.get(id)
    {
        Ok(radius.0 * sweep(start_angle.0, end_angle.0))
    } else {
        let (a, b) = s.line(id)?;
        Ok(a.distance(b).0)
    }
}
pub(crate) fn sweep(start: f64, end: f64) -> f64 {
    let value = (end - start).rem_euclid(std::f64::consts::TAU);
    if value <= 1e-12 {
        std::f64::consts::TAU
    } else {
        value
    }
}
pub(crate) fn distance_to_segment(p: Position, a: Position, b: Position) -> f64 {
    let dx = b.x.0 - a.x.0;
    let dy = b.y.0 - a.y.0;
    let denominator = dx * dx + dy * dy;
    let t = if denominator <= 1e-24 {
        0.0
    } else {
        ((p.x.0 - a.x.0) * dx + (p.y.0 - a.y.0) * dy) / denominator
    }
    .clamp(0.0, 1.0);
    (p.x.0 - a.x.0 - t * dx).hypot(p.y.0 - a.y.0 - t * dy)
}
