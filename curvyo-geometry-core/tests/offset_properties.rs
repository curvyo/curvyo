//! Property tests of the offset kernel on random straight-edged polygons
//! (`specs/0038-path-offset` criteria 6, 9 and 11, `adrs.md` "tests"): an outset contains the
//! operand, an inset lies inside it, the area grows with the distance, and every result keeps the
//! output rules of criterion 33. The seed is fixed, so a failure reproduces.

// Native only: `proptest` needs the operating system.
#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    missing_docs
)]

mod common;

use common::offset_check::{area, check_node_rules, covers};
use common::{Anchors, KERNEL_TOLERANCE, distance_to_segment, grid_points, polygon};
use curvyo_document_core::Length;
use curvyo_geometry_core::{
    OffsetCap, OffsetError, OffsetJoin, OffsetOutlines, Outline, offset, signed_area_mm2,
};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed};

fn config() -> Config {
    Config {
        rng_seed: RngSeed::Fixed(0x0038_0FF5),
        failure_persistence: None,
        ..Config::default()
    }
}

/// A random polygon of 3 to 10 vertices on a 0.001 mm grid within 40 mm, wound either way; it
/// may cross itself.
fn random_polygon() -> impl Strategy<Value = Anchors> {
    prop::collection::vec((0..40_000_i32, 0..40_000_i32), 3..11).prop_map(|points| {
        let points: Vec<(f64, f64)> = points
            .into_iter()
            .map(|(x, y)| (f64::from(x) / 1000.0, f64::from(y) / 1000.0))
            .collect();
        polygon(&points)
    })
}

fn join() -> impl Strategy<Value = OffsetJoin> {
    prop_oneof![
        Just(OffsetJoin::Round),
        Just(OffsetJoin::Miter),
        Just(OffsetJoin::Bevel)
    ]
}

fn run(anchors: &Anchors, d: f64, join: OffsetJoin) -> Result<OffsetOutlines, OffsetError> {
    offset(
        &[Outline::new(anchors, true)],
        Length::from_mm(d),
        join,
        OffsetCap::Round,
        KERNEL_TOLERANCE,
    )
}

/// The operand's painted region as a Boolean result of its own (nonzero), for the area.
fn painted_area(anchors: &Anchors) -> f64 {
    let operand = vec![anchors.clone()];
    let union = common::run(curvyo_geometry_core::BooleanOp::Union, &[operand]);
    union.map_or(0.0, |result| common::area(&result))
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn an_outset_contains_the_operand_and_keeps_the_output_rules(
        anchors in random_polygon(),
        d in 1..5000_i32,
        join in join(),
    ) {
        let d = f64::from(d) / 1000.0;
        let Ok(result) = run(&anchors, d, join) else {
            // Only an operand without area is refused.
            prop_assert!(painted_area(&anchors).abs() < 1e-9);
            return Ok(());
        };
        prop_assert!(check_node_rules(&result).is_ok(), "{:?}", check_node_rules(&result));
        // Every vertex of the operand lies within d of its region, so it is covered.
        for (point, _, _) in &anchors {
            prop_assert!(covers(&result, *point), "{point:?} is not covered");
        }
        prop_assert!(area(&result) + 1e-6 >= painted_area(&anchors));
    }

    #[test]
    fn an_inset_lies_inside_the_operand(
        anchors in random_polygon(),
        d in 1..5000_i32,
        join in join(),
    ) {
        let d = f64::from(d) / 1000.0;
        let Ok(result) = run(&anchors, -d, join) else {
            return Ok(());
        };
        prop_assert!(check_node_rules(&result).is_ok(), "{:?}", check_node_rules(&result));
        let operand = vec![anchors.clone()];
        let region = common::ok(curvyo_geometry_core::BooleanOp::Union, &[operand]);
        for outline in result.flattened(0.002) {
            for point in outline {
                let p = ((point.x * 1000.0).round() as i64, (point.y * 1000.0).round() as i64);
                // Inside the region, or on its boundary within the kernel tolerance.
                let near_boundary = region.outlines().iter().any(|o| {
                    let g = grid_points(o);
                    (0..g.len()).any(|i| distance_to_segment(p, g[i], g[(i + 1) % g.len()]) <= 10.0)
                });
                prop_assert!(common::covers(&region, point) || near_boundary, "{point:?} outside");
            }
        }
        prop_assert!(area(&result) <= painted_area(&anchors) + 1e-6);
    }

    #[test]
    fn the_area_grows_with_the_distance(
        anchors in random_polygon(),
        a in -3000..3000_i32,
        b in -3000..3000_i32,
        join in join(),
    ) {
        let (low, high) = (f64::from(a.min(b)) / 1000.0, f64::from(a.max(b)) / 1000.0);
        let area_at = |d: f64| run(&anchors, d, join).map_or(0.0, |r| area(&r));
        if low.abs() < 0.001 || high.abs() < 0.001 {
            return Ok(());
        }
        // A rounding allowance: the arcs of two distances are cut into different chords.
        let slack = 0.01 * (1.0 + painted_area(&anchors).sqrt());
        prop_assert!(area_at(low) <= area_at(high) + slack, "{} > {}", area_at(low), area_at(high));
    }
}

#[test]
fn the_signed_area_of_an_outset_outer_outline_is_positive() {
    let anchors = polygon(&[(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)]);
    let result = run(&anchors, 1.0, OffsetJoin::Round).unwrap();
    assert!(signed_area_mm2(&result.flattened(0.01)[0]) > 0.0);
}
