//! Independent tester tests for `specs/0050-layers-and-objects-panel` at the document model and the
//! file format: the registers, the effective values, the commands, the moves, format version 12,
//! damaged files and the output statement of criterion 54. Written from the specification; the
//! on-disk register keys were read from `adrs.md` only for the crafted files.
//!
//! The randomised cases use a fixed-seed generator instead of `proptest`, so no dev-dependency is
//! added to this crate.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation
)]
#![allow(clippy::items_after_statements)]
#![allow(missing_docs, clippy::doc_markdown, clippy::needless_pass_by_value)]
#![allow(
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::type_complexity
)]

use std::io::{Cursor, Read, Write};
use std::time::Instant;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, Length, MAX_NAME_CHARS, MovePlan, MoveRefusal, MoveTarget,
    NodeId, NodeKind, ObjectEditError, Opacity, OpenError, Point, RectBounds, pack, unpack,
};
use loro::{ExportMode, LoroDoc, TreeID, TreeParentId};

fn rect(d: &Document, x: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn op(v: f64) -> Opacity {
    Opacity::new(v).unwrap()
}

fn mv(d: &Document, nodes: &[NodeId], target: MoveTarget) -> Result<(), MoveRefusal> {
    d.move_nodes(&MovePlan {
        nodes: nodes.to_vec(),
        target,
    })
}

fn label_count(d: &Document) -> usize {
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let ids: Vec<loro::ID> = l.oplog_frontiers().iter().collect();
    let mut n = 0;
    l.travel_change_ancestors(&ids, &mut |_| {
        n += 1;
        std::ops::ControlFlow::Continue(())
    })
    .unwrap();
    n
}

fn last_label(d: &Document) -> String {
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let ids: Vec<loro::ID> = l.oplog_frontiers().iter().collect();
    let mut changes: Vec<(u32, String)> = Vec::new();
    l.travel_change_ancestors(&ids, &mut |c| {
        changes.push((
            c.lamport,
            c.message
                .map(|m| m.split(';').next().unwrap_or_default().to_string())
                .unwrap_or_default(),
        ));
        std::ops::ControlFlow::Continue(())
    })
    .unwrap();
    changes.sort();
    changes.pop().unwrap().1
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

fn tree_of(l: &LoroDoc) -> loro::LoroTree {
    let t = l.get_tree("paths");
    t.enable_fractional_index(0);
    t
}

/// A document with one rectangle, as a bare Loro document, to be damaged by hand.
fn bare() -> (Document, LoroDoc, TreeID) {
    let d = Document::new(1);
    let _ = rect(&d, 0.0);
    let l = LoroDoc::new();
    l.import(&d.export_loro_snapshot().unwrap()).unwrap();
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

fn json(d: &Document) -> serde_json::Value {
    serde_json::from_slice(&d.export_json().unwrap()).unwrap()
}

// ---------------------------------------------------------------------------
// Registers and effective values (criteria 18, 20, 22, 24, 27)
// ---------------------------------------------------------------------------

#[test]
fn registers_default_to_visible_unlocked_opaque_and_unnamed() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    let r = d.node_registers(a).unwrap();
    assert!(r.visible && !r.locked);
    assert_eq!(r.opacity.get(), 1.0);
    assert_eq!(r.name, None);
}

#[test]
fn a_document_without_flags_exports_no_flag_keys_at_all() {
    let d = Document::new(1);
    rect(&d, 0.0);
    let json: serde_json::Value = serde_json::from_slice(&d.export_json().unwrap()).unwrap();
    let text = json["objects"].to_string();
    for key in ["\"visible\"", "\"locked\"", "\"layer\"", "\"name\""] {
        assert!(!text.contains(key), "{key} in {text}");
    }
}

#[test]
fn setters_take_many_ids_in_one_commit_and_refuse_stale_ids_without_writing() {
    let d = Document::new(1);
    let ids: Vec<NodeId> = (0..5).map(|i| rect(&d, f64::from(i) * 20.0)).collect();
    let n0 = label_count(&d);
    d.set_node_visible(&ids, false).unwrap();
    assert_eq!(label_count(&d), n0 + 1);
    assert_eq!(last_label(&d), "set_node_visible");
    assert!(ids.iter().all(|&i| !d.node_registers(i).unwrap().visible));
    d.set_node_locked(&ids, true).unwrap();
    assert_eq!(last_label(&d), "set_node_locked");
    // Opacity of a locked node is refused.
    let n1 = label_count(&d);
    assert_eq!(
        d.set_node_opacity(&ids, op(0.5)),
        Err(ObjectEditError::Locked)
    );
    assert_eq!(label_count(&d), n1);
    d.set_node_locked(&ids, false).unwrap();
    d.set_node_opacity(&ids, op(0.5)).unwrap();
    assert_eq!(last_label(&d), "set_node_opacity");
    // A stale id: nothing written, even for the live ones.
    let stale: NodeId = serde_json::from_str("{\"peer\":\"77\",\"counter\":123456}").unwrap();
    let n2 = label_count(&d);
    let mixed = [ids[0], stale];
    assert_eq!(
        d.set_node_visible(&mixed, true),
        Err(ObjectEditError::NoSuchObject)
    );
    assert_eq!(
        d.set_node_locked(&mixed, true),
        Err(ObjectEditError::NoSuchObject)
    );
    assert_eq!(
        d.set_node_opacity(&mixed, op(0.1)),
        Err(ObjectEditError::NoSuchObject)
    );
    assert_eq!(
        d.rename_node(stale, "x"),
        Err(ObjectEditError::NoSuchObject)
    );
    assert_eq!(label_count(&d), n2);
    assert!(
        !d.node_registers(ids[0]).unwrap().visible,
        "first id untouched by the failed call"
    );
    // Duplicate ids in one call are fine.
    d.set_node_visible(&[ids[1], ids[1]], true).unwrap();
    assert!(d.node_registers(ids[1]).unwrap().visible);
}

#[test]
fn rename_cleans_the_name_as_the_spec_says() {
    let d = Document::new(1);
    let a = rect(&d, 0.0);
    d.rename_node(a, "  hello \u{0}\u{7}world\t ").unwrap();
    assert_eq!(
        d.node_registers(a).unwrap().name.as_deref(),
        Some("hello world")
    );
    d.rename_node(a, &"é".repeat(500)).unwrap();
    assert_eq!(
        d.node_registers(a).unwrap().name.unwrap().chars().count(),
        MAX_NAME_CHARS
    );
    assert_eq!(MAX_NAME_CHARS, 128);
    d.rename_node(a, "a\u{202E}b\u{2066}c\u{2069}").unwrap();
    assert_eq!(d.node_registers(a).unwrap().name.as_deref(), Some("abc"));
    d.rename_node(a, "   ").unwrap();
    assert_eq!(d.node_registers(a).unwrap().name, None);
    d.rename_node(a, "<b>x</b>").unwrap();
    assert_eq!(
        d.node_registers(a).unwrap().name.as_deref(),
        Some("<b>x</b>")
    );
}

#[test]
fn effective_values_come_from_the_ancestors_and_name_the_cause() {
    let d = Document::new(1);
    let outer = d.new_layer(None);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let g = d.group_objects(&[a, b]).unwrap();
    mv(&d, &[g], MoveTarget::FrontOf(Some(outer))).unwrap();
    d.set_node_opacity(&[outer], op(0.5)).unwrap();
    d.set_node_opacity(&[g], op(0.5)).unwrap();
    d.set_node_opacity(&[a], op(0.4)).unwrap();
    d.set_node_visible(&[outer], false).unwrap();
    d.set_node_locked(&[g], true).unwrap();
    let read = d.read_nodes();
    let get = |id: NodeId| read.iter().find(|n| n.id == id).unwrap();
    let ra = get(a);
    assert!(!ra.visible, "hidden through the layer");
    assert!(ra.locked, "locked through the group");
    assert!((ra.opacity.get() - 0.1).abs() < 1e-9, "0.5 * 0.5 * 0.4");
    assert_eq!(ra.hidden_by, Some(outer));
    assert_eq!(ra.locked_by, Some(g));
    assert!(get(a).registers.visible, "own flag untouched");
    let rb = get(b);
    assert!((rb.opacity.get() - 0.25).abs() < 1e-9);
    // The layer itself: own flag off, no cause above it.
    let rl = get(outer);
    assert!(!rl.visible);
    assert_eq!(rl.hidden_by, None);
    assert!(matches!(rl.kind, NodeKind::Layer));
    assert!(matches!(get(g).kind, NodeKind::Group));
    assert!(matches!(ra.kind, NodeKind::Leaf(_)));
    // Showing the layer restores the node as it was, own flags included.
    d.set_node_visible(&[outer], true).unwrap();
    d.set_node_visible(&[a], false).unwrap();
    d.set_node_visible(&[outer], false).unwrap();
    d.set_node_visible(&[outer], true).unwrap();
    let read = d.read_nodes();
    let ra = read.iter().find(|n| n.id == a).unwrap();
    assert!(!ra.registers.visible && !ra.visible);
}

/// A small deterministic generator.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

#[test]
fn randomised_trees_agree_with_a_naive_walk_of_effective_values() {
    for seed in 1..=40_u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let d = Document::new(1);
        let layers: Vec<NodeId> = (0..rng.below(4)).map(|_| d.new_layer(None)).collect();
        let n = 3 + rng.below(12);
        let mut leaves = Vec::new();
        for i in 0..n {
            let leaf = rect(&d, i as f64 * 15.0);
            leaves.push(leaf);
        }
        // Random groups of two adjacent loose leaves.
        let mut parent_of: std::collections::HashMap<NodeId, Option<NodeId>> =
            leaves.iter().map(|&l| (l, None)).collect();
        let mut groups = Vec::new();
        for _ in 0..rng.below(3) {
            let free: Vec<NodeId> = leaves
                .iter()
                .copied()
                .filter(|l| parent_of[l].is_none())
                .collect();
            if free.len() < 2 {
                break;
            }
            let g = d.group_objects(&[free[0], free[1]]).unwrap();
            parent_of.insert(free[0], Some(g));
            parent_of.insert(free[1], Some(g));
            parent_of.insert(g, None);
            groups.push(g);
        }
        // Random layer membership for loose nodes.
        let mut top: Vec<NodeId> = leaves
            .iter()
            .copied()
            .filter(|l| parent_of[l] == Some(None).flatten())
            .chain(groups.iter().copied())
            .collect();
        top.retain(|t| parent_of.get(t) == Some(&None));
        for t in top {
            if !layers.is_empty() && rng.below(2) == 0 {
                let l = layers[rng.below(layers.len() as u64) as usize];
                mv(&d, &[t], MoveTarget::FrontOf(Some(l))).unwrap();
                parent_of.insert(t, Some(l));
            }
        }
        // Random flags.
        let all: Vec<NodeId> = layers
            .iter()
            .copied()
            .chain(groups.iter().copied())
            .chain(leaves.iter().copied())
            .collect();
        let mut own: std::collections::HashMap<NodeId, (bool, bool, f64)> =
            std::collections::HashMap::new();
        for &id in &all {
            let v = rng.below(4) != 0;
            let k = rng.below(4) == 0;
            let o = f64::from(rng.below(101) as u32) / 100.0;
            d.set_node_visible(&[id], v).unwrap();
            if k {
                d.set_node_locked(&[id], true).unwrap();
            }
            d.set_node_opacity(&[id], op(o)).unwrap_or(());
            own.insert(id, (v, k, o));
        }
        // Parent chain: a leaf's group is the parent_of entry for the leaf; groups were put in
        // layers through the map above.
        let read = d.read_nodes();
        let parent = |id: NodeId| parent_of.get(&id).copied().flatten();
        for &id in &all {
            let mut vis = true;
            let mut lock = false;
            let mut opa = 1.0;
            let mut cur = Some(id);
            while let Some(c) = cur {
                let (v, k, o) = own[&c];
                vis &= v;
                lock |= k;
                opa *= o;
                cur = parent(c);
            }
            let r = read.iter().find(|n| n.id == id).unwrap();
            assert_eq!(r.visible, vis, "seed {seed} visible {id:?}");
            // A node locked after its opacity was set could have refused the opacity write; the
            // expected value follows what the register holds.
            let regs = d.node_registers(id).unwrap();
            let mut opa2 = 1.0;
            let mut cur = Some(id);
            while let Some(c) = cur {
                opa2 *= d.node_registers(c).unwrap().opacity.get();
                cur = parent(c);
            }
            assert!(
                (r.opacity.get() - opa2).abs() < 1e-9,
                "seed {seed} opacity {id:?}"
            );
            assert_eq!(r.locked, lock, "seed {seed} locked {id:?}");
            assert_eq!(regs.visible, own[&id].0);
            let _ = opa;
        }
        assert_eq!(
            read.len(),
            all.len(),
            "seed {seed}: every node is read once"
        );
    }
}

// ---------------------------------------------------------------------------
// Layers: new and delete (criteria 34 to 37), moves (29 to 31)
// ---------------------------------------------------------------------------

#[test]
fn new_layer_is_named_with_the_smallest_free_number_and_stacked_in_front_of_the_given_one() {
    let d = Document::new(1);
    let l1 = d.new_layer(None);
    let l2 = d.new_layer(None);
    let l3 = d.new_layer(None);
    let name = |id: NodeId| d.node_registers(id).unwrap().name.unwrap();
    assert_eq!(
        [name(l1), name(l2), name(l3)],
        ["Layer 1", "Layer 2", "Layer 3"]
    );
    d.rename_node(l2, "Cut").unwrap();
    let again = d.new_layer(Some(l1));
    assert_eq!(name(again), "Layer 2");
    assert_eq!(d.children(None), [l1, again, l2, l3]);
    // A stale or non-layer anchor falls back to the top of the stack.
    let leaf = rect(&d, 0.0);
    let after_leaf = d.new_layer(Some(leaf));
    assert_eq!(*d.children(None).last().unwrap(), after_leaf);
    assert_eq!(last_label(&d), "new_layer");
    assert!(d.is_layer(after_leaf) && !d.is_layer(leaf));
}

#[test]
fn delete_layer_refuses_non_layers_and_stale_ids_and_reports_the_count() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let g = d.group_objects(&[a, b]).unwrap();
    mv(&d, &[g], MoveTarget::FrontOf(Some(l))).unwrap();
    let leaf = rect(&d, 40.0);
    assert!(d.delete_layer(leaf).is_err(), "a leaf is not a layer");
    assert!(d.delete_layer(g).is_err(), "a group is not a layer");
    let n0 = label_count(&d);
    let count = d.delete_layer(l).unwrap();
    assert_eq!(count, 3, "the group and its two leaves");
    assert_eq!(label_count(&d), n0 + 1);
    assert_eq!(last_label(&d), "delete_layer");
    assert!(d.delete_layer(l).is_err(), "gone");
    assert_eq!(d.object_ids(), [leaf]);
}

#[test]
fn a_layer_stays_when_its_last_child_goes_but_an_emptied_group_does_not() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = rect(&d, 0.0);
    mv(&d, &[a], MoveTarget::FrontOf(Some(l))).unwrap();
    d.delete_objects(&[a]).unwrap();
    assert!(d.is_layer(l));
    let b = rect(&d, 0.0);
    mv(&d, &[b], MoveTarget::FrontOf(Some(l))).unwrap();
    mv(&d, &[b], MoveTarget::FrontOf(None)).unwrap();
    assert!(d.is_layer(l), "moving out the last child leaves the layer");
}

#[test]
fn move_rules_of_criterion_30() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let l2 = d.new_layer(None);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let c = rect(&d, 40.0);
    let g = d.group_objects(&[a, b]).unwrap();
    // Layers stay at the top level.
    assert_eq!(
        mv(&d, &[l2], MoveTarget::FrontOf(Some(g))),
        Err(MoveRefusal::LayerNotTopLevel)
    );
    assert_eq!(
        mv(&d, &[l2], MoveTarget::FrontOf(Some(l))),
        Err(MoveRefusal::LayerNotTopLevel)
    );
    assert_eq!(
        mv(&d, &[l2], MoveTarget::Above(a)),
        Err(MoveRefusal::LayerNotTopLevel)
    );
    // Never into itself or a descendant.
    assert_eq!(
        mv(&d, &[g], MoveTarget::FrontOf(Some(g))),
        Err(MoveRefusal::IntoItself)
    );
    assert_eq!(
        mv(&d, &[g], MoveTarget::Above(a)),
        Err(MoveRefusal::IntoItself)
    );
    // Never into a leaf.
    mv(&d, &[c], MoveTarget::FrontOf(Some(g))).unwrap();
    assert_eq!(
        mv(&d, &[b], MoveTarget::FrontOf(Some(c))),
        Err(MoveRefusal::NotAContainer)
    );
    // Nothing, and stale ids.
    assert_eq!(
        mv(&d, &[], MoveTarget::FrontOf(None)),
        Err(MoveRefusal::Nothing)
    );
    let stale: NodeId = serde_json::from_str("{\"peer\":\"77\",\"counter\":123456}").unwrap();
    assert_eq!(
        mv(&d, &[stale], MoveTarget::FrontOf(None)),
        Err(MoveRefusal::NoSuchNode)
    );
    assert_eq!(
        mv(&d, &[a], MoveTarget::FrontOf(Some(stale))),
        Err(MoveRefusal::NoSuchNode)
    );
    // Locked nodes and locked containers.
    d.set_node_locked(&[l], true).unwrap();
    assert_eq!(
        mv(&d, &[a], MoveTarget::FrontOf(Some(l))),
        Err(MoveRefusal::LockedTarget)
    );
    d.set_node_locked(&[l], false).unwrap();
    d.set_node_locked(&[a], true).unwrap();
    assert_eq!(
        mv(&d, &[a], MoveTarget::FrontOf(Some(l))),
        Err(MoveRefusal::Locked)
    );
    // Locked through a container.
    d.set_node_locked(&[a], false).unwrap();
    d.set_node_locked(&[g], true).unwrap();
    assert_eq!(
        mv(&d, &[a], MoveTarget::FrontOf(None)),
        Err(MoveRefusal::Locked)
    );
    assert_eq!(
        mv(&d, &[b], MoveTarget::FrontOf(Some(l))),
        Err(MoveRefusal::Locked)
    );
    // Nothing was written by any refusal.
    assert_eq!(d.children(Some(g)).len(), 3);
}

#[test]
fn a_refused_move_writes_nothing_even_for_the_nodes_that_were_fine() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    d.set_node_locked(&[b], true).unwrap();
    let before = d.export_json().unwrap();
    let n0 = label_count(&d);
    assert!(mv(&d, &[a, b], MoveTarget::FrontOf(Some(l))).is_err());
    assert_eq!(d.export_json().unwrap(), before);
    assert_eq!(label_count(&d), n0);
}

#[test]
fn a_move_is_one_commit_keeps_ids_and_the_relative_order() {
    let d = Document::new(1);
    let l = d.new_layer(None);
    let ids: Vec<NodeId> = (0..5).map(|i| rect(&d, f64::from(i) * 15.0)).collect();
    let before: Vec<String> = ids
        .iter()
        .map(|&i| format!("{:?}", d.object(i).unwrap()))
        .collect();
    let n0 = label_count(&d);
    mv(&d, &[ids[3], ids[1]], MoveTarget::FrontOf(Some(l))).unwrap();
    assert_eq!(label_count(&d), n0 + 1);
    assert_eq!(last_label(&d), "move_nodes");
    assert_eq!(d.children(Some(l)), [ids[1], ids[3]], "relative order kept");
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(format!("{:?}", d.object(*id).unwrap()), before[i]);
    }
    // BackOf and Below.
    mv(&d, &[ids[0]], MoveTarget::BackOf(Some(l))).unwrap();
    assert_eq!(d.children(Some(l)), [ids[0], ids[1], ids[3]]);
    mv(&d, &[ids[4]], MoveTarget::Below(ids[1])).unwrap();
    assert_eq!(d.children(Some(l)), [ids[0], ids[4], ids[1], ids[3]]);
    // Moving a node to where it is changes nothing.
    let before = d.export_json().unwrap();
    mv(&d, &[ids[0]], MoveTarget::BackOf(Some(l))).unwrap();
    assert_eq!(d.export_json().unwrap(), before);
}

#[test]
fn the_group_depth_limit_holds_for_moves() {
    let d = Document::new(1);
    let mut inner = rect(&d, 0.0);
    let mut deepest = inner;
    for i in 0..32 {
        inner = d
            .group_objects(&[inner, rect(&d, f64::from(i + 1) * 12.0)])
            .unwrap();
        if i == 0 {
            deepest = inner;
        }
    }
    let g = d
        .group_objects(&[rect(&d, 900.0), rect(&d, 920.0)])
        .unwrap();
    assert_eq!(
        mv(&d, &[g], MoveTarget::FrontOf(Some(deepest))),
        Err(MoveRefusal::TooDeep)
    );
}

// ---------------------------------------------------------------------------
// Layers are transparent at the top level (criterion 42)
// ---------------------------------------------------------------------------

#[test]
fn a_node_in_a_layer_has_no_group_parent_but_a_node_parent() {
    let d = Document::new(1);
    let l1 = d.new_layer(None);
    let a = rect(&d, 0.0);
    let _loose = rect(&d, 20.0);
    let l2 = d.new_layer(None);
    let b = rect(&d, 40.0);
    mv(&d, &[a], MoveTarget::FrontOf(Some(l1))).unwrap();
    mv(&d, &[b], MoveTarget::FrontOf(Some(l2))).unwrap();
    // parent_group of a node in a layer is None (the top level), parent_node is the layer.
    assert_eq!(d.parent_group(a), None);
    assert_eq!(d.parent_node(a), Some(l1));
    assert_eq!(d.children(Some(l1)), [a]);
}

// ---------------------------------------------------------------------------
// File format (criteria 46, 47, 54)
// ---------------------------------------------------------------------------

fn manifest(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(&read_member(bytes, "manifest.json")).unwrap()
}

fn rich_document() -> (Document, Vec<NodeId>) {
    let d = Document::new(1);
    let hidden = d.new_layer(None);
    d.rename_node(hidden, "Hidden \u{e9}").unwrap();
    d.set_node_visible(&[hidden], false).unwrap();
    d.set_node_opacity(&[hidden], op(0.35)).unwrap();
    let cut = d.new_layer(None);
    d.rename_node(cut, "Cut").unwrap();
    let a = rect(&d, 0.0);
    let b = rect(&d, 20.0);
    let c = rect(&d, 40.0);
    let g = d.group_objects(&[a, b]).unwrap();
    d.rename_node(g, "Pair").unwrap();
    d.set_node_locked(&[b], true).unwrap();
    mv(&d, &[g], MoveTarget::FrontOf(Some(hidden))).unwrap();
    mv(&d, &[c], MoveTarget::FrontOf(Some(cut))).unwrap();
    d.set_node_opacity(&[c], op(0.6)).unwrap();
    let loose = rect(&d, 60.0);
    d.set_node_visible(&[loose], false).unwrap();
    (d, vec![hidden, cut, a, b, c, g, loose])
}

#[test]
fn ac46_a_project_with_layers_groups_names_and_flags_round_trips_exactly() {
    let (d, ids) = rich_document();
    let bytes = pack(&d, "t").unwrap();
    // 0050 took 12; 0049 took 13 after it, so the writer names the current one.
    assert_eq!(manifest(&bytes)["format_version"], CURRENT_FORMAT_VERSION);
    const _: () = assert!(CURRENT_FORMAT_VERSION >= 12);
    let r = unpack(5, &bytes).unwrap();
    assert_eq!(r.export_json().unwrap(), d.export_json().unwrap());
    assert_eq!(r.children(None), d.children(None));
    for id in ids {
        assert_eq!(r.node_registers(id), d.node_registers(id), "{id:?}");
        assert_eq!(r.is_layer(id), d.is_layer(id));
        assert_eq!(r.parent_node(id), d.parent_node(id));
    }
    let again = pack(&r, "t").unwrap();
    assert_eq!(
        unpack(6, &again).unwrap().export_json().unwrap(),
        d.export_json().unwrap(),
        "byte-stable on a second save"
    );
}

#[test]
fn ac46_a_newer_file_is_refused_with_or_without_layers() {
    for with_layers in [true, false] {
        let d = if with_layers {
            rich_document().0
        } else {
            Document::new(1)
        };
        let bytes = pack(&d, "t").unwrap();
        let mut m = manifest(&bytes);
        m["format_version"] = (CURRENT_FORMAT_VERSION + 1).into();
        let r = unpack(1, &rebuild_zip(&bytes, &m, None));
        assert!(
            matches!(&r, Err(OpenError::FormatTooNew { found, supported })
                if *found == CURRENT_FORMAT_VERSION + 1 && *supported == CURRENT_FORMAT_VERSION),
            "{:?}",
            r.map(|_| ())
        );
    }
}

#[test]
fn ac46_files_of_earlier_builds_open_without_layers_or_flags() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut opened = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if path.extension().is_none_or(|e| e != "curvyo")
            || name.starts_with("layers")
            // `0057-clone-overrides` (format 14) is a later build: its golden stores opacities.
            || name.starts_with("clone_overrides")
            // `0046-document-shapes` (format 15) is a later build: its template sources store names.
            || name.starts_with("template_")
            // `0058-bitmap-import` (format 25) is a later build: its golden stores names and flags.
            || name.starts_with("image_")
            || name.contains("damaged")
            || name.contains("truncated")
            || name.contains("too_deep")
            || name.contains("leaf_child")
        {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        let Ok(doc) = unpack(3, &bytes) else { continue };
        opened += 1;
        for n in doc.read_nodes() {
            assert!(
                !matches!(n.kind, NodeKind::Layer),
                "{name}: layer in an old file"
            );
            assert!(n.registers.visible && !n.registers.locked, "{name}");
            assert_eq!(n.registers.opacity.get(), 1.0, "{name}");
            // These intentionally named format 28 and 29 goldens are not earlier nameless files.
            let named_current_sketch = matches!(
                name.as_str(),
                "sketch_page_owner_v28.curvyo"
                    | "sketch_dimension_label_v28.curvyo"
                    | "sketch_page_owner_v29.curvyo"
                    | "sketch_dimension_label_v29.curvyo"
            ) && matches!(
                n.kind,
                NodeKind::Leaf(curvyo_document_core::ObjectSnapshot::Sketch(_))
            );
            assert_eq!(
                n.registers.name.as_deref(),
                named_current_sketch.then_some("Sketch"),
                "{name}"
            );
        }
        // Opening writes nothing: the file is not rewritten on open (no commit of its own).
        let text = String::from_utf8(doc.export_json().unwrap()).unwrap();
        for key in ["\"visible\"", "\"locked\"", "\"layer\""] {
            assert!(!text.contains(key), "{name} gained {key}");
        }
    }
    assert!(opened >= 5, "only {opened} older fixtures were found");
}

#[test]
fn ac46_the_golden_file_pins_the_format() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let golden = dir.join("layers_v12.curvyo");
    let bytes = std::fs::read(&golden).expect("a golden file layers_v12.curvyo");
    assert_eq!(manifest(&bytes)["format_version"], 12);
    let doc = unpack(1, &bytes).unwrap();
    let nodes = doc.read_nodes();
    assert!(nodes.iter().any(|n| matches!(n.kind, NodeKind::Layer)));
    assert!(nodes.iter().any(|n| !n.registers.visible), "a hidden node");
    assert!(nodes.iter().any(|n| n.registers.locked), "a locked node");
    assert!(
        nodes.iter().any(|n| n.registers.opacity.get() < 1.0),
        "a faded node"
    );
    assert!(
        nodes.iter().any(|n| n.registers.name.is_some()),
        "a named node"
    );
}

fn set_meta(l: &LoroDoc, node: TreeID, key: &str, f: impl FnOnce(&loro::LoroMap, &str)) {
    let meta = tree_of(l).get_meta(node).unwrap();
    f(&meta, key);
}

#[test]
fn ac47_wrongly_typed_or_out_of_range_registers_are_refused_as_damaged() {
    type Setter = Box<dyn Fn(&loro::LoroMap)>;
    let cases: Vec<(&str, Setter)> = vec![
        (
            "opacity string",
            Box::new(|m| m.insert("opacity", "half").unwrap()),
        ),
        (
            "opacity 1.5",
            Box::new(|m| m.insert("opacity", 1.5).unwrap()),
        ),
        (
            "opacity -0.1",
            Box::new(|m| m.insert("opacity", -0.1).unwrap()),
        ),
        (
            "opacity NaN",
            Box::new(|m| m.insert("opacity", f64::NAN).unwrap()),
        ),
        (
            "opacity inf",
            Box::new(|m| m.insert("opacity", f64::INFINITY).unwrap()),
        ),
        (
            "opacity bool",
            Box::new(|m| m.insert("opacity", true).unwrap()),
        ),
        (
            "visible string",
            Box::new(|m| m.insert("visible", "yes").unwrap()),
        ),
        (
            "visible number",
            Box::new(|m| m.insert("visible", 1.0).unwrap()),
        ),
        (
            "locked string",
            Box::new(|m| m.insert("locked", "no").unwrap()),
        ),
        (
            "locked number",
            Box::new(|m| m.insert("locked", 0.0).unwrap()),
        ),
        ("name number", Box::new(|m| m.insert("name", 5.0).unwrap())),
        ("name bool", Box::new(|m| m.insert("name", true).unwrap())),
    ];
    for (what, set) in &cases {
        let (d, l, leaf) = bare();
        set_meta(&l, leaf, "", |m, _| set(m));
        let r = unpack(2, &craft(&d, &l));
        assert!(
            matches!(r, Err(OpenError::Damaged)),
            "{what}: {:?}",
            r.map(|_| ())
        );
    }
    // The same registers on a group and on a layer are checked too.
    let (d, l, _leaf) = bare();
    let t = tree_of(&l);
    let g = t.create(TreeParentId::Root).unwrap();
    t.get_meta(g).unwrap().insert("group", true).unwrap();
    t.get_meta(g).unwrap().insert("layer", true).unwrap();
    t.get_meta(g).unwrap().insert("opacity", 7.0).unwrap();
    let r = unpack(2, &craft(&d, &l));
    assert!(matches!(r, Err(OpenError::Damaged)), "{:?}", r.map(|_| ()));
    // Boundary values are fine.
    for v in [0.0, 1.0, 0.5] {
        let (d, l, leaf) = bare();
        set_meta(&l, leaf, "", |m, _| m.insert("opacity", v).unwrap());
        let doc = unpack(2, &craft(&d, &l)).unwrap();
        assert_eq!(
            doc.node_registers(doc.object_ids()[0])
                .unwrap()
                .opacity
                .get(),
            v
        );
    }
}

#[test]
fn ac47_a_layer_marker_below_the_top_level_reads_as_a_plain_group() {
    let (d, l, leaf) = bare();
    let t = tree_of(&l);
    let outer = t.create(TreeParentId::Root).unwrap();
    t.get_meta(outer).unwrap().insert("group", true).unwrap();
    let inner = t.create(TreeParentId::Node(outer)).unwrap();
    t.get_meta(inner).unwrap().insert("group", true).unwrap();
    t.get_meta(inner).unwrap().insert("layer", true).unwrap();
    t.mov(leaf, TreeParentId::Node(inner)).unwrap();
    let doc = unpack(2, &craft(&d, &l)).unwrap();
    let nodes = doc.read_nodes();
    assert!(
        !nodes.iter().any(|n| matches!(n.kind, NodeKind::Layer)),
        "{nodes:?}"
    );
    assert_eq!(
        nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Group))
            .count(),
        2
    );
}

#[test]
fn ac47_a_layer_marker_on_a_leaf_is_refused() {
    let (d, l, leaf) = bare();
    set_meta(&l, leaf, "", |m, _| m.insert("layer", true).unwrap());
    let r = unpack(2, &craft(&d, &l));
    assert!(r.is_err(), "a layer marker on a node that is not a group");
}

#[test]
fn ac47_ten_thousand_layers_open_without_a_hang() {
    let (d, l, _leaf) = bare();
    let t = tree_of(&l);
    for i in 0..10_000 {
        let g = t.create(TreeParentId::Root).unwrap();
        let m = t.get_meta(g).unwrap();
        m.insert("group", true).unwrap();
        m.insert("layer", true).unwrap();
        m.insert("name", format!("Layer {i}")).unwrap();
    }
    let bytes = craft(&d, &l);
    let t0 = Instant::now();
    let doc = unpack(2, &bytes).unwrap();
    let nodes = doc.read_nodes();
    assert!(
        t0.elapsed().as_secs() < 10,
        "open + read took {:?}",
        t0.elapsed()
    );
    assert_eq!(
        nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Layer))
            .count(),
        10_000
    );
}

#[test]
fn ac47_a_tree_nested_deeper_than_32_is_refused_with_layers_around_it_too() {
    let (d, l, leaf) = bare();
    let t = tree_of(&l);
    let layer = t.create(TreeParentId::Root).unwrap();
    t.get_meta(layer).unwrap().insert("group", true).unwrap();
    t.get_meta(layer).unwrap().insert("layer", true).unwrap();
    let mut parent = TreeParentId::Node(layer);
    let mut last = layer;
    for _ in 0..33 {
        let g = t.create(parent).unwrap();
        t.get_meta(g).unwrap().insert("group", true).unwrap();
        parent = TreeParentId::Node(g);
        last = g;
    }
    t.mov(leaf, TreeParentId::Node(last)).unwrap();
    let r = unpack(2, &craft(&d, &l));
    assert!(matches!(r, Err(OpenError::Damaged)), "{:?}", r.map(|_| ()));
}

#[test]
fn ac17_names_read_from_a_file_are_shown_cleaned_and_cut_to_128_characters() {
    let (d, l, leaf) = bare();
    let long = "x".repeat(300);
    set_meta(&l, leaf, "", |m, _| {
        m.insert("name", long.as_str()).unwrap();
    });
    let doc = unpack(2, &craft(&d, &l)).unwrap();
    let name = doc
        .node_registers(doc.object_ids()[0])
        .unwrap()
        .name
        .unwrap();
    assert_eq!(name.chars().count(), 128);
    let (d, l, leaf) = bare();
    set_meta(&l, leaf, "", |m, _| {
        m.insert("name", "a\u{202E}b\u{2067}c").unwrap();
    });
    let doc = unpack(2, &craft(&d, &l)).unwrap();
    assert_eq!(
        doc.node_registers(doc.object_ids()[0])
            .unwrap()
            .name
            .as_deref(),
        Some("abc"),
        "bidirectional controls are dropped when read"
    );
}

#[test]
fn ac54_save_and_document_json_carry_hidden_and_locked_nodes_with_their_flags() {
    let d = Document::new(1);
    let hidden_layer = d.new_layer(None);
    d.rename_node(hidden_layer, "Hid").unwrap();
    let kid = rect(&d, 0.0);
    let locked_leaf = rect(&d, 20.0);
    mv(&d, &[kid], MoveTarget::FrontOf(Some(hidden_layer))).unwrap();
    d.set_node_visible(&[hidden_layer], false).unwrap();
    d.set_node_locked(&[locked_leaf], true).unwrap();
    // document.json (Document::export_json).
    let v = json(&d);
    let text = v.to_string();
    assert!(text.contains("\"layer\""), "{text}");
    assert!(text.contains("\"Hid\""));
    let mut found_hidden_layer = false;
    let mut found_locked_leaf = false;
    fn walk(v: &serde_json::Value, hid: &mut bool, lock: &mut bool) {
        match v {
            serde_json::Value::Object(m) => {
                if m.get("shape").and_then(|s| s.as_str()) == Some("layer")
                    && m.get("visible") == Some(&serde_json::Value::Bool(false))
                {
                    *hid = true;
                }
                if m.get("locked") == Some(&serde_json::Value::Bool(true)) {
                    *lock = true;
                }
                m.values().for_each(|c| walk(c, hid, lock));
            }
            serde_json::Value::Array(a) => a.iter().for_each(|c| walk(c, hid, lock)),
            _ => {}
        }
    }
    walk(&v, &mut found_hidden_layer, &mut found_locked_leaf);
    assert!(
        found_hidden_layer,
        "the hidden layer with visible=false: {text}"
    );
    assert!(
        found_locked_leaf,
        "the locked leaf with locked=true: {text}"
    );
    // The project save (Document::export_loro_snapshot) holds both too: reopened, the flags are there.
    let snap = d.export_loro_snapshot().unwrap();
    let l = LoroDoc::new();
    l.import(&snap).unwrap();
    let reopened = unpack(3, &pack(&d, "t").unwrap()).unwrap();
    assert!(!reopened.node_registers(hidden_layer).unwrap().visible);
    assert!(reopened.node_registers(locked_leaf).unwrap().locked);
    assert_eq!(
        reopened.object_ids().len(),
        2,
        "both leaves are in the save"
    );
}

#[test]
fn undo_free_replica_sync_keeps_the_flags() {
    // Two peers: a flag set on one replica reaches the other through the snapshot exchange.
    let (d, ids) = rich_document();
    let b = unpack(9, &pack(&d, "t").unwrap()).unwrap();
    b.set_node_visible(&[ids[1]], false).unwrap();
    let updates = b.export_updates(&d.version()).unwrap();
    d.import_updates(&updates).unwrap();
    assert!(!d.node_registers(ids[1]).unwrap().visible);
}
