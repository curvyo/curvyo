//! Second acceptance round of `specs/0137-document-library` in the interaction layer: the time
//! budgets in thread CPU time (criteria 3 and 13), the Saved version state of the open document
//! (criterion 11), the page-gone and source-gone notices and the Layers and Pages Came from
//! tooltips (criterion 7), and the "text left its path" notice (criterion 5 and the fix of 1700aa54).
//! Written from the spec; the first round's file covers the words.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    missing_docs
)]

use std::collections::HashSet;
use std::time::{Duration, Instant};

use curvyo_document_core::{CameFrom, DocumentSize, UnixMillis};
use curvyo_ui_core::library_index::{DocumentState, IndexedDocument, IndexedPage, LibraryIndex};
use curvyo_ui_core::library_index_text::{
    came_from_tooltip, page_gone_text, source_gone_text, texts_released_text,
};
use curvyo_ui_core::library_tree::{
    LibrarySource, RowKind, SourceKind, TreeView, library_rows, row_key,
};

static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// The thread's CPU time (Linux schedstat); a wall clock only where the file is missing.
fn thread_cpu() -> Duration {
    let schedstat = std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<u64>().ok());
    if let Some(ns) = schedstat {
        return Duration::from_nanos(ns);
    }
    START.get_or_init(Instant::now).elapsed()
}

fn library(documents: usize, pages: usize) -> LibraryIndex {
    let mut index = LibraryIndex::default();
    for d in 0..documents {
        index.put(IndexedDocument {
            source: "ws".into(),
            path: vec![format!("Folder {}", d % 20), format!("Box lid {d}.curvyo")],
            size: 1,
            modified: UnixMillis::new(0),
            state: DocumentState::Readable,
            pages: (0..pages)
                .map(|p| IndexedPage {
                    id: format!("{d}-{p}"),
                    name: format!("Side {p}"),
                    kind: "drawing".into(),
                    size: DocumentSize::from_mm(210.0, 297.0),
                    picture: true,
                })
                .collect(),
        });
    }
    index
}

fn sources() -> Vec<LibrarySource> {
    vec![LibrarySource {
        key: "ws".into(),
        name: "Box job".into(),
        kind: SourceKind::OpenWorkspace,
        error: None,
    }]
}

fn view<'a>(
    expanded: &'a HashSet<String>,
    query: &'a str,
    saved: Option<(&'a str, &'a [String])>,
) -> TreeView<'a> {
    TreeView {
        expanded,
        query,
        now: (1_700_000_500_000, 0),
        saved_version: saved,
    }
}

/// The source and its folders expanded, so the documents' rows are on the screen.
fn expanded_to_documents(folders: usize) -> HashSet<String> {
    let mut keys = HashSet::new();
    keys.insert(row_key(RowKind::Source, "ws", &[], None));
    for k in 0..folders {
        keys.insert(row_key(
            RowKind::Folder,
            "ws",
            &[format!("Folder {k}")],
            None,
        ));
    }
    keys
}

// ---- criterion 3: search within 100 ms of thread CPU time for 1,000 documents ---------------------

#[test]
fn ac3_search_over_1000_documents_takes_under_100_ms_of_thread_cpu() {
    let index = library(1_000, 2);
    let _ = index.search("lid");
    let start = thread_cpu();
    let hits = index.search("LID 9");
    let spent = thread_cpu().saturating_sub(start);
    assert_ne!(hits.len(), 0);
    assert!(
        spent < Duration::from_millis(100),
        "search took {spent:?} of thread CPU"
    );
}

// ---- criterion 13: 500 documents with 8 pages within 200 ms of thread CPU -----------------------

#[test]
fn ac13_the_first_screen_of_500_documents_with_8_pages_takes_under_200_ms_of_thread_cpu() {
    let index = library(500, 8);
    let expanded = expanded_to_documents(20);
    let src = sources();
    let _ = library_rows(&src, &index, &view(&expanded, "", None));
    let start = thread_cpu();
    let rows = library_rows(&src, &index, &view(&expanded, "", None));
    let spent = thread_cpu().saturating_sub(start);
    assert!(
        rows.len() >= 500,
        "every document has a row: {}",
        rows.len()
    );
    assert!(
        spent < Duration::from_millis(200),
        "rows took {spent:?} of thread CPU"
    );
}

// ---- criterion 11: Saved version is the state of the open document only -----------------------

#[test]
fn ac11_only_the_open_document_shows_the_saved_version_label() {
    let index = library(3, 1);
    let expanded = expanded_to_documents(3);
    let open = vec!["Folder 1".to_owned(), "Box lid 1.curvyo".to_owned()];
    let rows = library_rows(
        &sources(),
        &index,
        &view(&expanded, "", Some(("ws", open.as_slice()))),
    );
    let docs: Vec<_> = rows
        .iter()
        .filter(|r| r.kind == RowKind::Document)
        .collect();
    let labelled: Vec<&str> = docs
        .iter()
        .filter(|r| r.line.starts_with("Saved version"))
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(labelled.len(), 1, "exactly the open document: {labelled:?}");
    assert_eq!(
        labelled[0], "Box lid 1",
        "the label is on the open document's row"
    );
    assert_eq!(
        docs.iter()
            .filter(|r| !r.line.contains("Saved version"))
            .count(),
        2
    );
}

#[test]
fn ac11_without_an_open_document_no_row_claims_a_saved_version() {
    let index = library(2, 1);
    let expanded = HashSet::new();
    let rows = library_rows(&sources(), &index, &view(&expanded, "", None));
    assert!(rows.iter().all(|r| !r.line.contains("Saved version")));
}

// ---- criterion 7: notices and tooltips -----------------------------------------------------------

#[test]
fn ac7_the_page_gone_notice_names_the_document_and_the_page() {
    let text = page_gone_text("Box job", "Page 2");
    assert!(
        text.contains("Box job") && text.contains("Page 2"),
        "{text}"
    );
    let source = source_gone_text("Box job");
    assert!(source.contains("Box job"), "{source}");
}

#[test]
fn ac7_the_layers_and_pages_tooltip_reads_from_document_to_page() {
    let record = CameFrom {
        document: "Box job".to_owned(),
        page: "Lid".to_owned(),
        page_id: "p-1".to_owned(),
        item: Some("Lid tab".to_owned()),
        at: UnixMillis::new(1_700_000_000_000),
    };
    let tip = came_from_tooltip(&record, 1_700_000_500_000, 0);
    assert!(tip.contains("From Box job › Lid"), "{tip}");
}

// ---- criterion 5 and the 1700aa54 fix: text left its path -----------------------------------------

#[test]
fn ac5_a_text_that_left_its_path_is_said_with_a_count() {
    assert_eq!(texts_released_text(0), None, "nothing to say for none");
    let one = texts_released_text(1).expect("one text is reported");
    assert!(
        one.contains("1 text") && one.contains("left its path"),
        "{one}"
    );
    let two = texts_released_text(2).expect("two texts are reported");
    assert!(two.contains('2'), "{two}");
}
