//! Independent copies replace graph identities without changing geometric intent.
use crate::{Entity, EntityId, Expression, Relation, Sketch};

impl Sketch {
    /// Copies every stable identity into a caller-supplied fresh object namespace.
    /// Parameter names and all geometric values stay unchanged, including dangling references.
    #[must_use]
    pub fn with_fresh_namespace(&self, namespace: &str) -> Self {
        let mut copy = self.clone();
        rename(&mut copy.origin, namespace);
        copy.entities = copy
            .entities
            .into_iter()
            .map(|(mut id, mut entity)| {
                rename(&mut id, namespace);
                match &mut entity {
                    Entity::Point { .. } => (),
                    Entity::Line { start, end } => {
                        rename(start, namespace);
                        rename(end, namespace);
                    }
                    Entity::Circle { center, .. } => rename(center, namespace),
                    Entity::Arc {
                        center, start, end, ..
                    } => {
                        rename(center, namespace);
                        rename(start, namespace);
                        rename(end, namespace);
                    }
                    Entity::Ellipse { center, focus, .. } => {
                        rename(center, namespace);
                        rename(focus, namespace);
                    }
                }
                (id, entity)
            })
            .collect();
        copy.constraints = copy
            .constraints
            .into_iter()
            .map(|(id, mut constraint)| {
                relation(&mut constraint.relation, namespace);
                (format!("{namespace}:{}", id.0).into(), constraint)
            })
            .collect();
        copy.parameters.0 = copy
            .parameters
            .0
            .into_iter()
            .map(|(id, mut parameter)| {
                expression(&mut parameter.expression, namespace);
                (format!("{namespace}:{}", id.0).into(), parameter)
            })
            .collect();
        copy.construction = copy
            .construction
            .into_iter()
            .map(|mut id| {
                rename(&mut id, namespace);
                id
            })
            .collect();
        copy.contact_seeds = copy
            .contact_seeds
            .into_iter()
            .map(|(id, point)| (format!("{namespace}:{}", id.0).into(), point))
            .collect();
        copy
    }
}
fn rename(id: &mut EntityId, namespace: &str) {
    id.0 = format!("{namespace}:{}", id.0);
}
fn relation(relation: &mut Relation, namespace: &str) {
    match relation {
        Relation::On { point, curve } => {
            rename(point, namespace);
            rename(curve, namespace);
        }
        Relation::Sweep { arc, value } => {
            rename(arc, namespace);
            expression(value, namespace);
        }
        Relation::PerpendicularAt { a, b, point } => {
            rename(a, namespace);
            rename(b, namespace);
            rename(point, namespace);
        }
        Relation::OnAxis { point, .. } => rename(point, namespace),
        Relation::Horizontal(id) | Relation::Vertical(id) | Relation::Fixed(id) => {
            rename(id, namespace);
        }
        Relation::Coincident(a, b)
        | Relation::Parallel(a, b)
        | Relation::Perpendicular(a, b)
        | Relation::Tangent(a, b)
        | Relation::Equal(a, b) => {
            rename(a, namespace);
            rename(b, namespace);
        }
        Relation::Distance { a, b, value } | Relation::Angle { a, b, value } => {
            rename(a, namespace);
            rename(b, namespace);
            expression(value, namespace);
        }
        Relation::Symmetric {
            source,
            image,
            axis,
        } => {
            rename(source, namespace);
            rename(image, namespace);
            rename(axis, namespace);
        }
        Relation::Midpoint { point, line } => {
            rename(point, namespace);
            rename(line, namespace);
        }
        Relation::Length { curve, value }
        | Relation::Radius { curve, value }
        | Relation::Diameter { curve, value } => {
            rename(curve, namespace);
            expression(value, namespace);
        }
    }
}
fn expression(root: &mut Expression, namespace: &str) {
    let mut pending = vec![root];
    while let Some(value) = pending.pop() {
        match value {
            Expression::Literal(_) => (),
            Expression::Reference(id) => id.0 = format!("{namespace}:{}", id.0),
            Expression::Negate(value) => pending.push(value),
            Expression::Add(a, b)
            | Expression::Subtract(a, b)
            | Expression::Multiply(a, b)
            | Expression::Divide(a, b) => {
                pending.push(a);
                pending.push(b);
            }
        }
    }
}
