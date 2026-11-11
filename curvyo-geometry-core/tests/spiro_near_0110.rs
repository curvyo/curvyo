//! A Spiro curve solved from its earlier shape (`specs/0110-pen-modes-variable-stroke`, customer
//! 2026-11-11): a control moved in small steps follows one solution, a solution far from the
//! earlier shape is refused, and a fresh solve of an open curve grows it point by point.

#![allow(clippy::unwrap_used, missing_docs)]

use curvyo_document_core::{Point, Tolerance};
use curvyo_geometry_core::{SpiroControl, SpiroError, solve_spiro, solve_spiro_near, spiro_cubics};

fn smooth(points: &[(f64, f64)]) -> Vec<SpiroControl> {
    points
        .iter()
        .map(|&(x, y)| SpiroControl {
            point: Point::new(x, y),
            corner: false,
        })
        .collect()
}

fn all_near(shape: &curvyo_geometry_core::SpiroShape) -> Vec<Option<[f64; 4]>> {
    shape.segments().iter().copied().map(Some).collect()
}

#[test]
fn a_dragged_control_follows_one_solution_and_matches_a_fresh_solve() {
    let mut points = vec![(0.0, 0.0), (20.0, 10.0), (40.0, 0.0), (60.0, 10.0)];
    let mut shape = solve_spiro(&smooth(&points), false).unwrap();
    for step in 1..=8 {
        points[2].1 = -f64::from(step);
        let next = solve_spiro_near(&smooth(&points), false, &all_near(&shape), true).unwrap();
        for (a, b) in next.segments().iter().zip(shape.segments()) {
            assert!((a[0] - b[0]).abs() < 0.5, "a small step turns little");
        }
        shape = next;
    }
    let fresh = solve_spiro(&smooth(&points), false).unwrap();
    for (a, b) in shape
        .segments()
        .iter()
        .flatten()
        .zip(fresh.segments().iter().flatten())
    {
        assert!((a - b).abs() < 1e-6);
    }
    let cubics = spiro_cubics(&smooth(&points), false, &shape, Tolerance::from_mm(0.01)).unwrap();
    assert_eq!(cubics.first().unwrap().start, Point::new(0.0, 0.0));
}

#[test]
fn a_shape_of_the_wrong_length_is_refused() {
    let points = [(0.0, 0.0), (20.0, 10.0), (40.0, 0.0), (60.0, 10.0)];
    let shape = solve_spiro(&smooth(&points), false).unwrap();
    assert_eq!(
        solve_spiro_near(&smooth(&points), false, &all_near(&shape)[..2], true),
        Err(SpiroError::Invalid),
        "one entry per segment"
    );
}

/// A drag of a curve that already curls may leave the loop: a fresh solve that sweeps clearly
/// less is taken although it is far from the curl (customer 2026-11-11: fewer odd loops).
#[test]
fn a_drag_leaves_a_curl_for_the_plain_curve() {
    let mut points = vec![(200.0, 200.0), (260.0, 300.0), (330.0, 250.0)];
    let shape = solve_spiro(&smooth(&points), false).unwrap();
    points.push((350.0, 280.0));
    let mut near = all_near(&shape);
    near.push(None);
    let curl = solve_spiro_near(&smooth(&points), false, &near, true).unwrap();
    assert!(curl.segments()[2][0].abs() > 4.0, "a hairpin curls");
    points[3] = (440.0, 150.0);
    let plain = solve_spiro_near(&smooth(&points), false, &all_near(&curl), true).unwrap();
    let fresh = solve_spiro(&smooth(&points), false).unwrap();
    assert_eq!(plain, fresh);
    assert!(plain.segments()[2][0].abs() < 1.0);
}

#[test]
fn an_added_control_starts_straight_and_keeps_the_others() {
    let points = [(0.0, 0.0), (20.0, 10.0), (40.0, 0.0)];
    let shape = solve_spiro(&smooth(&points), false).unwrap();
    let mut longer = points.to_vec();
    longer.push((60.0, 12.0));
    let mut near = all_near(&shape);
    near.push(None);
    let grown = solve_spiro_near(&smooth(&longer), false, &near, true).unwrap();
    assert_eq!(grown.segments().len(), 3);
}

/// Past a fold of the solution family the curve has no solution near its earlier shape: the fresh
/// solve finds one on another branch (looping), and the near solve refuses it.
#[test]
fn past_a_fold_the_other_branch_is_refused() {
    let mut points = vec![(0.0, 0.0), (20.0, 10.0), (40.0, -9.0), (60.0, 10.0)];
    let shape = solve_spiro(&smooth(&points), false).unwrap();
    points[2].1 = -10.0;
    let fresh = solve_spiro(&smooth(&points), false).unwrap();
    assert!(
        (fresh.segments()[0][0] - shape.segments()[0][0]).abs() > 3.0,
        "the fresh solve jumps"
    );
    assert_eq!(
        solve_spiro_near(&smooth(&points), false, &all_near(&shape), true),
        Err(SpiroError::Far)
    );
}

/// A control added at a sharp turn reshapes the segment before it a lot; that is no jump, so it
/// is taken, as long as no segment loops.
#[test]
fn an_added_control_at_a_sharp_turn_reshapes_the_last_segment() {
    let points = [(0.0, 0.0), (20.0, 20.0), (40.0, 0.0)];
    let shape = solve_spiro(&smooth(&points), false).unwrap();
    let mut longer = points.to_vec();
    longer.push((60.0, 0.0));
    let mut near = all_near(&shape);
    near.push(None);
    let grown = solve_spiro_near(&smooth(&longer), false, &near, true).unwrap();
    assert!(
        (grown.segments()[1][0] - shape.segments()[1][0]).abs() > std::f64::consts::FRAC_PI_2,
        "the last segment turned far"
    );
}

/// How far the tangent sweeps along a segment, sampled as the solver compares it.
fn sweep(k: &[f64; 4]) -> f64 {
    let turns: Vec<f64> = [-0.5, -0.25, 0.0, 0.25, 0.5]
        .iter()
        .map(|&s: &f64| {
            k[0] * s + k[1] * s * s / 2.0 + k[2] * s.powi(3) / 6.0 + k[3] * s.powi(4) / 24.0
        })
        .collect();
    let high = turns.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let low = turns.iter().copied().fold(f64::INFINITY, f64::min);
    high - low
}

/// Lead decision 2026-11-11 (Proposal): a curve without a loop never gets one while a point
/// moves. The last point circles its neighbour; every answer sweeps at most 180 degrees per
/// segment, and where only a loop would fit the solve refuses.
#[test]
fn a_curve_without_a_loop_never_gets_one_while_a_point_moves() {
    let mut points = vec![(10.0, 20.0), (30.0, 10.0), (50.0, 20.0), (65.0, 20.0)];
    let mut shape = solve_spiro(&smooth(&points), false).unwrap();
    let mut refused = 0;
    for step in 1..=720 {
        let angle = f64::from(step).to_radians();
        points[3] = (50.0 + 15.0 * angle.cos(), 20.0 + 15.0 * angle.sin());
        match solve_spiro_near(&smooth(&points), false, &all_near(&shape), true) {
            Ok(next) => {
                assert!(
                    next.segments()
                        .iter()
                        .all(|k| sweep(k) <= std::f64::consts::PI),
                    "step {step}: no loop"
                );
                shape = next;
            }
            Err(error) => {
                assert_eq!(error, SpiroError::Far, "step {step}");
                refused += 1;
            }
        }
    }
    assert!(refused > 0, "the circle reaches places only a loop fits");
}
