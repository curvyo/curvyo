//! The rectangle's handles and chips on fixtures (`specs/0102-laser-fixtures` criteria 14, 18,
//! 22, 23, 37, 38, 40; feature `adrs.md` §6a): the handle set per kind, the hit order, drags,
//! double-click chips, the slots of a resized holder and the screw.
#![allow(clippy::float_cmp, clippy::assert_is_empty)]

use curvyo_document_core::{NodeId, Point};
use curvyo_laser_core::StopPart;
use curvyo_ui_core::{EntryOutcome, ResizeDirection};

use super::super::Session;
use super::handles::{FixtureHandle, fixture_handles};
use super::tests::{corner, laser, last_label, param, place};

/// The document point of the handle `handle` of the fixture `id`.
fn handle_at(s: &Session, id: NodeId, handle: FixtureHandle) -> Point {
    let (fixtures, fixture) = s.page_fixture(id).unwrap();
    fixture_handles(
        &fixture,
        &fixtures.page,
        &s.transform_handle_tolerances(),
        false,
    )
    .into_iter()
    .find(|(h, _)| *h == handle)
    .map_or_else(|| panic!("no handle {handle:?}"), |(_, at)| at)
}

fn handles_of(s: &Session, id: NodeId) -> Vec<FixtureHandle> {
    let (fixtures, fixture) = s.page_fixture(id).unwrap();
    fixture_handles(
        &fixture,
        &fixtures.page,
        &s.transform_handle_tolerances(),
        false,
    )
    .into_iter()
    .map(|(h, _)| h)
    .collect()
}

fn drag(s: &mut Session, from: Point, to: Point) {
    s.pointer_down(from, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

/// The document point of the page-local `(x, y)`.
fn on_page(s: &Session, x: f64, y: f64) -> Point {
    let o = s.active_page().origin;
    Point::new(o.x + x, o.y + y)
}

#[test]
fn each_kind_keeps_the_handles_of_its_set_and_none_skews() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    let set = handles_of(&s, holder);
    let resize = set
        .iter()
        .filter(|h| matches!(h, FixtureHandle::Resize(_)))
        .count();
    let rotate = set
        .iter()
        .filter(|h| matches!(h, FixtureHandle::Rotate(_)))
        .count();
    assert_eq!((resize, rotate), (8, 4), "{set:?}");
    assert!(set.contains(&FixtureHandle::Move));
    let stop = s.add_fixture("stop").unwrap();
    let set = handles_of(&s, stop);
    assert!(set.contains(&FixtureHandle::Leg(StopPart::LegA)));
    assert!(set.contains(&FixtureHandle::Leg(StopPart::LegB)));
    assert!(set.contains(&FixtureHandle::Thickness), "{set:?}");
    assert!(!set.iter().any(|h| matches!(h, FixtureHandle::Resize(_))));
    let screw = s.add_fixture("screw").unwrap();
    // Knobs are drawn from a head of 72 px on screen.
    super::tests::chip(&mut s, screw, 's', &["100", "20"]);
    let set = handles_of(&s, screw);
    assert!(set.contains(&FixtureHandle::Head) && set.contains(&FixtureHandle::Thread));
    assert!(
        !set.iter()
            .any(|h| matches!(h, FixtureHandle::Rotate(_) | FixtureHandle::Resize(_))),
        "a screw does not turn or resize: {set:?}"
    );
    let head = handle_at(&s, screw, FixtureHandle::Head);
    let thread = handle_at(&s, screw, FixtureHandle::Thread);
    assert!(
        head.x > thread.x,
        "the head at 0 degrees, the thread at 180"
    );
    let grid = s.add_fixture("grid").unwrap();
    assert!(handles_of(&s, grid).is_empty());
}

#[test]
fn an_n_resize_is_one_step_and_its_slots_keep_their_place_on_the_sheet() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    place(&mut s, holder, 100.0, 100.0);
    let slot = s.add_slot(holder).unwrap();
    s.select_fixture(holder);
    let before = s.document.fixture(holder).unwrap();
    let n = handle_at(&s, holder, FixtureHandle::Resize(ResizeDirection::N));
    let steps = s.undo_len();
    drag(&mut s, n, Point::new(n.x, n.y - 20.0));
    assert_eq!(s.undo_len(), steps + 1, "one step");
    assert_eq!(last_label(&s), "resize_fixture");
    assert_eq!(param(&s, holder, "height_mm"), 80.0);
    assert_eq!(corner(&s, holder), Point::new(100.0, 80.0));
    let after = s.document.fixture(holder).unwrap();
    let cy =
        |f: &curvyo_document_core::FixtureSnapshot| f.slots[0].params["cy_mm"].as_f64().unwrap();
    assert_eq!(
        cy(&after),
        cy(&before) + 20.0,
        "{slot:?} stays on the sheet"
    );
    s.undo();
    assert_eq!(s.document.fixture(holder).unwrap(), before);
}

#[test]
fn the_selected_fixtures_handles_are_hit_before_its_outline() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    let e = handle_at(&s, holder, FixtureHandle::Resize(ResizeDirection::E));
    drag(&mut s, e, Point::new(e.x + 10.0, e.y));
    assert_eq!(
        last_label(&s),
        "resize_fixture",
        "the squircle on the outline wins"
    );
    assert!((param(&s, holder, "width_mm") - 110.0).abs() < 1e-9);
    let inside = on_page(&s, corner(&s, holder).x + 30.0, corner(&s, holder).y + 20.0);
    let before = corner(&s, holder);
    drag(&mut s, inside, Point::new(inside.x + 5.0, inside.y));
    assert_eq!(
        last_label(&s),
        "move_fixture",
        "inside the selected box moves it"
    );
    assert_eq!(corner(&s, holder).x, before.x + 5.0);
}

#[test]
fn a_stop_thickness_drag_moves_the_outer_edges_and_keeps_the_inner_corner() {
    let mut s = laser();
    let stop = s.add_fixture("stop").unwrap();
    let origin = corner(&s, stop);
    let knob = handle_at(&s, stop, FixtureHandle::Thickness);
    assert!(
        (knob.y - (s.active_page().origin.y + origin.y)).abs() < 1e-9,
        "on the top edge"
    );
    drag(&mut s, knob, Point::new(knob.x, knob.y - 5.0));
    assert_eq!(last_label(&s), "resize_fixture");
    assert_eq!(param(&s, stop, "thickness_mm"), 10.0);
    assert_eq!(
        corner(&s, stop),
        Point::new(origin.x - 5.0, origin.y - 5.0),
        "the outer corner moves out by 5 mm, the inner corner stays"
    );
    assert_eq!(param(&s, stop, "leg_a_mm"), 60.0);
    let leg = handle_at(&s, stop, FixtureHandle::Leg(StopPart::LegA));
    drag(&mut s, leg, Point::new(leg.x - 100.0, leg.y));
    assert_eq!(
        param(&s, stop, "leg_a_mm"),
        10.0,
        "a leg stops at the thickness"
    );
    assert!(s.take_fixture_notice().is_none(), "a drag shows no notice");
}

#[test]
fn a_screw_knob_drag_stops_at_the_other_circle_and_its_centre_stays() {
    let mut s = laser();
    let screw = s.add_fixture("screw").unwrap();
    // Knobs are drawn from a head of 72 px on screen.
    super::tests::chip(&mut s, screw, 's', &["100", "20"]);
    let centre = corner(&s, screw);
    let thread = handle_at(&s, screw, FixtureHandle::Thread);
    s.pointer_down(thread, false);
    s.pointer_hover(Point::new(thread.x - 10.0, thread.y), false, false);
    assert_eq!(s.live_readout().unwrap().text, "⌀ 40.0 mm");
    s.pointer_hover(Point::new(thread.x - 60.0, thread.y), false, false);
    assert_eq!(
        s.live_readout().unwrap().text,
        "⌀ 100.0 mm max",
        "at the head"
    );
    s.pointer_up(Point::new(thread.x - 60.0, thread.y), false, false);
    assert!(s.live_readout().is_none());
    assert_eq!(last_label(&s), "resize_fixture");
    assert_eq!(
        param(&s, screw, "thread_diameter_mm"),
        100.0,
        "stops at the head"
    );
    assert_eq!(corner(&s, screw), centre);
    assert!(s.take_fixture_notice().is_none());
    let head = handle_at(&s, screw, FixtureHandle::Head);
    drag(&mut s, head, Point::new(head.x + 2.0, head.y));
    assert!((param(&s, screw, "head_diameter_mm") - 104.0).abs() < 1e-9);
}

#[test]
fn a_double_click_on_the_w_squircle_opens_the_width_chip_and_enter_is_one_step() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    let w = handle_at(&s, holder, FixtureHandle::Resize(ResizeDirection::W));
    s.pointer_down(w, false);
    s.pointer_up(w, false, false);
    s.pointer_down(w, false);
    s.pointer_up(w, false, false);
    s.double_click(w, false, false);
    let view = s.transform_entry().expect("the chip opens");
    assert_eq!(view.kind, "size");
    let names: Vec<&str> = view.fields.iter().map(|f| f.accessible_name).collect();
    assert_eq!(names, ["Width (mm)"], "an edge squircle opens its one axis");
    let right = corner(&s, holder).x + 100.0;
    assert_eq!(
        s.commit_transform_entry("120", "", 0),
        EntryOutcome::Committed
    );
    assert_eq!(last_label(&s), "resize_fixture");
    assert_eq!(
        corner(&s, holder).x + 120.0,
        right,
        "the opposite side stays"
    );
    assert!(s.transform_entry().is_none(), "Enter closes the chip");
    assert!(s.select_fixture(holder) && s.open_fixture_entry_for_key('m'));
    let chip = s.move_entry().expect("the Move chip");
    assert!(!chip.copy_allowed, "a fixture is not cloned");
    s.cancel_transform_entry();
    assert!(s.move_entry().is_none());
}

#[test]
fn a_screw_is_drawn_as_two_rings_and_is_no_snap_target() {
    let mut s = laser();
    s.add_fixture("screw").unwrap();
    let fixtures = s.active_fixtures().unwrap();
    let look = s.fixture_look(&fixtures);
    assert_eq!(look.screws.len(), 1);
    assert_eq!(
        (look.screws[0].head_mm, look.screws[0].thread_mm),
        (10.0, 6.0)
    );
    assert!(s.fixture_snap_targets(false).is_empty(), "criterion 40");
}

#[test]
fn a_double_click_on_a_stop_knob_opens_its_one_field_chip_and_enter_commits_it() {
    let mut s = laser();
    let stop = s.add_fixture("stop").unwrap();
    for (handle, name, text, key, want) in [
        (
            FixtureHandle::Thickness,
            "Thickness (mm)",
            "8",
            "thickness_mm",
            8.0,
        ),
        (
            FixtureHandle::Leg(StopPart::LegA),
            "Leg 1 (mm)",
            "40",
            "leg_a_mm",
            40.0,
        ),
    ] {
        let at = handle_at(&s, stop, handle);
        for _ in 0..2 {
            s.pointer_down(at, false);
            s.pointer_up(at, false, false);
        }
        s.double_click(at, false, false);
        let view = s.transform_entry().expect("the knob's chip");
        assert_eq!(view.kind, "fixture-length");
        let names: Vec<&str> = view.fields.iter().map(|f| f.accessible_name).collect();
        assert_eq!(names, [name]);
        assert_eq!(
            s.commit_transform_entry_fields(&[text], 0),
            EntryOutcome::Committed
        );
        assert_eq!(param(&s, stop, key), want);
    }
}

#[test]
fn an_edge_squircle_opens_its_one_axis_and_a_corner_both() {
    use ResizeDirection::{E, N, S, Se, W};
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    let names = |s: &mut Session, handle: FixtureHandle| -> Vec<&'static str> {
        let at = handle_at(s, holder, handle);
        for _ in 0..2 {
            s.pointer_down(at, false);
            s.pointer_up(at, false, false);
        }
        s.double_click(at, false, false);
        let names = s
            .transform_entry()
            .expect("the chip")
            .fields
            .iter()
            .map(|f| f.accessible_name)
            .collect();
        s.cancel_transform_entry();
        names
    };
    assert_eq!(names(&mut s, FixtureHandle::Resize(N)), ["Height (mm)"]);
    assert_eq!(names(&mut s, FixtureHandle::Resize(S)), ["Height (mm)"]);
    assert_eq!(names(&mut s, FixtureHandle::Resize(E)), ["Width (mm)"]);
    assert_eq!(names(&mut s, FixtureHandle::Resize(W)), ["Width (mm)"]);
    assert_eq!(
        names(&mut s, FixtureHandle::Resize(Se)),
        ["Width (mm)", "Height (mm)"]
    );
    let n = handle_at(&s, holder, FixtureHandle::Resize(N));
    for _ in 0..2 {
        s.pointer_down(n, false);
        s.pointer_up(n, false, false);
    }
    s.double_click(n, false, false);
    assert_eq!(
        s.commit_transform_entry("80", "", 0),
        EntryOutcome::Committed
    );
    assert_eq!(param(&s, holder, "height_mm"), 80.0);
    assert_eq!(param(&s, holder, "width_mm"), 100.0);
}

#[test]
fn the_angle_chip_prefill_has_no_degree_sign_of_its_own() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    assert!(s.select_fixture(holder) && s.open_fixture_entry_for_key('r'));
    assert_eq!(s.transform_entry().unwrap().fields[0].prefill, "0");
}

#[test]
fn a_typed_width_and_height_both_past_their_limits_are_named_in_one_notice() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    assert_eq!(
        super::tests::chip(&mut s, holder, 's', &["5000", "0"]),
        EntryOutcome::Committed
    );
    let notice = s.live_readout().unwrap().text;
    assert_eq!(
        notice,
        "Width and height are 1 to 1,000 mm. Set to 1,000 and 1 mm."
    );
    assert!(notice.chars().count() <= 70);
}

#[test]
fn the_cursor_over_a_fixture_handle_is_the_rectangles() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    let hover = |s: &mut Session, p: Point| {
        s.pointer_hover(p, false, false);
        s.cursor_hint()
    };
    let e = handle_at(&s, holder, FixtureHandle::Resize(ResizeDirection::E));
    assert_eq!(hover(&mut s, e), "resize:0.0");
    let n = handle_at(&s, holder, FixtureHandle::Resize(ResizeDirection::N));
    assert_eq!(hover(&mut s, n), "resize:90.0");
    let rotate = handle_at(&s, holder, FixtureHandle::Rotate(ResizeDirection::Nw));
    assert_eq!(hover(&mut s, rotate), "rotate");
    let inside = on_page(&s, corner(&s, holder).x + 25.0, corner(&s, holder).y + 20.0);
    assert_eq!(hover(&mut s, inside), "move");
    // A turned holder turns its arrows.
    assert_eq!(
        super::tests::chip(&mut s, holder, 'r', &["30"]),
        EntryOutcome::Committed
    );
    let e = handle_at(&s, holder, FixtureHandle::Resize(ResizeDirection::E));
    assert_eq!(hover(&mut s, e), "resize:30.0");
    let stop = s.add_fixture("stop").unwrap();
    let knob = handle_at(&s, stop, FixtureHandle::Thickness);
    assert_eq!(hover(&mut s, knob), "pointer");
    let leg = handle_at(&s, stop, FixtureHandle::Leg(StopPart::LegA));
    assert_eq!(hover(&mut s, leg), "resize:0.0");
}

#[test]
fn a_fixture_drag_shows_the_rectangles_readouts_and_a_ghost_of_its_start() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    let readout_during = |s: &mut Session, from: Point, to: Point| -> (String, bool) {
        s.pointer_down(from, false);
        s.pointer_hover(to, false, false);
        let text = s.live_readout().map(|r| r.text).unwrap_or_default();
        let ghost = s.fixture_ghost().is_some_and(|g| !g.triangles.is_empty());
        s.pointer_up(to, false, false);
        (text, ghost)
    };
    let e = handle_at(&s, holder, FixtureHandle::Resize(ResizeDirection::E));
    assert_eq!(
        readout_during(&mut s, e, Point::new(e.x + 10.0, e.y)),
        ("110.0 × 60.0 mm".to_owned(), true)
    );
    let inside = on_page(&s, corner(&s, holder).x + 30.0, corner(&s, holder).y + 20.0);
    assert_eq!(
        readout_during(&mut s, inside, Point::new(inside.x + 5.0, inside.y - 2.0)).0,
        "Δ 5.0, \u{2212}2.0 mm"
    );
    let rotate = handle_at(&s, holder, FixtureHandle::Rotate(ResizeDirection::Nw));
    let (text, _) = readout_during(&mut s, rotate, Point::new(rotate.x + 40.0, rotate.y - 40.0));
    assert!(text.ends_with('°'), "{text}");
    assert!(s.fixture_ghost().is_none(), "no ghost after the release");
}
