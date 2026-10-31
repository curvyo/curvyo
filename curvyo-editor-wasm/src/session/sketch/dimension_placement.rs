//! Dimension placement stays ephemeral until its typed relationship is accepted.
use super::Session;
use curvyo_document_core::{Point, SketchSnapshot};
use curvyo_sketch_core::{Expression, Quantity};
impl Session {
    pub(super) fn sketch_place_dimension(&mut self, point: Point) -> bool {
        if self.sketch.constraint_tool.as_deref() != Some("dimension")
            || self.sketch_dimension_action().is_none()
        {
            return false;
        }
        self.sketch.dimension_position = self.sketch_local(point);
        self.button_down = false;
        true
    }
    pub(super) fn sketch_dimension_annotation(
        &self,
        snapshot: &SketchSnapshot,
    ) -> Option<super::inspection::SketchAnnotation> {
        if self.sketch.constraint_tool.as_deref() != Some("dimension") {
            return None;
        }
        let action = self.sketch_dimension_action()?;
        let point = self.sketch.dimension_position.or(self.sketch.hover)?;
        let ids = self.sketch.entities.iter().cloned().collect::<Vec<_>>();
        let value = if action == "angle" {
            Quantity::angle(1.0)
        } else {
            Quantity::length(1.0)
        }
        .ok()?;
        let relation = curvyo_ui_core::sketch_relation(
            action,
            &snapshot.graph,
            &ids,
            Some(&Expression::Literal(value)),
        )
        .ok()?;
        let measured = curvyo_sketch_core::measure(&snapshot.solution, &relation).ok()?;
        let screen = self.view().document_to_screen(
            snapshot
                .placement
                .apply_point(Point::new(point.x.0, point.y.0)),
        );
        let text = if action == "angle" {
            format!("{:.2}°", measured.value().to_degrees())
        } else {
            format!("{:.2} mm", measured.value())
        };
        Some(super::inspection::SketchAnnotation {
            id: "pending-dimension".into(),
            x: screen.0,
            y: screen.1,
            text,
            dimension: true,
            selected: false,
            conflicting: false,
            preview: true,
            ..Default::default()
        })
    }
}
