//! Undo, redo and object undo with linked clones (`specs/0049-linked-clone` criterion 36,
//! `adrs.md` decision 9), merges between peers (criterion 13, decision 7) and the no-cycle
//! property (criterion 53).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::cast_precision_loss
)]

use curvyo_document_core::{
    Affine, AnchorId, CloneSnapshot, CopySource, Document, NewAnchor, NodeId, ObjectShape,
    ObjectSnapshot, Point, Vec2, pack, unpack,
};

fn square(d: &Document, n: u64, at: Point) -> NodeId {
    let corner = |k: u64, dx: f64, dy: f64| {
        NewAnchor::corner(
            AnchorId::new(1, n * 10 + k),
            Point::new(at.x + dx, at.y + dy),
        )
    };
    d.create_path(
        &[
            corner(0, 0.0, 0.0),
            corner(1, 10.0, 0.0),
            corner(2, 10.0, 10.0),
            corner(3, 0.0, 10.0),
        ],
        true,
    )
}

fn link(d: &Document, original: NodeId) -> NodeId {
    d.linked_clone(&[original], &[]).unwrap()[0]
}

fn clone_of(d: &Document, id: NodeId) -> CloneSnapshot {
    match d.object(id) {
        Some(ObjectSnapshot::Clone(clone)) => clone,
        other => panic!("not a clone: {other:?}"),
    }
}

fn undo_last(d: &Document) {
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
}

fn json(d: &Document) -> serde_json::Value {
    serde_json::from_slice(&d.export_json().unwrap()).unwrap()
}

#[test]
fn ac36_undo_of_unlink_makes_a_clone_again_field_by_field() {
    let d = Document::new(1);
    let p = square(&d, 1, Point::new(0.0, 0.0));
    let c = link(&d, p);
    d.translate_objects(&[c], Vec2::new(30.0, 0.0)).unwrap();
    let before = json(&d);
    let count = clone_of(&d, c).resolved.all_anchors().count();
    d.unlink_clones(&[CopySource {
        id: c,
        anchor_ids: (0..count as u64).map(|k| AnchorId::new(9, k)).collect(),
    }])
    .unwrap();
    assert!(matches!(d.object(c), Some(ObjectSnapshot::Path(_))));
    undo_last(&d);
    assert_eq!(json(&d), before);
    assert_eq!(
        clone_of(&d, c).chain.matrix,
        Affine::translation(Vec2::new(30.0, 0.0))
    );
}

#[test]
fn ac36_undo_of_a_delete_that_unlinked_clones_restores_root_and_links() {
    let d = Document::new(1);
    let p = square(&d, 1, Point::new(0.0, 0.0));
    let c1 = link(&d, p);
    let c2 = link(&d, p);
    d.translate_objects(&[c2], Vec2::new(0.0, 20.0)).unwrap();
    let before = json(&d);
    d.delete_objects(&[p]).unwrap();
    assert!(matches!(d.object(c1), Some(ObjectSnapshot::Path(_))));
    undo_last(&d);
    assert_eq!(json(&d), before);
    assert_eq!(clone_of(&d, c1).chain.origin, p);
    assert_eq!(clone_of(&d, c2).chain.origin, p);
}

#[test]
fn ac36_54_undo_of_a_delete_that_repointed_gives_the_old_link_back() {
    let d = Document::new(1);
    let o = square(&d, 1, Point::new(0.0, 0.0));
    let a = link(&d, o);
    d.translate_objects(&[a], Vec2::new(30.0, 0.0)).unwrap();
    let b = link(&d, a);
    let before = json(&d);
    d.delete_objects(&[a]).unwrap();
    assert_eq!(clone_of(&d, b).chain.origin, o);
    undo_last(&d);
    assert_eq!(json(&d), before);
    assert_eq!(clone_of(&d, b).chain.origin, a);
    assert_eq!(clone_of(&d, b).chain.matrix, Affine::IDENTITY);
}

#[test]
fn ac36b_undo_of_linked_clone_with_a_clone_behind_it_repairs_that_clone() {
    let d = Document::new(1);
    let o = square(&d, 1, Point::new(0.0, 0.0));
    let a = link(&d, o);
    let a_step = d.own_steps().last().unwrap().step;
    let b = link(&d, a);
    d.translate_objects(&[b], Vec2::new(30.0, 0.0)).unwrap();
    let drawn_before = clone_of(&d, b).resolved;
    // Taking back the Linked clone that made A removes A: B is re-pointed to O, never orphaned.
    d.undo_step(a_step).unwrap();
    assert!(d.object(a).is_none(), "the undo removed A");
    assert_eq!(d.read_objects().orphans, Vec::<NodeId>::new());
    let after = d.object(b).unwrap();
    let ObjectSnapshot::Clone(after) = after else {
        panic!("B stays a clone: {after:?}")
    };
    assert_eq!(after.chain.origin, o);
    for (x, y) in after.resolved.anchors.iter().zip(&drawn_before.anchors) {
        assert!((x.point.x - y.point.x).abs() < 1e-9 && (x.point.y - y.point.y).abs() < 1e-9);
    }
}

#[test]
fn ac36b_undo_of_creating_a_root_unlinks_its_clones() {
    let d = Document::new(1);
    let o = square(&d, 1, Point::new(0.0, 0.0));
    let created = d.own_steps().last().unwrap().step;
    let c = link(&d, o);
    d.undo_step(created).unwrap();
    assert!(matches!(d.object(c), Some(ObjectSnapshot::Path(_))));
    assert_eq!(d.read_objects().orphans, Vec::<NodeId>::new());
}

#[test]
fn ac36a_object_undo_of_a_delete_keeps_a_clone_on_its_new_original_when_its_link_cannot_return() {
    let d = Document::new(1);
    let o = square(&d, 1, Point::new(0.0, 0.0));
    let a = link(&d, o);
    let b = link(&d, a);
    d.delete_objects(&[a]).unwrap();
    assert_eq!(clone_of(&d, b).chain.origin, o);
    d.translate_objects(&[b], Vec2::new(5.0, 0.0)).unwrap();
    // Every way back leaves B a valid clone: never an orphan.
    let _ = d.undo_object_step(a);
    assert_eq!(d.read_objects().orphans, Vec::<NodeId>::new());
    assert!(matches!(d.object(b), Some(ObjectSnapshot::Clone(_))));
}

/// Criterion 36 (a), reduced from the `ac53` seed of run 590: the link an undo of Unlink would put
/// back names an original a peer deleted meanwhile, so the node is left out of the restore whole.
/// It stays the path it is, never a clone tag without a link, and the peer merges the undo.
#[test]
fn ac36a_undo_of_an_unlink_whose_original_a_peer_deleted_leaves_the_path() {
    let p = Document::new(1);
    let o = square(&p, 1, Point::new(0.0, 0.0));
    square(&p, 2, Point::new(20.0, 0.0));
    let q = peers(&p);
    let start = p.version();
    let c = link(&p, o);
    p.unlink_clones(&[CopySource {
        id: c,
        anchor_ids: (0..4).map(|k| AnchorId::new(9, k)).collect(),
    }])
    .unwrap();
    q.delete_objects(&[o]).unwrap();
    let from_p = p.export_updates(&start).unwrap();
    merge(&p, &q, &start);
    q.import_updates(&from_p).unwrap();
    let path_before = json(&p);
    let since = p.version();
    let report = p.undo_step(p.own_steps().last().unwrap().step).unwrap();
    assert_eq!(
        report.objects[0].result,
        curvyo_document_core::ObjectResult::Skipped(curvyo_document_core::SkipReason::ChangedLater)
    );
    assert!(matches!(p.object(c), Some(ObjectSnapshot::Path(_))));
    assert_eq!(json(&p), path_before);
    q.import_updates(&p.export_updates(&since).unwrap())
        .unwrap();
    assert_eq!(json(&p), json(&q));
}

#[test]
fn ac36_the_history_names_a_clone_as_a_clone() {
    let d = Document::new(1);
    let o = square(&d, 1, Point::new(0.0, 0.0));
    let c = link(&d, o);
    d.translate_objects(&[c], Vec2::new(1.0, 0.0)).unwrap();
    let step = d.own_steps().last().unwrap().step;
    let report = d.undo_step(step).unwrap();
    assert!(
        report
            .objects
            .iter()
            .any(|r| r.id == c && r.shape == ObjectShape::Clone)
    );
}

fn peers(d: &Document) -> Document {
    unpack(2, &pack(d, "t").unwrap()).unwrap()
}

fn merge(into: &Document, from: &Document, since: &curvyo_document_core::DocumentVersion) {
    into.import_updates(&from.export_updates(since).unwrap())
        .unwrap();
}

#[test]
fn ac13_an_edit_of_the_root_and_a_transform_of_the_clone_both_apply() {
    let p = Document::new(1);
    let o = square(&p, 1, Point::new(0.0, 0.0));
    let c = link(&p, o);
    let q = peers(&p);
    let start = p.version();
    p.translate_objects(&[o], Vec2::new(0.0, 5.0)).unwrap();
    q.translate_objects(&[c], Vec2::new(30.0, 0.0)).unwrap();
    let from_p = p.export_updates(&start).unwrap();
    merge(&p, &q, &start);
    q.import_updates(&from_p).unwrap();
    assert_eq!(json(&p), json(&q));
    let merged = clone_of(&p, c);
    assert_eq!(merged.resolved.anchors[0].point, Point::new(30.0, 5.0));
}

#[test]
fn ac13_a_repoint_and_a_transform_of_one_clone_merge_into_one_whole_link() {
    let p = Document::new(1);
    let o = square(&p, 1, Point::new(0.0, 0.0));
    let a = link(&p, o);
    p.translate_objects(&[a], Vec2::new(30.0, 0.0)).unwrap();
    let b = link(&p, a);
    let q = peers(&p);
    let start = p.version();
    p.delete_objects(&[a]).unwrap();
    q.translate_objects(&[b], Vec2::new(0.0, 7.0)).unwrap();
    let from_p = p.export_updates(&start).unwrap();
    merge(&p, &q, &start);
    q.import_updates(&from_p).unwrap();
    assert_eq!(json(&p), json(&q), "replicas converge");
    let read = p.read_objects();
    let link_of_b = read.objects.iter().find_map(|o| match o {
        ObjectSnapshot::Clone(c) if c.id() == b => Some((c.chain.origin, c.chain.matrix)),
        _ => None,
    });
    match link_of_b {
        // P's link won: B follows O with M_B·M_A, Q's move is lost.
        Some((origin, matrix)) => {
            assert_eq!(origin, o);
            assert_eq!(matrix, Affine::translation(Vec2::new(30.0, 0.0)));
        }
        // Q's link won: B follows the deleted A and is an orphan.
        None => assert_eq!(read.orphans, vec![b]),
    }
}

#[test]
fn ac35_a_clone_of_a_node_a_peer_deletes_is_an_orphan_both_repair_the_same() {
    let p = Document::new(1);
    let o = square(&p, 1, Point::new(0.0, 0.0));
    let q = peers(&p);
    let start = p.version();
    let c = link(&p, o);
    q.delete_objects(&[o]).unwrap();
    let from_p = p.export_updates(&start).unwrap();
    merge(&p, &q, &start);
    q.import_updates(&from_p).unwrap();
    assert_eq!(p.read_objects().orphans, vec![c]);
    let (vp, vq) = (p.version(), q.version());
    assert_eq!(p.remove_orphan_clones(), 1);
    assert_eq!(q.remove_orphan_clones(), 1);
    merge(&p, &q, &vq);
    q.import_updates(&p.export_updates(&vp).unwrap()).unwrap();
    assert_eq!(json(&p), json(&q));
    assert_eq!(p.object_ids(), Vec::<NodeId>::new());
}

/// One peer unlinks the clone `c` of a square while the other unlinks it and takes that back,
/// after `pad_a` and `pad_b` moves of the square that shift the merge order key by key. Returns
/// both replicas with each other's updates and `c`.
fn concurrent_unlinks(pad_a: u64, pad_b: u64) -> (Document, Document, NodeId) {
    let a = Document::new(1);
    let o = square(&a, 1, Point::new(0.0, 0.0));
    let c = link(&a, o);
    let b = peers(&a);
    let start = a.version();
    for _ in 0..pad_a {
        a.translate_objects(&[o], Vec2::new(1.0, 0.0)).unwrap();
    }
    for _ in 0..pad_b {
        b.translate_objects(&[o], Vec2::new(0.0, 1.0)).unwrap();
    }
    let unlink = |d: &Document, peer: u64| {
        d.unlink_clones(&[CopySource {
            id: c,
            anchor_ids: (0..4).map(|k| AnchorId::new(peer, 100 + k)).collect(),
        }])
        .unwrap();
    };
    unlink(&a, 7);
    unlink(&b, 8);
    undo_last(&b);
    sync(&a, &b, &start);
    (a, b, c)
}

/// Tester review F1, the merge case (`adrs.md` decision 1, amended 2026-10-14): key by key, an
/// Unlink against an Unlink and its undo can merge into a clone tag with the outline but no link.
/// Every timing merges, both replicas repair to the same state, and the node is kept as a path or
/// a clone, never deleted.
#[test]
fn ac53_unlink_against_unlink_and_undo_merges_for_every_timing() {
    let mut linkless = 0;
    for pad_a in 0..12 {
        for pad_b in 0..12 {
            let (a, b, c) = concurrent_unlinks(pad_a, pad_b);
            if !a.read_objects().orphans.is_empty() {
                linkless += 1;
            }
            let (va, vb) = (a.version(), b.version());
            assert_eq!(a.remove_orphan_clones(), 0);
            assert_eq!(b.remove_orphan_clones(), 0);
            a.import_updates(&b.export_updates(&vb).unwrap()).unwrap();
            b.import_updates(&a.export_updates(&va).unwrap()).unwrap();
            assert_eq!(json(&a), json(&b), "replicas converge");
            assert_eq!(a.read_objects().orphans, Vec::<NodeId>::new());
            assert!(matches!(
                a.object(c),
                Some(ObjectSnapshot::Path(_) | ObjectSnapshot::Clone(_))
            ));
        }
    }
    assert!(
        linkless > 0,
        "some timing leaves the clone tag without a link"
    );
}

/// A small deterministic generator, so the property test needs no dependency.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, below: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % below.max(1)
    }
}

/// Criterion 53: random sequences of the commands (with New layer, moves into a layer and Delete
/// layer of `0050`) on two replicas with merges never form a cycle, every chain ends in a path, and Linked clone never builds deeper than 8.
#[test]
fn ac53_no_sequence_of_commands_forms_a_cycle() {
    let runs = if cfg!(debug_assertions) { 60 } else { 1_000 };
    let mut rng = Lcg(0x0049);
    let mut next_anchor = 10_000;
    for _ in 0..runs {
        one_sequence(&mut rng, &mut next_anchor);
    }
}

/// The sequence the release run of `ac53` failed on (run 590, tester review F1): replica A
/// unlinks a clone whose original B deletes at the same time, and after the merge A undoes the
/// Unlink. The undo used to write the clone tag without a link, a node B refused to merge.
#[test]
fn ac53_the_sequence_of_run_590_merges() {
    one_sequence(&mut Lcg(0xfbdc_638b_e4fe_c5bd), &mut 24_400);
}

/// One random sequence of 30 rounds on two replicas, with merges and orphan repairs between them.
fn one_sequence(rng: &mut Lcg, next_anchor: &mut u64) {
    let a = Document::new(1);
    for k in 0..4 {
        square(&a, k + 1, Point::new(k as f64 * 20.0, 0.0));
    }
    let b = peers(&a);
    let mut last = a.version();
    for _ in 0..30 {
        for replica in [&a, &b] {
            act(replica, rng, next_anchor);
        }
        if rng.next(4) == 0 {
            sync(&a, &b, &last);
            let (va, vb) = (a.version(), b.version());
            a.remove_orphan_clones();
            b.remove_orphan_clones();
            a.import_updates(&b.export_updates(&vb).unwrap()).unwrap();
            b.import_updates(&a.export_updates(&va).unwrap()).unwrap();
            assert_eq!(json(&a), json(&b), "replicas converge");
            last = a.version();
        }
        for replica in [&a, &b] {
            check_no_cycle(replica);
        }
    }
}

/// Both replicas take what the other did since their last common version.
fn sync(a: &Document, b: &Document, last: &curvyo_document_core::DocumentVersion) {
    let from_a = a.export_updates(last).unwrap();
    let from_b = b.export_updates(last).unwrap();
    a.import_updates(&from_b).unwrap();
    b.import_updates(&from_a).unwrap();
}

fn act(d: &Document, rng: &mut Lcg, next_anchor: &mut u64) {
    let read = d.read_objects();
    if read.objects.is_empty() {
        return;
    }
    let pick = read.objects[rng.next(read.objects.len())].clone();
    let id = pick.id();
    let mut fresh = |n: usize| -> Vec<AnchorId> {
        let ids = (0..n as u64)
            .map(|k| AnchorId::new(77, *next_anchor + k))
            .collect();
        *next_anchor += n as u64;
        ids
    };
    match rng.next(10) {
        0 | 1 => {
            let _ = d.linked_clone(&[id], &[]);
        }
        2 => {
            let count = match &pick {
                ObjectSnapshot::Path(path) => path.all_anchors().count(),
                _ => 0,
            };
            let _ = d.duplicate_objects(
                &[CopySource {
                    id,
                    anchor_ids: fresh(count),
                }],
                Vec2::new(1.0, 0.0),
            );
        }
        3 => {
            if let ObjectSnapshot::Clone(clone) = &pick {
                let ids = fresh(clone.resolved.all_anchors().count());
                let _ = d.unlink_clones(&[CopySource {
                    id,
                    anchor_ids: ids,
                }]);
            }
        }
        4 => {
            if read.objects.len() > 2 {
                let _ = d.delete_objects(&[id]);
            }
        }
        5 => {
            let _ = d.translate_objects(&[id], Vec2::new(2.0, 1.0));
        }
        // The commands of `0050`: New layer, a move into a layer, Delete layer.
        6 => {
            let _ = d.new_layer(None);
        }
        7 => {
            let layers: Vec<NodeId> = d
                .children(None)
                .into_iter()
                .filter(|&node| d.is_layer(node))
                .collect();
            if let Some(&layer) = layers.first() {
                let _ = d.move_nodes(&curvyo_document_core::MovePlan {
                    nodes: vec![id],
                    target: curvyo_document_core::MoveTarget::FrontOf(Some(layer)),
                });
            }
        }
        8 => {
            if let Some(layer) = d.children(None).into_iter().find(|&node| d.is_layer(node)) {
                let _ = d.delete_layer(layer);
            }
        }
        _ => {
            if let Some(step) = d.own_steps().last().map(|s| s.step) {
                let _ = d.undo_step(step);
            }
        }
    }
}

fn check_no_cycle(d: &Document) {
    let read = d.read_objects();
    for object in read.objects.iter() {
        if let ObjectSnapshot::Clone(clone) = object {
            assert!(
                matches!(d.object(clone.chain.root), Some(ObjectSnapshot::Path(_))),
                "every chain ends in a path"
            );
        }
    }
    // The stored links, orphans included, never form a ring.
    let mut links = std::collections::HashMap::new();
    collect_links(&json(d)["objects"], &mut links);
    for start in links.keys() {
        let mut seen = std::collections::HashSet::new();
        let mut current = start.clone();
        while let Some(next) = links.get(&current) {
            assert!(seen.insert(current.clone()), "a ring through {current}");
            current = next.clone();
        }
    }
}

fn collect_links(
    objects: &serde_json::Value,
    links: &mut std::collections::HashMap<String, String>,
) {
    for object in objects.as_array().into_iter().flatten() {
        if object["shape"] == "clone" {
            links.insert(object["id"].to_string(), object["origin"].to_string());
        }
        collect_links(&object["objects"], links);
    }
}

/// `0050` criterion 45 with `0049` criterion 36: undo of Delete layer gives the clones outside the
/// layer their links back.
#[test]
fn undo_of_delete_layer_restores_the_clones_that_followed_its_content() {
    let d = Document::new(1);
    let layer = d.new_layer(None);
    let o = square(&d, 1, Point::new(0.0, 0.0));
    d.move_nodes(&curvyo_document_core::MovePlan {
        nodes: vec![o],
        target: curvyo_document_core::MoveTarget::FrontOf(Some(layer)),
    })
    .unwrap();
    let a = link(&d, o);
    d.move_nodes(&curvyo_document_core::MovePlan {
        nodes: vec![a],
        target: curvyo_document_core::MoveTarget::FrontOf(None),
    })
    .unwrap();
    let b = link(&d, a);
    let before = json(&d);
    d.delete_layer(layer).unwrap();
    assert!(
        matches!(d.object(a), Some(ObjectSnapshot::Path(_))),
        "A became a path"
    );
    assert_eq!(clone_of(&d, b).chain.origin, a, "B follows the path A");
    undo_last(&d);
    assert_eq!(json(&d), before);
    assert_eq!(clone_of(&d, a).chain.origin, o);
}
