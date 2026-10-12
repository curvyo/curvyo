//! Tester's acceptance tests for `specs/0038-path-offset` on the kernel: criteria 6 to 9, 11, 12,
//! 14 (kernel part) and 15 to 20, written from the specification; plus edge cases (zero, tiny,
//! non-finite and huge distances, self-crossing operands, open paths that cross themselves) and
//! proptest invariants (outset contains the operand, inset lies inside it).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::float_cmp,
    clippy::many_single_char_names,
    missing_docs
)]

mod common;

use std::f64::consts::PI;

use common::{Anchors, KERNEL_TOLERANCE, area, check_invariants, circle, covers, polygon, rect};
use curvyo_document_core::{Length, Point};
use curvyo_geometry_core::{
    BooleanResult, OffsetCap, OffsetError, OffsetJoin, Outline, offset, signed_area_mm2,
};
use proptest::prelude::*;

const JOINS: [OffsetJoin; 3] = [OffsetJoin::Round, OffsetJoin::Miter, OffsetJoin::Bevel];

fn run(
    outlines: &[(Anchors, bool)],
    d: f64,
    join: OffsetJoin,
    cap: OffsetCap,
) -> Result<BooleanResult, OffsetError> {
    let borrowed: Vec<Outline<'_>> = outlines.iter().map(|(a, c)| Outline::new(a, *c)).collect();
    offset(&borrowed, Length::from_mm(d), join, cap, KERNEL_TOLERANCE)
}

fn closed(outlines: &[Anchors], d: f64, join: OffsetJoin) -> Result<BooleanResult, OffsetError> {
    let v: Vec<(Anchors, bool)> = outlines.iter().map(|a| (a.clone(), true)).collect();
    run(&v, d, join, OffsetCap::Round)
}

fn ok(outlines: &[Anchors], d: f64, join: OffsetJoin) -> BooleanResult {
    let r = closed(outlines, d, join).unwrap_or_else(|e| panic!("{d} {join:?}: {e}"));
    check_invariants(&r).unwrap_or_else(|e| panic!("{d} {join:?}: criterion 11: {e}"));
    r
}

fn line(points: &[(f64, f64)], d: f64, cap: OffsetCap) -> Result<BooleanResult, OffsetError> {
    run(&[(polygon(points), false)], d, OffsetJoin::Round, cap)
}

fn reversed(mut a: Anchors) -> Anchors {
    a.reverse();
    for x in &mut a {
        std::mem::swap(&mut x.1, &mut x.2);
    }
    a
}

fn bbox(r: &BooleanResult) -> (f64, f64, f64, f64) {
    let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in r.outlines().iter().flatten() {
        b = (b.0.min(p.x), b.1.min(p.y), b.2.max(p.x), b.3.max(p.y));
    }
    b
}

fn near(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64), tol: f64) -> bool {
    (a.0 - b.0).abs() <= tol
        && (a.1 - b.1).abs() <= tol
        && (a.2 - b.2).abs() <= tol
        && (a.3 - b.3).abs() <= tol
}

fn has_node(r: &BooleanResult, x: f64, y: f64, tol: f64) -> bool {
    r.outlines()
        .iter()
        .flatten()
        .any(|p| (p.x - x).hypot(p.y - y) <= tol)
}

fn nodes_within(r: &BooleanResult, c: Point, radius: f64) -> Vec<Point> {
    r.outlines()
        .iter()
        .flatten()
        .copied()
        .filter(|p| (p.x - c.x).hypot(p.y - c.y) <= radius)
        .collect()
}

fn ring() -> Vec<Anchors> {
    vec![circle(0.0, 0.0, 20.0), reversed(circle(0.0, 0.0, 10.0))]
}

// ---------------------------------------------------------------- criterion 15 -----------------

#[test]
fn ac15_square_outset_miter_round_bevel() {
    let sq = [rect(0.0, 0.0, 20.0, 20.0)];
    let m = ok(&sq, 5.0, OffsetJoin::Miter);
    assert_eq!(m.outlines().len(), 1);
    assert_eq!(m.outlines()[0].len(), 4, "miter square has 4 nodes");
    assert!(
        near(bbox(&m), (-5.0, -5.0, 25.0, 25.0), 1e-6),
        "{:?}",
        bbox(&m)
    );
    let r = ok(&sq, 5.0, OffsetJoin::Round);
    let want = 400.0 + 4.0 * 20.0 * 5.0 + PI * 25.0;
    assert!((area(&r) - want).abs() <= 0.3, "round area {}", area(&r));
    let b = ok(&sq, 5.0, OffsetJoin::Bevel);
    assert!((area(&b) - 850.0).abs() <= 0.01, "bevel area {}", area(&b));
}

#[test]
fn ac15_square_inset_every_join_and_the_collapse() {
    let sq = [rect(0.0, 0.0, 20.0, 20.0)];
    for join in JOINS {
        let r = ok(&sq, -5.0, join);
        assert!((area(&r) - 100.0).abs() <= 1e-6, "{join:?} {}", area(&r));
        assert!(near(bbox(&r), (5.0, 5.0, 15.0, 15.0), 1e-6));
        assert_eq!(
            closed(&sq, -10.0, join),
            Err(OffsetError::Empty),
            "{join:?}"
        );
        let t = ok(&sq, -9.99, join);
        assert!(
            near(bbox(&t), (9.99, 9.99, 10.01, 10.01), 1e-6),
            "{join:?} {:?}",
            bbox(&t)
        );
    }
}

// ---------------------------------------------------------------- criterion 16 -----------------

#[test]
fn ac16_ring_outset_inset_and_vanish() {
    let out = ok(&ring(), 2.0, OffsetJoin::Round);
    assert_eq!(out.outlines().len(), 2, "outer plus hole");
    assert!(
        (area(&out) - PI * (484.0 - 64.0)).abs() <= 3.0,
        "{}",
        area(&out)
    );
    assert!(!covers(&out, Point::new(0.0, 0.0)), "the hole stays a hole");
    assert!(!covers(&out, Point::new(7.9, 0.0)) && covers(&out, Point::new(8.1, 0.0)));
    assert!(covers(&out, Point::new(21.9, 0.0)) && !covers(&out, Point::new(22.1, 0.0)));
    for join in JOINS {
        let inn = ok(&ring(), -2.0, join);
        assert!(
            (area(&inn) - PI * (324.0 - 144.0)).abs() <= 3.0,
            "{join:?} {}",
            area(&inn)
        );
        assert!(!covers(&inn, Point::new(11.9, 0.0)) && covers(&inn, Point::new(12.1, 0.0)));
        assert!(covers(&inn, Point::new(17.9, 0.0)) && !covers(&inn, Point::new(18.1, 0.0)));
        assert_eq!(closed(&ring(), -5.0, join), Err(OffsetError::Empty));
    }
}

// ---------------------------------------------------------------- criterion 17 -----------------

#[test]
fn ac17_open_line_caps_and_inset_refusal() {
    let pts = [(0.0, 0.0), (20.0, 0.0)];
    let round = line(&pts, 5.0, OffsetCap::Round).unwrap();
    check_invariants(&round).unwrap();
    assert_eq!(round.outlines().len(), 1);
    assert!(
        (area(&round) - (200.0 + PI * 25.0)).abs() <= 0.3,
        "{}",
        area(&round)
    );
    let flat = line(&pts, 5.0, OffsetCap::Flat).unwrap();
    assert!((area(&flat) - 200.0).abs() <= 0.01, "{}", area(&flat));
    assert!(near(bbox(&flat), (0.0, -5.0, 20.0, 5.0), 1e-6));
    let square = line(&pts, 5.0, OffsetCap::Square).unwrap();
    assert!((area(&square) - 300.0).abs() <= 0.01, "{}", area(&square));
    assert!(near(bbox(&square), (-5.0, -5.0, 25.0, 5.0), 1e-6));
    for cap in [OffsetCap::Round, OffsetCap::Flat, OffsetCap::Square] {
        assert_eq!(line(&pts, -5.0, cap), Err(OffsetError::InsetOfOpenPath));
    }
}

// ---------------------------------------------------------------- criterion 18 -----------------

fn l_shape() -> Anchors {
    polygon(&[
        (0.0, 0.0),
        (20.0, 0.0),
        (20.0, 10.0),
        (10.0, 10.0),
        (10.0, 20.0),
        (0.0, 20.0),
    ])
}

#[test]
fn ac18_l_shape_outset_reflex_corner_stays_sharp() {
    for join in JOINS {
        let r = ok(&[l_shape()], 2.0, join);
        assert!(
            has_node(&r, 12.0, 12.0, 1e-6),
            "{join:?}: reflex corner sharp at (12,12)"
        );
        // Convex corner (20, 0): Miter gives the point (22, -2); Round and Bevel do not.
        assert_eq!(
            has_node(&r, 22.0, -2.0, 1e-6),
            join == OffsetJoin::Miter,
            "{join:?}"
        );
        if join == OffsetJoin::Round {
            for p in nodes_within(&r, Point::new(20.0, 0.0), 2.5) {
                let dist = (p.x - 20.0).hypot(p.y);
                assert!((dist - 2.0).abs() <= 0.011, "round arc node at {dist}");
            }
        }
    }
}

#[test]
fn ac18_l_shape_inset_reflex_corner_follows_join() {
    for join in JOINS {
        let r = ok(&[l_shape()], -2.0, join);
        assert!(
            has_node(&r, 2.0, 2.0, 1e-6),
            "{join:?}: convex corner sharp at (2,2)"
        );
        assert!(
            has_node(&r, 18.0, 2.0, 1e-6),
            "{join:?}: convex corner sharp at (18,2)"
        );
        assert_eq!(
            has_node(&r, 8.0, 8.0, 1e-6),
            join == OffsetJoin::Miter,
            "{join:?}"
        );
        if join != OffsetJoin::Miter {
            let around = nodes_within(&r, Point::new(10.0, 10.0), 2.5);
            assert!(
                around.len() >= 2,
                "{join:?}: the gap at the reflex corner is closed"
            );
            for p in around {
                let dist = (p.x - 10.0).hypot(p.y - 10.0);
                assert!(
                    (dist - 2.0).abs() <= 0.011,
                    "{join:?}: node {p:?} at {dist}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------- criterion 19 -----------------

#[test]
fn ac19_miter_limit_clips_a_20_degree_corner() {
    let a = 10f64.to_radians();
    let tri = polygon(&[
        (0.0, 0.0),
        (50.0 * a.cos(), -50.0 * a.sin()),
        (50.0 * a.cos(), 50.0 * a.sin()),
    ]);
    let r = ok(&[tri], 5.0, OffsetJoin::Miter);
    let o = Point::new(0.0, 0.0);
    let far: Vec<Point> = nodes_within(&r, o, 30.0);
    assert!(
        !far.iter()
            .any(|p| (p.x.hypot(p.y) - 5.0 / a.sin()).abs() < 0.5),
        "no 28.8 mm tip"
    );
    let clip: Vec<Point> = far.iter().copied().filter(|p| p.x < 0.0).collect();
    assert_eq!(clip.len(), 2, "two clip nodes at the sharp corner: {far:?}");
    for p in &clip {
        assert!(
            (p.x.hypot(p.y) - 20.0).abs() <= 0.01,
            "clip node {p:?} at {}",
            p.x.hypot(p.y)
        );
    }
    assert!(
        (clip[0].y + clip[1].y).abs() <= 0.002,
        "symmetric about the bisector"
    );
    assert!((clip[0].x - clip[1].x).abs() <= 0.002);
    // The 80-degree corners stay sharp: 2 + 1 + 1 = 4 nodes in all.
    assert_eq!(r.outlines()[0].len(), 4, "{:?}", r.outlines());
}

#[test]
fn ac19_square_miter_tips_at_5_sqrt2() {
    let r = ok(&[rect(0.0, 0.0, 20.0, 20.0)], 5.0, OffsetJoin::Miter);
    for p in &r.outlines()[0] {
        let d = [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)]
            .iter()
            .map(|c| (p.x - c.0).hypot(p.y - c.1))
            .fold(f64::MAX, f64::min);
        assert!((d - 50f64.sqrt()).abs() <= 1e-6);
    }
}

// ---------------------------------------------------------------- criterion 20 -----------------

#[test]
fn ac20_circle_outset_is_a_polyline_near_r22() {
    let c = circle(0.0, 0.0, 20.0);
    let before = c.clone();
    let r = ok(std::slice::from_ref(&c), 2.0, OffsetJoin::Round);
    assert_eq!(r.outlines().len(), 1);
    for p in &r.outlines()[0] {
        assert!(
            (p.x.hypot(p.y) - 22.0).abs() <= 0.02,
            "{p:?} at {}",
            p.x.hypot(p.y)
        );
    }
    assert!((area(&r) - PI * 484.0).abs() <= 3.0, "{}", area(&r));
    assert_eq!(c, before, "the operand is unchanged");
}

// ---------------------------------------------------------------- criterion 11 -----------------

/// Fewest segments that keep every chord of an arc of `angle` at radius `r` within 0.01 mm.
fn fewest(angle: f64, r: f64) -> usize {
    let step = 2.0 * (1.0 - 0.01 / r).acos();
    (angle / step).ceil() as usize
}

#[test]
fn ac11_round_join_node_budget_and_chord_error() {
    for d in [0.5, 2.0, 5.0, 40.0] {
        let r = ok(&[rect(0.0, 0.0, 20.0, 20.0)], d, OffsetJoin::Round);
        // Arc around the corner (20, 0): nodes strictly in the corner quadrant.
        let arc: Vec<Point> = r.outlines()[0]
            .iter()
            .copied()
            .filter(|p| p.x >= 20.0 - 1e-9 && p.y <= 1e-9)
            .collect();
        let segments = arc.len() - 1;
        let n = fewest(PI / 2.0, d);
        assert!(
            segments >= n && segments <= 2 * n,
            "d {d}: {segments} segments, fewest {n}"
        );
        for w in arc.windows(2) {
            let m = Point::new(f64::midpoint(w[0].x, w[1].x), f64::midpoint(w[0].y, w[1].y));
            let sag = d - (m.x - 20.0).hypot(m.y);
            assert!(sag <= 0.0105, "d {d}: chord error {sag}");
        }
    }
}

#[test]
fn ac11_round_cap_node_budget() {
    let r = line(&[(0.0, 0.0), (20.0, 0.0)], 5.0, OffsetCap::Round).unwrap();
    let cap: Vec<Point> = r.outlines()[0]
        .iter()
        .copied()
        .filter(|p| p.x >= 20.0 - 1e-9)
        .collect();
    let segments = cap.len() - 1;
    let n = fewest(PI, 5.0);
    assert!(
        segments >= n && segments <= 2 * n,
        "{segments} segments, fewest {n}"
    );
}

#[test]
fn ac11_results_obey_output_rules_for_many_shapes() {
    let star = polygon(&[
        (0.0, 30.0),
        (7.0, 9.0),
        (29.0, 9.0),
        (11.0, -4.0),
        (18.0, -25.0),
        (0.0, -12.0),
        (-18.0, -25.0),
        (-11.0, -4.0),
        (-29.0, 9.0),
        (-7.0, 9.0),
    ]);
    for shape in [
        vec![star],
        ring(),
        vec![l_shape()],
        vec![circle(3.0, 4.0, 7.5)],
    ] {
        for join in JOINS {
            for d in [-1.3, 0.01, 0.7, 3.0, 12.0] {
                if let Ok(r) = closed(&shape, d, join) {
                    check_invariants(&r).unwrap_or_else(|e| panic!("{d} {join:?}: {e}"));
                    for o in r.outlines() {
                        assert!(
                            o.windows(2)
                                .all(|w| (w[0].x - w[1].x).hypot(w[0].y - w[1].y) >= 0.001 - 1e-9)
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------- criterion 6 / 9 --------------

#[test]
fn ac6_outer_outline_positive_holes_negative() {
    let r = ok(&ring(), 1.0, OffsetJoin::Round);
    let mut signs: Vec<bool> = r
        .outlines()
        .iter()
        .map(|o| signed_area_mm2(o) > 0.0)
        .collect();
    signs.sort_unstable();
    assert_eq!(signs, vec![false, true]);
}

#[test]
fn ac6_outset_of_a_clockwise_square_still_grows() {
    let cw = reversed(rect(0.0, 0.0, 20.0, 20.0));
    let r = ok(&[cw], 5.0, OffsetJoin::Miter);
    assert!(
        near(bbox(&r), (-5.0, -5.0, 25.0, 25.0), 1e-6),
        "{:?}",
        bbox(&r)
    );
}

#[test]
fn ac9_self_crossing_bowtie_and_pentagram_nonzero() {
    // Bow-tie: two triangles of opposite winding; nonzero paints both.
    let bowtie = polygon(&[(0.0, 0.0), (20.0, 20.0), (20.0, 0.0), (0.0, 20.0)]);
    let r = ok(&[bowtie], 1.0, OffsetJoin::Round);
    assert!(covers(&r, Point::new(2.0, 10.0)) && covers(&r, Point::new(18.0, 10.0)));
    // Pentagram drawn as one self-crossing outline: nonzero paints the centre.
    let pts: Vec<(f64, f64)> = (0..5)
        .map(|i| {
            let a = PI / 2.0 + f64::from(i) * 4.0 * PI / 5.0;
            (20.0 * a.cos(), 20.0 * a.sin())
        })
        .collect();
    let star = polygon(&pts);
    let grown = ok(std::slice::from_ref(&star), 1.0, OffsetJoin::Round);
    assert!(
        covers(&grown, Point::new(0.0, 0.0)),
        "nonzero centre is part of the region"
    );
    assert_eq!(
        grown.outlines().len(),
        1,
        "a clean outline, no hole at the centre"
    );
    let shrunk = ok(&[star], -0.5, OffsetJoin::Round);
    assert!(covers(&shrunk, Point::new(0.0, 0.0)));
}

// ---------------------------------------------------------------- criterion 8 ------------------

#[test]
fn ac8_open_polyline_join_choice_and_self_crossing_hole() {
    let zig = [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0)];
    let miter = run(
        &[(polygon(&zig), false)],
        2.0,
        OffsetJoin::Miter,
        OffsetCap::Flat,
    )
    .unwrap();
    assert!(
        has_node(&miter, 22.0, -2.0, 1e-6),
        "miter at the outer corner"
    );
    let round = run(
        &[(polygon(&zig), false)],
        2.0,
        OffsetJoin::Round,
        OffsetCap::Flat,
    )
    .unwrap();
    assert!(!has_node(&round, 22.0, -2.0, 1e-3));
    // A path that crosses itself and leaves a hole: a closed loop drawn open with a tail.
    let loopy = [
        (0.0, -5.0),
        (0.0, 20.0),
        (20.0, 20.0),
        (20.0, 0.0),
        (-5.0, 0.0),
    ];
    let r = line(&loopy, 1.0, OffsetCap::Round).unwrap();
    check_invariants(&r).unwrap();
    assert_eq!(r.outlines().len(), 2, "outline plus hole: compound");
    assert!(!covers(&r, Point::new(10.0, 10.0)));
    assert!(covers(&r, Point::new(0.0, 10.0)));
}

// ---------------------------------------------------------------- criterion 12 -----------------

#[test]
fn ac12_deterministic_and_start_node_independent() {
    let a = ok(&ring(), 1.7, OffsetJoin::Round);
    let b = ok(&ring(), 1.7, OffsetJoin::Round);
    assert_eq!(a.outlines(), b.outlines());
    let mut l = l_shape();
    l.rotate_left(2);
    let x = ok(&[l_shape()], -1.3, OffsetJoin::Bevel);
    let y = ok(&[l], -1.3, OffsetJoin::Bevel);
    assert_eq!(x.outlines(), y.outlines(), "same region, same result");
}

// ---------------------------------------------------------------- criterion 14 / edges ---------

#[test]
fn edge_zero_tiny_nonfinite_and_huge_distances() {
    let sq = [rect(0.0, 0.0, 20.0, 20.0)];
    assert_eq!(
        closed(&sq, 0.0, OffsetJoin::Round),
        Err(OffsetError::ZeroDistance)
    );
    assert_eq!(
        closed(&sq, -0.0, OffsetJoin::Round),
        Err(OffsetError::ZeroDistance)
    );
    assert_eq!(
        closed(&sq, 0.0004, OffsetJoin::Round),
        Err(OffsetError::ZeroDistance)
    );
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e8] {
        assert_eq!(
            closed(&sq, bad, OffsetJoin::Round),
            Err(OffsetError::OutOfRange),
            "{bad}"
        );
    }
    let small = ok(&sq, 0.001, OffsetJoin::Miter);
    assert!(near(bbox(&small), (-0.001, -0.001, 20.001, 20.001), 1e-9));
    let big = ok(&sq, 1000.0, OffsetJoin::Round);
    let want = 400.0 + 4.0 * 20.0 * 1000.0 + PI * 1e6;
    assert!((area(&big) - want).abs() / want < 1e-4, "{}", area(&big));
    assert_eq!(
        closed(&sq, -1000.0, OffsetJoin::Round),
        Err(OffsetError::Empty)
    );
}

#[test]
fn edge_out_of_range_operand_and_result() {
    let far = [rect(9_999_990.0, 0.0, 9_999_999.0, 10.0)];
    assert_eq!(
        closed(&far, 5.0, OffsetJoin::Miter),
        Err(OffsetError::OutOfRange)
    );
    let nan = [polygon(&[(0.0, 0.0), (f64::NAN, 0.0), (5.0, 5.0)])];
    assert_eq!(
        closed(&nan, 1.0, OffsetJoin::Round),
        Err(OffsetError::OutOfRange)
    );
}

#[test]
fn edge_closed_collinear_operand_has_no_area() {
    let flat = [polygon(&[(0.0, 0.0), (5.0, 0.0), (9.0, 0.0)])];
    assert_eq!(
        closed(&flat, 1.0, OffsetJoin::Round),
        Err(OffsetError::NoArea)
    );
    assert_eq!(
        closed(&flat, -1.0, OffsetJoin::Round),
        Err(OffsetError::NoArea)
    );
}

#[test]
fn edge_degenerate_open_paths() {
    // A single-point open path grown by d: a disc, or refused, never a panic.
    let dot = line(&[(5.0, 5.0)], 2.0, OffsetCap::Round);
    if let Ok(r) = &dot {
        check_invariants(r).unwrap();
    }
    let twice = line(&[(5.0, 5.0), (5.0, 5.0)], 2.0, OffsetCap::Round);
    if let Ok(r) = &twice {
        check_invariants(r).unwrap();
    }
    // A back-and-forth open path: the result is still clean.
    let back = line(&[(0.0, 0.0), (20.0, 0.0), (5.0, 0.0)], 2.0, OffsetCap::Flat).unwrap();
    check_invariants(&back).unwrap();
    // 20 x 4 band plus the Round join at the 180-degree turn (a half disc of radius 2).
    assert!(
        (area(&back) - (80.0 + 2.0 * PI)).abs() <= 0.05,
        "{}",
        area(&back)
    );
}

// ---------------------------------------------------------------- properties -------------------

fn convex(n: usize, rx: f64, ry: f64, phase: f64) -> Anchors {
    let pts: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let a = phase + 2.0 * PI * i as f64 / n as f64;
            (rx * a.cos(), ry * a.sin())
        })
        .collect();
    polygon(&pts)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn prop_outset_contains_operand_and_inset_inside(
        n in 3usize..12, rx in 2.0f64..40.0, ry in 2.0f64..40.0, phase in 0.0f64..std::f64::consts::TAU,
        d in 0.01f64..10.0, j in 0usize..3,
    ) {
        let shape = convex(n, rx, ry, phase);
        let join = JOINS[j];
        let base = signed_area_mm2(&shape.iter().map(|a| a.0).collect::<Vec<_>>()).abs();
        let out = closed(std::slice::from_ref(&shape), d, join).unwrap();
        prop_assert!(check_invariants(&out).is_ok());
        prop_assert!(area(&out) >= base - 1e-3);
        for a in &shape {
            prop_assert!(covers(&out, a.0), "outset covers the operand's corner {:?}", a.0);
        }
        match closed(std::slice::from_ref(&shape), -d, join) {
            Ok(inn) => {
                prop_assert!(check_invariants(&inn).is_ok());
                prop_assert!(area(&inn) <= base + 1e-3);
                prop_assert!(area(&inn) > 0.0);
            }
            Err(e) => prop_assert_eq!(e, OffsetError::Empty),
        }
    }
}
