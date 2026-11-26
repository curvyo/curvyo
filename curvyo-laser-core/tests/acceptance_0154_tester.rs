//! Independent acceptance tests for `specs/0154-library-into-slots` criteria 6 and 16 (the fit
//! of a unit into a fixture slot, the "filled" box rule of Terms), written from the specification
//! and ADR 0088 before the implementation was read. The signatures of `fit_into_slot`,
//! `slot_fit_matrix`, `FitMode` and `SlotFit` are the names the spec and ADR 0088 give.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    missing_docs
)]

use curvyo_document_core::{Affine, Angle, Length, Point};
use curvyo_laser_core::{FitMode, Slot, SlotFit, SlotShape, fit_into_slot, slot_fit_matrix};

const EPS: f64 = 1e-6;

fn mm(v: f64) -> Length {
    Length::from_mm(v)
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

fn fit(unit: (f64, f64), slot: &Slot, mode: FitMode) -> SlotFit {
    fit_into_slot((mm(unit.0), mm(unit.1)), slot, mode)
}

/// The box of the unit after the fit: its width and height in mm, in the slot's frame.
fn fitted_box(unit: (f64, f64), f: SlotFit) -> (f64, f64) {
    let (w, h) = (unit.0 * f.scale, unit.1 * f.scale);
    if f.turned { (h, w) } else { (w, h) }
}

#[test]
fn ac6_a_60_by_40_page_into_a_30_square_is_scaled_to_half_and_not_turned() {
    let slot = rect(30.0, 30.0);
    let f = fit((60.0, 40.0), &slot, FitMode::Fit);
    assert!((f.scale - 0.5).abs() < EPS, "scale {}", f.scale);
    assert!(
        !f.turned,
        "a box in a square is never turned by a gain of zero"
    );
    assert!(!f.matched);
    let (w, h) = fitted_box((60.0, 40.0), f);
    assert!(
        (w - 30.0).abs() < EPS && (h - 20.0).abs() < EPS,
        "box {w} x {h}"
    );
}

#[test]
fn ac6_a_40_by_60_page_into_a_60_by_40_slot_is_turned_and_not_scaled() {
    let slot = rect(60.0, 40.0);
    let f = fit((40.0, 60.0), &slot, FitMode::Fit);
    assert!(f.turned, "turned to fit the slot's orientation");
    assert!(f.matched, "60 x 40 in 40 x 60 turned is an exact match");
    assert!(
        (f.scale - 1.0).abs() < EPS,
        "an exact match is not scaled, scale {}",
        f.scale
    );
}

#[test]
fn ac6_a_40_by_60_page_into_a_30_by_20_slot_is_turned_and_scaled_to_half() {
    let slot = rect(30.0, 20.0);
    let f = fit((40.0, 60.0), &slot, FitMode::Fit);
    assert!(
        f.turned,
        "turning gains more than 0.1 mm on the longer side"
    );
    assert!((f.scale - 0.5).abs() < EPS, "scale {}", f.scale);
    let (w, h) = fitted_box((40.0, 60.0), f);
    assert!(
        (w - 30.0).abs() < EPS && (h - 20.0).abs() < EPS,
        "box {w} x {h}"
    );
}

#[test]
fn ac6_a_round_30_mm_piece_into_a_25_mm_circle_becomes_25_mm() {
    let slot = circle(25.0);
    let f = fit((30.0, 30.0), &slot, FitMode::Fit);
    assert!((f.scale - 25.0 / 30.0).abs() < EPS, "scale {}", f.scale);
    let (w, h) = fitted_box((30.0, 30.0), f);
    assert!(
        (w - 25.0).abs() < EPS && (h - 25.0).abs() < EPS,
        "box {w} x {h}"
    );
}

#[test]
fn ac6_a_25_square_into_a_25_05_by_25_slot_is_only_centred() {
    let slot = rect(25.05, 25.0);
    let f = fit((25.0, 25.0), &slot, FitMode::Fit);
    assert!(f.matched, "0.05 mm is within the 0.1 mm match");
    assert!(!f.turned, "a square is never turned");
    assert!((f.scale - 1.0).abs() < EPS, "scale {}", f.scale);
}

#[test]
fn ac6_an_exact_0_1_mm_difference_matches_and_is_not_scaled() {
    let slot = rect(25.0, 25.0);
    let f = fit((25.1, 25.0), &slot, FitMode::Fit);
    assert!(f.matched, "a difference of exactly 0.1 mm is a match");
    assert!((f.scale - 1.0).abs() < EPS, "scale {}", f.scale);
}

#[test]
fn ac6_a_difference_just_over_0_1_mm_is_not_a_match_and_is_scaled() {
    let slot = rect(25.0, 25.0);
    let f = fit((25.2, 25.0), &slot, FitMode::Fit);
    assert!(!f.matched);
    assert!(
        f.scale < 1.0,
        "a larger piece is scaled down, scale {}",
        f.scale
    );
    let (w, _h) = fitted_box((25.2, 25.0), f);
    assert!(
        w <= 25.0 + 1e-4,
        "the box is contained in the slot, width {w}"
    );
}

#[test]
fn ac6_a_turned_holder_fit_is_a_turn_of_the_unit_with_the_bed() {
    // A turned holder turns the piece with it: the edge direction of the placed box rotates by
    // the holder's angle, so a 90 degree holder changes the edge direction of the piece by 90.
    let slot = rect(60.0, 40.0);
    let f = fit((40.0, 60.0), &slot, FitMode::Fit);
    let centre = Point::new(10.0, 10.0);
    let slot_centre = Point::new(100.0, 50.0);
    let identity = slot_fit_matrix(centre, f, slot_centre, Affine::scale(1.0, 1.0));
    let turned_bed = Affine::rotation_about(
        Point::new(0.0, 0.0),
        Angle::from_radians(std::f64::consts::FRAC_PI_2),
    );
    let on_bed = slot_fit_matrix(centre, f, slot_centre, turned_bed);
    let edge = |m: Affine| {
        let a = m.apply_point(Point::new(0.0, 0.0));
        let b = m.apply_point(Point::new(1.0, 0.0));
        (b.y - a.y).atan2(b.x - a.x)
    };
    let mut delta = edge(on_bed) - edge(identity);
    while delta < 0.0 {
        delta += std::f64::consts::TAU;
    }
    assert!(
        (delta - std::f64::consts::FRAC_PI_2).abs() < 1e-6,
        "the piece's edge turns by 90 degrees with the bed, delta {delta}"
    );
}

#[test]
fn ac6_the_fit_matrix_centres_the_unit_on_the_slot_centre() {
    let slot = rect(30.0, 30.0);
    let f = fit((60.0, 40.0), &slot, FitMode::Fit);
    let unit_centre = Point::new(200.0, 120.0);
    let slot_centre = Point::new(15.0, -7.0);
    let m = slot_fit_matrix(unit_centre, f, slot_centre, Affine::scale(1.0, 1.0));
    let mapped = m.apply_point(unit_centre);
    assert!(
        (mapped.x - slot_centre.x).abs() < EPS && (mapped.y - slot_centre.y).abs() < EPS,
        "the unit centre lands on the slot centre, got {mapped:?}"
    );
}

#[test]
fn ac6_alt_real_size_keeps_the_size_and_centres_only() {
    let slot = rect(30.0, 20.0);
    let f = fit((40.0, 60.0), &slot, FitMode::RealSize);
    assert!(
        (f.scale - 1.0).abs() < EPS,
        "real size keeps the scale, scale {}",
        f.scale
    );
    assert!(!f.turned, "real size turns only for a 0.1 mm match");
    assert!(
        f.overflows,
        "a 40 x 60 piece at real size passes the 30 x 20 slot"
    );
}

#[test]
fn ac6_alt_real_size_turns_only_for_a_match() {
    let slot = rect(60.0, 40.0);
    let f = fit((40.0, 60.0), &slot, FitMode::RealSize);
    assert!(f.matched && f.turned, "a 0.1 mm match turns at real size");
    assert!((f.scale - 1.0).abs() < EPS);
    assert!(!f.overflows);
}

#[test]
fn ac6_a_real_size_fit_of_a_small_piece_does_not_overflow() {
    let slot = rect(30.0, 30.0);
    let f = fit((10.0, 10.0), &slot, FitMode::RealSize);
    assert!((f.scale - 1.0).abs() < EPS);
    assert!(!f.overflows);
}

#[test]
fn ac6_a_zero_size_unit_gives_a_finite_fit() {
    let slot = rect(30.0, 30.0);
    let f = fit((0.0, 0.0), &slot, FitMode::Fit);
    assert!(f.scale.is_finite(), "scale {}", f.scale);
}

#[test]
fn ac6_scaling_never_gives_a_box_larger_than_the_slot_box() {
    let slots = [
        rect(30.0, 30.0),
        rect(60.0, 40.0),
        rect(30.0, 20.0),
        circle(25.0),
    ];
    let units = [
        (60.0, 40.0),
        (40.0, 60.0),
        (30.0, 30.0),
        (5.0, 90.0),
        (120.0, 7.0),
    ];
    for slot in &slots {
        let (sw, sh) = match slot.shape {
            SlotShape::Rectangle => (slot.width.as_mm(), slot.height.as_mm()),
            SlotShape::Circle => (slot.diameter.as_mm(), slot.diameter.as_mm()),
        };
        for unit in units {
            let f = fit(unit, slot, FitMode::Fit);
            let (w, h) = fitted_box(unit, f);
            assert!(
                w <= sw + 1e-4 && h <= sh + 1e-4,
                "unit {unit:?} box {w} x {h} exceeds slot {sw} x {sh}"
            );
        }
    }
}

#[test]
fn criterion_16_holds_is_the_slot_outline_grown_by_the_tolerance() {
    let slot = rect(30.0, 30.0);
    let tol = mm(0.1);
    assert!(
        slot.holds(Point::new(15.05, 0.0), tol),
        "0.05 mm outside is inside the grown box"
    );
    assert!(
        !slot.holds(Point::new(15.2, 0.0), tol),
        "0.2 mm outside is not"
    );
    let round = circle(25.0);
    assert!(round.holds(Point::new(12.55, 0.0), tol));
    assert!(!round.holds(Point::new(12.7, 0.0), tol));
}
