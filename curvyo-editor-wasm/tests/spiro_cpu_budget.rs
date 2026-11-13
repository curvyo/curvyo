//! The CPU cost of solving a Spiro curve (`specs/0110-pen-modes-variable-stroke` criterion 2):
//! the session solves a drawn path synchronously while points are placed, so one solve of a
//! realistic path fits well inside a 16 ms frame, and a curve whose iteration does not converge
//! stops just as fast (ten iterations at most, `specs/0114`). Native thread CPU time is a lower bound for wasm, hence the 4 ms margin.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod cpu_time;

use std::time::Duration;

use curvyo_document_core::{Point, Tolerance};
use curvyo_geometry_core::{SpiroControl, SpiroCubic, SpiroError, evaluate_spiro};

/// A quarter of a 16 ms frame in an optimised build; unoptimised code runs about ten times slower.
const BUDGET: Duration = Duration::from_millis(if cfg!(debug_assertions) { 40 } else { 4 });

fn smooth(points: impl Iterator<Item = (f64, f64)>) -> Vec<SpiroControl> {
    points
        .map(|(x, y)| SpiroControl {
            point: Point::new(x, y),
            corner: false,
        })
        .collect()
}

fn timed(
    controls: &[SpiroControl],
    closed: bool,
) -> (Result<Vec<SpiroCubic>, SpiroError>, Duration) {
    cpu_time::cpu_time_of(|| evaluate_spiro(controls, closed, Tolerance::from_mm(0.01)))
}

#[test]
fn an_open_wave_of_forty_controls_solves_within_a_quarter_frame() {
    let controls = smooth((0..40).map(|i| {
        let x = 5.0 * f64::from(i);
        (x, 20.0 * (x / 20.0).sin())
    }));
    let (result, spent) = timed(&controls, false);
    let spans = result.expect("a gentle wave solves");
    assert!(spans.len() >= 39);
    assert!(spent < BUDGET, "open wave took {spent:?}");
}

#[test]
fn a_closed_wobbly_ring_of_twenty_four_controls_solves_within_a_quarter_frame() {
    let controls = smooth((0..24).map(|i| {
        let angle = std::f64::consts::TAU * f64::from(i) / 24.0;
        let radius = 50.0 + 6.0 * (3.0 * angle).sin();
        (radius * angle.cos(), radius * angle.sin())
    }));
    let (result, spent) = timed(&controls, true);
    let spans = result.expect("a ring solves");
    let first = spans.first().unwrap().start;
    let last = spans.last().unwrap().end;
    assert!(
        (first.x - last.x).hypot(first.y - last.y) < 1e-6,
        "the ring closes"
    );
    assert!(spent < BUDGET, "closed ring took {spent:?}");
}

#[test]
fn a_hard_zigzag_answers_within_a_quarter_frame_either_way() {
    let controls = smooth((0..20).map(|i| {
        let y = if i % 2 == 0 {
            0.0
        } else {
            8.0 + f64::from(i % 5)
        };
        (10.0 * f64::from(i), y)
    }));
    let (_, spent) = timed(&controls, false);
    assert!(spent < BUDGET, "zigzag took {spent:?}");
}

/// The rebuild of the Pen's live Spiro at one pointer event (`specs/0114` criterion 3): the hover
/// and the frame drawn after it solve the curve from its controls, within 2 ms in an optimised
/// build for waves of 3, 8 and 20 controls with the pointer circling the last one, and for the
/// hairpin (three controls nearly on a line) with the pointer circling its end.
#[test]
fn a_live_spiro_rebuild_per_pointer_event_fits_two_milliseconds() {
    use curvyo_editor_wasm::{Session, Tool};
    use curvyo_ui_core::PenMode;
    let budget = Duration::from_millis(if cfg!(debug_assertions) { 20 } else { 2 });
    let wave = |n: u32| -> Vec<Point> {
        (0..n)
            .map(|i| {
                let x = 5.0 * f64::from(i);
                Point::new(10.0 + x, 100.0 + 15.0 * (x / 15.0).sin())
            })
            .collect()
    };
    let hairpin = vec![
        Point::new(10.0, 10.0),
        Point::new(50.0, 10.0),
        Point::new(20.0, 10.5),
    ];
    for (placed, radius) in [
        (wave(3), 12.0),
        (wave(8), 12.0),
        (wave(20), 12.0),
        (hairpin, 20.0),
    ] {
        let mut session = Session::new(61);
        session.set_tool(Tool::Pen);
        assert!(session.set_pen_mode(PenMode::Spiro));
        for &p in &placed {
            session.pointer_hover(p, false, false);
            session.pointer_down(p, false);
            session.pointer_up(p, false, false);
        }
        let last = *placed.last().unwrap();
        let mut worst = Duration::ZERO;
        for step in 0..64 {
            let angle = std::f64::consts::TAU * f64::from(step) / 64.0;
            let at = Point::new(last.x + radius * angle.cos(), last.y + radius * angle.sin());
            let (_, spent) = cpu_time::cpu_time_of(|| {
                session.pointer_hover(at, false, false);
                session.draw_list()
            });
            worst = worst.max(spent);
        }
        assert!(
            worst < budget,
            "{} controls: worst rebuild took {worst:?}",
            placed.len()
        );
    }
}
