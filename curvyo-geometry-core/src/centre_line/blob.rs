//! The disc test (ADR 0063 §6, amended 2026-11-18 K): a region whose widest width nearly spans
//! its diameter has no centre line to describe it and stays filled at every setting.

use curvyo_document_core::Length;

use super::fit::tolerance;
use crate::boolean_grid::{GRID_MM, Point64};
use crate::medial_axis::Region;

/// The internal blob ratio (Proposal, 2026-11-18): the orca's oval pupil is 0.8.
const BLOB_RATIO: f64 = 0.5;
/// Up to this Thick parts limit (the slider's end), in mm, the blob ratio holds.
const BLOB_FADE_FROM_MM: f64 = 20.0;
/// At this Thick parts limit (the field's maximum, where it means no limit), in mm, the blob
/// ratio has faded into the disc test.
const BLOB_FADE_TO_MM: f64 = 100.0;

/// A region's widest width and the diameter of its outer ring, in millimetres.
#[derive(Debug, Clone, Copy)]
pub(super) struct BlobMeasure {
    widest: f64,
    diameter: f64,
}

impl BlobMeasure {
    /// The measure of `region` with its widest width (small holes filled).
    pub(super) fn of(region: &Region, widest: f64) -> Self {
        Self {
            widest,
            diameter: diameter_mm(&region.outer),
        }
    }

    /// `w_max + 2 T(w_max) >= D`.
    pub(super) fn is_disc(self) -> bool {
        self.fills(1.0)
    }

    /// A blob at the Thick parts `limit`: at least half as wide as it is long (a pupil, an oval
    /// eye), which has no stroke to centre-line (the internal ratio criterion 12 allows, the old
    /// default of the blob ratio). From the slider's end to the field's maximum the ratio fades
    /// into the disc test, so no region jumps from filled to lines where the limit ends.
    pub(super) fn is_blob(self, limit: Length) -> bool {
        self.fills(ratio_at(limit))
    }

    /// `w_max + 2 T(w_max) >= ratio · D`.
    fn fills(self, ratio: f64) -> bool {
        self.widest + 2.0 * tolerance(self.widest) >= ratio * self.diameter
    }
}

/// The blob ratio at the Thick parts `limit`: [`BLOB_RATIO`] up to [`BLOB_FADE_FROM_MM`], 1 (the
/// disc test) at [`BLOB_FADE_TO_MM`], linear between.
fn ratio_at(limit: Length) -> f64 {
    let fade = ((limit.as_mm() - BLOB_FADE_FROM_MM) / (BLOB_FADE_TO_MM - BLOB_FADE_FROM_MM))
        .clamp(0.0, 1.0);
    BLOB_RATIO + (1.0 - BLOB_RATIO) * fade
}

/// The largest distance between two vertices of the ring's convex hull.
fn diameter_mm(ring: &[Point64]) -> f64 {
    let hull = hull(ring);
    let mut best: i128 = 0;
    for (i, a) in hull.iter().enumerate() {
        for b in &hull[i + 1..] {
            let (dx, dy) = (i128::from(b.x - a.x), i128::from(b.y - a.y));
            best = best.max(dx * dx + dy * dy);
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let squared = best as f64;
    squared.sqrt() * GRID_MM
}

/// The convex hull by the monotone chain, exact on grid integers.
fn hull(ring: &[Point64]) -> Vec<Point64> {
    let mut points: Vec<Point64> = ring.to_vec();
    points.sort_by_key(|p| (p.x, p.y));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let cross = |o: Point64, a: Point64, b: Point64| {
        i128::from(a.x - o.x) * i128::from(b.y - o.y)
            - i128::from(a.y - o.y) * i128::from(b.x - o.x)
    };
    let mut lower: Vec<Point64> = Vec::new();
    for &p in &points {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<Point64> = Vec::new();
    for &p in points.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A region 3 mm wide and 5 mm long stays filled at the default limit and becomes lines
    /// without one; between the slider's end and the maximum the ratio fades, with no jump at
    /// the maximum (architect review, 7).
    #[test]
    fn the_blob_ratio_fades_into_the_disc_test_at_the_maximum() {
        let oval = BlobMeasure {
            widest: 3.0,
            diameter: 5.0,
        };
        assert!(oval.is_blob(Length::from_mm(6.0)));
        assert!(oval.is_blob(Length::from_mm(20.0)));
        assert!(!oval.is_blob(Length::from_mm(99.9)));
        assert!(!oval.is_disc());
        let mut last = 0.0;
        for tenth in 0..=1000 {
            let ratio = ratio_at(Length::from_mm(f64::from(tenth) / 10.0));
            assert!(ratio >= last, "monotone");
            last = ratio;
        }
        assert!(
            (ratio_at(Length::from_mm(99.9)) - 1.0).abs() < 1e-2,
            "no jump at 100 mm"
        );
    }
}
