//! The rows of the Library tab's tree (`specs/0137-document-library` criteria 1, 2, 3, 10, 11
//! and 13, UX notes sections 4 and 5): group headings, sources, folders, documents and page
//! rows in their order, the expand state or the search's choice of what stays.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};

use serde::Serialize;

use crate::library_index::{DocumentState, IndexedDocument, LibraryIndex, SearchMatch};
use crate::library_index_text::{
    SourceReason, document_error_text, document_line, page_size_text, source_error_text,
};
use crate::workspace_rows::natural_cmp;

/// What a source is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// The workspace open in this window.
    OpenWorkspace,
    /// Another workspace of the recent list.
    Workspace,
    /// A folder the maker added.
    Folder,
    /// A workspace the browser stores (UX notes 2.1); `true` for the open one.
    Stored(bool),
}

impl SourceKind {
    /// The group the source is listed under.
    const fn heading(self) -> &'static str {
        match self {
            Self::OpenWorkspace | Self::Workspace => "Workspaces",
            Self::Stored(_) => "In this browser",
            Self::Folder => "Folders",
        }
    }

    /// The word of the accessible name: "open workspace", "workspace in this browser".
    const fn word(self) -> &'static str {
        match self {
            Self::OpenWorkspace => "open workspace",
            Self::Workspace => "workspace",
            Self::Stored(_) => "workspace in this browser",
            Self::Folder => "folder",
        }
    }
}

/// One source of the library, in source order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibrarySource {
    /// Its key in the index.
    pub key: String,
    /// The name shown.
    pub name: String,
    /// What it is.
    pub kind: SourceKind,
    /// Why it cannot be read, when it cannot.
    pub error: Option<SourceReason>,
}

/// The kind of a tree row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RowKind {
    /// "Workspaces" or "Folders"; not in the keyboard order.
    Heading,
    /// A workspace or an added folder.
    Source,
    /// A folder inside a source.
    Folder,
    /// A document.
    Document,
    /// A page of a document.
    Page,
}

/// One row of the tree.
// The page reads each flag of a row on its own (chevron, armed Remove, picture, open workspace);
// an enum of combinations would only be unpacked again there.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LibraryRow {
    /// The row's stable key (its expand state is kept by it).
    pub key: String,
    /// What it is.
    pub kind: RowKind,
    /// The tree level, 0 for headings and sources.
    pub level: usize,
    /// The name.
    pub name: String,
    /// The second line (documents, pages, errors) or the count (sources, folders).
    pub line: String,
    /// Whether it can expand.
    pub expandable: bool,
    /// Whether it is expanded.
    pub expanded: bool,
    /// Whether it cannot be read.
    pub unreadable: bool,
    /// The source key.
    pub source: String,
    /// The path inside the source (documents, pages, folders).
    pub path: Vec<String>,
    /// The page id of a page row.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_id: Option<String>,
    /// The chip word of a page row.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chip: Option<String>,
    /// Whether a page row has a picture.
    pub picture: bool,
    /// The open workspace's source row.
    pub open: bool,
    /// A removable added folder.
    pub removable: bool,
    /// The matched part of the name, as char offsets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched: Option<(usize, usize)>,
    /// The accessible name ("Lid, document, 3 pages, changed 2 Nov 12:03").
    pub label: String,
}

/// How the rows are built.
#[derive(Debug, Clone, Copy)]
pub struct TreeView<'a> {
    /// The rows the maker expanded, by key (ignored while searching).
    pub expanded: &'a HashSet<String>,
    /// The search text; empty for none.
    pub query: &'a str,
    /// Now, Unix milliseconds, and the local offset, for the change times.
    pub now: (i64, i32),
    /// The document open in this window with unsaved changes: `(source, path)`.
    pub saved_version: Option<(&'a str, &'a [String])>,
}

/// A row key.
#[must_use]
pub fn row_key(kind: RowKind, source: &str, path: &[String], page: Option<&str>) -> String {
    let tag = match kind {
        RowKind::Heading => "h",
        RowKind::Source => "s",
        RowKind::Folder => "f",
        RowKind::Document => "d",
        RowKind::Page => "p",
    };
    let mut key = format!("{tag}:{source}\u{1f}{}", path.join("\u{1f}"));
    if let Some(page) = page {
        key.push('#');
        key.push_str(page);
    }
    key
}

/// A folder level of one source while the tree is built.
#[derive(Default)]
struct Level<'a> {
    folders: BTreeMap<String, Self>,
    documents: Vec<(&'a IndexedDocument, Option<&'a SearchMatch>)>,
}

impl Level<'_> {
    fn count(&self) -> usize {
        self.documents.len() + self.folders.values().map(Level::count).sum::<usize>()
    }
}

/// The rows of the tree for `sources` and `index` (UX notes section 4): the workspaces, then the
/// added folders, each a tree of its folders (first) and documents in natural order, page rows
/// under expanded documents. With a query, only what holds a match stays (P13).
#[must_use]
pub fn library_rows(
    sources: &[LibrarySource],
    index: &LibraryIndex,
    view: &TreeView<'_>,
) -> Vec<LibraryRow> {
    let searching = !view.query.trim().is_empty();
    let matches = index.search(view.query);
    let by_document: std::collections::HashMap<usize, &SearchMatch> =
        matches.iter().map(|m| (m.document, m)).collect();
    let mut rows = Vec::new();
    for heading in ["Workspaces", "In this browser", "Folders"] {
        let group: Vec<&LibrarySource> = sources
            .iter()
            .filter(|s| s.kind.heading() == heading)
            .collect();
        let mut group_rows = Vec::new();
        for source in group {
            let level = source_level(index, &source.key, &by_document, searching);
            if searching && level.count() == 0 {
                continue;
            }
            let key = row_key(RowKind::Source, &source.key, &[], None);
            let expanded = source.error.is_none() && (searching || view.expanded.contains(&key));
            group_rows.push(source_row(source, key, level.count(), expanded));
            if expanded {
                push_level(&mut group_rows, &level, &source.key, &mut Vec::new(), view);
            }
        }
        if !group_rows.is_empty() {
            rows.push(heading_row(heading));
            rows.extend(group_rows);
        }
    }
    rows
}

/// The documents of the source `key` as a folder tree; while searching only the matches.
fn source_level<'a>(
    index: &'a LibraryIndex,
    key: &str,
    by_document: &std::collections::HashMap<usize, &'a SearchMatch>,
    searching: bool,
) -> Level<'a> {
    let mut level = Level::default();
    for (i, document) in index.documents().iter().enumerate() {
        if document.source != key {
            continue;
        }
        let found = by_document.get(&i).copied();
        if searching && found.is_none() {
            continue;
        }
        let mut at = &mut level;
        for folder in &document.path[..document.path.len().saturating_sub(1)] {
            at = at.folders.entry(folder.clone()).or_default();
        }
        at.documents.push((document, found));
    }
    level
}

/// The row of a source holding `count` documents.
fn source_row(source: &LibrarySource, key: String, count: usize, expanded: bool) -> LibraryRow {
    LibraryRow {
        line: source.error.map_or_else(
            || count.to_string(),
            |reason| source_error_text(reason).to_owned(),
        ),
        expandable: source.error.is_none(),
        expanded,
        unreadable: source.error.is_some(),
        open: matches!(
            source.kind,
            SourceKind::OpenWorkspace | SourceKind::Stored(true)
        ),
        removable: source.kind == SourceKind::Folder,
        label: crate::library_index_text::source_label(
            &source.name,
            source.kind.word(),
            count,
            source.error,
        ),
        ..plain_row(key, RowKind::Source, 0, &source.name, &source.key)
    }
}

fn heading_row(name: &str) -> LibraryRow {
    LibraryRow {
        key: format!("h:{name}"),
        ..plain_row(String::new(), RowKind::Heading, 0, name, "")
    }
}

fn push_level(
    rows: &mut Vec<LibraryRow>,
    level: &Level<'_>,
    source: &str,
    at: &mut Vec<String>,
    view: &TreeView<'_>,
) {
    let searching = !view.query.trim().is_empty();
    let mut folders: Vec<(&String, &Level<'_>)> = level.folders.iter().collect();
    folders.sort_by(|a, b| natural_cmp(a.0, b.0));
    for (name, inner) in folders {
        at.push(name.clone());
        let key = row_key(RowKind::Folder, source, at, None);
        let expanded = searching || view.expanded.contains(&key);
        rows.push(LibraryRow {
            line: inner.count().to_string(),
            expandable: true,
            expanded,
            path: at.clone(),
            label: format!(
                "{name}, folder, {}",
                crate::workspace_text::document_count_text(inner.count())
            ),
            ..plain_row(key, RowKind::Folder, at.len(), name, source)
        });
        if expanded {
            push_level(rows, inner, source, at, view);
        }
        at.pop();
    }
    let mut documents = level.documents.clone();
    documents.sort_by(|a, b| natural_cmp(a.0.name(), b.0.name()).then(Ordering::Equal));
    for (document, found) in documents {
        push_document(rows, document, found, at.len() + 1, view);
    }
}

fn plain_row(key: String, kind: RowKind, level: usize, name: &str, source: &str) -> LibraryRow {
    LibraryRow {
        key,
        kind,
        level,
        name: name.to_owned(),
        line: String::new(),
        expandable: false,
        expanded: false,
        unreadable: false,
        source: source.to_owned(),
        path: Vec::new(),
        page_id: None,
        chip: None,
        picture: false,
        open: false,
        removable: false,
        matched: None,
        label: name.to_owned(),
    }
}

fn push_document(
    rows: &mut Vec<LibraryRow>,
    document: &IndexedDocument,
    found: Option<&SearchMatch>,
    level: usize,
    view: &TreeView<'_>,
) {
    let key = row_key(RowKind::Document, &document.source, &document.path, None);
    let readable = document.state == DocumentState::Readable;
    let expanded = readable
        && match found {
            Some(found) => !found.pages.is_empty(),
            None => view.expanded.contains(&key),
        };
    let saved = view.saved_version.is_some_and(|(source, path)| {
        source == document.source && path == document.path.as_slice()
    });
    let line = document_error_text(document.state).map_or_else(
        || {
            document_line(
                document.pages.len(),
                document.modified.get(),
                view.now.0,
                view.now.1,
                saved,
            )
        },
        str::to_owned,
    );
    let name = document.name();
    let time = crate::workspace_text::change_time_text(
        u64::try_from(document.modified.get().div_euclid(1000)).unwrap_or(0),
        u64::try_from(view.now.0.div_euclid(1000)).unwrap_or(0),
        view.now.1,
    );
    let first = document.pages.first();
    rows.push(LibraryRow {
        label: crate::library_index_text::document_label(
            name,
            document.pages.len(),
            &time,
            document_error_text(document.state),
        ),
        line,
        expandable: readable,
        expanded,
        unreadable: !readable,
        path: document.path.clone(),
        // The 40 x 28 row picture is the first page's (UX notes 4.2).
        page_id: first.map(|p| p.id.clone()),
        picture: first.is_some_and(|p| p.picture),
        matched: matched(name, view.query),
        ..plain_row(key, RowKind::Document, level, name, &document.source)
    });
    if expanded {
        push_pages(rows, document, found, level + 1, view.query);
    }
}

/// The page rows of an expanded document; while searching only its matching pages.
fn push_pages(
    rows: &mut Vec<LibraryRow>,
    document: &IndexedDocument,
    found: Option<&SearchMatch>,
    level: usize,
    query: &str,
) {
    for (index, page) in document.pages.iter().enumerate() {
        if found.is_some_and(|f| !f.pages.contains(&index)) {
            continue;
        }
        let key = row_key(
            RowKind::Page,
            &document.source,
            &document.path,
            Some(&page.id),
        );
        rows.push(LibraryRow {
            label: crate::library_index_text::page_label(
                &page.name,
                &page.kind,
                page.size,
                document.name(),
            ),
            line: page_size_text(page.size),
            path: document.path.clone(),
            page_id: Some(page.id.clone()),
            chip: Some(page.kind.clone()),
            picture: page.picture,
            matched: matched(&page.name, query),
            ..plain_row(key, RowKind::Page, level, &page.name, &document.source)
        });
    }
}

/// The char range of the first case-insensitive match of `query` in `name`.
fn matched(name: &str, query: &str) -> Option<(usize, usize)> {
    let needle: Vec<char> = query.trim().to_lowercase().chars().collect();
    if needle.is_empty() {
        return None;
    }
    let hay: Vec<char> = name.to_lowercase().chars().collect();
    (0..=hay.len().checked_sub(needle.len())?)
        .find(|&start| hay[start..start + needle.len()] == needle[..])
        .map(|start| (start, start + needle.len()))
}

#[cfg(test)]
#[path = "library_tree_tests.rs"]
mod tests;
