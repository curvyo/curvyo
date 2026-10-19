//! `specs/0021-ellipse-arcs-and-shaping/` criterion 62 (a) and (b): the evaluation budgets,
//! measured on the build host in a release build (`cargo test --release -- --ignored`).

#![allow(clippy::unwrap_used)]

use std::time::{Duration, Instant};

use curvyo_document_core::{
    Angle, ArcKind, Curve, EllipseArc, EllipseFrame, Length, Point, PointCount, Shape, StarFrame,
    drawing_outline_of, outline_of, shape_frame_bounds,
};

/// The mean time of one evaluation (outline, drawn outline and box) of `shape`.
fn mean(shape: &Shape, runs: u32) -> Duration {
    let start = Instant::now();
    for _ in 0..runs {
        std::hint::black_box((
            outline_of(shape),
            drawing_outline_of(shape),
            shape_frame_bounds(shape),
        ));
    }
    start.elapsed() / runs
}

fn polygon(n: u32, k: f64) -> Shape {
    Shape::Polygon {
        frame: StarFrame {
            center: Point::new(0.0, 0.0),
            radius: Length::from_mm(50.0),
            angle: Angle::from_radians(0.0),
        },
        point_count: PointCount::new(n).unwrap(),
        curve: Curve::new(k).unwrap(),
    }
}

#[test]
#[ignore = "a budget, run in release on the build host"]
fn budget_a_one_arc_curved_ellipse_or_hexagon_within_50_microseconds() {
    let frame = EllipseFrame {
        center: Point::new(0.0, 0.0),
        rx: Length::from_mm(30.0),
        ry: Length::from_mm(20.0),
    };
    let arc = Shape::Ellipse {
        frame,
        arc: EllipseArc::new(
            Angle::from_radians(0.3),
            Angle::from_radians(5.0),
            ArcKind::Pie,
        ),
        curve: Curve::ELLIPSE_DEFAULT,
    };
    let curved = Shape::Ellipse {
        frame,
        arc: None,
        curve: Curve::new(-0.4).unwrap(),
    };
    for shape in [arc, curved, polygon(6, 0.5)] {
        let t = mean(&shape, 2000);
        assert!(t <= Duration::from_micros(50), "{t:?}");
    }
}

#[test]
#[ignore = "a budget, run in release on the build host"]
fn budget_b_a_curved_1024_gon_within_2_milliseconds() {
    let t = mean(&polygon(1024, 0.7), 50);
    assert!(t <= Duration::from_millis(2), "{t:?}");
}
