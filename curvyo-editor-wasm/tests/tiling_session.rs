//! The Tile mode of the session (`specs/0069-tiling` criteria 1, 2, 14, 26, 27, 32, 34, 35, 37
//! and 39): entering writes nothing, Escape leaves the document unchanged, Apply is one step, a
//! reopen shows the stored recipe and an unchanged Apply writes nothing, undo is locked while the
//! bar is open, a tool or page switch cancels, the circle handles move and snap the centre, the
//! radius, the start and the sweep with a readout, their entry boxes keep or restore the values,
//! and Align and Distribute move a tiling as one object.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    AnchorId, Document, DocumentSnapping, GridSnapPoint, NewAnchor, NodeId, PageGrid, Point,
    RectBounds, TilingLayout, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::tiling::{CircleHandle, TileButton};
use curvyo_ui_core::{AlignAxis, AlignCommand, AlignOutcome, DistributeMode, object_bounds};

fn session_with_one_path() -> Session {
    session_of(&document_with_one_path())
}

fn document_with_one_path() -> Document {
    let document = Document::new(1);
    let _ = document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(30.0, 10.0)),
            NewAnchor::corner(AnchorId::new(1, 3), Point::new(30.0, 20.0)),
        ],
        true,
    );
    document
}

fn session_of(document: &Document) -> Session {
    let mut session = Session::open(2, &pack(document, "0.1.0").unwrap()).unwrap();
    session.set_tool(Tool::Select);
    session.resize_viewport(1600.0, 900.0);
    let _ = session.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    session
}

fn last_row(session: &mut Session) -> String {
    // Newest first.
    session.history_rows_json(0, 1)
}

#[test]
fn entering_and_escaping_write_nothing() {
    let mut session = session_with_one_path();
    assert!(matches!(session.tile_button(), TileButton::Tile(_)));
    let steps = session.history_len();
    assert!(session.tiling_enter(42));
    assert!(session.tiling_open());
    let _ = session.draw_list();
    assert!(!session.is_dirty());
    session.tiling_cancel();
    assert!(!session.tiling_open());
    assert!(!session.is_dirty());
    assert_eq!(session.history_len(), steps);
}

#[test]
fn apply_is_one_step_and_selects_the_tiling() {
    let mut session = session_with_one_path();
    let steps = session.history_len();
    assert!(session.tiling_enter(7));
    assert!(session.tiling_apply());
    assert!(!session.tiling_open());
    assert_eq!(session.history_len(), steps + 1);
    assert!(last_row(&mut session).contains("Tile"));
    assert!(matches!(session.tile_button(), TileButton::Edit(_)));
    assert_eq!(session.selected_object_count(), 20);
}

#[test]
fn a_reopen_shows_the_stored_recipe_and_an_unchanged_apply_writes_nothing() {
    let mut session = session_with_one_path();
    session.tiling_enter(7);
    let applied = session.tiling_recipe().unwrap();
    session.tiling_apply();
    let steps = session.history_len();
    assert!(session.tiling_enter(99));
    assert_eq!(
        session.tiling_recipe().unwrap(),
        applied,
        "the stored seed, not the new one"
    );
    assert!(session.tiling_status().unwrap().reopened);
    assert!(session.tiling_apply());
    assert_eq!(session.history_len(), steps);
    session.tiling_enter(1);
    let mut wider = applied;
    wider.grid.columns = 6;
    assert!(session.tiling_set_recipe(wider));
    session.tiling_apply();
    assert_eq!(session.history_len(), steps + 1);
    assert!(last_row(&mut session).contains("Edit tiling"));
}

#[test]
fn undo_is_locked_and_a_tool_switch_cancels() {
    let mut session = session_with_one_path();
    session.tiling_enter(3);
    session.tiling_apply();
    session.tiling_enter(3);
    let undo = session.undo();
    assert!(!undo.changed);
    assert!(session.tiling_open());
    session.set_tool(Tool::Node);
    assert!(!session.tiling_open());
}

#[test]
fn release_leaves_a_plain_group_that_can_be_tiled_again() {
    let mut session = session_with_one_path();
    session.tiling_enter(3);
    session.tiling_apply();
    let steps = session.history_len();
    assert!(session.tiling_release());
    assert_eq!(session.history_len(), steps + 1);
    assert!(last_row(&mut session).contains("Release tiling"));
    assert!(matches!(session.tile_button(), TileButton::Tile(_)));
}

#[test]
fn the_radius_handle_follows_a_drag() {
    let mut session = session_with_one_path();
    session.tiling_enter(3);
    let mut recipe = session.tiling_recipe().unwrap();
    recipe.layout = TilingLayout::Circle;
    assert!(session.tiling_set_recipe(recipe));
    // Box centre (20, 15), radius 20, centre (0, 15), a full turn from 0: the radius handle
    // sits at the middle of the arc, opposite the start.
    let handle = Point::new(-20.0, 15.0);
    session.pointer_down(handle, false);
    session.pointer_hover(Point::new(-30.0, 15.0), false, false);
    session.pointer_up(Point::new(-30.0, 15.0), false, false);
    let radius = session.tiling_recipe().unwrap().circle.radius.as_mm();
    assert!((radius - 30.0).abs() < 1e-9, "radius {radius}");
    assert!(session.tiling_open(), "a press never leaves the bar");
}

#[test]
fn the_json_of_the_bar_round_trips_through_the_session() {
    let mut session = session_with_one_path();
    session.tiling_enter(5);
    let json = session.tiling_recipe_json();
    let edited = json.replace("\"columns\":5", "\"columns\":3");
    assert!(session.tiling_set_recipe_json(&edited));
    assert_eq!(session.tiling_recipe().unwrap().grid.columns, 3);
    assert!(!session.tiling_set_recipe_json(&json.replace("\"columns\":5", "\"columns\":0")));
    assert!(session.tiling_status_json().contains("\"count\":12"));
}

/// The bar open on the one path with the Circle layout: box centre (20, 15), radius 20, centre
/// (0, 15), start 0 and a full turn: the start knob at (20, 15), the radius handle at (-20, 15).
fn circle_bar(session: &mut Session) {
    session.tiling_enter(3);
    let mut recipe = session.tiling_recipe().unwrap();
    recipe.layout = TilingLayout::Circle;
    assert!(session.tiling_set_recipe(recipe));
}

fn circle_centre(session: &Session) -> Point {
    let recipe = session.tiling_recipe().unwrap();
    session
        .tiling_box_centre()
        .unwrap()
        .translated(recipe.circle.centre)
}

fn with_grid_snapping() -> Session {
    let document = document_with_one_path();
    document.set_snapping(DocumentSnapping {
        enabled: true,
        objects: false,
        guides: false,
        nodes: false,
        page: false,
        grid: true,
        grid_point: GridSnapPoint::Centre,
        ..DocumentSnapping::DEFAULT
    });
    let page = document.pages()[0].id.clone();
    document
        .set_page_grid(
            &page,
            PageGrid {
                shown: true,
                ..PageGrid::DEFAULT
            },
        )
        .unwrap();
    session_of(&document)
}

#[test]
fn both_circle_handles_snap_to_the_grid_and_the_modifier_suspends() {
    let mut session = with_grid_snapping();
    circle_bar(&mut session);
    session.pointer_down(Point::new(0.0, 15.0), false);
    session.pointer_hover(Point::new(10.6, 20.4), false, false);
    session.pointer_up(Point::new(10.6, 20.4), false, false);
    let centre = circle_centre(&session);
    assert!(
        (centre.x - 10.0).abs() < 1e-9 && (centre.y - 20.0).abs() < 1e-9,
        "the centre snaps: {centre:?}"
    );
    // The radius handle now sits at (-10, 20); the radius is the distance to the snapped pointer
    // in any direction (the page grid starts at 0).
    session.pointer_down(Point::new(-10.0, 20.0), false);
    session.pointer_hover(Point::new(33.6, 20.4), false, false);
    session.pointer_up(Point::new(33.6, 20.4), false, false);
    let radius = session.tiling_recipe().unwrap().circle.radius.as_mm();
    assert!((radius - 25.0).abs() < 1e-9, "the radius snaps: {radius}");
    // The suspend keys of every other Select-tool handle (Ctrl+Alt) suspend it.
    session.pointer_down(Point::new(-15.0, 20.0), false);
    session.modifiers_changed(false, true, true);
    session.pointer_hover(Point::new(36.3, 20.0), false, false);
    session.pointer_up(Point::new(36.3, 20.0), false, false);
    session.modifiers_changed(false, false, false);
    let radius = session.tiling_recipe().unwrap().circle.radius.as_mm();
    assert!((radius - 26.3).abs() < 1e-9, "suspended: {radius}");
}

#[test]
fn a_handle_drag_shows_its_readout_and_the_fields_follow() {
    let mut session = session_with_one_path();
    circle_bar(&mut session);
    assert!(session.live_readout().is_none(), "no drag, no readout");
    session.pointer_down(Point::new(0.0, 15.0), false);
    session.pointer_hover(Point::new(-4.0, 12.5), false, false);
    let readout = session.live_readout().expect("a centre drag reads out");
    assert_eq!(readout.text, "Centre \u{2212}4.0, 12.5 mm");
    session.pointer_up(Point::new(-4.0, 12.5), false, false);
    assert!(session.live_readout().is_none());
    assert!(session.tiling_recipe_json().contains("\"centre_x\":-4.0"));
    session.pointer_down(Point::new(-24.0, 12.5), false);
    session.pointer_hover(Point::new(-46.0, 12.5), false, false);
    assert_eq!(session.live_readout().unwrap().text, "r 42.0 mm");
    session.pointer_up(Point::new(-46.0, 12.5), false, false);
}

#[test]
fn a_page_switch_cancels_the_bar() {
    let mut session = session_with_one_path();
    let first = session.active_page().id;
    let second = session.add_page();
    assert!(session.activate_page(&first));
    select_everything(&mut session);
    assert!(session.tiling_enter(3));
    assert!(session.tiling_open());
    assert!(session.activate_page(&second));
    assert!(!session.tiling_open());
}

fn select_everything(session: &mut Session) {
    let _ = session.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
}

#[test]
fn the_preview_draws_ghost_fills_and_the_dashed_circle() {
    let mut session = session_with_one_path();
    session.tiling_enter(3);
    let outlines_only = session.draw_list().triangle_count();
    let mut recipe = session.tiling_recipe().unwrap();
    recipe.variance.hue.random = curvyo_document_core::Angle::from_radians(1.0);
    assert!(session.tiling_set_recipe(recipe));
    assert!(session.draw_list().triangle_count() >= outlines_only);
    recipe.layout = TilingLayout::Circle;
    assert!(session.tiling_set_recipe(recipe));
    let with_circle = session.draw_list().triangle_count();
    recipe.circle.radius = curvyo_document_core::Length::from_mm(40.0);
    assert!(session.tiling_set_recipe(recipe));
    assert!(
        session.draw_list().triangle_count() > with_circle,
        "a longer dashed circle has more dashes"
    );
}

#[test]
fn a_grid_left_over_the_limit_does_not_block_the_circle() {
    let mut session = session_with_one_path();
    session.tiling_enter(3);
    let mut recipe = session.tiling_recipe().unwrap();
    recipe.grid.columns = 1000;
    recipe.grid.rows = 1000;
    assert!(
        session.tiling_set_recipe(recipe),
        "the refused count is shown"
    );
    recipe.layout = TilingLayout::Circle;
    assert!(session.tiling_set_recipe(recipe));
    let taken = session.tiling_recipe().unwrap();
    assert_eq!(taken.grid.columns, 1000);
    assert_eq!(taken.grid.rows, 100, "cut to the limit of 100,000");
    assert!(session.tiling_apply(), "the circle applies");
}

/// A tiling of the one path (5 x 4, 120 x 55 mm from (10, 10)) between a 4 mm and a 20 mm
/// square, all on the page.
fn tiling_between_squares() -> (Session, NodeId) {
    let document = document_with_one_path();
    let _ = document.create_rect(RectBounds::from_corners(
        Point::new(0.0, 100.0),
        Point::new(4.0, 104.0),
    ));
    let _ = document.create_rect(RectBounds::from_corners(
        Point::new(180.0, 100.0),
        Point::new(200.0, 120.0),
    ));
    let mut session = Session::open(2, &pack(&document, "t").unwrap()).unwrap();
    session.set_tool(Tool::Select);
    session.resize_viewport(1600.0, 900.0);
    session.pointer_hover(Point::new(20.0, 10.0), false, false);
    session.pointer_down(Point::new(20.0, 10.0), false);
    session.pointer_up(Point::new(20.0, 10.0), false, false);
    assert_eq!(session.selected_object_count(), 1, "the path alone");
    assert!(session.tiling_enter(3));
    assert!(session.tiling_apply());
    let TileButton::Edit(group) = session.tile_button() else {
        panic!("the tiling is selected")
    };
    select_everything(&mut session);
    assert_eq!(
        session.selected_object_count(),
        22,
        "the tiling's leaves and both squares"
    );
    (session, group)
}

/// The left top corner of every child of `group`, as saved.
fn corners(session: &Session, group: NodeId) -> Vec<Point> {
    let document = unpack(1, &session.pack("t").unwrap()).unwrap();
    document
        .children(Some(group))
        .into_iter()
        .map(|child| object_bounds(&document.object(child).unwrap()).0)
        .collect()
}

fn moved_as_one(before: &[Point], after: &[Point], dx: f64) {
    assert_eq!(before.len(), 20);
    for (b, a) in before.iter().zip(after) {
        assert!(
            (a.x - b.x - dx).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6,
            "{b:?} -> {a:?}"
        );
    }
}

#[test]
fn align_moves_a_tiling_as_one_object() {
    let (mut session, group) = tiling_between_squares();
    let before = corners(&session, group);
    assert_eq!(
        session.align_selection(AlignCommand::Right),
        AlignOutcome::Aligned
    );
    // The selection's right edge is 200; the tiling's was 10 + 5 x 20 + 4 x 5 = 130.
    moved_as_one(&before, &corners(&session, group), 70.0);
    let document = unpack(1, &session.pack("t").unwrap()).unwrap();
    assert!(document.tiling(group).is_some(), "still a tiling");
}

#[test]
fn distribute_moves_a_tiling_as_one_object() {
    let (mut session, group) = tiling_between_squares();
    let before = corners(&session, group);
    assert_eq!(
        session.distribute_selection(AlignAxis::Horizontal, DistributeMode::Gaps, None),
        AlignOutcome::Aligned
    );
    // Span 0 to 200, boxes 4 + 120 + 20: gaps of 28, so the tiling starts at 32.
    moved_as_one(&before, &corners(&session, group), 22.0);
}

#[test]
fn no_transform_handle_is_hit_while_the_bar_is_open() {
    let mut session = session_with_one_path();
    // Around the box (10..30, 10..20), find a point where the Select tool shows a handle cursor.
    let mut probes = Vec::new();
    for ix in 0..=30 {
        for iy in 0..=30 {
            probes.push(Point::new(5.0 + f64::from(ix), 5.0 + f64::from(iy) * 0.6));
        }
    }
    let mut handle_points = Vec::new();
    for &probe in &probes {
        session.pointer_hover(probe, false, false);
        let hint = session.cursor_hint();
        if hint.starts_with("resize") || hint.starts_with("skew") || hint == "rotate" {
            handle_points.push(probe);
        }
    }
    assert!(!handle_points.is_empty(), "the Select tool has handles");
    assert!(session.tiling_enter(3));
    for &probe in &handle_points {
        session.pointer_hover(probe, false, false);
        assert_eq!(session.cursor_hint(), "default", "at {probe:?}");
    }
    let _ = session.draw_list();
}

fn degrees(value: curvyo_document_core::Angle) -> f64 {
    value.as_radians().to_degrees()
}

/// The point at `angle` degrees (counterclockwise on screen) on the circle of `circle_bar`.
fn on_circle(session: &Session, angle: f64) -> Point {
    let centre = circle_centre(session);
    let radius = session.tiling_recipe().unwrap().circle.radius.as_mm();
    Point::new(
        radius.mul_add(angle.to_radians().cos(), centre.x),
        radius.mul_add(-angle.to_radians().sin(), centre.y),
    )
}

#[test]
fn the_start_handle_turns_the_arrangement_and_keeps_the_sweep() {
    let mut session = session_with_one_path();
    circle_bar(&mut session);
    let mut recipe = session.tiling_recipe().unwrap();
    recipe.circle.sweep = curvyo_document_core::Angle::from_radians(120f64.to_radians());
    assert!(session.tiling_set_recipe(recipe));
    session.pointer_down(on_circle(&session, 0.0), false);
    session.pointer_hover(on_circle(&session, 90.0), false, false);
    assert_eq!(
        session.live_readout().unwrap().text,
        "start 90.0\u{b0} \u{b7} sweep 120.0\u{b0}"
    );
    session.pointer_up(on_circle(&session, 90.0), false, false);
    let circle = session.tiling_recipe().unwrap().circle;
    assert!((degrees(circle.start) - 90.0).abs() < 1e-9);
    assert!(
        (degrees(circle.sweep) - 120.0).abs() < 1e-9,
        "the sweep is kept"
    );
    // Ctrl snaps to the 15 and 22.5 degree stops.
    session.pointer_down(on_circle(&session, 90.0), false);
    session.modifiers_changed(false, true, false);
    session.pointer_hover(on_circle(&session, 31.0), false, true);
    session.pointer_up(on_circle(&session, 31.0), false, true);
    session.modifiers_changed(false, false, false);
    let start = degrees(session.tiling_recipe().unwrap().circle.start);
    assert!((start - 30.0).abs() < 1e-9, "snapped start {start}");
}

#[test]
fn the_sweep_handle_changes_the_sweep_and_snaps_to_the_full_turn() {
    let mut session = session_with_one_path();
    circle_bar(&mut session);
    let mut recipe = session.tiling_recipe().unwrap();
    recipe.circle.sweep = curvyo_document_core::Angle::from_radians(120f64.to_radians());
    assert!(session.tiling_set_recipe(recipe));
    session.pointer_down(on_circle(&session, 120.0), false);
    session.pointer_hover(on_circle(&session, 200.0), false, false);
    assert_eq!(
        session.live_readout().unwrap().text,
        "end \u{2212}160.0\u{b0} \u{b7} sweep 200.0\u{b0}"
    );
    // On round the circle: unwrapped, never flipped to a negative sweep.
    session.pointer_hover(on_circle(&session, 300.0), false, false);
    session.pointer_hover(on_circle(&session, 359.9), false, false);
    session.pointer_up(on_circle(&session, 359.9), false, false);
    let sweep = degrees(session.tiling_recipe().unwrap().circle.sweep);
    assert!(
        (sweep - 360.0).abs() < 1e-9,
        "snapped to the full turn: {sweep}"
    );
    // A small sweep, Ctrl: the stops.
    let mut recipe = session.tiling_recipe().unwrap();
    recipe.circle.sweep = curvyo_document_core::Angle::from_radians(60f64.to_radians());
    assert!(session.tiling_set_recipe(recipe));
    session.pointer_down(on_circle(&session, 60.0), false);
    session.modifiers_changed(false, true, false);
    session.pointer_hover(on_circle(&session, 46.0), false, true);
    session.pointer_up(on_circle(&session, 46.0), false, true);
    session.modifiers_changed(false, false, false);
    let sweep = degrees(session.tiling_recipe().unwrap().circle.sweep);
    assert!((sweep - 45.0).abs() < 1e-9, "snapped sweep {sweep}");
}

#[test]
fn a_double_click_on_a_handle_opens_its_entry_box() {
    let mut session = session_with_one_path();
    circle_bar(&mut session);
    let radius_handle = Point::new(-20.0, 15.0);
    assert!(!session.double_click(radius_handle, false, false));
    assert_eq!(session.tiling_entry(), Some(CircleHandle::Radius));
    assert!(
        session
            .tiling_entry_json()
            .contains("\"handle\":\"radius\"")
    );
    // A double-click on the centre opens the centre's box instead.
    session.tiling_close_entry(false);
    let _ = session.double_click(Point::new(0.0, 15.0), false, false);
    assert_eq!(session.tiling_entry(), Some(CircleHandle::Centre));
    // Away from every handle: nothing opens, the bar stays.
    session.tiling_close_entry(false);
    let _ = session.double_click(Point::new(10.0, 40.0), false, false);
    assert_eq!(session.tiling_entry(), None);
    assert!(session.tiling_open());
}

#[test]
fn enter_keeps_the_typed_values_and_escape_restores_them() {
    let mut session = session_with_one_path();
    circle_bar(&mut session);
    let before = session.tiling_recipe().unwrap();
    assert!(session.tiling_open_entry(CircleHandle::Start));
    // Typing previews through the recipe; 270 is stored as -90.
    let json = session
        .tiling_recipe_json()
        .replace("\"start\":0.0", "\"start\":270.0");
    assert!(session.tiling_set_recipe_json(&json));
    assert!((degrees(session.tiling_recipe().unwrap().circle.start) + 90.0).abs() < 1e-9);
    // Escape: the values it opened with, the tiling still open.
    session.tiling_close_entry(true);
    assert_eq!(session.tiling_recipe(), Some(before));
    assert!(session.tiling_open());
    assert_eq!(session.tiling_entry(), None);
    // Enter: the typed radius stays.
    assert!(session.tiling_open_entry(CircleHandle::Radius));
    let json = session
        .tiling_recipe_json()
        .replace("\"radius\":20.0", "\"radius\":35.0");
    assert!(session.tiling_set_recipe_json(&json));
    session.tiling_close_entry(false);
    let radius = session.tiling_recipe().unwrap().circle.radius.as_mm();
    assert!((radius - 35.0).abs() < 1e-9);
    // A refused value (radius 0) keeps the last valid preview.
    assert!(session.tiling_open_entry(CircleHandle::Radius));
    let json = session
        .tiling_recipe_json()
        .replace("\"radius\":35.0", "\"radius\":0.0");
    assert!(!session.tiling_set_recipe_json(&json));
    let radius = session.tiling_recipe().unwrap().circle.radius.as_mm();
    assert!((radius - 35.0).abs() < 1e-9);
}

#[test]
fn a_press_elsewhere_closes_the_box_as_escape_does() {
    let mut session = session_with_one_path();
    circle_bar(&mut session);
    let before = session.tiling_recipe().unwrap();
    assert!(session.tiling_open_entry(CircleHandle::Sweep));
    let json = session
        .tiling_recipe_json()
        .replace("\"sweep\":360.0", "\"sweep\":90.0");
    assert!(session.tiling_set_recipe_json(&json));
    session.pointer_down(Point::new(10.0, 40.0), false);
    session.pointer_up(Point::new(10.0, 40.0), false, false);
    assert_eq!(session.tiling_entry(), None);
    assert_eq!(session.tiling_recipe(), Some(before));
    // A tool switch drops the box with the bar.
    assert!(session.tiling_open_entry(CircleHandle::Centre));
    session.set_tool(Tool::Pen);
    assert!(!session.tiling_open());
}

#[test]
fn the_keys_c_r_a_s_open_the_boxes_in_the_circle_only() {
    let mut session = session_with_one_path();
    session.tiling_enter(3);
    let key = |session: &mut Session, key: &str| {
        session.key_down(KeyInput {
            key,
            ..KeyInput::default()
        })
    };
    // In the grid the keys open nothing.
    let _ = key(&mut session, "a");
    assert_eq!(session.tiling_entry(), None);
    assert!(session.tiling_open());
    let mut recipe = session.tiling_recipe().unwrap();
    recipe.layout = TilingLayout::Circle;
    assert!(session.tiling_set_recipe(recipe));
    for (code, handle) in [
        ("c", CircleHandle::Centre),
        ("r", CircleHandle::Radius),
        ("a", CircleHandle::Start),
        ("s", CircleHandle::Sweep),
    ] {
        assert_eq!(key(&mut session, code), KeyOutcome::EntryOpened);
        assert_eq!(session.tiling_entry(), Some(handle));
        assert!(session.tiling_open(), "{code} keeps the bar open");
        session.tiling_close_entry(true);
    }
}
