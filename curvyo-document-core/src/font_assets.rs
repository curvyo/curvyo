//! Which font files a document's texts use, for embedding them as assets (ADR 0047): the
//! font bytes live in the asset store under the hash a text's [`FontRef`] already carries.

use std::collections::HashSet;

use crate::assets::AssetHash;
use crate::document::Document;
use crate::text_model::FontRef;

impl Document {
    /// The faces the texts of the tree use, one entry per distinct `(file hash, index)`, in the
    /// order of first use (ADR 0047 decision 2).
    #[must_use]
    pub fn used_fonts(&self) -> Vec<FontRef> {
        let mut seen: HashSet<(String, u32)> = HashSet::new();
        let mut fonts = Vec::new();
        for id in self.object_ids() {
            let Some(text) = self.text(id) else {
                continue;
            };
            if seen.insert((text.font.face_hash.clone(), text.font.face_index)) {
                fonts.push(text.font);
            }
        }
        fonts
    }

    /// The hashes of the font files the texts of the tree use.
    #[must_use]
    pub fn used_font_hashes(&self) -> HashSet<AssetHash> {
        self.used_fonts()
            .iter()
            .filter_map(|font| AssetHash::parse(&font.face_hash))
            .collect()
    }
}
