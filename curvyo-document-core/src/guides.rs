//! Marking paths and shapes as guides and back (`specs/0046-document-shapes` criteria 2, 9 and
//! 10, ADR 0019 §2): which nodes of a selection can carry the `guide` register, and the one
//! commit that sets or removes it.

use std::collections::HashSet;

use loro::{LoroTree, TreeID};

use crate::clone_codec::is_clone_meta;
use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::{is_group_node, leaves_under, node_id};
use crate::node_registers::{KEY_GUIDE, read_registers};
use crate::objects::ObjectEditError;
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;

/// The label of a mark or unmark's commit; its history name is "Guide".
pub const SET_GUIDE_COMMIT_LABEL: &str = "set_guide";

/// What the nodes of a selection are as guides: how many can carry the flag and how many do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GuideMarking {
    /// The paths, compound paths and shapes among the nodes and under their groups.
    pub markable: usize,
    /// How many of them are guides already.
    pub guides: usize,
}

impl GuideMarking {
    /// Whether the Guide button acts: something can be marked (criterion 9).
    #[must_use]
    pub const fn enabled(self) -> bool {
        self.markable > 0
    }

    /// Whether the Guide button shows as pressed: everything markable is a guide (criterion 10).
    #[must_use]
    pub const fn all_guides(self) -> bool {
        self.markable > 0 && self.guides == self.markable
    }
}

/// Whether the leaf `id` can carry the flag: a path or a shape; a linked clone only when it
/// carries its own flag already (ADR 0022 §2: putting a text on a clone marks the clone alone),
/// so Guide off turns that clone back into artwork while Guide on still skips clones (they read
/// their root's flag, ADR 0019 §2).
fn is_markable(tree: &LoroTree, id: TreeID) -> bool {
    !is_group_node(tree, id)
        && tree
            .get_meta(id)
            .is_ok_and(|meta| !is_clone_meta(&meta) || read_registers(&meta).guide)
}

impl Document {
    /// The markable leaves of `ids`: a leaf itself, every leaf below a group or a layer; each
    /// once, in the order met. Ids that do not exist are left out.
    fn markable_leaves(&self, ids: &[NodeId]) -> Vec<TreeID> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut seen = HashSet::new();
        let mut leaves = Vec::new();
        for &id in ids {
            let tree_id = tree_id_of(id);
            if !node_exists(&tree, tree_id) {
                continue;
            }
            let under = if is_group_node(&tree, tree_id) {
                leaves_under(&tree, Some(tree_id))
            } else {
                vec![tree_id]
            };
            leaves.extend(
                under
                    .into_iter()
                    .filter(|&leaf| is_markable(&tree, leaf) && seen.insert(leaf)),
            );
        }
        leaves
    }

    /// How many nodes of `ids` (with their groups' paths and shapes) can be guides, and how many
    /// are (criteria 9 and 10).
    #[must_use]
    pub fn guide_marking(&self, ids: &[NodeId]) -> GuideMarking {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let leaves = self.markable_leaves(ids);
        // A dimension is a guide by kind (ADR 0027 §1): the button shows pressed on it.
        let guides = leaves
            .iter()
            .filter(|&&leaf| {
                tree.get_meta(leaf).is_ok_and(|meta| {
                    read_registers(&meta).guide || crate::dimension_codec::is_dimension_meta(&meta)
                })
            })
            .count();
        GuideMarking {
            markable: leaves.len(),
            guides,
        }
    }

    /// Marks every markable node of `ids` (for a group: its paths and shapes) as a guide, or turns
    /// it back into an ordinary node, in one commit labelled [`SET_GUIDE_COMMIT_LABEL`] (criteria 9
    /// and 10). Marking writes `guide: true`; unmarking removes the key, so a file with no guide
    /// has none (criterion 2). Nothing else of the node changes. Allowed on a locked node, as the
    /// other registers of `0050`. Returns the ids written, in the order met.
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] if no node of `ids` can be marked (nothing is written).
    ///
    /// # Panics
    /// Does not panic in practice: every leaf written was resolved live just before.
    pub fn set_guide(&self, ids: &[NodeId], guide: bool) -> Result<Vec<NodeId>, ObjectEditError> {
        let leaves = self.markable_leaves(ids);
        if leaves.is_empty() {
            return Err(ObjectEditError::NoSuchObject);
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        for &leaf in &leaves {
            // invariant: every leaf was confirmed live by `markable_leaves`, so its meta map exists.
            #[allow(clippy::unwrap_used)]
            let meta = tree.get_meta(leaf).unwrap();
            // A node already in the asked state is not written, so the step names only changes;
            // a dimension carries no flag (it is converted, `Document::convert_dimensions`).
            if read_registers(&meta).guide != guide
                && !crate::dimension_codec::is_dimension_meta(&meta)
            {
                write_guide(&meta, guide);
            }
        }
        self.commit_with_label(SET_GUIDE_COMMIT_LABEL);
        Ok(leaves.into_iter().map(node_id).collect())
    }
}

/// Whether the leaf `object` is a guide, given which nodes carry their own flag (`own`): a path's
/// or shape's own register; a linked clone's own or its root's (ADR 0019 §2 as extended by ADR
/// 0022 §2, criterion 34). The one place of the clone rule, for the current read and for a
/// version's.
pub(crate) fn effective_guide(
    object: &crate::primitive_model::ObjectSnapshot,
    own: impl Fn(NodeId) -> bool,
) -> bool {
    match object {
        crate::primitive_model::ObjectSnapshot::Clone(clone) => {
            own(clone.id()) || own(clone.chain.root)
        }
        crate::primitive_model::ObjectSnapshot::Sketch(_)
        | crate::primitive_model::ObjectSnapshot::Path(_)
        | crate::primitive_model::ObjectSnapshot::Primitive(_)
        | crate::primitive_model::ObjectSnapshot::Text(_)
        | crate::primitive_model::ObjectSnapshot::Image(_) => own(object.id()),
        // A dimension is a guide by kind (ADR 0027 §1).
        crate::primitive_model::ObjectSnapshot::Dimension(_) => true,
    }
}

/// Whether every leaf of `ids` carries the `guide` register: the rule for a path made from inputs
/// (`specs/0046-document-shapes` criterion 34, `adrs.md` decision 5). `false` for no input.
pub(crate) fn all_guides(tree: &LoroTree, ids: &[TreeID]) -> bool {
    !ids.is_empty()
        && ids.iter().all(|&id| {
            tree.get_meta(id)
                .is_ok_and(|meta| read_registers(&meta).guide)
        })
}

/// Writes the `guide` register of a node: `true` when `guide`, else no key (criterion 2).
pub(crate) fn write_guide(meta: &loro::LoroMap, guide: bool) {
    // invariant: a map insert or delete on a live map does not fail.
    #[allow(clippy::unwrap_used)]
    if guide {
        meta.insert(KEY_GUIDE, true).unwrap();
    } else {
        meta.delete(KEY_GUIDE).unwrap();
    }
}
