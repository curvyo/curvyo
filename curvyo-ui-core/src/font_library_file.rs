//! Reads and writes the font library file, `font-library.toml` (`specs/0075-font-library`
//! criterion 12, `adrs.md` decisions 1 and 8): its text to a [`FontLibrary`] and back in one
//! canonical order, so reading and writing again gives the same text. It opens no file; the
//! platform layer moves the text.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::Deserialize;

use crate::family_key::family_key;
use crate::font_library::{
    ALL_ID, BuiltInTag, FontLibrary, MAX_LIST_KEYS, MAX_OWN_TAGS, Override, check_tag_name,
};

/// The schema of the file. Tags replaced groups before the file was ever released, so `format`
/// stayed 1 (2026-10-20). Format 2 adds the built-in tag `single_line`
/// (`specs/0076-single-line-fonts/adrs.md` decision 7); format 1 reads unchanged.
pub const FONT_LIBRARY_FORMAT: i64 = 2;
/// The largest file, in bytes, checked on the text before parsing.
pub const MAX_FONT_LIBRARY_BYTES: usize = 256 * 1024;

/// Why the text was not read. Nothing of it is used in either case.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FontLibraryFileError {
    /// The file's `format` is newer than this version's: it is never written over.
    #[error("the font library file was made by a newer version")]
    Newer,
    /// The file breaks a rule: it is kept until the first change sets it aside.
    #[error("the font library file is damaged: {0}")]
    Broken(String),
}

#[derive(Deserialize)]
struct RawHeader {
    format: Option<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    #[allow(dead_code)] // read by `RawHeader` first; kept so the strict parse knows the key
    format: i64,
    chosen: Option<String>,
    #[serde(default)]
    favourites: Vec<String>,
    #[serde(default)]
    overrides: BTreeMap<String, RawOverride>,
    /// Own tags by family key.
    #[serde(default)]
    tags: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOverride {
    #[serde(default)]
    added: Vec<String>,
    #[serde(default)]
    removed: Vec<String>,
}

fn broken(reason: impl Into<String>) -> FontLibraryFileError {
    FontLibraryFileError::Broken(reason.into())
}

/// The family keys of a list, normalised; more than [`MAX_LIST_KEYS`] is a broken file.
fn keys(list: &[String], what: &str) -> Result<BTreeSet<String>, FontLibraryFileError> {
    if list.len() > MAX_LIST_KEYS {
        return Err(broken(format!(
            "{what} holds more than {MAX_LIST_KEYS} fonts"
        )));
    }
    Ok(list
        .iter()
        .map(|name| family_key(name))
        .filter(|key| !key.is_empty())
        .collect())
}

/// The own tags by family: every name valid, not a built-in name, one spelling per tag.
fn read_tags(
    raw: BTreeMap<String, Vec<String>>,
) -> Result<BTreeMap<String, BTreeSet<String>>, FontLibraryFileError> {
    if raw.len() > MAX_LIST_KEYS {
        return Err(broken(format!("tags hold more than {MAX_LIST_KEYS} fonts")));
    }
    let mut spellings: BTreeMap<String, String> = BTreeMap::new();
    let mut tags: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (family, names) in raw {
        let key = family_key(&family);
        for name in names {
            let valid = check_tag_name(&name)
                .ok()
                .filter(|checked| *checked == name);
            if valid.is_none() || BuiltInTag::from_name(&name).is_some() {
                return Err(broken(format!("tag {name} is not valid")));
            }
            let lower = name.to_lowercase();
            if spellings.get(&lower).is_some_and(|other| *other != name) {
                return Err(broken(format!("tag {name} has two spellings")));
            }
            spellings.insert(lower, name.clone());
            tags.entry(key.clone()).or_default().insert(name);
        }
    }
    if spellings.len() > MAX_OWN_TAGS {
        return Err(broken(format!("more than {MAX_OWN_TAGS} tags")));
    }
    tags.retain(|_, names| !names.is_empty());
    Ok(tags)
}

/// Reads the text of a font library file.
///
/// # Errors
/// [`FontLibraryFileError::Newer`] for a newer `format`, else
/// [`FontLibraryFileError::Broken`] naming the first rule the text breaks.
pub fn read_font_library(text: &str) -> Result<FontLibrary, FontLibraryFileError> {
    if text.len() > MAX_FONT_LIBRARY_BYTES {
        return Err(broken("the file is larger than 256 KiB"));
    }
    // The version first and alone: a newer file may carry keys this version does not know.
    let header: RawHeader = toml::from_str(text).map_err(|e| broken(e.message()))?;
    match header.format {
        Some(1 | FONT_LIBRARY_FORMAT) => {}
        Some(format) if format > FONT_LIBRARY_FORMAT => return Err(FontLibraryFileError::Newer),
        Some(format) => return Err(broken(format!("format {format} is not valid"))),
        None => return Err(broken("format is missing")),
    }
    let raw: RawFile = toml::from_str(text).map_err(|e| broken(e.message()))?;
    let mut library = FontLibrary {
        favourites: keys(&raw.favourites, "favourites")?,
        tags: read_tags(raw.tags)?,
        ..FontLibrary::default()
    };
    for (id, raw_override) in raw.overrides {
        let tag = BuiltInTag::from_id(&id)
            .ok_or_else(|| broken(format!("overrides.{id} is not a built-in tag")))?;
        let over = Override {
            added: keys(&raw_override.added, &id)?,
            removed: keys(&raw_override.removed, &id)?,
        };
        if over.added.intersection(&over.removed).next().is_some() {
            return Err(broken(format!("overrides.{id} adds and removes one font")));
        }
        if !over.added.is_empty() || !over.removed.is_empty() {
            library.overrides.insert(tag, over);
        }
    }
    // A chosen cell that names nothing falls back to All (criterion 4), as `0045` drops state.
    library.chosen = raw.chosen.filter(|id| id != ALL_ID);
    if let Some(id) = library.chosen.clone()
        && !library.choose(&id)
    {
        library.chosen = None;
    }
    Ok(library)
}

/// A TOML basic string.
fn quoted(text: &str) -> String {
    toml::Value::String(text.to_owned()).to_string()
}

fn list<'a>(items: impl IntoIterator<Item = &'a String>) -> String {
    let items: Vec<String> = items.into_iter().map(|item| quoted(item)).collect();
    format!("[{}]", items.join(", "))
}

/// The canonical text of `library`: `format`, `chosen` (absent for All), `favourites`, the
/// overrides in chip order, then `[tags]` by family key; every list sorted.
#[must_use]
pub fn write_font_library(library: &FontLibrary) -> String {
    let mut out = format!("format = {FONT_LIBRARY_FORMAT}\n");
    if let Some(chosen) = &library.chosen {
        let _ = writeln!(out, "chosen = {}", quoted(chosen));
    }
    let _ = writeln!(out, "favourites = {}", list(&library.favourites));
    for (tag, over) in &library.overrides {
        let _ = write!(
            out,
            "\n[overrides.{}]\nadded = {}\nremoved = {}\n",
            tag.id(),
            list(&over.added),
            list(&over.removed)
        );
    }
    if !library.tags.is_empty() {
        out.push_str("\n[tags]\n");
        for (family, tags) in &library.tags {
            let _ = writeln!(out, "{} = {}", quoted(family), list(tags));
        }
    }
    out
}
