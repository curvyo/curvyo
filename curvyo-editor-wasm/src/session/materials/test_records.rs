//! The commands of a test in the Materials tab (`specs/0112` criteria 22 to 27): open, judge,
//! note, photo and delete, each one step with the label of UX section 11.

use curvyo_document_core::{
    CROP_TEST_PHOTO_COMMIT_LABEL, DELETE_TEST_COMMIT_LABEL, EDIT_TEST_NOTE_COMMIT_LABEL,
    JUDGE_TEST_COMMIT_LABEL, REMOVE_TEST_PHOTO_COMMIT_LABEL, SET_TEST_PHOTO_COMMIT_LABEL,
};
use curvyo_laser_core::grids::TestId;
use curvyo_laser_core::library::{RecordRefusal, Stamp, remove_photo, write_photo};
use curvyo_laser_core::new_record_id;
use curvyo_laser_core::settings::CellIndex;

use super::super::Session;
use super::{now_ms, sixteen};

fn test_id(text: &str) -> Result<TestId, String> {
    TestId::parse(text).ok_or_else(|| "The test was not found.".to_owned())
}

impl Session {
    /// Opens the test `test` of the unfolded material in place of its body (criterion 23), or
    /// goes back with `None`: view state.
    pub fn open_material_test(&mut self, test: Option<&str>) {
        self.materials.tab.open_test = test.and_then(TestId::parse);
    }

    /// A tap on the cell `column`, `row` of the test (criterion 24): the best cell, and the
    /// laser's set of the test's kind from its values, one step "Choose best cell"; a tap on the
    /// best cell changes nothing (G14). `random` holds 16 caller bytes (the history entry id).
    ///
    /// # Errors
    /// The refusal ("Nothing changed." for the best cell).
    pub fn judge_test(
        &mut self,
        material: &str,
        test: &str,
        column: u8,
        row: u8,
        random: &[u8],
    ) -> Result<(), String> {
        let test = test_id(test)?;
        let entry = new_record_id(sixteen(random, 0));
        let now = now_ms(self);
        self.library_step(JUDGE_TEST_COMMIT_LABEL, |this, change| {
            let stamp = Stamp {
                entry_id: &entry,
                now,
            };
            this.change_record(change, material, |m| {
                m.judge(test, CellIndex { column, row }, stamp)
            })
        })
    }

    /// Leaving the Note field (criterion 26): one step "Edit note"; an unchanged note does
    /// nothing (`Ok(false)`).
    ///
    /// # Errors
    /// "Notes are at most 500 characters.", or the refusal.
    pub fn set_test_note(
        &mut self,
        material: &str,
        test: &str,
        note: &str,
    ) -> Result<bool, String> {
        let test = test_id(test)?;
        let now = now_ms(self);
        match self.library_step(EDIT_TEST_NOTE_COMMIT_LABEL, |this, change| {
            this.change_record(change, material, |m| m.set_note(test, note, now))
        }) {
            Ok(()) => Ok(true),
            Err(refused) if refused == RecordRefusal::Unchanged.to_string() => Ok(false),
            Err(refused) => Err(refused),
        }
    }

    /// The photo bytes the frontend encoded (criterion 27): checked (JPEG or PNG, at most 600 px
    /// and 1 MiB), written next to the record, one step "Add photo", or "Crop photo" with
    /// `cropped`; a photo of the other kind is removed in the same step.
    ///
    /// # Errors
    /// "Use a JPEG or PNG image.", the size texts, or the refusal.
    pub fn set_test_photo(
        &mut self,
        material: &str,
        test: &str,
        bytes: &[u8],
        cropped: bool,
    ) -> Result<(), String> {
        let test = test_id(test)?;
        let (kind, write) =
            write_photo(material, test, bytes.to_vec()).map_err(|refusal| refusal.to_string())?;
        let old = self
            .materials
            .library
            .find(material)
            .and_then(|m| m.test(test))
            .ok_or_else(|| "The test was not found.".to_owned())?
            .photo;
        let label = if cropped {
            CROP_TEST_PHOTO_COMMIT_LABEL
        } else {
            SET_TEST_PHOTO_COMMIT_LABEL
        };
        let now = now_ms(self);
        self.library_step(label, |this, change| {
            if old == Some(kind) {
                // The same kind: the record keeps its photo; `modified` still rises so the step
                // has a marker (ADR 0055 §2).
                this.change_record(change, material, |m| {
                    m.modified = curvyo_laser_core::settings::next_modified(m.modified, now);
                    Ok(())
                })?;
            } else {
                this.change_record(change, material, |m| m.set_photo(test, Some(kind), now))?;
            }
            if let Some(old) = old.filter(|old| *old != kind) {
                this.change_photo(change, remove_photo(material, test, old));
            }
            this.change_photo(change, write);
            Ok(())
        })
    }

    /// The photo's trash (criterion 27): one step "Remove photo".
    ///
    /// # Errors
    /// The refusal.
    pub fn remove_test_photo(&mut self, material: &str, test: &str) -> Result<(), String> {
        let test = test_id(test)?;
        let old = self
            .materials
            .library
            .find(material)
            .and_then(|m| m.test(test))
            .and_then(|t| t.photo)
            .ok_or_else(|| "Nothing changed.".to_owned())?;
        let now = now_ms(self);
        self.library_step(REMOVE_TEST_PHOTO_COMMIT_LABEL, |this, change| {
            this.change_record(change, material, |m| m.set_photo(test, None, now))?;
            this.change_photo(change, remove_photo(material, test, old));
            Ok(())
        })
    }

    /// The test's armed trash (criterion 25): one step "Delete test"; its id is retired and its
    /// photo removed; the detail goes back to the material.
    ///
    /// # Errors
    /// The refusal.
    pub fn delete_test(&mut self, material: &str, test: &str) -> Result<(), String> {
        let test = test_id(test)?;
        let now = now_ms(self);
        self.library_step(DELETE_TEST_COMMIT_LABEL, |this, change| {
            let photo = this
                .materials
                .library
                .find(material)
                .and_then(|m| m.test(test))
                .and_then(|t| t.photo);
            this.change_record(change, material, |m| m.delete_test(test, now).map(|_| ()))?;
            change.retire([test]);
            if let Some(kind) = photo {
                this.change_photo(change, remove_photo(material, test, kind));
            }
            Ok(())
        })?;
        if self.materials.tab.open_test == Some(test) {
            self.materials.tab.open_test = None;
        }
        Ok(())
    }
}
