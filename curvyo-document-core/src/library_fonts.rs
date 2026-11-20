//! The font entries of the synced library list (ADR 0062 decisions 2, 3, 4 and 6,
//! `specs/0116-library-sync` criteria 16 to 19): one register per field, keyed by the file's hash.
//!
//! The root map `fonts` holds the primitive registers `<sha-256 hex>.<field>`: `sync`,
//! `file_name`, `folder`, `size`, `tags`, `favourite`, `cuttable`, `documents`. Each register is
//! last-writer-wins on its own, so two devices that edit different fields both keep their
//! edit. A font switched off keeps only `sync = false`, so a state-only snapshot holds no name,
//! tag or size of it (criteria 4 and 27). The font bytes are not here: they are blobs keyed by
//! the same hash (`curvyo-sync-core::library_blob`). A list that holds a font entry is written as
//! format version 2.

use std::collections::BTreeMap;

use loro::{LoroValue, ValueOrContainer};

use crate::assets::AssetHash;
use crate::library_list::{
    KEY_FORMAT_VERSION, LIBRARY_LIST_FORMAT_VERSION, LibraryList, LibraryListError, ROOT,
};

/// The largest font file a synced entry names: 16 MiB (ADR 0062 decision 6, Proposal).
pub const MAX_SYNCED_FONT_BYTES: u64 = 16 * 1024 * 1024;
/// The most bytes of all on-switched font entries together: 512 MiB (ADR 0062 decision 6).
pub const MAX_SYNCED_FONTS_BYTES: u64 = 512 * 1024 * 1024;
/// The most licence and readme files one font entry carries (`0083`).
pub const MAX_FONT_DOCUMENTS: usize = 16;
/// The largest licence or readme file an entry carries: 1 MiB.
pub const MAX_FONT_DOCUMENT_BYTES: u64 = 1024 * 1024;

const FONTS: &str = "fonts";
const SYNC: &str = "sync";
const FILE_NAME: &str = "file_name";
const FOLDER: &str = "folder";
const SIZE: &str = "size";
const TAGS: &str = "tags";
const FAVOURITE: &str = "favourite";
const CUTTABLE: &str = "cuttable";
const DOCUMENTS: &str = "documents";
/// Separates the items of a list register; no tag, name or hash holds it.
const LIST_SEP: char = '\n';

/// The maker's choice for the Cuttable tag of a family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CuttableChoice {
    /// The automatic check decides.
    #[default]
    Auto,
    /// The maker added the tag by hand.
    Yes,
    /// The maker took the tag out by hand.
    No,
}

impl CuttableChoice {
    const fn word(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Yes => "yes",
            Self::No => "no",
        }
    }

    fn from_word(word: &str) -> Option<Self> {
        match word {
            "auto" => Some(Self::Auto),
            "yes" => Some(Self::Yes),
            "no" => Some(Self::No),
            _ => None,
        }
    }
}

/// A licence or readme file that travels with a font (`0083`, criterion 18).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontDocument {
    /// The SHA-256 of its bytes: its blob's key.
    pub hash: AssetHash,
    /// Its size in bytes.
    pub size: u64,
    /// Its file name inside the font's folder.
    pub name: String,
}

/// What a switched-on font entry says about its file and its metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontDetails {
    /// The file name (no folder), as it is named in the fonts folder.
    pub file_name: String,
    /// The `.zip` subfolder of the fonts folder the file sits in (`0083`).
    pub folder: Option<String>,
    /// The file's size in bytes.
    pub size: u64,
    /// The own and overridden built-in tags of the file's first family.
    pub tags: Vec<String>,
    /// Whether the family is starred.
    pub favourite: bool,
    /// The Cuttable override.
    pub cuttable: CuttableChoice,
    /// The kept licence and readme files.
    pub documents: Vec<FontDocument>,
}

/// One font entry: on with its details, or the marker of a switched-off font.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontEntry {
    /// The SHA-256 of the file's bytes.
    pub hash: AssetHash,
    /// Whether the font's sync is on.
    pub sync: bool,
    /// The details; `None` in a marker.
    pub details: Option<FontDetails>,
}

/// Why a font cannot be switched on (criterion 19): the numbers for its notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FontBudgetError {
    /// The file or one of its documents is over its limit.
    #[error("the font file is over the per-file limit")]
    TooLarge {
        /// The offending size in bytes.
        size: u64,
        /// The limit in bytes.
        limit: u64,
    },
    /// The synced fonts together would pass the library limit.
    #[error("the synced fonts would pass the library limit")]
    OverBudget {
        /// The total in bytes the library would reach.
        total: u64,
        /// The limit in bytes.
        limit: u64,
    },
}

fn key(hash: &AssetHash, field: &str) -> String {
    format!("{}.{field}", hash.as_str())
}

/// Splits `<hash>.<field>`; `None` for a key of another shape.
fn split_key(key: &str) -> Option<(AssetHash, &str)> {
    let (hash, field) = key.split_once('.')?;
    Some((AssetHash::parse(hash)?, field))
}

fn is_plain(text: &str) -> bool {
    !text.is_empty() && !text.contains(['/', '\\', '\n', '\0']) && text != ".." && text != "."
}

impl FontDetails {
    fn validated(&self) -> Result<(), LibraryListError> {
        let tag_ok = |tag: &String| !tag.is_empty() && !tag.contains(LIST_SEP);
        let folder_ok = self.folder.as_deref().is_none_or(is_plain);
        let documents_ok = self.documents.len() <= MAX_FONT_DOCUMENTS
            && self.documents.iter().all(|d| is_plain(&d.name));
        if is_plain(&self.file_name)
            && self.file_name.len() <= 255
            && folder_ok
            && documents_ok
            && self.tags.iter().all(tag_ok)
        {
            Ok(())
        } else {
            Err(LibraryListError::BadId)
        }
    }
}

impl LibraryList {
    /// Switches the font `hash` on with `details`, writing only the registers that differ (the
    /// same call twice writes nothing). The first font entry makes the list format 2.
    ///
    /// # Errors
    /// [`LibraryListError::BadId`] for a name that is not a plain file name,
    /// [`LibraryListError::Encode`].
    pub fn font_set(
        &self,
        hash: &AssetHash,
        details: &FontDetails,
    ) -> Result<(), LibraryListError> {
        details.validated()?;
        let map = self.doc.get_map(FONTS);
        let documents = details
            .documents
            .iter()
            .map(|d| format!("{} {} {}", d.hash.as_str(), d.size, d.name))
            .collect::<Vec<_>>()
            .join("\n");
        // Fixed order: the update bytes of a golden must not depend on a hash map.
        let registers: [(&str, LoroValue); 8] = [
            (SYNC, LoroValue::from(true)),
            (FILE_NAME, LoroValue::from(details.file_name.as_str())),
            (
                FOLDER,
                LoroValue::from(details.folder.as_deref().unwrap_or("")),
            ),
            (
                SIZE,
                LoroValue::from(i64::try_from(details.size).unwrap_or(i64::MAX)),
            ),
            (TAGS, LoroValue::from(details.tags.join("\n"))),
            (FAVOURITE, LoroValue::from(details.favourite)),
            (CUTTABLE, LoroValue::from(details.cuttable.word())),
            (DOCUMENTS, LoroValue::from(documents)),
        ];
        let mut wrote = false;
        for (field, value) in registers {
            let at = key(hash, field);
            let same = matches!(map.get(&at), Some(ValueOrContainer::Value(held)) if held == value);
            if !same {
                map.insert(&at, value)
                    .map_err(|_| LibraryListError::Encode)?;
                wrote = true;
            }
        }
        if wrote {
            self.raise_format_for_fonts()?;
            self.doc.commit();
        }
        Ok(())
    }

    /// Switches the font `hash` off: only `sync = false` stays, so no name, tag or size of it
    /// remains in a state-only snapshot (criteria 4 and 27); a font without an entry writes
    /// nothing.
    ///
    /// # Errors
    /// [`LibraryListError::Encode`].
    pub fn font_switch_off(&self, hash: &AssetHash) -> Result<(), LibraryListError> {
        if self.font_entry(hash).is_none() {
            return Ok(());
        }
        self.font_clear(hash)?;
        self.doc
            .get_map(FONTS)
            .insert(&key(hash, SYNC), false)
            .map_err(|_| LibraryListError::Encode)?;
        self.doc.commit();
        Ok(())
    }

    /// Deletes the font `hash`'s entry (the font was deleted); an absent one does nothing.
    ///
    /// # Errors
    /// [`LibraryListError::Encode`].
    pub fn font_remove(&self, hash: &AssetHash) -> Result<(), LibraryListError> {
        if self.font_entry(hash).is_none() {
            return Ok(());
        }
        self.font_clear(hash)?;
        self.doc.commit();
        Ok(())
    }

    fn font_clear(&self, hash: &AssetHash) -> Result<(), LibraryListError> {
        let map = self.doc.get_map(FONTS);
        for field in [
            SYNC, FILE_NAME, FOLDER, SIZE, TAGS, FAVOURITE, CUTTABLE, DOCUMENTS,
        ] {
            let at = key(hash, field);
            if map.get(&at).is_some() {
                map.delete(&at).map_err(|_| LibraryListError::Encode)?;
            }
        }
        Ok(())
    }

    /// Every font entry, on or off, sorted by hash; entries whose registers have a bad shape
    /// are skipped.
    #[must_use]
    pub fn font_entries(&self) -> Vec<FontEntry> {
        self.read_fonts(None)
    }

    /// The entry of the font `hash`, if the list knows it.
    #[must_use]
    pub fn font_entry(&self, hash: &AssetHash) -> Option<FontEntry> {
        self.read_fonts(Some(hash)).into_iter().next()
    }

    /// The valid entries, of the font `only` or of all.
    fn read_fonts(&self, only: Option<&AssetHash>) -> Vec<FontEntry> {
        let mut fields: BTreeMap<AssetHash, BTreeMap<String, LoroValue>> = BTreeMap::new();
        self.doc.get_map(FONTS).for_each(|at, value| {
            if let (Some((hash, field)), ValueOrContainer::Value(value)) = (split_key(at), value)
                && only.is_none_or(|only| *only == hash)
            {
                fields
                    .entry(hash)
                    .or_default()
                    .insert(field.to_owned(), value);
            }
        });
        fields
            .into_iter()
            .filter_map(|(hash, fields)| read_entry(hash, &fields))
            .collect()
    }

    /// The bytes the switched-on fonts hold on the budget: each file and each licence file
    /// once, however many entries name it.
    #[must_use]
    pub fn font_total_bytes(&self) -> u64 {
        total_bytes(&self.font_entries())
    }

    /// Whether `details` of the font `hash` may be switched on (criterion 19): the file and
    /// each document within the per-file limit, the total within the library limit. A font
    /// that is on already is never refused for its own bytes.
    ///
    /// # Errors
    /// [`FontBudgetError`] with the numbers of the notice.
    pub fn font_check_budget(
        &self,
        hash: &AssetHash,
        details: &FontDetails,
    ) -> Result<(), FontBudgetError> {
        if details.size > MAX_SYNCED_FONT_BYTES {
            return Err(FontBudgetError::TooLarge {
                size: details.size,
                limit: MAX_SYNCED_FONT_BYTES,
            });
        }
        if let Some(big) = details
            .documents
            .iter()
            .find(|d| d.size > MAX_FONT_DOCUMENT_BYTES)
        {
            return Err(FontBudgetError::TooLarge {
                size: big.size,
                limit: MAX_FONT_DOCUMENT_BYTES,
            });
        }
        let mut entries = self.font_entries();
        entries.retain(|entry| &entry.hash != hash);
        let held = total_bytes(&entries);
        let total = held + extra_bytes(&entries, hash, details);
        if total > MAX_SYNCED_FONTS_BYTES {
            return Err(FontBudgetError::OverBudget {
                total,
                limit: MAX_SYNCED_FONTS_BYTES,
            });
        }
        Ok(())
    }

    fn raise_format_for_fonts(&self) -> Result<(), LibraryListError> {
        let held = matches!(
            self.doc.get_map(ROOT).get(KEY_FORMAT_VERSION),
            Some(ValueOrContainer::Value(LoroValue::I64(version))) if version >= LIBRARY_LIST_FORMAT_VERSION
        );
        if held {
            return Ok(());
        }
        self.doc
            .get_map(ROOT)
            .insert(KEY_FORMAT_VERSION, LIBRARY_LIST_FORMAT_VERSION)
            .map_err(|_| LibraryListError::Encode)
    }
}

/// The distinct blob bytes of `entries`' on entries.
fn total_bytes(entries: &[FontEntry]) -> u64 {
    let mut seen: BTreeMap<&AssetHash, u64> = BTreeMap::new();
    for entry in entries {
        if let Some(details) = entry.details.as_ref().filter(|_| entry.sync) {
            seen.insert(&entry.hash, details.size);
            for document in &details.documents {
                seen.insert(&document.hash, document.size);
            }
        }
    }
    seen.values().sum()
}

/// The bytes `details` adds to `entries`: the hashes none of them names yet.
fn extra_bytes(entries: &[FontEntry], hash: &AssetHash, details: &FontDetails) -> u64 {
    let named = |wanted: &AssetHash| {
        entries.iter().any(|entry| {
            entry.sync
                && entry.details.as_ref().is_some_and(|d| {
                    &entry.hash == wanted || d.documents.iter().any(|doc| &doc.hash == wanted)
                })
        })
    };
    let file = if named(hash) { 0 } else { details.size };
    let docs: u64 = details
        .documents
        .iter()
        .filter(|d| !named(&d.hash))
        .map(|d| d.size)
        .sum();
    file + docs
}

/// Reads one entry; `None` for registers of a bad shape or a size past the limits of
/// `font_check_budget`, so a hostile peer's entry is never seen (ADR 0062 decision 6).
fn read_entry(hash: AssetHash, fields: &BTreeMap<String, LoroValue>) -> Option<FontEntry> {
    let sync = match fields.get(SYNC) {
        Some(LoroValue::Bool(on)) => *on,
        _ => return None,
    };
    if !sync {
        return Some(FontEntry {
            hash,
            sync,
            details: None,
        });
    }
    let text = |field: &str| match fields.get(field) {
        Some(LoroValue::String(text)) => Some(text.to_string()),
        _ => None,
    };
    let size = match fields.get(SIZE) {
        Some(LoroValue::I64(size)) => u64::try_from(*size)
            .ok()
            .filter(|size| *size <= MAX_SYNCED_FONT_BYTES)?,
        _ => return None,
    };
    let favourite = match fields.get(FAVOURITE) {
        Some(LoroValue::Bool(on)) => *on,
        _ => return None,
    };
    let file_name = text(FILE_NAME).filter(|name| is_plain(name))?;
    let folder = text(FOLDER).filter(|folder| !folder.is_empty());
    if folder.as_deref().is_some_and(|folder| !is_plain(folder)) {
        return None;
    }
    let tags = text(TAGS)?
        .split(LIST_SEP)
        .filter(|tag| !tag.is_empty())
        .map(str::to_owned)
        .collect();
    let cuttable = CuttableChoice::from_word(&text(CUTTABLE)?)?;
    let documents = text(DOCUMENTS)?
        .split(LIST_SEP)
        .filter(|line| !line.is_empty())
        .map(read_document)
        .collect::<Option<Vec<_>>>()
        .filter(|documents| documents.len() <= MAX_FONT_DOCUMENTS)?;
    Some(FontEntry {
        hash,
        sync,
        details: Some(FontDetails {
            file_name,
            folder,
            size,
            tags,
            favourite,
            cuttable,
            documents,
        }),
    })
}

fn read_document(line: &str) -> Option<FontDocument> {
    let mut parts = line.splitn(3, ' ');
    let hash = AssetHash::parse(parts.next()?)?;
    let size = parts
        .next()?
        .parse()
        .ok()
        .filter(|size| *size <= MAX_FONT_DOCUMENT_BYTES)?;
    let name = parts.next().filter(|name| is_plain(name))?.to_owned();
    Some(FontDocument { hash, size, name })
}
