//! Reading and sweeping an operation's membership keys and the node chains they are looked up by.

use loro::{LoroTree, TreeID};

use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::{group_parent, node_id};
use crate::manufacturing_codec::{is_manufacturing_node, manufacturing_nodes};
use crate::operation::{delete_key, operation_nodes};
use crate::operation_codec::{MEMBER_PREFIX, decode_path, path_is_live};
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;

impl Document {
    /// `id` and the nodes that hold it, nearest first, up to the top level of the drawing; a node
    /// of a manufacturing (a placement) stops there. Empty when `id` is not live. The chain the
    /// nearest rule walks (ADR 0046 §5, §6).
    #[must_use]
    pub fn node_chain(&self, id: NodeId) -> Vec<NodeId> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut node = tree_id_of(id);
        if !node_exists(&tree, node) || is_manufacturing_node(&tree, node) {
            return Vec::new();
        }
        let mut chain = vec![id];
        while let Some(parent) = group_parent(&tree, node) {
            if is_manufacturing_node(&tree, parent) || chain.len() > crate::MAX_GROUP_DEPTH * 4 {
                break;
            }
            chain.push(node_id(parent));
            node = parent;
        }
        chain
    }

    /// Removes, uncommitted, every membership key whose path names a node that is no longer live
    /// (ADR 0046 §3, Frage 169 A). Called by every local step (`commit_step`), so a key goes in
    /// the step that deletes its node, whichever command deleted it, and undo brings it back with
    /// the node. Keys that do not read are left alone. A key left dead by a peer's merge goes at
    /// the next local step; reads ignore it until then.
    pub(crate) fn sweep_dead_member_keys(&self) {
        if !self.member_keys_may_exist.get() {
            return;
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        // ADR 0057 §3: the derived operations of a grid deleted, cut or ungrouped in this step,
        // or removed by a merge, go too. Every derived operation holds a membership key when it
        // is written, so the flag above covers them.
        crate::test_grid_rows::sweep_orphan_derived_rows(&tree);
        let entries = self.member_entries();
        if entries.is_empty() {
            // Every key is gone: the next commits pay nothing again until a key is written.
            self.member_keys_may_exist.set(false);
            return;
        }
        for (op, key, path) in entries {
            if path_is_live(&tree, &path) {
                continue;
            }
            // ADR 0054 §5: deleting a test grid's top group deletes its row in the same step.
            let grid_row = tree
                .get_meta(op)
                .is_ok_and(|meta| crate::operation_codec::grid_switch(&meta) == Some(true));
            if grid_row && node_exists(&tree, op) {
                // An operation node is a child of its manufacturing, never of a group, so no
                // group is emptied here.
                let _ = LoroTree::delete(&tree, op);
            } else {
                delete_key(&tree, op, &key);
            }
        }
    }

    /// Removes, uncommitted, every membership key whose path `doomed` accepts. Used by the clone
    /// repairs, where a clone stops being a clone and its members' keys go (ADR 0046 §2 (c)).
    pub(crate) fn remove_member_keys_where(&self, doomed: impl Fn(&[TreeID]) -> bool) {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        for (op, key, path) in self.member_entries() {
            if doomed(&path) {
                delete_key(&tree, op, &key);
            }
        }
    }

    /// Removes, uncommitted, the member keys (paths of more than one node) of the clones `clones`,
    /// which stop being clones in the caller's step (ADR 0046 §2 (c)).
    pub(crate) fn remove_member_keys_of_clones(&self, clones: &[TreeID]) {
        self.remove_member_keys_where(|path| path.len() > 1 && clones.contains(&path[0]));
    }

    /// Every membership key of every operation whose text reads, with its operation and path.
    /// Keys that do not read are left out: they are kept in the file and never touched.
    pub(crate) fn member_entries(&self) -> Vec<(TreeID, String, Vec<TreeID>)> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut out = Vec::new();
        for manufacturing in manufacturing_nodes(&tree) {
            for op in operation_nodes(&tree, manufacturing) {
                let Ok(meta) = tree.get_meta(op) else {
                    continue;
                };
                meta.for_each(|key, _| {
                    if let Some(path) = key.strip_prefix(MEMBER_PREFIX).and_then(decode_path) {
                        out.push((op, key.to_owned(), path));
                    }
                });
            }
        }
        out
    }

    /// The path of `member` through the clone `clone`: the clone's id, then the nodes between
    /// its original and `member` (the groups and `member` itself). `None` when `clone` is not a
    /// clone, or `member` is not below its original (ADR 0046 §2, Frage 167 B).
    #[must_use]
    pub fn clone_member_path(&self, clone: NodeId, member: NodeId) -> Option<Vec<NodeId>> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let links = self.clone_links();
        let (origin, _) = links.get(&clone)?;
        let mut below = Vec::new();
        let mut node = tree_id_of(member);
        if !node_exists(&tree, node) || node_id(node) == *origin {
            return None;
        }
        below.push(member);
        while let Some(parent) = group_parent(&tree, node) {
            if node_id(parent) == *origin {
                let mut path = vec![clone];
                path.extend(below.into_iter().rev());
                return Some(path);
            }
            below.push(node_id(parent));
            node = parent;
        }
        None
    }

    /// The membership keys a leaf is looked up by, nearest first (ADR 0046 §7, §8): the prefixes
    /// of `path` longest first (for a clone member, the member, the groups between it and the
    /// clone, the clone), then the groups above the first node. `path` is one id for an own
    /// item.
    #[must_use]
    pub fn key_chain(&self, path: &[NodeId]) -> Vec<Vec<NodeId>> {
        let mut chain: Vec<Vec<NodeId>> = (1..=path.len())
            .rev()
            .map(|len| path[..len].to_vec())
            .collect();
        if let Some(&first) = path.first() {
            chain.extend(
                self.node_chain(first)
                    .into_iter()
                    .skip(1)
                    .map(|above| vec![above]),
            );
        }
        chain
    }
}

#[cfg(test)]
mod sweep_flag_tests {
    use crate::{
        Document, DocumentSize, NewOperation, NewOutputPage, Point, RectBounds, pack, unpack,
    };

    fn square(d: &Document, x: f64) -> crate::NodeId {
        d.create_rect(RectBounds::from_corners(
            Point::new(x, 0.0),
            Point::new(x + 5.0, 5.0),
        ))
    }

    /// `0101` ADR 0046 §3, performance: a document without membership keys skips the sweep of
    /// every commit (a template load commits once per node); one with keys keeps it on, and a
    /// reopened file reads its keys back.
    #[test]
    fn the_sweep_is_off_until_a_membership_key_exists_and_reads_one_from_a_file() {
        let d = Document::new(1);
        square(&d, 0.0);
        assert!(
            !d.member_keys_may_exist.get(),
            "no key: the sweep stays off"
        );

        let laser = d
            .add_output_page(NewOutputPage {
                process: "curvyo.laser",
                label: "Laser",
                profile: None,
                work_area: DocumentSize::from_mm(400.0, 300.0),
            })
            .unwrap();
        let m = laser.kind.manufacturing().unwrap();
        let rect = square(&d, 10.0);
        d.add_operation(
            m,
            &NewOperation {
                kind: "cut",
                name: "Cut 1",
            },
            0,
            &[vec![rect]],
        )
        .unwrap();
        assert!(
            d.member_keys_may_exist.get(),
            "a key was written: the sweep is on"
        );

        let back = unpack(2, &pack(&d, "flag").unwrap()).unwrap();
        assert!(
            back.member_keys_may_exist.get(),
            "the keys of a file are read at open"
        );
        let plain = Document::new(3);
        square(&plain, 0.0);
        let plain_back = unpack(4, &pack(&plain, "flag").unwrap()).unwrap();
        assert!(!plain_back.member_keys_may_exist.get());
    }
}
