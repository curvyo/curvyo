//! Parameter evaluation rejects cycles before geometry is changed.
use crate::{Dimension, Expression, ParameterId, Quantity, SketchError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A named typed parameter with a stable-ID expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    /// Display name used while parsing new expressions.
    pub name: String,
    /// Required result dimension.
    pub dimension: Dimension,
    /// The authoritative expression, independent of display rounding.
    pub expression: Expression,
}
impl Parameter {
    /// Creates a constant parameter.
    #[must_use]
    pub fn literal(name: &str, value: Quantity) -> Self {
        Self::expression(name, value.dimension(), Expression::Literal(value))
    }
    /// Creates a parameter from a typed expression tree.
    #[must_use]
    pub fn expression(name: &str, dimension: Dimension, expression: Expression) -> Self {
        Self {
            name: name.into(),
            dimension,
            expression,
        }
    }
}

/// Stable parameter records with deterministic evaluation order.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Parameters(pub BTreeMap<ParameterId, Parameter>);
impl Parameters {
    /// Inserts or replaces a stable parameter record.
    pub fn insert(&mut self, id: ParameterId, parameter: Parameter) {
        self.0.insert(id, parameter);
    }
    /// Accesses a parameter for an explicit caller-owned edit.
    pub fn get_mut(&mut self, id: &ParameterId) -> Option<&mut Parameter> {
        self.0.get_mut(id)
    }
    /// Resolves every expression without mutating the parameter records.
    ///
    /// # Errors
    /// Rejects invalid names, cycles, missing references and incompatible units.
    pub fn evaluate(&self) -> Result<BTreeMap<ParameterId, Quantity>, SketchError> {
        let mut names = BTreeSet::new();
        for (id, p) in &self.0 {
            if id.0.is_empty() || !crate::parser::valid_name(&p.name) || !names.insert(&p.name) {
                return Err(SketchError::Formula(format!(
                    "invalid or duplicate parameter name {}",
                    p.name
                )));
            }
        }
        let mut values = BTreeMap::new();
        let mut visiting = BTreeSet::new();
        for root in self.0.keys() {
            let mut stack = vec![(root.clone(), false)];
            while let Some((id, ready)) = stack.pop() {
                if values.contains_key(&id) {
                    continue;
                }
                let p = self
                    .0
                    .get(&id)
                    .ok_or_else(|| SketchError::Formula(format!("unknown parameter {}", id.0)))?;
                if ready {
                    let value = p
                        .expression
                        .evaluate(&values)
                        .map_err(|e| SketchError::Formula(format!("{}: {e}", p.name)))?;
                    if value.dimension() != p.dimension {
                        return Err(SketchError::Formula(format!(
                            "{}: wrong result units",
                            p.name
                        )));
                    }
                    visiting.remove(&id);
                    values.insert(id, value);
                } else {
                    if !visiting.insert(id.clone()) {
                        return Err(SketchError::Formula(format!("cycle at {}", p.name)));
                    }
                    stack.push((id, true));
                    let mut dependencies = vec![];
                    p.expression.references(&mut dependencies);
                    for dependency in dependencies {
                        stack.push((dependency, false));
                    }
                }
            }
        }
        Ok(values)
    }
}
