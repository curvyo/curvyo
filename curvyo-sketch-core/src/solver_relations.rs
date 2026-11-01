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
        let value = match c.relation.expression() {
            Some(value) => Some(value.evaluate(&values)?.value()),
            None => None,
        };
        if let Relation::Tangent(a, b) = &c.relation
            && crate::contact::required(s, a, b)
        {
            crate::contact::append(s, input, ps, id, a, b)?;
            continue;
        }
        let relation = match &c.relation {
            Relation::PerpendicularAt { a, b, point } => {
                perpendicular_at(s, input, ps, id, a, b, point)?
            }
            Relation::OnAxis { point, axis } => {
                json!({"type":match axis {crate::Axis::X=>"y",crate::Axis::Y=>"x"},"point":input.id(point),"value":0.0})
            }
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
                // Joined arcs keep the side of the current pose; ACS never flips a join.
                if crate::tangency::internal_joint(s, a, b) == Some(true) {
                    relation["internal"] = true.into();
                }
                relation
            }
            Relation::Equal(a, b) => json!({"type":"equal","a":input.id(a),"b":input.id(b)}),
            Relation::Fixed(entity) => {
                fixed(s, input, ps, id, entity)?;
                continue;
            }
            Relation::Symmetric { .. }
            | Relation::SymmetricPoint { .. }
            | Relation::SymmetricAxis { .. }
            | Relation::HorizontalPoints(..)
            | Relation::VerticalPoints(..)
            | Relation::PointOn { .. } => point_relation(s, input, ps, id, &c.relation)?,
            Relation::Midpoint { point, line } => {
                json!({"type":"midpoint","entities":[input.id(point),input.id(line)]})
            }
            Relation::Distance { a, b, .. } => {
                json!({"type":"distance","a":input.id(a),"b":input.id(b),"value":value})
            }
            Relation::HorizontalDistance { a, b, .. } => axis(input, "horizontal", a, b, value),
            Relation::VerticalDistance { a, b, .. } => axis(input, "vertical", a, b, value),
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
            Relation::AxisAngle { line, .. } => {
                json!({"type":"direction","line":input.id(line),"value":value})
            }
            Relation::EllipseRadius {
                curve,
                axis: crate::EllipseAxis::Minor,
                ..
            } => {
                json!({"type":"equal","a":{"entity":input.id(curve),"property":"radmin"},"b":value})
            }
            Relation::EllipseRadius {
                curve,
                axis: crate::EllipseAxis::Major,
                ..
            } => major_radius(s, input, ps, id, curve, value)?,
        };
        input.add_constraint(ps, id, relation);
    }
    Ok(())
}
/// A signed distance along the local X (`horizontal`) or Y (`vertical`) axis: ACS's `b - a`
/// coordinate difference, so the stored point order keeps the side.
fn axis(input: &Input, direction: &str, a: &EntityId, b: &EntityId, value: Option<f64>) -> Value {
    json!({"type":format!("{direction}_distance"),"a":input.id(a),"b":input.id(b),"value":value})
}

/// Symmetry, point alignment and point-on relations, with their operand-only helpers.
fn point_relation(
    s: &Sketch,
    input: &Input,
    ps: &mut Vec<Value>,
    id: &ConstraintId,
    relation: &Relation,
) -> Result<Value, SketchError> {
    Ok(match relation {
        Relation::SymmetricPoint {
            source,
            image,
            center,
        } => {
            // The centre is the midpoint of an operand-only line through both points.
            let key = format!("symmetric_operand_{}", id.0);
            ps.push(
                json!({"type":"line","id":key,"p1_id":input.id(source),"p2_id":input.id(image)}),
            );
            json!({"type":"midpoint","entities":[input.id(center),key]})
        }
        Relation::Symmetric {
            source,
            image,
            axis,
        } => {
            json!({"type":"mirror","source":input.id(source),"image":input.id(image),"axis":input.id(axis)})
        }
        Relation::SymmetricAxis {
            source,
            image,
            axis,
        } => {
            let key = axis_operand(ps, id, *axis);
            json!({"type":"mirror","source":input.id(source),"image":input.id(image),"axis":key})
        }
        Relation::HorizontalPoints(a, b) => {
            json!({"type":"horizontal","a":input.id(a),"b":input.id(b)})
        }
        Relation::VerticalPoints(a, b) => {
            json!({"type":"vertical","a":input.id(a),"b":input.id(b)})
        }
        Relation::PointOn { point, curve } => point_on(s, input, ps, id, point, curve),
        _ => {
            return Err(SketchError::Invalid(
                "not a symmetry, point alignment or point-on relation".into(),
            ));
        }
    })
}

/// An ellipse's major semi-axis: ACS derives it and has no relation for it, so an operand point
/// is held on the end of the major axis it starts nearest (two unknowns, two equations) and its
/// distance from the centre is the value.
fn major_radius(
    s: &Sketch,
    input: &mut Input,
    ps: &mut Vec<Value>,
    id: &ConstraintId,
    curve: &EntityId,
    value: Option<f64>,
) -> Result<Value, SketchError> {
    let Some(Entity::Ellipse { center, focus, .. }) = s.entities.get(curve) else {
        return Err(SketchError::Invalid(
            "ellipse radius needs an ellipse".into(),
        ));
    };
    let (c, f) = (s.point(center)?, s.point(focus)?);
    let (major, _) = crate::measure::ellipse_radii(s, curve)?;
    let focal = c.distance(f).0;
    let (ux, uy) = if focal > 0.0 {
        ((f.x.0 - c.x.0) / focal, (f.y.0 - c.y.0) / focal)
    } else {
        (1.0, 0.0)
    };
    let key = format!("ellipse_radius_operand_{}", id.0);
    ps.push(json!({"type":"point","id":key,"x":c.x.0 + major * ux,"y":c.y.0 + major * uy,"fixed":false}));
    input.add_constraint(
        ps,
        id,
        json!({"type":"ellipse_axis","ellipse":input.id(curve),"point":key,"which":"major"}),
    );
    Ok(json!({"type":"distance","a":input.id(center),"b":key,"value":value}))
}

/// An operand-only line along the local X or Y axis between two fixed operand points; it adds
/// no unknowns.
fn axis_operand(ps: &mut Vec<Value>, id: &ConstraintId, axis: crate::Axis) -> String {
    let key = format!("axis_operand_{}", id.0);
    let (x, y) = match axis {
        crate::Axis::X => (1.0, 0.0),
        crate::Axis::Y => (0.0, 1.0),
    };
    for (suffix, px, py) in [("a", 0.0, 0.0), ("b", x, y)] {
        ps.push(json!({"type":"point","id":format!("{key}_{suffix}"),"x":px,"y":py,"fixed":true}));
    }
    ps.push(json!({"type":"line","id":key,"p1_id":format!("{key}_a"),"p2_id":format!("{key}_b")}));
    key
}

/// A point on a segment's infinite line, or on the full circle of an arc: ACS keeps an arc's
/// point on its span, so the arc's circle is an operand circle of equal radius on its centre
/// (one unknown, one equation).
fn point_on(
    s: &Sketch,
    input: &Input,
    ps: &mut Vec<Value>,
    id: &ConstraintId,
    point: &EntityId,
    curve: &EntityId,
) -> Value {
    match s.entities.get(curve) {
        Some(Entity::Line { .. }) => {
            json!({"type":"on","point":input.id(point),"curve":input.id(curve),"extension":true})
        }
        Some(Entity::Arc { center, radius, .. }) => {
            let key = format!("point_on_operand_{}", id.0);
            ps.push(json!({"type":"circle","id":key,"c_id":input.id(center),"radius":radius.0,"fixed":false}));
            ps.push(
                json!({"type":"equal","id":format!("{key}_radius"),"a":key,"b":input.id(curve)}),
            );
            json!({"type":"on","point":input.id(point),"curve":key})
        }
        _ => json!({"type":"on","point":input.id(point),"curve":input.id(curve)}),
    }
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

#[allow(clippy::too_many_arguments)] // The original relation supplies three concrete stable references.
fn perpendicular_at(
    s: &Sketch,
    input: &Input,
    ps: &mut Vec<Value>,
    id: &ConstraintId,
    a: &EntityId,
    b: &EntityId,
    point: &EntityId,
) -> Result<Value, SketchError> {
    match (s.entities.get(a), s.entities.get(b)) {
        (Some(Entity::Line { .. }), Some(Entity::Arc { .. })) => {
            Ok(json!({"type":"normal","line":input.id(a),"curve":input.id(b)}))
        }
        (Some(Entity::Arc { .. }), Some(Entity::Line { .. })) => {
            Ok(json!({"type":"normal","line":input.id(b),"curve":input.id(a)}))
        }
        (Some(Entity::Arc { center: first, .. }), Some(Entity::Arc { center: second, .. })) => {
            let left = format!("perpendicular_operand_{}_a", id.0);
            let right = format!("perpendicular_operand_{}_b", id.0);
            for (key, center) in [(&left, first), (&right, second)] {
                // Operand-only lines reuse existing variables; no graph geometry or unknowns.
                ps.push(json!({"type":"line","id":key,"p1_id":input.id(center),"p2_id":input.id(point)}));
            }
            Ok(json!({"type":"perpendicular","a":left,"b":right}))
        }
        _ => Err(SketchError::Invalid(
            "Endpoint perpendicular needs connected line/arc geometry".into(),
        )),
    }
}
