//! The follower rule of sketch instances (`specs/0099-sketch-pages/adrs.md` A6): when a command
//! turns a node of a chain into a path or removes it, a clone below it that holds own values
//! would jump (its values are residue on a path), so it is unlinked into a path from what it
//! draws now, deepest first; every clone without own values keeps following and draws what it drew.

use std::collections::{HashMap, HashSet};

use loro::LoroTree;

use crate::document::{Document, OBJECTS_TREE};
use crate::path_model::{NodeId, PathSnapshot};
use crate::primitive_model::ObjectSnapshot;

/// A clone planned to become a path: its id, the outline it draws and its opacity register.
pub(crate) struct Follower {
    pub(crate) clone: NodeId,
    pub(crate) path: PathSnapshot,
    pub(crate) opacity: f64,
}

impl Document {
    /// The clones that hold own values and whose chain passes through a node of `changed` (or ends
    /// at one), not in `skip`, deepest first, each with what it draws now. Reads only.
    pub(crate) fn value_followers(
        &self,
        changed: &HashSet<NodeId>,
        skip: &HashSet<NodeId>,
    ) -> Vec<Follower> {
        let read = self.read_objects();
        let origins: HashMap<NodeId, NodeId> = read
            .objects
            .iter()
            .filter_map(|object| match object {
                ObjectSnapshot::Clone(clone) => Some((clone.id(), clone.chain.origin)),
                _ => None,
            })
            .collect();
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut found: Vec<(usize, Follower)> = read
            .objects
            .iter()
            .filter_map(|object| {
                let ObjectSnapshot::Clone(clone) = object else {
                    return None;
                };
                if !clone.chain.own_params
                    || skip.contains(&clone.id())
                    || changed.contains(&clone.id())
                {
                    return None;
                }
                let mut at = clone.chain.origin;
                let mut passes = changed.contains(&at);
                let mut steps = 0;
                while !passes && let Some(&next) = origins.get(&at) {
                    steps += 1;
                    if steps > origins.len() {
                        break;
                    }
                    at = next;
                    passes = changed.contains(&at);
                }
                passes.then(|| {
                    (
                        clone.chain.depth,
                        Follower {
                            clone: clone.id(),
                            path: (*clone.resolved).clone(),
                            opacity: crate::clone_write::source_opacity(&tree, clone),
                        },
                    )
                })
            })
            .collect();
        found.sort_by_key(|(depth, _)| std::cmp::Reverse(*depth));
        found.into_iter().map(|(_, follower)| follower).collect()
    }

    /// Writes `followers` as paths in place, with fresh anchor ids; uncommitted.
    pub(crate) fn unlink_followers(&self, tree: &LoroTree, followers: Vec<Follower>) {
        for Follower {
            clone,
            mut path,
            opacity,
        } in followers
        {
            let ids = self.repair_anchor_ids(path.all_anchors().count());
            for (anchor, id) in path.all_anchors_mut().zip(ids) {
                anchor.id = id;
            }
            if let Ok(meta) = tree.get_meta(crate::paths::tree_id_of(clone)) {
                crate::clone_write::unlink_meta(&meta, &path, opacity);
            }
        }
    }
}
