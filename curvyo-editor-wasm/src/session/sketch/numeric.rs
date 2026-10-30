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
        let result = if self.sketch.tool == SketchTool::Fillet {
            self.sketch_preview_fillet_text(first)
                .and_then(|()| self.sketch_commit_preview())
        } else {
            self.sketch_numeric_inner(first, second)
        };
        self.sketch.error = result.as_ref().err().map(ToString::to_string);
        result
    }
    fn sketch_numeric_inner(&mut self, first: &str, second: &str) -> Result<bool, SketchEditError> {
        let snapshot = self.sketch_snapshot_for_edit()?;
        let number = |text| {
            let expression =
                super::input::parse_value(text, Dimension::Length, &snapshot.graph.parameters)?;
            expression
                .evaluate(
                    &snapshot
                        .graph
                        .parameters
                        .evaluate()
                        .map_err(|e| SketchEditError(e.to_string()))?,
                )
                .map(curvyo_sketch_core::Quantity::value)
                .map_err(|e| SketchEditError(e.to_string()))
        };
        let first = number(first)?;
        let anchor = self.sketch.picks.first().copied().or(self.sketch.hover);
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
        let result = self.sketch_construct(&command)?;
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
