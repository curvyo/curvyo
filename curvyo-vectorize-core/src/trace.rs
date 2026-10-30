//! The one trace (ADR 0023 §2, §3, amended by `specs/0089` adrs.md §5): Denoise, Upscale, mask,
//! background and specks, regions and holes, one crack chain per boundary, fitted, guarded,
//! mapped to document millimetres, with the counts the status line shows.

use std::ops::ControlFlow;

use curvyo_document_core::Affine;
use kurbo::{BezPath, Point, Shape};

use crate::boundary::chains;
use crate::clearance::segment_clearances;
use crate::filter::{median, upscale_2x};
use crate::guarded::{BAND_SHARE, Band, guarded};
use crate::outline::{APART_EDGES, Bound, CLEARANCE_REACH, Chain, Contour, node_count, to_path};
use crate::params::{Geometry, TraceParams, geometry};
use crate::progress::{Phase, Poller, Progress};
use crate::raster::Raster;
use crate::regions::{Labels, Region, label};
use crate::select::{Fields, fields, is_content};

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
    /// Content pixels of the picture (after Denoise, before Upscale and specks).
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

/// Traces `raster` with `params`: the pixels the selection method makes content (membership of
/// the method times the alpha at least ½, after Denoise and Upscale; Invert flips the method's
/// part) become closed outlines, mapped to document millimetres by `pixel_map`
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

/// The method's score and the alpha (`select.rs`), Denoise and Upscale on both, their cut,
/// border shapes and specks: the labelled mask the outlines are walked on.
fn mask(
    raster: Raster<'_>,
    pixel_map: Affine,
    params: &TraceParams,
    poller: &mut Poller<'_>,
) -> Result<Masked, TraceError> {
    let (mut width, mut height) = (raster.width as usize, raster.height as usize);
    let Fields {
        mut score,
        mut alpha,
    } = fields(raster, params);
    let radius = params.denoise.radius();
    score = median(&score, width, height, radius, poller)?;
    if let Some(a) = &alpha {
        alpha = Some(median(a, width, height, radius, poller)?);
    }
    let content = |score: &[u8], alpha: &Option<Vec<u8>>| -> Vec<bool> {
        match alpha {
            Some(a) => score
                .iter()
                .zip(a)
                .map(|(&s, &a)| is_content(params, s, a))
                .collect(),
            None => score.iter().map(|&s| is_content(params, s, 255)).collect(),
        }
    };
    let foreground_pixels = content(&score, &alpha).iter().filter(|&&c| c).count() as u64;
    let mut map = pixel_map;
    let mut unit = 1;
    if params.upscale.applies(raster.width, raster.height) {
        score = upscale_2x(&score, width, height, poller)?;
        if let Some(a) = &alpha {
            alpha = Some(upscale_2x(a, width, height, poller)?);
        }
        (width, height) = (width * 2, height * 2);
        map = Affine::scale(0.5, 0.5).compose(pixel_map);
        unit = 2;
    }
    let mut mask = content(&score, &alpha);
    drop((score, alpha));
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
