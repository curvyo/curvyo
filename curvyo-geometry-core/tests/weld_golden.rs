//! Fixtures (c), (d) and (e) of `specs/0068-laserable-lettering` criterion 27, and criterion 25:
//! Weld on the degenerate fixtures (a) to (i) of `0016` criterion 40, on the 20 squares of `0037`
//! criterion 17 (a) and on 2,000 such squares. Each result is summarised (closed and open
//! outlines, nodes, total line length, a hash of the grid coordinates) in
//! `fixtures/weld/weld.golden`, embedded so that the same test runs in the wasm build:
//!
//! ```text
//! cargo test --target wasm32-unknown-unknown -p curvyo-geometry-core --test weld_golden
//! ```
//!
//! Each case must weld within 2 s in a release build (asserted natively in release only; CI runs
//! this file in the `performance-budgets` job). Regenerate with `CURVYO_UPDATE_GOLDEN=1` (native)
//! and review the diff.

#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

mod common;

use common::golden::{input_part, normalise, parse_operands};
use common::{KERNEL_TOLERANCE, Operand, rect, single};
use curvyo_geometry_core::{BooleanError, Outline, Welded, weld};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const GOLDEN: &str = include_str!("fixtures/weld/weld.golden");

const FIXTURES: &[(&str, &str)] = &[
    (
        "ac40a",
        include_str!("fixtures/boolean/ac40a_identical_squares.fixture"),
    ),
    (
        "ac40b",
        include_str!("fixtures/boolean/ac40b_squares_sharing_an_edge.fixture"),
    ),
    (
        "ac40c",
        include_str!("fixtures/boolean/ac40c_squares_sharing_a_corner.fixture"),
    ),
    (
        "ac40d",
        include_str!("fixtures/boolean/ac40d_square_inside_sharing_an_edge.fixture"),
    ),
    (
        "ac40e",
        include_str!("fixtures/boolean/ac40e_duplicated_node.fixture"),
    ),
    (
        "ac40f",
        include_str!("fixtures/boolean/ac40f_zero_length_segment.fixture"),
    ),
    (
        "ac40g",
        include_str!("fixtures/boolean/ac40g_bow_tie.fixture"),
    ),
    (
        "ac40h",
        include_str!("fixtures/boolean/ac40h_sliver.fixture"),
    ),
];

fn weld_closed(operands: &[Operand]) -> Result<Welded, BooleanError> {
    let outlines: Vec<Vec<Outline<'_>>> = operands
        .iter()
        .map(|operand| operand.iter().map(|a| Outline::new(a, true)).collect())
        .collect();
    let borrowed: Vec<&[Outline<'_>]> = outlines.iter().map(Vec::as_slice).collect();
    weld(&borrowed, KERNEL_TOLERANCE)
}

/// `count` squares of side 10 mm at x = 5 i, each overlapping its left neighbour by 5 mm, the
/// highest index on top (`0037` criterion 17 (a) and (b)).
fn squares(count: u32) -> Vec<Operand> {
    (0..count)
        .map(|i| {
            let x = 5.0 * f64::from(i);
            single(rect(x, 0.0, x + 10.0, 10.0))
        })
        .collect()
}

/// Total line length in millimetres, closing segments of closed outlines included.
fn length(welded: &Welded) -> f64 {
    welded
        .outlines
        .iter()
        .map(|(points, closed)| {
            let mut ends: Vec<_> = points.clone();
            if *closed {
                ends.push(points[0]);
            }
            ends.windows(2)
                .map(|p| {
                    let (dx, dy) = (p[1].x - p[0].x, p[1].y - p[0].y);
                    (dx * dx + dy * dy).sqrt()
                })
                .sum::<f64>()
        })
        .sum()
}

fn summary_line(name: &str, result: &Result<Welded, BooleanError>) -> String {
    let Ok(welded) = result else {
        return format!("{name}: refused {:?}\n", result.as_ref().err().unwrap());
    };
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for (points, closed) in &welded.outlines {
        hash = (hash ^ u64::from(*closed)).wrapping_mul(0x0100_0000_01b3);
        for p in points {
            for v in [p.x, p.y] {
                hash = (hash ^ ((v * 1000.0).round() as i64 as u64)).wrapping_mul(0x0100_0000_01b3);
            }
        }
    }
    let closed = welded.outlines.iter().filter(|o| o.1).count();
    let nodes: usize = welded.outlines.iter().map(|o| o.0.len()).sum();
    format!(
        "{name}: closed {closed} open {} nodes {nodes} length_mm {:.3} hidden {} hash {hash:016x}\n",
        welded.outlines.len() - closed,
        length(welded),
        welded.hidden.len()
    )
}

/// Runs `case` and checks its budget of 2 s in a native release build.
fn timed(name: &str, case: impl FnOnce() -> Result<Welded, BooleanError>) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    let started = std::time::Instant::now();
    let result = case();
    #[cfg(not(target_arch = "wasm32"))]
    {
        let elapsed = started.elapsed();
        println!("{name}: {elapsed:?}");
        if !cfg!(debug_assertions) {
            assert!(
                elapsed < std::time::Duration::from_secs(2),
                "{name}: {elapsed:?}"
            );
        }
    }
    summary_line(name, &result)
}

/// Criterion 27 (d): one closed 105 x 10 mm outline and 19 open 10 mm lines, 420 mm in total.
#[test]
fn ac27d_twenty_squares_give_one_outline_and_nineteen_lines() {
    let welded = weld_closed(&squares(20)).unwrap();
    let closed: Vec<_> = welded.outlines.iter().filter(|o| o.1).collect();
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].0.len(), 4);
    assert_eq!(welded.outlines.len(), 20);
    assert!(
        (length(&welded) - 420.0).abs() < 1e-6,
        "{}",
        length(&welded)
    );
}

/// Criteria 27 (c) to (e) and 25: the fixtures weld within budget to their golden results.
#[test]
fn ac27_the_fixtures_weld_to_their_golden_results() {
    let mut got = String::from(
        "# Weld (specs/0068-laserable-lettering criterion 27 (c) to (e)).\n# Regenerate with CURVYO_UPDATE_GOLDEN=1.\n",
    );
    for (name, text) in FIXTURES {
        let operands = parse_operands(input_part(&normalise(text)));
        got += &timed(name, || weld_closed(&operands));
    }
    let tiny: Operand = (0..5000)
        .map(|i| {
            let (col, row) = (f64::from(i % 100), f64::from(i / 100));
            rect(col, row, col + 0.5, row + 0.5)
        })
        .collect();
    let tiny = [tiny, single(rect(-1.0, -1.0, 50.0, 25.0))];
    got += &timed("ac40i", || weld_closed(&tiny));
    let twenty = squares(20);
    got += &timed("squares20", || weld_closed(&twenty));
    let many = squares(2000);
    got += &timed("squares2000", || weld_closed(&many));
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/weld/weld.golden");
        std::fs::write(path, &got).unwrap();
        return;
    }
    assert_eq!(got, normalise(GOLDEN));
}
