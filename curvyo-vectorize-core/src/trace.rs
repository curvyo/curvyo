//! The one trace (ADR 0023 §2, §3, amended by `specs/0089` adrs.md §5): Denoise, Upscale, mask,
//! background and specks, regions and holes, one crack chain per boundary, fitted, guarded,
//! mapped to document millimetres, with the counts the status line shows.

use std::ops::ControlFlow;

use curvyo_document_core::Affine;
use kurbo::{BezPath, Point, Shape};

use crate::boundary::chains;
use crate::clearance::segment_clearances;
use crate::filter::{median, upscale_2x};
use crate::fit::Seg;
use crate::guard::conflicts;
use crate::outline::{
    APART_EDGES, Bound, CLEARANCE_REACH, Chain, Contour, SPAN_LEVELS, node_count, to_path,
};
use crate::params::{Geometry, TraceParams, geometry};
use crate::progress::{Phase, Poller, Progress};
use crate::raster::Raster;
use crate::regions::{Labels, Region, label};

/// A single region with this many holes or more is not traced (criterion 16, `adrs.md`
/// decision 3).
pub const MAX_HOLES_PER_REGION: usize = 65_000;

/// The counts of a trace (criterion 11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    /// Outlines, outer and holes.
    pub paths: usize,
    /// Anchor points of all outlines.
    pub nodes: usize,
    /// Foreground regions and holes removed as specks.
    pub specks_removed: usize,
}

/// What a trace made (criterion "Trace result" in Terms).
#[derive(Debug, Clone, PartialEq)]
pub struct TraceResult {
    /// Closed outlines in document millimetres, in first-scan order of their regions, each
    /// region's outer outline before its holes; holes wound against their outer outline.
    pub outlines: Vec<BezPath>,
    /// The counts.
    pub counts: Counts,
    /// Foreground pixels of the picture (after Denoise, before Upscale and specks).
    pub foreground_pixels: u64,
    /// Every pixel of the picture is foreground after specks: one outline covers it all.
    pub covers_whole_image: bool,
    /// What the fit had to do (`specs/0089` adrs.md §2).
    pub diagnostics: Diagnostics,
}

/// What the fit of a trace had to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Diagnostics {
    /// Outlines kept at a coarser contour because their fit crossed itself or came too close to
    /// another outline (criterion 10).
    pub fallbacks: usize,
}

/// Why a trace made nothing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TraceError {
    /// `poll` asked to stop.
    #[error("the trace was stopped")]
    Cancelled,
    /// The input cannot be traced.
    #[error("the picture cannot be traced: {0}")]
    Invalid(&'static str),
    /// A region has [`MAX_HOLES_PER_REGION`] holes or more.
    #[error("too much detail")]
    TooDetailed,
}

/// Traces `raster` with `params`: the pixels below the Threshold (after Denoise and Upscale; the
/// others with Invert) become closed outlines, mapped to document millimetres by `pixel_map`
/// (pixel corners, ADR 0017 §6). `poll` is called every 64 rows of the passes over the picture
/// and before each region; `Break` returns [`TraceError::Cancelled`].
///
/// # Errors
/// [`TraceError::Cancelled`] when `poll` breaks; [`TraceError::Invalid`] when the bytes do not
/// match the size or `pixel_map` is not finite or singular; [`TraceError::TooDetailed`] for a
/// region with [`MAX_HOLES_PER_REGION`] holes or more.
pub fn trace(
    raster: Raster<'_>,
    pixel_map: Affine,
    params: &TraceParams,
    poll: &mut dyn FnMut(Progress) -> ControlFlow<()>,
) -> Result<TraceResult, TraceError> {
    let geometry = geometry(
        params.smooth_clamped(),
        params.tolerance_px(),
        params.corners,
    );
    trace_with(raster, pixel_map, params, &geometry, poll).map(|(result, _)| result)
}

/// The cleaned mask of a trace and how it maps to millimetres.
struct Masked {
    labels: Labels,
    /// Traced pixels to millimetres.
    map: Affine,
    /// Traced pixels per source pixel (2 after Upscale).
    unit: usize,
    min_area_mm2: f64,
    specks_removed: u64,
    foreground_pixels: u64,
}

/// [`trace`] with an explicit `geometry`; also returns the chains of the outlines in order
/// (for the tests of the contour before simplification).
pub(crate) fn trace_with(
    raster: Raster<'_>,
    pixel_map: Affine,
    params: &TraceParams,
    geometry: &Geometry,
    poll: &mut dyn FnMut(Progress) -> ControlFlow<()>,
) -> Result<(TraceResult, Vec<Chain>), TraceError> {
    if raster.checked_pixels().is_none() {
        return Err(TraceError::Invalid("the pixel bytes do not match the size"));
    }
    if !pixel_map.is_finite() || pixel_map.inverse().is_none() {
        return Err(TraceError::Invalid("the pixel map is not invertible"));
    }
    let mut poller = Poller::new(poll);
    let masked = mask(raster, pixel_map, params, &mut poller)?;
    let labels = &masked.labels;
    let covers_whole_image = labels.regions.len() == 1 && labels.regions[0].foreground;
    let mut specks_removed = masked.specks_removed;
    let regions = chains(labels, geometry.corner_cos, masked.unit, &mut poller)?;
    let flat: Vec<&Chain> = regions
        .iter()
        .flat_map(|(outer, holes)| std::iter::once(outer).chain(holes))
        .collect();
    poller.check(Phase::Tracing, 0, flat.len().max(1))?;
    let (clearances, rooms) = room(&flat, geometry, masked.unit, &mut poller)?;
    // A fit may shrink a shape the Specks kept below the Specks area: it then keeps the shape as
    // its pixel border, which the Specks measured (criteria 10 and 19).
    let band = Band {
        min_area_px: masked.min_area_mm2 / masked.map.determinant().abs(),
    };
    let (fitted, fallbacks) = guarded(&flat, geometry, &clearances, rooms, &band, &mut poller)?;
    // Back into regions: each region's outer outline before its holes.
    let mut paths = fitted.iter().map(|segs| to_path(segs));
    let mut traced: Vec<Traced> = regions
        .iter()
        .map(|(_, holes)| {
            let outer = paths.next().unwrap_or_default();
            (
                outer,
                holes
                    .iter()
                    .map(|_| paths.next().unwrap_or_default())
                    .collect(),
            )
        })
        .collect();
    // Smoothing can shrink a region just above the Specks size below it (criterion 19; the
    // band kept the fits down to `BAND_SHARE` of it).
    if masked.min_area_mm2 > 0.0 {
        specks_removed +=
            drop_small_outlines(&mut traced, masked.map, BAND_SHARE * masked.min_area_mm2);
    }
    let outlines: Vec<BezPath> = traced
        .into_iter()
        .flat_map(|(outer, holes)| std::iter::once(outer).chain(holes))
        .map(|mut path| {
            path.apply_affine(to_kurbo(masked.map));
            path
        })
        .collect();
    let counts = Counts {
        paths: outlines.len(),
        nodes: outlines.iter().map(node_count).sum(),
        specks_removed: usize::try_from(specks_removed).unwrap_or(usize::MAX),
    };
    let result = TraceResult {
        outlines,
        counts,
        foreground_pixels: masked.foreground_pixels,
        covers_whole_image,
        diagnostics: Diagnostics { fallbacks },
    };
    Ok((result, flat.into_iter().cloned().collect()))
}

/// Per chain: the clearance per segment of its walked contour, and the room per edge among
/// the freely smoothed contours.
type Rooms = (Vec<Vec<f64>>, Vec<Vec<f64>>);

/// The room of every chain of `flat`: per segment of its walked contour (`clearance.rs`), and
/// per edge among the freely smoothed contours.
fn room(
    flat: &[&Chain],
    geometry: &Geometry,
    unit: usize,
    poller: &mut Poller<'_>,
) -> Result<Rooms, TraceError> {
    #[allow(clippy::cast_precision_loss)] // 1 or 2
    let reach = CLEARANCE_REACH * unit as f64;
    let apart = APART_EDGES * unit;
    let walked: Vec<Vec<Point>> = flat.iter().map(|c| c.midpoints()).collect();
    let clearances = segment_clearances(
        &walked.iter().map(Vec::as_slice).collect::<Vec<_>>(),
        reach,
        apart,
    );
    drop(walked);
    poller.check(Phase::Tracing, 0, flat.len().max(1))?;
    let free: Vec<Contour> = flat
        .iter()
        .zip(&clearances)
        .map(|(chain, clearance)| {
            chain.contour(
                geometry,
                clearance,
                Bound::Free,
                &vec![false; chain.span_count()],
            )
        })
        .collect();
    let polylines: Vec<Vec<Point>> = free.iter().map(Contour::polyline).collect();
    let smooth_clearances = segment_clearances(
        &polylines.iter().map(Vec::as_slice).collect::<Vec<_>>(),
        reach,
        apart,
    );
    drop(polylines);
    let rooms = free
        .iter()
        .zip(&smooth_clearances)
        .zip(flat)
        .map(|((contour, clearance), chain)| {
            contour.room_by_edge(clearance, chain.edge_count(), reach)
        })
        .collect();
    Ok((clearances, rooms))
}

/// Denoise, Upscale, the Threshold, background and specks: the labelled mask the outlines are
/// walked on.
fn mask(
    raster: Raster<'_>,
    pixel_map: Affine,
    params: &TraceParams,
    poller: &mut Poller<'_>,
) -> Result<Masked, TraceError> {
    let (mut width, mut height) = (raster.width as usize, raster.height as usize);
    let mut luma: Vec<u8> = raster.lumas().collect();
    luma = median(&luma, width, height, params.denoise.radius(), poller)?;
    let is_foreground = |value: u8| (value < params.threshold) != params.invert;
    let foreground_pixels = luma.iter().filter(|&&v| is_foreground(v)).count() as u64;
    let mut map = pixel_map;
    let mut unit = 1;
    if params.upscale.applies(raster.width, raster.height) {
        luma = upscale_2x(&luma, width, height, poller)?;
        (width, height) = (width * 2, height * 2);
        map = Affine::scale(0.5, 0.5).compose(pixel_map);
        unit = 2;
    }
    let mut mask: Vec<bool> = luma.iter().map(|&v| is_foreground(v)).collect();
    drop(luma);
    let specks = params.specks_mm();
    let min_area_mm2 = specks * specks;
    let pixel_area_mm2 = map.determinant().abs();
    let is_speck = |pixels: u64| {
        #[allow(clippy::cast_precision_loss)] // at most 64 Mpx, exact in f64
        let area = pixels as f64 * pixel_area_mm2;
        area < min_area_mm2
    };
    let mut specks_removed = 0;
    if min_area_mm2 > 0.0 || params.ignore_background {
        // The background regions are dropped, foreground specks become background, then the
        // holes that are specks are filled.
        let labels = label(&mask, width, height, poller)?;
        let background = |r: &Region| params.ignore_background && r.touches_border;
        labels.flip(&mut mask, |r| r.foreground && background(r), false);
        specks_removed += labels.flip(
            &mut mask,
            |r| r.foreground && !background(r) && is_speck(r.pixels),
            false,
        );
        drop(labels);
        if min_area_mm2 > 0.0 {
            let labels = label(&mask, width, height, poller)?;
            specks_removed += labels.flip(
                &mut mask,
                |r| !r.foreground && !r.touches_border && is_speck(r.pixels),
                true,
            );
        }
    }
    let labels = label(&mask, width, height, poller)?;
    Ok(Masked {
        labels,
        map,
        unit,
        min_area_mm2,
        specks_removed,
        foreground_pixels,
    })
}

/// One region's outer outline and its holes.
type Traced = (BezPath, Vec<BezPath>);

/// A shape whose pixels cover less than this many Specks areas is fitted with at most
/// [`TINY_TOLERANCE_PX`] and, in the band, is its pixel border.
const BAND_TINY: f64 = 1.5;

/// The tolerance of a shape of a few pixels, source px.
const TINY_TOLERANCE_PX: f64 = 0.25;

/// An outline keeps its fit down to this share of the Specks area (the harness and 0025
/// criterion 63 allow 0.95 for pixel rounding); below it the Specks band applies.
const BAND_SHARE: f64 = 0.95;

/// The Specks band: a fitted outline whose area falls below the Specks area `min_area_px`,
/// which its pixels reached (the Specks step kept them), keeps its pixel border (criteria 10
/// and 19).
struct Band {
    min_area_px: f64,
}

impl Band {
    /// Whether the outline `segs` falls into the band.
    fn holds(&self, segs: &[Seg]) -> bool {
        self.min_area_px > 0.0 && to_path(segs).area().abs() < BAND_SHARE * self.min_area_px
    }
}

/// One outline in the guard: its parts per span, each span's fall-back level, which corners
/// have gone back to their vertices, whether it has become its pixel border; the room of its
/// edges among the smoothed contours, its freely smoothed contour and, once a span needs one,
/// its contour smoothed within the walked room.
struct Guarded {
    spans: Vec<Vec<Seg>>,
    levels: Vec<u8>,
    at_vertex: Vec<bool>,
    border: bool,
    /// The geometry it is fitted with: the trace's, or with the tolerance of a shape of a few
    /// pixels ([`TINY_TOLERANCE_PX`]), so whether its fit keeps the Specks area does not depend
    /// on the Tolerance.
    geometry: Geometry,
    room: Vec<f64>,
    free: Option<Contour>,
    clamped: Option<Contour>,
}

impl Guarded {
    /// All parts in order, and for each the span it belongs to.
    fn parts(&self) -> (Vec<Seg>, Vec<usize>) {
        let mut segs = Vec::new();
        let mut owner = Vec::new();
        for (i, span) in self.spans.iter().enumerate() {
            segs.extend(span.iter().copied());
            owner.extend(std::iter::repeat_n(i, span.len()));
        }
        (segs, owner)
    }

    /// The outline fitted at its levels (the contours made when first needed).
    fn refit(&mut self, chain: &Chain, clearance: &[f64]) {
        let geometry = &self.geometry;
        if self.free.is_none() {
            self.free = Some(chain.contour(
                geometry,
                clearance,
                Bound::Smoothed(&self.room),
                &self.at_vertex,
            ));
        }
        if self.clamped.is_none() && self.levels.iter().any(|&l| l > 0) {
            self.clamped = Some(chain.contour(geometry, clearance, Bound::Walked, &self.at_vertex));
        }
        // invariant: `free` was just made.
        let free = self.free.as_ref().unwrap_or_else(|| unreachable!());
        let clamped = self.clamped.as_ref().unwrap_or(free);
        self.spans = chain.fitted(geometry, &self.levels, free, clamped);
    }

    /// Puts the corners of the spans `spans` at their vertices; false when they were already.
    fn corners_to_vertices(&mut self, spans: &[usize]) -> bool {
        let count = self.at_vertex.len();
        let mut changed = false;
        for &i in spans {
            for k in [i, (i + 1) % count.max(1)] {
                if let Some(flag) = self.at_vertex.get_mut(k)
                    && !*flag
                {
                    *flag = true;
                    changed = true;
                }
            }
        }
        if changed {
            self.free = None;
            self.clamped = None;
        }
        changed
    }
}

/// The fitted outlines of `chains` (each within the room its `clearances` and `rooms` leave)
/// and how many fell back: an outline in the Specks `band` becomes its pixel border; every
/// span with a segment that crosses its own outline or comes too close to another goes one
/// level coarser (`outline::SPAN_LEVELS`: the fit within the walked room, the walked contour
/// simplified within half its room, the contour as walked); where the walked contour is not
/// enough, the span's corners go back to their vertices, and then the outline becomes its
/// pixel border (criterion 10).
fn guarded(
    chains: &[&Chain],
    geometry: &Geometry,
    clearances: &[Vec<f64>],
    rooms: Vec<Vec<f64>>,
    band: &Band,
    poller: &mut Poller<'_>,
) -> Result<(Vec<Vec<Seg>>, usize), TraceError> {
    let total = chains.len();
    let mut state: Vec<Guarded> = Vec::with_capacity(total);
    for (k, (chain, edge_room)) in chains.iter().zip(rooms).enumerate() {
        if k % 64 == 0 {
            poller.check(Phase::Tracing, k, total)?;
        }
        let tiny = band.min_area_px > 0.0
            && to_path(&chain.pixel_border()).area().abs() < BAND_TINY * band.min_area_px;
        let mut outline = Guarded {
            spans: Vec::new(),
            levels: vec![0; chain.span_count()],
            at_vertex: vec![false; chain.span_count()],
            border: false,
            geometry: if tiny {
                Geometry {
                    tolerance_px: geometry.tolerance_px.min(TINY_TOLERANCE_PX),
                    ..*geometry
                }
            } else {
                *geometry
            },
            room: edge_room,
            free: None,
            clamped: None,
        };
        outline.refit(chain, &clearances[k]);
        state.push(outline);
    }
    let mut fell_back = vec![false; total];
    loop {
        for (k, outline) in state.iter_mut().enumerate() {
            if !outline.border && band.holds(&outline.spans.concat()) {
                // The walked contour simplified within half its room keeps nearly the pixel
                // area; a shape too small even for that is its pixel border.
                if outline.levels.iter().any(|&l| l < 2) {
                    for level in &mut outline.levels {
                        *level = (*level).max(2);
                    }
                    outline.refit(chains[k], &clearances[k]);
                }
                if band.holds(&outline.spans.concat()) {
                    // A shape of a few pixels is its pixel border; a larger one (a sliver or a
                    // ragged shape whose fit shrank) its pixel border simplified within its
                    // room, when that keeps the area.
                    let border = chains[k].pixel_border();
                    let simplified = chains[k].border_simplified(&clearances[k]);
                    let tiny = to_path(&border).area().abs() < BAND_TINY * band.min_area_px;
                    outline.spans = if tiny || band.holds(&simplified) {
                        vec![border]
                    } else {
                        vec![simplified]
                    };
                    outline.border = true;
                }
            }
        }
        let owners: Vec<(Vec<Seg>, Vec<usize>)> = state.iter().map(Guarded::parts).collect();
        let outlines: Vec<Vec<Seg>> = owners.iter().map(|(parts, _)| parts.clone()).collect();
        let conflicting = conflicts(&outlines);
        let mut changed = false;
        for (k, flagged) in conflicting.iter().enumerate() {
            let outline = &mut state[k];
            if flagged.is_empty() || outline.border {
                continue;
            }
            changed = true;
            fell_back[k] = true;
            let mut spans: Vec<usize> = flagged.iter().map(|&i| owners[k].1[i]).collect();
            spans.dedup();
            let coarser: Vec<usize> = spans
                .iter()
                .copied()
                .filter(|&i| outline.levels[i] + 1 < SPAN_LEVELS)
                .collect();
            if coarser.is_empty() {
                // The walked contour from corner to corner still conflicts: the corners
                // themselves (placed beyond the pixels) go back to their vertices, then the
                // outline becomes its pixel border.
                if outline.corners_to_vertices(&spans) {
                    outline.levels.fill(0);
                    outline.refit(chains[k], &clearances[k]);
                } else {
                    outline.spans = vec![chains[k].pixel_border()];
                    outline.border = true;
                }
                continue;
            }
            for &i in &coarser {
                outline.levels[i] += 1;
            }
            outline.refit(chains[k], &clearances[k]);
        }
        if !changed {
            break;
        }
        poller.check(Phase::Tracing, total, total)?;
    }
    let fallbacks = fell_back.iter().filter(|&&f| f).count();
    let fitted = state.into_iter().map(|g| g.spans.concat()).collect();
    Ok((fitted, fallbacks))
}

/// Drops the outer outlines (with their holes) and the holes whose area in mm² is below
/// `min_area_mm2`; returns how many outlines were dropped.
fn drop_small_outlines(traced: &mut Vec<Traced>, pixel_map: Affine, min_area_mm2: f64) -> u64 {
    let scale = pixel_map.determinant().abs();
    let small = |path: &BezPath| path.area().abs() * scale < min_area_mm2;
    let mut dropped = 0;
    traced.retain_mut(|(outer, holes)| {
        if small(outer) {
            dropped += 1;
            return false;
        }
        let before = holes.len();
        holes.retain(|hole| !small(hole));
        dropped += (before - holes.len()) as u64;
        true
    });
    dropped
}

/// The document map as a `kurbo` map (both in SVG order).
fn to_kurbo(map: Affine) -> kurbo::Affine {
    kurbo::Affine::new(map.to_array())
}
