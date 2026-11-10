//! The Fixtures block's commands and view at session level (`specs/0102-laser-fixtures` criteria
//! 1 to 6, 14, 15, 18 to 20, 22, 23, 28, 29, 33 and 37).
#![allow(clippy::float_cmp, clippy::assert_is_empty)]

use curvyo_document_core::{Affine, FixtureTurn, Length, NodeId, Point, Vec2};
use curvyo_ui_core::EntryOutcome;

use super::super::{Session, Tool};
use super::FixtureKindView;

pub(super) fn laser() -> Session {
    let mut s = Session::new(1);
    s.resize_viewport(1200.0, 800.0);
    s.show_default_view();
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    s.set_tool(Tool::Select);
    s
}

pub(super) fn view(s: &Session) -> super::FixturesView {
    s.fixtures_view().expect("a laser page")
}

pub(super) fn last_label(s: &Session) -> String {
    s.document.own_steps().last().unwrap().label.clone()
}

pub(super) fn mm(v: f64) -> Length {
    Length::from_mm(v)
}

/// The page-local frame corner of the fixture `id`.
pub(super) fn corner(s: &Session, id: NodeId) -> Point {
    let m = s.document.fixture(id).unwrap().matrix.unwrap();
    Point::new(m.e, m.f)
}

/// Puts the frame origin of the fixture `id` at the page-local `(x, y)` in one step.
pub(super) fn place(s: &mut Session, id: NodeId, x: f64, y: f64) -> bool {
    let m = s.document.fixture(id).unwrap().matrix.unwrap();
    let moved = Affine::translation(Vec2::new(x - m.e, y - m.f)).compose(m);
    s.write_fixture_matrix(id, moved, FixtureTurn::Move)
}

/// Selects the fixture `id`, opens the chip of the key `key` (`m`, `r`, `s`) and commits `texts`
/// in it (the Move chip in Relative mode).
pub(super) fn chip(s: &mut Session, id: NodeId, key: char, texts: &[&str]) -> EntryOutcome {
    assert!(s.select_fixture(id));
    assert!(s.open_fixture_entry_for_key(key), "the chip {key} opens");
    if key == 'm' {
        let pair = [texts[0], texts.get(1).copied().unwrap_or("0")];
        return s.commit_move_entry(
            pair[0],
            pair[1],
            curvyo_ui_core::MoveEntryMode {
                absolute: false,
                copy: false,
            },
        );
    }
    s.commit_transform_entry_fields(texts, 0)
}

/// The stored parameter `key` of the fixture `id`.
pub(super) fn param(s: &Session, id: NodeId, key: &str) -> f64 {
    s.document.fixture(id).unwrap().params[key]
        .as_f64()
        .unwrap()
}

#[test]
fn a_new_laser_page_shows_five_add_buttons_and_no_row() {
    let s = laser();
    let v = view(&s);
    assert!(v.rows.is_empty());
    assert!(v.can_add_grid);
    assert!(v.snapping_fixtures);
    assert!(
        Session::new(1).fixtures_view().is_none(),
        "off a laser page"
    );
}

#[test]
fn each_add_button_adds_one_active_selected_fixture_centred_in_one_step() {
    let mut s = laser();
    let size = s.active_page().size;
    for (kind, label, name) in [
        ("holder", "add_holder", "Holder 1"),
        ("clamp", "add_clamp", "Clamp 1"),
        ("stop", "add_stop", "Stop 1"),
        ("screw", "add_screw", "Screw 1"),
        ("grid", "add_grid", "Grid 1"),
    ] {
        let steps = s.undo_len();
        let id = s.add_fixture(kind).unwrap();
        assert_eq!(s.undo_len(), steps + 1);
        assert_eq!(last_label(&s), label);
        assert_eq!(s.selected_fixture(), Some(id));
        let row = view(&s)
            .rows
            .into_iter()
            .find(|r| r.id == id.full_id())
            .unwrap();
        assert_eq!(row.name, name);
        assert!(row.active && row.visible);
    }
    let holder = view(&s).rows[0].clone();
    assert_eq!(holder.kind, FixtureKindView::Holder);
    let id = s.fixture_part_named(&holder.id).unwrap();
    assert_eq!(
        (param(&s, id, "width_mm"), param(&s, id, "height_mm")),
        (100.0, 60.0)
    );
    let c = corner(&s, id);
    assert_eq!(c.x, (size.width.as_mm() - 100.0) / 2.0);
    assert_eq!(c.y, (size.height.as_mm() - 60.0) / 2.0);
    let stop = s.fixture_part_named(&view(&s).rows[2].id).unwrap();
    assert_eq!(
        (
            param(&s, stop, "leg_a_mm"),
            param(&s, stop, "leg_b_mm"),
            param(&s, stop, "thickness_mm")
        ),
        (60.0, 60.0, 5.0)
    );
    let screw = view(&s).rows[3].clone();
    assert_eq!(screw.kind, FixtureKindView::Screw);
    let screw = s.fixture_part_named(&screw.id).unwrap();
    assert_eq!(
        (
            param(&s, screw, "thread_diameter_mm"),
            param(&s, screw, "head_diameter_mm")
        ),
        (6.0, 10.0)
    );
    let centre = corner(&s, screw);
    assert_eq!(
        (centre.x, centre.y),
        (size.width.as_mm() / 2.0, size.height.as_mm() / 2.0),
        "a screw's frame origin is its centre"
    );
    assert!(!view(&s).can_add_grid, "the Grid button hides");
    assert!(s.add_fixture("grid").is_none(), "one grid per page");
    assert!(s.add_fixture("pocket").is_none());
    assert_eq!(
        s.add_fixture("holder")
            .map(|_| view(&s).rows[5].name.clone())
            .unwrap(),
        "Holder 2"
    );
}

#[test]
fn rename_active_eye_and_remove_are_one_step_each_and_undo_restores_slots() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    s.add_slot(holder).unwrap();
    assert!(s.rename_fixture(holder, "Coaster board"));
    assert_eq!(last_label(&s), "rename_fixture");
    assert!(s.set_fixture_active(holder, false));
    assert_eq!(last_label(&s), "set_fixture_inactive");
    assert_eq!(s.selected_fixture(), None, "off the bed ends the selection");
    assert!(s.set_fixture_active(holder, true));
    assert_eq!(last_label(&s), "set_fixture_active");
    assert!(s.set_fixture_visible(holder, false));
    assert_eq!(last_label(&s), "set_fixture_visible");
    let before = s.document.fixture(holder).unwrap();
    let steps = s.undo_len();
    assert!(s.delete_fixture(holder));
    assert_eq!(s.undo_len(), steps + 1);
    assert!(view(&s).rows.is_empty());
    s.undo();
    let back = &s.document.manufacturings()[0].fixtures[0];
    assert_eq!(
        back.record(),
        before.record(),
        "slots, Active and place come back"
    );
}

#[test]
fn several_fixtures_may_be_active_at_once() {
    let mut s = laser();
    let a = s.add_fixture("holder").unwrap();
    let b = s.add_fixture("clamp").unwrap();
    assert!(s.set_fixture_active(a, true) || view(&s).rows[0].active);
    assert!(view(&s).rows.iter().all(|r| r.active));
    assert!(s.set_fixture_active(b, false));
    assert!(view(&s).rows[0].active);
}

#[test]
fn grid_fields_are_set_to_their_nearest_limit_with_a_notice() {
    let mut s = laser();
    let grid = s.add_fixture("grid").unwrap();
    let g = view(&s).rows[0].grid.clone().unwrap();
    assert_eq!((g.spacing_mm, g.major_every, g.origin_x_mm), (10.0, 5, 0.0));
    assert!(g.tapes.bottom && g.tapes.left && g.tapes.top && g.tapes.right);
    assert!(s.set_fixture_length(grid, "spacing_mm", mm(500.0)));
    assert_eq!(
        s.take_fixture_notice().as_deref(),
        Some("Spacing is 1 to 100 mm. Set to 100 mm.")
    );
    assert_eq!(view(&s).rows[0].grid.clone().unwrap().spacing_mm, 100.0);
    assert!(s.set_grid_major_every(grid, 1.0));
    assert_eq!(view(&s).rows[0].grid.clone().unwrap().major_every, 2);
    assert!(s.take_fixture_notice().unwrap().len() <= 70);
    assert!(s.set_grid_tape(grid, "top", false));
    let tapes = view(&s).rows[0].grid.clone().unwrap().tapes;
    assert!(!tapes.top && tapes.bottom && tapes.left && tapes.right);
    assert!(s.set_fixture_length(grid, "origin_x_mm", mm(3.0)));
    assert_eq!(view(&s).rows[0].grid.clone().unwrap().origin_x_mm, 3.0);
    assert!(
        !s.set_fixture_length(grid, "width_mm", mm(3.0)),
        "a grid has no width"
    );
    assert!(s.select_fixture(grid));
    assert!(!s.open_fixture_entry_for_key('m'), "the grid never moves");
}

#[test]
fn holder_clamp_stop_and_screw_have_no_block_field_and_their_chips_set_the_limits() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    let clamp = s.add_fixture("clamp").unwrap();
    let stop = s.add_fixture("stop").unwrap();
    let screw = s.add_fixture("screw").unwrap();
    let steps = s.undo_len();
    for (id, key) in [
        (holder, "width_mm"),
        (clamp, "height_mm"),
        (stop, "thickness_mm"),
        (screw, "head_diameter_mm"),
    ] {
        assert!(
            !s.set_fixture_length(id, key, mm(5.0)),
            "no block field: {key}"
        );
    }
    assert_eq!(s.undo_len(), steps);
    assert!(view(&s).rows.iter().all(|r| r.grid.is_none()));
    assert_eq!(
        chip(&mut s, holder, 's', &["2000", "60.0"]),
        EntryOutcome::Committed
    );
    assert_eq!(param(&s, holder, "width_mm"), 1000.0);
    assert_eq!(last_label(&s), "resize_fixture");
    assert_eq!(
        s.live_readout().map(|r| r.text).as_deref(),
        Some("Width is 1 to 1,000 mm. Set to 1,000 mm.")
    );
    assert_eq!(
        chip(&mut s, clamp, 's', &["40.0", "0"]),
        EntryOutcome::Committed
    );
    assert_eq!(param(&s, clamp, "height_mm"), 1.0);
    assert_eq!(
        chip(&mut s, stop, 's', &["60.0", "20", "80"]),
        EntryOutcome::Committed
    );
    assert_eq!(param(&s, stop, "leg_b_mm"), 20.0);
    assert_eq!(
        param(&s, stop, "thickness_mm"),
        20.0,
        "at most the shorter leg"
    );
    assert_eq!(
        s.live_readout().map(|r| r.text).as_deref(),
        Some("Thickness is 0.5 to 20 mm. Set to 20 mm.")
    );
    assert_eq!(
        chip(&mut s, screw, 's', &["10.0", "12"]),
        EntryOutcome::Committed
    );
    assert_eq!(
        param(&s, screw, "thread_diameter_mm"),
        10.0,
        "never above the head"
    );
    assert_eq!(
        s.live_readout().map(|r| r.text).as_deref(),
        Some("Thread diameter is 0.5 to 10 mm. Set to 10 mm.")
    );
    assert_eq!(
        chip(&mut s, screw, 's', &["4", "6"]),
        EntryOutcome::Committed
    );
    assert_eq!(
        param(&s, screw, "head_diameter_mm"),
        10.0,
        "never below the thread"
    );
    assert_eq!(param(&s, screw, "thread_diameter_mm"), 6.0);
    assert_eq!(
        s.live_readout().map(|r| r.text).as_deref(),
        Some("Head diameter is 10 to 1,000 mm. Set to 10 mm.")
    );
}

#[test]
fn slots_are_added_edited_and_removed_and_hidden_at_fifty() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    let slot = s.add_slot(holder).unwrap();
    assert_eq!(last_label(&s), "add_slot");
    assert_eq!(s.selected_slot(), Some(slot));
    let row = view(&s).rows[0].slots[0].clone();
    assert_eq!((row.name.as_str(), row.shape), ("Slot 1", "rect"));
    assert_eq!(
        (row.cx_mm, row.cy_mm, row.w_mm, row.h_mm),
        (50.0, 30.0, 20.0, 20.0)
    );
    assert!(!row.outside);
    assert!(s.set_slot_shape(slot, "circle"));
    assert_eq!(view(&s).rows[0].slots[0].shape, "circle");
    assert!(s.set_slot_length(slot, "d_mm", mm(0.1)));
    assert_eq!(view(&s).rows[0].slots[0].w_mm, 0.5, "set to its limit");
    assert!(s.take_fixture_notice().is_some());
    assert!(s.set_slot_length(slot, "cx_mm", mm(200.0)));
    assert!(view(&s).rows[0].slots[0].outside, "allowed and drawn");
    assert_eq!(
        s.take_fixture_notice().as_deref(),
        Some("Slot Slot 1 lies outside Holder 1.")
    );
    assert!(s.rename_slot(slot, "Coaster"));
    assert_eq!(view(&s).rows[0].slots[0].name, "Coaster");
    assert!(s.remove_slot(slot));
    assert_eq!(last_label(&s), "remove_slot");
    for _ in 0..50 {
        s.add_slot(holder).unwrap();
    }
    assert!(!view(&s).rows[0].can_add_slot);
    assert!(s.add_slot(holder).is_none());
}

#[test]
fn the_move_and_angle_chips_move_and_turn_in_one_step_each() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    assert!(place(&mut s, holder, 10.0, 20.0));
    assert_eq!(
        chip(&mut s, holder, 'm', &["5", "-5"]),
        EntryOutcome::Committed
    );
    assert_eq!(last_label(&s), "move_fixture");
    assert_eq!(corner(&s, holder), Point::new(15.0, 15.0));
    assert_eq!(chip(&mut s, holder, 'r', &["90"]), EntryOutcome::Committed);
    assert_eq!(last_label(&s), "rotate_fixture");
    let m = s.document.fixture(holder).unwrap().matrix.unwrap();
    assert!(m.a.abs() < 1e-9 && (m.b - 1.0).abs() < 1e-9);
    // The turn is about the board's centre (65, 45 on the page).
    let centre = m.apply_point(Point::new(50.0, 30.0));
    assert!((centre.x - 65.0).abs() < 1e-9 && (centre.y - 45.0).abs() < 1e-9);
    assert_eq!(
        chip(&mut s, holder, 'r', &["x"]),
        EntryOutcome::Invalid {
            field: 0,
            reason: curvyo_ui_core::InvalidReason::NotANumber
        }
    );
    let screw = s.add_fixture("screw").unwrap();
    assert!(
        !s.open_fixture_entry_for_key('r'),
        "a screw does not turn: {screw:?}"
    );
}

#[test]
fn the_snap_switch_is_one_step_and_reads_on_by_default() {
    let mut s = laser();
    assert!(s.set_snapping_fixtures(false));
    assert_eq!(last_label(&s), "set_fixtures_snap_off");
    assert!(!view(&s).snapping_fixtures);
    s.undo();
    assert!(view(&s).snapping_fixtures);
}

#[test]
fn a_damaged_value_reads_as_its_default_with_one_notice() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    let mut params = s.document.fixture(holder).unwrap().params;
    params.insert("width_mm".to_owned(), serde_json::json!("wide"));
    s.document.set_fixture_params(holder, &params).unwrap();
    let v = view(&s);
    assert_eq!(
        v.selection_announcement.as_deref(),
        Some("Holder 1 selected, 100.0 by 60.0 mm."),
        "the default width"
    );
    assert_eq!(v.damage_notices, ["1 value damaged on Holder 1."]);
}

#[test]
fn a_new_laser_page_starts_with_the_profiles_set_and_differs_compares_it() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    s.add_slot(holder).unwrap();
    let id = s.save_machine_profile([7; 16], 1).unwrap_or_default();
    let machine = |s: &Session| -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(&s.manufacturing_view_json()).unwrap()["machine"]
            .clone()
    };
    assert_eq!(machine(&s)["fixtures_in_profile"], 1, "{id}");
    assert!(machine(&s)["state"].is_null(), "same set");
    assert!(place(&mut s, holder, 1.0, 1.0));
    assert_eq!(machine(&s)["state"], "differs", "the set is compared");
    let profile = machine(&s)["copy"]["id"].as_str().unwrap().to_owned();
    let steps = s.undo_len();
    s.add_output_page_with("curvyo.laser", Some(&profile))
        .unwrap();
    assert_eq!(s.undo_len(), steps + 1, "page and set in one step");
    let rows = view(&s).rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].slots.len(), 1);
}

#[test]
fn deleting_a_laser_page_removes_its_fixtures_and_undo_brings_them_back() {
    let mut s = laser();
    s.add_fixture("holder").unwrap();
    let page = s.active_page().id;
    assert!(s.delete_page(&page));
    assert!(s.document.manufacturings().is_empty());
    s.undo();
    assert_eq!(s.document.manufacturings()[0].fixtures.len(), 1);
}

/// The Layers rows of the active laser page as JSON (`specs/0102` criterion 12).
fn layers_rows(s: &mut Session) -> Vec<serde_json::Value> {
    let view: serde_json::Value = serde_json::from_str(&s.layers_view_json(0, 60)).unwrap();
    view["rows"].as_array().unwrap().clone()
}

#[test]
fn criterion_12_the_layers_tree_lists_each_fixture_under_the_fixtures_row() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    let clamp = s.add_fixture("clamp").unwrap();
    assert!(s.set_fixture_active(clamp, false));
    let rows = layers_rows(&mut s);
    let kinds: Vec<&str> = rows.iter().map(|r| r["kind"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        ["page-root", "note", "fixtures", "fixture", "fixture"]
    );
    let (holder_row, clamp_row) = (&rows[3], &rows[4]);
    assert_eq!(holder_row["id"], serde_json::to_value(holder).unwrap());
    assert_eq!(holder_row["name"], "Holder 1");
    assert_eq!(holder_row["level"], 3, "one level under the Fixtures row");
    assert_eq!(holder_row["fixture"]["kind"], "Holder");
    assert_eq!(holder_row["fixture"]["active"], true);
    assert_eq!(holder_row["fixture"]["id"], holder.full_id());
    assert_eq!(holder_row["container"], false, "no chevron");
    assert_eq!(holder_row["label"], "Holder 1, Holder, on the bed");
    assert_eq!(clamp_row["fixture"]["active"], false);
    assert_eq!(clamp_row["label"], "Clamp 1, Clamp, not on the bed");
    assert!(
        clamp_row["fixture"]["id"].is_string() && rows[2]["fixture"].is_null(),
        "only fixture rows carry the fixture line"
    );
}

#[test]
fn criterion_12_selecting_a_fixture_row_selects_the_fixture_on_the_canvas() {
    let mut s = laser();
    let holder = s.add_fixture("holder").unwrap();
    s.selection.clear();
    assert!(s.select_fixture(holder));
    assert_eq!(s.selected_fixture(), Some(holder));
    assert_eq!(view(&s).selected, Some(holder.full_id()));
}
