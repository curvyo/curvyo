//! Independent acceptance tests for `specs/0090-grid` criteria 4 to 9 (grid layout) and 12 to 15
//! (grid snap resolution), written from the specification before the implementation was read.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::assert_is_empty,
    clippy::many_single_char_names
)]

use std::f64::consts::PI;

use curvyo_document_core::{
    Angle, DocumentSize, DocumentSnapping, GridMode, GridScale, Length, OriginCorner, Page,
    PageGrid, Point, ViewTransform,
};
use curvyo_ui_core::{
    GridFeature, GridLine, GridSnapInput, PX_PER_MM_AT_100, SnapProbe, SnapSettings, SnapState,
    SnapTarget, SnapTargets, TargetKind, grid_layout, resolve_with_grid, snap_to_grid,
};

const EXACT: f64 = 1e-9;
const FULL: f32 = 1.0 - 1e-6;

fn page(origin: Point, w: f64, h: f64, grid: PageGrid) -> Page {
    Page {
        id: "p".to_owned(),
        name: "P".to_owned(),
        origin,
        size: DocumentSize::from_mm(w, h),
        grid,
    }
}

fn shown() -> PageGrid {
    PageGrid {
        shown: true,
        ..PageGrid::DEFAULT
    }
}

fn scale_of(zoom: f64) -> f64 {
    zoom * PX_PER_MM_AT_100
}

/// The view at `zoom` with document point `origin` at the screen's top-left corner, and the
/// visible rectangle of a `w` x `h` px canvas.
fn view(zoom: f64, origin: Point, w: f64, h: f64) -> (ViewTransform, (Point, Point)) {
    let s = scale_of(zoom);
    (
        ViewTransform::new(s, origin),
        (origin, Point::new(origin.x + w / s, origin.y + h / s)),
    )
}

fn is_vertical(l: &GridLine) -> bool {
    (l.start.x - l.end.x).abs() < 1e-9 && (l.start.y - l.end.y).abs() > 1e-9
}

/// Sorted x of vertical lines with `alpha >= min_alpha`.
fn vertical_xs(lines: &[GridLine], min_alpha: f32) -> Vec<f64> {
    let mut xs: Vec<f64> = lines
        .iter()
        .filter(|l| is_vertical(l) && l.alpha >= min_alpha)
        .map(|l| l.start.x)
        .collect();
    xs.sort_by(f64::total_cmp);
    xs.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    xs
}

fn min_gap(xs: &[f64]) -> f64 {
    xs.windows(2)
        .map(|w| w[1] - w[0])
        .fold(f64::INFINITY, f64::min)
}

fn has(xs: &[f64], x: f64) -> bool {
    xs.iter().any(|v| (v - x).abs() < 1e-6)
}

fn line_at(lines: &[GridLine], x: f64) -> Option<GridLine> {
    lines
        .iter()
        .copied()
        .find(|l| is_vertical(l) && (l.start.x - x).abs() < 1e-6)
}

fn big_page(grid: PageGrid) -> Page {
    page(Point::new(-1000.0, -1000.0), 3000.0, 3000.0, grid)
}

// ---------------------------------------------------------------- criterion 4

#[test]
fn ac4_hidden_grid_draws_nothing() {
    let p = big_page(PageGrid::DEFAULT);
    for zoom in [0.02, 1.0, 80.0] {
        let (v, vis) = view(zoom, Point::new(0.0, 0.0), 1920.0, 1080.0);
        assert!(grid_layout(&p, v, vis).lines.is_empty());
    }
}

#[test]
fn ac4_lines_lie_inside_the_page_rectangle_and_the_visible_rectangle_only() {
    for angle in [0.0, 30.0, 90.0, 137.0] {
        for scale in [GridScale::Linear, GridScale::Logarithmic] {
            let grid = PageGrid {
                angle: Angle::from_radians(f64::to_radians(angle)),
                scale,
                origin: OriginCorner::BottomRight,
                ..shown()
            };
            let p = page(Point::new(-40.0, 25.0), 120.0, 80.0, grid);
            // Visible area overlaps the page partly.
            let (v, vis) = view(2.0, Point::new(-60.0, 40.0), 900.0, 500.0);
            let lines = grid_layout(&p, v, vis).lines;
            assert!(!lines.is_empty(), "angle {angle} {scale:?}");
            let lo = Point::new(p.origin.x.max(vis.0.x), p.origin.y.max(vis.0.y));
            let hi = Point::new(
                (p.origin.x + 120.0).min(vis.1.x),
                (p.origin.y + 80.0).min(vis.1.y),
            );
            for l in &lines {
                for q in [l.start, l.end] {
                    assert!(
                        q.x >= lo.x - 1e-6
                            && q.y >= lo.y - 1e-6
                            && q.x <= hi.x + 1e-6
                            && q.y <= hi.y + 1e-6,
                        "angle {angle}: {q:?} outside {lo:?}..{hi:?}"
                    );
                }
                assert!((0.0..=1.0).contains(&l.alpha));
                assert!((0.0..=1.0).contains(&l.emphasis));
            }
        }
    }
}

#[test]
fn ac4_a_page_outside_the_visible_rectangle_draws_nothing() {
    let p = page(Point::new(5000.0, 5000.0), 100.0, 100.0, shown());
    let (v, vis) = view(1.0, Point::new(0.0, 0.0), 1920.0, 1080.0);
    assert!(grid_layout(&p, v, vis).lines.is_empty());
}

#[test]
fn ac4_major_lines_are_stronger_than_minor_lines() {
    let p = big_page(shown());
    let (v, vis) = view(5.0, Point::new(0.0, 0.0), 1920.0, 1080.0);
    let lines = grid_layout(&p, v, vis).lines;
    let major = line_at(&lines, 5.0).unwrap();
    let minor = line_at(&lines, 1.0).unwrap();
    assert!(major.emphasis > minor.emphasis, "{major:?} vs {minor:?}");
}

// ---------------------------------------------------------------- criterion 5

#[test]
fn ac5_adaptive_minor_spacing_follows_the_zoom_25_5_1_0_2_mm() {
    let p = big_page(shown());
    for (zoom, spacing) in [(0.2, 25.0), (1.0, 5.0), (5.0, 1.0), (25.0, 0.2)] {
        let (v, vis) = view(zoom, Point::new(0.0, 0.0), 1920.0, 1080.0);
        let lines = grid_layout(&p, v, vis).lines;
        let full = vertical_xs(&lines, FULL);
        let gap = min_gap(&full);
        assert!(
            (gap - spacing).abs() < 1e-6,
            "zoom {zoom}: full-opacity spacing {gap}, expected {spacing}"
        );
        // Every full line sits on a multiple of the spacing from the origin.
        for x in &full {
            let k = x / spacing;
            assert!((k - k.round()).abs() < 1e-6, "zoom {zoom}: line at {x}");
        }
    }
}

#[test]
fn ac5_finer_lines_fade_in_between_levels() {
    let p = big_page(shown());
    // At 200 %, 1 mm is 7.56 px: inside the 4..12 px fade band.
    let (v, vis) = view(2.0, Point::new(0.0, 0.0), 1920.0, 1080.0);
    let lines = grid_layout(&p, v, vis).lines;
    let fading = line_at(&lines, 1.0).expect("1 mm line is fading in at 200 %");
    assert!(fading.alpha > 0.0 && fading.alpha < 1.0, "{fading:?}");
    let expected = ((2.0 * PX_PER_MM_AT_100 - 4.0) / 8.0) as f32;
    assert!(
        (fading.alpha - expected).abs() < 0.02,
        "alpha {} expected about {expected}",
        fading.alpha
    );
    assert!(line_at(&lines, 5.0).unwrap().alpha >= FULL);
}

#[test]
fn ac5_zooming_never_makes_a_line_jump_in_opacity_or_weight() {
    let p = big_page(shown());
    let probes = [0.2, 1.0, 5.0, 25.0, 0.04, 3.0];
    let mut zoom: f64 = 0.03;
    let mut previous: Option<Vec<(f32, f32)>> = None;
    while zoom < 60.0 {
        let (v, vis) = view(zoom, Point::new(-0.5, -0.5), 12000.0, 1080.0);
        let lines = grid_layout(&p, v, vis).lines;
        let now: Vec<(f32, f32)> = probes
            .iter()
            .map(|x| line_at(&lines, *x).map_or((0.0, 0.0), |l| (l.alpha, l.emphasis)))
            .collect();
        if let Some(before) = &previous {
            for ((a, b), x) in before.iter().zip(&now).zip(probes) {
                assert!(
                    (a.0 - b.0).abs() < 0.05,
                    "line {x} alpha jumps {} -> {} at zoom {zoom}",
                    a.0,
                    b.0
                );
                // Weight only matters for a visible line.
                if a.0 > 0.0 && b.0 > 0.0 {
                    assert!(
                        (a.1 - b.1).abs() < 0.05,
                        "line {x} emphasis jumps {} -> {} at zoom {zoom}",
                        a.1,
                        b.1
                    );
                }
            }
        }
        previous = Some(now);
        zoom *= 1.002;
    }
}

#[test]
fn ac5_levels_nest_a_coarse_line_keeps_its_document_coordinate_at_every_zoom() {
    let grid = PageGrid {
        origin: OriginCorner::TopLeft,
        ..shown()
    };
    // The page's top-left corner, the grid origin, sits 1.25 mm off any 5 mm line of document 0.
    let p = page(Point::new(-998.75, -1000.0), 3000.0, 3000.0, grid);
    let (v, vis) = view(0.2, Point::new(0.0, 0.0), 1920.0, 1080.0);
    let coarse = vertical_xs(&grid_layout(&p, v, vis).lines, FULL);
    assert!(!coarse.is_empty());
    for zoom in [0.3, 0.7, 1.0, 1.9, 4.99, 5.0, 12.0, 25.0] {
        let (v, vis) = view(zoom, Point::new(0.0, 0.0), 1920.0, 1080.0);
        let xs = vertical_xs(&grid_layout(&p, v, vis).lines, FULL);
        let right = vis.1.x;
        for x in coarse.iter().filter(|x| **x <= right) {
            assert!(has(&xs, *x), "coarse line {x} missing at zoom {zoom}");
        }
    }
}

// ---------------------------------------------------------------- criterion 6

#[test]
fn ac6_fixed_grid_keeps_the_base_spacing_and_thins_out_when_dense() {
    let grid = PageGrid {
        mode: GridMode::Fixed,
        ..shown()
    };
    let p = big_page(grid);
    // 100 % and 2500 %: always 5 mm, never finer.
    for zoom in [1.0, 25.0, 80.0] {
        let (v, vis) = view(zoom, Point::new(0.0, 0.0), 1920.0, 1080.0);
        let lines = grid_layout(&p, v, vis).lines;
        let all = vertical_xs(&lines, f32::MIN_POSITIVE);
        assert!(
            (min_gap(&all) - 5.0).abs() < 1e-6,
            "zoom {zoom}: fixed spacing {}",
            min_gap(&all)
        );
    }
    // 5 mm at 8 px: minors half faded.
    let zoom = 8.0 / (5.0 * PX_PER_MM_AT_100);
    let (v, vis) = view(zoom, Point::new(0.0, 0.0), 1920.0, 1080.0);
    let lines = grid_layout(&p, v, vis).lines;
    let minor = line_at(&lines, 5.0).expect("minor present at 8 px");
    assert!(minor.alpha > 0.0 && minor.alpha < 1.0, "{minor:?}");
    // 5 mm at 3 px: minors left out, majors (25 mm at 15 px) remain.
    let zoom = 3.0 / (5.0 * PX_PER_MM_AT_100);
    let (v, vis) = view(zoom, Point::new(0.0, 0.0), 1920.0, 1080.0);
    let lines = grid_layout(&p, v, vis).lines;
    assert!(
        line_at(&lines, 5.0).is_none(),
        "minor below 4 px is left out"
    );
    assert!(line_at(&lines, 25.0).is_some(), "major still drawn");
    // 25 mm majors at 3 px: nothing at all.
    let zoom = 3.0 / (25.0 * PX_PER_MM_AT_100);
    let (v, vis) = view(zoom.max(0.02), Point::new(0.0, 0.0), 1920.0, 1080.0);
    let lines = grid_layout(&p, v, vis).lines;
    assert!(
        lines.iter().all(|l| l.alpha == 0.0) || lines.is_empty(),
        "nothing below 4 px"
    );
}

// ---------------------------------------------------------------- criterion 7

#[test]
fn ac7_rotation_is_clockwise_around_the_grid_origin_with_unchanged_spacing() {
    // From a corner origin only the line family that runs into the page passes through the
    // origin: at +30° the lines of constant u, at -30° those of constant v.
    for (degrees, u_through_origin) in [(30.0_f64, true), (-30.0, false)] {
        rotated_grid_lines_sit_on_the_rotated_axes(degrees.to_radians(), u_through_origin);
    }
}

fn rotated_grid_lines_sit_on_the_rotated_axes(a: f64, u_through_origin: bool) {
    let grid = PageGrid {
        angle: Angle::from_radians(a),
        origin: OriginCorner::BottomLeft,
        ..shown()
    };
    // The bottom-left corner, the grid origin, is at (50, 50).
    let p = page(Point::new(50.0, -150.0), 200.0, 200.0, grid);
    let origin = Point::new(50.0, 50.0);
    let (v, vis) = view(1.0, Point::new(0.0, 0.0), 1920.0, 1080.0);
    let lines = grid_layout(&p, v, vis).lines;
    assert!(!lines.is_empty());
    // Clockwise on a y-down screen: the u axis is (cos a, sin a), the v axis (-sin a, cos a).
    let u = (a.cos(), a.sin());
    let vax = (-a.sin(), a.cos());
    let mut u_lines = Vec::new(); // lines of constant u (running along v)
    let mut v_lines = Vec::new();
    for l in lines.iter().filter(|l| l.alpha >= FULL) {
        let d = (l.end.x - l.start.x, l.end.y - l.start.y);
        let len = d.0.hypot(d.1);
        if len < 1e-6 {
            continue;
        }
        let d = (d.0 / len, d.1 / len);
        let along_u = (d.0 * u.0 + d.1 * u.1).abs();
        let along_v = (d.0 * vax.0 + d.1 * vax.1).abs();
        let rel = (l.start.x - origin.x, l.start.y - origin.y);
        if (along_v - 1.0).abs() < 1e-9 {
            u_lines.push(rel.0 * u.0 + rel.1 * u.1);
        } else if (along_u - 1.0).abs() < 1e-9 {
            v_lines.push(rel.0 * vax.0 + rel.1 * vax.1);
        } else {
            panic!("line {l:?} is not along the rotated grid axes");
        }
    }
    for set in [&mut u_lines, &mut v_lines] {
        assert!(set.len() > 3);
        for c in set.iter() {
            let k = c / 5.0;
            assert!((k - k.round()).abs() < 1e-6, "coordinate {c} not on 5 mm");
        }
        set.sort_by(f64::total_cmp);
        set.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        assert!((min_gap(set) - 5.0).abs() < 1e-6);
    }
    // A line through the grid origin exists in the family that runs into the page.
    let through = |set: &[f64]| set.iter().any(|c| c.abs() < 1e-6);
    assert!(if u_through_origin {
        through(&u_lines)
    } else {
        through(&v_lines)
    });
}

// ---------------------------------------------------------------- criterion 8

fn log_grid(mode: GridMode) -> PageGrid {
    PageGrid {
        scale: GridScale::Logarithmic,
        mode,
        ..shown()
    }
}

/// A page whose left edge, the log grid's origin, is at x = 50 mm: decades start at 50, 75, 100.
fn log_page(grid: PageGrid) -> Page {
    page(Point::new(50.0, -1000.0), 3000.0, 3000.0, grid)
}

#[test]
fn ac8_log_lines_sit_at_log10_n_of_each_repeating_decade() {
    // Decade = 5 mm x 5 = 25 mm. At 1000 % a decade is 945 px.
    let p = log_page(log_grid(GridMode::Fixed));
    let (v, vis) = view(10.0, Point::new(60.0, 0.0), 4000.0, 1080.0);
    let lines = grid_layout(&p, v, vis).lines;
    let xs = vertical_xs(&lines, f32::MIN_POSITIVE);
    for decade in [-1.0_f64, 0.0] {
        let start = 100.0 + decade * 25.0;
        for n in 1..=10 {
            let x = start + f64::from(n).log10() * 25.0;
            assert!(has(&xs, x), "decade {decade}: n = {n} line at {x} missing");
        }
        let decade_line = line_at(&lines, start).unwrap();
        assert!(decade_line.emphasis >= FULL, "decade line is major");
        let inner = line_at(&lines, start + 2.0_f64.log10() * 25.0).unwrap();
        assert!(inner.emphasis < decade_line.emphasis);
    }
    // Fixed mode adds no subdivision lines.
    assert!(!has(&xs, 100.0 + 1.5_f64.log10() * 25.0));
    // Repeat, not mirror: the decade left of the origin has its dense end on the right.
    assert!(has(&xs, 75.0 + 9.0_f64.log10() * 25.0));
    assert!(!has(&xs, 100.0 - 9.0_f64.log10() * 25.0));
}

#[test]
fn ac8_adaptive_log_adds_the_next_subdivision_in_cells_wider_than_48_px() {
    let p = log_page(log_grid(GridMode::Adaptive));
    let (v, vis) = view(10.0, Point::new(60.0, 0.0), 4000.0, 1080.0);
    let xs = vertical_xs(&grid_layout(&p, v, vis).lines, f32::MIN_POSITIVE);
    // Cell n = 1..2 is 284 px wide: 1.1 .. 1.9 appear.
    for tenth in [1.1_f64, 1.5, 1.9] {
        assert!(
            has(&xs, 100.0 + tenth.log10() * 25.0),
            "n = {tenth} missing in a 284 px cell"
        );
    }
    // At 100 %, a decade is 94.5 px; cell 1..2 is 28 px (< 48): no tenths.
    let (v, vis) = view(1.0, Point::new(60.0, 0.0), 1920.0, 1080.0);
    let xs = vertical_xs(&grid_layout(&p, v, vis).lines, f32::MIN_POSITIVE);
    assert!(!has(&xs, 100.0 + 1.5_f64.log10() * 25.0));
    assert!(has(&xs, 100.0 + 2.0_f64.log10() * 25.0));
}

// ---------------------------------------------------------------- criterion 17

fn horizontal_ys(lines: &[GridLine]) -> Vec<f64> {
    lines
        .iter()
        .filter(|l| (l.start.y - l.end.y).abs() < 1e-9)
        .map(|l| l.start.y)
        .collect()
}

#[test]
fn ac17_the_grid_starts_at_the_origin_corner_and_log_decades_grow_into_the_page() {
    // An A4 page at (0, 0): 297 is no multiple of 5, so only a bottom corner has lines on the
    // bottom edge and 5 mm above it.
    let (v, vis) = view(2.0, Point::new(-5.0, -5.0), 4000.0, 4000.0);
    for (origin, bottom) in [
        (OriginCorner::TopLeft, false),
        (OriginCorner::TopRight, false),
        (OriginCorner::BottomLeft, true),
        (OriginCorner::BottomRight, true),
    ] {
        let p = a4(PageGrid {
            origin,
            mode: GridMode::Fixed,
            ..shown()
        });
        let ys = horizontal_ys(&grid_layout(&p, v, vis).lines);
        assert_eq!(has(&ys, 297.0), bottom, "{origin:?}");
        assert_eq!(has(&ys, 292.0), bottom, "{origin:?}");
        assert_eq!(has(&ys, 295.0), !bottom, "{origin:?}");
    }
    // A logarithmic grid from the bottom-left corner: its first decade runs upward from y = 297.
    let p = a4(PageGrid {
        origin: OriginCorner::BottomLeft,
        ..log_grid(GridMode::Fixed)
    });
    let (v, vis) = view(10.0, Point::new(0.0, 240.0), 1000.0, 4000.0);
    let ys = horizontal_ys(&grid_layout(&p, v, vis).lines);
    for n in 1..=10 {
        let y = 297.0 - f64::from(n).log10() * 25.0;
        assert!(has(&ys, y), "n = {n} line at {y} missing");
    }
    assert!(!has(&ys, 297.0 - 25.0 + 3.0_f64.log10() * 25.0));
}

// ------------------------------------------------- criteria 6, 8, 9 (bounds)

#[test]
fn ac6_ac8_ac9_drawn_neighbours_are_never_closer_than_4_px() {
    for mode in [GridMode::Adaptive, GridMode::Fixed] {
        for scale in [GridScale::Linear, GridScale::Logarithmic] {
            for zoom in [0.02, 0.05, 0.13, 0.4, 1.0, 2.2, 7.0, 31.0, 80.0] {
                let grid = PageGrid {
                    mode,
                    scale,
                    ..shown()
                };
                let p = big_page(grid);
                let (v, vis) = view(zoom, Point::new(-3.0, -3.0), 1920.0, 1080.0);
                let xs = vertical_xs(&grid_layout(&p, v, vis).lines, f32::MIN_POSITIVE);
                if xs.len() < 2 {
                    continue;
                }
                let gap_px = min_gap(&xs) * scale_of(zoom);
                assert!(
                    gap_px >= 4.0 - 1e-6,
                    "{mode:?} {scale:?} zoom {zoom}: neighbours {gap_px} px apart"
                );
            }
        }
    }
}

#[test]
fn ac9_line_count_is_bounded_by_the_screen_not_the_page_or_zoom() {
    let (w, h) = (3840.0, 2160.0);
    let diagonal = f64::hypot(w, h);
    // Per family at most extent/4 px lines per level, levels form a geometric series (x2).
    let bound = (2.0 * 2.0 * diagonal / 4.0 * 1.25) as usize + 16;
    for angle in [0.0, 17.0, 45.0] {
        for mode in [GridMode::Adaptive, GridMode::Fixed] {
            for scale in [GridScale::Linear, GridScale::Logarithmic] {
                for spacing in [1e-6, 5.0, 1e6] {
                    let grid = PageGrid {
                        mode,
                        scale,
                        spacing: Length::from_mm(spacing),
                        angle: Angle::from_radians(f64::to_radians(angle)),
                        ..shown()
                    };
                    let huge = page(Point::new(-1e7, -1e7), 2e7, 2e7, grid);
                    for zoom in [0.02, 0.37, 1.0, 13.0, 80.0] {
                        let (v, vis) = view(zoom, Point::new(12.3, 45.6), w, h);
                        let n = grid_layout(&huge, v, vis).lines.len();
                        assert!(
                            n <= bound,
                            "{angle}° {mode:?} {scale:?} spacing {spacing} zoom {zoom}: {n} lines > {bound}"
                        );
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------- criteria 12 to 15

fn settings(scale: f64) -> SnapSettings {
    SnapSettings::from_document(
        DocumentSnapping {
            enabled: true,
            ..DocumentSnapping::DEFAULT
        },
        1.0 / scale,
    )
}

fn snapped(pages: &[Page], p: Point, zoom: f64, s: &SnapSettings) -> Option<Point> {
    let points = [p];
    let input = GridSnapInput {
        pages,
        points: &points,
        scale: scale_of(zoom),
    };
    snap_to_grid(&input, s, &mut None).map(|c| {
        assert!((c.point.x - (p.x + c.offset.x)).abs() < 1e-12);
        assert!((c.point.y - (p.y + c.offset.y)).abs() < 1e-12);
        c.point
    })
}

fn a4(grid: PageGrid) -> Page {
    page(Point::new(0.0, 0.0), 210.0, 297.0, grid)
}

#[test]
fn ac12_ac15_intersection_snap_is_exact_on_the_grid() {
    let pages = [a4(shown())];
    let q = snapped(
        &pages,
        Point::new(20.7, 19.4),
        1.0,
        &settings(scale_of(1.0)),
    )
    .unwrap();
    assert!(
        (q.x - 20.0).abs() < EXACT && (q.y - 20.0).abs() < EXACT,
        "{q:?}"
    );
}

#[test]
fn ac12_a_line_snaps_only_the_coordinate_across_it() {
    // Capture at 100 % = 8 px = 2.117 mm. x is 0.5 from a line, y is 2.5 from every line.
    let pages = [a4(shown())];
    let q = snapped(
        &pages,
        Point::new(20.5, 22.5),
        1.0,
        &settings(scale_of(1.0)),
    )
    .unwrap();
    assert!((q.x - 20.0).abs() < EXACT, "{q:?}");
    assert!((q.y - 22.5).abs() < EXACT, "{q:?}");
}

#[test]
fn ac12_only_full_opacity_lines_pull_what_you_see_is_what_you_snap() {
    let pages = [a4(shown())];
    let p = Point::new(21.3, 21.2);
    // 200 %: the 1 mm lines are still fading in (7.56 px), 5 mm lines are 1.3 mm away > capture.
    assert_eq!(snapped(&pages, p, 2.0, &settings(scale_of(2.0))), None);
    // 500 %: the 1 mm lines are at full opacity.
    let q = snapped(&pages, p, 5.0, &settings(scale_of(5.0))).unwrap();
    assert!(
        (q.x - 21.0).abs() < EXACT && (q.y - 21.0).abs() < EXACT,
        "{q:?}"
    );
}

#[test]
fn ac10_ac12_hidden_grid_pasteboard_and_switch_off_give_no_target() {
    let s = settings(scale_of(1.0));
    // Hidden grid.
    assert_eq!(
        snapped(&[a4(PageGrid::DEFAULT)], Point::new(20.3, 20.3), 1.0, &s),
        None
    );
    // Pasteboard, also just outside the page near a would-be line.
    for p in [
        Point::new(-20.3, -20.3),
        Point::new(-0.4, 20.0),
        Point::new(210.4, 50.3),
    ] {
        assert_eq!(snapped(&[a4(shown())], p, 1.0, &s), None, "{p:?}");
    }
    // Target switch off, and snapping off.
    let off = SnapSettings { grid: false, ..s };
    assert_eq!(
        snapped(&[a4(shown())], Point::new(20.3, 20.3), 1.0, &off),
        None
    );
    let disabled = SnapSettings {
        enabled: false,
        ..s
    };
    assert_eq!(
        snapped(&[a4(shown())], Point::new(20.3, 20.3), 1.0, &disabled),
        None
    );
}

#[test]
fn ac12_overlapping_pages_the_page_on_top_decides() {
    let lower = a4(shown());
    let upper = page(
        Point::new(100.0, 0.0),
        100.0,
        100.0,
        PageGrid {
            spacing: Length::from_mm(7.0),
            ..shown()
        },
    );
    let s = settings(scale_of(1.0));
    // Lower grid has (115, 15) at 0.28 mm, upper grid (114, 14) at 1.13 mm.
    let p = Point::new(114.8, 14.8);
    let q = snapped(&[lower.clone(), upper.clone()], p, 1.0, &s).unwrap();
    assert!(
        (q.x - 114.0).abs() < EXACT && (q.y - 14.0).abs() < EXACT,
        "{q:?}"
    );
    let q = snapped(&[upper.clone(), lower.clone()], p, 1.0, &s).unwrap();
    assert!(
        (q.x - 115.0).abs() < EXACT && (q.y - 15.0).abs() < EXACT,
        "{q:?}"
    );
    // A hidden grid on the top page means no grid target there.
    let hidden_upper = Page {
        grid: PageGrid::DEFAULT,
        ..upper
    };
    assert_eq!(snapped(&[lower, hidden_upper], p, 1.0, &s), None);
}

#[test]
fn ac13_rotated_grid_snaps_in_grid_coordinates() {
    let a = 30.0_f64.to_radians();
    let grid = PageGrid {
        angle: Angle::from_radians(a),
        origin: OriginCorner::BottomRight,
        ..shown()
    };
    // The bottom-right corner, the grid origin, is at (100, 100); the u axis points into the page.
    let pages = [page(Point::new(-110.0, -197.0), 210.0, 297.0, grid)];
    let target = Point::new(100.0 - 5.0 * a.cos(), 100.0 - 5.0 * a.sin());
    let p = Point::new(target.x + 0.4, target.y - 0.3);
    let q = snapped(&pages, p, 1.0, &settings(scale_of(1.0))).unwrap();
    assert!(
        (q.x - target.x).abs() < EXACT && (q.y - target.y).abs() < EXACT,
        "{q:?} expected {target:?}"
    );
}

#[test]
fn ac13_x_and_y_switches_refer_to_the_grid_axes() {
    let a = 30.0_f64.to_radians();
    let grid = PageGrid {
        angle: Angle::from_radians(a),
        ..shown()
    };
    let pages = [a4(grid)];
    let u = (a.cos(), a.sin());
    let vax = (-a.sin(), a.cos());
    let base = settings(scale_of(1.0));
    // Grid coordinates (u, v) = (20.4, 30.3), well inside the page.
    let p = Point::new(20.4 * u.0 + 30.3 * vax.0, 20.4 * u.1 + 30.3 * vax.1);
    let points = [p];
    let input = GridSnapInput {
        pages: &pages,
        points: &points,
        scale: scale_of(1.0),
    };
    let x_only = SnapSettings { y: false, ..base };
    let c = snap_to_grid(&input, &x_only, &mut None).unwrap();
    let du = c.offset.x * u.0 + c.offset.y * u.1;
    let dv = c.offset.x * vax.0 + c.offset.y * vax.1;
    assert!((du + 0.4).abs() < 1e-9, "u corrected by {du}");
    assert!(dv.abs() < 1e-9, "v must stay: {dv}");
    let y_only = SnapSettings { x: false, ..base };
    let c = snap_to_grid(&input, &y_only, &mut None).unwrap();
    let du = c.offset.x * u.0 + c.offset.y * u.1;
    let dv = c.offset.x * vax.0 + c.offset.y * vax.1;
    assert!(du.abs() < 1e-9, "u must stay: {du}");
    assert!((dv + 0.3).abs() < 1e-9, "v corrected by {dv}");
}

#[test]
fn ac14_log_grid_targets_are_the_drawn_log_lines() {
    let grid = PageGrid {
        scale: GridScale::Logarithmic,
        mode: GridMode::Fixed,
        ..shown()
    };
    let pages = [a4(grid)];
    let x3 = 3.0_f64.log10() * 25.0;
    let y7 = 25.0 + 7.0_f64.log10() * 25.0;
    let zoom = 10.0;
    let q = snapped(
        &pages,
        Point::new(x3 + 0.05, y7 - 0.04),
        zoom,
        &settings(scale_of(zoom)),
    )
    .unwrap();
    assert!(
        (q.x - x3).abs() < EXACT && (q.y - y7).abs() < EXACT,
        "{q:?}"
    );
    // The target is one of the drawn lines.
    let (v, vis) = view(zoom, Point::new(0.0, 0.0), 1920.0, 1080.0);
    let xs = vertical_xs(&grid_layout(&pages[0], v, vis).lines, FULL);
    assert!(has(&xs, q.x));
}

#[test]
fn ac12_held_grid_feature_is_retained_inside_the_release_zone() {
    let pages = [a4(shown())];
    let s = settings(scale_of(1.0));
    let mut held = None;
    let first = [Point::new(20.5, 20.5)];
    let input = GridSnapInput {
        pages: &pages,
        points: &first,
        scale: scale_of(1.0),
    };
    let c = snap_to_grid(&input, &s, &mut held).unwrap();
    assert!(!c.retained);
    assert!(matches!(
        c.held.feature,
        GridFeature::Intersection { u, v } if (u - 20.0).abs() < EXACT && (v - 20.0).abs() < EXACT
    ));
    // 2.5 mm away on x (beyond capture 2.12 mm, inside release 3.17 mm at 12 px default).
    let release_px = DocumentSnapping::DEFAULT.release_px;
    let inside = f64::midpoint(s.capture, s.release);
    assert!(release_px > DocumentSnapping::DEFAULT.capture_px);
    let second = [Point::new(20.0 + inside, 20.2)];
    let input = GridSnapInput {
        pages: &pages,
        points: &second,
        scale: scale_of(1.0),
    };
    let c = snap_to_grid(&input, &s, &mut held).expect("retained inside release");
    assert!(c.retained);
    assert!((c.point.x - 20.0).abs() < EXACT);
    // Beyond release: let go.
    let third = [Point::new(20.0 + s.release + 0.3, 22.5)];
    let input = GridSnapInput {
        pages: &pages,
        points: &third,
        scale: scale_of(1.0),
    };
    let c = snap_to_grid(&input, &s, &mut held);
    assert!(
        c.as_ref().is_none_or(|c| (c.point.x - 20.0).abs() > 1.0),
        "{c:?}"
    );
}

#[test]
fn ac12_on_an_exact_tie_every_other_target_kind_wins_over_the_grid() {
    let pages = [a4(shown())];
    let s = settings(scale_of(1.0));
    for kind in [
        TargetKind::Guide,
        TargetKind::Box,
        TargetKind::Page,
        TargetKind::Node,
    ] {
        let target =
            SnapTarget::vertical(20.0, (Point::new(20.0, 0.0), Point::new(20.0, 297.0)), kind);
        let targets = SnapTargets::new(vec![target]);
        let p = Point::new(20.6, 22.5);
        let points = [p];
        let input = GridSnapInput {
            pages: &pages,
            points: &points,
            scale: scale_of(1.0),
        };
        let r = resolve_with_grid(
            &s,
            &targets,
            &SnapProbe::point(p),
            &input,
            None,
            &mut SnapState::default(),
        );
        assert!((p.x + r.offset.x - 20.0).abs() < EXACT);
        assert!(r.held_x.is_some(), "{kind:?} must win the tie");
        assert!(!r.grid, "{kind:?}: grid must lose an exact tie");
    }
}

#[test]
fn ac12_a_nearer_grid_line_beats_a_farther_indexed_target() {
    let pages = [a4(shown())];
    let s = settings(scale_of(1.0));
    let target = SnapTarget::vertical(
        21.5,
        (Point::new(21.5, 0.0), Point::new(21.5, 297.0)),
        TargetKind::Guide,
    );
    let targets = SnapTargets::new(vec![target]);
    let p = Point::new(20.3, 22.5);
    let points = [p];
    let input = GridSnapInput {
        pages: &pages,
        points: &points,
        scale: scale_of(1.0),
    };
    let r = resolve_with_grid(
        &s,
        &targets,
        &SnapProbe::point(p),
        &input,
        None,
        &mut SnapState::default(),
    );
    assert!((p.x + r.offset.x - 20.0).abs() < EXACT, "{r:?}");
    assert!(r.grid);
}

#[test]
fn ac7_angle_360_draws_like_angle_0() {
    let p0 = big_page(shown());
    // The document stores every angle normalised (criterion 3), so layout sees 360° as 0°.
    let p360 = big_page(
        PageGrid {
            angle: Angle::from_radians(2.0 * PI),
            ..shown()
        }
        .normalized()
        .unwrap(),
    );
    let (v, vis) = view(1.0, Point::new(0.0, 0.0), 800.0, 600.0);
    let a = grid_layout(&p0, v, vis).lines;
    let b = grid_layout(&p360, v, vis).lines;
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert!((x.start.x - y.start.x).abs() < 1e-6 && (x.start.y - y.start.y).abs() < 1e-6);
    }
}

#[test]
fn ac3_subdivision_2_and_huge_subdivision_still_lay_out_and_nest() {
    for sub in [2_u32, 3, 1000, u32::MAX] {
        let grid = PageGrid {
            subdivision: sub,
            ..shown()
        };
        let p = big_page(grid);
        for zoom in [0.02, 1.0, 80.0] {
            let (v, vis) = view(zoom, Point::new(0.0, 0.0), 1920.0, 1080.0);
            let lines = grid_layout(&p, v, vis).lines;
            for l in &lines {
                assert!(l.start.x.is_finite() && l.end.y.is_finite());
                assert!(l.alpha.is_finite() && l.emphasis.is_finite());
            }
            assert!(lines.len() < 5000, "sub {sub} zoom {zoom}: {}", lines.len());
        }
    }
}
