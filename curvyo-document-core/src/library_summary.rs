//! The page summaries a save writes into the container as `library.json` (ADR 0084 §1,
//! `specs/0137` criterion 2), so the library lists a document's pages without opening it.
//!
//! The member is not part of the document: a build that does not know it never reads it, and it
//! is written for a save only, never for a relay snapshot or a recovery copy.

use serde::{Deserialize, Serialize};

use crate::document::Document;
use crate::page_kind::PageKind;
use crate::units::DocumentSize;

/// The container member name.
pub const LIBRARY_MEMBER: &str = "library.json";
/// The `format` this build writes and the newest it reads.
pub const LIBRARY_FORMAT: u32 = 1;
/// The largest `library.json` a reader takes; a larger one is read as absent.
pub const MAX_LIBRARY_BYTES: u64 = 256 * 1024;

/// One page as the library lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct PageSummary {
    /// The page id.
    pub id: String,
    /// The page name.
    pub name: String,
    /// The word of the page chip: `drawing`, `sketch`, or an output page's process word
    /// (`laser`); an unknown kind keeps its stored text.
    pub kind: String,
    /// The page size.
    pub size: DocumentSize,
}

/// The pages of one document in page order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LibrarySummary {
    /// Every live page in list order.
    pub pages: Vec<PageSummary>,
}

#[derive(Serialize, Deserialize)]
struct Wire {
    format: u32,
    pages: Vec<WirePage>,
}

#[derive(Serialize, Deserialize)]
struct WirePage {
    id: String,
    name: String,
    kind: String,
    width_mm: f64,
    height_mm: f64,
}

/// The chip word of `kind` (the `0098` page chip).
#[must_use]
pub fn kind_word(kind: &PageKind) -> String {
    match kind {
        PageKind::Drawing => "drawing".to_owned(),
        PageKind::Sketch => "sketch".to_owned(),
        PageKind::Output { process, .. } => process
            .rsplit('.')
            .next()
            .unwrap_or(process.as_str())
            .to_owned(),
        PageKind::Unknown(stored) => stored.clone(),
    }
}

/// The summary of every live page of `document`, in page order.
#[must_use]
pub fn library_summary(document: &Document) -> LibrarySummary {
    LibrarySummary {
        pages: document
            .pages()
            .into_iter()
            .map(|page| PageSummary {
                kind: kind_word(&page.kind),
                id: page.id,
                name: page.name,
                size: page.size,
            })
            .collect(),
    }
}

impl LibrarySummary {
    /// The member bytes.
    ///
    /// # Errors
    /// [`serde_json::Error`] if a value cannot be written (a non-finite size never is).
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&Wire {
            format: LIBRARY_FORMAT,
            pages: self
                .pages
                .iter()
                .map(|page| WirePage {
                    id: page.id.clone(),
                    name: page.name.clone(),
                    kind: page.kind.clone(),
                    width_mm: page.size.width.as_mm(),
                    height_mm: page.size.height.as_mm(),
                })
                .collect(),
        })
    }

    /// The summary in `bytes`; `None` for bytes that are too large, not a summary, of a newer
    /// `format`, or with a size that is not a valid page size.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() as u64 > MAX_LIBRARY_BYTES {
            return None;
        }
        let wire: Wire = serde_json::from_slice(bytes).ok()?;
        if wire.format > LIBRARY_FORMAT {
            return None;
        }
        let pages = wire
            .pages
            .into_iter()
            .map(|page| {
                let checked = |mm: f64| {
                    crate::document_size::validated_document_side(crate::Length::from_mm(mm)).ok()
                };
                let size = DocumentSize::new(checked(page.width_mm)?, checked(page.height_mm)?);
                Some(PageSummary {
                    id: page.id,
                    name: page.name,
                    kind: page.kind,
                    size,
                })
            })
            .collect::<Option<_>>()?;
        Some(Self { pages })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_summary_lists_every_page_in_order_with_its_kind_word() {
        let d = Document::new(1);
        d.add_page();
        d.add_page_of(&PageKind::Sketch).unwrap();
        let summary = library_summary(&d);
        let names: Vec<_> = summary.pages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Page 1", "Page 2", "Sketch 1"]);
        let kinds: Vec<_> = summary.pages.iter().map(|p| p.kind.as_str()).collect();
        assert_eq!(kinds, ["drawing", "drawing", "sketch"]);
        assert_eq!(summary.pages[0].size, d.pages()[0].size);
    }

    #[test]
    fn the_bytes_read_back_and_a_newer_format_reads_as_absent() {
        let d = Document::new(1);
        let summary = library_summary(&d);
        let bytes = summary.to_bytes().unwrap();
        assert_eq!(LibrarySummary::from_bytes(&bytes), Some(summary));
        let newer = String::from_utf8(bytes)
            .unwrap()
            .replacen("\"format\":1", "\"format\":2", 1);
        assert_eq!(LibrarySummary::from_bytes(newer.as_bytes()), None);
        assert_eq!(LibrarySummary::from_bytes(b"not json"), None);
        let large = vec![b' '; usize::try_from(MAX_LIBRARY_BYTES).unwrap() + 1];
        assert_eq!(LibrarySummary::from_bytes(&large), None);
    }

    #[test]
    fn the_output_process_word_is_its_last_part() {
        let kind = PageKind::Output {
            manufacturing: crate::NodeId::from_parts(1, 1),
            process: "curvyo.laser".into(),
        };
        assert_eq!(kind_word(&kind), "laser");
    }
}
