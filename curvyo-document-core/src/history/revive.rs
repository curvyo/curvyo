//! Bringing a deleted tree node back with its old id (ADR 0014 §11).
//!
//! The handler API refuses to move a deleted node, so undo of Delete injects one tree `Move`
//! change of this peer through the JSON update import. The CRDT accepts it as an ordinary move: the
//! node keeps its id, its meta map and its nested containers. `loro` is pinned exactly and
//! `tests/loro_revival_canary.rs` fails when an upgrade stops accepting the change.

use loro::{
    ContainerID, ContainerType, FractionalIndex, JsonChange, JsonOp, JsonOpContent, JsonTreeOp,
    LoroDoc, TreeID,
};

use super::error::HistoryError;
use crate::document::OBJECTS_TREE;

/// Moves the deleted node `target` back to the root of the objects tree at `index`, as one change
/// of this peer with `message`. Everything pending is committed first, so the change is complete
/// and the next local counter continues after it.
///
/// # Errors
/// [`HistoryError::Revive`] when the import is refused or leaves anything pending.
pub(crate) fn revive_node(
    doc: &LoroDoc,
    target: TreeID,
    index: FractionalIndex,
    message: &str,
) -> Result<(), HistoryError> {
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
    let mut schema = doc.export_json_updates_without_peer_compression(&version, &version);
    schema.changes.push(JsonChange {
        id: loro::ID::new(peer, counter),
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
            container: ContainerID::new_root(OBJECTS_TREE, ContainerType::Tree),
            counter,
        }],
    });
    let status = doc
        .import_json_updates(schema)
        .map_err(|_| HistoryError::Revive)?;
    if status.pending.is_some() {
        return Err(HistoryError::Revive);
    }
    Ok(())
}
