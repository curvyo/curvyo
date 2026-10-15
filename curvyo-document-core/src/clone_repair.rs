//! The repair of the clones whose original a command removes (`specs/0049-linked-clone`
//! criteria 30, 31 and 54, ADR 0015 §5): each is re-pointed to the next original that stays, with
//! the skipped matrices composed, or unlinked into a path when its chain ends at a removed root.

use std::collections::HashSet;

use loro::{LoroTree, TreeID};

use crate::clone_codec::{CloneLink, write_clone_link};
use crate::clone_write::unlink_meta;
use crate::document::Document;
use crate::object_read::read_one_clone;
use crate::path_model::{AnchorId, NodeId, PathSnapshot};

/// What one repair did, for the notice of criterion 31.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CloneRepair {
    /// Clones that now follow the next original that stays.
    pub repointed: usize,
    /// Clones that became paths.
    pub unlinked: usize,
}

/// The anchor ids the repair gives an unlinked clone live in their own half of the counter
/// space, `(peer, 2^63 | n)`, which the session's minter (counting up from 0) never reaches.
/// The repair runs inside commands that take no minted ids (Delete, a Boolean, Break apart, an
/// undo), and its counter is never below the peer's operation count, so it never repeats.
const REPAIR_ID_SPACE: u64 = 1 << 63;

/// One decided repair, read before any write.
enum Fix {
    Repoint(NodeId, CloneLink),
    Unlink(NodeId, PathSnapshot, f64),
}

impl Document {
    /// Repairs, without committing, every live clone outside `removed` (nodes and everything
    /// under them) whose original is in it. Call it before the nodes are deleted, in the commit
    /// that deletes them: it reads the state before the command. Records the counts for
    /// [`Document::take_clone_repair`].
    pub(crate) fn repair_clones_of_removed(&self, tree: &LoroTree, removed: &[TreeID]) {
        let links = self.clone_links();
        if links.is_empty() {
            return;
        }
        let mut gone: HashSet<NodeId> = HashSet::new();
        for &node in removed {
            gone.insert(NodeId::from_parts(node.peer, node.counter));
            for leaf in crate::group_tree::leaves_under(tree, Some(node)) {
                gone.insert(NodeId::from_parts(leaf.peer, leaf.counter));
            }
        }
        let mut fixes = Vec::new();
        for clone in self.object_ids() {
            let Some(&(origin, matrix)) = links.get(&clone) else {
                continue;
            };
            if gone.contains(&clone) || !gone.contains(&origin) {
                continue;
            }
            let (mut node, mut composed) = (origin, matrix);
            for _ in 0..links.len() {
                match links.get(&node) {
                    Some(&(next, m)) if gone.contains(&node) => {
                        composed = composed.compose(m);
                        node = next;
                    }
                    _ => break,
                }
            }
            if !gone.contains(&node) {
                fixes.push(Fix::Repoint(
                    clone,
                    CloneLink {
                        origin: node,
                        matrix: composed,
                    },
                ));
            } else if !links.contains_key(&node)
                && let Some(read) = read_one_clone(tree, clone)
            {
                let opacity = crate::clone_write::source_opacity(tree, &read);
                fixes.push(Fix::Unlink(clone, read.resolved, opacity));
            }
        }
        let copies = self.style_copies(&gone, &fixes);
        let mut repair = self.clone_repair.get();
        for (clone, style, opacity) in copies {
            if let Some(meta) = meta_of(tree, clone) {
                crate::clone_overrides::write_own_style(&meta, &style, opacity);
            }
        }
        for fix in fixes {
            match fix {
                Fix::Repoint(clone, link) => {
                    if let Some(meta) = meta_of(tree, clone) {
                        write_clone_link(&meta, link);
                        repair.repointed += 1;
                    }
                }
                Fix::Unlink(clone, mut path, opacity) => {
                    let ids = self.repair_anchor_ids(path.all_anchors().count());
                    for (anchor, id) in path.all_anchors_mut().zip(ids) {
                        anchor.id = id;
                    }
                    if let Some(meta) = meta_of(tree, clone) {
                        unlink_meta(&meta, &path, opacity);
                        repair.unlinked += 1;
                    }
                }
            }
        }
        self.clone_repair.set(repair);
    }

    /// The clones that stay clones, have no own style and lose their style source to the removal,
    /// with the style and `opacity` register they show: they get them as their own, so their look
    /// does not change (ADR 0016 §4, `specs/0057-clone-overrides` criterion 31 (b)).
    fn style_copies(
        &self,
        gone: &HashSet<NodeId>,
        fixes: &[Fix],
    ) -> Vec<(NodeId, crate::style_model::Style, f64)> {
        let unlinked: HashSet<NodeId> = fixes
            .iter()
            .filter_map(|fix| match fix {
                Fix::Unlink(id, ..) => Some(*id),
                Fix::Repoint(..) => None,
            })
            .collect();
        let tree = self.loro().get_tree(crate::document::OBJECTS_TREE);
        let read = self.read_objects();
        read.objects
            .iter()
            .filter_map(|object| match object {
                crate::primitive_model::ObjectSnapshot::Clone(clone)
                    if !clone.chain.own_style
                        && gone.contains(&clone.chain.style_source)
                        && !gone.contains(&clone.id())
                        && !unlinked.contains(&clone.id()) =>
                {
                    let opacity = crate::clone_write::source_opacity(&tree, clone);
                    Some((clone.id(), clone.resolved.style.clone(), opacity))
                }
                _ => None,
            })
            .collect()
    }

    /// What the clone repairs since the last call did, and resets the count: the session reads it
    /// after a command that removes objects, for its notice ("2 clones became paths.").
    pub fn take_clone_repair(&self) -> CloneRepair {
        self.clone_repair.take()
    }

    /// `count` fresh anchor ids for unlinked clones (see [`REPAIR_ID_SPACE`]).
    pub(crate) fn repair_anchor_ids(&self, count: usize) -> Vec<AnchorId> {
        let peer = self.loro().peer_id();
        let ops = self
            .loro()
            .oplog_vv()
            .get(&peer)
            .copied()
            .map_or(0, |n| u64::try_from(n).unwrap_or(0));
        let base = ops.max(self.repair_ids.get());
        let count = count as u64;
        self.repair_ids.set(base + count);
        (base..base + count)
            .map(|n| AnchorId::new(peer, REPAIR_ID_SPACE | n))
            .collect()
    }
}

fn meta_of(tree: &LoroTree, id: NodeId) -> Option<loro::LoroMap> {
    tree.get_meta(TreeID::new(id.peer, id.counter)).ok()
}
