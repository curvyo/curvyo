//! The staged finish of a subject's skeletons (ADR 0063 amendment 2026-11-18, W): each stage keeps
//! its last result keyed by the options it reads, so a slider reruns only its own stage and those
//! after it.

use curvyo_document_core::{Length, Point as DocPoint};
use kurbo::{CubicBez, Point};

use super::chains::Chain;
use super::fit::{Fitted, tolerance};
use super::rings::{self, KeptRegion};
use super::thick::{self, Cut};
use super::{CentreLines, CentreOptions, CentrePath, CentreSkeleton, RegionSkeleton};
use super::{finish_chains, fit_through, inner, kept};
use crate::Outline;
use crate::medial_axis::AxisSample;

/// How often each stage ran, for the tests of the memo.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StageRuns {
    /// The cut at Thick parts.
    pub cut: usize,
    /// The fit of the main and through chains (Optimise).
    pub through: usize,
    /// The branch cut and fit (Overlap) with the kept parts.
    pub branches: usize,
    /// The inner areas (Offset).
    pub inner: usize,
    /// The chains made for a Join tips setting other than the default.
    pub tips: usize,
}

/// A region's rings in mm and their segments of the subject's own outlines.
type RegionRings = (Vec<Vec<Point>>, Vec<Vec<CubicBez>>);

/// The key of the chain stages: Join tips and the Thick parts limit (mm, with its tolerance),
/// `None` for no limit.
type ThickKey = (bool, Option<u64>);

/// The paths and kept parts of a region for one set of options.
#[derive(Debug, Clone, Default)]
struct Finished {
    paths: Vec<CentrePath>,
    filled: Vec<KeptRegion>,
    /// Whether inner areas follow, and the holes they skip (inside a thick part).
    inner: Option<Vec<bool>>,
}

/// The inner areas stage's memo: its keys (Thick parts, Offset, cut-outs) and the areas.
type InnerMemo = ((ThickKey, u64, u64), Vec<Vec<DocPoint>>);

/// One region's memo: each stage's last result and the keys it was made for.
#[derive(Debug, Clone, Default)]
struct RegionMemo {
    cut: Option<(ThickKey, Option<Cut>)>,
    through: Option<Through>,
    finished: Option<(ThickKey, u64, u64, Finished)>,
    inner: Option<InnerMemo>,
}

/// A subject's skeletons with the memo of their finish stages (ADR 0063 amendment 2026-11-18, W):
/// the worker keeps one per subject, so moving a slider never rebuilds a skeleton. The result is
/// the same as [`CentreSkeleton::finish`] for the same options.
#[derive(Debug, Clone)]
pub struct CentreStages {
    skeleton: CentreSkeleton,
    /// Per region its rings in mm and their segments of the subject's own outlines.
    rings: Vec<RegionRings>,
    memo: Vec<RegionMemo>,
    runs: StageRuns,
}

/// A fingerprint of the cut-outs, so the inner areas are made again when they change.
fn cutouts_key(cutouts: &[Outline<'_>]) -> u64 {
    // FNV-1a over the coordinates' bits: deterministic and cheap.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bits: u64| {
        hash ^= bits;
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    };
    for outline in cutouts {
        eat(u64::from(outline.closed));
        for (p, i, o) in outline.anchors {
            for v in [p.x, p.y, i.x, i.y, o.x, o.y] {
                eat(v.to_bits());
            }
        }
    }
    hash
}

impl CentreStages {
    /// The stages of `skeleton`, nothing finished yet.
    #[must_use]
    pub fn new(skeleton: CentreSkeleton) -> Self {
        let sources = rings::source_rings(&skeleton.sources);
        let rings = skeleton
            .regions
            .iter()
            .map(|region| {
                let rings = region.region.rings_mm();
                let segments = rings::region_segments(&rings, &sources);
                (rings, segments)
            })
            .collect();
        let memo = vec![RegionMemo::default(); skeleton.regions.len()];
        Self {
            skeleton,
            rings,
            memo,
            runs: StageRuns::default(),
        }
    }

    /// How often each stage ran since [`CentreStages::new`].
    #[must_use]
    pub fn runs(&self) -> StageRuns {
        self.runs
    }

    /// The centre lines for `options`, rerunning only the stages whose options changed;
    /// `cutouts` as in [`CentreSkeleton::finish`].
    pub fn finish(&mut self, options: &CentreOptions, cutouts: &[Outline<'_>]) -> CentreLines {
        let mut lines = CentreLines::default();
        let cutouts_key = cutouts_key(cutouts);
        for ((region, (rings, segments)), memo) in self
            .skeleton
            .regions
            .iter()
            .zip(&self.rings)
            .zip(&mut self.memo)
        {
            let input = RegionInput {
                region,
                rings,
                segments,
            };
            input.finish(
                options,
                cutouts,
                cutouts_key,
                memo,
                &mut self.runs,
                &mut lines,
            );
        }
        lines
    }
}

impl CentreSkeleton {
    /// The centre lines for `options`; `cutouts` are other selected shapes whose grown outline
    /// is cut out of the inner areas (criterion 14).
    #[must_use]
    pub fn finish(&self, options: &CentreOptions, cutouts: &[Outline<'_>]) -> CentreLines {
        let mut lines = CentreLines::default();
        let sources = rings::source_rings(&self.sources);
        let key = cutouts_key(cutouts);
        let mut runs = StageRuns::default();
        for region in &self.regions {
            let rings = region.region.rings_mm();
            let segments = rings::region_segments(&rings, &sources);
            let input = RegionInput {
                region,
                rings: &rings,
                segments: &segments,
            };
            let mut memo = RegionMemo::default();
            input.finish(options, cutouts, key, &mut memo, &mut runs, &mut lines);
        }
        lines
    }
}

/// One region with its rings, as the stages read it.
struct RegionInput<'a> {
    region: &'a RegionSkeleton,
    rings: &'a [Vec<Point>],
    segments: &'a [Vec<CubicBez>],
}

/// The option keys of the stages: Thick parts (mm with its tolerance), Optimise and Overlap as
/// bits.
#[derive(Debug, Clone, Copy)]
struct Keys {
    thick: ThickKey,
    optimise: u64,
    overlap: u64,
}

impl RegionInput<'_> {
    /// Adds this region's result for `options` to `lines`, through `memo`: the stages in order,
    /// each rerun only when its keys changed.
    fn finish(
        &self,
        options: &CentreOptions,
        cutouts: &[Outline<'_>],
        cutouts_key: u64,
        memo: &mut RegionMemo,
        runs: &mut StageRuns,
        lines: &mut CentreLines,
    ) {
        let skeleton = self.region;
        if skeleton.failed {
            lines.failed += 1;
            lines.filled.push(rings::whole(self.segments));
            return;
        }
        if skeleton.blob.is_disc()
            || options
                .thick_parts
                .is_some_and(|limit| skeleton.blob.is_blob(limit))
        {
            lines.filled.push(rings::whole(self.segments));
            return;
        }
        // A part counts as thick when it is wider than the limit by more than the tolerance,
        // so a stroke drawn exactly that wide does not flicker on its traced noise.
        let limit = options
            .thick_parts
            .map(|l| l.as_mm() + tolerance(l.as_mm()));
        if !skeleton.chain_set_ready(options.join_tips) {
            runs.tips += 1;
        }
        let (all_chains, all_fitted) = skeleton.chain_set(options.join_tips);
        let keys = Keys {
            thick: (options.join_tips, limit.map(f64::to_bits)),
            optimise: options.optimise.to_bits(),
            overlap: options.overlap.to_bits(),
        };
        self.run_cut(all_chains, limit, keys.thick, memo, runs);
        let cut = memo.cut.as_ref().and_then(|(_, cut)| cut.as_ref());
        let chains: &[Chain] = cut.map_or(all_chains, |cut| &cut.pieces);
        if cut.is_none() && all_fitted.iter().all(Option::is_none) {
            lines.filled.push(rings::whole(self.segments));
            return;
        }
        let cached = cut.is_none().then_some(all_fitted);
        run_through(
            chains,
            cached,
            options.optimise,
            keys,
            &mut memo.through,
            &mut memo.finished,
            runs,
        );
        let Some(finished) = self.run_branches(
            chains,
            cut,
            options,
            keys,
            memo.through.as_ref(),
            &mut memo.finished,
            runs,
        ) else {
            return;
        };
        lines.filled.extend(finished.filled.iter().cloned());
        lines.paths.extend(finished.paths.iter().cloned());
        if let (Some(offset), Some(kept_holes)) = (options.inner_offset, finished.inner.as_ref()) {
            let key = (keys.thick, offset.as_mm().to_bits(), cutouts_key);
            let areas = self.run_inner(offset, kept_holes, cutouts, key, &mut memo.inner, runs);
            lines.inner_areas.extend(areas.iter().cloned());
        }
    }

    /// The inner areas stage: the holes moved by `offset` less the grown `cutouts`, rerun when
    /// `key` (Thick parts, Offset, cut-outs) changed; the stage's result.
    fn run_inner<'m>(
        &self,
        offset: Length,
        kept_holes: &[bool],
        cutouts: &[Outline<'_>],
        key: (ThickKey, u64, u64),
        inner: &'m mut Option<InnerMemo>,
        runs: &mut StageRuns,
    ) -> &'m [Vec<DocPoint>] {
        if inner.as_ref().is_none_or(|(k, _)| *k != key) {
            runs.inner += 1;
            *inner = Some((key, inner_areas(self.region, offset, cutouts, kept_holes)));
        }
        inner.as_ref().map_or(&[], |(_, areas)| areas.as_slice())
    }

    /// The cut stage: the chains cut at the Thick parts `limit`, rerun when `key` changed.
    fn run_cut(
        &self,
        chains: &[Chain],
        limit: Option<f64>,
        key: ThickKey,
        memo: &mut RegionMemo,
        runs: &mut StageRuns,
    ) {
        if memo.cut.as_ref().is_some_and(|(k, _)| *k == key) {
            return;
        }
        runs.cut += 1;
        let skeleton = self.region;
        let cut =
            limit.and_then(|limit| thick::cut(chains, &skeleton.holes, &self.rings[1..], limit));
        memo.cut = Some((key, cut));
        memo.through = None;
    }

    /// The branches stage: the paths with their branches cut at the overlap and the kept parts,
    /// rerun when a key changed; the stage's result.
    #[allow(clippy::too_many_arguments)]
    fn run_branches<'m>(
        &self,
        chains: &[Chain],
        cut: Option<&Cut>,
        options: &CentreOptions,
        keys: Keys,
        through: Option<&Through>,
        finished: &'m mut Option<(ThickKey, u64, u64, Finished)>,
        runs: &mut StageRuns,
    ) -> Option<&'m Finished> {
        if finished.as_ref().is_none_or(|(k, o, v, _)| {
            *k != keys.thick || *o != keys.optimise || *v != keys.overlap
        }) {
            runs.branches += 1;
            let through = through.map_or(&[][..], |(_, _, t)| t.as_slice());
            let paths = self.finish_paths(chains, cut, through, options);
            *finished = Some((keys.thick, keys.optimise, keys.overlap, paths));
        }
        finished.as_ref().map(|(_, _, _, f)| f)
    }

    /// The paths of `chains` with the branches cut at the overlap, and the parts kept filled.
    fn finish_paths(
        &self,
        chains: &[Chain],
        cut: Option<&Cut>,
        through: &[Option<Fitted>],
        options: &CentreOptions,
    ) -> Finished {
        let paths = finish_chains(chains, through, options.overlap, options.optimise);
        let Some(cut) = cut else {
            return Finished {
                paths: paths.into_iter().map(|(_, p)| p).collect(),
                filled: Vec::new(),
                inner: Some(Vec::new()),
            };
        };
        let whole = || Finished {
            filled: vec![rings::whole(self.segments)],
            ..Finished::default()
        };
        if paths.is_empty() {
            return whole();
        }
        // The thick parts hold the thick samples and those of the pieces that gave no path.
        let mut thick: Vec<Point> = cut.discs.iter().map(|s| s.point).collect();
        let mut strokes: Vec<&[AxisSample]> = Vec::new();
        for (i, piece) in cut.pieces.iter().enumerate() {
            if paths.iter().any(|(k, _)| *k == i) {
                strokes.push(&piece.samples);
            } else {
                thick.extend(piece.samples.iter().map(|s| s.point));
            }
        }
        let Some(parts) = kept::parts(self.rings, self.segments, &strokes, &thick) else {
            return whole();
        };
        Finished {
            paths: paths.into_iter().map(|(_, p)| p).collect(),
            filled: parts,
            inner: Some(cut.thick_holes.clone()),
        }
    }
}

/// The through stage's memo: the main and through chains fitted for one Thick parts and Optimise.
type Through = (ThickKey, u64, Vec<Option<Fitted>>);

/// The through stage: the main and through chains fitted for `optimise` (`cached` at the
/// default), rerun when Thick parts or Optimise changed; a rerun drops the branches stage.
fn run_through(
    chains: &[Chain],
    cached: Option<&[Option<Fitted>]>,
    optimise: f64,
    keys: Keys,
    through: &mut Option<Through>,
    finished: &mut Option<(ThickKey, u64, u64, Finished)>,
    runs: &mut StageRuns,
) {
    if through
        .as_ref()
        .is_some_and(|(k, o, _)| *k == keys.thick && *o == keys.optimise)
    {
        return;
    }
    runs.through += 1;
    *through = Some((
        keys.thick,
        keys.optimise,
        fit_through(chains, cached, optimise),
    ));
    *finished = None;
}

/// The inner areas of `skeleton`'s region moved by `offset`, less the grown cut-outs; holes
/// marked in `kept_holes` give none.
fn inner_areas(
    skeleton: &RegionSkeleton,
    offset: Length,
    cutouts: &[Outline<'_>],
    kept_holes: &[bool],
) -> Vec<Vec<DocPoint>> {
    inner::inner_areas(&skeleton.region, offset, cutouts, kept_holes)
}
