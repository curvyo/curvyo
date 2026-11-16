//! Pins, pin and screw grids, their knobs and Jobs toggles, and Apply fixtures from page at
//! session level (`specs/0113-fixture-sets-and-pins` criteria 18 to 21, 25 to 27, 33 to 37).
#![allow(clippy::float_cmp, clippy::assert_is_empty)]

use curvyo_document_core::{FixtureJobSwitch, NodeId, Point};

use super::super::Session;
use super::handles::{FixtureHandle, fixture_handles};
use super::tests::{corner, laser, last_label, param, view};

/// A job pin dropped at the page-local `(x, y)`, selected, placement ended.
fn pin_at(s: &mut Session, x: f64, y: f64) -> NodeId {
    let o = s.active_page().origin;
    assert!(s.arm_job_fixture("pin"));
    s.pointer_down(Point::new(o.x + x, o.y + y), false);
    s.pointer_up(Point::new(o.x + x, o.y + y), false, false);
    s.end_fixture_placement();
    s.selected_fixture().unwrap()
}

fn handles_of(s: &Session, id: NodeId) -> Vec<FixtureHandle> {
    let (page, fixture) = s.page_fixture(id).unwrap();
    fixture_handles(
        &fixture,
        &page.page,
        &s.transform_handle_tolerances(),
        false,
    )
    .into_iter()
    .map(|(h, _)| h)
    .collect()
}

#[test]
fn criteria_18_19_a_pin_is_a_six_mm_circle_and_its_fields_clamp_with_notices() {
    let mut s = laser();
    let pin = pin_at(&mut s, 50.0, 50.0);
    let row = view(&s).job_fixtures[0].clone();
    let round = row.round.clone().unwrap();
    assert_eq!(round.diameter_mm, Some(6.0));
    assert_eq!((round.columns, round.rows), (1, 1));
    assert_eq!(row.line2, "Pin · ⌀ 6 mm");
    assert!(s.set_fixture_grid_value(pin, "diameter_mm", 2000.0));
    assert_eq!(last_label(&s), "resize_fixture");
    assert_eq!(
        s.take_fixture_notice().as_deref(),
        Some("Diameter is 0.5 to 1,000 mm. Set to 1,000 mm.")
    );
    assert!(s.set_fixture_grid_value(pin, "diameter_mm", 6.0));
    assert!(s.set_fixture_grid_value(pin, "columns", 20.0));
    assert!(s.set_fixture_grid_value(pin, "rows", 30.0));
    assert_eq!(
        s.take_fixture_notice().as_deref(),
        Some("At most 400 pins. Rows set to 20.")
    );
    assert_eq!(param(&s, pin, "rows"), 20.0, "stored as a whole count");
    assert!(s.set_fixture_grid_value(pin, "row_spacing_mm", 0.0));
    assert_eq!(
        s.take_fixture_notice().as_deref(),
        Some("Spacing is 1 to 1,000 mm. Set to 1 mm.")
    );
    assert!(
        view(&s).job_fixtures[0]
            .line2
            .starts_with("Pin · 20 × 20 · ⌀ 6 mm")
    );
}

#[test]
fn criterion_21_a_pin_grid_has_diameter_columns_and_rows_knobs_and_never_turns() {
    let mut s = laser();
    let pin = pin_at(&mut s, 40.0, 40.0);
    assert!(s.set_fixture_grid_value(pin, "diameter_mm", 400.0));
    let single = handles_of(&s, pin);
    assert!(single.contains(&FixtureHandle::Diameter), "at 72 px across");
    assert!(s.set_fixture_grid_value(pin, "diameter_mm", 6.0));
    let single = handles_of(&s, pin);
    assert!(!single.contains(&FixtureHandle::Diameter));
    assert!(
        !single.contains(&FixtureHandle::Columns),
        "hidden at count 1"
    );
    assert!(!single.iter().any(|h| matches!(h, FixtureHandle::Rotate(_))));
    assert!(s.set_fixture_grid_value(pin, "columns", 3.0));
    assert!(s.set_fixture_grid_value(pin, "rows", 2.0));
    let grid = handles_of(&s, pin);
    assert!(grid.contains(&FixtureHandle::Columns) && grid.contains(&FixtureHandle::Rows));
    assert!(!s.open_fixture_entry_for_key('s'), "S opens no chip (F11)");
    assert!(!s.open_fixture_entry_for_key('r'), "no R");
    // The Columns knob sits on the last pin of row 1: a drag of it changes the column spacing.
    let o = s.active_page().origin;
    let knob = Point::new(o.x + 80.0, o.y + 40.0);
    s.pointer_down(knob, false);
    s.pointer_hover(Point::new(o.x + 100.0, o.y + 40.0), false, false);
    assert_eq!(
        s.fixture_readout().map(|r| r.text).as_deref(),
        Some("Spacing 30.0 mm")
    );
    s.pointer_up(Point::new(o.x + 100.0, o.y + 40.0), false, false);
    assert_eq!(param(&s, pin, "column_spacing_mm"), 30.0);
    assert_eq!(
        corner(&s, pin),
        Point::new(40.0, 40.0),
        "the first pin stays"
    );
}

#[test]
fn criterion_19_a_grid_draws_every_pin_and_hits_by_its_rings() {
    let mut s = laser();
    let pin = pin_at(&mut s, 40.0, 40.0);
    s.set_fixture_grid_value(pin, "columns", 2.0);
    s.set_fixture_grid_value(pin, "rows", 2.0);
    let page = s.active_fixtures().unwrap();
    assert_eq!(s.fixture_look(&page).pins.len(), 4);
    s.clear_fixture_selection();
    let o = s.active_page().origin;
    // The ring of the pin at (60, 60), 3 mm out from its centre.
    s.pointer_down(Point::new(o.x + 63.0, o.y + 60.0), false);
    s.pointer_up(Point::new(o.x + 63.0, o.y + 60.0), false, false);
    assert_eq!(s.selected_fixture(), Some(pin));
}

#[test]
fn criterion_19_a_press_inside_a_pin_selects_it_as_a_solid_dowel() {
    let mut s = laser();
    let pin = pin_at(&mut s, 40.0, 40.0);
    s.set_fixture_grid_value(pin, "columns", 2.0);
    s.clear_fixture_selection();
    let o = s.active_page().origin;
    // Between the two pins (40, 40) and (60, 40), off every ring: nothing.
    s.pointer_down(Point::new(o.x + 50.0, o.y + 40.0), false);
    s.pointer_up(Point::new(o.x + 50.0, o.y + 40.0), false, false);
    assert_eq!(s.selected_fixture(), None);
    // 1 mm from the second pin's centre, well inside its 6 mm circle.
    s.pointer_down(Point::new(o.x + 61.0, o.y + 40.0), false);
    s.pointer_up(Point::new(o.x + 61.0, o.y + 40.0), false, false);
    assert_eq!(s.selected_fixture(), Some(pin));
}

#[test]
fn criteria_25_27_the_jobs_toggles_are_the_role_jobs_output() {
    let mut s = laser();
    let pin = pin_at(&mut s, 40.0, 40.0);
    let round = view(&s).job_fixtures[0].round.clone().unwrap();
    let mark = round.centre_mark.unwrap();
    let diameters = round.diameters.unwrap();
    assert!(!mark.on && !diameters.on, "both start off");
    let job = s.fixture_job_named(&mark.job).unwrap();
    assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Output, true));
    assert_eq!(last_label(&s), "set_fixture_job_output_on");
    let row = view(&s).job_fixtures[0].clone();
    assert!(row.round.unwrap().centre_mark.unwrap().on);
    assert!(row.line2.ends_with("Centre mark"), "{}", row.line2);
    let jobs = s.fixture_jobs_view().unwrap();
    let roles: Vec<Option<&str>> = jobs.groups[0].jobs.iter().map(|j| j.role).collect();
    assert_eq!(roles, [Some("centre_mark"), Some("diameters")]);
    assert!(jobs.add_to.is_none(), "a pin takes no further job");
    assert!(s.set_fixture_grid_value(pin, "mark_mm", 60.0));
    assert_eq!(
        s.take_fixture_notice().as_deref(),
        Some("Size is 0.5 to 50 mm. Set to 50 mm.")
    );
}

#[test]
fn ux_6_2_a_centre_mark_is_ink_while_output_is_off_and_only_the_score_look_while_on() {
    let mut s = laser();
    pin_at(&mut s, 40.0, 40.0);
    let mark = view(&s).job_fixtures[0]
        .round
        .clone()
        .unwrap()
        .centre_mark
        .unwrap();
    let job = s.fixture_job_named(&mark.job).unwrap();
    let ink = |s: &Session| {
        !s.fixture_look(&s.active_fixtures().unwrap())
            .marks
            .is_empty()
    };
    let scored = |s: &Session| {
        !s.fixture_job_triangles(&s.active_fixtures().unwrap())
            .is_empty()
    };
    assert!(ink(&s), "Output off: the marks in fixture ink, 1 px");
    assert!(!scored(&s), "and no Not assigned look over them");
    assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Output, true));
    assert!(scored(&s), "Output on: the Score look");
    assert!(!ink(&s), "with no ink line under its dashes");
    let page = s.active_page().id;
    assert!(s.toggle_operations_view(&page));
    assert!(ink(&s) && !scored(&s), "View: Drawn keeps the fixture ink");
}

/// Two laser pages; job fixtures on the first. The second is active.
fn two_pages() -> (Session, String, NodeId) {
    let mut s = laser();
    let first = s.active_page().id;
    pin_at(&mut s, 20.0, 20.0);
    let far = pin_at(&mut s, 30.0, 30.0);
    let o = s.active_page();
    s.write_fixture_matrix(
        far,
        curvyo_document_core::Affine::translation(curvyo_document_core::Vec2::new(
            o.size.width.as_mm() + 5.0,
            10.0,
        )),
        curvyo_document_core::FixtureTurn::Move,
    );
    let source = s.active_fixtures().unwrap().manufacturing;
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    assert_ne!(s.active_page().id, first);
    (s, first, source)
}

#[test]
fn criteria_33_to_37_apply_copies_the_job_fixtures_of_another_page() {
    let (mut s, _, source) = two_pages();
    let candidates = s.apply_fixture_candidates();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].count, 2);
    assert!(s.set_apply_preview(Some(source)));
    assert_eq!(view(&s).apply_picked, Some(source.full_id()));
    assert!(
        !s.fixture_overlay().triangles.is_empty(),
        "the dashed preview"
    );
    let steps = s.undo_len();
    assert_eq!(s.apply_fixtures_from_page(source), Some(2));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(last_label(&s), "apply_page_fixtures");
    assert_eq!(
        s.take_fixture_notice().as_deref(),
        Some("1 fixture lies outside the work area.")
    );
    let v = view(&s);
    assert_eq!(v.job_fixtures.len(), 2);
    assert_eq!(v.apply_picked, None, "the list folds");
    assert_eq!(
        s.selected_fixture(),
        None,
        "the copies are not selected (F7)"
    );
    let copy = s.fixture_part_named(&v.job_fixtures[0].id).unwrap();
    assert_eq!(
        corner(&s, copy),
        Point::new(20.0, 20.0),
        "same place, never mirrored"
    );
    assert!(
        s.apply_fixtures_from_page(source).is_some(),
        "a second Apply replaces the first copies"
    );
    assert_eq!(view(&s).job_fixtures.len(), 2);
}

#[test]
fn apply_onto_a_full_page_replaces_its_job_fixtures_in_one_step() {
    let (mut s, _, source) = two_pages();
    for k in 0..u32::try_from(curvyo_document_core::MAX_JOB_FIXTURES).unwrap() {
        let (column, row) = (f64::from(k % 16), f64::from(k / 16));
        pin_at(&mut s, 10.0 + column * 10.0, 50.0 + row * 10.0);
    }
    assert_eq!(
        view(&s).job_fixtures.len(),
        curvyo_document_core::MAX_JOB_FIXTURES
    );
    assert_eq!(
        s.apply_fixture_candidates().len(),
        1,
        "a full page still offers the source: Apply replaces"
    );
    let steps = s.undo_len();
    assert_eq!(s.apply_fixtures_from_page(source), Some(2));
    assert_eq!(s.undo_len(), steps + 1, "one step");
    assert_eq!(view(&s).job_fixtures.len(), 2, "replaced, not appended");
    s.undo();
    assert_eq!(
        view(&s).job_fixtures.len(),
        curvyo_document_core::MAX_JOB_FIXTURES,
        "one undo brings them back"
    );
}

#[test]
fn criterion_33_the_apply_list_leaves_out_this_page_and_pages_without_job_fixtures() {
    let mut s = laser();
    assert!(s.apply_fixture_candidates().is_empty());
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    assert!(s.apply_fixture_candidates().is_empty());
    pin_at(&mut s, 10.0, 10.0);
    assert!(
        s.apply_fixture_candidates().is_empty(),
        "not this page itself"
    );
}

#[test]
fn an_unchanged_frame_and_a_selection_rebuild_no_fixture_lists() {
    // The 0113 UX review: an idle frame and a row press rebuilt every pin of a 20 x 20 grid.
    let mut s = laser();
    let pin = pin_at(&mut s, 50.0, 50.0);
    assert!(s.set_fixture_grid_value(pin, "columns", 4.0));
    s.clear_fixture_selection();
    let builds = |s: &Session| s.fixtures.lists_cache.builds.get();
    let first = s.frame_draw_list();
    let before = builds(&s);
    assert_eq!(s.frame_draw_list(), first, "the same frame");
    assert_eq!(builds(&s), before, "an idle frame builds nothing");
    assert!(s.select_fixture(pin));
    let _ = s.frame_draw_list();
    assert_eq!(builds(&s), before, "a selection builds nothing");
    assert!(s.set_fixture_grid_value(pin, "rows", 3.0));
    let _ = s.frame_draw_list();
    assert_eq!(builds(&s), before + 1, "a change builds once");
}
