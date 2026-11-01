//! Grid layout (`specs/0090-grid` criteria 4 to 9); the snapping tests are in `snap_tests.rs`.
#![allow(clippy::float_cmp)] // Exact grid coordinates are the point of these checks.

use std::f64::consts::{FRAC_PI_4, FRAC_PI_6};

use curvyo_document_core::{
    Angle, DocumentSize, GridMode, GridScale, Length, OriginCorner, Page, PageGrid, Point,
    ViewTransform,
};

use super::*;
use crate::viewport::PX_PER_MM_AT_100;

pub(super) const TOL: f64 = 1e-9;

pub(super) fn page_with(grid: PageGrid) -> Page {
    Page {
        id: "first".into(),
        name: "Page 1".into(),
        origin: Point::new(0.0, 0.0),
        size: DocumentSize::from_mm(210.0, 297.0),
        grid,
    }
}

pub(super) fn shown() -> PageGrid {
    PageGrid {
        shown: true,
        ..PageGrid::DEFAULT
    }
}

pub(super) fn scale(zoom: f64) -> f64 {
    zoom * PX_PER_MM_AT_100
}

fn view(zoom: f64) -> ViewTransform {
    ViewTransform::new(scale(zoom), Point::new(0.0, 0.0))
}

fn everything() -> (Point, Point) {
    (Point::new(-1e4, -1e4), Point::new(1e4, 1e4))
}

/// The spacing of the finest lines drawn at full opacity.
fn finest_full_spacing(grid: &PageGrid, zoom: f64) -> f64 {
    let mut full: Vec<f64> = axis_lines(grid, scale(zoom), 0.0, 60.0)
        .into_iter()
        .filter(|l| l.alpha >= 1.0)
        .map(|l| l.pos)
        .collect();
    full.sort_by(f64::total_cmp);
    full.windows(2)
        .map(|w| w[1] - w[0])
        .fold(f64::INFINITY, f64::min)
}

// ---- criterion 5: adaptive levels -------------------------------------------------------

#[test]
fn the_adaptive_level_follows_the_zoom_by_the_subdivision() {
    let grid = shown();
    for (zoom, spacing) in [(0.2, 25.0), (1.0, 5.0), (5.0, 1.0), (25.0, 0.2)] {
        let got = finest_full_spacing(&grid, zoom);
        assert!((got - spacing).abs() < 1e-6, "{zoom}: {got}");
    }
}

#[test]
fn coarser_lines_keep_their_coordinate_at_every_zoom() {
    let grid = shown();
    for zoom in [0.3, 1.0, 2.2, 5.0, 11.0] {
        let lines = axis_lines(&grid, scale(zoom), 0.0, 30.0);
        for coarse in [0.0, 25.0] {
            assert!(
                lines.iter().any(|l| (l.pos - coarse).abs() < TOL),
                "{zoom}: {coarse}"
            );
        }
    }
}

#[test]
fn finer_lines_fade_in_between_4_and_12_px_without_jumps() {
    let grid = shown();
    let alpha_at = |zoom: f64, pos: f64| {
        axis_lines(&grid, scale(zoom), pos - 1e-6, pos + 1e-6)
            .first()
            .map_or(0.0, |l| l.alpha)
    };
    let emphasis_at = |zoom: f64, pos: f64| {
        axis_lines(&grid, scale(zoom), pos - 1e-6, pos + 1e-6)
            .first()
            .map_or(0.0, |l| l.emphasis)
    };
    let mut zoom: f64 = 0.5;
    let mut last = (alpha_at(zoom, 1.0), emphasis_at(zoom, 5.0));
    while zoom < 30.0 {
        zoom *= 1.01;
        let now = (alpha_at(zoom, 1.0), emphasis_at(zoom, 5.0));
        assert!((now.0 - last.0).abs() < 0.1, "alpha jumps at {zoom}");
        assert!((now.1 - last.1).abs() < 0.1, "emphasis jumps at {zoom}");
        last = now;
    }
    // The 1 mm line is 4 px apart at 4 px / (1 mm · 3.78 px/mm) and fully drawn at 12 px.
    assert_eq!(alpha_at(4.0 / PX_PER_MM_AT_100 * 0.999, 1.0), 0.0);
    assert!((alpha_at(8.0 / PX_PER_MM_AT_100, 1.0) - 0.5).abs() < 1e-3);
    assert_eq!(alpha_at(12.0 / PX_PER_MM_AT_100, 1.0), 1.0);
}

#[test]
fn every_subdivision_th_line_is_major() {
    let lines = axis_lines(&shown(), scale(1.0), 0.0, 50.0);
    for line in lines {
        let major = (line.pos / 25.0).round() * 25.0 == line.pos;
        assert_eq!(line.emphasis == 1.0, major, "{}", line.pos);
    }
}

// ---- criterion 6: fixed --------------------------------------------------------------------

#[test]
fn a_fixed_grid_keeps_the_base_spacing_and_thins_out_below_12_px() {
    let grid = PageGrid {
        mode: GridMode::Fixed,
        ..shown()
    };
    for zoom in [1.0, 5.0, 25.0] {
        assert!(
            (finest_full_spacing(&grid, zoom) - 5.0).abs() < 1e-9,
            "{zoom}"
        );
    }
    // 5 mm at 8 px: half opacity minors, majors full.
    let lines = axis_lines(&grid, 8.0 / 5.0, 0.0, 25.0);
    assert!(
        lines
            .iter()
            .any(|l| l.emphasis == 0.0 && (l.alpha - 0.5).abs() < 1e-3)
    );
    // 5 mm under 4 px: only majors.
    let lines = axis_lines(&grid, 3.0 / 5.0, 0.0, 100.0);
    assert!(lines.iter().all(|l| l.emphasis == 1.0));
    assert_eq!(lines.len(), 5);
    // Majors under 4 px too: nothing.
    assert_eq!(axis_lines(&grid, 0.5 / 5.0, 0.0, 100.0).len(), 0);
}

// ---- criterion 7: rotation -------------------------------------------------------------------

#[test]
fn a_rotated_grid_turns_its_lines_clockwise_around_the_origin() {
    let grid = PageGrid {
        angle: Angle::from_radians(FRAC_PI_6),
        ..shown()
    };
    let page = page_with(grid);
    let layout = grid_layout(&page, view(1.0), everything());
    assert_ne!(layout.lines.len(), 0);
    let frame = GridFrame::of(&page);
    for line in &layout.lines {
        let (u0, v0) = frame.to_grid(line.start);
        let (u1, v1) = frame.to_grid(line.end);
        assert!(
            (u0 - u1).abs() < 1e-6 || (v0 - v1).abs() < 1e-6,
            "a line is constant in one grid axis"
        );
    }
    // Clockwise on screen (y down): the u axis points right and down.
    let u = frame.u_axis();
    assert!(u.x > 0.0 && u.y > 0.0);
    assert_eq!(frame.origin, Point::new(0.0, 0.0));
}

// ---- criterion 17: the origin corner -----------------------------------------------------------

/// The horizontal lines' y and the vertical lines' x of a fixed 5 mm grid on the 210 x 297 page.
fn axis_positions(origin: OriginCorner) -> (Vec<f64>, Vec<f64>) {
    let page = page_with(PageGrid {
        mode: GridMode::Fixed,
        origin,
        ..shown()
    });
    let layout = grid_layout(&page, view(4.0), everything());
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for line in &layout.lines {
        if (line.start.x - line.end.x).abs() < 1e-9 {
            xs.push(line.start.x);
        } else {
            ys.push(line.start.y);
        }
    }
    (xs, ys)
}

#[test]
fn the_grid_starts_at_the_origin_corner_and_runs_along_its_two_edges() {
    let has = |values: &[f64], at: f64| values.iter().any(|v| (v - at).abs() < 1e-9);
    // 297 is no multiple of 5: only a bottom corner puts lines on 297 and 292.
    let (xs, ys) = axis_positions(OriginCorner::TopLeft);
    assert!(has(&xs, 0.0) && has(&ys, 0.0) && has(&ys, 295.0) && !has(&ys, 292.0));
    let (xs, ys) = axis_positions(OriginCorner::BottomLeft);
    assert!(has(&xs, 0.0) && has(&ys, 297.0) && has(&ys, 292.0) && !has(&ys, 295.0));
    // 210 is a multiple of 5, so a right corner keeps the same columns.
    let (xs, ys) = axis_positions(OriginCorner::TopRight);
    assert!(has(&xs, 210.0) && has(&xs, 0.0) && has(&ys, 0.0) && !has(&ys, 292.0));
    let (xs, ys) = axis_positions(OriginCorner::BottomRight);
    assert!(has(&xs, 210.0) && has(&ys, 297.0) && has(&ys, 2.0));
}

#[test]
fn the_grid_axes_point_into_the_page_from_the_origin_corner() {
    for (origin, zero, u, v) in [
        (OriginCorner::TopLeft, (0.0, 0.0), (1.0, 0.0), (0.0, 1.0)),
        (
            OriginCorner::TopRight,
            (210.0, 0.0),
            (-1.0, 0.0),
            (0.0, 1.0),
        ),
        (
            OriginCorner::BottomLeft,
            (0.0, 297.0),
            (1.0, 0.0),
            (0.0, -1.0),
        ),
        (
            OriginCorner::BottomRight,
            (210.0, 297.0),
            (-1.0, 0.0),
            (0.0, -1.0),
        ),
    ] {
        let frame = GridFrame::of(&page_with(PageGrid { origin, ..shown() }));
        assert_eq!(frame.origin, Point::new(zero.0, zero.1), "{origin:?}");
        assert!((frame.u_axis().x - u.0).abs() < 1e-12 && (frame.u_axis().y - u.1).abs() < 1e-12);
        assert!((frame.v_axis().x - v.0).abs() < 1e-12 && (frame.v_axis().y - v.1).abs() < 1e-12);
        // A point inside the page has positive grid coordinates.
        let (gu, gv) = frame.to_grid(Point::new(30.0, 40.0));
        assert!(gu > 0.0 && gv > 0.0, "{origin:?}");
    }
}

// ---- criterion 8: logarithmic -------------------------------------------------------------------

#[test]
fn a_log_grid_repeats_decades_with_lines_at_log10_n() {
    let grid = PageGrid {
        scale: GridScale::Logarithmic,
        mode: GridMode::Fixed,
        ..shown()
    };
    // Decade 25 mm; at 400 % a decade is 378 px, every inner gap above 12 px.
    let lines = axis_lines(&grid, scale(4.0), -25.0, 25.0);
    for d in [-25.0, 0.0, 25.0] {
        let decade = lines.iter().find(|l| (l.pos - d).abs() < TOL).unwrap();
        assert_eq!(decade.emphasis, 1.0);
    }
    for n in 2..10 {
        let pos = f64::from(n).log10() * 25.0;
        assert!(lines.iter().any(|l| (l.pos - pos).abs() < TOL), "{n}");
        assert!(
            lines.iter().any(|l| (l.pos - (pos - 25.0)).abs() < TOL),
            "-{n}"
        );
    }
    assert_eq!(lines.len(), 2 * 8 + 3, "fixed mode does not refine");
}

#[test]
fn an_adaptive_log_grid_refines_cells_wider_than_48_px() {
    let grid = PageGrid {
        scale: GridScale::Logarithmic,
        ..shown()
    };
    let one_one = 1.1_f64.log10() * 25.0;
    let at = |zoom: f64| {
        axis_lines(&grid, scale(zoom), 0.0, 25.0)
            .into_iter()
            .find(|l| (l.pos - one_one).abs() < TOL)
            .map_or(0.0, |l| l.alpha)
    };
    // Cell [1, 2] is 25 · log10(2) mm = 7.5 mm: 48 px at 169 %.
    assert_eq!(at(1.6), 0.0);
    assert!(at(6.0) > 0.99);
}

#[test]
fn log_lines_closer_than_4_px_are_left_out() {
    let grid = PageGrid {
        scale: GridScale::Logarithmic,
        ..shown()
    };
    for zoom in [0.02, 0.1, 0.5, 1.0, 3.0, 20.0, 80.0] {
        let mut lines: Vec<f64> = axis_lines(&grid, scale(zoom), -300.0, 300.0)
            .into_iter()
            .map(|l| l.pos)
            .collect();
        lines.sort_by(f64::total_cmp);
        for w in lines.windows(2) {
            assert!((w[1] - w[0]) * scale(zoom) >= 4.0 - 1e-6, "{zoom}: {w:?}");
        }
    }
}

// ---- criteria 4 and 9: clipping and bounded cost ---------------------------------------------------

#[test]
fn a_hidden_grid_draws_nothing_and_a_shown_one_stays_inside_its_page() {
    let page = page_with(PageGrid::DEFAULT);
    assert_eq!(grid_layout(&page, view(1.0), everything()).lines.len(), 0);
    let page = page_with(PageGrid {
        angle: Angle::from_radians(FRAC_PI_4),
        ..shown()
    });
    let visible = (Point::new(50.0, -30.0), Point::new(400.0, 100.0));
    let layout = grid_layout(&page, view(1.0), visible);
    assert_ne!(layout.lines.len(), 0);
    for line in layout.lines {
        for p in [line.start, line.end] {
            assert!(p.x >= 50.0 - TOL && p.x <= 210.0 + TOL, "{p:?}");
            assert!(p.y >= -TOL && p.y <= 100.0 + TOL, "{p:?}");
        }
    }
}

#[test]
fn the_line_count_is_bounded_by_the_screen_at_every_zoom() {
    let (w, h): (f64, f64) = (3840.0, 2160.0);
    let budget = 2.0 * (w.hypot(h) / 4.0) + 64.0;
    let page = |grid: PageGrid| Page {
        size: DocumentSize::from_mm(10_000.0, 10_000.0),
        ..page_with(grid)
    };
    for grid in [
        shown(),
        PageGrid {
            angle: Angle::from_radians(FRAC_PI_4),
            ..shown()
        },
        PageGrid {
            subdivision: 2,
            spacing: Length::from_mm(0.01),
            ..shown()
        },
        PageGrid {
            scale: GridScale::Logarithmic,
            angle: Angle::from_radians(0.3),
            ..shown()
        },
        PageGrid {
            mode: GridMode::Fixed,
            spacing: Length::from_mm(0.5),
            ..shown()
        },
    ] {
        let page = page(grid);
        let mut zoom = 0.02;
        while zoom <= 80.0 {
            let s = scale(zoom);
            let visible = (
                Point::new(100.0, 100.0),
                Point::new(100.0 + w / s, 100.0 + h / s),
            );
            let count = grid_layout(&page, view(zoom), visible).lines.len();
            assert!(
                f64::from(u32::try_from(count).unwrap()) <= budget,
                "{grid:?} {zoom}: {count}"
            );
            zoom *= 1.7;
        }
    }
}
