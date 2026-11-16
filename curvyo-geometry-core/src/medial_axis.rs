//! The approximate medial axis of one filled region (ADR 0063 §2 and §3): axis samples with their
//! clearance radius, as edges between junctions and ends, plus closed rings.
//!
//! The region is triangulated by a constrained Delaunay triangulation of its outline; every chord
//! (a triangle edge inside the region) carries one axis point, the point of the chord farthest
//! from the outline. End triangles reach the tip, junction triangles meet at their centre. The
//! raw graph is then trimmed at rounded ends and cleared of spurs shorter than their junction's
//! width.

pub(crate) mod boundary;
mod chords;
mod extract;
mod graph;
mod mesh;
pub(crate) mod region;
mod spurs;
#[cfg(test)]
pub(crate) mod tests;
mod trim;
#[cfg(test)]
mod walk_tests;

use kurbo::Point;

pub(crate) use region::{Region, regions};

/// Why the centre lines of a shape could not be found. Nothing is changed in any of these cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CentreLineError {
    /// A coordinate or handle is not finite.
    #[error("a coordinate is not finite")]
    NonFinite,
    /// A coordinate lies beyond [`crate::MAX_COORDINATE_MM`].
    #[error("a coordinate is out of range")]
    OutOfRange,
    /// The outline could not be triangulated (its edges cross after normalisation).
    #[error("the outline could not be triangulated")]
    Triangulation,
}

/// A point on the axis, in millimetres, and its clearance: the distance to the nearest outline,
/// half the local width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AxisSample {
    /// The position.
    pub(crate) point: Point,
    /// The distance to the outline.
    pub(crate) radius: f64,
}

/// A run of the axis between two vertices (junctions or ends), both ends included.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AxisEdge {
    /// The vertex the samples start at (an index into [`MedialAxis::vertices`]).
    pub(crate) from: usize,
    /// The vertex the samples end at.
    pub(crate) to: usize,
    /// The samples, from `from` to `to`, at least two.
    pub(crate) samples: Vec<AxisSample>,
}

/// The cleaned axis of one region.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct MedialAxis {
    /// Junctions (three or more edges) and ends (one edge).
    pub(crate) vertices: Vec<AxisSample>,
    /// The edges between them, in a deterministic order.
    pub(crate) edges: Vec<AxisEdge>,
    /// Closed loops without a junction, each starting at its smallest point (x, then y).
    pub(crate) rings: Vec<Vec<AxisSample>>,
    /// The largest clearance of the raw axis: the radius of the largest disc in the region.
    pub(crate) widest: f64,
    /// The region's holes in ring order.
    pub(crate) holes: Vec<HoleMeasure>,
}

/// A hole's size, and for a small hole (at most the region's widest width across, a highlight
/// in an iris) the width of the largest disc around it with the hole filled, in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct HoleMeasure {
    /// The larger extent of the hole's bounding box.
    pub(crate) diameter: f64,
    /// For a small hole: the widest width with the hole filled.
    pub(crate) filled: Option<f64>,
}

/// The medial axis of `region`.
///
/// # Errors
/// [`CentreLineError::Triangulation`] when the outline's edges cross.
pub(crate) fn medial_axis(region: &Region) -> Result<MedialAxis, CentreLineError> {
    let mesh = mesh::triangulate(region)?;
    let boundary = boundary::Boundary::new(&region.rings_mm());
    let mut graph = graph::Graph::build(&mesh, &boundary);
    let widest = graph.samples.iter().map(|s| s.radius).fold(0.0, f64::max);
    spurs::prune_spurs(&mut graph, &boundary, false);
    trim::trim_ends(&mut graph, &boundary);
    spurs::prune_spurs(&mut graph, &boundary, true);
    trim::trim_steep_ends(&mut graph, &boundary);
    let mut axis = extract::extract(&graph);
    axis.widest = widest;
    axis.holes = holes(&region.rings_mm(), &boundary, widest);
    Ok(axis)
}

/// The measures of the holes (rings after the first) of a region whose largest clearance is
/// `widest`: a hole no wider than the widest width is small, and its filled width is twice the
/// largest clearance from the other rings on the ray from its centre away from them.
fn holes(rings: &[Vec<Point>], boundary: &boundary::Boundary, widest: f64) -> Vec<HoleMeasure> {
    rings
        .iter()
        .enumerate()
        .skip(1)
        .map(|(k, hole)| {
            let diameter = diameter(hole);
            let filled = (diameter <= 2.0 * widest).then(|| {
                #[allow(clippy::cast_precision_loss)]
                let n = hole.len().max(1) as f64;
                let centre = Point::new(
                    hole.iter().map(|p| p.x).sum::<f64>() / n,
                    hole.iter().map(|p| p.y).sum::<f64>() / n,
                );
                let Some((_, foot, _)) = boundary.nearest_without(centre, k) else {
                    return 2.0 * widest;
                };
                let clearance = |p: Point| boundary.distance_without(p, k);
                let away = centre - foot;
                if away.hypot() == 0.0 {
                    return 2.0 * widest;
                }
                let away = away / away.hypot();
                // Up to where the ray leaves the region (sphere tracing): beyond, the distance
                // to the outline grows again outside.
                let mut exit = 0.0;
                for _ in 0..64 {
                    let step = clearance(centre + away * exit);
                    if step < 1e-4 {
                        break;
                    }
                    exit += step;
                }
                let (mut low, mut high) = (0.0_f64, exit);
                for _ in 0..40 {
                    let (a, b) = (low + (high - low) * 0.382, low + (high - low) * 0.618);
                    if clearance(centre + away * a) < clearance(centre + away * b) {
                        low = a;
                    } else {
                        high = b;
                    }
                }
                2.0 * clearance(centre + away * f64::midpoint(low, high)).max(widest)
            });
            HoleMeasure { diameter, filled }
        })
        .collect()
}

/// The larger extent of a ring's bounding box, in millimetres.
pub(crate) fn diameter(ring: &[Point]) -> f64 {
    let (mut low, mut high) = (
        Point::new(f64::INFINITY, f64::INFINITY),
        Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for p in ring {
        low = Point::new(low.x.min(p.x), low.y.min(p.y));
        high = Point::new(high.x.max(p.x), high.y.max(p.y));
    }
    (high.x - low.x).max(high.y - low.y)
}
