//! Page auto-layout and page drags measuring a variable-width stroke by its visible outline.
#![allow(clippy::unwrap_used)]

use curvyo_document_core::{AnchorId, NewAnchor, NodeId, Page, Point};

use crate::session::Session;

/// An L whose centre-line box centre lies on the page, but whose leg beyond the right edge is
/// widened so far that the centre of its visible outline lies on the pasteboard.
fn lopsided_widened_l(session: &mut Session) -> (Page, NodeId) {
    let page = session.active_page();
    let right = page.origin.x + page.size.width.as_mm();
    let id = session.document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(97, 1), Point::new(right - 10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(97, 2), Point::new(right + 2.0, 10.0)),
            NewAnchor::corner(AnchorId::new(97, 3), Point::new(right + 2.0, 40.0)),
        ],
        false,
    );
    let plain = curvyo_ui_core::object_outline_bounds(&session.document_read().leaves[0]);
    assert!(
        f64::midpoint(plain.0.x, plain.1.x) < right,
        "the centre line's centre is on the page"
    );
    session
        .document
        .set_width_point(id, AnchorId::new(97, 4), 1.0, 80.0)
        .unwrap();
    let object = session.document_read().leaves[0].clone();
    let (low, high) = session.page_fit_bounds(&object).unwrap();
    assert!(
        f64::midpoint(low.x, high.x) > right,
        "the visible outline's centre is on the pasteboard"
    );
    (page, id)
}

#[test]
fn page_auto_layout_counts_a_widened_stroke_by_its_visible_outline() {
    let mut session = Session::new(97);
    let (page, id) = lopsided_widened_l(&mut session);
    let contents = session.page_contents().unwrap();
    assert!(!contents.get(&page.id).is_some_and(|ids| ids.contains(&id)));
    let original = session.document.path(id).unwrap();
    assert!(session.move_page_to(&page.id, Point::new(10.0, 10.0)));
    assert_eq!(
        session.document.path(id).unwrap(),
        original,
        "the page leaves it behind"
    );
}

#[test]
fn a_page_drag_leaves_a_stroke_whose_visible_outline_centre_is_outside() {
    let mut session = Session::new(98);
    let (page, id) = lopsided_widened_l(&mut session);
    let original = session.document.path(id).unwrap();
    let edge = Point::new(
        page.origin.x - 1.0 / session.view().scale(),
        page.origin.y + 20.0,
    );
    assert!(session.page_drag_start(edge));
    assert!(session.page_drag_end(Point::new(edge.x - 30.0, edge.y)));
    assert_ne!(session.active_page().origin, page.origin, "the page moved");
    assert_eq!(
        session.document.path(id).unwrap(),
        original,
        "the stroke stayed"
    );
}
