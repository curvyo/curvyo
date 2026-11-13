//! Width points under the pointer in width editing: their hit area, the typed width a
//! double-click opens and the mouse wheel over them (`specs/0115` criteria 5 to 7).

use curvyo_document_core::{Length, Point};
use curvyo_ui_core::{WidthGesture, width_hit};

use super::{Session, WidthDrag, measure, width_path};

/// The full width a wheel notch adds or takes, millimetres.
const WHEEL_STEP_MM: f64 = 0.5;
/// The full width a wheel notch changes with Shift, millimetres.
const WHEEL_FINE_STEP_MM: f64 = 0.1;
/// How far beyond a diamond's corners it still takes the pointer, screen pixels.
const HIT_MARGIN_PX: f64 = 3.0;

impl Session {
    /// The hit radius of a width diamond: its half size and a margin, 11 px (`specs/0115`
    /// criterion 5).
    pub(in crate::session) fn width_hit_tolerance(&self) -> Length {
        Length::from_mm(
            (curvyo_render_core::WIDTH_DIAMOND_PX / 2.0 + HIT_MARGIN_PX) / self.view().scale(),
        )
    }

    /// The width point under `pointer` in width editing.
    fn width_point_at(&self, pointer: Point) -> Option<curvyo_document_core::WidthPoint> {
        let state = self.width_edit.as_ref()?;
        let path = width_path(&self.document, &self.spiro_cache, state.path)?;
        let measure = measure(&path)?;
        let id = width_hit(
            &measure,
            &path.width_points,
            pointer,
            self.width_hit_tolerance(),
        )?;
        path.width_points
            .iter()
            .find(|point| point.id == id)
            .copied()
    }

    /// Where the selected width point's diamond sits, canvas px, live during a gesture: the
    /// typed width and the wheel readout open beside it (UX review 2026-11-12).
    #[must_use]
    pub fn width_point_screen(&self) -> Option<(f64, f64)> {
        let state = self.width_edit.as_ref()?;
        let id = state.selected?;
        let point = self
            .width_points_for_display()?
            .1
            .into_iter()
            .find(|point| point.id == id)?;
        let path = width_path(&self.document, &self.spiro_cache, state.path)?;
        let at = curvyo_ui_core::width_handle(
            &measure(&path)?,
            point.position,
            Length::from_mm(0.0),
            0.0,
        )?;
        Some(self.view().document_to_screen(at))
    }

    /// Whether `pointer` is over a width point in width editing: the host's wheel goes to
    /// [`Session::width_wheel`] there instead of zooming.
    #[must_use]
    pub fn width_point_under(&self, pointer: Point) -> bool {
        self.width_point_at(pointer).is_some()
    }

    /// Selects the width point under `pointer` for the typed width; `false` when there is none.
    pub(in crate::session) fn width_select_under(&mut self, pointer: Point) -> bool {
        let Some(point) = self.width_point_at(pointer) else {
            return false;
        };
        self.end_width_wheel();
        if let Some(state) = &mut self.width_edit {
            state.drag = None;
            state.selected = Some(point.id);
        }
        true
    }

    /// The wheel turned `notches` (positive widens) over `pointer`: the width point there, or
    /// the one a burst already changes, gets 0.5 mm of full width per notch (0.1 mm with
    /// `fine`), never below zero, shown live. The burst is one undo step, written by
    /// [`Session::end_width_wheel`]. `false` when the pointer is over no width point: the host
    /// zooms as usual.
    pub fn width_wheel(&mut self, pointer: Point, notches: f64, fine: bool) -> bool {
        self.cancel_stale_width();
        if !notches.is_finite() {
            return false;
        }
        let under = self.width_point_at(pointer);
        let Some(path) = self.width_edit.as_ref().map(|state| state.path) else {
            return false;
        };
        let burst = self
            .width_edit
            .as_ref()
            .and_then(|state| state.drag.as_ref())
            .filter(|drag| drag.wheel)
            .map(|drag| drag.gesture.preview());
        let point = match (burst, under) {
            (Some(burst), Some(under)) if burst.id != under.id => {
                self.end_width_wheel();
                under
            }
            (Some(burst), _) => burst,
            (None, Some(under)) => under,
            (None, None) => return false,
        };
        let Some(source) = self.document.path(path) else {
            return false;
        };
        let base = source.style.stroke.width.as_mm();
        if !base.is_finite() || base <= 0.0 {
            return true;
        }
        let step = if fine {
            WHEEL_FINE_STEP_MM
        } else {
            WHEEL_STEP_MM
        };
        let width = notches.mul_add(step, base * point.multiplier).max(0.0);
        let mut gesture = WidthGesture::existing(point, pointer);
        if gesture.set_multiplier(width / base).is_none() {
            return true;
        }
        if let Some(state) = &mut self.width_edit {
            state.selected = Some(point.id);
            state.ghost = None;
            let source = match state.drag.take() {
                Some(drag) if drag.wheel => drag.source,
                _ => Some(source),
            };
            state.drag = Some(WidthDrag {
                gesture,
                added: false,
                wheel: true,
                source,
                pointer,
            });
        }
        true
    }

    /// Ends a wheel burst: its width is written as one undo step. `false` when no burst was in
    /// flight or the width was refused.
    pub fn end_width_wheel(&mut self) -> bool {
        self.cancel_stale_width();
        let Some(state) = &mut self.width_edit else {
            return false;
        };
        if !state.drag.as_ref().is_some_and(|drag| drag.wheel) {
            return false;
        }
        let Some(drag) = state.drag.take() else {
            return false;
        };
        let path = state.path;
        if !self.width_point_has_outline(path, drag.gesture.preview()) {
            return false;
        }
        let result = self.recording(|this| Some(drag.gesture.commit(&this.document, path)));
        self.note_width_result(result)
    }
}
