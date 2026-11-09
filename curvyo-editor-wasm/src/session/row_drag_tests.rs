//! The page-row drag (`specs/0099-sketch-pages` criteria 8 to 10).
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

use curvyo_document_core::{FIRST_PAGE_ID, ObjectSnapshot, Point};

use super::{DropBadge, Session};
use crate::session::sketch_use::tests::{centre_of, page, setup};

fn centre(s: &Session, id: &str) -> Point {
    let p = page(s, id);
    Point::new(
        p.origin.x + p.size.width.as_mm() / 2.0,
        p.origin.y + p.size.height.as_mm() / 2.0,
    )
}

#[test]
fn a_drop_on_a_drawing_page_places_one_selected_clone_under_the_pointer() {
    let (mut s, sketch_page, sketch) = setup(99_081);
    s.activate_page(&sketch_page);
    let steps = s.undo_len();
    let before = s.document.version();
    assert!(s.row_drag_begin(&sketch_page));
    // Off the canvas: no badge, no target.
    assert_eq!(s.row_drag_move(None).badge, DropBadge::None);
    let at = Point::new(
        centre(&s, FIRST_PAGE_ID).x + 7.3,
        centre(&s, FIRST_PAGE_ID).y + 3.1,
    );
    let answer = s.row_drag_move(Some(at));
    assert_eq!(answer.badge, DropBadge::Clone);
    assert_eq!(answer.target.as_deref(), Some(FIRST_PAGE_ID));
    assert!(!s.draw_list().triangles.is_empty());
    assert_eq!(
        s.document.version(),
        before,
        "nothing is written before the drop"
    );
    assert!(s.row_drag_drop());
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(s.active_page().id, FIRST_PAGE_ID);
    let clone = s.selection.ids()[0];
    let Some(ObjectSnapshot::Clone(c)) = s.document.object(clone) else {
        panic!("not a clone");
    };
    assert_eq!(c.chain.root, sketch);
    assert!(c.chain.own_placement);
    // The box centre lies at the (possibly snapped) pointer: within a snap distance.
    assert!(centre_of(&s, clone).vector_to(at).length() < 5.0);
}

#[test]
fn a_drop_on_a_sketch_page_or_escape_writes_nothing() {
    let (mut s, sketch_page, _) = setup(99_082);
    let other = s.add_page_of_kind("sketch", None).unwrap();
    s.activate_page(&sketch_page);
    let steps = s.undo_len();
    assert!(s.row_drag_begin(&sketch_page));
    for id in [sketch_page.as_str(), other.as_str()] {
        let answer = s.row_drag_move(Some(centre(&s, id)));
        assert_eq!(answer.badge, DropBadge::NotOnSketchPage);
        assert!(answer.faint);
    }
    assert!(!s.row_drag_drop());
    assert_eq!(s.undo_len(), steps);
    assert!(s.row_drag_begin(&sketch_page));
    s.row_drag_move(Some(centre(&s, FIRST_PAGE_ID)));
    s.row_drag_cancel();
    assert!(!s.row_drag_drop());
    assert_eq!(s.undo_len(), steps);
    assert_eq!(s.active_page().id, sketch_page);
}

#[test]
fn an_empty_sketch_shows_the_refusal_badge_and_places_nothing() {
    let (mut s, _, _) = setup(99_083);
    let empty = s.add_page_of_kind("sketch", None).unwrap();
    let steps = s.undo_len();
    assert!(s.row_drag_begin(&empty));
    let answer = s.row_drag_move(Some(centre(&s, FIRST_PAGE_ID)));
    assert_eq!(answer.badge, DropBadge::NoSketch);
    assert!(!s.row_drag_drop());
    assert_eq!(s.undo_len(), steps);
    // Since `0100` a drawing row places a page; a laser row only reorders.
    let laser = s.add_page_of_kind("curvyo.laser", None).unwrap();
    assert!(!s.row_drag_begin(&laser), "a laser row only reorders");
}

#[test]
fn a_drop_on_the_pasteboard_keeps_the_active_page() {
    let (mut s, sketch_page, _) = setup(99_084);
    s.activate_page(&sketch_page);
    assert!(s.row_drag_begin(&sketch_page));
    let answer = s.row_drag_move(Some(Point::new(-400.0, -400.0)));
    assert_eq!(answer.badge, DropBadge::Clone);
    assert_eq!(answer.target, None);
    assert!(s.row_drag_drop());
    assert_eq!(s.active_page().id, sketch_page);
    assert_eq!(s.selection.ids().len(), 1);
}

#[test]
fn a_drop_past_the_clone_limit_is_refused_and_changes_nothing() {
    let (mut s, sketch_page, _) = setup(99_085);
    s.activate_page(&sketch_page);
    let steps = s.undo_len();
    let before = s.document.version();
    assert!(s.row_drag_begin(&sketch_page));
    s.row_drag_move(Some(Point::new(2e7, 0.0)));
    assert!(!s.row_drag_drop());
    assert_eq!(s.undo_len(), steps);
    assert_eq!(s.document.version(), before);
    assert_eq!(s.active_page().id, sketch_page);
}

/// A laser page next to the drawing page; its id and centre.
fn laser(s: &mut Session) -> (String, Point) {
    let id = s.add_page_of_kind("curvyo.laser", None).unwrap();
    let c = centre(s, &id);
    (id, c)
}

#[test]
fn criterion_13_a_drawing_page_row_dropped_on_a_laser_page_places_it_in_one_step() {
    let mut s = Session::new(100_013);
    let (laser, at) = laser(&mut s);
    s.activate_page(FIRST_PAGE_ID);
    let steps = s.undo_len();
    assert!(s.row_drag_begin(FIRST_PAGE_ID));
    let answer = s.row_drag_move(Some(at));
    assert_eq!(answer.badge, DropBadge::PlacedPage);
    assert_eq!(answer.target.as_deref(), Some(laser.as_str()));
    assert!(s.row_drag_drop());
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(s.active_page().id, laser);
    let placed = s.selected_placement().unwrap();
    let frame = s.placed_frame(placed).unwrap();
    let mid = Point::new(
        f64::midpoint(frame[0].x, frame[2].x),
        f64::midpoint(frame[0].y, frame[2].y),
    );
    assert!(
        mid.vector_to(at).length() < 5.0,
        "centred under the pointer"
    );
}

#[test]
fn criterion_14_a_drawing_page_row_elsewhere_says_only_on_a_laser_page_and_places_nothing() {
    let mut s = Session::new(100_014);
    laser(&mut s);
    s.activate_page(FIRST_PAGE_ID);
    let steps = s.undo_len();
    assert!(s.row_drag_begin(FIRST_PAGE_ID));
    let answer = s.row_drag_move(Some(centre(&s, FIRST_PAGE_ID)));
    assert_eq!(answer.badge, DropBadge::OnlyOnLaserPage);
    assert!(answer.faint);
    assert!(!s.row_drag_drop());
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn criterion_17_18_a_placed_page_drags_on_its_laser_page_and_snaps_back_off_it() {
    let mut s = Session::new(100_017);
    let (laser, at) = laser(&mut s);
    assert!(s.place_page_on_active(FIRST_PAGE_ID));
    assert_eq!(s.active_page().id, laser);
    let placed = s.selected_placement().unwrap();
    s.set_tool(crate::session::Tool::Select);
    let before = s.placed_frame(placed).unwrap();
    // A press anywhere in the frame selects and drags it.
    s.pointer_down(at, false);
    s.pointer_hover(Point::new(at.x + 10.0, at.y), false, false);
    s.pointer_up(Point::new(at.x + 10.0, at.y), false, false);
    let after = s.placed_frame(placed).unwrap();
    assert!((after[0].x - before[0].x - 10.0).abs() < 1e-9);
    // Far off the page: refused, nothing written, the notice.
    let steps = s.undo_len();
    s.pointer_down(at, false);
    s.pointer_up(Point::new(at.x + 10_000.0, at.y), false, false);
    assert_eq!(s.undo_len(), steps);
    assert_eq!(
        s.take_transform_notice(),
        "A placed page stays on its laser page."
    );
    assert_eq!(s.placed_frame(placed).unwrap(), after);
}
