//! Tester's white-box edges of `specs/0068-laserable-lettering` on the Weld kernel: at g = 0 Weld
//! is Union node for node (criteria 18, 30), the order of operands does not matter (criterion
//! 29), the result holds every operand (area of the weld at least the largest operand's), a
//! hairline stroke band is empty rather than refused, and a gap that is not a number or negative
//! counts as zero.

#![allow(clippy::unwrap_used, missing_docs)]

use curvyo_document_core::{Length, LineCap, LineJoin, Point, Tolerance, Vec2};
use curvyo_geometry_core::{
    BooleanOp, Outline, StrokeBand, WeldOperand, boolean, signed_area_mm2, weld,
};
use proptest::prelude::*;

type Triple = (Point, Vec2, Vec2);

const TOLERANCE: Tolerance = Tolerance::from_mm(0.01);

fn corners(points: &[(f64, f64)]) -> Vec<Triple> {
    points
        .iter()
        .map(|&(x, y)| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO))
        .collect()
}

fn shape(x: f64, y: f64, w: f64, h: f64, triangle: bool) -> Vec<Triple> {
    if triangle {
        corners(&[(x, y), (x + w, y), (x + w * 0.3, y + h)])
    } else {
        corners(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h)])
    }
}

fn weld_closed(shapes: &[Vec<Triple>], gap_mm: f64) -> curvyo_geometry_core::Weld {
    let outlines: Vec<[Outline<'_>; 1]> = shapes.iter().map(|s| [Outline::new(s, true)]).collect();
    let operands: Vec<WeldOperand<'_>> = outlines
        .iter()
        .map(|o| WeldOperand {
            regions: o,
            banded: &[],
            band: None,
        })
        .collect();
    weld(&operands, Length::from_mm(gap_mm), TOLERANCE).unwrap()
}

fn net_area(outlines: &[Vec<Point>]) -> f64 {
    outlines
        .iter()
        .map(|o| signed_area_mm2(o))
        .sum::<f64>()
        .abs()
}

fn shapes_strategy() -> impl Strategy<Value = Vec<Vec<Triple>>> {
    prop::collection::vec(
        (
            0.0..30.0_f64,
            0.0..30.0_f64,
            1.0..15.0_f64,
            1.0..15.0_f64,
            any::<bool>(),
        ),
        1..6,
    )
    .prop_map(|v| {
        v.into_iter()
            .map(|(x, y, w, h, t)| shape(x, y, w, h, t))
            .collect()
    })
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        rng_seed: proptest::test_runner::RngSeed::Fixed(0x0068),
        ..ProptestConfig::default()
    })]

    /// Criteria 18 and 30: at g = 0, Weld of closed shapes is Union's result node for node.
    #[test]
    fn ac18_weld_at_zero_gap_is_union_node_for_node(shapes in shapes_strategy()) {
        let welded = weld_closed(&shapes, 0.0);
        let outlines: Vec<[Outline<'_>; 1]> =
            shapes.iter().map(|s| [Outline::new(s, true)]).collect();
        let operands: Vec<&[Outline<'_>]> = outlines.iter().map(|o| &o[..]).collect();
        let union = boolean(BooleanOp::Union, &operands, TOLERANCE).unwrap();
        prop_assert_eq!(welded.result, union);
    }

    /// Criterion 29: the order of the operands does not change the result; the result is never
    /// smaller than the largest operand.
    #[test]
    fn ac29_order_does_not_matter_and_every_operand_is_inside(
        shapes in shapes_strategy(),
        gap in prop::sample::select(vec![0.0, 0.05, 0.1, 0.5, 1.0]),
    ) {
        let forward = weld_closed(&shapes, gap);
        let mut reversed = shapes.clone();
        reversed.reverse();
        let backward = weld_closed(&reversed, gap);
        prop_assert_eq!(&forward, &backward);
        let largest = shapes
            .iter()
            .map(|s| {
                let points: Vec<Point> = s.iter().map(|t| t.0).collect();
                signed_area_mm2(&points).abs()
            })
            .fold(0.0, f64::max);
        prop_assert!(net_area(forward.result.outlines()) >= largest - 0.01);
    }
}

/// A stroke thinner than one grid pitch adds nothing; it does not refuse the weld.
#[test]
fn a_hairline_band_is_empty_not_refused() {
    let line = corners(&[(0.0, 0.0), (40.0, 0.0)]);
    let square = shape(15.0, -5.0, 10.0, 10.0, false);
    let open = [Outline::new(&line, false)];
    let closed = [Outline::new(&square, true)];
    for width in [0.0001, 0.0004] {
        let operands = [
            WeldOperand {
                regions: &[],
                banded: &open,
                band: Some(StrokeBand {
                    width: Length::from_mm(width),
                    join: LineJoin::Miter,
                    cap: LineCap::Butt,
                }),
            },
            WeldOperand {
                regions: &closed,
                banded: &[],
                band: None,
            },
        ];
        let welded = weld(&operands, Length::from_mm(0.0), TOLERANCE);
        let welded = welded.unwrap_or_else(|e| panic!("width {width}: {e:?}"));
        assert!((net_area(welded.result.outlines()) - 100.0).abs() < 0.01);
    }
}

/// A gap that is negative or not a number counts as zero (the kernel's documented contract).
#[test]
fn a_negative_or_nan_gap_counts_as_zero() {
    let corner = [
        shape(0.0, 0.0, 20.0, 20.0, false),
        shape(20.0, 20.0, 20.0, 20.0, false),
    ];
    let zero = weld_closed(&corner, 0.0);
    assert_eq!(zero.pieces, 2);
    for gap in [-1.0, f64::NAN, f64::NEG_INFINITY] {
        assert_eq!(weld_closed(&corner, gap), zero, "gap {gap}");
    }
}
