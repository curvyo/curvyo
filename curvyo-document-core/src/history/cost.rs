//! What an undo costs before it runs, and the warm-up that keeps the first one fast (ADR 0014 §13,
//! `specs/0020-undo-redo` criterion 30).

use super::checkout::Detached;
use super::step_id::StepRef;
use super::step_log::find_step;
use crate::document::Document;

/// A step that lies more operations than this behind the latest version is announced with the
/// long-operation state too: reading its versions costs in proportion to that distance (a checkout
/// of 1,000 steps back took 75 ms in the spike).
pub const FAR_STEP_OPS: usize = 1_000;

/// A step with at most this many operations is undone within 100 ms and shows no busy state; a
/// larger one is announced with the long-operation state first.
pub const SMALL_STEP_OPS: usize = 2_000;

/// A log with at most this many operations builds the per-object index of `0041` well within
/// 100 ms (about 220 ms cold at 25,000 steps, which is more than 100,000 operations): a larger one
/// is announced with the long-operation state before its first object undo.
const OBJECT_INDEX_SMALL_OPS: usize = 40_000;

impl Document {
    /// Reads the history once so that the first undo is as fast as any later one: the container
    /// chains of the log and the checkout cache. The editor calls it in an idle callback after the
    /// first paint of an opened file. Idempotent.
    pub fn warm_history(&self) {
        if self.commit_state.is_warm() {
            return;
        }
        let doc = self.loro();
        self.assert_outside_batch();
        doc.commit();
        self.container_index.borrow_mut().extend(doc);
        self.sync_object_index();
        // One checkout to the version before the newest change and back builds the cache.
        let newest = doc.oplog_frontiers();
        if let Some(change) = newest.iter().find_map(|id| doc.get_change(id))
            && let Ok(guard) = Detached::at(doc, &change.deps)
        {
            drop(guard);
        }
        self.commit_state.set_warm();
    }

    /// Whether the per-object history can be read without the long-operation state: its index is
    /// built, or the log is small enough to build it at once.
    #[must_use]
    pub fn object_history_is_cheap(&self) -> bool {
        self.object_index_is_built() || self.loro().len_ops() <= OBJECT_INDEX_SMALL_OPS
    }

    /// Whether the history has been read since the document was opened. A new document is warm.
    #[must_use]
    pub fn history_is_warm(&self) -> bool {
        self.commit_state.is_warm()
    }

    /// How many operations were made after `step` ended, by anyone: the distance an undo reads
    /// across besides the step itself. `None` when the step is not in the log.
    #[must_use]
    pub fn step_distance(&self, step: StepRef) -> Option<usize> {
        let log = find_step(self.loro(), step)?;
        let start = self.loro().frontiers_to_vv(&log.after())?;
        let now = self.loro().oplog_vv();
        Some(
            now.iter()
                .map(|(peer, counter)| {
                    let since = start.get(peer).copied().unwrap_or(0);
                    usize::try_from(counter - since).unwrap_or(0)
                })
                .sum(),
        )
    }

    /// A number that changes whenever the operation log does (a step, an undo, a merge): the
    /// History tab reads its rows again only when it changes.
    #[must_use]
    pub fn history_stamp(&self) -> u32 {
        u32::try_from(self.loro().len_ops()).unwrap_or(u32::MAX)
    }

    /// The number of operations in `step`, read from the log before an undo is attempted.
    #[must_use]
    pub fn step_op_count(&self, step: StepRef) -> Option<usize> {
        find_step(self.loro(), step).map(|log| log.len())
    }
}
