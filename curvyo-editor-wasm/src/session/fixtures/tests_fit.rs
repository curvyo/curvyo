//! The fit rule during a drag at session level (`specs/0102-laser-fixtures` criteria 25 and 26):
//! 0.1 mm, centre to centre, turned by 90 degrees only when that is the fit, one step "Move to
//! slot", the cue in the drag answer, and a placed page dropped from the page list.

use curvyo_document_core::{NodeId, Point, RectBounds};

use super::super::Session;
use super::tests::{corner, laser, last_label, mm};

/// A laser page with a holder whose one slot is `w` by `h` at the board's centre; the slot's
/// centre in document space.
fn slotted(width: f64, height: f64) -> (Session, NodeId, Point) {
    let mut session = laser();
    let holder = session.add_fixture("holder").unwrap();
    super::tests::chip(&mut session, holder, 's', &["320", "240"]);
    super::tests::place(&mut session, holder, 10.0, 10.0);
    let slot = session.add_slot(holder).unwrap();
    session.set_slot_length(slot, "cx_mm", mm(160.0));
    session.set_slot_length(slot, "cy_mm", mm(120.0));
    session.set_slot_length(slot, "w_mm", mm(width));
    session.set_slot_length(slot, "h_mm", mm(height));
    session.clear_fixture_selection();
    let origin = session.active_page().origin;
    let frame = corner(&session, holder);
    let centre = Point::new(origin.x + frame.x + 160.0, origin.y + frame.y + 120.0);
    (session, slot, centre)
}

/// A `w` by `h` rectangle on the laser page with its top-left corner at `at`.
fn rect(s: &Session, at: Point, w: f64, h: f64) -> NodeId {
    s.document
        .create_rect(RectBounds::from_corners(at, Point::new(at.x + w, at.y + h)))
}

fn bounds(s: &Session, id: NodeId) -> (Point, Point) {
    let object = s.objects().iter().find(|o| o.id() == id).unwrap().clone();
    curvyo_ui_core::object_outline_bounds(&object)
}

/// Drags the rectangle `id` by its left edge so its centre ends `miss` off `target`, and
/// returns the cue seen before the release.
fn drag_to(s: &mut Session, id: NodeId, target: Point, miss: f64) -> Option<super::FitCue> {
    let (min, max) = bounds(s, id);
    let grip = Point::new(min.x, f64::midpoint(min.y, max.y));
    let centre = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
    let to = grip
        .translated(centre.vector_to(target))
        .translated(curvyo_document_core::Vec2::new(miss, miss));
    s.pointer_down(grip, false);
    s.pointer_hover(Point::new(grip.x + 5.0, grip.y), false, false);
    s.pointer_hover(to, false, false);
    let cue = s.fixture_fit_cue();
    s.pointer_up(to, false, false);
    cue
}

#[test]
fn a_part_of_the_slots_size_lands_centre_to_centre_in_one_step() {
    let (mut s, slot, target) = slotted(30.0, 20.0);
    let o = s.active_page().origin;
    let id = rect(&s, Point::new(o.x + 340.0, o.y + 20.0), 30.05, 20.0);
    let cue = drag_to(&mut s, id, target, 0.3).expect("fits");
    assert_eq!(cue.slot, slot.full_id());
    assert!(!cue.turned);
    assert_eq!(last_label(&s), "move_to_slot");
    let (min, max) = bounds(&s, id);
    let centre = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
    assert!(
        centre.vector_to(target).length() < 1e-6,
        "{centre:?} {target:?}"
    );
    assert!(
        ((max.x - min.x) - 30.05).abs() < 1e-9,
        "the size never changes"
    );
    assert_eq!(s.fixture_fit_cue(), None, "the cue ends with the release");
}

#[test]
fn the_other_orientation_is_a_fit_with_a_turn() {
    let (mut s, _, target) = slotted(20.0, 30.0);
    let o = s.active_page().origin;
    let id = rect(&s, Point::new(o.x + 340.0, o.y + 20.0), 30.0, 20.0);
    let cue = drag_to(&mut s, id, target, 0.3).expect("fits turned");
    assert!(cue.turned);
    let (min, max) = bounds(&s, id);
    assert!(((max.x - min.x) - 20.0).abs() < 1e-6 && ((max.y - min.y) - 30.0).abs() < 1e-6);
}

#[test]
fn a_square_part_in_a_square_slot_is_never_turned() {
    let (mut s, _, target) = slotted(20.0, 20.0);
    let o = s.active_page().origin;
    let id = rect(&s, Point::new(o.x + 340.0, o.y + 20.0), 20.0, 20.0);
    assert!(!drag_to(&mut s, id, target, 0.3).unwrap().turned);
}

#[test]
fn a_part_off_by_more_than_a_tenth_lands_where_released() {
    let (mut s, _, target) = slotted(30.0, 20.0);
    let o = s.active_page().origin;
    let id = rect(&s, Point::new(o.x + 340.0, o.y + 20.0), 30.2, 20.0);
    assert_eq!(drag_to(&mut s, id, target, 0.3), None);
    assert_ne!(last_label(&s), "move_to_slot");
}

#[test]
fn no_fit_while_the_holder_is_off_the_bed_or_the_switch_is_off() {
    let (mut s, _, target) = slotted(30.0, 20.0);
    let o = s.active_page().origin;
    let id = rect(&s, Point::new(o.x + 340.0, o.y + 20.0), 30.0, 20.0);
    s.set_snapping_fixtures(false);
    assert_eq!(drag_to(&mut s, id, target, 0.3), None);
}

#[test]
fn a_page_row_dropped_over_a_fitting_slot_is_placed_turned_into_it() {
    // A drawing page of 210 x 297 mm and a slot of 297 x 210 mm.
    let (mut s, slot, target) = slotted(297.0, 210.0);
    let source = s.document.pages()[0].id.clone();
    let size = s.document.pages()[0].size;
    assert_eq!((size.width.as_mm(), size.height.as_mm()), (210.0, 297.0));
    assert!(s.row_drag_begin(&source));
    let answer = s.row_drag_move(Some(Point::new(target.x + 0.4, target.y - 0.3)));
    let fit = answer.fit.expect("the cue is in the drag answer");
    assert_eq!(fit.slot, slot.full_id());
    assert!(fit.turned);
    assert!(s.row_drag_drop());
    let placed = &s.document.placed_pages()[0];
    assert!(placed.matrix.a.abs() < 1e-9 && (placed.matrix.b - 1.0).abs() < 1e-9);
}
