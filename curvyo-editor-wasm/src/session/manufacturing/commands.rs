//! The commands of a laser page (`0093` criteria 9, 10; `0098` criteria 4, 12): each is one
//! history step recorded like every other command; values are clamped by the extension's schema.

use curvyo_document_core::{
    ADD_OUTPUT_PAGE_COMMIT_LABEL, DocumentSize, NewOutputPage, NodeId, PageCorner, ProfileChange,
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
        if let Some((label, size)) = family_page(process) {
            return self.add_family_page(process, workbench.label, label, size);
        }
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
                let sets = &profile.fixture_sets;
                if let (Some(id), false) = (page.kind.manufacturing(), sets.is_empty()) {
                    let origin = profile.values.origin;
                    let anchor = PageCorner {
                        right: origin.is_right(),
                        bottom: origin.is_bottom(),
                    };
                    d.set_manufacturing_profile_and_sets(
                        id,
                        &copy,
                        profile.work_area,
                        anchor,
                        ProfileChange::Pick,
                        curvyo_document_core::ProfileSets {
                            sets,
                            modified: profile.modified,
                        },
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

    /// Add page of a machine family other than the laser (ADR 0095 §11 step G2): "<word> N" of
    /// the family's default built-in size, committed under the family's `label`, and no profile
    /// copy yet (the family copy comes with its first slice).
    fn add_family_page(
        &mut self,
        process: &str,
        word: &str,
        label: &str,
        size: DocumentSize,
    ) -> Option<String> {
        let page = self.recording_unguarded(|this| {
            this.document.batch(label, |d| {
                d.add_output_page(NewOutputPage {
                    process,
                    label: word,
                    profile: None,
                    work_area: size,
                })
                .ok()
            })
        })?;
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

/// The commit label and page size of Add page with the machine family `process`; `None` for the
/// laser and every process that is not one of the families of ADR 0095.
fn family_page(process: &str) -> Option<(&'static str, DocumentSize)> {
    match process {
        curvyo_cutter_core::PROCESS_ID => Some((
            curvyo_cutter_core::ADD_PAGE_COMMIT_LABEL,
            curvyo_cutter_core::DEFAULT_BUILTIN.page_size(),
        )),
        curvyo_embroidery_core::PROCESS_ID => Some((
            curvyo_embroidery_core::ADD_PAGE_COMMIT_LABEL,
            curvyo_embroidery_core::DEFAULT_BUILTIN.page_size(),
        )),
        curvyo_cnc_core::PROCESS_ID => Some((
            curvyo_cnc_core::ADD_PAGE_COMMIT_LABEL,
            curvyo_cnc_core::DEFAULT_BUILTIN.page_size(),
        )),
        _ => None,
    }
}
