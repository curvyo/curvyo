//! The commands that act on a tree with groups (`specs/0023-groups` criteria 13, 14 and 19): a
//! deep duplicate with fresh ids, a delete that takes a whole group, the removal of a group that
//! lost its last child in the same commit, and results that stay inside their group.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::collections::HashSet;
use std::path::Path;

use curvyo_document_core::{
    AnchorId, CopySource, Document, Length, NewAnchor, NodeId, ObjectEditError, ObjectSnapshot,
    Point, RectBounds, Vec2,
};

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn path(document: &Document, n: u64, x: f64) -> NodeId {
    document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, n), Point::new(x, 0.0)),
            NewAnchor::corner(AnchorId::new(1, n + 1), Point::new(x + 10.0, 5.0)),
            NewAnchor::corner(AnchorId::new(1, n + 2), Point::new(x + 20.0, 0.0)),
        ],
        false,
    )
}

fn anchor_ids(document: &Document, id: NodeId) -> Vec<AnchorId> {
    match document.object(id).unwrap() {
        ObjectSnapshot::Path(path) => path.all_anchors().map(|a| a.id).collect(),
        ObjectSnapshot::Primitive(_) => Vec::new(),
    }
}

/// `group(path, group(rect, path))` after one rectangle, so there is something below it.
fn nested(document: &Document) -> (NodeId, NodeId, NodeId, NodeId, NodeId, NodeId) {
    let below = rect(document, -30.0);
    let p1 = path(document, 1, 0.0);
    let r = rect(document, 40.0);
    let p2 = path(document, 10, 60.0);
    let inner = document.group_objects(&[r, p2]).unwrap();
    let outer = document.group_objects(&[p1, inner]).unwrap();
    (below, outer, inner, p1, r, p2)
}

#[test]
fn a_duplicate_of_a_group_is_a_group_of_copies_with_new_ids_above_the_original() {
    let document = Document::new(1);
    let (below, outer, inner, p1, r, p2) = nested(&document);
    // One fresh id per anchor of all leaves in paint order: p1 has three, p2 has three.
    let fresh: Vec<AnchorId> = (100..106).map(|n| AnchorId::new(2, n)).collect();
    let copies = document
        .duplicate_objects(
            &[CopySource {
                id: outer,
                anchor_ids: fresh.clone(),
            }],
            Vec2::new(5.0, 7.0),
        )
        .unwrap();
    assert_eq!(copies.len(), 1);
    let copy = copies[0];
    assert!(document.is_group(copy));
    assert_eq!(document.children(None), vec![below, outer, copy]);
    let copy_children = document.children(Some(copy));
    assert_eq!(copy_children.len(), 2);
    assert!(
        document.is_group(copy_children[1]),
        "the inner group is copied as a group"
    );
    let leaves = document.leaves_of(copy);
    assert_eq!(leaves.len(), 3);
    let originals = [p1, r, p2];
    for (leaf, original) in leaves.iter().zip(originals) {
        assert!(![p1, r, p2, inner, outer].contains(leaf), "new node ids");
        assert_ne!(*leaf, original);
    }
    // New anchor ids: exactly the fresh ones, none of the originals.
    let mut copied_anchors = anchor_ids(&document, leaves[0]);
    copied_anchors.extend(anchor_ids(&document, leaves[2]));
    assert_eq!(copied_anchors, fresh);
    let originals_anchors: HashSet<_> = [p1, p2]
        .iter()
        .flat_map(|&p| anchor_ids(&document, p))
        .collect();
    assert!(
        copied_anchors
            .iter()
            .all(|a| !originals_anchors.contains(a))
    );
    // The copy is displaced, the original is not.
    let ObjectSnapshot::Path(moved) = document.object(leaves[0]).unwrap() else {
        panic!("a path");
    };
    let ObjectSnapshot::Path(still) = document.object(p1).unwrap() else {
        panic!("a path");
    };
    assert_eq!(moved.anchors[0].point, Point::new(5.0, 7.0));
    assert_eq!(still.anchors[0].point, Point::new(0.0, 0.0));
}

#[test]
fn a_duplicate_is_one_commit_and_refuses_the_wrong_number_of_anchor_ids() {
    let document = Document::new(1);
    let (_, outer, ..) = nested(&document);
    let before = document.export_json().unwrap();
    assert_eq!(
        document.duplicate_objects(
            &[CopySource {
                id: outer,
                anchor_ids: vec![AnchorId::new(2, 1)],
            }],
            Vec2::new(1.0, 1.0),
        ),
        Err(ObjectEditError::AnchorIds)
    );
    assert_eq!(
        document.export_json().unwrap(),
        before,
        "nothing was written"
    );
    assert_eq!(document.anchor_count(outer), 6);
    let fresh: Vec<AnchorId> = (100..106).map(|n| AnchorId::new(2, n)).collect();
    let commits = document.version();
    document
        .duplicate_objects(
            &[CopySource {
                id: outer,
                anchor_ids: fresh,
            }],
            Vec2::new(1.0, 1.0),
        )
        .unwrap();
    assert_ne!(document.version(), commits);
}

#[test]
fn deleting_a_group_deletes_its_descendants() {
    let document = Document::new(1);
    let (below, outer, ..) = nested(&document);
    document.delete_objects(&[outer]).unwrap();
    assert_eq!(document.children(None), vec![below]);
    assert_eq!(document.object_ids(), vec![below]);
}

#[test]
fn deleting_a_group_and_one_of_its_leaves_in_one_call_is_fine() {
    let document = Document::new(1);
    let (below, outer, _, p1, ..) = nested(&document);
    document.delete_objects(&[p1, outer]).unwrap();
    assert_eq!(document.children(None), vec![below]);
}

#[test]
fn deleting_every_leaf_of_a_nested_group_removes_the_emptied_groups() {
    let document = Document::new(1);
    let (below, outer, inner, p1, r, p2) = nested(&document);
    document.delete_objects(&[r, p2]).unwrap();
    assert!(!document.contains_node(inner));
    assert_eq!(document.children(Some(outer)), vec![p1]);
    document.delete_objects(&[p1]).unwrap();
    assert!(!document.contains_node(outer));
    assert_eq!(document.children(None), vec![below]);
}

#[test]
fn a_node_tool_delete_of_the_last_nodes_removes_the_emptied_group() {
    let document = Document::new(1);
    let a = path(&document, 1, 0.0);
    let b = rect(&document, 40.0);
    let group = document.group_objects(&[a, b]).unwrap();
    let ids = anchor_ids(&document, a);
    document.delete_anchors(a, &ids).unwrap();
    assert!(document.is_group(group), "the rectangle is still in it");
    document.delete_objects(&[b]).unwrap();
    assert!(!document.contains_node(group));
}

#[test]
fn the_kind_specific_commands_treat_a_group_id_as_no_such_object() {
    let document = Document::new(1);
    let (_, outer, ..) = nested(&document);
    assert_eq!(
        document.translate_objects(&[outer], Vec2::new(1.0, 1.0)),
        Err(ObjectEditError::NoSuchObject)
    );
    assert!(document.object(outer).is_none());
    assert!(document.path(outer).is_none());
    assert!(document.primitive(outer).is_none());
}

#[test]
fn a_replacement_stays_in_the_group_of_its_base_and_an_emptied_group_is_removed() {
    let document = Document::new(1);
    let a = path(&document, 1, 0.0);
    let b = path(&document, 10, 40.0);
    let c = rect(&document, 80.0);
    let group = document.group_objects(&[a, b]).unwrap();
    let outline = |n: u64| {
        vec![(
            vec![
                NewAnchor::corner(AnchorId::new(3, n), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(3, n + 1), Point::new(5.0, 5.0)),
                NewAnchor::corner(AnchorId::new(3, n + 2), Point::new(0.0, 5.0)),
            ],
            true,
        )]
    };
    let result = document
        .replace_with_path(&[a, b], a, &outline(1), "boolean_union")
        .unwrap();
    assert_eq!(document.children(Some(group)), vec![result]);
    assert_eq!(document.children(None), vec![group, c]);
    // Replacing the only child keeps the group, since the result takes its place.
    let again = document
        .replace_with_path(&[result], result, &outline(10), "boolean_union")
        .unwrap();
    assert_eq!(document.children(Some(group)), vec![again]);
}

#[test]
fn break_apart_pieces_take_the_place_of_the_compound_inside_its_group() {
    let document = Document::new(1);
    let a = path(&document, 1, 0.0);
    let b = rect(&document, 40.0);
    let group = document.group_objects(&[a, b]).unwrap();
    let region = |n: u64| {
        vec![vec![(
            vec![
                NewAnchor::corner(AnchorId::new(4, n), Point::new(0.0, 0.0)),
                NewAnchor::corner(AnchorId::new(4, n + 1), Point::new(5.0, 5.0)),
                NewAnchor::corner(AnchorId::new(4, n + 2), Point::new(0.0, 5.0)),
            ],
            true,
        )]]
    };
    let mut regions = region(1);
    regions.extend(region(10));
    let pieces = document.break_apart(&[(a, regions)]).unwrap();
    assert_eq!(pieces.len(), 2);
    assert_eq!(document.children(Some(group))[..2], pieces[..]);
    assert_eq!(document.children(Some(group)).len(), 3);
}

/// A command that deletes tree nodes must also remove the groups it emptied
/// (`adrs.md`, "empty groups are removed in the same commit"): every source file outside the
/// history engine that calls `tree.delete(` also calls `remove_emptied_groups`. A new command that
/// deletes a node in a new file fails here until it does.
#[test]
fn every_command_that_deletes_a_node_removes_emptied_groups() {
    fn scan(dir: &Path, found: &mut Vec<(String, bool)>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n != "history") {
                    scan(&path, found);
                }
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let code = text.split("#[cfg(test)]").next().unwrap_or_default();
            if code.contains("tree.delete(") && !path.ends_with("groups.rs") {
                found.push((
                    path.file_name().unwrap().to_string_lossy().into_owned(),
                    code.contains("remove_emptied_groups("),
                ));
            }
        }
    }
    let mut found = Vec::new();
    scan(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut found,
    );
    assert!(found.len() >= 5, "the scan found the commands: {found:?}");
    for (file, removes) in found {
        assert!(
            removes,
            "{file} deletes a node but does not remove emptied groups"
        );
    }
}
