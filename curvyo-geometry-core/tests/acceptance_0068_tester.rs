//! Independent kernel acceptance tests for `0068-laserable-lettering` (Weld), written from the
//! specification and the public signature of `weld` before the implementation was read:
//! criterion 4 (140 mm against Flatten's 160 mm), 5 and 9 (hidden shapes removed, a disc that
//! fills a ring's hole kept), 14 (no line doubled, property test with a fixed seed) and 27 (d)
//! (the 20 squares give 420 mm).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::assert_is_empty,
    missing_docs
)]

mod common;

use common::{Anchors, KERNEL_TOLERANCE, Operand, circle, rect};
use curvyo_document_core::Point;
use curvyo_geometry_core::{Outline, Visible, Welded, visible_regions, weld};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};

fn with_operands<T>(operands: &[Operand], run: impl FnOnce(&[&[Outline<'_>]]) -> T) -> T {
    let outlines: Vec<Vec<Outline<'_>>> = operands
        .iter()
        .map(|o| o.iter().map(|a| Outline::new(a, true)).collect())
        .collect();
    let borrowed: Vec<&[Outline<'_>]> = outlines.iter().map(Vec::as_slice).collect();
    run(&borrowed)
}

fn welded(operands: &[Operand]) -> Welded {
    with_operands(operands, |o| weld(o, KERNEL_TOLERANCE)).unwrap()
}

fn distance(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// The segments of an outline, the closing one included when it is closed.
fn segments(points: &[Point], closed: bool) -> Vec<(Point, Point)> {
    let mut s: Vec<(Point, Point)> = points.windows(2).map(|w| (w[0], w[1])).collect();
    if closed && points.len() > 1 {
        s.push((points[points.len() - 1], points[0]));
    }
    s
}

fn line_length(w: &Welded) -> f64 {
    w.outlines
        .iter()
        .flat_map(|(p, c)| segments(p, *c))
        .map(|(a, b)| distance(a, b))
        .sum()
}

fn shoelace(points: &[Point]) -> f64 {
    segments(points, true)
        .iter()
        .map(|(a, b)| a.x * b.y - b.x * a.y)
        .sum::<f64>()
        / 2.0
}

/// A closed outline reversed (a hole under the nonzero rule), handles swapped.
fn reversed(a: &Anchors) -> Anchors {
    a.iter()
        .rev()
        .map(|&(p, hin, hout)| (p, hout, hin))
        .collect()
}

/// Criterion 14: the first pair of segments that lie within `within` mm of each other over more
/// than `within` mm of length, if any.
fn doubled(w: &Welded, within: f64) -> Option<((Point, Point), (Point, Point))> {
    let all: Vec<(Point, Point)> = w
        .outlines
        .iter()
        .flat_map(|(p, c)| segments(p, *c))
        .collect();
    for (i, &(a0, a1)) in all.iter().enumerate() {
        let length = distance(a0, a1);
        if length <= within {
            continue;
        }
        let (ux, uy) = ((a1.x - a0.x) / length, (a1.y - a0.y) / length);
        for &(b0, b1) in &all[i + 1..] {
            // Box check first: B must come within `within` of A's box.
            if b0.x.min(b1.x) > a0.x.max(a1.x) + within
                || b0.x.max(b1.x) < a0.x.min(a1.x) - within
                || b0.y.min(b1.y) > a0.y.max(a1.y) + within
                || b0.y.max(b1.y) < a0.y.min(a1.y) - within
            {
                continue;
            }
            let along = |p: Point| (p.x - a0.x) * ux + (p.y - a0.y) * uy;
            let across = |p: Point| (p.x - a0.x) * -uy + (p.y - a0.y) * ux;
            let (s0, s1, d0, d1) = (along(b0), along(b1), across(b0), across(b1));
            if (s1 - s0).abs() <= f64::EPSILON {
                continue;
            }
            let lo = s0.min(s1).max(0.0);
            let hi = s0.max(s1).min(length);
            if hi - lo <= within {
                continue;
            }
            let at = |s: f64| d0 + (d1 - d0) * (s - s0) / (s1 - s0);
            if at(lo).abs() <= within && at(hi).abs() <= within {
                return Some(((a0, a1), (b0, b1)));
            }
        }
    }
    None
}

/// Criterion 4: the lower square (0, 0) to (20, 20), the upper (10, 10) to (30, 30). One closed
/// outline of 8 nodes enclosing 700 mm² and one open outline (10, 20), (10, 10), (20, 10); 140 mm
/// of line, where Flatten's two objects hold 160 mm.
#[test]
fn ac04_two_offset_squares_give_140_mm_against_flattens_160() {
    let operands = vec![
        vec![rect(0.0, 0.0, 20.0, 20.0)],
        vec![rect(10.0, 10.0, 30.0, 30.0)],
    ];
    let w = welded(&operands);
    assert!(w.hidden.is_empty());
    let closed: Vec<&Vec<Point>> = w.outlines.iter().filter(|o| o.1).map(|o| &o.0).collect();
    let open: Vec<&Vec<Point>> = w.outlines.iter().filter(|o| !o.1).map(|o| &o.0).collect();
    assert_eq!((closed.len(), open.len()), (1, 1), "{:?}", w.outlines);
    assert_eq!(closed[0].len(), 8);
    assert!((shoelace(closed[0]).abs() - 700.0).abs() < 1e-6);
    let line: Vec<(f64, f64)> = open[0].iter().map(|p| (p.x, p.y)).collect();
    let mut expected = vec![(10.0, 20.0), (10.0, 10.0), (20.0, 10.0)];
    if line.first() != expected.first() {
        expected.reverse();
    }
    assert_eq!(line.len(), 3);
    for (got, want) in line.iter().zip(&expected) {
        assert!(distance(Point::new(got.0, got.1), Point::new(want.0, want.1)) < 1e-6);
    }
    assert!(
        (line_length(&w) - 140.0).abs() < 1e-6,
        "{}",
        line_length(&w)
    );

    // Flatten of the same pair: the 20 mm trim edge is in both objects.
    let flatten: f64 = with_operands(&operands, |o| visible_regions(o, KERNEL_TOLERANCE))
        .unwrap()
        .into_iter()
        .map(|v| match v {
            Visible::Trimmed(outlines) => outlines
                .iter()
                .flat_map(|p| segments(p, true))
                .map(|(a, b)| distance(a, b))
                .sum::<f64>(),
            Visible::Untouched => 80.0,
            Visible::Hidden => 0.0,
        })
        .sum();
    assert!((flatten - 160.0).abs() < 1e-6, "Flatten {flatten}");
}

/// Criterion 27 (d): 20 squares of side 10 mm at x = 5 i, the highest index on top. One closed
/// 105 x 10 mm outline and 19 open 10 mm lines, 420 mm in total, nothing doubled.
#[test]
fn ac27d_twenty_squares_give_420_mm() {
    let operands: Vec<Operand> = (0..20)
        .map(|i| {
            let x = 5.0 * f64::from(i);
            vec![rect(x, 0.0, x + 10.0, 10.0)]
        })
        .collect();
    let w = welded(&operands);
    assert!(w.hidden.is_empty());
    let closed: Vec<&Vec<Point>> = w.outlines.iter().filter(|o| o.1).map(|o| &o.0).collect();
    let open: Vec<&Vec<Point>> = w.outlines.iter().filter(|o| !o.1).map(|o| &o.0).collect();
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].len(), 4, "a clean 105 x 10 rectangle");
    assert!((shoelace(closed[0]).abs() - 1050.0).abs() < 1e-6);
    assert_eq!(open.len(), 19);
    for line in &open {
        let l: f64 = segments(line, false)
            .iter()
            .map(|(a, b)| distance(*a, *b))
            .sum();
        assert!((l - 10.0).abs() < 1e-6, "an open line of {l} mm");
    }
    assert!(
        (line_length(&w) - 420.0).abs() < 1e-6,
        "{}",
        line_length(&w)
    );
    assert_eq!(doubled(&w, 0.002), None);
}

/// Criterion 5: a small disc below a large concentric disc is hidden and removed: one closed
/// outline of radius 20.
#[test]
fn ac05_a_hidden_disc_is_removed() {
    let w = welded(&[vec![circle(0.0, 0.0, 10.0)], vec![circle(0.0, 0.0, 20.0)]]);
    assert_eq!(w.hidden, vec![0]);
    assert_eq!(w.outlines.len(), 1);
    assert!(w.outlines[0].1);
    for p in &w.outlines[0].0 {
        assert!((distance(*p, Point::new(0.0, 0.0)) - 20.0).abs() < 0.011);
    }
}

/// Criteria 5, 9 and 15: a disc of radius 10 that fills the hole of a ring (20 minus 10) exactly,
/// below the ring and above it. The disc is not hidden; the union has no hole, so the result is
/// the closed circle of radius 20 and the circle of radius 10 once, closed and wound as a shape.
#[test]
fn ac05_ac09_a_disc_that_fills_a_rings_hole_is_kept() {
    let ring: Operand = vec![circle(0.0, 0.0, 20.0), reversed(&circle(0.0, 0.0, 10.0))];
    let disc: Operand = vec![circle(0.0, 0.0, 10.0)];
    for (name, operands) in [
        ("disc below", vec![disc.clone(), ring.clone()]),
        ("disc above", vec![ring.clone(), disc.clone()]),
    ] {
        let w = welded(&operands);
        assert!(w.hidden.is_empty(), "{name}: nothing hidden");
        assert_eq!(w.outlines.len(), 2, "{name}: {:?}", w.outlines.len());
        assert!(w.outlines.iter().all(|o| o.1), "{name}: both closed");
        let radii: Vec<f64> = w
            .outlines
            .iter()
            .map(|o| distance(o.0[0], Point::new(0.0, 0.0)))
            .collect();
        assert!(radii.iter().any(|r| (r - 20.0).abs() < 0.011), "{name}");
        assert!(radii.iter().any(|r| (r - 10.0).abs() < 0.011), "{name}");
        let areas: Vec<f64> = w.outlines.iter().map(|o| shoelace(&o.0)).collect();
        assert!(
            areas[0].signum() == areas[1].signum(),
            "{name}: the inner circle is wound as a shape, {areas:?}"
        );
        let expected = std::f64::consts::TAU * 30.0;
        assert!(
            (line_length(&w) - expected).abs() < 0.2,
            "{name}: {}",
            line_length(&w)
        );
        assert_eq!(doubled(&w, 0.002), None, "{name}");
    }
}

/// One random operand: a rectangle or a disc on a 0.5 mm grid, so that shared and coincident
/// edges come up often.
fn shape() -> impl Strategy<Value = Anchors> {
    let coordinate = (0_i32..60).prop_map(|v| f64::from(v) * 0.5);
    let size = (2_i32..40).prop_map(|v| f64::from(v) * 0.5);
    prop_oneof![
        (
            coordinate.clone(),
            coordinate.clone(),
            size.clone(),
            size.clone()
        )
            .prop_map(|(x, y, w, h)| rect(x, y, x + w, y + h)),
        (coordinate.clone(), coordinate, size).prop_map(|(x, y, r)| circle(x, y, r / 2.0)),
    ]
}

/// Criterion 14: on 200 random shape sets with a fixed seed, no two segments of the result lie
/// within 0.002 mm of each other over more than 0.002 mm (two grid units: lines closer than one
/// grid unit are coincident, Terms), and no outline is degenerate. Every failing set is counted.
#[test]
fn ac14_no_line_is_doubled_on_200_random_sets() {
    let mut runner = TestRunner::new_with_rng(
        Config::default(),
        TestRng::deterministic_rng(RngAlgorithm::ChaCha),
    );
    let strategy = prop::collection::vec(shape(), 2..7);
    let mut failures: Vec<String> = Vec::new();
    for case in 0..200 {
        let set = strategy.new_tree(&mut runner).unwrap().current();
        let operands: Vec<Operand> = set.into_iter().map(|a| vec![a]).collect();
        let w = welded(&operands);
        if let Some(pair) = doubled(&w, 0.002) {
            failures.push(format!("case {case}: doubled {pair:?}"));
        }
        for (points, closed) in &w.outlines {
            let min = if *closed { 3 } else { 2 };
            if points.len() < min || !points.iter().all(|p| p.x.is_finite() && p.y.is_finite()) {
                failures.push(format!("case {case}: degenerate {points:?}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of 200 sets fail:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Criterion 14, the smallest set the property test found: a rectangle below a disc whose
/// outline crosses the rectangle's right edge at y = 10 - sqrt(18). The visible piece of that
/// edge, (31.5, 5.5) to (31.5, 5.757), must be in the result once.
#[test]
fn ac14_a_disc_crossing_a_rectangle_edge_leaves_it_once() {
    let w = welded(&[
        vec![rect(22.5, 5.5, 31.5, 9.0)],
        vec![circle(28.0, 10.0, 5.5)],
    ]);
    assert_eq!(doubled(&w, 0.002), None, "{:?}", w.outlines);
}

/// Criterion 14, the other half on a case with hand-known lines: every point of a visible-region
/// boundary is on the result. Three squares in a row overlapping by half; each trim edge appears.
#[test]
fn ac14_every_visible_boundary_is_in_the_result() {
    let operands = vec![
        vec![rect(0.0, 0.0, 10.0, 10.0)],
        vec![rect(5.0, 2.0, 15.0, 8.0)],
        vec![rect(-3.0, 4.0, 3.0, 6.0)],
    ];
    let w = welded(&operands);
    let all: Vec<(Point, Point)> = w
        .outlines
        .iter()
        .flat_map(|(p, c)| segments(p, *c))
        .collect();
    let on_result = |p: Point| {
        all.iter().any(|&(a, b)| {
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            distance(p, Point::new(a.x + t * dx, a.y + t * dy)) <= 0.001
        })
    };
    // Visible boundaries: the middle square's left edge over the lower one (x = 5, y 2..8), the
    // top bar's right edge over the lower (x = 3, y 4..6), the bar's horizontal edges inside.
    for (x, y) in [
        (5.0, 2.5),
        (5.0, 7.5),
        (3.0, 5.0),
        (1.0, 4.0),
        (1.0, 6.0),
        (-3.0, 5.0),
    ] {
        assert!(on_result(Point::new(x, y)), "({x}, {y}) is missing");
    }
    // Hidden: the lower square's left edge under the bar, its right edge under the middle one.
    for (x, y) in [(0.0, 5.0), (10.0, 5.0)] {
        assert!(
            !on_result(Point::new(x, y)),
            "({x}, {y}) is hidden but kept"
        );
    }
    assert_eq!(doubled(&w, 0.002), None);
}
