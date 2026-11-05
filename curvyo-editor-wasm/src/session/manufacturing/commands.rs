//! The commands of a laser page (`0093` criteria 9, 10; `0098` criteria 4, 12): each is one
//! history step recorded like every other command; values are clamped by the extension's schema.

use curvyo_document_core::{Length, NewOutputPage, NodeId};
use curvyo_extension_core::{FixtureKind, Param, ParamValue, WorkbenchContribution};
use curvyo_laser_core::{ProfileCopy, write_copy};

use super::super::Session;

impl Session {
    /// Add page with an output kind (`specs/0098-page-kinds` criterion 4): "Laser N" with the
    /// copy and work area of the profile last picked on this computer (`0026` criterion 10),
    /// nothing placed, in one step; the new page becomes active. `None` for a process that is
    /// not compiled in or is switched off.
    pub fn add_output_page(&mut self, process: &str) -> Option<String> {
        self.add_output_page_with(process, None)
    }

    /// As [`Session::add_output_page`] with the library profile `profile` (the machine list of
    /// the Add page group), else the last pick.
    pub fn add_output_page_with(&mut self, process: &str, profile: Option<&str>) -> Option<String> {
        if !self.process_enabled(process) {
            return None;
        }
        let manifest = curvyo_ui_core::manifest(process)?;
        let workbench = manifest.workbench()?;
        let picked = profile.and_then(|id| self.manufacturing.machine.profiles.find(id).cloned());
        let chosen = picked.is_some();
        let profile = picked.unwrap_or_else(|| self.initial_machine_profile());
        let copy = write_copy(&ProfileCopy::of(&profile));
        let page = self.recording(|this| {
            this.document
                .add_output_page(NewOutputPage {
                    process,
                    label: workbench.label,
                    profile: Some(&copy),
                    work_area: profile.work_area,
                })
                .ok()
        })?;
        // A machine chosen in the Add page group is this computer's last pick (criterion 1).
        if chosen {
            self.remember_machine_pick(&profile.id);
        }
        self.activate_page(&page.id);
        Some(page.id)
    }

    /// Deletes the placement `id` of the active laser page (`0098` criterion 12): only the
    /// reference goes, in one step.
    pub fn delete_placement(&mut self, id: NodeId) -> bool {
        let deleted = self.recording(|this| this.document.delete_placement(id).is_ok());
        if deleted {
            self.manufacturing.selected_placement = None;
        }
        deleted
    }

    /// Installs a fixture of `kind` with its defaults at the end of the active setup (criterion
    /// 9). `None` for a kind the manifest does not declare.
    pub fn add_fixture(&mut self, kind: &str) -> Option<NodeId> {
        let manufacturing = self.active_manufacturing()?;
        let contribution = workbench_of(&manufacturing.process)?;
        let fixture = contribution.fixture_kind(kind)?;
        let stored = format!("{}/{}", manufacturing.process, fixture.id);
        let params = Param::defaults(&fixture.params);
        self.recording(|this| {
            this.document
                .add_fixture(manufacturing.id, &stored, fixture.label, &params)
                .ok()
        })
    }

    /// Removes the fixture `id` of the active manufacturing.
    pub fn delete_fixture(&mut self, id: NodeId) -> bool {
        if !self.fixture_of_active(id) {
            return false;
        }
        self.recording(|this| this.document.delete_fixture(id).is_ok())
    }

    /// Hides or shows the fixture `id` (criterion 10's eye button; one undo step).
    pub fn set_fixture_visible(&mut self, id: NodeId, visible: bool) -> bool {
        if !self.fixture_of_active(id) {
            return false;
        }
        self.recording(|this| {
            this.document
                .set_fixture_visible(id, visible)
                .unwrap_or(false)
        })
    }

    /// Sets the length parameter `key` of the fixture `id`, clamped into its bounds; the whole
    /// parameter map is written in one step.
    pub fn set_fixture_length(&mut self, id: NodeId, key: &str, value: Length) -> bool {
        let Some(manufacturing) = self.active_manufacturing() else {
            return false;
        };
        let Some(fixture) = manufacturing.fixtures.iter().find(|f| f.id == id) else {
            return false;
        };
        let Some(contribution) = workbench_of(&manufacturing.process) else {
            return false;
        };
        let Some(kind) = fixture_kind_of(&contribution, &manufacturing.process, &fixture.kind)
        else {
            return false;
        };
        let Some(accepted) = kind
            .params
            .iter()
            .find(|param| param.key == key)
            .and_then(|param| param.accept(ParamValue::Length(value)))
        else {
            return false;
        };
        let mut params = fixture.params.clone();
        params.insert(key.to_owned(), accepted.to_stored());
        self.recording(|this| {
            this.document
                .set_fixture_params(id, &params)
                .unwrap_or(false)
        })
    }

    fn fixture_of_active(&self, id: NodeId) -> bool {
        self.active_manufacturing()
            .is_some_and(|m| m.fixtures.iter().any(|f| f.id == id))
    }
}

/// The workbench contribution of the process `process`.
pub(super) fn workbench_of(process: &str) -> Option<WorkbenchContribution> {
    curvyo_ui_core::manifest(process)?.workbench().cloned()
}

/// The fixture kind a stored `<process>/<kind>` names in this contribution.
pub(super) fn fixture_kind_of<'a>(
    contribution: &'a WorkbenchContribution,
    process: &str,
    stored: &str,
) -> Option<&'a FixtureKind> {
    let kind = stored.strip_prefix(process)?.strip_prefix('/')?;
    contribution.fixture_kind(kind)
}
