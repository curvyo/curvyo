//! The font preview's faces and tiles for the frontend (`specs/0077-font-preview` criteria 11,
//! 13, 15, 17 and 18, `adrs.md` decisions 2 and 3): bytes cross once per face, then each tile is
//! one call with the sample, answered as JSON.

use curvyo_text_core::{FaceKind, preview_path_within};
use curvyo_ui_core::PreviewFaces;
use serde_json::json;

/// The faces the preview holds, within their byte budget.
#[derive(Debug, Default)]
pub struct PreviewFacesGlue {
    faces: PreviewFaces,
}

impl PreviewFacesGlue {
    /// Keeps face `index` of `bytes` under `id`; `false` for bytes that hold no usable face.
    pub fn add(&mut self, id: &str, bytes: &[u8], index: u32) -> bool {
        self.faces.add(id, bytes.to_vec(), index).is_ok()
    }

    /// Whether the face `id` is held.
    #[must_use]
    pub fn has(&self, id: &str) -> bool {
        self.faces.has(id)
    }

    /// `text` in the face `id` as one line, however wide (the tile clips it at its edge,
    /// customer decision 2026-10-23), as JSON
    /// `{"d", "box": [x, y, width, height], "cap", "missing": [..], "single_line"}` in em units,
    /// y down, origin on the baseline; `missing` lists the characters the face has no glyph for.
    /// Empty when the face is not held.
    #[must_use]
    pub fn tile(&mut self, id: &str, text: &str) -> String {
        self.tile_within(id, text, f64::INFINITY)
    }

    /// As [`Self::tile`], but drawn only up to `width_caps` cap heights from the line start: the
    /// glyphs past the tile's edge are dropped, the one straddling it is kept.
    #[must_use]
    pub fn tile_within(&mut self, id: &str, text: &str, width_caps: f64) -> String {
        let Some(face) = self.faces.get(id) else {
            return String::new();
        };
        let Some((d, area)) = preview_path_within(&face, text, width_caps) else {
            return String::new();
        };
        let missing: Vec<String> = face
            .missing_chars(text)
            .into_iter()
            .map(String::from)
            .collect();
        json!({
            "d": d,
            "box": [area.x0, area.y0, area.width(), area.height()],
            "cap": face.metrics().cap_height,
            "missing": missing,
            "single_line": face.kind() == FaceKind::SingleLine,
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font(path: &str) -> Vec<u8> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        std::fs::read(dir.join("../frontend/public/fonts").join(path)).unwrap()
    }

    fn tile(glue: &mut PreviewFacesGlue, id: &str, text: &str) -> serde_json::Value {
        serde_json::from_str(&glue.tile(id, text)).unwrap()
    }

    #[test]
    fn a_tile_carries_its_path_box_and_missing_characters() {
        let mut glue = PreviewFacesGlue::default();
        assert!(glue.add("sans#0", &font("source-sans-3/SourceSans3-Regular.otf"), 0));
        assert!(glue.add("script#0", &font("hershey-script-1/HersheyScript1.svg"), 0));
        let sans = tile(&mut glue, "sans#0", "Jürgen 漢");
        assert_ne!(sans["d"].as_str().unwrap(), "");
        assert_eq!(sans["missing"], json!(["漢"]));
        assert_eq!(sans["single_line"], json!(false));
        assert!(sans["cap"].as_f64().unwrap() > 0.5);
        let script = tile(&mut glue, "script#0", "Anna");
        assert_eq!(script["single_line"], json!(true));
        assert_eq!(script["missing"], json!([]));
    }

    #[test]
    fn a_face_not_held_or_not_a_font_gives_nothing() {
        let mut glue = PreviewFacesGlue::default();
        assert!(!glue.add("x", b"not a font", 0));
        assert!(!glue.has("x"));
        assert_eq!(glue.tile("x", "Anna"), "");
    }
}
