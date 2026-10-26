//! `Session`'s Close path command (`specs/0034-pen-path-extension` criteria 18 to 21,
//! `specs/0055-compound-path-node-editing` criterion 7): which open outlines the Node bar's buttons
//! act on, what they would do, and the one commit that does it.

use curvyo_ui_core::{ClosableCounts, JoinType, closable_counts, close_path_set, plan_close_paths};

use super::{Session, Tool};

/// What the Close path buttons show: how many open paths and open outlines of compound paths they
/// would close, and how many they would skip for having fewer than three nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClosePathState {
    /// Open ordinary paths with three or more nodes.
    pub closable: usize,
    /// Open ordinary paths with fewer than three nodes, or three whose ends coincide.
    pub skipped: usize,
    /// How many of the skipped paths are of the second kind.
    pub same_ends: usize,
    /// Open outlines of compound paths with three or more nodes.
    pub closable_outlines: usize,
    /// Open outlines of compound paths skipped, by the same rule.
    pub skipped_outlines: usize,
    /// How many of the skipped outlines have three nodes with coinciding ends.
    pub same_ends_outlines: usize,
}

impl From<ClosableCounts> for ClosePathState {
    fn from(counts: ClosableCounts) -> Self {
        Self {
            closable: counts.closable,
            skipped: counts.skipped,
            same_ends: counts.same_ends,
            closable_outlines: counts.closable_outlines,
            skipped_outlines: counts.skipped_outlines,
            same_ends_outlines: counts.same_ends_outlines,
        }
    }
}

/// What a press of a Close path button did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClosePathOutcome {
    /// How many paths were closed.
    pub closed: usize,
    /// How many open paths were skipped.
    pub skipped: usize,
    /// How many of the skipped paths have three nodes with coinciding ends.
    pub same_ends: usize,
    /// How many outlines of compound paths were closed.
    pub closed_outlines: usize,
    /// How many open outlines of compound paths were skipped.
    pub skipped_outlines: usize,
    /// How many of the skipped outlines have three nodes with coinciding ends.
    pub same_ends_outlines: usize,
}

impl ClosePathOutcome {
    /// The outcome of a plan with these counts; `written` says whether anything was closed.
    fn of(counts: ClosableCounts, written: bool) -> Self {
        Self {
            closed: if written { counts.closable } else { 0 },
            skipped: counts.skipped,
            same_ends: counts.same_ends,
            closed_outlines: if written { counts.closable_outlines } else { 0 },
            skipped_outlines: counts.skipped_outlines,
            same_ends_outlines: counts.same_ends_outlines,
        }
    }
}

impl Session {
    /// What the Close path buttons would do right now; zero outside the Node tool.
    #[must_use]
    pub fn close_path_state(&self) -> ClosePathState {
        if self.tool != Tool::Node {
            return ClosePathState::default();
        }
        let paths = self.paths();
        let set = close_path_set(&paths, self.node.selection(), &self.selection);
        closable_counts(&set).into()
    }

    /// Closes every open outline of the set with three or more nodes, the first node becoming the
    /// closing node with `join` applied, as one commit (`0034` criteria 19 to 21, `0055`
    /// criterion 7). Does nothing outside the Node tool.
    pub fn close_paths(&mut self, join: JoinType) -> ClosePathOutcome {
        self.recording(|this| -> ClosePathOutcome {
            if this.tool != Tool::Node {
                return ClosePathOutcome::default();
            }
            this.end_colour_pick();
            let paths = this.paths();
            let set = close_path_set(&paths, this.node.selection(), &this.selection);
            let plan = plan_close_paths(&set, join);
            if plan.growths.is_empty() {
                return ClosePathOutcome::of(plan.counts, false);
            }
            match this.document.close_paths(&plan.growths) {
                Ok(()) => ClosePathOutcome::of(plan.counts, true),
                Err(_) => ClosePathOutcome::default(),
            }
        })
    }
}
