//! Wiping the document's history (`specs/0020-undo-redo` criteria 47 to 51, ADR 0014 §9): the
//! drawing stays, the log is cut, one step records the wipe, and a replica that still holds the
//! older steps cannot merge.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use curvyo_document_core::{
    Author, Document, HistoryError, NodeId, Point, RectBounds, StepKind, SyncError, Vec2, pack,
    unpack,
};

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds::from_corners(
        Point::new(x, 0.0),
        Point::new(x + 10.0, 10.0),
    ))
}

fn busy_document(steps: usize) -> (Document, Vec<NodeId>) {
    let document = Document::new(5);
    let ids: Vec<NodeId> = (0..10)
        .map(|n| rect(&document, f64::from(n) * 20.0))
        .collect();
    for n in 0..steps.saturating_sub(10) {
        document
            .translate_objects(&[ids[n % 10]], Vec2::new(1.0 + (n % 3) as f64, 0.0))
            .unwrap();
    }
    (document, ids)
}

#[test]
fn a_wipe_keeps_the_drawing_and_leaves_one_row() {
    let (mut document, ids) = busy_document(200);
    let before = document.export_json().unwrap();
    assert_eq!(document.history_len(), 200);
    let removed = document.wipe_history().unwrap();
    assert_eq!(removed, 200);
    assert_eq!(
        document.export_json().unwrap(),
        before,
        "the drawing is byte-equal"
    );
    assert_eq!(document.object_ids(), ids);

    let rows = document.history_page(0, 10);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].label, "wipe_history");
    assert_eq!(rows[0].kind, StepKind::Document);
    assert_eq!(rows[0].author, Author::You);
    assert!(document.history_epoch().is_some());
    assert_eq!(document.own_step_count(), 0, "the wipe is on no stack");

    // Steps made afterwards follow it, by the same author.
    document
        .translate_objects(&[ids[0]], Vec2::new(3.0, 0.0))
        .unwrap();
    let rows = document.history_page(0, 10);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].author, Author::You);
    let step = document.own_steps()[0].step;
    document.undo_step(step).unwrap();
    assert_eq!(document.export_json().unwrap(), before);
}

#[test]
fn the_wiped_snapshot_opens_with_one_row_and_the_same_objects() {
    let (mut document, ids) = busy_document(200);
    let before = document.export_json().unwrap();
    document.wipe_history().unwrap();
    let reopened = unpack(9, &pack(&document, "test").unwrap()).unwrap();
    assert_eq!(reopened.history_len(), 1);
    assert_eq!(reopened.object_ids(), ids);
    assert_eq!(reopened.export_json().unwrap(), before);
    assert_eq!(reopened.history_epoch(), document.history_epoch());
}

#[test]
fn a_wiped_file_is_smaller_than_the_file_with_its_history() {
    let (mut document, _) = busy_document(500);
    let with_history = pack(&document, "test").unwrap().len();
    document.wipe_history().unwrap();
    let without = pack(&document, "test").unwrap().len();
    eprintln!("500 steps: {with_history} bytes, wiped: {without} bytes");
    assert!(without < with_history);
}

#[test]
fn a_replica_with_older_steps_is_refused_and_nothing_is_merged() {
    let (mut a, ids) = busy_document(50);
    let b = unpack(2, &pack(&a, "test").unwrap()).unwrap();
    b.translate_objects(&[ids[1]], Vec2::new(40.0, 0.0))
        .unwrap();
    a.wipe_history().unwrap();
    let before = a.export_json().unwrap();
    let version = a.version();

    let from_b = b.export_updates(&a.version()).unwrap();
    assert_eq!(a.import_updates(&from_b), Err(SyncError::EpochMismatch));
    assert_eq!(a.export_json().unwrap(), before, "A is unchanged");
    assert_eq!(a.version(), version);
    // The other direction is refused too.
    let from_a = a.export_updates(&b.version()).unwrap();
    assert_eq!(b.import_updates(&from_a), Err(SyncError::EpochMismatch));
}

#[test]
fn replicas_of_one_epoch_merge_as_before() {
    let (mut a, ids) = busy_document(50);
    a.wipe_history().unwrap();
    let b = unpack(2, &pack(&a, "test").unwrap()).unwrap();
    assert_eq!(a.history_epoch(), b.history_epoch());
    b.translate_objects(&[ids[1]], Vec2::new(40.0, 0.0))
        .unwrap();
    a.translate_objects(&[ids[2]], Vec2::new(5.0, 0.0)).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    let to_b = a.export_updates(&b.version()).unwrap();
    a.import_updates(&to_a).unwrap();
    b.import_updates(&to_b).unwrap();
    assert_eq!(a.export_json().unwrap(), b.export_json().unwrap());
}

#[test]
fn a_shared_document_is_not_wiped() {
    let (mut document, _) = busy_document(20);
    document.mark_shared();
    assert_eq!(document.wipe_history(), Err(HistoryError::Shared));
    assert_eq!(document.history_len(), 20);
}

#[test]
fn a_document_written_with_the_optional_key_loads_in_a_reader_that_ignores_it() {
    // History_wiped is one more root key; the drawing reads the same without knowing it.
    let (mut document, ids) = busy_document(20);
    document.wipe_history().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&document.export_json().unwrap()).unwrap();
    assert_eq!(json["format_version"], 9);
    assert_eq!(json["objects"].as_array().unwrap().len(), ids.len());
}
