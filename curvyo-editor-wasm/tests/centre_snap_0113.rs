//! `specs/0113-fixture-sets-and-pins` criterion 30 as amended after the demo (2026-11-16): a job
//! fixture's centre snaps to the centres of circles and rectangles while the Centres target is
//! on, whatever the Objects target says, also to the objects a laser page shows through a placed
//! drawing page, with the `0060` marker; and nothing snaps while Centres is off.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    Affine, Document, DocumentSize, DocumentSnapping, EllipseFrame, Length, NewOutputPage, Point,
    RectBounds, Vec2, pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool};

/// A drawing page holding a circle (r 10 at 30, 30) and a rectangle (60 to 100, 20 to 40), its
/// local mm; a laser page of 400 x 300 that shows it through a placement moved by (50, 60) when
/// `placed`, else holding the same two shapes as its own items. Returns the session on the laser
/// page.
fn session(placed: bool) -> Session {
    let d = Document::new(1);
    let drawing = d.pages()[0].clone();
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let laser_page = d.pages().into_iter().find(|p| p.id == laser.id).unwrap();
    let o = if placed {
        drawing.origin
    } else {
        laser_page.origin
    };
    let _ = d.create_ellipse(EllipseFrame {
        center: Point::new(o.x + 30.0, o.y + 30.0),
        rx: Length::from_mm(10.0),
        ry: Length::from_mm(10.0),
    });
    let _ = d.create_rect(RectBounds::from_corners(
        Point::new(o.x + 60.0, o.y + 20.0),
        Point::new(o.x + 100.0, o.y + 40.0),
    ));
    if placed {
        d.place_page(
            &laser.id,
            &drawing.id,
            Affine::translation(Vec2::new(50.0, 60.0)),
        )
        .unwrap();
    }
    let mut s = Session::open(9030, &pack(&d, "centres").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    assert!(s.activate_page(&laser.id));
    s
}

fn snapping(s: &mut Session, objects: bool, centres: bool) {
    s.set_snapping(DocumentSnapping {
        enabled: true,
        objects,
        centres,
        ..DocumentSnapping::DEFAULT
    });
}

/// A point of the laser page from work-area mm.
fn at(s: &Session, x: f64, y: f64) -> Point {
    let o = s.active_page().origin;
    Point::new(o.x + x, o.y + y)
}

/// Arms a pin, hovers at the work-area `(x, y)` and drops it there; its centre in work-area mm.
fn drop_pin(s: &mut Session, x: f64, y: f64) -> Point {
    assert!(s.arm_job_fixture("pin"));
    let p = at(s, x, y);
    s.pointer_hover(p, false, false);
    let id = s.drop_fixture_placement().expect("a dropped pin");
    s.end_fixture_placement();
    local_centre(s, id)
}

/// The centre of the fixture `id` in work-area mm (its stored matrix is work-area local).
fn local_centre(s: &Session, id: curvyo_document_core::NodeId) -> Point {
    let stored = unpack(9031, &s.pack("centres").unwrap()).unwrap();
    let m = stored
        .fixture(id)
        .expect("a stored fixture")
        .matrix
        .expect("a matrix");
    Point::new(m.e, m.f)
}

fn near(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6
}

#[test]
fn a_pin_snaps_to_the_centre_of_a_circle_and_of_a_rectangle_on_the_laser_page() {
    let mut s = session(false);
    snapping(&mut s, true, true);
    let circle = drop_pin(&mut s, 30.6, 29.6);
    assert!(near(circle, Point::new(30.0, 30.0)), "circle: {circle:?}");
    let rect = drop_pin(&mut s, 80.5, 30.4);
    assert!(near(rect, Point::new(80.0, 30.0)), "rectangle: {rect:?}");
}

#[test]
fn a_pin_snaps_to_the_centres_a_placed_drawing_page_shows() {
    let mut s = session(true);
    snapping(&mut s, true, true);
    let circle = drop_pin(&mut s, 80.6, 89.6);
    assert!(
        near(circle, Point::new(80.0, 90.0)),
        "the placed circle's centre: {circle:?}"
    );
    let rect = drop_pin(&mut s, 130.5, 90.4);
    assert!(
        near(rect, Point::new(130.0, 90.0)),
        "the placed rectangle's centre: {rect:?}"
    );
}

#[test]
fn centres_is_its_own_target_independent_of_objects() {
    let mut s = session(false);
    snapping(&mut s, false, true);
    let on = drop_pin(&mut s, 30.6, 29.6);
    assert!(near(on, Point::new(30.0, 30.0)), "Objects off, Centres on");
    snapping(&mut s, true, false);
    let off = drop_pin(&mut s, 30.6, 29.6);
    assert!(
        near(off, Point::new(30.6, 29.6)),
        "Centres off: no snap: {off:?}"
    );
}

#[test]
fn a_dragged_job_fixture_snaps_to_a_centre_and_shows_the_marker() {
    let mut s = session(false);
    snapping(&mut s, true, true);
    let start = drop_pin(&mut s, 200.0, 200.0);
    let pin = s.fixtures_view().unwrap().job_fixtures[0].id.clone();
    let pin = s.fixture_part_named(&pin).unwrap();
    assert!(s.select_fixture(pin));
    let from = at(&s, start.x, start.y);
    s.pointer_down(from, false);
    s.pointer_hover(at(&s, 120.0, 120.0), false, false);
    let far = s.frame_draw_list().triangle_count();
    let to = at(&s, 30.5, 30.6);
    s.pointer_hover(to, false, false);
    let snapped = s.frame_draw_list().triangle_count();
    assert!(
        snapped > far,
        "the snap marker is drawn: {far} -> {snapped}"
    );
    s.pointer_up(to, false, false);
    assert!(near(local_centre(&s, pin), Point::new(30.0, 30.0)));
}
