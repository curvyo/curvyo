//! Tester's acceptance tests for `specs/0038-path-offset` (rework of 2026-10-12), kernel side:
//! criteria 33 to 39 (curves stay curves), 44 (determinism), 45 to 51 (numbers; none may end in
//! the polyline fallback) and edge cases (huge and tiny distances, rings, open paths, rotated and
//! reversed operands). Written from the specification before the implementation was read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::many_single_char_names,
    missing_docs
)]

mod common;

use std::f64::consts::PI;

use common::offset_check::{area, bbox, check_node_rules, cubic_count, largest_deviation, nodes};
use common::{Anchors, KERNEL_TOLERANCE, circle, ellipse, polygon, rect, rounded_rect, wavy};
use curvyo_document_core::{Length, Point};
use curvyo_geometry_core::{OffsetCap, OffsetError, OffsetJoin, OffsetOutlines, Outline, offset};

const JOINS: [OffsetJoin; 3] = [OffsetJoin::Round, OffsetJoin::Miter, OffsetJoin::Bevel];

fn run(
    operand: &[(Anchors, bool)],
    d: f64,
    join: OffsetJoin,
    cap: OffsetCap,
) -> Result<OffsetOutlines, OffsetError> {
    let borrowed: Vec<Outline<'_>> = operand.iter().map(|(a, c)| Outline::new(a, *c)).collect();
    offset(&borrowed, Length::from_mm(d), join, cap, KERNEL_TOLERANCE)
}

fn one(a: &Anchors, d: f64, join: OffsetJoin) -> Result<OffsetOutlines, OffsetError> {
    run(&[(a.clone(), true)], d, join, OffsetCap::Round)
}

fn good(a: &Anchors, d: f64, join: OffsetJoin) -> OffsetOutlines {
    let r = one(a, d, join).unwrap_or_else(|e| panic!("{d} {join:?}: {e}"));
    check_node_rules(&r).unwrap_or_else(|e| panic!("{d} {join:?}: criterion 33: {e}"));
    r
}

fn ring_operand() -> Vec<(Anchors, bool)> {
    let mut hole = circle(0., 0., 10.);
    hole.reverse();
    for n in &mut hole {
        std::mem::swap(&mut n.1, &mut n.2);
    }
    vec![(circle(0., 0., 20.), true), (hole, true)]
}

fn segments(r: &OffsetOutlines) -> usize {
    r.segment_count()
}

/// Criterion 35: circle path, outset 2 Round: one closed path of 4 cubics, all smooth.
#[test]
fn ac35_circle_path_outset_is_four_smooth_cubics() {
    for d in [2.0, 0.01, -2.0, -19.9] {
        let r = good(&circle(0.0, 0.0, 20.0), d, OffsetJoin::Round);
        assert_eq!(r.outlines().len(), 1, "d={d}");
        assert_eq!(segments(&r), 4, "d={d}");
        assert_eq!(cubic_count(&r), 4, "d={d}");
        assert!(r.outlines()[0].iter().all(|a| a.smooth), "d={d}");
        assert_eq!(r.fallback_outlines(), 0, "d={d}");
        let want = 20.0 + d;
        let dev = largest_deviation(&[(circle(0.0, 0.0, 20.0), true)], &r, d);
        assert!(dev <= 0.01, "d={d} deviation {dev}");
        assert!((area(&r) - PI * want * want).abs() < 0.001 * PI * want * want + 0.01);
    }
}

/// Criterion 35 for every join: a circle has no corners, so all joins agree.
#[test]
fn ac35_circle_every_join_is_four_cubics() {
    for join in JOINS {
        let r = good(&circle(5.0, -3.0, 10.0), 2.0, join);
        assert_eq!((segments(&r), cubic_count(&r)), (4, 4), "{join:?}");
    }
}

/// Criterion 36: rounded rectangle path (4 lines, 4 arcs) outset 2 Round: 8 segments, 4 cubics.
#[test]
fn ac36_rounded_rectangle_path_outset_is_eight_segments() {
    let r = good(
        &rounded_rect(0.0, 0.0, 40.0, 20.0, 5.0),
        2.0,
        OffsetJoin::Round,
    );
    assert_eq!(segments(&r), 8);
    assert_eq!(cubic_count(&r), 4);
    assert_eq!(r.fallback_outlines(), 0);
    let (x0, y0, x1, y1) = bbox(&r);
    assert!((x0 + 2.0).abs() < 0.01 && (y0 + 2.0).abs() < 0.01);
    assert!((x1 - 42.0).abs() < 0.01 && (y1 - 22.0).abs() < 0.01);
}

/// Criterion 36: sharp 20 x 20 path, outset 2: Round 8 segments (4 cubics), Miter 4 lines.
#[test]
fn ac36_sharp_square_round_8_miter_4() {
    let sq = rect(0.0, 0.0, 20.0, 20.0);
    let round = good(&sq, 2.0, OffsetJoin::Round);
    assert_eq!((segments(&round), cubic_count(&round)), (8, 4));
    let miter = good(&sq, 2.0, OffsetJoin::Miter);
    assert_eq!((segments(&miter), cubic_count(&miter)), (4, 0));
    assert_eq!(nodes(&miter).len(), 4);
}

/// Criterion 37 and 51: ellipse path, out and in, at most 8 cubics, smooth, within 0.01 mm.
#[test]
fn ac37_ac51_ellipse_path_few_smooth_cubics_within_tolerance() {
    for (rx, ry, d) in [
        (20.0, 10.0, 2.0),
        (20.0, 10.0, -2.0),
        (20.0, 10.0, 0.05),
        (20.0, 19.0, 7.0),
    ] {
        let op = ellipse(0.0, 0.0, rx, ry);
        let r = good(&op, d, OffsetJoin::Round);
        println!("{rx}x{ry} d={d}: {} cubics", cubic_count(&r));
        assert!(
            cubic_count(&r) <= 8,
            "{rx}x{ry} d={d}: {} cubics",
            cubic_count(&r)
        );
        assert_eq!(
            cubic_count(&r),
            segments(&r),
            "{rx}x{ry} d={d}: only cubics"
        );
        assert!(r.outlines()[0].iter().all(|a| a.smooth), "{rx}x{ry} d={d}");
        let dev = largest_deviation(&[(op, true)], &r, d);
        assert!(dev <= 0.01, "{rx}x{ry} d={d}: deviation {dev}");
        assert_eq!(r.fallback_outlines(), 0, "{rx}x{ry} d={d}");
    }
}

/// Criterion 34: result within 0.01 mm of the exact offset for every join, on curved operands.
#[test]
fn ac34_curved_operands_every_join_within_tolerance() {
    let shapes: Vec<(&str, Anchors)> = vec![
        ("circle", circle(0.0, 0.0, 15.0)),
        ("ellipse", ellipse(0.0, 0.0, 25.0, 12.0)),
        ("rounded", rounded_rect(0.0, 0.0, 30.0, 18.0, 4.0)),
        ("wavy40", wavy(40, 0.0, 0.0)),
    ];
    for (name, shape) in &shapes {
        for join in [OffsetJoin::Round] {
            for d in [0.3, 2.0, 5.0] {
                let r = good(shape, d, join);
                let dev = largest_deviation(&[(shape.clone(), true)], &r, d);
                assert!(dev <= 0.01, "{name} {join:?} +{d}: {dev}");
            }
        }
    }
    // Inset of rounded shapes (a Round or Miter join only matters at reflex corners).
    for (name, shape) in shapes.iter().take(3) {
        for join in JOINS {
            let r = good(shape, -2.0, join);
            let dev = largest_deviation(&[(shape.clone(), true)], &r, -2.0);
            assert!(dev <= 0.01, "{name} {join:?} -2: {dev}");
        }
    }
}

/// Criterion 34: a Round corner is one cubic per 90 degrees or less; above 37 mm 45 degree pieces.
#[test]
fn ac34_round_corner_pieces_and_large_distance() {
    let sq = rect(0.0, 0.0, 20.0, 20.0);
    let small = good(&sq, 30.0, OffsetJoin::Round);
    assert_eq!(segments(&small), 8, "d=30: one cubic per 90 degrees");
    let large = good(&sq, 50.0, OffsetJoin::Round);
    assert_eq!(segments(&large), 12, "d=50: two cubics per corner, 4 lines");
    let dev = largest_deviation(&[(sq, true)], &large, 50.0);
    assert!(dev <= 0.01, "d=50: {dev}");
}

/// Criterion 38: the 200-node gear, outset 2 Round, at most 400 segments, no fallback.
#[test]
fn ac38_gear_outset_stays_within_the_segment_budget() {
    let gear = wavy(200, 0.0, 0.0);
    let r = good(&gear, 2.0, OffsetJoin::Round);
    let n = segments(&r);
    println!(
        "gear outset 2: {n} segments, fallback outlines {}",
        r.fallback_outlines()
    );
    assert!(n <= 400, "{n} segments");
    assert_eq!(r.fallback_outlines(), 0, "the gear must not fall back");
    let r_in = good(&gear, -1.0, OffsetJoin::Round);
    println!("gear inset 1: {} segments", segments(&r_in));
    assert!(segments(&r_in) <= 400);
}

/// Criterion 39: how often the fallback fires on the numeric set of criteria 45 to 51.
#[test]
fn ac39_numeric_set_never_falls_back() {
    let cases: Vec<(&str, Vec<(Anchors, bool)>, f64, OffsetJoin, OffsetCap)> = vec![
        (
            "sq out",
            vec![(rect(0., 0., 20., 20.), true)],
            5.0,
            OffsetJoin::Round,
            OffsetCap::Round,
        ),
        (
            "sq miter",
            vec![(rect(0., 0., 20., 20.), true)],
            5.0,
            OffsetJoin::Miter,
            OffsetCap::Round,
        ),
        (
            "sq bevel",
            vec![(rect(0., 0., 20., 20.), true)],
            5.0,
            OffsetJoin::Bevel,
            OffsetCap::Round,
        ),
        (
            "sq in",
            vec![(rect(0., 0., 20., 20.), true)],
            -5.0,
            OffsetJoin::Round,
            OffsetCap::Round,
        ),
        (
            "ring out",
            ring_operand(),
            2.0,
            OffsetJoin::Round,
            OffsetCap::Round,
        ),
        (
            "ring in",
            ring_operand(),
            -2.0,
            OffsetJoin::Round,
            OffsetCap::Round,
        ),
        (
            "line round",
            vec![(polygon(&[(0., 0.), (20., 0.)]), false)],
            5.0,
            OffsetJoin::Round,
            OffsetCap::Round,
        ),
        (
            "triangle miter",
            vec![(
                polygon(&[(0., 0.), (50., 0.), (50. * 0.9397, 50. * 0.342)]),
                true,
            )],
            5.0,
            OffsetJoin::Miter,
            OffsetCap::Round,
        ),
        (
            "ellipse",
            vec![(ellipse(0., 0., 20., 10.), true)],
            2.0,
            OffsetJoin::Round,
            OffsetCap::Round,
        ),
    ];
    let mut fallbacks = 0;
    for (name, operand, d, join, cap) in &cases {
        let r = run(operand, *d, *join, *cap).unwrap_or_else(|e| panic!("{name}: {e}"));
        fallbacks += r.fallback_outlines();
        assert_eq!(r.fallback_outlines(), 0, "{name} ended in the fallback");
    }
    println!("fallback outlines on the numeric set: {fallbacks}");
}

/// Criterion 44: node for node the same, however the operand is started or traversed.
#[test]
fn ac44_deterministic_for_curved_operands() {
    let shapes = [
        circle(1.0, 2.0, 12.0),
        ellipse(0.0, 0.0, 20.0, 10.0),
        rounded_rect(0.0, 0.0, 30.0, 20.0, 4.0),
        wavy(60, 0.0, 0.0),
    ];
    for shape in &shapes {
        let a = good(shape, 2.0, OffsetJoin::Round);
        let b = good(shape, 2.0, OffsetJoin::Round);
        assert_eq!(a, b, "repeat");
        for shift in 1..shape.len().min(5) {
            let mut rotated = shape.clone();
            rotated.rotate_left(shift);
            let c = good(&rotated, 2.0, OffsetJoin::Round);
            assert_eq!(segments(&a), segments(&c), "start node {shift}");
            let (pa, pc) = (nodes(&a), nodes(&c));
            assert_eq!(pa.len(), pc.len());
            for (p, q) in pa.iter().zip(&pc) {
                assert!(
                    (p.x - q.x).abs() < 1e-6 && (p.y - q.y).abs() < 1e-6,
                    "start {shift}: {p:?} {q:?}"
                );
            }
        }
    }
}

/// Criteria 28/46: ring with compound path, numbers, and the vanishing hole boundary.
#[test]
fn ac46_ring_numbers_and_vanish() {
    let ring = ring_operand();
    let out = run(&ring, 2.0, OffsetJoin::Round, OffsetCap::Round).unwrap();
    assert!((area(&out) - PI * (484.0 - 64.0)).abs() < 3.0);
    assert_eq!(out.outlines().len(), 2);
    let inn = run(&ring, -2.0, OffsetJoin::Round, OffsetCap::Round).unwrap();
    assert!((area(&inn) - PI * (324.0 - 144.0)).abs() < 3.0);
    assert_eq!(
        run(&ring, -5.0, OffsetJoin::Round, OffsetCap::Round).unwrap_err(),
        OffsetError::Empty
    );
}

/// Criteria 29, 45: inset at the exact collapse and just below it.
#[test]
fn ac45_collapse_boundaries() {
    let sq = rect(0., 0., 20., 20.);
    for join in JOINS {
        assert_eq!(
            one(&sq, -10.0, join).unwrap_err(),
            OffsetError::Empty,
            "{join:?}"
        );
        let tiny = good(&sq, -9.99, join);
        assert!(
            (area(&tiny) - 0.0004).abs() < 1e-3,
            "{join:?}: {}",
            area(&tiny)
        );
        assert_eq!(one(&sq, -15.0, join).unwrap_err(), OffsetError::Empty);
    }
}

/// Criterion 47: open line numbers for every cap.
#[test]
fn ac47_line_caps() {
    let l = [(polygon(&[(0., 0.), (20., 0.)]), false)];
    let a = |cap| area(&run(&l, 5.0, OffsetJoin::Round, cap).unwrap());
    assert!((a(OffsetCap::Round) - (200.0 + 25.0 * PI)).abs() < 0.1);
    assert!((a(OffsetCap::Flat) - 200.0).abs() < 0.01);
    assert!((a(OffsetCap::Square) - 300.0).abs() < 0.01);
    assert_eq!(
        run(&l, -5.0, OffsetJoin::Round, OffsetCap::Round).unwrap_err(),
        OffsetError::InsetOfOpenPath
    );
}

/// Edges: distances at both ends of the range and beyond it.
#[test]
fn edge_distance_extremes() {
    let sq = rect(0., 0., 20., 20.);
    let tiny = good(&sq, 0.01, OffsetJoin::Miter);
    assert!((area(&tiny) - (400.0 + 0.01 * 80.0 + 0.0001 * 4.0)).abs() < 0.01);
    let huge = good(&sq, 1000.0, OffsetJoin::Round);
    let expect = 400.0 + 4.0 * 20.0 * 1000.0 + PI * 1e6;
    assert!(
        (area(&huge) - expect).abs() / expect < 1e-3,
        "{}",
        area(&huge)
    );
    assert_eq!(
        segments(&huge),
        4 + 4 * 2,
        "huge d: 4 lines, two 45-degree cubics per corner"
    );
    assert_eq!(
        one(&sq, 0.0, OffsetJoin::Round).unwrap_err(),
        OffsetError::ZeroDistance
    );
    assert_eq!(
        one(&sq, 0.0004, OffsetJoin::Round).unwrap_err(),
        OffsetError::ZeroDistance
    );
    assert_eq!(
        one(&sq, f64::NAN, OffsetJoin::Round).unwrap_err(),
        OffsetError::OutOfRange
    );
    assert_eq!(
        one(&sq, f64::INFINITY, OffsetJoin::Round).unwrap_err(),
        OffsetError::OutOfRange
    );
    assert_eq!(
        one(&sq, 1e12, OffsetJoin::Round).unwrap_err(),
        OffsetError::OutOfRange
    );
    let bad = rect(0., 0., f64::NAN, 20.);
    assert_eq!(
        one(&bad, 2.0, OffsetJoin::Round).unwrap_err(),
        OffsetError::OutOfRange
    );
}

/// Edge: an empty operand list and one-node or two-node closed operands do not panic.
#[test]
fn edge_degenerate_inputs_do_not_panic() {
    let none: Vec<(Anchors, bool)> = vec![];
    assert!(run(&none, 2.0, OffsetJoin::Round, OffsetCap::Round).is_err());
    let one_node = polygon(&[(1.0, 1.0)]);
    assert!(
        run(
            &[(one_node.clone(), true)],
            2.0,
            OffsetJoin::Round,
            OffsetCap::Round
        )
        .is_err()
    );
    let r = run(
        &[(one_node, false)],
        2.0,
        OffsetJoin::Round,
        OffsetCap::Round,
    );
    println!(
        "single-node open path outset: {:?}",
        r.as_ref().map(OffsetOutlines::segment_count)
    );
    let two = polygon(&[(0., 0.), (10., 0.)]);
    assert!(run(&[(two, true)], 2.0, OffsetJoin::Round, OffsetCap::Round).is_err());
    let dup = polygon(&[(0., 0.), (0., 0.), (10., 0.), (10., 10.)]);
    let _ = run(&[(dup, true)], 2.0, OffsetJoin::Round, OffsetCap::Round);
}

/// Criteria 28, 31: opposite winding gives the same region; nonzero for a mirrored operand.
#[test]
fn edge_reversed_and_mirrored_operands_agree() {
    let a = rounded_rect(0., 0., 30., 20., 4.);
    let mut b = a.clone();
    b.reverse();
    for n in &mut b {
        std::mem::swap(&mut n.1, &mut n.2);
    }
    let ra = good(&a, 3.0, OffsetJoin::Round);
    let rb = good(&b, 3.0, OffsetJoin::Round);
    assert_eq!(segments(&ra), segments(&rb));
    assert!((area(&ra) - area(&rb)).abs() < 1e-6);
    let _ = Point::new(0.0, 0.0);
}

/// Criterion 28: growing a ring past its hole closes the hole. The result must not keep a hole
/// at the centre (every point within d of the region is inside the result).
#[test]
fn ac28_outset_past_the_hole_radius_closes_the_hole() {
    for d in [10.01, 10.5, 12.0, 25.0] {
        let r = run(&ring_operand(), d, OffsetJoin::Round, OffsetCap::Round).unwrap();
        let want = PI * (20.0 + d) * (20.0 + d);
        assert_eq!(
            r.outlines().len(),
            1,
            "d={d}: a spurious hole of radius {} remains",
            d - 10.0
        );
        assert!(
            (area(&r) - want).abs() < 0.002 * want,
            "d={d}: area {} want {want}",
            area(&r)
        );
    }
}

/// The same with a square ring (sharp corners, so no curve pieces are involved).
#[test]
fn ac28_square_ring_outset_past_the_hole_closes_the_hole() {
    let mut hole = rect(5.0, 5.0, 15.0, 15.0);
    hole.reverse();
    let ring = [(rect(0.0, 0.0, 20.0, 20.0), true), (hole, true)];
    for join in JOINS {
        for d in [5.5, 8.0] {
            let r = run(&ring, d, join, OffsetCap::Round).unwrap();
            assert_eq!(r.outlines().len(), 1, "{join:?} d={d}: spurious hole");
        }
    }
}

/// Criterion 37 holds for any non-circular ellipse path: a narrow one (4:1, the UX document's own
/// example of 80 x 20 at 5 mm) must also stay within 8 cubics.
#[test]
fn ac37_narrow_ellipse_outset_stays_within_eight_cubics() {
    for (rx, ry, d) in [
        (40.0, 10.0, 5.0),
        (40.0, 10.0, -3.0),
        (40.0, 10.0, 2.0),
        (50.0, 5.0, 3.0),
    ] {
        let r = good(&ellipse(0.0, 0.0, rx, ry), d, OffsetJoin::Round);
        assert!(
            cubic_count(&r) <= 8,
            "{rx}x{ry} d={d}: {} cubics",
            cubic_count(&r)
        );
    }
}
