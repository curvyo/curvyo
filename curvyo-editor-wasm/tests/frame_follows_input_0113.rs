//! Regression for the `specs/0113-fixture-sets-and-pins` demo: on a laser page with a 20 x 20 pin
//! grid (the cached fixture lists), every frame follows the input that changed it. A selection,
//! a marquee and its release, a hover and each wheel step change the frame at once, and a frame
//! after the input ends shows nothing of it.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{NodeId, Point, ViewTransform};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_render_core::DrawList;

/// A laser page with one job fixture: a 20 x 20 pin grid at page-local (10, 300).
fn grid_page() -> (Session, NodeId) {
    let mut s = Session::new(1);
    s.resize_viewport(1600.0, 1000.0);
    s.show_default_view();
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    s.set_tool(Tool::Select);
    assert!(s.arm_job_fixture("pin"));
    let p = at(&s, 10.0, 300.0);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    s.end_fixture_placement();
    let view = s.fixtures_view().unwrap();
    let grid = s.fixture_part_named(&view.job_fixtures[0].id).unwrap();
    for (field, value) in [
        ("columns", 20.0),
        ("rows", 20.0),
        ("column_spacing_mm", 10.0),
        ("row_spacing_mm", 10.0),
    ] {
        s.set_fixture_grid_value(grid, field, value);
    }
    (s, grid)
}

/// The document point of the page-local `(x, y)`.
fn at(s: &Session, x: f64, y: f64) -> Point {
    let o = s.active_page().origin;
    Point::new(o.x + x, o.y + y)
}

/// What the host submits for a frame: the draw list and the view it is drawn under.
type Frame = (DrawList, ViewTransform);

fn frame(s: &Session) -> Frame {
    (s.frame_draw_list(), s.view())
}

/// Clears the selection by a click on empty page and settles the hover there.
fn rest(s: &mut Session) -> Frame {
    let empty = at(s, 150.0, 40.0);
    s.pointer_down(empty, false);
    s.pointer_up(empty, false, false);
    s.pointer_hover(empty, false, false);
    assert!(s.selected_fixture().is_none());
    frame(s)
}

#[test]
fn a_selection_changes_the_frame_and_its_end_clears_it() {
    let (mut s, _) = grid_page();
    let idle = rest(&mut s);
    assert!(frame(&s) == idle, "an idle frame repeats itself");
    let pin = at(&s, 10.0 + 3.0, 300.0);
    s.pointer_down(pin, false);
    s.pointer_up(pin, false, false);
    assert!(s.selected_fixture().is_some(), "the press hit the grid");
    assert!(
        frame(&s) != idle,
        "the selection box is drawn in the next frame"
    );
    assert!(
        rest(&mut s) == idle,
        "the next frame after deselecting has no box"
    );
}

#[test]
fn a_marquee_is_drawn_while_held_and_gone_in_the_frame_after_release() {
    let (mut s, _) = grid_page();
    let idle = rest(&mut s);
    let (from, to) = (at(&s, 150.0, 40.0), at(&s, 190.0, 90.0));
    s.pointer_down(from, false);
    s.pointer_hover(to, false, false);
    assert!(
        frame(&s) != idle,
        "the marquee is drawn while the button is held"
    );
    s.pointer_up(to, false, false);
    s.pointer_hover(from, false, false);
    assert!(
        frame(&s) == idle,
        "the frame after the release shows no marquee"
    );
}

#[test]
fn a_hover_over_the_grid_leaves_no_trace_in_the_next_frame() {
    let (mut s, _) = grid_page();
    let idle = rest(&mut s);
    s.pointer_hover(at(&s, 10.0 + 3.0, 300.0), false, false);
    s.pointer_hover(at(&s, 150.0, 40.0), false, false);
    assert!(
        frame(&s) == idle,
        "leaving the grid restores the idle frame"
    );
}

#[test]
fn every_wheel_step_redraws_the_frame_at_once() {
    let (mut s, _) = grid_page();
    let mut last = rest(&mut s);
    for step in 0..4 {
        s.wheel(0.0, -100.0, 800.0, 500.0, false, true);
        let now = frame(&s);
        assert!(
            now != last,
            "zoom step {step}: the frame follows the zoom at once"
        );
        last = now;
    }
    for step in 0..3 {
        s.wheel(0.0, 60.0, 800.0, 500.0, false, false);
        let now = frame(&s);
        assert!(
            now != last,
            "pan step {step}: the frame follows the pan at once"
        );
        last = now;
    }
}
