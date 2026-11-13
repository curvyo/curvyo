//! Tester acceptance for `specs/0115-spiro-smoothing-and-width-handles` criterion 1: a Spiro is
//! the curvature-continuous curve of Inkscape's Spiro mode, clothoid segments between smooth
//! points and circular arcs at open ends and beside corners; three points give their circle.
//! Written from the specification, with properties of the curve itself (not of its coefficients).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

use curvyo_document_core::Point;
use curvyo_geometry_core::{SpiroControl, SpiroCubic, Tolerance, evaluate_spiro};
use proptest::prelude::*;

const TOL: f64 = 1e-5;

fn smooth(points: &[(f64, f64)]) -> Vec<SpiroControl> {
    points
        .iter()
        .map(|&(x, y)| SpiroControl {
            point: Point::new(x, y),
            corner: false,
        })
        .collect()
}

fn bez(c: &SpiroCubic, t: f64) -> (f64, f64) {
    let u = 1.0 - t;
    let (a, b, cc, d) = (c.start, c.control1, c.control2, c.end);
    let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    (
        w[0] * a.x + w[1] * b.x + w[2] * cc.x + w[3] * d.x,
        w[0] * a.y + w[1] * b.y + w[2] * cc.y + w[3] * d.y,
    )
}

/// The curve as a fine polyline with cumulative arc length, and the arc length at each control
/// (the ends of the spans that end on a control).
struct Curve {
    pts: Vec<(f64, f64)>,
    len: Vec<f64>,
    knots: Vec<f64>,
}

fn curve(controls: &[SpiroControl], closed: bool) -> Curve {
    let cubics = evaluate_spiro(controls, closed, Tolerance::from_mm(TOL)).expect("solves");
    let mut pts = vec![(cubics[0].start.x, cubics[0].start.y)];
    let mut len = vec![0.0];
    let mut knots = vec![0.0];
    for c in &cubics {
        let n = 4000;
        for i in 1..=n {
            let p = bez(c, f64::from(i) / f64::from(n));
            let q = *pts.last().unwrap();
            len.push(len.last().unwrap() + (p.0 - q.0).hypot(p.1 - q.1));
            pts.push(p);
        }
        if controls
            .iter()
            .any(|k| (k.point.x - c.end.x).hypot(k.point.y - c.end.y) < 1e-6)
        {
            knots.push(*len.last().unwrap());
        }
    }
    Curve { pts, len, knots }
}

impl Curve {
    fn at(&self, s: f64) -> (f64, f64) {
        let i = self
            .len
            .partition_point(|&l| l < s)
            .clamp(1, self.len.len() - 1);
        let (l0, l1) = (self.len[i - 1], self.len[i]);
        let f = if l1 > l0 { (s - l0) / (l1 - l0) } else { 0.0 };
        let (a, b) = (self.pts[i - 1], self.pts[i]);
        (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
    }

    /// Signed curvature (1/mm) at arc length `s` from three points `h` apart.
    fn kappa(&self, s: f64, h: f64) -> f64 {
        let (a, b, c) = (self.at(s - h), self.at(s), self.at(s + h));
        let cross = (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0);
        let (ab, bc, ca) = (
            (b.0 - a.0).hypot(b.1 - a.1),
            (c.0 - b.0).hypot(c.1 - b.1),
            (a.0 - c.0).hypot(a.1 - c.1),
        );
        2.0 * cross / (ab * bc * ca)
    }

    /// Curvature samples inside segment `i` (between knots `i` and `i + 1`), away from its ends.
    fn segment_kappa(&self, i: usize) -> Vec<(f64, f64)> {
        let (a, b) = (self.knots[i], self.knots[i + 1]);
        let h = 0.2;
        (1..20)
            .map(|j| {
                let s = a + (b - a) * f64::from(j) / 20.0;
                (s, self.kappa(s, h))
            })
            .filter(|&(s, _)| s - h > a && s + h < b)
            .collect()
    }

    /// Curvature just left and right of knot `i`, extrapolated linearly to the knot.
    fn kappa_sides(&self, i: usize) -> (f64, f64) {
        let s = self.knots[i];
        let h = 0.2;
        let l1 = self.kappa(s - 2.0 * h, h);
        let l2 = self.kappa(s - 4.0 * h, h);
        let r1 = self.kappa(s + 2.0 * h, h);
        let r2 = self.kappa(s + 4.0 * h, h);
        (2.0f64.mul_add(l1, -l2), 2.0f64.mul_add(r1, -r2))
    }
}

/// Least-squares line through `(s, k)`; returns the largest residual.
fn line_residual(samples: &[(f64, f64)]) -> f64 {
    let n = samples.len() as f64;
    let ms = samples.iter().map(|p| p.0).sum::<f64>() / n;
    let mk = samples.iter().map(|p| p.1).sum::<f64>() / n;
    let sxx: f64 = samples.iter().map(|p| (p.0 - ms).powi(2)).sum();
    let sxy: f64 = samples.iter().map(|p| (p.0 - ms) * (p.1 - mk)).sum();
    let slope = if sxx > 0.0 { sxy / sxx } else { 0.0 };
    samples
        .iter()
        .map(|p| (p.1 - (mk + slope * (p.0 - ms))).abs())
        .fold(0.0, f64::max)
}

fn spread(samples: &[(f64, f64)]) -> f64 {
    let lo = samples.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let hi = samples
        .iter()
        .map(|p| p.1)
        .fold(f64::NEG_INFINITY, f64::max);
    hi - lo
}

fn circumcircle(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> ((f64, f64), f64) {
    let d = 2.0 * (a.0 * (b.1 - c.1) + b.0 * (c.1 - a.1) + c.0 * (a.1 - b.1));
    let sq = |p: (f64, f64)| p.0 * p.0 + p.1 * p.1;
    let ux = (sq(a) * (b.1 - c.1) + sq(b) * (c.1 - a.1) + sq(c) * (a.1 - b.1)) / d;
    let uy = (sq(a) * (c.0 - b.0) + sq(b) * (a.0 - c.0) + sq(c) * (b.0 - a.0)) / d;
    ((ux, uy), (a.0 - ux).hypot(a.1 - uy))
}

// ---------- three points, circles, lines ----------

#[test]
fn three_points_give_their_circle() {
    for pts in [
        [(10.0, 20.0), (30.0, 10.0), (50.0, 20.0)],
        [(0.0, 0.0), (5.0, 12.0), (40.0, 3.0)],
        [(10.0, 10.0), (12.0, 30.0), (14.0, 10.0)],
    ] {
        let (c, r) = circumcircle(pts[0], pts[1], pts[2]);
        let cubics = evaluate_spiro(&smooth(&pts), false, Tolerance::from_mm(TOL)).unwrap();
        for k in &cubics {
            for i in 0..=20 {
                let p = bez(k, f64::from(i) / 20.0);
                let off = ((p.0 - c.0).hypot(p.1 - c.1) - r).abs();
                assert!(off < 1e-3, "{pts:?}: {p:?} is {off:.5} mm off the circle");
            }
        }
    }
}

#[test]
fn a_closed_regular_polygon_gives_its_circle() {
    for n in [3usize, 4, 6, 9] {
        let pts: Vec<_> = (0..n)
            .map(|i| {
                let a = std::f64::consts::TAU * i as f64 / n as f64;
                (50.0 + 20.0 * a.cos(), 40.0 + 20.0 * a.sin())
            })
            .collect();
        let cubics = evaluate_spiro(&smooth(&pts), true, Tolerance::from_mm(TOL)).unwrap();
        for k in &cubics {
            for i in 0..=20 {
                let p = bez(k, f64::from(i) / 20.0);
                let off = ((p.0 - 50.0).hypot(p.1 - 40.0) - 20.0).abs();
                assert!(off < 1e-3, "{n}-gon: {p:?} is {off:.5} mm off");
            }
        }
    }
}

#[test]
fn collinear_controls_give_a_straight_line() {
    let pts = [(0.0, 0.0), (10.0, 5.0), (30.0, 15.0), (32.0, 16.0)];
    let cubics = evaluate_spiro(&smooth(&pts), false, Tolerance::from_mm(TOL)).unwrap();
    for k in &cubics {
        for i in 0..=10 {
            let p = bez(k, f64::from(i) / 10.0);
            assert!((p.1 - p.0 / 2.0).abs() < 1e-4, "{p:?} off the line");
        }
    }
}

// ---------- G2, clothoids, arcs ----------

const WAVE: [(f64, f64); 6] = [
    (10.0, 20.0),
    (30.0, 10.0),
    (50.0, 20.0),
    (70.0, 12.0),
    (85.0, 25.0),
    (100.0, 15.0),
];

#[test]
fn an_open_spiro_is_curvature_continuous_at_every_smooth_point() {
    let c = curve(&smooth(&WAVE), false);
    assert_eq!(c.knots.len(), WAVE.len(), "one knot per control");
    for i in 1..WAVE.len() - 1 {
        let (l, r) = c.kappa_sides(i);
        assert!(
            (l - r).abs() < 2e-3 + 0.02 * l.abs().max(r.abs()),
            "knot {i}: curvature {l:.5} | {r:.5}"
        );
    }
}

#[test]
fn a_closed_spiro_is_curvature_continuous_everywhere() {
    let pts = [
        (10.0, 10.0),
        (40.0, 5.0),
        (60.0, 25.0),
        (45.0, 50.0),
        (15.0, 40.0),
    ];
    let c = curve(&smooth(&pts), true);
    for i in 1..pts.len() {
        let (l, r) = c.kappa_sides(i);
        assert!(
            (l - r).abs() < 2e-3 + 0.02 * l.abs().max(r.abs()),
            "knot {i}: curvature {l:.5} | {r:.5}"
        );
    }
}

#[test]
fn open_ends_are_circular_arcs_and_inner_segments_clothoids() {
    let c = curve(&smooth(&WAVE), false);
    let last = WAVE.len() - 2;
    for i in [0, last] {
        let k = c.segment_kappa(i);
        assert!(
            spread(&k) < 2e-3,
            "end segment {i}: curvature varies by {:.5}",
            spread(&k)
        );
    }
    let mut varying = 0;
    for i in 1..last {
        let k = c.segment_kappa(i);
        assert!(
            line_residual(&k) < 2e-3,
            "segment {i}: curvature not linear in arc length, residual {:.5}",
            line_residual(&k)
        );
        if spread(&k) > 1e-2 {
            varying += 1;
        }
    }
    assert!(
        varying > 0,
        "the inner segments are clothoids, not all arcs"
    );
}

#[test]
fn segments_beside_a_corner_are_circular_arcs() {
    let mut controls = smooth(&WAVE);
    controls[3].corner = true;
    let c = curve(&controls, false);
    // Segment 2 runs into the corner, segment 3 out of it.
    for i in [2, 3] {
        let k = c.segment_kappa(i);
        assert!(
            spread(&k) < 2e-3,
            "segment {i} beside the corner varies by {:.5}",
            spread(&k)
        );
    }
    let k = c.segment_kappa(1);
    assert!(line_residual(&k) < 2e-3, "segment 1 stays a clothoid");
    let (l, r) = c.kappa_sides(2);
    assert!(
        (l - r).abs() < 2e-3 + 0.02 * l.abs().max(r.abs()),
        "G2 at the smooth point 2"
    );
}

// ---------- invariance ----------

fn samples(controls: &[SpiroControl], closed: bool) -> Option<Vec<(f64, f64)>> {
    let cubics = evaluate_spiro(controls, closed, Tolerance::from_mm(1e-4)).ok()?;
    Some(
        cubics
            .iter()
            .flat_map(|k| (0..=16).map(move |i| bez(k, f64::from(i) / 16.0)))
            .collect(),
    )
}

/// One direction of the Hausdorff distance from `a`'s points to `b`'s polyline.
fn reach(a: &[(f64, f64)], b: &[(f64, f64)]) -> f64 {
    a.iter()
        .map(|p| {
            b.windows(2)
                .map(|w| {
                    let (q, r) = (w[0], w[1]);
                    let (dx, dy) = (r.0 - q.0, r.1 - q.1);
                    let l2 = dx * dx + dy * dy;
                    let t = if l2 > 0.0 {
                        (((p.0 - q.0) * dx + (p.1 - q.1) * dy) / l2).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    (p.0 - q.0 - t * dx).hypot(p.1 - q.1 - t * dy)
                })
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max)
}

fn hausdorff(a: &[(f64, f64)], b: &[(f64, f64)]) -> f64 {
    reach(a, b).max(reach(b, a))
}

#[test]
fn reversing_the_controls_draws_the_same_curve() {
    let mut cases: Vec<(Vec<(f64, f64)>, bool)> = vec![
        (WAVE.to_vec(), false),
        (WAVE[..4].to_vec(), false),
        (WAVE[..5].to_vec(), true),
        (
            vec![
                (10.0, 10.0),
                (14.0, 30.0),
                (18.0, 10.0),
                (22.0, 30.0),
                (26.0, 10.0),
            ],
            false,
        ),
    ];
    cases.push((
        vec![
            (50.0, 50.0),
            (90.0, 50.0),
            (90.0, 90.0),
            (55.0, 90.0),
            (55.0, 58.0),
        ],
        false,
    ));
    for (pts, closed) in cases {
        let fwd = samples(&smooth(&pts), closed).expect("solves");
        let mut rev = pts.clone();
        rev.reverse();
        let back = samples(&smooth(&rev), closed).expect("reversed solves");
        let d = hausdorff(&fwd, &back);
        assert!(
            d < 0.01,
            "{pts:?} closed {closed}: reversed curve is {d:.4} mm off"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Moving, turning and scaling the controls moves, turns and scales the curve.
    #[test]
    fn the_curve_follows_a_similarity_of_its_controls(
        pts in prop::collection::vec((0.0f64..100.0, 0.0f64..100.0), 3..7),
        closed in any::<bool>(),
        angle in 0.0f64..std::f64::consts::TAU,
        scale in 0.3f64..3.0,
        dx in -50.0f64..50.0,
        dy in -50.0f64..50.0,
    ) {
        // Controls closer than 2 mm are a different question (coincident points).
        for (i, a) in pts.iter().enumerate() {
            for b in &pts[i + 1..] {
                prop_assume!((a.0 - b.0).hypot(a.1 - b.1) > 2.0);
            }
        }
        let map = |p: (f64, f64)| {
            let (s, c) = angle.sin_cos();
            (scale * (c * p.0 - s * p.1) + dx, scale * (s * p.0 + c * p.1) + dy)
        };
        let Some(base) = samples(&smooth(&pts), closed) else {
            return Ok(());
        };
        let moved: Vec<_> = pts.iter().copied().map(map).collect();
        let Some(image) = samples(&smooth(&moved), closed) else {
            return Err(TestCaseError::fail("the moved controls do not solve"));
        };
        let mapped: Vec<_> = base.into_iter().map(map).collect();
        let d = hausdorff(&mapped, &image);
        prop_assert!(d < 0.02 * scale.max(1.0), "{d:.4} mm off");
    }
}

/// Minimal control sets the similarity property shrank to (2026-11-12): each solves as given and
/// at quarter turns, but not turned by 30 degrees. A turn changes nothing about the curve.
#[test]
fn a_turned_solvable_curve_still_solves() {
    let cases: [(&[(f64, f64)], bool); 2] = [
        (
            &[
                (75.991_728_371_310_8, 80.841_282_166_450_98),
                (58.574_337_347_772_04, 81.667_859_085_433_05),
                (72.133_666_954_016_16, 91.106_284_594_911_57),
                (0.0, 0.0),
            ],
            true,
        ),
        (
            &[
                (54.764_355_622_008_59, 55.660_121_644_920_18),
                (65.935_027_395_251_4, 14.681_108_280_795_25),
                (31.918_975_939_207_936, 58.016_474_083_936_22),
                (16.319_130_877_764_874, 50.242_173_218_370_61),
                (34.334_580_995_234_52, 28.858_846_260_046_224),
                (33.288_150_478_585_84, 34.665_745_679_221_054),
            ],
            false,
        ),
    ];
    for (pts, closed) in cases {
        assert!(
            samples(&smooth(pts), closed).is_some(),
            "{pts:?} solves as given"
        );
        for degrees in [30.0_f64, 45.0, 90.0, 180.0] {
            let (s, c) = degrees.to_radians().sin_cos();
            let turned: Vec<_> = pts
                .iter()
                .map(|&(x, y)| (c * x - s * y, s * x + c * y))
                .collect();
            assert!(
                samples(&smooth(&turned), closed).is_some(),
                "{pts:?} closed {closed} turned {degrees} degrees does not solve"
            );
        }
    }
}
