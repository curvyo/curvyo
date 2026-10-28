//! The Trace tool's main-thread budgets (`specs/0025-raster-trace` criteria 56 and 58), as thread
//! CPU time (`tests/cpu_time/`): a pan or zoom with a preview of 100,000 nodes redraws within
//! 16 ms (the cached, culled outline; the GPU part is not measured here), and no handler the page
//! runs on the main thread around a trace of a 16 Mpx image blocks for more than 50 ms (the trace
//! itself runs in the worker). Budgets are asserted in release builds (the `performance-budgets`
//! job); a debug build prints the times:
//!
//! ```text
//! cargo test --release -p curvyo-editor-wasm --test trace_view_budget -- --nocapture
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    missing_docs
)]

mod cpu_time;

use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_document_core::NodeId;
use curvyo_editor_wasm::{Session, Tool, TraceViewMode};
use curvyo_ui_core::TRACE_WIRE_HEADER;
use image::ImageEncoder;

const FRAME: Duration = Duration::from_millis(16);
const HANDLER: Duration = Duration::from_millis(50);

/// `count` squares of 4 nodes on a grid of 250 columns, 0.4 mm apart, as the worker's numbers.
fn squares(count: usize) -> Vec<f64> {
    let mut wire = vec![0.0; TRACE_WIRE_HEADER];
    wire[1] = count as f64;
    wire[2] = (count * 4) as f64;
    wire[TRACE_WIRE_HEADER - 1] = count as f64;
    for i in 0..count {
        let (x, y) = ((i % 250) as f64 * 0.4, (i / 250) as f64 * 0.4);
        wire.extend([6.0, 0.0, x, y, 1.0, x + 0.2, y, 1.0, x + 0.2, y + 0.2]);
        wire.extend([1.0, x, y + 0.2, 1.0, x, y, 4.0]);
    }
    wire
}

/// A session with one imported `side` x `side` px picture of black discs of radius 7 px on a
/// 24 px grid, selected in the Trace tool.
fn traced_image(side: u32) -> (Session, NodeId) {
    let mut rgba = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        for x in 0..side {
            let (dx, dy) = (f64::from(x % 24) - 11.5, f64::from(y % 24) - 11.5);
            let v = if dx * dx + dy * dy < 49.0 { 0 } else { 255 };
            rgba.extend([v, v, v, 255]);
        }
    }
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&rgba, side, side, image::ExtendedColorType::Rgba8)
        .unwrap();
    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    session.import_begin(None);
    session.import_add(&png, "rings.png");
    session.import_finish(false);
    session.set_tool(Tool::Trace);
    let image = session.trace_view().shown.unwrap();
    (session, image)
}

fn check(what: &str, took: Duration, budget: Duration) {
    println!("{what}: {:.2} ms CPU", took.as_secs_f64() * 1000.0);
    if !cfg!(debug_assertions) {
        assert!(took < budget, "{what} took {took:?}, budget {budget:?}");
    }
}

/// Criterion 56: pan and zoom with the traced preview of at least 100,000 nodes of a 3200 x 3200
/// px picture, whole in view, in the Overlay and the Trace view; each frame of the moving view is
/// the slowest of five, and the frame at rest after the zoom (which tessellates once for the new
/// zoom) is reported.
#[test]
fn a_pan_or_zoom_over_a_100000_node_preview_redraws_within_16_ms() {
    let (mut session, image) = traced_image(3200);
    let source = session.trace_source(image, false).unwrap();
    let wire = curvyo_ui_core::trace_job(&source.rgba, 3200, 3200, source.map, 128, 80, 0.0);
    let nodes = wire[2];
    println!("preview of {nodes} nodes");
    assert!(nodes >= 100_000.0);
    assert!(session.set_trace_preview(image, &wire));
    for mode in [TraceViewMode::Overlay, TraceViewMode::Trace] {
        session.set_trace_view_mode(mode);
        session.set_trace_view_moving(false);
        let (_, first) = cpu_time_of(|| session.draw_list());
        println!(
            "{mode:?}, first frame (tessellates): {:.2} ms CPU (reported)",
            first.as_secs_f64() * 1000.0
        );
        let (_, still) = cpu_time_of(|| session.draw_list());
        check(&format!("{mode:?}, a frame at rest"), still, FRAME);
        let slowest = |session: &mut Session, step: &dyn Fn(&mut Session)| {
            session.set_trace_view_moving(true);
            (0..5)
                .map(|_| {
                    step(session);
                    cpu_time_of(|| session.draw_list()).1
                })
                .max()
                .unwrap()
        };
        let pan = slowest(&mut session, &|s| {
            s.wheel(0.0, 40.0, 600.0, 400.0, false, false);
        });
        check(&format!("{mode:?}, pan"), pan, FRAME);
        let zoom = slowest(&mut session, &|s| {
            s.wheel(0.0, -120.0, 600.0, 400.0, false, true);
        });
        check(&format!("{mode:?}, zoom in"), zoom, FRAME);
        let zoom = slowest(&mut session, &|s| {
            s.wheel(0.0, 120.0, 600.0, 400.0, false, true);
        });
        check(&format!("{mode:?}, zoom out"), zoom, FRAME);
        session.set_trace_view_moving(false);
        let (_, rest) = cpu_time_of(|| session.draw_list());
        println!(
            "{mode:?}, the frame at rest after the zoom (tessellates): {:.2} ms CPU (reported)",
            rest.as_secs_f64() * 1000.0
        );
    }
}

/// Criterion 58: the main-thread side of a trace of a 16 Mpx picture: reading its reduced copy
/// and its full pixels for the worker, taking a 100,000-node result as preview and as result,
/// a changed value dropping the results, a view change, and the reduced copy of the next
/// preview (kept from the first).
#[test]
fn the_main_thread_handlers_of_a_16_mpx_trace_stay_under_50_ms() {
    let (mut session, image) = traced_image(4000);
    let (reduced, took) = cpu_time_of(|| session.trace_source(image, true).unwrap());
    assert_eq!((reduced.width, reduced.height), (1000, 1000));
    check("reduced copy for the worker", took, HANDLER);
    let (full, took) = cpu_time_of(|| session.trace_source(image, false).unwrap());
    assert_eq!(full.width * full.height, 16_000_000);
    check("full pixels for the worker", took, HANDLER);
    let wire = squares(25_000);
    let ((), took) = cpu_time_of(|| assert!(session.set_trace_preview(image, &wire)));
    check("100,000-node result as preview", took, HANDLER);
    let ((), took) = cpu_time_of(|| assert!(session.set_trace_result(image, &wire)));
    check("100,000-node result kept", took, HANDLER);
    let ((), took) = cpu_time_of(|| session.clear_trace_results());
    check("a changed value", took, HANDLER);
    let ((), took) = cpu_time_of(|| session.cycle_trace_view_mode());
    check("Shift+\\ to the Trace view", took, HANDLER);
    let (_, took) = cpu_time_of(|| session.trace_source(image, true).unwrap());
    check("the reduced copy again", took, HANDLER);
}
