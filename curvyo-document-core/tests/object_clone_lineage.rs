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
    use curvyo_document_core::{Length, StyleEdit};
    let document = Document::new(1);
    let original = document.create_rect(bounds(0.0, 10.0));
    document
        .set_rect_bounds(original, bounds(0.0, 30.0))
        .unwrap();
    document
        .edit_style(&[original], &StyleEdit::StrokeWidth(Length::from_mm(2.0)))
        .unwrap();
    let clone = duplicate(&document, original, Vec2::new(50.0, 0.0));
    document
        .translate_objects(&[clone], Vec2::new(0.0, 4.0))
        .unwrap();
    let original_before = document.object(original).unwrap();
    let stroke = |id: NodeId| {
        serde_json::to_value(document.object(id).unwrap()).unwrap()["Primitive"]["style"]["stroke"]
            ["width"]
            .as_f64()
            .unwrap()
    };
    assert!((stroke(clone) - 2.0).abs() < 1e-9);

    // First the clone's own move.
    let done = document.undo_object_step(clone).unwrap().unwrap();
    assert_eq!(done.target.label, "translate_objects");
    assert_eq!(done.target.inherited_from, None);
    assert_eq!(done.passed_over, 0);

    // Then the colour it inherited: the stroke is thin again, the clone stays where it is.
    let done = document.undo_object_step(clone).unwrap().unwrap();
    assert_eq!(done.target.label, "edit_style");
    assert_eq!(done.target.inherited_from, Some(original));
    assert!(done.report.wrote);
    assert!((stroke(clone) - 0.25).abs() < 1e-9);
    assert_eq!(
        document.object(original).unwrap(),
        original_before,
        "the original is untouched"
    );
    assert_eq!(
        labels(&document, original),
        ["edit_style", "set_rect_bounds", "create_rect"],
        "no step of the clone landed in the original's history"
    );
    let list = rows(&document, clone);
    let style = list.iter().find(|r| r.label == "edit_style").unwrap();
    assert!(matches!(style.status, ObjectStatus::UndoneBy(_)));

    // Redo brings it back.
    let redo = document.redo_object_step(clone).unwrap().unwrap();
    assert_eq!(redo.target.label, "edit_style");
    assert!((stroke(clone) - 2.0).abs() < 1e-9);
}

#[test]
fn an_object_undo_gets_the_size_back_but_never_removes_a_clone() {
    let (document, original, clone) = drawn_resized_and_duplicated();
    // The inherited resize: the copy is as wide as before it and keeps its own place.
    let done = document.undo_object_step(clone).unwrap().unwrap();
    assert!(done.report.wrote);
    assert_eq!(done.target.label, "set_rect_bounds");
    // The original's drawing is the start of the lineage, which only Ctrl+Z takes back.
    assert!(document.undo_object_step(clone).unwrap().is_none());
    assert!(document.object(clone).is_some());
    assert!(document.object(original).is_some());
    let redo = document.redo_object_step(clone).unwrap().unwrap();
    assert_eq!(redo.target.label, "set_rect_bounds");
}

#[test]
fn a_copy_of_a_moved_original_keeps_its_place() {
    let document = Document::new(1);
    let original = document.create_rect(bounds(0.0, 10.0));
    document
        .translate_objects(&[original], Vec2::new(5.0, 0.0))
        .unwrap();
    let clone = duplicate(&document, original, Vec2::new(50.0, 0.0));
    let at = document.object(clone).unwrap();
    // The inherited move wrote nothing: the copy's displacement is its own, and the drawing is
    // the start of the lineage.
    let done = document.undo_object_step(clone).unwrap().unwrap();
    assert!(!done.report.wrote);
    assert_eq!(document.object(clone).unwrap(), at);
    assert!(document.object(original).is_some());
}

/// A path that was drawn, recoloured and had a node moved, and its copy.
fn path_with_a_style_and_a_node_edit() -> (Document, NodeId, NodeId) {
    use curvyo_document_core::{AnchorId, Color, NewAnchor, StyleEdit};
    let document = Document::new(1);
    let anchors = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]
        .iter()
        .enumerate()
        .map(|(n, (x, y))| NewAnchor::corner(AnchorId::new(1, n as u64 + 1), Point::new(*x, *y)))
        .collect::<Vec<_>>();
    let path = document.create_path(&anchors, true);
    document
        .edit_style(
            &[path],
            &StyleEdit::StrokeColor(Color { r: 200, g: 0, b: 0 }),
        )
        .unwrap();
    document
        .move_anchors(&[(path, AnchorId::new(1, 2), Point::new(12.0, 1.0))])
        .unwrap();
    let copy = document
        .duplicate_objects(
            &[CopySource {
                id: path,
                anchor_ids: vec![
                    AnchorId::new(2, 1),
                    AnchorId::new(2, 2),
                    AnchorId::new(2, 3),
                ],
            }],
            Vec2::new(30.0, 0.0),
        )
        .unwrap()[0];
    (document, path, copy)
}

#[test]
fn a_path_copy_passes_over_the_node_edit_and_takes_back_the_colour() {
    let (document, path, copy) = path_with_a_style_and_a_node_edit();
    let done = document.undo_object_step(copy).unwrap().unwrap();
    assert_eq!(done.target.label, "edit_style");
    assert_eq!(
        done.passed_over, 1,
        "the node edit only moved the original's nodes"
    );
    assert!(done.report.wrote);
    assert_eq!(done.target.inherited_from, Some(path));
    // The next press finds nothing it can write, and the copy stays.
    let done = document.undo_object_step(copy).unwrap().unwrap();
    assert!(!done.report.wrote);
    assert!(document.object(copy).is_some());
}

#[test]
fn a_press_that_finds_only_steps_it_cannot_write_leaves_no_row() {
    let (document, path, _) = path_with_a_style_and_a_node_edit();
    // Wipe the original's story up to the node edit, then copy it again: the copy inherits the
    // floor and the steps after it, which here are only a node edit.
    document.wipe_object_history(path).unwrap();
    document
        .move_anchors(&[(
            path,
            curvyo_document_core::AnchorId::new(1, 3),
            Point::new(9.0, 11.0),
        )])
        .unwrap();
    let copy = document
        .duplicate_objects(
            &[CopySource {
                id: path,
                anchor_ids: vec![
                    curvyo_document_core::AnchorId::new(3, 1),
                    curvyo_document_core::AnchorId::new(3, 2),
                    curvyo_document_core::AnchorId::new(3, 3),
                ],
            }],
            Vec2::new(0.0, 30.0),
        )
        .unwrap()[0];
    let before = document.object_history_len(copy);
    let version = document.version();
    let done = document.undo_object_step(copy).unwrap().unwrap();
    assert!(!done.report.wrote);
    assert!(done.target.inherited_from.is_some());
    assert_eq!(document.version(), version, "nothing was written");
    assert_eq!(document.object_history_len(copy), before);
}
