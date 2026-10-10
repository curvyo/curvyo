//! The document's history: steps delimited by a header in the commit message, their identity, and
//! the undo and redo engine over the operation log (ADR 0014; `specs/0020-undo-redo`).

mod checkout;
mod commit;
mod cost;
mod error;
mod merge;
mod node_value;
mod report;
mod restore;
mod restore_plan;
mod revive;
mod rows;
mod stack_machine;
mod step_header;
mod step_id;
mod step_index;
mod step_log;
mod touched;
mod updates;
mod wipe;

pub(crate) use commit::CommitState;
pub use commit::OwnStep;
pub use cost::{FAR_STEP_OPS, SMALL_STEP_OPS};
pub use error::{HistoryError, SyncError};
pub use report::{
    Direction, ObjectReport, ObjectResult, ObjectShape, RestoreReport, Role, SkipReason,
};
pub use rows::{Author, HistoryRow, StepKind, TouchedObject};
pub use stack_machine::{Event, STACK_LIMIT, StackMachine, Status};
pub use step_header::{LABEL_NEW_DOCUMENT, LABEL_REDO, LABEL_UNDO, LABEL_WIPE as WIPE_LABEL};
pub use step_id::{MIN_SHOWN_ID_LEN, StepRef, shown_ids};
pub(crate) use step_index::StepIndex;
pub(crate) use touched::ContainerIndex;
