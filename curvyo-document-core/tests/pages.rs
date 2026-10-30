//! Spatial page commands preserve independent frames and their history.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Document, DocumentSize, Point, Vec2, pack, unpack};

#[test]
fn pages_edit_roundtrip_and_undo() {
    let doc = Document::new(71);
    let first = doc.pages()[0].clone();
    assert_eq!(first.origin, Point::new(0.0, 0.0));
    let second = doc.add_page();
    assert_eq!(doc.pages().len(), 2);
    assert_eq!(second.size, first.size);
    doc.rename_page(&second.id, "Coasters 🐈").unwrap();
    doc.resize_page(&second.id, DocumentSize::from_mm(100.0, 120.0), &[])
        .unwrap();
    doc.move_page(&second.id, Vec2::new(-30.0, 40.0), &[])
        .unwrap();
    doc.reorder_page(&second.id, 0).unwrap();
    assert_eq!(doc.pages()[0].id, second.id);
    let restored = unpack(72, &pack(&doc, "pages").unwrap()).unwrap();
    assert_eq!(restored.pages(), doc.pages());
    doc.delete_page(&second.id).unwrap();
    assert_eq!(doc.pages(), vec![first]);
    let step = doc.own_steps().last().unwrap().step;
    doc.undo_step(step).unwrap();
    assert_eq!(doc.pages().len(), 2);
}

#[test]
fn invalid_or_unchanged_edits_write_nothing() {
    let doc = Document::new(73);
    let page = doc.pages()[0].clone();
    let before = doc.version();
    assert!(doc.delete_page(&page.id).is_err());
    assert!(doc.rename_page("missing", "name").is_err());
    assert!(!doc.rename_page(&page.id, &page.name).unwrap());
    assert!(!doc.move_page(&page.id, Vec2::new(0.0, 0.0), &[]).unwrap());
    assert!(
        doc.resize_page(&page.id, DocumentSize::from_mm(-1.0, 20.0), &[])
            .is_err()
    );
    assert!(doc.reorder_page(&page.id, 1).is_err());
    assert_eq!(before, doc.version());
}

#[test]
fn ninth_page_wraps_below_the_row() {
    let doc = Document::new(74);
    for _ in 1..9 {
        doc.add_page();
    }
    let pages = doc.pages();
    assert!((pages[8].origin.x - pages[0].origin.x).abs() < 1e-9);
    assert!(pages[8].origin.y > pages[7].origin.y + pages[7].size.height.as_mm());
}

fn merge(a: &Document, b: &Document) {
    let av = a.version();
    let bv = b.version();
    let au = a.export_updates(&bv).unwrap();
    let bu = b.export_updates(&av).unwrap();
    a.import_updates(&bu).unwrap();
    b.import_updates(&au).unwrap();
}
#[test]
fn independent_fields_and_concurrent_deletion_converge() {
    let a = Document::new(80);
    let second = a.add_page();
    let b = unpack(81, &pack(&a, "test").unwrap()).unwrap();
    a.rename_page(&second.id, "Board").unwrap();
    b.resize_page(&second.id, DocumentSize::from_mm(100.0, 80.0), &[])
        .unwrap();
    merge(&a, &b);
    assert_eq!(a.pages(), b.pages());
    assert_eq!(a.pages()[1].name, "Board");
    assert_eq!(a.pages()[1].size, DocumentSize::from_mm(100.0, 80.0));
    a.delete_page("first").unwrap();
    b.delete_page(&second.id).unwrap();
    merge(&a, &b);
    assert_eq!(a.pages(), b.pages());
    assert_eq!(a.pages().len(), 1);
    let fallback = a.pages()[0].clone();
    a.add_page();
    assert_eq!(a.pages()[0], fallback);
}
#[test]
fn equal_concurrent_ranks_and_exhausted_midpoints_can_be_reordered() {
    let a = Document::new(82);
    let b = unpack(83, &pack(&a, "test").unwrap()).unwrap();
    a.add_page();
    b.add_page();
    merge(&a, &b);
    let third = a.add_page();
    a.reorder_page(&third.id, 2).unwrap();
    assert_eq!(a.pages()[2].id, third.id);
    for _ in 0..100 {
        let last = a.pages().last().unwrap().id.clone();
        a.reorder_page(&last, 1).unwrap();
        assert_eq!(a.pages()[1].id, last);
    }
}
#[test]
fn deleting_first_then_resizing_uses_the_visible_survivor() {
    let doc = Document::new(84);
    let second = doc.add_page();
    doc.delete_page("first").unwrap();
    doc.resize(DocumentSize::from_mm(40.0, 50.0)).unwrap();
    assert_eq!(doc.pages()[0].id, second.id);
    assert_eq!(doc.pages()[0].size, doc.size());
    assert_eq!(doc.size(), DocumentSize::from_mm(40.0, 50.0));
}
#[test]
fn page_history_preview_keeps_its_own_frames() {
    let doc = Document::new(85);
    doc.add_page();
    let step = doc.own_steps().last().unwrap().step;
    doc.add_page();
    assert_eq!(doc.preview_document_at(step).unwrap().pages.len(), 2);
    assert_eq!(doc.pages().len(), 3);
}
#[test]
fn golden_multiple_pages_v26_roundtrips() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pages_v26.curvyo");
    let doc = Document::new(86);
    let second = doc.add_page();
    doc.rename_page(&second.id, "Coasters").unwrap();
    doc.resize_page(&second.id, DocumentSize::from_mm(100.0, 100.0), &[])
        .unwrap();
    if std::env::var_os("UPDATE_PAGE_FIXTURE").is_some() {
        std::fs::write(&path, pack(&doc, "test").unwrap()).unwrap();
    }
    let opened = unpack(87, &std::fs::read(path).unwrap()).unwrap();
    assert_eq!(opened.pages(), doc.pages());
}

#[test]
fn page_ids_are_unique_inside_a_batch() {
    let doc = Document::new(88);
    doc.batch("add_page", |d| {
        d.add_page();
        d.add_page();
    });
    assert_eq!(doc.pages().len(), 3);
    assert_eq!(doc.history_len(), 1);
}
