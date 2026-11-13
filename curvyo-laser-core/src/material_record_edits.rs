//! The edits of one format-2 material record: sets per laser, tests, judging, note and photo
//! (ADR 0058 §2, §3; `specs/0112` criteria 17 to 26).

use crate::material::{
    CellIndex, MAX_HISTORY_ENTRIES, MAX_MATERIAL_NAME_CHARS, MachineTie, SetKind, SetNumbers,
    Thickness, next_modified,
};
use crate::material_record::{
    HistoryEntry, HistoryEvent, LaserSets, MAX_LASERS, MAX_NOTE_CHARS, MAX_TESTS, Material,
    PhotoKind, Set, SetSource, TestRecord,
};
use crate::test_id::TestId;

/// Why an edit of a record changed nothing; the text is the notice.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RecordRefusal {
    /// The edit changes nothing.
    #[error("Nothing changed.")]
    Unchanged,
    /// A name outside 1 to 60 characters.
    #[error("Names are 1 to 60 characters.")]
    InvalidName,
    /// The numbers do not fit the kind: an interval exactly for Engrave.
    #[error("The numbers do not fit the set.")]
    WrongNumbers,
    /// The material already holds sets for 16 lasers.
    #[error("A material holds sets for at most 16 lasers.")]
    TooManyLasers,
    /// The material already holds 200 tests.
    #[error("A material holds at most 200 tests.")]
    TooManyTests,
    /// The edit would pass 500 history entries.
    #[error("{name} has 500 history entries; nothing was saved.")]
    FullHistory {
        /// The material's name.
        name: String,
    },
    /// The laser has no set of the kind.
    #[error("There is no {kind} set for this laser.")]
    NoSet {
        /// The kind's word.
        kind: &'static str,
    },
    /// No test has the id.
    #[error("The test was not found.")]
    UnknownTest,
    /// The test is Lasered: its recipe stays as burned (ADR 0058 §3).
    #[error("The test is lasered.")]
    Lasered,
    /// The cell lies outside the test's grid.
    #[error("The cell is outside the grid.")]
    NoCell,
    /// A note over 500 characters.
    #[error("Notes are at most 500 characters.")]
    LongNote,
}

/// A history entry about to be appended: its id and date, passed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp<'a> {
    /// A fresh lowercase UUID v4.
    pub entry_id: &'a str,
    /// Unix milliseconds.
    pub now: u64,
}

impl Material {
    fn touch(&mut self, now: u64) {
        self.modified = next_modified(self.modified, now);
    }

    fn room_for_entry(&self) -> Result<(), RecordRefusal> {
        if self.history.len() >= MAX_HISTORY_ENTRIES {
            return Err(RecordRefusal::FullHistory {
                name: self.name.clone(),
            });
        }
        Ok(())
    }

    fn append(&mut self, stamp: Stamp<'_>, machine: &str, kind: SetKind, event: HistoryEvent) {
        self.history.push(HistoryEntry {
            id: stamp.entry_id.to_owned(),
            date: stamp.now,
            machine: machine.to_owned(),
            kind,
            event,
        });
    }

    /// Writes `set` as the laser's set of `kind`, adding the laser when it has no sets yet
    /// (criterion 21), and appends a `set` entry; the caller raises `modified`.
    fn write_set(
        &mut self,
        machine: &MachineTie,
        kind: SetKind,
        set: Set,
        stamp: Stamp<'_>,
    ) -> Result<(), RecordRefusal> {
        if set.numbers.interval.is_some() != (kind == SetKind::Engrave) {
            return Err(RecordRefusal::WrongNumbers);
        }
        if self.set_for(&machine.id, kind) == Some(&set) {
            return Err(RecordRefusal::Unchanged);
        }
        self.room_for_entry()?;
        let index = match self.lasers.iter().position(|l| l.machine.id == machine.id) {
            Some(index) => index,
            None if self.lasers.len() >= MAX_LASERS => return Err(RecordRefusal::TooManyLasers),
            None => {
                self.lasers.push(LaserSets {
                    machine: machine.clone(),
                    sets: std::collections::BTreeMap::new(),
                });
                self.lasers.len() - 1
            }
        };
        let event = HistoryEvent::Set {
            numbers: set.numbers,
            source: set.source.clone(),
        };
        self.lasers[index].sets.insert(kind, set);
        self.append(stamp, &machine.id, kind, event);
        Ok(())
    }

    /// A typed set (criteria 19 to 21): added or edited, its source Typed (a hand edit of a
    /// tested set too, G13).
    ///
    /// # Errors
    /// Numbers that do not fit the kind, no change, 16 lasers or 500 entries.
    pub fn type_set(
        &mut self,
        machine: &MachineTie,
        kind: SetKind,
        numbers: SetNumbers,
        stamp: Stamp<'_>,
    ) -> Result<(), RecordRefusal> {
        let set = Set {
            numbers,
            source: SetSource::Typed,
        };
        self.write_set(machine, kind, set, stamp)?;
        self.touch(stamp.now);
        Ok(())
    }

    /// Removes the laser's set of `kind` and appends a `removed` entry; the laser stays listed.
    ///
    /// # Errors
    /// No such set, or 500 entries.
    pub fn delete_set(
        &mut self,
        machine_id: &str,
        kind: SetKind,
        stamp: Stamp<'_>,
    ) -> Result<(), RecordRefusal> {
        if self.set_for(machine_id, kind).is_none() {
            return Err(RecordRefusal::NoSet { kind: kind.label() });
        }
        self.room_for_entry()?;
        for laser in &mut self.lasers {
            if laser.machine.id == machine_id {
                laser.sets.remove(&kind);
            }
        }
        self.append(stamp, machine_id, kind, HistoryEvent::Removed);
        self.touch(stamp.now);
        Ok(())
    }

    /// Renames the material.
    ///
    /// # Errors
    /// A name outside 1 to 60 characters, or no change.
    pub fn rename(&mut self, name: &str, now: u64) -> Result<(), RecordRefusal> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > MAX_MATERIAL_NAME_CHARS {
            return Err(RecordRefusal::InvalidName);
        }
        if self.name == name {
            return Err(RecordRefusal::Unchanged);
        }
        name.clone_into(&mut self.name);
        self.touch(now);
        Ok(())
    }

    /// Changes the thickness.
    ///
    /// # Errors
    /// No change.
    pub fn set_thickness(&mut self, thickness: Thickness, now: u64) -> Result<(), RecordRefusal> {
        if self.thickness == thickness {
            return Err(RecordRefusal::Unchanged);
        }
        self.thickness = thickness;
        self.touch(now);
        Ok(())
    }

    /// Writes the test of a grid (ADR 0058 §3): a new test at the pick, or the recipe and
    /// machine of a test that is not Lasered rewritten; best cell, note and photo are kept, a
    /// best cell outside the new recipe is dropped.
    ///
    /// # Errors
    /// 200 tests, a Lasered test, or no change.
    pub fn put_test(&mut self, test: TestRecord, now: u64) -> Result<(), RecordRefusal> {
        match self.tests.iter().position(|t| t.id == test.id) {
            Some(index) => {
                let old = &mut self.tests[index];
                if old.lasered.is_some() {
                    return Err(RecordRefusal::Lasered);
                }
                if old.recipe == test.recipe && old.machine == test.machine {
                    return Err(RecordRefusal::Unchanged);
                }
                old.recipe = test.recipe;
                old.machine = test.machine;
                old.best = old
                    .best
                    .filter(|cell| old.recipe.has_cell(cell.column, cell.row));
            }
            None if self.tests.len() >= MAX_TESTS => return Err(RecordRefusal::TooManyTests),
            None => self.tests.push(test),
        }
        self.touch(now);
        Ok(())
    }

    /// Takes the test out of the record without retiring its id: a pick of another material
    /// moves it (ADR 0058 §3).
    ///
    /// # Errors
    /// No such test.
    pub fn take_test(&mut self, id: TestId, now: u64) -> Result<TestRecord, RecordRefusal> {
        let index = self.test_index(id)?;
        self.touch(now);
        Ok(self.tests.remove(index))
    }

    /// Deletes the test (criterion 25); the caller retires the returned test's id and removes
    /// its photo.
    ///
    /// # Errors
    /// No such test.
    pub fn delete_test(&mut self, id: TestId, now: u64) -> Result<TestRecord, RecordRefusal> {
        self.take_test(id, now)
    }

    fn test_index(&self, id: TestId) -> Result<usize, RecordRefusal> {
        self.tests
            .iter()
            .position(|t| t.id == id)
            .ok_or(RecordRefusal::UnknownTest)
    }

    /// Judging (criterion 24): `cell` becomes the test's best cell, and its numbers (the two
    /// axis values over the shared values) the set of the recipe's kind for the test's laser,
    /// source the test, with a `set` entry. A tap on the best cell changes nothing (G14).
    ///
    /// # Errors
    /// No such test or cell, no change, 16 lasers or 500 entries.
    pub fn judge(
        &mut self,
        id: TestId,
        cell: CellIndex,
        stamp: Stamp<'_>,
    ) -> Result<(), RecordRefusal> {
        let test = &self.tests[self.test_index(id)?];
        if !test.recipe.has_cell(cell.column, cell.row) {
            return Err(RecordRefusal::NoCell);
        }
        if test.best == Some(cell) {
            return Err(RecordRefusal::Unchanged);
        }
        let (machine, kind) = (test.machine.clone(), test.recipe.kind());
        let set = Set {
            numbers: test.recipe.cell_values(cell.column, cell.row),
            source: SetSource::Test(id),
        };
        match self.write_set(&machine, kind, set, stamp) {
            Ok(()) | Err(RecordRefusal::Unchanged) => {}
            Err(other) => return Err(other),
        }
        let index = self.test_index(id)?;
        self.tests[index].best = Some(cell);
        self.touch(stamp.now);
        Ok(())
    }

    /// Sets the test's note (criterion 26).
    ///
    /// # Errors
    /// No such test, a note over 500 characters, or no change.
    pub fn set_note(&mut self, id: TestId, note: &str, now: u64) -> Result<(), RecordRefusal> {
        if note.chars().count() > MAX_NOTE_CHARS {
            return Err(RecordRefusal::LongNote);
        }
        let index = self.test_index(id)?;
        if self.tests[index].note == note {
            return Err(RecordRefusal::Unchanged);
        }
        note.clone_into(&mut self.tests[index].note);
        self.touch(now);
        Ok(())
    }

    /// Sets or clears the kind of the test's photo (criterion 27); the bytes go to the host.
    ///
    /// # Errors
    /// No such test, or no change.
    pub fn set_photo(
        &mut self,
        id: TestId,
        photo: Option<PhotoKind>,
        now: u64,
    ) -> Result<(), RecordRefusal> {
        let index = self.test_index(id)?;
        if self.tests[index].photo == photo {
            return Err(RecordRefusal::Unchanged);
        }
        self.tests[index].photo = photo;
        self.touch(now);
        Ok(())
    }
}

#[cfg(test)]
#[path = "material_record_edits_tests.rs"]
mod tests;
