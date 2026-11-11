//! Tester acceptance for `specs/0110-pen-modes-variable-stroke` criterion 9: the profile and
//! B-spline rules of ADR 0053, checked against references written here, not against the
//! implementation.
#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names
)]

use curvyo_document_core::Point;
use curvyo_geometry_core::{
    SplineControl, SplineCubic, SplineError, WidthPoint, WidthProfile, evaluate_bspline,
};
use proptest::prelude::*;

const TOL: f64 = 1e-9;

fn wp(id: u128, position: f64, multiplier: f64) -> WidthPoint {
    WidthPoint::new(id, position, multiplier, false).unwrap()
}

fn wpc(id: u128, position: f64, multiplier: f64) -> WidthPoint {
    WidthPoint::new(id, position, multiplier, true).unwrap()
}

fn smooth(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * (3.0 * t * t - 2.0 * t * t * t)
}

// ---------- profile ----------

#[test]
fn c9_empty_profile_is_uniform_one() {
    let p = WidthProfile::new(vec![], false).unwrap();
    for at in [0.0, 0.3, 1.0] {
        assert!((p.sample(at).unwrap() - 1.0).abs() < TOL);
    }
}

/// Open point from the lead: one width point makes the whole stroke uniform at its multiplier
/// (ADR 0053 implementation notes, "one point is uniform at that point's multiplier").
#[test]
fn c9_one_point_is_uniform_at_its_multiplier_everywhere() {
    for closed in [false, true] {
        let point = WidthPoint::new(7, 0.3, 2.5, closed).unwrap();
        let p = WidthProfile::new(vec![point], closed).unwrap();
        for at in [0.0, 0.1, 0.3, 0.6, 0.99] {
            assert!((p.sample(at).unwrap() - 2.5).abs() < TOL, "{closed} {at}");
        }
        assert_eq!(p.constant_multiplier(), Some(2.5));
    }
}

#[test]
fn c9_smoothstep_between_points_and_endpoint_extension_on_open_paths() {
    let p = WidthProfile::new(vec![wp(1, 0.2, 0.5), wp(2, 0.6, 3.0)], false).unwrap();
    for t in [0.0, 0.1, 0.25, 0.5, 0.9, 1.0] {
        let at = 0.2 + 0.4 * t;
        assert!(
            (p.sample(at).unwrap() - smooth(0.5, 3.0, t)).abs() < TOL,
            "{at}"
        );
    }
    assert!((p.sample(0.0).unwrap() - 0.5).abs() < TOL);
    assert!((p.sample(0.1).unwrap() - 0.5).abs() < TOL);
    assert!((p.sample(0.8).unwrap() - 3.0).abs() < TOL);
    assert!((p.sample(1.0).unwrap() - 3.0).abs() < TOL);
}

#[test]
fn c9_closed_profile_interpolates_across_the_seam() {
    // Last point at 0.8 (m 2), first at 0.2 (m 0): the seam interval is 0.8 .. 1.2.
    let p = WidthProfile::new(vec![wpc(1, 0.2, 0.0), wpc(2, 0.8, 2.0)], true).unwrap();
    for at in [0.85, 0.9, 0.95, 0.0, 0.05, 0.1, 0.15] {
        let shifted = if at < 0.2 { at + 1.0 } else { at };
        let t = (shifted - 0.8) / 0.4;
        assert!(
            (p.sample(at).unwrap() - smooth(2.0, 0.0, t)).abs() < TOL,
            "{at}"
        );
    }
    assert!((p.sample(0.0).unwrap() - 1.0).abs() < TOL, "seam is midway");
}

#[test]
fn c9_duplicate_positions_take_the_greatest_identifier_regardless_of_input_order() {
    for order in [[1u128, 9, 4], [9, 4, 1], [4, 1, 9]] {
        let points = order
            .iter()
            .map(|&id| wp(id, 0.5, id as f64))
            .collect::<Vec<_>>();
        let p = WidthProfile::new(points, false).unwrap();
        assert!((p.sample(0.5).unwrap() - 9.0).abs() < TOL, "{order:?}");
    }
}

#[test]
fn c9_invalid_points_are_rejected_not_clamped() {
    for (position, multiplier) in [
        (-0.01, 1.0),
        (1.01, 1.0),
        (f64::NAN, 1.0),
        (0.5, -0.1),
        (0.5, f64::NAN),
        (0.5, f64::INFINITY),
    ] {
        assert!(
            WidthPoint::new(1, position, multiplier, false).is_none(),
            "{position} {multiplier}"
        );
    }
    assert!(
        WidthPoint::new(1, 1.0, 1.0, false).is_some(),
        "open includes 1"
    );
    assert!(
        WidthPoint::new(1, 1.0, 1.0, true).is_none(),
        "closed is [0, 1)"
    );
    assert!(
        WidthPoint::new(1, 0.0, 0.0, true).is_some(),
        "zero width is valid"
    );
}

proptest! {
    #[test]
    fn c9_profile_never_overshoots_or_goes_negative(
        raw in proptest::collection::vec((0.0f64..1.0, 0.0f64..10.0), 1..8),
        closed in any::<bool>(),
        at in 0.0f64..1.0,
    ) {
        let points: Vec<_> = raw
            .iter()
            .enumerate()
            .map(|(i, &(pos, m))| WidthPoint::new(i as u128, pos, m, closed).unwrap())
            .collect();
        let lo = raw.iter().map(|r| r.1).fold(f64::INFINITY, f64::min);
        let hi = raw.iter().map(|r| r.1).fold(0.0, f64::max);
        let p = WidthProfile::new(points, closed).unwrap();
        let v = p.sample(at).unwrap();
        prop_assert!(v.is_finite());
        prop_assert!(v >= lo - 1e-9 && v <= hi + 1e-9, "{v} not in [{lo}, {hi}]");
        prop_assert!(p.max_multiplier() >= v - 1e-9);
    }
}

// ---------- B-spline reference ----------

fn c(x: f64, y: f64) -> SplineControl {
    SplineControl {
        point: Point::new(x, y),
        corner: false,
    }
}

fn corner(x: f64, y: f64) -> SplineControl {
    SplineControl {
        point: Point::new(x, y),
        corner: true,
    }
}

fn bez(s: &SplineCubic, t: f64) -> (f64, f64) {
    let u = 1.0 - t;
    let (a, b, cc, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    (
        a * s.start.x + b * s.control1.x + cc * s.control2.x + d * s.end.x,
        a * s.start.y + b * s.control1.y + cc * s.control2.y + d * s.end.y,
    )
}

/// De Boor on an open clamped uniform knot vector of degree `min(3, n - 1)`.
fn de_boor_open(points: &[(f64, f64)], u: f64) -> (f64, f64) {
    let n = points.len();
    let p = (n - 1).min(3);
    let spans = n - p;
    let mut knots = vec![0.0; p + 1];
    knots.extend((1..spans).map(|k| k as f64));
    knots.extend(std::iter::repeat_n(spans as f64, p + 1));
    let mut k = p;
    while k + 1 < n && u >= knots[k + 1] {
        k += 1;
    }
    let mut d: Vec<(f64, f64)> = (0..=p).map(|j| points[j + k - p]).collect();
    for r in 1..=p {
        for j in (r..=p).rev() {
            let i = j + k - p;
            let denom = knots[i + p + 1 - r] - knots[i];
            let a = if denom == 0.0 {
                0.0
            } else {
                (u - knots[i]) / denom
            };
            d[j] = (
                (1.0 - a) * d[j - 1].0 + a * d[j].0,
                (1.0 - a) * d[j - 1].1 + a * d[j].1,
            );
        }
    }
    d[p]
}

fn assert_open_matches(points: &[(f64, f64)]) {
    let controls: Vec<_> = points.iter().map(|&(x, y)| c(x, y)).collect();
    let spans = evaluate_bspline(&controls, false).unwrap();
    let p = (points.len() - 1).min(3);
    assert_eq!(spans.len(), points.len() - p, "{points:?}");
    for (i, span) in spans.iter().enumerate() {
        for step in 0..=10 {
            let t = f64::from(step) / 10.0;
            let got = bez(span, t);
            let want = de_boor_open(points, i as f64 + t);
            assert!(
                (got.0 - want.0).abs() < 1e-6 && (got.1 - want.1).abs() < 1e-6,
                "{points:?} span {i} t {t}: {got:?} vs {want:?}"
            );
        }
    }
}

#[test]
fn c9_open_bspline_two_controls_is_the_line() {
    assert_open_matches(&[(0.0, 0.0), (10.0, 4.0)]);
}

#[test]
fn c9_open_bspline_three_controls_is_the_quadratic() {
    assert_open_matches(&[(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)]);
    let spans = evaluate_bspline(&[c(0.0, 0.0), c(10.0, 10.0), c(20.0, 0.0)], false).unwrap();
    let mid = bez(&spans[0], 0.5);
    assert!((mid.0 - 10.0).abs() < 1e-9 && (mid.1 - 5.0).abs() < 1e-9);
}

#[test]
fn c9_open_bspline_many_controls_match_de_boor_and_are_clamped() {
    let pts = [
        (0.0, 0.0),
        (5.0, 12.0),
        (11.0, -3.0),
        (20.0, 8.0),
        (24.0, 1.0),
        (31.0, 9.0),
        (40.0, 0.0),
    ];
    assert_open_matches(&pts);
    let controls: Vec<_> = pts.iter().map(|&(x, y)| c(x, y)).collect();
    let spans = evaluate_bspline(&controls, false).unwrap();
    assert_eq!(spans[0].start, Point::new(0.0, 0.0));
    assert_eq!(spans.last().unwrap().end, Point::new(40.0, 0.0));
}

#[test]
fn c9_closed_bspline_is_periodic_with_n_spans_and_a_smooth_seam() {
    let pts = [
        (0.0, 0.0),
        (20.0, 0.0),
        (25.0, 15.0),
        (5.0, 22.0),
        (-6.0, 9.0),
    ];
    let n = pts.len();
    let controls: Vec<_> = pts.iter().map(|&(x, y)| c(x, y)).collect();
    let spans = evaluate_bspline(&controls, true).unwrap();
    assert_eq!(spans.len(), n);
    for (i, span) in spans.iter().enumerate() {
        let q = |k: isize| pts[(i as isize + k).rem_euclid(n as isize) as usize];
        for step in 0..=8 {
            let t = f64::from(step) / 8.0;
            let b = [
                (1.0 - t).powi(3) / 6.0,
                (3.0 * t.powi(3) - 6.0 * t * t + 4.0) / 6.0,
                (-3.0 * t.powi(3) + 3.0 * t * t + 3.0 * t + 1.0) / 6.0,
                t.powi(3) / 6.0,
            ];
            let want = (0..4).fold((0.0, 0.0), |acc, j| {
                let p = q(j as isize - 1);
                (acc.0 + b[j] * p.0, acc.1 + b[j] * p.1)
            });
            let got = bez(span, t);
            assert!(
                (got.0 - want.0).abs() < 1e-6 && (got.1 - want.1).abs() < 1e-6,
                "span {i} t {t}"
            );
        }
    }
    let first = spans[0].start;
    assert!((first.x - (pts[n - 1].0 + 4.0 * pts[0].0 + pts[1].0) / 6.0).abs() < 1e-9);
    assert!((first.y - (pts[n - 1].1 + 4.0 * pts[0].1 + pts[1].1) / 6.0).abs() < 1e-9);
    // Tangent continuity at the seam: last span's outgoing handle and first span's incoming one
    // are collinear with the seam point and of equal length (C1 for the uniform periodic spline).
    let last = spans[n - 1];
    let din = (last.end.x - last.control2.x, last.end.y - last.control2.y);
    let dout = (first.x - spans[0].control1.x, first.y - spans[0].control1.y);
    assert!((din.0 + dout.0).abs() < 1e-9 && (din.1 + dout.1).abs() < 1e-9);
}

#[test]
fn c9_closed_bspline_with_fewer_than_three_controls_is_refused() {
    for controls in [vec![c(0.0, 0.0)], vec![c(0.0, 0.0), c(10.0, 0.0)]] {
        assert!(matches!(
            evaluate_bspline(&controls, true),
            Err(SplineError::TooFewClosed)
        ));
    }
}

#[test]
fn c9_a_corner_splits_the_open_curve_into_runs_meeting_exactly_at_the_corner() {
    // Run 1: [A, B] is a line; run 2: [B, C, D] is a quadratic.
    let a = (0.0, 0.0);
    let b = (10.0, 0.0);
    let cc = (20.0, 10.0);
    let d = (30.0, 0.0);
    let spans = evaluate_bspline(
        &[c(a.0, a.1), corner(b.0, b.1), c(cc.0, cc.1), c(d.0, d.1)],
        false,
    )
    .unwrap();
    assert_eq!(spans.len(), 2, "{spans:?}");
    assert_eq!(spans[0].end, Point::new(b.0, b.1));
    assert_eq!(spans[1].start, Point::new(b.0, b.1));
    let mid_line = bez(&spans[0], 0.5);
    assert!((mid_line.0 - 5.0).abs() < 1e-9 && mid_line.1.abs() < 1e-9);
    let mid_quad = bez(&spans[1], 0.5);
    assert!((mid_quad.0 - 20.0).abs() < 1e-9 && (mid_quad.1 - 5.0).abs() < 1e-9);
}

#[test]
fn c9_closed_bspline_with_one_corner_starts_and_ends_at_that_corner() {
    let spans = evaluate_bspline(
        &[corner(0.0, 0.0), c(20.0, 0.0), c(20.0, 20.0), c(0.0, 20.0)],
        true,
    )
    .unwrap();
    assert_eq!(spans[0].start, Point::new(0.0, 0.0));
    assert_eq!(spans.last().unwrap().end, Point::new(0.0, 0.0));
    for pair in spans.windows(2) {
        assert_eq!(pair[0].end, pair[1].start);
    }
    // The wrap run [P0, P1, P2, P3, P0] is one open cubic run over five controls: two spans.
    assert_open_matches(&[
        (0.0, 0.0),
        (20.0, 0.0),
        (20.0, 20.0),
        (0.0, 20.0),
        (0.0, 0.0),
    ]);
    assert_eq!(spans.len(), 2);
}

#[test]
fn c9_nonfinite_controls_are_refused() {
    assert!(evaluate_bspline(&[c(0.0, 0.0), c(f64::NAN, 1.0), c(3.0, 3.0)], false).is_err());
    assert!(evaluate_bspline(&[c(0.0, 0.0), c(f64::INFINITY, 1.0)], false).is_err());
}
