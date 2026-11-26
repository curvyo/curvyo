//! Goldens of `specs/0156-fused-lettering` criteria 16 and 18 (`adrs.md` decision 4): Bridge on
//! the i of criterion 8, a geometric ü, the bars of criterion 9, the criterion 16 cases (a
//! straight dot-to-stem bridge and a curved one between a vertical and a horizontal stroke) and
//! the stencil O of criterion 24. Each result is summarised (outlines, nodes, area, a hash of the
//! grid coordinates of the Outline and of every connector) in `fixtures/fuse/fuse.golden`,
//! embedded so that the same test runs in the wasm build:
//!
//! ```text
//! cargo test --target wasm32-unknown-unknown -p curvyo-geometry-core --test fuse_golden
//! ```
//!
//! Regenerate with `CURVYO_UPDATE_GOLDEN=1` (native) and review the diff.

#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::format_push_string,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

mod common;

use common::golden::normalise;
use common::{KERNEL_TOLERANCE, Operand, circle, rect, single};
use curvyo_document_core::{Length, Point};
use curvyo_geometry_core::{
    FuseError, Fused, Outline, fuse_cut_out, fuse_stencil, signed_area_mm2,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const GOLDEN: &str = include_str!("fixtures/fuse/fuse.golden");

const WIDTH: Length = Length::from_mm(1.0);

fn borrowed(operands: &[Operand]) -> Vec<Vec<Outline<'_>>> {
    operands
        .iter()
        .map(|operand| operand.iter().map(|a| Outline::new(a, true)).collect())
        .collect()
}

fn cut_out(operands: &[Operand]) -> Result<Fused, FuseError> {
    let outlines = borrowed(operands);
    let slices: Vec<&[Outline<'_>]> = outlines.iter().map(Vec::as_slice).collect();
    fuse_cut_out(
        &slices,
        WIDTH,
        Some(Length::from_mm(5.0)),
        2000,
        KERNEL_TOLERANCE,
    )
}

fn stencil(operands: &[Operand]) -> Result<Fused, FuseError> {
    let outlines = borrowed(operands);
    let slices: Vec<&[Outline<'_>]> = outlines.iter().map(Vec::as_slice).collect();
    fuse_stencil(&slices, WIDTH, 2, 2000, KERNEL_TOLERANCE)
}

fn hash(outlines: &[Vec<Point>]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for outline in outlines {
        hash = (hash ^ 0xff).wrapping_mul(0x0100_0000_01b3);
        for p in outline {
            for v in [p.x, p.y] {
                hash = (hash ^ ((v * 1000.0).round() as i64 as u64)).wrapping_mul(0x0100_0000_01b3);
            }
        }
    }
    hash
}

fn summary(name: &str, result: &Result<Fused, FuseError>) -> String {
    let fused = match result {
        Ok(fused) => fused,
        Err(error) => return format!("{name}: refused {error:?}\n"),
    };
    let nodes: usize = fused.outline.iter().map(Vec::len).sum();
    let area: f64 = fused.outline.iter().map(|o| signed_area_mm2(o)).sum();
    let mut line = format!(
        "{name}: outlines {} nodes {nodes} area_mm2 {area:.3} hash {:016x} bridges {}",
        fused.outline.len(),
        hash(&fused.outline),
        fused.connectors.len()
    );
    for connector in &fused.connectors {
        let nodes: usize = connector.iter().map(Vec::len).sum();
        line += &format!(" [{nodes} {:016x}]", hash(connector));
    }
    line + "\n"
}

fn letter_u_umlaut() -> Vec<Operand> {
    vec![
        vec![
            rect(0.0, 4.0, 2.0, 12.0),
            rect(6.0, 4.0, 8.0, 12.0),
            rect(0.0, 10.0, 8.0, 12.0),
        ],
        single(circle(1.0, 1.5, 1.0)),
        single(circle(7.0, 1.5, 1.0)),
    ]
}

/// Criteria 16 and 18: every case gives its golden result.
#[test]
fn the_bridge_cases_give_their_golden_results() {
    let mut got = String::from(
        "# Bridge (specs/0156-fused-lettering criteria 16 and 18).\n# Regenerate with CURVYO_UPDATE_GOLDEN=1.\n",
    );
    let i = vec![
        single(rect(0.0, 0.0, 2.0, 10.0)),
        single(circle(1.0, 13.0, 1.0)),
    ];
    got += &summary("i", &cut_out(&i));
    got += &summary("u_umlaut", &cut_out(&letter_u_umlaut()));
    let bars = vec![
        single(rect(0.0, 0.0, 2.0, 10.0)),
        single(rect(5.0, 3.0, 7.0, 13.0)),
    ];
    got += &summary("bars", &cut_out(&bars));
    // Criterion 16d: a vertical stroke and a horizontal one, set diagonally apart.
    let corner = vec![
        single(rect(0.0, 0.0, 2.0, 10.0)),
        single(rect(4.0, 12.0, 14.0, 14.0)),
    ];
    got += &summary("vertical_horizontal", &cut_out(&corner));
    let mut hole = circle(0.0, 0.0, 6.0);
    hole.reverse();
    for (_, a, b) in &mut hole {
        std::mem::swap(a, b);
    }
    let o = vec![vec![circle(0.0, 0.0, 10.0), hole]];
    got += &summary("stencil_o", &stencil(&o));
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/fuse/fuse.golden");
        std::fs::write(path, &got).unwrap();
        return;
    }
    assert_eq!(got, normalise(GOLDEN));
}
