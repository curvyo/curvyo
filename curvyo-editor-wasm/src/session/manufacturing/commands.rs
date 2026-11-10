//! The commands of a laser page (`0093` criteria 9, 10; `0098` criteria 4, 12): each is one
//! history step recorded like every other command; values are clamped by the extension's schema.

use curvyo_document_core::{
    ADD_OUTPUT_PAGE_COMMIT_LABEL, NewOutputPage, NodeId, PageCorner, ProfileChange,
};
use curvyo_extension_core::WorkbenchContribution;
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
        // The page starts with the profile's fixture set in the same step (`0102` criterion 29).
        let page = self.recording_unguarded(|this| {
            this.document.batch(ADD_OUTPUT_PAGE_COMMIT_LABEL, |d| {
                let page = d
                    .add_output_page(NewOutputPage {
                        process,
                        label: workbench.label,
                        profile: Some(&copy),
                        work_area: profile.work_area,
                    })
                    .ok()?;
                if let (Some(id), false) = (page.kind.manufacturing(), profile.fixtures.is_empty())
                {
                    let origin = profile.values.origin;
                    let anchor = PageCorner {
                        right: origin.is_right(),
                        bottom: origin.is_bottom(),
                    };
                    d.set_manufacturing_profile_and_fixtures(
                        id,
                        &copy,
                        profile.work_area,
                        anchor,
                        ProfileChange::Pick,
                        &profile.fixtures,
                        // A page that has just been added owns no objects to carry.
                        &curvyo_document_core::PageContents::new(),
                    )
                    .ok()?;
                }
                Some(page)
            })
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
        let deleted = self.recording_unguarded(|this| this.document.delete_placement(id).is_ok());
        if deleted {
            let kept: Vec<NodeId> = self
                .selection
                .ids()
                .iter()
                .copied()
                .filter(|&selected| selected != id)
                .collect();
            self.selection.set(&kept);
        }
        deleted
    }
}

/// The workbench contribution of the process `process`.
pub(super) fn workbench_of(process: &str) -> Option<WorkbenchContribution> {
    curvyo_ui_core::manifest(process)?.workbench().cloned()
}
