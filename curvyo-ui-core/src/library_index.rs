//! The library index (`specs/0137-document-library` criteria 1 to 3, 7, 10, ADR 0084 §3): one
//! record per document of every source, the change check that decides what to read again, the
//! per-user cache text, the search over document and page names and the lookup of Open source.
//!
//! The host scans the sources and reads the container members; this module only keeps what they
//! say. The cache holds no work: a damaged or newer one is rebuilt.

use serde::{Deserialize, Serialize};

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, CameFrom, DocumentSize, Length, LibrarySummary, UnixMillis,
};

/// The cache file's name in the app data directory.
pub const LIBRARY_INDEX_FILE: &str = "library-index.json";
/// The cache `format` this build writes and the newest it reads.
pub const LIBRARY_INDEX_FORMAT: u32 = 1;

/// What one document file is to the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentState {
    /// A document this build reads.
    Readable,
    /// Not a Curvyo file.
    NotCurvyo,
    /// A container that cannot be read through.
    Damaged,
    /// Made by a newer Curvyo.
    Newer,
}

/// One page of an indexed document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexedPage {
    /// The page id.
    pub id: String,
    /// The page name.
    pub name: String,
    /// The chip word (`drawing`, `sketch`, `laser`).
    pub kind: String,
    /// The page size.
    #[serde(with = "size_mm")]
    pub size: DocumentSize,
    /// Whether the container holds a picture of the page.
    pub picture: bool,
}

/// One document file of a source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexedDocument {
    /// The key of its source: a workspace's id, an added folder's path.
    pub source: String,
    /// Its path inside the source, the file name last.
    pub path: Vec<String>,
    /// The file size when it was read.
    pub size: u64,
    /// The modification time when it was read.
    pub modified: UnixMillis,
    /// What it is.
    pub state: DocumentState,
    /// Its pages in page order; empty unless readable.
    pub pages: Vec<IndexedPage>,
}

impl IndexedDocument {
    /// The document's name: the file name without its extension.
    #[must_use]
    pub fn name(&self) -> &str {
        let file = self.path.last().map_or("", String::as_str);
        file.rsplit_once('.').map_or(file, |(stem, _)| stem)
    }
}

/// The record of a document from what its container says: the manifest's `format_version`, the
/// bytes of `library.json` and the ids of the pages that have a picture. `summary` is `None` for
/// a file without the member; the host then reads it natively ([`fallback_summary`]).
#[must_use]
pub fn readable_pages(
    format_version: u32,
    summary: Option<&LibrarySummary>,
    pictures: &[String],
) -> (DocumentState, Vec<IndexedPage>) {
    if format_version > CURRENT_FORMAT_VERSION {
        return (DocumentState::Newer, Vec::new());
    }
    let pages = summary.map_or_else(Vec::new, |summary| {
        summary
            .pages
            .iter()
            .map(|page| IndexedPage {
                id: page.id.clone(),
                name: page.name.clone(),
                kind: page.kind.clone(),
                size: page.size,
                picture: pictures.contains(&page.id),
            })
            .collect()
    });
    (DocumentState::Readable, pages)
}

/// The page list of a container that has no `library.json` (ADR 0084 §1 fallback): the document
/// is unpacked in memory, never written back; `None` when it does not open.
#[must_use]
pub fn fallback_summary(bytes: &[u8]) -> Option<LibrarySummary> {
    curvyo_document_core::unpack(1, bytes)
        .ok()
        .map(|document| curvyo_document_core::library_summary(&document))
}

/// The record of the container `bytes` at `path` of `source` (ADR 0084 §1, decision 10): its
/// members, else the native fallback for a file without the page list. A readable container
/// whose pages cannot be read at all is damaged.
#[must_use]
pub fn record_of_container(
    source: &str,
    path: Vec<String>,
    modified: UnixMillis,
    bytes: &[u8],
) -> IndexedDocument {
    let members = curvyo_document_core::read_library_members(std::io::Cursor::new(bytes));
    record_of_members(
        source,
        path,
        (bytes.len() as u64, modified),
        members,
        || Some(bytes.to_vec()),
    )
}

/// The record of a document from its `members`; `whole` loads the container bytes for the
/// fallback, only when the page list is missing. `(size, modified)` are the file's.
#[must_use]
pub fn record_of_members(
    source: &str,
    path: Vec<String>,
    (size, modified): (u64, UnixMillis),
    members: curvyo_document_core::LibraryMembers,
    whole: impl FnOnce() -> Option<Vec<u8>>,
) -> IndexedDocument {
    use curvyo_document_core::LibraryMembers;
    let (state, pages) = match members {
        LibraryMembers::Readable {
            format_version,
            summary,
            pictures,
        } => {
            let summary = summary
                .as_deref()
                .and_then(LibrarySummary::from_bytes)
                .or_else(|| {
                    (format_version <= CURRENT_FORMAT_VERSION)
                        .then(whole)
                        .flatten()
                        .and_then(|bytes| fallback_summary(&bytes))
                });
            if summary.is_none() && format_version <= CURRENT_FORMAT_VERSION {
                (DocumentState::Damaged, Vec::new())
            } else {
                readable_pages(format_version, summary.as_ref(), &pictures)
            }
        }
        LibraryMembers::NotCurvyo => (DocumentState::NotCurvyo, Vec::new()),
        LibraryMembers::Damaged => (DocumentState::Damaged, Vec::new()),
    };
    IndexedDocument {
        source: source.to_owned(),
        path,
        size,
        modified,
        state,
        pages,
    }
}

/// Every indexed document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LibraryIndex {
    documents: Vec<IndexedDocument>,
    /// Where each `(source, path)` is in `documents`.
    at: std::collections::HashMap<(String, Vec<String>), usize>,
}

#[derive(Serialize, Deserialize)]
struct IndexFile {
    format: u32,
    documents: Vec<IndexedDocument>,
}

#[derive(Deserialize)]
struct IndexVersion {
    format: u32,
}

/// A match of the search: a document and the pages of it that match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    /// The index of the document in [`LibraryIndex::documents`].
    pub document: usize,
    /// Whether the document's own name matches.
    pub by_name: bool,
    /// The indices of its matching pages.
    pub pages: Vec<usize>,
}

impl LibraryIndex {
    /// The cache in `text`; an empty index when it is damaged or newer (it holds no work).
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let newer = serde_json::from_str::<IndexVersion>(text)
            .map_or(true, |v| v.format > LIBRARY_INDEX_FORMAT);
        if newer {
            return Self::default();
        }
        serde_json::from_str::<IndexFile>(text)
            .map(|file| Self::of(file.documents))
            .unwrap_or_default()
    }

    fn of(documents: Vec<IndexedDocument>) -> Self {
        let mut index = Self {
            documents,
            at: std::collections::HashMap::new(),
        };
        index.reindex();
        index
    }

    fn reindex(&mut self) {
        self.at = self
            .documents
            .iter()
            .enumerate()
            .map(|(i, d)| ((d.source.clone(), d.path.clone()), i))
            .collect();
    }

    /// The cache text.
    #[must_use]
    pub fn to_text(&self) -> String {
        serde_json::to_string(&IndexFile {
            format: LIBRARY_INDEX_FORMAT,
            documents: self.documents.clone(),
        })
        // invariant: plain records of strings and numbers always serialise.
        .unwrap_or_default()
    }

    /// Every document, in source order and then by path.
    #[must_use]
    pub fn documents(&self) -> &[IndexedDocument] {
        &self.documents
    }

    /// Whether the document at `path` of `source` must be read again: it is not indexed, or its
    /// size or modification time differs.
    #[must_use]
    pub fn needs_read(
        &self,
        source: &str,
        path: &[String],
        size: u64,
        modified: UnixMillis,
    ) -> bool {
        self.find(source, path)
            .is_none_or(|d| d.size != size || d.modified != modified)
    }

    fn find(&self, source: &str, path: &[String]) -> Option<&IndexedDocument> {
        let index = *self.at.get(&(source.to_owned(), path.to_vec()))?;
        self.documents.get(index)
    }

    /// Puts `document` into the index, replacing the record of the same file.
    pub fn put(&mut self, document: IndexedDocument) {
        let key = (document.source.clone(), document.path.clone());
        if let Some(&index) = self.at.get(&key) {
            self.documents[index] = document;
        } else {
            self.at.insert(key, self.documents.len());
            self.documents.push(document);
        }
    }

    /// Drops the records of `source` whose path is not among `present` (the file went).
    pub fn keep_present(&mut self, source: &str, present: &[Vec<String>]) {
        let present: std::collections::HashSet<&Vec<String>> = present.iter().collect();
        self.documents
            .retain(|d| d.source != source || present.contains(&d.path));
        self.reindex();
    }

    /// Keeps only the records of `sources` and orders them by that source order, then by path.
    pub fn order_by_sources(&mut self, sources: &[String]) {
        self.documents.retain(|d| sources.contains(&d.source));
        self.documents.sort_by(|a, b| {
            let rank = |key: &str| sources.iter().position(|s| s == key);
            rank(&a.source)
                .cmp(&rank(&b.source))
                .then_with(|| a.path.cmp(&b.path))
        });
        self.reindex();
    }

    /// The documents and pages whose names hold `query` (Unicode lower case, substring), in
    /// index order; an empty query matches nothing.
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<SearchMatch> {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        self.documents
            .iter()
            .enumerate()
            .filter_map(|(index, document)| {
                let by_name = document.name().to_lowercase().contains(&needle);
                let pages: Vec<usize> = document
                    .pages
                    .iter()
                    .enumerate()
                    .filter(|(_, page)| page.name.to_lowercase().contains(&needle))
                    .map(|(i, _)| i)
                    .collect();
                (by_name || !pages.is_empty()).then_some(SearchMatch {
                    document: index,
                    by_name,
                    pages,
                })
            })
            .collect()
    }

    /// The document Open source opens for `record` (ADR 0084 §7): among the readable documents
    /// that hold its page id, the first whose name is the record's document name, else the
    /// first in source order.
    #[must_use]
    pub fn source_of(&self, record: &CameFrom) -> Option<&IndexedDocument> {
        let holding: Vec<&IndexedDocument> = self
            .documents
            .iter()
            .filter(|d| d.state == DocumentState::Readable)
            .filter(|d| d.pages.iter().any(|p| p.id == record.page_id))
            .collect();
        holding
            .iter()
            .find(|d| d.name() == record.document)
            .or_else(|| holding.first())
            .copied()
    }
}

mod size_mm {
    use super::{DocumentSize, Length};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct Mm {
        width_mm: f64,
        height_mm: f64,
    }

    pub(super) fn serialize<S: Serializer>(size: &DocumentSize, s: S) -> Result<S::Ok, S::Error> {
        Mm {
            width_mm: size.width.as_mm(),
            height_mm: size.height.as_mm(),
        }
        .serialize(s)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<DocumentSize, D::Error> {
        let mm = Mm::deserialize(d)?;
        Ok(DocumentSize::new(
            Length::from_mm(mm.width_mm),
            Length::from_mm(mm.height_mm),
        ))
    }
}

#[cfg(test)]
#[path = "library_index_tests.rs"]
mod tests;
