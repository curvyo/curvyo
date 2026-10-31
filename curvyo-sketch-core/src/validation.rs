//! Graph validation rejects malformed records before entering the numerical backend.
use crate::{Dimension, Entity, EntityId, Relation, Sketch, SketchError};

pub(crate) fn validate(sketch: &Sketch) -> Result<(), SketchError> {
    sketch.point(&sketch.origin)?;
    let values = sketch.parameters.evaluate()?;
    for (id, point) in &sketch.contact_seeds {
        if !point.x.is_finite()
            || !point.y.is_finite()
            || !sketch
                .constraints
                .get(id)
                .is_some_and(|c| matches!(c.relation, Relation::Tangent(..)))
        {
            return Err(invalid("invalid tangent checkpoint seed"));
        }
    }
    for id in &sketch.construction {
        entity(sketch, id)?;
    }
    for (id, entity) in &sketch.entities {
        entity_id(id)?;
        validate_entity(sketch, entity)?;
    }
    for (id, constraint) in &sketch.constraints {
        if id.0.is_empty() {
            return Err(invalid("empty constraint ID"));
        }
        relation(sketch, &constraint.relation)?;
        let dimension = constraint
            .relation
            .expression()
            .zip(constraint.relation.dimension());
        if constraint.reference && dimension.is_none() {
            return Err(invalid("only dimensions can be reference-only"));
        }
        if let Some((expression, dimension)) = dimension
            && !constraint.reference
        {
            let value = expression.evaluate(&values)?;
            if value.dimension() != dimension
                || (dimension == Dimension::Length && value.value() <= 0.0)
            {
                return Err(invalid(&format!("{}: invalid dimension value", id.0)));
            }
        }
    }
    Ok(())
}
/// Entity IDs are non-empty; the `@` prefix is reserved for selection-only references such as
/// the local axes, so a record can never shadow one.
fn entity_id(id: &EntityId) -> Result<(), SketchError> {
    if id.0.is_empty() {
        Err(invalid("empty entity ID"))
    } else if id.0.starts_with('@') {
        Err(invalid("entity IDs starting with @ are reserved"))
    } else {
        Ok(())
    }
}
fn invalid(message: &str) -> SketchError {
    SketchError::Invalid(message.into())
}
fn positive(value: f64) -> Result<(), SketchError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(invalid("radius must be finite and positive"))
    }
}
fn validate_entity(s: &Sketch, e: &Entity) -> Result<(), SketchError> {
    match e {
        Entity::Point { position, .. } => {
            if !position.x.is_finite() || !position.y.is_finite() {
                return Err(invalid("non-finite point"));
            }
        }
        Entity::Line { start, end } => {
            s.point(start)?;
            s.point(end)?;
        }
        Entity::Circle { center, radius, .. } => {
            s.point(center)?;
            positive(radius.0)?;
        }
        Entity::Arc {
            center,
            start,
            end,
            radius,
            start_angle,
            end_angle,
            ..
        } => {
            s.point(center)?;
            s.point(start)?;
            s.point(end)?;
            positive(radius.0)?;
            if !start_angle.0.is_finite() || !end_angle.0.is_finite() {
                return Err(invalid("non-finite arc angle"));
            }
        }
        Entity::Ellipse {
            center,
            focus,
            minor_radius,
            ..
        } => {
            s.point(center)?;
            s.point(focus)?;
            positive(minor_radius.0)?;
        }
    }
    Ok(())
}
fn round(s: &Sketch, id: &EntityId) -> Result<(), SketchError> {
    if matches!(
        s.entities.get(id),
        Some(Entity::Circle { .. } | Entity::Arc { .. })
    ) {
        Ok(())
    } else {
        Err(invalid("expected circle or arc"))
    }
}
fn entity(s: &Sketch, id: &EntityId) -> Result<(), SketchError> {
    if s.entities.contains_key(id) {
        Ok(())
    } else {
        Err(invalid(&format!("missing entity {}", id.0)))
    }
}
fn relation(s: &Sketch, r: &Relation) -> Result<(), SketchError> {
    if r.expression().is_some() {
        return dimension(s, r);
    }
    match r {
        Relation::PerpendicularAt { a, b, point } => {
            crate::endpoint_tangent(s, a, point)?;
            crate::endpoint_tangent(s, b, point)?;
            if !matches!(s.entities.get(a), Some(Entity::Arc { .. }))
                && !matches!(s.entities.get(b), Some(Entity::Arc { .. }))
            {
                return Err(invalid("Endpoint perpendicular needs at least one arc"));
            }
        }
        Relation::On { point, curve } => {
            s.point(point)?;
            entity(s, curve)?;
        }
        Relation::Horizontal(a) | Relation::Vertical(a) => {
            s.line(a)?;
        }
        Relation::Parallel(a, b) | Relation::Perpendicular(a, b) => {
            s.line(a)?;
            s.line(b)?;
        }
        Relation::Tangent(a, b) => {
            entity(s, a)?;
            entity(s, b)?;
        }
        Relation::Equal(a, b) => {
            if s.line(a).is_ok() {
                s.line(b)?;
            } else {
                round(s, a)?;
                round(s, b)?;
            }
        }
        Relation::Fixed(a) => entity(s, a)?,
        Relation::Midpoint { point, line } => {
            s.point(point)?;
            s.line(line)?;
        }
        _ => point_relation(s, r)?,
    }
    Ok(())
}
/// Relations that place points: on an axis or curve, together, level, plumb or mirrored.
fn point_relation(s: &Sketch, r: &Relation) -> Result<(), SketchError> {
    match r {
        Relation::OnAxis { point, .. } => {
            s.point(point)?;
        }
        Relation::Coincident(a, b)
        | Relation::SymmetricAxis {
            source: a,
            image: b,
            ..
        }
        | Relation::HorizontalPoints(a, b)
        | Relation::VerticalPoints(a, b) => {
            s.point(a)?;
            s.point(b)?;
        }
        Relation::Symmetric {
            source,
            image,
            axis,
        } => {
            s.point(source)?;
            s.point(image)?;
            s.line(axis)?;
        }
        Relation::SymmetricPoint {
            source,
            image,
            center,
        } => {
            s.point(source)?;
            s.point(image)?;
            s.point(center)?;
        }
        Relation::PointOn { point, curve } => {
            s.point(point)?;
            if !matches!(
                s.entities.get(curve),
                Some(
                    Entity::Line { .. }
                        | Entity::Circle { .. }
                        | Entity::Arc { .. }
                        | Entity::Ellipse { .. }
                )
            ) {
                return Err(invalid("point on needs a line, circle, arc or ellipse"));
            }
        }
        _ => return Err(invalid("unknown relation")),
    }
    Ok(())
}
/// Dimensions measure the kind of geometry they name.
fn dimension(s: &Sketch, r: &Relation) -> Result<(), SketchError> {
    match r {
        Relation::Sweep { arc, .. } => {
            if !matches!(s.entities.get(arc), Some(Entity::Arc { .. })) {
                return Err(invalid("sweep needs arc"));
            }
        }
        Relation::Angle { a, b, .. } => {
            s.line(a)?;
            s.line(b)?;
        }
        Relation::AxisAngle { line, .. } => {
            s.line(line)?;
        }
        Relation::EllipseRadius { curve, .. } => {
            if !matches!(s.entities.get(curve), Some(Entity::Ellipse { .. })) {
                return Err(invalid("ellipse radius needs an ellipse"));
            }
        }
        Relation::Distance { a, b, .. } => {
            s.point(a)?;
            if s.point(b).is_err() {
                s.line(b)?;
            }
        }
        Relation::Length { curve, .. } => {
            if s.line(curve).is_err() && !matches!(s.entities.get(curve), Some(Entity::Arc { .. }))
            {
                return Err(invalid("length needs line or arc"));
            }
        }
        Relation::Radius { curve, .. } | Relation::Diameter { curve, .. } => round(s, curve)?,
        _ => return Err(invalid("not a dimension")),
    }
    Ok(())
}

/// Checks individual records without requiring their references to resolve.
/// Missing references, duplicate names and dependency cycles may result from merges.
///
/// # Errors
/// Rejects malformed numeric values, IDs, names or reference-only relation kinds.
pub fn validate_records(sketch: &Sketch) -> Result<(), SketchError> {
    if sketch.origin.0.is_empty() {
        return Err(invalid("empty origin ID"));
    }
    for (id, entity) in &sketch.entities {
        entity_id(id)?;
        match entity {
            Entity::Point { position, .. }
                if !position.x.is_finite() || !position.y.is_finite() =>
            {
                return Err(invalid("non-finite point"));
            }
            Entity::Circle { radius, .. } | Entity::Arc { radius, .. } => positive(radius.0)?,
            Entity::Ellipse { minor_radius, .. } => positive(minor_radius.0)?,
            _ => {}
        }
        if let Entity::Arc {
            start_angle,
            end_angle,
            ..
        } = entity
            && (!start_angle.0.is_finite() || !end_angle.0.is_finite())
        {
            return Err(invalid("non-finite arc angle"));
        }
    }
    for (id, parameter) in &sketch.parameters.0 {
        if id.0.is_empty() || !crate::parser::valid_name(&parameter.name) {
            return Err(invalid("invalid parameter record"));
        }
    }
    for (id, constraint) in &sketch.constraints {
        if id.0.is_empty() {
            return Err(invalid("empty constraint ID"));
        }
        if constraint.reference && constraint.relation.dimension().is_none() {
            return Err(invalid("only dimensions can be reference-only"));
        }
    }
    Ok(())
}
