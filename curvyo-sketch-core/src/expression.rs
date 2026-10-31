//! Typed expression trees retain stable parameter references.
use crate::{ParameterId, Parameters, Quantity, SketchError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A persisted formula uses IDs rather than mutable display names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    /// A dimensioned constant.
    Literal(Quantity),
    /// A stable reference to another parameter.
    Reference(ParameterId),
    /// Addition of matching quantities.
    Add(Box<Self>, Box<Self>),
    /// Subtraction of matching quantities.
    Subtract(Box<Self>, Box<Self>),
    /// Multiplication with a scalar operand.
    Multiply(Box<Self>, Box<Self>),
    /// Division by a scalar or matching dimension.
    Divide(Box<Self>, Box<Self>),
    /// Sign inversion preserving units.
    Negate(Box<Self>),
}
impl Expression {
    /// Parses units, arithmetic and parameter names into stable references.
    ///
    /// # Errors
    /// Rejects invalid syntax, units and unresolved parameter names.
    pub fn parse(text: &str, parameters: &Parameters) -> Result<Self, SketchError> {
        crate::parser::parse(text, parameters)
    }
    /// Evaluates against an already resolved parameter environment.
    ///
    /// # Errors
    /// Rejects missing references, incompatible units and non-finite arithmetic.
    pub fn evaluate(
        &self,
        values: &BTreeMap<ParameterId, Quantity>,
    ) -> Result<Quantity, SketchError> {
        let mut stack = vec![(self, false)];
        let mut results = vec![];
        while let Some((expression, ready)) = stack.pop() {
            match expression {
                Self::Literal(q) => results.push(Quantity::new(q.value(), q.dimension())?),
                Self::Reference(id) => {
                    let q = values.get(id).ok_or_else(|| {
                        SketchError::Formula(format!("unresolved parameter {}", id.0))
                    })?;
                    results.push(Quantity::new(q.value(), q.dimension())?);
                }
                Self::Negate(a) => {
                    if ready {
                        let q = pop(&mut results)?;
                        results.push(Quantity::new(-q.value(), q.dimension())?);
                    } else {
                        stack.push((expression, true));
                        stack.push((a, false));
                    }
                }
                Self::Add(a, b)
                | Self::Subtract(a, b)
                | Self::Multiply(a, b)
                | Self::Divide(a, b) => {
                    if ready {
                        let right = pop(&mut results)?;
                        let left = pop(&mut results)?;
                        results.push(match expression {
                            Self::Add(..) => left.add(right, false)?,
                            Self::Subtract(..) => left.add(right, true)?,
                            Self::Multiply(..) => left.multiply(right, false)?,
                            _ => left.multiply(right, true)?,
                        });
                    } else {
                        stack.push((expression, true));
                        stack.push((b, false));
                        stack.push((a, false));
                    }
                }
            }
        }
        pop(&mut results)
    }
    /// Appends stable parameter dependencies using an iterative traversal.
    pub fn references(&self, out: &mut Vec<ParameterId>) {
        let mut stack = vec![self];
        while let Some(e) = stack.pop() {
            match e {
                Self::Reference(id) => out.push(id.clone()),
                Self::Add(a, b)
                | Self::Subtract(a, b)
                | Self::Multiply(a, b)
                | Self::Divide(a, b) => {
                    stack.push(a);
                    stack.push(b);
                }
                Self::Negate(a) => stack.push(a),
                Self::Literal(_) => {}
            }
        }
    }
}
fn pop(values: &mut Vec<Quantity>) -> Result<Quantity, SketchError> {
    values
        .pop()
        .ok_or_else(|| SketchError::Formula("incomplete expression".into()))
}
