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
    assert!(
        !s.row_drag_begin(FIRST_PAGE_ID),
        "a drawing row places nothing yet"
    );
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
