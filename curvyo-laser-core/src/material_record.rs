//! The material record of format 2 in memory: one material for all lasers with sets per laser,
//! tests and history (ADR 0058 §2, `specs/0112` criteria 18 to 26).

use std::collections::BTreeMap;

use crate::material::{CellIndex, MachineTie, SetKind, SetNumbers, Thickness};
use crate::test_id::TestId;
use crate::test_recipe::TestRecipe;

/// The most lasers one material holds sets for.
pub const MAX_LASERS: usize = 16;
/// The most tests one material holds.
pub const MAX_TESTS: usize = 200;
/// The longest note of a test, in characters (criterion 26).
pub const MAX_NOTE_CHARS: usize = 500;

/// Where a set's numbers came from (criterion 19).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetSource {
    /// The best cell of this record's test with the id ("Test `<id>`").
    Test(TestId),
    /// Typed in the material's fields ("Typed").
    Typed,
    /// A `tested` history entry of a format-1 record ("Test of `<date>`").
    History(String),
}

/// One kind's set of one laser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Set {
    /// Power, speed, passes and, for Engrave, the interval.
    pub numbers: SetNumbers,
    /// Their source.
    pub source: SetSource,
}

/// The sets of one laser in a material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaserSets {
    /// The laser's profile; its name is for display only.
    pub machine: MachineTie,
    /// At most one set per kind.
    pub sets: BTreeMap<SetKind, Set>,
}

/// The image kind of a test's photo, also its file's extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhotoKind {
    /// A JPEG file.
    Jpg,
    /// A PNG file.
    Png,
}

impl PhotoKind {
    /// The stored text and file extension.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Jpg => "jpg",
            Self::Png => "png",
        }
    }

    /// The kind a stored text names.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::Jpg, Self::Png]
            .into_iter()
            .find(|kind| kind.as_str() == text)
    }
}

/// One test of a material (criterion 22).
#[derive(Debug, Clone, PartialEq)]
pub struct TestRecord {
    /// Its id, burned into the sheet.
    pub id: TestId,
    /// When it was made, in Unix milliseconds.
    pub created: u64,
    /// The laser of the grid's page.
    pub machine: MachineTie,
    /// What the grid varies.
    pub recipe: TestRecipe,
    /// When it was lasered, `None` for Not lasered.
    pub lasered: Option<u64>,
    /// The best cell, inside the recipe's counts.
    pub best: Option<CellIndex>,
    /// At most 500 characters, empty for none.
    pub note: String,
    /// The photo next to the record.
    pub photo: Option<PhotoKind>,
}

/// What one history entry records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryEvent {
    /// A set written: by a typed edit or a judged test.
    Set {
        /// The numbers.
        numbers: SetNumbers,
        /// Their source.
        source: SetSource,
    },
    /// A set removed.
    Removed,
}

/// One entry of the append-only history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    /// A lowercase UUID v4.
    pub id: String,
    /// Unix milliseconds, passed in by the caller.
    pub date: u64,
    /// The laser's profile id.
    pub machine: String,
    /// The set's kind.
    pub kind: SetKind,
    /// What happened.
    pub event: HistoryEvent,
}

/// One material of the maker's library, shared by all lasers.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// A lowercase UUID v4, also its file's name.
    pub id: String,
    /// The last change in Unix milliseconds; it rises with every write.
    pub modified: u64,
    /// 1 to 60 characters, not unique.
    pub name: String,
    /// The thickness.
    pub thickness: Thickness,
    /// The lasers with sets, in the order their first set was added.
    pub lasers: Vec<LaserSets>,
    /// The tests, newest last.
    pub tests: Vec<TestRecord>,
    /// The history, oldest first.
    pub history: Vec<HistoryEntry>,
}

impl Material {
    /// The sets of the laser `profile_id`.
    #[must_use]
    pub fn laser(&self, profile_id: &str) -> Option<&LaserSets> {
        self.lasers.iter().find(|l| l.machine.id == profile_id)
    }

    /// The set of `kind` for the laser `profile_id` (ADR 0058 §7: what a linked operation on a
    /// page of that laser runs).
    #[must_use]
    pub fn set_for(&self, profile_id: &str, kind: SetKind) -> Option<&Set> {
        self.laser(profile_id)?.sets.get(&kind)
    }

    /// The test with the id.
    #[must_use]
    pub fn test(&self, id: TestId) -> Option<&TestRecord> {
        self.tests.iter().find(|t| t.id == id)
    }

    /// The number of sets over all lasers (`<n> lasers · <n> sets · <n> tests`).
    #[must_use]
    pub fn set_count(&self) -> usize {
        self.lasers.iter().map(|l| l.sets.len()).sum()
    }
}

/// The state line of a linked operation whose material has no set of `kind` for the page's
/// laser (criterion 21).
#[must_use]
pub fn no_laser_set_text(kind: SetKind) -> String {
    format!(
        "Material has no {} set for this laser, using stored numbers",
        kind.label()
    )
}

/// The ids of every test of the loaded records: with the retired ids and the open project's grid
/// ids, the taken set a new id avoids (ADR 0058 §4).
pub fn live_test_ids(reads: &[MaterialRead]) -> impl Iterator<Item = TestId> + '_ {
    reads
        .iter()
        .flat_map(|read| match read {
            MaterialRead::Loaded(material) => material.tests.as_slice(),
            _ => &[],
        })
        .map(|test| test.id)
}

/// The photo file name of a test: `<material id>.<test id>.jpg|png` (ADR 0058 §1).
#[must_use]
pub fn photo_file_name(material: &str, test: TestId, kind: PhotoKind) -> String {
    format!("{material}.{test}.{}", kind.as_str())
}

/// One record of the library as loaded: readable, damaged, or of a format this build does not
/// read (ADR 0058 §6).
#[derive(Debug, Clone, PartialEq)]
pub enum MaterialRead {
    /// A record this build reads.
    Loaded(Material),
    /// A damaged record, never used and never rewritten.
    Damaged {
        /// The id its file name carries.
        id: String,
        /// The reason shown in the Materials tab.
        reason: String,
    },
    /// A record of a newer format, left untouched.
    Newer {
        /// The id its file name carries.
        id: String,
    },
    /// A record of format 1, skipped until its conversion is built (plan Part F, Frage 214).
    Older {
        /// The id its file name carries.
        id: String,
    },
}

impl MaterialRead {
    /// The record's id.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Loaded(material) => &material.id,
            Self::Damaged { id, .. } | Self::Newer { id } | Self::Older { id } => id,
        }
    }
}
