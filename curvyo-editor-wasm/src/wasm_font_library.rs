//! The font library's `wasm-bindgen` surface (`specs/0075-font-library`): the maker's library,
//! the check cache and the faces of a font file the browser adds. Every method is a
//! pass-through to the plain-Rust glue; strings and scalars only (ADR 0001 §5).

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;
use crate::{FontChecksGlue, FontLibraryGlue};

/// The maker's font library (one per page, it outlives a project's session).
#[wasm_bindgen]
#[derive(Debug, Default)]
pub struct WasmFontLibrary {
    glue: FontLibraryGlue,
}

#[wasm_bindgen]
impl WasmFontLibrary {
    /// An empty library: no stars, no own tags, All chosen.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads the file text: `"ok"`, `"newer"` or `"broken"`.
    pub fn load(&mut self, text: &str) -> String {
        self.glue.load(text).to_owned()
    }

    /// The canonical file text.
    #[must_use]
    pub fn text(&self) -> String {
        self.glue.text()
    }

    /// What is known of the listed families, as JSON (see `FontLibraryGlue::set_families`).
    pub fn set_families(&mut self, json: &str) {
        self.glue.set_families(json);
    }

    /// One check result (`-1` unreadable).
    pub fn set_check(&mut self, name: &str, pieces: i32) {
        self.glue.set_check(name, i64::from(pieces));
    }

    /// The strip's, the list's and the tag lines' view for the listed family names (JSON).
    #[must_use]
    pub fn view(&self, names: &str) -> String {
        self.glue.view(names)
    }

    /// Stars or unstars a family; the new state.
    pub fn toggle_favourite(&mut self, name: &str) -> bool {
        self.glue.toggle_favourite(name)
    }

    /// Puts a tag on a family, as JSON `{"changed"}` or `{"error"}`.
    pub fn add_tag(&mut self, name: &str, tag: &str) -> String {
        self.glue.add_tag(name, tag)
    }

    /// Takes a tag off a family.
    pub fn remove_tag(&mut self, name: &str, id: &str) -> bool {
        self.glue.remove_tag(name, id)
    }

    /// The add field's completions, as a JSON array.
    #[must_use]
    pub fn completions(&self, name: &str, prefix: &str) -> String {
        self.glue.completions(name, prefix)
    }

    /// Rename, as JSON `{}` or `{"error", "taken"?}`.
    pub fn rename_tag(&mut self, id: &str, name: &str) -> String {
        self.glue.rename_tag(id, name)
    }

    /// Deletes an own tag from every family.
    pub fn delete_tag(&mut self, id: &str) -> bool {
        self.glue.delete_tag(id)
    }

    /// Chooses a strip cell.
    pub fn choose(&mut self, id: &str) -> bool {
        self.glue.choose(id)
    }
}

/// The cuttable check with its cache.
#[wasm_bindgen]
#[derive(Debug, Default)]
pub struct WasmFontChecks {
    glue: FontChecksGlue,
}

#[wasm_bindgen]
impl WasmFontChecks {
    /// An empty cache.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads the cache text.
    pub fn load(&mut self, text: &str) {
        self.glue.load(text);
    }

    /// The cache text.
    #[must_use]
    pub fn text(&self) -> String {
        self.glue.text()
    }

    /// Whether the system fonts were asked for on this computer.
    #[must_use]
    pub fn system_requested(&self) -> bool {
        self.glue.system_requested()
    }

    /// Marks the system fonts as asked for; whether that is new.
    pub fn request_system(&mut self) -> bool {
        self.glue.request_system()
    }

    /// The cached result of a listed face whose stamp matches: pieces, `-1`, or `-2`.
    #[must_use]
    pub fn by_stamp(&self, host_id: &str, index: u32, stamp: &str) -> i32 {
        i32::try_from(self.glue.by_stamp(host_id, index, stamp)).unwrap_or(i32::MAX)
    }

    /// Checks one face (or finds it by hash) and keeps the result: pieces or `-1`.
    pub fn check(&mut self, host_id: &str, index: u32, stamp: &str, bytes: &[u8]) -> i32 {
        i32::try_from(self.glue.check(host_id, index, stamp, bytes)).unwrap_or(i32::MAX)
    }
}

/// The faces of a font file the browser's Add font accepts, as JSON.
#[wasm_bindgen]
#[must_use]
pub fn font_file_faces(bytes: &[u8]) -> String {
    crate::font_file_faces(bytes)
}

#[wasm_bindgen]
impl WasmSession {
    /// The texts of the open project using a face of `family` from the Curvyo fonts block.
    #[must_use]
    pub fn texts_using_family(&self, family: &str) -> u32 {
        u32::try_from(self.session.texts_using_family(family)).unwrap_or(u32::MAX)
    }
}
