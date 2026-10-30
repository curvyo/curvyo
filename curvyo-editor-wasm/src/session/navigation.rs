//! `Session`'s pan/zoom glue (`specs/0004-canvas-navigation-and-
//! selection/specification.md`, acceptance criteria 1-11): the
//! screen↔document conversion, the wheel event (pan, or Ctrl-zoom about
//! the cursor), the middle-mouse/Space drag-pan gesture, resize, and the
//! zoom read-out. Split out of `session/mod.rs` (architect review: that
//! file grew past a size that still read as "one responsibility" once
//! navigation delegation landed directly in it) the same way
//! `session/select.rs` and `session/shapes.rs` already are — a child
//! module of `session`, so it reaches `Session`'s otherwise-private
//! `viewport` field and its methods join the same type's `impl Session`
//! `session/mod.rs` itself defines.

use curvyo_document_core::{Point, ViewTransform};
use curvyo_ui_core::Viewport;

use super::Session;

/// How many wheel-delta pixels correspond to one "doubling" of zoom
/// (Ctrl+scroll, acceptance criterion 6) — not itself pinned by any
/// acceptance criterion (only the resulting range and the cursor-fixed
/// point are, criteria 6-8), chosen so an ordinary mouse-wheel notch
/// (~100px after the host's `deltaMode` normalization) feels like a
/// deliberate, moderate zoom step rather than a jump.
const ZOOM_WHEEL_SENSITIVITY_PX: f64 = 400.0;

/// Converts a wheel event's vertical delta (screen pixels, already
/// normalized by the host from whichever `deltaMode` the browser used)
/// into a multiplicative zoom factor for [`curvyo_ui_core::Viewport::
/// zoom_about`]: scrolling up (negative `delta_y`) zooms in (`factor >
/// 1`), scrolling down zooms out (`factor < 1`), continuously rather than
/// in fixed steps.
fn zoom_factor_from_wheel_delta(delta_y: f64) -> f64 {
    (-delta_y / ZOOM_WHEEL_SENSITIVITY_PX).exp2()
}

impl Session {
    /// Puts the document's top-left corner 128 px in from the canvas's
    /// top-left corner at 100 % zoom (criterion 11a) until the canvas size is
    /// known; every size report then centres the document in the canvas
    /// (`resize_viewport`). The host calls it once
    /// for a project that was just created or opened (`WasmSession::new` and
    /// `open`), not `Session::new`: headless sessions keep the plain view of
    /// `0004`, which tests rely on. The view stays centred through window
    /// resizes until the first pan or zoom.
    pub fn show_default_view(&mut self) {
        self.viewport = Viewport::with_document_inset();
    }

    /// The current view transform, for the host's GPU layer to build
    /// this frame's screen transform from.
    #[must_use]
    pub fn view(&self) -> ViewTransform {
        self.viewport.view()
    }

    /// Converts canvas-relative CSS pixels to a document point via the
    /// current viewport (`specs/0004-canvas-navigation-and-selection/
    /// adrs.md`: "all screen↔document conversion happens in Rust") — the
    /// wasm pointer methods call this before dispatching to this
    /// module's own document-space entry points.
    #[must_use]
    pub fn screen_to_document(&self, screen_x: f64, screen_y: f64) -> Point {
        self.viewport.screen_to_document(screen_x, screen_y)
    }

    /// The current zoom level's integer percentage read-out (acceptance
    /// criterion 9).
    #[must_use]
    pub fn zoom_percent(&self) -> i64 {
        self.viewport.zoom_percent()
    }

    /// A wheel event at canvas-relative CSS pixel `(screen_x, screen_y)`:
    /// Ctrl (Cmd) held zooms about that point (acceptance criterion 6);
    /// otherwise pans — Shift swaps a vertical-only wheel's `delta_y`
    /// onto the horizontal axis (acceptance criterion 2), while any
    /// native `delta_x` (a two-finger trackpad scroll) still applies on
    /// top, so a diagonal trackpad scroll pans diagonally either way.
    /// Never forwarded to the active tool (acceptance criteria 5, 24:
    /// "navigation input is offered to the viewport before the active
    /// tool").
    pub fn wheel(
        &mut self,
        delta_x: f64,
        delta_y: f64,
        screen_x: f64,
        screen_y: f64,
        shift: bool,
        ctrl: bool,
    ) {
        if ctrl {
            let factor = zoom_factor_from_wheel_delta(delta_y);
            self.viewport.zoom_about(screen_x, screen_y, factor);
        } else if shift {
            self.viewport.pan_by_screen_delta(delta_x + delta_y, 0.0);
        } else {
            self.viewport.pan_by_screen_delta(delta_x, delta_y);
        }
        self.refresh_colour_pick_hover(screen_x, screen_y);
        self.refresh_trace_pick_hover(screen_x, screen_y);
    }

    /// Starts a drag-pan gesture (middle-mouse or Space+primary,
    /// acceptance criteria 3, 4) at canvas-relative CSS pixel
    /// `(screen_x, screen_y)`. Never forwarded to the active tool — the
    /// host calls this instead of [`Session::pointer_down`] for exactly
    /// these two gestures, so the active tool's own in-progress state is
    /// untouched by construction (acceptance criterion 5).
    pub fn begin_pan(&mut self, screen_x: f64, screen_y: f64) {
        self.viewport.begin_drag_pan(screen_x, screen_y);
    }

    /// Continues the drag-pan gesture [`Session::begin_pan`] started,
    /// keeping its anchor document point exactly under the live cursor.
    pub fn pan_to(&mut self, screen_x: f64, screen_y: f64) {
        self.viewport.continue_drag_pan(screen_x, screen_y);
        self.refresh_colour_pick_hover(screen_x, screen_y);
        self.refresh_trace_pick_hover(screen_x, screen_y);
    }

    /// Ends the drag-pan gesture, if one is in flight.
    pub fn end_pan(&mut self) {
        self.viewport.end_drag_pan();
    }

    /// Whether a drag-pan gesture is currently in flight — the host's
    /// grab/grabbing cursor convention (`docs/design-system.md`'s "Pan
    /// cursor").
    #[must_use]
    pub fn is_panning(&self) -> bool {
        self.viewport.is_drag_panning()
    }

    /// Resizes the canvas, keeping the zoom and the document point at
    /// the viewport's own center fixed (acceptance criterion 10). A view that
    /// is still the untouched default keeps the document centred in the new
    /// size instead (`Viewport::centre_document`).
    /// A side pane covering the canvas's right part (`cover_right`) is left out: the viewport
    /// is the visible part.
    pub fn resize_viewport(&mut self, width: f64, height: f64) {
        self.canvas_css_width = width;
        self.viewport
            .resize((width - self.covered_right).max(0.0), height);
        let size = self.document.size();
        self.viewport
            .centre_document(size.width.as_mm(), size.height.as_mm());
    }

    /// A side pane now lies over the canvas's right `covered` CSS px (`0` for none,
    /// `specs/0077-font-preview` criterion 2): the canvas element keeps its size, the viewport
    /// (fit, centring, the off-screen and placement rules) shrinks to the visible part. Announce
    /// the width change first (`keep_view_origin_for_width_change`) to keep the top-left origin.
    pub fn cover_right(&mut self, covered: f64) {
        let covered = if covered.is_finite() {
            covered.max(0.0)
        } else {
            0.0
        };
        if (covered - self.covered_right).abs() < f64::EPSILON {
            return;
        }
        self.covered_right = covered;
        let (_, height) = self.viewport.canvas_size();
        self.resize_viewport(self.canvas_css_width, height);
    }

    /// The properties panel is about to open or close and change the canvas
    /// width by `delta` CSS pixels: that one resize keeps the view's top-left
    /// origin, so the document does not move on screen
    /// (`specs/0007-stroke-and-fill-styling` criterion 39).
    pub fn keep_view_origin_for_width_change(&mut self, delta: f64) {
        if delta.is_finite() {
            self.viewport.keep_origin_for_width_change(delta);
        }
    }

    /// Records the display's device pixel ratio (`window.devicePixelRatio`),
    /// which an axis-aligned selection box snaps to
    /// (`edit-interaction-polish` criterion 65). A value that is not a
    /// positive finite number reads as 1.
    pub fn set_device_pixel_ratio(&mut self, ratio: f64) {
        self.device_pixel_ratio = if ratio.is_finite() && ratio > 0.0 {
            ratio
        } else {
            1.0
        };
    }

    /// The device pixel ratio last set, 1 before any.
    #[must_use]
    pub fn device_pixel_ratio(&self) -> f64 {
        self.device_pixel_ratio
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::Point;

    use super::*;

    /// The document's left and right edge, top and bottom edge on screen.
    fn document_edges(session: &Session) -> ((f64, f64), (f64, f64)) {
        let size = session.document.size();
        let view = session.view();
        let (left, top) = view.document_to_screen(Point::new(0.0, 0.0));
        let (right, bottom) =
            view.document_to_screen(Point::new(size.width.as_mm(), size.height.as_mm()));
        ((left, right), (top, bottom))
    }

    /// The customer's observation: a new project showed the document at the
    /// left. After the first size report it sits in the middle of the canvas
    /// right of the tool rail, and a later window resize keeps it there.
    #[test]
    fn a_new_project_is_centred_after_the_size_report_and_after_a_window_resize() {
        let mut session = Session::new(1);
        session.show_default_view();
        // A4 is 794 px wide at 100 %: a canvas wide enough for it with room.
        session.resize_viewport(1500.0, 1300.0);
        let ((left, right), (top, bottom)) = document_edges(&session);
        assert!(
            ((left - 128.0) - (1500.0 - right)).abs() < 1e-9,
            "{left} {right}"
        );
        assert!((top - (1300.0 - bottom)).abs() < 1e-9, "{top} {bottom}");

        session.resize_viewport(1100.0, 1250.0);
        let ((left, right), (top, bottom)) = document_edges(&session);
        assert!(
            ((left - 128.0) - (1100.0 - right)).abs() < 1e-9,
            "{left} {right}"
        );
        assert!((top - (1250.0 - bottom)).abs() < 1e-9, "{top} {bottom}");
    }

    /// After the first pan the view is the maker's own: a resize keeps the
    /// centre point as before.
    #[test]
    fn a_panned_view_is_not_centred_again() {
        let mut session = Session::new(1);
        session.show_default_view();
        session.resize_viewport(1500.0, 1300.0);
        session.wheel(30.0, 0.0, 400.0, 400.0, false, false);
        let centre = session.screen_to_document(750.0, 650.0);

        session.resize_viewport(1100.0, 1250.0);

        let after = session.screen_to_document(550.0, 625.0);
        assert!((centre.x - after.x).abs() < 1e-9 && (centre.y - after.y).abs() < 1e-9);
    }

    /// `0077` criterion 2: a pane over the canvas's right part shrinks the viewport to the
    /// visible part, keeps the view where it was when announced, and a later canvas resize
    /// keeps leaving the covered part out.
    #[test]
    fn a_pane_over_the_canvas_leaves_the_view_and_shrinks_the_viewport_to_the_visible_part() {
        let mut session = Session::new(1);
        session.show_default_view();
        session.resize_viewport(1500.0, 1300.0);
        session.wheel(30.0, 0.0, 400.0, 400.0, false, false);
        let corner = session.screen_to_document(0.0, 0.0);

        session.keep_view_origin_for_width_change(-500.0);
        session.cover_right(500.0);
        assert_eq!(session.viewport.canvas_size(), (1000.0, 1300.0));
        let after = session.screen_to_document(0.0, 0.0);
        assert!((corner.x - after.x).abs() < 1e-9 && (corner.y - after.y).abs() < 1e-9);

        session.resize_viewport(1400.0, 1300.0);
        assert_eq!(session.viewport.canvas_size(), (900.0, 1300.0));
        session.keep_view_origin_for_width_change(500.0);
        session.cover_right(0.0);
        assert_eq!(session.viewport.canvas_size(), (1400.0, 1300.0));
    }
}
