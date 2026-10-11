//! Undo and redo of the steps that move objects between parents (`specs/0023-groups`, the review
//! note of `specs/0020-undo-redo/adrs.md`: "a command that groups objects must add the restore of
//! the fractional index and the parent"). Every test compares `document.json` before and after,
//! which nests groups, so a wrong parent or a wrong place shows.

#![allow(
    clippy::many_single_char_names,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::cast_precision_loss
)]

use curvyo_document_core::{
    AnchorId, CopySource, Document, NewAnchor, NodeId, ObjectShape, Point, RectBounds, StepRef,
    Vec2, pack, unpack,
};

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds::from_corners(
        Point::new(x, 0.0),
        Point::new(x + 10.0, 10.0),
    ))
}

fn path(document: &Document, n: u64, x: f64) -> NodeId {
    document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, n), Point::new(x, 0.0)),
            NewAnchor::corner(AnchorId::new(1, n + 1), Point::new(x + 10.0, 5.0)),
        ],
        false,
    )
}

fn json(document: &Document) -> Vec<u8> {
    document.export_json().unwrap()
}

fn last_step(document: &Document) -> StepRef {
    document.own_steps().last().unwrap().step
}

fn replica(document: &Document, peer: u64) -> Document {
    unpack(peer, &pack(document, "test").unwrap()).unwrap()
}

fn sync(a: &Document, b: &Document) {
    let to_b = a.export_updates(&b.version()).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    b.import_updates(&to_b).unwrap();
    a.import_updates(&to_a).unwrap();
}

/// Undoes and redoes the newest step twice, checking the document each time.
fn undo_redo_round(document: &Document, before: &[u8], after: &[u8]) {
    let step = last_step(document);
    for _ in 0..2 {
        let undo = document.undo_step(step).unwrap();
        assert_eq!(undo.skipped(), 0, "{undo:?}");
        assert_eq!(json(document), before, "after undo");
        let redo = document.redo_step(step).unwrap();
        assert_eq!(redo.skipped(), 0, "{redo:?}");
        assert_eq!(json(document), after, "after redo");
    }
}

#[test]
fn undo_of_group_moves_the_members_back_in_their_places_and_removes_the_group() {
    let document = Document::new(1);
    let (a, b, c, d) = (
        rect(&document, 0.0),
        rect(&document, 20.0),
        rect(&document, 40.0),
        rect(&document, 60.0),
    );
    let before = json(&document);
    let group = document.group_objects(&[a, c]).unwrap();
    let after = json(&document);
    assert_eq!(document.children(None), vec![b, group, d]);
    undo_redo_round(&document, &before, &after);
    assert_eq!(document.children(None), vec![b, group, d]);
    document.undo_step(last_step(&document)).unwrap();
    assert_eq!(
        document.children(None),
        vec![a, b, c, d],
        "the order is as drawn"
    );
    assert!(!document.contains_node(group));
}

#[test]
fn undo_of_ungroup_brings_the_group_back_with_its_children_in_order() {
    let document = Document::new(1);
    let (a, b, c) = (
        rect(&document, 0.0),
        rect(&document, 20.0),
        rect(&document, 40.0),
    );
    let group = document.group_objects(&[a, b, c]).unwrap();
    let before = json(&document);
    document.ungroup(&[group]).unwrap();
    let after = json(&document);
    undo_redo_round(&document, &before, &after);
    document.undo_step(last_step(&document)).unwrap();
    assert!(document.is_group(group), "the same node id");
    assert_eq!(document.children(Some(group)), vec![a, b, c]);
}

#[test]
fn nested_groups_undo_and_redo_one_level_at_a_time() {
    let document = Document::new(1);
    let (a, b, c, d) = (
        rect(&document, 0.0),
        rect(&document, 20.0),
        rect(&document, 40.0),
        rect(&document, 60.0),
    );
    let inner = document.group_objects(&[b, c]).unwrap();
    let step_inner = last_step(&document);
    let outer = document.group_objects(&[a, inner, d]).unwrap();
    let all_grouped = json(&document);
    let step_outer = last_step(&document);

    document.undo_step(step_outer).unwrap();
    assert_eq!(document.children(None), vec![a, inner, d]);
    document.undo_step(step_inner).unwrap();
    assert_eq!(document.children(None), vec![a, b, c, d]);
    document.redo_step(step_inner).unwrap();
    document.redo_step(step_outer).unwrap();
    assert_eq!(json(&document), all_grouped);
    assert_eq!(document.children(Some(outer)), vec![a, inner, d]);
}

#[test]
fn undo_of_a_delete_that_emptied_a_nested_group_restores_the_groups_and_the_leaf() {
    let document = Document::new(1);
    let (a, b, c) = (
        rect(&document, 0.0),
        rect(&document, 20.0),
        rect(&document, 40.0),
    );
    let inner = document.group_objects(&[a, b]).unwrap();
    let outer = document.group_objects(&[inner, c]).unwrap();
    let before = json(&document);
    document.delete_objects(&[a, b]).unwrap();
    assert!(!document.contains_node(inner));
    let after = json(&document);
    undo_redo_round(&document, &before, &after);
    document.undo_step(last_step(&document)).unwrap();
    assert_eq!(document.children(Some(outer)), vec![inner, c]);
    assert_eq!(document.children(Some(inner)), vec![a, b]);
}

#[test]
fn undo_of_deleting_one_leaf_of_a_group_brings_it_back_into_the_group_at_its_place() {
    let document = Document::new(1);
    let ids: Vec<NodeId> = (0..4)
        .map(|n| rect(&document, f64::from(n) * 20.0))
        .collect();
    let group = document.group_objects(&ids).unwrap();
    let before = json(&document);
    document.delete_objects(&[ids[1]]).unwrap();
    let after = json(&document);
    undo_redo_round(&document, &before, &after);
    document.undo_step(last_step(&document)).unwrap();
    assert_eq!(document.children(Some(group)), ids);
}

#[test]
fn undo_of_deleting_a_whole_group_node_brings_it_back_with_its_children() {
    let document = Document::new(1);
    let (a, b, c) = (
        rect(&document, 0.0),
        rect(&document, 20.0),
        rect(&document, 40.0),
    );
    let group = document.group_objects(&[a, b]).unwrap();
    let before = json(&document);
    document.delete_objects(&[group]).unwrap();
    assert_eq!(document.children(None), vec![c]);
    let after = json(&document);
    undo_redo_round(&document, &before, &after);
}

#[test]
fn undo_of_a_duplicate_of_a_group_removes_the_copies_group_and_leaves() {
    let document = Document::new(1);
    let (a, b) = (path(&document, 1, 0.0), path(&document, 10, 40.0));
    let group = document.group_objects(&[a, b]).unwrap();
    let before = json(&document);
    document
        .duplicate_objects(
            &[CopySource {
                id: group,
                anchor_ids: (50..54).map(|n| AnchorId::new(2, n)).collect(),
            }],
            Vec2::new(5.0, 5.0),
        )
        .unwrap();
    assert_eq!(document.object_ids().len(), 4);
    let after = json(&document);
    undo_redo_round(&document, &before, &after);
}

#[test]
fn undo_of_a_replacement_inside_a_group_puts_the_operands_back_into_it() {
    let document = Document::new(1);
    let (a, b, c) = (
        path(&document, 1, 0.0),
        path(&document, 10, 40.0),
        rect(&document, 80.0),
    );
    let group = document.group_objects(&[a, b]).unwrap();
    let before = json(&document);
    let outline = vec![(
        vec![
            NewAnchor::corner(AnchorId::new(3, 1), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(3, 2), Point::new(5.0, 5.0)),
            NewAnchor::corner(AnchorId::new(3, 3), Point::new(0.0, 5.0)),
        ],
        true,
    )];
    document
        .replace_with_path(&[a, b], a, &outline, "boolean_union")
        .unwrap();
    let after = json(&document);
    undo_redo_round(&document, &before, &after);
    document.undo_step(last_step(&document)).unwrap();
    assert_eq!(document.children(Some(group)), vec![a, b]);
    assert_eq!(document.children(None), vec![group, c]);
}

#[test]
fn the_report_names_a_group_as_a_group() {
    let document = Document::new(1);
    let (a, b) = (rect(&document, 0.0), rect(&document, 20.0));
    document.group_objects(&[a, b]).unwrap();
    let report = document.undo_step(last_step(&document)).unwrap();
    assert!(report.objects.iter().any(|o| o.shape == ObjectShape::Group));
    assert!(
        report
            .objects
            .iter()
            .any(|o| o.shape == ObjectShape::Rectangle)
    );
}

#[test]
fn a_member_a_peer_deleted_after_the_group_stays_deleted_when_the_group_is_undone() {
    let a = Document::new(1);
    let (x, y) = (rect(&a, 0.0), rect(&a, 20.0));
    a.group_objects(&[x, y]).unwrap();
    let step = last_step(&a);
    let b = replica(&a, 2);
    b.delete_objects(&[y]).unwrap();
    sync(&a, &b);

    let report = a.undo_step(step).unwrap();
    assert!(
        report.skipped() >= 1,
        "the deleted member is not brought back"
    );
    assert_eq!(a.children(None), vec![x]);
    sync(&a, &b);
    assert_eq!(b.children(None), vec![x]);
    assert_eq!(json(&a), json(&b));
}

#[test]
fn a_group_a_peer_already_took_apart_is_left_alone_by_the_undo_of_grouping_it() {
    let a = Document::new(1);
    let (x, y, z) = (rect(&a, 0.0), rect(&a, 20.0), rect(&a, 40.0));
    let group = a.group_objects(&[x, y]).unwrap();
    let step = last_step(&a);
    let b = replica(&a, 2);
    assert_eq!(b.ungroup(&[group]).unwrap(), vec![x, y]);
    sync(&a, &b);

    // The group is gone and its members are at the root: there is nothing to move back, and the
    // undo must not move or delete anything.
    let before = json(&a);
    let report = a.undo_step(step).unwrap();
    assert!(!report.wrote, "{report:?}");
    assert!(report.skipped() >= 1);
    assert_eq!(a.children(None), vec![x, y, z]);
    assert_eq!(json(&a), before);
    sync(&a, &b);
    assert_eq!(json(&a), json(&b));
}

#[test]
fn a_member_a_peer_moved_into_another_group_stays_there_when_the_first_group_is_undone() {
    let a = Document::new(1);
    let (x, y, z, w) = (
        rect(&a, 0.0),
        rect(&a, 20.0),
        rect(&a, 40.0),
        rect(&a, 60.0),
    );
    let first = a.group_objects(&[x, y]).unwrap();
    let step = last_step(&a);
    let b = replica(&a, 2);
    // The peer takes y out of the first group by ungrouping and regrouping it with z and w.
    b.ungroup(&[first]).unwrap();
    let second = b.group_objects(&[y, z, w]).unwrap();
    sync(&a, &b);

    a.undo_step(step).unwrap();
    assert_eq!(
        a.children(None),
        vec![x, second],
        "y stays in the peer's group"
    );
    assert_eq!(a.children(Some(second)), vec![y, z, w]);
}
