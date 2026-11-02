//! What a pattern costs (`specs/0071-pattern-fill` criterion 18, acceptance report finding 2): its
//! pieces and its work, so a pattern that would hold the UI thread too long is refused. The pieces
//! and the work of making the raw pieces are known before anything is made; the work of cutting
//! them is charged step by step and ends the cutting when the budget runs out; the least work of
//! cutting closed pieces at the boundary is estimated too, so such a pattern is refused before
//! any work when it clearly does not fit. Work is counted in units of about a nanosecond of a
//! native release build on the build VM; the unit costs are set so that the measured time stays
//! below about 0.7 ns per unit in every case measured, which leaves room for a CI runner about
//! 1.9 times slower (`plan.md`, "Measurements").

use curvyo_document_core::{Angle, Length, PatternGenerator};

use super::region::Region;
use super::{Bounds, PatternError, blend, cells, leaves, lines, scales};

/// Above this much work the pattern is not generated: at most about 70 ms of a native release
/// build on the build VM (`plan.md`, "Measurements").
pub const PATTERN_MAX_WORK: u64 = 100_000_000;

/// A step of making a pattern, charged at its unit cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    /// Making, placing and writing one anchor of a raw piece.
    Anchor,
    /// Throwing one candidate leaf.
    Throw,
    /// Cutting one line at the boundary, besides its points.
    Line,
    /// One point of a line cut at the boundary.
    LinePoint,
    /// Testing one boundary edge against a line or a ray.
    EdgeTest,
    /// One overlay (a closed piece or a strip cut at the boundary), besides its edges.
    Overlay,
    /// One edge of a piece in an overlay.
    OverlayEdge,
    /// One edge of the region near a piece in its overlay: the region's edges are long and lie
    /// side by side along a detailed outline, which costs the overlay several times a piece's
    /// own edge (the needle star of `plan.md`, "Measurements").
    AroundEdge,
    /// One pair of edges of the region near a piece in its overlay: along a detailed outline the
    /// overlay compares them with each other, so its work grows faster than their count.
    AroundPair,
    /// Scanning one region edge to cut a tile of the region.
    Scan,
    /// One line outlined as a strip in Shapes mode, besides its points.
    Strip,
    /// One point of a line outlined as a strip in Shapes mode.
    StripPoint,
    /// One unit of folding in a strip (`clip::folds`).
    StripFold,
    /// One crossing of two lines along the axes whose strips are merged in Shapes mode.
    Crossing,
    /// One crossing of two other lines whose strips are merged in Shapes mode.
    RoundedCrossing,
    /// One crossing of a line with the boundary, whose strip is cut in Shapes mode.
    StripEdge,
    /// One edge in the overlay that cuts a strip of Shapes mode.
    StripOverlayEdge,
    /// One edge in the overlay that merges the strips of Shapes mode.
    MergedEdge,
    /// One edge of the region, made before anything else.
    RegionEdge,
    /// The region's cell map, made before anything else.
    Region,
}

impl Step {
    /// The work of one step.
    const fn unit(self) -> u64 {
        match self {
            Self::Anchor => 45,
            Self::Throw => 35,
            Self::Line => 3_000,
            Self::LinePoint => 90,
            Self::EdgeTest => 12,
            Self::Overlay => 1_500,
            Self::OverlayEdge => 140,
            Self::AroundEdge => 1_300,
            Self::AroundPair => 3,
            Self::Scan => 60,
            Self::Strip => 24_000,
            Self::StripPoint => 700,
            Self::StripFold => 900,
            Self::Crossing => 2_600,
            Self::RoundedCrossing => 4_000,
            Self::StripEdge => 36_000,
            Self::StripOverlayEdge | Self::MergedEdge => 600,
            Self::RegionEdge => 4_500,
            Self::Region => 700_000,
        }
    }
}

/// The work of `steps`, each `(step, how many)`.
fn work_of(steps: &[(Step, u64)]) -> u64 {
    steps.iter().fold(0_u64, |sum, &(step, count)| {
        sum.saturating_add(step.unit().saturating_mul(count))
    })
}

/// The work a pattern may still do; every step is charged before it is done.
pub(crate) struct Budget {
    left: u64,
}

impl Budget {
    /// [`PATTERN_MAX_WORK`] less `spent`.
    pub(crate) const fn after(spent: u64) -> Self {
        Self {
            left: PATTERN_MAX_WORK.saturating_sub(spent),
        }
    }

    /// Charges `steps`, each `(step, how many)`.
    ///
    /// # Errors
    /// [`PatternError::TooDetailed`] when their work is more than is left.
    pub(crate) fn spend(&mut self, steps: &[(Step, u64)]) -> Result<(), PatternError> {
        let work = work_of(steps);
        if work > self.left {
            return Err(PatternError::TooDetailed);
        }
        self.left -= work;
        Ok(())
    }
}

/// The cost of a pattern before it is made.
pub(crate) struct Estimate {
    /// The pieces the lattice over the region's box holds.
    pub(crate) pieces: usize,
    /// The work of making the region and the raw pieces.
    pub(crate) work: u64,
    /// About the least work of cutting the closed pieces that meet the boundary: refused before
    /// any work when it does not fit with `work`, charged only as the cutting is done.
    pub(crate) cutting: u64,
}

/// The pieces a pattern of closed pieces is expected to cut at the boundary: its share of the
/// region's cells that the boundary crosses, each charged at least one overlay of three edges of
/// its own and three of the region (a piece of the size of a cell or larger meets more). With
/// `whole` pieces only, such a piece is left out after a test of its points against the
/// boundary, charged at three points.
fn boundary_cutting(
    generator: &PatternGenerator,
    pieces: usize,
    region: &Region,
    whole: bool,
) -> u64 {
    let closed = matches!(
        generator,
        PatternGenerator::Dots { .. }
            | PatternGenerator::Honeycomb { .. }
            | PatternGenerator::BrickWall { .. }
            | PatternGenerator::LeafForest { .. }
    );
    if !closed {
        return 0;
    }
    let (boundary, cells) = region.boundary_cells();
    let cut =
        u128::from(count(pieces)) * u128::from(count(boundary)) / u128::from(count(cells).max(1));
    let cut = u64::try_from(cut).unwrap_or(u64::MAX);
    if whole {
        return work_of(&[(Step::OverlayEdge, cut.saturating_mul(3))]);
    }
    work_of(&[
        (Step::Overlay, cut),
        (Step::OverlayEdge, cut.saturating_mul(3)),
        (Step::AroundEdge, cut.saturating_mul(3)),
    ])
}

fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

fn right_angle(angle: Angle) -> Angle {
    Angle::from_radians(angle.as_radians() + std::f64::consts::FRAC_PI_2)
}

/// The pieces and the work of making `region` and `generator`'s raw pieces over `bounds`; `whole`
/// when only whole pieces are kept.
pub(crate) fn estimate(
    generator: &PatternGenerator,
    bounds: Bounds,
    region: &Region,
    whole: bool,
) -> Estimate {
    let made = raw_estimate(generator, bounds, region);
    let edges: usize = region.contours.iter().map(Vec::len).sum();
    Estimate {
        pieces: made.pieces,
        work: made.work.saturating_add(work_of(&[
            (Step::Region, 1),
            (Step::RegionEdge, count(edges)),
        ])),
        cutting: boundary_cutting(generator, made.pieces, region, whole),
    }
}

/// The pieces and the work of making `generator`'s raw pieces over `bounds`.
fn raw_estimate(generator: &PatternGenerator, bounds: Bounds, region: &Region) -> Estimate {
    let per = |pieces: usize, anchors: usize| Estimate {
        pieces,
        work: work_of(&[(Step::Anchor, count(pieces.saturating_mul(anchors)))]),
        cutting: 0,
    };
    match *generator {
        PatternGenerator::Hatch { spacing, angle } => {
            per(lines::hatch_count(bounds, spacing, angle), 2)
        }
        PatternGenerator::Crosshatch {
            spacing,
            angle,
            second_angle,
        } => per(
            lines::hatch_count(bounds, spacing, angle).saturating_add(lines::hatch_count(
                bounds,
                spacing,
                second_angle,
            )),
            2,
        ),
        PatternGenerator::Grid { spacing, angle } => per(
            lines::hatch_count(bounds, spacing, angle).saturating_add(lines::hatch_count(
                bounds,
                spacing,
                right_angle(angle),
            )),
            2,
        ),
        PatternGenerator::Dots { spacing, .. } => per(cells::dots_count(bounds, spacing), 4),
        PatternGenerator::Honeycomb { cell_size, .. } => {
            per(cells::honeycomb_count(bounds, cell_size), 6)
        }
        PatternGenerator::BrickWall { width, height, .. } => {
            per(cells::bricks_count(bounds, width, height), 4)
        }
        PatternGenerator::LeafForest {
            leaf_size, density, ..
        } => {
            let leaves = per(leaves::leaf_count(bounds, leaf_size, density), 2);
            let throws = work_of(&[(
                Step::Throw,
                count(leaves::leaf_throws(bounds, leaf_size, density)),
            )]);
            Estimate {
                pieces: leaves.pieces,
                work: leaves.work.saturating_add(throws),
                cutting: 0,
            }
        }
        PatternGenerator::SeaWaves {
            radius,
            rings,
            ring_spacing,
        } => per(
            scales::sea_waves_count(bounds, radius, rings, ring_spacing),
            3,
        ),
        PatternGenerator::Waves {
            wavelength,
            amplitude,
            spacing,
            angle,
        } => {
            let (count, nodes) = lines::waves_count(bounds, wavelength, amplitude, spacing, angle);
            per(count, nodes)
        }
        PatternGenerator::InterpolationWaves { lines } => {
            per(lines.get() as usize, blend::samples(region))
        }
    }
}

/// Whether lines at `angle` run along an axis, so their crossings lie on the grid.
fn on_axis(angle: Angle) -> bool {
    let quarter = angle.as_radians() / std::f64::consts::FRAC_PI_2;
    (quarter - quarter.round()).abs() < 1e-12
}

/// How the strips of Shapes mode are cut at the region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Strips {
    /// Each strip on its own: Interpolation waves, whose lines meet in fans at the region's sides,
    /// where any union of their strips costs far more than cutting each on its own.
    EachOwn,
    /// Parallel lines (Hatch, Waves) in batches of lines; `overlap` when neighbouring strips can
    /// overlap (spacing below the strip's width across the lines), so the batches are merged.
    Batches {
        /// Whether neighbouring strips can overlap.
        overlap: bool,
    },
    /// Crossing lines (Grid, Crosshatch, Sea waves) merged into one outline, so a laser cuts
    /// each crossing once (acceptance report finding 4), with their crossings as the step they
    /// are charged at: crossings of lines along the axes lie on the grid and are united before
    /// the cut, the others are rounded and cut in one overlay (`plan.md`, "Measurements").
    Merged((Step, u64)),
}

/// How the strips of `generator` are cut, for strips `line_width` wide.
pub(crate) fn strips(generator: &PatternGenerator, bounds: Bounds, line_width: Length) -> Strips {
    let lattice = |spacing, a: Angle, b: Angle| {
        let crossings = count(lines::hatch_count(bounds, spacing, a))
            .saturating_mul(count(lines::hatch_count(bounds, spacing, b)));
        let step = if on_axis(a) && on_axis(b) {
            Step::Crossing
        } else {
            Step::RoundedCrossing
        };
        Strips::Merged((step, crossings))
    };
    let w = line_width.as_mm();
    match *generator {
        PatternGenerator::Hatch { spacing, .. } => Strips::Batches {
            overlap: spacing.as_mm() < w,
        },
        PatternGenerator::Waves {
            wavelength,
            amplitude,
            spacing,
            ..
        } => {
            // Across the lines a strip is widest where the wave is steepest.
            let slope = amplitude.as_mm() * std::f64::consts::TAU / wavelength.as_mm();
            Strips::Batches {
                overlap: spacing.as_mm() < w * (1.0 + slope * slope).sqrt(),
            }
        }
        PatternGenerator::Crosshatch {
            spacing,
            angle,
            second_angle,
        } => lattice(spacing, angle, second_angle),
        PatternGenerator::Grid { spacing, angle } => lattice(spacing, angle, right_angle(angle)),
        PatternGenerator::SeaWaves {
            radius,
            rings,
            ring_spacing,
        } => Strips::Merged((
            Step::RoundedCrossing,
            // Each arc crosses the arcs of the scales beside, above and below it.
            count(scales::sea_waves_count(bounds, radius, rings, ring_spacing))
                .saturating_mul(count(scales::rings_drawn(radius, rings, ring_spacing)))
                / 2,
        )),
        // Closed pieces have no strips.
        _ => Strips::EachOwn,
    }
}
