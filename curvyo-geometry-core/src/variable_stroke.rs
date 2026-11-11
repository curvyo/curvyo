//! The outline of a stroke whose width follows a relative profile (ADR 0053, amendment of
//! 2026-11-10): the centre line sampled adaptively, offset along its normal by the local half
//! width and united on the 0.001 mm grid, within 0.01 mm by construction.

mod pieces;
mod samples;

use curvyo_document_core::{Length, Point as DocPoint, Tolerance, Vec2 as DocVec};
use i_overlay::core::overlay_rule::OverlayRule;
use kurbo::{BezPath, CubicBez, Line, PathEl, PathSeg, Point, Vec2};

use crate::boolean_grid::{
    MAX_COORDINATE_MM, Paths64, canonical_millimetres, double_area, normalize, run, snap_polygon,
};
use crate::offset::{OffsetAnchor, OffsetOutlines};
use crate::offset_pieces::segments_between;
use crate::stroke_dash::{Measured, dash_spans, object_is_dashed};
use crate::stroke_loops::Cap;
use crate::stroke_markers::marker_loops;
use crate::{Outline, StrokeOutlineError, StrokeShape, WidthProfile, stroke_outline};
use pieces::{Node, Polygons, Shape, chain_polygons, dot};
use samples::{Sample, Targets, sample_path};

/// The tolerance the construction targets are made for; a finer one is met by magnifying.
const NATIVE_TOLERANCE_MM: f64 = 0.01;
/// A direction shorter than this has none: the tangent falls back to the sample's.
const DIRECTION_FLOOR_MM: f64 = 1e-9;
/// A dash that ends this close to a closed path's end runs on through its start.
const SEAM_MM: f64 = 1e-9;

/// A variable stroke could not be converted to an outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VariableStrokeError {
    /// The tolerance, a coordinate or the width is out of range, or the stroke paints nothing.
    #[error(transparent)]
    Outline(#[from] StrokeOutlineError),
    /// The centre line has no length, so no position along it is defined.
    #[error("the path has no length")]
    ZeroLength,
    /// The profile was made for the other closure, or the path cannot carry a profile.
    #[error("this variable width profile does not fit the path")]
    Unsupported,
}

/// The region the stroke paints along `outline` with widths `style.width × profile`, as closed
/// polyline outlines on the 0.001 mm grid (outer outlines positive, holes negative). A constant
/// profile is the ordinary [`stroke_outline`] at that width.
///
/// # Errors
/// [`VariableStrokeError`]: nonfinite or out-of-range input, a zero-length centre line, a profile
/// for the other closure; `Outline(Empty)` when the stroke paints nothing.
pub fn variable_stroke_outline(
    outline: Outline<'_>,
    style: &StrokeShape,
    profile: &WidthProfile,
    tolerance: Tolerance,
) -> Result<OffsetOutlines, VariableStrokeError> {
    outline_with_density(outline, style, profile, tolerance, 1)
}

/// [`variable_stroke_outline`] with every sampled interval cut into `density` parameter steps;
/// the tests compare against a dense reference this way.
fn outline_with_density(
    outline: Outline<'_>,
    style: &StrokeShape,
    profile: &WidthProfile,
    tolerance: Tolerance,
    density: u32,
) -> Result<OffsetOutlines, VariableStrokeError> {
    if outline.closed != profile.is_closed() {
        return Err(VariableStrokeError::Unsupported);
    }
    let tol = tolerance.as_mm();
    if !(tol.is_finite() && tol > 0.0) {
        return Err(StrokeOutlineError::ToleranceTooSmall.into());
    }
    if tol < NATIVE_TOLERANCE_MM {
        return scaled_outline(outline, style, profile, tol, density);
    }
    if let Some(multiplier) = profile.constant_multiplier() {
        let width = style.width.as_mm() * multiplier;
        if width == 0.0 {
            return Err(StrokeOutlineError::Empty.into());
        }
        let mut actual = style.clone();
        actual.width = Length::from_mm(width);
        return Ok(stroke_outline(&[outline], &actual, tolerance)?);
    }
    varying_outline(outline, style, profile, tol, density)
}

/// The outline of a varying profile at a tolerance of at least [`NATIVE_TOLERANCE_MM`].
fn varying_outline(
    outline: Outline<'_>,
    style: &StrokeShape,
    profile: &WidthProfile,
    tolerance: f64,
    density: u32,
) -> Result<OffsetOutlines, VariableStrokeError> {
    let base = style.width.as_mm();
    let max_width = checked_max_width(outline, base, profile)?;
    let anchors = kurbo_anchors(outline);
    let segments = segments_between(&anchors, outline.closed);
    let cubics: Vec<CubicBez> = segments.iter().map(as_cubic).collect();
    let half_at =
        |position: f64| 0.5 * base * profile.sample(position.clamp(0.0, 1.0)).unwrap_or(0.0);
    let forced: Vec<f64> = profile.positions().collect();
    let targets = Targets::new(tolerance, max_width);
    let (mut samples, mut total) =
        sample_path(&cubics, targets, &forced, &half_at).ok_or(VariableStrokeError::ZeroLength)?;
    if density > 1 {
        total = samples::densify(&mut samples, &cubics, density);
    }
    let path = Sampled {
        samples: &samples,
        cubics: &cubics,
        total,
        half_at: &half_at,
    };
    let shape = Shape {
        join: style.join,
        cap: style.cap,
        sagitta: targets.chord,
    };
    let mut polygons = Polygons::default();
    stroke_polygons(
        &path,
        outline.closed,
        style,
        &segments,
        shape,
        &mut polygons,
    );
    let subjects = snapped_subjects(&polygons);
    let markers: Paths64 = marker_loops(&anchors, outline.closed, &segments, style.markers, base)
        .iter()
        .map(|pieces| snap_polygon(&as_doc(&flattened(pieces, targets.chord))))
        .filter(|path| path.len() >= 3)
        .collect();
    let united = if markers.is_empty() {
        normalize(&subjects)
    } else {
        run(OverlayRule::Union, &subjects, &markers)
    };
    if united.is_empty() {
        return Err(StrokeOutlineError::Empty.into());
    }
    Ok(corner_rings(&united))
}

/// The largest width of the profile on `base`, once the width and the reach of the outline are
/// in range and the stroke paints something.
fn checked_max_width(
    outline: Outline<'_>,
    base: f64,
    profile: &WidthProfile,
) -> Result<f64, VariableStrokeError> {
    let max_width = base * profile.max_multiplier();
    if !(base.is_finite() && base >= 0.0 && max_width.is_finite() && max_width <= MAX_COORDINATE_MM)
    {
        return Err(StrokeOutlineError::OutOfRange.into());
    }
    if !in_range(outline, 4.0 * max_width.max(base)) {
        return Err(StrokeOutlineError::OutOfRange.into());
    }
    if max_width == 0.0 {
        return Err(StrokeOutlineError::Empty.into());
    }
    Ok(max_width)
}

/// The anchors of `outline` as kurbo points and handle vectors.
fn kurbo_anchors(outline: Outline<'_>) -> Vec<(Point, Vec2, Vec2)> {
    outline
        .anchors
        .iter()
        .map(|(p, a, b)| {
            (
                Point::new(p.x, p.y),
                Vec2::new(a.x, a.y),
                Vec2::new(b.x, b.y),
            )
        })
        .collect()
}

/// The stroke pieces on the 0.001 mm grid: simple polygons as they are, each folded ring split
/// into its positive parts.
fn snapped_subjects(polygons: &Polygons) -> Paths64 {
    let mut subjects: Paths64 = polygons
        .simple
        .iter()
        .map(|polygon| snap_polygon(&as_doc(polygon)))
        .filter(|path| path.len() >= 3)
        .collect();
    for ring in &polygons.folded {
        let path = snap_polygon(&as_doc(ring));
        if path.len() >= 3 {
            subjects.extend(
                normalize(&vec![path])
                    .into_iter()
                    .filter(|p| double_area(p) > 0),
            );
        }
    }
    subjects
}

/// The united grid paths as outlines of corner anchors in millimetres.
fn corner_rings(united: &Paths64) -> OffsetOutlines {
    let rings = canonical_millimetres(united)
        .into_iter()
        .map(|ring| {
            ring.into_iter()
                .map(|point| OffsetAnchor {
                    point,
                    handle_in: DocVec::ZERO,
                    handle_out: DocVec::ZERO,
                    smooth: false,
                })
                .collect()
        })
        .collect();
    OffsetOutlines::from_parts(rings, 0)
}

/// The sampled centre line and its widths.
struct Sampled<'a> {
    samples: &'a [Sample],
    cubics: &'a [CubicBez],
    total: f64,
    half_at: &'a dyn Fn(f64) -> f64,
}

impl Sampled<'_> {
    /// The nodes of the whole path, closed back to the first node when `closed`.
    fn nodes(&self, closed: bool) -> Vec<Node> {
        let mut nodes: Vec<Node> = self
            .samples
            .iter()
            .enumerate()
            .map(|(index, sample)| Node {
                point: sample.point,
                tangent: sample.tangent,
                half: (self.half_at)(sample.s / self.total),
                junction: index > 0 && self.samples[index - 1].segment != sample.segment,
            })
            .collect();
        if closed && let Some(first) = nodes.first().copied() {
            nodes.push(Node {
                junction: true,
                ..first
            });
        }
        nodes
    }

    /// The nodes between the chord lengths `a < b`, cut at both.
    fn between(&self, a: f64, b: f64) -> Vec<Node> {
        let mut nodes = vec![self.node_at(a)];
        for (index, sample) in self.samples.iter().enumerate() {
            if sample.s > a && sample.s < b {
                nodes.push(Node {
                    point: sample.point,
                    tangent: sample.tangent,
                    half: (self.half_at)(sample.s / self.total),
                    junction: index > 0 && self.samples[index - 1].segment != sample.segment,
                });
            }
        }
        nodes.push(self.node_at(b));
        nodes
    }

    /// The node at chord length `s`, on the sampled interval that contains it.
    fn node_at(&self, s: f64) -> Node {
        let index = self
            .samples
            .windows(2)
            .position(|pair| pair[0].s <= s && s <= pair[1].s && pair[1].s > pair[0].s)
            .unwrap_or(0);
        let (a, b) = (
            self.samples[index],
            self.samples[(index + 1).min(self.samples.len() - 1)],
        );
        let fraction = if b.s > a.s {
            ((s - a.s) / (b.s - a.s)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let t = a.t + (b.t - a.t) * fraction;
        let cubic = self.cubics[a.segment];
        let derivative = kurbo::ParamCurveDeriv::deriv(&cubic);
        let direction = kurbo::ParamCurve::eval(&derivative, t).to_vec2();
        let chord = b.point - a.point;
        let tangent = [direction, chord]
            .into_iter()
            .find(|v| v.hypot() > DIRECTION_FLOOR_MM)
            .map_or(a.tangent, |v| v / v.hypot());
        Node {
            point: kurbo::ParamCurve::eval(&cubic, t),
            tangent,
            half: (self.half_at)(s / self.total),
            junction: false,
        }
    }
}

/// The stroke polygons of the sampled path: solid, or one chain per dash and a dot per
/// zero-length dash.
fn stroke_polygons(
    path: &Sampled<'_>,
    closed: bool,
    style: &StrokeShape,
    segments: &[PathSeg],
    shape: Shape,
    out: &mut Polygons,
) {
    let base = style.width.as_mm();
    let cap = match style.cap {
        curvyo_document_core::LineCap::Butt => Cap::Butt,
        curvyo_document_core::LineCap::Round => Cap::Round,
        curvyo_document_core::LineCap::Square => Cap::Square,
    };
    let measured = [Measured::new(segments)];
    let spans = if object_is_dashed(&measured, &style.dash, base, cap) {
        dash_spans(&style.dash, base, cap, path.total)
    } else {
        None
    };
    let Some(mut spans) = spans else {
        chain_polygons(&path.nodes(closed), !closed, shape, out);
        return;
    };
    let end = path.total - SEAM_MM;
    if closed && spans.len() == 1 && spans[0].0 <= 0.0 && spans[0].1 >= end {
        chain_polygons(&path.nodes(true), false, shape, out);
        return;
    }
    if closed
        && spans.len() >= 2
        && spans[0].0 <= 0.0
        && spans[0].1 > 0.0
        && spans[spans.len() - 1].1 >= end
    {
        let first = spans.remove(0);
        let last = spans.pop().unwrap_or(first);
        let mut chain = path.between(last.0, path.total);
        let mut rest = path.between(0.0, first.1);
        if let Some(seam) = rest.first_mut() {
            seam.junction = true;
        }
        chain.extend(rest);
        chain_polygons(&chain, true, shape, out);
    }
    for (start, stop) in spans {
        if stop > start {
            chain_polygons(&path.between(start, stop), true, shape, out);
        } else {
            dot(&path.node_at(start), shape, out);
        }
    }
}

/// `segment` as a cubic (a line with its handles at the thirds).
fn as_cubic(segment: &PathSeg) -> CubicBez {
    match *segment {
        PathSeg::Cubic(cubic) => cubic,
        PathSeg::Quad(quad) => quad.raise(),
        PathSeg::Line(Line { p0, p1 }) => {
            CubicBez::new(p0, p0.lerp(p1, 1.0 / 3.0), p0.lerp(p1, 2.0 / 3.0), p1)
        }
    }
}

/// The loop `pieces` flattened within `sagitta`.
fn flattened(pieces: &[PathSeg], sagitta: f64) -> Vec<Point> {
    let mut path = BezPath::new();
    for (index, piece) in pieces.iter().enumerate() {
        if index == 0 {
            path.move_to(kurbo::ParamCurve::start(piece));
        }
        match *piece {
            PathSeg::Line(line) => path.line_to(line.p1),
            PathSeg::Quad(quad) => path.quad_to(quad.p1, quad.p2),
            PathSeg::Cubic(cubic) => path.curve_to(cubic.p1, cubic.p2, cubic.p3),
        }
    }
    let mut points = Vec::new();
    kurbo::flatten(&path, sagitta, |element| {
        if let PathEl::MoveTo(p) | PathEl::LineTo(p) = element {
            points.push(p);
        }
    });
    points
}

fn as_doc(points: &[Point]) -> Vec<DocPoint> {
    points.iter().map(|p| DocPoint::new(p.x, p.y)).collect()
}

/// Whether every coordinate and handle of `outline`, grown by `reach`, stays within range.
fn in_range(outline: Outline<'_>, reach: f64) -> bool {
    let ok = |value: f64| value.is_finite() && value.abs() + reach <= MAX_COORDINATE_MM;
    outline.anchors.iter().all(|(p, a, b)| {
        [p.x, p.y, p.x + a.x, p.y + a.y, p.x + b.x, p.y + b.y]
            .into_iter()
            .all(ok)
    })
}

// Uniformly magnifying the computation also magnifies the kernel's grid pitch and error budget
// back down in source coordinates. Translation keeps large absolute origins out of the grid.
fn scaled_outline(
    outline: Outline<'_>,
    style: &StrokeShape,
    profile: &WidthProfile,
    tolerance: f64,
    density: u32,
) -> Result<OffsetOutlines, VariableStrokeError> {
    let origin = outline
        .anchors
        .first()
        .ok_or(VariableStrokeError::ZeroLength)?
        .0;
    let scale = NATIVE_TOLERANCE_MM / (tolerance * 0.95);
    let coordinate = outline
        .anchors
        .iter()
        .fold(origin.x.abs().max(origin.y.abs()), |m, (p, a, b)| {
            [p.x, p.y, a.x, a.y, b.x, b.y]
                .into_iter()
                .fold(m, |m, value| m.max(value.abs()))
        })
        .max(style.width.as_mm().abs());
    if !scale.is_finite()
        || !coordinate.is_finite()
        || 128.0 * f64::EPSILON * coordinate > tolerance * 0.025
    {
        return Err(StrokeOutlineError::OutOfRange.into());
    }
    let point = |p: DocPoint| DocPoint::new((p.x - origin.x) * scale, (p.y - origin.y) * scale);
    let vector = |v: DocVec| DocVec::new(v.x * scale, v.y * scale);
    let anchors: Vec<_> = outline
        .anchors
        .iter()
        .map(|(p, a, b)| (point(*p), vector(*a), vector(*b)))
        .collect();
    if anchors.iter().any(|(p, a, b)| {
        [p.x, p.y, a.x, a.y, b.x, b.y]
            .iter()
            .any(|value| !value.is_finite() || value.abs() > MAX_COORDINATE_MM)
    }) {
        return Err(StrokeOutlineError::OutOfRange.into());
    }
    let mut enlarged = style.clone();
    enlarged.width = Length::from_mm(style.width.as_mm() * scale);
    if !enlarged.width.as_mm().is_finite() || enlarged.width.as_mm() > MAX_COORDINATE_MM {
        return Err(StrokeOutlineError::OutOfRange.into());
    }
    let result = outline_with_density(
        Outline::new(&anchors, outline.closed),
        &enlarged,
        profile,
        Tolerance::from_mm(NATIVE_TOLERANCE_MM),
        density,
    )?;
    let fallback = result.fallback_outlines();
    let mut rings = result.into_outlines();
    for anchor in rings.iter_mut().flatten() {
        anchor.point = DocPoint::new(
            anchor.point.x / scale + origin.x,
            anchor.point.y / scale + origin.y,
        );
        anchor.handle_in = DocVec::new(anchor.handle_in.x / scale, anchor.handle_in.y / scale);
        anchor.handle_out = DocVec::new(anchor.handle_out.x / scale, anchor.handle_out.y / scale);
        if !(anchor.point.x.is_finite() && anchor.point.y.is_finite())
            || anchor.point.x.abs().max(anchor.point.y.abs()) > MAX_COORDINATE_MM
        {
            return Err(StrokeOutlineError::OutOfRange.into());
        }
    }
    Ok(OffsetOutlines::from_parts(rings, fallback))
}

#[cfg(test)]
mod reference_tests;
