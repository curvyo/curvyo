//! The document's history: steps delimited by a header in the commit message, their identity, and
//! the undo and redo engine over the operation log (ADR 0014; `specs/0020-undo-redo`).

mod commit;
mod error;
mod revive;
mod step_header;
mod step_id;

pub(crate) use commit::CommitState;
pub use commit::OwnStep;
pub use error::HistoryError;
pub use step_id::{MIN_SHOWN_ID_LEN, StepRef, shown_ids};
