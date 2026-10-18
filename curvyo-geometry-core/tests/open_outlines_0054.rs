//! The kernel side of `specs/0054-combine-open-paths`: open outlines in the Boolean region and the
//! touch test, depth by "contains" for touching and crossing closed outlines, and the fill area
//! pins of criterion 3.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    missing_docs
)]

mod common;

use common::{Anchors, area, circle, covers, polygon, rect};
use curvyo_document_core::{Point, Vec2};
use curvyo_geometry_core::{
    BooleanError, BooleanOp, Outline, Tolerance, boolean, contains_point_in_outlines,
    outline_has_area, outline_nesting, touching_outlines,
};

const TOL: Tolerance = common::KERNEL_TOLERANCE;

/// The open "U" of criterion 2: (0, 0), (0, 10), (10, 10), (10, 0).
fn u_shape() -> Anchors {
    polygon(&[(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)])
}

fn shifted(anchors: &Anchors, dx: f64, dy: f64) -> Anchors {
    anchors
        .iter()
        .map(|&(p, i, o)| (Point::new(p.x + dx, p.y + dy), i, o))
        .collect()
}

fn reversed(anchors: &Anchors) -> Anchors {
    anchors.iter().rev().map(|&(p, i, o)| (p, o, i)).collect()
}

fn closed(all: &[Anchors]) -> Vec<Outline<'_>> {
    all.iter().map(|a| Outline::new(a, true)).collect()
}

fn depths(all: &[Anchors]) -> Vec<usize> {
    outline_nesting(&closed(all))
        .iter()
        .map(|n| n.depth)
        .collect()
}

// ------------------------------------------------------------------ Booleans (21, 22)

/// Criterion 21: Union of the open U and a far square gives 2 outlines; the U is closed by a
/// straight edge from (10, 0) to (0, 0) and encloses 100 mm². Curved end handles do not bulge it.
#[test]
fn c21_union_closes_an_open_outline_with_a_straight_line() {
    let mut u = u_shape();
    u[0].1 = Vec2::new(-5.0, -5.0);
    u[3].2 = Vec2::new(5.0, -5.0);
    let far = rect(50.0, 0.0, 60.0, 10.0);
    let a = [Outline::new(&u, false)];
    let b = [Outline::new(&far, true)];
    let result = boolean(BooleanOp::Union, &[&a, &b], TOL).unwrap();
    assert_eq!(result.outlines().len(), 2);
    assert!((area(&result) - 200.0).abs() < 0.05, "{}", area(&result));
    assert!(covers(&result, Point::new(5.0, 0.5)));
    assert!(
        !covers(&result, Point::new(5.0, -0.5)),
        "nothing below the closing line"
    );
}

/// Criterion 22: a plate with a straight score line, minus a disc: the plate minus the disc, the
/// score line adds nothing. An operand of straight lines only stays refused (`0016` criterion 16).
#[test]
fn c22_a_straight_open_outline_adds_nothing_and_lines_alone_have_no_area() {
    let plate = rect(0.0, 0.0, 100.0, 100.0);
    let score = polygon(&[(0.0, 50.0), (100.0, 50.0)]);
    let disc = circle(50.0, 50.0, 10.0);
    let with_line = [Outline::new(&plate, true), Outline::new(&score, false)];
    let without = [Outline::new(&plate, true)];
    let cutter = [Outline::new(&disc, true)];
    assert_eq!(
        boolean(BooleanOp::Difference, &[&with_line, &cutter], TOL),
        boolean(BooleanOp::Difference, &[&without, &cutter], TOL)
    );
    let dot = polygon(&[(5.0, 5.0), (5.0, 5.0)]);
    let lines = [Outline::new(&score, false), Outline::new(&dot, false)];
    let square = rect(0.0, 0.0, 10.0, 10.0);
    let sq = [Outline::new(&square, true)];
    assert_eq!(
        boolean(BooleanOp::Union, &[&lines, &sq], TOL),
        Err(BooleanError::EmptyOperands(vec![0]))
    );
    assert!(!outline_has_area(&Outline::new(&score, false)));
    assert!(!outline_has_area(&Outline::new(&dot, false)));
    assert!(outline_has_area(&Outline::new(&u_shape(), false)));
}

// ------------------------------------------------------------------ touch (1, 12, 15)

/// An open outline is read open: the U does not touch itself across its gap, ends that nearly meet touch, and a score line
/// across a plate touches the plate; one inside the plate away from its edge does not.
#[test]
fn touch_reads_open_outlines_open() {
    let plate = rect(0.0, 0.0, 100.0, 100.0);
    let across = polygon(&[(-10.0, 50.0), (110.0, 50.0)]);
    let inside = polygon(&[(10.0, 50.0), (90.0, 50.0)]);
    let nearly_closed = polygon(&[
        (0.0, 0.0),
        (0.0, 10.0),
        (10.0, 10.0),
        (10.0, 0.0),
        (0.0005, 0.0),
    ]);
    let found = |all: &[(&Anchors, bool)]| {
        let outlines: Vec<Outline<'_>> = all.iter().map(|(a, c)| Outline::new(a, *c)).collect();
        touching_outlines(&outlines)
    };
    assert_eq!(found(&[(&plate, true), (&across, false)]), vec![(0, 1)]);
    assert_eq!(
        found(&[(&plate, true), (&inside, false)]),
        Vec::<(usize, usize)>::new()
    );
    // Ends 0.0005 mm apart: read open, the first and last segments touch (nothing joins them);
    // read closed, the tiny closing segment makes them neighbours, as in `0035`.
    assert_eq!(found(&[(&nearly_closed, false)]), vec![(0, 0)]);
    assert_eq!(
        found(&[(&nearly_closed, true)]),
        Vec::<(usize, usize)>::new()
    );
    let u = u_shape();
    assert_eq!(found(&[(&u, false)]), Vec::<(usize, usize)>::new());
}

// ------------------------------------------------------------------ depth (7)

/// 7b: a square touching the inside of the left edge of another is a hole.
#[test]
fn c7b_touching_from_inside_is_a_hole() {
    let all = [rect(0.0, 0.0, 40.0, 40.0), rect(0.0, 10.0, 20.0, 30.0)];
    assert_eq!(depths(&all), vec![0, 1]);
    let swapped = [all[1].clone(), all[0].clone()];
    assert_eq!(depths(&swapped), vec![1, 0]);
}

/// 7c: crossing squares do not contain each other.
#[test]
fn c7c_crossing_squares_are_both_shapes() {
    let all = [rect(0.0, 0.0, 20.0, 20.0), rect(10.0, 10.0, 30.0, 30.0)];
    assert_eq!(depths(&all), vec![0, 0]);
    // Their painted area as one compound path is 700 mm², the picture before Combine.
    let outlines = closed(&all);
    let union = boolean(BooleanOp::Union, &[&outlines], TOL).unwrap();
    assert!((area(&union) - 700.0).abs() < 1.0, "{}", area(&union));
}

/// 7d: coincident squares are both shapes.
#[test]
fn c7d_coincident_squares_are_both_shapes() {
    let all = [rect(0.0, 0.0, 20.0, 20.0), rect(0.0, 0.0, 20.0, 20.0)];
    assert_eq!(depths(&all), vec![0, 0]);
    let opposite = [all[0].clone(), reversed(&all[0])];
    assert_eq!(depths(&opposite), vec![0, 0]);
}

/// 7e: a figure eight on its own has depth 0.
#[test]
fn c7e_a_figure_eight_is_a_shape() {
    let eight = polygon(&[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)]);
    assert_eq!(depths(&[eight]), vec![0]);
}

/// 7g: two squares crossing each other inside a third are both holes (depth 1).
#[test]
fn c7g_crossing_holes_inside_a_shape_are_both_holes() {
    let all = [
        rect(0.0, 0.0, 40.0, 40.0),
        rect(5.0, 5.0, 25.0, 25.0),
        rect(15.0, 15.0, 35.0, 35.0),
    ];
    assert_eq!(depths(&all), vec![0, 1, 1]);
}

/// A concave A whose notch runs across B between B's nodes does not contain B, although every
/// node of B lies inside A.
#[test]
fn a_rectangle_across_a_notch_is_not_contained() {
    let notched = polygon(&[
        (0.0, 0.0),
        (30.0, 0.0),
        (30.0, 30.0),
        (20.0, 30.0),
        (20.0, 10.0),
        (10.0, 10.0),
        (10.0, 30.0),
        (0.0, 30.0),
    ]);
    let across = rect(5.0, 20.0, 25.0, 28.0);
    assert_eq!(depths(&[notched.clone(), across]), vec![0, 0]);
    // The same rectangle touching the notch's floor from below, inside the base, is a hole.
    let below = rect(5.0, 2.0, 25.0, 10.0);
    assert_eq!(depths(&[notched, below]), vec![0, 1]);
}

/// A circle touching a larger one from inside is a hole; touching from outside it is a shape.
#[test]
fn curves_touching_from_inside_and_outside() {
    let big = circle(0.0, 0.0, 20.0);
    assert_eq!(depths(&[big.clone(), circle(10.0, 0.0, 10.0)]), vec![0, 1]);
    assert_eq!(depths(&[big, circle(30.0, 0.0, 10.0)]), vec![0, 0]);
}

/// Open outlines have no depth and contain nothing.
#[test]
fn open_outlines_have_no_depth_and_contain_nothing() {
    let around = polygon(&[(0.0, 0.0), (0.0, 40.0), (40.0, 40.0), (40.0, 0.0)]);
    let square = rect(10.0, 10.0, 20.0, 20.0);
    let plate = rect(-10.0, -10.0, 50.0, 50.0);
    let outlines = [
        Outline::new(&around, false),
        Outline::new(&square, true),
        Outline::new(&plate, true),
    ];
    let depths: Vec<usize> = outline_nesting(&outlines).iter().map(|n| n.depth).collect();
    assert_eq!(depths, vec![0, 1, 0], "the open U is in no depth count");
}

/// Criterion 33: the nesting of touching and crossing outlines is the same every time.
#[test]
fn nesting_of_touching_outlines_is_deterministic() {
    let all = [
        circle(0.0, 0.0, 20.0),
        circle(10.0, 0.0, 10.0),
        rect(-5.0, -5.0, 25.0, 5.0),
    ];
    assert_eq!(
        outline_nesting(&closed(&all)),
        outline_nesting(&closed(&all))
    );
}

// ------------------------------------------------------------------ fill area (3)

/// Criterion 3: the U alone paints (5, 5); inside a square run the same way (20, 20) is painted
/// (winding 2), run the other way it is not (winding 0).
#[test]
fn c3_the_fill_area_closes_an_open_outline_and_follows_its_direction() {
    let u = u_shape();
    assert!(contains_point_in_outlines(
        &[Outline::new(&u, false)],
        Point::new(5.0, 5.0)
    ));
    let inner = shifted(&u, 15.0, 15.0);
    // Same direction as `inner` (both have the same sign of area).
    let same = polygon(&[(0.0, 0.0), (0.0, 40.0), (40.0, 40.0), (40.0, 0.0)]);
    let other = reversed(&same);
    let fill = |square: &Anchors| {
        contains_point_in_outlines(
            &[Outline::new(square, true), Outline::new(&inner, false)],
            Point::new(20.0, 20.0),
        )
    };
    assert!(fill(&same), "run alike: winding 2, painted");
    assert!(!fill(&other), "run the other way: winding 0, not painted");
}
