//! The timeline of one object (`specs/0041-object-history` criteria 1, 3, 4, 19 to 22, 36): the
//! steps that touched it, derived from the log, with the same ids as the global history.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::cast_precision_loss
)]

use curvyo_document_core::{
    AnchorId, Author, Document, NewAnchor, NodeId, ObjectRowClass, ObjectStatus, Point, RectBounds,
    Role, StepKind, Vec2, pack, unpack,
};

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds::from_corners(
        Point::new(x, 0.0),
        Point::new(x + 10.0, 10.0),
    ))
}

fn square(first: u64, x: f64) -> (Vec<NewAnchor>, bool) {
    let corners = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
    (
        corners
            .iter()
            .enumerate()
            .map(|(n, (dx, dy))| {
                NewAnchor::corner(AnchorId::new(1, first + n as u64), Point::new(x + dx, *dy))
            })
            .collect(),
        true,
    )
}

fn labels(document: &Document, id: NodeId) -> Vec<String> {
    document
        .object_history_page(id, 0, 100)
        .into_iter()
        .map(|row| row.label)
        .collect()
}

#[test]
fn a_move_of_three_objects_is_a_row_in_each_timeline_and_once_in_the_document_list() {
    let document = Document::new(1);
    let ids: Vec<NodeId> = (0..3)
        .map(|n| rect(&document, f64::from(n) * 20.0))
        .collect();
    document
        .translate_objects(&ids, Vec2::new(5.0, 0.0))
        .unwrap();
    let global = document.history_page(0, document.history_len());
    assert_eq!(global.len(), 4, "three creations and one move");
    let move_row = &global[0];
    for id in &ids {
        let rows = document.object_history_page(*id, 0, 10);
        assert_eq!(rows.len(), 2, "its creation and the move");
        assert_eq!(rows[0].step, move_row.step);
        assert_eq!(
            rows[0].shown_id, move_row.shown_id,
            "the same id as in the document list"
        );
        assert_eq!(rows[0].others, 2);
        assert_eq!(rows[0].role, Role::Changed);
        assert_eq!(rows[0].kind, StepKind::Change);
        assert_eq!(rows[1].label, "create_rect");
        assert_eq!(rows[1].role, Role::Created);
        assert_eq!(rows[1].others, 0);
        assert_eq!(rows[0].author, Author::You);
        assert_eq!(rows[0].status, ObjectStatus::Applied);
    }
}

#[test]
fn another_objects_steps_are_not_in_the_timeline() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    let b = rect(&document, 20.0);
    document
        .translate_objects(&[b], Vec2::new(1.0, 0.0))
        .unwrap();
    assert_eq!(labels(&document, a), ["create_rect"]);
    assert_eq!(labels(&document, b), ["translate_objects", "create_rect"]);
    assert_eq!(document.object_history_len(b), 2);
}

#[test]
fn a_node_edit_is_a_change_of_its_path_and_no_node_has_a_timeline() {
    let document = Document::new(1);
    let path = document.create_path(&square(1, 0.0).0, true);
    document
        .move_anchors(&[(path, AnchorId::new(1, 2), Point::new(12.0, 1.0))])
        .unwrap();
    assert_eq!(labels(&document, path), ["move_anchors", "create_path"]);
}

#[test]
fn a_deleted_objects_timeline_ends_with_its_deletion() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    let b = rect(&document, 20.0);
    document.delete_objects(&[a, b]).unwrap();
    let rows = document.object_history_page(a, 0, 10);
    assert_eq!(rows[0].label, "delete_objects");
    assert_eq!(rows[0].role, Role::Deleted);
    assert_eq!(rows[0].others, 1);
    assert_eq!(rows[1].role, Role::Created);
}

#[test]
fn a_boolean_result_begins_with_its_operands_and_they_end_with_the_step() {
    let document = Document::new(1);
    let a = document.create_path(&square(1, 0.0).0, true);
    let b = document.create_path(&square(11, 5.0).0, true);
    let c = document.create_path(&square(21, 9.0).0, true);
    let result = document
        .replace_with_path(&[a, b, c], a, &[square(31, 0.0)], "boolean_union")
        .unwrap();
    let rows = document.object_history_page(result, 0, 10);
    assert_eq!(rows.len(), 1, "the result inherits nothing");
    assert_eq!(rows[0].label, "boolean_union");
    assert_eq!(rows[0].role, Role::Created);
    assert_eq!(rows[0].kind, StepKind::Replace);
    assert_eq!(rows[0].sources.len(), 3);
    assert!(rows[0].sources.contains(&a));
    assert_eq!(rows[0].class, ObjectRowClass::Step);
    for source in [a, b, c] {
        let rows = document.object_history_page(source, 0, 10);
        assert_eq!(rows[0].label, "boolean_union");
        assert_eq!(rows[0].role, Role::Deleted);
        assert!(rows[0].sources.is_empty());
    }
}

#[test]
fn the_index_follows_new_commits_and_merged_steps_of_another_peer() {
    let a = Document::new(1);
    let id = rect(&a, 0.0);
    assert_eq!(a.object_history_len(id), 1, "builds the index");
    a.translate_objects(&[id], Vec2::new(1.0, 0.0)).unwrap();
    assert_eq!(a.object_history_len(id), 2, "extended by the new commit");

    let b = unpack(2, &pack(&a, "test").unwrap()).unwrap();
    b.translate_objects(&[id], Vec2::new(0.0, 1.0)).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    a.import_updates(&to_a).unwrap();
    let rows = a.object_history_page(id, 0, 10);
    assert_eq!(rows.len(), 3, "extended by the import");
    assert_eq!(rows[0].author, Author::Earlier);
    assert_eq!(rows[1].author, Author::You);
}

#[test]
fn the_timeline_survives_save_and_open() {
    let document = Document::new(1);
    let id = rect(&document, 0.0);
    document
        .translate_objects(&[id], Vec2::new(1.0, 0.0))
        .unwrap();
    let reopened = unpack(7, &pack(&document, "test").unwrap()).unwrap();
    assert_eq!(labels(&reopened, id), ["translate_objects", "create_rect"]);
    let rows = reopened.object_history_page(id, 0, 10);
    assert_eq!(rows[0].author, Author::Earlier, "an earlier session");
}

#[test]
fn the_page_of_a_long_timeline_is_cut_where_asked() {
    let document = Document::new(1);
    let id = rect(&document, 0.0);
    for _ in 0..9 {
        document
            .translate_objects(&[id], Vec2::new(1.0, 0.0))
            .unwrap();
    }
    assert_eq!(document.object_history_len(id), 10);
    let page = document.object_history_page(id, 3, 4);
    assert_eq!(page.len(), 4);
    let all = document.object_history_page(id, 0, 10);
    assert_eq!(page, all[3..7]);
}
