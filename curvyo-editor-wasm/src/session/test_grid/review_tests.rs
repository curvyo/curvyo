//! Session tests of the 0112 UX review fixes: the outline under the pointer and the box of a
//! placed grid while the tool stays active (criteria 2, 7), the grid kept selected after "New
//! material" from the bar (UX G9), the bed check at the grid's place (criterion 5), the panel
//! staying off Style on a placement and the judge table's axis units.

use curvyo_document_core::Point;
use curvyo_ui_core::PanelTab;

use super::super::Tool;
use super::super::materials::tests::RANDOM;
use super::tests::{laser_with_fonts, place_grid, recipe_of};

#[test]
fn the_tool_draws_the_grids_outline_under_the_pointer_in_the_preview_colour() {
    let mut s = laser_with_fonts();
    s.set_tool(Tool::TestGrid);
    let origin = s.active_page().origin;
    let at = Point::new(origin.x + 20.0, origin.y + 20.0);
    assert_eq!(s.test_grid_draw_list(s.view()).triangle_count(), 0);
    s.pointer_hover(at, false, false);
    let list = s.test_grid_draw_list(s.view());
    assert_ne!(list.triangle_count(), 0, "the outline at the pointer");
    // The context outline's look: `--preview-new` (the accent) over the white casing.
    let look = curvyo_render_core::build_context_outline(
        s.view(),
        [
            at,
            Point::new(at.x + 10.0, at.y),
            Point::new(at.x + 10.0, at.y + 10.0),
            Point::new(at.x, at.y + 10.0),
        ],
        1.0,
    );
    assert_ne!(look.triangle_count(), 0);
    assert!(
        list.triangles
            .iter()
            .all(|v| look.triangles.iter().any(|w| w.color == v.color)),
        "no new colour"
    );
    s.pointer_leave();
    assert_eq!(s.test_grid_draw_list(s.view()).triangle_count(), 0);
    // Over a placed grid a press selects it: no outline there.
    let group = s.test_grid_press(Some(at)).unwrap();
    let cell = s.grid_object(group).unwrap().cells[0].id;
    let (lo, hi) = curvyo_ui_core::object_bounds(&s.document.object(cell).unwrap());
    s.pointer_hover(
        Point::new(f64::midpoint(lo.x, hi.x), f64::midpoint(lo.y, hi.y)),
        false,
        false,
    );
    assert_eq!(s.test_grid_draw_list(s.view()).triangle_count(), 0);
}

#[test]
fn criterion_7_a_just_placed_grid_shows_its_box_while_the_tool_stays_active() {
    let mut s = laser_with_fonts();
    s.set_tool(Tool::TestGrid);
    let origin = s.active_page().origin;
    s.test_grid_press(Some(Point::new(origin.x + 20.0, origin.y + 20.0)))
        .unwrap();
    assert_eq!(s.tool(), Tool::TestGrid);
    let objects = s.objects();
    assert!(
        s.group_decoration_input_in(&objects).is_some(),
        "the grid's dashed box"
    );
}

#[test]
fn ux_g9_new_material_from_the_bar_keeps_the_grid_selected_and_the_bar_on_it() {
    let mut s = laser_with_fonts();
    let group = place_grid(&mut s);
    s.set_tool(Tool::TestGrid);
    s.select_grid(group);
    s.open_material_form(Some(group));
    let id = s.add_material("Birch 3 mm", 3.0, &RANDOM).unwrap();
    assert_eq!(s.whole_selected_grid(), Some(group), "still selected");
    let view = s.test_grid_view().unwrap();
    assert_eq!(
        view.group,
        Some(group),
        "the bar on the grid, not the draft"
    );
    assert_eq!(recipe_of(&s, group).material.unwrap().id, id);
}

#[test]
fn criterion_5_a_grid_that_would_run_past_the_bed_edge_from_its_place_is_refused() {
    let mut s = laser_with_fonts();
    let bed = s.active_manufacturing().unwrap().work_area();
    let origin = s.active_page().origin;
    s.set_tool(Tool::TestGrid);
    let at = Point::new(
        origin.x + bed.width.as_mm() / 2.0,
        origin.y + bed.height.as_mm() / 2.0,
    );
    let group = s.test_grid_press(Some(at)).unwrap();
    let mut refused = None;
    for cell in 11..=50 {
        let before = recipe_of(&s, group);
        match s.set_test_grid_field("cell", &cell.to_string()) {
            Ok(_) => {
                let (lo, hi) = s.grid_box(group).unwrap();
                assert!(lo.x >= origin.x - 0.01 && lo.y >= origin.y - 0.01);
                assert!(
                    hi.x <= origin.x + bed.width.as_mm() + 0.01
                        && hi.y <= origin.y + bed.height.as_mm() + 0.01,
                    "cell {cell}: the grid stays on the bed"
                );
            }
            Err(cause) => {
                assert_eq!(recipe_of(&s, group), before, "the grid stays as it was");
                refused = Some(cause);
                break;
            }
        }
    }
    let cause = refused.expect("a grid at the bed's centre runs past its edge");
    assert!(
        cause.starts_with("Grid runs ") && cause.contains("past the bed"),
        "{cause}"
    );
    assert!(cause.chars().count() <= 70);
    let view = s.test_grid_view().unwrap();
    assert_eq!(view.invalid.unwrap().cause, cause, "kept and marked");
}

#[test]
fn placing_a_grid_keeps_the_panel_off_style() {
    let mut s = laser_with_fonts();
    assert!(s.press_panel_tab("operations"));
    let _ = s.panel_view();
    s.set_tool(Tool::TestGrid);
    let origin = s.active_page().origin;
    s.test_grid_press(Some(Point::new(origin.x + 20.0, origin.y + 20.0)))
        .unwrap();
    let _ = s.panel_view();
    assert_eq!(s.panel_tabs.get().active(), PanelTab::Operations);
    // From Document too: the grid's rows are what the maker looks at next.
    s.selection.clear();
    assert!(s.press_panel_tab("document"));
    let _ = s.panel_view();
    s.test_grid_press(Some(Point::new(origin.x + 150.0, origin.y + 150.0)))
        .unwrap();
    let _ = s.panel_view();
    assert_eq!(s.panel_tabs.get().active(), PanelTab::Operations);
}

#[test]
fn the_judge_table_axes_carry_their_units() {
    let mut s = laser_with_fonts();
    let birch = s.add_material("Birch 3 mm", 3.0, &RANDOM).unwrap();
    let group = place_grid(&mut s);
    assert_eq!(s.pick_test_grid_material(Some(&birch)), Ok(true));
    let id = recipe_of(&s, group).id.to_string();
    s.unfold_material(Some(&birch));
    s.open_material_test(Some(&id));
    let detail = s.materials_view().unwrap().unfolded.unwrap().test.unwrap();
    assert_eq!(detail.columns.unit, " %");
    assert_eq!(detail.rows.unit, " mm/min");
}
