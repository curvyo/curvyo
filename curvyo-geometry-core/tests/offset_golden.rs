//! The golden results of the offset kernel (`specs/0038-path-offset` criterion 44): a fixed set of
//! cases (curves, holes, every join, the curved paths of the criterion) rendered as nodes
//! and handles in integer nanometres (1e-6 mm) and compared byte for byte with
//! `fixtures/offset/cases.golden`, natively and in Node on `wasm32-unknown-unknown`, so a result
//! that differs between targets by 1e-6 mm or in its segment count fails:
//!
//! ```text
//! cargo test --target wasm32-unknown-unknown -p curvyo-geometry-core --test offset_golden
//! ```
//!
//! To regenerate after an intended change, run natively with `CURVYO_UPDATE_GOLDEN=1` and review
//! the diff.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

mod common;

use std::fmt::Write as _;

use common::{Anchors, KERNEL_TOLERANCE, circle, ellipse, polygon, rect, rounded_rect, wavy};
use curvyo_document_core::Length;
use curvyo_geometry_core::{OffsetJoin, Outline, offset};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const GOLDEN: &str = include_str!("fixtures/offset/cases.golden");

fn reversed(mut anchors: Anchors) -> Anchors {
    anchors.reverse();
    for anchor in &mut anchors {
        std::mem::swap(&mut anchor.1, &mut anchor.2);
    }
    anchors
}

/// `(name, outlines, closed, distance, join)`.
type Case = (&'static str, Vec<Anchors>, bool, f64, OffsetJoin);

#[allow(clippy::too_many_lines)] // a flat table of cases
fn cases() -> Vec<Case> {
    let l_shape = polygon(&[
        (0.0, 0.0),
        (30.0, 0.0),
        (30.0, 10.0),
        (10.0, 10.0),
        (10.0, 30.0),
        (0.0, 30.0),
    ]);
    let ring = vec![circle(0.0, 0.0, 20.0), reversed(circle(0.0, 0.0, 10.0))];
    vec![
        (
            "circle path round outset (criterion 35)",
            vec![circle(0.0, 0.0, 20.0)],
            true,
            2.0,
            OffsetJoin::Round,
        ),
        (
            "ellipse path round outset (criterion 37)",
            vec![ellipse(0.0, 0.0, 20.0, 10.0)],
            true,
            2.0,
            OffsetJoin::Round,
        ),
        (
            "ellipse path round inset (criterion 37)",
            vec![ellipse(0.0, 0.0, 20.0, 10.0)],
            true,
            -2.0,
            OffsetJoin::Round,
        ),
        (
            "rounded rectangle path round outset (criterion 36)",
            vec![rounded_rect(0.0, 0.0, 100.0, 50.0, 5.0)],
            true,
            2.0,
            OffsetJoin::Round,
        ),
        (
            "gear round outset (criterion 38)",
            vec![wavy(200, 0.0, 0.0)],
            true,
            2.0,
            OffsetJoin::Round,
        ),
        (
            "square miter outset",
            vec![rect(0.0, 0.0, 20.0, 20.0)],
            true,
            5.0,
            OffsetJoin::Miter,
        ),
        (
            "square round outset",
            vec![rect(0.0, 0.0, 20.0, 20.0)],
            true,
            5.0,
            OffsetJoin::Round,
        ),
        (
            "l bevel inset",
            vec![l_shape.clone()],
            true,
            -2.0,
            OffsetJoin::Bevel,
        ),
        (
            "l round inset",
            vec![l_shape],
            true,
            -2.0,
            OffsetJoin::Round,
        ),
        (
            "ring round outset",
            ring.clone(),
            true,
            2.0,
            OffsetJoin::Round,
        ),
        ("ring miter inset", ring, true, -2.0, OffsetJoin::Miter),
    ]
}

fn render() -> String {
    let mut text = String::new();
    for (name, outlines, closed, d, join) in cases() {
        let borrowed: Vec<Outline<'_>> = outlines.iter().map(|a| Outline::new(a, closed)).collect();
        let result = offset(&borrowed, Length::from_mm(d), join, KERNEL_TOLERANCE);
        writeln!(text, "# {name}").unwrap();
        match result {
            Err(error) => writeln!(text, "error {error:?}").unwrap(),
            Ok(result) => {
                let nm = |v: f64| (v * 1.0e6).round() as i64;
                for outline in result.outlines() {
                    let nodes: Vec<String> = outline
                        .iter()
                        .map(|a| {
                            format!(
                                "{},{};{},{};{},{}{}",
                                nm(a.point.x),
                                nm(a.point.y),
                                nm(a.handle_in.x),
                                nm(a.handle_in.y),
                                nm(a.handle_out.x),
                                nm(a.handle_out.y),
                                if a.smooth { "s" } else { "" }
                            )
                        })
                        .collect();
                    writeln!(text, "{}", nodes.join(" ")).unwrap();
                }
            }
        }
    }
    text
}

#[test]
fn every_case_matches_its_golden_result() {
    let text = render();
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/offset/cases.golden");
        std::fs::write(path, &text).unwrap();
        return;
    }
    assert_eq!(
        text,
        GOLDEN.replace("\r\n", "\n"),
        "the offset results differ from the golden file"
    );
}
