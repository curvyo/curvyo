//! Shared helpers of the text-core tests: the included fonts as fixtures.
#![allow(clippy::unwrap_used, dead_code)]

pub mod sfnt;

use std::path::PathBuf;

use curvyo_text_core::{Face, FaceKey, FaceStore};

/// The folder of the included fonts (ADR 0020 §3), the engine's fixture faces.
pub fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

/// The bytes of an included font file, by its path under the fonts folder.
pub fn font_bytes(path: &str) -> Vec<u8> {
    std::fs::read(fonts_dir().join(path)).unwrap()
}

/// Source Serif 4 Regular (TrueType outlines, GPOS kerning, an fi ligature).
pub const SERIF: &str = "source-serif-4/SourceSerif4-Regular.ttf";
/// Source Sans 3 Regular (CFF outlines).
pub const SANS: &str = "source-sans-3/SourceSans3-Regular.otf";

/// A store holding one included face, and its key.
pub fn store_with(path: &str) -> (FaceStore, FaceKey) {
    let mut store = FaceStore::new();
    let key = store.add(font_bytes(path), 0).unwrap();
    (store, key)
}

/// The face of a store built by [`store_with`].
pub fn face<'a>(store: &'a FaceStore, key: &FaceKey) -> &'a Face {
    store.get(key).unwrap()
}
