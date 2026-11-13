//! The commands of the Materials tab on materials and their sets (`specs/0112` criteria 16 to
//! 21): each record edit is one step with the label of UX section 11; the order and the tab's
//! folding are no step.

use curvyo_document_core::{
    ADD_MATERIAL_COMMIT_LABEL, ADD_MATERIAL_SET_COMMIT_LABEL, DELETE_MATERIAL_COMMIT_LABEL,
    DELETE_MATERIAL_SET_COMMIT_LABEL, EDIT_MATERIAL_COMMIT_LABEL, NodeId,
    RENAME_MATERIAL_COMMIT_LABEL,
};
use curvyo_laser_core::library::{Material, PhotoWrite, Stamp, remove_photo};
use curvyo_laser_core::settings::{
    Field, Interval, MachineTie, Passes, PowerPercent, SetKind, SetNumbers, Settings, Thickness,
    TypedNumber,
};
use curvyo_laser_core::{FeedRate, new_record_id};

use super::super::Session;
use super::super::operations::typed_number;
use super::step::LibraryChange;
use super::{MaterialWrite, now_ms, sixteen};

/// The thickness typed into a form, or the chip text.
pub(in crate::session) fn thickness_of(value: f64) -> Result<Thickness, String> {
    if value.is_nan() {
        return Err("Enter a thickness.".to_owned());
    }
    Thickness::from_mm(value).ok_or_else(|| "Thickness is 0.1 to 100 mm.".to_owned())
}

/// The set kind of a kind text.
pub(in crate::session) fn set_kind(kind: &str) -> Result<SetKind, String> {
    SetKind::parse(kind).ok_or_else(|| "The material was not found.".to_owned())
}

/// The numbers a new set starts with (UX G7): 50 %, 3000 mm/min or the maximum feed, 1 pass and
/// for Engrave 0.10 mm (Proposal: a record set holds no empty numbers).
fn new_set_numbers(kind: SetKind, max_feed: FeedRate) -> SetNumbers {
    // invariant: 50 % and at least 1 mm/min are valid.
    #[allow(clippy::expect_used)]
    SetNumbers {
        settings: Settings {
            power: PowerPercent::from_tenths(500).expect("valid power"),
            speed: FeedRate::new(u64::from(max_feed.get().min(3000))).expect("valid speed"),
            passes: Passes::ONE,
        },
        interval: (kind == SetKind::Engrave).then_some(Interval::DEFAULT),
    }
}

/// The photo writes that remove every photo of `material` (deleted with it, ADR 0058 §4).
fn photo_removals(material: &Material) -> Vec<PhotoWrite> {
    material
        .tests
        .iter()
        .filter_map(|test| {
            test.photo
                .map(|kind| remove_photo(&material.id, test.id, kind))
        })
        .collect()
}

impl Session {
    /// The laser `profile_id` as material `id` names it: the page's, one of the material's, one
    /// picked by "+ Sets for another laser", or a profile of the library.
    pub(in crate::session) fn laser_tie(&self, id: &str, profile_id: &str) -> Option<MachineTie> {
        if let Some(laser) = self.active_laser().filter(|l| l.id == profile_id) {
            return Some(laser);
        }
        if let Some(laser) = self
            .materials
            .library
            .find(id)
            .and_then(|m| m.laser(profile_id))
        {
            return Some(laser.machine.clone());
        }
        if let Some(laser) = self
            .materials
            .tab
            .picked_lasers
            .get(id)
            .and_then(|picked| picked.iter().find(|l| l.id == profile_id))
        {
            return Some(laser.clone());
        }
        self.machine_profiles()
            .find(profile_id)
            .map(|profile| MachineTie {
                id: profile.id.clone(),
                name: profile.values.name.clone(),
            })
    }

    /// The maximum feed of the laser `profile_id`: its profile's, else the page's.
    fn laser_max_feed(&self, profile_id: &str) -> FeedRate {
        self.machine_profiles()
            .find(profile_id)
            .map_or_else(|| self.active_max_feed(), |p| p.values.max_feed)
    }

    /// Opens the New material form (UX 7.2); `for_grid` names the grid whose bar asked, which
    /// Add then picks the new material for (UX G9). View state, no step.
    pub fn open_material_form(&mut self, for_grid: Option<NodeId>) {
        self.materials.tab.form_open = true;
        self.materials.tab.form_for_grid = for_grid;
        self.materials.tab.open_test = None;
    }

    /// Closes the New material form without a record.
    pub fn cancel_material_form(&mut self) {
        self.materials.tab.form_open = false;
        self.materials.tab.form_for_grid = None;
    }

    /// Add on the New material form (criterion 17): a record with no sets, one step "Add
    /// material"; it is unfolded, and picked for the grid that opened the form in the same step
    /// (UX G9). Its id. `random` holds 16 caller bytes (the record id).
    ///
    /// # Errors
    /// "Enter a name.", the thickness text, or the refusal.
    pub fn add_material(
        &mut self,
        name: &str,
        thickness_mm: f64,
        random: &[u8],
    ) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a name.".to_owned());
        }
        if name.chars().count() > curvyo_laser_core::settings::MAX_MATERIAL_NAME_CHARS {
            return Err("Names are 1 to 60 characters.".to_owned());
        }
        let thickness = thickness_of(thickness_mm)?;
        let id = new_record_id(sixteen(random, 0));
        let material = Material {
            id: id.clone(),
            modified: now_ms(self),
            name: name.to_owned(),
            thickness,
            lasers: Vec::new(),
            tests: Vec::new(),
            history: Vec::new(),
        };
        let mut change = LibraryChange::default();
        self.add_record(&mut change, material)?;
        let grid = self.materials.tab.form_for_grid.take();
        let pick = grid.and_then(|group| self.prepare_grid_material(&mut change, group, &id).ok());
        let result = self.commit_library_change(ADD_MATERIAL_COMMIT_LABEL, change, |d| {
            if let Some(write) = &pick {
                write.apply(d)?;
            }
            Ok(())
        });
        result?;
        // The regenerated grid's items are new: the grid stays selected and the bar on it.
        if let (Some(group), Some(_)) = (grid, &pick) {
            self.select_grid(group);
        }
        self.materials.tab.form_open = false;
        self.materials.tab.unfolded = Some(id.clone());
        self.materials.tab.open_test = None;
        Ok(id)
    }

    /// Renames material `id` in place (criterion 17), one step "Rename material"; an empty or
    /// unchanged name does nothing (`Ok(false)`).
    ///
    /// # Errors
    /// A name over 60 characters, or the refusal.
    pub fn rename_material(&mut self, id: &str, name: &str) -> Result<bool, String> {
        let name = name.trim().to_owned();
        if name.is_empty() || self.materials.library.name_of(id) == Some(name.as_str()) {
            return Ok(false);
        }
        let now = now_ms(self);
        self.library_step(RENAME_MATERIAL_COMMIT_LABEL, |this, change| {
            this.change_record(change, id, |m| m.rename(&name, now))
        })?;
        Ok(true)
    }

    /// Sets the thickness of material `id`, one step "Edit material"; `Ok(false)` when unchanged.
    ///
    /// # Errors
    /// The chip text, or the refusal.
    pub fn set_material_thickness(&mut self, id: &str, mm: f64) -> Result<bool, String> {
        let thickness = thickness_of(mm)?;
        if self
            .materials
            .library
            .find(id)
            .is_some_and(|material| material.thickness == thickness)
        {
            return Ok(false);
        }
        let now = now_ms(self);
        self.library_step(EDIT_MATERIAL_COMMIT_LABEL, |this, change| {
            this.change_record(change, id, |m| m.set_thickness(thickness, now))
        })?;
        Ok(true)
    }

    /// Deletes material `id`, a damaged one included (criterion 17, the armed second press), one
    /// step "Delete material": its tests' ids are retired, its photos removed.
    ///
    /// # Errors
    /// The refusal (unknown, newer).
    pub fn delete_material(&mut self, id: &str) -> Result<(), String> {
        self.library_step(DELETE_MATERIAL_COMMIT_LABEL, |this, change| {
            if let Some(material) = this.delete_record(change, id)? {
                change.retire(material.tests.iter().map(|t| t.id));
                for write in photo_removals(&material) {
                    this.change_photo(change, write);
                }
            }
            Ok(())
        })?;
        if self.materials.tab.unfolded.as_deref() == Some(id) {
            self.materials.tab.unfolded = None;
            self.materials.tab.open_test = None;
        }
        Ok(())
    }

    /// Moves the row `id` to `index` of the rows (criterion 17): a list preference, no step.
    /// Whether it moved.
    pub fn move_material(&mut self, id: &str, index: usize) -> bool {
        let before: Vec<String> = self.materials.library.order().to_vec();
        let Some(text) = self.materials.library.move_row(id, index) else {
            return false;
        };
        if self.materials.library.order() == before.as_slice() {
            return false;
        }
        self.materials.outbox.push(MaterialWrite::Order { text });
        true
    }

    /// Unfolds the material row `id` (one at a time), or folds it with `None`: view state.
    pub fn unfold_material(&mut self, id: Option<&str>) {
        let tab = &mut self.materials.tab;
        tab.unfolded = id.map(str::to_owned);
        tab.open_test = None;
        tab.laser_pick_open = false;
        tab.page_laser_folded = false;
        tab.open_lasers.clear();
    }

    /// Folds or unfolds the laser block `profile_id` of the unfolded material: view state.
    pub fn toggle_material_laser(&mut self, profile_id: &str) {
        let page = self.active_profile_id();
        let tab = &mut self.materials.tab;
        if page.as_deref() == Some(profile_id) {
            tab.page_laser_folded = !tab.page_laser_folded;
        } else if !tab.open_lasers.remove(profile_id) {
            tab.open_lasers.insert(profile_id.to_owned());
        }
    }

    /// Unfolds or folds the "+ Sets for another laser" list (UX G12): view state.
    pub fn open_material_laser_pick(&mut self, open: bool) {
        self.materials.tab.laser_pick_open = open;
    }

    /// A pick of "+ Sets for another laser" (criterion 18): the laser's block appears unfolded
    /// with no set; the laser enters the record with its first set. Whether it was offered.
    pub fn pick_material_laser(&mut self, id: &str, profile_id: &str) -> bool {
        let Some(laser) = self.laser_tie(id, profile_id) else {
            return false;
        };
        let tab = &mut self.materials.tab;
        let picked = tab.picked_lasers.entry(id.to_owned()).or_default();
        if !picked.iter().any(|l| l.id == laser.id) {
            picked.push(laser);
        }
        tab.open_lasers.insert(profile_id.to_owned());
        tab.laser_pick_open = false;
        true
    }

    /// "+ `<kind>` set" of the laser `profile_id` in material `id` (criterion 20): the set with the
    /// starting numbers of UX G7 and the source Typed, one step "Add set"; a laser not in the
    /// material yet enters it (criterion 21). `random` holds 16 caller bytes (the history entry
    /// id).
    ///
    /// # Errors
    /// The refusal.
    pub fn add_material_set(
        &mut self,
        id: &str,
        profile_id: &str,
        kind: &str,
        random: &[u8],
    ) -> Result<(), String> {
        let kind = set_kind(kind)?;
        let laser = self
            .laser_tie(id, profile_id)
            .ok_or_else(|| "This machine is not in your profiles.".to_owned())?;
        if self
            .materials
            .library
            .find(id)
            .is_some_and(|m| m.set_for(profile_id, kind).is_some())
        {
            return Err("Nothing changed.".to_owned());
        }
        let numbers = new_set_numbers(kind, self.laser_max_feed(profile_id));
        let entry = new_record_id(sixteen(random, 0));
        let now = now_ms(self);
        self.library_step(ADD_MATERIAL_SET_COMMIT_LABEL, |this, change| {
            let stamp = Stamp {
                entry_id: &entry,
                now,
            };
            this.change_record(change, id, |m| m.type_set(&laser, kind, numbers, stamp))
        })
    }

    /// Types `value` into the `field` (`power`, `speed`, `passes`, `interval`) of the `kind` set
    /// of the laser `profile_id` in material `id` (criterion 19), one step "Edit material"; the
    /// source becomes Typed (G13). `Ok(false)` when the number is the stored one. `random` holds
    /// 16 caller bytes (the history entry id).
    ///
    /// # Errors
    /// The chip text, or the refusal.
    pub fn edit_material_set(
        &mut self,
        id: &str,
        profile_id: &str,
        kind: &str,
        field: &str,
        value: f64,
        random: &[u8],
    ) -> Result<bool, String> {
        let kind = set_kind(kind)?;
        let current = self
            .materials
            .library
            .find(id)
            .and_then(|m| m.set_for(profile_id, kind))
            .map(|set| set.numbers)
            .ok_or_else(|| "The material was not found.".to_owned())?;
        let laser = self
            .laser_tie(id, profile_id)
            .ok_or_else(|| "The material was not found.".to_owned())?;
        let max_feed = self.laser_max_feed(profile_id);
        let mut numbers = current;
        let typed = match field {
            "power" => Some(Field::Power),
            "speed" => Some(Field::Speed),
            "passes" => Some(Field::Passes),
            "interval" if kind == SetKind::Engrave => None,
            _ => return Err("Enter a number.".to_owned()),
        };
        match typed {
            Some(field) => match typed_number(field, value, max_feed)? {
                TypedNumber::Power(power) => numbers.settings.power = power,
                TypedNumber::Speed(speed) => numbers.settings.speed = speed,
                TypedNumber::Passes(passes) => numbers.settings.passes = passes,
            },
            None => numbers.interval = Some(Interval::typed(value).map_err(str::to_owned)?),
        }
        if numbers == current {
            return Ok(false);
        }
        let entry = new_record_id(sixteen(random, 0));
        let now = now_ms(self);
        self.library_step(EDIT_MATERIAL_COMMIT_LABEL, |this, change| {
            let stamp = Stamp {
                entry_id: &entry,
                now,
            };
            this.change_record(change, id, |m| m.type_set(&laser, kind, numbers, stamp))
        })?;
        Ok(true)
    }

    /// The set trash (UX G11): removes the `kind` set of the laser `profile_id` in material
    /// `id`, one step "Delete set". `random` holds 16 caller bytes (the history entry id).
    ///
    /// # Errors
    /// The refusal.
    pub fn delete_material_set(
        &mut self,
        id: &str,
        profile_id: &str,
        kind: &str,
        random: &[u8],
    ) -> Result<(), String> {
        let kind = set_kind(kind)?;
        let entry = new_record_id(sixteen(random, 0));
        let now = now_ms(self);
        self.library_step(DELETE_MATERIAL_SET_COMMIT_LABEL, |this, change| {
            let stamp = Stamp {
                entry_id: &entry,
                now,
            };
            this.change_record(change, id, |m| m.delete_set(profile_id, kind, stamp))
        })
    }
}
