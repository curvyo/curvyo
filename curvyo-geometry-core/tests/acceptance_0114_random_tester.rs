//! Tester acceptance for `specs/0114-spiro-smoothing-and-width-handles` criteria 1 and 2 on
//! random controls: the Spiro is a pure function of its controls (the same controls give the
//! same curve, whatever was solved before), and any finite controls, open or closed, 2 to 20
//! points, coincident, collinear or far from the origin, give a finite curve through every
//! control without a panic. Written from the spec and the public API of `spiro.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use curvyo_document_core::{Point, Tolerance};
use curvyo_geometry_core::{SpiroControl, SpiroCubic, evaluate_spiro};
use proptest::prelude::*;

const TOL: f64 = 0.01;

fn controls(points: &[(f64, f64)], corners: &[bool]) -> Vec<SpiroControl> {
    points
        .iter()
        .zip(corners.iter().chain(std::iter::repeat(&false)))
        .map(|(&(x, y), &corner)| SpiroControl {
            point: Point::new(x, y),
            corner,
        })
        .collect()
}

fn eval(c: &[SpiroControl], closed: bool) -> Vec<SpiroCubic> {
    evaluate_spiro(c, closed, Tolerance::from_mm(TOL))
        .unwrap_or_else(|e| panic!("{c:?} closed {closed}: {e}"))
}

fn finite(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

/// The spans are finite, chained end to start, and start and end at the controls in order
/// (relative to the size of the coordinates).
fn check_curve(c: &[SpiroControl], closed: bool, spans: &[SpiroCubic]) -> Result<(), String> {
    let scale = c
        .iter()
        .flat_map(|q| [q.point.x.abs(), q.point.y.abs()])
        .fold(1.0, f64::max);
    let eps = 1e-6 * scale;
    let close = |a: Point, b: Point| (a.x - b.x).hypot(a.y - b.y) <= eps;
    if spans.is_empty() {
        return Err("no spans".into());
    }
    for s in spans {
        if ![s.start, s.control1, s.control2, s.end]
            .into_iter()
            .all(finite)
        {
            return Err(format!("non-finite span {s:?}"));
        }
    }
    for w in spans.windows(2) {
        if !close(w[0].end, w[1].start) {
            return Err(format!("gap between {:?} and {:?}", w[0].end, w[1].start));
        }
    }
    if !close(spans[0].start, c[0].point) {
        return Err(format!(
            "starts at {:?}, not {:?}",
            spans[0].start, c[0].point
        ));
    }
    let last = spans.last().unwrap().end;
    let want = if closed {
        c[0].point
    } else {
        c.last().unwrap().point
    };
    if !close(last, want) {
        return Err(format!("ends at {last:?}, not {want:?}"));
    }
    // Every control is the end of some span (or the start of the first).
    for q in c {
        let on = close(spans[0].start, q.point) || spans.iter().any(|s| close(s.end, q.point));
        if !on {
            return Err(format!("does not pass through {:?}", q.point));
        }
    }
    Ok(())
}

/// Whether the spans are the control polygon: every span straight with its handles at the
/// thirds (the fallback of `spiro_cubics`).
fn is_polygon(spans: &[SpiroCubic]) -> bool {
    spans.iter().all(|s| {
        let third = |a: f64, b: f64| a + (b - a) / 3.0;
        let tol = 1e-9 * (1.0 + s.start.x.abs().max(s.start.y.abs()));
        (s.control1.x - third(s.start.x, s.end.x)).abs() <= tol
            && (s.control1.y - third(s.start.y, s.end.y)).abs() <= tol
            && (s.control2.x - third(s.end.x, s.start.x)).abs() <= tol
            && (s.control2.y - third(s.end.y, s.start.y)).abs() <= tol
    })
}

/// Random controls of every hard kind: free, with coincident neighbours, collinear, tiny steps,
/// far from the origin.
fn points() -> impl Strategy<Value = Vec<(f64, f64)>> {
    let free = prop::collection::vec((-100.0f64..100.0, -100.0f64..100.0), 2..=20);
    let coincident = prop::collection::vec(((-50.0f64..50.0, -50.0f64..50.0), 1usize..4), 1..=8)
        .prop_map(|groups| {
            groups
                .into_iter()
                .flat_map(|(p, n)| std::iter::repeat_n(p, n))
                .take(20)
                .collect::<Vec<_>>()
        })
        .prop_filter("2 points at least", |v| v.len() >= 2);
    let collinear = (
        prop::collection::vec(-100.0f64..100.0, 2..=20),
        -3.2f64..3.2,
    )
        .prop_map(|(ts, a)| ts.into_iter().map(|t| (t * a.cos(), t * a.sin())).collect());
    let huge = (
        prop::collection::vec((-100.0f64..100.0, -100.0f64..100.0), 2..=20),
        prop::sample::select(vec![1e4, 1e6, 1e8]),
        prop::sample::select(vec![1e-3, 1.0, 1e3]),
    )
        .prop_map(|(v, off, scale)| {
            v.into_iter()
                .map(|(x, y)| (off + scale * x, -off + scale * y))
                .collect()
        });
    prop_oneof![free, coincident, collinear, huge]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// Criteria 1 and 2: any finite controls give a finite curve through them, and solving
    /// them again, also after other controls, gives the same curve bit for bit.
    #[test]
    fn random_controls_give_a_finite_curve_that_is_a_function_of_them(
        pts in points(),
        corners in prop::collection::vec(prop::bool::weighted(0.15), 20),
        closed in any::<bool>(),
        other in prop::collection::vec((-100.0f64..100.0, -100.0f64..100.0), 3..8),
    ) {
        // A closed curve needs three controls (documented `SpiroError::Invalid`).
        let closed = closed && pts.len() >= 3;
        let c = controls(&pts, &corners);
        let first = eval(&c, closed);
        if let Err(e) = check_curve(&c, closed, &first) {
            return Err(TestCaseError::fail(format!("{c:?} closed {closed}: {e}")));
        }
        // Something else is solved in between: no state carries over.
        let _ = evaluate_spiro(&controls(&other, &[]), !closed, Tolerance::from_mm(TOL));
        let again = eval(&c, closed);
        prop_assert_eq!(&first, &again, "the same controls draw another curve");
    }
}

/// A tiny deterministic generator (no seed from the clock: the run is repeatable).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Criterion 2 ("never the control polygon", measured on the Pen walks) beyond the walks: the
/// share of random smooth control sets 3 to 20 points in a 100 mm square that are drawn as their
/// control polygon, printed for the report, and every one finite.
#[test]
fn random_smooth_controls_rarely_fall_back_to_the_polygon() {
    let mut rng = Lcg(0x0114);
    let (mut total, mut polygon) = (0, 0);
    let mut examples = Vec::new();
    for _ in 0..3000 {
        let n = 3 + (rng.next() * 18.0) as usize;
        let closed = rng.next() < 0.5;
        let pts: Vec<_> = (0..n)
            .map(|_| (100.0 * rng.next(), 100.0 * rng.next()))
            .collect();
        let c = controls(&pts, &[]);
        let spans = eval(&c, closed);
        check_curve(&c, closed, &spans).unwrap();
        total += 1;
        // Collinear sets are straight by right; only bent polygons count.
        if is_polygon(&spans) {
            polygon += 1;
            if examples.len() < 3 {
                examples.push((n, closed, pts.clone()));
            }
        }
    }
    println!(
        "polygon fallback: {polygon} of {total} ({:.1} %); first examples {examples:?}",
        100.0 * f64::from(polygon) / f64::from(total)
    );
}

/// Coincident, collinear and degenerate minimal cases, open and closed: finite, no panic.
#[test]
fn degenerate_control_sets_give_a_finite_curve() {
    let cases: Vec<Vec<(f64, f64)>> = vec![
        vec![(0.0, 0.0), (0.0, 0.0)],
        vec![(0.0, 0.0), (0.0, 0.0), (0.0, 0.0)],
        vec![(0.0, 0.0), (10.0, 0.0), (10.0, 0.0), (20.0, 5.0)],
        vec![(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)],
        vec![(0.0, 0.0), (20.0, 0.0), (10.0, 0.0)],
        vec![(0.0, 0.0), (10.0, 0.0), (0.0, 0.0), (10.0, 0.0)],
        vec![(0.0, 0.0), (1e-9, 0.0), (10.0, 10.0)],
        vec![(1e8, 1e8), (1e8 + 10.0, 1e8), (1e8 + 10.0, 1e8 + 10.0)],
        vec![(-1e8, 1e8), (1e8, -1e8), (1e8, 1e8)],
        vec![(0.0, 0.0), (50.0, 0.0), (20.0, 0.5)],
    ];
    for pts in &cases {
        for closed in [false, true] {
            if closed && pts.len() < 3 {
                assert!(
                    evaluate_spiro(&controls(pts, &[]), true, Tolerance::from_mm(TOL)).is_err()
                );
                continue;
            }
            for corners in [vec![], vec![true; pts.len()]] {
                let c = controls(pts, &corners);
                let spans = eval(&c, closed);
                check_curve(&c, closed, &spans)
                    .unwrap_or_else(|e| panic!("{pts:?} closed {closed}: {e}"));
                assert_eq!(spans, eval(&c, closed), "{pts:?}: deterministic");
            }
        }
    }
}
