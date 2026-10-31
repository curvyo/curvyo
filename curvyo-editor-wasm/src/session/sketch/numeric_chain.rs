//! Typed chain dimensions retain the explicitly requested endpoint direction.
use super::Session;
use curvyo_document_core::SketchEditError;
use curvyo_sketch_core::{
    Construction, ContinuationMode, Dimension, Length, Position, endpoint_tangent,
};
impl Session {
    pub(super) fn sketch_numeric_line(
        &mut self,
        length: &str,
        angle: &str,
    ) -> Result<bool, SketchEditError> {
        let graph = self.sketch_intent_for_edit()?;
        let values = graph
            .parameters
            .evaluate()
            .map_err(|e| SketchEditError(e.to_string()))?;
        let value = |text, unit| {
            super::input::parse_value(text, unit, &graph.parameters)?
                .evaluate(&values)
                .map(curvyo_sketch_core::Quantity::value)
                .map_err(|e| SketchEditError(e.to_string()))
        };
        let length = value(length, Dimension::Length)?;
        if length <= 1e-9 {
            return Err(SketchEditError("Length must be positive".into()));
        }
        let start = self
            .sketch
            .picks
            .first()
            .copied()
            .or(self.sketch.hover)
            .ok_or_else(missing)?;
        let mode = self.sketch.mode;
        let direction = if mode == ContinuationMode::FreeLine {
            let angle = value(angle, Dimension::Angle)?;
            (angle.cos(), angle.sin())
        } else {
            let tangent = match self.sketch.chain.as_ref() {
                // A free first segment takes its direction from the sketch X axis.
                None => super::first_segment::START_DIRECTION,
                Some((_, last)) => {
                    let previous = self.sketch.chain_curve.as_ref().ok_or_else(missing)?;
                    let solved = self.sketch_creation_base()?;
                    endpoint_tangent(&solved, previous, last)
                        .map_err(|e| SketchEditError(e.to_string()))?
                }
            };
            super::first_segment::mode_direction(mode, tangent)
        };
        let end = Position::mm(
            start.x.0 + length * direction.0,
            start.y.0 + length * direction.1,
        );
        let end = if super::first_segment::is_arc(mode) {
            curvyo_sketch_core::arc_radius_endpoint(
                start,
                direction,
                Length(length),
                self.sketch.hover.ok_or_else(missing)?,
            )
            .map_err(|e| SketchEditError(e.to_string()))?
        } else {
            end
        };
        let prefix = self.minter.mint().to_hex();
        let chain = self.sketch.chain.clone();
        let command = if let Some((_, last)) = &chain {
            Construction::Continue {
                previous: self.sketch.chain_curve.clone().ok_or_else(missing)?,
                start: last.clone(),
                end: format!("{prefix}:end").into(),
                position: end,
                mode,
            }
        } else {
            Construction::Polyline(vec![start, end])
        };
        let step = self.sketch_polyline_step(&prefix, &[start], command, end);
        if step.degenerate {
            return Err(SketchEditError(super::creation::DEGENERATE.into()));
        }
        let changed =
            self.sketch_construct_related(&prefix, &step.command, step.relation.as_ref())?;
        self.sketch.cancel();
        self.continue_sketch_chain(&prefix, &step.command, chain.as_ref(), step.ends);
        Ok(changed)
    }
}
fn missing() -> SketchEditError {
    SketchEditError("Pick the construction anchor first".into())
}
