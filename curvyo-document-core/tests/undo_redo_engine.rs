//! Undo and redo as the document engine does them (`specs/0020-undo-redo` criteria 10, 11, 15 to
//! 23, 56, 57), on two replicas that merge in the test.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use curvyo_document_core::{
    Document, Length, NodeId, Point, RectBounds, StyleEdit, Vec2, pack, unpack,
};

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds::from_corners(
        Point::new(x, 0.0),
        Point::new(x + 10.0, 10.0),
    ))
}

fn json(document: &Document) -> Vec<u8> {
    document.export_json().unwrap()
}

/// A second replica of `document` with its own peer.
fn replica(document: &Document, peer: u64) -> Document {
    unpack(peer, &pack(document, "test").unwrap()).unwrap()
}

fn sync(a: &Document, b: &Document) {
    let to_b = a.export_updates(&b.version()).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    b.import_updates(&to_b).unwrap();
    a.import_updates(&to_a).unwrap();
}

#[test]
fn the_first_step_does_not_undo_the_documents_setup() {
    let document = Document::new(1);
    let empty = json(&document);
    let _ = rect(&document, 0.0);
    let after = json(&document);

    let step = document.own_steps()[0].step;
    let report = document.undo_step(step).unwrap();
    assert!(report.wrote);
    assert_eq!(json(&document), empty, "size, unit and version are intact");

    document.redo_step(step).unwrap();
    assert_eq!(json(&document), after);
}

#[test]
fn one_press_takes_back_all_objects_of_a_step_and_redo_brings_them_back() {
    let document = Document::new(1);
    let ids: Vec<NodeId> = (0..3)
        .map(|n| rect(&document, f64::from(n) * 20.0))
        .collect();
    let before = json(&document);
    document
        .translate_objects(&ids, Vec2::new(5.0, 7.0))
        .unwrap();
    let after = json(&document);
    let step = document.own_steps().last().unwrap().step;

    let report = document.undo_step(step).unwrap();
    assert_eq!(report.total(), 3);
    assert_eq!(report.skipped(), 0);
    assert_eq!(json(&document), before);
    document.redo_step(step).unwrap();
    assert_eq!(json(&document), after);
}

#[test]
fn undo_of_delete_brings_every_object_back_with_the_same_id_and_z_order() {
    let document = Document::new(1);
    let ids: Vec<NodeId> = (0..5)
        .map(|n| rect(&document, f64::from(n) * 20.0))
        .collect();
    let before = json(&document);
    document.delete_objects(&[ids[1], ids[3]]).unwrap();
    assert_eq!(document.object_ids().len(), 3);
    let step = document.own_steps().last().unwrap().step;

    document.undo_step(step).unwrap();
    assert_eq!(json(&document), before);
    assert_eq!(document.object_ids(), ids);

    // Save and open: the revived objects are ordinary objects.
    let reopened = replica(&document, 9);
    assert_eq!(json(&reopened), before);

    document.redo_step(step).unwrap();
    assert_eq!(document.object_ids(), [ids[0], ids[2], ids[4]]);
}

#[test]
fn a_field_a_peer_changed_since_keeps_the_peers_value() {
    let a = Document::new(1);
    let id = rect(&a, 0.0);
    let b = replica(&a, 2);

    a.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(2.0)))
        .unwrap();
    let step = a.own_steps().last().unwrap().step;
    sync(&a, &b);
    b.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(5.0)))
        .unwrap();
    sync(&a, &b);

    let report = a.undo_step(step).unwrap();
    assert_eq!(report.skipped(), 1, "the object kept a later change");
    assert!(!report.wrote, "everything was skipped, nothing was written");
    sync(&a, &b);
    let width = |d: &Document| d.object(id).unwrap().style().stroke.width.as_mm();
    assert!((width(&a) - 5.0).abs() < 1e-9);
    assert!((width(&b) - 5.0).abs() < 1e-9);
}

#[test]
fn fields_the_peer_did_not_touch_go_back() {
    let a = Document::new(1);
    let id = rect(&a, 0.0);
    let b = replica(&a, 2);

    // One step of A writes the stroke width and the fill; B changes only the width afterwards.
    a.batch("two_edits", |d| {
        d.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(2.0)))
            .unwrap();
        d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    });
    let step = a.own_steps().last().unwrap().step;
    sync(&a, &b);
    b.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(5.0)))
        .unwrap();
    sync(&a, &b);

    let report = a.undo_step(step).unwrap();
    assert_eq!(
        report.objects[0].result,
        curvyo_document_core::ObjectResult::Partial
    );
    sync(&a, &b);
    let object = a.object(id).unwrap();
    assert!((object.style().stroke.width.as_mm() - 5.0).abs() < 1e-9);
    assert!(!object.style().fill.enabled, "the fill went back");
    assert_eq!(json(&a), json(&b), "criterion 20: the undo replicates");
}

#[test]
fn an_object_a_peer_deleted_is_not_brought_back_by_an_undo() {
    let a = Document::new(1);
    let id = rect(&a, 0.0);
    let b = replica(&a, 2);
    a.translate_objects(&[id], Vec2::new(5.0, 0.0)).unwrap();
    let step = a.own_steps().last().unwrap().step;
    sync(&a, &b);
    b.delete_objects(&[id]).unwrap();
    sync(&a, &b);

    let report = a.undo_step(step).unwrap();
    assert_eq!(report.skipped(), 1);
    assert_eq!(a.object_ids().len(), 0);
}

#[test]
fn an_object_created_by_the_step_stays_when_a_peer_changed_it() {
    let a = Document::new(1);
    let id = rect(&a, 0.0);
    let step = a.own_steps()[0].step;
    let b = replica(&a, 2);
    b.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(3.0)))
        .unwrap();
    sync(&a, &b);

    let report = a.undo_step(step).unwrap();
    assert!(report.created_object_kept());
    assert_eq!(a.object_ids(), [id]);
}

#[test]
fn a_peers_own_steps_do_not_enter_my_stack_and_my_undo_leaves_theirs_alone() {
    let a = Document::new(1);
    let first = rect(&a, 0.0);
    let b = replica(&a, 2);
    let theirs = rect(&b, 50.0);
    sync(&a, &b);
    assert_eq!(a.own_step_count(), 1, "only my own steps are listed");
    a.undo_step(a.own_steps()[0].step).unwrap();
    sync(&a, &b);
    assert_eq!(a.object_ids(), [theirs]);
    assert_eq!(json(&a), json(&b));
    assert!(!a.object_ids().contains(&first));
}

#[test]
fn undo_and_redo_of_document_steps_restore_the_size() {
    let document = Document::new(1);
    let before = json(&document);
    document
        .resize(curvyo_document_core::DocumentSize::new(
            Length::from_mm(100.0),
            Length::from_mm(50.0),
        ))
        .unwrap();
    let after = json(&document);
    let step = document.own_steps()[0].step;
    document.undo_step(step).unwrap();
    assert_eq!(json(&document), before);
    document.redo_step(step).unwrap();
    assert_eq!(json(&document), after);
}

#[test]
fn an_unknown_step_is_refused() {
    let document = Document::new(1);
    let result = document.undo_step(curvyo_document_core::StepRef::new(99, 5));
    assert_eq!(result, Err(curvyo_document_core::HistoryError::NoSuchStep));
}

#[test]
fn a_large_result_split_into_several_changes_is_one_step() {
    use curvyo_document_core::{AnchorId, NewAnchor};
    let document = Document::new(1);
    let before = json(&document);
    let anchors: Vec<NewAnchor> = (0..5_000_u64)
        .map(|n| {
            NewAnchor::corner(
                AnchorId::new(1, n),
                Point::new((n % 100) as f64, (n / 100) as f64),
            )
        })
        .collect();
    let _ = document.create_path(&anchors, true);
    // After Save and Open, Loro keeps the commit as several changes with one message.
    let reopened = replica(&document, 9);
    let rows = reopened.history_page(0, 10);
    assert_eq!(rows.len(), 1, "one commit is one row");
    let step = rows[0].step;
    let report = reopened.undo_step(step).unwrap();
    assert!(report.wrote);
    assert_eq!(
        json(&reopened),
        before,
        "one press takes the whole result back"
    );
    assert_eq!(reopened.history_page(0, 10).len(), 1);
}

#[test]
fn chained_undo_and_redo_on_one_compound_path_keep_a_peers_edit_of_another_node() {
    use curvyo_document_core::{AnchorId, NewAnchor};
    let square = |first: u64, x: f64| -> (Vec<NewAnchor>, bool) {
        (
            (0..4_u64)
                .map(|n| {
                    let (dx, dy) = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)][n as usize];
                    NewAnchor::corner(AnchorId::new(1, first + n), Point::new(x + dx, dy))
                })
                .collect(),
            true,
        )
    };
    let a = Document::new(1);
    let first = a.create_path(&square(1, 0.0).0, true);
    let second = a.create_path(&square(11, 100.0).0, true);
    let compound = a
        .replace_with_path(
            &[first, second],
            first,
            &[square(21, 0.0), square(31, 100.0)],
            "combine_paths",
        )
        .unwrap();
    let b = replica(&a, 2);
    let original = json(&a);

    // A: move the whole compound, then one node of its first outline.
    a.translate_objects(&[compound], Vec2::new(5.0, 0.0))
        .unwrap();
    let step_move = a.own_steps().last().unwrap().step;
    sync(&a, &b);
    a.move_anchors(&[(compound, AnchorId::new(1, 21), Point::new(50.0, 50.0))])
        .unwrap();
    let step_node = a.own_steps().last().unwrap().step;
    // B, in between: moves another node of the same outline.
    b.move_anchors(&[(compound, AnchorId::new(1, 22), Point::new(77.0, 77.0))])
        .unwrap();
    sync(&a, &b);

    let point_of = |d: &Document, anchor: u64| {
        let curvyo_document_core::ObjectSnapshot::Path(path) = d.object(compound).unwrap() else {
            panic!("a path")
        };
        let p = path
            .all_anchors()
            .find(|n| n.id == AnchorId::new(1, anchor))
            .unwrap()
            .point;
        (p.x, p.y)
    };
    a.undo_step(step_node).unwrap();
    a.undo_step(step_move).unwrap();
    sync(&a, &b);
    assert_eq!(point_of(&a, 21), (0.0, 0.0), "my nodes are back");
    assert_eq!(
        point_of(&a, 22),
        (77.0, 77.0),
        "the peer's node keeps the peer's value"
    );
    assert_eq!(json(&a), json(&b));
    assert_ne!(json(&a), original);

    a.redo_step(step_move).unwrap();
    a.redo_step(step_node).unwrap();
    sync(&a, &b);
    assert_eq!(point_of(&a, 21), (50.0, 50.0));
    assert_eq!(point_of(&a, 22), (77.0, 77.0));
    assert_eq!(json(&a), json(&b));
}
