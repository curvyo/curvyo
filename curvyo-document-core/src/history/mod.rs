//! The document's history: steps delimited by a header in the commit message, their identity, and
//! the undo and redo engine over the operation log (ADR 0014; `specs/0020-undo-redo`).

mod checkout;
mod commit;
mod error;
mod node_value;
mod report;
mod restore;
mod revive;
mod stack_machine;
mod step_header;
mod step_id;
mod step_log;
mod touched;
mod updates;

pub(crate) use commit::CommitState;
pub use commit::OwnStep;
pub use error::{HistoryError, SyncError};
pub use report::{Direction, ObjectReport, ObjectResult, RestoreReport, Role, SkipReason};
pub use stack_machine::{Event, STACK_LIMIT, StackMachine, Status};
pub use step_id::{MIN_SHOWN_ID_LEN, StepRef, shown_ids};
pub(crate) use touched::ContainerIndex;
