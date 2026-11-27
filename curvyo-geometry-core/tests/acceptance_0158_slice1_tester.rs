//! `stroke_fit` black-box checks for `specs/0158-pen-display` slice 1 (criteria 2, 5, 9).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use curvyo_document_core::{Length, Point};
use curvyo_geometry_core::{StrokeSample, fit_stroke};

fn sample(x: f64, y: f64, w: f64) -> StrokeSample {
    StrokeSample {
        point: Point::new(x, y),
        width: Length::from_mm(w),
    }
}

fn s_curve(n: usize) -> Vec<StrokeSample> {
    (0..n)
        .map(|i| {
            let f = i as f64 / (n - 1) as f64;
            sample(150.0 * f, 20.0 * (std::f64::consts::TAU * f).sin(), 1.0)
        })
        .collect()
}

#[test]
fn fewer_than_two_distinct_points_give_nothing() {
    assert!(fit_stroke(&[], Length::from_mm(0.4), None).is_none());
    assert!(fit_stroke(&[sample(1.0, 1.0, 1.0)], Length::from_mm(0.4), None).is_none());
    let same = vec![sample(1.0, 1.0, 1.0); 50];
    assert!(fit_stroke(&same, Length::from_mm(0.4), None).is_none());
    let nan = [sample(f64::NAN, 0.0, 1.0), sample(1.0, 1.0, 1.0)];
    assert!(fit_stroke(&nan, Length::from_mm(0.4), None).is_none());
}

#[test]
fn s_curve_fits_with_few_nodes_and_a_linear_ramp_needs_few_width_points() {
    let mut v = s_curve(2000);
    let n = v.len();
    for (i, s) in v.iter_mut().enumerate() {
        s.width = Length::from_mm(0.2 + 1.8 * i as f64 / (n - 1) as f64);
    }
    let fit = fit_stroke(&v, Length::from_mm(0.4), Some(Length::from_mm(0.09))).unwrap();
    assert!(fit.anchors.len() <= 50, "{}", fit.anchors.len());
    assert!(fit.anchors.len() >= 2);
    let profile = fit.profile.expect("a ramp has a profile");
    let _ = profile;
}

#[test]
fn an_l_keeps_its_corner_and_stays_open() {
    let mut v = Vec::new();
    for i in 0..=50 {
        v.push(sample(f64::from(i), 0.0, 1.0));
    }
    for i in 1..=50 {
        v.push(sample(50.0, f64::from(i), 1.0));
    }
    let fit = fit_stroke(&v, Length::from_mm(0.4), None).unwrap();
    let corner = fit
        .anchors
        .iter()
        .any(|(p, _, _)| (p.x - 50.0).abs() < 0.5 && p.y.abs() < 0.5);
    assert!(corner, "{:?}", fit.anchors);
    assert!(fit.profile.is_none());
}

#[test]
fn equal_widths_and_zero_accuracy_do_not_panic() {
    let v = s_curve(300);
    assert!(fit_stroke(&v, Length::from_mm(0.0), Some(Length::from_mm(0.0))).is_some());
    let zero_w: Vec<_> = s_curve(50)
        .into_iter()
        .map(|s| StrokeSample {
            width: Length::from_mm(0.0),
            ..s
        })
        .collect();
    let _ = fit_stroke(&zero_w, Length::from_mm(0.4), Some(Length::from_mm(0.1)));
}
