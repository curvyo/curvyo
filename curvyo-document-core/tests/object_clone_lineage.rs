//! A clone keeps its history (`specs/0041-object-history` criteria 23 to 28, 33, 37): the inherited
//! rows are computed from the source's steps at the copy, never copied into the clone.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::cast_precision_loss
)]

use curvyo_document_core::{
    CopySource, Document, NodeId, ObjectRow, ObjectRowClass, ObjectStatus, Point, RectBounds, Vec2,
    pack, unpack,
};

fn bounds(x: f64, width: f64) -> RectBounds {
    RectBounds::from_corners(Point::new(x, 0.0), Point::new(x + width, 10.0))
}

fn duplicate(document: &Document, id: NodeId, offset: Vec2) -> NodeId {
    document
        .duplicate_objects(
            &[CopySource {
                id,
                anchor_ids: vec![],
            }],
            offset,
        )
        .unwrap()[0]
}

fn rows(document: &Document, id: NodeId) -> Vec<ObjectRow> {
    document.object_history_page(id, 0, 100)
}

fn labels(document: &Document, id: NodeId) -> Vec<String> {
    rows(document, id).into_iter().map(|r| r.label).collect()
}

/// A rectangle that was drawn and resized, and its clone.
fn drawn_resized_and_duplicated() -> (Document, NodeId, NodeId) {
    let document = Document::new(1);
    let original = document.create_rect(bounds(0.0, 10.0));
    document
        .set_rect_bounds(original, bounds(0.0, 30.0))
        .unwrap();
    let clone = duplicate(&document, original, Vec2::new(50.0, 0.0));
    (document, original, clone)
}

#[test]
fn a_clone_starts_with_its_duplicate_and_inherits_the_rows_of_its_original() {
    let (document, original, clone) = drawn_resized_and_duplicated();
    let list = rows(&document, clone);
    assert_eq!(
        list.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
        ["duplicate_objects", "set_rect_bounds", "create_rect"]
    );
    assert_eq!(
        list[0].class,
        ObjectRowClass::CloneStart { source: original }
    );
    assert_eq!(list[0].inherited_from, None);
    for inherited in &list[1..] {
        assert_eq!(inherited.inherited_from, Some(original));
        assert_eq!(inherited.class, ObjectRowClass::Step);
        assert_eq!(inherited.status, ObjectStatus::Applied);
    }
    // The same steps, so the same ids as in the original's own list.
    let source = rows(&document, original);
    assert_eq!(list[1].step, source[0].step);
    assert_eq!(list[1].shown_id, source[0].shown_id);
    assert_eq!(list[2].step, source[1].step);
    // Nothing of the clone is in the original's list.
    assert_eq!(
        labels(&document, original),
        ["set_rect_bounds", "create_rect"]
    );
}

#[test]
fn later_steps_of_either_object_never_appear_in_the_other() {
    let (document, original, clone) = drawn_resized_and_duplicated();
    document
        .translate_objects(&[original], Vec2::new(1.0, 0.0))
        .unwrap();
    document
        .translate_objects(&[clone], Vec2::new(0.0, 1.0))
        .unwrap();
    let original_rows = labels(&document, original);
    let clone_rows = labels(&document, clone);
    assert_eq!(
        original_rows,
        ["translate_objects", "set_rect_bounds", "create_rect"]
    );
    assert_eq!(
        clone_rows,
        [
            "translate_objects",
            "duplicate_objects",
            "set_rect_bounds",
            "create_rect"
        ]
    );
    let own_move = rows(&document, clone)[0].step;
    assert_ne!(own_move, rows(&document, original)[0].step);
}

#[test]
fn a_clone_of_a_clone_follows_the_lineage_through_both() {
    let (document, original, clone) = drawn_resized_and_duplicated();
    document
        .translate_objects(&[clone], Vec2::new(0.0, 1.0))
        .unwrap();
    let grandchild = duplicate(&document, clone, Vec2::new(0.0, 50.0));
    let list = rows(&document, grandchild);
    assert_eq!(
        list.iter().map(|r| r.label.as_str()).collect::<Vec<_>>(),
        [
            "duplicate_objects",
            "translate_objects",
            "duplicate_objects",
            "set_rect_bounds",
            "create_rect"
        ]
    );
    assert_eq!(list[0].class, ObjectRowClass::CloneStart { source: clone });
    assert_eq!(list[1].inherited_from, Some(clone));
    assert_eq!(
        list[2].class,
        ObjectRowClass::CloneStart { source: original }
    );
    assert_eq!(list[2].inherited_from, Some(clone));
    assert_eq!(list[3].inherited_from, Some(original));
}

#[test]
fn a_copy_never_takes_over_the_origin_of_its_source() {
    let (document, original, clone) = drawn_resized_and_duplicated();
    let grandchild = duplicate(&document, clone, Vec2::new(0.0, 50.0));
    let from_grandchild = rows(&document, grandchild);
    let from_clone = rows(&document, clone);
    // The grandchild points at its direct source: its start row names the clone.
    assert_eq!(
        from_grandchild[0].class,
        ObjectRowClass::CloneStart { source: clone }
    );
    assert_eq!(
        from_clone[0].class,
        ObjectRowClass::CloneStart { source: original }
    );
}

#[test]
fn the_lineage_survives_save_and_open() {
    let (document, original, clone) = drawn_resized_and_duplicated();
    let reopened = unpack(7, &pack(&document, "test").unwrap()).unwrap();
    assert_eq!(
        labels(&reopened, clone),
        ["duplicate_objects", "set_rect_bounds", "create_rect"]
    );
    assert_eq!(
        rows(&reopened, clone)[0].class,
        ObjectRowClass::CloneStart { source: original }
    );
}

#[test]
fn an_object_undo_on_a_clone_goes_back_into_the_inherited_rows_and_spares_the_original() {
    let (document, original, clone) = drawn_resized_and_duplicated();
    document
        .translate_objects(&[clone], Vec2::new(0.0, 4.0))
        .unwrap();
    let original_before = document.object(original).unwrap();

    // First the clone's own move.
    let done = document.undo_object_step(clone).unwrap().unwrap();
    assert_eq!(done.target.label, "translate_objects");
    assert_eq!(done.target.inherited_from, None);

    // Then the resize it inherited: the clone is as wide as before the resize, at its own place.
    let done = document.undo_object_step(clone).unwrap().unwrap();
    assert_eq!(done.target.label, "set_rect_bounds");
    assert_eq!(done.target.inherited_from, Some(original));
    assert!(done.report.wrote);
    let clone_now = serde_json::to_value(document.object(clone).unwrap()).unwrap();
    let width = |value: &serde_json::Value| {
        value["Primitive"]["shape"]["Rect"]["bounds"]["width"]
            .as_f64()
            .unwrap()
    };
    assert!(
        (width(&clone_now) - 10.0).abs() < 1e-9,
        "as wide as before the resize"
    );
    assert_eq!(
        document.object(original).unwrap(),
        original_before,
        "the original is untouched"
    );
    assert_eq!(
        labels(&document, original),
        ["set_rect_bounds", "create_rect"],
        "no step of the clone landed in the original's history"
    );

    // The rows say so: the inherited resize is undone by this object undo.
    let list = rows(&document, clone);
    let resize = list.iter().find(|r| r.label == "set_rect_bounds").unwrap();
    assert!(matches!(resize.status, ObjectStatus::UndoneBy(_)));

    // Redo brings it back.
    let redo = document.redo_object_step(clone).unwrap().unwrap();
    assert_eq!(redo.target.label, "set_rect_bounds");
    let clone_after = serde_json::to_value(document.object(clone).unwrap()).unwrap();
    assert!((width(&clone_after) - 30.0).abs() < 1e-9, "wide again");
}

#[test]
fn going_back_through_all_inherited_rows_removes_the_clone_and_redo_brings_it_back() {
    let (document, original, clone) = drawn_resized_and_duplicated();
    document.undo_object_step(clone).unwrap().unwrap(); // the inherited resize
    let done = document.undo_object_step(clone).unwrap().unwrap(); // the original's drawing
    assert_eq!(done.target.label, "create_rect");
    assert!(done.object_gone);
    assert!(document.object(clone).is_none());
    assert!(document.object(original).is_some());
    assert!(document.undo_object_step(clone).unwrap().is_none());
    let redo = document.redo_object_step(clone).unwrap().unwrap();
    assert_eq!(redo.target.label, "create_rect");
    assert!(document.object(clone).is_some(), "with its id");
}
