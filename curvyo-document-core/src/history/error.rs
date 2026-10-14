//! Why a history command could not run.

/// Why undo, redo or a history read failed. None of them changes the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HistoryError {
    /// The step is not in this document's operation log (a wiped or foreign step).
    #[error("this step is not in the document's history")]
    NoSuchStep,
    /// The log could not be read at the versions the step needs.
    #[error("the history of this step cannot be read")]
    Unreadable,
    /// The engine that brings a deleted object back was refused by the CRDT library.
    #[error("a deleted object could not be brought back")]
    Revive,
    /// The history of a shared document is not wiped (`specs/0020-undo-redo` criterion 50).
    #[error("the history of a shared document cannot be wiped yet")]
    Shared,
    /// There is no step to remove: a wipe would only add a row (`specs/0020-undo-redo` criterion 47).
    #[error("there is no history to wipe")]
    NoSteps,
    /// The log could not be rewritten; the document is as it was.
    #[error("the history could not be wiped")]
    Wipe,
    /// The object does not exist (any more).
    #[error("no such object")]
    NoSuchObject,
    /// A group's version is not cloned yet (`specs/0042-history-branches` criterion 25).
    #[error("a group's version cannot be cloned")]
    Group,
}

/// Why an exchange of updates between two replicas failed. The document is not changed by a
/// failed import that was refused up front; an import that already applied part of the updates
/// reports [`SyncError::Pending`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SyncError {
    /// The updates could not be encoded.
    #[error("the updates could not be encoded")]
    Encode,
    /// The bytes are not updates this build can read, or the document would not be valid with
    /// them.
    #[error("the updates are not valid for this document")]
    Malformed,
    /// Part of the updates depends on history this replica does not hold (for example a copy that
    /// still has steps from before a wipe).
    #[error("the updates depend on history this document does not have")]
    Pending,
    /// The two documents do not hold the same history: one was wiped and the other still has the
    /// steps from before (`specs/0020-undo-redo` criterion 49, ADR 0014 §9). Nothing was merged.
    #[error("the documents were not wiped together and cannot be merged")]
    EpochMismatch,
}
