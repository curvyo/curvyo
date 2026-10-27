//! The included faces as the host lists and loads them, for the SVG import tests: every face of
//! `frontend/public/fonts/index.json` listed in the Included block and loaded into a store.
#![allow(clippy::unwrap_used, dead_code)]

use std::path::PathBuf;

use curvyo_document_core::FontKind;
use curvyo_text_core::{FaceKind, FaceStore};
use curvyo_ui_core::{
    FontBlock, ImportFaces, ListedFace, SvgImport, SvgPlacement, substitute_face, svg_import,
};

/// The included faces listed and loaded.
pub struct Fonts {
    pub faces: Vec<ListedFace>,
    pub store: FaceStore,
}

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

impl Fonts {
    pub fn included() -> Self {
        let index: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
                .unwrap();
        let mut store = FaceStore::new();
        let mut faces = Vec::new();
        for face in index["faces"].as_array().unwrap() {
            let file = face["file"].as_str().unwrap();
            let bytes = std::fs::read(fonts_dir().join(file)).unwrap();
            let key = store.add(bytes, 0).unwrap();
            let kind = match store.get(&key).unwrap().kind() {
                FaceKind::Outline => FontKind::Outline,
                FaceKind::SingleLine => FontKind::SingleLine,
            };
            faces.push(ListedFace {
                block: FontBlock::Included,
                family: face["family"].as_str().unwrap().to_owned(),
                style: face["style"].as_str().unwrap().to_owned(),
                host_id: file.to_owned(),
                index: 0,
                key: Some(key),
                kind,
            });
        }
        Self { faces, store }
    }

    pub fn import(&self, bytes: &[u8], placement: SvgPlacement) -> SvgImport {
        let drawing = curvyo_svg_core::read_svg(bytes).unwrap();
        self.import_drawing(&drawing, placement)
    }

    pub fn import_drawing(
        &self,
        drawing: &curvyo_svg_core::SvgDrawing,
        placement: SvgPlacement,
    ) -> SvgImport {
        let substitute = substitute_face(&self.faces).and_then(|at| self.faces[at].key);
        svg_import(
            drawing,
            placement,
            &ImportFaces {
                faces: &self.faces,
                store: &self.store,
                substitute,
            },
            512 * 1024 * 1024,
        )
    }
}
