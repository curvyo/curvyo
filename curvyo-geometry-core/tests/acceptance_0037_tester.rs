//! Independent kernel acceptance tests for `0037-fracture-and-flatten` criterion 15 (result
//! geometry of curved operands), written from the specification before the implementation was
//! read: every node within 0.01 mm of an operand's true outline, chords within 0.01 mm, no two
//! consecutive nodes closer than 0.001 mm, no node on the line between its neighbours, the node
//! budget, and no extra nodes left by the fold on straight edges.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    missing_docs
)]

mod common;

use common::{KERNEL_TOLERANCE, Operand, circle, rect};
use curvyo_document_core::Point;
use curvyo_geometry_core::{Outline, Visible, fracture_cells, visible_regions};

fn with_operands<T>(operands: &[Operand], run: impl FnOnce(&[&[Outline<'_>]]) -> T) -> T {
    let outlines: Vec<Vec<Outline<'_>>> = operands
        .iter()
        .map(|o| o.iter().map(|a| Outline::new(a, true)).collect())
        .collect();
    let borrowed: Vec<&[Outline<'_>]> = outlines.iter().map(Vec::as_slice).collect();
    run(&borrowed)
}

/// All outlines of every Fracture piece and every trimmed Flatten result.
type Pieces = Vec<Vec<Vec<Point>>>;

fn results(operands: &[Operand]) -> (Pieces, Pieces) {
    let fracture = with_operands(operands, |o| fracture_cells(o, KERNEL_TOLERANCE, 5_000)).unwrap();
    let flatten = with_operands(operands, |o| visible_regions(o, KERNEL_TOLERANCE)).unwrap();
    (
        fracture.pieces.into_iter().map(|(_, p)| p).collect(),
        flatten
            .into_iter()
            .filter_map(|v| match v {
                Visible::Trimmed(p) => Some(p),
                _ => None,
            })
            .collect(),
    )
}

fn distance(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// Distance of `p` from the line through `a` and `b`.
fn off_line(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    ((p.x - a.x) * dy - (p.y - a.y) * dx).abs() / dx.hypot(dy)
}

/// `0016` criterion 24: no two consecutive nodes closer than 0.001 mm, no node within 0.001 mm
/// of the line between its neighbours, at least three nodes.
fn assert_clean(outline: &[Point]) {
    let n = outline.len();
    assert!(n >= 3);
    for i in 0..n {
        let (prev, here, next) = (outline[(i + n - 1) % n], outline[i], outline[(i + 1) % n]);
        assert!(distance(here, next) >= 0.001, "nodes too close at {here:?}");
        assert!(
            off_line(here, prev, next) > 0.001,
            "collinear node at {here:?}"
        );
    }
}

/// Criterion 15: two discs of radius 10 mm, 5 mm apart. Every node and every chord midpoint lies
/// within 0.01 mm of one of the two circles; the outlines are clean; the node budget holds.
#[test]
fn c15_curved_operands_give_clean_outlines_within_tolerance() {
    let centres = [(0.0, 0.0), (5.0, 0.0)];
    let operands: Vec<Operand> = centres
        .iter()
        .map(|&(x, y)| vec![circle(x, y, 10.0)])
        .collect();
    let on_a_circle = |p: Point| {
        centres
            .iter()
            .any(|&(x, y)| (distance(p, Point::new(x, y)) - 10.0).abs() <= 0.01)
    };
    let (fracture, flatten) = results(&operands);
    assert_eq!(fracture.len(), 3);
    assert_eq!(flatten.len(), 1);
    for piece in fracture.iter().chain(&flatten) {
        for outline in piece {
            assert_clean(outline);
            let n = outline.len();
            for i in 0..n {
                let (a, b) = (outline[i], outline[(i + 1) % n]);
                assert!(on_a_circle(a), "node {a:?} off both circles");
                let mid = Point::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y));
                assert!(on_a_circle(mid), "chord {a:?}-{b:?} deviates");
            }
        }
    }
    // Node budget (`0016` criterion 26: at most 142 nodes for a full disc of radius 10 mm). Each
    // arc of the cut is in at most two pieces, so all pieces together hold at most four discs.
    let nodes: usize = fracture.iter().flatten().map(Vec::len).sum();
    assert!(nodes <= 4 * 142, "{nodes} nodes");
    let trimmed: usize = flatten.iter().flatten().map(Vec::len).sum();
    assert!(trimmed <= 2 * 142, "{trimmed} nodes");
}

/// Criterion 15 on a deep stack: 25 discs in a chain, each covered by up to seven others. The
/// fold must not leave drift: still clean, still within 0.01 mm of some disc.
#[test]
fn c15_a_deep_stack_of_discs_stays_clean_and_within_tolerance() {
    let centres: Vec<(f64, f64)> = (0..25).map(|i| (3.0 * f64::from(i), 0.0)).collect();
    let operands: Vec<Operand> = centres
        .iter()
        .map(|&(x, y)| vec![circle(x, y, 10.0)])
        .collect();
    let (fracture, flatten) = results(&operands);
    assert!(fracture.len() > 25, "{} pieces", fracture.len());
    for outline in fracture.iter().chain(&flatten).flatten() {
        assert_clean(outline);
        for &p in outline {
            assert!(
                centres
                    .iter()
                    .any(|&(x, y)| (distance(p, Point::new(x, y)) - 10.0).abs() <= 0.01),
                "node {p:?} off every circle"
            );
        }
    }
}

/// Criterion 15 with criterion 17 (a): the fold leaves no node on a straight edge, so every
/// column of the chain of 20 squares is exactly a rectangle of 4 nodes.
#[test]
fn c15_pieces_of_a_chain_of_squares_have_four_nodes() {
    let operands: Vec<Operand> = (0..20)
        .map(|i| {
            let x = 5.0 * f64::from(i);
            vec![rect(x, 0.0, x + 10.0, 10.0)]
        })
        .collect();
    let (fracture, flatten) = results(&operands);
    assert_eq!(fracture.len(), 21);
    assert_eq!(flatten.len(), 19);
    for piece in fracture.iter().chain(&flatten) {
        assert_eq!(piece.len(), 1);
        assert_eq!(piece[0].len(), 4, "{:?}", piece[0]);
    }
}
