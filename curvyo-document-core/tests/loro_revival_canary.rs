//! Canary for ADR 0014 §11: undo of Delete brings a deleted tree node back with its old id by
//! injecting one tree `Move` change through the JSON update import. The Loro handler API refuses
//! that move; the CRDT accepts it. This test fails when a Loro upgrade stops accepting the change
//! or stops keeping the node's containers. A failing canary means: stay on the pinned version
//! (`loro = "=1.16.2"` in the root `Cargo.toml`) and bring soft delete (ADR 0014 Q7 B) back to
//! the customer.
//!
//! It deliberately uses Loro directly and not `curvyo-document-core`'s own reviver, so that it
//! stays an independent check of the library behaviour.

use loro::{
    Container, ContainerID, ContainerTrait, ContainerType, ExportMode, FractionalIndex, JsonChange,
    JsonOp, JsonOpContent, JsonTreeOp, LoroDoc, LoroMap, LoroMovableList, TreeID, TreeParentId,
    ValueOrContainer,
};

const TREE: &str = "paths";

/// Builds a document shaped like Curvyo's: one tree, a node whose meta map holds a register and an
/// `anchors` movable list of maps. Returns the node ids in z-order.
fn document(peer: u64, nodes: usize) -> (LoroDoc, Vec<TreeID>) {
    let doc = LoroDoc::new();
    doc.set_peer_id(peer).unwrap();
    let tree = doc.get_tree(TREE);
    tree.enable_fractional_index(0);
    let mut ids = Vec::new();
    for n in 0..nodes {
        let id = tree.create(TreeParentId::Root).unwrap();
        let meta = tree.get_meta(id).unwrap();
        meta.insert("x", n as f64).unwrap();
        let anchors = meta
            .insert_container("anchors", LoroMovableList::new())
            .unwrap();
        let anchor = anchors.push_container(LoroMap::new()).unwrap();
        anchor.insert("id", format!("a{n}")).unwrap();
        anchor.insert("point", vec![1.0_f64, 2.0]).unwrap();
        ids.push(id);
    }
    doc.commit_with(loro::CommitOptions::new().commit_msg("create;s=1;t=1"));
    (doc, ids)
}

/// The injected change of ADR 0014 §11.
fn revive(doc: &LoroDoc, target: TreeID, index: FractionalIndex, message: &str) {
    doc.commit();
    let peer = doc.peer_id();
    let vv = doc.oplog_vv();
    let next = vv.get(&peer).copied().unwrap_or(0);
    let frontiers = doc.oplog_frontiers();
    let lamport = frontiers
        .iter()
        .filter_map(|id| doc.get_change(id))
        .map(|c| c.lamport + c.len as u32)
        .max()
        .unwrap_or(0);
    let mut schema = doc.export_json_updates_without_peer_compression(&vv, &vv);
    schema.changes.push(JsonChange {
        id: loro::ID::new(peer, next),
        timestamp: 0,
        deps: frontiers.iter().collect(),
        lamport,
        msg: Some(message.to_owned()),
        ops: vec![JsonOp {
            content: JsonOpContent::Tree(JsonTreeOp::Move {
                target,
                parent: None,
                fractional_index: index,
            }),
            container: ContainerID::new_root(TREE, ContainerType::Tree),
            counter: next,
        }],
    });
    let status = doc
        .import_json_updates(schema)
        .expect("Loro refused the injected tree move");
    assert!(
        status.pending.is_none(),
        "the injected tree move was left pending"
    );
}

fn index_of(doc: &LoroDoc, id: TreeID) -> FractionalIndex {
    FractionalIndex::from_hex_string(doc.get_tree(TREE).fractional_index(id).unwrap())
}

fn anchors_of(doc: &LoroDoc, id: TreeID) -> LoroMovableList {
    let meta = doc.get_tree(TREE).get_meta(id).unwrap();
    match meta.get("anchors") {
        Some(ValueOrContainer::Container(Container::MovableList(list))) => list,
        _ => panic!("the node lost its anchors list"),
    }
}

fn alive(doc: &LoroDoc, id: TreeID) -> bool {
    let tree = doc.get_tree(TREE);
    tree.contains(id) && matches!(tree.is_node_deleted(&id), Ok(false))
}

#[test]
fn the_injected_move_revives_the_same_node_with_its_containers_and_z_order() {
    let (doc, ids) = document(7, 3);
    let victim = ids[1];
    let tree = doc.get_tree(TREE);
    let old_index = index_of(&doc, victim);
    let meta_id = tree.get_meta(victim).unwrap().id();
    let anchors_id = anchors_of(&doc, victim).id();
    let value_before = tree.get_meta(victim).unwrap().get_deep_value();

    tree.delete(victim).unwrap();
    doc.commit_with(loro::CommitOptions::new().commit_msg("delete;s=2;t=2"));
    assert!(!alive(&doc, victim));
    assert_eq!(doc.get_tree(TREE).roots(), vec![ids[0], ids[2]]);

    revive(&doc, victim, old_index, "undo;s=3;t=3;u=7@0");

    assert!(alive(&doc, victim), "the node is live again");
    assert_eq!(doc.get_tree(TREE).roots(), ids, "z-order is restored");
    let tree = doc.get_tree(TREE);
    assert_eq!(tree.get_meta(victim).unwrap().id(), meta_id);
    assert_eq!(anchors_of(&doc, victim).id(), anchors_id);
    assert_eq!(
        tree.get_meta(victim).unwrap().get_deep_value(),
        value_before
    );

    // The revived node takes ordinary edits.
    tree.get_meta(victim).unwrap().insert("x", 99.0).unwrap();
    doc.commit_with(loro::CommitOptions::new().commit_msg("edit;s=4;t=4"));

    // Save and load keep it live, with the same containers.
    let bytes = doc.export(ExportMode::Snapshot).unwrap();
    let reopened = LoroDoc::new();
    reopened.import(&bytes).unwrap();
    assert!(alive(&reopened, victim));
    assert_eq!(anchors_of(&reopened, victim).id(), anchors_id);
    assert_eq!(
        reopened
            .get_tree(TREE)
            .get_meta(victim)
            .unwrap()
            .get("x")
            .map(|v| v.get_deep_value()),
        Some(99.0.into())
    );

    // The next local counter continues after the injected change.
    let message = doc
        .get_change(loro::ID::new(
            7,
            doc.oplog_vv().get(&7).copied().unwrap() - 1,
        ))
        .unwrap()
        .message;
    assert_eq!(message.as_deref(), Some("edit;s=4;t=4"));
}

#[test]
fn a_peer_edit_made_while_the_node_was_deleted_survives_the_revival() {
    let (a, ids) = document(7, 2);
    let victim = ids[0];
    let b = LoroDoc::new();
    b.set_peer_id(8).unwrap();
    b.import(&a.export(ExportMode::all_updates()).unwrap())
        .unwrap();

    // A deletes the node; B, who did not see the delete, edits the node and its first anchor.
    let old_index = index_of(&a, victim);
    a.get_tree(TREE).delete(victim).unwrap();
    a.commit_with(loro::CommitOptions::new().commit_msg("delete;s=2;t=2"));
    b.get_tree(TREE)
        .get_meta(victim)
        .unwrap()
        .insert("x", 50.0)
        .unwrap();
    let anchor = match anchors_of(&b, victim).get(0) {
        Some(ValueOrContainer::Container(Container::Map(map))) => map,
        _ => panic!("no anchor map"),
    };
    anchor.insert("point", vec![5.0_f64, 6.0]).unwrap();
    b.commit();

    // The delete wins on A after the merge; the peer's edits sit in the tombstone.
    a.import(&b.export(ExportMode::all_updates()).unwrap())
        .unwrap();
    assert!(!alive(&a, victim));

    revive(&a, victim, old_index, "undo;s=3;t=3;u=7@0");
    assert!(alive(&a, victim));
    let meta = a.get_tree(TREE).get_meta(victim).unwrap();
    assert_eq!(meta.get("x").map(|v| v.get_deep_value()), Some(50.0.into()));
    let anchor = match anchors_of(&a, victim).get(0) {
        Some(ValueOrContainer::Container(Container::Map(map))) => map,
        _ => panic!("no anchor map"),
    };
    assert_eq!(
        anchor.get("point").map(|v| v.get_deep_value()),
        Some(vec![5.0_f64, 6.0].into())
    );
}

#[test]
fn the_revival_still_imports_after_a_peers_newer_changes_arrived() {
    let (a, ids) = document(7, 2);
    let victim = ids[1];
    let b = LoroDoc::new();
    b.set_peer_id(8).unwrap();
    b.import(&a.export(ExportMode::all_updates()).unwrap())
        .unwrap();
    let old_index = index_of(&a, victim);
    a.get_tree(TREE).delete(victim).unwrap();
    a.commit_with(loro::CommitOptions::new().commit_msg("delete;s=2;t=2"));

    // B makes many later changes (a high Lamport clock) and A merges them before the revival.
    for n in 0..50 {
        b.get_tree(TREE)
            .get_meta(ids[0])
            .unwrap()
            .insert("x", f64::from(n))
            .unwrap();
        b.commit_with(loro::CommitOptions::new().commit_msg(&format!("edit;s={n};t=1")));
    }
    a.import(&b.export(ExportMode::all_updates()).unwrap())
        .unwrap();

    revive(&a, victim, old_index, "undo;s=3;t=3;u=7@0");
    assert!(alive(&a, victim));
    // The change replicates: B shows the node live after the merge.
    b.import(&a.export(ExportMode::all_updates()).unwrap())
        .unwrap();
    assert!(alive(&b, victim));
    assert_eq!(a.get_deep_value(), b.get_deep_value());
}

#[test]
fn a_node_deleted_concurrently_by_a_peer_follows_the_crdt_order_after_the_revival() {
    let (a, ids) = document(7, 2);
    let victim = ids[0];
    let b = LoroDoc::new();
    b.set_peer_id(8).unwrap();
    b.import(&a.export(ExportMode::all_updates()).unwrap())
        .unwrap();
    let old_index = index_of(&a, victim);
    a.get_tree(TREE).delete(victim).unwrap();
    a.commit_with(loro::CommitOptions::new().commit_msg("delete;s=2;t=2"));
    // B deletes the same node without having seen A's delete.
    b.get_tree(TREE).delete(victim).unwrap();
    b.commit_with(loro::CommitOptions::new().commit_msg("delete;s=1;t=2"));

    revive(&a, victim, old_index, "undo;s=3;t=3;u=7@0");
    // Both replicas merge to the same state, whatever it is.
    a.import(&b.export(ExportMode::all_updates()).unwrap())
        .unwrap();
    b.import(&a.export(ExportMode::all_updates()).unwrap())
        .unwrap();
    assert_eq!(a.get_deep_value(), b.get_deep_value());
    assert_eq!(alive(&a, victim), alive(&b, victim));
}

#[test]
fn a_fractional_index_between_changed_neighbours_orders_between_them() {
    let (doc, ids) = document(7, 3);
    let tree = doc.get_tree(TREE);
    let victim = ids[1];
    tree.delete(victim).unwrap();
    doc.commit();
    // The neighbours change: a new node is placed between them.
    let extra = tree.create(TreeParentId::Root).unwrap();
    tree.mov_after(extra, ids[0]).unwrap();
    doc.commit();
    let before = index_of(&doc, extra);
    let after = index_of(&doc, ids[2]);
    let between = FractionalIndex::new_between(&before, &after).expect("room between two indices");

    revive(&doc, victim, between, "undo;s=9;t=9");
    assert_eq!(
        doc.get_tree(TREE).roots(),
        vec![ids[0], extra, victim, ids[2]]
    );
}

#[test]
fn a_reader_of_the_revived_history_sees_it_as_a_change_of_this_peer() {
    let (doc, ids) = document(7, 1);
    let index = index_of(&doc, ids[0]);
    doc.get_tree(TREE).delete(ids[0]).unwrap();
    doc.commit_with(loro::CommitOptions::new().commit_msg("delete;s=2;t=2"));
    revive(&doc, ids[0], index, "undo;s=3;t=3;u=7@0");
    let last = doc.oplog_frontiers().iter().next().unwrap();
    let change = doc.get_change(last).unwrap();
    assert_eq!(change.id.peer, 7);
    assert_eq!(change.message.as_deref(), Some("undo;s=3;t=3;u=7@0"));
}
