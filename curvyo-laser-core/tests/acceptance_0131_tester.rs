//! Independent acceptance tests for `specs/0131-controller-bed-size`, criteria 1 (comparison),
//! 2 (the size the press writes), 3 (range and finiteness) and 4 (no memory: the offer is a pure
//! function of page and report). Written from the specification and the ADR wording before
//! reading the implementation.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines
)]

use curvyo_document_core::{DocumentSize, Length};
use curvyo_laser_core::machine_size_offer;

fn page(w: f64, h: f64) -> DocumentSize {
    DocumentSize::new(Length::from_mm(w), Length::from_mm(h))
}

fn offered(w: f64, h: f64) -> Option<(f64, f64)> {
    machine_size_offer(page(400.0, 300.0), [w, h]).map(|s| (s.width.as_mm(), s.height.as_mm()))
}

// ---- criterion 1: the comparison (tolerance 0.01 mm) -------------------------------------------

#[test]
fn c1_controller_400_850_against_copy_400_300_is_offered_as_400_850() {
    // The customer's case: $130=400.0, $131=850.0 against a 400 x 300 copy.
    assert_eq!(offered(400.0, 850.0), Some((400.0, 850.0)));
}

#[test]
fn c1_equal_size_gives_no_offer() {
    let same = machine_size_offer(page(400.0, 850.0), [400.0, 850.0]);
    assert!(same.is_none());
}

#[test]
fn c1_difference_within_tolerance_gives_no_offer() {
    // 0.005 mm apart on either side is within 0.01 mm.
    assert!(machine_size_offer(page(400.0, 300.0), [400.005, 300.0]).is_none());
    assert!(machine_size_offer(page(400.0, 300.0), [400.0, 299.995]).is_none());
}

#[test]
fn c1_difference_beyond_tolerance_is_offered() {
    // 0.05 mm apart: offered.
    assert!(machine_size_offer(page(400.0, 300.0), [400.05, 300.0]).is_some());
    assert!(machine_size_offer(page(400.0, 300.0), [400.0, 300.05]).is_some());
}

#[test]
fn c1_each_side_alone_counts() {
    assert_eq!(offered(400.0, 301.0), Some((400.0, 301.0)));
    assert_eq!(offered(401.0, 300.0), Some((401.0, 300.0)));
}

#[test]
fn c1_offered_size_is_the_reported_orientation_not_swapped() {
    // A 300 x 400 report against a 400 x 300 copy is offered as 300 x 400.
    assert_eq!(offered(300.0, 400.0), Some((300.0, 400.0)));
}

// ---- criterion 3: range and finiteness ---------------------------------------------------------

#[test]
fn c3_range_bounds_are_inclusive_offers() {
    assert_eq!(offered(1.0, 1.0), Some((1.0, 1.0)));
    assert_eq!(offered(10_000.0, 10_000.0), Some((10_000.0, 10_000.0)));
}

#[test]
fn c3_below_one_mm_gives_no_offer() {
    assert!(offered(0.99, 300.0).is_none());
    assert!(offered(400.0, 0.0).is_none());
    assert!(offered(0.0, 0.0).is_none());
}

#[test]
fn c3_above_ten_thousand_mm_gives_no_offer() {
    assert!(offered(400.0, 10_000.01).is_none());
    assert!(offered(10_000.5, 300.0).is_none());
}

#[test]
fn c3_negative_gives_no_offer() {
    assert!(offered(-400.0, 850.0).is_none());
    assert!(offered(400.0, -850.0).is_none());
}

#[test]
fn c3_non_finite_gives_no_offer() {
    assert!(offered(f64::NAN, 850.0).is_none());
    assert!(offered(400.0, f64::NAN).is_none());
    assert!(offered(f64::INFINITY, 850.0).is_none());
    assert!(offered(400.0, f64::NEG_INFINITY).is_none());
}

#[test]
fn c3_one_bad_side_hides_the_offer_even_if_the_other_differs() {
    // The good side differs from the copy, the bad side is out of range: no offer at all.
    assert!(offered(400.0, 20_000.0).is_none());
    assert!(offered(f64::NAN, 500.0).is_none());
}

// ---- criterion 2: the size the press writes is exactly the reported one ------------------------

#[test]
fn c2_offer_returns_exactly_the_reported_numbers() {
    let s = machine_size_offer(page(400.0, 300.0), [400.0, 850.0]).unwrap();
    assert_eq!(s, page(400.0, 850.0));
}

// ---- criterion 4: no refusal memory; a hand edit brings the offer back -------------------------

#[test]
fn c4_offer_is_a_pure_function_of_copy_and_report() {
    // After a hand edit back to 400 x 300 the same report is offered again, and after the copy
    // reaches the report the offer is gone. Nothing is remembered between calls.
    let report = [400.0, 850.0];
    assert!(machine_size_offer(page(400.0, 300.0), report).is_some());
    assert!(machine_size_offer(page(400.0, 850.0), report).is_none());
    assert!(machine_size_offer(page(400.0, 300.0), report).is_some());
}
