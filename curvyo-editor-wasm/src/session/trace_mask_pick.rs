//! Pick mask paths, the Trace tool's pick mode (`specs/0094-masked-composite-trace` slice 1b,
//! criteria 34 to 43, `ux-notes.md` §11): while it is armed, a press on a path-like object or a
//! group of them adds it to or removes it from the mask paths by the Shift+click route of the
//! tool, a press on a text, a sketch or a second image is refused, and the mode stays armed.

use curvyo_document_core::{NodeId, Point};
use curvyo_ui_core::hit_test_objects;

use super::trace_region::is_mask_path;
use super::{Session, Tool};

/// What the pick mode holds; part of the Trace tool's state, so leaving the tool disarms it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct MaskPick {
    armed: bool,
    /// The last press hit an object that cannot be a mask path: the bar's notice row says so
    /// until the next accepted pick or the end of the mode.
    refused: bool,
}

/// What an armed press on an object does.
enum PickTarget {
    /// A path-like object or a group of them: added or removed.
    Mask,
    /// A text, a sketch, a second image or a group holding one: not added, the row shows.
    Refused,
    /// The traced image: nothing.
    Image,
}

impl Session {
    /// Arms (`true`) or disarms the pick mode; only the Trace tool arms it, and arming it
    /// disarms the pipette (plan, Proposal).
    pub fn set_trace_mask_pick(&mut self, armed: bool) {
        let armed = armed && self.tool == Tool::Trace;
        if armed {
            self.set_trace_pick(None);
        }
        self.trace.mask_pick = MaskPick {
            armed,
            refused: false,
        };
        self.hovered_object = None;
    }

    /// Whether the pick mode is armed.
    #[must_use]
    pub const fn trace_mask_pick(&self) -> bool {
        self.trace.mask_pick.armed
    }

    /// Whether the last armed press was refused (criterion 38).
    #[must_use]
    pub const fn trace_mask_pick_refused(&self) -> bool {
        self.trace.mask_pick.refused
    }

    /// Clear mask (criterion 40): only the traced image stays selected; the mask paths stay in
    /// the document, nothing is written and the mode is unchanged.
    pub fn clear_trace_mask(&mut self) {
        if self.tool != Tool::Trace {
            return;
        }
        let images = self.trace_view().images;
        if let [image] = images.as_slice() {
            self.selection.set(&[*image]);
        }
    }

    /// The object a press at `point` hits and what the pick does with it; `None` on empty canvas.
    fn trace_mask_pick_target(
        &self,
        point: Point,
        images: &[NodeId],
    ) -> Option<(NodeId, PickTarget)> {
        let pickable = self.pickable_objects();
        let hit = *hit_test_objects(&pickable, point, self.object_tolerance()).first()?;
        let units = self.units();
        let members = units.leaves_with(&hit);
        if members.iter().all(|member| images.contains(member)) {
            return Some((hit, PickTarget::Image));
        }
        let objects = self.objects();
        let path_like = members.iter().all(|member| {
            objects
                .iter()
                .find(|object| object.id() == *member)
                .is_some_and(is_mask_path)
        });
        Some((
            hit,
            if path_like {
                PickTarget::Mask
            } else {
                PickTarget::Refused
            },
        ))
    }

    /// A press while armed: adds or removes the mask path under it (the Shift+click route),
    /// refuses what cannot be one, ignores the image and empty canvas. Returns whether the mode
    /// took the press; without a traced image it does not.
    pub(super) fn trace_mask_pick_press(&mut self, point: Point) -> bool {
        if !self.trace.mask_pick.armed {
            return false;
        }
        let images = self.trace_view().images;
        if images.is_empty() {
            return false;
        }
        match self.trace_mask_pick_target(point, &images) {
            Some((hit, PickTarget::Mask)) => {
                self.trace_select_hit(hit, true);
                self.trace.mask_pick.refused = false;
            }
            Some((_, PickTarget::Refused)) => self.trace.mask_pick.refused = true,
            Some((_, PickTarget::Image)) | None => {}
        }
        true
    }

    /// The pointer moved while armed: the object a press would add or remove gets the hover box
    /// (UX notes §11.4).
    pub(super) fn trace_mask_pick_hover(&mut self, point: Point) {
        if !self.trace.mask_pick.armed {
            return;
        }
        let images = self.trace_view().images;
        self.hovered_object = match self.trace_mask_pick_target(point, &images) {
            Some((hit, PickTarget::Mask)) => Some(hit),
            _ => None,
        };
    }
}
