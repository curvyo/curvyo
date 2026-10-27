//! The one trace (ADR 0023 §2, §3): mask, specks, regions and holes, one outline per boundary,
//! mapped to document millimetres, with the counts the status line shows.

use std::ops::ControlFlow;

use curvyo_document_core::Affine;
use kurbo::{BezPath, Shape};
use visioncortex::BinaryImage;

use crate::outline::{Crop, node_count, trace_crop};
use crate::params::{TraceParams, curve_settings};
use crate::progress::{Phase, Poller, Progress};
use crate::raster::{Raster, foreground_mask};
use crate::regions::{Labels, label};

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
    /// Pixels below the Threshold, before specks were removed.
    pub foreground_pixels: u64,
    /// Every pixel of the picture is foreground after specks: one outline covers it all.
    pub covers_whole_image: bool,
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

/// Traces `raster` with `params`: the pixels below the Threshold become closed outlines, mapped
/// to document millimetres by `pixel_map` (pixel corners, ADR 0017 §6). `poll` is called every
/// 64 rows of the passes over the mask and before each region; `Break` returns
/// [`TraceError::Cancelled`].
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
    if raster.checked_pixels().is_none() {
        return Err(TraceError::Invalid("the pixel bytes do not match the size"));
    }
    if !pixel_map.is_finite() || pixel_map.inverse().is_none() {
        return Err(TraceError::Invalid("the pixel map is not invertible"));
    }
    let mut poller = Poller::new(poll);
    let (width, height) = (raster.width as usize, raster.height as usize);
    let mut mask = foreground_mask(raster, params.threshold);
    let foreground_pixels = mask.iter().filter(|&&f| f).count() as u64;
    let specks = params.specks_mm();
    let min_area_mm2 = specks * specks;
    let pixel_area_mm2 = pixel_map.determinant().abs();
    let is_speck = |pixels: u64| {
        #[allow(clippy::cast_precision_loss)] // at most 16 Mpx, exact in f64
        let area = pixels as f64 * pixel_area_mm2;
        area < min_area_mm2
    };
    let mut specks_removed = 0;
    if min_area_mm2 > 0.0 {
        // Foreground specks become background, then the holes that are specks are filled.
        let labels = label(&mask, width, height, &mut poller)?;
        specks_removed += labels.flip(&mut mask, |r| r.foreground && is_speck(r.pixels), false);
        drop(labels);
        let labels = label(&mask, width, height, &mut poller)?;
        specks_removed += labels.flip(
            &mut mask,
            |r| !r.foreground && !r.touches_border && is_speck(r.pixels),
            true,
        );
    }
    let labels = label(&mask, width, height, &mut poller)?;
    drop(mask);
    let covers_whole_image = labels.regions.len() == 1 && labels.regions[0].foreground;
    let mut traced = trace_regions(&labels, params, &mut poller)?;
    // Smoothing can shrink a region just above the Specks size below it (criterion 19).
    if min_area_mm2 > 0.0 {
        specks_removed += drop_small_outlines(&mut traced, pixel_map, min_area_mm2);
    }
    let outlines: Vec<BezPath> = traced
        .into_iter()
        .flat_map(|(outer, holes)| std::iter::once(outer).chain(holes))
        .map(|mut path| {
            path.apply_affine(to_kurbo(pixel_map));
            path
        })
        .collect();
    let counts = Counts {
        paths: outlines.len(),
        nodes: outlines.iter().map(node_count).sum(),
        specks_removed: usize::try_from(specks_removed).unwrap_or(usize::MAX),
    };
    Ok(TraceResult {
        outlines,
        counts,
        foreground_pixels,
        covers_whole_image,
    })
}

/// One region's outer outline and its holes, in pixel coordinates.
type Traced = (BezPath, Vec<BezPath>);

/// The outlines of every foreground region in label order.
fn trace_regions(
    labels: &Labels,
    params: &TraceParams,
    poller: &mut Poller<'_>,
) -> Result<Vec<Traced>, TraceError> {
    let owners = labels.hole_owners();
    let mut holes: Vec<Vec<u32>> = vec![Vec::new(); labels.regions.len()];
    for (hole, owner) in owners.iter().enumerate() {
        if let Some(owner) = owner {
            // invariant: region indices fit `u32` (the labelling numbers them so).
            holes[*owner as usize].push(u32::try_from(hole).unwrap_or(u32::MAX));
        }
    }
    if holes.iter().any(|h| h.len() >= MAX_HOLES_PER_REGION) {
        return Err(TraceError::TooDetailed);
    }
    let settings = curve_settings(params.smooth_clamped());
    let total = labels.regions.iter().filter(|r| r.foreground).count();
    let mut traced = Vec::with_capacity(total);
    for (index, region) in labels.regions.iter().enumerate() {
        if !region.foreground {
            continue;
        }
        poller.check(Phase::Tracing, traced.len(), total)?;
        let id = u32::try_from(index).unwrap_or(u32::MAX);
        let own = &holes[index];
        let outer_crop = outer_crop(labels, id, own, &owners);
        let Some(outer) = trace_crop(&outer_crop, true, settings) else {
            continue;
        };
        drop(outer_crop);
        let inner = own
            .iter()
            .filter_map(|&hole| trace_crop(&hole_crop(labels, hole), false, settings))
            .collect();
        traced.push((outer, inner));
    }
    poller.check(Phase::Tracing, total, total)?;
    Ok(traced)
}

/// The region `id` with its holes filled, cropped to their common box.
fn outer_crop(labels: &Labels, id: u32, holes: &[u32], owners: &[Option<u32>]) -> Crop {
    let region = &labels.regions[id as usize];
    let (mut left, mut top, mut right, mut bottom) =
        (region.left, region.top, region.right, region.bottom);
    for &hole in holes {
        let h = &labels.regions[hole as usize];
        left = left.min(h.left);
        top = top.min(h.top);
        right = right.max(h.right);
        bottom = bottom.max(h.bottom);
    }
    crop(labels, (left, top, right, bottom), |label| {
        label == id || owners[label as usize] == Some(id)
    })
}

/// The hole `id` cropped to its box.
fn hole_crop(labels: &Labels, id: u32) -> Crop {
    let h = &labels.regions[id as usize];
    crop(labels, (h.left, h.top, h.right, h.bottom), |label| {
        label == id
    })
}

/// The pixels in the box `(left, top, right, bottom)` whose label `inside` accepts.
fn crop(
    labels: &Labels,
    (left, top, right, bottom): (usize, usize, usize, usize),
    inside: impl Fn(u32) -> bool,
) -> Crop {
    let mut image = BinaryImage::new_w_h(right - left, bottom - top);
    for y in top..bottom {
        let row = &labels.ids[y * labels.width + left..y * labels.width + right];
        for (x, &label) in row.iter().enumerate() {
            if inside(label) {
                image.set_pixel(x, y - top, true);
            }
        }
    }
    Crop { left, top, image }
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
