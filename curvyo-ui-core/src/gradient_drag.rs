//! What a drag of a gradient handle makes of the gradient (`specs/0124-gradient-fill`
//! criterion 6, UX notes §6.4): an end moves alone, Shift locks its angle about the other end
//! to the angle stops, Ctrl moves the whole line, the Radial centre always moves the whole
//! circle, an inner stop slides along the line.

use curvyo_document_core::{Angle, GradientBox, GradientGeometry, Point, Vec2};

use crate::angle_snap::snap_angle;
use crate::gradient_handles::{GradientHandle, GradientLayout, box_fractions};

/// The state a handle drag starts from.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientDragStart {
    /// The handle pressed.
    pub handle: GradientHandle,
    /// The geometry at the press.
    pub geometry: GradientGeometry,
    /// The handles at the press.
    pub layout: GradientLayout,
    /// The press point, document millimetres.
    pub press: Point,
}

/// `p` moved by `d`.
fn moved(p: Point, d: Vec2) -> Point {
    Point::new(p.x + d.x, p.y + d.y)
}

/// `free` turned about `pivot` to the nearest angle stop, its distance kept. The angle is the
/// page's (0 right, counter-clockwise with Y down), as the readout shows it.
fn snapped_about(pivot: Point, free: Point) -> Point {
    let (dx, dy) = (free.x - pivot.x, free.y - pivot.y);
    let length = dx.hypot(dy);
    if length.is_nan() || length <= 0.0 {
        return free;
    }
    let angle = snap_angle(Angle::from_radians((-dy).atan2(dx))).as_radians();
    let (sin, cos) = angle.sin_cos();
    Point::new(pivot.x + cos * length, pivot.y - sin * length)
}

impl GradientDragStart {
    /// The geometry with the pointer at `pointer` in `frame`; `None` for an inner stop (see
    /// [`GradientDragStart::dragged_offset`]) or a point the box cannot express.
    #[must_use]
    pub fn dragged_geometry(
        &self,
        frame: &GradientBox,
        pointer: Point,
        shift: bool,
        ctrl: bool,
    ) -> Option<GradientGeometry> {
        let d = Vec2::new(pointer.x - self.press.x, pointer.y - self.press.y);
        let (from, to) = (self.layout.from, self.layout.to);
        let whole = ctrl || self.handle == GradientHandle::Centre;
        let (new_from, new_to) = match self.handle {
            GradientHandle::Stop(_) => return None,
            _ if whole => (moved(from, d), moved(to, d)),
            GradientHandle::Start => {
                let free = moved(from, d);
                (if shift { snapped_about(to, free) } else { free }, to)
            }
            GradientHandle::End | GradientHandle::Radius | GradientHandle::Centre => {
                let free = moved(to, d);
                (
                    from,
                    if shift {
                        snapped_about(from, free)
                    } else {
                        free
                    },
                )
            }
        };
        let (a, b) = (
            box_fractions(frame, new_from)?,
            box_fractions(frame, new_to)?,
        );
        Some(match self.geometry {
            GradientGeometry::Linear { .. } => GradientGeometry::Linear { start: a, end: b },
            GradientGeometry::Radial { ratio, .. } => GradientGeometry::Radial {
                centre: a,
                radius: b,
                ratio,
            },
        })
    }

    /// The offset of the dragged inner stop with the pointer at `pointer`: its point at the
    /// press moved with the pointer and projected on the line, `0..=1` (the document clamps it
    /// between its neighbours); `None` for an end.
    #[must_use]
    pub fn dragged_offset(&self, pointer: Point) -> Option<f64> {
        let GradientHandle::Stop(index) = self.handle else {
            return None;
        };
        let at = self
            .layout
            .handles
            .iter()
            .find(|(handle, _)| *handle == GradientHandle::Stop(index))?
            .1;
        let d = Vec2::new(pointer.x - self.press.x, pointer.y - self.press.y);
        self.layout.offset_along(moved(at, d))
    }
}
