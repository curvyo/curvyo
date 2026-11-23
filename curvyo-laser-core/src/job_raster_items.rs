//! The raster items of one Image block (ADR 0069 §2, §13; `specs/0123` criteria 1 to 3, 14, 37 d): its
//! images and gradient-filled leaves, whose sweeps are worked through the kept caches.

use std::sync::Arc;

use curvyo_document_core::{AssetHash, Length, NodeId, PlacedGradient, Point};
use curvyo_image_core::WorkingImage;

use crate::image_adjust_cache::{ADJUST_WORK, AdjustedPicture, PyramidCache, reduced_level};
use crate::image_adjust_resolve::AdjustRegisters;
use crate::job_fingerprint::raster_fingerprint;
use crate::job_flatten::flat;
use crate::job_input::{BlockInput, LeafGeometry, LeafInput, LeafOutline, PointUm};
use crate::job_numbers::BlockNumbers;
use crate::job_raster_plan::{RasterSettings, leaf_image_settings};
use crate::job_raster_rows::Sweep;
use crate::job_raster_sample::pixel_pitch_mm;
use crate::job_raster_work::{HeldSource, SweepCache};
use crate::operation_kind::OperationKind;

/// The grey pyramids and the raster items' sweeps kept between plans.
#[derive(Debug, Default)]
pub(crate) struct RasterCaches {
    pub(crate) pyramids: PyramidCache,
    pub(crate) sweeps: SweepCache,
}

/// Where a raster leaf's greys come from, before its sampler is made.
enum RasterKind<'a> {
    Image {
        frame: &'a [Point; 4],
        asset: &'a AssetHash,
        pixels: &'a WorkingImage,
        adjust: &'a AdjustRegisters,
    },
    Gradient(&'a PlacedGradient),
}

/// The dots a budgeted step of the job must still have for a picture's adjustment to be worked in
/// it (ADR 0073 §8, `specs/0135` criterion 13): with fewer the adjustment waits for the next step,
/// so the frame of an edit stays light; a step that worked a band of one ends there.
pub const ADJUST_DOTS: u64 = 200_000;

/// One raster item of a block.
pub(crate) struct RasterLeaf<'a> {
    node: NodeId,
    geometry: &'a LeafGeometry,
    stacking: u32,
    rings: Vec<Vec<PointUm>>,
    kind: RasterKind<'a>,
}

/// The dot settings of the raster item `geometry` of a block with `numbers` (ADR 0072 amendment
/// A4): a picture burns with its own dither.
fn raster_settings(numbers: &BlockNumbers, geometry: &LeafGeometry) -> RasterSettings {
    RasterSettings {
        image: leaf_image_settings(&numbers.options.image, geometry),
        interval_um: i64::from(numbers.interval.hundredths()) * 10,
        angle_deg: numbers.options.scan_angle.degrees(),
        bidirectional: numbers.options.bidirectional,
    }
}

/// The key of the sweeps of `leaf` with `numbers`.
fn leaf_key(leaf: &RasterLeaf<'_>, numbers: &BlockNumbers) -> u64 {
    raster_fingerprint(
        &leaf.rings,
        leaf.geometry,
        raster_settings(numbers, leaf.geometry),
        numbers.picture_interval,
    )
}

/// The adjusted picture of an image leaf with `numbers` (ADR 0073 §6 to §8; `specs/0135`
/// criteria 19, 60, 61): its effective steps at the operation's Interval, on the reduced level a
/// coarse preview fits. `Err(true)` when a budgeted step had to work on it (it ends there),
/// `Err(false)` when the step's dots are too few to start.
fn adjusted_picture(
    (frame, asset, pixels, adjust): (&[Point; 4], &AssetHash, &WorkingImage, &AdjustRegisters),
    numbers: &BlockNumbers,
    pyramids: &PyramidCache,
    budget: Option<u64>,
) -> Result<AdjustedPicture, bool> {
    let dot = numbers.interval.hundredths();
    let level = if dot > numbers.picture_interval.hundredths() {
        reduced_level(
            Length::from_mm(pixel_pitch_mm(frame, pixels.width, pixels.height)),
            Length::from_mm(f64::from(dot) / 100.0),
        )
    } else {
        0
    };
    let may_work = budget.is_none_or(|dots| dots >= ADJUST_DOTS);
    if !may_work && !pyramids.resolvable(asset, adjust) {
        return Err(false);
    }
    let interval_um = u32::from(numbers.picture_interval.hundredths()) * 10;
    let resolved = pyramids.resolve((asset, pixels, frame), adjust, interval_um, level);
    if let Some(held) = pyramids.held(&resolved.key) {
        return Ok(held);
    }
    if resolved.key.steps.is_empty() {
        // The `0123` pyramid of the asset itself is built in any step, as before.
        return Ok(pyramids.picture(&resolved.key, pixels));
    }
    if !may_work {
        return Err(false);
    }
    if budget.is_some() {
        // A band of the adjustment, and the step ends (criterion 13: no tick adjusts a whole
        // picture); the next step finds it held once it is finished.
        let _ = pyramids.advance(&resolved.key, pixels, ADJUST_WORK);
        return Err(true);
    }
    Ok(pyramids.picture(&resolved.key, pixels))
}

/// Advances the sweeps of raster item `leaf` by about `dots` dots; whether they are finished.
/// `budgeted` is a step of the host's ticks, in which a picture's adjustment runs alone.
fn advance_leaf(
    leaf: &RasterLeaf<'_>,
    numbers: &BlockNumbers,
    caches: &mut RasterCaches,
    dots: &mut u64,
    budgeted: bool,
) -> bool {
    let settings = raster_settings(numbers, leaf.geometry);
    let key = leaf_key(leaf, numbers);
    let RasterCaches { pyramids, sweeps } = caches;
    if sweeps.get(leaf.node, key).is_some() {
        return true;
    }
    let source = match leaf.kind {
        RasterKind::Image {
            frame,
            asset,
            pixels,
            adjust,
        } => {
            let budget = budgeted.then_some(*dots);
            match adjusted_picture((frame, asset, pixels, adjust), numbers, pyramids, budget) {
                Ok(picture) => HeldSource::Image {
                    pyramid: picture.pyramid,
                    frame: *frame,
                    interval_mm: f64::from(numbers.interval.hundredths()) / 100.0,
                },
                Err(worked) => {
                    if worked {
                        *dots = 0;
                    }
                    return false;
                }
            }
        }
        RasterKind::Gradient(gradient) => HeldSource::Gradient(gradient.clone()),
    };
    sweeps.advance((leaf.node, key), &leaf.rings, settings, || source, dots)
}

/// The sweeps of the raster items in stacking order, bottom first (criterion 14), each with its
/// item's node and stacking place.
pub(crate) fn raster_sweeps(
    raster: &mut [RasterLeaf<'_>],
    numbers: &BlockNumbers,
    caches: &mut RasterCaches,
) -> Vec<(NodeId, u32, Arc<Sweep>)> {
    raster.sort_by_key(|leaf| leaf.stacking);
    let mut out = Vec::new();
    for leaf in raster.iter() {
        let mut all = u64::MAX;
        advance_leaf(leaf, numbers, caches, &mut all, false);
        let key = leaf_key(leaf, numbers);
        let sweeps = caches.sweeps.get(leaf.node, key).unwrap_or_default();
        out.extend(
            sweeps
                .iter()
                .map(|sweep| (leaf.node, leaf.stacking, Arc::clone(sweep))),
        );
    }
    out
}

/// Advances the sweeps of the raster items of `block` with `numbers` by about `dots` dots, in
/// the order its plan reads them; whether all of them are finished.
pub(crate) fn prepare_raster(
    block: &BlockInput,
    numbers: &BlockNumbers,
    caches: &mut RasterCaches,
    dots: &mut u64,
) -> bool {
    let kind = numbers.kind;
    if kind != OperationKind::Image {
        return true;
    }
    let mut raster = Vec::new();
    for leaf in &block.leaves {
        raster_leaf(leaf, &mut raster);
    }
    raster.sort_by_key(|leaf| leaf.stacking);
    raster
        .iter()
        .all(|leaf| advance_leaf(leaf, numbers, caches, dots, true))
}

/// Puts an image or a gradient-filled leaf of an Image block into `raster`; any other leaf, and
/// an image whose pixels cannot be read (criterion 36), runs nothing (criterion 48).
pub(crate) fn raster_leaf<'a>(leaf: &'a LeafInput, raster: &mut Vec<RasterLeaf<'a>>) {
    let (outlines, raster_kind) = match &leaf.geometry {
        LeafGeometry::Outlines(_) | LeafGeometry::Image { pixels: None, .. } => return,
        // ADR 0077 §4: an AI step without its stored result does not burn the picture.
        LeafGeometry::Image { adjust, .. } if adjust.missing_results() > 0 => return,
        LeafGeometry::Image {
            frame,
            asset,
            pixels: Some(pixels),
            adjust,
        } => (
            vec![frame_outline(frame)],
            RasterKind::Image {
                frame,
                asset,
                pixels,
                adjust,
            },
        ),
        LeafGeometry::Gradient { outlines, gradient } => {
            (outlines.clone(), RasterKind::Gradient(gradient))
        }
    };
    let rings: Vec<Vec<PointUm>> = outlines.iter().filter_map(flat).collect();
    if !rings.is_empty() {
        raster.push(RasterLeaf {
            node: leaf.node,
            geometry: &leaf.geometry,
            stacking: leaf.stacking,
            rings,
            kind: raster_kind,
        });
    }
}

/// An image's frame as one closed outline.
pub(crate) fn frame_outline(frame: &[Point; 4]) -> LeafOutline {
    LeafOutline {
        anchors: frame
            .iter()
            .map(|&p| {
                (
                    p,
                    curvyo_document_core::Vec2::ZERO,
                    curvyo_document_core::Vec2::ZERO,
                )
            })
            .collect(),
        closed: true,
    }
}
