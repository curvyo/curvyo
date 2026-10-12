//! Acceptance tests of the offset kernel (`specs/0038-path-offset` criteria 6 to 9, 11, 12 and 15
//! to 20): the numbers of the spec's "Tests" section, and the output rules of criterion 11.

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

use common::{
    Anchors, KERNEL_TOLERANCE, area, circle, covers, distance_to_segment, grid_points, node_count,
    polygon, rect,
};
use curvyo_document_core::{Length, Point, Vec2};
use curvyo_geometry_core::{
    BooleanResult, OffsetCap, OffsetError, OffsetJoin, Outline, offset, signed_area_mm2,
};

fn closed(outlines: &[Anchors], d: f64, join: OffsetJoin) -> Result<BooleanResult, OffsetError> {
    let borrowed: Vec<Outline<'_>> = outlines.iter().map(|a| Outline::new(a, true)).collect();
    offset(
        &borrowed,
        Length::from_mm(d),
        join,
        OffsetCap::Round,
        KERNEL_TOLERANCE,
    )
}

fn open(line: &Anchors, d: f64, cap: OffsetCap) -> Result<BooleanResult, OffsetError> {
    offset(
        &[Outline::new(line, false)],
        Length::from_mm(d),
        OffsetJoin::Round,
        cap,
        KERNEL_TOLERANCE,
    )
}

fn ok(outlines: &[Anchors], d: f64, join: OffsetJoin) -> BooleanResult {
    closed(outlines, d, join).unwrap_or_else(|e| panic!("offset {d} {join:?} refused: {e}"))
}

fn square() -> Anchors {
    rect(0.0, 0.0, 20.0, 20.0)
}

fn reversed(mut anchors: Anchors) -> Anchors {
    anchors.reverse();
    for anchor in &mut anchors {
        std::mem::swap(&mut anchor.1, &mut anchor.2);
    }
    anchors
}

fn sorted_points(result: &BooleanResult) -> Vec<(i64, i64)> {
    let mut points: Vec<(i64, i64)> = result
        .outlines()
        .iter()
        .flat_map(|o| grid_points(o))
        .collect();
    points.sort_unstable();
    points
}

fn distance(a: Point, b: Point) -> f64 {
    (b.x - a.x).hypot(b.y - a.y)
}

/// Criterion 11: corner polylines on the grid, at least 0.001 mm between consecutive nodes, no node
/// within 0.001 mm of the line between its neighbours, outer outlines positive.
fn assert_output_rules(result: &BooleanResult) {
    for outline in result.outlines() {
        let grid = grid_points(outline);
        assert!(grid.len() >= 3);
        let n = grid.len();
        for i in 0..n {
            let (prev, here, next) = (grid[(i + n - 1) % n], grid[i], grid[(i + 1) % n]);
            assert_ne!(here, next, "repeated vertex");
            assert!(
                distance_to_segment(here, prev, next) > 1.0,
                "vertex {here:?} lies on the line between its neighbours"
            );
        }
    }
    let outer: f64 = result
        .outlines()
        .iter()
        .map(|o| signed_area_mm2(o))
        .filter(|a| *a > 0.0)
        .sum();
    assert!(outer > 0.0, "outer outlines have positive area");
}

// ---- criterion 15: the square ----

#[test]
fn a_square_outset_with_miter_is_a_bigger_square_of_four_nodes() {
    let result = ok(&[square()], 5.0, OffsetJoin::Miter);
    assert_eq!(result.outlines().len(), 1);
    assert_eq!(
        sorted_points(&result),
        vec![
            (-5000, -5000),
            (-5000, 25000),
            (25000, -5000),
            (25000, 25000)
        ]
    );
    assert_output_rules(&result);
}

#[test]
fn a_square_outset_with_round_and_bevel_has_the_spec_areas() {
    let round = ok(&[square()], 5.0, OffsetJoin::Round);
    let expected = 400.0 + 4.0 * 20.0 * 5.0 + PI * 25.0;
    assert!((area(&round) - expected).abs() < 0.3, "{}", area(&round));
    assert_output_rules(&round);
    let bevel = ok(&[square()], 5.0, OffsetJoin::Bevel);
    assert!((area(&bevel) - 850.0).abs() < 0.01, "{}", area(&bevel));
    assert_eq!(node_count(&bevel), 8);
}

#[test]
fn a_square_inset_is_the_smaller_square_with_every_join() {
    for join in [OffsetJoin::Round, OffsetJoin::Miter, OffsetJoin::Bevel] {
        let result = ok(&[square()], -5.0, join);
        assert_eq!(
            sorted_points(&result),
            vec![(5000, 5000), (5000, 15000), (15000, 5000), (15000, 15000)],
            "{join:?}"
        );
        assert!((area(&result) - 100.0).abs() < 1e-9);
    }
}

#[test]
fn a_square_inset_to_a_line_is_empty_and_just_short_of_it_is_a_tiny_square() {
    assert_eq!(
        closed(&[square()], -10.0, OffsetJoin::Miter),
        Err(OffsetError::Empty)
    );
    assert_eq!(
        closed(&[square()], -10.0, OffsetJoin::Round),
        Err(OffsetError::Empty)
    );
    let tiny = ok(&[square()], -9.99, OffsetJoin::Miter);
    assert_eq!(
        sorted_points(&tiny),
        vec![(9990, 9990), (9990, 10010), (10010, 9990), (10010, 10010)]
    );
}

// ---- criterion 16: the ring ----

fn ring() -> Vec<Anchors> {
    vec![circle(0.0, 0.0, 20.0), reversed(circle(0.0, 0.0, 10.0))]
}

#[test]
fn a_ring_outset_grows_the_outline_and_shrinks_the_hole() {
    let result = ok(&ring(), 2.0, OffsetJoin::Round);
    assert_eq!(result.outlines().len(), 2);
    let expected = PI * (484.0 - 64.0);
    assert!((area(&result) - expected).abs() < 3.0, "{}", area(&result));
    assert!(!covers(&result, Point::new(0.0, 0.0)));
    assert!(covers(&result, Point::new(9.0, 0.0)));
    assert!(covers(&result, Point::new(21.5, 0.0)));
    assert_output_rules(&result);
}

#[test]
fn a_ring_inset_shrinks_the_outline_and_grows_the_hole() {
    let result = ok(&ring(), -2.0, OffsetJoin::Round);
    let expected = PI * (324.0 - 144.0);
    assert!((area(&result) - expected).abs() < 3.0, "{}", area(&result));
    assert!(!covers(&result, Point::new(11.5, 0.0)));
    assert!(!covers(&result, Point::new(18.5, 0.0)));
    assert_eq!(
        closed(&ring(), -5.0, OffsetJoin::Round),
        Err(OffsetError::Empty)
    );
}

// ---- criterion 17: the open line ----

fn line() -> Anchors {
    polygon(&[(0.0, 0.0), (20.0, 0.0)])
}

#[test]
fn an_open_line_grows_into_a_closed_outline_with_each_cap() {
    let round = open(&line(), 5.0, OffsetCap::Round).unwrap();
    assert_eq!(round.outlines().len(), 1);
    let expected = 200.0 + PI * 25.0;
    assert!((area(&round) - expected).abs() < 0.3, "{}", area(&round));
    assert_output_rules(&round);

    let flat = open(&line(), 5.0, OffsetCap::Flat).unwrap();
    assert_eq!(
        sorted_points(&flat),
        vec![(0, -5000), (0, 5000), (20000, -5000), (20000, 5000)]
    );
    let square = open(&line(), 5.0, OffsetCap::Square).unwrap();
    assert_eq!(
        sorted_points(&square),
        vec![(-5000, -5000), (-5000, 5000), (25000, -5000), (25000, 5000)]
    );
}

#[test]
fn an_open_line_cannot_be_inset() {
    assert_eq!(
        open(&line(), -5.0, OffsetCap::Round),
        Err(OffsetError::InsetOfOpenPath)
    );
}

// ---- criterion 18: the reflex corner ----

/// An L of arm width 10: (0,0) (30,0) (30,10) (10,10) (10,30) (0,30). Reflex corner at (10,10).
fn l_shape() -> Anchors {
    polygon(&[
        (0.0, 0.0),
        (30.0, 0.0),
        (30.0, 10.0),
        (10.0, 10.0),
        (10.0, 30.0),
        (0.0, 30.0),
    ])
}

#[test]
fn an_l_outset_keeps_the_reflex_corner_sharp_and_joins_the_convex_ones() {
    for join in [OffsetJoin::Round, OffsetJoin::Miter, OffsetJoin::Bevel] {
        let result = ok(&[l_shape()], 2.0, join);
        let points = sorted_points(&result);
        assert!(
            points.contains(&(12000, 12000)),
            "{join:?}: the reflex corner is one sharp node"
        );
        let near_reflex = points
            .iter()
            .filter(|&&(x, y)| (x - 12000).abs() < 2000 && (y - 12000).abs() < 2000)
            .count();
        assert_eq!(near_reflex, 1, "{join:?}");
    }
    let miter = ok(&[l_shape()], 2.0, OffsetJoin::Miter);
    assert!(sorted_points(&miter).contains(&(-2000, -2000)));
    let bevel = ok(&[l_shape()], 2.0, OffsetJoin::Bevel);
    assert!(!sorted_points(&bevel).contains(&(-2000, -2000)));
}

#[test]
fn an_l_inset_joins_the_reflex_corner_and_keeps_the_convex_ones_sharp() {
    let miter = ok(&[l_shape()], -2.0, OffsetJoin::Miter);
    let points = sorted_points(&miter);
    assert!(points.contains(&(2000, 2000)), "convex corner stays sharp");
    assert!(points.contains(&(8000, 8000)), "miter at the reflex corner");
    let bevel = ok(&[l_shape()], -2.0, OffsetJoin::Bevel);
    let points = sorted_points(&bevel);
    assert!(points.contains(&(2000, 2000)));
    assert!(
        !points.contains(&(8000, 8000)),
        "the reflex corner is bevelled"
    );
    assert!(points.contains(&(8000, 10000)) && points.contains(&(10000, 8000)));
}

// ---- criterion 19: the miter limit ----

#[test]
fn a_twenty_degree_corner_is_clipped_at_four_times_the_distance() {
    // Apex at the origin, opening to the right, symmetric about the x axis, 20 degrees.
    let half = 10.0_f64.to_radians();
    let length = 60.0;
    let triangle = polygon(&[
        (0.0, 0.0),
        (length * half.cos(), length * half.sin()),
        (length * half.cos(), -length * half.sin()),
    ]);
    let result = ok(&[triangle], 5.0, OffsetJoin::Miter);
    let apex = Point::new(0.0, 0.0);
    let outline = &result.outlines()[0];
    let near: Vec<Point> = outline
        .iter()
        .copied()
        .filter(|p| distance(*p, apex) < 25.0)
        .collect();
    assert_eq!(near.len(), 2, "two clip nodes at the apex: {near:?}");
    for p in &near {
        assert!(
            (distance(*p, apex) - 20.0).abs() < 0.01,
            "{p:?} is {} from the apex",
            distance(*p, apex)
        );
    }
    assert!(
        (near[0].x - near[1].x).abs() < 0.002,
        "symmetric about the bisector"
    );
    assert!((near[0].y + near[1].y).abs() < 0.002);
    assert!(
        outline
            .iter()
            .all(|p| (distance(*p, apex) - 28.8).abs() > 0.5),
        "no node at the unclipped tip"
    );
    // The two 80-degree corners stay single sharp nodes: 3 + 2 - 1 = 4 nodes in all.
    assert_eq!(outline.len(), 4);
}

// ---- criterion 20: curves become polylines ----

#[test]
fn a_circle_outset_is_a_polygon_near_the_bigger_circle() {
    let result = ok(&[circle(0.0, 0.0, 20.0)], 2.0, OffsetJoin::Round);
    assert_eq!(result.outlines().len(), 1);
    for p in &result.outlines()[0] {
        let r = p.x.hypot(p.y);
        assert!((r - 22.0).abs() <= 0.02, "node at radius {r}");
    }
    let expected = PI * 22.0 * 22.0;
    assert!((area(&result) - expected).abs() < 3.0, "{}", area(&result));
    assert_output_rules(&result);
}

#[test]
fn round_joins_stay_within_the_tolerance_and_the_node_budget() {
    // A square outset by 5 mm with Round: the corner arcs have radius 5. Every node of an arc
    // lies within 0.01 mm of it, and a quarter arc has between the fewest chords that keep
    // 0.01 mm and twice that.
    let result = ok(&[square()], 5.0, OffsetJoin::Round);
    let corner = Point::new(20.0, 20.0);
    let arc: Vec<Point> = result.outlines()[0]
        .iter()
        .copied()
        .filter(|p| p.x >= 20.0 && p.y >= 20.0)
        .collect();
    for p in &arc {
        assert!((distance(*p, corner) - 5.0).abs() <= 0.01);
    }
    let fewest = ((PI / 2.0) / (2.0 * (1.0 - 0.01 / 5.0_f64).acos())).ceil() as usize;
    let segments = arc.len() - 1;
    assert!(
        segments >= fewest && segments <= 2 * fewest,
        "{segments} segments, fewest {fewest}"
    );
}

// ---- criteria 9, 12, 14 ----

#[test]
fn a_self_crossing_outline_offsets_its_painted_region() {
    // A bow-tie: two triangles meeting at (10, 10).
    let bow = polygon(&[(0.0, 0.0), (20.0, 20.0), (20.0, 0.0), (0.0, 20.0)]);
    let result = ok(&[bow], 1.0, OffsetJoin::Miter);
    assert_output_rules(&result);
    assert!(covers(&result, Point::new(10.0, 10.0)));
    assert!(covers(&result, Point::new(-0.5, 10.0)));
}

#[test]
fn the_result_does_not_depend_on_the_outline_order() {
    let a = ok(&ring(), 1.5, OffsetJoin::Round);
    let mut swapped = ring();
    swapped.reverse();
    let b = ok(&swapped, 1.5, OffsetJoin::Round);
    assert_eq!(a, b);
}

#[test]
fn a_closed_outline_without_area_is_refused() {
    let flat = polygon(&[(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)]);
    assert_eq!(
        closed(&[flat], 2.0, OffsetJoin::Round),
        Err(OffsetError::NoArea)
    );
}

#[test]
fn coordinates_out_of_range_and_zero_distances_are_refused() {
    let far = rect(0.0, 0.0, 2.0e7, 10.0);
    assert_eq!(
        closed(&[far], 1.0, OffsetJoin::Round),
        Err(OffsetError::OutOfRange)
    );
    let mut nan = square();
    nan[0].1 = Vec2::new(f64::NAN, 0.0);
    assert_eq!(
        closed(&[nan], 1.0, OffsetJoin::Round),
        Err(OffsetError::OutOfRange)
    );
    let edge = rect(0.0, 0.0, 1.0e7, 10.0);
    assert_eq!(
        closed(&[edge], 1.0, OffsetJoin::Round),
        Err(OffsetError::OutOfRange),
        "a result beyond the range is refused"
    );
    assert_eq!(
        closed(&[square()], 0.0004, OffsetJoin::Round),
        Err(OffsetError::ZeroDistance)
    );
    assert_eq!(
        closed(&[square()], f64::INFINITY, OffsetJoin::Round),
        Err(OffsetError::OutOfRange)
    );
}
