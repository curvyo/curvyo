//! Session tests of the test grids (`specs/0028` criteria 1 to 14): add, fields, refusals, the
//! mark, hand edits and Rebuild, the Kind change and the canvas selection.

use curvyo_document_core::{Direction, NodeId, OperationSnapshot, Vec2};

use super::super::Session;
use super::super::text_tests;

/// A session with the included fonts loaded and a laser page active.
pub(in crate::session) fn laser_with_fonts() -> Session {
    let mut s = text_tests::session();
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    // The host hands in the faces the session asks for.
    let _ = s.grid_label_font();
    text_tests::load_wanted(&mut s);
    assert!(s.grid_label_font().is_some());
    s
}

fn ops(s: &Session) -> Vec<OperationSnapshot> {
    let manufacturing = s.active_manufacturing().unwrap();
    s.document.operations(manufacturing.id).unwrap()
}

fn undo_label(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}

/// Adds the default grid; its row and top group.
pub(in crate::session) fn add_grid(s: &mut Session) -> (NodeId, NodeId) {
    let row = s.press_add_test_grid().unwrap();
    let group = s.document.test_grid_of_row(row).unwrap();
    (row, group)
}

#[test]
fn criterion_3_add_test_grid_makes_a_selected_default_grid_in_one_step() {
    let mut s = laser_with_fonts();
    let steps = s.undo_len();
    let (row, group) = add_grid(&mut s);
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Add test grid");
    assert_eq!(
        s.selection.ids().len(),
        36,
        "the selection holds the grid's leaves"
    );
    assert_eq!(s.selected_test_grid(), Some(group));
    let op = ops(&s).into_iter().find(|op| op.id == row).unwrap();
    assert!(op.grid);
    assert_eq!(
        (op.name.as_str(), op.kind.as_deref()),
        ("Test grid 1", Some("cut"))
    );
    let grid = s.document.test_grid(group).unwrap();
    assert_eq!(grid.cells.len(), 25);
    assert_eq!(grid.labels.len(), 11, "unit line, five powers, five speeds");
    let view = s.test_grid_view().unwrap();
    assert_eq!(view.row, Some(row));
    assert_eq!(view.name, "Test grid 1");
    assert_eq!(view.machine, "Generic laser");
    assert_eq!(view.fields.columns, 5);
    assert_eq!(view.result.powers, [20.0, 40.0, 60.0, 80.0, 100.0]);
    assert!(!view.hand_edited);
    assert_eq!(view.worked, None);
    // The grid sits at the bed's top left.
    let page = s.active_page();
    let first = s.document.object(grid.cells[0].id).unwrap();
    let (min, _) = curvyo_ui_core::object_bounds(&first);
    assert!(min.x >= page.origin.x && min.y >= page.origin.y);
    assert!(min.x - page.origin.x < 30.0 && min.y - page.origin.y < 30.0);
}

#[test]
fn criterion_3_a_second_grid_takes_the_first_free_spot_and_the_next_name() {
    let mut s = laser_with_fonts();
    let (_, first) = add_grid(&mut s);
    let (row, second) = add_grid(&mut s);
    assert_ne!(first, second);
    let names: Vec<String> = ops(&s).into_iter().map(|op| op.name).collect();
    assert!(names.contains(&"Test grid 2".to_owned()));
    let a = s.document.test_grid(first).unwrap().cells[0].id;
    let b = s.document.test_grid(second).unwrap().cells[0].id;
    let (a_min, _) = curvyo_ui_core::object_bounds(&s.document.object(a).unwrap());
    let (b_min, _) = curvyo_ui_core::object_bounds(&s.document.object(b).unwrap());
    assert!(b_min.x > a_min.x + 1.0, "placed to the right of the first");
    assert_eq!(s.test_grid_view().unwrap().row, Some(row));
}

#[test]
fn criterion_2_add_refuses_while_the_label_font_loads_and_names_why() {
    let mut s = Session::new(300_028);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let steps = s.undo_len();
    assert_eq!(
        s.press_add_test_grid(),
        Err("The label font is still loading.".to_owned())
    );
    assert_eq!(s.undo_len(), steps);
    assert_eq!(
        s.test_grid_notice().as_deref(),
        Some("The label font is still loading.")
    );
}

#[test]
fn criterion_9_a_field_rebuilds_in_place_in_one_step() {
    let mut s = laser_with_fonts();
    let (_, group) = add_grid(&mut s);
    let steps = s.undo_len();
    assert_eq!(s.set_test_grid_field("columns", "6"), Ok(true));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Edit test grid");
    assert_eq!(s.document.test_grid(group).unwrap().cells.len(), 30);
    assert_eq!(s.test_grid_view().unwrap().fields.columns, 6);
    assert_eq!(s.set_test_grid_field("columns", "6"), Ok(false));
}

#[test]
fn criterion_2_an_invalid_value_is_kept_with_its_chip_and_nothing_changes() {
    let mut s = laser_with_fonts();
    let (_, group) = add_grid(&mut s);
    let steps = s.undo_len();
    assert_eq!(
        s.set_test_grid_field("columns", "13"),
        Err("Columns are 2 to 12.".to_owned())
    );
    assert_eq!(s.undo_len(), steps);
    let view = s.test_grid_view().unwrap();
    let invalid = view.invalid.unwrap();
    assert_eq!(
        (invalid.field.as_str(), invalid.typed.as_str()),
        ("columns", "13")
    );
    assert_eq!(view.fields.columns, 5);
    assert_eq!(s.document.test_grid(group).unwrap().cells.len(), 25);
    // A grid past the bed names the cause.
    let wide = s
        .set_test_grid_field("cell", "50")
        .and_then(|_| s.set_test_grid_field("columns", "12"));
    let cause = wide.unwrap_err();
    assert!(
        cause.starts_with("Grid is ") && cause.contains(" mm wide, the bed is "),
        "{cause}"
    );
    assert!(s.test_grid_view().unwrap().invalid.is_some());
}

#[test]
fn criteria_13_14_marking_a_cell_shows_the_worked_line_and_unmarks_on_a_second_press() {
    let mut s = laser_with_fonts();
    add_grid(&mut s);
    assert!(s.press_test_cell(2, 1));
    assert_eq!(undo_label(&mut s), "Mark cell");
    let worked = s.test_grid_view().unwrap().worked.unwrap();
    assert_eq!(worked.power_pct, 60.0);
    assert!(worked.text.starts_with("Worked: 60 % power, "));
    assert!(s.press_test_cell(3, 3));
    assert_eq!(s.test_grid_view().unwrap().worked.unwrap().cell.column, 3);
    assert!(s.press_test_cell(3, 3));
    assert_eq!(undo_label(&mut s), "Unmark cell");
    assert_eq!(s.test_grid_view().unwrap().worked, None);
    assert!(!s.press_test_cell(9, 0), "no such cell");
}

#[test]
fn criterion_9_an_edit_keeps_the_mark_of_a_cell_with_the_same_numbers() {
    let mut s = laser_with_fonts();
    add_grid(&mut s);
    assert!(s.press_test_cell(0, 0));
    assert_eq!(s.set_test_grid_field("gap", "4"), Ok(true));
    assert!(s.test_grid_view().unwrap().worked.is_some());
    assert_eq!(s.set_test_grid_field("power_from", "25"), Ok(true));
    assert_eq!(s.test_grid_view().unwrap().worked, None);
}

#[test]
fn criterion_10_a_moved_cell_is_a_hand_edit_and_rebuild_puts_it_back() {
    let mut s = laser_with_fonts();
    let (_, group) = add_grid(&mut s);
    let cell = s.document.test_grid(group).unwrap().cells[3].id;
    s.recording_unguarded(|this| {
        this.document
            .translate_objects(&[cell], Vec2::new(2.0, 0.0))
            .unwrap();
    });
    let view = s.test_grid_view().unwrap();
    assert!(view.hand_edited);
    // A field commit is stored, the items stay until Rebuild.
    assert_eq!(s.set_test_grid_field("rows", "4"), Ok(true));
    assert_eq!(s.document.test_grid(group).unwrap().cells.len(), 25);
    assert!(s.rebuild_test_grid());
    assert_eq!(undo_label(&mut s), "Rebuild test grid");
    assert_eq!(s.document.test_grid(group).unwrap().cells.len(), 20);
    assert!(!s.test_grid_view().unwrap().hand_edited);
}

#[test]
fn criterion_10_moving_the_whole_grid_is_no_hand_edit() {
    let mut s = laser_with_fonts();
    let (_, group) = add_grid(&mut s);
    let grid = s.document.test_grid(group).unwrap();
    let all: Vec<NodeId> = grid
        .cells
        .iter()
        .chain(&grid.labels)
        .map(|item| item.id)
        .collect();
    s.recording_unguarded(|this| {
        this.document
            .translate_objects(&all, Vec2::new(30.0, 20.0))
            .unwrap();
    });
    assert!(!s.test_grid_view().unwrap().hand_edited);
    // An edit rebuilds where the grid now is.
    let before = s.document.test_grid(group).unwrap().cells[0].id;
    let (at, _) = curvyo_ui_core::object_bounds(&s.document.object(before).unwrap());
    assert_eq!(s.set_test_grid_field("gap", "4"), Ok(true));
    let after = s.document.test_grid(group).unwrap().cells[0].id;
    let (now, _) = curvyo_ui_core::object_bounds(&s.document.object(after).unwrap());
    assert!((now.x - at.x).abs() < 1e-6 && (now.y - at.y).abs() < 1e-6);
}

#[test]
fn criterion_8_a_kind_change_rebuilds_clears_the_mark_and_moves_the_row() {
    let mut s = laser_with_fonts();
    s.add_test_operation("cut").unwrap();
    let (row, group) = add_grid(&mut s);
    assert!(s.press_test_cell(1, 1));
    assert_eq!(s.set_test_grid_kind("engrave"), Ok(true));
    assert_eq!(s.test_grid_view().unwrap().worked, None);
    let list = ops(&s);
    let op = list.iter().find(|op| op.id == row).unwrap();
    assert_eq!(op.kind.as_deref(), Some("engrave"));
    let cell = s.document.test_grid(group).unwrap().cells[0].id;
    assert!(
        s.document.object(cell).unwrap().style().fill.enabled,
        "an engraved cell is filled"
    );
}

#[test]
fn criterion_7_labels_off_makes_no_label_text() {
    let mut s = laser_with_fonts();
    let (_, group) = add_grid(&mut s);
    assert_eq!(s.set_test_grid_labels(false), Ok(true));
    assert_eq!(s.document.test_grid(group).unwrap().labels.len(), 0);
    assert_eq!(
        s.set_test_grid_label_kind("cut"),
        Err("Labels are engraved or scored.".to_owned())
    );
}

#[test]
fn criterion_12_a_selected_cell_square_highlights_its_table_cell() {
    let mut s = laser_with_fonts();
    let (row, group) = add_grid(&mut s);
    let cell = s.document.test_grid(group).unwrap().cells[7].id;
    s.selection.set(&[cell]);
    let view = s.test_grid_view().unwrap();
    assert_eq!(view.row, Some(row));
    let selected = view.selected_cell.unwrap();
    assert_eq!((selected.column, selected.row), (2, 1));
}

#[test]
fn ux_6_1_selecting_a_grid_on_the_canvas_shows_the_operations_tab() {
    let mut s = laser_with_fonts();
    let (_, group) = add_grid(&mut s);
    s.selection.set(&[]);
    let _ = s.panel_view();
    assert!(s.press_panel_tab("document"));
    s.selection.set(&[group]);
    let view = s.panel_view();
    let active: Vec<&str> = view
        .tabs
        .iter()
        .filter(|t| t.selected)
        .map(|t| t.name)
        .collect();
    assert_eq!(active, ["operations"]);
}

#[test]
fn criterion_2_the_last_grid_is_remembered_for_the_machine() {
    let mut s = laser_with_fonts();
    add_grid(&mut s);
    assert_eq!(s.set_test_grid_field("rows", "3"), Ok(true));
    let writes = s.take_material_writes_json();
    assert!(writes.contains("\"op\":\"grid_choice\""), "{writes}");
    let (_, group) = add_grid(&mut s);
    assert_eq!(s.document.test_grid(group).unwrap().cells.len(), 15);
}

#[test]
fn ux_7_3_the_worked_chip_is_drawn_in_both_views_and_cells_take_their_kind_look() {
    let mut s = laser_with_fonts();
    let (_, group) = add_grid(&mut s);
    let view = s.view();
    let overlay_len = |s: &Session| {
        let objects = s.objects();
        s.operation_looked_artwork(&objects, view)
            .map(|(_, overlay)| overlay.triangles.len())
            .unwrap()
    };
    let page = s.active_page().id;
    assert!(s.toggle_operations_view(&page));
    let unmarked = overlay_len(&s);
    assert!(s.press_test_cell(1, 1));
    assert!(overlay_len(&s) > unmarked, "the Worked chip in View: Drawn");
    assert!(s.toggle_operations_view(&page));
    let objects = s.objects();
    let (looked, _) = s.operation_looked_artwork(&objects, view).unwrap();
    let cell = s.document.test_grid(group).unwrap().cells[0].id;
    let drawn = looked.iter().find(|o| o.id() == cell).unwrap();
    let plain = s.document.object(cell).unwrap();
    assert_ne!(
        drawn.style(),
        plain.style(),
        "a cell is drawn in the Cut look"
    );
}

#[test]
fn ux_6_5_a_whole_grid_shows_the_centre_move_only_and_one_cell_keeps_its_handles() {
    let mut s = laser_with_fonts();
    let (_, group) = add_grid(&mut s);
    assert!(s.whole_test_grid_selected());
    let whole = s.select_transform_decoration_input();
    assert!(
        whole.handles.len() <= 1,
        "box and centre move only, got {} handles",
        whole.handles.len()
    );
    // S, R and K open nothing for a grid (a press of the keys is consumed).
    assert!(!s.select.has_entry());
    let cell = s.document.test_grid(group).unwrap().cells[0].id;
    s.selection.set(&[cell]);
    assert!(!s.whole_test_grid_selected());
    assert!(s.select_transform_decoration_input().handles.len() > 1);
}

/// A circle on the laser page away from the bed's top left, outside every group, deselected.
fn far_circle(s: &mut Session) -> NodeId {
    let origin = s.active_page().origin;
    let circle = s.recording_unguarded(|this| {
        this.document.create_primitive(
            curvyo_document_core::Shape::Ellipse {
                frame: curvyo_document_core::EllipseFrame {
                    center: curvyo_document_core::Point::new(origin.x + 160.0, origin.y + 160.0),
                    rx: curvyo_document_core::Length::from_mm(10.0),
                    ry: curvyo_document_core::Length::from_mm(10.0),
                },
                arc: None,
                curve: curvyo_document_core::Curve::ELLIPSE_DEFAULT,
            },
            curvyo_document_core::Angle::from_radians(0.0),
        )
    });
    s.selection.set(&[]);
    assert!(!s.in_test_grid(circle));
    circle
}

#[test]
fn criterion_6_a_kind_press_with_a_grid_selected_adds_nothing_and_names_the_skip() {
    // `specs/0111` criterion 8 supersedes the empty operation this press added.
    let mut s = laser_with_fonts();
    let (_, group) = add_grid(&mut s);
    assert_eq!(s.selected_test_grid(), Some(group));
    let steps = s.undo_len();
    let rows = ops(&s).len();
    assert_eq!(s.press_operation_kind("score", false), None);
    assert_eq!(s.undo_len(), steps);
    assert_eq!(ops(&s).len(), rows, "no operation is added");
    let notice = s.operations_notice().expect("the skip is named");
    assert!(notice.chars().count() <= 70, "{notice}");
}

#[test]
fn criterion_6_a_grid_item_is_not_assigned_by_hand_to_another_operation() {
    let mut s = laser_with_fonts();
    let (_, group) = add_grid(&mut s);
    let circle = far_circle(&mut s);
    let cut = s.add_test_operation("cut").unwrap();
    let cell = s.document.test_grid(group).unwrap().cells[0].id;
    let steps = s.undo_len();
    s.selection.set(&[cell]);
    assert!(!s.set_selection_in_operation(cut, true));
    assert!(!s.toggle_selection_by_number(1));
    assert_eq!(s.undo_len(), steps, "no step is written");
    let notice = s.operations_notice().expect("the refusal is named");
    assert!(notice.chars().count() <= 70, "{notice}");
    // In a mixed selection the other items are assigned and the grid item is named.
    s.selection.set(&[cell, circle]);
    assert!(s.set_selection_in_operation(cut, true));
    let members = ops(&s).into_iter().find(|o| o.id == cut).unwrap().members;
    assert_eq!(members, vec![vec![circle]]);
    assert_eq!(s.operations_notice(), Some(notice));
}

#[test]
fn criterion_9_after_a_field_edit_the_selection_holds_the_rebuilt_grid() {
    let mut s = laser_with_fonts();
    let (_, group) = add_grid(&mut s);
    assert_eq!(s.set_test_grid_field("columns", "6"), Ok(true));
    assert_eq!(s.selection.ids().len(), 42, "30 cells and 12 labels");
    assert!(
        s.selection
            .ids()
            .iter()
            .all(|&id| s.document.object(id).is_some())
    );
    assert!(s.whole_test_grid_selected());
    assert_eq!(s.selected_test_grid(), Some(group));
    // Rebuild and a Kind change follow the new leaves too.
    assert_eq!(s.set_test_grid_kind("score"), Ok(true));
    assert!(s.whole_test_grid_selected());
    // An unrelated selection is left alone by an edit through the pressed row.
    let circle = far_circle(&mut s);
    s.selection.set(&[circle]);
    let row = s.test_grid_view().and_then(|view| view.row);
    s.select_test_grid_row(row);
    assert_eq!(s.set_test_grid_field("rows", "4"), Ok(true));
    assert_eq!(s.selection.ids(), [circle]);
}

#[test]
fn criterion_6_a_kind_press_after_a_grid_edit_never_silently_does_nothing() {
    let mut s = laser_with_fonts();
    add_grid(&mut s);
    assert_eq!(s.set_test_grid_field("columns", "6"), Ok(true));
    let steps = s.undo_len();
    assert_eq!(s.press_operation_kind("cut", false), None);
    assert!(s.operations_notice().is_some(), "the refusal is named");
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn ux_5_number_keys_skip_grid_rows() {
    let mut s = laser_with_fonts();
    s.add_test_operation("cut").unwrap();
    add_grid(&mut s);
    s.selection.set(&[]);
    let second = s.add_test_operation("cut").unwrap();
    let circle = far_circle(&mut s);
    let shown: Vec<NodeId> = s
        .operations_view()
        .unwrap()
        .rows
        .iter()
        .map(|r| r.id)
        .collect();
    assert_eq!(shown.iter().position(|&id| id == second), Some(2));
    // Shown order: Cut 1, Test grid 1 (no digit), Cut 2: key 2 is Cut 2.
    s.selection.set(&[circle]);
    assert!(s.toggle_selection_by_number(2));
    let members = ops(&s)
        .into_iter()
        .find(|o| o.id == second)
        .unwrap()
        .members;
    assert_eq!(members, vec![vec![circle]]);
    assert!(
        !s.toggle_selection_by_number(3),
        "only two rows have a digit"
    );
}

#[test]
fn ux_6_5_the_select_bar_hides_flip_guide_object_to_path_and_linked_clone_for_a_grid() {
    let mut s = laser_with_fonts();
    let (_, group) = add_grid(&mut s);
    assert!(s.select_bar_hides_grid_commands());
    assert!(!s.linked_clone_shown());
    assert!(!s.select_bar_state().object_to_path);
    // One cell entered by a double-click is edited like any item.
    let cell = s.document.test_grid(group).unwrap().cells[0].id;
    s.selection.set(&[cell]);
    assert!(!s.select_bar_hides_grid_commands());
    assert!(s.linked_clone_shown());
}
