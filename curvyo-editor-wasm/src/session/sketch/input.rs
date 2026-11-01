//! Typed sketch input resolves names before changing authoritative records.
use super::Session;
use curvyo_document_core::SketchEditError;
use curvyo_sketch_core::{
    Constraint, ConstraintId, Dimension, Expression, Parameter, ParameterId, Quantity, Relation,
};

/// A typed dimension that contradicts other constraints (UX notes revision 2 review, change 6).
const CONFLICTS: &str = "Conflicts with other constraints: the sketch keeps its last shape";
/// A typed dimension the solver finds no shape for; solver failure does not prove a conflict (AC7).
const NO_SOLUTION: &str = "No shape fits this value: the sketch keeps its last shape";

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
        // A dimension placed on the canvas is typed into its entry box.
        let boxed = self.sketch.constraint_tool.as_deref() == Some("dimension")
            && self.sketch.dimension_position.is_some();
        let picks = self.sketch.entities.clone();
        let result = self.sketch_action_inner(action, text, reference);
        // A placed dimension, once entered, returns to Select with its geometry (AC43).
        if boxed && result.is_ok() {
            self.sketch_applied_select(picks);
        }
        self.sketch.error = result.as_ref().err().map(ToString::to_string);
        if boxed {
            match &result {
                Ok((changed, id)) => self.sketch_check_entered(id, *changed),
                Err(error) => self.sketch_refuse_entry(error),
            }
        }
        result.map(|(changed, _)| changed)
    }

    /// The box's one field refused `error`.
    fn sketch_refuse_entry(&mut self, error: &SketchEditError) {
        self.sketch.entry_error = Some(super::state::EntryError {
            field: Some(0),
            message: error.0.clone(),
        });
    }

    /// After the dimension `id` was typed into the box: a value that leaves no valid shape keeps
    /// the box open on that dimension with the reason, and marks it on the canvas (UX notes
    /// revision 2 review, change 6). The dimension stays, inspectable and undoable (AC7); when
    /// the value `changed` the document, Escape in the box takes that step back.
    fn sketch_check_entered(&mut self, id: &ConstraintId, changed: bool) {
        let Some(snapshot) = self
            .sketch
            .active
            .and_then(|node| self.document.sketch(node))
        else {
            return;
        };
        let failed = snapshot.status == Some(curvyo_sketch_core::SolveStatus::SolverFailure);
        let message = if snapshot.conflicting.contains(id) {
            Some(CONFLICTS)
        } else if failed {
            Some(NO_SOLUTION)
        } else {
            None
        };
        self.sketch.failed_dimension = failed.then(|| id.clone());
        self.sketch.entry_error = message.map(|message| super::state::EntryError {
            field: Some(0),
            message: message.to_owned(),
        });
        self.sketch.refused_step = (changed && message.is_some()).then(|| self.undo_len());
        if message.is_some() {
            self.sketch_edit_dimension(id);
        }
    }

    fn sketch_action_inner(
        &mut self,
        action: &str,
        text: &str,
        reference: bool,
    ) -> Result<(bool, ConstraintId), SketchEditError> {
        let snapshot = self.sketch_snapshot_for_edit()?;
        let value = if matches!(
            action,
            "distance"
                | "horizontal-distance"
                | "vertical-distance"
                | "length"
                | "radius"
                | "diameter"
                | "angle"
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
        let mut relations =
            curvyo_ui_core::sketch_relations(action, &snapshot.graph, &ids, value.as_ref())
                .map_err(SketchEditError)?;
        if reference && relations.iter().any(|r| r.expression().is_none()) {
            return Err(SketchEditError("Only dimensions can be references".into()));
        }
        if relations.len() > 1 {
            // Parallel, Equal or Fixed of several members: one edit, one history step (AC40).
            let constraints = relations
                .into_iter()
                .map(|relation| {
                    let id: ConstraintId = self.minter.mint().to_hex().into();
                    (id, Some(Constraint::driving(relation)))
                })
                .collect::<Vec<_>>();
            let first = constraints[0].0.clone();
            let constraints = constraints.into_iter().collect();
            let changed = self.sketch_apply_delta(&curvyo_document_core::SketchDelta {
                constraints,
                ..Default::default()
            })?;
            return Ok((changed, first));
        }
        let relation = relations
            .pop()
            .ok_or_else(|| SketchEditError("Nothing to apply".into()))?;
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
            Ok((changed, id))
        } else {
            self.sketch_set_constraint(id.clone(), constraint)
                .map(|changed| (changed, id))
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
        // An edit typed into the canvas box answers there (UX notes revision 2, section 3); the
        // document change closes the box, so whether it was open is read first.
        let boxed = self.sketch.canvas_edit.as_ref() == Some(id);
        let result = self.sketch_dimension_text_inner(id, text, reference);
        if boxed {
            match &result {
                Ok(changed) => self.sketch_check_entered(id, *changed),
                Err(error) => self.sketch_refuse_entry(error),
            }
        }
        result
    }

    fn sketch_dimension_text_inner(
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
        let dimension = constraint
            .relation
            .dimension()
            .ok_or_else(|| SketchEditError("This relationship has no numeric value".into()))?;
        let expression = parse_value(text, dimension, &snapshot.graph.parameters)?;
        if let Some(value) = constraint.relation.expression_mut() {
            *value = expression;
        }
        keep_axis_side(&snapshot.solution, &mut constraint.relation);
        constraint.reference = reference;
        self.sketch_set_constraint(id.clone(), constraint)
    }
}

/// Orders an axis distance's points so the solved geometry measures it non-negative (AC46).
///
/// A reference axis distance follows the geometry, so a drag can leave its signed measure
/// negative while the text shows the size; a value entered then keeps that side.
fn keep_axis_side(solution: &curvyo_sketch_core::Sketch, relation: &mut Relation) {
    let negative = curvyo_sketch_core::measure(solution, relation).is_ok_and(|q| q.value() < 0.0);
    if negative
        && let Relation::HorizontalDistance { a, b, .. } | Relation::VerticalDistance { a, b, .. } =
            relation
    {
        std::mem::swap(a, b);
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
