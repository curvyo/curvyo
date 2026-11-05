//! Pick mask paths in the Trace tool (`specs/0094-masked-composite-trace` slice 1b, criteria 34
//! to 41).

use curvyo_document_core::{
    AnchorId, AnchorKind, Angle, EllipseFrame, InnerRatio, Length, NewAnchor, NodeId,
    ObjectSnapshot, Point, PointCount, RectBounds, StarFrame, Vec2,
};
use curvyo_ui_core::oriented_bounds;

use super::{Session, Tool, TraceSelection};
use crate::session::test_pages::PageKinds as _;

const PNG: &[u8] =
    include_bytes!("../../../curvyo-image-core/tests/fixtures/images/png_rgba_8.png");

fn import_images(session: &mut Session, count: usize) -> Vec<NodeId> {
    session.import_begin(None);
    for i in 0..count {
        session.import_add(PNG, &format!("logo-{i}.png"));
    }
    session.import_finish(false);
    session
        .objects()
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Image(image) => Some(image.id),
            _ => None,
        })
        .collect()
}

/// The centre of `id`'s box.
fn centre(session: &Session, id: NodeId) -> Point {
    let object = session
        .objects()
        .iter()
        .find(|o| o.id() == id)
        .unwrap()
        .clone();
    let c = oriented_bounds(&object).document_corners();
    Point::new(f64::midpoint(c[0].x, c[2].x), f64::midpoint(c[0].y, c[2].y))
}

fn rect(session: &Session, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    session.document.create_rect(RectBounds {
        origin: Point::new(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

fn click(session: &mut Session, point: Point) {
    session.pointer_down(point, false);
    session.pointer_up(point, false, false);
}

/// The Trace tool on one image, nothing else selected; the image's centre.
fn traced() -> (Session, NodeId, Point) {
    let mut session = Session::new(1);
    let image = import_images(&mut session, 1)[0];
    session.selection.set(&[image]);
    session.set_tool(Tool::Trace);
    let at = centre(&session, image);
    (session, image, at)
}

/// A rectangle over the image whose left edge, 5 mm left of the image's centre, is hit outside it.
fn rect_over(session: &Session, at: Point) -> (NodeId, Point) {
    let id = rect(session, at.x - 5.0, at.y - 5.0, 10.0, 10.0);
    (id, Point::new(at.x - 5.0, at.y))
}

#[test]
fn c34_c35_the_mode_is_tool_state_and_tells_the_cursor() {
    let (mut session, image, _) = traced();
    assert!(!session.trace_mask_pick(), "disarmed on entry");
    assert_eq!(session.cursor_hint(), "default");
    session.set_trace_mask_pick(true);
    assert!(session.trace_mask_pick());
    assert_eq!(session.cursor_hint(), "pick-mask");
    session.set_trace_mask_pick(false);
    assert_eq!(session.cursor_hint(), "default", "the tool's own cursor");
    assert_eq!(session.trace_view().mask, []);
    assert_eq!(session.selection.ids(), [image]);
    // Only the Trace tool arms it.
    session.set_tool(Tool::Select);
    session.set_trace_mask_pick(true);
    assert!(!session.trace_mask_pick());
}

#[test]
fn c36_a_pick_adds_a_second_removes_and_the_mode_stays() {
    let (mut session, image, at) = traced();
    let (path, edge) = rect_over(&session, at);
    session.set_trace_mask_pick(true);
    click(&mut session, edge);
    let view = session.trace_view();
    assert_eq!(view.selection, TraceSelection::Images);
    assert_eq!(view.mask, [path]);
    assert_eq!(session.selection.ids().len(), 2);
    assert!(session.selection.contains(image) && session.selection.contains(path));
    assert!(
        session.trace_region().is_some(),
        "the preview is traced inside it"
    );
    assert!(!session.trace_mask_outside(), "Inside");
    assert!(session.trace_mask_pick(), "still armed");
    click(&mut session, edge);
    assert_eq!(session.trace_view().mask, []);
    assert_eq!(session.selection.ids(), [image]);
    assert!(
        session.trace_mask_pick(),
        "still armed after the last one is removed"
    );
}

/// Each accepted kind gives the region the Select route gives for the same object.
#[test]
fn c37_every_path_like_kind_and_a_group_give_the_select_routes_mask() {
    type Make = fn(&mut Session, Point) -> (Vec<NodeId>, Point);
    let kinds: [(&str, Make); 7] = [
        ("open path", |s, at| {
            let anchors = [
                Point::new(at.x - 5.0, at.y - 5.0),
                Point::new(at.x + 5.0, at.y),
                Point::new(at.x - 5.0, at.y + 5.0),
            ]
            .map(|point| NewAnchor {
                id: AnchorId::new(9, 1),
                point,
                handle_in: Vec2::ZERO,
                handle_out: Vec2::ZERO,
                kind: AnchorKind::Corner,
            });
            let id = s.document.create_path(&anchors, false);
            (vec![id], Point::new(at.x - 5.0, at.y - 5.0))
        }),
        ("compound path", |s, at| {
            let a = rect(s, at.x - 5.0, at.y - 5.0, 10.0, 10.0);
            let b = rect(s, at.x + 20.0, at.y - 5.0, 3.0, 3.0);
            s.set_tool(Tool::Select);
            s.selection.set(&[a, b]);
            let _ = s.apply_combine();
            let id = s.selection.ids()[0];
            s.set_tool(Tool::Trace);
            (vec![id], Point::new(at.x - 5.0, at.y))
        }),
        ("rectangle", |s, at| {
            let (id, edge) = rect_over(s, at);
            (vec![id], edge)
        }),
        ("ellipse", |s, at| {
            let id = s.document.create_ellipse(EllipseFrame {
                center: at,
                rx: Length::from_mm(5.0),
                ry: Length::from_mm(5.0),
            });
            (vec![id], Point::new(at.x - 5.0, at.y))
        }),
        ("polygon", |s, at| {
            let id = s.document.create_polygon(
                StarFrame {
                    center: at,
                    radius: Length::from_mm(5.0),
                    angle: Angle::from_radians(std::f64::consts::PI),
                },
                PointCount::new(6).unwrap(),
            );
            (vec![id], Point::new(at.x - 5.0, at.y))
        }),
        ("star", |s, at| {
            let id = s.document.create_star(
                StarFrame {
                    center: at,
                    radius: Length::from_mm(5.0),
                    angle: Angle::from_radians(std::f64::consts::PI),
                },
                PointCount::new(5).unwrap(),
                InnerRatio::new(0.5).unwrap(),
            );
            (vec![id], Point::new(at.x - 5.0, at.y))
        }),
        ("group", |s, at| {
            let a = rect(s, at.x - 5.0, at.y - 5.0, 6.0, 10.0);
            let b = rect(s, at.x + 2.0, at.y - 5.0, 3.0, 10.0);
            s.set_tool(Tool::Select);
            s.selection.set(&[a, b]);
            let _ = s.group_selection();
            let leaves = s.selection.ids().to_vec();
            s.set_tool(Tool::Trace);
            (leaves, Point::new(at.x - 5.0, at.y))
        }),
    ];
    for (name, make) in kinds {
        let (mut session, image, at) = traced();
        let (ids, hit) = make(&mut session, at);
        // The Select route: the image and the object selected together.
        let mut chosen = vec![image];
        chosen.extend(&ids);
        session.selection.set(&chosen);
        let by_select = (session.trace_view().mask, session.trace_mask_json());
        assert_ne!(by_select.1, "", "{name}: the Select route makes a region");
        session.selection.set(&[image]);
        session.set_trace_mask_pick(true);
        click(&mut session, hit);
        assert_eq!(
            (session.trace_view().mask, session.trace_mask_json()),
            by_select,
            "{name}: the pick gives the Select route's region"
        );
        assert!(!session.trace_mask_pick_refused(), "{name}");
    }
}

#[test]
fn c38_a_text_and_a_second_image_are_refused_and_the_image_does_nothing() {
    let mut session = super::text_tests::with_text("Name");
    let images = import_images(&mut session, 2);
    let text = session
        .objects()
        .iter()
        .find_map(|o| matches!(o, ObjectSnapshot::Text(_)).then(|| o.id()))
        .unwrap();
    // The second image away from the first, so a click on it hits it.
    let first = centre(&session, images[0]);
    session.set_tool(Tool::Select);
    let from = centre(&session, images[1]);
    let to = Point::new(from.x + 40.0, from.y + 40.0);
    session.pointer_down(from, false);
    session.pointer_hover(to, false, false);
    session.pointer_up(to, false, false);
    assert!(centre(&session, images[1]).x > first.x + 30.0, "moved");
    session.selection.set(&[images[0]]);
    session.set_tool(Tool::Trace);
    session.set_trace_mask_pick(true);
    for (name, at) in [
        ("text", centre(&session, text)),
        ("second image", centre(&session, images[1])),
    ] {
        click(&mut session, at);
        assert_eq!(session.selection.ids(), [images[0]], "{name}: not selected");
        assert!(session.trace_mask_pick_refused(), "{name}: the row");
        assert!(session.trace_mask_pick(), "{name}: still armed");
        assert_eq!(session.trace_view().selection, TraceSelection::Images);
    }
    // The next accepted pick clears the row.
    let (path, edge) = rect_over(&session, first);
    click(&mut session, edge);
    assert_eq!(session.trace_view().mask, [path]);
    assert!(!session.trace_mask_pick_refused());
    // The traced image and empty canvas change nothing.
    let before = session.selection.ids().to_vec();
    click(&mut session, first);
    assert_eq!(session.selection.ids(), before, "the image");
    click(&mut session, Point::new(first.x - 300.0, first.y - 300.0));
    assert_eq!(session.selection.ids(), before, "empty canvas");
    // A double-click on the image creates no paths while armed.
    assert_eq!(
        session.double_click_hint(first, false, false),
        super::select::DoubleClickHint::None
    );
    session.set_trace_mask_pick(false);
    assert_eq!(
        session.double_click_hint(first, false, false),
        super::select::DoubleClickHint::CreateTrace
    );
}

#[test]
fn c38_disarming_clears_the_refused_row() {
    let mut session = super::text_tests::with_text("Name");
    let image = import_images(&mut session, 1)[0];
    let text = session
        .objects()
        .iter()
        .find_map(|o| matches!(o, ObjectSnapshot::Text(_)).then(|| o.id()))
        .unwrap();
    session.selection.set(&[image]);
    session.set_tool(Tool::Trace);
    session.set_trace_mask_pick(true);
    let at = centre(&session, text);
    click(&mut session, at);
    assert!(session.trace_mask_pick_refused());
    session.set_trace_mask_pick(false);
    assert!(!session.trace_mask_pick_refused());
}

#[test]
fn c39_ending_keeps_the_mask_and_leaving_the_tool_or_creating_disarms() {
    let (mut session, image, at) = traced();
    let (path, edge) = rect_over(&session, at);
    session.set_trace_mask_pick(true);
    click(&mut session, edge);
    session.set_trace_mask_pick(false);
    assert_eq!(session.trace_view().mask, [path], "the mask is kept");
    assert_eq!(session.selection.ids().len(), 2);
    session.set_trace_mask_pick(true);
    session.set_tool(Tool::Select);
    session.set_tool(Tool::Trace);
    assert!(!session.trace_mask_pick(), "not armed on entry");
    // Create paths ends the tool by the same `set_tool(Tool::Select)` (`trace.rs`).
    session.set_trace_mask_pick(true);
    session.selection.set(&[image]);
    session.set_tool(Tool::Select);
    session.set_tool(Tool::Trace);
    assert!(!session.trace_mask_pick());
}

#[test]
fn c40_clear_mask_keeps_the_paths_the_mode_and_writes_no_history() {
    let (mut session, image, at) = traced();
    let ids: Vec<NodeId> = (0..3)
        .map(|i| rect(&session, at.x - 5.0 - f64::from(i), at.y - 5.0, 10.0, 10.0))
        .collect();
    let mut chosen = vec![image];
    chosen.extend(&ids);
    session.selection.set(&chosen);
    assert_eq!(session.trace_view().mask.len(), 3);
    session.set_trace_mask_pick(true);
    let objects = session.objects().to_vec();
    let steps = session.history_len();
    session.clear_trace_mask();
    assert_eq!(session.selection.ids(), [image], "only the image");
    assert_eq!(session.trace_view().mask, []);
    assert_eq!(
        &*session.objects(),
        objects.as_slice(),
        "the paths are unchanged"
    );
    assert_eq!(session.history_len(), steps, "no history step");
    assert!(session.trace_mask_pick(), "the armed state is unchanged");
    session.set_trace_mask_pick(false);
    session.selection.set(&chosen);
    session.clear_trace_mask();
    assert!(!session.trace_mask_pick());
}

#[test]
fn c41_shift_click_and_the_pick_give_the_same_mask() {
    let (mut session, image, at) = traced();
    let (_, edge) = rect_over(&session, at);
    session.pointer_down(edge, true);
    session.pointer_up(edge, true, false);
    let by_shift = (session.selection.ids().to_vec(), session.trace_mask_json());
    session.selection.set(&[image]);
    session.set_trace_mask_pick(true);
    click(&mut session, edge);
    assert_eq!(
        (session.selection.ids().to_vec(), session.trace_mask_json()),
        by_shift
    );
}

/// UX notes §11.4: while armed, the candidate under the pointer gets the hover box; a refused
/// object does not.
#[test]
fn the_candidate_under_the_pointer_gets_the_hover_box() {
    let mut session = super::text_tests::with_text("Name");
    let image = import_images(&mut session, 1)[0];
    let text = session
        .objects()
        .iter()
        .find_map(|o| matches!(o, ObjectSnapshot::Text(_)).then(|| o.id()))
        .unwrap();
    session.selection.set(&[image]);
    session.set_tool(Tool::Trace);
    let (path, edge) = rect_over(&session, centre(&session, image));
    session.pointer_hover(edge, false, false);
    assert!(
        session.select_decoration_input().hovered.is_none(),
        "disarmed"
    );
    session.set_trace_mask_pick(true);
    session.pointer_hover(edge, false, false);
    assert_eq!(
        session.select_decoration_input().hovered.map(|(id, _)| id),
        Some(path)
    );
    session.pointer_hover(centre(&session, text), false, false);
    assert!(
        session.select_decoration_input().hovered.is_none(),
        "a text"
    );
}

/// The pipette and the pick exclude each other (plan, Proposal).
#[test]
fn the_pipette_and_the_pick_exclude_each_other() {
    let (mut session, _, _) = traced();
    session.set_trace_mask_pick(true);
    session.set_trace_pick(Some(super::TracePickMode::Colours));
    assert!(!session.trace_mask_pick());
    session.set_trace_mask_pick(true);
    assert_eq!(session.trace_pick_mode(), None);
}

/// Criterion 38: a sketch is refused like a text.
#[test]
fn c38_a_sketch_is_refused() {
    use curvyo_sketch_core::{Construction, Length as SketchLength, Position};
    let mut session = Session::new(1);
    let image = import_images(&mut session, 1)[0];
    let at = centre(&session, image);
    session.to_sketch_page();
    let sketch = session.new_sketch().unwrap();
    session
        .sketch_construct(&Construction::Circle {
            center: Position::mm(at.x + 30.0, at.y),
            radius: SketchLength(5.0),
        })
        .unwrap();
    session.to_drawing_page();
    let edge = session
        .objects()
        .iter()
        .find(|o| o.id() == sketch)
        .map(|o| {
            let c = oriented_bounds(o).document_corners();
            Point::new(c[0].x, f64::midpoint(c[0].y, c[2].y))
        })
        .unwrap();
    session.selection.set(&[image]);
    session.set_tool(Tool::Trace);
    session.set_trace_mask_pick(true);
    click(&mut session, edge);
    assert_eq!(session.selection.ids(), [image]);
    assert!(session.trace_mask_pick_refused());
}
