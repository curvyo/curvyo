//! The Trace tool's pipette (`specs/0089-trace-quality-and-colour` criteria 16 and 17,
//! `ux-notes.md` §6): while it is armed, a primary press on the image the preview shows takes
//! the colour of its working image under the pointer (`curvyo_ui_core::trace_sample_colour`)
//! for the host, and no tool sees the press. Remove background disarms after one pick; Pick
//! colours stays armed (Frage 89 A). Pan and zoom keep working.

use curvyo_document_core::Point;
use curvyo_ui_core::{TraceRgb, trace_sample_colour};

use super::{Session, Tool};

/// What the pipette is armed for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracePickMode {
    /// The background colour of Remove background: one pick, then disarmed.
    Background,
    /// The colours of Pick colours: armed until the host disarms it.
    Colours,
}

/// What the pipette holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct TracePick {
    mode: Option<TracePickMode>,
    /// The colour a click at the pointer would take (`None`: no colour there).
    hover: Option<TraceRgb>,
    /// The pick the host has not taken yet, and whether Shift added it.
    taken: Option<(TraceRgb, bool)>,
    /// The release that follows a pick belongs to no gesture.
    swallow_release: bool,
}

impl Session {
    /// Arms the pipette for `mode`, or disarms it with `None`.
    pub fn set_trace_pick(&mut self, mode: Option<TracePickMode>) {
        self.trace.pick.mode = if self.tool == Tool::Trace { mode } else { None };
        self.trace.pick.hover = None;
        if let Some(point) = self.pointer_position {
            self.trace_pick_move(point);
        }
    }

    /// What the pipette is armed for, if it is.
    #[must_use]
    pub const fn trace_pick_mode(&self) -> Option<TracePickMode> {
        self.trace.pick.mode
    }

    /// The colour a click would take while armed: `Some(None)` where there is none (outside
    /// the image, a transparent pixel); `None` while disarmed.
    #[must_use]
    pub const fn trace_pick_hover(&self) -> Option<Option<TraceRgb>> {
        match self.trace.pick.mode {
            Some(_) => Some(self.trace.pick.hover),
            None => None,
        }
    }

    /// The pick not taken yet, with whether Shift added it; once.
    pub fn take_trace_pick(&mut self) -> Option<(TraceRgb, bool)> {
        self.trace.pick.taken.take()
    }

    /// The colour of the shown image's working image at `point`.
    fn trace_sample_at(&mut self, point: Point) -> Option<TraceRgb> {
        let shown = self.trace_view().shown?;
        let (_, images) = self.trace_images();
        let snapshot = images.into_iter().find(|image| image.id == shown)?;
        let working = self.working_image(&snapshot.asset)?;
        let map = snapshot
            .pixel_map(working.width, working.height)
            .inverse()?;
        let pixel = map.apply_point(point);
        if !(pixel.x >= 0.0 && pixel.y >= 0.0) {
            return None;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // checked >= 0
        let (x, y) = (pixel.x.floor() as u32, pixel.y.floor() as u32);
        trace_sample_colour(&working.pixels, working.width, working.height, x, y)
    }

    /// A primary press while armed: takes the colour for the host (Shift adds), disarms after a
    /// background pick; where there is no colour, nothing is taken. Returns whether the
    /// pipette consumed the press.
    pub(super) fn trace_pick_press(&mut self, point: Point, shift: bool) -> bool {
        let Some(mode) = self.trace.pick.mode else {
            return false;
        };
        self.trace.pick.swallow_release = true;
        if let Some(colour) = self.trace_sample_at(point) {
            self.trace.pick.taken = Some((colour, shift));
            if mode == TracePickMode::Background {
                self.trace.pick.mode = None;
            }
        }
        true
    }

    /// The pointer moved while armed: remembers what a click would take. Returns whether the
    /// pipette consumed the move.
    pub(super) fn trace_pick_move(&mut self, point: Point) -> bool {
        if self.trace.pick.mode.is_none() {
            return false;
        }
        self.trace.pick.hover = self.trace_sample_at(point);
        true
    }

    /// A pan or zoom moved the document under the pointer at canvas pixel `(screen_x,
    /// screen_y)`: the colour a click would take is read again.
    pub(super) fn refresh_trace_pick_hover(&mut self, screen_x: f64, screen_y: f64) {
        if self.trace.pick.mode.is_some() {
            let point = self.screen_to_document(screen_x, screen_y);
            self.trace_pick_move(point);
        }
    }

    /// The release of a press the pipette consumed.
    pub(super) fn trace_pick_release(&mut self) -> bool {
        std::mem::take(&mut self.trace.pick.swallow_release)
    }
}
