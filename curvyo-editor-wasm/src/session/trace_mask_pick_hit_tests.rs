//! The hit rule of Pick mask paths and of Shift+click in the Trace tool inside the traced image's
//! frame (`specs/0094-masked-composite-trace` criteria 13, 35 to 38 and 41): the image is never
//! hit, the topmost mask-eligible object under the pointer is, above or below the image, a closed
//! one by its interior.

use curvyo_document_core::{
    AnchorId, AnchorKind, EllipseFrame, Length, NewAnchor, NodeId, ObjectSnapshot, Point,
    RectBounds, Vec2,
};
use curvyo_ui_core::oriented_bounds;

use super::{Session, Tool};

/// A white PNG of 400 x 400 px, so the image covers every test object below.
fn photo() -> Vec<u8> {
    use image::ImageEncoder;
    let rgba = vec![255; 400 * 400 * 4];
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&rgba, 400, 400, image::ExtendedColorType::Rgba8)
        .unwrap();
    out
}

fn import(session: &mut Session) -> NodeId {
    session.import_begin(None);
    session.import_add(&photo(), "photo.png");
    session.import_finish(false);
    session
        .objects()
        .iter()
        .rev()
        .find_map(|o| matches!(o, ObjectSnapshot::Image(_)).then(|| o.id()))
        .unwrap()
}

/// The image's box as (low, high).
fn frame(session: &Session, id: NodeId) -> (Point, Point) {
    let object = session
        .objects()
        .iter()
        .find(|o| o.id() == id)
        .unwrap()
        .clone();
    let c = oriented_bounds(&object).document_corners();
    (
        Point::new(c[0].x.min(c[2].x), c[0].y.min(c[2].y)),
        Point::new(c[0].x.max(c[2].x), c[0].y.max(c[2].y)),
    )
}

/// An unfilled circle (an ellipse primitive has the creation style: stroke, no fill).
fn circle(session: &Session, center: Point, r: f64) -> NodeId {
    session.document.create_ellipse(EllipseFrame {
        center,
        rx: Length::from_mm(r),
        ry: Length::from_mm(r),
    })
}

fn open_path(session: &Session, points: &[Point]) -> NodeId {
    let anchors: Vec<NewAnchor> = points
        .iter()
        .map(|&point| NewAnchor {
            id: AnchorId::new(9, 1),
            point,
            handle_in: Vec2::ZERO,
            handle_out: Vec2::ZERO,
            kind: AnchorKind::Corner,
        })
        .collect();
    session.document.create_path(&anchors, false)
}

fn click(session: &mut Session, point: Point) {
    session.pointer_down(point, false);
    session.pointer_up(point, false, false);
}

fn shift_click(session: &mut Session, point: Point) {
    session.pointer_down(point, true);
    session.pointer_up(point, true, false);
}

/// Enters the Trace tool on `image` alone and arms the pick.
fn armed(session: &mut Session, image: NodeId) {
    session.selection.set(&[image]);
    session.set_tool(Tool::Trace);
    session.set_trace_mask_pick(true);
}

fn assert_inside(session: &Session, image: NodeId, at: Point) {
    let (low, high) = frame(session, image);
    assert!(
        at.x > low.x && at.x < high.x && at.y > low.y && at.y < high.y,
        "the click lies inside the image's frame"
    );
}

/// The image's centre.
fn middle(session: &Session, image: NodeId) -> Point {
    let (low, high) = frame(session, image);
    Point::new(f64::midpoint(low.x, high.x), f64::midpoint(low.y, high.y))
}

#[test]
fn a_circle_above_the_image_is_picked_inside_it_and_a_second_click_removes_it() {
    let mut session = Session::new(1);
    let image = import(&mut session);
    let at = middle(&session, image);
    let path = circle(&session, at, 10.0);
    assert!(
        session
            .objects()
            .iter()
            .find(|o| o.id() == path)
            .is_some_and(|o| !o.style().fill.paints()),
        "outline only"
    );
    armed(&mut session, image);
    assert_inside(&session, image, at);
    session.pointer_hover(at, false, false);
    assert_eq!(
        session.select_decoration_input().hovered.map(|(id, _)| id),
        Some(path),
        "the hover box marks the circle"
    );
    click(&mut session, at);
    assert_eq!(session.trace_view().mask, [path]);
    assert!(session.selection.contains(image));
    click(&mut session, at);
    assert_eq!(session.trace_view().mask, []);
    assert_eq!(session.selection.ids(), [image]);
}

/// Where an import into a new session puts the image's centre.
fn import_centre() -> Point {
    let mut probe = Session::new(1);
    let image = import(&mut probe);
    middle(&probe, image)
}

#[test]
fn a_path_below_the_image_is_picked_on_its_stroke_and_inside() {
    let at = import_centre();
    let mut session = Session::new(1);
    // The path first, so the image is drawn over it.
    let path = circle(&session, at, 10.0);
    let image = import(&mut session);
    let ids: Vec<NodeId> = session.objects().iter().map(ObjectSnapshot::id).collect();
    assert!(
        ids.iter().position(|&id| id == path) < ids.iter().position(|&id| id == image),
        "the path lies below the image"
    );
    assert_eq!(middle(&session, image), at);
    armed(&mut session, image);
    let edge = Point::new(at.x - 10.0, at.y);
    assert_inside(&session, image, edge);
    click(&mut session, edge);
    assert_eq!(session.trace_view().mask, [path], "on its stroke");
    click(&mut session, at);
    assert_eq!(session.trace_view().mask, [], "inside it, removed");
}

#[test]
fn an_open_path_counts_by_its_stroke_only() {
    let mut session = Session::new(1);
    let image = import(&mut session);
    let at = middle(&session, image);
    let path = open_path(
        &session,
        &[
            Point::new(at.x - 10.0, at.y - 10.0),
            Point::new(at.x + 10.0, at.y),
            Point::new(at.x - 10.0, at.y + 10.0),
        ],
    );
    armed(&mut session, image);
    click(&mut session, at);
    assert_eq!(session.trace_view().mask, [], "inside the chord: nothing");
    assert!(!session.trace_mask_pick_refused());
    click(&mut session, Point::new(at.x - 10.0, at.y - 10.0));
    assert_eq!(session.trace_view().mask, [path], "on the stroke");
}

#[test]
fn of_two_overlapping_circles_the_topmost_wins() {
    let mut session = Session::new(1);
    let image = import(&mut session);
    let at = middle(&session, image);
    let below = circle(&session, at, 20.0);
    let above = circle(&session, Point::new(at.x + 5.0, at.y), 10.0);
    armed(&mut session, image);
    click(&mut session, at);
    assert_eq!(session.trace_view().mask, [above]);
    click(&mut session, at);
    assert_eq!(session.trace_view().mask, [], "the second click removes it");
    click(&mut session, Point::new(at.x - 15.0, at.y));
    assert_eq!(session.trace_view().mask, [below], "only in the lower one");
}

#[test]
fn a_path_partly_outside_the_image_is_picked_inside_and_outside_the_frame() {
    let mut session = Session::new(1);
    let image = import(&mut session);
    let (low, _) = frame(&session, image);
    let at = middle(&session, image);
    let centre = Point::new(low.x, at.y);
    let path = circle(&session, centre, 10.0);
    armed(&mut session, image);
    let inside = Point::new(low.x + 5.0, at.y);
    assert_inside(&session, image, inside);
    click(&mut session, inside);
    assert_eq!(session.trace_view().mask, [path]);
    click(&mut session, Point::new(low.x - 5.0, at.y));
    assert_eq!(session.trace_view().mask, []);
}

/// The centre and width of a text's box.
fn text_box(text: &curvyo_document_core::TextSnapshot) -> (Point, f64) {
    let c = oriented_bounds(&ObjectSnapshot::Text(Box::new(text.clone()))).document_corners();
    (
        Point::new(f64::midpoint(c[0].x, c[2].x), f64::midpoint(c[0].y, c[2].y)),
        (c[2].x - c[0].x).abs(),
    )
}

#[test]
fn a_text_above_a_path_does_not_stop_the_pick_and_a_text_alone_is_refused() {
    let mut session = super::text_tests::with_text("Name");
    let text = super::text_tests::texts(&session).remove(0);
    let image = import(&mut session);
    let at = middle(&session, image);
    let (from, width) = text_box(&text);
    // A circle round the image's centre, then a copy of the text centred on it, over the circle.
    let path = circle(&session, at, width);
    let placement = curvyo_document_core::Affine {
        e: text.placement.e + at.x - from.x,
        f: text.placement.f + at.y - from.y,
        ..text.placement
    };
    session
        .document
        .create_text(&text.string, &text.font, text.size, placement, &text.layout)
        .unwrap();
    let copy = super::text_tests::texts(&session).remove(1);
    let (centre, _) = text_box(&copy);
    assert!((centre.x - at.x).abs() < 1e-6 && (centre.y - at.y).abs() < 1e-6);
    armed(&mut session, image);
    assert_inside(&session, image, at);
    click(&mut session, at);
    assert_eq!(session.trace_view().mask, [path], "the path under the text");
    assert!(!session.trace_mask_pick_refused());
    // Without the path the text is the topmost hit besides the image: refused.
    session.selection.set(&[image]);
    session.document.delete_objects(&[path]).unwrap();
    click(&mut session, at);
    assert_eq!(session.selection.ids(), [image]);
    assert!(session.trace_mask_pick_refused());
}

#[test]
fn the_image_and_empty_canvas_do_nothing() {
    let mut session = Session::new(1);
    let image = import(&mut session);
    let at = middle(&session, image);
    let (low, _) = frame(&session, image);
    armed(&mut session, image);
    click(&mut session, at);
    assert_eq!(session.selection.ids(), [image], "the image");
    click(&mut session, Point::new(low.x - 100.0, low.y - 100.0));
    assert_eq!(session.selection.ids(), [image], "empty canvas");
    assert!(!session.trace_mask_pick_refused());
}

/// Shift+click had the same defect: inside the frame it toggled the image, not the path.
#[test]
fn shift_click_inside_the_frame_toggles_the_path_not_the_image() {
    let mut session = Session::new(1);
    let image = import(&mut session);
    let at = middle(&session, image);
    let path = circle(&session, at, 10.0);
    session.selection.set(&[image]);
    session.set_tool(Tool::Trace);
    shift_click(&mut session, at);
    assert_eq!(session.trace_view().mask, [path]);
    assert!(session.selection.contains(image));
    shift_click(&mut session, at);
    assert_eq!(session.selection.ids(), [image]);
    // A rectangle drawn under the image is reached too.
    let at = import_centre();
    let mut session = Session::new(1);
    let rect = session.document.create_rect(RectBounds {
        origin: Point::new(at.x - 5.0, at.y - 5.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    });
    let image = import(&mut session);
    session.selection.set(&[image]);
    session.set_tool(Tool::Trace);
    shift_click(&mut session, at);
    assert_eq!(session.trace_view().mask, [rect]);
}
