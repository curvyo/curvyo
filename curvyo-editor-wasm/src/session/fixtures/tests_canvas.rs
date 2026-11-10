//! Fixtures on the canvas at session level (`specs/0102-laser-fixtures` criteria 5 to 11, 13, 14,
//! 24, 27): hit by the outline only on the active laser page, drags, keys, drawing and snap
//! targets.
#![allow(clippy::assert_is_empty)]

use curvyo_document_core::Point;

use super::super::{KeyInput, KeyOutcome, Session, Tool};
use super::keys::GRID_DELETE_NOTICE;
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

fn drag(s: &mut Session, from: Point, to: Point) {
    s.pointer_down(from, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

/// A holder on a fresh laser page and the document point of the middle of its top edge.
fn holder() -> (Session, curvyo_document_core::NodeId, Point) {
    let mut s = laser();
    let id = s.add_fixture("holder").unwrap();
    s.clear_fixture_selection();
    let o = s.active_page().origin;
    let c = corner(&s, id);
    (s, id, Point::new(o.x + c.x + 50.0, o.y + c.y))
}

#[test]
fn a_press_on_the_outline_selects_the_fixture_and_inside_does_not() {
    let (mut s, id, top) = holder();
    click(&mut s, Point::new(top.x, top.y + 30.0));
    assert_eq!(s.selected_fixture(), None, "inside a board is not a hit");
    click(&mut s, top);
    assert_eq!(s.selected_fixture(), Some(id));
    assert_eq!(s.selected_object_count(), 0, "never with objects");
    assert_eq!(corner(&s, id).x, corner(&s, id).x);
}

#[test]
fn a_slot_outline_selects_its_holder_and_marquee_or_select_all_never_does() {
    let (mut s, id, top) = holder();
    s.add_slot(id).unwrap();
    s.clear_fixture_selection();
    // The slot is 20 mm square at the board's centre: its top edge is 20 mm below the board's.
    click(&mut s, Point::new(top.x, top.y + 20.0));
    assert_eq!(s.selected_fixture(), Some(id));
    let o = s.active_page().origin;
    drag(
        &mut s,
        Point::new(o.x + 2.0, o.y + 2.0),
        Point::new(o.x + 390.0, o.y + 290.0),
    );
    assert_eq!(s.selected_fixture(), None);
    s.select_fixture(id);
    key(&mut s, "a", false);
    s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    assert_eq!(s.selected_fixture(), None, "Select all ends it");
}

#[test]
fn an_inactive_or_hidden_fixture_is_not_hit() {
    let (mut s, id, top) = holder();
    s.set_fixture_active(id, false);
    click(&mut s, top);
    assert_eq!(s.selected_fixture(), None);
    s.set_fixture_active(id, true);
    s.set_fixture_visible(id, false);
    click(&mut s, top);
    assert_eq!(s.selected_fixture(), None);
}

#[test]
fn a_fixture_of_another_laser_page_only_activates_that_page() {
    let (mut s, _, top) = holder();
    let first = s.active_page().id;
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    s.set_tool(Tool::Select);
    assert_ne!(s.active_page().id, first);
    click(&mut s, top);
    assert_eq!(s.selected_fixture(), None);
}

#[test]
fn a_drag_moves_the_holder_in_one_step_and_a_corner_rotate_glyph_turns_it() {
    let (mut s, id, top) = holder();
    let before = corner(&s, id);
    drag(&mut s, top, Point::new(top.x + 10.0, top.y + 5.0));
    assert_eq!(last_label(&s), "move_fixture");
    let after = corner(&s, id);
    assert!((after.x - before.x - 10.0).abs() < 1e-9 && (after.y - before.y - 5.0).abs() < 1e-9);
    let (fixtures, placed) = s.page_fixture(id).unwrap();
    let handle = super::handles::fixture_handles(
        &placed,
        &fixtures.page,
        &s.transform_handle_tolerances(),
        false,
    )
    .into_iter()
    .find(|(h, _)| *h == super::handles::FixtureHandle::Rotate(curvyo_ui_core::ResizeDirection::Nw))
    .map(|(_, at)| at)
    .expect("the corner rotate glyph");
    let centre = Point::new(top.x + 10.0, top.y + 35.0);
    // A quarter turn clockwise about the centre: the glyph goes from the top left to the top
    // right.
    let to = Point::new(
        centre.x - (handle.y - centre.y),
        centre.y + (handle.x - centre.x),
    );
    drag(&mut s, handle, to);
    assert_eq!(last_label(&s), "rotate_fixture");
    let m = s.document.fixture(id).unwrap().matrix.unwrap();
    assert!(m.a.abs() < 1e-6 && (m.b - 1.0).abs() < 1e-6, "{m:?}");
}

#[test]
fn the_grid_never_moves_and_delete_on_it_only_says_how() {
    let mut s = laser();
    let grid = s.add_fixture("grid").unwrap();
    assert_eq!(key(&mut s, "ArrowRight", false), KeyOutcome::Consumed);
    assert_eq!(key(&mut s, "Delete", false), KeyOutcome::Consumed);
    assert_eq!(s.take_fixture_notice().as_deref(), Some(GRID_DELETE_NOTICE));
    assert_eq!(view(&s).rows.len(), 1);
    assert_eq!(s.selected_fixture(), Some(grid));
}

#[test]
fn arrows_nudge_and_delete_arms_then_removes() {
    let (mut s, id, _) = holder();
    s.select_fixture(id);
    let before = corner(&s, id);
    key(&mut s, "ArrowRight", false);
    key(&mut s, "ArrowDown", true);
    let after = corner(&s, id);
    assert!((after.x - before.x - 1.0).abs() < 1e-9 && (after.y - before.y - 10.0).abs() < 1e-9);
    assert_eq!(key(&mut s, "Delete", false), KeyOutcome::Consumed);
    assert_eq!(s.armed_fixture(), Some(id));
    assert_eq!(view(&s).armed, Some(id.full_id()));
    assert_eq!(key(&mut s, "Delete", false), KeyOutcome::Deleted);
    assert!(view(&s).rows.is_empty());
}

#[test]
fn escape_ends_the_fixture_selection() {
    let (mut s, id, _) = holder();
    s.select_fixture(id);
    assert!(matches!(
        key(&mut s, "Escape", false),
        KeyOutcome::Escape(_)
    ));
    assert_eq!(s.selected_fixture(), None);
}

#[test]
fn fixtures_draw_until_off_the_bed_or_hidden_and_the_selection_adds_a_box() {
    let mut s = laser();
    let bare = s.frame_draw_list().triangles.len();
    let id = s.add_fixture("clamp").unwrap();
    s.clear_fixture_selection();
    let drawn = s.frame_draw_list().triangles.len();
    assert!(drawn > bare);
    s.select_fixture(id);
    assert!(
        s.frame_draw_list().triangles.len() > drawn,
        "box and rotate glyph"
    );
    s.set_fixture_active(id, false);
    assert_eq!(s.frame_draw_list().triangles.len(), bare);
    s.set_fixture_active(id, true);
    s.set_fixture_visible(id, false);
    assert_eq!(s.frame_draw_list().triangles.len(), bare);
}

#[test]
fn the_grid_draws_major_lines_tapes_and_its_numbers() {
    let mut s = laser();
    let bare = s.frame_draw_list().triangles.len();
    let grid = s.add_fixture("grid").unwrap();
    let with_tapes = s.frame_draw_list().triangles.len();
    assert!(with_tapes > bare);
    let tapes = s
        .fixture_labels()
        .iter()
        .filter(|l| l.kind == "tape")
        .count();
    assert!(tapes > 0, "numbers on the tapes");
    for edge in ["bottom", "left", "top", "right"] {
        s.set_grid_tape(grid, edge, false);
    }
    assert!(s.frame_draw_list().triangles.len() < with_tapes);
    // The selected grid keeps its tag chip (criterion 8); the tape numbers are gone.
    assert!(s.fixture_labels().iter().all(|l| l.kind == "tag"));
}

#[test]
fn snap_targets_come_from_active_fixtures_while_the_switch_is_on() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    s.add_slot(holder).unwrap();
    let stop = s.add_fixture("stop").unwrap();
    s.add_fixture("clamp").unwrap();
    assert_eq!(
        s.fixture_snap_targets(false).len(),
        8 + 9 + 5,
        "clamps give none"
    );
    s.set_fixture_visible(stop, false);
    assert_eq!(
        s.fixture_snap_targets(false).len(),
        22,
        "the eye keeps the targets"
    );
    s.set_fixture_active(stop, false);
    assert_eq!(s.fixture_snap_targets(false).len(), 17);
    s.set_snapping_fixtures(false);
    assert!(s.fixture_snap_targets(false).is_empty());
}
