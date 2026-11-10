//! The laser operations of the active laser page (`specs/0101-laser-operations` criteria 2 to 10,
//! 15, 16; ADR 0046 §1 to §4): the commands that add, rename, colour, delete and reorder
//! operations, assign and remove items, and switch Show, Output and Cuts last. What runs is
//! decided by `curvyo-laser-core`; the document writes each command as one step.

mod assign;
mod commands;
mod index;
mod items;
mod look;
#[cfg(test)]
mod tests;
mod view_json;

use curvyo_document_core::{NodeId, OperationSnapshot};
use curvyo_laser_core::operations::KindRead;

/// The text of a refused assignment whose selection holds only guides or dimensions.
pub(super) const NEVER_RUN_NOTICE: &str = "Guides never run.";

/// How the output page is drawn (criterion 18): by its operations' looks, or as drawn. Session
/// state, not saved.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationsViewMode {
    /// Every leaf by its operation's look (the default).
    #[default]
    Operations,
    /// The drawing's own style of every item.
    Drawn,
}

/// What the session keeps for operations: the notice of the last assignment, shown until the next
/// operation command (criterion 10), and the view switch (criterion 18).
#[derive(Debug, Default)]
pub(in crate::session) struct OperationsState {
    notice: Option<String>,
    view_mode: OperationsViewMode,
    /// The output page's leaves, for the drawing read they were built from (`index.rs`).
    leaves: index::LeafCache,
}

/// The operations of the active laser page, in stored list order.
struct ActiveOperations {
    manufacturing: NodeId,
    operations: Vec<OperationSnapshot>,
    cuts_last: bool,
}

impl ActiveOperations {
    /// Each operation's kind as read, in list order.
    fn kinds(&self) -> Vec<KindRead> {
        self.operations
            .iter()
            .map(|op| KindRead::of(op.kind.as_deref()))
            .collect()
    }
}

impl super::Session {
    /// Whether the active page is a laser page with its extension on: the Operations tab exists.
    pub(in crate::session) fn operations_available(&self) -> bool {
        self.active_operations().is_some()
    }

    /// The operations of the active laser page, `None` on another page or while its extension is
    /// switched off (criterion 28).
    fn active_operations(&self) -> Option<ActiveOperations> {
        let manufacturing = self.active_manufacturing()?;
        if !self.manufacturing_available(manufacturing.id) {
            return None;
        }
        let operations = self.document.operations(manufacturing.id).ok()?;
        Some(ActiveOperations {
            manufacturing: manufacturing.id,
            operations,
            cuts_last: curvyo_laser_core::operations::cuts_last(&manufacturing),
        })
    }

    /// Whether `id` is an operation of the active laser page: an operation of another laser page
    /// is never touched from here (criterion 9).
    fn is_active_operation(&self, id: NodeId) -> bool {
        self.active_operations()
            .is_some_and(|active| active.operations.iter().any(|op| op.id == id))
    }

    /// The notice of the last operation command, `None` when it had none (criterion 10).
    #[must_use]
    pub fn operations_notice(&self) -> Option<String> {
        self.operations.notice.clone()
    }
}
