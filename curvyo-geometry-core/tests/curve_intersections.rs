//! Where contours cut each other (`specs/0036-cut-at-crossings` criteria 2 to 4, 11, 12 and 12a,
//! 13 at kernel level): lines and circles, nodes and inner points of segments, shared stretches,
//! a loop in one cubic, a property test on random cubic pairs, and the budget of the grid.
//! The budget is asserted in release builds only.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::many_single_char_names,
    missing_docs
)]

mod common;

use std::time::{Duration, Instant};

use common::{Anchors, circle, polygon};
use curvyo_document_core::{Point, Tolerance, Vec2};
use curvyo_geometry_core::{
    CutParameter, Outline, cut_parameters, nearest_point_on_segment, subdivide_at_parameter,
};

fn line(a: (f64, f64), b: (f64, f64)) -> Anchors {
    polygon(&[a, b])
}

fn outlines(all: &[(Anchors, bool)]) -> Vec<Outline<'_>> {
    all.iter()
        .map(|(anchors, closed)| Outline::new(anchors, *closed))
        .collect()
}

fn cuts(all: &[(Anchors, bool)]) -> Vec<Vec<CutParameter>> {
    cut_parameters(&outlines(all))
}

/// The point of `anchors` at a cut, as the piece splitter places it.
fn position(anchors: &Anchors, closed: bool, cut: CutParameter) -> Point {
    let count = anchors.len();
    let _ = closed;
    let (from, to) = (
        anchors[cut.segment % count],
        anchors[(cut.segment + 1) % count],
    );
    if cut.t == 0.0 {
        return from.0;
    }
    subdivide_at_parameter(from.0, from.2, to.1, to.0, cut.t).new_point
}

fn near(a: Point, b: Point, tolerance: f64) -> bool {
    (a.x - b.x).hypot(a.y - b.y) <= tolerance
}

fn rotated(anchors: &Anchors, degrees: f64) -> Anchors {
    let (s, c) = degrees.to_radians().sin_cos();
    let turn = |x: f64, y: f64| (x * c - y * s, x * s + y * c);
    anchors
        .iter()
        .map(|&(p, i, o)| {
            let p = turn(p.x, p.y);
            let i = turn(i.x, i.y);
            let o = turn(o.x, o.y);
            (
                Point::new(p.0, p.1),
                Vec2::new(i.0, i.1),
                Vec2::new(o.0, o.1),
            )
        })
        .collect()
}

#[test]
fn two_crossing_lines_meet_at_five_five() {
    let all = [
        (line((0.0, 0.0), (10.0, 10.0)), false),
        (line((0.0, 10.0), (10.0, 0.0)), false),
    ];
    let result = cuts(&all);
    for (list, (anchors, closed)) in result.iter().zip(&all) {
        assert_eq!(list.len(), 1);
        assert!(near(
            position(anchors, *closed, list[0]),
            Point::new(5.0, 5.0),
            0.01
        ));
    }
}

/// The circle's four nodes lie on the line: the cuts are at its nodes, no node is added.
#[test]
fn a_line_through_the_nodes_of_a_circle() {
    let all = [
        (circle(0.0, 0.0, 10.0), true),
        (line((-20.0, 0.0), (40.0, 0.0)), false),
    ];
    let result = cuts(&all);
    assert_eq!(
        result[0],
        vec![
            CutParameter { segment: 0, t: 0.0 },
            CutParameter { segment: 2, t: 0.0 }
        ],
        "cuts at the nodes (10, 0) and (-10, 0)"
    );
    assert_eq!(result[1].len(), 2, "the line is cut at x = -10 and x = 10");
    let xs: Vec<f64> = result[1]
        .iter()
        .map(|cut| position(&all[1].0, false, *cut).x)
        .collect();
    assert!(
        near(Point::new(xs[0], 0.0), Point::new(-10.0, 0.0), 0.01),
        "{xs:?}"
    );
    assert!(
        near(Point::new(xs[1], 0.0), Point::new(10.0, 0.0), 0.01),
        "{xs:?}"
    );
}

/// The same circle turned by 30 degrees: the cuts fall inside segments and stay on the curve.
#[test]
fn a_line_through_a_turned_circle_cuts_inside_segments() {
    let turned = rotated(&circle(0.0, 0.0, 10.0), 30.0);
    let all = [(turned, true), (line((-20.0, 0.0), (20.0, 0.0)), false)];
    let result = cuts(&all);
    assert_eq!(result[0].len(), 2);
    assert!(result[0].iter().all(|cut| cut.t > 0.0 && cut.t < 1.0));
    for cut in &result[0] {
        let at = position(&all[0].0, true, *cut);
        // Four Bézier arcs are a circle to within 3e-4 of the radius.
        assert!(
            (at.x.hypot(at.y) - 10.0).abs() < 5e-3,
            "on the circle: {at:?}"
        );
        assert!(at.y.abs() < 0.01, "on the line: {at:?}");
    }
}

/// E4 of `0048`: two circles of radius 10 whose centres are 10 apart, four open arcs.
#[test]
fn two_overlapping_circles_cross_twice() {
    let all = [
        (circle(0.0, 0.0, 10.0), true),
        (circle(10.0, 0.0, 10.0), true),
    ];
    let result = cuts(&all);
    let crossings = [Point::new(5.0, 8.66), Point::new(5.0, -8.66)];
    for (list, (anchors, closed)) in result.iter().zip(&all) {
        assert_eq!(list.len(), 2, "{list:?}");
        for cut in list {
            let at = position(anchors, *closed, *cut);
            assert!(
                crossings.iter().any(|&c| near(at, c, 0.01)),
                "{at:?} is a crossing"
            );
        }
    }
}

/// The two contours are cut at the very same point (Newton on the crossing), not a flattening
/// tolerance apart.
#[test]
fn the_cuts_of_two_crossing_curves_coincide() {
    let all = [
        (circle(0.0, 0.0, 10.0), true),
        (rotated(&circle(7.0, 3.0, 9.0), 17.0), true),
    ];
    let result = cuts(&all);
    assert_eq!(result[0].len(), 2);
    assert_eq!(result[1].len(), 2);
    for cut in &result[0] {
        let at = position(&all[0].0, true, *cut);
        assert!(
            result[1]
                .iter()
                .any(|other| near(position(&all[1].0, true, *other), at, 1e-9)),
            "{at:?} is not a cut of the second circle too"
        );
    }
}

/// A closed path that crosses itself is cut at each pass through the crossing.
#[test]
fn a_figure_eight_is_cut_at_both_passes_of_its_crossing() {
    let eight = polygon(&[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)]);
    let all = [(eight, true)];
    let result = cuts(&all);
    assert_eq!(result[0].len(), 2);
    assert_eq!(result[0][0].segment, 0);
    assert_eq!(result[0][1].segment, 2);
    for cut in &result[0] {
        assert!(near(
            position(&all[0].0, true, *cut),
            Point::new(5.0, 5.0),
            0.01
        ));
    }
}

#[test]
fn a_t_stem_ends_on_a_line() {
    let all = [
        (line((0.0, 0.0), (10.0, 0.0)), false),
        (line((5.0, 0.0), (5.0, 8.0)), false),
    ];
    let result = cuts(&all);
    assert_eq!(result[0], vec![CutParameter { segment: 0, t: 0.5 }]);
    assert!(result[1].is_empty(), "the stem ends at the cut");
}

#[test]
fn collinear_lines_are_cut_at_the_ends_of_the_shared_stretch() {
    let all = [
        (line((0.0, 0.0), (10.0, 0.0)), false),
        (line((5.0, 0.0), (15.0, 0.0)), false),
    ];
    let result = cuts(&all);
    assert_eq!(result[0], vec![CutParameter { segment: 0, t: 0.5 }]);
    assert_eq!(result[1], vec![CutParameter { segment: 0, t: 0.5 }]);
}

/// Two identical circles share every point: the cuts are at the ends of the shared stretches,
/// which are the nodes, and no cut is made inside the doubled lines.
#[test]
fn two_identical_circles_are_cut_at_their_nodes() {
    let all = [
        (circle(0.0, 0.0, 10.0), true),
        (circle(0.0, 0.0, 10.0), true),
    ];
    let result = cuts(&all);
    for list in &result {
        assert_eq!(list.len(), 4, "{list:?}");
        assert!(list.iter().all(|cut| cut.t == 0.0));
    }
}

/// A cubic with a loop: its two halves cross away from their join.
#[test]
fn a_loop_in_one_cubic_is_found() {
    let loop_curve: Anchors = vec![
        (Point::new(0.0, 0.0), Vec2::ZERO, Vec2::new(10.0, 10.0)),
        (Point::new(10.0, 0.0), Vec2::new(-20.0, 10.0), Vec2::ZERO),
    ];
    let all = [(loop_curve, false)];
    let result = cuts(&all);
    assert_eq!(result[0].len(), 2, "{:?}", result[0]);
    let a = position(&all[0].0, false, result[0][0]);
    let b = position(&all[0].0, false, result[0][1]);
    assert!(near(a, b, 0.01), "{a:?} {b:?}");
    assert!(near(a, Point::new(2.857, 4.286), 0.01), "{a:?}");
}

/// Neighbouring segments of a smooth outline are no crossing.
#[test]
fn a_circle_alone_has_no_cut() {
    let all = [(circle(0.0, 0.0, 10.0), true)];
    assert_eq!(cuts(&all)[0], vec![]);
    let open_arc: Anchors = circle(0.0, 0.0, 10.0).into_iter().take(3).collect();
    assert_eq!(cuts(&[(open_arc, false)])[0], vec![]);
}

/// A zero-length segment (a duplicated node) is no cut.
#[test]
fn a_duplicated_node_is_no_cut() {
    let doubled = polygon(&[
        (0.0, 0.0),
        (10.0, 0.0),
        (10.0, 0.0),
        (10.0, 10.0),
        (0.0, 10.0),
    ]);
    assert_eq!(cuts(&[(doubled, true)])[0], vec![]);
}

/// An open path that doubles back on itself is cut where its end touches it.
#[test]
fn a_path_that_doubles_back_is_cut_at_the_turning_back_end() {
    let back = polygon(&[(0.0, 0.0), (10.0, 0.0), (5.0, 0.0)]);
    let result = cuts(&[(back, false)]);
    assert_eq!(result[0], vec![CutParameter { segment: 0, t: 0.5 }]);
}

/// Criterion 11: the same contours in another order give the same cuts.
#[test]
fn the_order_of_the_contours_does_not_change_the_cuts() {
    let a = (circle(0.0, 0.0, 10.0), true);
    let b = (circle(10.0, 0.0, 10.0), true);
    let c = (line((-20.0, 3.0), (30.0, 3.0)), false);
    let forward = cuts(&[a.clone(), b.clone(), c.clone()]);
    let backward = cuts(&[c, b, a]);
    assert_eq!(forward[0], backward[2]);
    assert_eq!(forward[1], backward[1]);
    assert_eq!(forward[2], backward[0]);
}

// ---------------------------------------------------------------------------------------------
// Property test: random cubic pairs.

fn nearest_distance(anchors: &Anchors, closed: bool, point: Point) -> f64 {
    let count = anchors.len();
    let segments = if closed { count } else { count - 1 };
    (0..segments)
        .map(|k| {
            let (from, to) = (anchors[k], anchors[(k + 1) % count]);
            nearest_point_on_segment(from.0, from.2, to.1, to.0, point, Tolerance::from_mm(1e-6)).1
        })
        .map(curvyo_document_core::Length::as_mm)
        .fold(f64::INFINITY, f64::min)
}

fn cubic_open(p: [(f64, f64); 4]) -> Anchors {
    let at = |x: f64, y: f64| Point::new(x, y);
    vec![
        (
            at(p[0].0, p[0].1),
            Vec2::ZERO,
            Vec2::new(p[1].0 - p[0].0, p[1].1 - p[0].1),
        ),
        (
            at(p[3].0, p[3].1),
            Vec2::new(p[2].0 - p[3].0, p[2].1 - p[3].1),
            Vec2::ZERO,
        ),
    ]
}

mod random_pairs {
    use super::*;
    use proptest::prelude::*;

    fn coordinate() -> impl Strategy<Value = f64> {
        (-200i32..=200).prop_map(|n| f64::from(n) * 0.1)
    }

    /// Four control points with x not decreasing: the curve is a graph over x and has no
    /// self-crossing and no hairpin, so every cut it gets comes from the other curve.
    fn control() -> impl Strategy<Value = [(f64, f64); 4]> {
        (
            proptest::array::uniform4(coordinate()),
            proptest::array::uniform4(coordinate()),
        )
            .prop_map(|(mut xs, ys)| {
                xs.sort_by(f64::total_cmp);
                [
                    (xs[0], ys[0]),
                    (xs[1], ys[1]),
                    (xs[2], ys[2]),
                    (xs[3], ys[3]),
                ]
            })
    }

    proptest! {
        /// Every cut point lies within 0.01 mm of both curves, and the halves a cut makes lie on
        /// the original curve (the cut is made by de Casteljau, so this checks the parameter).
        #[test]
        fn cut_points_lie_on_both_curves(a in control(), b in control()) {
            let all = [(cubic_open(a), false), (cubic_open(b), false)];
            let result = cut_parameters(&outlines(&all));
            for (index, list) in result.iter().enumerate() {
                for cut in list {
                    let at = position(&all[index].0, false, *cut);
                    let other = &all[1 - index];
                    let own = nearest_distance(&all[index].0, false, at);
                    prop_assert!(own <= 1e-6, "off its own curve by {own}");
                    let there = nearest_distance(&other.0, false, at);
                    prop_assert!(there <= 0.01, "{there} from the other curve at {at:?}");
                }
            }
        }
    }
}

/// Brute force: the crossings of the dense polylines of two open cubics, with a clear angle.
fn brute_force_crossings(a: &Anchors, b: &Anchors) -> Vec<Point> {
    let sample = |anchors: &Anchors| -> Vec<Point> {
        let (from, to) = (anchors[0], anchors[1]);
        (0..=600)
            .map(|k| {
                let t = f64::from(k) / 600.0;
                subdivide_at_parameter(from.0, from.2, to.1, to.0, t).new_point
            })
            .collect()
    };
    let (pa, pb) = (sample(a), sample(b));
    let mut found = Vec::new();
    for i in 0..pa.len() - 1 {
        for j in 0..pb.len() - 1 {
            let (p, q) = (pa[i], pa[i + 1]);
            let (r, s) = (pb[j], pb[j + 1]);
            let d = (q.x - p.x, q.y - p.y);
            let e = (s.x - r.x, s.y - r.y);
            let den = d.0 * e.1 - d.1 * e.0;
            let scale = d.0.hypot(d.1) * e.0.hypot(e.1);
            if scale == 0.0 || den.abs() < 0.2 * scale {
                continue;
            }
            let u = ((r.x - p.x) * e.1 - (r.y - p.y) * e.0) / den;
            let v = ((r.x - p.x) * d.1 - (r.y - p.y) * d.0) / den;
            if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
                found.push(Point::new(p.x + d.0 * u, p.y + d.1 * u));
            }
        }
    }
    found
}

mod recall {
    use super::*;
    use proptest::prelude::*;

    fn coordinate() -> impl Strategy<Value = f64> {
        (-200i32..=200).prop_map(|n| f64::from(n) * 0.1)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(120))]

        /// A clear crossing of two cubics found by brute force is a cut on both curves.
        #[test]
        fn clear_crossings_are_found(
            a in proptest::array::uniform4((coordinate(), coordinate())),
            b in proptest::array::uniform4((coordinate(), coordinate())),
        ) {
            let all = [(cubic_open(a), false), (cubic_open(b), false)];
            let result = cut_parameters(&outlines(&all));
            for crossing in brute_force_crossings(&all[0].0, &all[1].0) {
                for index in 0..2 {
                    // A crossing at an end of an open curve is no cut of it (it adds no piece).
                    let anchors = &all[index].0;
                    let at_end = near(anchors[0].0, crossing, 0.05)
                        || near(anchors[1].0, crossing, 0.05);
                    let hit = at_end
                        || result[index].iter().any(|cut| {
                            near(position(anchors, false, *cut), crossing, 0.05)
                        });
                    prop_assert!(hit, "curve {index} is not cut at {crossing:?}: {:?}", result[index]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Budget: 50 horizontal and 50 vertical lines, 2500 crossings.

#[test]
fn a_grid_of_a_hundred_lines() {
    let mut all: Vec<(Anchors, bool)> = Vec::new();
    for k in 0..50 {
        let y = f64::from(k) * 2.0 + 1.0;
        all.push((line((0.0, y), (100.0, y)), false));
    }
    for k in 0..50 {
        let x = f64::from(k) * 2.0 + 1.0;
        all.push((line((x, 0.0), (x, 100.0)), false));
    }
    let started = Instant::now();
    let result = cuts(&all);
    let took = started.elapsed();
    println!("grid of 100 lines: {took:?}");
    for list in &result {
        assert_eq!(list.len(), 50);
    }
    let pieces: usize = result.iter().map(|list| list.len() + 1).sum();
    assert_eq!(pieces, 5100);
    if !cfg!(debug_assertions) {
        assert!(took <= Duration::from_secs(2), "{took:?}");
    }
}

/// Two circles that touch from outside are cut once each, at the touching point (here a node of
/// both): the stretch within 0.001 mm of the other circle is shorter than 0.01 mm.
#[test]
fn tangent_circles_are_cut_once_at_the_touching_point() {
    let all = [
        (circle(0.0, 0.0, 10.0), true),
        (circle(20.0, 0.0, 10.0), true),
    ];
    let result = cuts(&all);
    assert_eq!(result[0], vec![CutParameter { segment: 0, t: 0.0 }]);
    assert_eq!(result[1], vec![CutParameter { segment: 2, t: 0.0 }]);
}
