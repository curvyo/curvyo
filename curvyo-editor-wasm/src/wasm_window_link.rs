//! The pen window's link on the `wasm-bindgen` surface (`specs/0158-pen-display` slice 2a and 2b,
//! ADR 0096 A10): bundles in and out, the main window's hold and save, Follow view and the pen
//! view's frame, the state for the page. A further `impl WasmSession` block, every method a
//! pass-through to `Session`.

use wasm_bindgen::prelude::*;

use crate::session::{LinkRole, LinkState};
use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// Starts this window's link: `main` when it opens the pen window, `pen` when the pen page
    /// starts. Any other role is ignored.
    pub fn link_start(&mut self, role: &str) {
        let role = match role {
            "main" => LinkRole::Main,
            "pen" => LinkRole::Pen,
            _ => return,
        };
        self.session.link_start(role);
    }

    /// Ends the link (the pen window closed without the hub's message reaching the page).
    pub fn link_end(&mut self) {
        self.session.link_end();
    }

    /// One message from the other window, imported at the next `link_frame`.
    pub fn link_receive(&mut self, bytes: &[u8]) {
        self.session.link_receive(bytes);
    }

    /// One frame of the link, at the start of the frame: the bundle to send, `undefined` when
    /// there is nothing or no link.
    pub fn link_frame(&mut self) -> Option<Vec<u8>> {
        if !self.session.has_window_link() {
            return None;
        }
        // A frame of its own only when an import or an arrived selection changed the picture.
        let out = self.session.untracked_mut().link_frame();
        if self.session.untracked_mut().link_take_redraw() {
            self.session.force_frame();
        }
        out
    }

    /// Main: the immediate `Hold` (`on`) or `Release` bundle around a fate-deciding dialog.
    pub fn link_hold(&mut self, on: bool) -> Option<Vec<u8>> {
        self.session.link_hold(on)
    }

    /// Pen: Ctrl+S asks the main window to save.
    pub fn link_request_save(&mut self) {
        self.session.link_request_save();
    }

    /// Main: whether the pen window asked for a save since the last call.
    pub fn link_take_save(&mut self) -> bool {
        self.session.untracked_mut().link_take_save()
    }

    /// Pen: the project bytes that arrived, to open as a fresh session and `link_continue` on.
    pub fn link_take_project(&mut self) -> Option<Vec<u8>> {
        self.session.untracked_mut().link_take_project()
    }

    /// Continues the link of `old` in this session, the host's replacement for it (New, Open, a
    /// reload): call before `old` is freed.
    pub fn link_continue(&mut self, old: &mut WasmSession) {
        self.session.link_continue(&mut old.session);
    }

    /// A value field is busy (typed in, dragged, a chip open): an arriving selection waits.
    pub fn link_set_field_busy(&mut self, busy: bool) {
        self.session.link_set_field_busy(busy);
    }

    /// `none`, `waiting`, `held`, `out-of-sync` or `live`.
    #[must_use]
    pub fn link_state(&self) -> String {
        match self.session.link_state() {
            LinkState::None => "none",
            LinkState::Waiting => "waiting",
            LinkState::Held => "held",
            LinkState::OutOfSync => "out-of-sync",
            LinkState::Live => "live",
        }
        .to_owned()
    }

    /// Whether imports changed the document since the last call.
    pub fn link_take_changed(&mut self) -> bool {
        self.session.untracked_mut().link_take_changed()
    }

    /// The link's next notice, empty when there is none.
    pub fn link_take_notice(&mut self) -> String {
        self.session
            .untracked_mut()
            .link_take_notice()
            .unwrap_or_default()
    }

    /// Pen: Follow view on or off (2b criterion 5).
    pub fn link_set_follow(&mut self, on: bool) {
        self.session.link_set_follow(on);
    }

    /// Whether Follow view is on.
    #[must_use]
    pub fn link_follow(&self) -> bool {
        self.session.link_follow()
    }

    /// Main: the pen window's visible rectangle on this canvas in CSS px, `[left, top, right,
    /// bottom]`, or empty when there is none to draw (2b criterion 6).
    #[must_use]
    pub fn link_pen_frame(&self) -> Vec<f64> {
        self.session
            .link_pen_frame()
            .map_or_else(Vec::new, |frame| frame.to_vec())
    }

    /// The pen rail's Fit page: the active page whole in the canvas right of `free_left` and
    /// below `free_top` (CSS px), 24 px margin elsewhere.
    pub fn view_fit_page(&mut self, free_left: f64, free_top: f64) {
        self.session.view_fit_page(free_left, free_top);
    }

    /// The pen rail's Zoom in or out about the free canvas's centre (CSS px).
    pub fn zoom_step(&mut self, zoom_in: bool, screen_x: f64, screen_y: f64) {
        self.session.zoom_step(zoom_in, screen_x, screen_y);
    }
}
