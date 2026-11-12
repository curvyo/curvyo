//! The material record with its sets, history and limits (ADR 0055 §2, `specs/0028` criteria
//! 15 to 18), in memory; its file format comes with the library (Part B of the plan).

use std::collections::BTreeMap;
use std::fmt;

use crate::machine_profile::FeedRate;
use crate::test_grid::GridRange;

/// The longest material name, in characters.
pub const MAX_MATERIAL_NAME_CHARS: usize = 60;
/// The most history entries one record holds (spec criterion 29).
pub const MAX_HISTORY_ENTRIES: usize = 500;

/// A power in percent of the machine's maximum power, 1 to 100 with at most one decimal, kept in
/// tenths so equality is exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PowerPercent(u16);

impl PowerPercent {
    /// The smallest power, in tenths of a percent.
    pub const MIN_TENTHS: u16 = 10;
    /// The largest power, in tenths of a percent.
    pub const MAX_TENTHS: u16 = 1000;

    /// The power of `tenths` tenths of a percent, `None` outside 1 to 100 %.
    #[must_use]
    pub fn from_tenths(tenths: u16) -> Option<Self> {
        (Self::MIN_TENTHS..=Self::MAX_TENTHS)
            .contains(&tenths)
            .then_some(Self(tenths))
    }

    /// The exact power `percent`: `None` when it is not finite, has more than one decimal or lies
    /// outside 1 to 100 (a stored value is never rounded, ADR 0054 §1).
    #[must_use]
    pub fn from_percent(percent: f64) -> Option<Self> {
        let tenths = percent * 10.0;
        let whole = tenths.round();
        if !tenths.is_finite() || (tenths - whole).abs() > 1e-6 {
            return None;
        }
        Self::rounded(percent)
    }

    /// `percent` rounded to 0.1 % (a grid value, spec S3), `None` outside 1 to 100 after rounding.
    #[must_use]
    pub fn rounded(percent: f64) -> Option<Self> {
        let tenths = (percent * 10.0).round();
        if !tenths.is_finite() || tenths < 0.0 || tenths > f64::from(Self::MAX_TENTHS) {
            return None;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        // checked above: a whole number within 0 to 1000
        Self::from_tenths(tenths as u16)
    }

    /// Tenths of a percent.
    #[must_use]
    pub const fn tenths(self) -> u16 {
        self.0
    }

    /// The percent as a number (`60.5`).
    #[must_use]
    pub fn percent(self) -> f64 {
        f64::from(self.0) / 10.0
    }
}

impl fmt::Display for PowerPercent {
    /// "60" or "60.5", the way a grid label and the settings line write it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (whole, tenth) = (self.0 / 10, self.0 % 10);
        if tenth == 0 {
            write!(f, "{whole}")
        } else {
            write!(f, "{whole}.{tenth}")
        }
    }
}

/// How many times a set runs, 1 to 20.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Passes(u8);

impl Passes {
    /// The fewest passes.
    pub const MIN: u8 = 1;
    /// The most passes.
    pub const MAX: u8 = 20;
    /// One pass, the value of an empty field (UX 8.3).
    pub const ONE: Self = Self(1);

    /// `n` passes, `None` outside 1 to 20.
    #[must_use]
    pub fn new(n: u64) -> Option<Self> {
        u8::try_from(n)
            .ok()
            .filter(|n| (Self::MIN..=Self::MAX).contains(n))
            .map(Self)
    }

    /// The plain number.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// A material thickness, 0.1 to 100 mm with one decimal, kept in tenths of a millimetre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Thickness(u16);

impl Thickness {
    /// The thinnest, in tenths of a millimetre.
    pub const MIN_TENTHS: u16 = 1;
    /// The thickest, in tenths of a millimetre.
    pub const MAX_TENTHS: u16 = 1000;

    /// The exact thickness `mm`: `None` when not finite, with more than one decimal or outside
    /// 0.1 to 100 mm.
    #[must_use]
    pub fn from_mm(mm: f64) -> Option<Self> {
        let tenths = mm * 10.0;
        let whole = tenths.round();
        if !tenths.is_finite() || (tenths - whole).abs() > 1e-6 || whole < 0.0 {
            return None;
        }
        if whole > f64::from(Self::MAX_TENTHS) {
            return None;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        // checked above: a whole number within 0 to 1000
        let tenths = whole as u16;
        (Self::MIN_TENTHS..=Self::MAX_TENTHS)
            .contains(&tenths)
            .then_some(Self(tenths))
    }

    /// The thickness in mm.
    #[must_use]
    pub fn mm(self) -> f64 {
        f64::from(self.0) / 10.0
    }
}

/// The three kinds a set belongs to; also the kind of a test grid (spec "Words").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SetKind {
    /// Cut, for the Cut and Cut-out operations.
    Cut,
    /// Score.
    Score,
    /// Engrave, for Engrave and the hatch of Engrave + Outline.
    Engrave,
}

impl SetKind {
    /// The three kinds in the order of the UI (Cut, Score, Engrave).
    pub const ALL: [Self; 3] = [Self::Cut, Self::Score, Self::Engrave];

    /// The stored text (a published file key).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cut => "cut",
            Self::Score => "score",
            Self::Engrave => "engrave",
        }
    }

    /// The kind a stored text names.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == text)
    }

    /// The word shown in the UI ("Cut").
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cut => "Cut",
            Self::Score => "Score",
            Self::Engrave => "Engrave",
        }
    }
}

/// Power, speed and passes of one kind (spec "Words": Settings).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Settings {
    /// Percent of the machine's maximum power.
    pub power: PowerPercent,
    /// Speed in mm/min, at least 1; a speed above the machine's maximum feed is not damage.
    pub speed: FeedRate,
    /// Passes.
    pub passes: Passes,
}

/// Where a set's numbers came from (ADR 0055 §2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetSource {
    /// The `tested` history entry with this id.
    Test(String),
    /// Typed in the material's fields.
    Edited,
}

/// One kind's set of a material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialSet {
    /// The numbers.
    pub settings: Settings,
    /// Their source.
    pub source: SetSource,
}

/// The cell of a grid, by column (power) and row (speed), from the top left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellIndex {
    /// The column, from 0.
    pub column: u8,
    /// The row, from 0.
    pub row: u8,
}

/// What one history entry records.
#[derive(Debug, Clone, PartialEq)]
pub enum HistoryEvent {
    /// A test saved into the record.
    Tested {
        /// The grid's fields (without kind and labels).
        grid: GridRange,
        /// The marked cell.
        cell: CellIndex,
        /// The numbers of the set it made.
        set: Settings,
    },
    /// A set typed in the material's fields.
    Edited {
        /// The new numbers.
        set: Settings,
    },
    /// A set removed.
    Removed,
}

/// One entry of the append-only test history (spec criterion 18).
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    /// A lowercase UUID v4.
    pub id: String,
    /// Unix milliseconds, passed in by the caller.
    pub date: u64,
    /// The set's kind.
    pub kind: SetKind,
    /// What happened.
    pub event: HistoryEvent,
}

/// The machine a record was tested on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineTie {
    /// The profile id (a UUID or `generic-laser`).
    pub id: String,
    /// The profile's name when the record was made, for display only.
    pub name: String,
}

/// One material of the maker's library (spec criterion 16).
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// A lowercase UUID v4, also its file's name.
    pub id: String,
    /// The last change in Unix milliseconds; it rises with every write ([`next_modified`]), so an
    /// operation's link tells a newer record from an older one (ADR 0054 §1, ADR 0055 §2).
    pub modified: u64,
    /// 1 to 60 characters, not unique.
    pub name: String,
    /// The thickness.
    pub thickness: Thickness,
    /// The machine it is for.
    pub machine: MachineTie,
    /// At most one set per kind.
    pub sets: BTreeMap<SetKind, MaterialSet>,
    /// The test history, oldest first.
    pub history: Vec<HistoryEntry>,
}

impl Material {
    /// The set of `kind`, if the record has one.
    #[must_use]
    pub fn set(&self, kind: SetKind) -> Option<&Settings> {
        self.sets.get(&kind).map(|set| &set.settings)
    }
}

/// The `modified` of a record written at `now` whose last change was `previous`:
/// `max(now, previous + 1)`, also for an undo or redo (ADR 0055 §2), so it always rises.
#[must_use]
pub const fn next_modified(previous: u64, now: u64) -> u64 {
    let after = previous.saturating_add(1);
    if now > after { now } else { after }
}

/// One record of the library as loaded: readable, damaged, or of a newer format (ADR 0055 §3).
#[derive(Debug, Clone, PartialEq)]
pub enum MaterialRead {
    /// A record this build reads.
    Loaded(Material),
    /// A damaged record, never used and never rewritten.
    Damaged {
        /// The id its file name carries.
        id: String,
        /// The reason shown in the Materials section.
        reason: String,
    },
    /// A record of a newer `format_version`, left untouched.
    Newer {
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
            Self::Damaged { id, .. } | Self::Newer { id } => id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_keeps_one_decimal_exactly_and_refuses_more_or_out_of_range() {
        assert_eq!(PowerPercent::from_percent(60.0).unwrap().tenths(), 600);
        assert_eq!(
            PowerPercent::from_percent(60.5).unwrap().to_string(),
            "60.5"
        );
        assert_eq!(PowerPercent::from_percent(1.0).unwrap().to_string(), "1");
        assert_eq!(PowerPercent::from_percent(100.0).unwrap().tenths(), 1000);
        for bad in [0.5, 0.9, 100.5, 100.1, 60.25, f64::NAN, f64::INFINITY, -3.0] {
            assert_eq!(PowerPercent::from_percent(bad), None, "{bad}");
        }
        assert_eq!(PowerPercent::rounded(60.25).unwrap().tenths(), 603);
        assert_eq!(PowerPercent::rounded(0.94), None, "rounds to 0.9 %");
        assert_eq!(PowerPercent::rounded(100.04).unwrap().tenths(), 1000);
    }

    #[test]
    fn passes_and_thickness_keep_their_ranges() {
        assert_eq!(Passes::new(1), Some(Passes::ONE));
        assert_eq!(Passes::new(20).unwrap().get(), 20);
        assert_eq!(Passes::new(0), None);
        assert_eq!(Passes::new(21), None);
        assert_eq!(Thickness::from_mm(3.0).unwrap().mm(), 3.0);
        assert_eq!(Thickness::from_mm(0.1).unwrap().mm(), 0.1);
        assert_eq!(Thickness::from_mm(100.0).unwrap().mm(), 100.0);
        for bad in [0.0, 100.1, 2.55, f64::NAN, -1.0] {
            assert_eq!(Thickness::from_mm(bad), None, "{bad}");
        }
    }

    #[test]
    fn modified_always_rises_also_when_the_clock_is_behind() {
        assert_eq!(next_modified(100, 5_000), 5_000);
        assert_eq!(next_modified(5_000, 5_000), 5_001, "same millisecond");
        assert_eq!(
            next_modified(9_000, 5_000),
            9_001,
            "a clock behind the record"
        );
        assert_eq!(next_modified(u64::MAX, 1), u64::MAX);
    }

    #[test]
    fn set_kinds_read_their_texts_only() {
        for kind in SetKind::ALL {
            assert_eq!(SetKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(SetKind::parse("cut_out"), None);
        assert_eq!(SetKind::Engrave.label(), "Engrave");
    }
}
