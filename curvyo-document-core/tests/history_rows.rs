//! The rows of the global history (`specs/0020-undo-redo` criteria 32 to 38, 59, 60): what a step
//! is called in the log, who made it, when, what kind it is, what it touched and whether it is
//! undone, also after Save and Open, after a merge and for files written by earlier builds.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::collections::HashSet;

use curvyo_document_core::{
    Author, Document, HistoryRow, Length, NodeId, ObjectShape, Point, RectBounds, Role, Status,
    StepKind, StyleEdit, Timestamp, Vec2, pack, unpack,
};

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds::from_corners(
        Point::new(x, 0.0),
        Point::new(x + 10.0, 10.0),
    ))
}

fn all_rows(document: &Document) -> Vec<HistoryRow> {
    document.history_page(0, document.history_len())
}

fn reopened(document: &Document, peer: u64) -> Document {
    unpack(peer, &pack(document, "test").unwrap()).unwrap()
}

#[test]
fn a_new_document_has_no_rows_and_each_step_adds_one() {
    let document = Document::new(5);
    assert_eq!(document.history_len(), 0);
    document.set_step_time(Timestamp::from_secs(1_700_000_000));
    let a = rect(&document, 0.0);
    document
        .translate_objects(&[a], Vec2::new(1.0, 0.0))
        .unwrap();
    let rows = all_rows(&document);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].label, "translate_objects", "newest first");
    assert_eq!(rows[1].label, "create_rect");
    assert_eq!(rows[0].author, Author::You);
    assert_eq!(rows[0].time, Timestamp::from_secs(1_700_000_000));
    assert_eq!(rows[1].kind, StepKind::Create);
    assert_eq!(rows[0].kind, StepKind::Change);
    assert_eq!(rows[0].status, Status::Applied);
    assert!(rows[0].shown_id.len() >= 7);
    assert_ne!(rows[0].shown_id, rows[1].shown_id);
}

#[test]
fn the_touched_objects_are_listed_with_their_shape_and_role() {
    let document = Document::new(5);
    let a = rect(&document, 0.0);
    let b = rect(&document, 20.0);
    document.delete_objects(&[a, b]).unwrap();
    let rows = all_rows(&document);
    assert_eq!(rows[0].kind, StepKind::Delete);
    assert_eq!(
        rows[0].objects.len(),
        2,
        "the deleted objects stay in the row"
    );
    assert!(
        rows[0]
            .objects
            .iter()
            .all(|o| o.shape == ObjectShape::Rectangle)
    );
    assert!(rows[0].objects.iter().all(|o| o.role == Role::Deleted));
}

#[test]
fn undo_and_redo_are_statuses_of_the_original_step_and_no_rows() {
    let document = Document::new(5);
    let _ = rect(&document, 0.0);
    let step = document.own_steps()[0].step;
    document.undo_step(step).unwrap();
    let rows = all_rows(&document);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, Status::Undone);
    document.redo_step(step).unwrap();
    assert_eq!(all_rows(&document)[0].status, Status::Applied);
    // A new step after an undo leaves the undone one on a branch.
    document.undo_step(step).unwrap();
    let _ = rect(&document, 30.0);
    let rows = all_rows(&document);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].status, Status::OnBranch);
}

#[test]
fn rows_survive_save_and_open_with_the_same_ids_names_and_statuses() {
    let document = Document::new(5);
    document.set_step_time(Timestamp::from_secs(1_700_000_100));
    let a = rect(&document, 0.0);
    document
        .edit_style(&[a], &StyleEdit::StrokeWidth(Length::from_mm(2.0)))
        .unwrap();
    document.undo_step(document.own_steps()[1].step).unwrap();
    let before = all_rows(&document);

    let after = all_rows(&reopened(&document, 9));
    assert_eq!(before.len(), after.len());
    for (b, a) in before.iter().zip(&after) {
        assert_eq!(b.shown_id, a.shown_id);
        assert_eq!(b.label, a.label);
        assert_eq!(b.time, a.time);
        assert_eq!(b.kind, a.kind);
        assert_eq!(b.status, a.status);
        assert_eq!(b.step, a.step);
        assert_eq!(b.objects, a.objects);
    }
    assert!(after.iter().all(|row| row.author == Author::Earlier));
}

#[test]
fn two_replicas_that_merge_show_the_same_rows() {
    let a = Document::new(1);
    let _ = rect(&a, 0.0);
    let b = reopened(&a, 2);
    let _ = rect(&b, 50.0);
    let _ = rect(&a, 100.0);
    let to_b = a.export_updates(&b.version()).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    b.import_updates(&to_b).unwrap();
    a.import_updates(&to_a).unwrap();
    let key = |rows: Vec<HistoryRow>| -> Vec<_> {
        rows.into_iter()
            .map(|r| (r.step, r.shown_id, r.label, r.time, r.kind, r.objects))
            .collect()
    };
    assert_eq!(key(all_rows(&a)), key(all_rows(&b)));
    assert_eq!(all_rows(&a).len(), 3);
}

#[test]
fn twenty_thousand_steps_have_no_duplicate_shown_id() {
    let document = Document::new(5);
    let a = rect(&document, 0.0);
    for n in 0..20_000_i32 {
        document
            .translate_objects(&[a], Vec2::new(f64::from(n % 3 + 1), 0.0))
            .unwrap();
    }
    let rows = document.history_page(0, 20_001);
    assert_eq!(rows.len(), 20_001);
    let ids: HashSet<_> = rows.iter().map(|r| r.shown_id.clone()).collect();
    assert_eq!(ids.len(), rows.len());
}

#[test]
fn a_file_from_an_earlier_build_lists_its_steps_with_unknown_time_and_earlier_author() {
    let bytes = std::fs::read("tests/fixtures/compound_v8.curvyo").unwrap();
    let document = unpack(99, &bytes).unwrap();
    let rows = all_rows(&document);
    assert_ne!(rows.len(), 0);
    assert!(rows.iter().all(|r| r.time == Timestamp::UNKNOWN));
    assert!(rows.iter().all(|r| r.author == Author::Earlier));
    assert!(rows.iter().all(|r| !r.label.is_empty()));
    assert!(rows.iter().all(|r| r.shown_id.len() >= 7));
}

#[test]
fn history_adds_no_format_version_of_its_own() {
    let document = Document::new(5);
    let _ = rect(&document, 0.0);
    let json: serde_json::Value = serde_json::from_slice(&document.export_json().unwrap()).unwrap();
    assert_eq!(
        json["format_version"],
        curvyo_document_core::CURRENT_FORMAT_VERSION
    );
}

#[test]
fn each_peers_own_time_survives_a_merge_whatever_loro_does_with_timestamps() {
    let a = Document::new(1);
    a.set_step_time(Timestamp::from_secs(2_000_000_000));
    let _ = rect(&a, 0.0);
    let b = reopened(&a, 2);
    // B's clock runs behind A's.
    b.set_step_time(Timestamp::from_secs(1_500_000_000));
    let _ = rect(&b, 50.0);
    let to_a = b.export_updates(&a.version()).unwrap();
    a.import_updates(&to_a).unwrap();
    let rows = all_rows(&a);
    let times: Vec<_> = rows.iter().map(|r| r.time.as_secs()).collect();
    assert!(
        times.contains(&2_000_000_000) && times.contains(&1_500_000_000),
        "{times:?}"
    );
}

#[test]
fn a_step_that_touched_an_object_keeps_it_in_its_row_after_the_object_was_deleted() {
    use curvyo_document_core::{AnchorId, NewAnchor};
    let document = Document::new(5);
    let path = document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(10.0, 0.0)),
        ],
        false,
    );
    document
        .move_anchors(&[(path, AnchorId::new(1, 2), Point::new(20.0, 5.0))])
        .unwrap();
    document.delete_objects(&[path]).unwrap();
    let reopened = reopened(&document, 9);
    let rows = all_rows(&reopened);
    let moved = rows.iter().find(|r| r.label == "move_anchors").unwrap();
    assert_eq!(moved.kind, StepKind::Change);
    assert_eq!(
        moved.objects.len(),
        1,
        "the nested anchor container resolves to the path"
    );
    assert_eq!(moved.objects[0].shape, ObjectShape::Path);
}
