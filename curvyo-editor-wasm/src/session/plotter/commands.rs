//! The commands of a plotter page (`specs/0159-cutting-plotter` criteria 1 to 5, 8, `adrs.md`
//! PL11, ADR 0098): Add page, Pick and Save as profile, Set mat, Set grip and Mat grid, each one
//! history step; the machine's numbers are the Machines tab's.

use curvyo_cutter_core::{
    ADD_PAGE_COMMIT_LABEL, BUILTINS, CutterCopy, CutterProfile, DEFAULT_BUILTIN, Grip, MatRead,
    MatSize, PROCESS_ID, WORD, builtin, mat_params, resize_anchor, write_copy, write_record,
};
use curvyo_document_core::{FixtureAdd, NewFixture, NewOutputPage, NodeId, ProfileChange};
use curvyo_laser_core::{LibraryWrite, new_record_id};

use super::super::Session;

/// The step of Set mat.
pub(in crate::session) const SET_MAT_LABEL: &str = "set_mat";
/// The step of Set grip.
pub(in crate::session) const SET_GRIP_LABEL: &str = "set_grip";
/// The step of the Mat grid switch.
pub(in crate::session) const MAT_GRID_LABEL: &str = "set_mat_grid";
/// The step of Pick profile on a plotter page.
pub(in crate::session) const PICK_PROFILE_LABEL: &str = "pick_profile";

impl Session {
    /// Every plotter profile the pick lists: the built-ins, then the maker's (criterion 2).
    pub(in crate::session) fn plotter_profiles(&self) -> Vec<CutterProfile> {
        let mut out: Vec<CutterProfile> = BUILTINS
            .iter()
            .map(curvyo_cutter_core::Builtin::profile)
            .collect();
        out.extend(self.manufacturing.plotter.profiles.iter().cloned());
        out
    }

    pub(in crate::session) fn plotter_profile(&self, id: &str) -> Option<CutterProfile> {
        builtin(id).or_else(|| self.manufacturing.plotter.find(id).cloned())
    }

    /// Add page with Plotter (criterion 1): "Plotter N" with the copy of `profile` (else the
    /// Cricut Maker 3), its page the first mat's printed field or the work area, and one mat
    /// fixture per mat of the profile, the first active, in one step.
    pub(in crate::session) fn add_plotter_page(&mut self, profile: Option<&str>) -> Option<String> {
        let profile = profile
            .and_then(|id| self.plotter_profile(id))
            .unwrap_or_else(|| DEFAULT_BUILTIN.profile());
        let copy = write_copy(&CutterCopy::of(&profile));
        let size = profile.values.page_size();
        let page = self.recording_unguarded(|this| {
            this.document.batch(ADD_PAGE_COMMIT_LABEL, |d| {
                let page = d
                    .add_output_page(NewOutputPage {
                        process: PROCESS_ID,
                        label: WORD,
                        profile: Some(&copy),
                        work_area: size,
                    })
                    .ok()?;
                let id = page.kind.manufacturing()?;
                add_mats(d, id, &profile.values.mats, Grip::default(), true)?;
                Some(page)
            })
        })?;
        self.activate_page(&page.id);
        Some(page.id)
    }

    /// Pick profile on the active plotter page (criterion 2): the copy of `id`, the page resized
    /// to its first mat or its work area, the mats it offers present with the first active, in
    /// one step "Pick profile". `false` for another page, an unknown id or no change.
    pub fn pick_plotter_profile(&mut self, id: &str) -> bool {
        let (Some(active), Some(profile)) = (self.active_plotter(), self.plotter_profile(id))
        else {
            return false;
        };
        let copy = write_copy(&CutterCopy::of(&profile));
        let size = profile.values.page_size();
        let grip = active.active_mat().map_or(Grip::default(), |m| m.grip);
        let printing = active.active_mat().is_none_or(|m| m.grid);
        let contents = self.page_contents().unwrap_or_default();
        self.recording_unguarded(|this| {
            this.document
                .batch(PICK_PROFILE_LABEL, |d| {
                    let have: Vec<MatSize> = active.mats.iter().map(|m| m.1).collect();
                    let missing: Vec<MatSize> = profile
                        .values
                        .mats
                        .iter()
                        .copied()
                        .filter(|m| !have.contains(m))
                        .collect();
                    add_mats(d, active.id, &missing, grip, printing)?;
                    let first = profile.values.mats.first().copied();
                    for fixture in d.manufacturing(active.id)?.fixtures {
                        if let Some(size) = MatSize::of_fixture_kind(&fixture.kind) {
                            d.set_fixture_active(fixture.id, Some(size) == first).ok()?;
                        }
                    }
                    d.set_manufacturing_profile(
                        active.id,
                        &copy,
                        size,
                        resize_anchor(&active.copy),
                        ProfileChange::Pick,
                        &contents,
                    )
                    .ok()
                })
                .is_some()
        })
    }

    /// Save as profile (criterion 2): the copy becomes a new user record of kind `cutter`, synced
    /// as a laser profile (criterion 18), and the copy points at it in one step. `None` on
    /// another page; the new id otherwise.
    pub fn save_plotter_profile(&mut self, random: [u8; 16], now_ms: i64) -> Option<String> {
        let active = self.active_plotter()?;
        let profile = CutterProfile {
            id: new_record_id(random),
            modified: now_ms,
            values: active.copy.values.clone(),
        };
        let text = write_record(&profile);
        let copy = write_copy(&CutterCopy::of(&profile));
        let contents = self.page_contents().unwrap_or_default();
        let id = profile.id.clone();
        self.recording_unguarded(|this| {
            this.document
                .set_manufacturing_profile(
                    active.id,
                    &copy,
                    active.page.size,
                    resize_anchor(&active.copy),
                    ProfileChange::SaveAs,
                    &contents,
                )
                .unwrap_or(false)
        });
        self.manufacturing.plotter.put(profile);
        self.manufacturing.machine.queue(LibraryWrite::Write {
            id: id.clone(),
            text,
        });
        Some(id)
    }

    /// Set mat (criteria 1, 3; ADR 0098 decision 1): the mat `size` (`12x12`, `12x24`) becomes the
    /// active one and the page its cut area, anchored at the cut area's top-left corner, in one
    /// step "Set mat".
    pub fn set_plotter_mat(&mut self, size: &str) -> bool {
        let (Some(active), Some(size)) = (self.active_plotter(), MatSize::parse(size)) else {
            return false;
        };
        if !active.copy.values.mats.contains(&size)
            || active.active_mat().is_some_and(|m| m.size == size)
        {
            return false;
        }
        let grip = active.active_mat().map_or(Grip::default(), |m| m.grip);
        let printing = active.active_mat().is_none_or(|m| m.grid);
        let copy = write_copy(&active.copy);
        let contents = self.page_contents().unwrap_or_default();
        self.recording_unguarded(|this| {
            this.document
                .batch(SET_MAT_LABEL, |d| {
                    if !active.mats.iter().any(|m| m.1 == size) {
                        add_mats(d, active.id, &[size], grip, printing)?;
                    }
                    for fixture in d.manufacturing(active.id)?.fixtures {
                        if let Some(own) = MatSize::of_fixture_kind(&fixture.kind) {
                            d.set_fixture_active(fixture.id, own == size).ok()?;
                        }
                    }
                    d.set_manufacturing_profile(
                        active.id,
                        &copy,
                        size.page_size(),
                        resize_anchor(&active.copy),
                        ProfileChange::Pick,
                        &contents,
                    )
                    .ok()
                })
                .is_some()
        })
    }

    /// Set grip (criterion 3): the grip of every mat of the page, in one step "Set grip".
    pub fn set_plotter_grip(&mut self, grip: &str) -> bool {
        let Some(grip) = Grip::parse(grip) else {
            return false;
        };
        self.write_mats(SET_GRIP_LABEL, |mat| mat.grip = grip)
    }

    /// The Mat grid switch (criterion 5): on every mat of the page, one step "Mat grid".
    pub fn set_plotter_mat_grid(&mut self, on: bool) -> bool {
        self.write_mats(MAT_GRID_LABEL, |mat| mat.grid = on)
    }

    fn write_mats(&mut self, label: &str, edit: impl Fn(&mut MatRead)) -> bool {
        let Some(active) = self.active_plotter() else {
            return false;
        };
        let Some(manufacturing) = self.document.manufacturing(active.id) else {
            return false;
        };
        let writes: Vec<(NodeId, serde_json::Map<String, serde_json::Value>)> = manufacturing
            .fixtures
            .iter()
            .filter(|f| MatSize::is_mat_kind(&f.kind))
            .filter_map(|f| {
                let mut mat = MatRead::of(&f.kind, &f.params);
                edit(&mut mat);
                let params = mat_params(mat.grip, mat.grid);
                (params != f.params).then_some((f.id, params))
            })
            .collect();
        if writes.is_empty() {
            return false;
        }
        self.recording_unguarded(|this| {
            this.document.batch(label, |d| {
                writes
                    .iter()
                    .all(|(id, params)| d.set_fixture_params(*id, params).is_ok())
            })
        })
    }
}

/// Adds one mat fixture per size of `sizes` to `manufacturing`, the first of them active only
/// when the page has no active mat yet.
fn add_mats(
    d: &curvyo_document_core::Document,
    manufacturing: NodeId,
    sizes: &[MatSize],
    grip: Grip,
    printing: bool,
) -> Option<()> {
    let params = mat_params(grip, printing);
    for (index, size) in sizes.iter().enumerate() {
        let id = d
            .add_fixture(
                manufacturing,
                &NewFixture {
                    add: FixtureAdd::Holder,
                    kind: size.fixture_kind(),
                    label: "Mat",
                    params: &params,
                    matrix: None,
                },
            )
            .ok()?;
        if index > 0 {
            d.set_fixture_active(id, false).ok()?;
        }
    }
    Some(())
}
