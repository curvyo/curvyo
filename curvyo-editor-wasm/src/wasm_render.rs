//! The canvas attachment and per-frame submission of `WasmSession`: the
//! `wgpu` surface, resize and render, skipped while the frame generation
//! (`frame_gate.rs`) shows nothing new. A further `impl WasmSession` block
//! (`wasm_api.rs` holds the struct).

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::frame_gate::TrackedSession;
use crate::gpu::Gpu;
use crate::session::Session;
use crate::wasm_api::WasmSession;

impl WasmSession {
    /// `session` with no canvas attached yet.
    pub(crate) fn of(session: Session) -> Self {
        Self {
            session: TrackedSession::new(session),
            gpu: None,
        }
    }

    /// Builds the frame and submits it to `gpu`; recorded as drawn only when the submit worked,
    /// so a lost surface is drawn again on the next frame.
    fn submit(session: &mut TrackedSession, gpu: &mut Gpu) -> Result<(), JsValue> {
        let frame = session.untracked_mut();
        let mut draw_list = frame.frame_draw_list();
        let pictures = frame.frame_images(&mut draw_list);
        let result = gpu.render(&draw_list, frame.view(), &pictures);
        if result.is_ok() {
            session.frame_drawn();
        } else {
            session.force_frame();
        }
        result
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// Attaches this session to `canvas`, creating the `wgpu`
    /// device/surface (ADR 0001 §3). Call once, after construction,
    /// before the first [`WasmSession::render`]. `width`×`height` are the
    /// surface's backing-buffer (physical) pixel size and
    /// `device_pixel_ratio` is `window.devicePixelRatio` — the host sizes
    /// the buffer `css_size * device_pixel_ratio` (`useEditorSession.ts`'s
    /// attach effect) so the canvas renders at the display's actual
    /// resolution instead of being upscaled and softened on any `HiDPI`
    /// screen.
    ///
    /// # Errors
    /// A `JsValue` (a plain string) if no adapter/device could be
    /// obtained — see [`crate::gpu::Gpu::attach`].
    pub async fn attach_canvas(
        &mut self,
        canvas: HtmlCanvasElement,
        width: u32,
        height: u32,
        device_pixel_ratio: f64,
    ) -> Result<(), JsValue> {
        self.gpu = Some(Gpu::attach(canvas, width, height, device_pixel_ratio).await?);
        self.set_viewport_css_size(width, height, device_pixel_ratio);
        // A new surface shows nothing yet.
        self.session.force_frame();
        Ok(())
    }

    /// Records the canvas's CSS (layout) pixel size on the viewport
    /// (acceptance criterion 10) — `width`/`height` are the backing-
    /// buffer (physical) pixel size this method's two callers both
    /// receive; dividing by `device_pixel_ratio` recovers the CSS size
    /// pointer events and `Session::screen_to_document` already agree on.
    /// Also hands the ratio to the session, which snaps the selection box
    /// to device pixels with it.
    fn set_viewport_css_size(&mut self, width: u32, height: u32, device_pixel_ratio: f64) {
        self.session.set_device_pixel_ratio(device_pixel_ratio);
        let ratio = self.session.device_pixel_ratio();
        self.session
            .resize_viewport(f64::from(width) / ratio, f64::from(height) / ratio);
    }

    /// Reconfigures the attached canvas's `wgpu` surface to `width`×
    /// `height` physical pixels and `device_pixel_ratio`, **and renders
    /// the next frame immediately**, in this same call — call on every
    /// resize. Folding the two together (rather than reconfiguring here
    /// and leaving the next frame to the host's own animation-frame loop)
    /// is what the canvas-perf spike's requirement actually asks for
    /// (`specs/0002-path-node-editing/adrs.md`'s PASS note, requirement 2:
    /// "reconfigures the `wgpu` surface on every resize... then render,
    /// all in one frame"): reconfigure-then-wait-for-the-next-tick leaves
    /// one empty/stale frame on screen for every resize, which on a
    /// webview without a free-running compositor (or a throttled/
    /// backgrounded one) can be visibly stuck rather than a single
    /// imperceptible frame.
    ///
    /// # Errors
    /// A `JsValue` (a plain string) if the subsequent render's surface
    /// texture could not be acquired — see [`crate::gpu::Gpu::render`].
    /// A no-op (`Ok(())`) before [`WasmSession::attach_canvas`] has
    /// completed, same as [`WasmSession::render`].
    pub fn resize(
        &mut self,
        width: u32,
        height: u32,
        device_pixel_ratio: f64,
    ) -> Result<(), JsValue> {
        self.set_viewport_css_size(width, height, device_pixel_ratio);
        let Some(gpu) = &mut self.gpu else {
            return Ok(());
        };
        gpu.resize(width, height, device_pixel_ratio);
        // A reconfigured surface is drawn anew, whatever the session.
        self.session.force_frame();
        Self::submit(&mut self.session, gpu)
    }

    /// Builds this frame's draw list and submits it, unless the canvas
    /// shows the session's present picture already (`specs/0126` phase 1:
    /// an idle document builds and submits nothing). A no-op (not an
    /// error) before [`WasmSession::attach_canvas`] has completed.
    ///
    /// # Errors
    /// A `JsValue` (a plain string) if the frame could not be
    /// submitted.
    pub fn render(&mut self) -> Result<(), JsValue> {
        let Some(gpu) = &mut self.gpu else {
            return Ok(());
        };
        if !self.session.needs_frame() {
            return Ok(());
        }
        Self::submit(&mut self.session, gpu)
    }

    /// The render loop's one call per frame, as numbers: the frame
    /// generation (changes with every change of the session), 1 when a
    /// frame is to be drawn, the view's origin x and y and its scale, the
    /// flags (1 a sync link, 2 a width notice, 4 the unreadable-image
    /// notice waiting) and the frames submitted so far.
    #[must_use]
    pub fn frame_state(&self) -> Vec<f64> {
        self.session.frame_state().to_vec()
    }

    /// The source lines of the session's changes since the last call,
    /// `file:line count` one per line, for the performance probe
    /// (`tools/perf/`): what keeps a document drawing.
    pub fn take_change_sites(&mut self) -> String {
        self.session.take_change_sites()
    }

    /// Draws the next frame even though the session did not change: the
    /// page was hidden, or the host cannot tell what the canvas shows.
    pub fn request_frame(&mut self) {
        self.session.force_frame();
    }
}
