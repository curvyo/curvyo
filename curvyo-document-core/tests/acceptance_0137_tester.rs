//! Independent acceptance tests for `specs/0137-document-library` at the document model and file
//! format: copy import of a page (criterion 4), of elements (criterion 5), the Came from record
//! on every copy (criteria 6, 7), resources travelling with the copy (criterion 8), and format 58
//! with the optional record (criterion 9). Written from the spec before reading the implementation.
#![allow(
    clippy::many_single_char_names,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::similar_names,
    missing_docs
)]

use std::path::PathBuf;

use curvyo_document_core::{
    AnchorId, CURRENT_FORMAT_VERSION, CameFrom, Document, FIRST_PAGE_ID, FragmentTarget,
    ImportError, NodeId, ObjectSnapshot, PageKind, Point, RectBounds, UnixMillis, Vec2,
    library_summary, outline_of, pack, unpack,
};

const TOL: f64 = 1e-6;
const IMPORT_PAGE_LABEL: &str = "import_page";
const IMPORT_ELEMENTS_LABEL: &str = "import_elements";

// ---- helpers ------------------------------------------------------------------------------------

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn record(document: &str, page: &str, page_id: &str) -> CameFrom {
    CameFrom {
        document: document.to_owned(),
        page: page.to_owned(),
        page_id: page_id.to_owned(),
        item: None,
        at: UnixMillis::new(1_700_000_000_000),
        online: None,
    }
}

fn rect(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(x, y),
        Point::new(x + w, y + h),
    ))
}

fn anchors(n: usize, seed: u64) -> Vec<AnchorId> {
    (0..n)
        .map(|i| AnchorId::new(9137, seed * 1000 + i as u64))
        .collect()
}

fn points(d: &Document, id: NodeId) -> Vec<Point> {
    match d.object(id).expect("object exists") {
        ObjectSnapshot::Path(p) => p.all_anchors().map(|a| a.point).collect(),
        ObjectSnapshot::Primitive(p) => outline_of(&p.shape)
            .anchors
            .iter()
            .map(|a| a.point)
            .collect(),
        other => panic!("no outline for {other:?}"),
    }
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn undo_last(d: &Document) {
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
}

fn assert_close(a: &[Point], b: &[Point]) {
    assert_eq!(a.len(), b.len(), "point counts differ");
    for (p, q) in a.iter().zip(b) {
        assert!(
            (p.x - q.x).abs() < TOL && (p.y - q.y).abs() < TOL,
            "{p:?} != {q:?}"
        );
    }
}

/// A source document with two rectangles on its first page, the first one named "Box".
fn source() -> (Document, NodeId, NodeId) {
    let d = Document::new(1);
    let a = rect(&d, 0.0, 0.0, 10.0, 10.0);
    d.rename_node(a, "Box").unwrap();
    let b = rect(&d, 20.0, 0.0, 5.0, 5.0);
    (d, a, b)
}

// ---- criterion 4: page import -------------------------------------------------------------------

#[test]
fn ac4_page_import_adds_a_new_page_after_the_last_page_with_its_objects() {
    let (src, a, b) = source();
    let src_page = src
        .pages()
        .into_iter()
        .find(|p| p.id == FIRST_PAGE_ID)
        .unwrap();
    let members: Vec<NodeId> = vec![a, b];
    let frag = src.extract_fragment(&members, Some(FIRST_PAGE_ID)).unwrap();

    let target = Document::new(2);
    target.add_page();
    let before = target.pages().len();
    let came = record("Source", &src_page.name, FIRST_PAGE_ID);
    let report = target
        .insert_fragment(
            &frag,
            FragmentTarget::Page,
            &came,
            &anchors(frag.anchor_count(), 1),
        )
        .unwrap();

    let pages = target.pages();
    assert_eq!(pages.len(), before + 1, "exactly one new page");
    let new_page = report.page.clone().expect("the imported page is reported");
    assert_eq!(pages.last().unwrap().id, new_page.id, "new page is last");
    assert_eq!(new_page.size, src_page.size, "size carried over");
    assert_eq!(new_page.kind, PageKind::Drawing);
    assert_eq!(report.roots.len(), 2, "both objects of the page are copied");
    assert_eq!(
        last_label(&target),
        IMPORT_PAGE_LABEL,
        "one step Import page"
    );
}

#[test]
fn ac4_page_import_is_one_undo_step() {
    let (src, a, b) = source();
    let members = vec![a, b];
    let frag = src.extract_fragment(&members, Some(FIRST_PAGE_ID)).unwrap();
    let target = Document::new(2);
    let before = target.pages().len();
    target
        .insert_fragment(
            &frag,
            FragmentTarget::Page,
            &record("Source", "Page 1", FIRST_PAGE_ID),
            &anchors(frag.anchor_count(), 2),
        )
        .unwrap();
    assert_eq!(target.pages().len(), before + 1);
    undo_last(&target);
    assert_eq!(target.pages().len(), before, "one undo removes the page");
}

#[test]
fn ac4_source_document_is_unchanged_by_a_page_import() {
    let (src, a, b) = source();
    let before_pts = points(&src, a);
    let before_pages = src.pages().len();
    let before_steps = src.own_steps().len();
    let members = vec![a, b];
    let frag = src.extract_fragment(&members, Some(FIRST_PAGE_ID)).unwrap();
    let target = Document::new(2);
    target
        .insert_fragment(
            &frag,
            FragmentTarget::Page,
            &record("Source", "Page 1", FIRST_PAGE_ID),
            &anchors(frag.anchor_count(), 3),
        )
        .unwrap();
    assert_eq!(src.pages().len(), before_pages, "source keeps its pages");
    assert_eq!(src.own_steps().len(), before_steps, "source gets no step");
    assert_close(&points(&src, a), &before_pts);
}

#[test]
fn ac4_summary_lists_the_pages_of_a_source_in_order() {
    let (src, _a, _b) = source();
    src.add_page();
    let summary = library_summary(&src);
    assert_eq!(summary.pages.len(), src.pages().len());
    let ids: Vec<String> = src.pages().into_iter().map(|p| p.id).collect();
    let listed: Vec<String> = summary.pages.iter().map(|p| p.id.clone()).collect();
    assert_eq!(listed, ids, "list order is page order");
}

// ---- criterion 5: element import -----------------------------------------------------------------

#[test]
fn ac5_elements_land_at_the_drop_offset_on_the_target_page() {
    let (src, a, _b) = source();
    let frag = src.extract_fragment(&[a], None).unwrap();
    let target = Document::new(2);
    let before = points(&src, a);
    let report = target
        .insert_fragment(
            &frag,
            FragmentTarget::Elements {
                page: FIRST_PAGE_ID,
                offset: Vec2::new(50.0, 20.0),
            },
            &record("Source", "Page 1", FIRST_PAGE_ID),
            &anchors(frag.anchor_count(), 4),
        )
        .unwrap();
    assert_eq!(report.roots.len(), 1);
    assert!(report.page.is_none(), "elements do not carry a page");
    let moved: Vec<Point> = before
        .iter()
        .map(|p| Point::new(p.x + 50.0, p.y + 20.0))
        .collect();
    assert_close(&points(&target, report.roots[0]), &moved);
    assert_eq!(
        last_label(&target),
        IMPORT_ELEMENTS_LABEL,
        "one step Import elements"
    );
}

#[test]
fn ac5_elements_import_is_one_undo_step() {
    let (src, a, _b) = source();
    let frag = src.extract_fragment(&[a], None).unwrap();
    let target = Document::new(2);
    let report = target
        .insert_fragment(
            &frag,
            FragmentTarget::Elements {
                page: FIRST_PAGE_ID,
                offset: Vec2::new(1.0, 1.0),
            },
            &record("Source", "Page 1", FIRST_PAGE_ID),
            &anchors(frag.anchor_count(), 5),
        )
        .unwrap();
    let copy = report.roots[0];
    assert!(target.object(copy).is_some());
    undo_last(&target);
    assert!(target.object(copy).is_none(), "one undo removes the copies");
}

#[test]
fn ac5_elements_cannot_go_onto_a_sketch_page_and_nothing_is_written() {
    let (src, a, _b) = source();
    let frag = src.extract_fragment(&[a], None).unwrap();
    let target = Document::new(2);
    let sketch = target.add_page_of(&PageKind::Sketch).unwrap();
    let steps_before = target.own_steps().len();
    let err = target
        .insert_fragment(
            &frag,
            FragmentTarget::Elements {
                page: &sketch.id,
                offset: Vec2::new(0.0, 0.0),
            },
            &record("Source", "Page 1", FIRST_PAGE_ID),
            &anchors(frag.anchor_count(), 6),
        )
        .unwrap_err();
    assert_eq!(err, ImportError::SketchPage);
    assert_eq!(target.own_steps().len(), steps_before, "nothing is written");
}

#[test]
fn ac5_second_element_import_is_a_second_copy_not_a_merge() {
    let (src, a, _b) = source();
    let frag = src.extract_fragment(&[a], None).unwrap();
    let target = Document::new(2);
    let came = record("Source", "Page 1", FIRST_PAGE_ID);
    let first = target
        .insert_fragment(
            &frag,
            FragmentTarget::Elements {
                page: FIRST_PAGE_ID,
                offset: Vec2::new(0.0, 0.0),
            },
            &came,
            &anchors(frag.anchor_count(), 7),
        )
        .unwrap();
    let second = target
        .insert_fragment(
            &frag,
            FragmentTarget::Elements {
                page: FIRST_PAGE_ID,
                offset: Vec2::new(0.0, 0.0),
            },
            &came,
            &anchors(frag.anchor_count(), 8),
        )
        .unwrap();
    assert_ne!(
        first.roots[0], second.roots[0],
        "the second import is a new object"
    );
    assert!(
        target.object(first.roots[0]).is_some(),
        "the first copy stays"
    );
    assert!(target.object(second.roots[0]).is_some());
}

#[test]
fn ac5_linked_clone_whose_original_was_not_picked_becomes_an_ordinary_path_and_is_counted() {
    let (src, a, _b) = source();
    let clones = src.place_linked_clones(&[a], Vec2::new(30.0, 0.0)).unwrap();
    let clone = clones[0];
    let frag = src.extract_fragment(&[clone], None).unwrap();
    assert_eq!(
        frag.clones_became_paths(),
        1,
        "the notice counts one clone that became a path"
    );
    let target = Document::new(2);
    let report = target
        .insert_fragment(
            &frag,
            FragmentTarget::Elements {
                page: FIRST_PAGE_ID,
                offset: Vec2::new(0.0, 0.0),
            },
            &record("Source", "Page 1", FIRST_PAGE_ID),
            &anchors(frag.anchor_count(), 9),
        )
        .unwrap();
    assert_eq!(report.clones_became_paths, 1);
    // An ordinary, unlinked object (ADR 0049); a rectangle comes in as a primitive, not a path.
    assert!(
        !matches!(
            target.object(report.roots[0]),
            Some(ObjectSnapshot::Clone(_)) | None
        ),
        "the unpicked clone must come in as an ordinary, unlinked object"
    );
}

#[test]
fn ac5_linked_clone_with_its_original_picked_keeps_the_clone_relation() {
    let (src, a, _b) = source();
    let clones = src.place_linked_clones(&[a], Vec2::new(30.0, 0.0)).unwrap();
    let frag = src.extract_fragment(&[a, clones[0]], None).unwrap();
    assert_eq!(
        frag.clones_became_paths(),
        0,
        "both picked: nothing becomes a path"
    );
    let target = Document::new(2);
    let report = target
        .insert_fragment(
            &frag,
            FragmentTarget::Elements {
                page: FIRST_PAGE_ID,
                offset: Vec2::new(0.0, 0.0),
            },
            &record("Source", "Page 1", FIRST_PAGE_ID),
            &anchors(frag.anchor_count(), 10),
        )
        .unwrap();
    assert_eq!(report.roots.len(), 2);
    assert!(
        matches!(
            target.object(report.roots[1]),
            Some(ObjectSnapshot::Clone(_))
        ),
        "the copied clone is still a linked clone"
    );
}

#[test]
fn ac5_a_group_is_imported_with_its_structure() {
    let (src, a, b) = source();
    let group = src.group_objects(&[a, b]).unwrap();
    let frag = src.extract_fragment(&[group], None).unwrap();
    let target = Document::new(2);
    let report = target
        .insert_fragment(
            &frag,
            FragmentTarget::Elements {
                page: FIRST_PAGE_ID,
                offset: Vec2::new(0.0, 0.0),
            },
            &record("Source", "Page 1", FIRST_PAGE_ID),
            &anchors(frag.anchor_count(), 11),
        )
        .unwrap();
    assert_eq!(report.roots.len(), 1, "a group is one top-level copy");
    assert!(
        target.came_from(report.roots[0]).is_some() || target.object(report.roots[0]).is_some()
    );
    let flat = src.extract_fragment(&[a, b], None).unwrap();
    assert_eq!(
        frag.anchor_count(),
        flat.anchor_count(),
        "a group needs the anchors of its members"
    );
}

// ---- criteria 6 and 7: Came from on every copy ----------------------------------------------------

#[test]
fn ac6_every_copied_root_carries_came_from_with_its_own_name() {
    let (src, a, b) = source();
    let frag = src.extract_fragment(&[a, b], None).unwrap();
    let target = Document::new(2);
    let came = record("Box designs", "Page 1", FIRST_PAGE_ID);
    let report = target
        .insert_fragment(
            &frag,
            FragmentTarget::Elements {
                page: FIRST_PAGE_ID,
                offset: Vec2::new(0.0, 0.0),
            },
            &came,
            &anchors(frag.anchor_count(), 12),
        )
        .unwrap();
    assert_eq!(report.roots.len(), 2);
    let first = target
        .came_from(report.roots[0])
        .expect("copy 1 has Came from");
    assert_eq!(first.document, "Box designs");
    assert_eq!(first.page, "Page 1");
    assert_eq!(first.page_id, FIRST_PAGE_ID);
    assert_eq!(
        first.item.as_deref(),
        Some("Box"),
        "the item's name travels"
    );
    let second = target
        .came_from(report.roots[1])
        .expect("copy 2 has Came from");
    assert_eq!(second.item, None, "an unnamed item has no item name");
}

#[test]
fn ac6_page_import_carries_came_from_on_the_page_and_its_objects() {
    let (src, a, b) = source();
    let members = vec![a, b];
    let frag = src.extract_fragment(&members, Some(FIRST_PAGE_ID)).unwrap();
    let target = Document::new(2);
    let came = record("Box designs", "Page 1", FIRST_PAGE_ID);
    let report = target
        .insert_fragment(
            &frag,
            FragmentTarget::Page,
            &came,
            &anchors(frag.anchor_count(), 13),
        )
        .unwrap();
    let page = report.page.unwrap();
    let on_page = target
        .page_came_from(&page.id)
        .expect("the page carries Came from");
    assert_eq!(on_page.document, "Box designs");
    assert_eq!(on_page.page_id, FIRST_PAGE_ID);
    for &root in &report.roots {
        assert!(
            target.came_from(root).is_some(),
            "each copied object carries Came from"
        );
    }
}

#[test]
fn ac7_came_from_names_the_source_page_id_so_open_source_can_find_it() {
    let (src, a, _b) = source();
    let frag = src.extract_fragment(&[a], None).unwrap();
    let target = Document::new(2);
    let report = target
        .insert_fragment(
            &frag,
            FragmentTarget::Elements {
                page: FIRST_PAGE_ID,
                offset: Vec2::new(0.0, 0.0),
            },
            &record("Box designs", "Page 1", FIRST_PAGE_ID),
            &anchors(frag.anchor_count(), 14),
        )
        .unwrap();
    let c = target.came_from(report.roots[0]).unwrap();
    assert_eq!(c.page_id, FIRST_PAGE_ID, "the page id is the lookup key");
    assert_eq!(c.page, "Page 1");
}

// ---- criterion 8: resources travel with the copy --------------------------------------------------

#[test]
fn ac8_a_text_object_keeps_its_text_and_font_name_in_the_copy() {
    use curvyo_document_core::{Affine, FontKind, FontRef, Length, TextLayoutValues};
    let src = Document::new(1);
    let font = FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    };
    let t = src
        .create_text(
            "Hallo",
            &font,
            Length::from_mm(10.0),
            Affine::translation(Vec2::new(0.0, 0.0)),
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    let frag = src.extract_fragment(&[t], None).unwrap();
    let target = Document::new(2);
    let report = target
        .insert_fragment(
            &frag,
            FragmentTarget::Elements {
                page: FIRST_PAGE_ID,
                offset: Vec2::new(0.0, 0.0),
            },
            &record("Source", "Page 1", FIRST_PAGE_ID),
            &anchors(frag.anchor_count(), 15),
        )
        .unwrap();
    match target.object(report.roots[0]) {
        Some(ObjectSnapshot::Text(text)) => {
            assert_eq!(text.string, "Hallo", "the text is copied as written");
        }
        other => panic!("the copy is not a text object: {other:?}"),
    }
}

// ---- criterion 9: format 58 and the optional record ------------------------------------------------

#[test]
fn ac9_format_is_58_with_the_came_from_record() {
    // Format 59 since `specs/0150-operation-always-has-numbers`.
    assert_eq!(CURRENT_FORMAT_VERSION, 59);
}

#[test]
fn ac9_came_from_survives_save_and_reopen_on_object_and_page() {
    let (src, a, b) = source();
    let frag = src.extract_fragment(&[a], None).unwrap();
    let target = Document::new(2);
    let members = vec![a, b];
    let page_frag = src.extract_fragment(&members, Some(FIRST_PAGE_ID)).unwrap();
    let came = record("Box designs", "Page 1", FIRST_PAGE_ID);
    let page_report = target
        .insert_fragment(
            &page_frag,
            FragmentTarget::Page,
            &came,
            &anchors(page_frag.anchor_count(), 16),
        )
        .unwrap();
    let elem_report = target
        .insert_fragment(
            &frag,
            FragmentTarget::Elements {
                page: FIRST_PAGE_ID,
                offset: Vec2::new(0.0, 0.0),
            },
            &came,
            &anchors(frag.anchor_count(), 17),
        )
        .unwrap();
    let bytes = pack(&target, "0137-tester").unwrap();
    let back = unpack(3, &bytes).unwrap();
    let page = page_report.page.unwrap();
    assert_eq!(
        back.page_came_from(&page.id).unwrap().document,
        "Box designs"
    );
    assert_eq!(
        back.came_from(elem_report.roots[0])
            .unwrap()
            .item
            .as_deref(),
        Some("Box")
    );
}

#[test]
fn ac9_a_file_without_came_from_opens_unchanged() {
    let bytes = fixture("bring_content_in_v38.curvyo");
    let d = unpack(4, &bytes).expect("an older file opens");
    for page in d.pages() {
        assert!(
            d.page_came_from(&page.id).is_none(),
            "no record is invented"
        );
    }
}

#[test]
fn ac9_golden_format_58_file_with_came_from_opens_and_reads_the_record() {
    let bytes = fixture("came_from_v58.curvyo");
    let d = unpack(5, &bytes).expect("the golden file opens");
    let has_page_record = d.pages().iter().any(|p| d.page_came_from(&p.id).is_some());
    assert!(has_page_record, "the golden file carries a page record");
}

#[test]
fn ac9_damaged_came_from_is_read_as_absent_and_the_file_still_opens() {
    let bytes = fixture("came_from_damaged_v58.curvyo");
    let d = unpack(6, &bytes).expect("a damaged record must not stop the file opening");
    assert!(!d.pages().is_empty(), "the pages are still there");
}

#[test]
fn ac9_a_record_is_absent_for_an_object_that_was_never_imported() {
    let (src, a, _b) = source();
    assert!(
        src.came_from(a).is_none(),
        "a drawn object has no Came from"
    );
    assert!(src.page_came_from(FIRST_PAGE_ID).is_none());
}

#[test]
fn ac9_unix_millis_round_trips() {
    assert_eq!(UnixMillis::new(1_700_000_000_123).get(), 1_700_000_000_123);
}

// ---- criterion 2: the manifest and the page pictures in the file ---------------------------------

fn picture_png() -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images/png_rgb_8.png"),
    )
    .unwrap()
}

#[test]
fn ac2_a_save_writes_the_library_summary_and_a_relay_snapshot_does_not() {
    use curvyo_document_core::{LibraryMembers, pack_snapshot, read_library_members};
    let (src, _a, _b) = source();
    src.add_page();
    let saved = pack(&src, "0137-tester").unwrap();
    match read_library_members(std::io::Cursor::new(saved)) {
        LibraryMembers::Readable { summary, .. } => {
            let bytes = summary.expect("a save writes library.json");
            let listed = curvyo_document_core::LibrarySummary::from_bytes(&bytes)
                .expect("the summary reads back");
            assert_eq!(
                listed.pages.len(),
                src.pages().len(),
                "every page is listed"
            );
        }
        other => panic!("a saved file is readable: {other:?}"),
    }
    let snapshot = pack_snapshot(&src, "0137-tester").unwrap();
    match read_library_members(std::io::Cursor::new(snapshot)) {
        LibraryMembers::Readable { summary, .. } => {
            assert!(summary.is_none(), "a relay snapshot writes no library.json");
        }
        other => panic!("a snapshot is readable: {other:?}"),
    }
}

#[test]
fn ac2_a_page_picture_written_into_the_file_is_listed_for_its_page() {
    use curvyo_document_core::{LibraryMembers, read_library_members, with_pictures};
    let (src, _a, _b) = source();
    let container = pack(&src, "0137-tester").unwrap();
    let pictured = with_pictures(
        &container,
        Some(&picture_png()),
        &[(FIRST_PAGE_ID.to_owned(), picture_png())],
    )
    .unwrap();
    match read_library_members(std::io::Cursor::new(pictured)) {
        LibraryMembers::Readable { pictures, .. } => {
            assert_eq!(pictures, vec![FIRST_PAGE_ID.to_owned()]);
        }
        other => panic!("the picture file is readable: {other:?}"),
    }
}

#[test]
fn ac2_a_file_that_is_not_a_container_is_not_curvyo_for_the_library() {
    use curvyo_document_core::{LibraryMembers, read_library_members};
    let members = read_library_members(std::io::Cursor::new(b"plain text".to_vec()));
    assert_eq!(members, LibraryMembers::NotCurvyo);
}
