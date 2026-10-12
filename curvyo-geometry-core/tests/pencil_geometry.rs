//! The geometry of `specs/0032-pen-tablet-input`: the centre line fitted to freehand samples
//! (criterion 7) and the variable-width outline of a pressure stroke (criterion 8).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

use curvyo_document_core::{Length, Point, Tolerance};
use curvyo_geometry_core::{
    OutlineTriple, WidthProfile, fit_freehand, nearest_point_on_segment, signed_area_mm2,
    variable_outline,
};

const FIT_ACCURACY: Tolerance = Tolerance::from_mm(0.08);

/// Distance from `query` to the open path through `anchors`.
fn distance_to_path(anchors: &[OutlineTriple], query: Point) -> f64 {
    anchors
        .windows(2)
        .map(|pair| {
            let (_, distance, _) = nearest_point_on_segment(
                pair[0].0,
                pair[0].2,
                pair[1].1,
                pair[1].0,
                query,
                Tolerance::from_mm(0.000_1),
            );
            distance.as_mm()
        })
        .fold(f64::INFINITY, f64::min)
}

fn assert_samples_within(anchors: &[OutlineTriple], samples: &[Point], limit: f64) {
    for (index, sample) in samples.iter().enumerate() {
        let distance = distance_to_path(anchors, *sample);
        assert!(
            distance <= limit,
            "sample {index} is {distance} mm off the fit"
        );
    }
}

#[test]
fn criterion_7_a_straight_line_of_100_samples_fits_two_nodes() {
    let samples: Vec<Point> = (0..100)
        .map(|i| Point::new(10.0 + 0.5 * i as f64, 20.0 + 0.25 * i as f64))
        .collect();
    let fit = fit_freehand(&samples, FIT_ACCURACY);
    assert_eq!(fit.len(), 2);
    assert_samples_within(&fit, &samples, 0.1);
}

#[test]
fn criterion_7_a_half_circle_of_200_samples_fits_under_ten_nodes() {
    let samples: Vec<Point> = (0..200)
        .map(|i| {
            let angle = std::f64::consts::PI * i as f64 / 199.0;
            Point::new(50.0 + 30.0 * angle.cos(), 50.0 + 30.0 * angle.sin())
        })
        .collect();
    let fit = fit_freehand(&samples, FIT_ACCURACY);
    assert!(fit.len() >= 2 && fit.len() < 10, "{} nodes", fit.len());
    assert_samples_within(&fit, &samples, 0.1);
}

#[test]
fn criterion_7_uneven_spacing_and_repeated_samples_still_fit() {
    // A pointer that slows down and stops: spacing from 0.01 mm to 3 mm and repeated points.
    let mut samples = Vec::new();
    let mut x = 0.0;
    for i in 0..150 {
        let step = if i % 30 < 10 {
            0.01
        } else {
            3.0 * f64::from(i % 7) / 6.0
        };
        x += step;
        let y = 10.0 * (x / 40.0).sin();
        samples.push(Point::new(x, y));
        if i % 25 == 0 {
            samples.push(Point::new(x, y));
        }
    }
    let fit = fit_freehand(&samples, FIT_ACCURACY);
    assert!(fit.len() >= 2);
    assert_samples_within(&fit, &samples, 0.1);
}

#[test]
fn fewer_than_two_distinct_samples_fit_nothing() {
    assert_eq!(fit_freehand(&[], FIT_ACCURACY), Vec::new());
    assert_eq!(fit_freehand(&[Point::new(1.0, 1.0)], FIT_ACCURACY), Vec::new());
    let same = [Point::new(1.0, 1.0); 5];
    assert_eq!(fit_freehand(&same, FIT_ACCURACY), Vec::new());
}

/// A straight centre line from (0, 0) to (`length`, 0) as two anchors.
fn straight(length: f64) -> Vec<OutlineTriple> {
    use curvyo_document_core::Vec2;
    vec![
        (Point::new(0.0, 0.0), Vec2::ZERO, Vec2::ZERO),
        (Point::new(length, 0.0), Vec2::ZERO, Vec2::ZERO),
    ]
}

/// Samples along the x axis every 0.5 mm with the width `width(x)`.
fn profile_along_x(length: f64, width: impl Fn(f64) -> f64) -> WidthProfile {
    let count = (length / 0.5) as usize + 1;
    let points: Vec<Point> = (0..count)
        .map(|i| Point::new(length * i as f64 / (count - 1) as f64, 0.0))
        .collect();
    let widths: Vec<Length> = points.iter().map(|p| Length::from_mm(width(p.x))).collect();
    WidthProfile::from_samples(&points, &widths)
}

fn total_area(outlines: &[Vec<Point>]) -> f64 {
    outlines
        .iter()
        .map(|o| signed_area_mm2(o))
        .sum::<f64>()
        .abs()
}

/// Height of the outline along the vertical line at `x`: the span between the lowest and the
/// highest crossing of its edges.
fn height_at(outlines: &[Vec<Point>], x: f64) -> f64 {
    let mut low = f64::INFINITY;
    let mut high = f64::NEG_INFINITY;
    for outline in outlines {
        for (index, a) in outline.iter().enumerate() {
            let b = outline[(index + 1) % outline.len()];
            if (a.x - x) * (b.x - x) <= 0.0 && (a.x - b.x).abs() > 0.0 {
                let t = (x - a.x) / (b.x - a.x);
                let y = a.y + t * (b.y - a.y);
                low = low.min(y);
                high = high.max(y);
            }
        }
    }
    high - low
}

#[test]
fn criterion_8_constant_full_pressure_gives_a_capsule() {
    let outlines = variable_outline(&straight(50.0), &profile_along_x(50.0, |_| 2.0)).unwrap();
    assert_eq!(outlines.len(), 1);
    let expected = 50.0 * 2.0 + std::f64::consts::PI;
    let area = total_area(&outlines);
    assert!(
        (area - expected).abs() <= 0.3,
        "area {area}, expected {expected}"
    );
}

#[test]
fn criterion_8_a_linear_width_is_thin_at_the_start_and_half_way_in_the_middle() {
    let outlines = variable_outline(
        &straight(50.0),
        &profile_along_x(50.0, |x| 0.2 + (2.0 - 0.2) * x / 50.0),
    )
    .unwrap();
    let start = height_at(&outlines, 0.0);
    let middle = height_at(&outlines, 25.0);
    assert!((start - 0.2).abs() <= 0.05, "start width {start}");
    assert!((middle - 1.1).abs() <= 0.05, "middle width {middle}");
}

/// Signed distance from `point` to the exact variable-width outline of a centre line: the
/// smallest `|point - c(s)| - r(s)` over a dense sampling of the centre line.
fn distance_to_exact(centre: &[(Point, f64)], point: Point) -> f64 {
    centre
        .iter()
        .map(|(c, radius)| (point.x - c.x).hypot(point.y - c.y) - radius)
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn criterion_8_the_outline_is_within_two_hundredths_of_the_exact_outline() {
    use curvyo_document_core::Vec2;
    // A quarter circle of radius 20 mm (one cubic), width rising from 0.5 to 3 mm.
    let k = 0.552_284_749_830_793_4 * 20.0;
    let centre_line: Vec<OutlineTriple> = vec![
        (Point::new(20.0, 0.0), Vec2::ZERO, Vec2::new(0.0, k)),
        (Point::new(0.0, 20.0), Vec2::new(k, 0.0), Vec2::ZERO),
    ];
    let sample_count = 400;
    let points: Vec<Point> = (0..sample_count)
        .map(|i| {
            let angle = std::f64::consts::FRAC_PI_2 * i as f64 / (sample_count - 1) as f64;
            Point::new(20.0 * angle.cos(), 20.0 * angle.sin())
        })
        .collect();
    let width = |fraction: f64| 0.5 + 2.5 * fraction;
    let widths: Vec<Length> = (0..sample_count)
        .map(|i| Length::from_mm(width(i as f64 / (sample_count - 1) as f64)))
        .collect();
    let outlines =
        variable_outline(&centre_line, &WidthProfile::from_samples(&points, &widths)).unwrap();
    // The exact outline of the cubic itself (not of the circle it approximates).
    let dense = 20_000;
    let exact: Vec<(Point, f64)> = (0..=dense)
        .map(|i| {
            let t = f64::from(i) / f64::from(dense);
            let mt = 1.0 - t;
            let (p0, p1, p2, p3) = (
                centre_line[0].0,
                centre_line[0].0.translated(centre_line[0].2),
                centre_line[1].0.translated(centre_line[1].1),
                centre_line[1].0,
            );
            let w = [mt * mt * mt, 3.0 * mt * mt * t, 3.0 * mt * t * t, t * t * t];
            let c = Point::new(
                w[0] * p0.x + w[1] * p1.x + w[2] * p2.x + w[3] * p3.x,
                w[0] * p0.y + w[1] * p1.y + w[2] * p2.y + w[3] * p3.y,
            );
            (c, width(t) / 2.0)
        })
        .collect();
    let mut worst: f64 = 0.0;
    for outline in &outlines {
        for (index, a) in outline.iter().enumerate() {
            let b = outline[(index + 1) % outline.len()];
            let mid = Point::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y));
            worst = worst
                .max(distance_to_exact(&exact, *a).abs())
                .max(distance_to_exact(&exact, mid).abs());
        }
    }
    // The parameter of the dense sampling is not the arc length, so the width of the reference
    // differs slightly from the arc-length interpolation; the test allows for the sampling only.
    assert!(worst <= 0.02 + 0.005, "worst deviation {worst} mm");
}

#[test]
fn criterion_8_a_stroke_that_crosses_itself_keeps_its_hole() {
    use curvyo_document_core::Vec2;
    // A loop: out along x, round a circle of radius 10 and back across the start.
    let k = 0.552_284_749_830_793_4 * 10.0;
    let centre_line: Vec<OutlineTriple> = vec![
        (Point::new(-5.0, 0.0), Vec2::ZERO, Vec2::ZERO),
        (Point::new(10.0, 0.0), Vec2::new(0.0, k), Vec2::new(0.0, -k)),
        (
            Point::new(0.0, -10.0),
            Vec2::new(k, 0.0),
            Vec2::new(-k, 0.0),
        ),
        (
            Point::new(-10.0, 0.0),
            Vec2::new(0.0, -k),
            Vec2::new(0.0, k),
        ),
        (Point::new(0.0, 10.0), Vec2::new(-k, 0.0), Vec2::new(k, 0.0)),
        (Point::new(5.0, -5.0), Vec2::ZERO, Vec2::ZERO),
    ];
    let points: Vec<Point> = centre_line.iter().map(|a| a.0).collect();
    let widths = vec![Length::from_mm(1.0); points.len()];
    let outlines =
        variable_outline(&centre_line, &WidthProfile::from_samples(&points, &widths)).unwrap();
    let holes = outlines.iter().filter(|o| signed_area_mm2(o) < 0.0).count();
    assert!(holes >= 1, "{} outlines, {holes} holes", outlines.len());
}

#[test]
fn an_empty_centre_line_has_no_outline() {
    let profile = WidthProfile::from_samples(&[], &[]);
    assert!(variable_outline(&[], &profile).is_err());
}
