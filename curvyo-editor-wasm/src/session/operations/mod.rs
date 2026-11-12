//! The laser operations of the active laser page (`specs/0101-laser-operations` criteria 2 to 10,
//! 15, 16; ADR 0046 §1 to §4): the commands that add, rename, colour, delete and reorder
//! operations, assign and remove items, and switch Show, Output and Cuts last. What runs is
//! decided by `curvyo-laser-core`; the document writes each command as one step.

mod assign;
mod commands;
mod grid_look;
mod index;
mod items;
mod look;
mod settings;
mod settings_view;
#[cfg(test)]
mod tests;
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
    /// The numbers typed into an operation with No settings while power or speed is missing
    /// (`specs/0028` UX notes §8.3).
    settings_draft: Option<(NodeId, curvyo_laser_core::settings::SetOverride)>,
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
