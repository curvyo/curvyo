//! The pattern generators and their clipping (`specs/0071-pattern-fill` criteria 5 to 18, 23):
//! every generator on a 100 mm square with a 20 mm square hole, with and without margin, in both
//! modes.

#![allow(
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    missing_docs
)]

use curvyo_document_core::{
    Angle, GeneratorKind, Length, PatternFill, PatternGenerator, PatternMode, PatternParam,
    PatternSeed, Point, Tolerance, Vec2,
};
use curvyo_geometry_core::{
    PatternError, PatternOutline, PatternPiece, pattern_pieces, pattern_pieces_of,
};

const TOLERANCE: Tolerance = Tolerance::from_mm(0.01);
const SLACK: f64 = 0.011;

fn square(x: f64, y: f64, side: f64, reversed: bool) -> PatternOutline {
    let mut corners = vec![(x, y), (x + side, y), (x + side, y + side), (x, y + side)];
    if reversed {
        corners.reverse();
    }
    let anchors = corners
        .into_iter()
        .map(|(px, py)| (Point::new(px, py), Vec2::ZERO, Vec2::ZERO))
        .collect();
    (anchors, true)
}

/// The 100 mm square from -50 to 50 with the hole from -10 to 10.
fn holed() -> Vec<PatternOutline> {
    vec![
        square(-50.0, -50.0, 100.0, false),
        square(-10.0, -10.0, 20.0, true),
    ]
}

fn fill(kind: GeneratorKind) -> PatternFill {
    PatternFill {
        generator: PatternGenerator::defaults(kind),
        margin: Length::from_mm(0.0),
        mode: PatternMode::Lines,
        seed: PatternSeed::new(7),
        whole_pieces: false,
    }
}

fn run(outlines: &[PatternOutline], fill: &PatternFill) -> Vec<PatternPiece> {
    pattern_pieces(outlines, fill, Length::from_mm(0.25), TOLERANCE).unwrap()
}

/// Points along every outline of `pieces`: anchors and ten points on each segment.
fn samples(pieces: &[PatternPiece]) -> Vec<Point> {
    let mut out = Vec::new();
    for piece in pieces {
        for (anchors, closed) in &piece.outlines {
            let n = anchors.len();
            let segments = if *closed { n } else { n.saturating_sub(1) };
            for i in 0..segments {
                let (p0, _, h0) = anchors[i];
                let (p3, h3, _) = anchors[(i + 1) % n];
                let p1 = Point::new(p0.x + h0.x, p0.y + h0.y);
                let p2 = Point::new(p3.x + h3.x, p3.y + h3.y);
                for k in 0..=10 {
                    let t = f64::from(k) / 10.0;
                    let u = 1.0 - t;
                    let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
                    out.push(Point::new(
                        w[0] * p0.x + w[1] * p1.x + w[2] * p2.x + w[3] * p3.x,
                        w[0] * p0.y + w[1] * p1.y + w[2] * p2.y + w[3] * p3.y,
                    ));
                }
            }
            if n == 1 {
                out.push(anchors[0].0);
            }
        }
    }
    out
}

/// How far `p` lies inside the axis-aligned square from `lo` to `hi` (negative outside).
fn depth_in(p: Point, lo: f64, hi: f64) -> f64 {
    (p.x - lo).min(hi - p.x).min(p.y - lo).min(hi - p.y)
}

/// The distance from `p` outside the square to it (0 inside).
fn distance_to_square(p: Point, lo: f64, hi: f64) -> f64 {
    let dx = (lo - p.x).max(p.x - hi).max(0.0);
    let dy = (lo - p.y).max(p.y - hi).max(0.0);
    if depth_in(p, lo, hi) > 0.0 {
        -depth_in(p, lo, hi)
    } else {
        (dx * dx + dy * dy).sqrt()
    }
}

fn check_region(pieces: &[PatternPiece], margin: f64, what: &str) {
    for p in samples(pieces) {
        assert!(
            depth_in(p, -50.0, 50.0) >= margin - SLACK,
            "{what}: {p:?} outside the region or the margin"
        );
        assert!(
            distance_to_square(p, -10.0, 10.0) >= margin - SLACK,
            "{what}: {p:?} in the hole or its margin"
        );
    }
}

/// Criteria 13, 15: no generator draws inside the hole or outside the outline.
#[test]
fn ac13_ac15_every_generator_stays_in_the_region_and_out_of_the_hole() {
    for kind in GeneratorKind::ALL {
        for mode in [PatternMode::Lines, PatternMode::Shapes] {
            let mut f = fill(kind);
            f.mode = mode;
            let pieces = run(&holed(), &f);
            assert!(!pieces.is_empty(), "{kind:?} {mode:?} draws something");
            check_region(&pieces, 0.0, &format!("{kind:?} {mode:?}"));
        }
    }
}

/// Criterion 14: with a margin nothing comes closer than it to the outline or the hole.
#[test]
fn ac14_the_margin_is_kept_from_the_outline_and_the_hole() {
    for kind in GeneratorKind::ALL {
        let mut f = fill(kind);
        f.margin = Length::from_mm(3.0);
        let pieces = run(&holed(), &f);
        assert!(!pieces.is_empty(), "{kind:?}");
        check_region(&pieces, 3.0, &format!("{kind:?} margin"));
    }
}

/// Criterion 16: Lines gives open lines for line generators and closed cells for the others;
/// Shapes gives closed outlines only.
#[test]
fn ac16_lines_and_shapes() {
    let open_kinds = [
        GeneratorKind::Hatch,
        GeneratorKind::Crosshatch,
        GeneratorKind::Grid,
        GeneratorKind::SeaWaves,
        GeneratorKind::Waves,
        GeneratorKind::InterpolationWaves,
    ];
    for kind in GeneratorKind::ALL {
        let lines = run(&holed(), &fill(kind));
        let all_closed = lines
            .iter()
            .all(|p| p.outlines.iter().all(|(_, closed)| *closed));
        assert_eq!(all_closed, !open_kinds.contains(&kind), "{kind:?} lines");
        let mut f = fill(kind);
        f.mode = PatternMode::Shapes;
        let shapes = run(&holed(), &f);
        assert!(
            shapes
                .iter()
                .all(|p| p.outlines.iter().all(|(_, closed)| *closed)),
            "{kind:?} shapes"
        );
    }
}

fn ys_of_horizontal_lines(pieces: &[PatternPiece]) -> Vec<f64> {
    let mut ys: Vec<f64> = pieces.iter().map(|p| p.outlines[0].0[0].0.y).collect();
    ys.sort_by(f64::total_cmp);
    ys.dedup();
    ys
}

/// Criterion 5: Hatch lines are parallel and one spacing apart; Grid adds the perpendicular set.
#[test]
fn ac5_hatch_lines_are_parallel_and_spaced() {
    let mut f = fill(GeneratorKind::Hatch);
    f.generator = f
        .generator
        .with(PatternParam::Angle, 0.0)
        .unwrap()
        .with(PatternParam::Spacing, 4.0)
        .unwrap();
    let outer = vec![square(-50.0, -50.0, 100.0, false)];
    let pieces = run(&outer, &f);
    let ys = ys_of_horizontal_lines(&pieces);
    assert_eq!(ys.len(), 25, "lines at -48 .. 48 -> {ys:?}");
    for w in ys.windows(2) {
        assert!((w[1] - w[0] - 4.0).abs() < 1e-9, "{ys:?}");
    }
    for piece in &pieces {
        let (a, b) = (
            piece.outlines[0].0[0].0,
            piece.outlines[0].0.last().unwrap().0,
        );
        assert!((a.y - b.y).abs() < 1e-9, "horizontal");
    }
    let mut grid = fill(GeneratorKind::Grid);
    grid.generator = grid.generator.with(PatternParam::Spacing, 4.0).unwrap();
    assert_eq!(run(&outer, &grid).len(), 2 * pieces.len());
    let cross = run(&outer, &fill(GeneratorKind::Crosshatch));
    assert!(cross.len() > run(&outer, &fill(GeneratorKind::Hatch)).len());
}

fn centre(piece: &PatternPiece) -> Point {
    let anchors = &piece.outlines[0].0;
    let n = anchors.len() as f64;
    Point::new(
        anchors.iter().map(|a| a.0.x).sum::<f64>() / n,
        anchors.iter().map(|a| a.0.y).sum::<f64>() / n,
    )
}

/// Criterion 8: running bond, row n+1 shifted by half a brick (10 mm for 20 x 10 x 1).
#[test]
fn ac8_brick_rows_shift_by_half_a_brick() {
    let outer = vec![square(-50.0, -50.0, 100.0, false)];
    let pieces = run(&outer, &fill(GeneratorKind::BrickWall));
    let whole: Vec<Point> = pieces
        .iter()
        .filter(|p| p.outlines[0].0.len() == 4)
        .map(centre)
        .collect();
    let row = |y: f64| -> Vec<f64> {
        let mut xs: Vec<f64> = whole
            .iter()
            .filter(|c| (c.y - y).abs() < 1e-6 && c.x.abs() < 35.0)
            .map(|c| c.x)
            .collect();
        xs.sort_by(f64::total_cmp);
        xs
    };
    let (a, b) = (row(5.0), row(15.0));
    assert!(!a.is_empty() && !b.is_empty());
    let shift = (b[0] - a[0]).rem_euclid(20.0);
    assert!((shift - 10.0).abs() < 1e-9, "{a:?} {b:?}");
    // A whole brick is 19 x 9 mm.
    let brick = &whole_brick(&pieces);
    let xs: Vec<f64> = brick.iter().map(|p| p.x).collect();
    let ys: Vec<f64> = brick.iter().map(|p| p.y).collect();
    let span = |v: &[f64]| {
        v.iter().copied().fold(f64::MIN, f64::max) - v.iter().copied().fold(f64::MAX, f64::min)
    };
    assert!((span(&xs) - 19.0).abs() < 1e-9 && (span(&ys) - 9.0).abs() < 1e-9);
}

fn whole_brick(pieces: &[PatternPiece]) -> Vec<Point> {
    pieces
        .iter()
        .find(|p| {
            (centre(p).y - 5.0).abs() < 1e-6
                && centre(p).x.abs() < 35.0
                && p.outlines[0].0.len() == 4
        })
        .unwrap()
        .outlines[0]
        .0
        .iter()
        .map(|a| a.0)
        .collect()
}

/// Criterion 6: dots of the diameter on a lattice with every second row shifted half a spacing.
#[test]
fn ac6_dots_are_staggered() {
    let outer = vec![square(-50.0, -50.0, 100.0, false)];
    let pieces = run(&outer, &fill(GeneratorKind::Dots));
    let centres: Vec<Point> = pieces
        .iter()
        .filter(|p| p.outlines[0].0.len() == 4)
        .map(centre)
        .collect();
    let row0: Vec<f64> = centres
        .iter()
        .filter(|c| c.y.abs() < 1e-9)
        .map(|c| c.x)
        .collect();
    let row1: Vec<f64> = centres
        .iter()
        .filter(|c| (c.y - 3.0).abs() < 1e-9)
        .map(|c| c.x)
        .collect();
    assert!(row0.iter().any(|x| x.abs() < 1e-9));
    assert!(row1.iter().any(|x| (x - 1.5).abs() < 1e-9));
    let dot = pieces
        .iter()
        .find(|p| centre(p).x.abs() < 1e-9 && centre(p).y.abs() < 1e-9)
        .unwrap();
    assert!((dot.outlines[0].0[0].0.x - 0.5).abs() < 1e-9, "radius 0.5");
}

/// Criterion 7: hexagons `cell_size - wall` across the flats.
#[test]
fn ac7_honeycomb_cells_are_regular_hexagons() {
    let outer = vec![square(-50.0, -50.0, 100.0, false)];
    let pieces = run(&outer, &fill(GeneratorKind::Honeycomb));
    let cell = pieces
        .iter()
        .find(|p| {
            p.outlines[0].0.len() == 6 && centre(p).x.abs() < 1e-9 && centre(p).y.abs() < 1e-9
        })
        .unwrap();
    let pts: Vec<Point> = cell.outlines[0].0.iter().map(|a| a.0).collect();
    let across = pts[1].x - pts[5].x;
    assert!((across - 5.5).abs() < 1e-9, "{across}");
    let side = |a: Point, b: Point| ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt();
    for i in 0..6 {
        assert!((side(pts[i], pts[(i + 1) % 6]) - side(pts[0], pts[1])).abs() < 1e-9);
    }
}

/// Criterion 9: no two leaves overlap; the seed changes the forest.
#[test]
fn ac9_leaves_do_not_overlap_and_the_seed_reshuffles_them() {
    let outer = vec![square(-50.0, -50.0, 100.0, false)];
    let f = fill(GeneratorKind::LeafForest);
    let pieces = run(&outer, &f);
    assert!(pieces.len() > 50, "{}", pieces.len());
    let whole: Vec<&PatternPiece> = pieces
        .iter()
        .filter(|p| p.outlines[0].0.len() == 2)
        .collect();
    for (i, a) in whole.iter().enumerate() {
        for b in &whole[i + 1..] {
            let (ca, cb) = (centre(a), centre(b));
            let half = |p: &PatternPiece| {
                let q = &p.outlines[0].0;
                ((q[0].0.x - q[1].0.x).powi(2) + (q[0].0.y - q[1].0.y).powi(2)).sqrt() / 2.0
            };
            let d = ((ca.x - cb.x).powi(2) + (ca.y - cb.y).powi(2)).sqrt();
            assert!(d >= half(a) + half(b), "leaves overlap at {ca:?} {cb:?}");
        }
    }
    let mut other = f;
    other.seed = PatternSeed::new(8);
    assert_ne!(run(&outer, &other), pieces);
}

/// Criterion 10: half circles in rows, every second row shifted by one radius.
#[test]
fn ac10_sea_waves_are_half_circles_in_shifted_rows() {
    let outer = vec![square(-50.0, -50.0, 100.0, false)];
    let pieces = run(&outer, &fill(GeneratorKind::SeaWaves));
    let arcs: Vec<&PatternPiece> = pieces
        .iter()
        .filter(|p| p.outlines[0].0.len() == 3)
        .collect();
    let starts = |y: f64| -> Vec<f64> {
        arcs.iter()
            .filter(|p| {
                (p.outlines[0].0[0].0.y - y).abs() < 1e-9
                    && (p.outlines[0].0[1].0.y - (y - 5.0)).abs() < 1e-9
            })
            .map(|p| p.outlines[0].0[0].0.x + 5.0)
            .collect()
    };
    let (row0, row1) = (starts(0.0), starts(5.0));
    assert!(row0.iter().any(|x| x.abs() < 1e-9), "{row0:?}");
    assert!(row1.iter().any(|x| (x - 5.0).abs() < 1e-9), "{row1:?}");
}

/// Criterion 11: Waves with amplitude 0 equal Hatch with the same spacing and angle.
#[test]
fn ac11_flat_waves_are_hatch() {
    let mut waves = fill(GeneratorKind::Waves);
    waves.generator = waves
        .generator
        .with(PatternParam::Amplitude, 0.0)
        .unwrap()
        .with(PatternParam::Angle, 30.0)
        .unwrap();
    let mut hatch = fill(GeneratorKind::Hatch);
    hatch.generator = hatch.generator.with(PatternParam::Angle, 30.0).unwrap();
    assert_eq!(run(&holed(), &waves), run(&holed(), &hatch));
}

/// Criterion 12: the first line follows the top boundary, the last the bottom one.
#[test]
fn ac12_interpolation_runs_from_top_to_bottom() {
    let outer = vec![square(-50.0, -50.0, 100.0, false)];
    let pieces = run(&outer, &fill(GeneratorKind::InterpolationWaves));
    assert_eq!(pieces.len(), 12);
    let ys: Vec<f64> = pieces
        .iter()
        .map(|p| p.outlines[0].0.iter().map(|a| a.0.y).sum::<f64>() / p.outlines[0].0.len() as f64)
        .collect();
    assert!(ys.iter().any(|y| (y + 50.0).abs() < 1e-6), "top {ys:?}");
    assert!(ys.iter().any(|y| (y - 50.0).abs() < 1e-6), "bottom {ys:?}");
}

/// Criterion 17: the same input gives the same pieces; the seed only matters to Leaf forest.
#[test]
fn ac17_deterministic_and_the_seed_only_changes_seeded_generators() {
    for kind in GeneratorKind::ALL {
        let f = fill(kind);
        let a = run(&holed(), &f);
        assert_eq!(a, run(&holed(), &f), "{kind:?}");
        let mut reseeded = f;
        reseeded.seed = PatternSeed::new(99);
        assert_eq!(a == run(&holed(), &reseeded), !kind.is_seeded(), "{kind:?}");
    }
}

/// Criterion 18: a pattern over the hard limit is refused before any work.
#[test]
fn ac18_too_many_pieces_are_refused() {
    let mut f = fill(GeneratorKind::Dots);
    f.generator = f.generator.with(PatternParam::Spacing, 0.05).unwrap();
    f.generator = f.generator.with(PatternParam::Diameter, 0.05).unwrap();
    let big = vec![square(0.0, 0.0, 1000.0, false)];
    assert!(matches!(
        pattern_pieces(&big, &f, Length::from_mm(0.25), TOLERANCE),
        Err(PatternError::TooMany { estimate }) if estimate > 500_000
    ));
}

/// Criterion 23: an open outline gives no pieces and no error.
#[test]
fn ac23_an_open_shape_has_no_pattern() {
    let (anchors, _) = square(0.0, 0.0, 10.0, false);
    assert_eq!(
        run(&[(anchors, false)], &fill(GeneratorKind::Hatch)),
        Vec::new()
    );
}

/// ADR 0029 §4: moving the shape moves the pattern with it.
#[test]
fn the_pattern_moves_with_the_shape() {
    let f = fill(GeneratorKind::Dots);
    let here = pattern_pieces_of(
        &holed(),
        Angle::from_radians(0.0),
        &f,
        Length::from_mm(0.25),
        TOLERANCE,
    )
    .unwrap();
    let moved_outlines: Vec<PatternOutline> = holed()
        .into_iter()
        .map(|(a, c)| {
            (
                a.into_iter()
                    .map(|(p, h, k)| (Point::new(p.x + 33.3, p.y - 12.7), h, k))
                    .collect(),
                c,
            )
        })
        .collect();
    let there = pattern_pieces_of(
        &moved_outlines,
        Angle::from_radians(0.0),
        &f,
        Length::from_mm(0.25),
        TOLERANCE,
    )
    .unwrap();
    assert_eq!(here.len(), there.len());
    for (a, b) in here.iter().zip(&there) {
        let (pa, pb) = (a.outlines[0].0[0].0, b.outlines[0].0[0].0);
        assert!((pb.x - pa.x - 33.3).abs() < 1e-6 && (pb.y - pa.y + 12.7).abs() < 1e-6);
    }
}

/// A region with a wavy top (two waves of 10 mm amplitude around y = 20) and a straight bottom at
/// y = 100, 100 mm wide.
fn wavy_top() -> Vec<PatternOutline> {
    let a = |x: f64, y: f64, h: f64| (Point::new(x, y), Vec2::new(h, 0.0), Vec2::new(-h, 0.0));
    let corner = |x: f64, y: f64| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO);
    vec![(
        vec![
            corner(0.0, 100.0),
            corner(100.0, 100.0),
            corner(100.0, 20.0),
            a(75.0, 10.0, 8.0),
            a(50.0, 20.0, 8.0),
            a(25.0, 30.0, 8.0),
            corner(0.0, 20.0),
        ],
        true,
    )]
}

/// Criterion 12 (customer 2026-11-02: like Inkscape's Interpolate): on a wavy top and a straight
/// bottom, the first line follows the wave, the last is the straight bottom, and the line half way
/// carries half the wave, point for point.
#[test]
fn ac12_interpolation_blends_a_wave_into_a_straight_line() {
    let mut f = fill(GeneratorKind::InterpolationWaves);
    f.generator = f.generator.with(PatternParam::Lines, 3.0).unwrap();
    let pieces = run(&wavy_top(), &f);
    let lines: Vec<Vec<Point>> = pieces
        .iter()
        .map(|p| p.outlines[0].0.iter().map(|a| a.0).collect())
        .collect();
    assert_eq!(
        lines.len(),
        3,
        "the hole-free region keeps every line whole"
    );
    let span = |line: &[Point]| {
        let ys = line.iter().map(|p| p.y);
        ys.clone().fold(f64::MIN, f64::max) - ys.fold(f64::MAX, f64::min)
    };
    let mean_y = |line: &[Point]| line.iter().map(|p| p.y).sum::<f64>() / line.len() as f64;
    let mut by_height = lines.clone();
    by_height.sort_by(|a, b| mean_y(a).total_cmp(&mean_y(b)));
    let (top, middle, bottom) = (&by_height[0], &by_height[1], &by_height[2]);
    assert!(
        (span(top) - 20.0).abs() < 0.2,
        "the top follows the wave: {}",
        span(top)
    );
    assert!(span(bottom) < 1e-6, "the bottom is straight");
    assert!((mean_y(bottom) - 100.0).abs() < 1e-6);
    assert!(
        (span(middle) - 10.0).abs() < 0.3,
        "half the wave: {}",
        span(middle)
    );
    // The middle line lies between the two and carries the wave's shape: its highest point is
    // above the middle of the region where the top is highest (x = 75), its lowest where the top
    // is lowest (x = 25).
    let at = |line: &[Point], x: f64| {
        line.iter()
            .min_by(|a, b| (a.x - x).abs().total_cmp(&(b.x - x).abs()))
            .unwrap()
            .y
    };
    assert!((at(middle, 75.0) - 55.0).abs() < 0.3);
    assert!((at(middle, 25.0) - 65.0).abs() < 0.3);
}

/// Acceptance report finding 3: crossing line sets give one straight path per line part, never a
/// path that turns from one line onto another at a crossing.
#[test]
fn ac5_grid_and_crosshatch_give_each_line_its_own_straight_path() {
    for kind in [GeneratorKind::Grid, GeneratorKind::Crosshatch] {
        let pieces = run(&holed(), &fill(kind));
        for piece in &pieces {
            assert_eq!(piece.outlines.len(), 1, "{kind:?}");
            let (anchors, closed) = &piece.outlines[0];
            assert!(!closed, "{kind:?}");
            assert_eq!(
                anchors.len(),
                2,
                "{kind:?}: a straight line part, not a staircase"
            );
        }
        // The lines through the hole are cut in two, the others are whole: twice the lines
        // crossing the hole plus the rest.
        assert!(pieces.len() > 40, "{kind:?}: {}", pieces.len());
    }
}

/// Acceptance report finding 4 (Proposal): in Shapes mode the strips of crossing lines are merged,
/// so no two pieces overlap and a laser cuts every crossing once. Grid on a square is one mesh.
#[test]
fn ac16_shapes_merges_crossing_strips_into_one_outline() {
    let outline = [square(0.0, 0.0, 30.0, false)];
    for kind in [GeneratorKind::Grid, GeneratorKind::Crosshatch] {
        let mut f = fill(kind);
        f.mode = PatternMode::Shapes;
        let pieces = run(&outline, &f);
        assert_eq!(pieces.len(), 1, "{kind:?}: one merged outline");
        assert!(
            pieces[0].outlines.len() > 10,
            "{kind:?}: the cells between the strips are its holes"
        );
    }
    // Parallel strips that do not touch stay one piece each.
    let mut hatch = fill(GeneratorKind::Hatch);
    hatch.mode = PatternMode::Shapes;
    let pieces = run(&outline, &hatch);
    assert!(pieces.len() > 10);
    assert!(pieces.iter().all(|p| p.outlines.len() == 1));
}

/// Acceptance report finding 2: the limit counts work, not only pieces. Interpolation waves with
/// many lines are refused as too detailed, a coarser pattern on the same shape is made.
#[test]
fn ac18_a_pattern_with_too_much_work_is_refused_as_too_detailed() {
    let many = PatternFill {
        generator: PatternGenerator::InterpolationWaves {
            lines: curvyo_document_core::PatternCount::new(100_000).unwrap(),
        },
        ..fill(GeneratorKind::InterpolationWaves)
    };
    assert_eq!(
        pattern_pieces(&holed(), &many, Length::from_mm(0.25), TOLERANCE),
        Err(PatternError::TooDetailed)
    );
    let fine_hatch = PatternFill {
        mode: PatternMode::Shapes,
        ..fill(GeneratorKind::Hatch)
    };
    assert!(pattern_pieces(&holed(), &fine_hatch, Length::from_mm(0.25), TOLERANCE).is_ok());
}

/// UI review 2026-11-02, item 3: Sea waves counts only the rings that have a radius, so more
/// rings than fit in a scale give the same arcs as the rings that fit and no refusal over arcs
/// that are not drawn.
#[test]
fn ac18_sea_waves_count_only_the_rings_that_fit() {
    let with_rings = |rings: f64| {
        let mut f = fill(GeneratorKind::SeaWaves);
        f.generator = f
            .generator
            .with(PatternParam::ScaleRadius, 2.24)
            .and_then(|g| g.with(PatternParam::RingSpacing, 1.0))
            .and_then(|g| g.with(PatternParam::Rings, rings))
            .unwrap();
        pattern_pieces(&holed(), &f, Length::from_mm(0.25), TOLERANCE)
    };
    let fitting = with_rings(3.0).unwrap();
    assert_eq!(with_rings(500.0).unwrap(), fitting);
}

fn whole(kind: GeneratorKind, mode: PatternMode, margin: f64) -> PatternFill {
    PatternFill {
        mode,
        margin: Length::from_mm(margin),
        whole_pieces: true,
        ..fill(kind)
    }
}

/// The centre of a closed piece's anchors.
fn anchor_centre(piece: &PatternPiece) -> Point {
    let anchors = &piece.outlines[0].0;
    let n = anchors.len() as f64;
    let (x, y) = anchors
        .iter()
        .fold((0.0, 0.0), |(x, y), (p, _, _)| (x + p.x, y + p.y));
    Point::new(x / n, y / n)
}

/// Whether the convex polygon `points` lies at least `gap` inside the region of [`holed`] inset
/// by `margin`: every point inside the outer square, and the polygon apart from the hole grown by
/// `margin` (the hole is convex, so a separating axis of one of the two shows it).
fn polygon_clear(points: &[Point], margin: f64, gap: f64) -> bool {
    if points
        .iter()
        .any(|p| depth_in(*p, -50.0, 50.0) < margin + gap)
    {
        return false;
    }
    let edges = |ring: &[Point]| -> Vec<Vec2> {
        (0..ring.len())
            .map(|i| {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                Vec2::new(-(b.y - a.y), b.x - a.x)
            })
            .collect()
    };
    let hole = [
        Point::new(-10.0, -10.0),
        Point::new(10.0, -10.0),
        Point::new(10.0, 10.0),
        Point::new(-10.0, 10.0),
    ];
    // With a margin the grown hole has round corners; the box test with the margin added on each
    // side is enough for the lattice pieces checked here (they sit along the sides).
    let mut axes = edges(points);
    axes.extend(edges(&hole));
    axes.iter().any(|axis| {
        let length = (axis.x * axis.x + axis.y * axis.y).sqrt();
        let project = |p: &Point| (p.x * axis.x + p.y * axis.y) / length;
        let (a_lo, a_hi) = points
            .iter()
            .map(project)
            .fold((f64::MAX, f64::MIN), |(l, h), v| (l.min(v), h.max(v)));
        let (b_lo, b_hi) = hole
            .iter()
            .map(project)
            .fold((f64::MAX, f64::MIN), |(l, h), v| (l.min(v), h.max(v)));
        a_lo > b_hi + margin + gap || b_lo > a_hi + margin + gap
    })
}

/// The honeycomb cells of [`fill`]'s defaults (cell 6 mm, wall 0.5 mm) in the frame of
/// [`holed`], as in `cells::honeycomb`.
fn honeycomb_cells() -> Vec<(Point, Vec<Point>)> {
    let c = 6.0;
    let half_root3 = 3.0_f64.sqrt() / 2.0;
    let radius = (c - 0.5) / 3.0_f64.sqrt();
    let (w, h) = (radius * half_root3, radius / 2.0);
    let mut out = Vec::new();
    for j in -20..=20 {
        let shift = if j % 2 == 0 { 0.0 } else { c / 2.0 };
        for i in -20..=20 {
            let p = Point::new(f64::from(i) * c + shift, f64::from(j) * c * half_root3);
            out.push((
                p,
                vec![
                    Point::new(p.x, p.y - radius),
                    Point::new(p.x + w, p.y - h),
                    Point::new(p.x + w, p.y + h),
                    Point::new(p.x, p.y + radius),
                    Point::new(p.x - w, p.y + h),
                    Point::new(p.x - w, p.y - h),
                ],
            ));
        }
    }
    out
}

fn near(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6
}

/// Criterion 29: with whole pieces only, Honeycomb draws exactly the cells the region leaves
/// whole, each with its exact six corners; no cut cell is left, none of the whole ones is missing.
#[test]
fn ac29_whole_pieces_draw_exactly_the_uncut_honeycomb_cells() {
    for mode in [PatternMode::Lines, PatternMode::Shapes] {
        let pieces = run(&holed(), &whole(GeneratorKind::Honeycomb, mode, 0.0));
        check_region(&pieces, 0.0, "whole honeycomb");
        assert!(
            pieces
                .iter()
                .all(|p| p.outlines.len() == 1 && p.outlines[0].1 && p.outlines[0].0.len() == 6)
        );
        let centres: Vec<Point> = pieces.iter().map(anchor_centre).collect();
        let cells = honeycomb_cells();
        let clear: Vec<Point> = cells
            .iter()
            .filter(|(_, corners)| polygon_clear(corners, 0.0, 0.002))
            .map(|(c, _)| *c)
            .collect();
        let possible: Vec<Point> = cells
            .iter()
            .filter(|(_, corners)| polygon_clear(corners, 0.0, -0.002))
            .map(|(c, _)| *c)
            .collect();
        for c in &clear {
            assert!(
                centres.iter().any(|k| near(*k, *c)),
                "{mode:?}: whole cell {c:?} kept"
            );
        }
        for k in &centres {
            assert!(
                possible.iter().any(|c| near(*k, *c)),
                "{mode:?}: {k:?} is a whole cell"
            );
        }
        let all = run(&holed(), &fill(GeneratorKind::Honeycomb));
        assert!(pieces.len() < all.len(), "{mode:?}: cut cells left out");
    }
}

/// Criterion 29: the margin cuts as much as the outline; whole dots keep their four exact arcs.
#[test]
fn ac29_whole_pieces_keep_the_margin_and_the_exact_dots() {
    let pieces = run(
        &holed(),
        &whole(GeneratorKind::Dots, PatternMode::Shapes, 3.0),
    );
    assert!(!pieces.is_empty(), "whole dots are drawn");
    check_region(&pieces, 3.0, "whole dots, margin 3");
    assert!(
        pieces
            .iter()
            .all(|p| p.outlines.len() == 1 && p.outlines[0].0.len() == 4)
    );
    // Every dot clearly inside the inset region is kept.
    let dots = fill(GeneratorKind::Dots).generator;
    let s = dots.get(PatternParam::Spacing).unwrap();
    let reach = 3.0 + dots.get(PatternParam::Diameter).unwrap() / 2.0 + 0.01;
    let centres: Vec<Point> = pieces.iter().map(anchor_centre).collect();
    let mut checked = 0;
    for j in -30_i32..=30 {
        let shift = if j % 2 == 0 { 0.0 } else { s / 2.0 };
        for i in -30_i32..=30 {
            let c = Point::new(f64::from(i) * s + shift, f64::from(j) * s);
            let clear =
                depth_in(c, -50.0, 50.0) > reach && distance_to_square(c, -10.0, 10.0) > reach;
            checked += usize::from(clear);
            if clear {
                assert!(centres.iter().any(|k| near(*k, c)), "dot {c:?} kept");
            }
        }
    }
    assert!(checked > 100);
}

/// Criterion 29: Sea waves keep or leave out a scale with all its rings, never part of one.
#[test]
fn ac29_whole_sea_waves_keep_every_ring_of_a_kept_scale() {
    for mode in [PatternMode::Lines, PatternMode::Shapes] {
        let pieces = run(&holed(), &whole(GeneratorKind::SeaWaves, mode, 0.0));
        assert!(!pieces.is_empty(), "{mode:?}");
        check_region(&pieces, 0.0, "whole sea waves");
        if mode == PatternMode::Lines {
            let mut per_centre: Vec<(Point, usize)> = Vec::new();
            for piece in &pieces {
                let anchors = &piece.outlines[0].0;
                assert_eq!(anchors.len(), 3, "an uncut arc");
                let c = Point::new(
                    f64::midpoint(anchors[0].0.x, anchors[2].0.x),
                    anchors[0].0.y,
                );
                match per_centre.iter_mut().find(|(k, _)| near(*k, c)) {
                    Some((_, n)) => *n += 1,
                    None => per_centre.push((c, 1)),
                }
            }
            assert!(per_centre.iter().all(|(_, n)| *n == 3), "three rings each");
        }
        let cut = run(
            &holed(),
            &PatternFill {
                mode,
                ..fill(GeneratorKind::SeaWaves)
            },
        );
        assert_ne!(pieces, cut, "{mode:?}: something left out");
    }
}

/// Criterion 29: the switch leaves the line generators as they are, and is deterministic.
#[test]
fn ac29_whole_pieces_apply_to_shapes_only_and_are_deterministic() {
    for kind in GeneratorKind::ALL {
        for mode in [PatternMode::Lines, PatternMode::Shapes] {
            let on = whole(kind, mode, 1.0);
            let off = PatternFill {
                whole_pieces: false,
                ..on
            };
            let pieces = run(&holed(), &on);
            assert_eq!(
                pieces,
                run(&holed(), &on),
                "{kind:?} {mode:?} deterministic"
            );
            if !kind.has_whole_pieces() {
                assert_eq!(pieces, run(&holed(), &off), "{kind:?} {mode:?} unchanged");
            }
        }
    }
}
