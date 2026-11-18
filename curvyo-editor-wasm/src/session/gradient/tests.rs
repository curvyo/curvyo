//! `specs/0124-gradient-fill` criteria 2, 3, 6 to 8, 10 and 18 through the session: the block,
//! its commands, the canvas handles with their drags, yields, keys, double-clicks and entry
//! boxes, and the grey of a laser page.

use curvyo_document_core::{FillPaint, GradientGeometry, NodeId, ObjectSnapshot, Point};
use curvyo_ui_core::{EditHandle, ResizeDirection};

use super::super::align_test_support::{draw_rect, select_all};
use super::super::history_preview::HistoryRowPress;
use super::super::{KeyInput, Session, Tool};

/// A 1200 x 800 canvas at 1 px per mm with one 100 x 40 mm rectangle selected, Gradient chosen.
fn shaded_session() -> (Session, NodeId) {
    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    let id = draw_rect(&mut session, (10.0, 10.0), (110.0, 50.0));
    session.set_tool(Tool::Select);
    select_all(&mut session);
    session.set_fill_paint(true);
    assert!(session.set_fill_kind("gradient"));
    (session, id)
}

fn object(session: &Session, id: NodeId) -> ObjectSnapshot {
    session.document.object(id).unwrap()
}

fn geometry(session: &Session, id: NodeId) -> GradientGeometry {
    object(session, id)
        .style()
        .fill
        .gradient
        .clone()
        .unwrap()
        .geometry
}

fn view(session: &Session) -> serde_json::Value {
    serde_json::to_value(session.gradient_view()).unwrap()
}

fn handles(session: &Session) -> serde_json::Value {
    serde_json::to_value(session.gradient_handles()).unwrap()
}

fn key(session: &mut Session, name: &str) {
    session.key_down(KeyInput {
        key: name,
        ..KeyInput::default()
    });
}

/// The screen point of handle `n` of the handles view, as a document point at 1 px per mm.
fn handle_point(session: &Session, n: usize) -> Point {
    let h = &handles(session)["handles"][n];
    session.screen_to_document(h["x"].as_f64().unwrap(), h["y"].as_f64().unwrap())
}

/// Criteria 2 and 3: Gradient starts Linear from the Colour to white at Angle 0; the block shows
/// the two fixed rows.
#[test]
fn choosing_gradient_shows_the_block() {
    let (session, id) = shaded_session();
    let fill = object(&session, id).style().fill.clone();
    assert_eq!(fill.paint, FillPaint::Gradient);
    let v = view(&session);
    assert_eq!(v["shown"], true);
    assert_eq!(v["kind"], "linear");
    assert_eq!(v["angle"], "0.0");
    assert_eq!(v["stops"][1]["hex"], "#FFFFFFFF");
    assert_eq!(v["stops"][0]["fixed"], true);
    assert_eq!(v["activeStop"], 0);
}

/// Criterion 3: Kind, Angle (wrapped), Radius, a stop's hex and position, Add, Remove and
/// Reverse, each one undo step.
#[test]
fn the_block_commands_are_one_step_each() {
    let (mut session, id) = shaded_session();
    assert_eq!(session.set_gradient_angle_text("190"), Ok(true));
    assert_eq!(view(&session)["angle"], "-170.0");
    assert_eq!(session.set_gradient_angle_text("x"), Err("number"));
    assert!(session.add_gradient_stop());
    assert_eq!(view(&session)["activeStop"], 1);
    assert_eq!(session.set_gradient_stop_position_text(1, "25"), Ok(true));
    assert_eq!(view(&session)["stops"][1]["position"], "25");
    assert_eq!(session.set_gradient_stop_hex(1, "#FF000080"), Ok(true));
    assert_eq!(view(&session)["stops"][1]["hex"], "#FF000080");
    assert!(session.reverse_gradient());
    assert_eq!(view(&session)["stops"][1]["position"], "75");
    assert_eq!(view(&session)["activeStop"], 1);
    assert!(session.remove_gradient_stop(1));
    assert_eq!(view(&session)["stops"].as_array().unwrap().len(), 2);
    assert!(!session.remove_gradient_stop(0), "ends stay");
    assert!(session.set_gradient_kind("radial"));
    assert_eq!(view(&session)["radius"], "50");
    assert_eq!(session.set_gradient_radius_text("0"), Err("range"));
    assert_eq!(session.set_gradient_radius_text("20"), Ok(true));
    assert_eq!(view(&session)["radius"], "20");
    // Undo walks back one edit at a time to the first Linear.
    for _ in 0..8 {
        session.undo();
    }
    assert_eq!(geometry(&session, id), GradientGeometry::DEFAULT_LINEAR);
    assert_eq!(view(&session)["stops"].as_array().unwrap().len(), 2);
}

/// Criterion 6: the handles sit on the edges; the covered W and E handles yield.
#[test]
fn the_handles_show_and_the_covered_shape_handles_yield() {
    let (session, _) = shaded_session();
    let h = handles(&session);
    assert_eq!(h["shown"], true);
    assert_eq!(h["handles"][0]["kind"], "start");
    let start = handle_point(&session, 0);
    assert!((start.x - 10.0).abs() < 1e-6 && (start.y - 30.0).abs() < 1e-6);
    let yielded = session.transform_handle_tolerances().yielded;
    assert!(yielded.contains(EditHandle::Resize(ResizeDirection::W)));
    assert!(yielded.contains(EditHandle::Resize(ResizeDirection::E)));
    assert!(!yielded.contains(EditHandle::Resize(ResizeDirection::N)));
}

/// Criteria 6 and 18: an end drag previews, the release is one step, Escape restores and the
/// release after it writes nothing; the shape is never moved.
#[test]
fn an_end_drag_is_one_step_and_escape_restores() {
    let (mut session, id) = shaded_session();
    let end = handle_point(&session, 1);
    let before = object(&session, id);
    let steps = session.undo_len();
    session.pointer_down(end, false);
    let to = Point::new(end.x - 20.0, end.y + 10.0);
    session.pointer_hover(to, false, false);
    assert_eq!(
        geometry(&session, id),
        GradientGeometry::DEFAULT_LINEAR,
        "previewed only"
    );
    assert!(
        handles(&session)["readout"]["text"]
            .as_str()
            .unwrap()
            .ends_with('°')
    );
    session.pointer_up(to, false, false);
    assert_eq!(session.undo_len(), steps + 1);
    assert_ne!(geometry(&session, id), GradientGeometry::DEFAULT_LINEAR);
    assert_eq!(
        curvyo_ui_core::object_outline_bounds(&object(&session, id)),
        curvyo_ui_core::object_outline_bounds(&before),
        "the shape stays"
    );
    session.undo();
    session.pointer_down(end, false);
    session.pointer_hover(to, false, false);
    key(&mut session, "Escape");
    session.pointer_up(to, false, false);
    assert_eq!(session.undo_len(), steps);
    assert_eq!(geometry(&session, id), GradientGeometry::DEFAULT_LINEAR);
}

/// Criteria 6 and 7: a double-click on the line adds a stop there; Delete removes the selected
/// inner stop; Delete on an end does nothing and keeps the shape.
#[test]
fn double_click_adds_and_delete_removes_a_stop() {
    let (mut session, id) = shaded_session();
    assert!(!session.double_click(Point::new(60.0, 30.0), false, false));
    assert_eq!(view(&session)["stops"].as_array().unwrap().len(), 3);
    assert_eq!(view(&session)["stops"][1]["position"], "50");
    assert_eq!(view(&session)["activeStop"], 1);
    key(&mut session, "Delete");
    assert_eq!(view(&session)["stops"].as_array().unwrap().len(), 2);
    let start = handle_point(&session, 0);
    session.pointer_down(start, false);
    session.pointer_up(start, false, false);
    key(&mut session, "Delete");
    assert!(session.document.object(id).is_some(), "the shape stays");
    assert_eq!(view(&session)["stops"].as_array().unwrap().len(), 2);
}

/// Criterion 7: a double-click on the End opens its box with the Angle; Enter writes Angle and
/// colour as one step.
#[test]
fn the_end_entry_box_commits_as_one_step() {
    let (mut session, _) = shaded_session();
    let end = handle_point(&session, 1);
    session.double_click(end, false, false);
    let entry = handles(&session)["entry"].clone();
    assert_eq!(entry["kind"], "end");
    assert_eq!(entry["field"], "angle");
    assert_eq!(entry["fieldText"], "0.0");
    let steps = session.undo_len();
    assert_eq!(session.commit_gradient_entry("x", "#000"), Err("number"));
    assert_eq!(session.commit_gradient_entry("45", "#00FF00"), Ok(true));
    assert_eq!(session.undo_len(), steps + 1);
    assert_eq!(view(&session)["angle"], "45.0");
    assert_eq!(view(&session)["stops"][1]["hex"], "#00FF00FF");
    assert!(handles(&session)["entry"].is_null());
}

/// Criterion 8: two shapes with different stop counts show "Mixed gradients"; Reverse acts on
/// each.
#[test]
fn a_mixed_selection_shows_mixed_gradients() {
    let (mut session, a) = shaded_session();
    let b = draw_rect(&mut session, (200.0, 10.0), (260.0, 50.0));
    session.set_tool(Tool::Select);
    session.selection.set(&[b]);
    session.set_fill_paint(true);
    session.set_fill_kind("gradient");
    session.add_gradient_stop();
    session.selection.set(&[a, b]);
    let v = view(&session);
    assert_eq!(v["mixedGradients"], true);
    assert_eq!(v["stops"].as_array().unwrap().len(), 0);
    assert!(session.reverse_gradient());
    assert_eq!(
        handles(&session)["handles"][0]["kind"],
        "start",
        "handles of the first shape"
    );
}

/// Criterion 3: a stop row's eyedropper takes a colour from the drawing into that stop, alpha
/// included, as one step "Edit gradient"; the panel names the target `stop-<n>`.
#[test]
fn a_stop_eyedropper_takes_a_colour_from_the_drawing() {
    let (mut session, a) = shaded_session();
    let b = draw_rect(&mut session, (200.0, 10.0), (260.0, 50.0));
    session.set_tool(Tool::Select);
    session.selection.set(&[b]);
    session.set_fill_paint(true);
    session.set_fill_kind("solid");
    session
        .document
        .edit_style(
            &[b],
            &curvyo_document_core::StyleEdit::FillRgba(
                curvyo_document_core::Color {
                    r: 18,
                    g: 52,
                    b: 86,
                },
                curvyo_document_core::Opacity::new(0.5).unwrap(),
            ),
        )
        .unwrap();
    session.selection.set(&[a]);
    session.begin_colour_pick(curvyo_ui_core::PaintTarget::GradientStop(1));
    assert_eq!(session.style_panel_view().pick_target, "stop-1");
    let steps = session.document.own_steps().len();
    let at = Point::new(230.0, 30.0);
    session.pointer_hover(at, false, false);
    session.pointer_down(at, false);
    session.pointer_up(at, false, false);
    assert_eq!(session.colour_pick_target(), None, "the pick ends");
    assert_eq!(view(&session)["stops"][1]["hex"], "#12345680");
    assert_eq!(session.document.own_steps().len(), steps + 1);
    assert_eq!(
        session.document.own_steps().last().unwrap().label,
        curvyo_document_core::EDIT_GRADIENT_LABEL
    );
    assert_eq!(
        session.take_colour_pick_announcement(),
        "Gradient stop color set to #12345680"
    );
}

/// UX review U6: dragging the Radial centre moves the whole circle and shows no readout; the
/// radius handle shows its Radius.
#[test]
fn a_radial_centre_drag_shows_no_readout() {
    let (mut session, _) = shaded_session();
    assert!(session.set_gradient_kind("radial"));
    assert_eq!(handles(&session)["handles"][0]["kind"], "centre");
    let centre = handle_point(&session, 0);
    session.pointer_down(centre, false);
    session.pointer_hover(Point::new(centre.x + 7.0, centre.y + 3.0), false, false);
    assert!(
        handles(&session)["readout"].is_null(),
        "{}",
        handles(&session)
    );
    session.pointer_up(Point::new(centre.x + 7.0, centre.y + 3.0), false, false);
    let radius = handle_point(&session, 1);
    session.pointer_down(radius, false);
    session.pointer_hover(Point::new(radius.x + 5.0, radius.y), false, false);
    assert!(
        handles(&session)["readout"]["text"]
            .as_str()
            .unwrap()
            .ends_with("mm")
    );
    session.pointer_up(Point::new(radius.x + 5.0, radius.y), false, false);
}

/// Criterion 21 as amended (UX review U9): with a group selected, each member gets its own
/// gradient in its own box, as with a Solid fill.
#[test]
fn a_selected_group_gives_each_member_its_own_gradient() {
    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    let a = draw_rect(&mut session, (10.0, 10.0), (60.0, 40.0));
    let b = draw_rect(&mut session, (100.0, 10.0), (200.0, 80.0));
    session.document.group_objects(&[a, b]).unwrap();
    session.set_tool(Tool::Select);
    let at = Point::new(10.0, 25.0);
    session.pointer_hover(at, false, false);
    session.pointer_down(at, false);
    session.pointer_up(at, false, false);
    assert_eq!(session.selection.ids(), [a, b], "the group, as its members");
    session.set_fill_paint(true);
    assert!(session.set_fill_kind("gradient"));
    for id in [a, b] {
        assert_eq!(
            geometry(&session, id),
            GradientGeometry::DEFAULT_LINEAR,
            "each member in its own box"
        );
    }
}

/// Criterion 10: on a laser page the gradient is drawn grey.
#[test]
fn a_laser_page_draws_the_gradient_grey() {
    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    session.add_output_page_with("curvyo.laser", None).unwrap();
    let page = session.active_page();
    let (x, y) = (page.origin.x + 10.0, page.origin.y + 10.0);
    draw_rect(&mut session, (x, y), (x + 50.0, y + 30.0));
    session.set_tool(Tool::Select);
    select_all(&mut session);
    session.set_fill_paint(true);
    session.set_fill_kind("solid");
    session.set_fill_kind("gradient");
    assert_eq!(session.set_gradient_stop_hex(0, "#FF0000"), Ok(true));
    // View: Drawn; the Operations view draws every fill by its kind's look (`0101`).
    if session.operations_view_on() {
        session.toggle_operations_view(&page.id);
    }
    let frame = session.frame_draw_list();
    let stops = frame.gradient_stops();
    assert_eq!(stops.len(), 2);
    for s in stops {
        assert_eq!(
            s.colour,
            [s.colour[0], s.colour[0], s.colour[0], 1.0],
            "{stops:?}"
        );
    }
    assert!(
        (stops[0].colour[0] - 54.0 / 255.0).abs() < 1e-6,
        "red is 21 % grey"
    );
}

/// Criterion 10 in the History previews: an object version and a document version of a shape
/// on a laser page draw its gradient grey as the live canvas does.
#[test]
fn a_history_preview_on_a_laser_page_draws_the_gradient_grey() {
    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    session.add_output_page_with("curvyo.laser", None).unwrap();
    let page = session.active_page();
    let (x, y) = (page.origin.x + 10.0, page.origin.y + 10.0);
    draw_rect(&mut session, (x, y), (x + 50.0, y + 30.0));
    session.set_tool(Tool::Select);
    select_all(&mut session);
    session.set_fill_paint(true);
    session.set_fill_kind("solid");
    session.set_fill_kind("gradient");
    assert_eq!(session.set_gradient_stop_hex(0, "#FF0000"), Ok(true));
    assert_eq!(session.set_gradient_stop_hex(0, "#00FF00"), Ok(true));
    if session.operations_view_on() {
        session.toggle_operations_view(&page.id);
    }
    for object in [true, false] {
        assert_eq!(
            session.preview_history_item(object, 1),
            HistoryRowPress::Previewed,
            "object scope {object}"
        );
        let frame = session.frame_draw_list();
        let stops = frame.gradient_stops();
        assert!(!stops.is_empty(), "object scope {object}");
        for s in stops {
            assert!(
                (s.colour[0] - s.colour[1]).abs() < 1e-6
                    && (s.colour[1] - s.colour[2]).abs() < 1e-6,
                "object scope {object}: {stops:?}"
            );
        }
        session.end_history_preview();
    }
}

/// Criterion 27: one `reflect` and two focal points, no mesh: exactly lines one and two, once.
#[test]
fn an_import_that_lost_gradient_features_says_so_once() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 100 50">
  <defs>
    <linearGradient id="a" spreadMethod="reflect"><stop offset="0" stop-color="#f00"/><stop offset="1" stop-color="#00f"/></linearGradient>
    <radialGradient id="b" fx="0.2" fy="0.2"><stop offset="0" stop-color="#0f0"/><stop offset="1" stop-color="#000"/></radialGradient>
    <radialGradient id="c" fx="0.8" fy="0.3"><stop offset="0" stop-color="#0f0"/><stop offset="1" stop-color="#000"/></radialGradient>
  </defs>
  <rect x="0" y="0" width="30" height="20" fill="url(#a)"/>
  <rect x="40" y="0" width="30" height="20" fill="url(#b)"/>
  <rect x="0" y="25" width="30" height="20" fill="url(#c)"/>
</svg>"##;
    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    session.import_svg(svg, "lost.svg").unwrap();
    assert_eq!(
        session.take_gradient_import_notice(),
        "1 gradient repeat was shown as a plain shade.\n2 focal points were read as the centre."
    );
    assert_eq!(session.take_gradient_import_notice(), "");
}
