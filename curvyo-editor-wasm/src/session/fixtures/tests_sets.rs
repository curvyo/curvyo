//! Machine sets, armed placement with centre snapping and Lock at session level
//! (`specs/0113-fixture-sets-and-pins` criteria 2 to 8, 13, 16, 17, 28, 30 to 32).
#![allow(clippy::float_cmp, clippy::assert_is_empty)]

use curvyo_document_core::{EllipseFrame, FixtureJobSwitch, Length, NodeId, Point};

use super::super::{KeyInput, KeyOutcome, Session, Tool};
use super::tests::{corner, laser, last_label, view};

fn key(s: &mut Session, k: &str, shift: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        shift,
        ..KeyInput::default()
    })
}

fn click(s: &mut Session, p: Point) {
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

/// The document point of the page-local `(x, y)`.
fn at(s: &Session, x: f64, y: f64) -> Point {
    let o = s.active_page().origin;
    Point::new(o.x + x, o.y + y)
}

/// A circle of 4 mm diameter centred at the page-local `(x, y)`.
fn circle(s: &Session, x: f64, y: f64) -> NodeId {
    s.document.create_ellipse(EllipseFrame {
        center: at(s, x, y),
        rx: Length::from_mm(2.0),
        ry: Length::from_mm(2.0),
    })
}

/// Switches the document's snapping on (a new document has it off, `0060` criterion 1).
fn snapping_on(s: &mut Session) {
    let mut on = s.snapping();
    on.enabled = true;
    assert!(s.set_snapping(on));
}

/// The page-local centre of the pin or screw `id`.
fn centre(s: &Session, id: NodeId) -> (f64, f64) {
    let c = corner(s, id);
    (c.x, c.y)
}

#[test]
fn criterion_3_plus_set_adds_set_n_on_this_page_unfolded_in_one_step() {
    let mut s = laser();
    let steps = s.undo_len();
    let a = s.add_fixture_set().unwrap();
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(last_label(&s), "add_fixture_set");
    let b = s.add_fixture_set().unwrap();
    let v = view(&s);
    assert_eq!(v.sets.len(), 2);
    assert_eq!(
        (v.sets[0].name.as_str(), v.sets[1].name.as_str()),
        ("Set 1", "Set 2")
    );
    assert!(v.sets.iter().all(|set| set.on_page && set.unfolded));
    assert_eq!(v.sets[0].line2, "No fixtures");
    assert_eq!(v.sets_on_page, 2);
    assert!(s.rename_fixture_set(b, "Coaster jig"));
    assert_eq!(view(&s).sets[1].name, "Coaster jig");
    assert!(s.move_fixture_set(b, 0));
    assert_eq!(view(&s).sets[0].name, "Coaster jig");
    assert!(s.set_fixture_set_unfolded(a, false));
    assert!(!view(&s).sets[1].unfolded, "folding is session state");
    assert_eq!(last_label(&s), "move_fixture_set", "and no step");
}

#[test]
fn criterion_17_plus_set_hides_at_sixteen_sets() {
    let mut s = laser();
    for _ in 0..16 {
        assert!(s.add_fixture_set().is_some());
    }
    assert!(!view(&s).can_add_set);
    assert!(s.add_fixture_set().is_none());
}

#[test]
fn criteria_2_and_8_a_set_off_this_page_draws_hits_and_runs_nothing() {
    let mut s = laser();
    let set = s.add_fixture_set().unwrap();
    let screw = s.add_set_fixture(set, "screw").unwrap();
    let holder = s.add_set_fixture(set, "holder").unwrap();
    assert_eq!(view(&s).sets[0].line2, "Screw, Holder · 2 fixtures");
    assert_eq!(s.selected_fixture(), Some(holder));
    let mark = s.fixture_jobs_view().unwrap().groups[0].jobs[0].id.clone();
    let mark = s.fixture_job_named(&mark).unwrap();
    assert!(s.set_fixture_job_switch(mark, FixtureJobSwitch::Output, true));
    assert_eq!(s.running_fixture_jobs(), 1);
    assert!(s.set_fixture_set_on_page(set, false));
    assert_eq!(last_label(&s), "set_fixture_set_off_page");
    assert_eq!(
        s.selected_fixture(),
        None,
        "a fixture of a set turned off is deselected"
    );
    assert!(!s.select_fixture(screw), "not selectable");
    assert_eq!(s.running_fixture_jobs(), 0, "its jobs do not run");
    let v = view(&s);
    assert_eq!(v.sets[0].line2, "Screw, Holder · not on this page");
    assert_eq!(v.sets_on_page, 0);
    assert!(v.sets[0].fixtures.iter().all(|f| !f.on_bed));
    assert!(v.sets[0].fixtures[0].line2.ends_with("not on this page"));
    let jobs = s.fixture_jobs_view().unwrap();
    assert!(!jobs.groups[0].jobs[0].on_this_page);
    let page = s.active_fixtures().unwrap();
    assert!(page.fixtures.iter().all(|f| !f.shown()));
    assert!(s.fixture_look(&page).screws.is_empty(), "nothing drawn");
    assert!(s.set_fixture_set_on_page(set, true));
    assert!(s.set_fixture_active(screw, false));
    assert_eq!(
        s.running_fixture_jobs(),
        0,
        "on the bed only with its own check on too (Frage 211 B)"
    );
    let tree = s.fixture_tree();
    assert_eq!(tree.len(), 2);
}

#[test]
fn criterion_5_removing_a_set_removes_its_fixtures_and_undo_restores_them() {
    let mut s = laser();
    let set = s.add_fixture_set().unwrap();
    s.add_set_fixture(set, "holder").unwrap();
    s.add_set_fixture(set, "pin").unwrap();
    assert!(s.remove_fixture_set(set));
    assert_eq!(last_label(&s), "remove_fixture_set");
    assert!(view(&s).sets.is_empty() && view(&s).rows().is_empty());
    s.undo();
    assert_eq!(view(&s).sets[0].fixtures.len(), 2);
}

#[test]
fn criterion_16_sets_list_first_in_their_order_then_job_fixtures() {
    let mut s = laser();
    assert!(s.arm_job_fixture("pin"));
    let p = at(&s, 10.0, 10.0);
    click(&mut s, p);
    s.end_fixture_placement();
    let a = s.add_fixture_set().unwrap();
    let b = s.add_fixture_set().unwrap();
    s.add_set_fixture(a, "holder").unwrap();
    s.add_set_fixture(b, "clamp").unwrap();
    s.move_fixture_set(b, 0);
    let names: Vec<String> = s.fixture_tree().into_iter().map(|row| row.1).collect();
    assert_eq!(names, ["Clamp 1", "Holder 1", "Pin 1"]);
}

#[test]
fn criterion_13_placement_drops_job_fixtures_and_stays_armed() {
    let mut s = laser();
    assert!(s.arm_job_fixture("pin"));
    assert_eq!(view(&s).armed_kind, Some(super::FixtureKindView::Pin));
    s.pointer_hover(at(&s, 40.0, 30.0), false, false);
    let steps = s.undo_len();
    let p = at(&s, 40.0, 30.0);
    click(&mut s, p);
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(last_label(&s), "add_job_fixture");
    let p = at(&s, 70.0, 30.0);
    click(&mut s, p);
    let v = view(&s);
    let names: Vec<&str> = v.job_fixtures.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["Pin 1", "Pin 2"]);
    assert!(!v.job_fixtures[0].locked);
    assert_eq!(
        centre(&s, s.fixture_part_named(&v.job_fixtures[1].id).unwrap()),
        (70.0, 30.0)
    );
    assert!(s.armed_job_fixture().is_some(), "stays armed (F4)");
    assert!(!s.arm_job_fixture("pin"), "a second press ends it");
    assert!(s.arm_job_fixture("screw"));
    assert!(matches!(
        key(&mut s, "Escape", false),
        KeyOutcome::Escape(_)
    ));
    assert!(s.armed_job_fixture().is_none());
    assert!(s.arm_job_fixture("holder"));
    s.set_tool(Tool::Pen);
    assert!(s.armed_job_fixture().is_none(), "another tool ends it");
    assert!(!s.arm_job_fixture("grid"), "no grid among job fixtures");
}

#[test]
fn criterion_13_while_armed_a_click_selects_nothing() {
    let mut s = laser();
    let set = s.add_fixture_set().unwrap();
    let holder = s.add_set_fixture(set, "holder").unwrap();
    s.clear_fixture_selection();
    let c = corner(&s, holder);
    assert!(s.arm_job_fixture("stop"));
    let p = at(&s, c.x + 50.0, c.y);
    click(&mut s, p);
    assert_ne!(s.selected_fixture(), Some(holder));
    assert_eq!(view(&s).job_fixtures.len(), 1, "the press dropped a stop");
}

#[test]
fn criterion_30_a_hanging_pin_snaps_its_centre_to_a_circle() {
    let mut s = laser();
    snapping_on(&mut s);
    circle(&s, 50.0, 40.0);
    assert!(s.arm_job_fixture("pin"));
    s.pointer_hover(at(&s, 50.4, 40.3), false, false);
    let p = at(&s, 50.4, 40.3);
    click(&mut s, p);
    let pin = s.fixture_part_named(&view(&s).job_fixtures[0].id).unwrap();
    let (x, y) = centre(&s, pin);
    assert!(
        (x - 50.0).abs() < 1e-9 && (y - 40.0).abs() < 1e-9,
        "{x}, {y}"
    );
    s.end_fixture_placement();
    let mut off = s.snapping();
    off.objects = false;
    s.set_snapping(off);
    assert!(s.arm_job_fixture("pin"));
    let p = at(&s, 50.4, 60.3);
    click(&mut s, p);
    let pin = s.fixture_part_named(&view(&s).job_fixtures[1].id).unwrap();
    let (x, y) = centre(&s, pin);
    assert!(
        (x - 50.4).abs() < 1e-9 && (y - 60.3).abs() < 1e-9,
        "no snap with Objects off"
    );
}

#[test]
fn criterion_30_keyboard_placement_starts_at_the_work_area_centre() {
    let mut s = laser();
    let size = s.active_page().size;
    assert!(s.arm_job_fixture("pin"));
    s.pointer_leave();
    assert!(s.nudge_fixture_placement(10.0, 0.0));
    let pin = s.drop_fixture_placement().unwrap();
    assert_eq!(
        centre(&s, pin),
        (size.width.as_mm() / 2.0 + 10.0, size.height.as_mm() / 2.0)
    );
}

#[test]
fn criterion_30_a_dragged_job_fixture_snaps_any_grid_centre() {
    let mut s = laser();
    snapping_on(&mut s);
    circle(&s, 100.0, 50.0);
    assert!(s.arm_job_fixture("pin"));
    let p = at(&s, 20.0, 20.0);
    click(&mut s, p);
    s.end_fixture_placement();
    let pin = s.selected_fixture().unwrap();
    assert!(s.set_fixture_grid_value(pin, "columns", 3.0));
    assert!(s.set_fixture_grid_value(pin, "column_spacing_mm", 30.0));
    s.clear_fixture_selection();
    // The last pin (x 80) is dragged near the circle: the grid moves by (20, 30) and a bit.
    let from = at(&s, 80.0 + 3.0, 20.0);
    s.pointer_down(from, false);
    s.pointer_hover(at(&s, 100.0 + 3.3, 50.4), false, false);
    s.pointer_up(at(&s, 100.0 + 3.3, 50.4), false, false);
    let (x, y) = centre(&s, pin);
    assert!(
        (x - 40.0).abs() < 1e-9 && (y - 50.0).abs() < 1e-9,
        "{x}, {y}"
    );
}

#[test]
fn criterion_32_the_lock_badge_yields_the_upper_right_to_the_job_chips() {
    // A pin's job chips stand at its box's upper right (`0103`); the badge takes the upper left.
    let mut s = laser();
    assert!(s.arm_job_fixture("pin"));
    let p = at(&s, 30.0, 30.0);
    click(&mut s, p);
    s.end_fixture_placement();
    let pin = s.selected_fixture().unwrap();
    assert!(s.set_fixture_locked(pin, true));
    let badge = s
        .fixture_labels()
        .into_iter()
        .find(|l| l.kind == "lock")
        .unwrap();
    let (x, y) = s.view().document_to_screen(at(&s, 27.0, 27.0));
    assert!(
        (badge.x - x).abs() < 1e-6 && (badge.y - y).abs() < 1e-6,
        "{badge:?} not at {x}, {y}"
    );
}

#[test]
fn criteria_31_32_a_locked_job_fixture_keeps_its_place() {
    let mut s = laser();
    assert!(s.arm_job_fixture("pin"));
    let p = at(&s, 30.0, 30.0);
    click(&mut s, p);
    s.end_fixture_placement();
    let pin = s.selected_fixture().unwrap();
    assert!(s.set_fixture_locked(pin, true));
    assert_eq!(last_label(&s), "lock_fixture");
    assert_eq!(s.selected_fixture_lock(), Some(true));
    let row = view(&s).job_fixtures[0].clone();
    assert!(row.line2.contains("locked"), "{}", row.line2);
    assert!(matches!(
        key(&mut s, "ArrowRight", false),
        KeyOutcome::Consumed
    ));
    assert_eq!(centre(&s, pin), (30.0, 30.0), "no nudge");
    assert!(!s.open_fixture_entry_for_key('m'), "no Move chip");
    let edge = at(&s, 33.0, 30.0);
    s.pointer_down(edge, false);
    s.pointer_hover(at(&s, 60.0, 60.0), false, false);
    s.pointer_up(at(&s, 60.0, 60.0), false, false);
    assert_eq!(centre(&s, pin), (30.0, 30.0), "no drag");
    assert_eq!(s.selected_fixture(), Some(pin), "still selectable");
    assert!(
        !s.set_fixture_grid_value(pin, "columns", 2.0),
        "no knob or field"
    );
    assert!(
        s.fixture_labels().iter().any(|l| l.kind == "lock"),
        "the lock badge"
    );
    assert!(s.delete_fixture(pin), "the trash still works");
    // Superseded by `0113` rework (criterion 32 amended 2026-11-16): a set fixture has its own
    // Lock and a locked set locks it, its own Lock then not pressable.
    let set = s.add_fixture_set().unwrap();
    let holder = s.add_set_fixture(set, "holder").unwrap();
    assert!(s.set_fixture_locked(holder, true), "a set fixture locks");
    assert!(s.set_fixture_locked(holder, false));
    assert!(s.set_fixture_set_locked(set, true));
    assert_eq!(last_label(&s), "lock_fixture_set");
    assert!(!s.set_fixture_locked(holder, true), "locked with its set");
    assert!(s.select_fixture(holder));
    assert_eq!(s.selected_fixture_lock(), Some(true));
    assert_eq!(s.selected_fixture_lock_pressable(), Some(false));
    assert!(s.add_set_fixture(set, "clamp").is_none(), "no add bar");
    let v = view(&s);
    assert!(v.sets[0].locked && !v.sets[0].can_add);
    assert!(v.sets[0].fixtures[0].locked_by_set);
    assert!(v.sets[0].line2.contains("locked"));
}

#[test]
fn criterion_17_the_job_bar_hides_at_sixty_four() {
    let mut s = laser();
    assert!(s.arm_job_fixture("pin"));
    for k in 0..64 {
        s.nudge_fixture_placement(0.0, 0.0);
        assert!(s.drop_fixture_placement().is_some(), "{k}");
    }
    assert!(!view(&s).can_add_job_fixture);
    assert!(
        s.armed_job_fixture().is_none(),
        "placement ends with the bar"
    );
    assert!(!s.arm_job_fixture("pin"));
}
