//! Session tests of the Test grid tool (`specs/0112` criteria 1 to 14, 22, 28): the press and its
//! step, the bar's commits and kept values, regeneration over hand edits, new ids after Lasered,
//! copies, removal off the page, the test record that follows the grid, Place on page.

use curvyo_document_core::{Direction, NodeId, OperationSnapshot, Point, Vec2};
use curvyo_laser_core::grids::{Recipe, TestId};

use super::super::Session;
use super::super::text_tests;

/// A session with the included fonts loaded and a laser page active.
pub(in crate::session) fn laser_with_fonts() -> Session {
    let mut s = text_tests::session();
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let _ = s.grid_label_font();
    text_tests::load_wanted(&mut s);
    assert!(s.grid_label_font().is_some());
    s
}

pub(in crate::session) fn ops(s: &Session) -> Vec<OperationSnapshot> {
    let manufacturing = s.active_manufacturing().unwrap();
    s.document.operations(manufacturing.id).unwrap()
}

pub(in crate::session) fn undo_label(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}

/// Places a grid with the bar's values at the bed's top left; its group.
pub(in crate::session) fn place_grid(s: &mut Session) -> NodeId {
    let origin = s.active_page().origin;
    s.set_tool(super::super::Tool::TestGrid);
    let group = s
        .test_grid_press(Some(Point::new(origin.x + 1.0, origin.y + 1.0)))
        .unwrap();
    s.set_tool(super::super::Tool::Select);
    s.select_grid(group);
    group
}

pub(in crate::session) fn recipe_of(s: &Session, group: NodeId) -> Recipe {
    s.grid_object(group).unwrap().recipe
}

fn derived(s: &Session) -> Vec<OperationSnapshot> {
    ops(s)
        .into_iter()
        .filter(|op| op.grid_part.is_some())
        .collect()
}

#[test]
fn criterion_1_the_tool_is_offered_on_laser_pages_only() {
    let mut s = text_tests::session();
    assert!(!s.test_grid_tool_available());
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    assert!(s.test_grid_tool_available());
}

#[test]
fn criterion_2_and_12_a_press_places_a_selected_grid_with_its_26_rows_in_one_step() {
    let mut s = laser_with_fonts();
    let steps = s.undo_len();
    let origin = s.active_page().origin;
    s.set_tool(super::super::Tool::TestGrid);
    let at = Point::new(origin.x + 40.0, origin.y + 30.0);
    let group = s.test_grid_press(Some(at)).unwrap();
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Add test grid");
    assert_eq!(s.selected_test_grid(), Some(group));
    let grid = s.grid_object(group).unwrap();
    assert_eq!(grid.cells.len(), 25);
    assert_eq!(grid.labels.len(), 3 + 5 + 5, "id, two titles, the values");
    let id = grid.recipe.id.to_string();
    let rows = derived(&s);
    assert_eq!(rows.len(), 26);
    assert_eq!(rows[0].name, format!("{id} labels"));
    assert!(
        rows.iter()
            .any(|op| op.name == format!("20 % · 600 · {id}"))
    );
    assert!(grid.recipe.material.is_none(), "no material, no test yet");
    // The grid's top-left lies at the press.
    let (min, _) = curvyo_ui_core::object_bounds(&s.document.object(grid.labels[0].id).unwrap());
    assert!((min.x - at.x).abs() < 5.0 && (min.y - at.y).abs() < 5.0);
}

#[test]
fn criterion_2_a_press_where_the_grid_does_not_fit_takes_the_first_free_spot() {
    let mut s = laser_with_fonts();
    let origin = s.active_page().origin;
    let size = s.active_manufacturing().unwrap().work_area();
    s.set_tool(super::super::Tool::TestGrid);
    let far = Point::new(
        origin.x + size.width.as_mm() - 1.0,
        origin.y + size.height.as_mm() - 1.0,
    );
    let group = s.test_grid_press(Some(far)).unwrap();
    let grid = s.grid_object(group).unwrap();
    let (min, _) = curvyo_ui_core::object_bounds(&s.document.object(grid.cells[0].id).unwrap());
    assert!(min.x - origin.x < 40.0 && min.y - origin.y < 40.0);
    // A press on the grid selects it and places nothing.
    let steps = s.undo_len();
    s.selection.clear();
    let (cell_min, cell_max) =
        curvyo_ui_core::object_bounds(&s.document.object(grid.cells[3].id).unwrap());
    let inside = Point::new(
        f64::midpoint(cell_min.x, cell_max.x),
        f64::midpoint(cell_min.y, cell_max.y),
    );
    assert_eq!(s.test_grid_press(Some(inside)), Ok(group));
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn criterion_3_5_and_14_a_bar_commit_regenerates_and_a_bad_value_is_kept() {
    let mut s = laser_with_fonts();
    let group = place_grid(&mut s);
    let steps = s.undo_len();
    assert_eq!(s.set_test_grid_field("columns_count", "3"), Ok(true));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Edit test grid");
    assert_eq!(derived(&s).len(), 1 + 3 * 5);
    assert_eq!(s.selected_test_grid(), Some(group));
    // Out of range: kept with its cause, nothing written.
    let err = s.set_test_grid_field("cell", "60").unwrap_err();
    assert_eq!(err, "Cell size is 3 to 50 mm.");
    assert_eq!(s.undo_len(), steps + 1);
    let view = s.test_grid_view().unwrap();
    assert_eq!(view.invalid.as_ref().unwrap().typed, "60");
    assert_eq!(view.cell_mm, 10.0, "never clamped");
    // Wider than the bed: the bed cause.
    assert_eq!(s.set_test_grid_field("cell", "50"), Ok(true));
    let err = s.set_test_grid_field("columns_count", "12").unwrap_err();
    assert!(
        err.starts_with("Grid is ") && err.contains("wide, the bed is"),
        "{err}"
    );
    assert_eq!(
        recipe_of(&s, group)
            .test
            .axis(curvyo_laser_core::grids::Side::Columns)
            .count(),
        3
    );
}

#[test]
fn criterion_4_choosing_the_other_axis_parameter_swaps_the_axes() {
    let mut s = laser_with_fonts();
    let group = place_grid(&mut s);
    assert_eq!(s.set_test_grid_axis("rows", "power"), Ok(true));
    let view = s.test_grid_view().unwrap();
    assert_eq!((view.columns.param, view.rows.param), ("speed", "power"));
    assert_eq!(view.shared[0].param, "passes");
    assert_eq!(s.set_test_grid_axis("columns", "passes"), Ok(true));
    let view = s.test_grid_view().unwrap();
    assert_eq!(view.columns.param, "passes");
    assert_eq!(
        view.shared[0].param, "speed",
        "the freed parameter is shared"
    );
    assert_eq!(s.set_test_grid_kind("engrave"), Ok(true));
    let view = s.test_grid_view().unwrap();
    assert!(view.columns.offered.contains(&"interval"));
    assert_eq!(view.crosshatch, Some(false));
    assert_eq!(s.set_test_grid_crosshatch(true), Ok(true));
    let cell = derived(&s)
        .into_iter()
        .find(|op| op.kind.as_deref() == Some("engrave") && !op.name.ends_with("labels"))
        .unwrap();
    assert_eq!(cell.options.unwrap()["crosshatch"], true);
    let _ = group;
}

#[test]
fn criterion_14_regeneration_overwrites_a_hand_edit_and_undo_brings_it_back() {
    let mut s = laser_with_fonts();
    let group = place_grid(&mut s);
    let cell = derived(&s)[1].id;
    s.recording_unguarded(|this| this.document.rename_operation(cell, "Mine").unwrap());
    assert_eq!(s.set_test_grid_field("rows_count", "4"), Ok(true));
    let after = derived(&s);
    assert!(after.iter().all(|op| op.name != "Mine"));
    assert!(after.iter().any(|op| op.id == cell), "node id kept");
    let _ = s.undo();
    assert!(derived(&s).iter().any(|op| op.name == "Mine"));
    let _ = group;
}

#[test]
fn criterion_11_a_lasered_grid_gets_a_new_id_at_its_next_change_and_keeps_it_otherwise() {
    let mut s = laser_with_fonts();
    let group = place_grid(&mut s);
    let first = recipe_of(&s, group).id;
    assert_eq!(s.set_test_grid_field("passes", "2"), Ok(true));
    assert_eq!(recipe_of(&s, group).id, first, "not Lasered: same id");
    s.mark_test_grid_lasered_for_tests(group, 1_762_948_800_000)
        .unwrap();
    assert!(s.grid_object(group).unwrap().lasered.is_some());
    // A rename does not count.
    let row = derived(&s)[2].id;
    s.recording_unguarded(|this| this.document.rename_operation(row, "x").unwrap());
    assert_eq!(recipe_of(&s, group).id, first);
    assert_eq!(s.set_test_grid_field("passes", "3"), Ok(true));
    let second = recipe_of(&s, group).id;
    assert_ne!(second, first);
    assert!(s.grid_object(group).unwrap().lasered.is_none());
    assert_eq!(
        s.take_test_grid_status(),
        Some(format!("New test id {second}."))
    );
    assert!(derived(&s)[0].name.starts_with(second.as_str()));
    // A hand edit of a row's settings of a Lasered grid.
    s.mark_test_grid_lasered_for_tests(group, 1_762_948_900_000)
        .unwrap();
    let cell = derived(&s)[3].id;
    assert_eq!(
        s.edit_operation_setting(cell, "main", "power", 55.0),
        Ok(true)
    );
    let third = recipe_of(&s, group).id;
    assert_ne!(third, second);
}

#[test]
fn criterion_8_and_13_a_copy_has_its_own_id_and_rows_a_delete_takes_them() {
    let mut s = laser_with_fonts();
    let group = place_grid(&mut s);
    let id = recipe_of(&s, group).id;
    let units = s.selected_units();
    let reply = s.duplicate_layer_target();
    let copy = s
        .selected_test_grid()
        .unwrap_or_else(|| panic!("{reply:?} {units:?} {}", s.selection.ids().len()));
    assert_ne!(copy, group);
    let copy_id = recipe_of(&s, copy).id;
    assert_ne!(copy_id, id);
    assert_eq!(derived(&s).len(), 52);
    assert_eq!(s.grid_object(copy).unwrap().derived.len(), 26);
    s.delete_selected();
    assert_eq!(derived(&s).len(), 26);
    let _ = s.undo();
    assert_eq!(derived(&s).len(), 52);
}

#[test]
fn criterion_13_a_grid_moved_off_the_laser_page_loses_its_rows_in_the_same_step() {
    let mut s = laser_with_fonts();
    let group = place_grid(&mut s);
    let page = s.active_page();
    let steps = s.undo_len();
    let leaves = s.document.leaves_of(group);
    let far = Vec2::new(page.size.width.as_mm() * 4.0 + 500.0, 0.0);
    s.recording_unguarded(|this| this.document.translate_objects(&leaves, far).unwrap());
    assert_eq!(s.undo_len(), steps + 1, "one step");
    assert_eq!(derived(&s).len(), 0);
    let _ = s.undo();
    assert_eq!(derived(&s).len(), 26);
}

#[test]
fn criterion_9_a_new_id_avoids_retired_and_project_ids() {
    let mut s = laser_with_fonts();
    let group = place_grid(&mut s);
    let taken = recipe_of(&s, group).id;
    // The same random bytes again would draw the same id: it is taken by the project.
    let next = s.next_test_id();
    assert_ne!(next, taken);
    s.load_retired_test_ids(Some(&format!(
        "{{\"format_version\": 1, \"retired\": [\"{next}\"]}}"
    )));
    assert!(s.materials.id_taken(next));
    let fresh = s.next_test_id();
    assert_ne!(fresh, next);
    assert!(TestId::parse(fresh.as_str()).is_some());
}
