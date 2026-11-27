//! Acceptance test for `specs/0146-material-test-picks` criterion 20 (ADR 0093 decision 10): the
//! round Engrave mark is the ellipse outline with equal radii, within 0.03 % of the radius of the
//! exact circle and 0.1 % of its area. Written black-box from the specification.
#![allow(clippy::many_single_char_names, clippy::cast_precision_loss)]

use curvyo_document_core::{EllipseFrame, Length, OutlineAnchor, Point, ellipse_outline};

/// Samples every cubic segment of the closed outline (anchor i to i + 1, wrapping).
fn samples(anchors: &[OutlineAnchor], steps: usize) -> Vec<(f64, f64)> {
    let n = anchors.len();
    let mut out = Vec::new();
    for i in 0..n {
        let a = &anchors[i];
        let b = &anchors[(i + 1) % n];
        let p0 = a.point;
        let p1 = Point::new(a.point.x + a.handle_out.x, a.point.y + a.handle_out.y);
        let p2 = Point::new(b.point.x + b.handle_in.x, b.point.y + b.handle_in.y);
        let p3 = b.point;
        for s in 0..steps {
            let t = s as f64 / steps as f64;
            let u = 1.0 - t;
            let x = u * u * u * p0.x
                + 3.0 * u * u * t * p1.x
                + 3.0 * u * t * t * p2.x
                + t * t * t * p3.x;
            let y = u * u * u * p0.y
                + 3.0 * u * u * t * p1.y
                + 3.0 * u * t * t * p2.y
                + t * t * t * p3.y;
            out.push((x, y));
        }
    }
    out
}

#[test]
fn round_engrave_mark_is_within_tolerance_at_cell_sizes_3_10_and_50_mm() {
    for side in [3.0_f64, 10.0, 50.0] {
        let r = side / 2.0;
        // The cell's square has its top-left corner at (0, 0); the circle is centred on it.
        let frame = EllipseFrame {
            center: Point::new(r, r),
            rx: Length::from_mm(r),
            ry: Length::from_mm(r),
        };
        let anchors = ellipse_outline(frame);
        assert_eq!(anchors.len(), 4, "four smooth anchors, one per quadrant");
        let pts = samples(&anchors, 2000);

        let worst_radial = pts
            .iter()
            .map(|(x, y)| (((x - r).powi(2) + (y - r).powi(2)).sqrt() - r).abs() / r)
            .fold(0.0_f64, f64::max);
        assert!(
            worst_radial <= 0.0003,
            "side {side} mm: radial error {worst_radial:e} of the radius exceeds 0.03 %"
        );

        // Shoelace area over the sampled closed polygon.
        let mut twice_area = 0.0;
        for w in 0..pts.len() {
            let (x1, y1) = pts[w];
            let (x2, y2) = pts[(w + 1) % pts.len()];
            twice_area += x1 * y2 - x2 * y1;
        }
        let area = (twice_area / 2.0).abs();
        let exact = std::f64::consts::PI * r * r;
        let rel = ((area - exact) / exact).abs();
        assert!(
            rel <= 0.001,
            "side {side} mm: area error {rel:e} exceeds 0.1 %"
        );
    }
}

#[test]
fn round_engrave_mark_has_equal_radii_and_a_square_bounding_box() {
    let r = 5.0;
    let anchors = ellipse_outline(EllipseFrame {
        center: Point::new(r, r),
        rx: Length::from_mm(r),
        ry: Length::from_mm(r),
    });
    let xs: Vec<f64> = anchors.iter().map(|a| a.point.x).collect();
    let ys: Vec<f64> = anchors.iter().map(|a| a.point.y).collect();
    let fold = |v: &[f64], f: fn(f64, f64) -> f64, init: f64| v.iter().copied().fold(init, f);
    assert!((fold(&xs, f64::min, f64::MAX) - 0.0).abs() < 1e-9);
    assert!((fold(&xs, f64::max, f64::MIN) - 2.0 * r).abs() < 1e-9);
    assert!((fold(&ys, f64::min, f64::MAX) - 0.0).abs() < 1e-9);
    assert!((fold(&ys, f64::max, f64::MIN) - 2.0 * r).abs() < 1e-9);
}
