//! Tests of `library_tree`.
use super::*;
use crate::library_index::{IndexedDocument, IndexedPage};
use curvyo_document_core::DocumentSize;

fn document(source: &str, path: &[&str], pages: &[&str]) -> IndexedDocument {
    IndexedDocument {
        source: source.into(),
        path: path.iter().map(|p| (*p).to_owned()).collect(),
        size: 1,
        modified: curvyo_document_core::UnixMillis::new(0),
        state: DocumentState::Readable,
        pages: pages
            .iter()
            .enumerate()
            .map(|(i, name)| IndexedPage {
                id: format!("{source}-{i}"),
                name: (*name).to_owned(),
                kind: "drawing".into(),
                size: DocumentSize::from_mm(100.0, 50.0),
                picture: false,
            })
            .collect(),
    }
}

fn setup() -> (Vec<LibrarySource>, LibraryIndex) {
    let sources = vec![
        LibrarySource {
            key: "ws".into(),
            name: "Box job".into(),
            kind: SourceKind::OpenWorkspace,
            error: None,
        },
        LibrarySource {
            key: "/logos".into(),
            name: "Logos".into(),
            kind: SourceKind::Folder,
            error: None,
        },
        LibrarySource {
            key: "/old".into(),
            name: "Old jobs".into(),
            kind: SourceKind::Folder,
            error: Some(SourceReason::Io),
        },
    ];
    let mut index = LibraryIndex::default();
    index.put(document("ws", &["Lid 10.curvyo"], &["Front"]));
    index.put(document("ws", &["Lid 2.curvyo"], &["Front", "Back"]));
    index.put(document("ws", &["Sides", "Left.curvyo"], &["Lid inner"]));
    index.put(document("/logos", &["Star.curvyo"], &["One"]));
    (sources, index)
}

fn names(rows: &[LibraryRow]) -> Vec<String> {
    rows.iter()
        .map(|r| format!("{}{}", "  ".repeat(r.level), r.name))
        .collect()
}

#[test]
fn the_tree_lists_workspaces_then_folders_with_folders_first_in_natural_order() {
    let (sources, index) = setup();
    let mut expanded = HashSet::new();
    expanded.insert(row_key(RowKind::Source, "ws", &[], None));
    expanded.insert(row_key(
        RowKind::Document,
        "ws",
        &["Lid 2.curvyo".into()],
        None,
    ));
    let view = TreeView {
        expanded: &expanded,
        query: "",
        now: (0, 0),
        saved_version: Some(("ws", &["Lid 2.curvyo".to_owned()])),
    };
    let rows = library_rows(&sources, &index, &view);
    assert_eq!(
        names(&rows),
        [
            "Workspaces",
            "Box job",
            "  Sides",
            "  Lid 2",
            "    Front",
            "    Back",
            "  Lid 10",
            "Folders",
            "Logos",
            "Old jobs"
        ]
    );
    assert_eq!(rows[1].line, "3");
    assert!(rows[3].line.starts_with("Saved version · 2 pages"));
    assert_eq!(rows[4].line, "100 × 50 mm");
    assert_eq!(rows[9].line, "Cannot be read: drive not connected.");
    assert!(rows[9].removable && rows[9].unreadable);
}

#[test]
fn the_search_keeps_matching_documents_collapsed_and_shows_matching_pages() {
    let (sources, index) = setup();
    let view = TreeView {
        expanded: &HashSet::new(),
        query: "lid",
        now: (0, 0),
        saved_version: None,
    };
    let rows = library_rows(&sources, &index, &view);
    assert_eq!(
        names(&rows),
        [
            "Workspaces",
            "Box job",
            "  Sides",
            "    Left",
            "      Lid inner",
            "  Lid 2",
            "  Lid 10"
        ]
    );
    assert!(!rows[5].expanded);
    assert_eq!(rows[5].matched, Some((0, 3)));
    assert_eq!(rows[4].matched, Some((0, 3)));
    let none = TreeView {
        query: "zebra",
        ..view
    };
    assert_eq!(library_rows(&sources, &index, &none), Vec::new());
}

#[test]
fn the_browser_lists_its_workspaces_under_in_this_browser_with_spoken_names() {
    let sources = vec![LibrarySource {
        key: "ws".into(),
        name: "Box job".into(),
        kind: SourceKind::Stored(true),
        error: None,
    }];
    let mut index = LibraryIndex::default();
    let mut doc = document("ws", &["Lid.curvyo"], &["Front"]);
    doc.pages[0].picture = true;
    index.put(doc);
    let mut expanded = HashSet::new();
    expanded.insert(row_key(RowKind::Source, "ws", &[], None));
    expanded.insert(row_key(
        RowKind::Document,
        "ws",
        &["Lid.curvyo".into()],
        None,
    ));
    let view = TreeView {
        expanded: &expanded,
        query: "",
        now: (0, 0),
        saved_version: None,
    };
    let rows = library_rows(&sources, &index, &view);
    assert_eq!(rows[0].name, "In this browser");
    assert!(rows[1].open);
    assert_eq!(
        rows[1].label,
        "Box job, workspace in this browser, 1 document"
    );
    assert_eq!(rows[2].label, "Lid, document, 1 page, changed 00:00");
    assert_eq!(
        (rows[2].page_id.as_deref(), rows[2].picture),
        (Some("ws-0"), true)
    );
    assert_eq!(
        rows[3].label,
        "Front, drawing page, 100 by 50 millimetres, in Lid"
    );
}
