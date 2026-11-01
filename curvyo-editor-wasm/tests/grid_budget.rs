//! The grid's frame budget (`specs/0090-grid` criterion 9, `adrs.md` decision 6): with 10,000
//! objects on a 3840 x 2160 canvas, a frame with the grid shown costs at most 1 ms of thread CPU
//! more than the same frame with it hidden, at the worst zooms and settings, and a grid snap with
//! the Nodes choice stays within the same budget. Timed by thread CPU (`tests/cpu_time/`), the
//! cheapest of five runs. Like the other budgets, asserted in a release build (CI's
//! `performance-budgets` job); a debug build only prints.

#![allow(clippy::unwrap_used)]

mod cpu_time;

use std::hint::black_box;

use curvyo_document_core::{
    Angle, Document, DocumentBackground, DocumentSize, DocumentSnapping, GridMode, GridScale,
    GridSnapPoint, Length, Page, PageGrid, Point, RectBounds, ViewTransform, pack,
};
use curvyo_editor_wasm::Session;
use curvyo_ui_core::{
    GridSnapInput, PX_PER_MM_AT_100, SnapProbe, SnapSettings, SnapState, SnapTargets, grid_layout,
    resolve_with_grid,
};

const CANVAS: (f64, f64) = (3840.0, 2160.0);
const FRAMES: u32 = 50;
const RUNS: u32 = 5;
const BUDGET_MS: f64 = 1.0;

fn scene() -> Session {
    let document = Document::new(9);
    document.batch("scene", |doc| {
        for index in 0..10_000 {
            let x = f64::from(index % 100) * 2.0;
            let y = f64::from(index / 100) * 2.9;
            let _ = doc.create_rect(RectBounds::from_corners(
                Point::new(x, y),
                Point::new(x + 1.0, y + 1.0),
            ));
        }
    });
    let mut session = Session::open(10, &pack(&document, "budget").unwrap()).unwrap();
    session.resize_viewport(CANVAS.0, CANVAS.1);
    session
}

/// The grid settings that cost the most: a level in its fade band, a rotated grid whose clipped
/// lines are long, an adaptive log grid with refinement, a dense fixed grid at its thinning edge.
fn worst_cases() -> [(PageGrid, f64); 5] {
    let shown = PageGrid {
        shown: true,
        ..PageGrid::DEFAULT
    };
    [
        (shown, 2.2),
        (
            PageGrid {
                angle: Angle::from_radians(std::f64::consts::FRAC_PI_4),
                ..shown
            },
            2.2,
        ),
        (
            PageGrid {
                scale: GridScale::Logarithmic,
                angle: Angle::from_radians(0.3),
                ..shown
            },
            9.0,
        ),
        (
            PageGrid {
                mode: GridMode::Fixed,
                spacing: Length::from_mm(1.1),
                ..shown
            },
            1.0,
        ),
        (
            PageGrid {
                subdivision: 2,
                ..shown
            },
            0.02,
        ),
    ]
}

/// What `frame_draw_list` adds for one shown grid: the layout and its triangles.
fn grid_frame(page: &Page, zoom: f64) -> usize {
    let scale = zoom * PX_PER_MM_AT_100;
    let view = ViewTransform::new(scale, Point::new(-5.0, -5.0));
    let visible = (
        view.screen_to_document(0.0, 0.0),
        view.screen_to_document(CANVAS.0, CANVAS.1),
    );
    let strokes: Vec<_> = grid_layout(page, view, visible)
        .lines
        .into_iter()
        .map(|line| curvyo_render_core::GridStroke {
            start: line.start,
            end: line.end,
            emphasis: line.emphasis,
            alpha: line.alpha,
        })
        .collect();
    curvyo_render_core::build_grid_lines(
        &strokes,
        curvyo_render_core::grid_ink(DocumentBackground::DEFAULT),
        view,
        1.0,
    )
    .triangles
    .len()
}

#[test]
fn a_shown_grid_costs_at_most_one_ms_per_frame_with_ten_thousand_objects() {
    let session = scene();
    for (grid, zoom) in worst_cases() {
        let mut page = session.active_page();
        page.grid = grid;
        // A page larger than the canvas at every zoom, so the clip never shortens the work.
        page.size = DocumentSize::from_mm(100_000.0, 100_000.0);
        // The cheapest of five runs, as the page budgets (`tests/pages.rs`): a shared runner's
        // preemption inflates one run, never all five.
        let mut vertices = 0;
        let mut time = std::time::Duration::MAX;
        for _ in 0..RUNS {
            let (run_vertices, run_time) = cpu_time::cpu_time_of(|| {
                let mut total = 0;
                for _ in 0..FRAMES {
                    total += black_box(grid_frame(&page, zoom));
                }
                total
            });
            vertices = run_vertices;
            time = time.min(run_time);
        }
        let per_frame = time.as_secs_f64() * 1000.0 / f64::from(FRAMES);
        let per_frame_vertices = vertices / FRAMES as usize;
        eprintln!("{grid:?} at {zoom}: {per_frame:.3} ms/frame, {per_frame_vertices} vertices");
        assert!(vertices > 0, "{grid:?}: the grid is drawn");
        if !cfg!(debug_assertions) {
            assert!(
                per_frame <= BUDGET_MS,
                "{grid:?} at {zoom}: {per_frame} ms/frame"
            );
        }
    }
}

#[test]
fn the_session_frame_with_the_grid_shown_carries_exactly_the_grid_layer_more() {
    let mut session = scene();
    let hidden = session.frame_draw_list();
    assert!(session.set_active_page_grid_shown(true));
    let shown = session.frame_draw_list();
    assert!(shown.triangles.len() > hidden.triangles.len());
    assert_eq!(shown.layers().len(), hidden.layers().len() + 1);
}

#[test]
fn a_grid_snap_with_the_nodes_choice_stays_within_the_budget() {
    let session = scene();
    let mut page = session.active_page();
    page.grid = PageGrid {
        shown: true,
        angle: Angle::from_radians(0.4),
        ..PageGrid::DEFAULT
    };
    let pages = [page];
    let settings = SnapSettings::from_document(
        DocumentSnapping {
            enabled: true,
            grid_point: GridSnapPoint::Nodes,
            ..DocumentSnapping::DEFAULT
        },
        1.0 / PX_PER_MM_AT_100,
    );
    // A moved selection of 200 rectangles: 800 nodes, every one a grid probe point.
    let nodes: Vec<Point> = (0..800)
        .map(|i| Point::new(f64::from(i % 40) * 2.0 + 0.3, f64::from(i / 40) * 2.9 + 0.2))
        .collect();
    let targets = SnapTargets::default();
    let mut time = std::time::Duration::MAX;
    for _ in 0..RUNS {
        let ((), run_time) = cpu_time::cpu_time_of(|| {
            let mut state = SnapState::default();
            for frame in 0..FRAMES {
                let shift = f64::from(frame) * 0.013;
                let moved: Vec<Point> = nodes
                    .iter()
                    .map(|p| Point::new(p.x + shift, p.y + shift))
                    .collect();
                let grid = GridSnapInput {
                    pages: &pages,
                    points: &moved,
                    scale: PX_PER_MM_AT_100,
                };
                black_box(resolve_with_grid(
                    &settings,
                    &targets,
                    &SnapProbe::default(),
                    &grid,
                    None,
                    &mut state,
                ));
            }
        });
        time = time.min(run_time);
    }
    let per_frame = time.as_secs_f64() * 1000.0 / f64::from(FRAMES);
    eprintln!("grid snap, 800 nodes: {per_frame:.3} ms/frame");
    if !cfg!(debug_assertions) {
        assert!(per_frame <= BUDGET_MS, "{per_frame} ms/frame");
    }
}
