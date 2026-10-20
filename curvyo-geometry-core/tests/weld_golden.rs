//! Fixtures (c) and (d) of `specs/0068-laserable-lettering` criterion 31 and criteria 24 and 29:
//! Weld at g = 0.1 mm on the degenerate fixtures (a) to (i) of `0016` criterion 40 and on the
//! stroked line and square of criterion 9. Each result is summarised (pieces, outlines, nodes,
//! area, a hash of the grid coordinates) in `fixtures/weld/weld.golden`, embedded so that the
//! same test runs in the wasm build:
//!
//! ```text
//! cargo test --target wasm32-unknown-unknown -p curvyo-geometry-core --test weld_golden
//! ```
//!
//! Each case must weld within 2 s in a release build (asserted natively in release only).
//! Regenerate with `CURVYO_UPDATE_GOLDEN=1` (native) and review the diff.

#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

mod common;

use std::fmt::Write as _;

use common::golden::{input_part, normalise, parse_operands};
use common::{KERNEL_TOLERANCE, Operand, check_invariants, node_count, polygon, rect, single};
use curvyo_document_core::{Length, LineCap, LineJoin};
use curvyo_geometry_core::{
    BooleanError, Outline, StrokeBand, Weld, WeldOperand, signed_area_mm2, weld,
};

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

const GAP: Length = Length::from_mm(0.1);

/// Welds closed operands, timed.
fn weld_closed(operands: &[Operand]) -> Result<Weld, BooleanError> {
    let outlines: Vec<Vec<Outline<'_>>> = operands
        .iter()
        .map(|operand| operand.iter().map(|a| Outline::new(a, true)).collect())
        .collect();
    let operands: Vec<WeldOperand<'_>> = outlines
        .iter()
        .map(|regions| WeldOperand {
            regions,
            banded: &[],
            band: None,
        })
        .collect();
    weld(&operands, GAP, KERNEL_TOLERANCE)
}

/// Criterion 9's fixture: a 40 mm line with a 2 mm round-capped stroke and a 10 mm square on it.
fn weld_line_and_square() -> Result<Weld, BooleanError> {
    let line = polygon(&[(0.0, 0.0), (40.0, 0.0)]);
    let square = rect(15.0, -5.0, 25.0, 5.0);
    let open = [Outline::new(&line, false)];
    let closed = [Outline::new(&square, true)];
    let operands = [
        WeldOperand {
            regions: &[],
            banded: &open,
            band: Some(StrokeBand {
                width: Length::from_mm(2.0),
                join: LineJoin::Round,
                cap: LineCap::Round,
            }),
        },
        WeldOperand {
            regions: &closed,
            banded: &[],
            band: None,
        },
    ];
    weld(&operands, GAP, KERNEL_TOLERANCE)
}

fn summary_line(name: &str, result: &Result<Weld, BooleanError>) -> String {
    let Ok(welded) = result else {
        return format!("{name}: refused {:?}\n", result.as_ref().err().unwrap());
    };
    assert_eq!(check_invariants(&welded.result), Ok(()), "{name}");
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for p in welded.result.outlines().iter().flatten() {
        for v in [p.x, p.y] {
            hash = (hash ^ ((v * 1000.0).round() as i64 as u64)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    let area: f64 = welded
        .result
        .outlines()
        .iter()
        .map(|o| signed_area_mm2(o))
        .sum();
    let mut line = String::new();
    writeln!(
        line,
        "{name}: pieces {} outlines {} nodes {} area_mm2 {area:.3} hash {hash:016x}",
        welded.pieces,
        welded.result.outlines().len(),
        node_count(&welded.result)
    )
    .unwrap();
    line
}

/// Runs `case` and checks its budget of 2 s in a native release build.
fn timed(name: &str, case: impl FnOnce() -> Result<Weld, BooleanError>) -> String {
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

#[test]
fn ac31_the_fixtures_weld_to_their_golden_results() {
    let mut got = String::from(
        "# Weld at g = 0.1 mm (specs/0068-laserable-lettering criterion 31 (c), (d)).\n# Regenerate with CURVYO_UPDATE_GOLDEN=1.\n",
    );
    for (name, text) in FIXTURES {
        let operands = parse_operands(input_part(&normalise(text)));
        got += &timed(name, || weld_closed(&operands));
    }
    let squares: Operand = (0..5000)
        .map(|i| {
            let (col, row) = (f64::from(i % 100), f64::from(i / 100));
            rect(col, row, col + 0.5, row + 0.5)
        })
        .collect();
    let tiny = [squares, single(rect(-1.0, -1.0, 50.0, 25.0))];
    got += &timed("ac40i", || weld_closed(&tiny));
    got += &timed("ac09_line_and_square", weld_line_and_square);
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/weld/weld.golden");
        std::fs::write(path, &got).unwrap();
        return;
    }
    assert_eq!(got, normalise(GOLDEN));
}
