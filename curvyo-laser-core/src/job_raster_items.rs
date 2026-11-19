//! The raster items of one Image block (ADR 0069 §2, §13; `specs/0123` criteria 1 to 3, 14, 37 d): its
//! images and gradient-filled leaves, whose sweeps are worked through the kept caches.

use std::sync::Arc;

use curvyo_document_core::{AssetHash, NodeId, PlacedGradient, Point};
use curvyo_image_core::WorkingImage;

use crate::job_fingerprint::raster_fingerprint;
use crate::job_flatten::flat;
use crate::job_input::{BlockInput, LeafGeometry, LeafInput, LeafOutline, PointUm};
use crate::job_numbers::BlockNumbers;
use crate::job_raster_plan::{PyramidCache, RasterSettings};
use crate::job_raster_rows::Sweep;
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
    },
    Gradient(&'a PlacedGradient),
}

/// One raster item of a block.
pub(crate) struct RasterLeaf<'a> {
    node: NodeId,
    geometry: &'a LeafGeometry,
    stacking: u32,
    rings: Vec<Vec<PointUm>>,
    kind: RasterKind<'a>,
}

/// The dot settings of the raster items of a block with `numbers`.
fn raster_settings(numbers: &BlockNumbers) -> RasterSettings<'_> {
    RasterSettings {
        image: &numbers.options.image,
        interval_um: i64::from(numbers.interval.hundredths()) * 10,
        angle_deg: numbers.options.scan_angle.degrees(),
        bidirectional: numbers.options.bidirectional,
    }
}

/// Advances the sweeps of raster item `leaf` by about `dots` dots; whether they are finished.
fn advance_leaf(
    leaf: &RasterLeaf<'_>,
    numbers: &BlockNumbers,
    caches: &mut RasterCaches,
    dots: &mut u64,
) -> bool {
    let settings = raster_settings(numbers);
    let key = raster_fingerprint(&leaf.rings, leaf.geometry, settings);
    let RasterCaches { pyramids, sweeps } = caches;
    sweeps.advance(
        (leaf.node, key),
        &leaf.rings,
        settings,
        || match leaf.kind {
            RasterKind::Image {
                frame,
                asset,
                pixels,
            } => HeldSource::Image {
                pyramid: pyramids.pyramid(asset, pixels),
                frame: *frame,
                interval_mm: f64::from(numbers.interval.hundredths()) / 100.0,
            },
            RasterKind::Gradient(gradient) => HeldSource::Gradient(gradient.clone()),
        },
        dots,
    )
}

/// The sweeps of the raster items in stacking order, bottom first (criterion 14), each with its
/// item's node and stacking place.
pub(crate) fn raster_sweeps(
    raster: &mut [RasterLeaf<'_>],
    numbers: &BlockNumbers,
    caches: &mut RasterCaches,
) -> Vec<(NodeId, u32, Arc<Sweep>)> {
    raster.sort_by_key(|leaf| leaf.stacking);
    let settings = raster_settings(numbers);
    let mut out = Vec::new();
    for leaf in raster.iter() {
        let mut all = u64::MAX;
        advance_leaf(leaf, numbers, caches, &mut all);
        let key = raster_fingerprint(&leaf.rings, leaf.geometry, settings);
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
        .all(|leaf| advance_leaf(leaf, numbers, caches, dots))
}

/// Puts an image or a gradient-filled leaf of an Image block into `raster`; any other leaf, and
/// an image whose pixels cannot be read (criterion 36), runs nothing (criterion 48).
pub(crate) fn raster_leaf<'a>(leaf: &'a LeafInput, raster: &mut Vec<RasterLeaf<'a>>) {
    let (outlines, raster_kind) = match &leaf.geometry {
        LeafGeometry::Outlines(_) | LeafGeometry::Image { pixels: None, .. } => return,
        LeafGeometry::Image {
            frame,
            asset,
            pixels: Some(pixels),
        } => (
            vec![frame_outline(frame)],
            RasterKind::Image {
                frame,
                asset,
                pixels,
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
