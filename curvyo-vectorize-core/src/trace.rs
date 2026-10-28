//! The one trace (ADR 0023 §2, §3): Denoise, Upscale, mask, background and specks, regions and
//! holes, one outline per boundary, mapped to document millimetres, with the counts the status
//! line shows.

use std::ops::ControlFlow;

use curvyo_document_core::Affine;
use kurbo::{BezPath, Shape};
use visioncortex::BinaryImage;

use crate::filter::{median, upscale_2x};
use crate::outline::{Crop, node_count, trace_crop};
use crate::params::{CurveSettings, TraceParams, corner_cos, curve_settings, reduce_tolerance_mm};
use crate::progress::{Phase, Poller, Progress};
use crate::raster::Raster;
use crate::reduce::reduce;
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
    trace_reducing(raster, pixel_map, params, true, poll)
}

/// [`trace`], with the node reduction only when `reduce_nodes` (off only in the test that
/// measures the reduction against the trace before it).
fn trace_reducing(
    raster: Raster<'_>,
    pixel_map: Affine,
    params: &TraceParams,
    reduce_nodes: bool,
    poll: &mut dyn FnMut(Progress) -> ControlFlow<()>,
) -> Result<TraceResult, TraceError> {
    if raster.checked_pixels().is_none() {
        return Err(TraceError::Invalid("the pixel bytes do not match the size"));
    }
    if !pixel_map.is_finite() || pixel_map.inverse().is_none() {
        return Err(TraceError::Invalid("the pixel map is not invertible"));
    }
    let mut poller = Poller::new(poll);
    let (mut width, mut height) = (raster.width as usize, raster.height as usize);
    let mut luma: Vec<u8> = raster.lumas().collect();
    luma = median(&luma, width, height, params.denoise.radius(), &mut poller)?;
    let is_foreground = |value: u8| (value < params.threshold) != params.invert;
    let foreground_pixels = luma.iter().filter(|&&v| is_foreground(v)).count() as u64;
    let mut map = pixel_map;
    if params.upscale.applies(raster.width, raster.height) {
        luma = upscale_2x(&luma, width, height, &mut poller)?;
        (width, height) = (width * 2, height * 2);
        map = Affine::scale(0.5, 0.5).compose(pixel_map);
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
        let labels = label(&mask, width, height, &mut poller)?;
        let background = |r: &Region| params.ignore_background && r.touches_border;
        labels.flip(&mut mask, |r| r.foreground && background(r), false);
        specks_removed += labels.flip(
            &mut mask,
            |r| r.foreground && !background(r) && is_speck(r.pixels),
            false,
        );
        drop(labels);
        if min_area_mm2 > 0.0 {
            let labels = label(&mask, width, height, &mut poller)?;
            specks_removed += labels.flip(
                &mut mask,
                |r| !r.foreground && !r.touches_border && is_speck(r.pixels),
                true,
            );
        }
    }
    let labels = label(&mask, width, height, &mut poller)?;
    drop(mask);
    let covers_whole_image = labels.regions.len() == 1 && labels.regions[0].foreground;
    let settings = curve_settings(params.smooth_clamped(), params.corners);
    let tolerance_mm = reduce_tolerance_mm(params.smooth_clamped());
    let reduction = reduce_nodes.then(|| Reduction {
        tolerance_px: tolerance_mm / map.determinant().abs().sqrt(),
        corner_cos: corner_cos(params.corners),
    });
    let mut outlines = Outlines::new(&labels, map, min_area_mm2, reduction, &mut poller)?;
    let (traced, dropped) = outlines.at(settings)?;
    specks_removed += dropped;
    let outlines: Vec<BezPath> = traced
        .into_iter()
        .flat_map(|(outer, holes)| std::iter::once(outer).chain(holes))
        .map(|mut path| {
            path.apply_affine(to_kurbo(map));
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

/// The node reduction after visioncortex (`adrs.md` 2026-10-28, second note, point 1).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Reduction {
    /// The tolerance in pixels of the traced picture.
    tolerance_px: f64,
    /// The cosine of the corner angle: sharper joints are never merged.
    corner_cos: f64,
}

impl Reduction {
    /// This reduction, or none when its tolerance is 0 (Smooth 80 and below).
    fn active(self) -> Option<Self> {
        (self.tolerance_px > 0.0).then_some(self)
    }
}

/// The outlines of one labelled mask with its hole ownership.
struct Outlines<'a, 'p, 'q> {
    labels: &'a Labels,
    map: Affine,
    min_area_mm2: f64,
    /// The node reduction of the settings; its tolerance may be 0, `None` only when the trace
    /// skips the reduction.
    reduction: Option<Reduction>,
    holes: Vec<Vec<u32>>,
    owners: Vec<Option<u32>>,
    poller: &'q mut Poller<'p>,
}

impl<'a, 'p, 'q> Outlines<'a, 'p, 'q> {
    fn new(
        labels: &'a Labels,
        map: Affine,
        min_area_mm2: f64,
        reduction: Option<Reduction>,
        poller: &'q mut Poller<'p>,
    ) -> Result<Self, TraceError> {
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
        Ok(Self {
            labels,
            map,
            min_area_mm2,
            reduction,
            holes,
            owners,
            poller,
        })
    }

    /// The outlines at `settings` and how many were dropped as below the Specks size after
    /// smoothing.
    fn at(&mut self, settings: CurveSettings) -> Result<(Vec<Traced>, u64), TraceError> {
        let reduction = self.reduction.and_then(Reduction::active);
        let mut traced = trace_regions(
            self.labels,
            &self.holes,
            &self.owners,
            settings,
            reduction,
            self.poller,
        )?;
        // Smoothing can shrink a region just above the Specks size below it (criterion 19).
        let dropped = if self.min_area_mm2 > 0.0 {
            drop_small_outlines(&mut traced, self.map, self.min_area_mm2)
        } else {
            0
        };
        Ok((traced, dropped))
    }
}

/// One region's outer outline and its holes, in pixel coordinates.
type Traced = (BezPath, Vec<BezPath>);

/// The outlines of every foreground region in label order, each reduced by `reduction`; `holes`
/// lists each region's holes, `owners` each hole's region.
fn trace_regions(
    labels: &Labels,
    holes: &[Vec<u32>],
    owners: &[Option<u32>],
    settings: CurveSettings,
    reduction: Option<Reduction>,
    poller: &mut Poller<'_>,
) -> Result<Vec<Traced>, TraceError> {
    let total = labels.regions.iter().filter(|r| r.foreground).count();
    let mut traced = Vec::with_capacity(total);
    for (index, region) in labels.regions.iter().enumerate() {
        if !region.foreground {
            continue;
        }
        poller.check(Phase::Tracing, traced.len(), total)?;
        let id = u32::try_from(index).unwrap_or(u32::MAX);
        let own = &holes[index];
        let outer_crop = outer_crop(labels, id, own, owners);
        let reduced = |path: BezPath| match reduction {
            Some(r) => reduce(&path, r.tolerance_px, r.corner_cos),
            None => path,
        };
        let Some(outer) = trace_crop(&outer_crop, true, settings).map(reduced) else {
            continue;
        };
        drop(outer_crop);
        let inner = own
            .iter()
            .filter_map(|&hole| trace_crop(&hole_crop(labels, hole), false, settings))
            .map(reduced)
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

/// The pixels in the box `(left, top, right, bottom)` whose label `inside` accepts, with every
/// pocket they enclose filled: visioncortex walks one boundary and gets stuck where two pixels of
/// the shape touch only diagonally across an empty pocket owned by another region (a noisy
/// photograph has them), so the crop holds the shape as visioncortex's own clusters do.
fn crop(
    labels: &Labels,
    (left, top, right, bottom): (usize, usize, usize, usize),
    inside: impl Fn(u32) -> bool,
) -> Crop {
    let (width, height) = (right - left, bottom - top);
    let mut shape = Vec::with_capacity(width * height);
    for y in top..bottom {
        let row = &labels.ids[y * labels.width + left..y * labels.width + right];
        shape.extend(row.iter().map(|&label| inside(label)));
    }
    fill_pockets(&mut shape, width, height);
    let mut image = BinaryImage::new_w_h(width, height);
    for (i, _) in shape.iter().enumerate().filter(|(_, set)| **set) {
        image.set_pixel(i % width, i / width, true);
    }
    Crop { left, top, image }
}

/// Sets every empty pixel of `shape` (`width` x `height`) that is not 4-connected to the edge.
fn fill_pockets(shape: &mut [bool], width: usize, height: usize) {
    let mut outside = vec![false; shape.len()];
    let mut stack: Vec<usize> = (0..shape.len())
        .filter(|&i| {
            let (x, y) = (i % width, i / width);
            !shape[i] && (x == 0 || y == 0 || x + 1 == width || y + 1 == height)
        })
        .collect();
    for &i in &stack {
        outside[i] = true;
    }
    while let Some(i) = stack.pop() {
        let (x, y) = (i % width, i / width);
        let neighbours = [
            (x > 0).then(|| i - 1),
            (x + 1 < width).then(|| i + 1),
            (y > 0).then(|| i - width),
            (y + 1 < height).then(|| i + width),
        ];
        for j in neighbours.into_iter().flatten() {
            if !shape[j] && !outside[j] {
                outside[j] = true;
                stack.push(j);
            }
        }
    }
    for (pixel, out) in shape.iter_mut().zip(outside) {
        *pixel |= !out;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pockets_are_filled_and_the_outside_stays() {
        let rows = ["#####.", "#...#.", "#.#.#.", "#...#.", "##.###"];
        let mut shape: Vec<bool> = rows
            .iter()
            .flat_map(|r| r.chars().map(|c| c == '#'))
            .collect();
        fill_pockets(&mut shape, 6, 5);
        let text: Vec<String> = shape
            .chunks(6)
            .map(|row| row.iter().map(|&s| if s { '#' } else { '.' }).collect())
            .collect();
        // The ring's inside reaches the edge through the gap in the bottom row: not a pocket.
        assert_eq!(text, ["#####.", "#...#.", "#.#.#.", "#...#.", "##.###"]);
        let closed = ["#####", "#...#", "#.#.#", "#...#", "#####"];
        let mut shape: Vec<bool> = closed
            .iter()
            .flat_map(|r| r.chars().map(|c| c == '#'))
            .collect();
        fill_pockets(&mut shape, 5, 5);
        assert!(shape.iter().all(|&s| s));
    }

    /// Criterion 63: every Silhouette outline lies within the Smooth tolerance (0.2 mm) of the
    /// trace before the node reduction, both ways, sampled every 0.1 px, plus 0.001 mm; and the
    /// reduction lowers the node count. Release, with the quality harness:
    /// `cargo test --release -p curvyo-vectorize-core --lib -- --ignored`.
    #[test]
    #[ignore = "quality harness: run in release with --ignored"]
    fn silhouette_stays_within_the_smooth_tolerance_on_the_set() {
        use crate::params::PRESETS;
        use crate::test_common::quality::{directed_distance, quality_set};
        let silhouette = PRESETS.iter().find(|p| p.key == "silhouette").unwrap();
        let tolerance_mm = reduce_tolerance_mm(silhouette.smooth) + 0.001;
        for image in quality_set() {
            let raster = image.picture.raster();
            let params = silhouette.apply(TraceParams::with_threshold(
                crate::raster::auto_threshold(raster),
            ));
            let mm_per_px = 25.4 / image.dpi;
            let map = Affine::scale(mm_per_px, mm_per_px);
            let mut go = |_| ControlFlow::Continue(());
            let after = trace_reducing(raster, map, &params, true, &mut go).unwrap();
            let before = trace_reducing(raster, map, &params, false, &mut go).unwrap();
            let in_px = |paths: &[BezPath]| -> Vec<BezPath> {
                paths
                    .iter()
                    .map(|p| kurbo::Affine::scale(1.0 / mm_per_px) * p.clone())
                    .collect()
            };
            let (a, b) = (in_px(&after.outlines), in_px(&before.outlines));
            let (w, h) = (raster.width as usize, raster.height as usize);
            let worst = directed_distance(&a, &b, w, h).max(directed_distance(&b, &a, w, h));
            assert!(
                worst * mm_per_px <= tolerance_mm,
                "{}: {:.4} mm",
                image.name,
                worst * mm_per_px
            );
            assert!(after.counts.nodes <= before.counts.nodes, "{}", image.name);
            assert_eq!(after.counts.paths, before.counts.paths, "{}", image.name);
        }
    }
}
