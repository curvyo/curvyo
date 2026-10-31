//! Independent copies replace graph identities without changing geometric intent.
use crate::{EntityId, Expression, Relation, Sketch};

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
                for point in entity.point_ids_mut() {
                    rename(point, namespace);
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
    for id in relation.entity_ids_mut() {
        rename(id, namespace);
    }
    if let Some(value) = relation.expression_mut() {
        expression(value, namespace);
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
