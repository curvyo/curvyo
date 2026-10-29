//! The `wasm-bindgen` surface of the Import button (`specs/0058-bitmap-import` criteria 1 to 5,
//! 26, 30): begin, add each file, finish; the busy notice before a long read; the notice of an
//! unreadable picture after an Open.

use wasm_bindgen::prelude::*;

use crate::session::{ImportOrigin, ImportOutcome};
use crate::wasm_api::WasmSession;

/// What an import did.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct ImportOutcomeView {
    /// How many objects came in, selected.
    pub imported: u32,
    /// The notice, lines separated by `\n`; empty for none.
    #[wasm_bindgen(getter_with_clone)]
    pub notice: String,
    /// Whether the notice reports a failure.
    pub alert: bool,
    /// The faces of an SVG's texts are loading: nothing changed, call `import_finish` again.
    pub waiting: bool,
}

impl From<ImportOutcome> for ImportOutcomeView {
    fn from(outcome: ImportOutcome) -> Self {
        Self {
            imported: u32::try_from(outcome.imported).unwrap_or(u32::MAX),
            notice: outcome.notice,
            alert: outcome.alert,
            waiting: outcome.waiting,
        }
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// Starts an import from `origin` (`"picker"`, `"drop"`, `"paste"` or `"paste-files"`, local
    /// files a paste named; anything else reads as the picker): centred on (`x`, `y`) in canvas CSS pixels when `placed` (the drop point, the
    /// pointer of a paste), else on the view centre (`specs/0087-image-clipboard-and-drop`
    /// criterion 2).
    pub fn import_begin(&mut self, origin: &str, placed: bool, x: f64, y: f64) {
        let origin = match origin {
            "drop" => ImportOrigin::Drop,
            "paste" => ImportOrigin::Paste,
            "paste-files" => ImportOrigin::PastedFiles,
            _ => ImportOrigin::Picker,
        };
        self.session
            .import_begin_from(origin, placed.then_some((x, y)));
    }

    /// Reads one file of the import (decoded or parsed at once; files past the 20th are
    /// skipped).
    pub fn import_add(&mut self, bytes: &[u8], file_name: &str) {
        self.session.import_add(bytes, file_name);
    }

    /// Refuses a file of the import that the host did not read because it is `bytes` long, over
    /// the import's file limit (a local file a paste or drop named).
    pub fn import_add_too_large(&mut self, file_name: &str, bytes: f64) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let bytes = bytes.max(0.0) as u64;
        self.session.import_add_too_large(file_name, bytes);
    }

    /// Commits the import as one step and selects what came in; while the faces of an SVG's
    /// texts load it says `waiting` and changes nothing (with `force`, it commits with the faces
    /// that are loaded).
    pub fn import_finish(&mut self, force: bool) -> ImportOutcomeView {
        self.session.import_finish(force).into()
    }

    /// "An image in this project could not be read." once per open; empty otherwise.
    pub fn take_bad_asset_notice(&mut self) -> String {
        self.session.take_bad_asset_notice()
    }
}

/// The busy notice before file `at` of `of` is read: "Importing 3 of 12 images..." for several
/// files, "Importing photo.jpg..." for one of 5 MB or more or a header of 8 megapixels or more;
/// empty when the file is read too fast to need one (UX notes, "Busy state").
#[wasm_bindgen]
#[must_use]
pub fn import_busy_notice(bytes: &[u8], file_name: &str, at: u32, of: u32) -> String {
    const LARGE_FILE: usize = 5 * 1024 * 1024;
    const LARGE_HEADER: u64 = 8_000_000;
    let large_header = curvyo_image_core::probe(bytes).is_ok_and(|info| {
        u64::from(info.stored_width) * u64::from(info.stored_height) >= LARGE_HEADER
    });
    if of > 1 || bytes.len() >= LARGE_FILE || large_header {
        curvyo_ui_core::importing_notice(file_name, at as usize, of as usize)
    } else {
        String::new()
    }
}
