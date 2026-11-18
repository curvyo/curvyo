//! The CPU cost of a drawn frame by phase on the performance page of `tools/perf/` (5 000
//! stroked paths, imported and fitted, `specs/0126-performance-pass` phase 4): the draw list
//! and the pictures (the wasm probe times the GPU vertices), at rest, zooming and panning. Thread CPU time
//! (`CLAUDE.md` §5); the budgets are asserted in a release build only, a debug build prints.
//! An integration test: the library's own test build checks every cached document read
//! against a fresh one, which would be most of the time measured.
//!
//! ```text
//! cargo test --release -p curvyo-editor-wasm --test frame_phase_cost -- --nocapture
//! ```

#![allow(clippy::unwrap_used, missing_docs)]

mod cpu_time;

use std::fmt::Write as _;
use std::time::Duration;

use cpu_time::cpu_time_of;

use curvyo_editor_wasm::Session;

/// The page `frontend/src/lib/perfProbe.ts` imports: `nodes` stroked closed curves in rows
/// of 100.
fn generated_svg(nodes: usize) -> String {
    let columns = 100;
    let mut paths = String::new();
    for i in 0..nodes {
        let x = 20 + (i % columns) * 10;
        let y = 20 + (i / columns) * 10;
        write!(
            paths,
            r##"<path d="M{x} {y} c3 -4 6 -4 8 0 s-2 5 -8 4 s-2 -3 0 -4z" fill="none" stroke="#000" stroke-width="0.4"/>"##
        )
        .unwrap();
    }
    let height = 40 + nodes.div_ceil(columns) * 10;
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1040" height="{height}" viewBox="0 0 1040 {height}">{paths}</svg>"#
    )
}

/// The perf page as the probe leaves it: imported (its paths selected) and fitted into a
/// canvas of 1 280 x 800 CSS pixels.
fn perf_session() -> Session {
    let mut session = Session::new(1);
    session.resize_viewport(1280.0, 800.0);
    session
        .import_svg(generated_svg(5000).as_bytes(), "perf-5000.svg")
        .unwrap();
    session.fit_document();
    session
}

/// The CPU time per frame of each phase over `frames` frames, `step` moving the view before
/// each, and the layers and vertices of the last frame.
#[derive(Debug, Default)]
struct PhaseCost {
    draw_list: Duration,
    images: Duration,
    layer_count: usize,
    vertex_count: usize,
}

fn measure(
    session: &mut Session,
    frames: u32,
    mut step: impl FnMut(&mut Session, u32),
) -> PhaseCost {
    let mut cost = PhaseCost::default();
    for frame in 0..frames {
        step(session, frame);
        let (mut list, draw_list) = cpu_time_of(|| session.frame_draw_list());
        let (pictures, images) = cpu_time_of(|| session.frame_images(&mut list));
        assert_eq!(pictures.len(), 0, "the perf page has no images");
        cost.draw_list += draw_list;
        cost.images += images;
        cost.layer_count = list.layers().len();
        cost.vertex_count = list.triangles.len();
    }
    cost.draw_list /= frames;
    cost.images /= frames;
    cost
}

fn report(label: &str, cost: &PhaseCost, budget: Duration) {
    println!(
        "{label}: draw list {:?}, pictures {:?}; {} layers, {} vertices (draw-list budget {budget:?})",
        cost.draw_list, cost.images, cost.layer_count, cost.vertex_count
    );
    if !cfg!(debug_assertions) {
        assert!(
            cost.draw_list <= budget,
            "{label}: the draw list took {:?}, over {budget:?}",
            cost.draw_list
        );
    }
}

/// The draw-list budgets of a frame of the perf page on the host CPU (release build). Measured
/// on 2026-11-18: 36 ms at rest (everything in view), 31 ms zooming and 28 ms panning, where only
/// what the canvas shows is tessellated; zoom and pan are held to about 1.5 times that.
const REST_BUDGET: Duration = Duration::from_millis(60);
const ZOOM_BUDGET: Duration = Duration::from_millis(45);
const PAN_BUDGET: Duration = Duration::from_millis(45);

#[test]
fn frame_cost_of_the_perf_page_by_phase() {
    let mut session = perf_session();
    // A first frame warms the caches the document read fills.
    let _ = session.frame_draw_list();
    let rest = measure(&mut session, 5, |_, _| {});
    report("rest", &rest, REST_BUDGET);
    // Ctrl+wheel about the canvas centre, ten notches in, one per frame, as the probe's
    // wheel-zoom (which ends about four times closer than the fit).
    let zoom = measure(&mut session, 10, |session, _| {
        session.wheel(0.0, -100.0, 640.0, 400.0, false, true);
    });
    report("zoom", &zoom, ZOOM_BUDGET);
    println!("zoomed to scale {:.2}", session.view().scale());
    // A middle-button drag on a circle of 150 px at that zoom, as the probe's pan-drag.
    session.begin_pan(790.0, 400.0);
    let pan = measure(&mut session, 10, |session, frame| {
        let angle = f64::from(frame + 1) / 30.0;
        session.pan_to(640.0 + 150.0 * angle.cos(), 400.0 + 150.0 * angle.sin());
    });
    session.end_pan();
    report("pan", &pan, PAN_BUDGET);
}
