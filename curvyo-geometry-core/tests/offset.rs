//! Acceptance tests of the offset kernel (`specs/0038-path-offset`, reworked 2026-10-12): curves
//! stay curves (criteria 33 to 39), the kernel behaviour (28, 30, 31) and the numeric tests
//! (45 to 49), measured on the exact curve result.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

mod common;

use std::f64::consts::PI;

use common::offset_check::{
    area, bbox, check_node_rules, covers, cubic_count, flattened, has_node, largest_deviation,
    nodes,
};
use common::{Anchors, KERNEL_TOLERANCE, circle, ellipse, polygon, rect, rounded_rect, wavy};
use curvyo_document_core::{Length, Point};
use curvyo_geometry_core::{
    OffsetError, OffsetJoin, OffsetOutlines, Outline, offset, signed_area_mm2,
};

const JOINS: [OffsetJoin; 3] = [OffsetJoin::Round, OffsetJoin::Miter, OffsetJoin::Bevel];

fn run(
    outlines: &[(Anchors, bool)],
    d: f64,
    join: OffsetJoin,
) -> Result<OffsetOutlines, OffsetError> {
    let borrowed: Vec<Outline<'_>> = outlines.iter().map(|(a, c)| Outline::new(a, *c)).collect();
    offset(&borrowed, Length::from_mm(d), join, KERNEL_TOLERANCE)
}

fn closed_all(outlines: &[Anchors]) -> Vec<(Anchors, bool)> {
    outlines.iter().map(|a| (a.clone(), true)).collect()
}

/// A result that obeys the node rules and needed no fallback.
fn ok(outlines: &[Anchors], d: f64, join: OffsetJoin) -> OffsetOutlines {
    let r = run(&closed_all(outlines), d, join)
        .unwrap_or_else(|e| panic!("offset {d} {join:?} refused: {e}"));
    check_node_rules(&r).unwrap_or_else(|e| panic!("{d} {join:?}: criterion 33: {e}"));
    assert_eq!(r.fallback_outlines(), 0, "{d} {join:?}: criterion 39");
    r
}

fn reversed(mut anchors: Anchors) -> Anchors {
    anchors.reverse();
    for anchor in &mut anchors {
        std::mem::swap(&mut anchor.1, &mut anchor.2);
    }
    anchors
}

fn square() -> Anchors {
    rect(0.0, 0.0, 20.0, 20.0)
}

// ------------------------------------------------------------ curves stay curves (33 to 39) ---

#[test]
fn ac35_a_circle_path_gives_four_smooth_cubics() {
    let c = circle(0.0, 0.0, 20.0);
    let r = ok(std::slice::from_ref(&c), 2.0, OffsetJoin::Round);
    assert_eq!(r.outlines().len(), 1);
    assert_eq!(r.segment_count(), 4);
    assert_eq!(cubic_count(&r), 4);
    assert!(r.outlines()[0].iter().all(|a| a.smooth));
    for p in nodes(&r) {
        assert!((p.x.hypot(p.y) - 22.0).abs() < 0.01, "{p:?}");
    }
    let deviation = largest_deviation(&[(c, true)], &r, 2.0);
    assert!(deviation <= 0.01, "criterion 34: {deviation}");
}

#[test]
fn ac36_rounded_and_sharp_rectangles_give_few_segments() {
    let rounded = rounded_rect(0.0, 0.0, 100.0, 50.0, 5.0);
    let r = ok(std::slice::from_ref(&rounded), 2.0, OffsetJoin::Round);
    assert_eq!(r.segment_count(), 8);
    assert_eq!(cubic_count(&r), 4);
    assert!(r.outlines()[0].iter().all(|a| a.smooth));
    assert!(largest_deviation(&[(rounded, true)], &r, 2.0) <= 0.01);

    let sharp = rect(0.0, 0.0, 20.0, 20.0);
    let round = ok(std::slice::from_ref(&sharp), 2.0, OffsetJoin::Round);
    assert_eq!(round.segment_count(), 8);
    assert_eq!(cubic_count(&round), 4);
    assert!(largest_deviation(&[(sharp.clone(), true)], &round, 2.0) <= 0.01);
    let miter = ok(std::slice::from_ref(&sharp), 2.0, OffsetJoin::Miter);
    assert_eq!(miter.segment_count(), 4);
    assert_eq!(cubic_count(&miter), 0);
    let bevel = ok(&[sharp], 2.0, OffsetJoin::Bevel);
    assert_eq!(bevel.segment_count(), 8);
}

#[test]
fn ac37_an_ellipse_path_gives_at_most_eight_smooth_cubics() {
    let e = ellipse(0.0, 0.0, 20.0, 10.0);
    for d in [2.0, -2.0, 5.0] {
        let r = ok(std::slice::from_ref(&e), d, OffsetJoin::Round);
        assert!(r.segment_count() <= 8, "{d}: {}", r.segment_count());
        assert_eq!(cubic_count(&r), r.segment_count());
        assert!(r.outlines()[0].iter().all(|a| a.smooth), "{d}");
        let deviation = largest_deviation(&[(e.clone(), true)], &r, d);
        assert!(deviation <= 0.01, "{d}: {deviation}");
    }
}

#[test]
fn ac38_the_gear_keeps_its_segment_count() {
    let gear = wavy(200, 0.0, 0.0);
    let r = ok(std::slice::from_ref(&gear), 2.0, OffsetJoin::Round);
    println!(
        "gear outset 2 mm: {} segments, {} fallback outlines",
        r.segment_count(),
        r.fallback_outlines()
    );
    assert!(r.segment_count() <= 400, "{}", r.segment_count());
    let deviation = largest_deviation(&[(gear, true)], &r, 2.0);
    assert!(deviation <= 0.01, "{deviation}");
}

#[test]
fn ac39_the_fallback_count_over_the_test_shapes() {
    let mut fired = 0;
    let mut outlines = 0;
    let shapes: Vec<Vec<Anchors>> = vec![
        vec![square()],
        vec![circle(3.0, 4.0, 7.5)],
        vec![ellipse(0.0, 0.0, 20.0, 10.0)],
        vec![rounded_rect(0.0, 0.0, 30.0, 10.0, 4.0)],
        vec![l_shape()],
        vec![star()],
        ring(),
        vec![wavy(200, 0.0, 0.0)],
    ];
    for shape in &shapes {
        for join in JOINS {
            for d in [-1.3, 0.01, 0.7, 3.0, 12.0] {
                if let Ok(r) = run(&closed_all(shape), d, join) {
                    check_node_rules(&r).unwrap_or_else(|e| panic!("{d} {join:?}: {e}"));
                    fired += r.fallback_outlines();
                    outlines += r.outlines().len();
                }
            }
        }
    }
    println!("criterion 39: the fallback fired for {fired} of {outlines} outlines");
}

#[test]
fn ac33_nodes_follow_the_output_rules() {
    for shape in [
        vec![star()],
        ring(),
        vec![l_shape()],
        vec![circle(3.0, 4.0, 7.5)],
    ] {
        for join in JOINS {
            for d in [-1.3, 0.7, 3.0] {
                if let Ok(r) = run(&closed_all(&shape), d, join) {
                    check_node_rules(&r).unwrap_or_else(|e| panic!("{d} {join:?}: {e}"));
                }
            }
        }
    }
}

// ------------------------------------------------------------ kernel behaviour (28, 30, 31) ---

#[test]
fn ac28_outer_outlines_are_positive_and_holes_negative() {
    let r = ok(&ring(), 1.0, OffsetJoin::Round);
    let mut signs: Vec<bool> = flattened(&r, 0.001)
        .iter()
        .map(|o| signed_area_mm2(o) > 0.0)
        .collect();
    signs.sort_unstable();
    assert_eq!(signs, vec![false, true]);
    let clockwise = ok(&[reversed(square())], 2.0, OffsetJoin::Miter);
    assert!(
        (bbox(&clockwise).0 + 2.0).abs() < 1e-9,
        "a clockwise square still grows"
    );
}

#[test]
fn ac31_a_self_crossing_operand_gives_a_clean_outline() {
    let bowtie = polygon(&[(0.0, 0.0), (20.0, 20.0), (20.0, 0.0), (0.0, 20.0)]);
    let r = run(&[(bowtie, true)], 1.0, OffsetJoin::Miter).unwrap();
    assert!(covers(&r, Point::new(3.0, 10.0)));
    assert!(covers(&r, Point::new(17.0, 10.0)));
    assert!(!covers(&r, Point::new(10.0, 2.0)));
}

#[test]
fn ac55_an_open_line_is_not_offset() {
    let line = [(polygon(&[(0.0, 0.0), (20.0, 0.0)]), false)];
    assert_eq!(
        run(&line, 5.0, OffsetJoin::Round),
        Err(OffsetError::OpenPath)
    );
    let mixed = [
        (square(), true),
        (polygon(&[(0.0, 30.0), (20.0, 30.0)]), false),
    ];
    assert_eq!(
        run(&mixed, -1.0, OffsetJoin::Round),
        Err(OffsetError::OpenPath)
    );
}

// ------------------------------------------------------------------ numeric tests (45 to 49) ---

#[test]
fn ac45_square_outset_inset_and_collapse() {
    let miter = ok(&[square()], 5.0, OffsetJoin::Miter);
    assert_eq!(miter.segment_count(), 4);
    for (x, y) in [(-5.0, -5.0), (25.0, -5.0), (25.0, 25.0), (-5.0, 25.0)] {
        assert!(has_node(&miter, x, y, 1e-9), "({x}, {y})");
    }
    let round = ok(&[square()], 5.0, OffsetJoin::Round);
    assert_eq!(round.segment_count(), 8);
    let expected = 400.0 + 4.0 * 20.0 * 5.0 + PI * 25.0;
    assert!((area(&round) - expected).abs() <= 0.05, "{}", area(&round));
    let bevel = ok(&[square()], 5.0, OffsetJoin::Bevel);
    assert!((area(&bevel) - 850.0).abs() <= 0.01, "{}", area(&bevel));
    for join in JOINS {
        let inset = ok(&[square()], -5.0, join);
        assert_eq!(inset.segment_count(), 4);
        assert!((area(&inset) - 100.0).abs() <= 1e-6);
        assert_eq!(
            run(&closed_all(&[square()]), -10.0, join),
            Err(OffsetError::Empty)
        );
        let tiny = ok(&[square()], -9.99, join);
        let b = bbox(&tiny);
        assert!((b.2 - b.0 - 0.02).abs() < 1e-6 && (b.3 - b.1 - 0.02).abs() < 1e-6);
    }
}

fn ring() -> Vec<Anchors> {
    vec![circle(0.0, 0.0, 20.0), reversed(circle(0.0, 0.0, 10.0))]
}

#[test]
fn ac46_ring_outset_inset_and_vanish() {
    let out = ok(&ring(), 2.0, OffsetJoin::Round);
    assert_eq!(out.outlines().len(), 2);
    assert!(
        (area(&out) - PI * (484.0 - 64.0)).abs() <= 3.0,
        "{}",
        area(&out)
    );
    assert_eq!(out.segment_count(), 8);
    let inset = ok(&ring(), -2.0, OffsetJoin::Round);
    assert!(
        (area(&inset) - PI * (324.0 - 144.0)).abs() <= 3.0,
        "{}",
        area(&inset)
    );
    assert_eq!(
        run(&closed_all(&ring()), -5.0, OffsetJoin::Round),
        Err(OffsetError::Empty)
    );
}

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

fn star() -> Anchors {
    polygon(&[
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
    ])
}

#[test]
fn ac48_the_reflex_corner_of_an_l() {
    for join in JOINS {
        let out = ok(&[l_shape()], 2.0, join);
        assert!(
            has_node(&out, 12.0, 12.0, 1e-9),
            "{join:?}: the reflex corner stays sharp"
        );
    }
    let round_in = ok(&[l_shape()], -2.0, OffsetJoin::Round);
    assert!(
        !has_node(&round_in, 8.0, 8.0, 1e-6),
        "the reflex corner gets an arc"
    );
    assert!(
        has_node(&round_in, 2.0, 2.0, 1e-9),
        "the convex corner stays sharp"
    );
    let miter_in = ok(&[l_shape()], -2.0, OffsetJoin::Miter);
    assert!(has_node(&miter_in, 8.0, 8.0, 1e-9));
}

#[test]
fn ac49_the_miter_limit() {
    let triangle = polygon(&[
        (0.0, 0.0),
        (40.0, 0.0),
        (20.0, 20.0 / (10f64.to_radians().tan())),
    ]);
    let apex = triangle[2].0;
    let r = ok(&[triangle], 5.0, OffsetJoin::Miter);
    let near_apex: Vec<Point> = nodes(&r)
        .into_iter()
        .filter(|p| (p.x - apex.x).hypot(p.y - apex.y) < 25.0)
        .collect();
    assert_eq!(near_apex.len(), 2, "{near_apex:?}");
    for p in &near_apex {
        assert!(((p.x - apex.x).hypot(p.y - apex.y) - 20.0).abs() <= 0.01);
    }
    assert!(
        (near_apex[0].x - apex.x + near_apex[1].x - apex.x).abs() < 1e-6,
        "symmetric"
    );
    let sq = ok(&[square()], 5.0, OffsetJoin::Miter);
    for p in nodes(&sq) {
        let d = [(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)]
            .iter()
            .map(|c| (p.x - c.0).hypot(p.y - c.1))
            .fold(f64::MAX, f64::min);
        assert!((d - 50f64.sqrt()).abs() <= 1e-6);
    }
}

// ------------------------------------------------------------------ determinism (44) ----------

#[test]
fn ac44_the_start_node_and_direction_do_not_matter() {
    let base = ellipse(0.0, 0.0, 20.0, 10.0);
    let mut rotated = base.clone();
    rotated.rotate_left(1);
    for d in [2.0, -2.0] {
        let a = ok(std::slice::from_ref(&base), d, OffsetJoin::Round);
        let b = ok(std::slice::from_ref(&rotated), d, OffsetJoin::Round);
        let c = ok(&[reversed(base.clone())], d, OffsetJoin::Round);
        assert_eq!(a.segment_count(), b.segment_count());
        assert_eq!(a.segment_count(), c.segment_count());
        for (p, q) in nodes(&a).iter().zip(nodes(&b)) {
            assert!((p.x - q.x).abs() < 1e-6 && (p.y - q.y).abs() < 1e-6);
        }
    }
}

#[test]
fn edge_zero_tiny_nonfinite_and_huge_distances() {
    let sq = closed_all(&[square()]);
    assert_eq!(
        run(&sq, 0.0, OffsetJoin::Round),
        Err(OffsetError::ZeroDistance)
    );
    assert_eq!(
        run(&sq, f64::NAN, OffsetJoin::Round),
        Err(OffsetError::OutOfRange)
    );
    let tiny = ok(&[square()], 0.01, OffsetJoin::Round);
    assert!((bbox(&tiny).0 + 0.01).abs() < 1e-6);
    let flat_closed = polygon(&[(0.0, 0.0), (5.0, 0.0), (9.0, 0.0)]);
    assert_eq!(
        run(&[(flat_closed, true)], 1.0, OffsetJoin::Round),
        Err(OffsetError::NoArea)
    );
}
