//! The laser operations of the active laser page (`specs/0101-laser-operations` criteria 3 to 10,
//! 15, 16; `specs/0111-laser-operation-tools`; ADR 0046 §1 to §4, ADR 0056): the commands that
//! create operations by assignment, rename, delete and reorder them, assign and remove items, and
//! switch Show, Output and Cuts last. What runs is decided by `curvyo-laser-core`; the document
//! writes each command as one step.

mod assign;
mod commands;
mod grid_look;
mod index;
mod items;
mod kind_press;
mod look;
mod options;
mod settings;
mod settings_view;
#[cfg(test)]
mod tests;
mod tools;
#[cfg(test)]
mod tools_option_tests;
#[cfg(test)]
mod tools_tests;
mod view_json;

pub(in crate::session) use settings::typed_number;
#[cfg(test)]
pub(in crate::session) use settings_view::SettingsView;

use curvyo_document_core::{NodeId, OperationSnapshot};
use curvyo_laser_core::operations::KindRead;

/// The text of a refused assignment whose selection holds only guides or dimensions.
pub(super) const NEVER_RUN_NOTICE: &str = "Guides never run.";

/// The notice when selected items of a test grid are not assigned (`specs/0028` criterion 6).
pub(super) const GRID_ITEMS_NOTICE: &str = "Test grid items run with their grid row only.";

/// The notice when the document refused to add an operation, so a press never does nothing
/// silently.
pub(super) const ADD_REFUSED_NOTICE: &str = "The operation could not be added.";

/// What the session keeps for operations: the notice of the last assignment, shown until the next
/// operation command (criterion 10), the polite status of the last kind press (`specs/0111` UX
/// notes §10) and the state of the operation tools (`tools.rs`).
#[derive(Debug, Default)]
pub(in crate::session) struct OperationsState {
    notice: Option<String>,
    status: Option<String>,
    tools: tools::OperationTools,
    /// The output page's leaves, for the drawing read they were built from (`index.rs`).
    leaves: index::LeafCache,
    /// The numbers typed into an operation with No settings while power or speed is missing
    /// (`specs/0028` UX notes §8.3), with its typed Interval (`specs/0111` criterion 19).
    settings_draft: Option<(NodeId, settings::SettingsDraft)>,
}

/// The operations of the active laser page, in stored list order.
pub(in crate::session) struct ActiveOperations {
    pub(in crate::session) manufacturing: NodeId,
    pub(in crate::session) operations: Vec<OperationSnapshot>,
    pub(in crate::session) cuts_last: bool,
}

impl ActiveOperations {
    /// Each operation's kind as read, in list order.
    pub(in crate::session) fn kinds(&self) -> Vec<KindRead> {
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
    pub(in crate::session) fn active_operations(&self) -> Option<ActiveOperations> {
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
    pub(in crate::session) fn is_active_operation(&self, id: NodeId) -> bool {
        self.active_operations()
            .is_some_and(|active| active.operations.iter().any(|op| op.id == id))
    }

    /// The notice of the last operation command, `None` when it had none (criterion 10).
    #[must_use]
    pub fn operations_notice(&self) -> Option<String> {
        self.operations.notice.clone()
    }

    /// The polite status of the last kind press ("Cut 3 added.", "Assigned to Engrave 1."), once
    /// (`specs/0111` UX notes §10); empty when there is none.
    pub fn take_operations_status(&mut self) -> String {
        self.operations.status.take().unwrap_or_default()
    }
}
