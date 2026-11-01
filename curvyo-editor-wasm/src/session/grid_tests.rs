//! The Grid panel, the rail toggle, the drawn grid and grid snapping in the session
//! (`specs/0090-grid` criteria 2 to 4, 10 to 12 and 15).

use curvyo_document_core::{DocumentSnapping, GridMode, GridSnapPoint, Point, RectBounds};

use super::{Session, Tool};

fn grid_session() -> Session {
    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    session
}

#[test]
fn a_new_document_shows_the_default_grid_record() {
    let session = grid_session();
    let record = session.active_page_grid();
    assert_eq!(record.page_name, "Page 1");
    assert!(!record.shown);
    assert_eq!(record.mode, "adaptive");
    assert_eq!(record.scale, "linear");
    assert_eq!(record.spacing, "5");
    assert_eq!(record.subdivision, "5");
    assert_eq!(record.angle, "0");
    assert_eq!(record.origin, "top_left");
    assert_eq!(record.unit, "mm");
}

#[test]
fn shift_click_toggles_the_active_page_grid_as_one_undoable_step() {
    let mut session = grid_session();
    let before = session.undo_len();
    assert!(session.toggle_active_page_grid());
    assert!(session.active_page().grid.shown);
    assert_eq!(session.undo_len(), before + 1);
    let _ = session.undo();
    assert!(!session.active_page().grid.shown);
    assert!(!session.active_page_grid().shown, "the record follows undo");
}

#[test]
fn the_panel_acts_on_the_active_page_only() {
    let mut session = grid_session();
    let first = session.active_page().id;
    let second = session.add_page();
    assert_eq!(session.active_page().id, second);
    assert_eq!(
        session.set_active_page_grid_field("spacing", "10"),
        "committed"
    );
    assert!(session.set_active_page_grid_mode("fixed"));
    assert_eq!(session.active_page().grid.mode, GridMode::Fixed);
    assert_eq!(session.document.page_grid(&first).spacing.as_mm(), 5.0);
    assert!(session.activate_page(&first));
    assert_eq!(session.active_page_grid().spacing, "5");
}

#[test]
fn refused_values_write_nothing_and_name_their_notice() {
    let mut session = grid_session();
    let version = session.document.version();
    for (field, text, key) in [
        ("spacing", "abc", "invalid:spacing_number"),
        ("spacing", "0", "invalid:spacing_positive"),
        ("spacing", "-2", "invalid:spacing_positive"),
        ("subdivision", "1", "invalid:subdivision"),
        ("subdivision", "2.5", "invalid:subdivision"),
        ("subdivision", "x", "invalid:subdivision"),
        ("angle", "north", "invalid:number"),
        ("angle", "", "invalid:number"),
    ] {
        assert_eq!(
            session.set_active_page_grid_field(field, text),
            key,
            "{field} {text}"
        );
    }
    assert_eq!(session.document.version(), version);
    assert_eq!(
        session.set_active_page_grid_field("spacing", "5"),
        "unchanged"
    );
}

#[test]
fn the_angle_is_taken_modulo_360_and_shown_in_plus_minus_180() {
    let mut session = grid_session();
    assert_eq!(
        session.set_active_page_grid_field("angle", "270"),
        "committed"
    );
    assert_eq!(session.active_page_grid().angle, "-90");
    assert_eq!(
        session.set_active_page_grid_field("angle", "360"),
        "committed"
    );
    assert_eq!(session.active_page_grid().angle, "0");
    assert_eq!(
        session.set_active_page_grid_field("angle", "12.5"),
        "committed"
    );
    assert_eq!(session.active_page_grid().angle, "12.5");
}

#[test]
fn the_origin_corner_is_one_undoable_step_and_works_with_the_grid_hidden() {
    let mut session = grid_session();
    let before = session.undo_len();
    assert!(session.set_active_page_origin("bottom_left"));
    assert_eq!(session.undo_len(), before + 1);
    assert_eq!(session.active_page_grid().origin, "bottom_left");
    assert!(!session.active_page().grid.shown);
    assert!(!session.set_active_page_origin("bottom_left"), "unchanged");
    assert!(!session.set_active_page_origin("centre"), "unknown corner");
    assert_eq!(session.undo_len(), before + 1);
    let _ = session.undo();
    assert_eq!(session.active_page_grid().origin, "top_left");
}

#[test]
fn a_shown_grid_is_drawn_between_the_page_area_and_the_artwork() {
    let mut session = grid_session();
    let hidden = session.frame_draw_list();
    assert!(session.set_active_page_grid_shown(true));
    let shown = session.frame_draw_list();
    let grid_vertices = shown.triangles.len() - hidden.triangles.len();
    assert!(grid_vertices > 0);
    assert_eq!(shown.layers().len(), hidden.layers().len() + 1);
    // The page area is layer 0; the grid is layer 1, right after it.
    assert_eq!(shown.layers()[1] - shown.layers()[0], grid_vertices);
    // The plain draw list (export, print, eyedropper paths) never carries the grid.
    assert_eq!(
        session.draw_list().triangles.len(),
        hidden.triangles.len() - 6
    );
}

#[test]
fn snapping_settings_carry_the_grid_target_and_snap_point() {
    let mut session = grid_session();
    let record = session.snapping_settings();
    assert!(record.grid);
    assert_eq!(record.grid_point, "centre");
    assert!(session.set_snapping_grid(false));
    assert!(!session.snapping_settings().grid);
    assert!(session.set_snapping_grid_point("nodes"));
    assert_eq!(session.document.snapping().grid_point, GridSnapPoint::Nodes);
    assert!(!session.set_snapping_grid_point("corner"));
}

#[test]
fn a_moved_selection_snaps_its_centre_to_a_shown_grid() {
    let mut session = grid_session();
    let id = session.document.create_rect(RectBounds::from_corners(
        Point::new(12.0, 12.0),
        Point::new(32.0, 32.0),
    ));
    session.set_snapping(DocumentSnapping {
        enabled: true,
        objects: false,
        guides: false,
        nodes: false,
        page: false,
        ..DocumentSnapping::DEFAULT
    });
    assert!(session.set_active_page_grid_shown(true));
    session.set_tool(Tool::Select);
    session.selection.set(&[id]);
    // Centre 22, 22 moves by 12.6, 13.3 to 34.6, 35.3: the 5 mm grid pulls it to 35, 35.
    session.pointer_down(Point::new(22.0, 22.0), false);
    session.pointer_hover(Point::new(34.6, 35.3), false, false);
    session.pointer_up(Point::new(34.6, 35.3), false, false);
    let (min, max) = curvyo_ui_core::object_bounds(&session.document.object(id).unwrap());
    let centre = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
    assert!(
        (centre.x - 35.0).abs() < 1e-9 && (centre.y - 35.0).abs() < 1e-9,
        "{centre:?}"
    );
}

#[test]
fn the_box_choice_snaps_the_nearest_reference_point() {
    let mut session = grid_session();
    let id = session.document.create_rect(RectBounds::from_corners(
        Point::new(12.0, 12.0),
        Point::new(33.4, 33.4),
    ));
    session.set_snapping(DocumentSnapping {
        enabled: true,
        objects: false,
        guides: false,
        nodes: false,
        page: false,
        grid_point: GridSnapPoint::Box,
        ..DocumentSnapping::DEFAULT
    });
    assert!(session.set_active_page_grid_shown(true));
    session.set_tool(Tool::Select);
    session.selection.set(&[id]);
    // The top-left corner moves from 12, 12 to 19.9, 19.85, nearer the grid than the centre and
    // the far corner: pulled to 20, 20.
    session.pointer_down(Point::new(22.0, 22.0), false);
    session.pointer_hover(Point::new(29.9, 29.85), false, false);
    session.pointer_up(Point::new(29.9, 29.85), false, false);
    let (min, _) = curvyo_ui_core::object_bounds(&session.document.object(id).unwrap());
    assert!(
        (min.x - 20.0).abs() < 1e-9 && (min.y - 20.0).abs() < 1e-9,
        "{min:?}"
    );
}

#[test]
fn a_hidden_grid_or_the_switch_off_does_not_pull() {
    let mut session = grid_session();
    let id = session.document.create_rect(RectBounds::from_corners(
        Point::new(12.0, 12.0),
        Point::new(32.0, 32.0),
    ));
    session.set_snapping(DocumentSnapping {
        enabled: true,
        objects: false,
        guides: false,
        nodes: false,
        page: false,
        ..DocumentSnapping::DEFAULT
    });
    session.set_tool(Tool::Select);
    session.selection.set(&[id]);
    session.pointer_down(Point::new(22.0, 22.0), false);
    session.pointer_hover(Point::new(34.6, 35.3), false, false);
    session.pointer_up(Point::new(34.6, 35.3), false, false);
    let (min, _) = curvyo_ui_core::object_bounds(&session.document.object(id).unwrap());
    assert!((min.x - 24.6).abs() < 1e-9, "{min:?}");
}
