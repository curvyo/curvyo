//! Per-part operations of a generated object (ADR 0082 §10, `specs/0147-rounded-box` criterion 15):
//! Detach part gives one part its own operations, Attach part takes them back. A detached part is
//! a membership key `member.<node>#<part>` on an operation; the part stays in the record, so it
//! still regenerates, moves with the box and keeps its place in the layout.

use loro::{LoroMap, LoroTree, TreeID, TreeParentId, ValueOrContainer};

use crate::document::{Document, OBJECTS_TREE};
use crate::generated_codec::{self, is_generated_meta};
use crate::manufacturing_codec::manufacturing_nodes;
use crate::node_registers::{KEY_NAME, clean_name};
use crate::operation::operation_nodes;
use crate::operation_codec::{MEMBER_PREFIX, decode_member, grid_switch, part_member_key};
use crate::path_codec::leaf_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;

/// The label of Detach part (one step).
pub const DETACH_PART_COMMIT_LABEL: &str = "detach_part";
/// The label of Attach part (one step).
pub const ATTACH_PART_COMMIT_LABEL: &str = "attach_part";

/// A refused Detach part or Attach part; it writes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PartError {
    /// No generated object with this id.
    #[error("no such generated object")]
    NoSuchObject,
    /// The record has no part with this key (or no keys yet and none were given).
    #[error("the object has no such part")]
    NoSuchPart,
    /// The operation to copy is not a live operation, or is a test grid row.
    #[error("no such operation")]
    NoSuchOperation,
    /// Detach part needs at least one operation to copy.
    #[error("the object has no operation to copy")]
    NoOperation,
    /// The part already has its own operations.
    #[error("the part is already detached")]
    AlreadyDetached,
}

/// One part membership key of an operation: the operation, the key text, the node and the part.
struct PartKey {
    op: TreeID,
    key: String,
    node: TreeID,
    part: &'static str,
}

/// Every part membership key of every operation of every manufacturing whose text reads.
fn part_keys_of(tree: &LoroTree) -> Vec<PartKey> {
    let mut out = Vec::new();
    for manufacturing in manufacturing_nodes(tree) {
        for op in operation_nodes(tree, manufacturing) {
            let Ok(meta) = tree.get_meta(op) else {
                continue;
            };
            meta.for_each(|key, _| {
                let read = key.strip_prefix(MEMBER_PREFIX).and_then(decode_member);
                if let Some((path, Some(part))) = read
                    && let [node] = path.as_slice()
                {
                    out.push(PartKey {
                        op,
                        key: key.to_owned(),
                        node: *node,
                        part,
                    });
                }
            });
        }
    }
    out
}

/// The name of a copy of operation `name` for `part_name`: "Cut 3 mm · Lid".
fn copy_name(name: &str, part_name: &str) -> String {
    clean_name(&format!("{name} · {part_name}")).unwrap_or_else(|| part_name.to_owned())
}

/// The registers of `original` a copy takes: every plain value except the memberships.
fn copy_registers(original: &LoroMap, copy: &LoroMap, name: &str) {
    original.for_each(|key, value| {
        if key.starts_with(MEMBER_PREFIX) || key == KEY_NAME {
            return;
        }
        if let ValueOrContainer::Value(value) = value {
            // invariant: inserting a plain value on an attached map cannot fail.
            #[allow(clippy::unwrap_used)]
            copy.insert(key, value).unwrap();
        }
    });
    // invariant: inserting a plain value on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    copy.insert(KEY_NAME, name).unwrap();
}

impl Document {
    /// The keys of the parts of the generated object `node` that have their own operations
    /// (detached), each once, in the order of their first operation.
    #[must_use]
    pub fn detached_parts(&self, node: NodeId) -> Vec<String> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(node);
        let mut out: Vec<String> = Vec::new();
        for entry in part_keys_of(&tree).iter().filter(|e| e.node == node) {
            if !out.iter().any(|k| k == entry.part) {
                out.push(entry.part.to_owned());
            }
        }
        out
    }

    /// Detach part: gives the part `part_key` of the generated object `node` its own operations,
    /// in one step labelled [`DETACH_PART_COMMIT_LABEL`]. Each of `operations` (the operations the
    /// box runs with) is copied under the name "<operation> · `part_name`", placed right after
    /// it, with the part's key as its only membership, so the job does not change. `keys` are the
    /// part keys of a fresh generation, written to a record that has none yet (an older file).
    /// Returns the copies, in the order of `operations`.
    ///
    /// # Errors
    /// [`PartError`]; nothing is written then.
    ///
    /// # Panics
    /// Does not panic in practice: every write goes to a node this call just confirmed live or
    /// just created.
    pub fn detach_part(
        &self,
        node: NodeId,
        part_key: &str,
        part_name: &str,
        operations: &[NodeId],
        keys: &[String],
    ) -> Result<Vec<NodeId>, PartError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_node = tree_id_of(node);
        if !leaf_exists(&tree, tree_node) {
            return Err(PartError::NoSuchObject);
        }
        let meta = tree
            .get_meta(tree_node)
            .map_err(|_| PartError::NoSuchObject)?;
        if !is_generated_meta(&meta) {
            return Err(PartError::NoSuchObject);
        }
        let mut record = generated_codec::read_record(&meta).ok_or(PartError::NoSuchObject)?;
        let written_keys = record.part_keys.is_empty();
        if written_keys {
            record.part_keys = keys.to_vec();
            if !record.keys_are_valid() || record.part_keys.is_empty() {
                return Err(PartError::NoSuchPart);
            }
        }
        if record.part_index(part_key).is_none() {
            return Err(PartError::NoSuchPart);
        }
        if operations.is_empty() {
            return Err(PartError::NoOperation);
        }
        if self.detached_parts(node).iter().any(|k| k == part_key) {
            return Err(PartError::AlreadyDetached);
        }
        let key = part_member_key(tree_node, part_key);
        let mut sources = Vec::new();
        for &op in operations {
            let op_meta = self
                .operation_meta(op)
                .map_err(|_| PartError::NoSuchOperation)?;
            if grid_switch(&op_meta) != Some(false) {
                return Err(PartError::NoSuchOperation);
            }
            sources.push((tree_id_of(op), op_meta));
        }
        if written_keys {
            generated_codec::write_record(&meta, &record);
        }
        tree.enable_fractional_index(0);
        let mut copies = Vec::new();
        for (op, op_meta) in sources {
            let Some(TreeParentId::Node(parent)) = tree.parent(op) else {
                return Err(PartError::NoSuchOperation);
            };
            let name = crate::manufacturing_codec::string(&op_meta, KEY_NAME).unwrap_or_default();
            let copy = tree
                .create(TreeParentId::Node(parent))
                .map_err(|_| PartError::NoSuchOperation)?;
            // invariant: both nodes are live siblings and the fractional index is on.
            #[allow(clippy::unwrap_used)]
            tree.mov_after(copy, op).unwrap();
            // invariant: the node was just created, so its meta map exists.
            #[allow(clippy::unwrap_used)]
            let copy_meta = tree.get_meta(copy).unwrap();
            copy_registers(&op_meta, &copy_meta, &copy_name(&name, part_name));
            // invariant: inserting a plain value on an attached map cannot fail.
            #[allow(clippy::unwrap_used)]
            copy_meta.insert(&key, true).unwrap();
            self.member_keys_may_exist.set(true);
            copies.push(crate::group_tree::node_id(copy));
        }
        self.commit_with_label(DETACH_PART_COMMIT_LABEL);
        Ok(copies)
    }

    /// Attach part: removes every membership key of the part `part_key` of `node` from every
    /// operation, in one step labelled [`ATTACH_PART_COMMIT_LABEL`]; the part runs with the box's
    /// operations again. `Ok(false)` and nothing written when it was not detached.
    ///
    /// # Errors
    /// [`PartError::NoSuchObject`] when `node` is no live generated object.
    pub fn attach_part(&self, node: NodeId, part_key: &str) -> Result<bool, PartError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_node = tree_id_of(node);
        let is_generated = leaf_exists(&tree, tree_node)
            && tree
                .get_meta(tree_node)
                .is_ok_and(|meta| is_generated_meta(&meta));
        if !is_generated {
            return Err(PartError::NoSuchObject);
        }
        let mut changed = false;
        for entry in part_keys_of(&tree) {
            if entry.node == tree_node && entry.part == part_key {
                crate::operation::delete_key(&tree, entry.op, &entry.key);
                changed = true;
            }
        }
        if changed {
            self.commit_with_label(ATTACH_PART_COMMIT_LABEL);
        }
        Ok(changed)
    }
}

/// Rewrites, uncommitted, the part keys of the generated node `node` that is being converted:
/// the key of each part in `keys` becomes the membership of the target beside it (a part's new
/// path, or the node itself when it becomes one path). Part keys with no part in `keys` stay (ADR 0082 §10
/// Lifecycle).
pub(crate) fn rewrite_part_keys(tree: &LoroTree, node: TreeID, keys: &[(String, TreeID)]) {
    for entry in part_keys_of(tree) {
        if entry.node != node {
            continue;
        }
        let Some((_, target)) = keys.iter().find(|(k, _)| k == entry.part) else {
            continue;
        };
        crate::operation::delete_key(tree, entry.op, &entry.key);
        if let Ok(meta) = tree.get_meta(entry.op) {
            // invariant: inserting a plain value on an attached map cannot fail.
            #[allow(clippy::unwrap_used)]
            meta.insert(&crate::operation_codec::member_key(&[*target]), true)
                .unwrap();
        }
    }
}
