//! The plan cache key of one block (ADR 0064 §9): FNV-1a 64 over the bits of everything its plan
//! reads, written out so no dependency and no randomly keyed hasher is involved.

use std::hash::{Hash, Hasher};

use crate::job_input::{BlockInput, BlockSource, LeafGeometry, LeafOutline, PointUm};
use crate::job_numbers::BlockNumbers;
use crate::job_raster_plan::RasterSettings;

/// FNV-1a 64 as a [`Hasher`], so derived `Hash` impls (node ids) feed it.
struct Fnv(u64);

impl Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
}

/// The key of the plan of `block` with `numbers`: kind, interval, the hatch options, the Image
/// settings, the Cut-out margin and every leaf's node, stacking order, coordinates, asset and
/// gradient. Power, speed, passes, overscan
/// and the Z offset do not change a plan and are left out.
#[must_use]
pub fn plan_fingerprint(block: &BlockInput, numbers: &BlockNumbers) -> u64 {
    let mut fnv = Fnv(0xcbf2_9ce4_8422_2325);
    numbers.kind.hash(&mut fnv);
    numbers.interval.hash(&mut fnv);
    numbers.options.crosshatch.hash(&mut fnv);
    numbers.options.bidirectional.hash(&mut fnv);
    numbers.options.scan_angle.hash(&mut fnv);
    numbers.options.image.hash(&mut fnv);
    if let BlockSource::Operation { margin, .. } = &block.source {
        margin.as_mm().to_bits().hash(&mut fnv);
    }
    block.leaves.len().hash(&mut fnv);
    for leaf in &block.leaves {
        leaf.node.hash(&mut fnv);
        leaf.stacking.hash(&mut fnv);
        geometry_hash(&leaf.geometry, &mut fnv);
    }
    block.cut_out.len().hash(&mut fnv);
    for item in &block.cut_out {
        item.item.hash(&mut fnv);
        item.stacking.hash(&mut fnv);
        item.leaves.len().hash(&mut fnv);
        for leaf in &item.leaves {
            outlines_hash(leaf, &mut fnv);
        }
    }
    fnv.finish()
}

/// The key of the sweeps of one raster item: its rings, its geometry (frame, asset or gradient)
/// and the dot settings.
pub(crate) fn raster_fingerprint(
    rings: &[Vec<PointUm>],
    geometry: &LeafGeometry,
    settings: RasterSettings<'_>,
) -> u64 {
    let mut fnv = Fnv(0xcbf2_9ce4_8422_2325);
    rings.hash(&mut fnv);
    geometry_hash(geometry, &mut fnv);
    settings.image.hash(&mut fnv);
    settings.interval_um.hash(&mut fnv);
    settings.angle_deg.to_bits().hash(&mut fnv);
    settings.bidirectional.hash(&mut fnv);
    fnv.finish()
}

fn geometry_hash(geometry: &LeafGeometry, fnv: &mut Fnv) {
    match geometry {
        LeafGeometry::Outlines(outlines) => {
            0_u8.hash(fnv);
            outlines_hash(outlines, fnv);
        }
        LeafGeometry::Image {
            frame,
            asset,
            pixels,
        } => {
            1_u8.hash(fnv);
            for corner in frame {
                corner.x.to_bits().hash(fnv);
                corner.y.to_bits().hash(fnv);
            }
            asset.hash(fnv);
            pixels.as_ref().map(|p| (p.width, p.height)).hash(fnv);
        }
        LeafGeometry::Gradient { outlines, gradient } => {
            2_u8.hash(fnv);
            outlines_hash(outlines, fnv);
            let map = gradient.to_gradient();
            for value in [map.a, map.b, map.c, map.d, map.e, map.f] {
                value.to_bits().hash(fnv);
            }
            gradient.is_radial().hash(fnv);
            for stop in gradient.stops().as_slice() {
                (stop.colour.r, stop.colour.g, stop.colour.b, stop.alpha).hash(fnv);
                stop.offset.get().to_bits().hash(fnv);
            }
        }
    }
}

fn outlines_hash(outlines: &[LeafOutline], fnv: &mut Fnv) {
    outlines.len().hash(fnv);
    for outline in outlines {
        outline.closed.hash(fnv);
        outline.anchors.len().hash(fnv);
        for (point, handle_in, handle_out) in &outline.anchors {
            for value in [
                point.x,
                point.y,
                handle_in.x,
                handle_in.y,
                handle_out.x,
                handle_out.y,
            ] {
                value.to_bits().hash(fnv);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_matches_its_reference_value() {
        // FNV-1a 64 of "a" is 0xaf63dc4c8601ec8c.
        let mut fnv = Fnv(0xcbf2_9ce4_8422_2325);
        fnv.write(b"a");
        assert_eq!(fnv.finish(), 0xaf63_dc4c_8601_ec8c);
    }
}
