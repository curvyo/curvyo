//! The writes that create and delete layers (`specs/0050-layers-and-objects-panel` criteria 34 and
//! 36, `adrs.md` decisions 1 and 8). A layer is a group at the top level with the `layer` marker
//! and always a name.

use loro::{LoroValue, TreeParentId};

use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::{KEY_GROUP, children_of, is_layer_node, node_id};
use crate::groups::remove_emptied_groups;
use crate::node_registers::{KEY_LAYER, KEY_NAME, read_registers};
use crate::objects::ObjectEditError;
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;

/// The label of New layer's commit.
pub const NEW_LAYER_COMMIT_LABEL: &str = "new_layer";
/// The label of Delete layer's commit.
pub const DELETE_LAYER_COMMIT_LABEL: &str = "delete_layer";

/// The name of the layer the first drawn object of a document with no layer makes
/// ([`Document::set_draw_into_initial_layer`], customer feedback 2026-10-14).
pub const INITIAL_LAYER_NAME: &str = "Initial Layer";

/// Writes a new layer node named `name` at the front of the top level, without a commit, and
/// returns it. The callers commit.
///
/// # Panics
/// Does not panic in practice: the tree operations act on the root and a node just created.
pub(crate) fn create_layer_node(tree: &loro::LoroTree, name: &str) -> loro::TreeID {
    let index = children_of(tree, None).len();
    create_layer_node_at(tree, index, name)
}

/// [`create_layer_node`] at the top-level position `index`.
fn create_layer_node_at(tree: &loro::LoroTree, index: usize, name: &str) -> loro::TreeID {
    tree.enable_fractional_index(0);
    // invariant: the root exists and the index is at most the number of its children.
    #[allow(clippy::unwrap_used)]
    let layer = tree.create_at(TreeParentId::Root, index).unwrap();
    // invariant: the node was just created, so its meta map exists.
    #[allow(clippy::unwrap_used)]
    {
        let meta = tree.get_meta(layer).unwrap();
        meta.insert(KEY_GROUP, true).unwrap();
        meta.insert(KEY_LAYER, true).unwrap();
        meta.insert(KEY_NAME, LoroValue::from(name)).unwrap();
    }
    layer
}

/// "Layer N" with the smallest positive N that no name of `taken` uses (criterion 7).
#[must_use]
pub(crate) fn next_layer_name<'a>(taken: impl IntoIterator<Item = &'a str>) -> String {
    let used: std::collections::HashSet<u64> = taken
        .into_iter()
        .filter_map(|name| name.strip_prefix("Layer ")?.parse().ok())
        .collect();
    // One of the first `used.len() + 1` numbers is free.
    let last = u64::try_from(used.len())
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let n = (1..=last).find(|n| !used.contains(n)).unwrap_or(last);
    format!("Layer {n}")
}

impl Document {
    /// Creates an empty layer named "Layer N" directly in front of the layer `in_front_of`, or at
    /// the front of the top level when that is `None` or no live layer, in one commit labelled
    /// [`NEW_LAYER_COMMIT_LABEL`] (criterion 34). Returns its id.
    ///
    pub fn new_layer(&self, in_front_of: Option<NodeId>) -> NodeId {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let roots = children_of(&tree, None);
        let names: Vec<String> = roots
            .iter()
            .filter(|&&id| is_layer_node(&tree, id))
            .filter_map(|&id| tree.get_meta(id).ok())
            .filter_map(|meta| read_registers(&meta).name)
            .collect();
        let name = next_layer_name(names.iter().map(String::as_str));
        let index = in_front_of
            .map(tree_id_of)
            .filter(|&id| node_exists(&tree, id) && is_layer_node(&tree, id))
            .and_then(|id| roots.iter().position(|&root| root == id))
            .map_or(roots.len(), |at| at + 1);
        let layer = create_layer_node_at(&tree, index, &name);
        self.commit_with_label(NEW_LAYER_COMMIT_LABEL);
        node_id(layer)
    }

    /// Deletes the layer `id` with everything in it, empty or not, in one commit labelled
    /// [`DELETE_LAYER_COMMIT_LABEL`] (criterion 36). The layer node goes in **one** tree delete,
    /// so an undo revives it with every descendant and every id (`adrs.md` decision 8). Returns
    /// how many nodes were inside it.
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] if `id` does not exist; [`ObjectEditError::NotALayer`] if
    /// it is not a layer; [`ObjectEditError::Locked`] if the layer is locked. Nothing is written.
    pub fn delete_layer(&self, id: NodeId) -> Result<usize, ObjectEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let layer = tree_id_of(id);
        if !node_exists(&tree, layer) {
            return Err(ObjectEditError::NoSuchObject);
        }
        if !is_layer_node(&tree, layer) {
            return Err(ObjectEditError::NotALayer);
        }
        let meta = tree
            .get_meta(layer)
            .map_err(|_| ObjectEditError::NoSuchObject)?;
        if read_registers(&meta).locked {
            return Err(ObjectEditError::Locked);
        }
        let mut inside = 0;
        let mut stack = children_of(&tree, Some(layer));
        while let Some(node) = stack.pop() {
            inside += 1;
            stack.extend(children_of(&tree, Some(node)));
        }
        // The clones outside the layer that follow something in it are repaired in the same
        // commit (`0050` criterion 36, `0049` criterion 31).
        self.repair_clones_of_removed(&tree, &[layer]);
        tree.delete(layer)
            .map_err(|_| ObjectEditError::NoSuchObject)?;
        // The rule of `specs/0023-groups` that every delete sweeps emptied groups; it never
        // removes a layer.
        remove_emptied_groups(&tree, &[None]);
        self.commit_with_label(DELETE_LAYER_COMMIT_LABEL);
        Ok(inside)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_next_layer_name_takes_the_smallest_free_number() {
        assert_eq!(next_layer_name([]), "Layer 1");
        assert_eq!(next_layer_name(["Layer 1", "Layer 3", "Cut"]), "Layer 2");
        assert_eq!(next_layer_name(["Layer 2"]), "Layer 1");
        assert_eq!(next_layer_name(["Layer 1", "Layer 2"]), "Layer 3");
        assert_eq!(next_layer_name(["Layer x", "Layer 0"]), "Layer 1");
    }
}
