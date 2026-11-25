//! The Git catalogue index `curvyo-catalogue.json` (ADR 0085 decision 3, `specs/0138` criteria 6
//! and 9): parsed and checked in one place; an entry that breaks a rule is left out, the index
//! itself is refused only when it is not an index or over its limits.

use std::collections::HashSet;

use serde::Deserialize;

use crate::error::CatalogueError;
use crate::request::Request;

/// The index file at the catalogue's base address.
pub const CATALOGUE_FILE: &str = "curvyo-catalogue.json";
/// The `catalogue_format` this build reads.
pub const SUPPORTED_CATALOGUE_FORMAT: u32 = 1;
/// The index's size limit (ADR 0085 decision 3).
pub const MAX_INDEX_BYTES: usize = 4 * 1024 * 1024;
/// The number of templates an index may list.
pub const MAX_TEMPLATES: usize = 10_000;
/// A template picture's size limit.
pub const MAX_PICTURE_BYTES: u64 = 1024 * 1024;
/// A template file's size limit.
pub const MAX_TEMPLATE_BYTES: u64 = 64 * 1024 * 1024;

/// A template's licence as the index states it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, serde::Serialize)]
pub struct Licence {
    /// The SPDX identifier, empty when unknown.
    #[serde(default)]
    pub spdx: String,
    /// The licence's name.
    #[serde(default)]
    pub name: String,
    /// The licence text's address.
    #[serde(default)]
    pub url: String,
}

/// A font a template uses, with its licence.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FontEntry {
    /// The family's name.
    pub family: String,
    /// The style's name.
    #[serde(default)]
    pub style: String,
    /// The SPDX identifier of its licence, empty when unknown.
    #[serde(default)]
    pub spdx: String,
}

/// One template of the catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TemplateEntry {
    /// The folder, 1 to 4 segments.
    pub path: String,
    /// The name shown.
    pub name: String,
    /// The author.
    #[serde(default)]
    pub author: String,
    /// The tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// The licence; an index without one reads as "Licence unknown", never as free.
    #[serde(default)]
    pub licence: Option<Licence>,
    /// The credit text, required for licences that need attribution.
    #[serde(default)]
    pub credit: Option<String>,
    /// The fonts the template uses.
    #[serde(default)]
    pub fonts: Vec<FontEntry>,
    /// The template file's size in bytes.
    pub size: u64,
    /// The SHA-256 of the template file, lower-case hex.
    pub sha256: String,
    /// The SHA-256 of the thumbnail, lower-case hex.
    #[serde(default)]
    pub thumbnail_sha256: Option<String>,
    /// The `format_version` of the template file.
    #[serde(default)]
    pub format_version: Option<u32>,
}

impl TemplateEntry {
    /// The template file's request below the catalogue's `base`.
    #[must_use]
    pub fn template_request(&self, base: &str) -> Request {
        Request::below(base, &format!("{}/template.curvyo", self.path))
    }

    /// The thumbnail's request below the catalogue's `base`.
    #[must_use]
    pub fn thumbnail_request(&self, base: &str) -> Request {
        Request::below(base, &format!("{}/thumbnail.png", self.path))
    }

    /// Whether the file is over the size limit and is never fetched.
    #[must_use]
    pub const fn too_large(&self) -> bool {
        self.size > MAX_TEMPLATE_BYTES
    }

    /// Whether the licence is stated.
    #[must_use]
    pub fn licence_known(&self) -> bool {
        self.licence
            .as_ref()
            .is_some_and(|l| !l.spdx.trim().is_empty() || !l.name.trim().is_empty())
    }

    /// Whether the template's fonts are not fetched: one of them has an unknown licence
    /// (criterion 9). A template without fonts has nothing to leave out.
    #[must_use]
    pub fn fonts_withheld(&self) -> bool {
        self.fonts
            .iter()
            .any(|font| !font_licence_is_known(&font.spdx))
    }
}

/// Whether `spdx` names a font licence this build treats as allowing embedding.
#[must_use]
pub fn font_licence_is_known(spdx: &str) -> bool {
    matches!(
        spdx.trim(),
        "OFL-1.1"
            | "OFL-1.1-RFN"
            | "Apache-2.0"
            | "MIT"
            | "CC0-1.0"
            | "Unlicense"
            | "Ubuntu-font-1.0"
    )
}

/// Whether the licence `spdx` needs the author's credit (CC BY and its variants).
#[must_use]
pub fn needs_credit(spdx: &str) -> bool {
    spdx.trim().to_ascii_uppercase().starts_with("CC-BY")
}

/// A checked catalogue index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalogue {
    /// The catalogue's own name.
    pub name: String,
    /// The templates that passed the checks, in the index's order.
    pub templates: Vec<TemplateEntry>,
    /// How many entries were left out for breaking a rule.
    pub skipped: usize,
}

#[derive(Deserialize)]
struct Wire {
    #[serde(default)]
    catalogue_format: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    templates: Vec<serde_json::Value>,
}

impl Catalogue {
    /// The index request below the catalogue's `base`.
    #[must_use]
    pub fn request(base: &str) -> Request {
        Request::below(base, CATALOGUE_FILE)
    }

    /// Parses and checks an index.
    ///
    /// # Errors
    /// [`CatalogueError`] for a file that is not an index, a newer format or one over the limits.
    pub fn parse(bytes: &[u8]) -> Result<Self, CatalogueError> {
        if bytes.len() > MAX_INDEX_BYTES {
            return Err(CatalogueError::TooLarge);
        }
        let wire: Wire = serde_json::from_slice(bytes).map_err(|_| CatalogueError::Invalid)?;
        if wire.catalogue_format > SUPPORTED_CATALOGUE_FORMAT {
            return Err(CatalogueError::Newer);
        }
        if wire.catalogue_format == 0 {
            return Err(CatalogueError::Invalid);
        }
        if wire.templates.len() > MAX_TEMPLATES {
            return Err(CatalogueError::TooLarge);
        }
        let mut seen = HashSet::new();
        let mut templates = Vec::new();
        let mut skipped = 0;
        for value in wire.templates {
            match serde_json::from_value::<TemplateEntry>(value) {
                Ok(entry) if entry_ok(&entry) && seen.insert(entry.path.clone()) => {
                    templates.push(entry);
                }
                _ => skipped += 1,
            }
        }
        Ok(Self {
            name: wire.name,
            templates,
            skipped,
        })
    }

    /// The entry at `path`.
    #[must_use]
    pub fn entry(&self, path: &str) -> Option<&TemplateEntry> {
        self.templates.iter().find(|entry| entry.path == path)
    }
}

fn hex64(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The path rule of ADR 0085 decision 3: 1 to 4 segments of `[a-z0-9][a-z0-9._-]*`, no `..`, at
/// most 200 characters.
fn path_ok(path: &str) -> bool {
    if path.is_empty() || path.len() > 200 {
        return false;
    }
    let segments: Vec<&str> = path.split('/').collect();
    segments.len() <= 4
        && segments.iter().all(|s| {
            let mut chars = s.chars();
            chars
                .next()
                .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                && s.chars().all(|c| {
                    c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-')
                })
                && !s.contains("..")
        })
}

fn entry_ok(entry: &TemplateEntry) -> bool {
    let credit_ok = entry.licence.as_ref().is_none_or(|licence| {
        !needs_credit(&licence.spdx) || entry.credit.as_ref().is_some_and(|c| !c.trim().is_empty())
    });
    path_ok(&entry.path)
        && !entry.name.trim().is_empty()
        && hex64(&entry.sha256)
        && entry.thumbnail_sha256.as_deref().is_none_or(hex64)
        && credit_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_path_rule_holds() {
        for good in ["a", "boxes/finger-box", "a/b/c/d", "x.y_z-1"] {
            assert!(path_ok(good), "{good}");
        }
        let long = "a".repeat(201);
        for bad in [
            "",
            "a/b/c/d/e",
            "../x",
            "a/../b",
            "A",
            "-a",
            "a//b",
            "a b",
            "a..b",
            &long,
        ] {
            assert!(!path_ok(bad), "{bad}");
        }
    }

    #[test]
    fn credit_licences_are_recognised() {
        assert!(needs_credit("CC-BY-4.0"));
        assert!(needs_credit("CC-BY-SA-4.0"));
        assert!(!needs_credit("CC0-1.0"));
        assert!(!needs_credit("MIT"));
    }

    #[test]
    fn font_licences_are_an_allow_list() {
        assert!(font_licence_is_known("OFL-1.1"));
        assert!(!font_licence_is_known(""));
        assert!(!font_licence_is_known("LicenseRef-proprietary"));
    }
}
