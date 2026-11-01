//! Typed construction values complete the same analytic commands as pointer picks.
use super::{Session, SketchTool};
use curvyo_document_core::SketchEditError;
use curvyo_sketch_core::{Construction, Dimension, Length, Position};

impl Session {
    /// Previews a fillet from a length expression without committing history.
    /// # Errors
    /// Reports invalid expressions or an ineligible selected joint.
    pub fn sketch_preview_fillet_text(&mut self, text: &str) -> Result<(), SketchEditError> {
        self.sketch.preview = None;
        self.sketch.preview_base = None;
        let result = self.sketch_fillet_text_inner(text);
        self.sketch.error = result.as_ref().err().map(ToString::to_string);
        result
    }

    fn sketch_fillet_text_inner(&mut self, text: &str) -> Result<(), SketchEditError> {
        let snapshot = self.sketch_snapshot_for_edit()?;
        let expression =
            super::input::parse_value(text, Dimension::Length, &snapshot.graph.parameters)?;
        let values = snapshot
            .graph
            .parameters
            .evaluate()
            .map_err(|e| SketchEditError(e.to_string()))?;
        let radius = expression
            .evaluate(&values)
            .map_err(|e| SketchEditError(e.to_string()))?;
        self.sketch_preview_fillet(Length(radius.value()))
    }

    /// Completes the active construction with millimetre values or typed formulas.
    /// # Errors
    /// Reports missing picks or malformed/degenerate values without replacing earlier picks.
    pub fn sketch_complete_numeric(
        &mut self,
        first: &str,
        second: &str,
    ) -> Result<bool, SketchEditError> {
        let refusal = self.sketch_field_refusal([first, second]);
        let fields = self.sketch_entry_field_count();
        let before = self.sketch_entity_ids();
        let result = if self.sketch.tool == SketchTool::Fillet {
            self.sketch_preview_fillet_text(first)
                .and_then(|()| self.sketch_commit_preview())
        } else {
            self.sketch_numeric_inner(first, second)
        };
        // A typed arc radius only fixes the next pick; every other entry commits a shape.
        if result.is_ok() && self.sketch.tool != SketchTool::ArcCenter {
            let open = self.sketch.tool == SketchTool::Polyline;
            self.sketch_shape_committed(&before, open);
        }
        // The box marks the field at fault and says why; values refused only together mark
        // the field of a one-field box (UX notes revision 2 review, change 5).
        let result = result.map_err(|error| {
            let (field, message) = refusal.map_or_else(
                || ((fields == 1).then_some(0), error.0),
                |(field, message)| (Some(field), message),
            );
            self.sketch.entry_error = Some(super::state::EntryError {
                field,
                message: message.clone(),
            });
            SketchEditError(message)
        });
        if result.is_ok() {
            self.sketch.entry_error = None;
        }
        self.sketch.error = result.as_ref().err().map(ToString::to_string);
        result
    }
    fn sketch_numeric_inner(&mut self, first: &str, second: &str) -> Result<bool, SketchEditError> {
        if self.sketch.tool == SketchTool::ArcCenter {
            return self.sketch_arc_radius_text(first);
        }
        if self.sketch.tool == SketchTool::Polyline {
            return self.sketch_numeric_line(first, second);
        }
        let graph = self.sketch_intent_for_edit()?;
        let number = |text| {
            let expression = super::input::parse_value(text, Dimension::Length, &graph.parameters)?;
            expression
                .evaluate(
                    &graph
                        .parameters
                        .evaluate()
                        .map_err(|e| SketchEditError(e.to_string()))?,
                )
                .map(curvyo_sketch_core::Quantity::value)
                .map_err(|e| SketchEditError(e.to_string()))
        };
        let first = number(first)?;
        let anchor = self
            .sketch_live_picks()
            .first()
            .copied()
            .or(self.sketch.hover);
        let command = match self.sketch.tool {
            SketchTool::Point => Construction::Point(Position::mm(first, number(second)?)),
            SketchTool::Circle => Construction::Circle {
                center: anchor.ok_or_else(missing)?,
                radius: Length(first),
            },
            SketchTool::Rectangle => {
                let start = anchor.ok_or_else(missing)?;
                Construction::Rectangle(
                    start,
                    Position::mm(start.x.0 + first, start.y.0 + number(second)?),
                )
            }
            SketchTool::Ellipse => {
                let center = anchor.ok_or_else(missing)?;
                let direction = self
                    .sketch
                    .picks
                    .get(1)
                    .map_or(0.0, |p| (p.y.0 - center.y.0).atan2(p.x.0 - center.x.0));
                Construction::Ellipse {
                    center,
                    axis_end: Position::mm(
                        center.x.0 + first * direction.cos(),
                        center.y.0 + first * direction.sin(),
                    ),
                    second_axis: Length(number(second)?),
                }
            }
            SketchTool::Slot => {
                let [start, end, ..] = self.sketch.picks.as_slice() else {
                    return Err(missing());
                };
                Construction::Slot {
                    start: *start,
                    end: *end,
                    width: Length(first),
                }
            }
            _ => {
                return Err(SketchEditError(
                    "This variant completes with canvas picks".into(),
                ));
            }
        };
        let prefix = self.minter.mint().to_hex();
        let relations = self.sketch_typed_relations(&prefix, &command, None);
        // Picks captured on endpoints keep their identities (AC36).
        let mut picks = self.sketch.picks.clone();
        if picks.is_empty() {
            picks.extend(anchor);
        }
        let joins = self.sketch_joins(&picks);
        let result = self.sketch_construct_related(&prefix, &command, &relations, &joins)?;
        self.sketch.cancel();
        Ok(result)
    }

    /// Converts the active original to ordinary path records as one explicit history step.
    /// # Errors
    /// Refuses missing/invalid geometry or a failed document conversion.
    pub fn sketch_convert_to_path(&mut self) -> Result<(), SketchEditError> {
        let id = self.recording(|this| {
            let snapshot = this.sketch_snapshot_for_edit()?;
            let ids = snapshot
                .resolved
                .all_anchors()
                .map(|_| this.minter.mint())
                .collect::<Vec<_>>();
            this.document.convert_sketch_to_path(snapshot.id, &ids)?;
            Ok::<_, SketchEditError>(snapshot.id)
        })?;
        self.set_workbench(super::Workbench::Vector);
        self.set_tool(crate::Tool::Select);
        self.selection.select_single(id);
        Ok(())
    }
}
fn missing() -> SketchEditError {
    SketchEditError("Pick the construction anchor first".into())
}
