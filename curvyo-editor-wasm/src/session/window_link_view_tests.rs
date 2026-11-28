//! Follow view and the pen window's frame at session level (`specs/0158-pen-display` slice 2b
//! criteria 5 and 6), and the selection the pen window opens with: two `Session`s and the byte
//! shuttle of `window_link_tests`, no window.

use curvyo_ui_core::SavedView;

use super::Session;
use super::align_test_support::draw_rect;
use super::window_link_tests::Pair;

/// Zoom factors closer than this are equal (criterion 5).
const ZOOM_EQUAL: f64 = 1e-9;
/// Centre points closer than this (mm) are equal (criterion 5).
const CENTRE_EQUAL_MM: f64 = 1e-6;
/// The frame equals the pen window's visible rectangle within this (criterion 6).
const FRAME_TOLERANCE_MM: f64 = 0.01;

fn main_window() -> Session {
    let mut main = Session::new(0xA1);
    main.resize_viewport(1600.0, 900.0);
    draw_rect(&mut main, (10.0, 10.0), (40.0, 30.0));
    main
}

fn view(session: &Session) -> SavedView {
    session.viewport.saved_view()
}

fn same_view(a: SavedView, b: SavedView) -> bool {
    (a.zoom - b.zoom).abs() <= ZOOM_EQUAL
        && (a.centre.x - b.centre.x).abs() <= CENTRE_EQUAL_MM
        && (a.centre.y - b.centre.y).abs() <= CENTRE_EQUAL_MM
}

/// Moves that crossed in either direction: `(main to pen, pen to main)`.
fn moves(pair: &Pair) -> (usize, usize) {
    let main = pair.views.iter().filter(|(m, v)| *m && v.moved).count();
    let pen = pair.views.iter().filter(|(m, v)| !*m && v.moved).count();
    (main, pen)
}

fn following() -> Pair {
    let mut pair = Pair::open(main_window());
    pair.pen.link_set_follow(true);
    pair.settle();
    pair
}

/// Criterion 5: off (the default) the views stay apart as in 2a.
#[test]
fn follow_view_off_keeps_the_views_apart() {
    let mut pair = Pair::open(main_window());
    assert!(!pair.pen.link_follow(), "off by default");
    let pen_before = view(&pair.pen);
    pair.main.wheel(40.0, 25.0, 400.0, 300.0, false, false);
    pair.main.zoom_step(true, 800.0, 450.0);
    pair.settle();
    assert!(same_view(view(&pair.pen), pen_before));
    pair.pen.wheel(-30.0, 10.0, 400.0, 300.0, false, false);
    let main_before = view(&pair.main);
    pair.settle();
    assert!(same_view(view(&pair.main), main_before));
    assert_eq!(moves(&pair), (0, 0));
}

/// Criterion 5: switched on, the pen window takes the main window's view at once (the main
/// window's view does not move).
#[test]
fn turning_follow_view_on_takes_the_main_windows_view() {
    let mut pair = Pair::open(main_window());
    pair.main.zoom_step(true, 300.0, 200.0);
    pair.settle();
    let main_view = view(&pair.main);
    assert!(!same_view(view(&pair.pen), main_view));
    pair.pen.link_set_follow(true);
    pair.settle();
    assert!(same_view(view(&pair.pen), main_view));
    assert!(
        same_view(view(&pair.main), main_view),
        "the main window keeps its view"
    );
    assert!(
        pair.main.link_follow(),
        "the main window knows the pen follows"
    );
}

/// Criterion 5 (its test): an alternating series of 20 pans and zooms ends with equal centre
/// and zoom; each change crosses once and nothing comes back (no echo).
#[test]
fn twenty_alternating_pans_and_zooms_end_equal_without_an_echo() {
    let mut pair = following();
    let start = moves(&pair);
    for i in 0..20 {
        let (mover, offset) = if i % 2 == 0 {
            (&mut pair.main, 0.0)
        } else {
            (&mut pair.pen, 5.0)
        };
        if i % 4 < 2 {
            mover.wheel(30.0 + offset, -20.0, 400.0, 300.0, false, false);
        } else {
            mover.zoom_step(i % 3 == 0, 500.0 + offset, 300.0);
        }
        pair.settle();
        assert!(same_view(view(&pair.main), view(&pair.pen)), "step {i}");
    }
    let (main, pen) = moves(&pair);
    assert_eq!(
        (main - start.0, pen - start.1),
        (10, 10),
        "one move per change"
    );
    // Quiet frames send nothing more.
    pair.settle();
    pair.settle();
    assert_eq!(moves(&pair), (main, pen));
}

/// Edge case: both windows zoom in the same frame; they settle on one view, the main window's,
/// and do not loop.
#[test]
fn crossing_changes_settle_on_the_main_windows_view() {
    let mut pair = following();
    pair.main.zoom_step(true, 400.0, 300.0);
    pair.pen.wheel(80.0, 60.0, 200.0, 200.0, false, false);
    pair.settle();
    let main_view = view(&pair.main);
    assert!(same_view(view(&pair.pen), main_view));
    let after = moves(&pair);
    pair.settle();
    pair.settle();
    assert_eq!(moves(&pair), after, "no loop");
}

/// Criterion 5: switched off, both keep the views they have.
#[test]
fn turning_follow_view_off_keeps_both_views() {
    let mut pair = following();
    pair.main.wheel(50.0, 0.0, 400.0, 300.0, false, false);
    pair.settle();
    pair.pen.link_set_follow(false);
    pair.settle();
    let (main, pen) = (view(&pair.main), view(&pair.pen));
    assert!(same_view(main, pen));
    pair.main.wheel(50.0, 0.0, 400.0, 300.0, false, false);
    pair.settle();
    assert!(same_view(view(&pair.pen), pen));
    assert!(!pair.main.link_follow());
}

/// View state is not document state: Follow view moves nothing in the document or the history.
#[test]
fn following_a_view_is_never_a_step() {
    let mut pair = following();
    let main_json = pair.main.document.export_json().unwrap();
    let undo = (pair.main.undo_len(), pair.pen.undo_len());
    pair.main.zoom_step(true, 400.0, 300.0);
    pair.settle();
    pair.pen.wheel(10.0, 10.0, 400.0, 300.0, false, false);
    pair.settle();
    assert_eq!(pair.main.document.export_json().unwrap(), main_json);
    assert!(pair.equal());
    assert_eq!((pair.main.undo_len(), pair.pen.undo_len()), undo);
}

fn assert_frame_is_the_pen_view(pair: &Pair) {
    let frame = pair
        .main
        .link_pen_visible()
        .expect("the main window has the pen view");
    let visible = pair.pen.view_rect_mm();
    for (a, b) in frame.iter().zip(visible) {
        assert!((a - b).abs() <= FRAME_TOLERANCE_MM, "{frame:?} {visible:?}");
    }
}

/// Criterion 6: after a pan, a zoom or a resize of the pen window the main window's frame is the
/// pen window's visible rectangle within 0.01 mm; the frame is in this canvas's pixels.
#[test]
fn the_frame_is_the_pen_windows_visible_rectangle() {
    let mut pair = Pair::open(main_window());
    pair.settle();
    assert_frame_is_the_pen_view(&pair);
    pair.pen.wheel(120.0, -45.0, 400.0, 300.0, false, false);
    pair.arrive();
    assert_frame_is_the_pen_view(&pair);
    pair.pen.zoom_step(false, 600.0, 360.0);
    pair.arrive();
    assert_frame_is_the_pen_view(&pair);
    pair.pen.resize_viewport(1366.0, 700.0);
    pair.arrive();
    assert_frame_is_the_pen_view(&pair);
    // On the main canvas: the frame's corners are the rectangle's corners on screen.
    let [left, top, _, _] = pair.main.link_pen_visible().unwrap();
    let [x0, y0, ..] = pair.main.link_pen_frame().unwrap();
    let back = pair.main.screen_to_document(x0, y0);
    assert!(
        (back.x - left).abs() < FRAME_TOLERANCE_MM && (back.y - top).abs() < FRAME_TOLERANCE_MM
    );
}

/// Criterion 6: with Follow view on the frame still is the pen window's own rectangle (same
/// centre as the main view, the pen window's shape).
#[test]
fn the_frame_follows_the_pen_window_while_following() {
    let mut pair = following();
    pair.main.zoom_step(true, 800.0, 450.0);
    pair.settle();
    assert_frame_is_the_pen_view(&pair);
    let [l, t, r, b] = pair.main.link_pen_visible().unwrap();
    let centre = view(&pair.main).centre;
    assert!((f64::midpoint(l, r) - centre.x).abs() < FRAME_TOLERANCE_MM);
    assert!((f64::midpoint(t, b) - centre.y).abs() < FRAME_TOLERANCE_MM);
}

/// Criterion 6: no frame without a pen window, and none after it is gone.
#[test]
fn there_is_no_frame_without_a_pen_window() {
    let main = main_window();
    assert_eq!(main.link_pen_frame(), None);
    let mut pair = Pair::open(main_window());
    pair.settle();
    assert!(pair.main.link_pen_frame().is_some());
    pair.main.link_end();
    assert_eq!(pair.main.link_pen_frame(), None);
}

/// Lead (2a observation): the pen window opens with the main window's selection, not only at
/// its next change.
#[test]
fn the_pen_window_opens_with_the_main_windows_selection() {
    let main = main_window();
    let selected = main.selection.ids().to_vec();
    assert_eq!(selected.len(), 1, "the drawn rectangle is selected");
    let pair = Pair::open(main);
    assert_eq!(pair.pen.selection.ids(), selected.as_slice());
    assert_eq!(pair.main.selection.ids(), selected.as_slice());
}

/// A new project in the main window starts the view exchange over: Follow stays on and the pen
/// window takes the main window's view of the new project.
#[test]
fn follow_view_survives_a_new_project() {
    let mut pair = following();
    let mut fresh = Session::new(0xA2);
    fresh.resize_viewport(1600.0, 900.0);
    fresh.link_continue(&mut pair.main);
    pair.main = fresh;
    pair.settle();
    pair.settle();
    assert!(pair.pen.link_follow());
    assert!(pair.main.link_follow());
    pair.main.zoom_step(true, 400.0, 300.0);
    pair.settle();
    assert!(same_view(view(&pair.main), view(&pair.pen)));
}
