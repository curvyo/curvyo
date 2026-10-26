//! The font library's `wasm-bindgen` surface (`specs/0075-font-library`): the maker's library,
//! the check cache and the faces of a font file the browser adds. Every method is a
//! pass-through to the plain-Rust glue; strings and scalars only (ADR 0001 §5).

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;
use crate::{FontArchiveGlue, FontChecksGlue, FontLibraryGlue, PreviewFacesGlue};

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

/// The faces the font preview draws (`specs/0077-font-preview`, `adrs.md` decision 2): one per
/// page, within a byte budget.
#[wasm_bindgen]
#[derive(Debug, Default)]
pub struct WasmPreviewFaces {
    glue: PreviewFacesGlue,
}

#[wasm_bindgen]
impl WasmPreviewFaces {
    /// An empty store.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Keeps face `index` of `bytes` under `id`; `false` for bytes that hold no usable face.
    pub fn add_preview_face(&mut self, id: &str, bytes: &[u8], index: u32) -> bool {
        self.glue.add(id, bytes, index)
    }

    /// Whether the face `id` is held.
    #[must_use]
    pub fn has_preview_face(&self, id: &str) -> bool {
        self.glue.has(id)
    }

    /// A tile's sample in the face `id` up to `width_caps` cap heights, as JSON (see
    /// `PreviewFacesGlue::tile`).
    pub fn preview_tile(&mut self, id: &str, text: &str, width_caps: f64) -> String {
        self.glue.tile_within(id, text, width_caps)
    }

    /// Takes the Unicode names file (`specs/0082-glyph-table` criterion 17).
    pub fn set_unicode_names(&mut self, bytes: &[u8]) -> bool {
        self.glue.set_unicode_names(bytes)
    }

    /// Whether the Unicode names are loaded.
    #[must_use]
    pub fn has_unicode_names(&self) -> bool {
        self.glue.has_unicode_names()
    }

    /// The glyph table of the face `id` for `query`, as JSON (see `PreviewFacesGlue::glyph_table`).
    pub fn glyph_table(&mut self, id: &str, query: &str) -> String {
        self.glue.glyph_table(id, query)
    }

    /// One batch of glyph table cells, as JSON (see `PreviewFacesGlue::glyph_cells`).
    pub fn glyph_cells(&mut self, id: &str, code_points: &[u32]) -> String {
        self.glue.glyph_cells(id, code_points)
    }

    /// The info fields of the face `id`, as JSON (see `PreviewFacesGlue::font_info`).
    pub fn font_info(&mut self, id: &str) -> String {
        self.glue.font_info(id)
    }

    /// The detail bar's names of one character, as JSON (see `PreviewFacesGlue::glyph_detail`).
    pub fn glyph_detail(&mut self, id: &str, code_point: u32) -> String {
        self.glue.glyph_detail(id, code_point)
    }
}

/// The faces of a font file the browser's Add font accepts, as JSON.
#[wasm_bindgen]
#[must_use]
pub fn font_file_faces(bytes: &[u8]) -> String {
    crate::font_file_faces(bytes)
}

/// A single-line face's name drawn in its own centre lines, for its Fonts-tab row, as JSON
/// (`specs/0076-single-line-fonts` criterion 6).
#[wasm_bindgen]
#[must_use]
pub fn face_preview_path(bytes: &[u8], index: u32, text: &str) -> String {
    crate::face_preview_path(bytes, index, text)
}

#[wasm_bindgen]
impl WasmSession {
    /// The texts of the open project using a face of `family` from the Curvyo fonts block.
    #[must_use]
    pub fn texts_using_family(&self, family: &str) -> u32 {
        u32::try_from(self.session.texts_using_family(family)).unwrap_or(u32::MAX)
    }
}

/// A file or folder name made safe for Windows, macOS and Linux (`specs/0083-font-info-and-zip`
/// criterion 9): the browser names a font folder with it.
#[wasm_bindgen]
#[must_use]
pub fn sanitised_name(name: &str) -> String {
    curvyo_text_core::sanitised_name(name)
}

/// A zip archive of fonts the browser's Add font reads (`specs/0083-font-info-and-zip`).
#[wasm_bindgen]
#[derive(Debug)]
pub struct WasmFontArchive {
    glue: FontArchiveGlue,
}

#[wasm_bindgen]
impl WasmFontArchive {
    /// Opens the archive's bytes; the error is `"unreadable"` or `"over_limit"`.
    ///
    /// # Errors
    /// The refusal's code when the archive is refused whole.
    pub fn open(bytes: Vec<u8>) -> Result<WasmFontArchive, String> {
        FontArchiveGlue::open(bytes)
            .map(|glue| Self { glue })
            .map_err(str::to_owned)
    }

    /// The entries as JSON (see `FontArchiveGlue::entries`).
    #[must_use]
    pub fn entries(&self) -> String {
        self.glue.entries()
    }

    /// The bytes of font or document entry `index`; the error is `"unsafe"` or `"over_limit"`.
    ///
    /// # Errors
    /// The skip's code.
    pub fn read(&mut self, index: usize) -> Result<Vec<u8>, String> {
        self.glue.read(index).map_err(str::to_owned)
    }
}
