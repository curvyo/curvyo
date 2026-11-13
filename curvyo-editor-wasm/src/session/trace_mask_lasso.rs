//! Drawing a lasso in the Trace tool (`specs/0114-image-trace-mask` Behaviour 3, criteria 4 to 6,
//! customer decisions of 2026-11-13, UX notes §6): each click sets a corner on a whole picture
//! pixel; a click on the first corner, a double click, Enter or a right click finish it closed by a
//! straight side; Escape and Ctrl+Z discard it; Backspace removes the last corner; another tool,
//! part or image, or leaving the tool, closes it. Fewer than three corners finish as nothing.

use curvyo_document_core::{NewTraceRegion, TraceRegionKind};
use curvyo_ui_core::{LassoDraft, trace_lasso_to_unit};

use super::trace_mask_input::MaskFrame;
use super::trace_mask_tool::{LassoInProgress, TraceRegionTool};
use super::{Session, Tool};

/// How near the first corner a click closes the lasso, screen pixels (UX notes §5).
pub(super) const CLOSE_TARGET_PX: f64 = 16.0;

impl Session {
    /// Whether a lasso is being drawn.
    #[must_use]
    pub fn trace_lasso_open(&self) -> bool {
        self.trace.mask.lasso.is_some()
    }

    /// A left press with Lasso pressed at `p` (picture pixels of `frame`): on the close target
    /// it finishes the lasso, else it sets a corner (the first starts a lasso in the active part).
    pub(super) fn trace_lasso_press(&mut self, frame: &MaskFrame, p: kurbo::Point) {
        let radius = if frame.screen_per_px > 0.0 {
            CLOSE_TARGET_PX / frame.screen_per_px
        } else {
            0.0
        };
        if let Some(lasso) = &mut self.trace.mask.lasso {
            if lasso.draft.closes_at(p, radius) {
                self.close_trace_lasso();
            } else {
                lasso.draft.add(p);
            }
            return;
        }
        let part = self.trace_active_mask_part().and_then(|part| part.stored);
        let mut draft = LassoDraft::default();
        draft.add(p);
        self.trace.mask.lasso = Some(LassoInProgress {
            draft,
            image: frame.image.id,
            owner: self.trace_mask_owner(frame.image.id),
            part,
            width: frame.width,
            height: frame.height,
        });
    }

    /// Finishes the lasso in progress: closed by a straight side, stored as one step "Draw lasso"
    /// in the part that was active when it started (added, or subtracted while Subtract is
    /// pressed); fewer than three corners or all on one line store nothing. Nothing without one.
    pub fn close_trace_lasso(&mut self) {
        let Some(lasso) = self.trace.mask.lasso.take() else {
            return;
        };
        let Some(corners) = lasso.draft.finished() else {
            return;
        };
        let region = NewTraceRegion {
            kind: TraceRegionKind::Lasso,
            subtract: self.trace.mask.subtract,
            points: trace_lasso_to_unit(&corners, lasso.width, lasso.height),
        };
        self.recording(|this| {
            this.with_part_ref(lasso.part.as_deref(), |part| {
                this.document
                    .draw_trace_region(lasso.owner, part, &region)
                    .is_ok()
            })
        });
    }

    /// Whether a subtracted region is being drawn: a rectangle dragged or a lasso in progress
    /// while Subtract is pressed (the Remove badge, UX notes §5).
    pub(super) fn trace_mask_drawing_subtracted(&self) -> bool {
        let drawing = self.trace.mask.lasso.is_some()
            || self.trace.mask.drag.as_ref().is_some_and(|drag| {
                matches!(drag.kind, super::trace_mask_tool::MaskDragKind::Draw)
            });
        self.tool == Tool::Trace && self.trace.mask.subtract && drawing
    }

    /// Discards the lasso in progress: nothing stored, no step. Whether there was one.
    pub fn discard_trace_lasso(&mut self) -> bool {
        self.trace.mask.lasso.take().is_some()
    }

    /// A double click in the Trace tool with a region tool pressed: it finishes a lasso in
    /// progress (its second click adds no corner, criterion 4) and never creates paths. Whether
    /// a region tool took it.
    pub(super) fn trace_mask_double_click(&mut self) -> bool {
        if self.tool != Tool::Trace || self.trace.mask.region_tool == TraceRegionTool::None {
            return false;
        }
        self.close_trace_lasso();
        true
    }

    /// A right press on the canvas (customer decision 1 of 2026-11-13): with a lasso in progress
    /// it finishes it, closed; without one it does nothing. Whether it took the press, so the
    /// host consumes it and no context menu opens.
    pub fn trace_mask_secondary_press(&mut self) -> bool {
        if self.tool != Tool::Trace || self.trace.mask.lasso.is_none() {
            return false;
        }
        self.close_trace_lasso();
        true
    }

    /// Backspace while drawing: the last corner goes; the only one discards the lasso. Whether a
    /// lasso was in progress.
    pub(super) fn trace_lasso_backspace(&mut self) -> bool {
        let Some(lasso) = &mut self.trace.mask.lasso else {
            return false;
        };
        if !lasso.draft.remove_last() {
            self.trace.mask.lasso = None;
        }
        true
    }

    /// Closes a lasso in progress drawn on another picture than the one traced now (another image
    /// chosen in the Layers panel, customer decision 4 of 2026-11-13).
    pub(super) fn close_trace_lasso_of_other_image(&mut self) {
        let drawn_on = self.trace.mask.lasso.as_ref().map(|lasso| lasso.image);
        if drawn_on.is_some() && drawn_on != self.trace_mask_image().map(|image| image.id) {
            self.close_trace_lasso();
        }
    }
}
