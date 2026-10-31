//! Centre-arc radius entry changes only the unconfirmed next pick.
use super::{Session, SketchTool};
use curvyo_document_core::SketchEditError;
use curvyo_sketch_core::{Dimension, Length, Position};
impl Session {
    pub(super) fn project_sketch_arc_hover(&mut self) {
        // A captured end stays on its endpoint; the arc takes that identity (AC36).
        if self.sketch.tool != SketchTool::ArcCenter || self.sketch.capture.is_some() {
            return;
        }
        let (Some(center), Some(point)) = (self.sketch.picks.first(), self.sketch.hover) else {
            return;
        };
        let radius = self
            .sketch
            .picks
            .get(1)
            .map(|start| center.distance(*start))
            .or(self.sketch.arc_radius);
        let Some(radius) = radius else {
            return;
        };
        let distance = center.distance(point).0;
        if distance <= 1e-9 {
            return;
        }
        self.sketch.hover = Some(Position::mm(
            center.x.0 + (point.x.0 - center.x.0) * radius.0 / distance,
            center.y.0 + (point.y.0 - center.y.0) * radius.0 / distance,
        ));
    }
    pub(super) fn sketch_arc_pick_annotations(&self) -> Vec<super::inspection::SketchAnnotation> {
        if !matches!(self.sketch.tool, SketchTool::Arc | SketchTool::ArcCenter) {
            return Vec::new();
        }
        let Some(placement) = self.sketch_placement() else {
            return Vec::new();
        };
        let roles = if self.sketch.tool == SketchTool::Arc {
            ["Start", "Through", "End"]
        } else {
            ["Centre", "Start", "End"]
        };
        self.sketch
            .picks
            .iter()
            .enumerate()
            .map(|(index, p)| {
                let screen = self.view().document_to_screen(
                    placement.apply_point(curvyo_document_core::Point::new(p.x.0, p.y.0)),
                );
                super::inspection::SketchAnnotation {
                    id: format!("pending-pick-{index}"),
                    x: screen.0,
                    y: screen.1 - 18.0,
                    text: roles[index.min(2)].into(),
                    dimension: false,
                    selected: false,
                    conflicting: false,
                    preview: true,
                    ..Default::default()
                }
            })
            .collect()
    }

    pub(super) fn sketch_arc_radius_text(&mut self, text: &str) -> Result<bool, SketchEditError> {
        if self.sketch.picks.len() != 1 {
            return Err(SketchEditError(
                "Pick the centre before setting radius; an accepted start stays fixed".into(),
            ));
        }
        let graph = self.sketch_intent_for_edit()?;
        let value = super::input::parse_value(text, Dimension::Length, &graph.parameters)?
            .evaluate(
                &graph
                    .parameters
                    .evaluate()
                    .map_err(|e| SketchEditError(e.to_string()))?,
            )
            .map_err(|e| SketchEditError(e.to_string()))?;
        if value.value() <= 1e-9 {
            return Err(SketchEditError("Radius must be positive".into()));
        }
        self.sketch.arc_radius = Some(Length(value.value()));
        self.project_sketch_arc_hover();
        self.refresh_sketch_construction_preview();
        Ok(true)
    }
}
