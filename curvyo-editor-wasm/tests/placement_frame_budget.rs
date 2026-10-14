//! The speed gate of a click-to-create placement (`specs/0052-click-to-create-shapes/`
//! criterion 32, `adrs.md` decision 7): with a 1024-point star, the placement preview frame of
//! `Session::draw_list` takes at most 1.1 times the create-drag frame of the same star, and one
//! keystroke in a frozen placement (edit, resolve, preview) takes under 1 ms. Budgets are
//! asserted in release builds only; a debug build prints the numbers.
//! Run: `cargo test --release -p curvyo-editor-wasm --test placement_frame_budget -- --nocapture`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use std::time::{Duration, Instant};

use curvyo_document_core::{Point, PointCount};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_ui_core::PolyStarMode;

const FRAMES: usize = 60;
const RATIO_BUDGET: f64 = 1.1;
const KEYSTROKE_BUDGET: Duration = Duration::from_millis(1);

const CENTRE: Point = Point::new(100.0, 100.0);

fn star_session() -> Session {
    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    session.set_tool(Tool::PolygonStar);
    session.set_poly_star_mode(PolyStarMode::Star);
    session.set_poly_star_point_count(PointCount::new(1024).unwrap());
    session
}

/// The CPU time the calling thread has used so far. Time the thread spends preempted by other
/// processes is not in it, so a busy CI runner does not inflate a frame (as for
/// `node_marquee_interactivity`).
#[cfg(unix)]
fn thread_time() -> Duration {
    let now = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
    Duration::new(
        u64::try_from(now.tv_sec).unwrap(),
        u32::try_from(now.tv_nsec).unwrap(),
    )
}

/// The CPU time of one `draw_list` call (wall time where no thread clock is available).
fn frame(session: &mut Session) -> Duration {
    #[cfg(unix)]
    {
        let start = thread_time();
        let list = session.draw_list();
        let took = thread_time().saturating_sub(start);
        assert!(list.triangle_count() > 0);
        took
    }
    #[cfg(not(unix))]
    {
        let start = Instant::now();
        let list = session.draw_list();
        let took = start.elapsed();
        assert!(list.triangle_count() > 0);
        took
    }
}

/// The fastest of `FRAMES` frames of each session, drawn alternately so that both see the same
/// machine load: the least noisy estimate of one frame's cost.
fn fastest_frames(
    drag: &mut Session,
    placement: &mut Session,
    pointer: impl Fn(usize) -> Point,
) -> (Duration, Duration) {
    let (mut drag_best, mut placement_best) = (Duration::MAX, Duration::MAX);
    for i in 0..FRAMES {
        drag.pointer_hover(pointer(i), false, false);
        drag_best = drag_best.min(frame(drag));
        placement.pointer_hover(pointer(i), false, false);
        placement_best = placement_best.min(frame(placement));
    }
    (drag_best, placement_best)
}

fn on_circle(i: usize) -> Point {
    let t = (i as f64).to_radians() * 6.0;
    Point::new(CENTRE.x + 60.0 * t.cos(), CENTRE.y + 60.0 * t.sin())
}

#[test]
fn a_placement_frame_costs_what_a_create_drag_frame_costs() {
    let mut drag = star_session();
    drag.pointer_down(CENTRE, false);
    let mut placement = star_session();
    placement.pointer_down(CENTRE, false);
    placement.pointer_up(CENTRE, false, false);
    assert!(placement.placement().is_some());
    let (drag_frame, placement_frame) = fastest_frames(&mut drag, &mut placement, on_circle);

    // Both frames draw the star: the placement's has the pivot marker on top.
    let (drag_triangles, placement_triangles) = (
        drag.draw_list().triangle_count(),
        placement.draw_list().triangle_count(),
    );
    assert!(
        placement_triangles >= drag_triangles,
        "{placement_triangles} < {drag_triangles}"
    );
    assert!(drag_triangles > star_session().draw_list().triangle_count() + 1000);
    let ratio = placement_frame.as_secs_f64() / drag_frame.as_secs_f64();
    println!(
        "1024-point star: create-drag frame {drag_frame:?}, placement frame {placement_frame:?}, \
         ratio {ratio:.3} (budget {RATIO_BUDGET})"
    );
    if !cfg!(debug_assertions) {
        assert!(
            ratio <= RATIO_BUDGET,
            "ratio {ratio:.3} over {RATIO_BUDGET}"
        );
    }
}

#[test]
fn a_keystroke_in_a_frozen_placement_is_under_a_millisecond() {
    let mut session = star_session();
    session.pointer_down(CENTRE, false);
    session.pointer_up(CENTRE, false, false);
    session.pointer_hover(on_circle(0), false, false);
    session.placement_edit(0, "2");
    let slowest = (0..FRAMES)
        .map(|i| {
            let text = format!("2{i}");
            let start = Instant::now();
            session.placement_edit(0, &text);
            start.elapsed()
        })
        .max()
        .unwrap();
    println!("one keystroke: slowest {slowest:?} (budget {KEYSTROKE_BUDGET:?})");
    if !cfg!(debug_assertions) {
        assert!(
            slowest < KEYSTROKE_BUDGET,
            "{slowest:?} over {KEYSTROKE_BUDGET:?}"
        );
    }
}
