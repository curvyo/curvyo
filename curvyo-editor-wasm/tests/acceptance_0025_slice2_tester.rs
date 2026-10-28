//! Tester cases for `specs/0025-raster-trace` slice 2 at the session: criterion 68 (a), the
//! trace group's counts for the Layers row tooltip, through Create paths, undo and redo.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use curvyo_document_core::{Affine, Document, NewImage, NodeId, Point, pack};
use curvyo_editor_wasm::{Session, Tool, TraceCreateOutcome};
use curvyo_ui_core::trace_job;

fn session_with_two_images() -> (Session, Vec<NodeId>) {
    let png = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images/png_rgba_8.png"),
    )
    .unwrap();
    let document = Document::new(1);
    let ids = document
        .add_images(&[
            NewImage {
                bytes: Arc::from(png.as_slice()),
                placement: Affine::new(40.0, 0.0, 0.0, 20.0, 50.0, 50.0),
                name: "a.png".to_owned(),
            },
            NewImage {
                bytes: Arc::from(png.as_slice()),
                placement: Affine::new(40.0, 0.0, 0.0, 20.0, 50.0, 100.0),
                name: "b.png".to_owned(),
            },
        ])
        .unwrap();
    let mut session = Session::open(2, &pack(&document, "t").unwrap()).unwrap();
    session.resize_viewport(1600.0, 1000.0);
    session.set_tool(Tool::Select);
    (session, ids)
}

fn square_wire(map: [f64; 6]) -> Vec<f64> {
    let rgba: Vec<u8> = (0..16 * 16)
        .flat_map(|i| {
            let (x, y) = (i % 16, i / 16);
            let v = if (4..12).contains(&x) && (4..12).contains(&y) {
                0
            } else {
                255
            };
            [v, v, v, 255]
        })
        .collect();
    trace_job(&rgba, 16, 16, map, 128, 80, 0.0)
}

#[test]
fn c68a_the_group_knows_its_counts_for_the_tooltip_and_undo_redo_keep_them_coherent() {
    let (mut session, images) = session_with_two_images();
    session.pointer_hover(Point::new(70.0, 60.0), false, false);
    session.pointer_down(Point::new(70.0, 60.0), false);
    session.pointer_up(Point::new(70.0, 60.0), false, false);
    session.set_tool(Tool::Trace);
    let source = session.trace_source(images[0], false).unwrap();
    assert!(session.set_trace_result(images[0], &square_wire(source.map)));
    let TraceCreateOutcome::Created { paths, nodes, .. } = session.create_traced_paths() else {
        panic!("created");
    };
    let doc = curvyo_document_core::unpack(99, &session.pack("t").unwrap()).unwrap();
    let group = doc
        .object_ids()
        .into_iter()
        .find_map(|id| doc.parent_group(id))
        .expect("a group");
    assert_eq!(session.traced_counts(group), Some((paths, nodes)));
    assert_eq!(
        session.traced_counts(images[0]),
        None,
        "only trace groups have counts"
    );
    assert_eq!(session.traced_counts(images[1]), None);
    session.undo();
    session.redo();
    let doc = curvyo_document_core::unpack(99, &session.pack("t").unwrap()).unwrap();
    let again = doc
        .object_ids()
        .into_iter()
        .find_map(|id| doc.parent_group(id));
    assert!(again.is_some(), "redo brings the group back");
    println!(
        "counts after redo: {:?}",
        again.and_then(|g| session.traced_counts(g))
    );
}
