//! Pattern fill generation (`specs/0071-pattern-fill`, ADR 0029 §3 to §5, §10): one pure function
//! from a closed region, a pattern and the line width to the pieces, clipped to the region with
//! its holes and margin.
//!
//! Pieces are made in the pattern's frame: its origin is the centre of the outline's box with the
//! object's rotation taken out, so moving or rotating the shape moves and rotates the pattern with
//! it, and the same frame-local outline always gives the same pieces. Generators use `+ - * /`,
//! `sqrt` and `libm` only, and clipping is integer arithmetic, so the pieces are the same bits on
//! every machine and in wasm.

mod blend;
mod cells;
mod clip;
mod cost;
mod cut;
mod leaves;
mod lines;
mod local;
mod region;
mod scales;
mod seed;
mod string_clip;

use curvyo_document_core::{Angle, Length, PatternFill, PatternGenerator, Point, Tolerance, Vec2};

use crate::OutlineTriple;
use crate::boolean::Outline;
use crate::boolean_grid::{GRID_MM, Point64, snap};
pub use cost::PATTERN_MAX_WORK;
use cut::Cutter;
use region::Region;

/// Above this many pieces the panel notes the count (criterion 18, first estimate; see `plan.md`).
pub const PATTERN_NOTE_PIECES: usize = 50_000;
/// Above this many pieces (estimated before any work) the pattern is not generated.
pub const PATTERN_MAX_PIECES: usize = 500_000;

/// One outline: anchors (point, handle in, handle out) and whether it is closed.
pub type PatternOutline = (Vec<OutlineTriple>, bool);

/// One piece of a pattern: one outline, or a closed outer outline with the holes a cut left.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternPiece {
    /// The outlines of the piece.
    pub outlines: Vec<PatternOutline>,
}

/// Why no pieces were made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PatternError {
    /// The lattice over the region's box would give more than [`PATTERN_MAX_PIECES`] pieces.
    #[error("the pattern would have about {estimate} pieces")]
    TooMany {
        /// The estimate.
        estimate: usize,
    },
    /// Making the pattern would take more than [`PATTERN_MAX_WORK`] (long lines with many
    /// points, many crossings, or a detailed outline), whatever its piece count.
    #[error("the pattern is too detailed to make")]
    TooDetailed,
}

/// The box a generator covers, in the frame, mm.
pub(crate) type Bounds = (Point, Point);

/// A generated piece before the cut: one exact outline.
pub(crate) struct Raw {
    anchors: Vec<OutlineTriple>,
    closed: bool,
    /// The piece runs along the region's own boundary (Interpolation waves' first and last
    /// lines): it is inside by construction and never cut in Lines mode.
    on_boundary: bool,
    /// The piece belongs to the one before it (a ring of a Sea waves scale): whole pieces only
    /// keeps or leaves out the two together.
    follows: bool,
}

impl Raw {
    pub(crate) const fn open(anchors: Vec<OutlineTriple>) -> Self {
        Self {
            anchors,
            closed: false,
            on_boundary: false,
            follows: false,
        }
    }

    /// An open piece that belongs to the one before it.
    pub(crate) const fn following(anchors: Vec<OutlineTriple>) -> Self {
        Self {
            anchors,
            closed: false,
            on_boundary: false,
            follows: true,
        }
    }

    /// An open piece along the region's boundary.
    pub(crate) const fn boundary(anchors: Vec<OutlineTriple>) -> Self {
        Self {
            anchors,
            closed: false,
            on_boundary: true,
            follows: false,
        }
    }

    pub(crate) const fn closed(anchors: Vec<OutlineTriple>) -> Self {
        Self {
            anchors,
            closed: true,
            on_boundary: false,
            follows: false,
        }
    }

    /// The box of the anchors and their handles, which holds the curve.
    fn grid_box(&self) -> (Point64, Point64) {
        let mut lo = Point64::new(i64::MAX, i64::MAX);
        let mut hi = Point64::new(i64::MIN, i64::MIN);
        for &(p, h_in, h_out) in &self.anchors {
            for q in [p, offset(p, h_in), offset(p, h_out)] {
                let g = snap(q);
                lo = Point64::new(lo.x.min(g.x), lo.y.min(g.y));
                hi = Point64::new(hi.x.max(g.x), hi.y.max(g.y));
            }
        }
        // The curve may round one unit beyond its control points.
        (
            Point64::new(lo.x - 1, lo.y - 1),
            Point64::new(hi.x + 1, hi.y + 1),
        )
    }
}

fn offset(p: Point, v: Vec2) -> Point {
    Point::new(p.x + v.x, p.y + v.y)
}

/// The pattern's frame on a shape: origin and angle (ADR 0029 §4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PatternFrame {
    origin: Point,
    sin: f64,
    cos: f64,
}

/// Rounds to 1e-6 mm, so a frame-local outline does not change with the float dust of a move.
fn round_nm(value: f64) -> f64 {
    (value * 1e6).round() / 1e6 + 0.0
}

impl PatternFrame {
    /// The frame of a shape with these outlines (document coordinates) and this `rotation`
    /// register: the centre of the box of its anchors and handles seen at its rotation.
    #[must_use]
    pub fn of(outlines: &[PatternOutline], rotation: Angle) -> Self {
        let (sin, cos) = (
            libm::sin(rotation.as_radians()),
            libm::cos(rotation.as_radians()),
        );
        let unrotate = |p: Point| Point::new(p.x * cos + p.y * sin, -p.x * sin + p.y * cos);
        let mut lo = Point::new(f64::INFINITY, f64::INFINITY);
        let mut hi = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (anchors, _) in outlines {
            for &(p, h_in, h_out) in anchors {
                for q in [p, offset(p, h_in), offset(p, h_out)] {
                    let r = unrotate(q);
                    lo = Point::new(lo.x.min(r.x), lo.y.min(r.y));
                    hi = Point::new(hi.x.max(r.x), hi.y.max(r.y));
                }
            }
        }
        let origin = if lo.x.is_finite() {
            Point::new(f64::midpoint(lo.x, hi.x), f64::midpoint(lo.y, hi.y))
        } else {
            Point::new(0.0, 0.0)
        };
        Self { origin, sin, cos }
    }

    /// `outlines` in the frame: rotation taken out, origin moved to 0, rounded to 1e-6 mm.
    #[must_use]
    pub fn to_local(&self, outlines: &[PatternOutline]) -> Vec<PatternOutline> {
        let (sin, cos) = (self.sin, self.cos);
        let point = |p: Point| {
            Point::new(
                round_nm(p.x * cos + p.y * sin - self.origin.x),
                round_nm(-p.x * sin + p.y * cos - self.origin.y),
            )
        };
        let vector = |v: Vec2| {
            Vec2::new(
                round_nm(v.x * cos + v.y * sin),
                round_nm(-v.x * sin + v.y * cos),
            )
        };
        outlines
            .iter()
            .map(|(anchors, closed)| {
                let mapped = anchors
                    .iter()
                    .map(|&(p, a, b)| (point(p), vector(a), vector(b)))
                    .collect();
                (mapped, *closed)
            })
            .collect()
    }

    /// `pieces` made in the frame, back in document coordinates.
    #[must_use]
    pub fn to_document(&self, pieces: &[PatternPiece]) -> Vec<PatternPiece> {
        let (sin, cos) = (self.sin, self.cos);
        let point = |p: Point| {
            let (x, y) = (p.x + self.origin.x, p.y + self.origin.y);
            Point::new(x * cos - y * sin, x * sin + y * cos)
        };
        let vector = |v: Vec2| Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos);
        pieces
            .iter()
            .map(|piece| PatternPiece {
                outlines: piece
                    .outlines
                    .iter()
                    .map(|(anchors, closed)| {
                        let mapped = anchors
                            .iter()
                            .map(|&(p, a, b)| (point(p), vector(a), vector(b)))
                            .collect();
                        (mapped, *closed)
                    })
                    .collect(),
            })
            .collect()
    }
}

/// The pieces of `fill` over a shape with these outlines (document coordinates) and `rotation`
/// register: [`pattern_pieces`] in the shape's [`PatternFrame`], mapped back.
///
/// # Errors
/// As [`pattern_pieces`].
pub fn pattern_pieces_of(
    outlines: &[PatternOutline],
    rotation: Angle,
    fill: &PatternFill,
    line_width: Length,
    tolerance: Tolerance,
) -> Result<Vec<PatternPiece>, PatternError> {
    let frame = PatternFrame::of(outlines, rotation);
    let local = frame.to_local(outlines);
    Ok(frame.to_document(&pattern_pieces(&local, fill, line_width, tolerance)?))
}

/// The pieces of `fill` over the region of `outlines` (frame-local, nonzero rule), inset by the
/// margin and clipped to it. Pieces wholly inside keep their exact curves; cut pieces are
/// polylines of the 0.001 mm grid. With `whole_pieces` on a generator of shapes, a piece the
/// region or margin would cut (or that touches the boundary) is left out instead, and so is the
/// rest of its Sea waves scale (criterion 29). An open outline anywhere gives no pieces
/// (criterion 23).
/// Lines mode gives open lines and closed cells as they are; Shapes mode turns lines into closed
/// strips `line_width` wide. Pieces come in generation order; a line cut into parts gives one piece
/// per part.
///
/// # Errors
/// [`PatternError::TooMany`] when the lattice over the region's box would hold more than
/// [`PATTERN_MAX_PIECES`] pieces, [`PatternError::TooDetailed`] when making the pieces would take
/// more than [`PATTERN_MAX_WORK`]. The first, and the second for the work of making the raw pieces,
/// are known before any work; the work of cutting them is charged step by step and ends the
/// cutting when it runs out.
pub fn pattern_pieces(
    outlines: &[PatternOutline],
    fill: &PatternFill,
    line_width: Length,
    tolerance: Tolerance,
) -> Result<Vec<PatternPiece>, PatternError> {
    let flatten_mm = tolerance.as_mm() - 2.0 * GRID_MM;
    let usable = outlines.iter().all(|(anchors, closed)| {
        *closed
            && anchors.iter().all(|(p, a, b)| {
                [p.x, p.y, a.x, a.y, b.x, b.y]
                    .iter()
                    .all(|v| v.is_finite() && v.abs() < crate::MAX_COORDINATE_MM)
            })
    });
    if outlines.is_empty() || !usable || flatten_mm.is_nan() || flatten_mm <= 0.0 {
        return Ok(Vec::new());
    }
    let borrowed: Vec<Outline<'_>> = outlines
        .iter()
        .map(|(anchors, closed)| Outline::new(anchors, *closed))
        .collect();
    let Some(region) = Region::new(&borrowed, fill.margin.as_mm(), flatten_mm) else {
        return Ok(Vec::new());
    };
    let bounds = region.bounds_mm();
    let whole = fill.whole_pieces && fill.generator.kind().has_whole_pieces();
    let estimate = cost::estimate(&fill.generator, bounds, &region, whole);
    if estimate.pieces > PATTERN_MAX_PIECES {
        return Err(PatternError::TooMany {
            estimate: estimate.pieces,
        });
    }
    if estimate.work.saturating_add(estimate.cutting) > PATTERN_MAX_WORK {
        return Err(PatternError::TooDetailed);
    }
    let strips = cost::strips(&fill.generator, bounds, line_width);
    let mut budget = cost::Budget::after(estimate.work);
    let mut cutter = Cutter::new(&region, fill.mode, line_width, flatten_mm, strips);
    let mut raw = generate(fill, bounds, &region);
    if whole {
        raw = cutter.keep_whole(raw, &mut budget)?;
    }
    cutter.cut(raw, &mut budget)
}

fn right_angle(angle: Angle) -> Angle {
    Angle::from_radians(angle.as_radians() + std::f64::consts::FRAC_PI_2)
}

/// The raw pieces of `fill` over `bounds`.
fn generate(fill: &PatternFill, bounds: Bounds, region: &Region) -> Vec<Raw> {
    match fill.generator {
        PatternGenerator::Hatch { spacing, angle } => lines::hatch(bounds, spacing, angle),
        PatternGenerator::Crosshatch {
            spacing,
            angle,
            second_angle,
        } => {
            let mut out = lines::hatch(bounds, spacing, angle);
            out.extend(lines::hatch(bounds, spacing, second_angle));
            out
        }
        PatternGenerator::Grid { spacing, angle } => {
            let mut out = lines::hatch(bounds, spacing, angle);
            out.extend(lines::hatch(bounds, spacing, right_angle(angle)));
            out
        }
        PatternGenerator::Dots { spacing, diameter } => cells::dots(bounds, spacing, diameter),
        PatternGenerator::Honeycomb { cell_size, wall } => {
            cells::honeycomb(bounds, cell_size, wall)
        }
        PatternGenerator::BrickWall {
            width,
            height,
            joint,
        } => cells::bricks(bounds, width, height, joint),
        PatternGenerator::LeafForest {
            leaf_size,
            size_jitter,
            rotation_jitter,
            density,
        } => leaves::leaves(
            bounds,
            leaf_size,
            size_jitter,
            rotation_jitter,
            density,
            fill.seed,
        ),
        PatternGenerator::SeaWaves {
            radius,
            rings,
            ring_spacing,
        } => scales::sea_waves(bounds, radius, rings, ring_spacing),
        PatternGenerator::Waves {
            wavelength,
            amplitude,
            spacing,
            angle,
        } => lines::waves(bounds, wavelength, amplitude, spacing, angle),
        PatternGenerator::InterpolationWaves { lines } => blend::interpolation_waves(region, lines),
    }
}
