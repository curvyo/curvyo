//! The state of the Pen's typed Length and Angle entry, from the key that opens it until Enter,
//! Escape or a press (`specs/0072-pen-angle-snap-and-typed-values/`, `adrs.md` decision 4).
//!
//! Ephemeral (ADR 0009 §2): nothing here touches the document. The session passes the end of
//! the live segment (the pointer, constrained while Ctrl is held) into every call; the entry
//! shows its values until the first edit, then keeps them, and resolves the typed and kept
//! values into the point of the next node.

use curvyo_document_core::{Angle, DisplayUnit, Length, Point, Tolerance};
use curvyo_geometry_core::MAX_COORDINATE_MM;

use crate::conversion::GEOMETRIC_TOLERANCE_MM;
use crate::display_unit_text::{decimal_text, format_status_length, parse_length};
use crate::transform_entry::parse_entry_number;

/// A length at or below this places nothing (criterion 26).
const LENGTH_TOLERANCE: Tolerance = Tolerance::from_mm(GEOMETRIC_TOLERANCE_MM);

/// One field of the Pen entry chip, in Tab order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PenEntryField {
    /// The length of the next segment, in the display unit.
    Length,
    /// The absolute angle of the next segment, degrees, clockwise on screen.
    Angle,
}

impl PenEntryField {
    /// The fields in Tab order (criterion 13).
    pub const ALL: [Self; 2] = [Self::Length, Self::Angle];

    const fn index(self) -> usize {
        match self {
            Self::Length => 0,
            Self::Angle => 1,
        }
    }
}

/// A segment from L as a length and an absolute angle.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Polar {
    length: Length,
    angle: Angle,
}

impl Polar {
    /// The segment from `last` to `end`; the angle in (-180°, 180°].
    fn of(last: Point, end: Point) -> Self {
        let offset = last.vector_to(end);
        Self {
            length: Length::from_mm(offset.length()),
            angle: Angle::from_radians(offset.y.atan2(offset.x)),
        }
    }
}

/// Why a typed value was not used as typed, said in the canvas notice slot (criteria 24, 25, 28).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PenEntryNotice {
    /// The Length text is not a number: this frozen length was used.
    LengthNotANumber(Length),
    /// The Angle text is not a number: this frozen angle was used.
    AngleNotANumber(Angle),
    /// A negative length was drawn in the opposite direction.
    NegativeLength,
    /// The point was beyond the coordinate limit: the length was shortened to reach it.
    TooFar,
}

/// The one case with no placeable node: the length is at or below the tolerance (criterion 26).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LengthNotPositive;

/// What Enter or a frozen press places.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PenEntryPlaced {
    /// The new node's point.
    pub point: Point,
    /// The segment's length as placed.
    pub length: Length,
    /// The segment's angle as placed, in (-180°, 180°].
    pub angle: Angle,
    /// What the notice slot says, if anything.
    pub notice: Option<PenEntryNotice>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    /// No field edited: the live segment decides (criterion 15).
    Following,
    /// A field was edited: the pointer and Ctrl change nothing (criterion 16).
    Frozen { pointer: Point, kept: Polar },
}

/// One open Pen entry chip.
#[derive(Debug, Clone, PartialEq)]
pub struct PenEntry {
    last: Point,
    phase: Phase,
    edited: [Option<String>; 2],
}

impl PenEntry {
    /// An entry for the segment that starts at the last node `last` (criterion 12).
    #[must_use]
    pub const fn new(last: Point) -> Self {
        Self {
            last,
            phase: Phase::Following,
            edited: [None, None],
        }
    }

    /// The last node L.
    #[must_use]
    pub const fn last(&self) -> Point {
        self.last
    }

    /// Whether a field was edited (criterion 16).
    #[must_use]
    pub const fn is_frozen(&self) -> bool {
        matches!(self.phase, Phase::Frozen { .. })
    }

    /// The pointer at the freeze, where the chip stays; `None` while following.
    #[must_use]
    pub const fn frozen_pointer(&self) -> Option<Point> {
        match self.phase {
            Phase::Frozen { pointer, .. } => Some(pointer),
            Phase::Following => None,
        }
    }

    /// The maker edited `field` to `text`. The first edit freezes the entry with the live
    /// segment ending at `live_end` and the chip at `pointer` (criterion 16).
    pub fn edit(&mut self, field: PenEntryField, text: &str, live_end: Point, pointer: Point) {
        if self.phase == Phase::Following {
            self.phase = Phase::Frozen {
                pointer,
                kept: Polar::of(self.last, live_end),
            };
        }
        self.edited[field.index()] = Some(text.to_owned());
    }

    /// The text `field` shows: the maker's once edited, else the kept or live value with the
    /// readout's decimals (criteria 15, 16).
    #[must_use]
    pub fn field_text(&self, field: PenEntryField, live_end: Point, unit: DisplayUnit) -> String {
        if let Some(text) = &self.edited[field.index()] {
            return text.clone();
        }
        let shown = self.kept_or_live(live_end);
        match field {
            PenEntryField::Length => format_status_length(shown.length, unit),
            PenEntryField::Angle => decimal_text(shown.angle.as_radians().to_degrees(), 1),
        }
    }

    /// The node Enter or a frozen press places, with the permit rules of criteria 24 to 28.
    /// Unfrozen, the live segment's end (criterion 19).
    ///
    /// # Errors
    /// [`LengthNotPositive`] when the length is at or below the tolerance (criterion 26).
    pub fn resolve(
        &self,
        live_end: Point,
        unit: DisplayUnit,
    ) -> Result<PenEntryPlaced, LengthNotPositive> {
        let Phase::Frozen { kept, .. } = self.phase else {
            let live = Polar::of(self.last, live_end);
            if live.length.as_mm() <= LENGTH_TOLERANCE.as_mm() {
                return Err(LengthNotPositive);
            }
            return Ok(PenEntryPlaced {
                point: live_end,
                length: live.length,
                angle: live.angle,
                notice: None,
            });
        };
        let mut notice = None;
        let mut length_mm = match &self.edited[0] {
            Some(text) => match parse_length(text, unit).map(Length::as_mm) {
                Some(mm) if mm.is_finite() => mm,
                _ => {
                    notice = Some(PenEntryNotice::LengthNotANumber(kept.length));
                    kept.length.as_mm()
                }
            },
            None => kept.length.as_mm(),
        };
        let mut degrees = match &self.edited[1] {
            Some(text) => match parse_entry_number(text, true) {
                Some(degrees) if degrees.is_finite() => degrees,
                _ => {
                    notice = notice.or(Some(PenEntryNotice::AngleNotANumber(kept.angle)));
                    kept.angle.as_radians().to_degrees()
                }
            },
            None => kept.angle.as_radians().to_degrees(),
        };
        if length_mm < 0.0 {
            length_mm = -length_mm;
            degrees += 180.0;
            notice = notice.or(Some(PenEntryNotice::NegativeLength));
        }
        if length_mm <= LENGTH_TOLERANCE.as_mm() {
            return Err(LengthNotPositive);
        }
        let degrees = half_turn_range(degrees);
        let (sin, cos) = degrees.to_radians().sin_cos();
        let reach = reach_inside_limit(self.last, cos, sin);
        if length_mm > reach {
            length_mm = reach;
            notice = notice.or(Some(PenEntryNotice::TooFar));
            // Shortened to nothing at the limit: the zero-length case (criterion 26).
            if length_mm <= LENGTH_TOLERANCE.as_mm() {
                return Err(LengthNotPositive);
            }
        }
        Ok(PenEntryPlaced {
            point: Point::new(self.last.x + length_mm * cos, self.last.y + length_mm * sin),
            length: Length::from_mm(length_mm),
            angle: Angle::from_radians(degrees.to_radians()),
            notice,
        })
    }

    fn kept_or_live(&self, live_end: Point) -> Polar {
        match self.phase {
            Phase::Following => Polar::of(self.last, live_end),
            Phase::Frozen { kept, .. } => kept,
        }
    }
}

/// `degrees` reduced modulo 360 into (-180°, 180°] (criterion 27).
fn half_turn_range(degrees: f64) -> f64 {
    let reduced = degrees.rem_euclid(360.0);
    if reduced > 180.0 {
        reduced - 360.0
    } else {
        reduced
    }
}

/// How far from `last` along the unit direction `(cos, sin)` a point stays within the
/// coordinate limit on both axes (criterion 28).
fn reach_inside_limit(last: Point, cos: f64, sin: f64) -> f64 {
    let axis = |from: f64, step: f64| {
        if step.abs() <= f64::EPSILON {
            f64::INFINITY
        } else {
            (MAX_COORDINATE_MM.copysign(step) - from) / step
        }
    };
    axis(last.x, cos).min(axis(last.y, sin)).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const L: Point = Point::new(100.0, 50.0);
    const MM: DisplayUnit = DisplayUnit::Mm;

    fn typed(length: &str, angle: &str) -> Result<PenEntryPlaced, LengthNotPositive> {
        let mut entry = PenEntry::new(L);
        let live = Point::new(130.0, 40.0);
        entry.edit(PenEntryField::Length, length, live, live);
        entry.edit(PenEntryField::Angle, angle, live, live);
        entry.resolve(live, MM)
    }

    fn assert_at(placed: PenEntryPlaced, x: f64, y: f64) {
        assert!(
            (placed.point.x - x).abs() < 0.005 && (placed.point.y - y).abs() < 0.005,
            "expected ({x}, {y}), got {:?}",
            placed.point
        );
    }

    /// Criterion 15: the live segment's values with the readout's decimals.
    #[test]
    fn an_untouched_chip_shows_the_live_segment() {
        let entry = PenEntry::new(L);
        let live = Point::new(130.0, 40.0);
        assert_eq!(entry.field_text(PenEntryField::Length, live, MM), "31.6");
        assert_eq!(entry.field_text(PenEntryField::Angle, live, MM), "-18.4");
    }

    /// Criterion 16: the first edit freezes; the untouched field keeps its value.
    #[test]
    fn the_first_edit_freezes_the_untouched_field() {
        let mut entry = PenEntry::new(L);
        let live = Point::new(130.0, 40.0);
        entry.edit(PenEntryField::Length, "40", live, live);
        let elsewhere = Point::new(0.0, 0.0);
        assert!(entry.is_frozen());
        assert_eq!(entry.field_text(PenEntryField::Length, elsewhere, MM), "40");
        assert_eq!(
            entry.field_text(PenEntryField::Angle, elsewhere, MM),
            "-18.4"
        );
    }

    /// Criterion 17: the polar step, y down.
    #[test]
    fn typed_values_give_the_polar_point() {
        for (angle, x, y) in [
            ("0", 130.0, 50.0),
            ("90", 100.0, 80.0),
            ("-90", 100.0, 20.0),
            ("45", 121.213, 71.213),
            ("180", 70.0, 50.0),
        ] {
            assert_at(typed("30", angle).unwrap(), x, y);
        }
    }

    /// Criterion 19: with no field edited, the node goes to the live segment's end.
    #[test]
    fn enter_untouched_places_at_the_live_end() {
        let live = Point::new(130.07, 50.0);
        let placed = PenEntry::new(L).resolve(live, MM).unwrap();
        assert_eq!(placed.point, live);
        assert_eq!(placed.notice, None);
    }

    /// Criterion 21: unit suffixes convert; a cm document shows and reads cm.
    #[test]
    fn lengths_take_unit_suffixes_and_the_display_unit() {
        assert_at(typed("1in", "0").unwrap(), 125.4, 50.0);
        assert_at(typed("2 cm", "0°").unwrap(), 120.0, 50.0);
        let mut entry = PenEntry::new(L);
        let live = Point::new(130.0, 40.0);
        assert_eq!(
            entry.field_text(PenEntryField::Length, live, DisplayUnit::Cm),
            "3.16"
        );
        entry.edit(PenEntryField::Length, "4", live, live);
        entry.edit(PenEntryField::Angle, "0", live, live);
        assert_at(entry.resolve(live, DisplayUnit::Cm).unwrap(), 140.0, 50.0);
    }

    /// Criterion 24: a field that is not a number uses its frozen value, with a notice.
    #[test]
    fn not_a_number_uses_the_frozen_value() {
        let placed = typed("", "0").unwrap();
        assert_at(placed, 100.0 + 10f64.hypot(30.0), 50.0);
        assert!(matches!(
            placed.notice,
            Some(PenEntryNotice::LengthNotANumber(_))
        ));
        let placed = typed("30", "1..2").unwrap();
        assert!(matches!(
            placed.notice,
            Some(PenEntryNotice::AngleNotANumber(_))
        ));
    }

    /// Criterion 25: a negative length goes the opposite way.
    #[test]
    fn a_negative_length_is_drawn_in_the_opposite_direction() {
        let placed = typed("-30", "0").unwrap();
        assert_at(placed, 70.0, 50.0);
        assert_eq!(placed.notice, Some(PenEntryNotice::NegativeLength));
    }

    /// Criterion 26: zero, typed or live, places nothing.
    #[test]
    fn a_zero_length_is_refused() {
        assert_eq!(typed("0", "0"), Err(LengthNotPositive));
        assert_eq!(PenEntry::new(L).resolve(L, MM), Err(LengthNotPositive));
    }

    /// Criterion 27: any finite angle is reduced modulo 360°, with no notice.
    #[test]
    fn angles_are_reduced_modulo_a_full_turn() {
        for angle in ["450", "-270"] {
            let placed = typed("30", angle).unwrap();
            assert_at(placed, 100.0, 80.0);
            assert_eq!(placed.notice, None);
            assert!((placed.angle.as_radians().to_degrees() - 90.0).abs() < 1e-9);
        }
    }

    /// Criterion 28: a point beyond the coordinate limit is pulled back along the angle.
    #[test]
    fn a_point_beyond_the_limit_is_shortened_to_it() {
        let placed = typed("20000000", "0").unwrap();
        assert_at(placed, 10_000_000.0, 50.0);
        assert_eq!(placed.notice, Some(PenEntryNotice::TooFar));
    }
}
