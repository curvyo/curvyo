//! The `wasm-bindgen` surface of Ctrl+C and Ctrl+V for images (`specs/0087-image-clipboard-and-drop`
//! criteria 1 to 16, `adrs.md` decisions 1 to 3): the chord gate, the copied pixels, the PNG the
//! worker encodes, which clipboard file a paste takes, and the notices.

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

/// The stored pixels Ctrl+C copies.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct ImageCopyJs {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Straight-alpha RGBA8, top row first.
    #[wasm_bindgen(getter_with_clone)]
    pub rgba: Vec<u8>,
}

#[wasm_bindgen]
impl WasmSession {
    /// Whether Ctrl+C and Ctrl+V may act on images now: not while the page reports it `blocked`,
    /// during a drag, with an entry open or while a text is edited (criteria 3 and 16).
    #[must_use]
    pub fn clipboard_allowed(&self, blocked: bool) -> bool {
        self.session.clipboard_allowed(blocked)
    }

    /// The topmost selected image's stored pixels, or `undefined` when the selection holds no
    /// image (criteria 12 and 13).
    #[must_use]
    pub fn image_copy(&mut self) -> Option<ImageCopyJs> {
        let copy = self.session.image_copy()?;
        Some(ImageCopyJs {
            width: copy.width,
            height: copy.height,
            rgba: copy.rgba,
        })
    }
}

/// The PNG of `rgba` (`width` x `height`, straight alpha) for the clipboard, in the worker
/// (criteria 12 and 14).
///
/// # Errors
/// The reason in words when the pixels are over the working limits or do not match the size.
#[wasm_bindgen]
pub fn encode_png(rgba: Vec<u8>, width: u32, height: u32) -> Result<Vec<u8>, JsError> {
    crate::clipboard_glue::encode_rgba_png(rgba, width, height)
        .map_err(|reason| JsError::new(&reason))
}

/// How a clipboard file of `bytes` labelled `mime` serves a paste: 2 an image by its content, 1 a
/// raster type the import refuses with its notice, 0 nothing to paste (criteria 4, 5 and 7).
#[wasm_bindgen]
#[must_use]
pub fn paste_rank(bytes: &[u8], mime: &str) -> u8 {
    crate::clipboard_glue::paste_rank_code(bytes, mime)
}

/// A notice of the image clipboard by `kind` (`"nothing"`, `"pasting"`,
/// `"copying"`, `"failed"` with `reason`, `"newer-browser"`, `"link"`; criteria 5, 14, 15,
/// 21).
#[wasm_bindgen]
#[must_use]
pub fn clipboard_notice(kind: &str, reason: &str) -> String {
    crate::clipboard_glue::clipboard_notice(kind, reason)
}
