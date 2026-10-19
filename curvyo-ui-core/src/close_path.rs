//! The Close path command (`specs/0034-pen-path-extension` criteria 18 to 21, on compound paths
//! `specs/0055-compound-path-node-editing` criterion 7): which open outlines it closes, and the
//! one [`PathGrowth`] per outline that [`curvyo_document_core::Document::close_paths`] writes in a
//! single commit. The Node bar's buttons use [`closable_counts`] for their enablement and tooltips
//! and [`plan_close_paths`] for the press, so a button and the command cannot disagree.

use curvyo_document_core::{
    AnchorSnapshot, COINCIDENT_MM, HandleSlot, NodeId, PathEnd, PathGrowth, PathSnapshot,
    SubpathRef, merged_junction,
};

use crate::closing_join::{JoinType, resolve_closing_node};
use crate::object_selection::ObjectSelection;
use crate::selection::NodeSelection;

/// One open outline the Close path buttons act on: of an ordinary path (its only outline) or of a
/// compound path.
#[derive(Debug, Clone, Copy)]
pub struct CloseTarget<'a> {
    /// The path the outline belongs to.
    pub path: &'a PathSnapshot,
    /// The open outline.
    pub outline: SubpathRef<'a>,
}

impl CloseTarget<'_> {
    /// Whether the outline is one of several of a compound path: the notes and the notice count
    /// it apart from paths (criterion 7).
    fn of_compound(&self) -> bool {
        self.path.is_compound()
    }
}

/// The open outlines the Close path buttons act on, in the order of `paths`, outline by outline.
/// The paths are those holding a selected node or the selected segment, and the selected objects
/// (`0034` criterion 18). Of an ordinary path its outline counts whichever node is selected; of a
/// compound path the open outlines that hold a selected node (or the selected segment), or all its
/// open outlines when none of its nodes is selected (`0055` criterion 7, decision 4). `paths` are
/// the ones the Node tool can edit.
#[must_use]
pub fn close_path_set<'a>(
    paths: &'a [PathSnapshot],
    nodes: &NodeSelection,
    objects: &ObjectSelection,
) -> Vec<CloseTarget<'a>> {
    let mut selected: Vec<(NodeId, curvyo_document_core::AnchorId)> = nodes.node_pairs().to_vec();
    if let Some((path, start, _)) = nodes.segment_with_path() {
        selected.push((path, start));
    }
    let mut targets = Vec::new();
    for path in paths {
        let own: Vec<_> = selected
            .iter()
            .filter(|&&(of, _)| of == path.id)
            .map(|&(_, anchor)| anchor)
            .collect();
        if own.is_empty() && !objects.ids().contains(&path.id) {
            continue;
        }
        let rule_applies = path.is_compound() && !own.is_empty();
        for outline in path.subpaths() {
            if outline.closed || outline.anchors.is_empty() {
                continue;
            }
            if rule_applies && !outline.anchors.iter().any(|a| own.contains(&a.id)) {
                continue;
            }
            targets.push(CloseTarget { path, outline });
        }
    }
    targets
}

/// What the command does to a set of outlines.
#[derive(Debug, Clone, PartialEq)]
pub struct ClosePlan {
    /// One growth per outline that gets closed, for `Document::close_paths`.
    pub growths: Vec<PathGrowth>,
    /// What is closed and skipped, as the notice counts it.
    pub counts: ClosableCounts,
}

/// What the buttons and the notice say about a set of outlines (`0034` criteria 19 to 21,
/// `0055` criterion 7): ordinary paths and outlines of compound paths are counted apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClosableCounts {
    /// Open ordinary paths that would be closed.
    pub closable: usize,
    /// Open ordinary paths skipped: fewer than three nodes, or three whose first and last node
    /// coincide, which would be merged and leave two.
    pub skipped: usize,
    /// How many of the skipped paths have three nodes with coinciding ends: the notice says so in
    /// its own words, "fewer than 3 nodes" would be wrong for them.
    pub same_ends: usize,
    /// Open outlines of compound paths that would be closed.
    pub closable_outlines: usize,
    /// Open outlines of compound paths skipped, by the same rule as `skipped`.
    pub skipped_outlines: usize,
    /// How many of the skipped outlines have three nodes with coinciding ends.
    pub same_ends_outlines: usize,
}

/// Whether the ends of `anchors` coincide and are merged into one node before closing (`0006`
/// criterion 10), which leaves one node fewer.
fn merges_ends(anchors: &[AnchorSnapshot]) -> bool {
    anchors.len() >= 3
        && anchors[0]
            .point
            .vector_to(anchors[anchors.len() - 1].point)
            .length()
            < COINCIDENT_MM
}

/// How many nodes an outline has once closed: the merge of coincident ends counts.
fn nodes_when_closed(anchors: &[AnchorSnapshot]) -> usize {
    anchors.len() - usize::from(merges_ends(anchors))
}

/// The counts among `targets`: open outlines that would be closed, and those skipped, paths and
/// outlines of compound paths apart. Closed outlines are never targets (criterion 21).
#[must_use]
pub fn closable_counts(targets: &[CloseTarget<'_>]) -> ClosableCounts {
    let mut counts = ClosableCounts::default();
    for target in targets {
        let anchors = target.outline.anchors;
        let (closable, skipped, same_ends) = if target.of_compound() {
            (
                &mut counts.closable_outlines,
                &mut counts.skipped_outlines,
                &mut counts.same_ends_outlines,
            )
        } else {
            (
                &mut counts.closable,
                &mut counts.skipped,
                &mut counts.same_ends,
            )
        };
        if nodes_when_closed(anchors) >= 3 {
            *closable += 1;
        } else {
            *skipped += 1;
            *same_ends += usize::from(anchors.len() >= 3);
        }
    }
    counts
}

/// Closes every open outline of `targets` with three or more nodes: one segment from its last
/// node to its first, the first node being the closing node with `join` applied by
/// [`resolve_closing_node`]. An outline whose first and last node coincide has them merged first.
#[must_use]
pub fn plan_close_paths(targets: &[CloseTarget<'_>], join: JoinType) -> ClosePlan {
    let counts = closable_counts(targets);
    let mut growths = Vec::with_capacity(counts.closable + counts.closable_outlines);
    for target in targets {
        let anchors = target.outline.anchors;
        if nodes_when_closed(anchors) < 3 {
            continue;
        }
        let len = anchors.len();
        let first = &anchors[0];
        let last = &anchors[len - 1];
        let (closing, drop, before): (AnchorSnapshot, Vec<_>, _) = if merges_ends(anchors) {
            (
                merged_junction(first, false, last, true),
                vec![last.id],
                anchors[len - 2].point,
            )
        } else {
            (*first, Vec::new(), last.point)
        };
        let after = anchors[1].point;
        let resolved = resolve_closing_node(&closing, before, after, HandleSlot::In, join);
        growths.push(PathGrowth {
            path: target.path.id,
            end: PathEnd::Last,
            at: last.id,
            added: Vec::new(),
            absorb: None,
            replace: vec![resolved],
            drop,
        });
    }
    ClosePlan { growths, counts }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{AnchorId, AnchorKind, Document, NewAnchor, Point};

    use super::*;

    /// Every open outline of `paths`, as the buttons see them when the paths are selected.
    fn targets<'a>(paths: &[&'a PathSnapshot]) -> Vec<CloseTarget<'a>> {
        paths
            .iter()
            .flat_map(|&path| {
                path.subpaths()
                    .filter(|outline| !outline.closed)
                    .map(move |outline| CloseTarget { path, outline })
            })
            .collect()
    }

    fn open(document: &Document, base: u64, points: &[(f64, f64)], closed: bool) -> PathSnapshot {
        let anchors: Vec<NewAnchor> = points
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| {
                NewAnchor::corner(AnchorId::new(1, base + i as u64), Point::new(x, y))
            })
            .collect();
        let id = document.create_path(&anchors, closed);
        document.path(id).unwrap()
    }

    /// Criteria 19 and 21: open ordinary paths of three nodes or more close, shorter ones are
    /// counted as skipped, closed paths are not counted at all.
    #[test]
    fn the_plan_closes_the_long_open_paths_and_counts_the_short_ones() {
        let document = Document::new(1);
        let a = open(
            &document,
            1,
            &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)],
            false,
        );
        let b = open(&document, 10, &[(0.0, 0.0), (10.0, 0.0)], false);
        let c = open(
            &document,
            20,
            &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)],
            true,
        );
        let plan = plan_close_paths(&targets(&[&a, &b, &c]), JoinType::Sharp);
        assert_eq!(plan.counts.closable, 1);
        assert_eq!(plan.counts.skipped, 1);
        assert_eq!(plan.growths.len(), 1);
        assert_eq!(plan.growths[0].path, a.id);
        assert_eq!(
            closable_counts(&targets(&[&b, &c])),
            ClosableCounts {
                skipped: 1,
                ..ClosableCounts::default()
            }
        );
    }

    /// Criterion 19: Smooth makes the first node Asymmetric along the tangent from the last node to
    /// the second.
    #[test]
    fn smooth_makes_the_first_node_asymmetric() {
        let document = Document::new(1);
        let a = open(
            &document,
            1,
            &[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)],
            false,
        );
        let plan = plan_close_paths(&targets(&[&a]), JoinType::Smooth);
        assert_eq!(plan.growths[0].replace[0].kind, AnchorKind::Asymmetric);
        let sharp = plan_close_paths(&targets(&[&a]), JoinType::Sharp);
        assert_eq!(sharp.growths[0].replace[0].kind, AnchorKind::Corner);
    }

    /// Criterion 19: coincident first and last nodes are merged into one first.
    #[test]
    fn coincident_ends_are_merged_before_closing() {
        let document = Document::new(1);
        let a = open(
            &document,
            1,
            &[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 0.0)],
            false,
        );
        let plan = plan_close_paths(&targets(&[&a]), JoinType::Sharp);
        assert_eq!(plan.counts.closable, 1);
        assert_eq!(plan.growths[0].drop, vec![a.anchors[3].id]);
        // Three distinct nodes remain: a triangle.
        document.close_paths(&plan.growths).unwrap();
        let closed = document.path(a.id).unwrap();
        assert!(closed.closed);
        assert_eq!(closed.anchors.len(), 3);
        // Fewer than three distinct nodes after the merge: skipped.
        let b = open(&document, 10, &[(0.0, 0.0), (20.0, 0.0), (0.0, 0.0)], false);
        let counts = closable_counts(&targets(&[&b]));
        assert_eq!(
            (counts.closable, counts.skipped, counts.same_ends),
            (0, 1, 1),
            "skipped, and not for having fewer than three nodes"
        );
    }
}
