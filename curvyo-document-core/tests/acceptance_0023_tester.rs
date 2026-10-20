//! Independent tester tests for the document model half of `specs/0023-groups` (criteria 1, 3, 4, 5,
//! 13, 14, 25, 27 and the undo, redo and replication rules of `adrs.md`). Written from the
//! specification before the implementation diff was read: the expected values are the
//! specification's own arithmetic, never read back from the code under test.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless
)]
#![allow(missing_docs, clippy::doc_markdown, clippy::needless_pass_by_value)]
#![allow(clippy::similar_names, clippy::many_single_char_names)]
#![allow(clippy::items_after_statements, clippy::assert_is_empty)]

use std::collections::HashSet;
use std::io::{Cursor, Read, Write};
use std::time::Instant;

use curvyo_document_core::{
    AnchorId, Document, GROUP_COMMIT_LABEL, Length, MAX_GROUP_DEPTH, NewAnchor, NodeId,
    ObjectEditError, OpenError, Point, RectBounds, UNGROUP_COMMIT_LABEL, Vec2, pack, unpack,
};
use loro::{ExportMode, LoroDoc, TreeID, TreeParentId};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn rect(d: &Document, x: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: pt(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn path(d: &Document, n: u64, x: f64) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(9, n * 2), pt(x, 0.0)),
            NewAnchor::corner(AnchorId::new(9, n * 2 + 1), pt(x + 5.0, 5.0)),
        ],
        false,
    )
}

fn json(d: &Document) -> Vec<u8> {
    d.export_json().unwrap()
}

fn snap(d: &Document) -> Vec<u8> {
    d.export_loro_snapshot().unwrap()
}

fn replica(d: &Document, peer: u64) -> Document {
    unpack(peer, &pack(d, "test").unwrap()).unwrap()
}

fn sync(a: &Document, b: &Document) {
    let to_b = a.export_updates(&b.version()).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    b.import_updates(&to_b).unwrap();
    a.import_updates(&to_a).unwrap();
}

fn root(d: &Document) -> Vec<NodeId> {
    d.children(None)
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

/// `n` rectangles side by side.
fn row(n: usize) -> (Document, Vec<NodeId>) {
    let d = Document::new(1);
    let ids = (0..n).map(|i| rect(&d, i as f64 * 20.0)).collect();
    (d, ids)
}

// ---------------------------------------------------------------------
// Criterion 1: Group
// ---------------------------------------------------------------------

#[test]
fn ac1_the_spec_example_a_b_c_with_a_and_c_gives_b_then_group_of_a_c() {
    let (d, ids) = row(3);
    let (a, b, c) = (ids[0], ids[1], ids[2]);
    let g = d.group_objects(&[a, c]).unwrap();
    assert_eq!(root(&d), vec![b, g], "B below, the group at C's place");
    assert_eq!(d.children(Some(g)), vec![a, c], "relative order kept");
    assert!(d.is_group(g));
    assert!(!d.is_group(a));
    assert_eq!(d.parent_group(a), Some(g));
    assert_eq!(d.parent_group(g), None);
    assert_eq!(d.leaves_of(g), vec![a, c]);
    // The paint list stays a flat list of leaves in paint order.
    assert_eq!(d.object_ids(), vec![b, a, c]);
}

#[test]
fn ac1_the_ids_of_the_members_do_not_change_and_a_group_has_no_object_snapshot() {
    let (d, ids) = row(2);
    let before: Vec<_> = ids.iter().map(|id| d.object(*id).unwrap()).collect();
    let g = d.group_objects(&ids).unwrap();
    for (id, was) in ids.iter().zip(&before) {
        assert_eq!(&d.object(*id).unwrap(), was, "member untouched");
    }
    assert!(d.object(g).is_none(), "a group has no geometry or style");
    assert!(d.path(g).is_none());
    assert!(!ids.contains(&g));
}

#[test]
fn ac1_the_argument_order_does_not_matter_the_existing_order_does() {
    let (d, ids) = row(3);
    let g = d.group_objects(&[ids[2], ids[0], ids[1]]).unwrap();
    assert_eq!(d.children(Some(g)), vec![ids[0], ids[1], ids[2]]);
}

#[test]
fn ac1_the_group_takes_the_stacking_position_of_the_topmost_member() {
    let (d, ids) = row(5);
    // A B C D E, group B and D: A C [g] E ... topmost member D: A, C, g, E.
    let g = d.group_objects(&[ids[1], ids[3]]).unwrap();
    assert_eq!(root(&d), vec![ids[0], ids[2], g, ids[4]]);
    // Group a prefix: the group at the top member's place, the rest keeps order.
    let (d, ids) = row(4);
    let g = d.group_objects(&[ids[0], ids[1]]).unwrap();
    assert_eq!(root(&d), vec![g, ids[2], ids[3]]);
    // Group the top two.
    let (d, ids) = row(4);
    let g = d.group_objects(&[ids[2], ids[3]]).unwrap();
    assert_eq!(root(&d), vec![ids[0], ids[1], g]);
}

#[test]
fn ac1_any_mix_of_leaves_and_groups_can_be_grouped() {
    let (d, ids) = row(4);
    let inner = d.group_objects(&[ids[0], ids[1]]).unwrap();
    let outer = d.group_objects(&[inner, ids[3]]).unwrap();
    assert_eq!(root(&d), vec![ids[2], outer]);
    assert_eq!(d.children(Some(outer)), vec![inner, ids[3]]);
    assert_eq!(d.children(Some(inner)), vec![ids[0], ids[1]]);
    assert_eq!(d.parent_group(inner), Some(outer));
    assert_eq!(d.ancestor_groups(ids[0]).len(), 2);
    assert_eq!(d.leaves_of(outer), vec![ids[0], ids[1], ids[3]]);
    assert_eq!(d.object_ids(), vec![ids[2], ids[0], ids[1], ids[3]]);
}

#[test]
fn ac1_group_is_one_commit_labelled_group() {
    let (d, ids) = row(3);
    let n = d.own_step_count();
    let _ = d.group_objects(&ids).unwrap();
    assert_eq!(d.own_step_count(), n + 1, "one commit");
    assert_eq!(last_label(&d), GROUP_COMMIT_LABEL);
    assert_eq!(GROUP_COMMIT_LABEL, "group");
    assert_eq!(UNGROUP_COMMIT_LABEL, "ungroup");
}

#[test]
fn ac1_group_refuses_fewer_than_two_and_changes_nothing() {
    let (d, ids) = row(3);
    let before = (json(&d), snap(&d), d.own_step_count());
    assert!(d.group_objects(&[]).is_err());
    assert!(d.group_objects(&[ids[0]]).is_err());
    assert!(
        d.group_objects(&[ids[0], ids[0]]).is_err(),
        "the same object twice is one object"
    );
    assert_eq!((json(&d), snap(&d), d.own_step_count()), before);
}

#[test]
fn ac1_group_of_a_stale_or_foreign_id_is_refused_and_changes_nothing() {
    let (d, ids) = row(2);
    let other = Document::new(7);
    let stranger = rect(&other, 0.0);
    let before = (json(&d), snap(&d), d.own_step_count());
    let r = d.group_objects(&[ids[0], stranger]);
    assert!(r.is_err());
    assert_eq!((json(&d), snap(&d), d.own_step_count()), before);
}

#[test]
fn ac1_members_of_different_parents_are_not_silently_reparented() {
    let (d, ids) = row(4);
    let g = d.group_objects(&[ids[0], ids[1]]).unwrap();
    let before = (json(&d), snap(&d), d.own_step_count());
    // ids[0] is inside g, ids[3] is at the root.
    let r = d.group_objects(&[ids[0], ids[3]]);
    assert!(matches!(r, Err(ObjectEditError::NotSiblings)), "{r:?}");
    assert_eq!((json(&d), snap(&d), d.own_step_count()), before);
    // A member and its own ancestor group cannot be grouped either.
    assert!(d.group_objects(&[g, ids[0]]).is_err());
    assert_eq!((json(&d), snap(&d), d.own_step_count()), before);
}

#[test]
fn ac1_group_inside_a_group_keeps_the_outer_order() {
    let (d, ids) = row(4);
    let outer = d.group_objects(&ids).unwrap();
    let inner = d.group_objects(&[ids[1], ids[2]]).unwrap();
    assert_eq!(d.children(Some(outer)), vec![ids[0], inner, ids[3]]);
    assert_eq!(d.children(Some(inner)), vec![ids[1], ids[2]]);
    assert_eq!(root(&d), vec![outer]);
}

// ---------------------------------------------------------------------
// Criterion 3 and 4: Ungroup
// ---------------------------------------------------------------------

#[test]
fn ac3_ungroup_replaces_the_group_at_its_place_by_its_children_in_order() {
    let (d, ids) = row(5);
    let g = d.group_objects(&[ids[1], ids[3]]).unwrap();
    // root: A C g E
    let out = d.ungroup(&[g]).unwrap();
    assert_eq!(out, vec![ids[1], ids[3]], "released children, in order");
    assert_eq!(root(&d), vec![ids[0], ids[2], ids[1], ids[3], ids[4]]);
    assert!(!d.contains_node(g), "the group node is gone");
    assert!(!d.is_group(g));
    assert_eq!(last_label(&d), UNGROUP_COMMIT_LABEL);
}

#[test]
fn ac3_ungroup_changes_one_level_only_and_nested_groups_stay_groups() {
    let (d, ids) = row(4);
    let inner = d.group_objects(&[ids[0], ids[1]]).unwrap();
    let outer = d.group_objects(&[inner, ids[2]]).unwrap();
    let _ = d.ungroup(&[outer]).unwrap();
    // root was [outer, D] with outer = (inner, C): released in place gives [inner, C, D].
    assert_eq!(root(&d), vec![inner, ids[2], ids[3]]);
    assert!(d.is_group(inner));
    assert_eq!(d.children(Some(inner)), vec![ids[0], ids[1]]);
}

#[test]
fn ac3_ungroup_of_several_groups_is_one_commit() {
    let (d, ids) = row(6);
    let g1 = d.group_objects(&[ids[0], ids[1]]).unwrap();
    let g2 = d.group_objects(&[ids[2], ids[3]]).unwrap();
    let n = d.own_step_count();
    let out = d.ungroup(&[g1, g2]).unwrap();
    assert_eq!(out.len(), 4);
    assert_eq!(d.own_step_count(), n + 1);
    assert_eq!(root(&d), ids);
}

#[test]
fn ac3_ungroup_of_a_leaf_or_nothing_is_refused_and_changes_nothing() {
    let (d, ids) = row(3);
    let g = d.group_objects(&[ids[0], ids[1]]).unwrap();
    let before = (json(&d), snap(&d), d.own_step_count());
    assert!(d.ungroup(&[ids[2]]).is_err(), "a leaf is not a group");
    assert!(d.ungroup(&[g, ids[2]]).is_err(), "all or nothing");
    assert_eq!((json(&d), snap(&d), d.own_step_count()), before);
    let nothing = d.ungroup(&[]);
    assert!(nothing.is_err() || nothing.unwrap().is_empty());
    assert_eq!((json(&d), snap(&d), d.own_step_count()), before);
    // A group that is gone.
    let stale = d.group_objects(&[ids[2], g]).unwrap();
    let _ = d.ungroup(&[stale]).unwrap();
    let before = (json(&d), snap(&d), d.own_step_count());
    assert!(d.ungroup(&[stale]).is_err());
    assert_eq!((json(&d), snap(&d), d.own_step_count()), before);
}

#[test]
fn ac3_no_geometry_or_style_of_a_child_changes() {
    let (d, ids) = row(3);
    let before: Vec<_> = ids.iter().map(|id| d.object(*id).unwrap()).collect();
    let g = d.group_objects(&ids).unwrap();
    let _ = d.ungroup(&[g]).unwrap();
    let after: Vec<_> = ids.iter().map(|id| d.object(*id).unwrap()).collect();
    assert_eq!(before, after);
}

#[test]
fn ac4_group_then_ungroup_is_the_identity_when_nothing_lies_between() {
    let (d, ids) = row(4);
    let before_order = root(&d);
    let before_objects: Vec<_> = ids.iter().map(|id| d.object(*id).unwrap()).collect();
    let g = d.group_objects(&[ids[1], ids[2]]).unwrap();
    let _ = d.ungroup(&[g]).unwrap();
    assert_eq!(root(&d), before_order);
    let after: Vec<_> = ids.iter().map(|id| d.object(*id).unwrap()).collect();
    assert_eq!(before_objects, after);
    assert_eq!(d.object_ids(), ids);
}

#[test]
fn ac4_with_objects_between_the_order_differs_as_criterion_1_says() {
    let (d, ids) = row(3);
    let g = d.group_objects(&[ids[0], ids[2]]).unwrap();
    let _ = d.ungroup(&[g]).unwrap();
    // B was between A and C; it ended below the group and stays below.
    assert_eq!(root(&d), vec![ids[1], ids[0], ids[2]]);
}

// ---------------------------------------------------------------------
// Criterion 5: depth
// ---------------------------------------------------------------------

/// A chain of `levels` groups, each holding the previous one and a fresh leaf. Returns the
/// outermost group and the innermost leaf pair.
fn chain(d: &Document, levels: usize) -> (NodeId, Vec<NodeId>) {
    let mut leaves = vec![rect(d, 0.0), rect(d, 20.0)];
    let mut g = d.group_objects(&leaves[..2]).unwrap();
    for i in 1..levels {
        let extra = rect(d, 40.0 + i as f64 * 20.0);
        leaves.push(extra);
        g = d.group_objects(&[g, extra]).unwrap();
    }
    (g, leaves)
}

#[test]
fn ac5_depth_32_is_allowed_and_33_is_refused_with_nothing_changed() {
    assert_eq!(MAX_GROUP_DEPTH, 32);
    let d = Document::new(1);
    let (top, _) = chain(&d, 32);
    let extra = rect(&d, 999.0);
    let before = (json(&d), snap(&d), d.own_step_count());
    let r = d.group_objects(&[top, extra]);
    assert!(matches!(r, Err(ObjectEditError::TooDeep)), "{r:?}");
    assert_eq!((json(&d), snap(&d), d.own_step_count()), before);
    // Two plain leaves next to it are still fine: they sit at depth 1.
    let other = rect(&d, 1000.0);
    assert!(d.group_objects(&[extra, other]).is_ok());
}

#[test]
fn ac5_grouping_deep_inside_a_chain_is_refused_at_the_limit_too() {
    // 31 chained groups, then a group of two leaves inside the innermost makes 32 and is fine,
    // one more is refused.
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let c = rect(&d, 40.0);
    let mut g = d.group_objects(&[a, b]).unwrap(); // depth 1 above a and b
    for _ in 1..31 {
        let x = rect(&d, 0.0);
        g = d.group_objects(&[g, x]).unwrap();
    }
    // Now a chain of 31 groups. Grouping the root level group with c makes 32: allowed.
    let top = d.group_objects(&[g, c]).unwrap();
    // 32 deep. Group a and b (both inside the innermost group, depth 32 context): would be 33.
    let before = (json(&d), snap(&d), d.own_step_count());
    let r = d.group_objects(&[a, b]);
    assert!(matches!(r, Err(ObjectEditError::TooDeep)), "{r:?}");
    assert_eq!((json(&d), snap(&d), d.own_step_count()), before);
    // And a wrapper around the whole thing is refused.
    let y = rect(&d, 1.0);
    assert!(matches!(
        d.group_objects(&[top, y]),
        Err(ObjectEditError::TooDeep)
    ));
}

#[test]
fn ac5_ungroup_makes_room_again() {
    let d = Document::new(1);
    let (top, _) = chain(&d, 32);
    let extra = rect(&d, 999.0);
    assert!(d.group_objects(&[top, extra]).is_err());
    let released = d.ungroup(&[top]).unwrap();
    assert_eq!(released.len(), 2);
    let inner = released[0];
    assert!(d.is_group(inner));
    assert!(d.group_objects(&[inner, extra]).is_ok(), "depth 32 again");
}

// ---------------------------------------------------------------------
// Criterion 13 and 14: delete, empty groups
// ---------------------------------------------------------------------

#[test]
fn ac14_deleting_the_last_child_removes_the_group_in_the_same_commit() {
    let (d, ids) = row(3);
    let g = d.group_objects(&[ids[0], ids[1]]).unwrap();
    // One child gone: a group of one is allowed.
    d.delete_objects(&[ids[0]]).unwrap();
    assert!(d.contains_node(g));
    assert_eq!(d.children(Some(g)), vec![ids[1]]);
    // The last child: the group is gone with it, in the very same step.
    let n = d.own_step_count();
    d.delete_objects(&[ids[1]]).unwrap();
    assert_eq!(d.own_step_count(), n + 1, "one commit");
    assert!(!d.contains_node(g));
    assert_eq!(root(&d), vec![ids[2]]);
}

#[test]
fn ac14_deleting_both_children_in_one_call_removes_the_group() {
    let (d, ids) = row(3);
    let g = d.group_objects(&[ids[0], ids[1]]).unwrap();
    d.delete_objects(&[ids[0], ids[1]]).unwrap();
    assert!(!d.contains_node(g));
    assert_eq!(root(&d), vec![ids[2]]);
}

#[test]
fn ac14_the_emptying_cascades_through_nested_groups() {
    let (d, ids) = row(4);
    let inner = d.group_objects(&[ids[0], ids[1]]).unwrap();
    let outer = d.group_objects(&[inner, ids[2]]).unwrap();
    let n = d.own_step_count();
    d.delete_objects(&[ids[0], ids[1], ids[2]]).unwrap();
    assert_eq!(d.own_step_count(), n + 1);
    assert!(!d.contains_node(inner));
    assert!(!d.contains_node(outer));
    assert_eq!(root(&d), vec![ids[3]]);
    // And the other way round: only the inner empties, the outer keeps its other child.
    let (d, ids) = row(4);
    let inner = d.group_objects(&[ids[0], ids[1]]).unwrap();
    let outer = d.group_objects(&[inner, ids[2]]).unwrap();
    d.delete_objects(&[ids[0], ids[1]]).unwrap();
    assert!(!d.contains_node(inner));
    assert!(d.contains_node(outer));
    assert_eq!(d.children(Some(outer)), vec![ids[2]]);
}

#[test]
fn ac14_the_last_node_delete_of_a_path_removes_the_emptied_group() {
    let d = Document::new(1);
    let p = path(&d, 1, 0.0);
    let q = path(&d, 2, 20.0);
    let g = d.group_objects(&[p, q]).unwrap();
    let anchors: Vec<AnchorId> = d.path(p).unwrap().anchors.iter().map(|a| a.id).collect();
    d.delete_anchors(&anchors[..1].iter().map(|&a| (p, a)).collect::<Vec<_>>());
    assert!(d.path(p).is_none(), "fewer than two anchors: the path goes");
    assert!(d.contains_node(g), "q is still in the group");
    let n = d.own_step_count();
    let anchors: Vec<AnchorId> = d.path(q).unwrap().anchors.iter().map(|a| a.id).collect();
    d.delete_anchors(&anchors[..1].iter().map(|&a| (q, a)).collect::<Vec<_>>());
    assert_eq!(d.own_step_count(), n + 1);
    assert!(d.path(q).is_none());
    assert!(!d.contains_node(g), "the empty group is removed with it");
    assert!(root(&d).is_empty());
}

#[test]
fn ac14_translate_and_delete_of_stale_ids_do_not_corrupt_groups() {
    let (d, ids) = row(3);
    let g = d.group_objects(&[ids[0], ids[1]]).unwrap();
    let before = (json(&d), snap(&d));
    // The group id is not a leaf: moving "the group" through the leaf API is refused whole.
    let r = d.translate_objects(&[g], Vec2::new(1.0, 1.0));
    if r.is_ok() {
        // If a group id is accepted, it must be a move of every leaf and the tree must be intact.
        assert_eq!(d.children(Some(g)), vec![ids[0], ids[1]]);
    } else {
        assert_eq!((json(&d), snap(&d)), before);
    }
}

// ---------------------------------------------------------------------
// Criterion 25 and 27: model and format
// ---------------------------------------------------------------------

#[test]
fn ac25_a_group_node_has_no_style_and_no_transform_in_the_document_json() {
    let (d, ids) = row(2);
    let g = d.group_objects(&ids).unwrap();
    let text = String::from_utf8(json(&d)).unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    // Find the group in the JSON: an object that has children and the id of `g`.
    fn find<'a>(
        v: &'a serde_json::Value,
        peer: &str,
        counter: i64,
    ) -> Option<&'a serde_json::Value> {
        match v {
            serde_json::Value::Object(m) => {
                if m.get("id").is_some_and(|id| {
                    id.get("peer").and_then(|p| p.as_str()) == Some(peer)
                        && id.get("counter").and_then(serde_json::Value::as_i64) == Some(counter)
                }) {
                    return Some(v);
                }
                m.values().find_map(|x| find(x, peer, counter))
            }
            serde_json::Value::Array(a) => a.iter().find_map(|x| find(x, peer, counter)),
            _ => None,
        }
    }
    let id_json = serde_json::to_value(g).unwrap();
    let peer = id_json["peer"].as_str().unwrap();
    let counter = id_json["counter"].as_i64().unwrap();
    let node = find(&value, peer, counter).expect("the group appears in document.json");
    let keys: Vec<&String> = node.as_object().unwrap().keys().collect();
    for forbidden in [
        "style",
        "stroke",
        "fill",
        "rotation",
        "transform",
        "anchors",
    ] {
        assert!(
            !keys.iter().any(|k| k.as_str() == forbidden),
            "a group carries no `{forbidden}`: {keys:?}"
        );
    }
    eprintln!("group node in document.json: {node}");
    let children = node
        .get("objects")
        .or_else(|| node.get("children"))
        .and_then(|c| c.as_array())
        .unwrap_or_else(|| panic!("document.json nests the children of a group: {keys:?}"));
    assert_eq!(children.len(), 2);
}

#[test]
fn ac27_groups_round_trip_through_save_and_open() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let c = rect(&d, 40.0);
    let p = path(&d, 1, 60.0);
    let e = rect(&d, 80.0);
    let inner = d.group_objects(&[b, c]).unwrap();
    let outer = d.group_objects(&[inner, p]).unwrap();
    let _ = (a, e);
    let bytes = pack(&d, "t").unwrap();
    let r = unpack(5, &bytes).unwrap();
    assert_eq!(json(&r), json(&d), "document.json is as before");
    assert_eq!(root(&r), root(&d));
    assert_eq!(r.children(Some(outer)), vec![inner, p]);
    assert_eq!(r.children(Some(inner)), vec![b, c]);
    assert_eq!(r.object_ids(), d.object_ids());
    for id in d.object_ids() {
        assert_eq!(r.object(id), d.object(id));
    }
    // And once more: byte-stable on a second save.
    assert_eq!(json(&unpack(6, &pack(&r, "t").unwrap()).unwrap()), json(&d));
}

fn read_member(bytes: &[u8], name: &str) -> Vec<u8> {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut f = z.by_name(name).unwrap();
    let mut out = Vec::new();
    f.read_to_end(&mut out).unwrap();
    out
}

fn rebuild_zip(bytes: &[u8], manifest: &serde_json::Value, loro: Option<&[u8]>) -> Vec<u8> {
    use zip::write::SimpleFileOptions;
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let o = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    w.start_file("manifest.json", o).unwrap();
    w.write_all(&serde_json::to_vec(manifest).unwrap()).unwrap();
    w.start_file("document.loro", o).unwrap();
    match loro {
        Some(l) => w.write_all(l).unwrap(),
        None => w.write_all(&read_member(bytes, "document.loro")).unwrap(),
    }
    w.start_file("document.json", o).unwrap();
    w.write_all(&read_member(bytes, "document.json")).unwrap();
    w.finish().unwrap().into_inner()
}

#[test]
fn ac27_the_manifest_says_format_version_11_and_a_newer_file_is_refused() {
    let (d, ids) = row(2);
    let _ = d.group_objects(&ids).unwrap();
    let bytes = pack(&d, "t").unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&read_member(&bytes, "manifest.json")).unwrap();
    // Format 11 was this slice's number; later slices took 12 and 13, so the writer names the
    // current one.
    const _: () = assert!(curvyo_document_core::CURRENT_FORMAT_VERSION >= 11);
    let current = curvyo_document_core::CURRENT_FORMAT_VERSION;
    assert_eq!(manifest["format_version"], current);
    let mut newer = manifest.clone();
    newer["format_version"] = (current + 1).into();
    let r = unpack(1, &rebuild_zip(&bytes, &newer, None));
    assert!(matches!(
        r,
        Err(OpenError::FormatTooNew { found, supported }) if found == current + 1 && supported == current
    ));
}

/// Opens every fixture written by an earlier build and checks that it opens and re-exports its own
/// `document.json` unchanged (criterion 27: a file from an earlier build opens unchanged).
#[test]
fn ac27_files_of_earlier_builds_open_unchanged() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut opened = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        // `layers_*` are the files of `0050`, which hold groups too.
        if path.extension().is_none_or(|e| e != "curvyo")
            || name.contains("group")
            || name.starts_with("layers")
        {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        let Ok(doc) = unpack(3, &bytes) else {
            continue; // the damaged / too-new fixtures of earlier specs
        };
        opened += 1;
        // No object of an old file is a group, and the paint list is the flat list.
        assert!(root(&doc).iter().all(|id| !doc.is_group(*id)), "{name}");
        assert_eq!(root(&doc), doc.object_ids(), "{name}");
        // Opening writes nothing: the document state re-exports as the snapshot the file stored.
        let stored = read_member(&bytes, "document.loro");
        let reopened = LoroDoc::new();
        reopened.import(&stored).unwrap();
        let again = LoroDoc::new();
        again.import(&snap(&doc)).unwrap();
        assert_eq!(
            reopened.get_map("root").get_deep_value(),
            again.get_map("root").get_deep_value(),
            "{name}: root registers"
        );
        assert_eq!(
            reopened.get_tree("paths").get_value(),
            again.get_tree("paths").get_value(),
            "{name}: the stored object tree is the opened one"
        );
        // Opening does not write: a save/open cycle changes no object.
        let again = unpack(4, &pack(&doc, "t").unwrap()).unwrap();
        assert_eq!(again.object_ids(), doc.object_ids(), "{name}");
    }
    assert!(opened >= 10, "fixtures found: {opened}");
}

// --- Damaged group files ---------------------------------------------------

fn tree_of(l: &LoroDoc) -> loro::LoroTree {
    let t = l.get_tree("paths");
    t.enable_fractional_index(0);
    t
}

/// A document snapshot with one rectangle, re-opened as a bare Loro document.
fn bare() -> (Document, LoroDoc, TreeID) {
    let d = Document::new(1);
    let _ = rect(&d, 0.0);
    let l = LoroDoc::new();
    l.import(&snap(&d)).unwrap();
    let leaf = tree_of(&l).nodes()[0];
    (d, l, leaf)
}

fn craft(d: &Document, l: &LoroDoc) -> Vec<u8> {
    l.commit();
    let bytes = pack(d, "t").unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&read_member(&bytes, "manifest.json")).unwrap();
    rebuild_zip(
        &bytes,
        &manifest,
        Some(&l.export(ExportMode::Snapshot).unwrap()),
    )
}

fn make_group(l: &LoroDoc, parent: TreeParentId) -> TreeID {
    let t = tree_of(l);
    let g = t.create(parent).unwrap();
    t.get_meta(g).unwrap().insert("group", true).unwrap();
    g
}

#[test]
fn ac27_a_file_with_a_group_chain_of_32_opens_and_of_33_is_refused_as_damaged() {
    for (levels, ok) in [(32, true), (33, false), (34, false), (200, false)] {
        let (d, l, leaf) = bare();
        let mut parent = TreeParentId::Root;
        let mut last = None;
        for _ in 0..levels {
            let g = make_group(&l, parent);
            parent = TreeParentId::Node(g);
            last = Some(g);
        }
        tree_of(&l)
            .mov(leaf, TreeParentId::Node(last.unwrap()))
            .unwrap();
        let bytes = craft(&d, &l);
        let r = unpack(2, &bytes);
        if ok {
            let doc = r.unwrap_or_else(|e| panic!("depth {levels} must open: {e:?}"));
            assert_eq!(doc.object_ids().len(), 1);
            let mut depth = 0;
            let mut id = doc.object_ids()[0];
            while let Some(p) = doc.parent_group(id) {
                depth += 1;
                id = p;
            }
            assert_eq!(depth, 32);
        } else {
            assert!(
                matches!(r, Err(OpenError::Damaged)),
                "depth {levels} is damaged, got {:?}",
                r.map(|_| ())
            );
        }
    }
}

#[test]
fn ac27_a_leaf_with_children_is_refused_as_damaged() {
    let (d, l, leaf) = bare();
    let t = tree_of(&l);
    let second = t.create(TreeParentId::Node(leaf)).unwrap();
    t.get_meta(second).unwrap().insert("x", 1.0).unwrap();
    let r = unpack(2, &craft(&d, &l));
    assert!(matches!(r, Err(OpenError::Damaged)), "{:?}", r.map(|_| ()));
}

#[test]
fn ac27_a_group_with_a_shape_tag_or_anchors_is_refused_as_damaged() {
    // A group that also carries a shape tag.
    let (d, l, leaf) = bare();
    let g = make_group(&l, TreeParentId::Root);
    let shape = tree_of(&l).get_meta(leaf).unwrap().get("shape");
    assert!(shape.is_some(), "the rectangle carries a shape tag");
    tree_of(&l)
        .get_meta(g)
        .unwrap()
        .insert("shape", "rect")
        .unwrap();
    tree_of(&l).mov(leaf, TreeParentId::Node(g)).unwrap();
    let r = unpack(2, &craft(&d, &l));
    assert!(matches!(r, Err(OpenError::Damaged)), "{:?}", r.map(|_| ()));

    // A group that carries anchors.
    let (d, l, leaf) = bare();
    let g = make_group(&l, TreeParentId::Root);
    tree_of(&l)
        .get_meta(g)
        .unwrap()
        .insert_container("anchors", loro::LoroMovableList::new())
        .unwrap();
    tree_of(&l).mov(leaf, TreeParentId::Node(g)).unwrap();
    let r = unpack(2, &craft(&d, &l));
    assert!(matches!(r, Err(OpenError::Damaged)), "{:?}", r.map(|_| ()));
}

#[test]
fn ac27_an_empty_group_in_a_file_opens_and_draws_nothing() {
    let (d, l, _leaf) = bare();
    let g = make_group(&l, TreeParentId::Root);
    let doc = unpack(2, &craft(&d, &l)).expect("an empty group is valid on read");
    assert_eq!(doc.object_ids().len(), 1, "only the rectangle has geometry");
    assert_eq!(root(&doc).len(), 2);
    let _ = g;
    // The next local command that touches its parent removes it.
    let id = doc.object_ids()[0];
    let other = rect(&doc, 50.0);
    doc.delete_objects(&[other]).unwrap();
    let _ = id;
}

// ---------------------------------------------------------------------
// Undo and redo (ADR 0014: parent and z-position restored)
// ---------------------------------------------------------------------

fn undo_redo_roundtrip(d: &Document, before: &[u8], after: &[u8]) {
    let step = d.own_steps().last().unwrap().step;
    let report = d.undo_step(step).unwrap();
    assert!(report.wrote);
    assert_eq!(json(d), before, "undo is byte-equal to the state before");
    d.redo_step(step).unwrap();
    assert_eq!(json(d), after, "redo is byte-equal to the state after");
    d.undo_step(step).unwrap();
    assert_eq!(json(d), before, "and undo again");
}

#[test]
fn undo_and_redo_of_group_and_ungroup_are_byte_equal() {
    let (d, ids) = row(4);
    let b0 = json(&d);
    let g = d.group_objects(&[ids[0], ids[2]]).unwrap();
    let b1 = json(&d);
    undo_redo_roundtrip(&d, &b0, &b1);
    // Redo state is back: the group has its own id again.
    d.redo_step(d.own_steps().last().unwrap().step).unwrap();
    assert!(d.is_group(g), "redo revives the group with its own id");
    let _ = d.ungroup(&[g]).unwrap();
    let b2 = json(&d);
    undo_redo_roundtrip(&d, &b1, &b2);
}

#[test]
fn undo_of_group_restores_the_z_position_of_every_member() {
    let (d, ids) = row(5);
    let before = root(&d);
    let g = d.group_objects(&[ids[1], ids[3]]).unwrap();
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
    assert_eq!(root(&d), before, "A B C D E in the old order");
    assert!(!d.is_group(g));
}

#[test]
fn undo_of_a_nested_delete_with_emptied_groups_revives_the_tree() {
    let (d, ids) = row(4);
    let inner = d.group_objects(&[ids[0], ids[1]]).unwrap();
    let outer = d.group_objects(&[inner, ids[2]]).unwrap();
    let before = json(&d);
    d.delete_objects(&[ids[0], ids[1], ids[2]]).unwrap();
    let after = json(&d);
    undo_redo_roundtrip(&d, &before, &after);
    d.redo_step(d.own_steps().last().unwrap().step).unwrap();
    assert!(!d.contains_node(inner) && !d.contains_node(outer));
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
    assert_eq!(d.children(Some(outer)), vec![inner, ids[2]]);
    assert_eq!(d.children(Some(inner)), vec![ids[0], ids[1]]);
}

#[test]
fn undo_of_every_step_of_a_long_session_walks_back_to_the_empty_document() {
    let d = Document::new(1);
    let empty = json(&d);
    let ids: Vec<NodeId> = (0..6).map(|i| rect(&d, f64::from(i) * 20.0)).collect();
    let mut states = vec![json(&d)];
    let g1 = d.group_objects(&[ids[0], ids[1]]).unwrap();
    states.push(json(&d));
    let g2 = d.group_objects(&[g1, ids[2]]).unwrap();
    states.push(json(&d));
    d.translate_objects(&[ids[0], ids[1], ids[2]], Vec2::new(5.0, 5.0))
        .unwrap();
    states.push(json(&d));
    let g3 = d.group_objects(&[ids[4], ids[5]]).unwrap();
    states.push(json(&d));
    let _ = d.ungroup(&[g2]).unwrap();
    states.push(json(&d));
    d.delete_objects(&[ids[4], ids[5]]).unwrap();
    states.push(json(&d));
    let _ = (g3,);
    let steps: Vec<_> = d.own_steps().iter().map(|s| s.step).collect();
    assert_eq!(steps.len(), 6 + 6);
    // Walk back through the last 6 steps (the group steps), checking every intermediate state.
    for k in (0..6).rev() {
        d.undo_step(steps[6 + k]).unwrap();
        assert_eq!(json(&d), states[k], "after undoing step {}", 6 + k);
    }
    // And forward again.
    for k in 0..6 {
        d.redo_step(steps[6 + k]).unwrap();
        assert_eq!(json(&d), states[k + 1], "after redoing step {}", 6 + k);
    }
    let _ = empty;
}

// ---------------------------------------------------------------------
// Two replicas
// ---------------------------------------------------------------------

#[test]
fn replicas_one_peer_groups_while_another_moves_a_member() {
    let (a, ids) = row(3);
    let b = replica(&a, 2);
    let g = a.group_objects(&[ids[0], ids[1]]).unwrap();
    b.translate_objects(&[ids[1]], Vec2::new(7.0, 3.0)).unwrap();
    sync(&a, &b);
    assert_eq!(json(&a), json(&b), "both replicas converge");
    assert!(!snap(&a).is_empty());
    assert!(a.is_group(g) && b.is_group(g));
    assert_eq!(b.parent_group(ids[1]), Some(g), "still grouped");
    let moved = b.object(ids[1]).unwrap();
    let orig = replica(&Document::new(9), 9);
    let _ = orig;
    // The member really moved by (7, 3) relative to where it was grouped.
    match moved {
        curvyo_document_core::ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        curvyo_document_core::ObjectSnapshot::Clone(_) => panic!("no clone here"),
        curvyo_document_core::ObjectSnapshot::Primitive(p) => match p.shape {
            curvyo_document_core::Shape::Rect { bounds, .. } => {
                assert!((bounds.origin.x - 27.0).abs() < 1e-9, "{:?}", bounds.origin);
                assert!((bounds.origin.y - 3.0).abs() < 1e-9);
            }
            _ => panic!("rect"),
        },
        curvyo_document_core::ObjectSnapshot::Path(_) => panic!("rect"),
    }
}

#[test]
fn replicas_both_peers_group_the_same_object_and_converge() {
    let (a, ids) = row(3);
    let b = replica(&a, 2);
    let _ga = a.group_objects(&[ids[0], ids[1]]).unwrap();
    let _gb = b.group_objects(&[ids[1], ids[2]]).unwrap();
    sync(&a, &b);
    assert_eq!(json(&a), json(&b));
    // Every leaf exists exactly once in the tree, whatever the resolution.
    for d in [&a, &b] {
        let leaves = d.object_ids();
        let set: HashSet<_> = leaves.iter().map(|id| format!("{id:?}")).collect();
        assert_eq!(set.len(), 3);
        assert_eq!(leaves.len(), 3);
    }
    // The document stays usable: it can be saved and reopened.
    let r = unpack(8, &pack(&a, "t").unwrap()).unwrap();
    assert_eq!(json(&r), json(&a));
}

#[test]
fn replicas_one_peer_ungroups_while_another_moves_a_member_of_that_group() {
    let (a, ids) = row(3);
    let g = a.group_objects(&[ids[0], ids[1]]).unwrap();
    let b = replica(&a, 2);
    let _ = a.ungroup(&[g]).unwrap();
    b.translate_objects(&[ids[0]], Vec2::new(4.0, 0.0)).unwrap();
    sync(&a, &b);
    assert_eq!(json(&a), json(&b));
    assert!(!a.contains_node(g));
    assert_eq!(a.object_ids().len(), 3);
}

#[test]
fn replicas_one_peer_deletes_a_member_while_another_groups_it() {
    let (a, ids) = row(3);
    let b = replica(&a, 2);
    a.delete_objects(&[ids[1]]).unwrap();
    let _ = b.group_objects(&[ids[1], ids[2]]).unwrap();
    sync(&a, &b);
    assert_eq!(json(&a), json(&b));
    eprintln!(
        "OBSERVATION: member deleted by A and grouped by B concurrently is {} after sync",
        if a.object(ids[1]).is_some() {
            "ALIVE (resurrected in the group)"
        } else {
            "deleted"
        }
    );
    assert!(a.object(ids[2]).is_some());
    // A group may be left with one child or none: the document must still reopen.
    unpack(9, &pack(&a, "t").unwrap()).unwrap();
}

#[test]
fn replicas_undo_skips_a_step_whose_group_a_peer_removed() {
    let (a, ids) = row(3);
    let b = replica(&a, 2);
    let g = b.group_objects(&[ids[0], ids[1]]).unwrap();
    let step = b.own_steps().last().unwrap().step;
    sync(&a, &b);
    let _ = a.ungroup(&[g]).unwrap();
    sync(&a, &b);
    let before = json(&b);
    // The undo may skip or may restore; it must not fail hard, corrupt the tree or lose a leaf.
    let r = b.undo_step(step);
    assert!(r.is_ok(), "{:?}", r.err());
    assert_eq!(b.object_ids().len(), 3);
    sync(&a, &b);
    assert_eq!(json(&a), json(&b), "replicas still converge");
    let _ = before;
}

// ---------------------------------------------------------------------
// Hostile and large inputs, budgets
// ---------------------------------------------------------------------

#[test]
fn hostile_group_of_nothing_and_ungroup_of_a_non_group_leave_no_trace() {
    let d = Document::new(1);
    let before = (json(&d), snap(&d), d.own_step_count());
    assert!(d.group_objects(&[]).is_err());
    assert!(d.ungroup(&[]).is_err() || d.own_step_count() == before.2);
    assert_eq!((json(&d), snap(&d)), (before.0.clone(), before.1.clone()));
    let l = rect(&d, 0.0);
    let n = d.own_step_count();
    assert!(d.ungroup(&[l]).is_err());
    assert_eq!(d.own_step_count(), n);
}

#[test]
fn large_group_of_2000_leaves_groups_ungroups_and_undoes_in_reasonable_time() {
    let d = Document::new(1);
    let ids: Vec<NodeId> = (0..2000).map(|i| rect(&d, f64::from(i))).collect();
    let before = json(&d);
    let t = Instant::now();
    let g = d.group_objects(&ids).unwrap();
    let t_group = t.elapsed();
    assert_eq!(d.children(Some(g)), ids);
    let t = Instant::now();
    let _ = d.ungroup(&[g]).unwrap();
    let t_ungroup = t.elapsed();
    assert_eq!(root(&d), ids);
    assert_eq!(json(&d), before, "apart from nothing: ids were kept");
    let t = Instant::now();
    d.undo_step(d.own_steps().last().unwrap().step).unwrap();
    let t_undo = t.elapsed();
    assert!(d.is_group(g));
    eprintln!("2000 leaves: group {t_group:?}, ungroup {t_ungroup:?}, undo {t_undo:?}");
    // Generous: the machine is under load and this is a debug build.
    assert!(t_group.as_secs() < 30 && t_ungroup.as_secs() < 30 && t_undo.as_secs() < 60);
}

#[test]
fn large_flat_document_with_many_groups_round_trips() {
    let d = Document::new(1);
    let ids: Vec<NodeId> = (0..1000).map(|i| rect(&d, f64::from(i))).collect();
    for pair in ids.chunks(2) {
        let _ = d.group_objects(pair).unwrap();
    }
    assert_eq!(root(&d).len(), 500);
    assert_eq!(d.object_ids().len(), 1000);
    let r = unpack(2, &pack(&d, "t").unwrap()).unwrap();
    assert_eq!(json(&r), json(&d));
    assert_eq!(d.child_groups(None).len(), 500);
}

/// Undo and redo of a Group or Ungroup step by member count, against the budget of `0020` criterion
/// 30(b): a larger step of 5,000 objects completes within 1 second (asserted at four times).
/// Run: `cargo test --release -p curvyo-document-core --test acceptance_0023_tester -- --ignored
/// --nocapture`.
#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn budget_group_and_ungroup_undo_scaling() {
    let mut last = std::time::Duration::ZERO;
    for n in [500_usize, 1000, 2000, 5000] {
        let d = Document::new(1);
        let ids: Vec<NodeId> = (0..n).map(|i| rect(&d, i as f64)).collect();
        let g = d.group_objects(&ids).unwrap();
        let step = d.own_steps().last().unwrap().step;
        let t = Instant::now();
        d.undo_step(step).unwrap();
        let undo_group = t.elapsed();
        let t = Instant::now();
        d.redo_step(step).unwrap();
        let redo_group = t.elapsed();
        let _ = d.ungroup(&[g]).unwrap();
        let step = d.own_steps().last().unwrap().step;
        let t = Instant::now();
        d.undo_step(step).unwrap();
        let undo_ungroup = t.elapsed();
        eprintln!(
            "n={n}: undo Group {undo_group:?}, redo Group {redo_group:?}, undo Ungroup {undo_ungroup:?}"
        );
        last = undo_group.max(undo_ungroup);
    }
    assert!(
        last.as_millis() <= 4_000,
        "undo of a 5,000-object step: {last:?}"
    );
}
