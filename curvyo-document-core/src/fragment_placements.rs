//! Placed pages inside a fragment become ordinary copies (ADR 0084 decision 6, `specs/0137`
//! criterion 4): in the private copy of the source, each placement taken is replaced by copies
//! of its drawing page's members under the placement's global matrix, so the target holds what
//! the laser page showed and no link to a page it does not have.

use loro::{TreeID, TreeParentId};

use crate::document::{Document, OBJECTS_TREE};
use crate::objects::CopySource;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::primitive_model::ObjectSnapshot;
use crate::units::Vec2;

impl Document {
    /// Replaces every placement under (or among) `wanted` by copies of its members from
    /// `placed`; the copies join `wanted` as top-level nodes. Uncommitted writes of the private
    /// copy, committed with its prune.
    pub(crate) fn bake_placements(
        &self,
        wanted: &mut Vec<TreeID>,
        placed: &[(NodeId, Vec<NodeId>)],
    ) {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let placements: Vec<NodeId> = self
            .placed_pages()
            .into_iter()
            .map(|p| p.id)
            .filter(|&id| {
                let node = tree_id_of(id);
                wanted.contains(&node) || crate::group_tree::has_ancestor_in(&tree, node, wanted)
            })
            .collect();
        if placements.is_empty() {
            return;
        }
        for placement in &placements {
            let members = placed
                .iter()
                .find(|(id, _)| id == placement)
                .map_or(&[][..], |(_, members)| members.as_slice());
            let copies = self.uncommitted(|doc| doc.copies_of_placement(*placement, members));
            wanted.retain(|&node| node != tree_id_of(*placement));
            wanted.extend(copies);
        }
        self.uncommitted(|doc| doc.remove_placement_nodes(&placements));
    }

    /// Copies of `members` mapped by the global matrix of `placement`, each at the end of the
    /// top level. Members that cannot be copied are left out.
    fn copies_of_placement(&self, placement: NodeId, members: &[NodeId]) -> Vec<TreeID> {
        let Some(map) = self.placed_page(placement).and_then(|p| p.global_map()) else {
            return Vec::new();
        };
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut made = Vec::new();
        for &member in members {
            let node = tree_id_of(member);
            if !crate::path_codec::node_exists(&tree, node) {
                continue;
            }
            let anchor_ids = self.repair_anchor_ids(crate::group_tree::anchors_under(&tree, node));
            let Ok(copies) = self.duplicate_objects(
                &[CopySource {
                    id: member,
                    anchor_ids,
                }],
                Vec2::ZERO,
            ) else {
                continue;
            };
            let Some(&copy) = copies.first() else {
                continue;
            };
            let leaves = if self.is_group(copy) {
                self.leaves_of(copy)
            } else {
                vec![copy]
            };
            let results: Vec<ObjectSnapshot> = leaves
                .iter()
                .filter_map(|&leaf| Some(self.unlinked_leaf(&self.object(leaf)?, map, None)))
                .collect();
            if self.transform_objects(&results, false).is_err() {
                continue;
            }
            let copy = tree_id_of(copy);
            tree.enable_fractional_index(0);
            if tree.mov(copy, TreeParentId::Root).is_ok() {
                made.push(copy);
            }
        }
        made
    }
}
