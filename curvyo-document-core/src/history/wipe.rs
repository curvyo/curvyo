//! Wiping the document's history (ADR 0014 §9; `specs/0020-undo-redo` criteria 47 to 51): the log
//! is cut at the present with a shallow snapshot, re-imported into the same document under the same
//! peer, and one step records that it happened.

use loro::{ExportMode, Frontiers, LoroDoc};

use super::error::HistoryError;
use super::step_header::LABEL_WIPE;
use crate::document::{Document, OBJECTS_TREE, ROOT_MAP};

/// The optional root key that holds the epoch: the encoded start of the history that was kept.
/// An older build ignores it (ADR 0014 §7).
pub(crate) const KEY_HISTORY_WIPED: &str = "history_wiped";

/// The text form of a set of operation ids.
pub(crate) fn encode_frontiers(frontiers: &Frontiers) -> String {
    let mut ids: Vec<String> = frontiers
        .iter()
        .map(|id| format!("{}@{}", id.peer, id.counter))
        .collect();
    ids.sort();
    ids.join(",")
}

impl Document {
    /// Whether the document has been shared (it has a keyring, ADR 0004 §1). Nothing builds a
    /// keyring yet; the sharing slice calls [`Document::mark_shared`].
    #[must_use]
    pub fn is_shared(&self) -> bool {
        self.commit_state.is_shared()
    }

    /// Records that the document has a keyring: its history is not wiped any more.
    pub fn mark_shared(&self) {
        self.commit_state.set_shared();
    }

    /// The epoch of the history: the encoded start of what a wipe kept, the same on every replica
    /// that has the wipe. `None` for a document that was never wiped.
    #[must_use]
    pub fn history_epoch(&self) -> Option<String> {
        match self
            .loro()
            .get_map(ROOT_MAP)
            .get(KEY_HISTORY_WIPED)?
            .get_deep_value()
        {
            loro::LoroValue::String(epoch) => Some(epoch.to_string()),
            _ => None,
        }
    }

    /// Removes every step before now from the file's log. The drawing, the object ids, the
    /// tombstones and the peer are unchanged; the log holds one step, "History wiped". Returns how
    /// many steps were removed. The wipe is not a step on any stack and cannot be undone: the
    /// caller empties its stacks.
    ///
    /// # Errors
    /// [`HistoryError::Shared`] for a shared document; [`HistoryError::NoSteps`] without a step; [`HistoryError::Wipe`] when the log could
    /// not be rewritten (the document is then as it was).
    pub fn wipe_history(&mut self) -> Result<usize, HistoryError> {
        if self.is_shared() {
            return Err(HistoryError::Shared);
        }
        self.assert_outside_batch();
        let removed = self.history_len();
        if removed == 0 {
            return Err(HistoryError::NoSteps);
        }
        let old = self.loro();
        old.commit();
        let peer = old.peer_id();
        let bytes = old
            .export(ExportMode::shallow_snapshot(&old.oplog_frontiers()))
            .map_err(|_| HistoryError::Wipe)?;
        let fresh = LoroDoc::new();
        fresh.set_peer_id(peer).map_err(|_| HistoryError::Wipe)?;
        crate::text_chars::configure(&fresh);
        fresh.import(&bytes).map_err(|_| HistoryError::Wipe)?;
        if !crate::path_codec::validate_path_tree(&fresh, OBJECTS_TREE) {
            return Err(HistoryError::Wipe);
        }
        self.replace_loro(fresh);
        let epoch = encode_frontiers(&self.loro().shallow_since_frontiers());
        self.loro()
            .get_map(ROOT_MAP)
            .insert(KEY_HISTORY_WIPED, epoch)
            .map_err(|_| HistoryError::Wipe)?;
        let header = self.next_header(LABEL_WIPE, &[]);
        self.commit_with_header(&header);
        Ok(removed)
    }

    /// Swaps in a rewritten log and forgets everything derived from the old one.
    fn replace_loro(&mut self, fresh: LoroDoc) {
        self.loro = fresh;
        *self.step_index.borrow_mut() = super::StepIndex::default();
        *self.container_index.borrow_mut() = super::ContainerIndex::default();
        *self.object_index.borrow_mut() = super::ObjectIndex::default();
        self.commit_state.forget_steps();
    }
}
