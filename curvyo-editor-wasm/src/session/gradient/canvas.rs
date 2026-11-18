//! The gradient handles on the canvas (`specs/0124-gradient-fill` criteria 6 and 7, UX notes
//! §6): which shape shows them, the shape handles they cover, a press that activates a stop
//! without moving focus, drags previewed until the release, Escape, the double-click that adds
//! a stop or opens an entry box, and Delete on an inner stop.

use curvyo_document_core::{Point, StopOffset, StyleEdit};
use curvyo_ui_core::{
    GradientDragStart, GradientHandle, GradientLayout, Scope, SelectTool,
    TransformHandleTolerances, YieldedHandles, gradient_layout, takes_gradient,
};

use super::super::{KeyInput, KeyOutcome, Session, Tool};
use super::view::{GradientReadoutJson, angle_text, radius_text, trimmed};
use super::{HandleDrag, OpenEntry, Shaded};

impl Session {
    /// The shape whose handles are drawn and its handles: the first selected shape with a
    /// Gradient fill, as previewed, in the Select tool, not locked, not move-only, not during a
    /// transform drag or a text edit, and large enough on screen (UX notes §6.1).
    pub(super) fn handle_subject(&self) -> Option<(Shaded, GradientLayout)> {
        if self.tool != Tool::Select
            || self.text_editing()
            || (self.select.drag_in_flight() && self.gradient.drag.is_none())
        {
            return None;
        }
        let objects = self.objects();
        // Only the shapes that paint a gradient are looked up by id: a frame over thousands of
        // selected Colour shapes stays a single pass.
        let painted: std::collections::HashMap<_, _> = objects
            .iter()
            .filter(|o| takes_gradient(o) && o.style().fill.painted_gradient().is_some())
            .map(|o| (o.id(), o))
            .collect();
        if painted.is_empty() {
            return None;
        }
        let units = self.units();
        let mut object = (*self.selection.ids().iter().find_map(|id| painted.get(id))?).clone();
        if units.is_inert(object.id()) || self.edit_scope() == Scope::MoveOnly {
            return None;
        }
        if self.style.is_active() {
            self.style.apply_to(std::slice::from_mut(&mut object));
        }
        let shaded = Shaded::of(&object)?;
        let layout = gradient_layout(&shaded.fill, &shaded.frame);
        layout
            .shown_at(self.view().scale())
            .then_some((shaded, layout))
    }

    /// The shape handles a drawn gradient handle covers (criterion 6): neither drawn nor hit.
    pub(in crate::session) fn gradient_yielded(
        &self,
        tolerances: TransformHandleTolerances,
    ) -> YieldedHandles {
        let Some((_, layout)) = self.handle_subject() else {
            return YieldedHandles::NONE;
        };
        let handles =
            SelectTool::transform_handles(&self.objects(), &self.selection, tolerances, true);
        layout.yielded(&handles, self.view().scale())
    }

    /// A press on a gradient handle: its stop becomes active and canvas-selected, and a drag
    /// starts; `false` (nothing taken) anywhere else. Focus is the host's: it stays on the canvas.
    pub(in crate::session) fn gradient_pointer_down(&mut self, point: Point) -> bool {
        let Some((shaded, layout)) = self.handle_subject() else {
            self.gradient.selected = None;
            return false;
        };
        let Some(handle) = layout.hit(point, self.view().scale()) else {
            self.gradient.selected = None;
            self.gradient.entry = None;
            return false;
        };
        let id = shaded.object.id();
        if self
            .gradient
            .entry
            .is_some_and(|entry| entry.handle != handle)
        {
            self.gradient.entry = None;
        }
        self.gradient.subject = Some(id);
        self.gradient.active = handle.stop(shaded.fill.stops.len());
        self.gradient.selected = Some(handle);
        self.gradient.drag = Some(HandleDrag {
            id,
            start: GradientDragStart {
                handle,
                geometry: shaded.fill.geometry,
                layout,
                press: point,
            },
            frame: shaded.frame,
            pointer: point,
            moved: false,
        });
        true
    }

    /// The pointer moved: a drag follows it as a preview (nothing written); otherwise the hover
    /// halo follows. `true` when a drag took the move.
    pub(in crate::session) fn gradient_pointer_move(&mut self, point: Point) -> bool {
        let Some(mut drag) = self.gradient.drag.take() else {
            self.gradient.hovered = self
                .handle_subject()
                .and_then(|(_, layout)| layout.hit(point, self.view().scale()));
            return false;
        };
        drag.pointer = point;
        let threshold = TransformHandleTolerances::at_scale(self.view().scale()).drag_threshold_mm;
        drag.moved |=
            (point.x - drag.start.press.x).hypot(point.y - drag.start.press.y) > threshold;
        if drag.moved {
            let edit = match drag.start.handle {
                GradientHandle::Stop(index) => drag
                    .start
                    .dragged_offset(point)
                    .map(|offset| StyleEdit::GradientStopOffset(index, offset)),
                _ => drag
                    .start
                    .dragged_geometry(&drag.frame, point, self.held.shift, self.held.ctrl)
                    .map(StyleEdit::GradientGeometry),
            };
            if let Some(edit) = edit {
                self.style.preview(&[drag.id], edit);
            }
        }
        self.gradient.drag = Some(drag);
        true
    }

    /// The release: a moved drag commits as one step "Edit gradient"; `true` when a gradient
    /// drag (or one Escape cancelled) took it.
    pub(in crate::session) fn gradient_pointer_up(&mut self) -> bool {
        let swallowed = std::mem::take(&mut self.gradient.swallow_release);
        let Some(drag) = self.gradient.drag.take() else {
            if swallowed {
                self.commit_style_preview();
            }
            return swallowed;
        };
        if drag.moved {
            self.commit_style_preview();
        }
        true
    }

    /// The readout chip of a moved drag: the Angle, the Radius or the stop's position; none for
    /// the Radial centre.
    pub(super) fn gradient_readout(&self, shaded: &Shaded) -> Option<GradientReadoutJson> {
        let drag = self.gradient.drag.as_ref().filter(|drag| drag.moved)?;
        let text = match drag.start.handle {
            GradientHandle::Stop(index) => format!(
                "{} %",
                trimmed(
                    shaded.fill.stops.as_slice().get(index)?.offset.get() * 100.0,
                    0
                )
            ),
            GradientHandle::Start | GradientHandle::End => format!("{}°", angle_text(shaded)?),
            GradientHandle::Radius => {
                format!("r {} mm", trimmed(radius_text(shaded)?.parse().ok()?, 1))
            }
            // The centre moves the whole circle: nothing it changes is a number (UX review U6).
            GradientHandle::Centre => return None,
        };
        let (x, y) = self.view().document_to_screen(drag.pointer);
        Some(GradientReadoutJson { text, x, y })
    }

    /// A double-click in the Select tool: on an end or radius handle the end entry box, on an
    /// inner stop or the Radial centre the stop entry box, on the line a new stop there (one
    /// step "Add gradient stop"); `false` elsewhere.
    pub(in crate::session) fn gradient_double_click(&mut self, point: Point) -> bool {
        let Some((shaded, layout)) = self.handle_subject() else {
            return false;
        };
        let id = shaded.object.id();
        let scale = self.view().scale();
        self.gradient.subject = Some(id);
        if let Some(handle) = layout.hit(point, scale) {
            self.gradient.active = handle.stop(shaded.fill.stops.len());
            self.gradient.entry = Some(OpenEntry { id, handle });
            return true;
        }
        let Some(offset) = layout
            .line_offset_at(point, scale)
            .and_then(|t| StopOffset::new(t).ok())
        else {
            return false;
        };
        let added = shaded.fill.stops.with_stop_added(offset).1;
        let wrote = self.recording(|this| {
            this.flush_style_preview();
            this.document
                .edit_style(&[id], &StyleEdit::AddGradientStop(Some(offset)))
                .is_ok()
        });
        if wrote {
            self.gradient.active = added;
            self.gradient.selected = Some(GradientHandle::Stop(added));
        }
        true
    }

    /// Escape, Delete and Backspace while a gradient handle is dragged, selected or has its
    /// entry box open (UX notes §6.4); `None` leaves the key to the table.
    pub(in crate::session) fn gradient_key(&mut self, input: &KeyInput<'_>) -> Option<KeyOutcome> {
        if input.dom_blocked || input.ctrl || input.alt {
            return None;
        }
        match input.key {
            "Escape" => {
                if self.gradient.drag.take().is_some() {
                    self.cancel_style_preview();
                    self.gradient.swallow_release = true;
                } else if self.gradient.entry.take().is_none()
                    && self.gradient.selected.take().is_none()
                {
                    return None;
                }
                Some(KeyOutcome::Consumed)
            }
            "Delete" | "Backspace" => {
                let handle = self.gradient.selected?;
                let (shaded, _) = self.handle_subject()?;
                if self.gradient.subject != Some(shaded.object.id()) {
                    self.gradient.selected = None;
                    return None;
                }
                // An end or the centre is never removed, and the shape is not deleted (P18).
                if let GradientHandle::Stop(index) = handle {
                    let id = shaded.object.id();
                    let wrote = self.recording(|this| {
                        this.document
                            .edit_style(&[id], &StyleEdit::RemoveGradientStop(index))
                            .is_ok()
                    });
                    if wrote {
                        self.gradient.selected = None;
                        // The next stop, or the previous one after the last inner stop.
                        let n = shaded.fill.stops.len();
                        self.gradient.active = if index + 2 == n { index - 1 } else { index };
                    }
                }
                Some(KeyOutcome::Consumed)
            }
            _ => None,
        }
    }
}
