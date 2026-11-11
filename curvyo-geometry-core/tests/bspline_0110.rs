//! Reference cases for the approved clamped and periodic B-spline geometry.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::Point;
use curvyo_geometry_core::{SplineControl, evaluate_bspline};

fn control(x: f64, y: f64) -> SplineControl {
    SplineControl {
        point: Point::new(x, y),
        corner: false,
    }
}

#[test]
fn four_open_controls_match_the_cubic_reference() {
    let spans = evaluate_bspline(
        &[
            control(0.0, 0.0),
            control(1.0, 0.0),
            control(1.0, 1.0),
            control(0.0, 1.0),
        ],
        false,
    )
    .unwrap();
    assert_eq!(spans.len(), 1);
    let c = spans[0];
    let x = (c.start.x + 3.0 * c.control1.x + 3.0 * c.control2.x + c.end.x) / 8.0;
    let y = (c.start.y + 3.0 * c.control1.y + 3.0 * c.control2.y + c.end.y) / 8.0;
    assert!((x - 0.75).abs() < 1e-12 && (y - 0.5).abs() < 1e-12);
}

#[test]
fn closed_uniform_seam_uses_periodic_reference() {
    let controls = [control(0.0, 0.0), control(6.0, 0.0), control(0.0, 6.0)];
    let spans = evaluate_bspline(&controls, true).unwrap();
    assert_eq!(spans.len(), 3);
    assert!((spans[0].start.x - 1.0).abs() < 1e-12);
    assert!((spans[0].start.y - 1.0).abs() < 1e-12);
    assert!((spans.last().unwrap().end.x - spans[0].start.x).abs() < 1e-12);
    assert!((spans.last().unwrap().end.y - spans[0].start.y).abs() < 1e-12);
}

#[test]
fn short_open_curves_preserve_endpoints_and_closed_corners() {
    let line = evaluate_bspline(&[control(0.0, 0.0), control(3.0, 0.0)], false).unwrap();
    assert!((line[0].control1.x - 1.0).abs() < 1e-12);
    assert!((line[0].control2.x - 2.0).abs() < 1e-12);
    let mut controls = [control(0.0, 0.0), control(2.0, 0.0), control(1.0, 2.0)];
    controls[1].corner = true;
    let spans = evaluate_bspline(&controls, true).unwrap();
    assert_eq!(spans.first().unwrap().start, controls[1].point);
    assert_eq!(spans.last().unwrap().end, controls[1].point);
    assert!(evaluate_bspline(&controls[..2], true).is_err());
    assert!(evaluate_bspline(&[control(f64::NAN, 0.0)], false).is_err());
    assert!(evaluate_bspline(&[control(f64::MAX, 0.0), control(f64::MAX, 1.0)], false).is_err());
}

#[test]
fn adjacent_clamped_spans_meet_at_the_interior_knot() {
    let points = [
        control(0.0, 0.0),
        control(1.0, 2.0),
        control(2.0, 3.0),
        control(3.0, 2.0),
        control(4.0, 0.0),
    ];
    let spans = evaluate_bspline(&points, false).unwrap();
    assert_eq!(spans.len(), 2);
    assert!((spans[0].end.x - 2.0).abs() < 1e-12);
    assert!((spans[0].end.y - 2.5).abs() < 1e-12);
    assert!((spans[0].end.x - spans[1].start.x).abs() < 1e-12);
    assert!((spans[0].end.y - spans[1].start.y).abs() < 1e-12);
}

#[test]
fn quadratic_degree_elevation_and_open_corner_break() {
    let controls = [control(0.0, 0.0), control(3.0, 3.0), control(6.0, 0.0)];
    let spans = evaluate_bspline(&controls, false).unwrap();
    assert!((spans[0].control1.x - 2.0).abs() < 1e-12);
    assert!((spans[0].control1.y - 2.0).abs() < 1e-12);
    assert!((spans[0].control2.x - 4.0).abs() < 1e-12);
    assert!((spans[0].control2.y - 2.0).abs() < 1e-12);
    let mut broken = controls;
    broken[1].corner = true;
    let spans = evaluate_bspline(&broken, false).unwrap();
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].end, broken[1].point);
    assert_eq!(spans[1].start, broken[1].point);
}

#[test]
fn coincident_controls_evaluate_to_finite_degenerate_geometry() {
    let controls = [control(2.0, 3.0); 4];
    for span in evaluate_bspline(&controls, true).unwrap() {
        for p in [span.start, span.control1, span.control2, span.end] {
            assert!(p.x.is_finite() && p.y.is_finite());
            assert!((p.x - 2.0).abs() < 1e-12 && (p.y - 3.0).abs() < 1e-12);
        }
    }
}
