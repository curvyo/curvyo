//! The writes of the held-back rule (ADR 0016 §2, `specs/0057-clone-overrides` criteria 3, 7
//! and 8): when a transform writer maps objects, every unselected clone with own placement whose
//! original moves gets the matrix that keeps its drawn outline, or is unlinked in place when that
//! matrix cannot exist.
//!
//! Every transform writer plans and writes it here, in its own commit: today
//! `translate_objects`, `rotate_object`, `resize_path` and `transform_objects`. Each one is listed
//! in `every_writer` of `tests/clone_held_back.rs`; a new transform writer must call this module
//! and add itself there, or it silently moves held clones.

use std::collections::HashSet;

use loro::{LoroTree, TreeID};

use crate::affine::Affine;
use crate::clone_codec::{is_clone_meta, read_overrides};
use crate::clone_transform::{CloneLinks, CloneMatrices, clone_matrices_after_maps};
use crate::document::{Document, OBJECTS_TREE};
use crate::object_read::read_one_clone;
use crate::objects::ObjectEditError;
use crate::path_model::{NodeId, PathSnapshot};
use crate::primitive_model::ObjectSnapshot;

/// What the held-back rule did in the last transform, for its notices (`0057` criteria 7 and 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CloneHold {
    /// Clones with own placement that stayed in place.
    pub held: usize,
    /// Clones with own placement that became paths because the map collapsed their original.
    pub unlinked: usize,
}

/// The held-back writes of one transform, decided before any write.
#[derive(Debug, Default)]
pub(crate) struct HoldPlan {
    /// `(clone, origin, new matrix)`.
    matrices: Vec<(NodeId, NodeId, Affine)>,
    /// `(clone, the path it becomes, its style source's opacity)`.
    unlinks: Vec<(NodeId, PathSnapshot, f64)>,
}

impl Document {
    /// The clones with own placement, read from the meta maps alone. Empty, and only the leaves
    /// read, in a document without clones.
    #[must_use]
    pub fn own_placement_clones(&self) -> HashSet<NodeId> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        crate::group_tree::leaves_under(&tree, None)
            .into_iter()
            .filter_map(|leaf| {
                let meta = tree.get_meta(leaf).ok()?;
                (is_clone_meta(&meta) && read_overrides(&meta).own_placement)
                    .then(|| NodeId::from_parts(leaf.peer, leaf.counter))
            })
            .collect()
    }

    /// What the held-back rule did since the last call, and resets the count: the session reads
    /// it after a transform for its notice.
    pub fn take_clone_hold(&self) -> CloneHold {
        self.clone_hold.take()
    }

    /// The held-back writes of `decided`, the result of the one rule for a transform
    /// (`links` is [`Document::clone_links`]).
    ///
    /// # Errors
    /// The first clone, in id order, whose new matrix would pass ±1e7 (`0049` criterion 17);
    /// nothing may then be written. The id is also kept for [`Document::take_clone_refusal`].
    pub(crate) fn plan_hold(
        &self,
        links: &CloneLinks,
        decided: &CloneMatrices,
    ) -> Result<HoldPlan, NodeId> {
        let mut plan = HoldPlan::default();
        for &(clone, matrix) in &decided.held_back {
            if !matrix.is_within_limit() {
                self.clone_refusal.set(Some(clone));
                return Err(clone);
            }
            if links[&clone].1 != matrix {
                plan.matrices.push((clone, links[&clone].0, matrix));
            }
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        for &clone in &decided.held_unlinked {
            if let Some(read) = read_one_clone(&tree, clone) {
                let opacity = crate::clone_write::source_opacity(&tree, &read);
                plan.unlinks.push((clone, read.resolved, opacity));
            }
        }
        Ok(plan)
    }

    /// The held-back writes when the objects of `results` replace the stored ones: each result's
    /// change is read back as an affine map of its drawn outline ([`crate::outline_map`]); a
    /// result that is no such map moves no held clone (it is an edit, not a transform). Nothing
    /// is read when no clone has own placement.
    pub(crate) fn hold_for_results(
        &self,
        links: &CloneLinks,
        results: &[ObjectSnapshot],
    ) -> Result<HoldPlan, ObjectEditError> {
        let held = if links.is_empty() {
            HashSet::new()
        } else {
            self.own_placement_clones()
        };
        if held.is_empty() {
            return Ok(HoldPlan::default());
        }
        let drawn = |object: &ObjectSnapshot| match object {
            ObjectSnapshot::Path(path) => Some(path.clone()),
            ObjectSnapshot::Sketch(sketch) => Some(sketch.resolved.clone()),
            ObjectSnapshot::Clone(clone) => Some(clone.resolved.clone()),
            ObjectSnapshot::Primitive(_) | ObjectSnapshot::Text(_) | ObjectSnapshot::Image(_) => {
                None
            }
        };
        let moved: Vec<(NodeId, Affine)> = results
            .iter()
            .filter_map(|result| {
                let before = drawn(&self.object(result.id())?)?;
                let after = drawn(result)?;
                Some((
                    result.id(),
                    crate::outline_map::outline_map(&before, &after)?,
                ))
            })
            .collect();
        let decided = clone_matrices_after_maps(links, &held, &moved);
        self.plan_hold(links, &decided)
            .map_err(ObjectEditError::CloneOutOfRange)
    }

    /// Writes `plan`, uncommitted, in the commit of the transform, and records its counts.
    pub(crate) fn write_hold(&self, tree: &LoroTree, plan: HoldPlan) {
        let mut hold = CloneHold::default();
        for (clone, origin, matrix) in plan.matrices {
            if let Some(meta) = meta_of(tree, clone)
                && crate::clone_write::write_matrix(&meta, origin, matrix)
            {
                hold.held += 1;
            }
        }
        for (clone, mut path, opacity) in plan.unlinks {
            let ids = self.repair_anchor_ids(path.all_anchors().count());
            for (anchor, id) in path.all_anchors_mut().zip(ids) {
                anchor.id = id;
            }
            if let Some(meta) = meta_of(tree, clone) {
                crate::clone_write::unlink_meta(&meta, &path, opacity);
                hold.unlinked += 1;
            }
        }
        self.clone_hold.set(hold);
    }
}

fn meta_of(tree: &LoroTree, id: NodeId) -> Option<loro::LoroMap> {
    tree.get_meta(TreeID::new(id.peer, id.counter)).ok()
}
