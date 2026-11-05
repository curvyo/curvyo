//! The session's laser pages (ADR 0032 §1, §2 as amended by ADR 0042): which manufacturing the
//! active page holds, which processes are available, the extension switches and the selected
//! placement. All of it is session view state; nothing here writes the document.

mod commands;
mod frame;
mod machine;
mod machine_view;
#[cfg(test)]
mod tests;
mod view_json;

use curvyo_document_core::{ManufacturingSnapshot, NodeId};
use curvyo_ui_core::ExtensionSettings;

use super::{Session, Workbench};

/// What the session keeps for laser pages.
#[derive(Debug, Default)]
pub(super) struct ManufacturingState {
    /// The extensions the maker switched off (`extensions.toml`), per user.
    pub(super) extensions: ExtensionSettings,
    /// The members of each placement for the drawing read they were computed from.
    pub(super) placed: frame::PlacedCache,
    /// The maker's machine profiles and the last pick (`specs/0026-machine-profile`).
    pub(super) machine: machine::MachineState,
    /// The placement a press selected (`specs/0098-page-kinds` criterion 12).
    pub(super) selected_placement: Option<NodeId>,
}

impl Session {
    /// The manufacturing of the active page, `None` on another page or when it is gone.
    pub(super) fn active_manufacturing(&self) -> Option<ManufacturingSnapshot> {
        match self.workbench() {
            Workbench::Manufacturing(id) => self.document.manufacturing(id),
            _ => None,
        }
    }

    /// Whether the extension of `process` is compiled in and switched on.
    #[must_use]
    pub fn process_enabled(&self, process: &str) -> bool {
        curvyo_ui_core::manifest(process).is_some()
            && self.manufacturing.extensions.is_enabled(process)
    }

    /// Whether the manufacturing `id` exists and its extension is on.
    #[must_use]
    pub fn manufacturing_available(&self, id: NodeId) -> bool {
        self.document
            .manufacturing(id)
            .is_some_and(|m| self.process_enabled(&m.process))
    }

    /// Switches the extension `id` on or off (`0093` criterion 11; not an undo step). An active
    /// laser page of a process switched off shows the vector tools and the notice. Returns the
    /// settings file's new text for the host to keep.
    pub fn set_extension_enabled(&mut self, id: &str, enabled: bool) -> String {
        self.manufacturing.extensions.set_enabled(id, enabled);
        self.sync_workbench();
        self.manufacturing.extensions.to_text()
    }

    /// Reads the per-user settings file `extensions.toml` the host kept (ADR 0034 §6).
    pub fn load_extension_settings(&mut self, text: &str) {
        self.manufacturing.extensions = ExtensionSettings::parse(text);
        self.sync_workbench();
    }

    /// The selected placement, while it still exists.
    #[must_use]
    pub fn selected_placement(&self) -> Option<NodeId> {
        let id = self.manufacturing.selected_placement?;
        self.document
            .manufacturings()
            .iter()
            .any(|m| m.placements.iter().any(|p| p.id == id))
            .then_some(id)
    }

    /// A press with the Select tool on the placed content of a laser page (`0098` criterion 12):
    /// selects the placement and clears the drawing selection. `false` when the press is on no
    /// placement, which also clears a selected one.
    pub(super) fn output_placement_press(&mut self, point: curvyo_document_core::Point) -> bool {
        let hit = self.placement_at(point);
        self.manufacturing.selected_placement = hit;
        if hit.is_some() {
            self.selection.clear();
        }
        hit.is_some()
    }

    /// Delete with a placement selected removes the reference only.
    pub(super) fn delete_selected_placement(&mut self) -> bool {
        self.selected_placement()
            .is_some_and(|id| self.delete_placement(id))
    }
}
