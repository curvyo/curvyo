//! The closed outline of an open centre line drawn with a width that varies along it, with round
//! joins and round ends (`specs/0032-pen-tablet-input` criterion 8, `adrs.md` "the
//! variable-width outline").
//!
//! The centre line is flattened within 0.01 mm, every vertex gets the width the profile gives at
//! its fraction of the line's length, the vertices are snapped to the 0.001 mm grid and
//! `i_overlay`'s integer variable-width stroke builds the union of the swept discs. Its arcs are
//! stepped so that every chord stays within 0.01 mm of the circle, and the result goes through
//! the boolean kernel's cleanup and canonical form. The error budget is 0.01 mm (centre line) +
//! 0.01 mm (arcs).

use curvyo_document_core::{Length, Point, Vec2};
use i_overlay::i_float::int::angle::Angle;
use i_overlay::mesh::int::arc::ArcOptions;
use i_overlay::mesh::int::variable_stroke::offset::IntVariableStrokeOffset;
use i_overlay::mesh::int::variable_stroke::{IntStrokeVertex, IntVariableStrokeStyle};
use i_overlay::mesh::math::MathMode;

use crate::OutlineTriple;
use crate::boolean_cleanup::cleanup;
use crate::boolean_grid::{GRID_MM, MAX_COORDINATE_MM, Paths64, canonical_millimetres, snap};
use crate::flatten::flatten_closed;

/// Largest distance of the flattened centre line from the curve, in millimetres.
const CENTRE_TOLERANCE_MM: f64 = 0.01;

/// Largest distance of an arc chord from its circle, in millimetres.
const ARC_TOLERANCE_MM: f64 = 0.01;

/// Why no outline was built.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VariableOutlineError {
    /// The centre line has fewer than two anchors, or the outline has no area.
    #[error("the stroke has no outline")]
    Empty,
    /// A coordinate or width is not finite or lies beyond the kernel's range.
    #[error("a coordinate or width is not finite or out of range")]
    OutOfRange,
}

/// The width along a stroke, by the fraction of its length: built from the samples a stroke was
/// drawn with and their widths, and read at any fraction from 0 (start) to 1 (end).
#[derive(Debug, Clone, PartialEq)]
pub struct WidthProfile {
    /// `(fraction of the length, width in mm)`, fractions non-decreasing from 0 to 1.
    stations: Vec<(f64, f64)>,
}

impl WidthProfile {
    /// The profile of samples at `points` with `widths` (same length; extra entries of either
    /// are ignored). A width is placed at the sample's fraction of the polyline's length.
    #[must_use]
    pub fn from_samples(points: &[Point], widths: &[Length]) -> Self {
        let count = points.len().min(widths.len());
        let mut lengths = Vec::with_capacity(count);
        let mut total = 0.0;
        for index in 0..count {
            if index > 0 {
                total += points[index - 1].vector_to(points[index]).length();
            }
            lengths.push(total);
        }
        let stations = lengths
            .iter()
            .zip(widths)
            .map(|(length, width)| {
                let fraction = if total > 0.0 { length / total } else { 0.0 };
                (fraction, width.as_mm())
            })
            .collect();
        Self { stations }
    }

    /// The width at `fraction` of the length, linearly interpolated between the samples; the
    /// first or last width outside them, zero for an empty profile.
    #[must_use]
    pub fn width_at(&self, fraction: f64) -> Length {
        let Some(first) = self.stations.first() else {
            return Length::from_mm(0.0);
        };
        let after = self.stations.partition_point(|(f, _)| *f < fraction);
        let width = if after == 0 {
            first.1
        } else if after >= self.stations.len() {
            self.stations[self.stations.len() - 1].1
        } else {
            let (f0, w0) = self.stations[after - 1];
            let (f1, w1) = self.stations[after];
            if f1 > f0 {
                w0 + (w1 - w0) * (fraction - f0) / (f1 - f0)
            } else {
                w1
            }
        };
        Length::from_mm(width)
    }

    /// The largest width of the profile.
    fn max_width_mm(&self) -> f64 {
        self.stations.iter().map(|(_, w)| *w).fold(0.0, f64::max)
    }
}

/// The vertices of the open centre line through `anchors`, within `CENTRE_TOLERANCE_MM`.
fn flatten_open(anchors: &[OutlineTriple]) -> Vec<Point> {
    let mut points = Vec::new();
    for pair in anchors.windows(2) {
        // A closed pair whose closing segment is straight adds no vertex for that segment, so
        // the pair's vertices are the open segment's, end included.
        let segment = [
            (pair[0].0, Vec2::ZERO, pair[0].2),
            (pair[1].0, pair[1].1, Vec2::ZERO),
        ];
        let mut vertices = flatten_closed(&segment, CENTRE_TOLERANCE_MM);
        vertices.pop();
        points.extend(vertices);
    }
    if let Some(last) = anchors.last() {
        points.push(last.0);
    }
    points
}

/// The arc options that keep every chord of a circle up to `radius_mm` within
/// `ARC_TOLERANCE_MM` (`specs/0038-path-offset/adrs.md`); the library clamps the step.
fn arc_options(radius_mm: f64) -> ArcOptions {
    let ratio = (1.0 - ARC_TOLERANCE_MM / radius_mm.max(ARC_TOLERANCE_MM)).clamp(-1.0, 1.0);
    let step = 2.0 * ratio.acos();
    let default = ArcOptions::default();
    ArcOptions {
        max_step: Angle::from_radians(step).unwrap_or(ArcOptions::MIN_STEP),
        ..default
    }
}

/// Whether `value` is finite and within the kernel's range.
fn in_range(value: f64) -> bool {
    value.is_finite() && value.abs() <= MAX_COORDINATE_MM
}

/// The outline of the open centre line `centre` (anchors with relative handles) drawn with the
/// widths of `profile`, round joins and round ends. Outer outlines have a positive
/// [`crate::signed_area_mm2`], holes (where the stroke crosses itself around an empty area) a
/// negative one, in the canonical form of [`crate::BooleanResult`].
///
/// # Errors
///
/// [`VariableOutlineError::Empty`] for fewer than two anchors or a result without area,
/// [`VariableOutlineError::OutOfRange`] for a coordinate or width that is not finite or out of
/// the kernel's range.
pub fn variable_outline(
    centre: &[OutlineTriple],
    profile: &WidthProfile,
) -> Result<Vec<Vec<Point>>, VariableOutlineError> {
    if centre.len() < 2 {
        return Err(VariableOutlineError::Empty);
    }
    let points = flatten_open(centre);
    let max_width = profile.max_width_mm();
    if !points.iter().all(|p| in_range(p.x) && in_range(p.y)) || !in_range(max_width) {
        return Err(VariableOutlineError::OutOfRange);
    }
    let mut lengths = Vec::with_capacity(points.len());
    let mut total = 0.0;
    for (index, point) in points.iter().enumerate() {
        if index > 0 {
            total += points[index - 1].vector_to(*point).length();
        }
        lengths.push(total);
    }
    let mut vertices: Vec<IntStrokeVertex<i64>> = Vec::with_capacity(points.len());
    for (point, length) in points.iter().zip(&lengths) {
        let fraction = if total > 0.0 { length / total } else { 0.0 };
        // In range (checked above), so the cast is exact.
        #[allow(clippy::cast_possible_truncation)]
        let width = (profile.width_at(fraction).as_mm() / GRID_MM)
            .round()
            .max(0.0) as i64;
        let snapped = snap(*point);
        match vertices.last_mut() {
            Some(last) if last.point == snapped => last.width = last.width.max(width),
            _ => vertices.push(IntStrokeVertex::new(snapped, width)),
        }
    }
    let style = IntVariableStrokeStyle::new()
        .math(MathMode::Integer)
        .arc(arc_options(max_width / 2.0));
    let shapes = vertices
        .variable_stroke(style)
        .map_err(|_| VariableOutlineError::OutOfRange)?;
    let raw: Paths64 = shapes.into_iter().flatten().collect();
    let outlines = canonical_millimetres(&cleanup(&raw));
    if outlines.is_empty() {
        return Err(VariableOutlineError::Empty);
    }
    Ok(outlines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_profile_interpolates_by_fraction_of_the_length() {
        let points = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(30.0, 0.0),
        ];
        let widths = [1.0, 2.0, 4.0].map(Length::from_mm);
        let profile = WidthProfile::from_samples(&points, &widths);
        assert!((profile.width_at(0.0).as_mm() - 1.0).abs() < 1e-12);
        assert!((profile.width_at(1.0 / 6.0).as_mm() - 1.5).abs() < 1e-12);
        assert!((profile.width_at(2.0 / 3.0).as_mm() - 3.0).abs() < 1e-12);
        assert!((profile.width_at(2.0).as_mm() - 4.0).abs() < 1e-12);
    }

    #[test]
    fn an_open_centre_line_flattens_without_its_closing_segment() {
        let anchors = [
            (Point::new(0.0, 0.0), Vec2::ZERO, Vec2::ZERO),
            (Point::new(10.0, 0.0), Vec2::ZERO, Vec2::ZERO),
            (Point::new(10.0, 10.0), Vec2::ZERO, Vec2::ZERO),
        ];
        assert_eq!(
            flatten_open(&anchors),
            vec![
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0)
            ]
        );
    }
}
