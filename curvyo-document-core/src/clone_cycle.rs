//! The one writer check against a clone that would draw itself (ADR 0045 §4): whether putting
//! nodes into a container closes a cycle of the edges clone → origin and group → child.

use std::collections::HashSet;

use loro::{LoroTree, TreeID};

use crate::clone_codec::{is_clone_meta, read_clone_link};
use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::{children_of, group_parent, is_group_node};
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;

impl Document {
    /// Whether moving the subtrees of `moved` into the container `into` would make a clone draw
    /// itself: `into` or a container above it is reachable from a node in a moved subtree over
    /// the edges clone → origin and group → child (ADR 0045 §4). O(nodes reached). `false` for a
    /// move to the top level.
    #[must_use]
    pub fn would_cycle(&self, moved: &[NodeId], into: Option<NodeId>) -> bool {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let Some(into) = into.map(tree_id_of) else {
            return false;
        };
        would_cycle_in(&tree, moved.iter().map(|&id| tree_id_of(id)), into)
    }
}

/// [`Document::would_cycle`] on the tree.
pub(crate) fn would_cycle_in(
    tree: &LoroTree,
    moved: impl IntoIterator<Item = TreeID>,
    into: TreeID,
) -> bool {
    let mut above: HashSet<TreeID> = HashSet::new();
    let mut current = Some(into);
    while let Some(node) = current {
        if !above.insert(node) {
            break;
        }
        current = group_parent(tree, node);
    }
    let mut reached: HashSet<TreeID> = HashSet::new();
    let mut todo: Vec<TreeID> = moved.into_iter().collect();
    while let Some(node) = todo.pop() {
        if !reached.insert(node) || !node_exists(tree, node) {
            continue;
        }
        if above.contains(&node) {
            return true;
        }
        if is_group_node(tree, node) {
            todo.extend(children_of(tree, Some(node)));
            continue;
        }
        if let Ok(meta) = tree.get_meta(node)
            && is_clone_meta(&meta)
            && let Some(link) = read_clone_link(&meta)
        {
            todo.push(tree_id_of(link.origin));
        }
    }
    false
}
