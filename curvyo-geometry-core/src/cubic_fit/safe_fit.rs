//! Keeps kurbo's cubic fitter (0.13.1) away from candidate cubics it cannot measure or mismeasures.
//!
//! `kurbo::fit::cubic_fit` has no upper bound on a candidate's arm lengths (its own comment: "We
//! could implement a maximum d value here"). Near a zero denominator a candidate's arms grow
//! without bound; when such a candidate passes the ray test on a "spicy" range,
//! `CurveDist::eval_arc` calls `inv_arclen` with `epsilon = 1e-9 / arclen`. Above about 1e7 mm of arc length the bracket cannot shrink below
//! one ulp and the solver loops for ever; above about 2e10 mm `1u64 << nmax` overflows (a panic
//! in debug builds). The guard recomputes the candidates of every range kurbo is about to fit,
//! and its ray test exactly as `fit_to_cubic` does, and reports a range that would measure an
//! oversized candidate as a cusp, so kurbo splits it instead. The candidate formulas below are kurbo's (MIT/Apache-2.0, the
//! Kurbo Authors), copied so the guard sees what the fitter will see.

use std::ops::Range;

use kurbo::common::{factor_quartic_inner, solve_cubic, solve_quadratic};
use kurbo::simplify::SimplifyBezPath;
use kurbo::{
    Affine, BezPath, CubicBez, CurveFitSample, ParamCurve, ParamCurveFit, Point, Vec2,
    fit_to_bezpath,
};

/// The longest control polygon, in millimetres, a candidate may have: far above any page and
/// far below the arc length at which kurbo's solver stalls (about 1e7 mm).
const MAX_CANDIDATE_MM: f64 = 1e4;

/// A Bézier path as kurbo's fit source at one accuracy, with every range whose candidates
/// kurbo cannot measure reported as a cusp at its middle.
struct GuardedSource {
    curve: SimplifyBezPath,
    accuracy: f64,
}

impl ParamCurveFit for GuardedSource {
    fn sample_pt_tangent(&self, t: f64, sign: f64) -> CurveFitSample {
        self.curve.sample_pt_tangent(t, sign)
    }

    fn sample_pt_deriv(&self, t: f64) -> (Point, Vec2) {
        self.curve.sample_pt_deriv(t)
    }

    fn moment_integrals(&self, range: Range<f64>) -> (f64, f64, f64) {
        self.curve.moment_integrals(range)
    }

    fn break_cusp(&self, range: Range<f64>) -> Option<f64> {
        // `fit_to_bezpath_rec` asks before each `fit_to_cubic`; the splits end at the chord of
        // one accuracy, where kurbo fits a line and computes no candidates.
        unmeasurable(&self.curve, range.clone(), self.accuracy)
            .then_some(f64::midpoint(range.start, range.end))
    }
}

/// `smooth` (a tangent-continuous path) simplified within `accuracy`, as `simplify_bezpath`
/// with its default options does, without the candidates kurbo cannot measure.
pub(crate) fn simplify(smooth: &BezPath, accuracy: f64) -> BezPath {
    if smooth.elements().len() <= 2 {
        return smooth.clone();
    }
    let source = GuardedSource {
        curve: SimplifyBezPath::new(smooth.elements().iter().copied()),
        accuracy,
    };
    fit_to_bezpath(&source, accuracy)
}

/// kurbo's `fit_to_cubic`, or `None` where it would measure an oversized candidate.
pub(crate) fn fit_to_cubic(
    source: &impl ParamCurveFit,
    range: Range<f64>,
    accuracy: f64,
) -> Option<(CubicBez, f64)> {
    if unmeasurable(source, range.clone(), accuracy) {
        return None;
    }
    kurbo::fit_to_cubic(source, range, accuracy)
}

/// Whether `fit_to_cubic(source, range, accuracy)` would take the arc length of a candidate
/// whose control polygon is longer than [`MAX_CANDIDATE_MM`]: a candidate that passes kurbo's
/// ray test on a spicy range (an infinite one never passes it).
// kurbo's own formulas, kept bit for bit so the guard sees the fitter's candidates.
#[allow(clippy::manual_midpoint)]
fn unmeasurable(source: &impl ParamCurveFit, range: Range<f64>, accuracy: f64) -> bool {
    let start = source.sample_pt_tangent(range.start, 1.0);
    let end = source.sample_pt_tangent(range.end, -1.0);
    let d = end.p - start.p;
    let chord2 = d.hypot2();
    let acc2 = accuracy * accuracy;
    if chord2 <= acc2 {
        return false;
    }
    let th = d.atan2();
    let th0 = mod_2pi(start.tangent.atan2() - th);
    let th1 = mod_2pi(th - end.tangent.atan2());
    let (mut area, mut x, mut y) = source.moment_integrals(range.clone());
    let (x0, y0) = (start.p.x, start.p.y);
    let (dx, dy) = (d.x, d.y);
    area -= dx * (y0 + 0.5 * dy);
    let dy_3 = dy * (1. / 3.);
    x -= dx * (x0 * y0 + 0.5 * (x0 * dy + y0 * dx) + dy_3 * dx);
    y -= dx * (y0 * y0 + y0 * dy + dy_3 * dy);
    x -= x0 * area;
    y = 0.5 * y - y0 * area;
    let moment = d.x * x + d.y * y;
    let chord2_inv = chord2.recip();
    let unit_area = area * chord2_inv;
    let mx = moment * chord2_inv.powi(2);
    let chord = chord2.sqrt();
    let huge: Vec<(f64, f64)> = candidate_arms(th0, th1, unit_area, mx)
        .into_iter()
        .filter(|&(d0, d1)| {
            // The control polygon of the unit-chord candidate is at most `1 + 2 (d0 + d1)` long.
            let polygon = (1.0 + 2.0 * (d0 + d1)) * chord;
            polygon.is_finite() && polygon > MAX_CANDIDATE_MM
        })
        .collect();
    if huge.is_empty() {
        return false;
    }
    let samples = Samples::of(source, range);
    if !samples.spicy {
        return false;
    }
    let aff = Affine::translate(start.p.to_vec2()) * Affine::rotate(th) * Affine::scale(chord);
    let (s0, c0) = th0.sin_cos();
    let (s1, c1) = th1.sin_cos();
    huge.into_iter().any(|(d0, d1)| {
        let unit = CubicBez::new(
            (0.0, 0.0),
            (d0 * c0, d0 * s0),
            (1.0 - d1 * c1, d1 * s1),
            (1.0, 0.0),
        );
        samples.ray_passes(aff * unit, acc2)
    })
}

/// kurbo's `N_SAMPLE` of `CurveDist`.
const N_SAMPLE: usize = 20;

/// kurbo's `CurveDist` samples of a range: the inner points and whether the range is spicy.
struct Samples {
    inner: Vec<CurveFitSample>,
    spicy: bool,
}

impl Samples {
    #[allow(clippy::cast_precision_loss)]
    fn of(source: &impl ParamCurveFit, range: Range<f64>) -> Self {
        const SPICY_THRESH: f64 = 0.2;
        let step = (range.end - range.start) * (1.0 / (N_SAMPLE + 1) as f64);
        let mut last_tan: Option<Vec2> = None;
        let mut spicy = false;
        let mut inner = Vec::with_capacity(N_SAMPLE);
        for i in 0..N_SAMPLE + 2 {
            let sample = source.sample_pt_tangent(range.start + i as f64 * step, 1.0);
            if let Some(last) = last_tan {
                let cross = sample.tangent.cross(last);
                let dot = sample.tangent.dot(last);
                if cross.abs() > SPICY_THRESH * dot.abs() {
                    spicy = true;
                }
            }
            last_tan = Some(sample.tangent);
            if i > 0 && i < N_SAMPLE + 1 {
                inner.push(sample);
            }
        }
        Self { inner, spicy }
    }

    /// kurbo's `CurveDist::eval_ray` as a pass or fail.
    fn ray_passes(&self, c: CubicBez, acc2: f64) -> bool {
        let p1 = 3.0 * (c.p1 - c.p0);
        let p2 = 3.0 * c.p2.to_vec2() - 6.0 * c.p1.to_vec2() + 3.0 * c.p0.to_vec2();
        let p3 = (c.p3 - c.p0) - 3.0 * (c.p2 - c.p1);
        let mut max_err2 = 0.0_f64;
        for sample in &self.inner {
            let roots = solve_cubic(
                (c.p0 - sample.p).dot(sample.tangent),
                p1.dot(sample.tangent),
                p2.dot(sample.tangent),
                p3.dot(sample.tangent),
            );
            let mut best = acc2 + 1.0;
            for t in roots.into_iter().filter(|t| (0.0..=1.0).contains(t)) {
                best = best.min(sample.p.distance_squared(c.eval(t)));
            }
            max_err2 = best.max(max_err2);
            if max_err2 > acc2 {
                return false;
            }
        }
        true
    }
}

/// kurbo's `mod_2pi` of `fit_to_cubic`.
fn mod_2pi(th: f64) -> f64 {
    let th_scaled = th * core::f64::consts::FRAC_1_PI * 0.5;
    core::f64::consts::PI * 2.0 * (th_scaled - th_scaled.round())
}

/// The arm lengths `(d0, d1)` of the candidates kurbo's `cubic_fit` returns for a unit chord.
fn candidate_arms(th0: f64, th1: f64, area: f64, mx: f64) -> Vec<(f64, f64)> {
    const EPS: f64 = 1e-12;
    let (s0, c0) = th0.sin_cos();
    let (s1, c1) = th1.sin_cos();
    let a4 = -9.
        * c0
        * (((2. * s1 * c1 * c0 + s0 * (2. * c1 * c1 - 1.)) * c0 - 2. * s1 * c1) * c0
            - c1 * c1 * s0);
    let a3 = 12.
        * ((((c1 * (30. * area * c1 - s1) - 15. * area) * c0 + 2. * s0
            - c1 * s0 * (c1 + 30. * area * s1))
            * c0
            + c1 * (s1 - 15. * area * c1))
            * c0
            - s0 * c1 * c1);
    let a2 = 12.
        * ((((70. * mx + 15. * area) * s1 * s1 + c1 * (9. * s1 - 70. * c1 * mx - 5. * c1 * area))
            * c0
            - 5. * s0 * s1 * (3. * s1 - 4. * c1 * (7. * mx + area)))
            * c0
            - c1 * (9. * s1 - 70. * c1 * mx - 5. * c1 * area));
    let a1 = 16.
        * (((12. * s0 - 5. * c0 * (42. * mx - 17. * area)) * s1
            - 70. * c1 * (3. * mx - area) * s0
            - 75. * c0 * c1 * area * area)
            * s1
            - 75. * c1 * c1 * area * area * s0);
    let a0 = 80. * s1 * (42. * s1 * mx - 25. * area * (s1 - c1 * area));
    let mut roots: Vec<f64> = Vec::new();
    if a4.abs() > EPS {
        let (a, b, c, d) = (a3 / a4, a2 / a4, a1 / a4, a0 / a4);
        if let Some(quads) = factor_quartic_inner(a, b, c, d, false) {
            for (qc1, qc0) in quads {
                let qroots = solve_quadratic(qc0, qc1, 1.0);
                if qroots.is_empty() {
                    roots.push(-0.5 * qc1);
                } else {
                    roots.extend(qroots);
                }
            }
        }
    } else if a3.abs() > EPS {
        roots.extend(solve_cubic(a0, a1, a2, a3));
    } else if a2.abs() > EPS || a1.abs() > EPS || a0.abs() > EPS {
        roots.extend(solve_quadratic(a0, a1, a2));
    } else {
        return vec![(1. / 3., 1. / 3.)];
    }
    let s01 = s0 * c1 + s1 * c0;
    roots
        .into_iter()
        .map(|d0| {
            if d0 > 0.0 {
                let d1 = (d0 * s0 - area * (10. / 3.)) / (0.5 * d0 * s01 - s1);
                if d1 > 0.0 { (d0, d1) } else { (s1 / s01, 0.0) }
            } else {
                (0.0, s0 / s01)
            }
        })
        .filter(|&(d0, d1)| d0 >= 0.0 && d1 >= 0.0)
        .collect()
}

/// A fitted cubic may stray this many times the accuracy from the piece's polyline.
pub(crate) const STRAY: f64 = 3.0;
/// Points sampled on each fitted cubic for the stray test.
const STRAY_SAMPLES: usize = 16;

/// Whether a point of `path` (sampled) lies farther than `limit` from the polyline `points`:
/// kurbo measures a candidate's error by rays from the source only, so it accepts a cubic that
/// runs on past the source's end along a straight run.
pub(crate) fn strays(path: &BezPath, points: &[Point], limit: f64) -> bool {
    use kurbo::ParamCurve;
    let within = |p: Point, k: usize| {
        let (a, b) = (points[k], points[k + 1]);
        let d = b - a;
        let len2 = d.hypot2();
        let t = if len2 > 0.0 {
            ((p - a).dot(d) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (a + d * t - p).hypot() <= limit
    };
    let steps = points.len() - 1;
    // The samples run along the points: search outward from the last step that held one.
    let mut hint: usize = 0;
    path.segments().any(|segment| {
        (1..STRAY_SAMPLES).any(|k| {
            #[allow(clippy::cast_precision_loss)]
            let p = segment.eval(k as f64 / STRAY_SAMPLES as f64);
            let found = (0..steps).find_map(|offset| {
                [hint.checked_add(offset), hint.checked_sub(offset)]
                    .into_iter()
                    .flatten()
                    .find(|&k| k < steps && within(p, k))
            });
            if let Some(k) = found {
                hint = k;
            }
            found.is_none()
        })
    })
}
