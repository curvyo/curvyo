//! Refuses a resize or skew that would deform a variable-width stroke (ADR 0053, "Width points are
//! geometry": until a nonuniform scale can keep the rendered outline, it is refused for a profiled
//! path with a visible explanation and the object stays as it is).

use curvyo_ui_core::EditHandle;

use super::Session;

/// The refusal of an uneven resize or a skew of a selection with a variable-width stroke.
pub(super) const UNEVEN_WIDTH_NOTICE: &str = "A variable-width stroke can only be scaled evenly (hold Ctrl); the resize or skew was cancelled.";

/// The refusal of a transform that would leave a variable stroke without an outline.
pub(super) const NO_OUTLINE_NOTICE: &str = "A variable stroke would have no outline there.";

/// The transform notices of `specs/0110` that are refusals: the host shows them beside the
/// pointer in the alert look (UX review U8).
const REFUSALS: [&str; 7] = [
    UNEVEN_WIDTH_NOTICE,
    NO_OUTLINE_NOTICE,
    super::spiro_cache::SPIRO_FAILED_NOTICE,
    super::spiro_cache::SPIRO_UNSOLVED_NOTICE,
    super::spiro_cache::CONVERT_REFERENCED_NOTICE,
    super::spiro_cache::CONVERT_SHARED_NOTICE,
    super::spiro_cache::CONVERT_FAILED_NOTICE,
];

impl Session {
    /// Whether the transform notice waiting to be taken is a refusal of this story, shown
    /// beside the pointer as an alert rather than in the notice slot (UX review U8).
    #[must_use]
    pub fn transform_notice_is_refusal(&self) -> bool {
        self.transform_notice
            .is_some_and(|notice| REFUSALS.contains(&notice))
    }

    /// Whether an object the Select tool transforms draws a stroke with width points, itself or
    /// as a leaf of a clone of a group.
    pub(super) fn selection_has_width_profile(&self) -> bool {
        self.transform_selection_ids().iter().any(|id| {
            self.document
                .object(*id)
                .is_some_and(|object| !curvyo_ui_core::profiled_paths(&object).is_empty())
        })
    }

    /// Whether the Select drag in flight, released at the last pointer position, resizes the two
    /// axes unevenly or skews.
    pub(super) fn select_drag_deforms(&self) -> bool {
        let Some(point) = self.pointer_position else {
            return false;
        };
        let (shift, ctrl) = (self.held.shift, self.held.ctrl);
        match self.select.dragging_handle() {
            Some(EditHandle::Resize(_)) => self
                .select
                .live_resize_factors(point, shift, ctrl)
                .is_some_and(|(sx, sy)| curvyo_ui_core::scales_unevenly(sx, sy)),
            Some(EditHandle::Skew(_)) => self.select_live_edit_in(&self.objects()).is_some(),
            _ => false,
        }
    }

    /// Whether the open transform entry, committed with `texts`, resizes unevenly or skews.
    pub(super) fn entry_deforms(&self, texts: [&str; 2], last_edited: usize) -> bool {
        self.open_skew_entry().is_some()
            || self
                .open_entry()
                .is_some_and(|entry| entry.scales_unevenly(texts, last_edited))
            || self
                .open_group_entry()
                .is_some_and(|entry| entry.deforms(texts, last_edited))
    }
}
