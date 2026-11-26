//! The fit rule: how a part sits in a slot, matched within a tolerance or scaled to it, turned by
//! 90 degrees only when that is the better fit (`specs/0102-laser-fixtures` criterion 25, Frage
//! 173 A; `specs/0154-library-into-slots` criterion 6, ADR 0088 decision 1).

use std::f64::consts::FRAC_PI_2;

use curvyo_document_core::{Affine, Angle, Length, Point, Vec2};

use crate::fixture_slot::Slot;

/// The slot a part fits, and whether the part is turned by 90 degrees to fit it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FitMatch {
    /// The index of the slot in the slice given to [`fit_slot`].
    pub slot: usize,
    /// True only when the part's other orientation is the fit.
    pub turned: bool,
}

/// How far past the tolerance a size difference may round and still fit, in mm.
const FIT_EPSILON_MM: f64 = 1e-9;

/// The fit tolerance, on each side (customer, 2026-11-10): a part this close to a slot's size
/// matches it, and a turn must gain more than this on the part's longer side.
pub const FIT_TOLERANCE: Length = Length::from_mm(0.1);

/// A unit side at or below this many mm has no size to scale by (ADR 0088 decision 1.3).
const NO_SIZE_MM: f64 = 1e-6;

/// How a unit is fitted into a slot (Frage 379 A).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitMode {
    /// Centred, scaled uniformly to fit inside the slot's box, turned when that fits better.
    Fit,
    /// Centred at its real size (the held Alt), turned only for a 0.1 mm match.
    RealSize,
}

/// How a unit sits in a slot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotFit {
    /// The uniform scale, a dimensionless ratio (1 for a match or the real size).
    pub scale: f64,
    /// Whether the unit is turned by 90 degrees.
    pub turned: bool,
    /// Whether the unit's size matched the slot within [`FIT_TOLERANCE`].
    pub matched: bool,
    /// Whether the unit at its real size passes a side of the slot by more than the tolerance.
    pub overflows: bool,
}

/// How the unit `unit` (width, height, in the board's frame) fits `slot` in `mode`: a 0.1 mm
/// match first (either orientation, unturned preferred), then the real size or the uniform scale
/// that makes its box fit inside the slot's box (a circle: the square of its diameter), turned
/// only when the turn gains more than [`FIT_TOLERANCE`] on the unit's longer side.
#[must_use]
pub fn fit_into_slot(unit: (Length, Length), slot: &Slot, mode: FitMode) -> SlotFit {
    if let Some(found) = fit_slot(unit, std::slice::from_ref(slot), FIT_TOLERANCE) {
        return SlotFit {
            scale: 1.0,
            turned: found.turned,
            matched: true,
            overflows: false,
        };
    }
    let (w, h) = (unit.0.as_mm(), unit.1.as_mm());
    let (sw, sh) = slot_mm(slot);
    match mode {
        FitMode::RealSize => {
            let tol = FIT_TOLERANCE.as_mm() + FIT_EPSILON_MM;
            SlotFit {
                scale: 1.0,
                turned: false,
                matched: false,
                overflows: w - sw > tol || h - sh > tol,
            }
        }
        FitMode::Fit => {
            let (Some(s0), Some(s90)) = (scale_to((w, h), (sw, sh)), scale_to((h, w), (sw, sh)))
            else {
                return SlotFit {
                    scale: 1.0,
                    turned: false,
                    matched: false,
                    overflows: false,
                };
            };
            let longer = w.max(h);
            let turned = (s90 - s0) * longer > FIT_TOLERANCE.as_mm() + FIT_EPSILON_MM;
            SlotFit {
                scale: if turned { s90 } else { s0 },
                turned,
                matched: false,
                overflows: false,
            }
        }
    }
}

/// The largest uniform scale that keeps `(w, h)` inside `(sw, sh)`; a side without size is left
/// out of the minimum, and a unit without any size has none.
fn scale_to((w, h): (f64, f64), (sw, sh): (f64, f64)) -> Option<f64> {
    let by_w = (w > NO_SIZE_MM).then(|| sw / w);
    let by_h = (h > NO_SIZE_MM).then(|| sh / h);
    match (by_w, by_h) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (one, other) => one.or(other),
    }
}

/// The map from the unit's space to document space that puts the unit (box centre
/// `unit_centre`) into a slot at `slot_centre` of a board whose frame maps to document space by
/// the rigid `board`: `board ∘ T(slot centre) ∘ R(turn) ∘ S(scale) ∘ T(−unit centre)`. A board
/// turned by any angle turns the unit with it.
#[must_use]
pub fn slot_fit_matrix(
    unit_centre: Point,
    fit: SlotFit,
    slot_centre: Point,
    board: Affine,
) -> Affine {
    let turn = if fit.turned {
        Affine::rotation_about(Point::new(0.0, 0.0), Angle::from_radians(FRAC_PI_2))
    } else {
        Affine::IDENTITY
    };
    board
        .compose(Affine::translation(Vec2::new(slot_centre.x, slot_centre.y)))
        .compose(turn)
        .compose(Affine::scale(fit.scale, fit.scale))
        .compose(Affine::translation(Vec2::new(
            -unit_centre.x,
            -unit_centre.y,
        )))
}

/// The first slot the part `item` (width, height) fits, unturned slots before turned ones, so a
/// part that fits without a turn is never turned. Both sides must match within `tolerance`
/// (inclusive); a size that differs by more than the tolerance in both orientations does not fit.
#[must_use]
pub fn fit_slot(item: (Length, Length), slots: &[Slot], tolerance: Length) -> Option<FitMatch> {
    let (w, h) = (item.0.as_mm(), item.1.as_mm());
    // "Within 0.1 mm" holds at the boundary: a difference of exactly the tolerance fits, whatever
    // the rounding of the two sizes (an explicit epsilon, CLAUDE.md §5).
    let tol = tolerance.as_mm() + FIT_EPSILON_MM;
    let matches = |(sw, sh): (f64, f64)| (w - sw).abs() <= tol && (h - sh).abs() <= tol;
    let sizes = || slots.iter().map(slot_mm);
    if let Some(slot) = sizes().position(matches) {
        return Some(FitMatch {
            slot,
            turned: false,
        });
    }
    let turned_matches = |(sw, sh): (f64, f64)| (h - sw).abs() <= tol && (w - sh).abs() <= tol;
    sizes()
        .position(turned_matches)
        .map(|slot| FitMatch { slot, turned: true })
}

/// A slot's fit size in millimetres.
fn slot_mm(slot: &Slot) -> (f64, f64) {
    let (width, height) = slot.extent();
    (width.as_mm(), height.as_mm())
}

#[cfg(test)]
#[path = "fixture_fit_tests.rs"]
mod into_slot_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture_kind::SlotShape;

    fn rect(w: f64, h: f64) -> Slot {
        Slot {
            shape: SlotShape::Rectangle,
            centre: Point::new(0.0, 0.0),
            width: Length::from_mm(w),
            height: Length::from_mm(h),
            diameter: Length::from_mm(0.0),
        }
    }

    fn mm(value: f64) -> Length {
        Length::from_mm(value)
    }

    fn tolerance() -> Length {
        FIT_TOLERANCE
    }

    #[test]
    fn a_part_of_the_slot_size_fits_without_a_turn() {
        let slots = [rect(20.0, 30.0)];
        assert_eq!(
            fit_slot((mm(20.0), mm(30.0)), &slots, tolerance()),
            Some(FitMatch {
                slot: 0,
                turned: false
            })
        );
    }

    #[test]
    fn a_size_within_the_tolerance_fits_and_one_beyond_it_does_not() {
        let slots = [rect(20.0, 30.0)];
        assert!(fit_slot((mm(20.05), mm(29.95)), &slots, tolerance()).is_some());
        assert_eq!(fit_slot((mm(20.2), mm(30.0)), &slots, tolerance()), None);
        assert_eq!(fit_slot((mm(20.0), mm(30.5)), &slots, tolerance()), None);
    }

    #[test]
    fn a_part_turned_by_ninety_degrees_fits_and_is_turned() {
        let slots = [rect(20.0, 30.0)];
        assert_eq!(
            fit_slot((mm(30.0), mm(20.0)), &slots, tolerance()),
            Some(FitMatch {
                slot: 0,
                turned: true
            })
        );
    }

    #[test]
    fn a_square_part_in_a_square_slot_is_never_turned() {
        let slots = [rect(20.0, 20.0)];
        assert_eq!(
            fit_slot((mm(20.05), mm(19.92)), &slots, tolerance()),
            Some(FitMatch {
                slot: 0,
                turned: false
            })
        );
    }

    #[test]
    fn an_unturned_fit_in_a_later_slot_beats_a_turned_fit_in_an_earlier_one() {
        let slots = [rect(50.0, 20.0), rect(20.0, 50.0)];
        assert_eq!(
            fit_slot((mm(20.0), mm(50.0)), &slots, tolerance()),
            Some(FitMatch {
                slot: 1,
                turned: false
            })
        );
    }

    #[test]
    fn a_circle_takes_a_square_part_of_its_diameter_unturned() {
        let circle = Slot {
            shape: SlotShape::Circle,
            centre: Point::new(0.0, 0.0),
            width: mm(0.0),
            height: mm(0.0),
            diameter: mm(12.0),
        };
        assert_eq!(
            fit_slot((mm(12.0), mm(12.0)), &[circle], tolerance()),
            Some(FitMatch {
                slot: 0,
                turned: false
            })
        );
    }

    #[test]
    fn no_slot_means_no_fit() {
        assert_eq!(fit_slot((mm(10.0), mm(10.0)), &[], tolerance()), None);
    }

    #[test]
    fn a_part_exactly_the_tolerance_wider_fits_and_a_hair_more_does_not() {
        let mm = Length::from_mm;
        let slots = [rect(210.0, 297.0)];
        // 210.1 - 210.0 rounds to 0.10000000000000853 in binary floating point.
        assert!(fit_slot((mm(210.1), mm(297.0)), &slots, mm(0.1)).is_some());
        assert!(
            fit_slot((mm(297.1), mm(210.0)), &slots, mm(0.1)).is_some(),
            "turned"
        );
        assert!(fit_slot((mm(210.101), mm(297.0)), &slots, mm(0.1)).is_none());
        assert!(fit_slot((mm(209.899), mm(297.0)), &slots, mm(0.1)).is_none());
    }
}
