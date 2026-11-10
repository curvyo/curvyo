//! The fit rule: a part fits a slot whose size it matches within a tolerance, turned by 90 degrees
//! only when the other orientation is the fit (`specs/0102-laser-fixtures` criterion 25, Frage
//! 173 A, Proposal tie rule).

use curvyo_document_core::Length;

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
mod tests {
    use super::*;
    use crate::fixture_kind::SlotShape;
    use curvyo_document_core::Point;

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

    /// The tolerance the customer chose on 2026-11-10: 0.1 mm on each side, centre to centre.
    const FIT_TOLERANCE_MM: f64 = 0.1;

    fn tolerance() -> Length {
        mm(FIT_TOLERANCE_MM)
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
