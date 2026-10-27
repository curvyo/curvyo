//! The `wasm-bindgen` surface of SVG files (`specs/0024-svg-import-export`): Open… of any file by
//! content, Import SVG…, and the words the host shows around them.

use wasm_bindgen::prelude::*;

use crate::session::{Session, SvgImportOutcome};
use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// Open… (criterion 1): `bytes` read as an SVG or a project by their content. An SVG opens as
    /// an untitled, dirty project ([`WasmSession::opened_svg`]).
    ///
    /// # Errors
    /// A `JsValue` holding the error dialog's sentence; nothing changed.
    pub fn open_file(peer: u64, bytes: &[u8], file_name: &str) -> Result<WasmSession, JsValue> {
        let mut session =
            Session::open_file(peer, bytes, file_name).map_err(|m| JsValue::from_str(&m))?;
        session.show_default_view();
        Ok(Self { session, gpu: None })
    }

    /// Whether the project was opened from an SVG and exists in no saved file yet.
    #[must_use]
    pub fn opened_svg(&self) -> bool {
        self.session.opened_svg()
    }

    /// Whether an opened SVG still waits for the faces of its texts (criterion 106).
    #[must_use]
    pub fn open_svg_waiting(&self) -> bool {
        self.session.open_svg_waiting()
    }

    /// Converts an opened SVG once the faces of its texts are loaded (with `force`, with the
    /// faces that are loaded); returns whether it still waits.
    pub fn continue_open_svg(&mut self, force: bool) -> bool {
        self.session.continue_open_svg(force)
    }

    /// The notice of an Open of an SVG that brought in nothing (criterion 8), once.
    pub fn take_open_notice(&mut self) -> String {
        self.session.take_open_notice()
    }

    /// Import SVG… (criterion 3): `"imported:<n>"` with the objects that came in, selected;
    /// `"nothing:<notice>"` when nothing could be imported; `"error:<sentence>"` when the file
    /// gives no drawing; `"waiting"` while the faces of its texts load (call again). Only
    /// `"imported"` changed anything.
    pub fn import_svg(&mut self, bytes: &[u8], file_name: &str) -> String {
        match self.session.import_svg(bytes, file_name) {
            Ok(SvgImportOutcome::Imported(count)) => format!("imported:{count}"),
            Ok(SvgImportOutcome::Nothing(notice)) => format!("nothing:{notice}"),
            Ok(SvgImportOutcome::Waiting) => "waiting".to_owned(),
            Err(message) => format!("error:{message}"),
        }
    }
}

/// The notice while a file of 1 MiB or more is read: "Opening poster.svg..." or
/// "Importing poster.svg..." (UX notes, "Large files").
#[wasm_bindgen]
#[must_use]
pub fn svg_busy_notice(opening: bool, file_name: &str) -> String {
    curvyo_ui_core::svg_busy_notice(opening, file_name)
}
