//! A whole page put into a slot as a live placed page at session level
//! (`specs/0154-library-into-slots` criteria 3, 6, 7, 14, slice 2).
#![allow(clippy::float_cmp)]

use curvyo_document_core::{Affine, Angle, DocumentSize, FixtureTurn, NodeId, PageContents, Point};
use curvyo_laser_core::FitMode;

use super::super::Session;
use super::super::align_test_support::draw_rect;
use super::SlotSource;
use super::tests::last_label;
use super::tests_into_slot::{laser_with_slots, page_source, slot_centre, source};

/// The corners of the placed page `id` in document space.
fn corners(s: &Session, id: NodeId) -> [Point; 4] {
    s.document
        .placed_pages()
        .into_iter()
        .find(|p| p.id == id)
        .and_then(|p| p.frame_corners())
        .expect("a live placed page")
}

fn side(c: &[Point; 4], a: usize, b: usize) -> f64 {
    c[a].vector_to(c[b]).length()
}

fn centre(c: &[Point; 4]) -> Point {
    Point::new(f64::midpoint(c[0].x, c[2].x), f64::midpoint(c[0].y, c[2].y))
}

/// A drawing page of the open document of `w` x `h` mm with one rectangle; the laser page stays
/// active.
fn open_page(s: &mut Session, w: f64, h: f64) -> String {
    let laser = s.active_page().id;
    let id = s.add_page();
    s.document
        .resize_page(&id, DocumentSize::from_mm(w, h), &PageContents::default())
        .unwrap();
    let o = s.active_page().origin;
    draw_rect(s, (o.x + 2.0, o.y + 2.0), (o.x + w - 2.0, o.y + h - 2.0));
    s.activate_page(&laser);
    id
}

fn only_placement(s: &Session) -> NodeId {
    let placed = s.document.placed_pages();
    assert_eq!(placed.len(), 1, "one placed page");
    placed[0].id
}

#[test]
fn a_library_page_is_added_and_placed_fitted_in_one_undo_step() {
    let (mut s, holder, slots) = laser_with_slots();
    let library = source();
    let laser = s.active_page().id;
    let pages = s.document.pages().len();
    let steps = s.document.own_steps().len();
    let done = s
        .place_library_page_into_slot(&page_source(&library), slots[0], FitMode::Fit, false)
        .unwrap();
    assert_eq!(done.refused, None);
    assert_eq!(
        done.notice.as_deref(),
        Some("Fitted into Slot 1 at 50\u{a0}%.")
    );
    assert_eq!(s.document.own_steps().len(), steps + 1, "one step");
    assert_eq!(last_label(&s), "place_page_into_slot");
    assert_eq!(s.document.pages().len(), pages + 1, "a page was added");
    assert_eq!(s.active_page().id, laser, "the laser page stays active");
    let id = only_placement(&s);
    assert_eq!(s.selection.ids(), &[id]);
    let c = corners(&s, id);
    assert!((side(&c, 0, 1) - 30.0).abs() < 1e-6 && (side(&c, 1, 2) - 20.0).abs() < 1e-6);
    assert!(centre(&c).vector_to(slot_centre(&s, holder, 50.0)).length() < 1e-6);
    assert!(s.filled_slots().contains(&slots[0]), "a placed page fills");
    assert_eq!(s.member_slots().len(), 1, "Layers names its slot");
    s.undo();
    assert!(s.document.placed_pages().is_empty());
    assert_eq!(s.document.pages().len(), pages, "undo removes the page too");
    assert_eq!(s.selection.ids(), &[] as &[NodeId]);
}

#[test]
fn a_page_of_the_open_document_is_placed_turned_and_not_scaled_when_it_matches() {
    let (mut s, holder, slots) = laser_with_slots();
    let page = open_page(&mut s, 40.0, 60.0);
    let pages = s.document.pages().len();
    let done = s
        .place_page_into_slot(&page, slots[1], FitMode::Fit, false)
        .unwrap();
    assert_eq!(done.notice.as_deref(), Some("Turned to fit Slot 2."));
    assert_eq!(s.document.pages().len(), pages, "no page was added");
    let c = corners(&s, only_placement(&s));
    let mut sides = [side(&c, 0, 1), side(&c, 1, 2)];
    sides.sort_by(f64::total_cmp);
    assert!((sides[0] - 40.0).abs() < 1e-6 && (sides[1] - 60.0).abs() < 1e-6);
    assert!(
        ((c[2].x - c[1].x).abs() - 60.0).abs() < 1e-6,
        "the page height lies along x"
    );
    assert!(
        centre(&c)
            .vector_to(slot_centre(&s, holder, 150.0))
            .length()
            < 1e-6
    );
}

#[test]
fn alt_places_at_real_size_and_a_turned_holder_turns_the_placed_page() {
    let (mut s, holder, slots) = laser_with_slots();
    let page = open_page(&mut s, 60.0, 40.0);
    s.place_page_into_slot(&page, slots[0], FitMode::RealSize, false)
        .unwrap();
    let c = corners(&s, only_placement(&s));
    assert!((side(&c, 0, 1) - 60.0).abs() < 1e-6 && (side(&c, 1, 2) - 40.0).abs() < 1e-6);
    s.undo();
    let matrix = s.active_fixtures().unwrap().get(holder).unwrap().matrix;
    let turn = Affine::rotation_about(Point::new(0.0, 0.0), Angle::from_radians(0.5));
    s.document
        .set_fixture_matrix(holder, matrix.compose(turn), FixtureTurn::Rotate)
        .unwrap();
    s.place_page_into_slot(&page, slots[1], FitMode::Fit, false)
        .unwrap();
    let c = corners(&s, only_placement(&s));
    let along = (c[1].y - c[0].y).atan2(c[1].x - c[0].x);
    assert!(
        (along - 0.5).abs() < 1e-6,
        "turned with the holder: {along}"
    );
}

#[test]
fn a_filled_slot_refuses_and_replace_swaps_the_placed_page_as_a_reference() {
    let (mut s, _, slots) = laser_with_slots();
    let page = open_page(&mut s, 60.0, 40.0);
    s.place_page_into_slot(&page, slots[1], FitMode::Fit, false)
        .unwrap();
    let first = only_placement(&s);
    let steps = s.document.own_steps().len();
    let refused = s
        .place_page_into_slot(&page, slots[1], FitMode::Fit, false)
        .unwrap();
    assert_eq!(refused.refused.as_deref(), Some("Slot 2 is filled."));
    assert_eq!(s.document.own_steps().len(), steps, "nothing is written");
    s.place_page_into_slot(&page, slots[1], FitMode::Fit, true)
        .unwrap();
    assert_eq!(last_label(&s), "replace_slot_content");
    assert_eq!(s.document.own_steps().len(), steps + 1);
    assert_ne!(only_placement(&s), first, "the old reference went");
    assert!(
        s.document.pages().iter().any(|p| p.id == page),
        "its source page stays"
    );
}

#[test]
fn picks_and_laser_pages_are_never_placed_live() {
    let (mut s, _, slots) = laser_with_slots();
    let library = source();
    let came_from = page_source(&library).came_from;
    let picks = SlotSource {
        source: &library,
        roots: &[],
        page: None,
        came_from,
    };
    let steps = s.document.own_steps().len();
    let error = s
        .place_library_page_into_slot(&picks, slots[0], FitMode::Fit, false)
        .unwrap_err();
    assert_eq!(error, "Only a whole page can be placed live.");
    let laser = s.active_page().id;
    let error = s
        .place_page_into_slot(&laser, slots[0], FitMode::Fit, false)
        .unwrap_err();
    assert_eq!(error, "Only a drawing page can be placed live.");
    assert_eq!(s.document.own_steps().len(), steps, "nothing is written");
}

#[test]
fn a_page_row_released_over_a_slot_places_fitted_one_step_and_off_a_slot_does_nothing() {
    let (mut s, holder, slots) = laser_with_slots();
    let page = open_page(&mut s, 60.0, 40.0);
    assert!(s.row_drag_begin(&page));
    s.row_drag_move(Some(Point::new(-900.0, -900.0)));
    assert!(s.row_drag_drop_into_slot(false).is_none(), "off every slot");
    let steps = s.document.own_steps().len();
    s.row_drag_move(Some(slot_centre(&s, holder, 50.0)));
    let done = s.row_drag_drop_into_slot(false).unwrap().unwrap();
    assert_eq!(
        done.notice.as_deref(),
        Some("Fitted into Slot 1 at 50\u{a0}%.")
    );
    assert_eq!(s.document.own_steps().len(), steps + 1);
    assert!(s.filled_slots().contains(&slots[0]));
}
