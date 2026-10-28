//! Undo and redo of a step that only created objects (read at its end alone), and the step that
//! created one object and changed another (read in full): z-order, ids and restored fields.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{Document, NodeId, Point, RectBounds, StepRef, Vec2, pack, unpack};

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds::from_corners(
        Point::new(x, 0.0),
        Point::new(x + 10.0, 10.0),
    ))
}

fn last_step(document: &Document) -> StepRef {
    document.own_steps().last().unwrap().step
}

fn sync(a: &Document, b: &Document) {
    let to_b = a.export_updates(&b.version()).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    b.import_updates(&to_b).unwrap();
    a.import_updates(&to_a).unwrap();
}

#[test]
fn created_into_a_group_keeps_its_place_when_a_peer_adds_a_sibling_before_undo_and_redo() {
    let document = Document::new(1);
    let (a, b) = (rect(&document, 0.0), rect(&document, 20.0));
    let group = document.group_objects(&[a, b]).unwrap();
    document.set_draw_parent(Some(group));
    let created = rect(&document, 40.0);
    let step = last_step(&document);
    assert_eq!(document.children(Some(group)), vec![a, b, created]);

    let peer = unpack(2, &pack(&document, "test").unwrap()).unwrap();
    peer.set_draw_parent(Some(group));
    let sibling = rect(&peer, 60.0);
    sync(&document, &peer);
    assert_eq!(document.children(Some(group)), vec![a, b, created, sibling]);

    let undo = document.undo_step(step).unwrap();
    assert_eq!(undo.skipped(), 0, "{undo:?}");
    assert_eq!(document.children(Some(group)), vec![a, b, sibling]);
    assert!(!document.contains_node(created));

    let redo = document.redo_step(step).unwrap();
    assert_eq!(redo.skipped(), 0, "{redo:?}");
    assert!(document.contains_node(created), "the same node id");
    assert_eq!(document.children(Some(group)), vec![a, b, created, sibling]);
}

#[test]
fn a_step_that_creates_one_object_and_changes_another_undoes_the_change_too() {
    let document = Document::new(1);
    let existing = rect(&document, 0.0);
    let before = document.export_json().unwrap();
    let mut created = None;
    document.batch("create and move", |document| {
        created = Some(rect(document, 40.0));
        document
            .translate_objects(&[existing], Vec2::new(5.0, 0.0))
            .unwrap();
    });
    let created = created.unwrap();
    let after = document.export_json().unwrap();
    assert_ne!(before, after);
    let step = last_step(&document);

    let undo = document.undo_step(step).unwrap();
    assert_eq!(undo.skipped(), 0, "{undo:?}");
    assert!(!document.contains_node(created));
    assert_eq!(
        document.export_json().unwrap(),
        before,
        "the change is undone"
    );

    let redo = document.redo_step(step).unwrap();
    assert_eq!(redo.skipped(), 0, "{redo:?}");
    assert_eq!(document.export_json().unwrap(), after);
}
