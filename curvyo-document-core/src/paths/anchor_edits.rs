//! `Document`'s node-selection commands that take `(path, anchor)` pairs across any number of
//! paths and end in one commit (`specs/0053-node-marquee-selection/adrs.md` decision 4).

use std::collections::HashMap;

use loro::{LoroMap, LoroMovableList};

use crate::document::{Document, OBJECTS_TREE};
use crate::path_codec::{
    KEY_HANDLE_IN, KEY_HANDLE_OUT, KEY_POINT, anchor_map_at, neighbour_tangent, read_closed,
    read_kind, read_point, read_vec2, write_kind, write_vec2,
};
use crate::path_model::{AnchorId, AnchorKind, NodeId, PathEditError};
use crate::smooth_handles::smooth_corner_handles;
use crate::subpath_codec::{AnchorPositions, anchor_positions, remove_outlines};

use super::{DEFAULT_HANDLE_LENGTH_MM, tree_id_of};

/// The `(path, anchor)` pairs of a call grouped by path, each path once, in the order it first
/// appears in the call, with its anchors in their call order.
fn group_by_path(anchors: &[(NodeId, AnchorId)]) -> Vec<(NodeId, Vec<AnchorId>)> {
    let mut slot_of: HashMap<NodeId, usize> = HashMap::new();
    let mut groups: Vec<(NodeId, Vec<AnchorId>)> = Vec::new();
    for &(path, anchor) in anchors {
        let slot = *slot_of.entry(path).or_insert_with(|| {
            groups.push((path, Vec::new()));
            groups.len() - 1
        });
        groups[slot].1.push(anchor);
    }
    groups
}

impl Document {
    /// Converts every named anchor to `kind` as **one commit**, whichever paths the anchors are
    /// on: a node selection's convert is one interaction, not one per path
    /// (`specs/0002-path-node-editing/specification.md` criterion 11,
    /// `specs/0053-node-marquee-selection/` criterion 20).
    ///
    /// Every pair is resolved before any anchor is written, so one stale path or anchor id
    /// anywhere in `anchors` refuses the whole call rather than converting a part of it.
    ///
    /// Per-anchor rule (`specs/0006-path-merge-split-and-node-types/adrs.md`'s conversion
    /// table):
    /// - **to the kind the anchor already has: no write at all** (`0006` criterion 16).
    /// - **to [`AnchorKind::Symmetric`]** (from any other kind): both handles reset to
    ///   `DEFAULT_HANDLE_LENGTH_MM`, collinear through the anchor along the chord between its
    ///   neighbours on its own path.
    /// - **[`AnchorKind::Corner`] → [`AnchorKind::Asymmetric`]**: same tangent direction, each
    ///   side keeps its own current length if it was non-zero, otherwise the default.
    /// - **every other conversion**: both handles stay where they are; only `kind` changes
    ///   (`specs/0002-path-node-editing/adrs.md` decision 3).
    ///
    /// If every named anchor already has `kind`, or `anchors` is empty, nothing is written and
    /// no commit happens.
    ///
    /// Each anchor is found in whichever outline of its path holds it, and its neighbours are
    /// those of that outline (`specs/0055-compound-path-node-editing/` criterion 3).
    ///
    /// # Errors
    /// [`PathEditError::NoSuchPath`] if any named path no longer exists;
    /// [`PathEditError::NoSuchAnchor`] if any named anchor no longer exists on its path.
    pub fn convert_anchor_kind(
        &self,
        anchors: &[(NodeId, AnchorId)],
        kind: AnchorKind,
    ) -> Result<(), PathEditError> {
        let mut resolved: Vec<(LoroMovableList, bool, usize)> = Vec::new();
        for (path, ids) in group_by_path(anchors) {
            let (meta, _) = self.path_parts(path)?;
            let positions = anchor_positions(&meta);
            for id in ids {
                let at = positions.at(id).ok_or(PathEditError::NoSuchAnchor)?;
                resolved.push((at.list, read_closed(&at.map), at.index));
            }
        }
        let mut changed = false;
        for (list, closed, index) in &resolved {
            changed |= convert_one(list, *closed, *index, kind);
        }
        if changed {
            self.commit_with_label("convert_anchor_kind");
        }
        Ok(())
    }

    /// Removes the named anchors, whichever paths they are on, as **one commit**
    /// (`specs/0053-node-marquee-selection/` criterion 18). On each path the rule of
    /// `specs/0002-path-node-editing/` criterion 13 applies: the remaining anchors keep their
    /// own handles, so each pair that becomes adjacent is joined by one segment from them, and
    /// a path left with fewer than two anchors is removed as a whole. A group left empty by
    /// those removals is removed in the same commit (`specs/0023-groups/` criterion 14); a
    /// layer is not (`specs/0050-layers-and-objects-panel/adrs.md` decision 2).
    ///
    /// Anchor ids that no longer exist are ignored rather than refused: a local selection can
    /// hold ids a collaborator has already deleted, and ADR 0009 §2 resolves that case lazily.
    /// A pair whose path no longer exists is skipped the same way: that path's nodes are
    /// already gone, so the Delete still removes the nodes of the live paths, in one commit.
    /// An empty `anchors`, or one whose pairs are all stale, writes nothing and commits
    /// nothing. So this command has no error case.
    ///
    /// On a compound path the rule applies per outline (`specs/0055-compound-path-node-editing/`
    /// criterion 4): an outline left with fewer than two anchors is removed (the next outline
    /// takes the first one's place), a compound path left with one outline is an ordinary path
    /// with the same id, style and place, and a path left with no outline is removed.
    ///
    /// # Panics
    /// Does not panic in practice: every index this deletes was just confirmed present in the
    /// same movable list, and every removed path was just confirmed to exist.
    pub fn delete_anchors(&self, anchors: &[(NodeId, AnchorId)]) {
        let mut resolved: Vec<(NodeId, LoroMap, OutlineCuts)> = Vec::new();
        for (path, ids) in group_by_path(anchors) {
            let Ok((meta, _)) = self.path_parts(path) else {
                continue;
            };
            let cuts = outline_cuts(&anchor_positions(&meta), &ids);
            if !cuts.is_empty() {
                resolved.push((path, meta, cuts));
            }
        }
        if resolved.is_empty() {
            return;
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut removed = Vec::new();
        for (path, meta, cuts) in resolved {
            if cuts.removes_every_outline() {
                removed.push(tree_id_of(path));
                continue;
            }
            let mut doomed = Vec::new();
            for (outline, list, indices) in cuts.outlines {
                if list.len().saturating_sub(indices.len()) < 2 {
                    doomed.push(outline);
                    continue;
                }
                // Delete from the highest index down so earlier indices stay valid.
                for index in indices.into_iter().rev() {
                    // invariant: `index` was found in this same list just above.
                    #[allow(clippy::unwrap_used)]
                    list.delete(index, 1).unwrap();
                }
            }
            remove_outlines(&meta, &doomed);
            if !doomed.is_empty() {
                // A text on a later outline stays on it.
                crate::text_carrier::renumber_carried_outlines(&tree, path, &|old| {
                    let before = doomed.iter().filter(|&&gone| gone < old as usize).count();
                    old - u32::try_from(before).unwrap_or(old)
                });
            }
        }
        if !removed.is_empty() {
            // A clone of a removed path keeps its look (`specs/0049-linked-clone/`).
            self.repair_clones_of_removed(&tree, &removed);
            let parents = crate::group_tree::parents_of(&tree, &removed);
            for tree_id in removed {
                // invariant: `path_parts` above confirmed this node exists, and each path
                // appears once in `resolved`.
                #[allow(clippy::unwrap_used)]
                tree.delete(tree_id).unwrap();
            }
            crate::groups::remove_emptied_groups(&tree, &parents);
        }
        self.commit_with_label("delete_anchors");
    }
}

/// What a Delete takes from one path: per outline that loses a node, its number, its list and the
/// sorted indices of the nodes it loses; and how many outlines the path has.
struct OutlineCuts {
    outlines: Vec<(usize, LoroMovableList, Vec<usize>)>,
    outline_count: usize,
}

impl OutlineCuts {
    fn is_empty(&self) -> bool {
        self.outlines.is_empty()
    }

    /// Whether every outline is left with fewer than two nodes: the object goes.
    fn removes_every_outline(&self) -> bool {
        self.outlines.len() == self.outline_count
            && self
                .outlines
                .iter()
                .all(|(_, list, indices)| list.len().saturating_sub(indices.len()) < 2)
    }
}

/// The anchors of `ids` that still exist, grouped by the outline that holds them.
fn outline_cuts(positions: &AnchorPositions, ids: &[AnchorId]) -> OutlineCuts {
    let mut by_outline: HashMap<usize, Vec<usize>> = HashMap::new();
    for &id in ids {
        if let Some(at) = positions.at(id) {
            by_outline.entry(at.outline).or_default().push(at.index);
        }
    }
    let mut outlines: Vec<(usize, LoroMovableList, Vec<usize>)> = by_outline
        .into_iter()
        .map(|(outline, mut indices)| {
            indices.sort_unstable();
            indices.dedup();
            (outline, positions.list(outline).clone(), indices)
        })
        .collect();
    outlines.sort_unstable_by_key(|(outline, ..)| *outline);
    OutlineCuts {
        outlines,
        outline_count: positions.outline_count(),
    }
}

/// Converts the anchor at `index` of `list` to `kind` by the rule of
/// [`Document::convert_anchor_kind`]. Returns whether anything was written.
fn convert_one(list: &LoroMovableList, closed: bool, index: usize, kind: AnchorKind) -> bool {
    let map = anchor_map_at(list, index);
    let current = read_kind(&map);
    if current == kind {
        return false;
    }
    match kind {
        AnchorKind::Symmetric => {
            let point = read_point(&map, KEY_POINT);
            let tangent = neighbour_tangent(list, closed, index, point)
                .normalized_to(DEFAULT_HANDLE_LENGTH_MM);
            write_vec2(&map, KEY_HANDLE_OUT, tangent);
            write_vec2(&map, KEY_HANDLE_IN, tangent.negated());
        }
        AnchorKind::Asymmetric if current == AnchorKind::Corner => {
            let point = read_point(&map, KEY_POINT);
            let handle_in = read_vec2(&map, KEY_HANDLE_IN);
            let handle_out = read_vec2(&map, KEY_HANDLE_OUT);
            let tangent = neighbour_tangent(list, closed, index, point);
            let (new_in, new_out) = smooth_corner_handles(tangent, handle_in, handle_out);
            write_vec2(&map, KEY_HANDLE_OUT, new_out);
            write_vec2(&map, KEY_HANDLE_IN, new_in);
        }
        // `Symmetric`→`Corner`, `Asymmetric`→`Corner`, `Symmetric`→`Asymmetric`:
        // shape-preserving, kind only.
        AnchorKind::Corner | AnchorKind::Asymmetric => {}
    }
    write_kind(&map, kind);
    true
}

#[cfg(test)]
mod tests {
    use crate::document::Document;
    use crate::path_model::{AnchorId, AnchorKind, NewAnchor, NodeId, PathEditError};
    use crate::units::Point;

    fn line(document: &Document, peer_tag: u64, xs: &[f64]) -> NodeId {
        let anchors: Vec<NewAnchor> = xs
            .iter()
            .enumerate()
            .map(|(i, &x)| NewAnchor::corner(AnchorId::new(peer_tag, i as u64), Point::new(x, 0.0)))
            .collect();
        document.create_path(&anchors, false)
    }

    fn id(peer_tag: u64, index: u64) -> AnchorId {
        AnchorId::new(peer_tag, index)
    }

    fn xs(document: &Document, path: NodeId) -> Vec<f64> {
        document
            .path(path)
            .expect("path exists")
            .anchors
            .iter()
            .map(|a| a.point.x)
            .collect()
    }

    /// `0053` criterion 18, first example: a marquee over the three middle nodes of a
    /// five-node line, then Delete, leaves a two-node path from (0, 0) to (40, 0).
    #[test]
    fn deleting_the_inner_nodes_leaves_the_two_ends() {
        let document = Document::new(1);
        let p = line(&document, 7, &[0.0, 10.0, 20.0, 30.0, 40.0]);
        document.delete_anchors(&[(p, id(7, 1)), (p, id(7, 2)), (p, id(7, 3))]);
        assert_eq!(xs(&document, p), vec![0.0, 40.0]);
    }

    /// `0053` criterion 18, third example: nodes of two paths, one Delete, one step, both paths
    /// changed.
    #[test]
    fn a_delete_across_two_paths_is_one_step() {
        let document = Document::new(1);
        let p = line(&document, 7, &[0.0, 10.0, 20.0]);
        let q = line(&document, 8, &[0.0, 10.0, 20.0]);
        let before = document.own_step_count();
        document.delete_anchors(&[(p, id(7, 1)), (q, id(8, 0))]);
        assert_eq!(document.own_step_count(), before + 1);
        assert_eq!(document.own_steps().last().unwrap().label, "delete_anchors");
        assert_eq!(xs(&document, p), vec![0.0, 20.0]);
        assert_eq!(xs(&document, q), vec![10.0, 20.0]);
    }

    /// `0053` criterion 18, second example, across paths: every node of one path and one node of
    /// another; the first path is removed, the second changed, in the same step.
    #[test]
    fn a_path_left_below_two_nodes_is_removed_in_the_same_step() {
        let document = Document::new(1);
        let p = line(&document, 7, &[0.0, 10.0]);
        let q = line(&document, 8, &[0.0, 10.0, 20.0]);
        let before = document.own_step_count();
        document.delete_anchors(&[(p, id(7, 0)), (q, id(8, 2)), (p, id(7, 1))]);
        assert_eq!(document.own_step_count(), before + 1);
        assert_eq!(document.path(p), None);
        assert_eq!(xs(&document, q), vec![0.0, 10.0]);
        assert_eq!(document.object_ids(), vec![q]);
    }

    /// `0053` criterion 18 with `0023` criterion 14: a group whose every path loses its nodes
    /// is removed in the same commit.
    #[test]
    fn a_group_emptied_by_a_cross_path_delete_is_removed() {
        let document = Document::new(1);
        let p = line(&document, 7, &[0.0, 10.0]);
        let q = line(&document, 8, &[0.0, 10.0]);
        let group = document.group_objects(&[p, q]).unwrap();
        let before = document.own_step_count();
        document.delete_anchors(&[(p, id(7, 0)), (q, id(8, 0))]);
        assert_eq!(document.own_step_count(), before + 1);
        assert!(!document.contains_node(group));
        assert_eq!(document.object_ids(), Vec::new());
    }

    /// Undo of a cross-path delete brings every removed path and node back (ADR 0014).
    #[test]
    fn undo_of_a_cross_path_delete_restores_both_paths() {
        let document = Document::new(1);
        let p = line(&document, 7, &[0.0, 10.0]);
        let q = line(&document, 8, &[0.0, 10.0, 20.0]);
        document.delete_anchors(&[(p, id(7, 0)), (q, id(8, 1))]);
        let step = document.own_steps().last().unwrap().step;
        document.undo_step(step).unwrap();
        assert_eq!(xs(&document, p), vec![0.0, 10.0]);
        assert_eq!(xs(&document, q), vec![0.0, 10.0, 20.0]);
    }

    /// `adrs.md` decision 4, dated note of 2026-10-14: a pair whose path a peer or an undo has
    /// removed is skipped, and the live paths are still deleted in one commit.
    #[test]
    fn a_missing_path_is_skipped_and_the_live_paths_are_deleted_in_one_commit() {
        let document = Document::new(1);
        let gone = line(&document, 7, &[0.0, 10.0]);
        document.delete_anchors(&[(gone, id(7, 0)), (gone, id(7, 1))]);
        let q = line(&document, 8, &[0.0, 10.0, 20.0]);
        let before = document.own_step_count();
        document.delete_anchors(&[(q, id(8, 1)), (gone, id(7, 0))]);
        assert_eq!(document.own_step_count(), before + 1);
        assert_eq!(xs(&document, q), vec![0.0, 20.0]);
        document.delete_anchors(&[(gone, id(7, 1))]);
        assert_eq!(
            document.own_step_count(),
            before + 1,
            "only a gone path: no commit"
        );
    }

    #[test]
    fn stale_anchor_ids_are_ignored_and_all_stale_commits_nothing() {
        let document = Document::new(1);
        let p = line(&document, 7, &[0.0, 10.0, 20.0]);
        let before = document.own_step_count();
        document.delete_anchors(&[(p, id(9, 9))]);
        document.delete_anchors(&[]);
        assert_eq!(document.own_step_count(), before);
        document.delete_anchors(&[(p, id(7, 1)), (p, id(9, 9)), (p, id(7, 1))]);
        assert_eq!(xs(&document, p), vec![0.0, 20.0]);
    }

    /// `0053` criterion 20: Make symmetric on nodes of two paths, one step, each node's
    /// handles along the chord of its own path's neighbours.
    #[test]
    fn a_convert_across_two_paths_is_one_step() {
        let document = Document::new(1);
        let p = line(&document, 7, &[0.0, 10.0, 20.0]);
        let q = line(&document, 8, &[0.0, 10.0, 20.0]);
        let before = document.own_step_count();
        document
            .convert_anchor_kind(&[(p, id(7, 1)), (q, id(8, 1))], AnchorKind::Symmetric)
            .unwrap();
        assert_eq!(document.own_step_count(), before + 1);
        for path in [p, q] {
            let snapshot = document.path(path).unwrap();
            assert_eq!(snapshot.anchors[1].kind, AnchorKind::Symmetric);
            assert!(snapshot.anchors[1].handle_out.x > 0.0);
            assert_eq!(snapshot.anchors[0].kind, AnchorKind::Corner);
        }
    }

    #[test]
    fn a_convert_with_one_stale_id_on_another_path_converts_nothing() {
        let document = Document::new(1);
        let p = line(&document, 7, &[0.0, 10.0, 20.0]);
        let q = line(&document, 8, &[0.0, 10.0, 20.0]);
        let before = document.own_step_count();
        assert_eq!(
            document.convert_anchor_kind(&[(p, id(7, 1)), (q, id(9, 9))], AnchorKind::Symmetric),
            Err(PathEditError::NoSuchAnchor)
        );
        assert_eq!(document.own_step_count(), before);
        assert_eq!(
            document.path(p).unwrap().anchors[1].kind,
            AnchorKind::Corner
        );
    }

    #[test]
    fn a_convert_to_the_kind_every_node_has_commits_nothing() {
        let document = Document::new(1);
        let p = line(&document, 7, &[0.0, 10.0, 20.0]);
        let q = line(&document, 8, &[0.0, 10.0, 20.0]);
        let before = document.own_step_count();
        document
            .convert_anchor_kind(&[(p, id(7, 1)), (q, id(8, 0))], AnchorKind::Corner)
            .unwrap();
        document
            .convert_anchor_kind(&[], AnchorKind::Corner)
            .unwrap();
        assert_eq!(document.own_step_count(), before);
    }
}
