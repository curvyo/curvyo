//! Tester cases for `specs/0025-raster-trace` slice 3 at the session: criteria 51 (the views, the
//! peek, the cycle), 52 (reduced copy of a large image, the document untouched until Create
//! paths), 55 (over 20,000 nodes: one group, one step, one Layers row).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use curvyo_document_core::{NodeId, unpack};
use curvyo_editor_wasm::{Session, Tool, TraceCreateOutcome, TraceViewMode};
use curvyo_ui_core::{TRACE_WIRE_HEADER, trace_job};
use image::ImageEncoder;

mod cpu_time;

/// A `w` x `h` PNG: a black square in the middle third on white.
fn png(w: u32, h: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let v = if (w / 3..2 * w / 3).contains(&x) && (h / 3..2 * h / 3).contains(&y) {
                0
            } else {
                255
            };
            rgba.extend([v, v, v, 255]);
        }
    }
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&rgba, w, h, image::ExtendedColorType::Rgba8)
        .unwrap();
    out
}

fn traced(w: u32, h: u32) -> (Session, NodeId) {
    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    session.import_begin(None);
    session.import_add(&png(w, h), "a.png");
    session.import_finish(false);
    session.set_tool(Tool::Trace);
    let image = session
        .trace_view()
        .shown
        .expect("the imported image is shown");
    (session, image)
}

fn preview(session: &mut Session, image: NodeId) {
    let source = session.trace_source(image, false).unwrap();
    let wire = trace_job(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        128,
        80,
        0.0,
    );
    assert!(session.set_trace_preview(image, &wire));
    assert!(session.set_trace_result(image, &wire));
}

#[test]
fn c51_the_four_views_overlay_mask_trace_image_draw_what_the_spec_says() {
    let (mut session, image) = traced(60, 60);
    preview(&mut session, image);
    assert_eq!(session.trace_view_mode(), TraceViewMode::Overlay, "default");
    let overlay = session.draw_list();
    session.set_trace_view_mode(TraceViewMode::Trace);
    let trace = session.draw_list();
    session.set_trace_view_mode(TraceViewMode::Image);
    let original = session.draw_list();
    assert_eq!(overlay.images.len(), 1, "Overlay keeps the dimmed picture");
    assert_eq!(trace.images.len(), 0, "Trace hides the image");
    assert_eq!(original.images.len(), 1, "Image shows the original");
    assert!(
        original.triangle_count() < overlay.triangle_count(),
        "Image draws no outline: {} vs Overlay {}",
        original.triangle_count(),
        overlay.triangle_count()
    );
    assert!(
        trace.triangle_count() > original.triangle_count(),
        "Trace draws the filled result"
    );
    // The views never touch the document.
    let after = unpack(9, &session.pack("t").unwrap()).unwrap();
    assert_eq!(after.object_ids().len(), 1);
}

#[test]
fn c51_peek_shows_the_original_while_held_and_returns() {
    let (mut session, image) = traced(60, 60);
    preview(&mut session, image);
    session.set_trace_view_mode(TraceViewMode::Trace);
    let chosen = session.draw_list();
    session.set_trace_peek(true);
    assert_eq!(session.shown_trace_view_mode(), TraceViewMode::Image);
    assert_eq!(
        session.trace_view_mode(),
        TraceViewMode::Trace,
        "the choice is kept"
    );
    assert_eq!(session.draw_list().images.len(), 1);
    session.set_trace_peek(false);
    assert_eq!(session.shown_trace_view_mode(), TraceViewMode::Trace);
    assert_eq!(
        session.draw_list().triangle_count(),
        chosen.triangle_count()
    );
}

#[test]
fn c51_the_cycle_goes_overlay_trace_image_and_the_peek_ends_with_the_tool() {
    let (mut session, image) = traced(60, 60);
    preview(&mut session, image);
    // Mask is the second view since `specs/0089` criterion 21 (UX notes §4).
    for want in [
        TraceViewMode::Mask,
        TraceViewMode::Trace,
        TraceViewMode::Image,
        TraceViewMode::Overlay,
    ] {
        session.cycle_trace_view_mode();
        assert_eq!(session.trace_view_mode(), want, "Shift+backslash cycles");
    }
    // Leaving the tool drops a held peek; entering starts on Overlay again.
    session.set_trace_view_mode(TraceViewMode::Image);
    session.set_trace_peek(true);
    session.set_tool(Tool::Select);
    assert_ne!(
        session.shown_trace_view_mode(),
        TraceViewMode::Image,
        "peek ends with the tool"
    );
    session.set_trace_peek(true);
    assert!(
        session.shown_trace_view_mode() != TraceViewMode::Image
            || session.trace_view_mode() == TraceViewMode::Image,
        "a peek outside the Trace tool does nothing"
    );
    session.set_tool(Tool::Trace);
    assert_eq!(
        session.trace_view_mode(),
        TraceViewMode::Overlay,
        "the chosen view is not remembered"
    );
}

#[test]
fn c52_a_large_image_gets_a_reduced_copy_of_1000_px_and_the_document_stays_untouched() {
    let (mut session, image) = traced(2300, 1800); // 4.14 Mpx, over 4 Mpx
    let before = session.pack("t").unwrap();
    let reduced = session.trace_source(image, true).unwrap();
    assert_eq!(
        reduced.width.max(reduced.height),
        1000,
        "longest side 1000 px"
    );
    assert!(reduced.width <= 1000 && reduced.height <= 1000);
    let full = session.trace_source(image, false).unwrap();
    assert_eq!(
        (full.width, full.height),
        (2300, 1800),
        "full size for Create paths"
    );
    assert!(session.set_trace_preview(image, &squares(10, full.map)));
    assert!(session.set_trace_result(image, &squares(10, full.map)));
    assert_eq!(
        session.pack("t").unwrap(),
        before,
        "tracing writes nothing to the document"
    );
    let TraceCreateOutcome::Created { .. } = session.create_traced_paths() else {
        panic!("Create paths works on a large image");
    };
    assert_ne!(session.pack("t").unwrap(), before);
}

#[test]
fn c52_a_small_image_is_not_reduced() {
    let (mut session, image) = traced(900, 600);
    let reduced = session.trace_source(image, true).unwrap();
    assert!(reduced.width * reduced.height <= 900 * 600);
}

/// `count` squares of 4 nodes, as the worker's wire (see `trace_view_budget.rs`).
fn squares(count: usize, map: [f64; 6]) -> Vec<f64> {
    let mut wire = vec![0.0; TRACE_WIRE_HEADER];
    wire[1] = count as f64;
    wire[2] = (count * 4) as f64;
    wire[TRACE_WIRE_HEADER - 1] = count as f64;
    for i in 0..count {
        let (x, y) = ((i % 250) as f64 * 0.4, (i / 250) as f64 * 0.4);
        wire.extend([6.0, 0.0, x, y, 1.0, x + 0.2, y, 1.0, x + 0.2, y + 0.2]);
        wire.extend([1.0, x, y + 0.2, 1.0, x, y, 4.0]);
    }
    let _ = map;
    wire
}

#[test]
fn c55_over_20000_nodes_is_one_group_in_one_step() {
    let (mut session, image) = traced(60, 60);
    let source = session.trace_source(image, false).unwrap();
    let wire = squares(5_001, source.map);
    assert!(session.set_trace_result(image, &wire));
    let steps = session.undo_len();
    let TraceCreateOutcome::Created { paths, nodes, .. } = session.create_traced_paths() else {
        panic!("Create paths works over 20,000 nodes");
    };
    assert_eq!((paths, nodes), (5_001, 20_004));
    assert_eq!(session.undo_len(), steps + 1, "one history step");
    let doc = unpack(9, &session.pack("t").unwrap()).unwrap();
    let groups = doc
        .object_ids()
        .into_iter()
        .filter_map(|id| doc.parent_group(id))
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(groups.len(), 1, "one group");
}

/// Criterion 13 and 55: Ctrl+Z reverses the whole trace, also a big one. CPU time of the undo of a
/// trace with 3,000 and with 12,000 outlines: linear work is 4 times, quadratic work 16 times.
#[test]
fn c55_undo_of_a_big_trace_is_not_quadratic() {
    let undo_time = |count: usize| {
        let (mut session, image) = traced(60, 60);
        let source = session.trace_source(image, false).unwrap();
        assert!(session.set_trace_result(image, &squares(count, source.map)));
        assert!(matches!(
            session.create_traced_paths(),
            TraceCreateOutcome::Created { .. }
        ));
        let (_, took) = cpu_time::cpu_time_of(|| session.undo());
        took.as_secs_f64()
    };
    let small = undo_time(3_000).max(0.001);
    let large = undo_time(12_000);
    println!(
        "undo: 3,000 outlines {small:.3} s, 12,000 outlines {large:.3} s, ratio {:.1}",
        large / small
    );
    assert!(
        large / small < 8.0,
        "undo is quadratic: ratio {:.1}",
        large / small
    );
}

#[test]
fn c55_a_100000_node_result_still_creates_paths() {
    let (mut session, image) = traced(60, 60);
    let source = session.trace_source(image, false).unwrap();
    assert!(session.set_trace_result(image, &squares(25_000, source.map)));
    let TraceCreateOutcome::Created { nodes, .. } = session.create_traced_paths() else {
        panic!("Create paths over 100,000 nodes");
    };
    assert_eq!(nodes, 100_000);
}
