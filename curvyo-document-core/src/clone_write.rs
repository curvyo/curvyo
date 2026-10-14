//! The writes that change a clone node in place: a new link, and the unlink that turns it into
//! an independent path (`specs/0049-linked-clone/adrs.md` decisions 1, 4 and 5), with the scan of
//! every clone link and the limit check that guard them.

use std::collections::HashSet;

use loro::{LoroMap, LoroTree, TreeID};

use crate::affine::{Affine, MATRIX_ENTRY_LIMIT};
use crate::clone_codec::{
    CloneLink, KEY_CLONE_LINK, is_clone_meta, read_clone_link, write_clone_link,
};
use crate::clone_read::{RawLeaf, resolve};
use crate::clone_transform::CloneLinks;
use crate::document::{Document, OBJECTS_TREE};
use crate::path_codec::{self, KEY_ANCHORS, KEY_CLOSED, KEY_ROTATION};
use crate::path_model::{NewAnchor, NodeId, PathSnapshot};
use crate::primitive_model::ObjectSnapshot;
use crate::subpath_codec::{KEY_EXTRA_SUBPATHS, write_extra_subpaths};

/// The largest coordinate a clone's drawn outline may reach after an edit (`0049` criterion 17).
pub(crate) const OUTLINE_LIMIT_MM: f64 = 1e7;

/// Every key a clone node may carry that an unlink replaces: its tag and link, and any outline,
/// style or rotation residue (ADR 0015 §4).
fn keys_replaced_by_unlink() -> impl Iterator<Item = &'static str> {
    [
        crate::shape_codec::KEY_SHAPE,
        KEY_CLONE_LINK,
        KEY_ANCHORS,
        KEY_CLOSED,
        KEY_EXTRA_SUBPATHS,
        KEY_ROTATION,
    ]
    .into_iter()
    .chain(crate::style_codec::ALL_STYLE_KEYS.iter().copied())
}

/// Writes `path` (its outlines, `closed` flags and style; rotation 0) into the clone node `meta`
/// under the same id and tree position, and removes the clone's tag and link: the unlink of
/// criterion 28. `path` carries the anchor ids the unlinked path takes. Uncommitted.
pub(crate) fn unlink_meta(meta: &LoroMap, path: &PathSnapshot) {
    clear_clone_keys(meta);
    path_codec::write_path_style(meta, path.closed, &path.style);
    let list = path_codec::insert_anchors_container(meta);
    for anchor in &path.anchors {
        path_codec::push_anchor(&list, anchor);
    }
    let extras: Vec<(Vec<NewAnchor>, bool)> = path
        .extra_subpaths
        .iter()
        .map(|outline| (outline.anchors.clone(), outline.closed))
        .collect();
    write_extra_subpaths(meta, &extras);
}

/// Removes from the clone node `meta` its tag and link and any outline, style or rotation
/// residue, so the caller can write it as a path. Uncommitted.
pub(crate) fn clear_clone_keys(meta: &LoroMap) {
    for key in keys_replaced_by_unlink() {
        if meta.get(key).is_some() {
            // invariant: deleting a key just confirmed present on an attached map cannot fail.
            #[allow(clippy::unwrap_used)]
            meta.delete(key).unwrap();
        }
    }
}

/// The style an object shows: its own, or for a clone its root's (`0049` criterion 33: a result
/// that takes the base operand's style takes the root's when the base is a clone).
pub(crate) fn leaf_style(tree: &LoroTree, id: NodeId, meta: &LoroMap) -> crate::style_model::Style {
    if is_clone_meta(meta)
        && let Some(clone) = crate::object_read::read_one_clone(tree, id)
    {
        return clone.resolved.style;
    }
    crate::style_codec::read_style(meta)
}

/// Whether `origin` is a live path, compound path or clone, so a link to it may be written back
/// (undo rule (a) of `0049` criterion 36).
pub(crate) fn origin_is_live(tree: &LoroTree, origin: NodeId) -> bool {
    let origin = TreeID::new(origin.peer, origin.counter);
    path_codec::leaf_exists(tree, origin)
        && tree.get_meta(origin).is_ok_and(|origin_meta| {
            !matches!(
                crate::shape_codec::leaf_kind(&origin_meta),
                crate::shape_codec::LeafKind::Primitive(_)
            )
        })
}

/// Writes a clone's new matrix, refusing when its stored origin is no longer `origin` (a peer
/// re-pointed it since the read): the link is written whole with the origin it was computed for.
pub(crate) fn write_matrix(meta: &LoroMap, origin: NodeId, matrix: Affine) -> bool {
    match read_clone_link(meta) {
        Some(link) if link.origin == origin => {
            if link.matrix != matrix {
                write_clone_link(meta, CloneLink { origin, matrix });
            }
            true
        }
        _ => false,
    }
}

impl Document {
    /// The link of every live clone leaf, `(origin, matrix)` by clone id, read from the meta
    /// maps alone (no outline is read): what [`crate::clone_matrices_after`] needs.
    #[must_use]
    pub fn clone_links(&self) -> CloneLinks {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        crate::group_tree::leaves_under(&tree, None)
            .into_iter()
            .filter_map(|leaf| {
                let meta = tree.get_meta(leaf).ok()?;
                if !is_clone_meta(&meta) {
                    return None;
                }
                let link = read_clone_link(&meta)?;
                Some((
                    NodeId::from_parts(leaf.peer, leaf.counter),
                    (link.origin, link.matrix),
                ))
            })
            .collect()
    }

    /// Checks `0049` criterion 17 for a write that replaces the objects in `changed` with these
    /// results: every clone that is changed, or follows a changed object directly or through a
    /// chain, keeps every matrix entry, composed matrix entry and outline coordinate within
    /// ±1e7. Returns the first clone in stacking order that would not. Costs nothing when the
    /// document has no clone.
    pub(crate) fn clone_limits_hold(
        &self,
        links: &CloneLinks,
        changed: &[ObjectSnapshot],
    ) -> Result<(), NodeId> {
        if links.is_empty() {
            return Ok(());
        }
        // The clones that follow a changed object, directly or through a chain, from the links
        // alone (no outline read).
        let mut followers: std::collections::HashMap<NodeId, Vec<NodeId>> =
            std::collections::HashMap::new();
        for (&clone, &(origin, _)) in links {
            followers.entry(origin).or_default().push(clone);
        }
        let by_id: std::collections::HashMap<NodeId, &ObjectSnapshot> =
            changed.iter().map(|object| (object.id(), object)).collect();
        let mut affected: HashSet<NodeId> = HashSet::new();
        let mut frontier: Vec<NodeId> = by_id.keys().copied().collect();
        for id in by_id.keys().filter(|id| links.contains_key(id)) {
            affected.insert(*id);
        }
        while let Some(id) = frontier.pop() {
            for &clone in followers.get(&id).into_iter().flatten() {
                if affected.insert(clone) {
                    frontier.push(clone);
                }
            }
        }
        if affected.is_empty() {
            return Ok(());
        }
        // The leaves their chains need: the clones and their roots, nothing else.
        let mut needed: HashSet<NodeId> = HashSet::new();
        for &clone in &affected {
            let mut current = clone;
            while needed.insert(current) {
                match links.get(&current) {
                    Some(&(origin, _)) => current = origin,
                    None => break,
                }
            }
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let leaves: Vec<RawLeaf> = crate::group_tree::leaves_under(&tree, None)
            .into_iter()
            .filter_map(|leaf| {
                let id = NodeId::from_parts(leaf.peer, leaf.counter);
                if !needed.contains(&id) {
                    return None;
                }
                match by_id.get(&id).copied() {
                    Some(ObjectSnapshot::Path(path)) => Some(RawLeaf::Path(path.clone())),
                    Some(ObjectSnapshot::Clone(clone)) => Some(RawLeaf::Clone(
                        id,
                        CloneLink {
                            origin: clone.chain.origin,
                            matrix: clone.chain.matrix,
                        },
                    )),
                    _ => raw_leaf_of(&tree, leaf),
                }
            })
            .collect();
        for object in resolve(leaves).leaves.iter().flatten() {
            if let ObjectSnapshot::Clone(clone) = object
                && affected.contains(&clone.id())
                && !clone_within_limits(clone)
            {
                self.clone_refusal.set(Some(clone.id()));
                return Err(clone.id());
            }
        }
        Ok(())
    }
}

impl Document {
    /// The clone named by the last refusal of `0049` criterion 17, once: the session reads it after
    /// a command for its notice. The refusing command itself returns the same id.
    pub fn take_clone_refusal(&self) -> Option<NodeId> {
        self.clone_refusal.take()
    }

    /// Criterion 17 for a node edit of `path`: `edit` is applied to a copy of the path as it is
    /// stored, and the clones that follow it are checked against the limits. Reads nothing when no
    /// clone follows the path.
    pub(crate) fn path_edit_limits(
        &self,
        path: NodeId,
        edit: impl FnOnce(&mut PathSnapshot),
    ) -> Result<(), crate::path_model::PathEditError> {
        let links = self.clone_links();
        if !links.values().any(|&(origin, _)| origin == path) {
            return Ok(());
        }
        let Some(mut edited) = self.path(path) else {
            return Ok(());
        };
        edit(&mut edited);
        self.clone_limits_hold(&links, &[ObjectSnapshot::Path(edited)])
            .map_err(crate::path_model::PathEditError::CloneOutOfRange)
    }
}

fn raw_leaf_of(tree: &LoroTree, leaf: TreeID) -> Option<RawLeaf> {
    crate::object_read::raw_leaf(tree, leaf)
}

/// Whether a resolved clone's matrix, composed matrix and outline stay within the limits of
/// criterion 17.
fn clone_within_limits(clone: &crate::clone_model::CloneSnapshot) -> bool {
    let within = |v: f64| v.is_finite() && v.abs() <= OUTLINE_LIMIT_MM;
    clone.chain.matrix.is_within_limit()
        && clone
            .chain
            .composed
            .to_array()
            .iter()
            .all(|v| v.is_finite() && v.abs() <= MATRIX_ENTRY_LIMIT)
        && clone
            .resolved
            .all_anchors()
            .all(|a| within(a.point.x) && within(a.point.y))
}

#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::path_model::{AnchorId, NewAnchor};
    use crate::units::{Point, Vec2};

    fn square(d: &Document, n: u64, x: f64) -> crate::path_model::NodeId {
        let c = |k: u64, dx: f64, dy: f64| {
            NewAnchor::corner(AnchorId::new(1, n * 10 + k), Point::new(x + dx, dy))
        };
        d.create_path(&[c(0, 0.0, 0.0), c(1, 5.0, 0.0), c(2, 5.0, 5.0)], true)
    }

    /// Review F2: the limit check of a move reads the roots of the affected clones only.
    #[test]
    fn a_nudge_reads_only_the_roots_its_clones_need() {
        let d = Document::new(1);
        let root = square(&d, 1, 0.0);
        let _clone = d.linked_clone(&[root], &[]).unwrap()[0];
        let paths: Vec<_> = (2..1_002_u32)
            .map(|n| square(&d, u64::from(n), f64::from(n) * 10.0))
            .collect();
        let before = crate::object_read::path_reads();
        d.translate_objects(&[paths[0]], Vec2::new(1.0, 0.0))
            .unwrap();
        assert_eq!(
            crate::object_read::path_reads() - before,
            0,
            "no clone follows it"
        );
        let before = crate::object_read::path_reads();
        d.translate_objects(&[root], Vec2::new(1.0, 0.0)).unwrap();
        assert!(
            crate::object_read::path_reads() - before <= 2,
            "the root, and no other path"
        );
    }
}
