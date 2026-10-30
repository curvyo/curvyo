//! User constraints expand into ACS equations with original-ID diagnostic mapping.
use crate::solver_input::Input;
use crate::{ConstraintId, Entity, EntityId, Relation, Sketch, SketchError};
use serde_json::{Value, json};

pub(crate) fn append(
    s: &Sketch,
    input: &mut Input,
    ps: &mut Vec<Value>,
) -> Result<(), SketchError> {
    let values = s.parameters.evaluate()?;
    for (id, c) in &s.constraints {
        if c.reference {
            continue;
        }
        let value = match &c.relation {
            Relation::Distance { value, .. }
            | Relation::Length { value, .. }
            | Relation::Radius { value, .. }
            | Relation::Diameter { value, .. }
            | Relation::Angle { value, .. }
            | Relation::Sweep { value, .. } => Some(value.evaluate(&values)?.value()),
            _ => None,
        };
        if let Relation::Tangent(a, b) = &c.relation
            && crate::contact::required(s, a, b)
        {
            crate::contact::append(s, input, ps, id, a, b)?;
            continue;
        }
        let relation = match &c.relation {
            Relation::On { point, curve } => {
                json!({"type":"on","point":input.id(point),"curve":input.id(curve)})
            }
            Relation::Sweep { arc, .. } => {
                json!({"type":"angle","arc":input.id(arc),"value":value})
            }
            Relation::Coincident(a, b) => {
                json!({"type":"coincident","a":input.id(a),"b":input.id(b)})
            }
            Relation::Horizontal(a) => json!({"type":"horizontal","line":input.id(a)}),
            Relation::Vertical(a) => json!({"type":"vertical","line":input.id(a)}),
            Relation::Parallel(a, b) => json!({"type":"parallel","a":input.id(a),"b":input.id(b)}),
            Relation::Perpendicular(a, b) => {
                json!({"type":"perpendicular","a":input.id(a),"b":input.id(b)})
            }
            Relation::Tangent(a, b) => {
                let mut relation = json!({"type":"tangent","a":input.id(a),"b":input.id(b)});
                if !crate::contact::joint(s, a, b) && (s.line(a).is_ok() || s.line(b).is_ok()) {
                    relation["extension"] = true.into();
                }
                relation
            }
            Relation::Equal(a, b) => json!({"type":"equal","a":input.id(a),"b":input.id(b)}),
            Relation::Fixed(entity) => {
                fixed(s, input, ps, id, entity)?;
                continue;
            }
            Relation::Symmetric {
                source,
                image,
                axis,
            } => {
                json!({"type":"mirror","source":input.id(source),"image":input.id(image),"axis":input.id(axis)})
            }
            Relation::Midpoint { point, line } => {
                json!({"type":"midpoint","entities":[input.id(point),input.id(line)]})
            }
            Relation::Distance { a, b, .. } => {
                json!({"type":"distance","a":input.id(a),"b":input.id(b),"value":value})
            }
            Relation::Length { curve, .. } => {
                json!({"type":"length","curve":input.id(curve),"value":value})
            }
            Relation::Radius { curve, .. } => {
                json!({"type":"radius","curve":input.id(curve),"value":value})
            }
            Relation::Diameter { curve, .. } => {
                json!({"type":"diameter","curve":input.id(curve),"value":value})
            }
            Relation::Angle { a, b, .. } => {
                json!({"type":"angle","a":input.id(a),"b":input.id(b),"value":value,"reverseA":false,"reverseB":false})
            }
        };
        input.add_constraint(ps, id, relation);
    }
    Ok(())
}
fn fixed(
    s: &Sketch,
    input: &mut Input,
    ps: &mut Vec<Value>,
    id: &ConstraintId,
    entity: &EntityId,
) -> Result<(), SketchError> {
    let e = s
        .entities
        .get(entity)
        .ok_or_else(|| SketchError::Invalid("missing fixed entity".into()))?;
    let points = match e {
        Entity::Point { .. } => vec![entity],
        Entity::Line { start, end } => vec![start, end],
        Entity::Circle { center, .. } => vec![center],
        Entity::Arc {
            center, start, end, ..
        } => vec![center, start, end],
        Entity::Ellipse { center, focus, .. } => vec![center, focus],
    };
    for point in points {
        let position = s.point(point)?;
        for (axis, value) in [("x", position.x.0), ("y", position.y.0)] {
            input.add_constraint(
                ps,
                id,
                json!({"type":axis,"point":input.id(point),"value":value}),
            );
        }
    }
    match e {
        Entity::Circle{radius,..}|Entity::Arc{radius,..}=>input.add_constraint(ps,id,json!({"type":"radius","curve":input.id(entity),"value":radius.0})),
        Entity::Ellipse{minor_radius,..}=>input.add_constraint(ps,id,json!({"type":"equal","a":{"entity":input.id(entity),"property":"radmin"},"b":minor_radius.0})),_=>{},
    }
    Ok(())
}
