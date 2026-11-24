//! The frame of the active laser page (`specs/0118` slice 2b, criteria 22 to 27): the button's
//! view and causes, the frame text the connector gets, and the head ring while it runs.

use curvyo_laser_core::job::FrameShape;

use super::{cut, laser_with_square, view};
use crate::session::Session;

#[test]
fn criterion_27_a_page_without_output_has_nothing_to_frame() {
    let (s, _square) = laser_with_square();
    assert_eq!(view(&s)["frame"]["enabled"], false);
    assert_eq!(view(&s)["frame"]["cause"], "Nothing to frame");
    assert!(s.frame_bytes(FrameShape::Rectangle).is_none());
}

#[test]
fn criterion_22_a_page_with_a_burn_frames_and_writes_no_history_step() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    assert_eq!(view(&s)["frame"]["enabled"], true);
    assert!(view(&s)["frame"]["cause"].is_null());
    let steps = s.undo_len();
    let text = String::from_utf8(s.frame_bytes(FrameShape::Rectangle).unwrap()).unwrap();
    assert_eq!(s.undo_len(), steps, "a frame is no history step");
    // The square's four corners and back to the first, laser off.
    assert_eq!(text.lines().filter(|l| l.starts_with("G1 ")).count(), 5);
    assert!(text.lines().any(|l| l == "M5"));
    assert!(!text.contains("M3") && !text.contains("M4") && !text.contains("G0"));
    assert!(
        text.lines()
            .filter(|l| l.starts_with("G1 "))
            .all(|l| l.ends_with(" S0"))
    );
}

#[test]
fn criterion_23_rubber_band_and_rectangle_are_two_texts_of_the_same_page() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    let rectangle = s.frame_bytes(FrameShape::Rectangle).unwrap();
    let band = s.frame_bytes(FrameShape::RubberBand).unwrap();
    // A square's hull is its box.
    assert_eq!(rectangle, band);
}

#[test]
fn criterion_26_the_ring_shows_at_the_reported_position_and_leaves_with_the_frame() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    let view = curvyo_document_core::ViewTransform::identity();
    assert!(s.frame_head_overlay(view).is_none());
    assert!(s.set_frame_head(20.0, 20.0, false));
    assert!(s.frame_head_overlay(view).is_some());
    assert!(
        !s.toggle_play(0.0),
        "Play pauses while the ring shows (W34)"
    );
    assert!(s.clear_frame_head());
    assert!(s.frame_head_overlay(view).is_none());
    assert!(!s.clear_frame_head());
}

/// A Cut holding `items` made by a kind press, so it runs the copy's default numbers
/// (`specs/0150` criterion 4), with `power` typed if given.
fn cut_without_numbers(
    s: &mut Session,
    items: &[curvyo_document_core::NodeId],
    power: Option<f64>,
) {
    s.selection.set(items);
    let id = s.press_operation_kind("cut", true).unwrap();
    s.selection.set(&[]);
    if let Some(power) = power {
        s.edit_operation_setting(id, "main", "power", power)
            .unwrap();
    }
}

#[test]
fn frage_335_a_new_cut_frames_its_outline_and_exports() {
    for power in [None, Some(60.0)] {
        let (mut s, square) = laser_with_square();
        cut_without_numbers(&mut s, &[square], power);
        let (mut priced, priced_square) = laser_with_square();
        cut(&mut priced, &[priced_square]);
        // Since `specs/0150` criteria 23 and 25 a new Cut has numbers: Export is enabled and the
        // frame traces the same outline.
        assert_eq!(view(&s)["export"]["enabled"], true, "{power:?}");
        assert_eq!(view(&s)["frame"]["enabled"], true, "{power:?}");
        assert!(view(&s)["frame"]["cause"].is_null());
        for shape in [FrameShape::Rectangle, FrameShape::RubberBand] {
            let text = String::from_utf8(s.frame_bytes(shape).unwrap()).unwrap();
            let with_numbers = String::from_utf8(priced.frame_bytes(shape).unwrap()).unwrap();
            assert_eq!(text, with_numbers, "the outline as with numbers");
            assert!(text.lines().any(|l| l == "M5") && text.lines().any(|l| l == "M9"));
            assert!(!text.contains("M3") && !text.contains("M4") && !text.contains("G0"));
            let moves: Vec<&str> = text.lines().filter(|l| l.starts_with("G1 ")).collect();
            assert_eq!(moves.len(), 5, "{text}");
            assert!(moves.iter().all(|l| l.ends_with(" F3000 S0")), "{text}");
        }
        assert!(
            s.set_frame_head(20.0, 20.0, false),
            "the ring has a job to stand on"
        );
    }
}

#[test]
fn frage_335_a_cut_without_numbers_outside_the_bed_is_outside_the_work_area() {
    let (mut s, _) = laser_with_square();
    let origin = s.active_manufacturing().unwrap().page.unwrap().origin;
    let far = crate::session::align_test_support::draw_rect(
        &mut s,
        (origin.x + 395.0, origin.y + 200.0),
        (origin.x + 425.0, origin.y + 210.0),
    );
    cut_without_numbers(&mut s, &[far], None);
    assert_eq!(view(&s)["frame"]["enabled"], false);
    assert_eq!(view(&s)["frame"]["cause"], "Outside the work area");
    assert!(s.frame_bytes(FrameShape::Rectangle).is_none());
}
