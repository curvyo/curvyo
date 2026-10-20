//! The band a stroke paints along open outlines, as a region on the 0.001 mm grid
//! (`specs/0068-laserable-lettering` criterion 9, `adrs.md` decision 3): `i_overlay`'s integer
//! stroke builder with the drawn width, join, cap and the renderer's miter limit of 4. Dashes are
//! not geometry here; the band is solid. The later Stroke to path command reuses it.

use curvyo_document_core::{Length, LineCap, LineJoin};
use i_overlay::mesh::int::stroke::offset::IntStrokeOffset;
use i_overlay::mesh::int::style::{IntLineCap, IntStrokeStyle};
use i_overlay::mesh::math::MathMode;

use crate::Outline;
use crate::boolean_grid::{GRID_MM, Path64, Paths64, snap};
use crate::flatten::flatten;
use crate::offset::{OffsetError, OffsetJoin};
use crate::offset_polyline::{arc_options, int_join, miter_min_turn};

/// How a stroke is drawn along an open outline: its full width, its join and its caps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeBand {
    /// The full stroke width.
    pub width: Length,
    /// The join at corners; a miter is clipped at the limit of 4, as drawn.
    pub join: LineJoin,
    /// The cap at both ends.
    pub cap: LineCap,
}

/// The largest distance a chord of a round join or cap may lie inside its arc, in millimetres
/// (`adrs.md` decision 2: the outer bound stays under g/2 + 0.01 mm).
pub(crate) const ARC_TOLERANCE_MM: f64 = 0.004;

/// The band of `band` along every open outline of `outlines`, as one painted region; empty when
/// the stroke is thinner than two grid pitches.
///
/// # Errors
/// [`OffsetError::OutOfRange`] when the band would leave the range of the integer engine.
pub(crate) fn stroke_band(
    outlines: &[Outline<'_>],
    band: StrokeBand,
    flatten_tolerance_mm: f64,
) -> Result<Paths64, OffsetError> {
    let width_mm = band.width.as_mm();
    if !(width_mm.is_finite() && width_mm > 0.0) {
        return Ok(Paths64::new());
    }
    // Finite and positive; a width beyond the grid range is refused by the bounds check below.
    #[allow(clippy::cast_possible_truncation)]
    let width_units = (width_mm / GRID_MM).round() as i64;
    let radius_mm = width_mm / 2.0;
    let polylines: Paths64 = outlines
        .iter()
        .filter(|outline| !outline.closed)
        .map(|outline| snap_polyline(&flatten(outline.anchors, false, flatten_tolerance_mm)))
        .filter(|polyline| polyline.len() >= 2)
        .collect();
    if polylines.is_empty() {
        return Ok(Paths64::new());
    }
    let join = match band.join {
        LineJoin::Miter => OffsetJoin::Miter,
        LineJoin::Round => OffsetJoin::Round,
        LineJoin::Bevel => OffsetJoin::Bevel,
    };
    let cap = match band.cap {
        LineCap::Butt => IntLineCap::Butt,
        LineCap::Square => IntLineCap::Square,
        LineCap::Round => IntLineCap::Round(arc_options(radius_mm, ARC_TOLERANCE_MM)),
    };
    let style = IntStrokeStyle::new(width_units)
        .start_cap(cap.clone())
        .end_cap(cap)
        .line_join(int_join(join, radius_mm, ARC_TOLERANCE_MM))
        .miter_min_turn(miter_min_turn(radius_mm))
        .math(MathMode::Integer);
    polylines
        .validate_stroke(&style)
        .map_err(|_| OffsetError::OutOfRange)?;
    let shapes = polylines
        .stroke(&style, false)
        .map_err(|_| OffsetError::OutOfRange)?;
    Ok(shapes.into_iter().flatten().collect())
}

/// A polyline snapped to the grid without consecutive duplicates; its two ends may coincide.
fn snap_polyline(points: &[curvyo_document_core::Point]) -> Path64 {
    let mut path: Path64 = Vec::with_capacity(points.len());
    for point in points {
        let snapped = snap(*point);
        if path.last() != Some(&snapped) {
            path.push(snapped);
        }
    }
    path
}
