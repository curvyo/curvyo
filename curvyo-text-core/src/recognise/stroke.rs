//! Matches a line traced from single-line (stroke) text against a single-line face by its centre
//! line (`specs/0091-trace-text-recognition` criterion 27).
//!
//! The traced outlines are filled on a grid along the line and thinned to their centre line
//! (Zhang-Suen); the read string is set in the face over the centre line's box (as
//! [`Recogniser::fit_text`]), and each glyph's stroke is measured against the centre line both
//! ways after a few small shifts. A line whose strokes are thick for its height is no stroke text.

use kurbo::{Affine, Point, Vec2};

use super::sample::f;
use super::stroke_grid::{chamfer, path_points, quantile, spans, thin};
use super::{RecognisedRun, Recogniser};
use crate::face::{Face, FaceKind};
use crate::layout::{LayoutOptions, layout};
use crate::units::Mm;

/// Grid rows per ink height (Proposal): enough for a stroke of 3 % of the height to fill a cell.
const ROWS_PER_HEIGHT: f64 = 64.0;
/// The most cells along the line; a longer line gets larger cells.
const MAX_COLUMNS: usize = 4096;
/// A line whose mean stroke width is more than this share of its ink height is outline text
/// (Proposal: single-line previews are about 4 % to 8 %, Source Sans 3 Regular about 13 %).
const THIN_SHARE: f64 = 0.1;
/// The share of a letter's distances judged: thinning leaves short spurs at stroke ends and joins.
const JUDGED_SHARE: f64 = 0.9;
/// Per glyph, shifts of up to this share of the size are tried along and across the line.
const SHIFT_SHARE: f64 = 0.06;
/// Shifts per direction and side.
const SHIFT_STEPS: i32 = 4;
/// The nearest shifts one way that are measured both ways.
const BOTH_WAYS: usize = 3;

/// A single-line face set over a line's centre line.
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeFit {
    /// The text: sure letters within the tolerance, the others guessed.
    pub run: RecognisedRun,
    /// The mean distance between the centre line and the glyphs' strokes, both ways, as a share
    /// of the ink height: the lower, the nearer the face.
    pub score: f64,
    /// The traced strokes' mean width: the stroke the text is drawn with.
    pub width: Mm,
}

/// The traced line's centre line on a grid along the line.
struct CentreLine {
    /// Line frame: x along `u`, y along the normal; the grid's cell (0, 0) starts at `origin`.
    u: Vec2,
    origin: Point,
    cell: f64,
    columns: usize,
    rows: usize,
    /// The centre line's cells.
    skeleton: Vec<bool>,
    /// Per cell: the distance to the nearest centre line cell, mm.
    distance: Vec<f64>,
    /// The centre line's box in the frame: (x0, y0, x1, y1).
    bounds: (f64, f64, f64, f64),
    /// The traced strokes' mean width, mm.
    width: f64,
}

impl CentreLine {
    fn n(&self) -> Vec2 {
        Vec2::new(-self.u.y, self.u.x)
    }

    fn cell_of(&self, p: Point) -> Option<usize> {
        let (x, y) = (p.x / self.cell, p.y / self.cell);
        if x < 0.0 || y < 0.0 {
            return None;
        }
        // invariant: non-negative and checked against the grid.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (cx, cy) = (x as usize, y as usize);
        (cx < self.columns && cy < self.rows).then_some(cy * self.columns + cx)
    }

    /// The distance of frame point `p` to the centre line; far outside the grid it is large.
    fn distance_at(&self, p: Point) -> f64 {
        self.cell_of(p).map_or(f64::INFINITY, |i| self.distance[i])
    }

    /// The centres of the centre line's cells in the frame within `x0..x1`.
    fn cells_between(&self, x0: f64, x1: f64) -> Vec<Point> {
        let mut out = Vec::new();
        for (i, _) in self.skeleton.iter().enumerate().filter(|(_, s)| **s) {
            let p = self.centre(i);
            if p.x >= x0 && p.x <= x1 {
                out.push(p);
            }
        }
        out
    }

    /// How far a glyph's stroke `points` (frame) lie from the centre line both ways at the best
    /// of `shifts`: the larger of the two [`JUDGED_SHARE`] quantiles.
    fn glyph_distance(&self, points: &[Point], shifts: &[Vec2], step: f64) -> f64 {
        let (x0, x1) = points
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
        let reach = shifts.iter().map(|s| s.x.abs()).fold(0.0, f64::max) + step;
        let near = self.cells_between(x0 - reach, x1 + reach);
        // The shifts that bring the stroke nearest the centre line, then both ways for those.
        let mut ranked: Vec<(f64, Vec2)> = shifts
            .iter()
            .map(|s| {
                let mut there: Vec<f64> =
                    points.iter().map(|&p| self.distance_at(p + *s)).collect();
                (quantile(&mut there, JUDGED_SHARE), *s)
            })
            .collect();
        ranked.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut best = f64::INFINITY;
        for (there, s) in ranked.iter().take(BOTH_WAYS) {
            let mut back: Vec<f64> = near
                .iter()
                .filter(|c| c.x >= x0 + s.x - step && c.x <= x1 + s.x + step)
                .map(|c| {
                    points
                        .iter()
                        .map(|&p| (p + *s).distance(*c))
                        .fold(f64::INFINITY, f64::min)
                })
                .collect();
            best = best.min(there.max(quantile(&mut back, JUDGED_SHARE)));
        }
        best
    }

    /// The mean distance of the centre line's cells to the nearest of `points` (frame).
    fn mean_back(&self, points: &[Point]) -> f64 {
        let mut set = vec![false; self.skeleton.len()];
        for p in points {
            if let Some(i) = self.cell_of(*p) {
                set[i] = true;
            }
        }
        let distance = chamfer(&set, self.columns, self.rows);
        let cells: Vec<f64> = self
            .skeleton
            .iter()
            .zip(&distance)
            .filter(|(s, _)| **s)
            .map(|(_, d)| d * self.cell)
            .collect();
        cells.iter().sum::<f64>() / f(cells.len().max(1))
    }

    fn centre(&self, i: usize) -> Point {
        // invariant: grid indices far below 2^52.
        #[allow(clippy::cast_precision_loss)]
        Point::new(
            ((i % self.columns) as f64 + 0.5) * self.cell,
            ((i / self.columns) as f64 + 0.5) * self.cell,
        )
    }
}

impl Recogniser {
    /// The traced shapes on a `columns` x `rows` grid, `frame` mapping outline space to grid
    /// units: each shape filled row by row at the cell centres, its outer outline less its holes,
    /// each by the non-zero rule (a traced outline never crosses itself, a drawn one may); shapes
    /// that overlap add up.
    fn filled(&self, frame: &dyn Fn(Point) -> Point, columns: usize, rows: usize) -> Vec<bool> {
        let mut grid = vec![false; columns * rows];
        let mut row = vec![0_u8; columns];
        for blob in &self.blobs {
            let rings: Vec<Vec<Point>> = blob
                .loops
                .iter()
                .map(|&l| self.loops[l].points.iter().map(|&p| frame(p)).collect())
                .collect();
            for y in 0..rows {
                let cy = f(y) + 0.5;
                row.fill(0);
                for (k, ring) in rings.iter().enumerate() {
                    let mark = if k == 0 { 1 } else { 2 };
                    for (x0, x1) in spans(ring, cy, columns) {
                        for cell in &mut row[x0..x1] {
                            if mark == 2 || *cell == 0 {
                                *cell = mark;
                            }
                        }
                    }
                }
                for (x, cell) in row.iter().enumerate() {
                    if *cell == 1 {
                        grid[y * columns + x] = true;
                    }
                }
            }
        }
        grid
    }

    /// The traced line's centre line along `direction`; `None` without outlines or when its
    /// strokes are too thick for single-line text.
    fn centre_line(&self, direction: Vec2) -> Option<CentreLine> {
        let length = direction.hypot();
        if self.loops.is_empty() || length <= 0.0 {
            return None;
        }
        let u = direction / length;
        let n = Vec2::new(-u.y, u.x);
        let (mut a0, mut a1, mut b0, mut b1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for p in self.loops.iter().flat_map(|l| l.points.iter()) {
            let v = p.to_vec2();
            a0 = a0.min(v.dot(u));
            a1 = a1.max(v.dot(u));
            b0 = b0.min(v.dot(n));
            b1 = b1.max(v.dot(n));
        }
        let height = b1 - b0;
        if height <= 0.0 || a1 <= a0 {
            return None;
        }
        let cell = (height / ROWS_PER_HEIGHT).max((a1 - a0) / f(MAX_COLUMNS - 4));
        // invariant: positive extents over a positive cell, capped by the column limit.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (columns, rows) = (
            ((a1 - a0) / cell).ceil() as usize + 4,
            (height / cell).ceil() as usize + 4,
        );
        let origin = Point::new(0.0, 0.0) + u * (a0 - 2.0 * cell) + n * (b0 - 2.0 * cell);
        let frame = |p: Point| {
            let v = p - origin;
            Point::new(v.dot(u) / cell, v.dot(n) / cell)
        };
        let mut grid = self.filled(&frame, columns, rows);
        let filled = grid.iter().filter(|b| **b).count();
        thin(&mut grid, columns, rows);
        let skeleton_cells = grid.iter().filter(|b| **b).count();
        if skeleton_cells == 0 {
            return None;
        }
        let width = f(filled) / f(skeleton_cells) * cell;
        if width > THIN_SHARE * height {
            return None;
        }
        let distance: Vec<f64> = chamfer(&grid, columns, rows)
            .into_iter()
            .map(|d| d * cell)
            .collect();
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        let mut line = CentreLine {
            u,
            origin,
            cell,
            columns,
            rows,
            skeleton: grid,
            distance,
            bounds: (0.0, 0.0, 0.0, 0.0),
            width,
        };
        for (i, _) in line.skeleton.iter().enumerate().filter(|(_, s)| **s) {
            let c = line.centre(i);
            x0 = x0.min(c.x);
            y0 = y0.min(c.y);
            x1 = x1.max(c.x);
            y1 = y1.max(c.y);
        }
        line.bounds = (x0, y0, x1, y1);
        Some(line)
    }

    /// `text` set in single-line `face` over the line's centre line along `direction` (outline
    /// space, y down), each letter sure when its stroke lies within the tolerance of the centre
    /// line both ways. `None` for an outline face, an empty text, a line without outlines or one
    /// whose strokes are too thick for single-line text.
    #[must_use]
    #[allow(clippy::many_single_char_names)] // the frame u, n and its coordinates
    pub fn fit_strokes(&self, face: &Face, text: &str, direction: Vec2) -> Option<StrokeFit> {
        if face.kind() != FaceKind::SingleLine || text.trim().is_empty() {
            return None;
        }
        let line = self.centre_line(direction)?;
        let (bx0, by0, bx1, by1) = line.bounds;
        let height = by1 - by0;
        let (size, spacing, at) = super::fit::set_over(face, text, (bx0, by0, bx1, by1))?;
        let options = LayoutOptions {
            letter_spacing: Mm(spacing),
            ..LayoutOptions::default()
        };
        let laid = layout(face, text, Mm(size), &options).ok()?;
        let placed = laid.lines.first()?;
        // Text space to the frame: the ink's corner on the centre line's.
        let to_frame = Affine::translate(at);
        let step = 0.5 * line.cell;
        let shift = SHIFT_SHARE * size;
        let shifts: Vec<Vec2> = (-SHIFT_STEPS..=SHIFT_STEPS)
            .flat_map(|i| {
                (-SHIFT_STEPS..=SHIFT_STEPS).map(move |j| {
                    Vec2::new(f64::from(i), f64::from(j)) * (shift / f64::from(SHIFT_STEPS))
                })
            })
            .collect();
        let char_at: Vec<usize> = text.char_indices().map(|(b, _)| b).collect();
        let mut guessed: Vec<usize> = Vec::new();
        let (mut sure, mut letters) = (0, 0);
        let mut all: Vec<Point> = Vec::new();
        for glyph in &placed.glyphs {
            let Some(path) = laid.glyph_in_line(face, glyph, Mm(0.01 * size)) else {
                continue;
            };
            let points: Vec<Point> = path_points(&path, step)
                .into_iter()
                .map(|p| to_frame * p)
                .collect();
            if points.is_empty() {
                continue;
            }
            letters += 1;
            if line.glyph_distance(&points, &shifts, step) <= self.tolerance {
                sure += 1;
            } else if let Some(i) = char_at.iter().position(|&b| b == glyph.cluster) {
                guessed.push(i);
            }
            all.extend(points);
        }
        if letters == 0 {
            return None;
        }
        let there: Vec<f64> = all
            .iter()
            .map(|&p| line.distance_at(p))
            .filter(|d| d.is_finite())
            .collect();
        if there.is_empty() {
            return None;
        }
        let back = line.mean_back(&all);
        let score = (there.iter().sum::<f64>() / f(there.len()) + back) / height;
        guessed.sort_unstable();
        guessed.dedup();
        let (u, n) = (line.u, line.n());
        let origin = line.origin.to_vec2() + u * at.x + n * at.y;
        let mut outlines: Vec<usize> = self.loops.iter().map(|l| l.outline).collect();
        outlines.sort_unstable();
        outlines.dedup();
        Some(StrokeFit {
            run: RecognisedRun {
                face: 0,
                text: text.to_owned(),
                size: Mm(size),
                placement: Affine::new([u.x, u.y, n.x, n.y, origin.x, origin.y]),
                letter_spacing: Mm(spacing),
                word_spacing: Mm(0.0),
                max_deviation: Mm(0.0),
                guessed,
                outlines,
                sure,
                letters,
            },
            score,
            width: Mm(line.width),
        })
    }
}
