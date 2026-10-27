//! The drag preview of the Select tool after the customer's report of 2026-10-27
//! (`specs/0009-unified-object-editing`, note of 2026-10-27): the moved object is drawn as
//! itself at the pointer, its committed outline stays behind as the ghost, and Escape restores
//! the frame from before the drag. Driven through `Session`'s public API only.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::sync::Arc;

use curvyo_document_core::{
    Affine, Document, Length, NewImage, ObjectSnapshot, Point, RectBounds, Vec2, pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_render_core::{DrawList, build_artwork, build_drag_ghost, build_live_edit_preview};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn open(document: &Document) -> Session {
    let mut session = Session::open(2, &pack(document, "t").unwrap()).unwrap();
    session.resize_viewport(1600.0, 1000.0);
    session.set_tool(Tool::Select);
    session
}

fn first_object(session: &Session) -> ObjectSnapshot {
    let document = unpack(99, &session.pack("t").unwrap()).unwrap();
    document.object(document.object_ids()[0]).unwrap()
}

/// Whether every vertex of `part` is in `frame` with its colour and position.
fn contains(frame: &DrawList, part: &DrawList) -> bool {
    part.triangles.iter().all(|v| {
        frame.triangles.iter().any(|w| {
            w.color == v.color
                && (w.position.x - v.position.x).abs() < 1e-6
                && (w.position.y - v.position.y).abs() < 1e-6
        })
    })
}

/// A 100 x 60 mm rectangle at (10, 20), selected, pressed on its top edge and moved by
/// (20, 10) mm with Ctrl as given. Returns the session mid-drag and the committed object.
fn rect_moved(ctrl: bool) -> (Session, ObjectSnapshot, DrawList) {
    let document = Document::new(1);
    let _ = document.create_rect(RectBounds {
        origin: pt(10.0, 20.0),
        width: Length::from_mm(100.0),
        height: Length::from_mm(60.0),
    });
    let mut session = open(&document);
    let press = pt(40.0, 20.0);
    session.pointer_hover(press, false, false);
    session.pointer_down(press, false);
    session.pointer_up(press, false, false);
    let before = session.draw_list();
    session.pointer_hover(press, false, false);
    session.pointer_down(press, false);
    session.pointer_hover(pt(60.0, 30.0), false, ctrl);
    let committed = first_object(&session);
    (session, committed, before)
}

#[test]
fn a_moved_object_is_drawn_at_the_pointer_and_its_outline_stays_behind() {
    let (mut session, committed, before) = rect_moved(false);
    let view = session.view();
    let moved = committed.translated(Vec2::new(20.0, 10.0));
    let during = session.draw_list();
    let old_artwork = build_artwork(std::slice::from_ref(&committed), view);
    assert!(old_artwork.triangle_count() > 0);
    assert!(
        contains(&during, &build_artwork(std::slice::from_ref(&moved), view)),
        "the object itself follows the pointer"
    );
    assert!(
        !contains(&during, &old_artwork),
        "the object is no longer drawn at its origin"
    );
    assert!(
        contains(
            &during,
            &build_drag_ghost(std::slice::from_ref(&committed), view)
        ),
        "the ghost outline stays at the origin"
    );
    assert!(
        !contains(
            &during,
            &build_live_edit_preview(std::slice::from_ref(&moved), view)
        ),
        "no blue outline travels with the pointer"
    );
    session.escape();
    assert_eq!(
        session.draw_list().triangles,
        before.triangles,
        "Escape restores the frame from before the drag"
    );
}

#[test]
fn a_ctrl_copy_drag_draws_the_copy_at_the_pointer_and_keeps_the_original() {
    let (session, committed, _) = rect_moved(true);
    let view = session.view();
    let copy = committed.translated(Vec2::new(20.0, 10.0));
    let during = session.draw_list();
    assert!(contains(
        &during,
        &build_artwork(std::slice::from_ref(&committed), view)
    ));
    assert!(contains(
        &during,
        &build_artwork(std::slice::from_ref(&copy), view)
    ));
    assert!(
        !contains(
            &during,
            &build_drag_ghost(std::slice::from_ref(&committed), view)
        ),
        "the original stays drawn as itself, so it needs no ghost"
    );
}

#[test]
fn a_moved_image_is_drawn_at_the_pointer_with_its_frame_left_behind() {
    let document = Document::new(1);
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images/png_rgba_8.png"),
    )
    .unwrap();
    let _ = document
        .add_images(&[NewImage {
            bytes: Arc::from(bytes.as_slice()),
            placement: Affine::new(40.0, 0.0, 0.0, 20.0, 50.0, 50.0),
            name: "a.png".to_owned(),
        }])
        .unwrap();
    let mut session = open(&document);
    let press = pt(80.0, 60.0);
    session.pointer_hover(press, false, false);
    session.pointer_down(press, false);
    session.pointer_up(press, false, false);
    session.pointer_hover(press, false, false);
    session.pointer_down(press, false);
    session.pointer_hover(pt(100.0, 70.0), false, false);
    let committed = first_object(&session);
    let during = session.draw_list();
    assert_eq!(during.images.len(), 1, "the picture is drawn once");
    let corners = during.images[0].corners;
    assert!((corners[0].x - 70.0).abs() < 1e-6 && (corners[0].y - 60.0).abs() < 1e-6);
    assert!(contains(
        &during,
        &build_drag_ghost(std::slice::from_ref(&committed), session.view())
    ));
}
