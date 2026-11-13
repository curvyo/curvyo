//! `specs/0114` criteria 1 and 2: a Spiro is the curvature-continuous curve of Inkscape's Spiro mode
//! (libspiro's `c` points): clothoid segments between smooth controls, circular arcs at open
//! ends and beside corners, always drawn and a function of its controls. The `spiro-inner` port of libspiro returns NaN for open `c` paths
//! (even three points), so these are property tests instead of reference curves (ADR 0053,
//! amendment 2026-11-11).
#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_lossless
)]

use curvyo_document_core::{Point, Tolerance};
use curvyo_geometry_core::{SpiroControl, evaluate_spiro, solve_spiro};

fn controls(points: &[(f64, f64)]) -> Vec<SpiroControl> {
    points
        .iter()
        .map(|&(x, y)| SpiroControl {
            point: Point::new(x, y),
            corner: false,
        })
        .collect()
}

/// The tangent angle (absolute, radians) and curvature (1/mm) at the start and the end of the
/// segment from `a` to `b` with coefficients `k`.
fn ends(a: (f64, f64), b: (f64, f64), k: [f64; 4]) -> [(f64, f64); 2] {
    let mut xy = [0.0; 2];
    spiro_inner::integrate_spiro(k, &mut xy);
    let chord = (b.0 - a.0).hypot(b.1 - a.1);
    let length = chord / xy[0].hypot(xy[1]);
    let rotation = (b.1 - a.1).atan2(b.0 - a.0) - xy[1].atan2(xy[0]);
    let theta =
        |s: f64| k[0] * s + k[1] * s * s / 2.0 + k[2] * s.powi(3) / 6.0 + k[3] * s.powi(4) / 24.0;
    let curvature =
        |s: f64| (k[0] + k[1] * s + k[2] * s * s / 2.0 + k[3] * s.powi(3) / 6.0) / length;
    [
        (rotation + theta(-0.5), curvature(-0.5)),
        (rotation + theta(0.5), curvature(0.5)),
    ]
}

fn wrapped(angle: f64) -> f64 {
    angle - std::f64::consts::TAU * (angle / std::f64::consts::TAU).round()
}

/// Tangent and curvature agree at every smooth knot; an open curve ends in arcs.
fn assert_curvature_continuous(points: &[(f64, f64)], closed: bool) {
    let shape = solve_spiro(&controls(points), closed).unwrap();
    let k = shape.segments();
    let n = points.len();
    let at = |i: usize| ends(points[i], points[(i + 1) % n], k[i]);
    let knots: Vec<usize> = if closed {
        (0..n).collect()
    } else {
        (1..n - 1).collect()
    };
    for knot in knots {
        let before = at((knot + k.len() - 1) % k.len())[1];
        let after = at(knot)[0];
        assert!(
            wrapped(before.0 - after.0).abs() < 1e-6,
            "knot {knot}: tangent"
        );
        assert!((before.1 - after.1).abs() < 1e-6, "knot {knot}: curvature");
    }
    if !closed {
        for end in [k[0], k[k.len() - 1]] {
            assert!(
                end[1..].iter().all(|c| c.abs() < 1e-12),
                "an end segment is an arc: {end:?}"
            );
        }
    }
    for segment in k {
        assert!(
            segment[2].abs() < 1e-12 && segment[3].abs() < 1e-12,
            "a clothoid: {segment:?}"
        );
    }
}

#[test]
fn open_and_closed_curves_are_curvature_continuous_clothoids() {
    for case in 0..12 {
        let points: Vec<_> = (0..5 + case % 3)
            .map(|i| (20.0 * i as f64, 12.0 * ((case * 17 + i * 11) as f64).sin()))
            .collect();
        assert_curvature_continuous(&points, false);
    }
    for case in 0..8 {
        let n = 5 + case % 3;
        let points: Vec<_> = (0..n)
            .map(|i| {
                let angle = std::f64::consts::TAU * i as f64 / n as f64;
                let radius = 40.0 + 6.0 * ((case * 13 + i * 7) as f64).sin();
                (radius * angle.cos(), radius * angle.sin())
            })
            .collect();
        assert_curvature_continuous(&points, true);
    }
}

fn samples(points: &[(f64, f64)], closed: bool) -> Vec<(f64, f64)> {
    evaluate_spiro(&controls(points), closed, Tolerance::from_mm(0.001))
        .unwrap()
        .iter()
        .flat_map(|c| {
            (0..=16).map(move |i| {
                let t = f64::from(i) / 16.0;
                let u = 1.0 - t;
                let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
                let p = [c.start, c.control1, c.control2, c.end];
                (
                    w.iter().zip(&p).map(|(w, q)| w * q.x).sum(),
                    w.iter().zip(&p).map(|(w, q)| w * q.y).sum(),
                )
            })
        })
        .collect()
}

/// Three controls of an open curve: the arc of their circle.
#[test]
fn three_controls_give_their_circle() {
    let points = [(0.0, 0.0), (30.0, 20.0), (60.0, 0.0)];
    // The circle through them: centre (30, y0) with 30² + y0² = (20 - y0)².
    let y0 = (400.0 - 900.0) / 40.0;
    let radius = 30.0_f64.hypot(y0);
    for (x, y) in samples(&points, false) {
        let r = (x - 30.0).hypot(y - y0);
        assert!(
            (r - radius).abs() < 0.01,
            "({x:.2}, {y:.2}) is {r:.4} from the centre"
        );
    }
}

/// Controls on a circle, closed: the circle.
#[test]
fn closed_controls_on_a_circle_give_the_circle() {
    let points: Vec<_> = (0..7)
        .map(|i| {
            let angle = std::f64::consts::TAU * f64::from(i) / 7.0 + 0.3 * f64::from(i % 2);
            (50.0 * angle.cos(), 50.0 * angle.sin())
        })
        .collect();
    for (x, y) in samples(&points, true) {
        assert!((x.hypot(y) - 50.0).abs() < 0.01, "({x:.2}, {y:.2})");
    }
}

/// The curve passes through every control.
#[test]
fn the_curve_passes_through_its_controls() {
    let points = [
        (0.0, 0.0),
        (20.0, 15.0),
        (45.0, -5.0),
        (60.0, 20.0),
        (90.0, 0.0),
    ];
    let curve = samples(&points, false);
    for p in points {
        let d = curve
            .iter()
            .map(|q| (p.0 - q.0).hypot(p.1 - q.1))
            .fold(f64::INFINITY, f64::min);
        assert!(d < 1e-6, "{p:?} is {d} off the curve");
    }
}

fn finite(spans: &[curvyo_geometry_core::SpiroCubic]) -> bool {
    spans.iter().all(|c| {
        [c.start, c.control1, c.control2, c.end]
            .iter()
            .all(|p| p.x.is_finite() && p.y.is_finite())
    })
}

/// `specs/0114` criterion 2: the curve is a function of the controls alone and always drawn. A
/// tight zigzag (whose iteration does not converge in ten steps), three controls nearly on a line
/// and a hairpin each give a finite curve through every control, the same every time.
#[test]
fn hard_controls_give_a_finite_curve_through_them() {
    let zigzag: Vec<_> = (0..10)
        .map(|i| (3.0 * f64::from(i), if i % 2 == 0 { 0.0 } else { 6.0 }))
        .collect();
    let hairpin = vec![(10.0, 10.0), (50.0, 10.0), (20.0, 10.5), (35.0, 30.0)];
    let flat = vec![(0.0, 0.0), (10.0, 1e-9), (20.0, 0.0)];
    for (points, closed) in [
        (zigzag, false),
        (hairpin, false),
        (flat.clone(), false),
        (flat, true),
    ] {
        let spans = evaluate_spiro(&controls(&points), closed, Tolerance::from_mm(0.01)).unwrap();
        assert!(finite(&spans), "{points:?}");
        assert_eq!(
            evaluate_spiro(&controls(&points), closed, Tolerance::from_mm(0.01)).unwrap(),
            spans,
            "{points:?}: the same curve every time"
        );
        for p in &points {
            let d = spans
                .iter()
                .flat_map(|c| [c.start, c.end])
                .map(|q| (q.x - p.0).hypot(q.y - p.1))
                .fold(f64::INFINITY, f64::min);
            assert!(d < 1e-6, "{points:?}: {p:?} is {d} off the curve");
        }
    }
}

/// Coincident neighbours (a point placed on another, a closed curve whose last point is its
/// first) are one point to the curve: it stays smooth and finite through them.
#[test]
fn coincident_controls_are_one_point() {
    let single = [(0.0, 0.0), (20.0, 15.0), (45.0, -5.0), (60.0, 20.0)];
    let doubled = [
        (0.0, 0.0),
        (20.0, 15.0),
        (20.0, 15.0),
        (45.0, -5.0),
        (60.0, 20.0),
    ];
    let near = |a: &[(f64, f64)], b: &[(f64, f64)]| {
        a.iter()
            .map(|p| {
                b.iter()
                    .map(|q| (p.0 - q.0).hypot(p.1 - q.1))
                    .fold(f64::INFINITY, f64::min)
            })
            .fold(0.0, f64::max)
    };
    let (one, two) = (samples(&single, false), samples(&doubled, false));
    assert!(near(&one, &two) < 0.01 && near(&two, &one) < 0.01);
    let ring = [
        (0.0, 0.0),
        (40.0, 0.0),
        (40.0, 30.0),
        (0.0, 30.0),
        (0.0, 0.0),
    ];
    let spans = evaluate_spiro(&controls(&ring), true, Tolerance::from_mm(0.01)).unwrap();
    assert!(finite(&spans));
    let spot = [(5.0, 5.0); 3];
    let spans = evaluate_spiro(&controls(&spot), false, Tolerance::from_mm(0.01)).unwrap();
    assert!(finite(&spans) && spans.iter().all(|c| c.start == c.end));
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(512))]

    /// No controls make the solver panic or draw a value that is not finite: random points,
    /// some on top of each other or a hair apart, open and closed.
    #[test]
    fn any_finite_controls_give_a_finite_curve(
        points in proptest::collection::vec((0.0f64..100.0, 0.0f64..100.0), 2..12),
        repeat in 0usize..12,
        hair in proptest::prelude::any::<bool>(),
        closed in proptest::prelude::any::<bool>(),
    ) {
        let mut points = points;
        if repeat < points.len() {
            let p = points[repeat];
            points.insert(repeat, if hair { (p.0 + 1e-7, p.1) } else { p });
        }
        let result = evaluate_spiro(&controls(&points), closed, Tolerance::from_mm(0.01));
        if closed && points.len() < 3 {
            proptest::prop_assert!(result.is_err());
        } else {
            proptest::prop_assert!(finite(&result.unwrap()));
        }
    }
}
