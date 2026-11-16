//! The session's Find centre lines command (`specs/0117-centre-lines` criteria 1 to 4, 12, 16
//! and 18): which selected objects are subjects, why a press is refused, and the write of the
//! prepared results in place of the subjects in one commit labelled `centre_lines`; the bar and
//! its slices are `centre_lines_bar.rs`. The session never runs the kernel but in
//! [`Session::apply_centre_lines`], the one-call Apply.

use std::sync::Arc;

use curvyo_document_core::{
    AnchorId, CopySource, DrawingNode, NodeId, ObjectSnapshot, PathWidthPoints, Point,
};
use curvyo_geometry_core::CentreOptions;
use curvyo_ui_core::{
    CENTRE_LINES_COMMIT_LABEL, CentreDrawing, CentreLinesRefusal, CentreSubjects,
    centre_line_subjects, centre_lines_available, object_bounds,
};

use super::{Session, Tool};
use crate::centre_lines_glue::CentreLinesWorker;

/// What [`Session::apply_centre_lines`] did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CentreLinesOutcome {
    /// Nothing: the Select tool is not ready, nothing is selected, or the write failed.
    #[default]
    Ignored,
    /// Subjects became centre paths.
    Applied {
        /// Subjects converted.
        converted: usize,
        /// Subjects kept filled (every region a blob, or a failed skeleton).
        kept: usize,
        /// Regions kept filled, of kept and of converted subjects (the notice's count).
        kept_regions: usize,
        /// Regions whose skeleton failed (kept as they are).
        failed: usize,
        /// Selected objects that are not subjects.
        skipped: usize,
        /// Clones of converted objects that became ordinary paths.
        unlinked: usize,
    },
    /// Refused; nothing changed and no history entry was made.
    Refused(CentreLinesRefusal),
    /// The document or the selection changed since the subjects were sent, or the result does
    /// not fit them: nothing was written; the bar sends the subjects again.
    Stale,
}

/// One subject's result: its drawing (`None` when it stays as it is), the regions kept filled
/// and the regions whose skeleton failed.
pub(super) struct SubjectResult {
    pub(super) drawing: Option<CentreDrawing>,
    pub(super) kept_regions: usize,
    pub(super) failed: usize,
}

/// The leaves of a drawing node in document order.
fn leaf_count(node: &DrawingNode) -> usize {
    match node {
        DrawingNode::Group { children, .. } => children.iter().map(leaf_count).sum(),
        _ => 1,
    }
}

impl Session {
    /// The leaves that are never subjects: hidden, locked, guides.
    fn centre_skips(&self) -> impl Fn(NodeId) -> bool + use<> {
        let read = self.document_read();
        let (inert, guides) = (Arc::clone(&read.inert), Arc::clone(&read.guides));
        move |id| inert.contains(&id) || guides.contains(&id)
    }

    /// The selection Find centre lines works on: the objects the Select or Node tool has
    /// selected, never their nodes (UX notes §2).
    fn centre_selection(&self) -> &[NodeId] {
        if matches!(self.tool, Tool::Select | Tool::Node) {
            self.selection.ids()
        } else {
            &[]
        }
    }

    /// Whether the command may run now: the Select or Node tool is active, no drag is in flight
    /// and no typed entry is open.
    pub(super) fn centre_ready(&self) -> bool {
        match self.tool {
            Tool::Select => self.rail_command_ready(),
            Tool::Node => !self.node.drag_in_flight(),
            _ => false,
        }
    }

    /// Whether Find centre lines is enabled: some selected leaf is a filled closed shape
    /// (criterion 1). The control is shown whenever the Node tool has a selection.
    #[must_use]
    pub fn centre_lines_available(&self) -> bool {
        let selection = self.centre_selection();
        if selection.is_empty() {
            return false;
        }
        let leaves = Arc::clone(&self.document_read().leaves);
        centre_lines_available(&leaves, selection, &self.centre_skips())
    }

    /// The document box around the selected objects, where the context menu opens from the
    /// keyboard (UX notes §1); `None` without a selection.
    #[must_use]
    pub fn centre_lines_anchor(&self) -> Option<(Point, Point)> {
        let selection = self.centre_selection();
        if selection.is_empty() {
            return None;
        }
        let groups = self.selected_group_leaves();
        let objects = self.objects();
        objects
            .iter()
            .filter(|object| selection.contains(&object.id()) || groups.contains(&object.id()))
            .map(object_bounds)
            .reduce(|(a, b), (c, d)| {
                (
                    Point::new(a.x.min(c.x), a.y.min(c.y)),
                    Point::new(b.x.max(d.x), b.y.max(d.y)),
                )
            })
    }

    /// Why a press would be refused now, if it would.
    #[must_use]
    pub fn centre_lines_refusal(&self) -> Option<CentreLinesRefusal> {
        let leaves = Arc::clone(&self.document_read().leaves);
        centre_line_subjects(&leaves, self.centre_selection(), &self.centre_skips()).err()
    }

    /// The subjects of the selection as they are drawn, or why there are none.
    pub(super) fn centre_subjects(&self) -> Result<CentreSubjects, CentreLinesRefusal> {
        let read = self.document_read();
        let leaves = self.spiro_selection_objects(&read.leaves);
        centre_line_subjects(&leaves, self.centre_selection(), &self.centre_skips())
    }

    /// Find centre lines on the selection with `options` in one commit, as the bar's Apply
    /// does, with the worker run here. A refusal changes nothing and makes no history entry.
    pub fn apply_centre_lines(&mut self, options: CentreOptions) -> CentreLinesOutcome {
        if !self.centre_ready() {
            return CentreLinesOutcome::Ignored;
        }
        // An Apply with nothing selected (an undo can clear the selection while the options
        // stay open) answers with the refusal instead of doing nothing silently.
        if self.selection.is_empty() {
            return CentreLinesOutcome::Refused(CentreLinesRefusal::NotFilled);
        }
        let subjects = match self.centre_lines_open() {
            Ok(wire) => wire,
            Err(refusal) => return CentreLinesOutcome::Refused(refusal),
        };
        let mut worker = CentreLinesWorker::new();
        if worker.set_subjects(&subjects, &mut |_| {}).is_err() {
            self.centre = None;
            return CentreLinesOutcome::Ignored;
        }
        let _ = worker.finish(&options, false);
        let result = worker.result();
        if let Err(outcome) = self.centre_lines_apply_begin(&result) {
            self.centre = None;
            return outcome;
        }
        loop {
            if let Some(outcome) = self.centre_lines_apply_step() {
                return outcome;
            }
        }
    }

    /// Writes the prepared drawings in place of their subjects in one commit, selects the
    /// results and counts what happened.
    pub(super) fn write_results(
        &mut self,
        found: &CentreSubjects,
        results: Vec<SubjectResult>,
    ) -> CentreLinesOutcome {
        let kept = results.iter().filter(|r| r.drawing.is_none()).count();
        let kept_regions = results.iter().map(|r| r.kept_regions).sum();
        let failed = results.iter().map(|r| r.failed).sum();
        let items: Vec<(NodeId, CentreDrawing)> = found
            .subjects
            .iter()
            .zip(results)
            .filter_map(|(subject, result)| Some((subject.id, result.drawing?)))
            .collect();
        let (created, unlinked) = if items.is_empty() {
            (Vec::new(), 0)
        } else {
            let read = self.document_read();
            let unlinks = self.centre_unlinks(&read.leaves, &found.followers);
            let unlinked = unlinks.len();
            let Some(created) = self.write_centre_lines(&items, &unlinks) else {
                // invariant: the drawing was planned from the objects just read with fresh ids.
                debug_assert!(false, "planned centre lines could not be written");
                return CentreLinesOutcome::Ignored;
            };
            self.command_refusal = None;
            self.node.clear_selection();
            self.selection.set(&created);
            self.hovered_object = None;
            (created, unlinked)
        };
        CentreLinesOutcome::Applied {
            converted: created.len(),
            kept,
            kept_regions,
            failed,
            skipped: found.skipped,
            unlinked,
        }
    }

    /// The linked clones of converted objects, to become ordinary paths (criterion 16).
    fn centre_unlinks(
        &mut self,
        leaves: &[ObjectSnapshot],
        followers: &[NodeId],
    ) -> Vec<CopySource> {
        leaves
            .iter()
            .filter_map(|object| match object {
                ObjectSnapshot::Clone(clone) if followers.contains(&clone.id()) => {
                    Some(CopySource {
                        id: clone.id(),
                        anchor_ids: clone
                            .resolved
                            .all_anchors()
                            .map(|_| self.minter.mint())
                            .collect(),
                    })
                }
                _ => None,
            })
            .collect()
    }

    /// Writes the drawings in place of their subjects, with every path's width points in one
    /// batched write, in one commit; the created top nodes.
    fn write_centre_lines(
        &mut self,
        items: &[(NodeId, CentreDrawing)],
        unlinks: &[CopySource],
    ) -> Option<Vec<NodeId>> {
        let nodes: Vec<(NodeId, DrawingNode)> = items
            .iter()
            .map(|(id, drawing)| (*id, drawing.node.clone()))
            .collect();
        let drawing_nodes: Vec<DrawingNode> = nodes.iter().map(|(_, n)| n.clone()).collect();
        let anchor_ids: Vec<_> = (0..DrawingNode::anchor_count(&drawing_nodes))
            .map(|_| self.minter.mint())
            .collect();
        let profiles: Vec<Vec<(AnchorId, f64, f64)>> = items
            .iter()
            .flat_map(|(_, d)| d.profiles.iter())
            .map(|profile| {
                profile
                    .iter()
                    .map(|&(at, multiplier)| (self.minter.mint(), at, multiplier))
                    .collect()
            })
            .collect();
        let removed: Vec<NodeId> = items.iter().map(|(id, _)| *id).collect();
        self.document.batch(CENTRE_LINES_COMMIT_LABEL, |doc| {
            if !unlinks.is_empty() && doc.unlink_clones(unlinks).is_err() {
                return None;
            }
            let created = doc.insert_drawing_above(&nodes, &anchor_ids).ok()?;
            let mut leaves = Vec::new();
            for (top, (_, node)) in created.iter().zip(&nodes) {
                if leaf_count(node) == 1 && !matches!(node, DrawingNode::Group { .. }) {
                    leaves.push(*top);
                } else {
                    leaves.extend(doc.children(Some(*top)));
                }
            }
            let written: Vec<PathWidthPoints> = leaves
                .into_iter()
                .zip(profiles)
                .filter(|(_, points)| !points.is_empty())
                .collect();
            doc.set_width_profiles(&written).ok()?;
            doc.delete_objects(&removed).ok()?;
            Some(created)
        })
    }
}
