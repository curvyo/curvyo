//! What an undo costs before it runs, and the warm-up that keeps the first one fast (ADR 0014 §13,
//! `specs/0020-undo-redo` criterion 30).

use super::checkout::Detached;
use super::step_id::StepRef;
use super::step_log::find_step;
use crate::document::Document;

/// A step with at most this many operations is undone within 100 ms and shows no busy state; a
/// larger one is announced with the long-operation state first.
pub const SMALL_STEP_OPS: usize = 2_000;

impl Document {
    /// Reads the history once so that the first undo is as fast as any later one: the container
    /// chains of the log and the checkout cache. The editor calls it in an idle callback after the
    /// first paint of an opened file. Idempotent.
    pub fn warm_history(&self) {
        if self.commit_state.is_warm() {
            return;
        }
        let doc = self.loro();
        doc.commit();
        self.container_index.borrow_mut().extend(doc);
        // One checkout to the version before the newest change and back builds the cache.
        let newest = doc.oplog_frontiers();
        if let Some(change) = newest.iter().find_map(|id| doc.get_change(id))
            && let Ok(guard) = Detached::at(doc, &change.deps)
        {
            drop(guard);
        }
        self.commit_state.set_warm();
    }

    /// Whether the history has been read since the document was opened. A new document is warm.
    #[must_use]
    pub fn history_is_warm(&self) -> bool {
        self.commit_state.is_warm()
    }

    /// The number of operations in `step`, read from the log before an undo is attempted.
    #[must_use]
    pub fn step_op_count(&self, step: StepRef) -> Option<usize> {
        find_step(self.loro(), step).map(|log| log.len())
    }
}
