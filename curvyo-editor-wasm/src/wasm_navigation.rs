//! The canvas navigation's `wasm-bindgen` surface (`specs/0004-canvas-
//! navigation-and-selection`, criteria 1 to 10): wheel pan and zoom, the
//! drag-pan gesture and the zoom read-out. A further `impl WasmSession` block
//! (`wasm_api.rs` holds the struct); strings and scalars only (ADR 0001 §5),
//! every method a pass-through to `Session`.

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// A wheel event at canvas-relative CSS pixel `(x, y)`: pans
    /// (acceptance criteria 1, 2) or, with Ctrl/Cmd held, zooms about
    /// that point (acceptance criterion 6). `delta_x`/`delta_y` are
    /// already normalized to pixels by the host (`deltaMode`).
    pub fn wheel(&mut self, delta_x: f64, delta_y: f64, x: f64, y: f64, shift: bool, ctrl: bool) {
        self.session.wheel(delta_x, delta_y, x, y, shift, ctrl);
    }

    /// Starts a drag-pan gesture (middle-mouse or Space+primary,
    /// acceptance criteria 3, 4) at canvas-relative CSS pixel `(x, y)`.
    /// The host calls this instead of [`WasmSession::pointer_down`] for
    /// exactly these two gestures.
    pub fn begin_pan(&mut self, x: f64, y: f64) {
        self.session.begin_pan(x, y);
    }

    /// Continues the drag-pan gesture [`WasmSession::begin_pan`] started.
    pub fn pan_to(&mut self, x: f64, y: f64) {
        self.session.pan_to(x, y);
    }

    /// Ends the drag-pan gesture, if one is in flight.
    pub fn end_pan(&mut self) {
        self.session.end_pan();
    }

    /// Whether a drag-pan gesture is currently in flight — the host's
    /// grab/grabbing cursor convention (`docs/design-system.md`'s "Pan
    /// cursor").
    #[must_use]
    pub fn is_panning(&self) -> bool {
        self.session.is_panning()
    }

    /// The view to come back to after switching documents (`specs/0094-workspace` criterion 10).
    #[must_use]
    pub fn saved_view_text(&self) -> String {
        self.session.saved_view_text()
    }

    /// Shows a view [`WasmSession::saved_view_text`] gave; `false` for other text.
    pub fn restore_view_text(&mut self, text: &str) -> bool {
        self.session.restore_view_text(text)
    }

    /// The current zoom level's integer percentage read-out (acceptance
    /// criterion 9), for the status bar's center segment.
    #[must_use]
    pub fn zoom_percent(&self) -> i32 {
        #[allow(clippy::cast_possible_truncation)]
        (self.session.zoom_percent() as i32)
    }
}
