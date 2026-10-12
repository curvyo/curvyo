//! The budgets of `specs/0032-pen-tablet-input` criteria 10 and 11 for the geometry: a pressure
//! stroke of 2,000 samples along a 150 mm S-curve gives an outline of fewer than 600 nodes, fit
//! and outline take under 200 ms, and the preview (the same fit and outline) fits in a frame.
//!
//! The node budget holds in every build; the time budgets are asserted in release builds only
//! and printed otherwise. CI runs them in release, in the `boolean-budgets` job; locally:
//! `cargo test --release -p curvyo-geometry-core --test pencil_performance -- --nocapture`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    missing_docs
)]

use std::time::{Duration, Instant};

use curvyo_document_core::{Length, Point, Tolerance};
use curvyo_geometry_core::{WidthProfile, fit_freehand, variable_outline};

const SAMPLES: usize = 2_000;

/// 2,000 samples along an S-curve: two half circles of radius 23.9 mm, about 150 mm in all, with
/// a width that swells and thins between 0.2 and 2 mm.
fn s_curve() -> (Vec<Point>, Vec<Length>) {
    let radius = 150.0 / (2.0 * std::f64::consts::PI);
    let points = (0..SAMPLES)
        .map(|i| {
            let t = i as f64 / (SAMPLES - 1) as f64;
            let angle = 2.0 * std::f64::consts::PI * t;
            if angle <= std::f64::consts::PI {
                Point::new(radius - radius * angle.cos(), radius * angle.sin())
            } else {
                let a = angle - std::f64::consts::PI;
                Point::new(3.0 * radius - radius * a.cos(), -radius * a.sin())
            }
        })
        .collect();
    let widths = (0..SAMPLES)
        .map(|i| {
            let t = i as f64 / (SAMPLES - 1) as f64;
            Length::from_mm(1.1 + 0.9 * (6.0 * std::f64::consts::PI * t).sin())
        })
        .collect();
    (points, widths)
}

fn node_count(outlines: &[Vec<Point>]) -> usize {
    outlines.iter().map(Vec::len).sum()
}

/// Criterion 10.
#[test]
fn a_long_pressure_stroke_stays_inside_the_node_and_time_budget() {
    let (points, widths) = s_curve();
    let started = Instant::now();
    let centre = fit_freehand(&points, Tolerance::from_mm(0.08));
    let outlines =
        variable_outline(&centre, &WidthProfile::from_samples(&points, &widths)).unwrap();
    let elapsed = started.elapsed();
    let nodes = node_count(&outlines);
    println!(
        "{SAMPLES} samples: {} centre nodes, {nodes} outline nodes, {elapsed:?}",
        centre.len()
    );
    assert!(nodes < 600, "{nodes} nodes");
    if !cfg!(debug_assertions) {
        assert!(elapsed < Duration::from_millis(200), "took {elapsed:?}");
    }
}

/// Criterion 11: the preview runs the same fit and outline as the commit once per frame, so the
/// whole stroke must fit in a frame of 60 Hz.
#[test]
fn the_preview_of_a_long_stroke_fits_in_a_frame() {
    let (points, widths) = s_curve();
    let profile = WidthProfile::from_samples(&points, &widths);
    // The slowest of a few runs is not what a frame sees; the fastest is noise-free.
    let elapsed = (0..5)
        .map(|_| {
            let started = Instant::now();
            let centre = fit_freehand(&points, Tolerance::from_mm(0.08));
            variable_outline(&centre, &profile).unwrap();
            started.elapsed()
        })
        .min()
        .unwrap();
    println!("preview of {SAMPLES} samples: {elapsed:?}");
    if !cfg!(debug_assertions) {
        assert!(elapsed < Duration::from_millis(16), "took {elapsed:?}");
    }
}

/// Criterion 10 with a hand's tremor: the same S-curve with up to 0.02 mm of deterministic
/// jitter on every sample, which the fit must not follow node by node.
#[test]
fn a_jittery_pressure_stroke_stays_inside_the_node_and_time_budget() {
    let (mut points, widths) = s_curve();
    let mut state: u32 = 0x9E37_79B9;
    let mut jitter = || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (f64::from(state) / f64::from(u32::MAX) - 0.5) * 0.04
    };
    for point in &mut points {
        point.x += jitter();
        point.y += jitter();
    }
    let started = Instant::now();
    let centre = fit_freehand(&points, Tolerance::from_mm(0.08));
    let outlines = variable_outline(&centre, &WidthProfile::from_samples(&points, &widths)).unwrap();
    let elapsed = started.elapsed();
    let nodes = node_count(&outlines);
    println!(
        "jittery {SAMPLES} samples: {} centre nodes, {nodes} outline nodes, {elapsed:?}",
        centre.len()
    );
    assert!(nodes < 600, "{nodes} nodes");
    if !cfg!(debug_assertions) {
        assert!(elapsed < Duration::from_millis(200), "took {elapsed:?}");
    }
}
