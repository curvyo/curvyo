//! The CPU cost of a variable-width outline (`specs/0110-pen-modes-variable-stroke` criterion 4,
//! ADR 0053 amendment of 2026-11-10, item 4): the session recomputes it on every move of a width
//! drag, so the benchmark paths (a smooth 4-node S-curve, a 12-node kinked path, a two-node arc,
//! each with 8 width points) stay within 2 ms of thread CPU time in an optimised build, and a
//! 200-node freehand path within 8 ms. It is a regression test, not a product limit. There is
//! no wasm CPU-time harness: the wasm figures of the ADR (4 ms and 16 ms) are not checked here.

#![allow(clippy::unwrap_used)]

use std::time::Duration;

use curvyo_document_core::{
    DashPattern, Length, LineCap, LineJoin, Markers, Point, Tolerance, Vec2,
};
use curvyo_geometry_core::{
    Outline, StrokeShape, WidthPoint, WidthProfile, variable_stroke_outline,
};

type Triple = (Point, Vec2, Vec2);

/// Unoptimised code runs about ten times slower.
const fn budget(release_ms: u64) -> Duration {
    Duration::from_millis(if cfg!(debug_assertions) {
        12 * release_ms
    } else {
        release_ms
    })
}

/// The CPU time the calling thread has used so far (wall time off Unix).
fn thread_time() -> Duration {
    #[cfg(unix)]
    {
        let now = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
        Duration::new(
            u64::try_from(now.tv_sec).unwrap(),
            u32::try_from(now.tv_nsec).unwrap(),
        )
    }
    #[cfg(not(unix))]
    {
        use std::sync::OnceLock;
        static START: OnceLock<std::time::Instant> = OnceLock::new();
        START.get_or_init(std::time::Instant::now).elapsed()
    }
}

fn eight_points() -> WidthProfile {
    let points = (0_u8..8)
        .map(|i| {
            let at = f64::from(i) / 7.0;
            let multiplier = if i % 2 == 0 { 0.4 } else { 1.8 };
            WidthPoint::new(u128::from(i) + 1, at, multiplier, false).unwrap()
        })
        .collect();
    WidthProfile::new(points, false).unwrap()
}

/// The least CPU time of a few runs, so one preemption does not count.
fn cheapest(anchors: &[Triple], join: LineJoin) -> Duration {
    let style = StrokeShape {
        width: Length::from_mm(3.0),
        join,
        cap: LineCap::Round,
        dash: DashPattern::default(),
        markers: Markers::default(),
    };
    let profile = eight_points();
    (0..5)
        .map(|_| {
            let start = thread_time();
            let result = variable_stroke_outline(
                Outline::new(anchors, false),
                &style,
                &profile,
                Tolerance::from_mm(0.01),
            );
            let spent = thread_time().saturating_sub(start);
            assert!(result.is_ok());
            spent
        })
        .min()
        .unwrap()
}

#[test]
fn a_smooth_four_node_s_curve_fits_the_budget() {
    let anchors = [
        (Point::new(0.0, 0.0), Vec2::ZERO, Vec2::new(10.0, 0.0)),
        (
            Point::new(30.0, 20.0),
            Vec2::new(-10.0, 0.0),
            Vec2::new(10.0, 0.0),
        ),
        (
            Point::new(60.0, -20.0),
            Vec2::new(-10.0, 0.0),
            Vec2::new(10.0, 0.0),
        ),
        (Point::new(90.0, 0.0), Vec2::new(-10.0, 0.0), Vec2::ZERO),
    ];
    let spent = cheapest(&anchors, LineJoin::Round);
    assert!(spent <= budget(2), "S-curve took {spent:?}");
}

#[test]
fn a_twelve_node_kinked_path_fits_the_budget() {
    let anchors: Vec<Triple> = (0..12)
        .map(|i| {
            let y = if i % 2 == 0 { 0.0 } else { 6.0 };
            (
                Point::new(8.0 * f64::from(i), y),
                Vec2::new(-2.0, 2.0),
                Vec2::new(2.0, 2.0),
            )
        })
        .collect();
    let spent = cheapest(&anchors, LineJoin::Miter);
    assert!(spent <= budget(2), "kinked path took {spent:?}");
}

#[test]
fn a_two_node_arc_fits_the_budget() {
    let k = 5.0 * 4.0 / 3.0;
    let anchors = [
        (Point::new(0.0, 0.0), Vec2::ZERO, Vec2::new(0.0, k)),
        (Point::new(10.0, 0.0), Vec2::new(0.0, k), Vec2::ZERO),
    ];
    let spent = cheapest(&anchors, LineJoin::Round);
    assert!(spent <= budget(2), "arc took {spent:?}");
}

#[test]
fn a_two_hundred_node_freehand_path_fits_the_budget() {
    let anchors: Vec<Triple> = (0..200)
        .map(|i| {
            let x = 1.5 * f64::from(i);
            let y = 12.0 * (x / 9.0).sin() + 1.5 * (x / 7.0).cos();
            let handle = Vec2::new(0.5, 0.6 * (x / 9.0).cos());
            (Point::new(x, y), Vec2::new(-handle.x, -handle.y), handle)
        })
        .collect();
    let spent = cheapest(&anchors, LineJoin::Round);
    assert!(spent <= budget(8), "freehand path took {spent:?}");
}
