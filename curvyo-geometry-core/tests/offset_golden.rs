//! The golden results of the offset kernel (`specs/0038-path-offset` criterion 12): a fixed set of
//! cases (curves, holes, every join and cap) rendered as integers on the 0.001 mm grid and compared
//! byte for byte with `fixtures/offset/cases.golden`, natively and in Node on
//! `wasm32-unknown-unknown`, so a result that differs between targets by one grid step fails:
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

use common::{Anchors, KERNEL_TOLERANCE, circle, polygon, rect};
use curvyo_document_core::Length;
use curvyo_geometry_core::{OffsetCap, OffsetJoin, Outline, offset};

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

/// `(name, outlines, closed, distance, join, cap)`.
type Case = (&'static str, Vec<Anchors>, bool, f64, OffsetJoin, OffsetCap);

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
    let mut open_curve = circle(0.0, 0.0, 5.0);
    open_curve.truncate(3);
    vec![
        (
            "square miter outset",
            vec![rect(0.0, 0.0, 20.0, 20.0)],
            true,
            5.0,
            OffsetJoin::Miter,
            OffsetCap::Round,
        ),
        (
            "square round outset",
            vec![rect(0.0, 0.0, 20.0, 20.0)],
            true,
            5.0,
            OffsetJoin::Round,
            OffsetCap::Round,
        ),
        (
            "l bevel inset",
            vec![l_shape.clone()],
            true,
            -2.0,
            OffsetJoin::Bevel,
            OffsetCap::Round,
        ),
        (
            "l round inset",
            vec![l_shape],
            true,
            -2.0,
            OffsetJoin::Round,
            OffsetCap::Round,
        ),
        (
            "ring round outset",
            ring.clone(),
            true,
            2.0,
            OffsetJoin::Round,
            OffsetCap::Round,
        ),
        (
            "ring miter inset",
            ring,
            true,
            -2.0,
            OffsetJoin::Miter,
            OffsetCap::Round,
        ),
        (
            "open arc round cap",
            vec![open_curve.clone()],
            false,
            1.5,
            OffsetJoin::Round,
            OffsetCap::Round,
        ),
        (
            "open arc square cap",
            vec![open_curve],
            false,
            1.5,
            OffsetJoin::Miter,
            OffsetCap::Square,
        ),
    ]
}

fn render() -> String {
    let mut text = String::new();
    for (name, outlines, closed, d, join, cap) in cases() {
        let borrowed: Vec<Outline<'_>> = outlines.iter().map(|a| Outline::new(a, closed)).collect();
        let result = offset(&borrowed, Length::from_mm(d), join, cap, KERNEL_TOLERANCE);
        writeln!(text, "# {name}").unwrap();
        match result {
            Err(error) => writeln!(text, "error {error:?}").unwrap(),
            Ok(result) => {
                for outline in result.outlines() {
                    let points: Vec<String> = outline
                        .iter()
                        .map(|p| {
                            format!(
                                "{},{}",
                                (p.x * 1000.0).round() as i64,
                                (p.y * 1000.0).round() as i64
                            )
                        })
                        .collect();
                    writeln!(text, "{}", points.join(" ")).unwrap();
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
