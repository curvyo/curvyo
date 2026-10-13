//! The performance budgets of `specs/0016-boolean-operations` criteria 44 and 45: two operands
//! of 1,000 curved nodes each within 100 ms, and of 10,000 within 2 s, for each of union,
//! difference and intersection (kernel call only, release build).
//!
//! The budgets are asserted in release builds only; a debug build runs the same cases (about
//! 0.05 s and 0.5 s) and prints the timings. CI runs them in release, in the `boolean-budgets`
//! job; locally: `cargo test --release -p curvyo-geometry-core --test boolean_performance`.
//!
//! The same job holds the offset budget of `specs/0038-path-offset` criterion 18: an operand of 200
//! curved nodes within 50 ms for every join, outward and inward (see the test for the distances).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

mod common;

use std::time::{Duration, Instant};

use common::{Operand, run, wavy};
use curvyo_document_core::Length;
use curvyo_geometry_core::{BooleanOp, OffsetJoin, Outline, offset};

fn measure(nodes: usize, budget: Duration) {
    let radius = nodes as f64 / std::f64::consts::TAU;
    let operands: [Operand; 2] = [
        vec![wavy(nodes, 0.0, 0.0)],
        vec![wavy(nodes, radius * 0.7, radius * 0.3)],
    ];
    for (name, op) in [
        ("union", BooleanOp::Union),
        ("difference", BooleanOp::Difference),
        ("intersection", BooleanOp::Intersection),
    ] {
        let started = Instant::now();
        let result = run(op, &operands).unwrap();
        let elapsed = started.elapsed();
        println!(
            "2 x {nodes} nodes, {name}: {elapsed:?} ({} outlines, {} nodes out)",
            result.outlines().len(),
            common::node_count(&result)
        );
        if !cfg!(debug_assertions) {
            assert!(
                elapsed < budget,
                "{name} took {elapsed:?}, budget {budget:?}"
            );
        }
    }
}

/// Criterion 44.
#[test]
fn two_operands_of_one_thousand_nodes() {
    measure(1_000, Duration::from_millis(100));
}

/// Criterion 45.
#[test]
fn two_operands_of_ten_thousand_nodes() {
    measure(10_000, Duration::from_secs(2));
}

/// `specs/0038-path-offset` criterion 18: the kernel part of one preview update, on the gear-like
/// operand of the Boolean budgets, within 50 ms for every join at 2 mm, -1 mm and 5 mm (about 1 ms
/// each on the development machine since the curve rebuild of 2026-10-13).
#[test]
fn an_offset_of_two_hundred_curved_nodes() {
    let operand = wavy(200, 0.0, 0.0);
    for join in [OffsetJoin::Round, OffsetJoin::Miter, OffsetJoin::Bevel] {
        for d in [2.0, -1.0, 5.0] {
            let budget = Duration::from_millis(50);
            let started = Instant::now();
            let result = offset(
                &[Outline::new(&operand, true)],
                Length::from_mm(d),
                join,
                common::KERNEL_TOLERANCE,
            )
            .unwrap();
            let elapsed = started.elapsed();
            println!(
                "offset of 200 nodes, {join:?} {d} mm: {elapsed:?} ({} segments out, {} fallback)",
                result.segment_count(),
                result.fallback_outlines()
            );
            if !cfg!(debug_assertions) {
                assert!(elapsed < budget, "{join:?} {d} took {elapsed:?}");
            }
        }
    }
}
