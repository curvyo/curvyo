//! Independent acceptance tests for `specs/0137-document-library` in the interaction layer: the
//! index and its search (criterion 3), the document and source error words (criteria 1, 10), the
//! Saved version label (criterion 11) and the Came from line and Open source lookup (criterion 7).
//! Written from the spec before reading the implementation.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use curvyo_document_core::{CameFrom, DocumentSize, UnixMillis};
use curvyo_ui_core::library_index::{DocumentState, IndexedDocument, IndexedPage, LibraryIndex};
use curvyo_ui_core::library_index_text::{
    SourceReason, came_from_text, document_error_text, document_line, source_error_text,
};

fn page(id: &str, name: &str) -> IndexedPage {
    IndexedPage {
        id: id.to_owned(),
        name: name.to_owned(),
        kind: "drawing".to_owned(),
        size: DocumentSize::from_mm(100.0, 60.0),
        picture: true,
    }
}

fn doc(source: &str, file: &str, pages: Vec<IndexedPage>) -> IndexedDocument {
    IndexedDocument {
        source: source.to_owned(),
        path: vec![file.to_owned()],
        size: 1_000,
        modified: UnixMillis::new(1_700_000_000_000),
        state: DocumentState::Readable,
        pages,
    }
}

fn record(document: &str, page: &str, page_id: &str) -> CameFrom {
    CameFrom {
        document: document.to_owned(),
        page: page.to_owned(),
        page_id: page_id.to_owned(),
        item: None,
        at: UnixMillis::new(1_700_000_000_000),
    }
}

fn library() -> LibraryIndex {
    let mut index = LibraryIndex::default();
    index.put(doc(
        "ws-1",
        "Box job.curvyo",
        vec![page("p-1", "Lid"), page("p-2", "Base")],
    ));
    index.put(doc("ws-1", "Logos.curvyo", vec![page("p-3", "Cut")]));
    index.put(doc(
        "folder:/x",
        "Last year.curvyo",
        vec![page("p-4", "Lid")],
    ));
    index
}

// ---- criterion 3: search ------------------------------------------------------------------------

#[test]
fn ac3_search_matches_document_names_as_a_case_insensitive_substring() {
    let index = library();
    let hits = index.search("BOX");
    assert_eq!(
        hits.len(),
        1,
        "only the document whose name holds box: {hits:?}"
    );
    assert_eq!(index.documents()[hits[0].document].name(), "Box job");
    assert!(hits[0].by_name, "the document matched by its name");
}

#[test]
fn ac3_search_matches_page_names_and_lists_the_matching_pages() {
    let index = library();
    let hits = index.search("lid");
    let names: Vec<&str> = hits
        .iter()
        .map(|h| index.documents()[h.document].name())
        .collect();
    assert_eq!(names.len(), 2, "two documents have a page Lid: {names:?}");
    let box_hit = hits
        .iter()
        .find(|h| index.documents()[h.document].name() == "Box job")
        .unwrap();
    assert!(!box_hit.by_name, "matched by a page, not the name");
    assert_eq!(box_hit.pages.len(), 1, "only the matching page is listed");
    let p = &index.documents()[box_hit.document].pages[box_hit.pages[0]];
    assert_eq!(p.name, "Lid");
}

#[test]
fn ac3_search_finds_nothing_for_an_unknown_text_and_nothing_on_an_empty_index() {
    assert_eq!(library().search("zzz-no-such").len(), 0);
    assert_eq!(LibraryIndex::default().search("box").len(), 0);
}

#[test]
fn ac3_an_index_reads_back_its_text_and_a_damaged_text_gives_an_empty_index() {
    let index = library();
    let back = LibraryIndex::parse(&index.to_text());
    assert_eq!(back.documents().len(), index.documents().len());
    assert!(
        LibraryIndex::parse("{ not an index").documents().is_empty(),
        "a damaged cache is rebuilt, never fatal"
    );
}

#[test]
fn ac3_a_document_needs_reading_only_when_its_size_or_time_changed() {
    let mut index = library();
    assert!(!index.needs_read(
        "ws-1",
        &["Logos.curvyo".to_owned()],
        1_000,
        UnixMillis::new(1_700_000_000_000)
    ));
    assert!(index.needs_read(
        "ws-1",
        &["Logos.curvyo".to_owned()],
        1_001,
        UnixMillis::new(1_700_000_000_000)
    ));
    index.keep_present("ws-1", &[vec!["Box job.curvyo".to_owned()]]);
    assert!(
        index
            .documents()
            .iter()
            .all(|d| d.source != "ws-1" || d.name() == "Box job"),
        "a document that is gone from its source leaves the index"
    );
}

// ---- criteria 1, 10: error words ----------------------------------------------------------------

#[test]
fn ac10_an_unreadable_source_says_cannot_be_read_with_its_reason() {
    assert!(source_error_text(SourceReason::NotFound).contains("Cannot be read"));
    assert!(source_error_text(SourceReason::NoAccess).contains("Cannot be read"));
    assert!(source_error_text(SourceReason::Io).contains("Cannot be read"));
    assert_eq!(SourceReason::from_code("not_found"), SourceReason::NotFound);
    assert_eq!(SourceReason::from_code("no_access"), SourceReason::NoAccess);
    assert_eq!(SourceReason::from_code("io"), SourceReason::Io);
}

#[test]
fn ac10_a_damaged_or_newer_document_is_marked_cannot_be_read_and_a_readable_one_is_not() {
    assert!(
        document_error_text(DocumentState::Damaged)
            .unwrap()
            .contains("Cannot be read")
    );
    assert!(
        document_error_text(DocumentState::Newer)
            .unwrap()
            .contains("Cannot be read")
    );
    assert!(
        document_error_text(DocumentState::NotCurvyo)
            .unwrap()
            .contains("Cannot be read")
    );
    assert!(document_error_text(DocumentState::Readable).is_none());
}

// ---- criterion 11: Saved version ----------------------------------------------------------------

#[test]
fn ac11_the_saved_version_label_is_shown_only_for_the_saved_state() {
    let now = 1_700_000_500_000;
    let saved = document_line(3, 1_700_000_000_000, now, 0, true);
    let plain = document_line(3, 1_700_000_000_000, now, 0, false);
    assert!(saved.starts_with("Saved version"), "label first: {saved}");
    assert!(saved.contains("3 pages"));
    assert!(
        !plain.contains("Saved version"),
        "no label without the flag: {plain}"
    );
}

// ---- criterion 7: Came from line and Open source lookup ------------------------------------------

#[test]
fn ac7_the_came_from_line_reads_from_document_to_page() {
    let r = record("Box job", "Lid", "p-1");
    assert_eq!(came_from_text(&r), "From Box job › Lid");
}

#[test]
fn ac7_open_source_finds_the_document_by_its_page_id_even_when_renamed() {
    let index = library();
    // The record names an old document name; the page id still finds the page's document.
    let r = record("Old box name", "Lid", "p-1");
    let found = index.source_of(&r).expect("the page id finds the source");
    assert_eq!(found.name(), "Box job", "found by page id, not by name");
}

#[test]
fn ac7_with_several_copies_of_the_page_the_one_with_the_same_name_wins() {
    let index = library();
    // Last year.curvyo holds page p-4 too; a record named "Last year" must pick that copy.
    let mut both = library();
    both.put(doc("ws-1", "Box job copy.curvyo", vec![page("p-9", "Lid")]));
    let r = record("Last year", "Lid", "p-4");
    assert_eq!(both.source_of(&r).unwrap().name(), "Last year");
    assert!(
        index
            .source_of(&record("Gone", "Lid", "p-missing"))
            .is_none()
    );
}

#[test]
fn ac7_no_source_for_a_page_that_is_gone_gives_none_and_the_line_stays() {
    let index = library();
    let r = record("Gone job", "Lid", "p-gone");
    assert!(index.source_of(&r).is_none(), "the button is absent");
    assert!(
        came_from_text(&r).contains("From Gone job"),
        "the line stays"
    );
}
