//! Leaf outlines as µm polylines (criterion 17): flattened at 0.0095 mm, rounded to µm, repeats
//! removed.

use curvyo_document_core::Tolerance;
use curvyo_geometry_core::flatten_outline;

use crate::job_input::{LeafOutline, PointUm};

/// The flattening tolerance: with the 0.0005 mm of µm rounding the written path stays within
/// 0.01 mm of the curve (criterion 17).
pub const FLATTEN_MM: f64 = 0.0095;

/// The outline flattened and rounded to µm, `None` with fewer than two distinct points.
pub(crate) fn flat(outline: &LeafOutline) -> Option<Vec<PointUm>> {
    let points = flatten_outline(
        &outline.anchors,
        outline.closed,
        Tolerance::from_mm(FLATTEN_MM),
    );
    let points = dedupe(points.into_iter().map(PointUm::of_mm), outline.closed);
    (points.len() >= 2).then_some(points)
}

/// The points with consecutive repeats removed (and a closing repeat of the first when closed).
pub(crate) fn dedupe(points: impl Iterator<Item = PointUm>, closed: bool) -> Vec<PointUm> {
    let mut out: Vec<PointUm> = Vec::new();
    for point in points {
        if out.last() != Some(&point) {
            out.push(point);
        }
    }
    if closed && out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}
