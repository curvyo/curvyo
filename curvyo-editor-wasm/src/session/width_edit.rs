//! Canvas gestures for editing one selected path's width profile (`specs/0110` criterion 4).
//!
//! The width points show as diamonds on the centre line; a click on the line adds one, a drag
//! along the line moves it, a drag across sets the width with the stroke edge under the pointer.
//! Every gesture changes the document once, on release.

use curvyo_document_core::{AnchorId, Length, NodeId, PathSnapshot, Point, WidthPoint};
use curvyo_geometry_core::PathMeasure;
use curvyo_ui_core::{WidthGesture, WidthGesturePart, width_hit, width_position_at};

use super::Session;

/// How far a drag moves, screen pixels, before its first movement chooses along or across.
const LATCH_PX: f64 = 3.0;

/// One drag in flight and the path as it was at the press.
#[derive(Debug)]
struct WidthDrag {
    gesture: WidthGesture,
    /// The press added a new point: the release writes it even without movement.
    added: bool,
    source: Option<PathSnapshot>,
    /// The latest pointer, where the live readout sits.
    pointer: Point,
}

/// Ephemeral state; the document is changed only on release.
#[derive(Debug)]
pub(super) struct WidthEditState {
    path: NodeId,
    selected: Option<AnchorId>,
    /// Where a click on the centre line would add a point (the ghost diamond).
    ghost: Option<Point>,
    drag: Option<WidthDrag>,
    notice: Option<String>,
}

impl WidthEditState {
    /// Drops the selected point and any gesture after the profile was replaced as a whole.
    pub(super) fn forget_selection(&mut self) {
        self.selected = None;
        self.drag = None;
    }
}

/// `id` as drawn: a Spiro path evaluated, so the width points sit on the visible curve.
fn width_path(
    document: &curvyo_document_core::Document,
    cache: &std::cell::RefCell<super::spiro_cache::SpiroCache>,
    id: NodeId,
) -> Option<PathSnapshot> {
    let path = document.path(id)?;
    if path.curve_kind == curvyo_document_core::CurveKind::Spiro {
        cache.borrow_mut().ready(document, &path, 0.01)
    } else {
        Some(path)
    }
}

fn measure(path: &PathSnapshot) -> Option<PathMeasure> {
    let path = curvyo_ui_core::evaluate_path(path).ok()?;
    let anchors: Vec<_> = path
        .anchors
        .iter()
        .map(|a| (a.point, a.handle_in, a.handle_out))
        .collect();
    Some(PathMeasure::new(&anchors, path.closed))
}

impl Session {
    /// Takes the current notice about variable-stroke artwork.
    pub fn take_unsupported_width_notice(&self) -> String {
        self.unsupported_width_notice
            .borrow_mut()
            .take()
            .unwrap_or_default()
    }

    /// Starts editing the width of the one selected ordinary path.
    pub fn begin_width_edit(&mut self) -> bool {
        let refuse = |message: &str| {
            *self.unsupported_width_notice.borrow_mut() = Some(message.to_owned());
            false
        };
        let [id] = self.selection.ids() else {
            return refuse("Select one path to edit its width.");
        };
        let Some(_) = self.document.path(*id) else {
            return refuse(
                "Width editing requires an ordinary path; convert this object to a path first.",
            );
        };
        let Some(path) = width_path(&self.document, &self.spiro_cache, *id) else {
            return refuse(super::spiro_cache::SPIRO_UNSOLVED_NOTICE);
        };
        if path.is_compound() {
            return refuse("Width editing is not available for compound paths.");
        }
        if self.units().is_inert(*id) {
            return refuse("Unlock the path before editing its width.");
        }
        if !path.style.stroke.enabled || path.style.stroke.width.as_mm() <= 0.0 {
            return refuse("Give the path a nonzero stroke before editing its width.");
        }
        if self.document.is_shared() || self.document.workspace_share().is_some() {
            return refuse("Width profiles cannot be edited while the document is shared.");
        }
        if measure(&path).is_none_or(|measured| measured.length().as_mm() <= 1e-9) {
            return refuse("This path has no usable length for width editing.");
        }
        self.width_edit = Some(WidthEditState {
            path: *id,
            selected: None,
            ghost: None,
            drag: None,
            notice: None,
        });
        true
    }

    /// Whether width editing is active.
    #[must_use]
    pub const fn width_edit_active(&self) -> bool {
        self.width_edit.is_some()
    }

    /// Leaves width editing (Done) and discards any uncommitted drag.
    pub fn end_width_edit(&mut self) {
        self.width_edit = None;
    }

    /// Takes the latest width edit error, or an empty string when none occurred.
    pub fn take_width_edit_notice(&mut self) -> String {
        self.width_edit
            .as_mut()
            .and_then(|state| state.notice.take())
            .unwrap_or_default()
    }

    /// The full width of the selected width point in millimetres, for the numeric field.
    #[must_use]
    pub fn width_point_mm(&self) -> Option<f64> {
        let state = self.width_edit.as_ref()?;
        let id = state.selected?;
        let point = self
            .width_points_for_display()?
            .1
            .into_iter()
            .find(|point| point.id == id)?;
        let base = self.document.path(state.path)?.style.stroke.width.as_mm();
        Some(base * point.multiplier)
    }

    /// Sets the full width of the selected width point to `mm` millimetres, as one undo step.
    /// `false` when no point is selected or the value is not a width.
    pub fn set_width_point_mm(&mut self, mm: f64) -> bool {
        let Some(state) = &self.width_edit else {
            return false;
        };
        let (id, path) = (state.selected, state.path);
        let (Some(id), Some(source)) = (id, self.document.path(path)) else {
            return false;
        };
        let base = source.style.stroke.width.as_mm();
        let Some(point) = source.width_points.iter().find(|p| p.id == id).copied() else {
            return false;
        };
        let mut gesture = WidthGesture::existing(point, Point::new(0.0, 0.0));
        if base.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
            || gesture.set_multiplier(mm / base).is_none()
        {
            return false;
        }
        if !self.width_point_has_outline(path, gesture.preview()) {
            return false;
        }
        let result = self.recording(|this| Some(gesture.commit(&this.document, path)));
        self.note_width_result(result)
    }

    /// Writes the notice of a refused commit; whether the commit was written.
    fn note_width_result(
        &mut self,
        result: Option<Result<(), curvyo_document_core::WidthPointError>>,
    ) -> bool {
        match result {
            Some(Ok(())) => true,
            Some(Err(error)) => {
                if let Some(state) = &mut self.width_edit {
                    state.notice = Some(error.to_string());
                }
                false
            }
            None => false,
        }
    }

    /// Resets the current profile as one undoable change.
    pub fn reset_width_edit(&mut self) -> bool {
        let Some(state) = &self.width_edit else {
            return false;
        };
        let id = state.path;
        // `None`: a move-only selection refused the edit (`specs/0106` criterion 9).
        match self.recording(|this| Some(this.document.reset_width_profile(id))) {
            None => return false,
            Some(Err(error)) => {
                *self.unsupported_width_notice.borrow_mut() = Some(error.to_string());
                return false;
            }
            Some(Ok(())) => {}
        }
        if let Some(state) = &mut self.width_edit {
            state.forget_selection();
        }
        true
    }

    /// Delete in width editing removes the selected point as one step; `false` outside it.
    pub(super) fn width_delete_selected(&mut self) -> bool {
        let Some(state) = &mut self.width_edit else {
            return false;
        };
        state.drag = None;
        let (path, point) = (state.path, state.selected.take());
        if let Some(point) = point {
            let result = self.recording(|this| Some(this.document.remove_width_point(path, point)));
            self.note_width_result(result);
        }
        true
    }

    /// Escape: a drag in flight is cancelled; otherwise width editing ends.
    pub(super) fn width_escape(&mut self) -> bool {
        let Some(state) = &mut self.width_edit else {
            return false;
        };
        if state.drag.take().is_none() {
            self.width_edit = None;
        }
        true
    }

    pub(super) fn width_pointer_down(&mut self, pointer: Point) -> bool {
        let Some(mut state) = self.width_edit.take() else {
            return false;
        };
        state.ghost = None;
        if let Some(path) = width_path(&self.document, &self.spiro_cache, state.path)
            && let Some(measure) = measure(&path)
        {
            let tolerance = self.point_tolerance_as_length();
            if let Some(id) = width_hit(&measure, &path.width_points, pointer, tolerance) {
                if let Some(point) = path.width_points.iter().find(|p| p.id == id) {
                    state.selected = Some(id);
                    state.drag = Some(WidthDrag {
                        gesture: WidthGesture::existing(*point, pointer),
                        added: false,
                        source: self.document.path(state.path),
                        pointer,
                    });
                }
            } else if self.on_centre_line(&measure, pointer) {
                let id = self.minter.mint();
                if let Some(gesture) = WidthGesture::add(&measure, &path.width_points, id, pointer)
                {
                    state.selected = Some(id);
                    state.drag = Some(WidthDrag {
                        gesture,
                        added: true,
                        source: self.document.path(state.path),
                        pointer,
                    });
                }
            } else {
                state.selected = None;
            }
        }
        self.width_edit = Some(state);
        true
    }

    /// Whether `pointer` lies on the centre line within the object tolerance.
    fn on_centre_line(&self, measure: &PathMeasure, pointer: Point) -> bool {
        width_position_at(measure, pointer).is_some_and(|position| {
            let centre = measure
                .at(Length::from_mm(measure.length().as_mm() * position))
                .point;
            centre.vector_to(pointer).length() <= self.object_tolerance().as_mm()
        })
    }

    pub(super) fn width_pointer_move(&mut self, pointer: Point) -> bool {
        self.cancel_stale_width();
        let Some(mut state) = self.width_edit.take() else {
            return false;
        };
        let hit = self.point_tolerance_as_length();
        // A drag chooses its axis once it has moved a few screen pixels.
        let threshold = Length::from_mm(LATCH_PX / self.view().scale());
        if let Some(path) = width_path(&self.document, &self.spiro_cache, state.path)
            && let Some(measure) = measure(&path)
        {
            if let Some(drag) = &mut state.drag {
                drag.pointer = pointer;
                let _ = drag
                    .gesture
                    .drag(&measure, path.style.stroke.width, pointer, threshold);
                state.ghost = None;
            } else {
                // The ghost diamond: where a click on the line would add a point.
                let over_point = width_hit(&measure, &path.width_points, pointer, hit).is_some();
                state.ghost = (!over_point && self.on_centre_line(&measure, pointer))
                    .then(|| width_position_at(&measure, pointer))
                    .flatten()
                    .map(|position| {
                        measure
                            .at(Length::from_mm(measure.length().as_mm() * position))
                            .point
                    });
            }
        }
        self.width_edit = Some(state);
        true
    }

    pub(super) fn width_pointer_up(&mut self, pointer: Point) -> bool {
        if !self.width_pointer_move(pointer) {
            return false;
        }
        let Some(state) = &mut self.width_edit else {
            return true;
        };
        let Some(drag) = state.drag.take() else {
            return true;
        };
        if !drag.added && drag.gesture.axis().is_none() {
            return true;
        }
        let path = state.path;
        if !self.width_point_has_outline(path, drag.gesture.preview()) {
            return true;
        }
        let result = self.recording(|this| Some(drag.gesture.commit(&this.document, path)));
        self.note_width_result(result);
        true
    }

    /// Whether `path` with `point` set still has an outline; a width out of range is refused with
    /// a notice and changes nothing (ADR 0053 amendment item 5).
    fn width_point_has_outline(&mut self, path: NodeId, point: WidthPoint) -> bool {
        let Some(mut candidate) = self.document.path(path) else {
            return false;
        };
        candidate.width_points.retain(|p| p.id != point.id);
        candidate.width_points.push(point);
        if self.width_artwork(&candidate).is_some() {
            return true;
        }
        if let Some(state) = &mut self.width_edit {
            state.notice = Some("This width has no outline: it reaches out of range.".to_owned());
        }
        false
    }

    /// Cancels a drag whose path changed under it (a remote edit, an undo).
    fn cancel_stale_width(&mut self) {
        let current = self
            .width_edit
            .as_ref()
            .and_then(|state| self.document.path(state.path));
        let Some(state) = &mut self.width_edit else {
            return;
        };
        if state
            .drag
            .as_ref()
            .is_some_and(|drag| drag.source != current)
        {
            state.drag = None;
            state.notice = Some("The path changed; the width edit was cancelled.".to_owned());
        }
    }

    pub(super) fn width_cancel_drag(&mut self) -> bool {
        let Some(state) = &mut self.width_edit else {
            return false;
        };
        state.drag = None;
        true
    }

    /// The path in width editing and its points as shown: the drag's point in its preview.
    pub(super) fn width_points_for_display(&self) -> Option<(NodeId, Vec<WidthPoint>)> {
        let state = self.width_edit.as_ref()?;
        let mut points = self.document.width_points(state.path).ok()?;
        if let Some(drag) = &state.drag {
            let point = drag.gesture.preview();
            points.retain(|p| p.id != point.id);
            points.push(point);
        }
        Some((state.path, points))
    }

    /// "Width 3.25 mm" at the pointer while a drag sets a width, in the display unit.
    pub(super) fn width_live_readout(&self) -> Option<super::shapes::LiveReadout> {
        let state = self.width_edit.as_ref()?;
        let drag = state.drag.as_ref()?;
        if drag.gesture.axis() != Some(WidthGesturePart::Width) {
            return None;
        }
        let path = width_path(&self.document, &self.spiro_cache, state.path)?;
        let width =
            Length::from_mm(path.style.stroke.width.as_mm() * drag.gesture.preview().multiplier);
        let unit = self.display_unit();
        Some(super::shapes::LiveReadout {
            text: format!(
                "Width {} {}",
                curvyo_ui_core::format_field_length(width, unit),
                unit.symbol()
            ),
            anchor: drag.pointer,
        })
    }

    /// The diamonds on the centre line, the selected one filled, and the ghost under the pointer.
    pub(super) fn width_handle_overlay(&self) -> curvyo_render_core::DrawList {
        let Some((id, points)) = self.width_points_for_display() else {
            return curvyo_render_core::DrawList::default();
        };
        let Some(path) = width_path(&self.document, &self.spiro_cache, id) else {
            return curvyo_render_core::DrawList::default();
        };
        let Some(measure) = measure(&path) else {
            return curvyo_render_core::DrawList::default();
        };
        let state = self.width_edit.as_ref();
        let selected = state.and_then(|state| state.selected);
        let mut diamonds: Vec<_> = points
            .iter()
            .filter_map(|point| {
                let at = curvyo_ui_core::width_handle(
                    &measure,
                    point.position,
                    Length::from_mm(0.0),
                    0.0,
                )?;
                Some(curvyo_render_core::WidthDiamond {
                    at,
                    selected: selected == Some(point.id),
                    ghost: false,
                })
            })
            .collect();
        if let Some(at) = state.and_then(|state| state.ghost) {
            diamonds.push(curvyo_render_core::WidthDiamond {
                at,
                selected: false,
                ghost: true,
            });
        }
        curvyo_render_core::build_width_diamonds(&diamonds, self.view())
    }

    /// `objects` with the path in width editing drawn with the drag's preview profile.
    pub(super) fn width_preview_artwork(
        &self,
        objects: &[curvyo_document_core::ObjectSnapshot],
    ) -> Option<Vec<curvyo_document_core::ObjectSnapshot>> {
        self.width_edit.as_ref()?.drag.as_ref()?;
        let (id, points) = self.width_points_for_display()?;
        Some(
            objects
                .iter()
                .map(|object| match object {
                    curvyo_document_core::ObjectSnapshot::Path(path) if path.id == id => {
                        let mut preview = path.clone();
                        preview.width_points.clone_from(&points);
                        curvyo_document_core::ObjectSnapshot::Path(preview)
                    }
                    _ => object.clone(),
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests;
