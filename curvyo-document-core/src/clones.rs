//! The commands of linked clones (`specs/0049-linked-clone`): Linked clone (criteria 1 to 3 and
//! 52), Unlink clone (criteria 28, 29 and 55) and the orphan repair after an Open or a merge
//! (criterion 35), one commit each.

use std::collections::HashSet;

use loro::{LoroTree, TreeID};
use thiserror::Error;

use crate::affine::Affine;
use crate::clone_codec::{CloneLink, is_clone_meta, write_clone};
use crate::clone_model::MAX_CLONE_DEPTH;
use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::is_group_node;
use crate::object_read::read_one_clone;
use crate::path_codec::node_exists;
use crate::path_model::{NewAnchor, NodeId};
use crate::paths::tree_id_of;
use crate::shape_codec::read_shape_tag;

/// The commit label of Linked clone (ADR 0002 §9).
pub const LINKED_CLONE_COMMIT_LABEL: &str = "linked_clone";

/// The commit label of Unlink clone (ADR 0002 §9).
pub const UNLINK_CLONE_COMMIT_LABEL: &str = "unlink_clone";

/// The commit label of the orphan repair (ADR 0002 §9).
pub const REMOVE_ORPHAN_CLONES_COMMIT_LABEL: &str = "remove_orphan_clones";

/// Why a clone command refused. Nothing is written in any case.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CloneError {
    /// A source no longer exists.
    #[error("an object to clone no longer exists")]
    NoSuchObject,
    /// A source is a group, or a clone whose original cannot be found (`0049` criterion 4).
    #[error("only paths, compound paths, shapes and clones can be cloned")]
    NotClonable,
    /// The primitives among the sources and the given conversions are not the same set, or a
    /// conversion names an object that is not a primitive.
    #[error("every shape to clone needs exactly one conversion to a path")]
    ConversionMismatch,
    /// These selected clones are already [`MAX_CLONE_DEPTH`] deep (`0049` criterion 52).
    #[error("linked clones can follow each other {MAX_CLONE_DEPTH} levels deep")]
    TooDeep(Vec<NodeId>),
    /// An object to unlink is not a clone, or its original cannot be found.
    #[error("only a linked clone can be unlinked")]
    NotAClone,
    /// The fresh anchor ids of an unlink are not one per anchor of the clone's outline, all
    /// different.
    #[error("the fresh anchor ids do not match the clone's anchors")]
    AnchorIds,
}

/// What one source is, checked before any write.
enum Source {
    Path,
    Primitive,
    Clone(usize),
}

impl Document {
    /// Makes one linked clone of each of `sources`, in **one commit** labelled
    /// [`LINKED_CLONE_COMMIT_LABEL`], and returns the new clones in the order of `sources` (a
    /// source named twice is cloned once). Each clone has the identity matrix, follows its source
    /// and sits directly above it, in the same group. A primitive source is first converted to a
    /// path in the same commit by `conversions` (its anchors as "Object to path" makes them, ids
    /// minted by the caller; `0049` criterion 2); a clone source gets a clone of the clone.
    ///
    /// # Errors
    /// [`CloneError::NoSuchObject`] for a source that does not exist;
    /// [`CloneError::NotClonable`] for a group or an orphan clone;
    /// [`CloneError::ConversionMismatch`] when `conversions` does not name exactly the primitive
    /// sources; [`CloneError::TooDeep`] with every source clone at depth [`MAX_CLONE_DEPTH`] or
    /// more. All are checked before the first write.
    pub fn linked_clone(
        &self,
        sources: &[NodeId],
        conversions: &[(NodeId, Vec<NewAnchor>)],
    ) -> Result<Vec<NodeId>, CloneError> {
        let mut seen = HashSet::with_capacity(sources.len());
        let sources: Vec<NodeId> = sources
            .iter()
            .copied()
            .filter(|s| seen.insert(*s))
            .collect();
        if sources.is_empty() {
            return Ok(Vec::new());
        }
        let kinds = self.classify_sources(&sources)?;
        let too_deep: Vec<NodeId> = sources
            .iter()
            .zip(&kinds)
            .filter(|(_, kind)| matches!(kind, Source::Clone(depth) if *depth >= MAX_CLONE_DEPTH))
            .map(|(id, _)| *id)
            .collect();
        if !too_deep.is_empty() {
            return Err(CloneError::TooDeep(too_deep));
        }
        let primitives: HashSet<NodeId> = sources
            .iter()
            .zip(&kinds)
            .filter(|(_, kind)| matches!(kind, Source::Primitive))
            .map(|(id, _)| *id)
            .collect();
        let converted: HashSet<NodeId> = conversions.iter().map(|(id, _)| *id).collect();
        if converted != primitives || converted.len() != conversions.len() {
            return Err(CloneError::ConversionMismatch);
        }
        if !conversions.is_empty() {
            self.convert_to_paths_uncommitted(conversions)
                .map_err(|_| CloneError::ConversionMismatch)?;
        }
        let created = sources
            .iter()
            .map(|&source| self.create_clone_above(source))
            .collect();
        self.commit_with_label(LINKED_CLONE_COMMIT_LABEL);
        Ok(created)
    }

    /// Turns each clone of `unlinks` into an independent path, in **one commit** labelled
    /// [`UNLINK_CLONE_COMMIT_LABEL`] (`0049` criterion 28): the same id and place, the anchors of
    /// its resolved outline with the given fresh ids (one per anchor, all outlines, in order;
    /// the caller mints them), its effective style and its style source's `opacity` register
    /// (`0057` criterion 30), no override, rotation 0. Clones that follow an unlinked
    /// clone keep following it (criterion 55). Everything is checked before the first write.
    ///
    /// # Errors
    /// [`CloneError::NoSuchObject`] for an id that does not exist; [`CloneError::NotAClone`] for
    /// one that is not a resolvable clone; [`CloneError::AnchorIds`] when the ids do not fit.
    pub fn unlink_clones(&self, unlinks: &[crate::CopySource]) -> Result<(), CloneError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut planned = Vec::with_capacity(unlinks.len());
        let mut fresh = HashSet::new();
        for unlink in unlinks {
            let tree_id = tree_id_of(unlink.id);
            if !crate::path_codec::leaf_exists(&tree, tree_id) {
                return Err(CloneError::NoSuchObject);
            }
            let meta = tree
                .get_meta(tree_id)
                .map_err(|_| CloneError::NoSuchObject)?;
            if !is_clone_meta(&meta) {
                return Err(CloneError::NotAClone);
            }
            let clone = read_one_clone(&tree, unlink.id).ok_or(CloneError::NotAClone)?;
            let opacity = crate::clone_write::source_opacity(&tree, &clone);
            let mut path = clone.resolved;
            if path.all_anchors().count() != unlink.anchor_ids.len()
                || !unlink.anchor_ids.iter().all(|id| fresh.insert(*id))
            {
                return Err(CloneError::AnchorIds);
            }
            for (anchor, id) in path.all_anchors_mut().zip(&unlink.anchor_ids) {
                anchor.id = *id;
            }
            planned.push((meta, path, opacity));
        }
        // Clones that follow an unlinked clone read it before it changes, and keep following it.
        for (meta, path, opacity) in &planned {
            crate::clone_write::unlink_meta(meta, path, *opacity);
        }
        self.commit_with_label(UNLINK_CLONE_COMMIT_LABEL);
        Ok(())
    }

    /// Repairs every orphan clone (`0049` criterion 35) in one commit labelled
    /// [`REMOVE_ORPHAN_CLONES_COMMIT_LABEL`], none when there is no orphan. A clone with no link
    /// whose stored outline reads as a path (merge residue, criterion 38) becomes that path first;
    /// then every clone that is still an orphan is deleted: one whose chain does not end in a live
    /// path or compound path, or whose outline is not finite, and every clone behind one. The sets
    /// depend only on the state, so peers that repair one state write the same. Returns how many
    /// were deleted; the clones that became paths are counted in [`Document::take_clone_repair`].
    pub fn remove_orphan_clones(&self) -> usize {
        let orphans = self.read_objects().orphans;
        if orphans.is_empty() {
            return 0;
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let converted = convert_linkless_outlines(&tree, &orphans);
        let doomed: Vec<TreeID> = if converted == 0 {
            orphans.iter().map(|&id| tree_id_of(id)).collect()
        } else {
            let mut repair = self.clone_repair.get();
            repair.unlinked += converted;
            self.clone_repair.set(repair);
            self.read_objects()
                .orphans
                .iter()
                .map(|&id| tree_id_of(id))
                .collect()
        };
        let parents = crate::group_tree::parents_of(&tree, &doomed);
        for &node in &doomed {
            let _ = tree.delete(node);
        }
        crate::groups::remove_emptied_groups(&tree, &parents);
        self.commit_with_label(REMOVE_ORPHAN_CLONES_COMMIT_LABEL);
        doomed.len()
    }

    fn classify_sources(&self, sources: &[NodeId]) -> Result<Vec<Source>, CloneError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        sources
            .iter()
            .map(|&id| {
                let tree_id = tree_id_of(id);
                if !node_exists(&tree, tree_id) {
                    return Err(CloneError::NoSuchObject);
                }
                if is_group_node(&tree, tree_id) {
                    return Err(CloneError::NotClonable);
                }
                let meta = tree
                    .get_meta(tree_id)
                    .map_err(|_| CloneError::NoSuchObject)?;
                if crate::text_codec::is_text_meta(&meta) {
                    return Err(CloneError::NotClonable);
                }
                if is_clone_meta(&meta) {
                    let clone = read_one_clone(&tree, id).ok_or(CloneError::NotClonable)?;
                    Ok(Source::Clone(clone.chain.depth))
                } else if read_shape_tag(&meta).is_some() {
                    Ok(Source::Primitive)
                } else {
                    Ok(Source::Path)
                }
            })
            .collect()
    }

    /// Creates a clone of `source` with the identity matrix directly above it. Uncommitted.
    fn create_clone_above(&self, source: NodeId) -> NodeId {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        // `mov_after` needs the fractional index on for this tree, as `duplicate_objects` does.
        tree.enable_fractional_index(0);
        // invariant: creating a node under the root of a live tree cannot fail, nor reading the
        // meta map of the node just created, nor moving it next to a source checked live above.
        #[allow(clippy::unwrap_used)]
        let created: TreeID = tree.create(loro::TreeParentId::Root).unwrap();
        #[allow(clippy::unwrap_used)]
        let meta = tree.get_meta(created).unwrap();
        write_clone(
            &meta,
            CloneLink {
                origin: source,
                matrix: Affine::IDENTITY,
            },
        );
        #[allow(clippy::unwrap_used)]
        tree.mov_after(created, tree_id_of(source)).unwrap();
        NodeId::from_parts(created.peer, created.counter)
    }
}

/// Turns each orphan among `orphans` that is a clone with no link and a stored outline that reads
/// as a path into that path, by removing its `shape` (`adrs.md` decision 6, amended 2026-10-14).
/// Uncommitted. Returns how many it turned.
fn convert_linkless_outlines(tree: &LoroTree, orphans: &[NodeId]) -> usize {
    let mut converted = 0;
    for &id in orphans {
        let Ok(meta) = tree.get_meta(tree_id_of(id)) else {
            continue;
        };
        if is_clone_meta(&meta)
            && meta.get(crate::clone_codec::KEY_CLONE_LINK).is_none()
            && crate::path_codec::validate_path_node(&meta)
            && meta.delete(crate::shape_codec::KEY_SHAPE).is_ok()
        {
            converted += 1;
        }
    }
    converted
}
