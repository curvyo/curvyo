//! Record repair permits progress through independently invalid merged components.
use crate::SketchDelta;
use curvyo_sketch_core::{Dimension, ParameterId, Parameters, Quantity, Sketch};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn allowed(before: &Sketch, after: &Sketch, delta: &SketchDelta) -> bool {
    if delta.entities.is_empty()
        && delta.parameters.is_empty()
        && !delta.constraints.is_empty()
        && delta.constraints.values().all(Option::is_none)
        && before.validate().is_err()
        && curvyo_sketch_core::validate_records(after).is_ok()
    {
        return true;
    }
    if !delta.entities.is_empty()
        || !delta.constraints.is_empty()
        || delta.parameters.is_empty()
        || before.validate().is_ok()
        || curvyo_sketch_core::validate_records(after).is_err()
    {
        return false;
    }
    for id in after.parameters.0.keys() {
        let was_valid = values(&before.parameters, std::slice::from_ref(id)).is_some();
        if (was_valid || delta.parameters.contains_key(id))
            && values(&after.parameters, std::slice::from_ref(id)).is_none()
        {
            return false;
        }
    }
    for (id, parameter) in &after.parameters.0 {
        if delta.parameters.contains_key(id)
            && after
                .parameters
                .0
                .iter()
                .any(|(other, p)| other != id && p.name == parameter.name)
        {
            return false;
        }
    }
    for constraint in before.constraints.values() {
        let Some(expression) = constraint.relation.expression() else {
            continue;
        };
        let refs = constraint
            .relation
            .parameter_ids()
            .into_iter()
            .collect::<Vec<_>>();
        let old = values(&before.parameters, &refs).and_then(|v| expression.evaluate(&v).ok());
        if let Some(old) = old {
            let Some(new) =
                values(&after.parameters, &refs).and_then(|v| expression.evaluate(&v).ok())
            else {
                return false;
            };
            if new.dimension() != old.dimension()
                || (new.dimension() == Dimension::Length && new.value() <= 0.0)
            {
                return false;
            }
        }
    }
    true
}

fn values(
    parameters: &Parameters,
    roots: &[ParameterId],
) -> Option<BTreeMap<ParameterId, Quantity>> {
    let mut pending = roots.to_vec();
    let mut seen = BTreeSet::new();
    let mut subset = Parameters::default();
    while let Some(id) = pending.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let parameter = parameters.0.get(&id)?;
        parameter.expression.references(&mut pending);
        subset.insert(id, parameter.clone());
    }
    subset.evaluate().ok()
}
