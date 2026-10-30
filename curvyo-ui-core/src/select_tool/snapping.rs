//! Moving resize features for constrained snapping.
use super::{SelectDrag, SelectTool};
use crate::EditHandle;
use curvyo_document_core::{Point, Vec2};
impl SelectTool {
    /// The dragged resize feature and its permitted direction, excluding fixed edges.
    #[must_use]
    pub fn resize_snap_point(
        &self,
        pointer: Point,
        shift: bool,
        ctrl: bool,
    ) -> Option<(Point, Option<Vec2>)> {
        let (start, handle, resized) = match &self.drag {
            SelectDrag::Transforming(drag) if drag.origin.is_active_at(pointer) => (
                drag.start_box,
                drag.handle,
                drag.resized_box_at(pointer, shift, ctrl)?,
            ),
            SelectDrag::GroupTransforming(drag) if drag.origin.is_active_at(pointer) => (
                drag.start_box,
                drag.handle,
                drag.resized_box_at(pointer, shift, ctrl)?,
            ),
            _ => return None,
        };
        let EditHandle::Resize(direction) = handle else {
            return None;
        };
        let unit = direction.unit_vector();
        let axis = |lo: f64, hi: f64, sign: f64, factor: f64| {
            if sign == 0.0 {
                f64::midpoint(lo, hi)
            } else if sign * factor < 0.0 {
                lo
            } else {
                hi
            }
        };
        let local = Point::new(
            axis(resized.min.x, resized.max.x, unit.x, resized.sx),
            axis(resized.min.y, resized.max.y, unit.y, resized.sy),
        );
        let constrained = ctrl || unit.x == 0.0 || unit.y == 0.0;
        let direction = constrained.then(|| {
            let d = if ctrl {
                Vec2::new(
                    unit.x * resized.sx.signum() * (start.max.x - start.min.x),
                    unit.y * resized.sy.signum() * (start.max.y - start.min.y),
                )
            } else {
                unit
            };
            start
                .to_document(Point::new(0.0, 0.0))
                .vector_to(start.to_document(Point::new(d.x, d.y)))
        });
        Some((start.to_document(local), direction))
    }
}
