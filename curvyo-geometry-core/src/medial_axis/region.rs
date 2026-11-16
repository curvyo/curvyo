//! The regions a set of outlines paints, each one outer ring with its holes on the 0.001 mm grid
//! in canonical order (ADR 0063 Decision 1).

use kurbo::Point;

use super::CentreLineError;
use crate::Outline;
use crate::boolean::grid_paths;
use crate::boolean_grid::{
    GRID_MM, MAX_COORDINATE_MM, Path64, normalize_shapes, rotate_to_start, to_millimetres,
};

/// The chord tolerance the outlines are flattened to: a quarter of the smallest `T(w)` (0.05 mm).
/// The clearance error is at most this, so the width (twice the clearance) is off by at most
/// `T(w)/2` below 1.67 mm and by less above (ADR 0063 amendment 2026-11-16, Proposal).
pub(crate) const FLATTEN_MM: f64 = 0.0125;

/// One connected filled region: an outer ring (positive area) and its holes (negative area),
/// each ring starting at its smallest grid point, holes sorted by that point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Region {
    /// The outer ring.
    pub(crate) outer: Path64,
    /// The holes.
    pub(crate) holes: Vec<Path64>,
}

impl Region {
    /// Every ring, outer first, in millimetres.
    pub(crate) fn rings_mm(&self) -> Vec<Vec<Point>> {
        self.rings()
            .map(|ring| {
                ring.iter()
                    .map(|p| {
                        let mm = to_millimetres(p);
                        Point::new(mm.x, mm.y)
                    })
                    .collect()
            })
            .collect()
    }

    /// Every ring, outer first, in grid units.
    pub(crate) fn rings(&self) -> impl Iterator<Item = &Path64> {
        std::iter::once(&self.outer).chain(&self.holes)
    }

    /// The number of ring vertices.
    #[cfg(test)]
    pub(crate) fn vertex_count(&self) -> usize {
        self.rings().map(Vec::len).sum()
    }
}

/// The regions `outlines` paint under the nonzero rule, sorted by their outer ring's start.
///
/// # Errors
/// [`CentreLineError::NonFinite`] or [`CentreLineError::OutOfRange`] for a bad coordinate.
pub(crate) fn regions(outlines: &[Outline<'_>]) -> Result<Vec<Region>, CentreLineError> {
    for outline in outlines {
        for (point, handle_in, handle_out) in outline.anchors {
            let values = [
                point.x,
                point.y,
                point.x + handle_in.x,
                point.y + handle_in.y,
                point.x + handle_out.x,
                point.y + handle_out.y,
            ];
            if values.iter().any(|v| !v.is_finite()) {
                return Err(CentreLineError::NonFinite);
            }
            if values.iter().any(|v| v.abs() > MAX_COORDINATE_MM) {
                return Err(CentreLineError::OutOfRange);
            }
        }
    }
    let paths = grid_paths(outlines, FLATTEN_MM - GRID_MM);
    Ok(regions_of_paths(&paths))
}

/// The regions of grid polygons under the nonzero rule.
pub(crate) fn regions_of_paths(paths: &[Path64]) -> Vec<Region> {
    let mut regions: Vec<Region> = normalize_shapes(&paths.to_vec())
        .into_iter()
        .filter_map(|shape| {
            let mut rings = shape.into_iter().map(|ring| rotate_to_start(&ring));
            let outer = rings.next()?;
            let mut holes: Vec<Path64> = rings.collect();
            holes.sort_by_key(|hole| (hole[0].x, hole[0].y));
            Some(Region { outer, holes })
        })
        .collect();
    regions.sort_by_key(|region| (region.outer[0].x, region.outer[0].y));
    regions
}
