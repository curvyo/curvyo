use super::*;
use crate::session::keys::KeyInput;
use curvyo_document_core::DisplayUnit;
use curvyo_document_core::{AnchorId, EllipseFrame, NewAnchor, NodeId};

fn line(session: &Session, from: (f64, f64), to: (f64, f64)) -> NodeId {
    session.document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(9, 1), Point::new(from.0, from.1)),
            NewAnchor::corner(AnchorId::new(9, 2), Point::new(to.0, to.1)),
        ],
        false,
    )
}

fn circle(session: &Session) -> NodeId {
    session.document.create_primitive(
        curvyo_document_core::Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(100.0, 100.0),
                rx: Length::from_mm(25.0),
                ry: Length::from_mm(25.0),
            },
            arc: None,
            curve: curvyo_document_core::Curve::ELLIPSE_DEFAULT,
        },
        Angle::from_radians(0.0),
    )
}

fn click(session: &mut Session, at: Point, shift: bool) {
    session.modifiers_changed(shift, false, false);
    session.pointer_hover(at, shift, false);
    session.pointer_down(at, shift);
    session.pointer_up(at, shift, false);
}

fn dimensions(session: &Session) -> Vec<curvyo_document_core::DimensionSnapshot> {
    session
        .document
        .read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Dimension(d) => Some((**d).clone()),
            _ => None,
        })
        .collect()
}

fn view(session: &Session) -> serde_json::Value {
    let _ = session.draw_list();
    serde_json::from_str(&session.measure_view_json()).unwrap()
}

/// A session with snapping on: picks snap to point features only then (criterion 6).
fn measure_session() -> (Session, NodeId) {
    let mut session = Session::new(1);
    let mut snapping = session.snapping();
    snapping.enabled = true;
    session.set_snapping(snapping);
    let path = line(&session, (10.0, 10.0), (60.0, 10.0));
    session.set_tool(Tool::Measure);
    (session, path)
}

#[test]
fn a_measurement_writes_nothing_and_stays_pinned_until_the_next_press() {
    let (mut session, _) = measure_session();
    let steps = session.document.own_step_count();
    click(&mut session, Point::new(10.0, 10.0), false);
    session.pointer_hover(Point::new(60.0, 10.0), false, false);
    click(&mut session, Point::new(60.0, 10.0), false);
    assert_eq!(session.document.own_step_count(), steps);
    let shown = view(&session);
    assert_eq!(shown["readout"]["lines"][0], "50 mm, 0°");
    assert_eq!(shown["mode"]["lines"][0], "Aligned");
    session.pointer_hover(Point::new(200.0, 200.0), false, false);
    assert!(
        view(&session)["readout"].is_object(),
        "pan or a move keeps it"
    );
    click(&mut session, Point::new(200.0, 200.0), false);
    session.pointer_hover(Point::new(205.0, 200.0), false, false);
    assert_eq!(
        view(&session)["readout"]["lines"][0],
        "5 mm, 0°",
        "the next press starts a new measurement"
    );
    assert_eq!(session.document.own_step_count(), steps);
}

#[test]
fn shift_at_the_second_pick_leaves_a_linked_dimension_in_one_step() {
    let (mut session, path) = measure_session();
    click(&mut session, Point::new(10.0, 10.0), false);
    session.pointer_hover(Point::new(60.0, 10.0), true, false);
    click(&mut session, Point::new(60.0, 10.0), true);
    // Above the box of the two points: Horizontal.
    session.pointer_hover(Point::new(35.0, -10.0), false, false);
    assert_eq!(view(&session)["mode"]["lines"][0], "Dimension, horizontal");
    let steps = session.document.own_step_count();
    click(&mut session, Point::new(35.0, -10.0), false);
    assert_eq!(session.document.own_step_count(), steps + 1);
    assert_eq!(
        session.document.own_steps().last().unwrap().label,
        curvyo_document_core::CREATE_DIMENSION_COMMIT_LABEL
    );
    let made = dimensions(&session);
    assert_eq!(made.len(), 1);
    assert_eq!(made[0].kind, DimensionKind::Horizontal);
    assert!(made[0].linked);
    assert_eq!(made[0].slots.nodes(), vec![path]);
    assert_eq!(view(&session)["dimensions"][0]["text"], "↔ 50 mm");
}

#[test]
fn a_curve_pick_with_shift_is_a_radius_and_alt_switches_to_diameter() {
    let mut session = Session::new(1);
    let shape = circle(&session);
    session.set_tool(Tool::Measure);
    click(&mut session, Point::new(125.0, 100.0), true);
    assert!(session.measure_alt(false, false));
    click(&mut session, Point::new(160.0, 60.0), false);
    let made = dimensions(&session);
    assert_eq!(made[0].kind, DimensionKind::Diameter);
    assert_eq!(made[0].slots.nodes(), vec![shape]);
    assert!((made[0].shown.value.raw() - 50.0).abs() < 1e-9);
}

#[test]
fn enter_on_a_selected_segment_places_its_length() {
    let (mut session, path) = measure_session();
    session.selection.select_single(path);
    assert!(session.measure_enter());
    let made = dimensions(&session);
    assert_eq!(made[0].kind, DimensionKind::Aligned);
    assert!((made[0].shown.value.raw() - 50.0).abs() < 1e-9);
    assert_eq!(
        session.selection.ids(),
        &[path],
        "the selection does not change"
    );
}

#[test]
fn deleting_the_path_unlinks_the_dimension_and_undo_links_it_again() {
    let (mut session, path) = measure_session();
    session.selection.select_single(path);
    assert!(session.measure_enter());
    session.set_tool(Tool::Select);
    session.selection.select_single(path);
    session.delete_selected();
    let shown = view(&session);
    assert_eq!(shown["dimensions"][0]["linked"], false);
    assert!(
        shown["dimensions"][0]["name"]
            .as_str()
            .unwrap()
            .ends_with("not linked")
    );
    assert_eq!(shown["dimensions"][0]["text"], "50 mm");
    session.undo();
    assert_eq!(view(&session)["dimensions"][0]["linked"], true);
}

#[test]
fn the_select_tool_drags_the_text_as_one_step_and_guide_converts() {
    let (mut session, path) = measure_session();
    session.selection.select_single(path);
    assert!(session.measure_enter());
    session.set_tool(Tool::Select);
    session.selection.clear();
    let shown = view(&session);
    let text = session.view().screen_to_document(
        shown["dimensions"][0]["x"].as_f64().unwrap(),
        shown["dimensions"][0]["y"].as_f64().unwrap(),
    );
    let steps = session.document.own_step_count();
    session.pointer_down(text, false);
    session.pointer_hover(Point::new(text.x + 10.0, text.y - 20.0), false, false);
    session.pointer_up(Point::new(text.x + 10.0, text.y - 20.0), false, false);
    assert_eq!(session.document.own_step_count(), steps + 1);
    assert_eq!(
        session.document.own_steps().last().unwrap().label,
        curvyo_document_core::MOVE_DIMENSION_COMMIT_LABEL
    );
    assert_eq!(session.selection.ids().len(), 1);
    let _ = view(&session);
    assert!(session.guide_marking().all_guides(), "Guide shows pressed");
    assert!(session.toggle_guide());
    assert_eq!(
        session.document.own_steps().last().unwrap().label,
        curvyo_document_core::CONVERT_DIMENSION_COMMIT_LABEL
    );
    assert_eq!(dimensions(&session).len(), 0);
    let group = session.selection.ids()[0];
    assert!(session.document.is_group(group));
}

#[test]
fn a_dimension_follows_its_path_in_the_drawn_objects() {
    let (mut session, path) = measure_session();
    session.selection.select_single(path);
    assert!(session.measure_enter());
    let moved: Vec<ObjectSnapshot> = session
        .document
        .read_objects()
        .objects
        .iter()
        .map(|o| {
            if o.id() == path {
                o.translated(Vec2::new(0.0, 30.0))
            } else {
                o.clone()
            }
        })
        .collect();
    let records = session.dimension_records(&moved);
    assert!((records[0].points[0].y - 40.0).abs() < 1e-9);
}

#[test]
fn m_is_the_measure_tool_except_in_select_with_a_selection() {
    let (mut session, path) = measure_session();
    session.set_tool(Tool::Select);
    let key = |key| KeyInput {
        key,
        shift: false,
        ctrl: false,
        alt: false,
        repeat: false,
        dom_blocked: false,
        mac: false,
        time_ms: 0.0,
    };
    session.key_down(key("m"));
    assert_eq!(session.tool(), Tool::Measure);
    session.set_tool(Tool::Select);
    session.selection.select_single(path);
    session.key_down(key("m"));
    assert_eq!(session.tool(), Tool::Select, "M opens the move entry there");
}

fn placed_and_unlinked() -> (Session, NodeId) {
    let (mut session, path) = measure_session();
    session.selection.select_single(path);
    assert!(session.measure_enter());
    let dimension = dimensions(&session)[0].id;
    session.set_tool(Tool::Select);
    session.selection.select_single(path);
    session.delete_selected();
    (session, dimension)
}

#[test]
fn the_node_tool_moves_a_kept_point_of_an_unlinked_dimension_as_edit_nodes() {
    let (mut session, dimension) = placed_and_unlinked();
    session.set_tool(Tool::Node);
    let steps = session.document.own_step_count();
    session.pointer_down(Point::new(60.5, 10.0), false);
    session.pointer_hover(Point::new(80.0, 30.0), false, false);
    let _ = session.draw_list();
    session.pointer_up(Point::new(80.0, 30.0), false, false);
    assert_eq!(session.document.own_step_count(), steps + 1);
    assert_eq!(
        session.document.own_steps().last().unwrap().label,
        curvyo_document_core::EDIT_DIMENSION_NODES_COMMIT_LABEL
    );
    let moved = session.document.dimension(dimension).unwrap();
    assert_eq!(moved.kept.points[1], Point::new(80.0, 30.0));
    assert!(
        (moved.shown.value.raw() - 50.0).abs() < 1e-9,
        "the number stays"
    );
}

#[test]
fn a_segment_under_the_pointer_shows_a_band_and_ctrl_shows_the_lock_badge() {
    let (mut session, _) = measure_session();
    session.pointer_hover(Point::new(25.0, 10.5), false, false);
    assert_eq!(
        session.measure_hover_band(),
        vec![Point::new(10.0, 10.0), Point::new(60.0, 10.0)]
    );
    session.pointer_hover(Point::new(10.0, 10.0), false, false);
    assert_eq!(
        session.measure_hover_band().len(),
        0,
        "a node shows the marker"
    );
    click(&mut session, Point::new(10.0, 10.0), false);
    assert!(!session.measure_angle_lock_badge());
    session.modifiers_changed(false, true, false);
    assert!(session.measure_angle_lock_badge());
}

#[test]
fn the_live_region_says_the_pinned_and_the_placed_value() {
    let (mut session, _) = measure_session();
    click(&mut session, Point::new(10.0, 10.0), false);
    session.pointer_hover(Point::new(60.0, 10.0), false, false);
    click(&mut session, Point::new(60.0, 10.0), false);
    assert_eq!(view(&session)["speech"], "50 millimetres, 0 degrees");
    click(&mut session, Point::new(10.0, 10.0), false);
    session.pointer_hover(Point::new(60.0, 10.0), true, false);
    click(&mut session, Point::new(60.0, 10.0), true);
    click(&mut session, Point::new(35.0, 30.0), false);
    assert_eq!(view(&session)["speech"], "Dimension 50 millimetres");
}

#[test]
fn a_selected_dimension_has_no_box_and_its_row_uses_the_display_unit() {
    let (mut session, path) = measure_session();
    session.selection.select_single(path);
    assert!(session.measure_enter());
    let dimension = dimensions(&session)[0].id;
    session.set_tool(Tool::Select);
    session.selection.select_single(dimension);
    let _ = session.draw_list();
    let objects = session.objects();
    let shown = session.without_selected_dimensions(std::borrow::Cow::Borrowed(&objects[..]));
    assert_eq!(session.select_decoration_input_in(&shown).selected.len(), 0);
    assert!(session.set_display_unit(DisplayUnit::Cm));
    let rows = session.layer_rows();
    let row = rows.row(dimension).unwrap();
    assert_eq!(row.name, "Dimension 5 cm");
    assert_eq!(session.selection_announcement(), "Dimension 5 cm");
}

#[test]
fn guide_converts_a_selected_dimension_with_no_frame_drawn() {
    let (mut session, path) = measure_session();
    session.selection.select_single(path);
    assert!(session.measure_enter());
    session.set_tool(Tool::Select);
    let id = dimensions(&session)[0].id;
    session.selection.set(&[id]);
    assert!(
        session.toggle_guide(),
        "converted without a frame in between"
    );
    assert_eq!(dimensions(&session).len(), 0);
    let group = session.selection.ids()[0];
    let children = session.document.children(Some(group));
    assert_eq!(
        children.len(),
        6,
        "three lines, two arrowheads and the text"
    );
}

#[test]
fn with_snapping_off_a_press_near_a_node_does_not_snap_and_alt_snaps() {
    let (mut session, _) = measure_session();
    let mut snapping = session.snapping();
    snapping.enabled = false;
    session.set_snapping(snapping);
    let near = Point::new(10.3, 10.2);
    session.pointer_hover(near, false, false);
    assert!(
        !matches!(
            session.measure_hover(),
            Some(MeasurePick::Point {
                slot: PointSlot::Anchor { .. },
                ..
            })
        ),
        "no snap to the node, the segment under the pointer is picked"
    );
    session.modifiers_changed(false, false, true);
    session.pointer_hover(near, false, false);
    assert!(matches!(
        session.measure_hover(),
        Some(MeasurePick::Point {
            slot: PointSlot::Anchor { .. },
            ..
        })
    ));
}

/// A path, another path and a dimension from the first path's start to a free point (30 mm),
/// with `selected` selected in the Select tool.
fn free_slot_dimension(selected: &[usize]) -> (Session, Vec<NodeId>) {
    let (mut session, path) = measure_session();
    let other = session.document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(9, 7), Point::new(10.0, 80.0)),
            NewAnchor::corner(AnchorId::new(9, 8), Point::new(60.0, 80.0)),
        ],
        false,
    );
    let dimension = session
        .document
        .add_dimension(
            DimensionKind::Aligned,
            DimensionSlots::Points(vec![
                PointSlot::Anchor {
                    node: path,
                    anchor: AnchorId::new(9, 1),
                },
                PointSlot::Free {
                    point: Point::new(10.0, 40.0),
                },
            ]),
            DimensionPlace::at_offset(Length::from_mm(5.0)),
        )
        .unwrap();
    let ids = vec![path, other, dimension];
    session.set_tool(Tool::Select);
    let chosen: Vec<NodeId> = selected.iter().map(|&i| ids[i]).collect();
    session.selection.set(&chosen);
    (session, ids)
}

/// The dimension's shown points while the selection is dragged by 20 mm down from `from`
/// inside its box, then after the release.
fn moved_points(session: &mut Session, from: Point) -> (Vec<Point>, Vec<Point>) {
    let to = Point::new(from.x, from.y + 20.0);
    session.pointer_hover(from, false, false);
    session.pointer_down(from, false);
    session.pointer_hover(Point::new(from.x, from.y + 10.0), false, false);
    session.pointer_hover(to, false, false);
    assert!(
        session
            .select
            .live_move_snapped(to, false, false, Vec2::ZERO)
            .is_some(),
        "a move is in flight"
    );
    let objects = session.objects();
    let live = session.select_live_edit_in(&objects);
    let mut drawn = Session::live_objects_in(&objects, live.as_ref()).into_owned();
    curvyo_document_core::resolve_dimensions_in(&mut drawn);
    let shown = |objects: &[ObjectSnapshot]| {
        objects
            .iter()
            .find_map(|o| match o {
                ObjectSnapshot::Dimension(d) => Some(d.shown.points.clone()),
                _ => None,
            })
            .unwrap()
    };
    let during = shown(&drawn);
    session.pointer_up(to, false, false);
    (during, shown(&session.objects()))
}

fn close(a: &[Point], b: &[Point]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(p, q)| p.vector_to(*q).length() < 1e-6)
}

#[test]
fn a_linked_dimension_with_a_free_slot_previews_what_the_release_saves() {
    // Moved with another object but without its path: it stays, in the preview too.
    let (mut session, _) = free_slot_dimension(&[1, 2]);
    let (during, after) = moved_points(&mut session, Point::new(35.0, 45.0));
    assert!(close(&during, &after), "{during:?} vs {after:?}");
    assert!(close(
        &after,
        &[Point::new(10.0, 10.0), Point::new(10.0, 40.0)]
    ));
    // Moved with its path: the free slot moves too, in the preview and the commit alike.
    let (mut session, _) = free_slot_dimension(&[0, 2]);
    let (during, after) = moved_points(&mut session, Point::new(35.0, 25.0));
    assert!(close(&during, &after), "{during:?} vs {after:?}");
    assert!(after[0].y > 25.0, "the path moved: {after:?}");
    assert!((after[1].y - after[0].y - 30.0).abs() < 1e-9, "{after:?}");
}

#[test]
fn a_resize_of_a_selection_with_a_dimension_maps_its_free_slot_in_preview_and_commit() {
    // The path (y 10) and the dimension to the free point (10, 40): the box is 10..40 high.
    let (mut session, ids) = free_slot_dimension(&[0, 2]);
    let (from, to) = (Point::new(35.0, 40.0), Point::new(35.0, 70.0));
    session.pointer_hover(from, false, false);
    session.pointer_down(from, false);
    session.pointer_hover(Point::new(35.0, 55.0), false, false);
    session.pointer_hover(to, false, false);
    let objects = session.objects();
    let live = session.select_live_edit_in(&objects);
    assert!(live.is_some(), "a resize is in flight");
    let mut drawn = Session::live_objects_in(&objects, live.as_ref()).into_owned();
    curvyo_document_core::resolve_dimensions_in(&mut drawn);
    let free_of = |objects: &[ObjectSnapshot]| {
        objects
            .iter()
            .find_map(|o| match o {
                ObjectSnapshot::Dimension(d) => match &d.slots {
                    DimensionSlots::Points(points) => Some((points.clone(), d.shown.clone())),
                    DimensionSlots::Curve(_) => None,
                },
                _ => None,
            })
            .unwrap()
    };
    let during = free_of(&drawn);
    session.pointer_up(to, false, false);
    let after = free_of(&session.objects());
    assert_eq!(during, after, "the preview is what the release writes");
    assert!(matches!(after.0[0], PointSlot::Anchor { node, .. } if node == ids[0]));
    let PointSlot::Free { point } = after.0[1] else {
        panic!("{after:?}")
    };
    assert!(
        point.y > 60.0,
        "the free slot is scaled with the box: {point:?}"
    );
}
