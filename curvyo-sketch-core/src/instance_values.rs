//! Replaces exposed parameter and dimension values of a graph by literals.
use crate::{
    ConstraintId, Dimension, Expression, ParameterId, Quantity, Relation, Sketch, SketchError,
};
use std::collections::BTreeMap;

/// Own values of one instance in canonical units (millimetres, radians), by what they replace.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InstanceValues {
    /// Replacement literals of named parameters.
    pub parameters: BTreeMap<ParameterId, Quantity>,
    /// Replacement literals of driving dimensions.
    pub dimensions: BTreeMap<ConstraintId, Quantity>,
}

/// The evaluated values of the parameters and of the driving dimensions of a graph.
pub type EvaluatedValues = (
    BTreeMap<ParameterId, Quantity>,
    BTreeMap<ConstraintId, Quantity>,
);

impl InstanceValues {
    /// Whether no value is set.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.parameters.is_empty() && self.dimensions.is_empty()
    }
}

/// The expression and the dimension a relation carries when it is a dimension a clone may set:
/// lengths (distance, length, radius, diameter, horizontal and vertical distance, ellipse radius)
/// and angles (angle, axis angle, sweep).
#[must_use]
pub fn dimension_expression(relation: &Relation) -> Option<(&Expression, Dimension)> {
    match relation {
        Relation::Distance { value, .. }
        | Relation::Length { value, .. }
        | Relation::Radius { value, .. }
        | Relation::Diameter { value, .. }
        | Relation::HorizontalDistance { value, .. }
        | Relation::VerticalDistance { value, .. }
        | Relation::EllipseRadius { value, .. } => Some((value, Dimension::Length)),
        Relation::Angle { value, .. }
        | Relation::AxisAngle { value, .. }
        | Relation::Sweep { value, .. } => Some((value, Dimension::Angle)),
        _ => None,
    }
}

fn dimension_expression_mut(relation: &mut Relation) -> Option<&mut Expression> {
    match relation {
        Relation::Distance { value, .. }
        | Relation::Length { value, .. }
        | Relation::Radius { value, .. }
        | Relation::Diameter { value, .. }
        | Relation::HorizontalDistance { value, .. }
        | Relation::VerticalDistance { value, .. }
        | Relation::EllipseRadius { value, .. }
        | Relation::Angle { value, .. }
        | Relation::AxisAngle { value, .. }
        | Relation::Sweep { value, .. } => Some(value),
        _ => None,
    }
}

impl Sketch {
    /// The dimension of the driving dimension `id`, `None` for a missing constraint, a reference
    /// dimension or a relation that carries no value.
    #[must_use]
    pub fn exposable_dimension(&self, id: &ConstraintId) -> Option<Dimension> {
        let constraint = self.constraints.get(id)?;
        if constraint.reference {
            return None;
        }
        dimension_expression(&constraint.relation).map(|(_, dimension)| dimension)
    }

    /// The evaluated values of every parameter and of every driving dimension as the graph states
    /// them (its expressions evaluated against the parameters).
    ///
    /// # Errors
    /// Fails when the parameters do not evaluate.
    pub fn evaluated_values(&self) -> Result<EvaluatedValues, SketchError> {
        let parameters = self.parameters.evaluate()?;
        let mut dimensions = BTreeMap::new();
        for (id, constraint) in &self.constraints {
            if constraint.reference {
                continue;
            }
            if let Some((expression, _)) = dimension_expression(&constraint.relation)
                && let Ok(value) = expression.evaluate(&parameters)
            {
                dimensions.insert(id.clone(), value);
            }
        }
        Ok((parameters, dimensions))
    }

    /// The graph with the given values as literals: a parameter's expression is replaced first,
    /// the parameters are evaluated by the solver as always, and a dimension's literal replaces
    /// that constraint's expression last, so an own dimension value wins over an own parameter
    /// value its formula uses.
    ///
    /// # Errors
    /// Fails for a value whose parameter or driving dimension is missing or whose dimension does
    /// not match.
    pub fn with_values(&self, values: &InstanceValues) -> Result<Self, SketchError> {
        let mut graph = self.clone();
        for (id, value) in &values.parameters {
            let parameter = graph
                .parameters
                .get_mut(id)
                .ok_or_else(|| SketchError::Invalid(format!("no parameter {}", id.0)))?;
            if parameter.dimension != value.dimension() {
                return Err(SketchError::Invalid(format!(
                    "wrong unit for parameter {}",
                    id.0
                )));
            }
            parameter.expression = Expression::Literal(*value);
        }
        for (id, value) in &values.dimensions {
            let dimension = self
                .exposable_dimension(id)
                .ok_or_else(|| SketchError::Invalid(format!("no dimension {}", id.0)))?;
            if dimension != value.dimension() {
                return Err(SketchError::Invalid(format!(
                    "wrong unit for dimension {}",
                    id.0
                )));
            }
            let constraint = graph
                .constraints
                .get_mut(id)
                .ok_or_else(|| SketchError::Invalid(format!("no dimension {}", id.0)))?;
            let slot = dimension_expression_mut(&mut constraint.relation)
                .ok_or_else(|| SketchError::Invalid(format!("no dimension {}", id.0)))?;
            *slot = Expression::Literal(*value);
        }
        Ok(graph)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{Constraint, EntityId, Parameter};

    fn graph() -> Sketch {
        let mut sketch = Sketch::new("o".into());
        sketch.parameters.insert(
            "w".into(),
            Parameter::literal("width", Quantity::length(80.0).unwrap()),
        );
        sketch.parameters.insert(
            "h".into(),
            Parameter::expression(
                "half",
                Dimension::Length,
                Expression::Divide(
                    Box::new(Expression::Reference("w".into())),
                    Box::new(Expression::Literal(Quantity::scalar(2.0).unwrap())),
                ),
            ),
        );
        sketch.constraints.insert(
            "c1".into(),
            Constraint::driving(Relation::Radius {
                curve: EntityId("circle".into()),
                value: Expression::Reference("h".into()),
            }),
        );
        sketch.constraints.insert(
            "c2".into(),
            Constraint {
                relation: Relation::Radius {
                    curve: EntityId("circle".into()),
                    value: Expression::Literal(Quantity::length(5.0).unwrap()),
                },
                reference: true,
            },
        );
        sketch
    }

    #[test]
    fn a_parameter_literal_makes_formulas_that_use_it_follow() {
        let mut values = InstanceValues::default();
        values
            .parameters
            .insert("w".into(), Quantity::length(100.0).unwrap());
        let with = graph().with_values(&values).unwrap();
        let (parameters, _) = with.evaluated_values().unwrap();
        assert!((parameters[&ParameterId("h".into())].value() - 50.0).abs() < 1e-9);
    }

    #[test]
    fn a_dimension_literal_wins_over_a_parameter_literal() {
        let mut values = InstanceValues::default();
        values
            .parameters
            .insert("w".into(), Quantity::length(100.0).unwrap());
        values
            .dimensions
            .insert("c1".into(), Quantity::length(7.0).unwrap());
        let with = graph().with_values(&values).unwrap();
        let (_, dimensions) = with.evaluated_values().unwrap();
        assert!((dimensions[&ConstraintId("c1".into())].value() - 7.0).abs() < 1e-9);
    }

    #[test]
    fn a_reference_dimension_cannot_be_set_and_a_wrong_unit_is_refused() {
        let sketch = graph();
        assert_eq!(sketch.exposable_dimension(&"c2".into()), None);
        assert_eq!(
            sketch.exposable_dimension(&"c1".into()),
            Some(Dimension::Length)
        );
        let mut reference = InstanceValues::default();
        reference
            .dimensions
            .insert("c2".into(), Quantity::length(1.0).unwrap());
        assert!(sketch.with_values(&reference).is_err());
        let mut wrong = InstanceValues::default();
        wrong
            .parameters
            .insert("w".into(), Quantity::angle(1.0).unwrap());
        assert!(sketch.with_values(&wrong).is_err());
    }
}
