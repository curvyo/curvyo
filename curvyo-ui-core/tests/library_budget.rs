//! The library's time budgets in thread CPU time (`specs/0137-document-library` criteria 3 and
//! 13): the search over 1,000 documents within 100 ms, the tree rows of 500 documents with 8
//! pages each within 200 ms.

#![allow(clippy::unwrap_used, clippy::cast_precision_loss)]

use std::collections::HashSet;
use std::time::Duration;

use curvyo_document_core::DocumentSize;
use curvyo_ui_core::library_index::{DocumentState, IndexedDocument, IndexedPage, LibraryIndex};
use curvyo_ui_core::library_tree::{
    LibrarySource, RowKind, SourceKind, TreeView, library_rows, row_key,
};

fn thread_cpu() -> Duration {
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<u64>().ok())
        .map_or_else(
            || {
                static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
                START.get_or_init(std::time::Instant::now).elapsed()
            },
            Duration::from_nanos,
        )
}

fn index(documents: usize, pages: usize) -> LibraryIndex {
    let mut index = LibraryIndex::default();
    for d in 0..documents {
        index.put(IndexedDocument {
            source: "ws".into(),
            path: vec![format!("Folder {}", d % 20), format!("Box lid {d}.curvyo")],
            size: 1,
            modified: curvyo_document_core::UnixMillis::new(0),
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

fn source() -> Vec<LibrarySource> {
    vec![LibrarySource {
        key: "ws".into(),
        name: "Box job".into(),
        kind: SourceKind::OpenWorkspace,
        error: None,
    }]
}

/// Criterion 3: the search of 1,000 documents takes at most 100 ms of thread CPU time.
#[test]
fn the_search_of_1000_documents_takes_at_most_100_ms() {
    let index = index(1000, 8);
    let start = thread_cpu();
    let found = index.search("side 3");
    let rows = library_rows(
        &source(),
        &index,
        &TreeView {
            expanded: &HashSet::new(),
            query: "side 3",
            now: (0, 0),
            saved_version: None,
        },
    );
    let took = thread_cpu().checked_sub(start).unwrap();
    assert_eq!(found.len(), 1000);
    assert!(rows.len() > 2000);
    assert!(took <= Duration::from_millis(100), "{took:?}");
}

/// Criterion 13: every row of 500 documents with 8 pages, all expanded, within 200 ms.
#[test]
fn the_rows_of_500_documents_take_at_most_200_ms() {
    let index = index(500, 8);
    let mut expanded = HashSet::new();
    expanded.insert(row_key(RowKind::Source, "ws", &[], None));
    for d in index.documents() {
        expanded.insert(row_key(RowKind::Folder, "ws", &d.path[..1], None));
        expanded.insert(row_key(RowKind::Document, "ws", &d.path, None));
    }
    let start = thread_cpu();
    let rows = library_rows(
        &source(),
        &index,
        &TreeView {
            expanded: &expanded,
            query: "",
            now: (0, 0),
            saved_version: None,
        },
    );
    let took = thread_cpu().checked_sub(start).unwrap();
    assert_eq!(rows.len(), 2 + 20 + 500 * 9);
    assert!(took <= Duration::from_millis(200), "{took:?}");
}
