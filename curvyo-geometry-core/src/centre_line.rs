//! Centre lines of filled shapes (ADR 0063, `specs/0117-centre-lines`): a staged entry that
//! builds each region's skeleton and chains once and finishes them for the options.

mod blob;
mod branch_trim;
mod chains;
mod density;
mod fit;
mod inner;
#[cfg(test)]
mod junction_tests;
mod junctions;
mod kept;
#[cfg(test)]
mod measure_tests;
mod profile;
#[cfg(test)]
mod quality_tests;
mod rings;
mod safe_fit;
mod stages;
#[cfg(test)]
mod tests;
mod thick;
#[cfg(test)]
mod tip_tests;
mod tips;

use crate::medial_axis::{
    AxisSample, CentreLineError, HoleMeasure, MedialAxis, Region, medial_axis, regions,
};
use crate::{Outline, OutlineTriple, WidthProfile};
use branch_trim::{ParentCurve, cut_branch};
use chains::{Chain, ChainEnd, chains};
use curvyo_document_core::{Length, Point as DocPoint};
use fit::{Budget, Fitted, fit, fit_with_tips, tolerance};
pub use stages::{CentreStages, StageRuns};

/// The options of Find centre lines (`specs/0117-centre-lines` criteria 10, 12, 13).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CentreOptions {
    /// The branch overlap `p` as a fraction of the parent's width, from -0.5 to 0.5.
    pub overlap: f64,
    /// Thick parts: the largest local width a part may have and still get a centre line; a
    /// wider part stays filled with its own nodes. `None` keeps only what no centre line can
    /// describe (a disc).
    pub thick_parts: Option<Length>,
    /// With Keep inner areas on: how far each hole moves inward (negative: outward).
    pub inner_offset: Option<Length>,
    /// How far the fit may stray to save nodes and width points, from 0 to 1: the curve and the
    /// profile stay within a quarter of `T(w)` at 0, half of it at 0.5, `T(w)` at the default
    /// 0.75 and three times `T(w)` at 1.
    pub optimise: f64,
    /// Join tips: arms that meet at a pointed tip of the shape run into the tip as one path,
    /// its width falling to 0 there; off, the other arm is a branch whose butt overlaps the
    /// path it joins (`specs/0117-centre-lines` criterion 9, CENTRE-TIPS).
    pub join_tips: bool,
}

impl Default for CentreOptions {
    fn default() -> Self {
        Self {
            overlap: 0.2,
            thick_parts: Some(DEFAULT_THICK_PARTS),
            inner_offset: None,
            optimise: DEFAULT_OPTIMISE,
            join_tips: DEFAULT_JOIN_TIPS,
        }
    }
}

/// The default of [`CentreOptions::optimise`].
pub const DEFAULT_OPTIMISE: f64 = 0.75;

/// The default of [`CentreOptions::join_tips`] (customer 2026-11-21, Proposal).
pub const DEFAULT_JOIN_TIPS: bool = true;

/// The default of [`CentreOptions::thick_parts`] (customer 2026-11-18, Proposal).
pub const DEFAULT_THICK_PARTS: Length = Length::from_mm(6.0);

/// A region kept filled (criterion 12): its rings, outer ring first, with the subject's own nodes.
pub type KeptRegion = Vec<Vec<OutlineTriple>>;

/// Whether a centre path is a main path (or ring) or a branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CentreRole {
    /// The main path of a region, or a closed ring: Round caps.
    Main,
    /// A branch: Butt caps, a free round end drawn by its profile.
    Branch,
}

/// One centre path: anchors as `(point, handle_in, handle_out)`, a base width and a profile.
#[derive(Debug, Clone)]
pub struct CentrePath {
    /// Main path or branch.
    pub role: CentreRole,
    /// The anchors in path order.
    pub anchors: Vec<OutlineTriple>,
    /// Whether the path is closed.
    pub closed: bool,
    /// The largest width: the base stroke width.
    pub width: Length,
    /// The measured widths as multipliers of `width`.
    pub profile: WidthProfile,
}

/// The result for one subject.
#[derive(Debug, Clone, Default)]
pub struct CentreLines {
    /// Centre paths: per region the main path, then its branches.
    pub paths: Vec<CentrePath>,
    /// Parts kept filled (criterion 12): each its outer outline then its holes, with the
    /// subject's own nodes, split only where a cut meets them.
    pub filled: Vec<KeptRegion>,
    /// Inner areas (criterion 13): closed polygons, outer outlines positive, holes negative.
    pub inner_areas: Vec<Vec<DocPoint>>,
    /// Regions whose skeleton failed and are kept as they are.
    pub failed: usize,
}

impl CentreLines {
    /// Whether every region stays filled, so the subject stays as it is.
    #[must_use]
    pub fn all_filled(&self) -> bool {
        self.paths.is_empty()
    }
}

/// One region's work that does not depend on the options.
#[derive(Debug, Clone)]
struct RegionSkeleton {
    region: Region,
    /// The chains with Join tips on (the default).
    chains: Vec<Chain>,
    /// Fitted main, through and ring chains, by chain index; branches are fitted in `finish`.
    fitted: Vec<Option<Fitted>>,
    /// The axis, kept for the chains with Join tips off when a tip was joined.
    axis: Option<std::sync::Arc<MedialAxis>>,
    /// The chains with Join tips off and their fit, made on first use.
    untied: std::sync::OnceLock<(Vec<Chain>, Vec<Option<Fitted>>)>,
    /// `w_max` and the outer ring's diameter.
    blob: blob::BlobMeasure,
    /// The holes' measures, in ring order.
    holes: Vec<HoleMeasure>,
    failed: bool,
}

/// The skeletons of one subject's regions (ADR 0063 §8): built once, finished per option change.
#[derive(Debug, Clone)]
pub struct CentreSkeleton {
    regions: Vec<RegionSkeleton>,
    /// The subject's outlines, for the parts kept filled.
    sources: Vec<Vec<OutlineTriple>>,
}

impl CentreSkeleton {
    /// Builds the skeleton of the region `outlines` paint (nonzero).
    ///
    /// # Errors
    /// [`CentreLineError`] for a nonfinite or out-of-range coordinate. A region that fails to
    /// triangulate is kept filled and counted, not an error.
    pub fn build(outlines: &[Outline<'_>]) -> Result<Self, CentreLineError> {
        let regions = regions(outlines)?
            .into_iter()
            .map(RegionSkeleton::build)
            .collect();
        let sources = outlines.iter().map(|o| o.anchors.to_vec()).collect();
        Ok(Self { regions, sources })
    }

    /// The skeleton of regions already on the grid.
    #[cfg(test)]
    pub(crate) fn from_regions(regions: Vec<Region>) -> Self {
        Self {
            regions: regions.into_iter().map(RegionSkeleton::build).collect(),
            sources: Vec::new(),
        }
    }
}

impl RegionSkeleton {
    fn build(region: Region) -> Self {
        let Ok(axis) = medial_axis(&region) else {
            return Self {
                blob: blob::BlobMeasure::of(&region, 0.0),
                region,
                chains: Vec::new(),
                fitted: Vec::new(),
                axis: None,
                untied: std::sync::OnceLock::new(),
                holes: Vec::new(),
                failed: true,
            };
        };
        let widest = axis
            .holes
            .iter()
            .filter_map(|h| h.filled)
            .fold(2.0 * axis.widest, f64::max);
        let (chains, joined) = chains(&axis, &region.rings_mm(), DEFAULT_JOIN_TIPS);
        let fitted = fit_default(&chains);
        Self {
            axis: joined.then(|| std::sync::Arc::new(axis.clone())),
            untied: std::sync::OnceLock::new(),
            blob: blob::BlobMeasure::of(&region, widest),
            region,
            chains,
            fitted,
            holes: axis.holes,
            failed: false,
        }
    }
}

/// The main, through and ring chains of `chains` fitted at the default Optimise.
fn fit_default(chains: &[Chain]) -> Vec<Option<Fitted>> {
    chains
        .iter()
        .map(|chain| {
            (!is_branch(chain))
                .then(|| fit(&chain.samples, chain.closed, Budget::of(DEFAULT_OPTIMISE)))
                .flatten()
        })
        .collect()
}

impl RegionSkeleton {
    /// The chains for Join tips `join` and their fit at the default Optimise; the other
    /// setting's are made on first use, and only when a tip was joined.
    fn chain_set(&self, join: bool) -> (&[Chain], &[Option<Fitted>]) {
        match (&self.axis, join == DEFAULT_JOIN_TIPS) {
            (Some(axis), false) => {
                let (chains, fitted) = self.untied.get_or_init(|| {
                    let (chains, _) = chains(axis, &self.region.rings_mm(), join);
                    let fitted = fit_default(&chains);
                    (chains, fitted)
                });
                (chains, fitted)
            }
            _ => (&self.chains, &self.fitted),
        }
    }

    /// Whether the chains for Join tips `join` are made yet.
    fn chain_set_ready(&self, join: bool) -> bool {
        self.axis.is_none() || join == DEFAULT_JOIN_TIPS || self.untied.get().is_some()
    }
}

/// The main path first, then the other through chains, then the branches cut at `overlap`, each
/// with its chain's index; `fitted` holds the fitted main and through chains.
fn finish_chains(
    chains: &[Chain],
    through: &[Option<Fitted>],
    overlap: f64,
    optimise: f64,
) -> Vec<(usize, CentrePath)> {
    let budget = Budget::of(optimise);
    let mut fitted = through.to_vec();
    let mut done: Vec<bool> = chains.iter().map(|c| !is_branch(c)).collect();
    // A parent's flattened curve, built once for all of its branches.
    let mut curves: Vec<Option<ParentCurve>> = chains.iter().map(|_| None).collect();
    // Branches of branches need their parent first.
    for _ in 0..chains.len() {
        let mut progress = false;
        for (i, chain) in chains.iter().enumerate() {
            if done[i] {
                continue;
            }
            let ready = [chain.start, chain.end].iter().all(|end| match end {
                ChainEnd::Junction {
                    parent: Some(p), ..
                } => done[*p],
                _ => true,
            });
            if ready {
                fitted[i] = fit_branch(chain, &fitted, &mut curves, overlap, budget);
                done[i] = true;
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }
    let mut order: Vec<usize> = (0..chains.len()).collect();
    order.sort_by_key(|&i| (!chains[i].main, is_branch(&chains[i])));
    order
        .into_iter()
        .filter_map(|i| {
            let f = fitted[i].clone()?;
            Some((
                i,
                CentrePath {
                    role: if is_branch(&chains[i]) {
                        CentreRole::Branch
                    } else {
                        CentreRole::Main
                    },
                    anchors: f.anchors,
                    closed: f.closed,
                    width: f.width,
                    profile: f.profile,
                },
            ))
        })
        .collect()
}

/// The main and through chains of `chains` fitted for `optimise`; branches are `None`. `cached`
/// holds them fitted at the default Optimise.
fn fit_through(
    chains: &[Chain],
    cached: Option<&[Option<Fitted>]>,
    optimise: f64,
) -> Vec<Option<Fitted>> {
    match cached {
        Some(cached) if optimise.to_bits() == DEFAULT_OPTIMISE.to_bits() => cached.to_vec(),
        _ => {
            let budget = Budget::of(optimise);
            chains
                .iter()
                .map(|chain| {
                    (!is_branch(chain))
                        .then(|| fit(&chain.samples, chain.closed, budget))
                        .flatten()
                })
                .collect()
        }
    }
}

/// The support points of a branch's round free end, as shares of its radius along the axis
/// (CENTRE-TIP A, customer 2026-11-17): four points placed so that the smoothstep between them
/// stays within 0.08 r of the true semicircle (0.077 r measured), the tolerance of criterion 8.
const ROUND_END_SUPPORT: [f64; 4] = [0.4, 0.825, 0.975, 1.0];

/// Whether a chain ends at a junction where it does not continue.
fn is_branch(chain: &Chain) -> bool {
    !chain.main
        && (matches!(chain.start, ChainEnd::Junction { .. })
            || matches!(chain.end, ChainEnd::Junction { .. }))
}

/// A branch cut at both junction ends and given its round free end, then fitted.
fn fit_branch(
    chain: &Chain,
    fitted: &[Option<Fitted>],
    curves: &mut [Option<ParentCurve>],
    overlap: f64,
    budget: Budget,
) -> Option<Fitted> {
    let mut samples = chain.samples.clone();
    let mut tips = [0, 0];
    for (index, end) in [chain.start, chain.end].into_iter().enumerate() {
        if index == 1 {
            samples.reverse();
        }
        match end {
            ChainEnd::Junction {
                centre,
                direction,
                width,
                parent,
                ..
            } => {
                if let Some(p) = parent
                    && let Some(parent) = fitted[p].as_ref()
                {
                    let curve = curves[p].get_or_insert_with(|| ParentCurve::new(parent));
                    samples = cut_branch(&samples, centre, direction, width, curve, overlap);
                }
            }
            ChainEnd::Free => {
                if round_end(&mut samples, true) {
                    tips[index] = ROUND_END_SUPPORT.len() + 1;
                }
            }
            ChainEnd::Cut => {}
        }
        if index == 1 {
            samples.reverse();
        }
    }
    fit_with_tips(&samples, false, budget, tips)
}

/// Extends a rounded free end (its last clearance above the tolerance) by `r` to the tip with
/// widths `2 sqrt(r² - s²)` (CENTRE-CAPS A); `at_start` names the end at index 0. Whether it
/// did.
fn round_end(samples: &mut Vec<AxisSample>, at_start: bool) -> bool {
    if samples.len() < 2 {
        return false;
    }
    let mut extended = false;
    if at_start {
        samples.reverse();
    }
    let last = samples[samples.len() - 1];
    let r = last.radius;
    if 2.0 * r > tolerance(2.0 * r) {
        let from = samples
            .iter()
            .rev()
            .find(|s| (s.point - last.point).hypot() >= r)
            .unwrap_or(&samples[0])
            .point;
        let d = last.point - from;
        if d.hypot() > 0.0 {
            let u = d / d.hypot();
            for share in ROUND_END_SUPPORT {
                let s = r * share;
                samples.push(AxisSample {
                    point: last.point + u * s,
                    radius: (r * r - s * s).max(0.0).sqrt(),
                });
            }
            extended = true;
        }
    }
    if at_start {
        samples.reverse();
    }
    extended
}
