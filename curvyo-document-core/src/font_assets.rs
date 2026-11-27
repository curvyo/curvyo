//! Which font files a document's texts use, for embedding them as assets (ADR 0047): the
//! font bytes live in the asset store under the hash a text's [`FontRef`] already carries. A generated
//! object's stored `{"font": {...}}` parameter (a barcode's human-readable text, `specs/0144` Frage 407 A,
//! ADR 0092 §3.6) names a face the same way and counts as a use.

use std::collections::HashSet;

use crate::assets::AssetHash;
use crate::document::Document;
use crate::text_model::FontRef;

/// The face of a stored `{"font": {"family", "style", "face_hash", "face_index"}}` value; `None` for
/// any other value (the included default is a `null` and names no file).
fn stored_font(value: &serde_json::Value) -> Option<FontRef> {
    let map = value.as_object()?.get("font")?.as_object()?;
    let text = |key: &str| map.get(key)?.as_str().map(str::to_owned);
    Some(FontRef {
        family: text("family")?,
        style: text("style")?,
        face_hash: text("face_hash")?,
        face_index: u32::try_from(map.get("face_index")?.as_u64()?).ok()?,
        kind: crate::text_model::FontKind::Outline,
    })
}

impl Document {
    /// The faces the texts of the tree and the `Font` parameters of generated objects use, one entry per distinct `(file hash, index)`, in the
    /// order of first use (ADR 0047 decision 2).
    #[must_use]
    pub fn used_fonts(&self) -> Vec<FontRef> {
        let mut seen: HashSet<(String, u32)> = HashSet::new();
        let mut fonts = Vec::new();
        let mut note = |font: FontRef| {
            if seen.insert((font.face_hash.clone(), font.face_index)) {
                fonts.push(font);
            }
        };
        for id in self.object_ids() {
            if let Some(text) = self.text(id) {
                note(text.font);
            } else if let Some(stored) = self.generated_params(id) {
                stored
                    .params
                    .values()
                    .filter_map(stored_font)
                    .for_each(&mut note);
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
