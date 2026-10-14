//! The document's history: steps delimited by a header in the commit message, their identity, and
//! the undo and redo engine over the operation log (ADR 0014; `specs/0020-undo-redo`).

mod branches;
mod checkout;
mod commit;
mod cost;
mod error;
mod lineage;
mod merge;
mod node_value;
mod object_index;
mod object_restore;
mod object_rows;
mod object_timeline;
mod object_wipe;
mod preview;
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
mod version_write;
mod wipe;

pub use branches::HistoryBranch;
pub(crate) use commit::CommitState;
pub use commit::OwnStep;
pub use cost::{FAR_STEP_OPS, SMALL_STEP_OPS};
pub use error::{HistoryError, SyncError};
pub(crate) use lineage::{
    KEY_CLONE_OF, KEY_HISTORY_FLOOR, decode_node, encode_clone_of, encode_node,
};
pub(crate) use object_index::ObjectIndex;
pub use object_restore::ObjectRestore;
pub(crate) use object_restore::WritableUndo;
pub use object_rows::{ObjectRow, ObjectRowClass, ObjectStatus, ObjectTarget, ObjectTargets};
pub use preview::HistoricalDocument;
pub use report::{
    Direction, ObjectReport, ObjectResult, ObjectShape, RestoreReport, Role, SkipReason,
};
pub use rows::{
    Author, HistoryRow, RowRevert, RowVersion, StepKind, TouchedObject, is_document_level_label,
};
pub use stack_machine::{BranchMark, Event, STACK_LIMIT, StackMachine, Status};
pub use step_header::{
    LABEL_CLONE_VERSION, LABEL_GO_TO_VERSION, LABEL_NEW_DOCUMENT, LABEL_OBJECT_REDO,
    LABEL_OBJECT_UNDO, LABEL_REDO, LABEL_UNDO, LABEL_WIPE as WIPE_LABEL, LABEL_WIPE_OBJECT,
};
pub use step_id::{MIN_SHOWN_ID_LEN, StepRef, shown_ids};
pub(crate) use step_index::StepIndex;
pub(crate) use touched::ContainerIndex;
pub use version_write::GoToReport;
