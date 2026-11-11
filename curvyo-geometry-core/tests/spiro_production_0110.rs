//! Spiro evaluation preserves controls and reports invalid inputs.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Point, Tolerance};
use curvyo_geometry_core::{SpiroControl, evaluate_spiro};

#[test]
fn open_corner_curve_reaches_each_control_and_remains_finite() {
    let controls = [
        SpiroControl {
            point: Point::new(0.0, 0.0),
            corner: false,
        },
        SpiroControl {
            point: Point::new(1.0, 1.0),
            corner: false,
        },
        SpiroControl {
            point: Point::new(2.0, 0.0),
            corner: true,
        },
        SpiroControl {
            point: Point::new(3.0, -1.0),
            corner: false,
        },
        SpiroControl {
            point: Point::new(4.0, 0.0),
            corner: false,
        },
    ];
    let result = evaluate_spiro(&controls, false, Tolerance::from_mm(0.001)).unwrap();
    for target in controls {
        assert!(result.iter().any(|span| {
            (span.start.x - target.point.x).hypot(span.start.y - target.point.y) < 0.001
                || (span.end.x - target.point.x).hypot(span.end.y - target.point.y) < 0.001
        }));
    }
}

#[test]
fn invalid_inputs_return_error_without_a_substitute_curve() {
    let controls = [
        SpiroControl {
            point: Point::new(f64::NAN, 0.0),
            corner: false,
        },
        SpiroControl {
            point: Point::new(1.0, 0.0),
            corner: false,
        },
    ];
    assert!(evaluate_spiro(&controls, false, Tolerance::from_mm(0.01)).is_err());
    let line = [
        SpiroControl {
            point: Point::new(0.0, 0.0),
            corner: false,
        },
        SpiroControl {
            point: Point::new(1.0, 0.0),
            corner: false,
        },
    ];
    assert!(evaluate_spiro(&line, true, Tolerance::from_mm(0.01)).is_err());
    assert!(evaluate_spiro(&line, false, Tolerance::from_mm(1e-20)).is_err());
}

fn sampled_distance(spans: &[curvyo_geometry_core::SpiroCubic], target: Point) -> f64 {
    spans
        .iter()
        .flat_map(|span| {
            (0..=500).map(move |i| {
                let t = f64::from(i) / 500.0;
                let u = 1.0 - t;
                let x = u.powi(3) * span.start.x
                    + 3.0 * u * u * t * span.control1.x
                    + 3.0 * u * t * t * span.control2.x
                    + t.powi(3) * span.end.x;
                let y = u.powi(3) * span.start.y
                    + 3.0 * u * u * t * span.control1.y
                    + 3.0 * u * t * t * span.control2.y
                    + t.powi(3) * span.end.y;
                (x - target.x).hypot(y - target.y)
            })
        })
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn six_point_loop_matches_independent_libspiro_geometry() {
    let controls = [
        (0.0, 1.0),
        (1.0, 2.0),
        (2.0, 0.0),
        (3.0, 0.0),
        (4.0, 2.0),
        (5.0, 1.0),
    ]
    .map(|(x, y)| SpiroControl {
        point: Point::new(x, y),
        corner: false,
    });
    let result = evaluate_spiro(&controls, false, Tolerance::from_mm(0.001)).unwrap();
    for target in [
        Point::new(2.144_661_689_23, -1.313_177_168_15),
        Point::new(4.874_642_975_31, 1.917_303_309_88),
    ] {
        assert!(sampled_distance(&result, target) < 0.02);
    }
}

#[test]
fn closed_quarter_circle_and_scaled_copy_match() {
    let controls = [(-1.0, 0.0), (0.0, 1.0), (1.0, 0.0), (0.0, -1.0)].map(|(x, y)| SpiroControl {
        point: Point::new(x, y),
        corner: false,
    });
    let scaled = controls.map(|c| SpiroControl {
        point: Point::new(10.0 * c.point.x + 7.0, 10.0 * c.point.y - 3.0),
        corner: c.corner,
    });
    let base = evaluate_spiro(&controls, true, Tolerance::from_mm(0.001)).unwrap();
    let other = evaluate_spiro(&scaled, true, Tolerance::from_mm(0.01)).unwrap();
    assert_eq!(base.len(), other.len());
    for (a, b) in base.iter().zip(&other) {
        for (x, y) in [
            (a.start, b.start),
            (a.control1, b.control1),
            (a.control2, b.control2),
            (a.end, b.end),
        ] {
            assert!((10.0 * x.x + 7.0 - y.x).hypot(10.0 * x.y - 3.0 - y.y) < 1e-6);
        }
    }
    assert!(sampled_distance(&base, Point::new(2_f64.sqrt() / 2.0, 2_f64.sqrt() / 2.0)) < 0.002);
}

#[test]
fn closed_corner_and_adjacent_corners_keep_every_control() {
    let points = [(0.0, 0.0), (2.0, 0.0), (3.0, 1.0), (2.0, 2.0), (0.0, 2.0)];
    for flags in [
        [true, false, false, true, false],
        [true, true, false, true, false],
    ] {
        let controls = std::array::from_fn::<_, 5, _>(|i| SpiroControl {
            point: Point::new(points[i].0, points[i].1),
            corner: flags[i],
        });
        let spans = evaluate_spiro(&controls, true, Tolerance::from_mm(0.001)).unwrap();
        for control in controls {
            assert!(spans.iter().any(|span| {
                (span.start.x - control.point.x).hypot(span.start.y - control.point.y) < 0.001
            }));
        }
    }
}
