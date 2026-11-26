//! The table test of the fit into a slot (`specs/0154-library-into-slots` criterion 6, ADR 0088
//! decision 1).

use super::*;
use crate::fixture_kind::SlotShape;

fn mm(value: f64) -> Length {
    Length::from_mm(value)
}

fn rect(w: f64, h: f64) -> Slot {
    Slot {
        shape: SlotShape::Rectangle,
        centre: Point::new(0.0, 0.0),
        width: mm(w),
        height: mm(h),
        diameter: mm(0.0),
    }
}

fn circle(d: f64) -> Slot {
    Slot {
        shape: SlotShape::Circle,
        centre: Point::new(0.0, 0.0),
        width: mm(0.0),
        height: mm(0.0),
        diameter: mm(d),
    }
}

/// One row of the criterion 6 table: unit, slot, mode, then scale, turned, matched, overflows.
struct Case {
    name: &'static str,
    unit: (f64, f64),
    slot: Slot,
    mode: FitMode,
    scale: f64,
    turned: bool,
    matched: bool,
    overflows: bool,
}

#[test]
#[allow(clippy::too_many_lines)] // one row per case of the criterion 6 table
fn the_fit_cases_of_criterion_6() {
    use FitMode::{Fit, RealSize};
    let case = |name, unit, slot, mode, scale, turned, matched, overflows| Case {
        name,
        unit,
        slot,
        mode,
        scale,
        turned,
        matched,
        overflows,
    };
    let cases = [
        case(
            "60x40 into 30x30 is 50 %",
            (60.0, 40.0),
            rect(30.0, 30.0),
            Fit,
            0.5,
            false,
            false,
            false,
        ),
        case(
            "40x60 into 60x40 is turned, not scaled",
            (40.0, 60.0),
            rect(60.0, 40.0),
            Fit,
            1.0,
            true,
            true,
            false,
        ),
        case(
            "40x60 into 30x20 is turned and 50 %",
            (40.0, 60.0),
            rect(30.0, 20.0),
            Fit,
            0.5,
            true,
            false,
            false,
        ),
        case(
            "round 30 into circle 25 is 25",
            (30.0, 30.0),
            circle(25.0),
            Fit,
            25.0 / 30.0,
            false,
            false,
            false,
        ),
        case(
            "25x25 into 25.05x25 is only centred",
            (25.0, 25.0),
            rect(25.05, 25.0),
            Fit,
            1.0,
            false,
            true,
            false,
        ),
        case(
            "a square in a square is never turned",
            (10.0, 10.0),
            rect(20.0, 20.0),
            Fit,
            2.0,
            false,
            false,
            false,
        ),
        case(
            "scaled up",
            (10.0, 5.0),
            rect(40.0, 20.0),
            Fit,
            4.0,
            false,
            false,
            false,
        ),
        case(
            "zero width scales by its height, turned along the longer side",
            (0.0, 10.0),
            rect(30.0, 20.0),
            Fit,
            3.0,
            true,
            false,
            false,
        ),
        case(
            "no size is not scaled",
            (0.0, 0.0),
            rect(30.0, 20.0),
            Fit,
            1.0,
            false,
            false,
            false,
        ),
        case(
            "Alt keeps the real size",
            (20.0, 10.0),
            rect(30.0, 20.0),
            RealSize,
            1.0,
            false,
            false,
            false,
        ),
        case(
            "Alt larger than the slot overflows",
            (60.0, 40.0),
            rect(30.0, 30.0),
            RealSize,
            1.0,
            false,
            false,
            true,
        ),
        case(
            "Alt still turns for a match",
            (40.0, 60.0),
            rect(60.0, 40.0),
            RealSize,
            1.0,
            true,
            true,
            false,
        ),
    ];
    for c in cases {
        let fit = fit_into_slot((mm(c.unit.0), mm(c.unit.1)), &c.slot, c.mode);
        assert!(
            (fit.scale - c.scale).abs() < 1e-9,
            "{}: scale {}",
            c.name,
            fit.scale
        );
        assert_eq!(fit.turned, c.turned, "{}: turned", c.name);
        assert_eq!(fit.matched, c.matched, "{}: matched", c.name);
        assert_eq!(fit.overflows, c.overflows, "{}: overflows", c.name);
    }
}

#[test]
fn a_turn_must_gain_more_than_the_tolerance_on_the_longer_side() {
    // 100 x 50 into 60.1 x 99.9: unturned 0.601 (50 -> 30.05 ... width bound), turned 0.999.
    let fit = fit_into_slot((mm(100.0), mm(50.0)), &rect(60.1, 99.9), FitMode::Fit);
    assert!(fit.turned);
    // 100 x 98 into 99.9 x 99.95: unturned min(0.999, 1.0199) = 0.999, turned
    // min(0.999, 0.9995) = 0.999: no gain, so no turn.
    let fit = fit_into_slot((mm(100.0), mm(98.0)), &rect(99.9, 99.95), FitMode::Fit);
    assert!(!fit.turned);
    // Unturned 0.5, turned 0.5009: the gain is 0.09 mm on the 100 mm side, not a turn.
    let fit = fit_into_slot((mm(100.0), mm(60.0)), &rect(50.0, 50.054), FitMode::Fit);
    assert!(!fit.turned, "{fit:?}");
    // Unturned 0.5, turned 0.5012: 0.12 mm, a turn.
    let fit = fit_into_slot((mm(100.0), mm(60.0)), &rect(50.0, 50.12), FitMode::Fit);
    assert!(fit.turned, "{fit:?}");
}

#[test]
fn a_fitted_unit_is_centred_in_the_slot_and_turns_with_its_board() {
    let fit = fit_into_slot((mm(60.0), mm(40.0)), &rect(30.0, 30.0), FitMode::Fit);
    let board = Affine::translation(Vec2::new(100.0, 50.0));
    let m = slot_fit_matrix(Point::new(30.0, 20.0), fit, Point::new(15.0, 15.0), board);
    let a = m.apply_point(Point::new(0.0, 0.0));
    let b = m.apply_point(Point::new(60.0, 40.0));
    assert!(
        (a.x - 100.0).abs() < 1e-9 && (a.y - 55.0).abs() < 1e-9,
        "{a:?}"
    );
    assert!(
        (b.x - 130.0).abs() < 1e-9 && (b.y - 75.0).abs() < 1e-9,
        "{b:?}"
    );
    // A board turned by 30 degrees turns the unit by 30 degrees.
    let board = Affine::rotation_about(
        Point::new(0.0, 0.0),
        Angle::from_radians(std::f64::consts::FRAC_PI_6),
    );
    let m = slot_fit_matrix(Point::new(30.0, 20.0), fit, Point::new(0.0, 0.0), board);
    let along = m.apply_vector(Vec2::new(1.0, 0.0));
    assert!((along.y.atan2(along.x) - std::f64::consts::FRAC_PI_6).abs() < 1e-9);
    assert!((along.length() - 0.5).abs() < 1e-9);
}

#[test]
fn a_turned_fit_maps_the_units_width_onto_the_slots_height() {
    let fit = fit_into_slot((mm(40.0), mm(60.0)), &rect(30.0, 20.0), FitMode::Fit);
    let m = slot_fit_matrix(
        Point::new(20.0, 30.0),
        fit,
        Point::new(0.0, 0.0),
        Affine::IDENTITY,
    );
    let along = m.apply_vector(Vec2::new(40.0, 0.0));
    assert!(
        along.x.abs() < 1e-9 && (along.y.abs() - 20.0).abs() < 1e-9,
        "{along:?}"
    );
}
