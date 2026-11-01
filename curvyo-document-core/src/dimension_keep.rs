//! Keeps each linked dimension's last measured value current in the one commit funnel
//! (ADR 0027 §4), so a step that removes what it measures leaves the value of the step before.

use std::collections::HashMap;

use crate::dimension_codec::{is_dimension_meta, read_dimension_snapshot, write_kept};
use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::{leaves_under, node_id};
use crate::path_model::NodeId;
use crate::primitive_model::ObjectSnapshot;

/// A kept point or value that moved less than this (mm or rad) is not rewritten.
const KEPT_TOLERANCE: crate::units::Tolerance = crate::units::Tolerance::from_mm(1e-9);

impl Document {
    /// Makes the next commit look for dimensions again: a writer that may add one calls it.
    pub(crate) fn forget_dimension_scan(&self) {
        *self.commit_state.no_dimensions_at.borrow_mut() = None;
    }

    /// Writes, without committing, the measured points and value of every linked dimension whose
    /// resolved state differs from its kept one by more than [`KEPT_TOLERANCE`], and returns
    /// whether the document has no dimension. When this session's last commit found none and
    /// nothing was committed or merged since, nothing is read; else each leaf's `shape` tag is.
    pub(crate) fn refresh_dimension_kept(&self) -> bool {
        if self.dimension_free_since_last_commit() {
            return true;
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let dimensions: Vec<(NodeId, loro::LoroMap)> = leaves_under(&tree, None)
            .into_iter()
            .filter_map(|leaf| {
                let meta = tree.get_meta(leaf).ok()?;
                is_dimension_meta(&meta).then(|| (node_id(leaf), meta))
            })
            .collect();
        if dimensions.is_empty() {
            return true;
        }
        let mut referents: HashMap<NodeId, Option<ObjectSnapshot>> = HashMap::new();
        for (id, meta) in dimensions {
            let Some(dimension) = read_dimension_snapshot(id, &meta) else {
                continue;
            };
            for node in dimension.slots.nodes() {
                referents.entry(node).or_insert_with(|| self.object(node));
            }
            let measured = crate::dimension_read::resolve(&dimension, |node| {
                referents.get(&node).and_then(Option::as_ref)
            });
            if let Some(measured) = measured
                && measured.differs_from(&dimension.kept, KEPT_TOLERANCE)
            {
                write_kept(&meta, &measured);
            }
        }
        false
    }
}
