//! Tester cases for `specs/0025-raster-trace` slice 1 at the session level: criterion 14 (the
//! paths sit on the pixels through a rotated and flipped placement, the image's opacity is not
//! copied), criterion 3 (a locked or hidden image cannot be traced), criterion 13 (the image
//! stays visible, one history step) and criterion 7 (leaving the tool writes nothing).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::sync::Arc;

use curvyo_document_core::{
    Affine, Document, NewImage, NodeId, ObjectSnapshot, Opacity, Point, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, Session, Tool, TraceCreateOutcome, TraceSelection};
use curvyo_ui_core::trace_job;

fn picture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images")
            .join(name),
    )
    .unwrap()
}

fn session_with(placement: Affine, extra: impl FnOnce(&Document, NodeId)) -> (Session, NodeId) {
    let document = Document::new(1);
    let id = document
        .add_images(&[NewImage {
            bytes: Arc::from(picture("png_rgba_8.png").as_slice()),
            placement,
            name: "a.png".to_owned(),
        }])
        .unwrap()[0];
    extra(&document, id);
    let mut session = Session::open(2, &pack(&document, "t").unwrap()).unwrap();
    session.resize_viewport(1600.0, 1000.0);
    session.set_tool(Tool::Select);
    (session, id)
}

fn click(session: &mut Session, at: Point) {
    session.pointer_hover(at, false, false);
    session.pointer_down(at, false);
    session.pointer_up(at, false, false);
}

fn key(session: &mut Session, k: &str) {
    session.key_down(KeyInput {
        key: k,
        ..KeyInput::default()
    });
}

fn reopened(session: &Session) -> Document {
    unpack(99, &session.pack("t").unwrap()).unwrap()
}

/// 4 x 2 px, columns 1 and 2 black.
fn block_4x2() -> Vec<u8> {
    (0..8)
        .flat_map(|i| {
            let v = if (1..3).contains(&(i % 4)) { 0 } else { 255 };
            [v, v, v, 255]
        })
        .collect()
}

#[test]
fn c14_rotated_flipped_placement_opacity_not_copied_one_step_image_stays() {
    // The 40 x 20 mm frame flipped horizontally and turned 90 degrees about its centre (20, 10),
    // hand-computed for unit-square coordinates (u, v): X = 30 - 20 v, Y = 30 - 40 u.
    let placement = Affine::new(0.0, -40.0, -20.0, 0.0, 30.0, 30.0);
    let (mut session, image) = session_with(placement, |document, id| {
        document
            .set_node_opacity(&[id], Opacity::new(0.4).unwrap())
            .unwrap();
    });
    // Select the image by clicking its middle (the frame covers X 10..30, Y 0..30 after the turn).
    click(&mut session, Point::new(20.0, 15.0));
    assert_eq!(session.selected_object_count(), 1);
    session.set_tool(Tool::Trace);
    assert_eq!(session.trace_view().selection, TraceSelection::Images);
    let steps_before = reopened(&session).object_ids().len();
    let source = session.trace_source(image, false).expect("a source");
    // The pixel map of the real 4 x 2 px picture over this placement, by hand:
    // pixel (x, y) -> (30 - 10 y, 30 - 10 x).
    let map = Affine::new(0.0, -10.0, -10.0, 0.0, 30.0, 30.0);
    let rgba = block_4x2();
    let wire = trace_job(&rgba, 4, 2, map.to_array(), 128, 80, 0.0);
    assert!(session.set_trace_result(image, &wire));
    // the source's own map is the placement over the *stored* picture size
    assert!((source.map[4] - 30.0).abs() < 1e-9 && (source.map[5] - 30.0).abs() < 1e-9);
    let outcome = session.create_traced_paths();
    assert!(matches!(
        outcome,
        TraceCreateOutcome::Created {
            images: 1,
            paths: 1,
            ..
        }
    ));
    let document = reopened(&session);
    assert_eq!(document.object_ids().len(), steps_before + 1);
    let ids = document.object_ids();
    let at = ids.iter().position(|&id| id == image).unwrap();
    let ObjectSnapshot::Path(compound) = document.object(ids[at + 1]).unwrap() else {
        panic!("the path sits directly above the image");
    };
    assert!(compound.style.fill.enabled && !compound.style.stroke.enabled);
    assert_eq!(
        compound.style.fill.opacity,
        Opacity::OPAQUE,
        "the image's opacity is not copied"
    );
    let corners = [(30.0, 20.0), (30.0, 0.0), (10.0, 0.0), (10.0, 20.0)];
    assert_ne!(compound.anchors.len(), 0);
    for anchor in &compound.anchors {
        let nearest = corners
            .iter()
            .map(|&(x, y)| ((anchor.point.x - x).powi(2) + (anchor.point.y - y).powi(2)).sqrt())
            .fold(f64::MAX, f64::min);
        assert!(
            nearest < 1e-6,
            "{:?} is {nearest} mm from a hand-computed corner",
            anchor.point
        );
    }
    for &(x, y) in &corners {
        assert!(
            compound
                .anchors
                .iter()
                .any(|a| (a.point.x - x).abs() < 1e-6 && (a.point.y - y).abs() < 1e-6),
            "corner ({x}, {y}) missing"
        );
    }
    let nodes = document.read_nodes();
    let group_opacity = nodes
        .iter()
        .find(|n| Some(n.id) == document.parent_group(ids[at + 1]))
        .map(|n| n.opacity);
    assert_eq!(
        group_opacity,
        Some(Opacity::OPAQUE),
        "the group has no opacity of the image"
    );
    // the image stays, visible and unchanged
    let ObjectSnapshot::Image(still) = document.object(image).unwrap() else {
        panic!("image");
    };
    assert_eq!(still.placement, placement);
    assert_eq!(session.tool(), Tool::Select);
    // one undo removes the group and nothing else
    session.undo();
    assert_eq!(reopened(&session).object_ids(), vec![image]);
}

#[test]
fn c3_a_locked_or_hidden_image_is_not_traced() {
    for hide in [false, true] {
        let (mut session, id) =
            session_with(Affine::new(40.0, 0.0, 0.0, 20.0, 50.0, 50.0), |d, id| {
                if hide {
                    d.set_node_visible(&[id], false).unwrap();
                } else {
                    d.set_node_locked(&[id], true).unwrap();
                }
            });
        click(&mut session, Point::new(70.0, 60.0));
        session.set_tool(Tool::Trace);
        assert_ne!(
            session.trace_view().selection,
            TraceSelection::Images,
            "hidden={hide}"
        );
        assert!(session.trace_source(id, false).is_none());
        assert_eq!(session.create_traced_paths(), TraceCreateOutcome::Ignored);
    }
}

#[test]
fn c7_entering_and_leaving_the_tool_writes_nothing() {
    let (mut session, image) =
        session_with(Affine::new(40.0, 0.0, 0.0, 20.0, 50.0, 50.0), |_, _| {});
    click(&mut session, Point::new(70.0, 60.0));
    let before = session.pack("t").unwrap();
    key(&mut session, "i");
    assert_eq!(session.tool(), Tool::Trace);
    let source = session.trace_source(image, true).unwrap();
    let wire = trace_job(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        128,
        80,
        0.5,
    );
    assert!(session.set_trace_preview(image, &wire));
    key(&mut session, "Escape");
    assert_ne!(session.tool(), Tool::Trace);
    assert_eq!(reopened(&session).object_ids(), vec![image]);
    session.undo();
    assert_eq!(
        reopened(&session).object_ids(),
        vec![image],
        "no history step to undo besides the import"
    );
    let _ = before;
}

#[test]
fn create_without_a_settled_full_size_result_is_ignored_and_a_preview_is_not_a_result() {
    let (mut session, image) =
        session_with(Affine::new(40.0, 0.0, 0.0, 20.0, 50.0, 50.0), |_, _| {});
    click(&mut session, Point::new(70.0, 60.0));
    session.set_tool(Tool::Trace);
    let source = session.trace_source(image, true).unwrap();
    let wire = trace_job(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        128,
        80,
        0.5,
    );
    session.set_trace_preview(image, &wire);
    assert_eq!(session.create_traced_paths(), TraceCreateOutcome::Ignored);
    assert!(
        !session.set_trace_result(image, &[1.0, 2.0]),
        "garbage is not a result"
    );
    assert!(!session.set_trace_result(image, &[]));
    assert!(!session.set_trace_preview(image, &[f64::NAN; 12]));
    assert_eq!(reopened(&session).object_ids(), vec![image]);
}
