//! Tests of the stroke fit (`specs/0158-pen-display` slice 1 criteria 2, 5, 9).

use super::*;

/// A deterministic jitter in `-amplitude..amplitude` (a hand's tremor), from an index.
fn jitter(i: usize, amplitude: f64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let x = ((i as f64) * 12.9898).sin() * 43_758.545_3;
    (x - x.floor() - 0.5) * 2.0 * amplitude
}

/// `count` samples along an S-curve `length` mm long, with a tremor of `tremor` mm across it and
/// widths rising from `widths.0` to `widths.1`.
pub(super) fn s_curve(
    count: usize,
    length: f64,
    tremor: f64,
    widths: (f64, f64),
) -> Vec<StrokeSample> {
    let raw = |t: f64| Point::new(t * 100.0, 20.0 * (std::f64::consts::TAU * t).sin());
    let fine: Vec<Point> = (0..=10_000).map(|k| raw(f64::from(k) / 10_000.0)).collect();
    let total: f64 = fine.windows(2).map(|w| (w[1] - w[0]).hypot()).sum();
    let scale = length / total;
    (0..count)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let t = i as f64 / (count - 1) as f64;
            let on = raw(t);
            let step = 1e-4;
            let along = raw((t + step).min(1.0)) - raw((t - step).max(0.0));
            let normal = kurbo::Vec2::new(-along.y, along.x) / along.hypot();
            let at = on.to_vec2() * scale + normal * jitter(i, tremor);
            StrokeSample {
                point: DocPoint::new(at.x + 20.0, at.y + 60.0),
                width: Length::from_mm(widths.0 + (widths.1 - widths.0) * t),
            }
        })
        .collect()
}

fn fitted_path(fitted: &FittedStroke) -> BezPath {
    let mut path = BezPath::new();
    let at = |p: DocPoint| Point::new(p.x, p.y);
    path.move_to(at(fitted.anchors[0].0));
    for pair in fitted.anchors.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let c1 = Point::new(a.0.x + a.2.x, a.0.y + a.2.y);
        let c2 = Point::new(b.0.x + b.1.x, b.0.y + b.1.y);
        path.curve_to(c1, c2, at(b.0));
    }
    path
}

/// The largest distance of any sample from the fitted centre line (full scan).
pub(super) fn worst_distance(fitted: &FittedStroke, samples: &[StrokeSample]) -> f64 {
    let path = fitted_path(fitted);
    samples
        .iter()
        .map(|s| {
            path.segments()
                .map(|seg| {
                    seg.nearest(Point::new(s.point.x, s.point.y), 1e-9)
                        .distance_sq
                })
                .fold(f64::INFINITY, f64::min)
                .sqrt()
        })
        .fold(0.0, f64::max)
}

#[test]
fn an_s_curve_of_2000_samples_fits_within_the_accuracy_with_few_nodes() {
    let samples = s_curve(2000, 150.0, 0.05, (0.2, 2.0));
    let fitted = fit_stroke(&samples, Length::from_mm(0.4), None).unwrap();
    let worst = worst_distance(&fitted, &samples);
    assert!(worst <= 0.4 + 1e-9, "a sample is {worst} mm away");
    let nodes = fitted.anchors.len();
    // At most 1 node per 3 mm of the 150 mm stroke, which is 50.
    assert!(nodes <= 50, "{nodes} nodes");
    assert!(nodes >= 3, "an S needs more than a line: {nodes}");
    assert!(fitted.profile.is_none());
}

#[test]
fn smoothing_zero_stays_within_the_floor() {
    let samples = s_curve(600, 80.0, 0.02, (1.0, 1.0));
    let fitted = fit_stroke(&samples, Length::from_mm(0.0), None).unwrap();
    let worst = worst_distance(&fitted, &samples);
    assert!(worst <= MIN_ACCURACY_MM + 1e-9, "{worst}");
}

#[test]
fn a_higher_smoothing_never_gives_more_nodes() {
    // Every Smoothing percent in a release build; every fifth in a debug build (time).
    let every = if cfg!(debug_assertions) { 5 } else { 1 };
    for samples in [
        s_curve(500, 50.0, 0.05, (1.0, 1.0)),
        s_curve(400, 40.0, 0.15, (1.0, 1.0)),
    ] {
        let mut last = usize::MAX;
        for step in (0..=100).step_by(every) {
            let accuracy = f64::from(step) / 100.0;
            let fitted = fit_stroke(&samples, Length::from_mm(accuracy), None).unwrap();
            let nodes = fitted.anchors.len();
            assert!(nodes <= last, "{nodes} nodes at {accuracy} mm after {last}");
            assert!(worst_distance(&fitted, &samples) <= accuracy.max(MIN_ACCURACY_MM) + 1e-9);
            last = nodes;
        }
    }
}

#[test]
fn a_drawn_l_keeps_one_corner() {
    let mut samples = Vec::new();
    for i in 0..=500_u16 {
        let t = f64::from(i) / 10.0;
        samples.push(DocPoint::new(10.0 + t, 10.0 + jitter(usize::from(i), 0.05)));
    }
    for i in 1..=500_u16 {
        let t = f64::from(i) / 10.0;
        samples.push(DocPoint::new(
            60.0 + jitter(600 + usize::from(i), 0.05),
            10.0 + t,
        ));
    }
    let samples: Vec<StrokeSample> = samples
        .into_iter()
        .map(|point| StrokeSample {
            point,
            width: Length::from_mm(1.0),
        })
        .collect();
    let fitted = fit_stroke(&samples, Length::from_mm(0.4), None).unwrap();
    let near_corner = fitted
        .anchors
        .iter()
        .filter(|a| distance(a.0, DocPoint::new(60.0, 10.0)) < 1.0)
        .count();
    assert_eq!(near_corner, 1, "{:?}", fitted.anchors);
    assert!(
        fitted.anchors.len() <= 5,
        "{} anchors",
        fitted.anchors.len()
    );
    assert!(worst_distance(&fitted, &samples) <= 0.4 + 1e-9);
}

#[test]
fn the_profile_stays_within_the_width_tolerance_with_few_points() {
    let samples = s_curve(2000, 150.0, 0.05, (0.2, 2.0));
    let bound = 0.05 * (2.0 - 0.2);
    let fitted = fit_stroke(&samples, Length::from_mm(0.4), Some(Length::from_mm(bound))).unwrap();
    let profile = fitted.profile.as_ref().unwrap();
    assert!((fitted.width.as_mm() - 2.0).abs() < 1e-9);
    let measure = crate::PathMeasure::new(&fitted.anchors, false);
    let total = measure.length().as_mm();
    for s in &samples {
        let at = (measure.nearest(s.point).as_mm() / total).clamp(0.0, 1.0);
        let width = profile.sample(at).unwrap() * fitted.width.as_mm();
        // The projection of a sample may differ from the pass's own by the fit error; a 2.0 mm
        // ramp over 150 mm moves 0.012 mm per mm, so 0.4 mm of error adds at most 0.005 mm.
        assert!(
            (width - s.width.as_mm()).abs() <= bound + 0.006,
            "{width} vs {}",
            s.width.as_mm()
        );
    }
    let points = profile.positions().count();
    assert!((2..=20).contains(&points), "{points} width points");
    // Narrower at the start than at the end (criterion 1).
    assert!(profile.sample(0.0).unwrap() < profile.sample(1.0).unwrap());
}

#[test]
fn equal_widths_give_a_flat_profile_and_degenerate_input_none() {
    let samples = s_curve(200, 40.0, 0.0, (1.0, 1.0));
    let fitted = fit_stroke(&samples, Length::from_mm(0.4), Some(Length::from_mm(0.0))).unwrap();
    assert_eq!(fitted.profile.unwrap().constant_multiplier(), Some(1.0));
    let one = StrokeSample {
        point: DocPoint::new(1.0, 1.0),
        width: Length::from_mm(1.0),
    };
    assert!(fit_stroke(&[one, one, one], Length::from_mm(0.4), None).is_none());
    assert!(fit_stroke(&[one], Length::from_mm(0.4), None).is_none());
}
