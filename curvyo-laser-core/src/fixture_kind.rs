//! The kinds of laser fixtures and the limits of their parameters (`specs/0102-laser-fixtures`
//! criteria 2, 15, 19, 20, 22, 23 and 32; ADR 0049 §1 and §2).

use curvyo_document_core::Length;

/// The prefix of a fixture's stored kind (`curvyo.laser/<kind>`, ADR 0049 §1).
pub const KIND_PREFIX: &str = "curvyo.laser/";

/// A fixture kind this build knows, or [`FixtureKind::Unknown`] for any other stored kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureKind {
    /// The engraved bed grid with major lines and tapes.
    EngravedGrid,
    /// A board with slots.
    Holder,
    /// A keep-out rectangle, hatched.
    Clamp,
    /// An L-shaped fence.
    Stop,
    /// A kind this build does not know: kept with its values, drawn never.
    Unknown,
}

impl FixtureKind {
    /// Reads a stored kind; every value that is not one of the four known kinds is unknown.
    #[must_use]
    pub fn from_stored(stored: &str) -> Self {
        match stored.strip_prefix(KIND_PREFIX) {
            Some("engraved_grid") => Self::EngravedGrid,
            Some("holder") => Self::Holder,
            Some("clamp") => Self::Clamp,
            Some("stop") => Self::Stop,
            _ => Self::Unknown,
        }
    }

    /// The stored kind text of a known kind; `None` for [`FixtureKind::Unknown`], whose stored
    /// text the document keeps as it was read.
    #[must_use]
    pub const fn stored(self) -> Option<&'static str> {
        match self {
            Self::EngravedGrid => Some("curvyo.laser/engraved_grid"),
            Self::Holder => Some("curvyo.laser/holder"),
            Self::Clamp => Some("curvyo.laser/clamp"),
            Self::Stop => Some("curvyo.laser/stop"),
            Self::Unknown => None,
        }
    }

    /// The word the maker reads for the kind: the tag of a selection and the row's word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::EngravedGrid => "Grid",
            Self::Holder => "Holder",
            Self::Clamp => "Clamp",
            Self::Stop => "Stop",
            Self::Unknown => "Unknown fixture",
        }
    }
}

/// The shape of a holder's slot (Frage 172 A: a rectangle or a circle).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotShape {
    /// A rectangle of a width and a height.
    Rectangle,
    /// A circle of a diameter.
    Circle,
}

/// The limits and the default of one length parameter, in millimetres (Proposal values of the
/// spec; a value outside the limits is set to the nearest one, never refused).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MmRange {
    /// The smallest allowed value.
    pub min: Length,
    /// The largest allowed value.
    pub max: Length,
    /// The value a new fixture or a damaged field reads as.
    pub default: Length,
}

impl MmRange {
    /// A range from its limits and its default, all in millimetres.
    #[must_use]
    pub fn new(min_mm: f64, max_mm: f64, default_mm: f64) -> Self {
        Self {
            min: Length::from_mm(min_mm),
            max: Length::from_mm(max_mm),
            default: Length::from_mm(default_mm),
        }
    }

    /// The value inside the range nearest to `value`, and whether it had to move. A value that is
    /// not finite reads as the default and counts as moved.
    #[must_use]
    pub fn clamp(self, value: Length) -> (Length, bool) {
        let mm = value.as_mm();
        if !mm.is_finite() {
            return (self.default, true);
        }
        if mm < self.min.as_mm() {
            (self.min, true)
        } else if mm > self.max.as_mm() {
            (self.max, true)
        } else {
            (value, false)
        }
    }
}

/// The spacing of the engraved grid: 1 to 100 mm, 10 by default (criterion 15).
#[must_use]
pub fn grid_spacing_range() -> MmRange {
    MmRange::new(1.0, 100.0, 10.0)
}

/// The grid's origin offset X or Y: a bed-sized offset, 0 by default (criterion 15).
#[must_use]
pub fn origin_offset_range() -> MmRange {
    MmRange::new(-10_000.0, 10_000.0, 0.0)
}

/// The number of lines between two major lines: 2 to 20, 5 by default (criterion 15).
pub const MAJOR_EVERY_RANGE: (u32, u32, u32) = (2, 20, 5);

/// A holder's width or height, a clamp's width or height: 1 to 1,000 mm, with the kind's default.
#[must_use]
pub fn board_side_range(default_mm: f64) -> MmRange {
    MmRange::new(1.0, 1000.0, default_mm)
}

/// A stop's leg: 1 to 1,000 mm, 60 mm by default (criterion 23).
#[must_use]
pub fn stop_leg_range() -> MmRange {
    MmRange::new(1.0, 1000.0, 60.0)
}

/// A stop's thickness: 0.5 to 50 mm, 5 mm by default (criterion 23).
#[must_use]
pub fn stop_thickness_range() -> MmRange {
    MmRange::new(0.5, 50.0, 5.0)
}

/// A slot's width, height or diameter: 0.5 to 1,000 mm, 20 mm by default (criteria 19 and 20).
#[must_use]
pub fn slot_size_range() -> MmRange {
    MmRange::new(0.5, 1000.0, 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_stored_kinds_read_and_anything_else_is_unknown() {
        assert_eq!(
            FixtureKind::from_stored("curvyo.laser/engraved_grid"),
            FixtureKind::EngravedGrid
        );
        assert_eq!(
            FixtureKind::from_stored("curvyo.laser/holder"),
            FixtureKind::Holder
        );
        assert_eq!(
            FixtureKind::from_stored("curvyo.laser/clamp"),
            FixtureKind::Clamp
        );
        assert_eq!(
            FixtureKind::from_stored("curvyo.laser/stop"),
            FixtureKind::Stop
        );
        assert_eq!(
            FixtureKind::from_stored("curvyo.laser/turntable"),
            FixtureKind::Unknown
        );
        assert_eq!(FixtureKind::from_stored("holder"), FixtureKind::Unknown);
    }

    #[test]
    fn a_known_kind_stores_its_text_and_an_unknown_one_has_none() {
        assert_eq!(
            FixtureKind::Stop.stored(),
            Some("curvyo.laser/stop"),
            "round trip of a known kind"
        );
        assert_eq!(FixtureKind::Unknown.stored(), None);
        assert_eq!(FixtureKind::Unknown.word(), "Unknown fixture");
        assert_eq!(FixtureKind::Clamp.word(), "Clamp");
    }

    #[test]
    fn a_value_outside_its_range_moves_to_the_nearest_limit_and_is_reported() {
        let spacing = grid_spacing_range();
        assert_eq!(
            spacing.clamp(Length::from_mm(0.5)),
            (Length::from_mm(1.0), true)
        );
        assert_eq!(
            spacing.clamp(Length::from_mm(250.0)),
            (Length::from_mm(100.0), true)
        );
        assert_eq!(
            spacing.clamp(Length::from_mm(10.0)),
            (Length::from_mm(10.0), false)
        );
    }

    #[test]
    fn a_value_that_is_not_finite_reads_as_the_default() {
        assert_eq!(
            board_side_range(60.0).clamp(Length::from_mm(f64::NAN)),
            (Length::from_mm(60.0), true)
        );
    }

    #[test]
    fn the_defaults_of_criterion_two_and_nineteen_are_the_proposal_sizes() {
        assert_eq!(board_side_range(100.0).default, Length::from_mm(100.0));
        assert_eq!(board_side_range(40.0).default, Length::from_mm(40.0));
        assert_eq!(slot_size_range().default, Length::from_mm(20.0));
        assert_eq!(stop_thickness_range().default, Length::from_mm(5.0));
        assert_eq!(MAJOR_EVERY_RANGE.2, 5);
    }
}
