//! Bringing deleted tree nodes back with their old ids (ADR 0014 §11).
//!
//! The handler API refuses to move a deleted node, so undo of Delete injects one tree `Move` change
//! of this peer through the JSON update import. The CRDT accepts it as an ordinary move: the node
//! keeps its id, its meta map and its nested containers. All nodes of one undo come back in one
//! change, so the undo is all or nothing. `loro` is pinned exactly and
//! `tests/loro_revival_canary.rs` fails when an upgrade stops accepting the change.

use loro::{
    ContainerID, ContainerType, FractionalIndex, JsonChange, JsonOp, JsonOpContent, JsonTreeOp,
    LoroDoc, TreeID,
};

use super::error::HistoryError;
use crate::document::OBJECTS_TREE;

/// Moves the deleted nodes `nodes` back to the root of the objects tree, each at its index, as one
/// change of this peer with `message`. Everything pending is committed first, so the change is
/// complete and the next local counter continues after it. Nothing is imported unless every node
/// is a deleted node of the tree.
///
/// # Errors
/// [`HistoryError::Revive`] when a node is not a deleted node, or the import is refused or leaves
/// anything pending.
pub(crate) fn revive_nodes(
    doc: &LoroDoc,
    nodes: &[(TreeID, FractionalIndex)],
    message: &str,
) -> Result<(), HistoryError> {
    if nodes.is_empty() {
        return Ok(());
    }
    let tree = doc.get_tree(OBJECTS_TREE);
    let all_deleted = nodes
        .iter()
        .all(|(id, _)| tree.contains(*id) && matches!(tree.is_node_deleted(id), Ok(true)));
    if !all_deleted {
        return Err(HistoryError::Revive);
    }
    doc.commit();
    let peer = doc.peer_id();
    let version = doc.oplog_vv();
    let counter = version.get(&peer).copied().unwrap_or(0);
    let frontiers = doc.oplog_frontiers();
    let lamport = frontiers
        .iter()
        .filter_map(|id| doc.get_change(id))
        .map(|change| change.lamport + u32::try_from(change.len).unwrap_or(u32::MAX))
        .max()
        .unwrap_or(0);
    let container = ContainerID::new_root(OBJECTS_TREE, ContainerType::Tree);
    let ops = nodes
        .iter()
        .zip(counter..)
        .map(|((target, index), op_counter)| JsonOp {
            content: JsonOpContent::Tree(JsonTreeOp::Move {
                target: *target,
                parent: None,
                fractional_index: index.clone(),
            }),
            container: container.clone(),
            counter: op_counter,
        })
        .collect();
    let mut schema = doc.export_json_updates_without_peer_compression(&version, &version);
    schema.changes.push(JsonChange {
        id: loro::ID::new(peer, counter),
        timestamp: 0,
        deps: frontiers.iter().collect(),
        lamport,
        msg: Some(message.to_owned()),
        ops,
    });
    let status = doc
        .import_json_updates(schema)
        .map_err(|_| HistoryError::Revive)?;
    if status.pending.is_some() {
        return Err(HistoryError::Revive);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree_doc(count: usize) -> (LoroDoc, Vec<TreeID>) {
        let doc = LoroDoc::new();
        doc.set_peer_id(5).unwrap();
        let tree = doc.get_tree(OBJECTS_TREE);
        tree.enable_fractional_index(0);
        let ids = (0..count)
            .map(|_| tree.create(loro::TreeParentId::Root).unwrap())
            .collect();
        doc.commit();
        (doc, ids)
    }

    fn index_of(doc: &LoroDoc, id: TreeID) -> FractionalIndex {
        FractionalIndex::from_hex_string(doc.get_tree(OBJECTS_TREE).fractional_index(id).unwrap())
    }

    #[test]
    fn several_nodes_come_back_in_one_change_in_their_old_order() {
        let (doc, ids) = tree_doc(4);
        let indices: Vec<_> = ids.iter().map(|id| index_of(&doc, *id)).collect();
        let tree = doc.get_tree(OBJECTS_TREE);
        tree.delete(ids[1]).unwrap();
        tree.delete(ids[2]).unwrap();
        doc.commit();
        let changes_before = doc.len_changes();
        revive_nodes(
            &doc,
            &[(ids[1], indices[1].clone()), (ids[2], indices[2].clone())],
            "undo;s=3;t=1;u=5@0",
        )
        .unwrap();
        assert_eq!(doc.len_changes(), changes_before + 1, "one change for both");
        assert_eq!(doc.get_tree(OBJECTS_TREE).roots(), ids);
    }

    #[test]
    fn a_node_that_cannot_be_revived_stops_the_whole_change() {
        let (doc, ids) = tree_doc(2);
        let index = index_of(&doc, ids[0]);
        doc.get_tree(OBJECTS_TREE).delete(ids[0]).unwrap();
        doc.commit();
        let before = doc.oplog_vv();
        // The second node is alive, not deleted: nothing may be imported, not even the first.
        let result = revive_nodes(
            &doc,
            &[(ids[0], index.clone()), (ids[1], index)],
            "undo;s=3;t=1;u=5@0",
        );
        assert_eq!(result, Err(HistoryError::Revive));
        assert_eq!(doc.oplog_vv(), before);
        assert_eq!(doc.get_tree(OBJECTS_TREE).roots(), vec![ids[1]]);
    }
}
