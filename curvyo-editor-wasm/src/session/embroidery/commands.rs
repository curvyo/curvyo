//! The commands of an embroidery page (`specs/0160-embroidery` criteria 1 to 3, UX 3, 4): Add
//! page, Pick and Save as profile, Edit speed and Set hoop, each one history step.

use curvyo_document_core::{
    FixtureAdd, NewFixture, NewOutputPage, NodeId, PageCorner, ProfileChange,
};
use curvyo_embroidery_core::{
    ADD_PAGE_COMMIT_LABEL, BUILTINS, DEFAULT_BUILTIN, EmbroideryCopy, EmbroideryProfile, HoopSize,
    PROCESS_ID, StitchesPerMin, WORD, builtin, write_copy, write_record,
};
use curvyo_laser_core::{LibraryWrite, new_record_id};

use super::super::Session;

/// The step of Set hoop.
pub(in crate::session) const SET_HOOP_LABEL: &str = "set_hoop";
/// The step of the Speed field.
pub(in crate::session) const EDIT_SPEED_LABEL: &str = "edit_embroidery_speed";
/// The step of Pick profile on an embroidery page.
const PICK_PROFILE_LABEL: &str = "pick_profile";
/// The message of a speed out of range (UX 10).
pub(in crate::session) const SPEED_MESSAGE: &str = "Speed is 100 to 1,500 stitches/min.";

/// The page grows from its top-left corner; the hoop stays centred on it.
const TOP_LEFT: PageCorner = PageCorner {
    right: false,
    bottom: false,
};

impl Session {
    /// Every embroidery profile the pick lists: the built-ins, then the maker's (criterion 2).
    pub(in crate::session) fn embroidery_profiles(&self) -> Vec<EmbroideryProfile> {
        let mut out: Vec<EmbroideryProfile> = BUILTINS
            .iter()
            .map(curvyo_embroidery_core::Builtin::profile)
            .collect();
        out.extend(self.manufacturing.embroidery.profiles.iter().cloned());
        out
    }

    fn embroidery_profile(&self, id: &str) -> Option<EmbroideryProfile> {
        builtin(id).or_else(|| self.manufacturing.embroidery.find(id).cloned())
    }

    /// Add page with Embroidery (criterion 1): "Embroidery N" with the copy of `profile` (else
    /// the W6 EU7), its page the largest sewing field plus 20 mm, and one hoop fixture per hoop
    /// of the profile, the first active, in one step.
    pub(in crate::session) fn add_embroidery_page(
        &mut self,
        profile: Option<&str>,
    ) -> Option<String> {
        let profile = profile
            .and_then(|id| self.embroidery_profile(id))
            .unwrap_or_else(|| DEFAULT_BUILTIN.profile());
        let copy = write_copy(&EmbroideryCopy::of(&profile));
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
                add_hoops(d, id, &profile.values.hoops, true)?;
                Some(page)
            })
        })?;
        self.activate_page(&page.id);
        Some(page.id)
    }

    /// Pick profile on the active embroidery page (criterion 2): the copy of `id`, the page sized
    /// to its hoops, its hoops present with the first active, in one step. `false` for another
    /// page, an unknown id or no change.
    pub fn pick_embroidery_profile(&mut self, id: &str) -> bool {
        let (Some(active), Some(profile)) = (self.active_embroidery(), self.embroidery_profile(id))
        else {
            return false;
        };
        let copy = write_copy(&EmbroideryCopy::of(&profile));
        let size = profile.values.page_size();
        let contents = self.page_contents().unwrap_or_default();
        self.recording_unguarded(|this| {
            this.document
                .batch(PICK_PROFILE_LABEL, |d| {
                    let have: Vec<HoopSize> = active.hoops.iter().map(|h| h.1).collect();
                    let missing: Vec<HoopSize> = profile
                        .values
                        .hoops
                        .iter()
                        .copied()
                        .filter(|h| !have.contains(h))
                        .collect();
                    add_hoops(d, active.id, &missing, false)?;
                    let first = profile.values.default_hoop();
                    activate(d, active.id, first)?;
                    d.set_manufacturing_profile(
                        active.id,
                        &copy,
                        size,
                        TOP_LEFT,
                        ProfileChange::Pick,
                        &contents,
                    )
                    .ok()
                })
                .is_some()
        })
    }

    /// Save as profile (criterion 2): the copy becomes a new user record of kind `embroidery`,
    /// synced as a laser profile, and the copy points at it in one step. The new id; `None` on
    /// another page.
    pub fn save_embroidery_profile(&mut self, random: [u8; 16], now_ms: i64) -> Option<String> {
        let active = self.active_embroidery()?;
        let profile = EmbroideryProfile {
            id: new_record_id(random),
            modified: now_ms,
            values: active.copy.values.clone(),
        };
        let text = write_record(&profile);
        let copy = write_copy(&EmbroideryCopy::of(&profile));
        let contents = self.page_contents().unwrap_or_default();
        let id = profile.id.clone();
        self.recording_unguarded(|this| {
            this.document
                .set_manufacturing_profile(
                    active.id,
                    &copy,
                    active.page.size,
                    TOP_LEFT,
                    ProfileChange::SaveAs,
                    &contents,
                )
                .unwrap_or(false)
        });
        self.manufacturing.embroidery.put(profile);
        self.manufacturing.machine.queue(LibraryWrite::Write {
            id: id.clone(),
            text,
        });
        Some(id)
    }

    /// The Speed field (UX 3): the copy's stitches per minute from the typed `text`, one step
    /// "Edit speed".
    ///
    /// # Errors
    /// The field message for a value that is no whole number from 100 to 1 500.
    pub fn set_embroidery_speed(&mut self, text: &str) -> Result<bool, String> {
        let active = self.active_embroidery().ok_or_else(String::new)?;
        let speed = text
            .trim()
            .replace(',', "")
            .parse::<u64>()
            .ok()
            .and_then(StitchesPerMin::new)
            .ok_or_else(|| SPEED_MESSAGE.to_owned())?;
        let mut copy = active.copy.clone();
        copy.values.speed = speed;
        let map = write_copy(&copy);
        let contents = self.page_contents().unwrap_or_default();
        Ok(self.recording_unguarded(|this| {
            this.document.batch(EDIT_SPEED_LABEL, |d| {
                d.set_manufacturing_profile(
                    active.id,
                    &map,
                    active.page.size,
                    TOP_LEFT,
                    ProfileChange::EditName,
                    &contents,
                )
                .unwrap_or(false)
            })
        }))
    }

    /// Set hoop (criterion 3): the hoop `key` (`150x250`, `100x100`) of the profile becomes the
    /// active one in one step "Set hoop"; the page keeps its size.
    pub fn set_embroidery_hoop(&mut self, key: &str) -> bool {
        let (Some(active), Some(size)) = (self.active_embroidery(), HoopSize::parse(key)) else {
            return false;
        };
        if !active.copy.values.hoops.contains(&size)
            || (active.hoop.size == size && !active.hoop.damaged && active.hoop.id.is_some())
        {
            return false;
        }
        self.recording_unguarded(|this| {
            this.document
                .batch(SET_HOOP_LABEL, |d| {
                    if !active.hoops.iter().any(|h| h.1 == size) {
                        add_hoops(d, active.id, &[size], false)?;
                    }
                    activate(d, active.id, size)
                })
                .is_some()
        })
    }

    /// The hoop of the active embroidery page, for the tests.
    #[cfg(test)]
    pub(in crate::session) fn embroidery_hoop(&self) -> Option<HoopSize> {
        self.active_embroidery().map(|p| p.hoop.size)
    }
}

/// Makes the hoop fixture of `size` the only active hoop of `manufacturing`.
fn activate(
    d: &curvyo_document_core::Document,
    manufacturing: NodeId,
    size: HoopSize,
) -> Option<()> {
    for fixture in d.manufacturing(manufacturing)?.fixtures {
        if HoopSize::is_hoop_kind(&fixture.kind) {
            let on = HoopSize::of_fixture_kind(&fixture.kind) == Some(size);
            d.set_fixture_active(fixture.id, on).ok()?;
        }
    }
    Some(())
}

/// Adds one hoop fixture per size of `sizes` to `manufacturing`, the first active when `first`.
fn add_hoops(
    d: &curvyo_document_core::Document,
    manufacturing: NodeId,
    sizes: &[HoopSize],
    first: bool,
) -> Option<()> {
    let params = serde_json::Map::new();
    for (index, size) in sizes.iter().enumerate() {
        let id = d
            .add_fixture(
                manufacturing,
                &NewFixture {
                    add: FixtureAdd::Holder,
                    kind: size.fixture_kind(),
                    label: "Hoop",
                    params: &params,
                    matrix: None,
                },
            )
            .ok()?;
        if index > 0 || !first {
            d.set_fixture_active(id, false).ok()?;
        }
    }
    Some(())
}
