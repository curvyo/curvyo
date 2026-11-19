//! The CPU cost of a drawn frame by phase on the performance page of `tools/perf/` (stroked
//! paths in rows of 100, imported and fitted, `specs/0126-performance-pass` phase 4): the draw
//! list, the pictures and the GPU vertex build (the wasm probe times the upload and the submit),
//! at rest, panning at the fit (the scale unchanged, everything in view), zooming and panning
//! zoomed in, with all its paths selected and with none, on 5 000 and on 20 000 paths. Thread CPU
//! time (`CLAUDE.md` §5); the budgets are asserted in a release build only, a debug build prints
//! (and skips the 20 000-path page).
//! An integration test: the library's own test build checks every cached document read
//! against a fresh one, which would be most of the time measured.
//!
//! ```text
//! cargo test --release -p curvyo-editor-wasm --test frame_phase_cost -- --nocapture --test-threads 1
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

/// The perf page of `nodes` paths as the probe leaves it: imported (its paths selected) and
/// fitted into a canvas of 1 280 x 800 CSS pixels, the pointer over its centre.
fn perf_session(nodes: usize) -> Session {
    let mut session = Session::new(1);
    session.resize_viewport(1280.0, 800.0);
    session
        .import_svg(generated_svg(nodes).as_bytes(), "perf.svg")
        .unwrap();
    session.fit_document();
    // The pointer rests over the canvas centre, as in the app while it zooms and pans: the
    // hover reads (handle under the pointer) run every frame.
    let centre = session.screen_to_document(640.0, 400.0);
    session.pointer_hover(centre, false, false);
    session
}

/// The CPU time per frame of each phase over `frames` frames, `step` moving the view before
/// each, and the layers and vertices of the last frame.
#[derive(Debug, Default)]
struct PhaseCost {
    draw_list: Duration,
    images: Duration,
    vertices: Duration,
    layer_count: usize,
    vertex_count: usize,
}

/// The vertex shape `gpu_vertex.rs` builds (wasm-only, so not callable here): a position
/// relative to the view origin and a colour in `f32`, and the layer depth.
type GpuShape = ([f32; 2], [f32; 4], f32);

/// The GPU vertices of `list` relative to `origin`, as `gpu_vertex::gpu_vertices` builds them.
#[allow(clippy::cast_possible_truncation)]
fn gpu_shape(list: &curvyo_render_core::DrawList, origin: (f64, f64)) -> Vec<GpuShape> {
    list.triangles
        .iter()
        .zip(list.vertex_depths())
        .map(|(vertex, depth)| {
            let c = vertex.color;
            (
                [
                    (vertex.position.x - origin.0) as f32,
                    (vertex.position.y - origin.1) as f32,
                ],
                [c.r, c.g, c.b, c.a].map(|v| f32::from(v) / 255.0),
                depth,
            )
        })
        .collect()
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
        let origin = session.screen_to_document(0.0, 0.0);
        let (shape, vertices) = cpu_time_of(|| gpu_shape(&list, (origin.x, origin.y)));
        std::hint::black_box(shape);
        cost.draw_list += draw_list;
        cost.images += images;
        cost.vertices += vertices;
        cost.layer_count = list.layers().len();
        cost.vertex_count = list.triangles.len();
    }
    cost.draw_list /= frames;
    cost.images /= frames;
    cost.vertices /= frames;
    cost
}

fn report(label: &str, cost: &PhaseCost, budget: Duration) {
    println!(
        "{label}: draw list {:?}, pictures {:?}, GPU vertices {:?}; {} layers, {} vertices \
         (draw-list budget {budget:?})",
        cost.draw_list, cost.images, cost.vertices, cost.layer_count, cost.vertex_count
    );
    if !cfg!(debug_assertions) {
        assert!(
            cost.draw_list <= budget,
            "{label}: the draw list took {:?}, over {budget:?}",
            cost.draw_list
        );
    }
}

/// The draw-list budgets of a frame of the perf page on the host CPU (release build): at
/// rest, panning at the fit, zooming, panning zoomed in.
struct Budgets([Duration; 4]);

const fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// 5 000 paths, the probe's import leaving them all selected and the pointer over the canvas.
/// Measured on 2026-11-19: 42 ms at rest, 39 ms zooming and 36 ms panning before the selection
/// pass (about 30 ms were lookups over the selection, `specs/0126-performance-pass` plan), 13,
/// 9 and 5 ms after it, 14 ms panning at the fit; held to the budgets of the page with nothing
/// selected.
const BUDGETS_5000: Budgets = Budgets([ms(20), ms(20), ms(15), ms(10)]);

/// 20 000 paths (the fit shows 8 201 of them: the page is four times as tall). Measured on
/// 2026-11-19: 37 ms at rest and panning at the fit, 23 ms zooming, 18 ms panning (11 ms of
/// it work over all objects whatever the view), held to about 1.5 times that.
const BUDGETS_20000: Budgets = Budgets([ms(55), ms(55), ms(35), ms(27)]);

/// Rest; a middle-button drag on a circle of 150 px at the fit (the scale unchanged, the
/// whole page width in view); ten Ctrl+wheel notches about the canvas centre (one per frame,
/// as the probe's wheel-zoom, which ends about four times closer than the fit), then a
/// middle-button drag on a circle of 150 px at that zoom (as the probe's pan-drag).
fn rest_pan_zoom_pan(session: &mut Session, label: &str, budgets: &Budgets) {
    let circle = |session: &mut Session, frame: u32| {
        let angle = f64::from(frame + 1) / 30.0;
        session.pan_to(640.0 + 150.0 * angle.cos(), 400.0 + 150.0 * angle.sin());
    };
    // A first frame warms the caches the document read fills.
    let _ = session.frame_draw_list();
    let rest = measure(session, 5, |_, _| {});
    report(&format!("{label} rest"), &rest, budgets.0[0]);
    let scale = session.view().scale();
    session.begin_pan(790.0, 400.0);
    let fit_pan = measure(session, 10, circle);
    session.end_pan();
    assert!(
        (session.view().scale() - scale).abs() < 1e-12,
        "a pan keeps the scale"
    );
    report(&format!("{label} fit pan"), &fit_pan, budgets.0[1]);
    let zoom = measure(session, 10, |session, _| {
        session.wheel(0.0, -100.0, 640.0, 400.0, false, true);
    });
    report(&format!("{label} zoom"), &zoom, budgets.0[2]);
    println!("zoomed to scale {:.2}", session.view().scale());
    session.begin_pan(790.0, 400.0);
    let pan = measure(session, 10, circle);
    session.end_pan();
    report(&format!("{label} pan"), &pan, budgets.0[3]);
}

fn all_selected(nodes: usize, budgets: &Budgets) {
    let mut session = perf_session(nodes);
    assert_eq!(
        session.selected_object_count(),
        nodes,
        "the import selects its paths"
    );
    rest_pan_zoom_pan(&mut session, &format!("{nodes} all selected"), budgets);
}

fn none_selected(nodes: usize, budgets: &Budgets) {
    let mut session = perf_session(nodes);
    session.escape();
    assert_eq!(
        session.selected_object_count(),
        0,
        "Escape clears the selection"
    );
    rest_pan_zoom_pan(&mut session, &format!("{nodes} none selected"), budgets);
}

#[test]
fn frame_cost_of_the_perf_page_by_phase_all_selected() {
    all_selected(5000, &BUDGETS_5000);
}

#[test]
fn frame_cost_of_the_perf_page_by_phase_none_selected() {
    none_selected(5000, &BUDGETS_5000);
}

#[test]
#[cfg_attr(debug_assertions, ignore = "a release-build measurement")]
fn frame_cost_of_the_20000_path_page_all_selected() {
    all_selected(20_000, &BUDGETS_20000);
}

#[test]
#[cfg_attr(debug_assertions, ignore = "a release-build measurement")]
fn frame_cost_of_the_20000_path_page_none_selected() {
    none_selected(20_000, &BUDGETS_20000);
}
