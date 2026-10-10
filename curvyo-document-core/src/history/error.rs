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
}
