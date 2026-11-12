//! The library edits of the Materials section and the Save as material form (`specs/0028`
//! criteria 15, 20 to 22): each one record write and one document step of the open project
//! (ADR 0055 §9), whose texts go into the journal; the order and the row selection are no step.

use curvyo_document_core::MaterialStep;
use curvyo_laser_core::material_library::{
    LibraryRefusal, Materials, NewMaterial, SaveTarget, TestResult,
};
use curvyo_laser_core::settings::{
    Field, MachineTie, Passes, SetKind, Settings, Thickness, TypedNumber, next_modified,
};
use curvyo_laser_core::{LibraryWrite, new_record_id};

use super::super::Session;
use super::super::operations::typed_number;
use super::{MaterialWrite, now_ms, sixteen};

/// What the Save as material form sends (UX notes §9).
#[derive(Debug, Clone, PartialEq)]
pub struct SaveAsMaterial {
    /// The material the set goes into; `None` for a new material.
    pub existing: Option<String>,
    /// The new material's name (ignored with `existing`).
    pub name: String,
    /// The new material's thickness in mm (ignored with `existing`).
    pub thickness_mm: f64,
    /// The passes of the set.
    pub passes: f64,
}

/// The passes typed into a form, or the chip text.
fn passes_of(value: f64) -> Result<Passes, String> {
    if !value.is_finite() || value.fract() != 0.0 || value < 0.0 {
        return Err("Passes are 1 to 20.".to_owned());
    }
    // checked above: a whole, non-negative number
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Passes::new(value as u64).ok_or_else(|| "Passes are 1 to 20.".to_owned())
}

/// The thickness typed into a form, or the chip text.
fn thickness_of(value: f64) -> Result<Thickness, String> {
    if value.is_nan() {
        return Err("Enter a thickness.".to_owned());
    }
    Thickness::from_mm(value).ok_or_else(|| "Thickness is 0.1 to 100 mm.".to_owned())
}

/// The set kind of a stored kind text.
fn set_kind(kind: &str) -> Result<SetKind, String> {
    SetKind::parse(kind).ok_or_else(|| "The material was not found.".to_owned())
}

impl Session {
    /// One library edit as one step labelled `step` (ADR 0055 §9): the record write, the refresh
    /// of every operation pointing to the material and the root marker, the journal entry.
    fn library_step(
        &mut self,
        step: MaterialStep,
        id: &str,
        edit: impl FnOnce(&mut Materials, u64) -> Result<LibraryWrite, LibraryRefusal>,
    ) -> Result<(), String> {
        let now = now_ms(self);
        let library = &mut self.materials.library;
        let before = library.text(id).map(str::to_owned);
        let name_before = library.name_of(id).map(str::to_owned);
        let modified_before = library.find(id).map_or(0, |material| material.modified);
        let write = edit(library, now).map_err(|refusal| refusal.to_string())?;
        let after = library.text(id).map(str::to_owned);
        let name = library
            .name_of(id)
            .map(str::to_owned)
            .or(name_before)
            .unwrap_or_default();
        // A delete stamps the marker past the record's last `modified`, so the step always writes.
        let modified = library.find(id).map_or_else(
            || next_modified(modified_before, now),
            |material| material.modified,
        );
        let refreshed = self.refreshed_settings(Some(id));
        let steps = self.undo_len();
        let committed = self.recording_unguarded(|this| {
            this.document
                .commit_material_edit(step, id, modified, &refreshed)
                .is_ok()
        });
        if !committed {
            // The record goes back to what it was: the step does not exist.
            let _ = self
                .materials
                .library
                .revert(id, after.as_deref(), before.as_deref(), now);
            return Err("Nothing changed.".to_owned());
        }
        if self.undo_len() > steps
            && let Some(target) = self.history.stack.undo_target()
        {
            self.materials
                .journal
                .record(target.step, id, &name, before, after);
        }
        self.materials.outbox.push(MaterialWrite::from(write));
        Ok(())
    }

    /// Save as material (criterion 15): the marked cell of the grid in the Grid block as a new
    /// material or as the grid kind's set of an existing one, one step "Save as material". The
    /// material's id, or the chip or notice text. `random` holds 32 caller bytes (the record id
    /// and the history entry id).
    ///
    /// # Errors
    /// The text of the first invalid field or of the refusal.
    pub fn save_as_material(
        &mut self,
        request: &SaveAsMaterial,
        random: &[u8],
    ) -> Result<String, String> {
        let (recipe, cell) = self
            .worked_test_cell()
            .ok_or_else(|| "No cell marked".to_owned())?;
        let target = if let Some(id) = &request.existing {
            SaveTarget::Existing(id.clone())
        } else {
            let name = request.name.trim().to_owned();
            if name.is_empty() {
                return Err("Enter a name.".to_owned());
            }
            let thickness = thickness_of(request.thickness_mm)?;
            let copy = self
                .active_copy()
                .ok_or_else(|| "No laser page.".to_owned())?;
            let machine = MachineTie {
                id: copy
                    .read
                    .copy
                    .id
                    .clone()
                    .ok_or_else(|| "This machine is not in your profiles.".to_owned())?,
                name: copy.read.copy.values.name.clone(),
            };
            SaveTarget::New(NewMaterial {
                id: new_record_id(sixteen(random, 0)),
                name,
                thickness,
                machine,
            })
        };
        let passes = passes_of(request.passes)?;
        let id = match &target {
            SaveTarget::Existing(id) => id.clone(),
            SaveTarget::New(new) => new.id.clone(),
        };
        let test = TestResult {
            kind: recipe.kind,
            grid: recipe.range,
            cell,
            set: Settings {
                passes,
                ..recipe.cell_settings(cell)
            },
        };
        let entry = new_record_id(sixteen(random, 16));
        self.library_step(MaterialStep::SaveAsMaterial, &id, |library, now| {
            library.save_test(target, test, &entry, now)
        })?;
        Ok(id)
    }

    /// Types `value` into the `field` (`power`, `speed`, `passes`) of the `kind` set of material
    /// `id` (criterion 22), one step "Edit material"; the source becomes "edited by hand".
    /// `Ok(false)` when the number is the stored one. `random` holds 16 caller bytes (the history
    /// entry id).
    ///
    /// # Errors
    /// The chip text, or the refusal.
    pub fn edit_material_set(
        &mut self,
        id: &str,
        kind: &str,
        field: &str,
        value: f64,
        random: &[u8],
    ) -> Result<bool, String> {
        let kind = set_kind(kind)?;
        let field = match field {
            "power" => Field::Power,
            "speed" => Field::Speed,
            "passes" => Field::Passes,
            _ => return Err("Enter a number.".to_owned()),
        };
        let max_feed = self
            .active_copy()
            .map_or(curvyo_laser_core::FeedRate::DEFAULT, |copy| {
                copy.read.copy.values.max_feed
            });
        let number = typed_number(field, value, max_feed)?;
        let Some(current) = self
            .materials
            .library
            .find(id)
            .and_then(|material| material.set(kind).copied())
        else {
            return Err("The material was not found.".to_owned());
        };
        let settings = match number {
            TypedNumber::Power(power) => Settings { power, ..current },
            TypedNumber::Speed(speed) => Settings { speed, ..current },
            TypedNumber::Passes(passes) => Settings { passes, ..current },
        };
        if settings == current {
            return Ok(false);
        }
        let entry = new_record_id(sixteen(random, 0));
        self.library_step(MaterialStep::EditMaterial, id, |library, now| {
            library.edit_set(id, kind, settings, entry, now)
        })?;
        Ok(true)
    }

    /// Removes the `kind` set of material `id` (criterion 22, the set trash), one step "Edit
    /// material". `random` holds 16 caller bytes (the history entry id).
    ///
    /// # Errors
    /// The refusal.
    pub fn remove_material_set(
        &mut self,
        id: &str,
        kind: &str,
        random: &[u8],
    ) -> Result<(), String> {
        let kind = set_kind(kind)?;
        let entry = new_record_id(sixteen(random, 0));
        self.library_step(MaterialStep::EditMaterial, id, |library, now| {
            library.remove_set(id, kind, entry, now)
        })
    }

    /// Renames material `id` in place (criterion 20), one step "Rename material"; an empty name
    /// reverts and an unchanged one does nothing (`Ok(false)`).
    ///
    /// # Errors
    /// A name over 60 characters, or the refusal.
    pub fn rename_material(&mut self, id: &str, name: &str) -> Result<bool, String> {
        let name = name.trim();
        if name.is_empty() || self.materials.library.name_of(id) == Some(name) {
            return Ok(false);
        }
        let name = name.to_owned();
        self.library_step(MaterialStep::RenameMaterial, id, |library, now| {
            library.rename(id, name, now)
        })?;
        Ok(true)
    }

    /// Sets the thickness of material `id` (criterion 22), one step "Edit material"; `Ok(false)`
    /// when unchanged.
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
        self.library_step(MaterialStep::EditMaterial, id, |library, now| {
            library.set_thickness(id, thickness, now)
        })?;
        Ok(true)
    }

    /// Deletes material `id`, a damaged one included (criterion 21, the armed second press), one
    /// step "Delete material"; Ctrl+Z in this session writes it back.
    ///
    /// # Errors
    /// The refusal (unknown, newer).
    pub fn delete_material(&mut self, id: &str) -> Result<(), String> {
        self.library_step(MaterialStep::DeleteMaterial, id, |library, _| {
            library.delete(id)
        })?;
        if self.materials.selected.as_deref() == Some(id) {
            self.materials.selected = None;
        }
        Ok(())
    }

    /// Moves the row `id` to `index` of this machine's rows (criterion 22): a list preference,
    /// no step. Whether it moved.
    pub fn move_material(&mut self, id: &str, index: usize) -> bool {
        let Some(profile) = self.active_profile_id() else {
            return false;
        };
        let before: Vec<String> = self.materials.library.order().to_vec();
        let Some(text) = self.materials.library.move_row(&profile, id, index) else {
            return false;
        };
        if self.materials.library.order() == before.as_slice() {
            return false;
        }
        self.materials.outbox.push(MaterialWrite::Order { text });
        true
    }

    /// Selects the material row `id` (its fields open), or none: view state.
    pub fn select_material(&mut self, id: Option<&str>) {
        self.materials.selected = id.map(str::to_owned);
    }

    /// The profile id of the active laser page's copy.
    pub(in crate::session) fn active_profile_id(&self) -> Option<String> {
        self.active_copy()?.read.copy.id
    }
}
