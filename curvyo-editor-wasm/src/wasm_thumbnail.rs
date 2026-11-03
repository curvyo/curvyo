//! The thumbnail a save writes into the container (`specs/0094-workspace` criterion 6): the
//! active page drawn by the canvas renderer into an offscreen canvas the page then encodes as
//! PNG.

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::gpu::Gpu;
use crate::wasm_api::WasmSession;

/// The renderer of the offscreen thumbnail canvas. One for the page's lifetime, apart from every
/// session, so attaching it never holds a session across an `await`.
#[wasm_bindgen]
pub struct WasmThumbnail {
    gpu: Gpu,
    css_width: f64,
    css_height: f64,
}

#[wasm_bindgen]
impl WasmThumbnail {
    /// The renderer of `canvas`, `width` x `height` physical pixels at `device_pixel_ratio`.
    ///
    /// # Errors
    /// A `JsValue` (a plain string) if no adapter or device could be obtained.
    pub async fn attach(
        canvas: HtmlCanvasElement,
        width: u32,
        height: u32,
        device_pixel_ratio: f64,
    ) -> Result<WasmThumbnail, JsValue> {
        let gpu = Gpu::attach(canvas, width, height, device_pixel_ratio).await?;
        Ok(Self {
            gpu,
            css_width: f64::from(width) / device_pixel_ratio,
            css_height: f64::from(height) / device_pixel_ratio,
        })
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// Draws the active page into `thumbnail`'s canvas; the page reads the canvas in the same
    /// task.
    ///
    /// # Errors
    /// A `JsValue` (a plain string) if the frame could not be submitted.
    pub fn render_thumbnail(&mut self, thumbnail: &mut WasmThumbnail) -> Result<(), JsValue> {
        let (mut draw_list, view) = self
            .session
            .thumbnail_frame(thumbnail.css_width, thumbnail.css_height);
        let pictures = self.session.frame_images(&mut draw_list);
        thumbnail.gpu.render(&draw_list, view, &pictures)
    }
}
