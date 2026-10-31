//! Typed sketch input resolves names before changing authoritative records.
use super::Session;
use curvyo_document_core::SketchEditError;
use curvyo_sketch_core::{
    Constraint, ConstraintId, Dimension, Expression, Parameter, ParameterId, Quantity, Relation,
};

impl Session {
    /// Applies a selection-compatible relation or dimension, including reference mode.
    /// # Errors
    /// Reports incompatible selection or invalid typed input without a document write.
    pub fn sketch_apply_action(
        &mut self,
        action: &str,
        text: &str,
        reference: bool,
    ) -> Result<bool, SketchEditError> {
        let result = self.sketch_action_inner(action, text, reference);
        self.sketch.error = result.as_ref().err().map(ToString::to_string);
        result
    }
    fn sketch_action_inner(
        &mut self,
        action: &str,
        text: &str,
        reference: bool,
    ) -> Result<bool, SketchEditError> {
        let snapshot = self.sketch_snapshot_for_edit()?;
        let value = if matches!(
            action,
            "distance" | "length" | "radius" | "diameter" | "angle"
        ) {
            Some(parse_value(
                text,
                if action == "angle" {
                    Dimension::Angle
                } else {
                    Dimension::Length
                },
                &snapshot.graph.parameters,
            )?)
        } else {
            None
        };
        let ids = self.sketch.entities.iter().cloned().collect::<Vec<_>>();
        let relation =
            curvyo_ui_core::sketch_relation(action, &snapshot.graph, &ids, value.as_ref())
                .map_err(SketchEditError)?;
        if reference && relation.expression().is_none() {
            return Err(SketchEditError("Only dimensions can be references".into()));
        }
        let id: ConstraintId = self.minter.mint().to_hex().into();
        let constraint = Constraint {
            relation,
            reference,
        };
        if self.sketch.constraint_tool.as_deref() == Some("dimension") {
            let position = self.sketch.dimension_position.ok_or_else(|| {
                SketchEditError("Place the dimension label on the canvas first".into())
            })?;
            if !position.x.0.is_finite() || !position.y.0.is_finite() {
                return Err(SketchEditError("Dimension position must be finite".into()));
            }
            let changed = self.recording(|this| {
                this.document.batch("edit_sketch", |doc| {
                    let changed = doc.apply_sketch_delta(
                        snapshot.id,
                        &curvyo_document_core::SketchDelta {
                            constraints: [(id.clone(), Some(constraint))].into(),
                            ..Default::default()
                        },
                    )?;
                    doc.set_sketch_label(snapshot.id, &id, position)?;
                    Ok::<_, SketchEditError>(changed)
                })
            })?;
            self.sketch.dimension_position = None;
            self.sketch.hover = None;
            Ok(changed)
        } else {
            self.sketch_set_constraint(id, constraint)
        }
    }

    /// Adds a parameter with a collision-free document actor identity.
    /// # Errors
    /// Rejects invalid names, dimensions or expressions without a history step.
    pub fn sketch_add_parameter(
        &mut self,
        name: &str,
        text: &str,
        dimension: Dimension,
    ) -> Result<ParameterId, SketchEditError> {
        let id: ParameterId = self.minter.mint().to_hex().into();
        self.sketch_parameter_text(id.clone(), name, text, dimension)?;
        Ok(id)
    }

    /// Parses a named parameter edit with explicit units; existing stable IDs survive rename.
    /// # Errors
    /// Leaves the prior valid value untouched on parse, cycle or unit errors.
    pub fn sketch_parameter_text(
        &mut self,
        id: ParameterId,
        name: &str,
        text: &str,
        dimension: Dimension,
    ) -> Result<bool, SketchEditError> {
        let graph = self.sketch_intent_for_edit()?;
        let expression = parse_expression(text, dimension, &graph.parameters)?;
        let parameter = Parameter::expression(name, dimension, expression);
        self.sketch_set_parameter(id, parameter)
    }

    /// Replaces an existing driving/reference dimension's expression.
    /// # Errors
    /// Rejects a missing or nondimensional relationship or malformed input.
    pub fn sketch_dimension_text(
        &mut self,
        id: &ConstraintId,
        text: &str,
        reference: bool,
    ) -> Result<bool, SketchEditError> {
        let snapshot = self.sketch_snapshot_for_edit()?;
        let mut constraint = snapshot
            .graph
            .constraints
            .get(id)
            .cloned()
            .ok_or_else(|| SketchEditError("Dimension no longer exists".into()))?;
        let dimension = if matches!(
            constraint.relation,
            Relation::Angle { .. } | Relation::Sweep { .. }
        ) {
            Dimension::Angle
        } else {
            Dimension::Length
        };
        let expression = parse_value(text, dimension, &snapshot.graph.parameters)?;
        match &mut constraint.relation {
            Relation::Distance { value, .. }
            | Relation::Length { value, .. }
            | Relation::Radius { value, .. }
            | Relation::Diameter { value, .. }
            | Relation::Angle { value, .. }
            | Relation::Sweep { value, .. } => *value = expression,
            _ => {
                return Err(SketchEditError(
                    "This relationship has no numeric value".into(),
                ));
            }
        }
        constraint.reference = reference;
        self.sketch_set_constraint(id.clone(), constraint)
    }
}

pub(super) fn parse_value(
    text: &str,
    dimension: Dimension,
    parameters: &curvyo_sketch_core::Parameters,
) -> Result<Expression, SketchEditError> {
    let expression = parse_expression(text, dimension, parameters)?;
    let values = parameters
        .evaluate()
        .map_err(|e| SketchEditError(e.to_string()))?;
    let value = expression
        .evaluate(&values)
        .map_err(|e| SketchEditError(e.to_string()))?;
    if value.dimension() != dimension {
        return Err(SketchEditError("Expression has incompatible units".into()));
    }
    Ok(expression)
}

fn parse_expression(
    text: &str,
    dimension: Dimension,
    parameters: &curvyo_sketch_core::Parameters,
) -> Result<Expression, SketchEditError> {
    let expression = if let Ok(number) = text.trim().parse::<f64>() {
        // Bare lengths are millimetres; the Sketch angle entry displays degrees.
        let number = if dimension == Dimension::Angle {
            number.to_radians()
        } else {
            number
        };
        Expression::Literal(
            Quantity::new(number, dimension).map_err(|e| SketchEditError(e.to_string()))?,
        )
    } else {
        Expression::parse(text, parameters).map_err(|e| SketchEditError(e.to_string()))?
    };
    Ok(expression)
}
