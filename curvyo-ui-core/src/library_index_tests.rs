//! Tests of `library_index`.
use super::*;

fn page(id: &str, name: &str) -> IndexedPage {
    IndexedPage {
        id: id.into(),
        name: name.into(),
        kind: "drawing".into(),
        size: DocumentSize::from_mm(210.0, 297.0),
        picture: true,
    }
}

fn document(source: &str, path: &[&str], pages: Vec<IndexedPage>) -> IndexedDocument {
    IndexedDocument {
        source: source.into(),
        path: path.iter().map(|p| (*p).to_owned()).collect(),
        size: 10,
        modified: UnixMillis::new(1000),
        state: DocumentState::Readable,
        pages,
    }
}

fn index() -> LibraryIndex {
    let mut index = LibraryIndex::default();
    index.put(document("ws", &["Lid.curvyo"], vec![page("1-1", "Front")]));
    index.put(document(
        "ws",
        &["Sides", "Box.curvyo"],
        vec![page("1-2", "Lid top"), page("1-3", "Back")],
    ));
    index.put(document(
        "/logos",
        &["Lid.curvyo"],
        vec![page("1-1", "Front")],
    ));
    index
}

#[test]
fn the_cache_reads_back_and_a_damaged_or_newer_one_is_empty() {
    let index = index();
    assert_eq!(LibraryIndex::parse(&index.to_text()), index);
    assert_eq!(LibraryIndex::parse("{"), LibraryIndex::default());
    let newer = index.to_text().replacen("\"format\":1", "\"format\":2", 1);
    assert_eq!(LibraryIndex::parse(&newer), LibraryIndex::default());
}

#[test]
fn a_document_is_read_again_when_its_size_or_time_changes() {
    let index = index();
    let path = vec!["Lid.curvyo".to_owned()];
    assert!(!index.needs_read("ws", &path, 10, UnixMillis::new(1000)));
    assert!(index.needs_read("ws", &path, 11, UnixMillis::new(1000)));
    assert!(index.needs_read("ws", &path, 10, UnixMillis::new(1001)));
    assert!(index.needs_read("other", &path, 10, UnixMillis::new(1000)));
}

#[test]
fn the_search_finds_documents_and_pages_ignoring_case() {
    let index = index();
    let found = index.search("LID");
    assert_eq!(found.len(), 3);
    assert!(found[0].by_name && found[0].pages.is_empty());
    assert_eq!((found[1].by_name, found[1].pages.clone()), (false, vec![0]));
    assert_eq!(index.search("  "), Vec::new());
    assert_eq!(index.search("zebra"), Vec::new());
}

#[test]
fn open_source_finds_the_page_id_and_prefers_the_same_name() {
    let mut index = index();
    let record = CameFrom {
        document: "Lid".into(),
        page: "Front".into(),
        page_id: "1-1".into(),
        item: None,
        at: UnixMillis::new(0),
    };
    assert_eq!(index.source_of(&record).unwrap().source, "ws");
    let renamed = CameFrom {
        document: "Other".into(),
        ..record.clone()
    };
    assert_eq!(index.source_of(&renamed).unwrap().source, "ws");
    index.order_by_sources(&["/logos".into(), "ws".into()]);
    assert_eq!(index.source_of(&renamed).unwrap().source, "/logos");
    let gone = CameFrom {
        page_id: "9-9".into(),
        ..record
    };
    assert!(index.source_of(&gone).is_none());
}

#[test]
fn a_newer_file_has_no_pages_and_pictures_are_marked() {
    let summary = LibrarySummary {
        pages: vec![curvyo_document_core::PageSummary {
            id: "first".into(),
            name: "Page 1".into(),
            kind: "drawing".into(),
            size: DocumentSize::from_mm(100.0, 100.0),
        }],
    };
    let (state, pages) = readable_pages(57, Some(&summary), &["first".into()]);
    assert_eq!(state, DocumentState::Readable);
    assert!(pages[0].picture);
    let (state, pages) = readable_pages(CURRENT_FORMAT_VERSION + 1, Some(&summary), &[]);
    assert_eq!((state, pages.len()), (DocumentState::Newer, 0));
}

#[test]
fn a_container_becomes_a_record_with_its_pages() {
    let document = curvyo_document_core::Document::new(1);
    let bytes = curvyo_document_core::pack(&document, "t").unwrap();
    let record = record_of_container("ws", vec!["Lid.curvyo".into()], UnixMillis::new(5), &bytes);
    assert_eq!(
        (record.state, record.pages.len()),
        (DocumentState::Readable, 1)
    );
    assert_eq!(record.size, bytes.len() as u64);
    let text = record_of_container("ws", vec!["x.curvyo".into()], UnixMillis::new(5), b"text");
    assert_eq!(text.state, DocumentState::NotCurvyo);
}

#[test]
fn a_file_without_the_page_list_is_read_natively() {
    let document = curvyo_document_core::Document::new(1);
    let bytes = curvyo_document_core::pack(&document, "t").unwrap();
    assert_eq!(fallback_summary(&bytes).unwrap().pages.len(), 1);
    assert!(fallback_summary(b"nope").is_none());
}
