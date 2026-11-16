//! Inner areas (ADR 0063 §7): each hole moved inward by the offset, minus the grown cut-outs.

use curvyo_document_core::{Length, Point as DocPoint, Tolerance, Vec2 as DocVec};

use crate::medial_axis::Region;
use crate::{BooleanOp, OffsetJoin, Outline, OutlineTriple, boolean, offset};

/// The tolerance of the offsets and the difference.
const TOLERANCE_MM: f64 = 0.01;

/// The inner areas of `region`'s holes, as closed polygons (holes of the result negative);
/// holes marked in `kept` lie in a part kept filled and give none.
pub(super) fn inner_areas(
    region: &Region,
    distance: Length,
    cutouts: &[Outline<'_>],
    kept: &[bool],
) -> Vec<Vec<DocPoint>> {
    let tolerance = Tolerance::from_mm(TOLERANCE_MM);
    let grown_cutouts: Vec<Vec<OutlineTriple>> = if cutouts.is_empty() {
        Vec::new()
    } else {
        offset(cutouts, distance, OffsetJoin::Round, tolerance)
            .map(|o| o.outlines().iter().map(|ring| triples(ring)).collect())
            .unwrap_or_default()
    };
    let mut areas = Vec::new();
    for (k, ring) in region.rings_mm().into_iter().skip(1).enumerate() {
        if kept.get(k).copied().unwrap_or(false) {
            continue;
        }
        let hole: Vec<OutlineTriple> = ring
            .iter()
            .map(|p| (DocPoint::new(p.x, p.y), DocVec::ZERO, DocVec::ZERO))
            .collect();
        let Ok(shrunk) = offset(
            &[Outline::new(&hole, true)],
            Length::from_mm(-distance.as_mm()),
            OffsetJoin::Round,
            tolerance,
        ) else {
            continue;
        };
        let shrunk: Vec<Vec<OutlineTriple>> =
            shrunk.outlines().iter().map(|ring| triples(ring)).collect();
        let base: Vec<Outline<'_>> = shrunk.iter().map(|r| Outline::new(r, true)).collect();
        let cut: Vec<Outline<'_>> = grown_cutouts
            .iter()
            .map(|r| Outline::new(r, true))
            .collect();
        let result = if cut.is_empty() {
            boolean(BooleanOp::Union, &[&base], tolerance)
        } else {
            boolean(BooleanOp::Difference, &[&base, &cut], tolerance)
        };
        if let Ok(result) = result {
            areas.extend(result.into_outlines());
        }
    }
    areas
}

fn triples(ring: &[crate::OffsetAnchor]) -> Vec<OutlineTriple> {
    ring.iter()
        .map(|a| (a.point, a.handle_in, a.handle_out))
        .collect()
}
