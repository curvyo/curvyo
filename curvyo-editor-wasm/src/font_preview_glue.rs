//! The font preview's faces, tiles and glyph table for the frontend (`specs/0077-font-preview`
//! criteria 11, 13, 15, 17 and 18, `adrs.md` decisions 2 and 3; `specs/0082-glyph-table`
//! `adrs.md` decisions 2, 3 and 6): bytes cross once per face, then each tile, table or row batch
//! of cells is one call, answered as JSON.

use curvyo_text_core::{
    FaceKind, FontFormat, UnicodeNames, block_of, font_info, glyph_cell, preview_path_within,
};
use curvyo_ui_core::{MAX_GLYPH_QUERY_CHARS, NO_BLOCK, PreviewFaces, glyph_table};
use serde_json::json;

/// The faces the preview holds, within their byte budget, and the Unicode names once loaded.
#[derive(Debug, Default)]
pub struct PreviewFacesGlue {
    faces: PreviewFaces,
    names: Option<UnicodeNames>,
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

impl PreviewFacesGlue {
    /// The info fields of the face `id` (`specs/0083-font-info-and-zip` criterion 2) as JSON
    /// `{"format": {"kind", "face", "faces"}, "full_name", "version", "designer", "vendor",
    /// "copyright", "trademark", "licence", "licence_url", "fs_type", "glyphs", "characters"}`,
    /// `null` where the font states nothing; empty when the face is not held.
    #[must_use]
    pub fn font_info(&mut self, id: &str) -> String {
        let Some(face) = self.faces.get(id) else {
            return String::new();
        };
        let info = font_info(&face);
        let format = match info.format {
            FontFormat::TrueType => json!({ "kind": "truetype" }),
            FontFormat::OpenTypeCff => json!({ "kind": "cff" }),
            FontFormat::OpenTypeCff2 => json!({ "kind": "cff2" }),
            FontFormat::Collection { face, faces } => {
                json!({ "kind": "collection", "face": face, "faces": faces })
            }
            FontFormat::SvgFont => json!({ "kind": "svg" }),
        };
        json!({
            "format": format,
            "full_name": info.full_name,
            "version": info.version,
            "designer": info.designer,
            "vendor": info.vendor,
            "copyright": info.copyright,
            "trademark": info.trademark,
            "licence": info.licence,
            "licence_url": info.licence_url,
            "fs_type": info.fs_type,
            "glyphs": info.glyphs,
            "characters": face.listed_chars().len(),
        })
        .to_string()
    }

    /// Takes the shipped Unicode names file (fetched on the first Glyphs open); `false` for
    /// bytes that are not one.
    pub fn set_unicode_names(&mut self, bytes: &[u8]) -> bool {
        self.names = UnicodeNames::parse(bytes).ok();
        self.names.is_some()
    }

    /// Whether the Unicode names are loaded.
    #[must_use]
    pub const fn has_unicode_names(&self) -> bool {
        self.names.is_some()
    }

    /// The glyph table of the face `id` for `query` (at most 64 characters used) as JSON
    /// `{"total", "shown", "single_line", "groups": [{"name", "cells": [code points]}]}`; empty
    /// when the face is not held.
    #[must_use]
    pub fn glyph_table(&mut self, id: &str, query: &str) -> String {
        let Some(face) = self.faces.get(id) else {
            return String::new();
        };
        let list = face.listed_chars();
        let query: String = query.chars().take(MAX_GLYPH_QUERY_CHARS).collect();
        let table = glyph_table(&list, &query, self.names.as_ref());
        let groups: Vec<serde_json::Value> = table
            .groups
            .iter()
            .map(|group| {
                let cells: Vec<u32> = group.cells.iter().map(|&i| u32::from(list[i].c)).collect();
                json!({ "name": group.name, "cells": cells })
            })
            .collect();
        json!({
            "total": list.len(),
            "shown": table.shown,
            "single_line": face.kind() == FaceKind::SingleLine,
            "groups": groups,
        })
        .to_string()
    }

    /// The cells of `code_points` in the face `id` as JSON `[{"d", "adv", "empty"}]` in cap-height
    /// units (`null` for a code point the face does not draw); empty when the face is not held.
    #[must_use]
    pub fn glyph_cells(&mut self, id: &str, code_points: &[u32]) -> String {
        let Some(face) = self.faces.get(id) else {
            return String::new();
        };
        let cells: Vec<serde_json::Value> = code_points
            .iter()
            .map(|&code| {
                char::from_u32(code)
                    .and_then(|c| glyph_cell(&face, c))
                    .map_or(
                        serde_json::Value::Null,
                        |cell| json!({ "d": cell.d, "adv": cell.advance, "empty": cell.empty }),
                    )
            })
            .collect();
        serde_json::Value::Array(cells).to_string()
    }

    /// The detail bar's text for `code_point` in the face `id`, as JSON `{"name", "glyph_name",
    /// "block", "abbreviation"}` (`null` where there is none; `name` and `abbreviation` are
    /// `null` until the names are loaded).
    #[must_use]
    pub fn glyph_detail(&mut self, id: &str, code_point: u32) -> String {
        let Some(c) = char::from_u32(code_point) else {
            return String::new();
        };
        let glyph_name = self.faces.get(id).and_then(|face| {
            let list = face.listed_chars();
            list.binary_search_by_key(&c, |l| l.c)
                .ok()
                .and_then(|at| list[at].name.as_deref().map(str::to_owned))
        });
        json!({
            "name": self.names.as_ref().and_then(|names| names.name_of(c)),
            "glyph_name": glyph_name,
            "block": block_of(c).unwrap_or(NO_BLOCK),
            "abbreviation": self.names.as_ref().and_then(|names| names.abbreviation_of(c)),
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
    fn font_info_crosses_as_json() {
        let mut glue = PreviewFacesGlue::default();
        assert!(glue.add("sans#0", &font("source-sans-3/SourceSans3-Regular.otf"), 0));
        let info: serde_json::Value = serde_json::from_str(&glue.font_info("sans#0")).unwrap();
        assert_eq!(info["format"]["kind"], "cff");
        assert!(info["characters"].as_u64().unwrap() > 500);
        assert_eq!(glue.font_info("nothing"), "");
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

    fn names() -> Vec<u8> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        std::fs::read(dir.join("../frontend/public/unicode/names-17.0.0.bin")).unwrap()
    }

    #[test]
    fn the_glyph_table_groups_searches_and_draws_cells() {
        let mut glue = PreviewFacesGlue::default();
        assert!(glue.add("sans#0", &font("source-sans-3/SourceSans3-Regular.otf"), 0));
        let table: serde_json::Value =
            serde_json::from_str(&glue.glyph_table("sans#0", "")).unwrap();
        assert_eq!(table["groups"][0]["name"], "Basic Latin");
        assert_eq!(table["total"], table["shown"]);
        // A word finds nothing by name until the names arrive.
        let heart: serde_json::Value =
            serde_json::from_str(&glue.glyph_table("sans#0", "diaeresis")).unwrap();
        assert_eq!(heart["shown"], 0);
        assert!(!glue.has_unicode_names());
        assert!(glue.set_unicode_names(&names()));
        let found: serde_json::Value =
            serde_json::from_str(&glue.glyph_table("sans#0", "a diaeresis")).unwrap();
        assert!(
            found["groups"][0]["cells"]
                .as_array()
                .unwrap()
                .contains(&json!(0xE4))
        );
        let cells: serde_json::Value =
            serde_json::from_str(&glue.glyph_cells("sans#0", &[0x48, 0x20, 0x6F22])).unwrap();
        assert_ne!(cells[0]["d"], "");
        assert_eq!(cells[1]["empty"], true);
        assert_eq!(cells[2], serde_json::Value::Null);
        let detail: serde_json::Value =
            serde_json::from_str(&glue.glyph_detail("sans#0", 0xE4)).unwrap();
        assert_eq!(detail["name"], "LATIN SMALL LETTER A WITH DIAERESIS");
        assert_eq!(detail["glyph_name"], "adieresis");
        assert_eq!(detail["block"], "Latin-1 Supplement");
        let space: serde_json::Value =
            serde_json::from_str(&glue.glyph_detail("sans#0", 0x20)).unwrap();
        assert_eq!(space["abbreviation"], "SP");
        assert!(!glue.set_unicode_names(b"nope"));
    }

    #[test]
    fn a_face_not_held_or_not_a_font_gives_nothing() {
        let mut glue = PreviewFacesGlue::default();
        assert!(!glue.add("x", b"not a font", 0));
        assert!(!glue.has("x"));
        assert_eq!(glue.tile("x", "Anna"), "");
    }
}
