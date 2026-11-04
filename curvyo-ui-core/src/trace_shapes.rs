//! The shape pass of Trace (`specs/0088-trace-recognition` Part B): which traced regions are
//! primitives within the picture-relative tolerance, written as a section of the worker's numbers
//! after the text section and read back.

use curvyo_document_core::{
    Angle, CornerRadii, Curve, EllipseFrame, InnerRatio, Length, Point, PointCount, RectBounds,
    Shape, StarFrame, drawing_outline_of_rotated,
};
use kurbo::BezPath;

use crate::svg_import_match::outline_path;
use crate::trace_shape_band::{band_of, stroked_area};
use crate::trace_shape_fit::{Fitted, fit_outline};
use crate::trace_shape_sample::{Sampled, sample_closed};
use crate::trace_transfer::count;

/// The number that opens the shape section (no count is negative; the text section uses -1 to -3
/// inside its own).
pub(crate) const SHAPE_MARK: f64 = -4.0;

/// The least tolerance, in picture pixels (`specs/0088` Frage 73: 0.2 mm at 10 px/mm; the rule of
/// `specs/0091` criterion 26).
pub const SHAPE_TOLERANCE_PX: f64 = 2.0;

/// The tolerance as a share of the square root of the outline's area (Proposal, `adrs.md`
/// Part B): large shapes may stray a little more than the pixel floor.
pub const SHAPE_TOLERANCE_SHARE: f64 = 0.01;

/// Numbers per outline in the section: the kind and six parameters.
const SHAPE_NUMBERS: usize = 7;

/// One recognised region of a trace result: its outer outline and its holes (indices into the
/// result's outlines, outer first) and each one's primitive, `None` for a hole that stays a path.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceShapeRegion {
    /// The outer outline, then its holes, as indices into the outlines.
    pub outlines: Vec<usize>,
    /// The primitive of each, with its rotation; the outer's is always there.
    pub shapes: Vec<Option<(Shape, Angle)>>,
    /// A band of constant width (one hole, outer and hole parallel offsets of one primitive,
    /// `specs/0088` Part B criterion 16): the centre-line primitive and the stroke width it is
    /// written as; `None` for a filled region.
    pub band: Option<TraceBand>,
}

/// The centre-line primitive of a band and its stroke width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceBand {
    /// The primitive at the band's centre line.
    pub shape: Shape,
    /// Its rotation about its frame's centre.
    pub rotation: Angle,
    /// The band's width: the stroke width.
    pub width: Length,
}

impl TraceShapeRegion {
    /// How many primitives it is written as: one for a band, else one per recognised outline.
    #[must_use]
    pub fn primitives(&self) -> usize {
        if self.band.is_some() {
            1
        } else {
            self.shapes.iter().filter(|s| s.is_some()).count()
        }
    }
}

/// The tolerance of an outline of `area` mm² in a picture of `pixel_mm` mm per pixel.
pub(crate) fn tolerance(area: f64, pixel_mm: f64) -> f64 {
    (SHAPE_TOLERANCE_PX * pixel_mm).max(SHAPE_TOLERANCE_SHARE * area.sqrt())
}

/// The recognised regions of `outlines` (document millimetres), layer by layer (`layers` the
/// outline count of each colour, empty for one colour), in a picture of `pixel_mm` mm per pixel.
/// A region is an outer outline and the holes inside it; the outer decides: one that matches no
/// primitive leaves its region as paths (`adrs.md` Part B, the hole rule).
#[must_use]
pub fn recognise_shapes(
    outlines: &[BezPath],
    layers: &[usize],
    pixel_mm: f64,
) -> Vec<TraceShapeRegion> {
    let all = [outlines.len()];
    let layers = if layers.is_empty() { &all[..] } else { layers };
    let mut regions = Vec::new();
    let mut at = 0;
    for &n in layers {
        let to = (at + n).min(outlines.len());
        recognise_layer(&outlines[at..to], at, pixel_mm, &mut regions);
        at = to;
    }
    regions
}

/// The regions of one layer's outlines (the first at index `first`).
fn recognise_layer(
    outlines: &[BezPath],
    first: usize,
    pixel_mm: f64,
    regions: &mut Vec<TraceShapeRegion>,
) {
    let sampled: Vec<Option<Sampled>> = outlines
        .iter()
        .map(|o| sample_closed(o, pixel_mm))
        .collect();
    // Outer outlines wind as the largest one does, holes against it.
    let Some(outer_sign) = sampled
        .iter()
        .flatten()
        .max_by(|a, b| a.area().total_cmp(&b.area()))
        .map(|s| s.signed_area > 0.0)
    else {
        return;
    };
    let is_outer = |s: &Sampled| (s.signed_area > 0.0) == outer_sign;
    let parents: Vec<Option<usize>> = sampled
        .iter()
        .map(|hole| {
            let hole = hole.as_ref().filter(|h| !is_outer(h))?;
            parent_of(hole, &sampled, &is_outer)
        })
        .collect();
    let fit = |s: &Sampled| fit_outline(s, tolerance(s.area(), pixel_mm));
    for (k, outer) in sampled.iter().enumerate() {
        let Some(outer) = outer.as_ref().filter(|s| is_outer(s)) else {
            continue;
        };
        let Some(shape) = fit(outer) else {
            continue;
        };
        let mut region = TraceShapeRegion {
            outlines: vec![first + k],
            shapes: vec![Some(shape)],
            band: None,
        };
        let mut holes = Vec::new();
        for (h, hole) in sampled.iter().enumerate() {
            if parents[h] == Some(k) {
                region.outlines.push(first + h);
                region.shapes.push(hole.as_ref().and_then(fit));
                holes.extend(hole.as_ref());
            }
        }
        if let ([Some(o), Some(i)], [hole]) = (&region.shapes[..], &holes[..]) {
            region.band = band(outer, hole, o, i, tolerance(outer.area(), pixel_mm));
        }
        regions.push(region);
    }
}

/// The band between the traced `outer` and its one `hole`, recognised as `o` and `i`: their
/// centre-line primitive when they are parallel offsets of one, and the stroked area it covers is
/// the traced band's within the tolerance along both outlines (the area guard of decision 16 for
/// the stroke).
fn band(outer: &Sampled, hole: &Sampled, o: &Fitted, i: &Fitted, tol: f64) -> Option<TraceBand> {
    let traced = outer.area() - hole.area();
    let ((shape, rotation), width) = band_of(o, i, traced, tol)?;
    let stroked = stroked_area(&shape, width);
    ((stroked - traced).abs() <= tol * (outer.perimeter + hole.perimeter)).then_some(TraceBand {
        shape,
        rotation,
        width: Length::from_mm(width),
    })
}

/// The smallest outer outline that holds `hole`.
fn parent_of(
    hole: &Sampled,
    sampled: &[Option<Sampled>],
    is_outer: &impl Fn(&Sampled) -> bool,
) -> Option<usize> {
    let first = [
        hole.centre[0] + hole.points[0][0],
        hole.centre[1] + hole.points[0][1],
    ];
    let [x0, y0, x1, y1] = hole.bounds;
    let mut best: Option<(usize, f64)> = None;
    for (k, outer) in sampled.iter().enumerate() {
        let Some(outer) = outer.as_ref().filter(|s| is_outer(s)) else {
            continue;
        };
        let [ox0, oy0, ox1, oy1] = outer.bounds;
        let around = ox0 <= x0 && oy0 <= y0 && ox1 >= x1 && oy1 >= y1;
        if around
            && outer.area() > hole.area()
            && best.is_none_or(|(_, area)| outer.area() < area)
            && outer.contains(first)
        {
            best = Some((k, outer.area()));
        }
    }
    best.map(|(k, _)| k)
}

/// The outline a primitive draws, as a path.
#[must_use]
pub fn primitive_path(shape: &Shape, rotation: Angle) -> BezPath {
    outline_path(&drawing_outline_of_rotated(shape, rotation))
}

/// [`primitive_path`] wound as `traced` is, so a hole stays a hole under the nonzero fill of a
/// compound path (`specs/0088` Part B: a primitive's outline always winds one way).
#[must_use]
pub fn primitive_path_like(shape: &Shape, rotation: Angle, traced: &BezPath) -> BezPath {
    let path = primitive_path(shape, rotation);
    if kurbo::Shape::area(&path) * kurbo::Shape::area(traced) < 0.0 {
        path.reverse_subpaths()
    } else {
        path
    }
}

/// `outlines` with each recognised outline of `regions` replaced by its primitive's, wound as the
/// traced one, and the regions kept. A region whose fill would change by more than its outlines'
/// own accepted area changes (a hole that no longer cuts, for any reason) keeps its traced paths
/// and is dropped: a hole is never lost.
pub(crate) fn with_primitives(
    mut outlines: Vec<BezPath>,
    regions: Vec<TraceShapeRegion>,
) -> (Vec<BezPath>, Vec<TraceShapeRegion>) {
    let mut kept = Vec::with_capacity(regions.len());
    for region in regions {
        let traced: Vec<&BezPath> = region.outlines.iter().map(|&i| &outlines[i]).collect();
        let drawn: Vec<BezPath> = region
            .outlines
            .iter()
            .zip(&region.shapes)
            .map(|(&i, shape)| match shape {
                Some((shape, rotation)) => primitive_path_like(shape, *rotation, &outlines[i]),
                None => outlines[i].clone(),
            })
            .collect();
        if !keeps_fill(&traced, &drawn) {
            continue;
        }
        for (&i, path) in region.outlines.iter().zip(drawn) {
            outlines[i] = path;
        }
        kept.push(region);
    }
    (outlines, kept)
}

/// Whether `drawn` fills what `traced` fills (outline for outline, nonzero): the net signed area
/// changes by no more than the outlines' own area changes add up to.
pub(crate) fn keeps_fill(traced: &[&BezPath], drawn: &[BezPath]) -> bool {
    let area = |p: &BezPath| kurbo::Shape::area(p);
    let before: f64 = traced.iter().map(|p| area(p)).sum();
    let after: f64 = drawn.iter().map(area).sum();
    let own: f64 = traced
        .iter()
        .zip(drawn)
        .map(|(t, d)| (area(t).abs() - area(d).abs()).abs())
        .sum();
    (after - before).abs() <= own + 1e-9 * before.abs().max(1.0)
}

/// The section of `regions` for the wire.
pub(crate) fn write_shapes(wire: &mut Vec<f64>, regions: &[TraceShapeRegion]) {
    #[allow(clippy::cast_precision_loss)] // counts far below 2^52
    {
        wire.extend([SHAPE_MARK, regions.len() as f64]);
        for region in regions {
            wire.push(region.outlines.len() as f64);
            wire.extend(region.outlines.iter().map(|&i| i as f64));
            for shape in &region.shapes {
                wire.extend(shape_numbers(shape.as_ref()));
            }
            // The band: its width (0 for none), then its centre-line primitive.
            match &region.band {
                Some(band) => {
                    wire.push(band.width.as_mm());
                    wire.extend(shape_numbers(Some(&(band.shape, band.rotation))));
                }
                None => wire.push(0.0),
            }
        }
    }
}

/// The kind and six parameters of one outline's primitive (kind 0: none).
fn shape_numbers(shape: Option<&Fitted>) -> [f64; SHAPE_NUMBERS] {
    let Some((shape, rotation)) = shape else {
        return [0.0; SHAPE_NUMBERS];
    };
    let turn = rotation.as_radians();
    match shape {
        Shape::Rect {
            bounds,
            corner_radii,
        } => {
            let (w, h) = (bounds.width.as_mm(), bounds.height.as_mm());
            [
                1.0,
                bounds.origin.x + w / 2.0,
                bounds.origin.y + h / 2.0,
                w,
                h,
                corner_radii.tl.as_mm(),
                turn,
            ]
        }
        Shape::Ellipse { frame, .. } => [
            2.0,
            frame.center.x,
            frame.center.y,
            frame.rx.as_mm(),
            frame.ry.as_mm(),
            turn,
            0.0,
        ],
        Shape::Polygon {
            frame, point_count, ..
        } => [
            3.0,
            frame.center.x,
            frame.center.y,
            frame.radius.as_mm(),
            frame.angle.as_radians(),
            f64::from(point_count.get()),
            0.0,
        ],
        Shape::Star {
            frame,
            point_count,
            inner_ratio,
            ..
        } => [
            4.0,
            frame.center.x,
            frame.center.y,
            frame.radius.as_mm(),
            frame.angle.as_radians(),
            f64::from(point_count.get()),
            inner_ratio.get(),
        ],
    }
}

/// Numbers of the shape section that are no primitive.
struct Malformed;

/// The primitive of one outline's numbers (`None` for kind 0).
fn shape_of(n: &[f64]) -> Result<Option<Fitted>, Malformed> {
    if n.iter().any(|v| !v.is_finite()) {
        return Err(Malformed);
    }
    let zero = Angle::from_radians(0.0);
    let star_frame = || StarFrame {
        center: Point::new(n[1], n[2]),
        radius: Length::from_mm(n[3]),
        angle: Angle::from_radians(n[4]),
    };
    let points = || PointCount::new(u32::try_from(count(n[5])?).ok()?).ok();
    let shape = match count(n[0]).ok_or(Malformed)? {
        0 => return Ok(None),
        1 => (
            Shape::Rect {
                bounds: RectBounds {
                    origin: Point::new(n[1] - n[3] / 2.0, n[2] - n[4] / 2.0),
                    width: Length::from_mm(n[3]),
                    height: Length::from_mm(n[4]),
                },
                corner_radii: CornerRadii::uniform(Length::from_mm(n[5])),
            },
            Angle::from_radians(n[6]),
        ),
        2 => (
            Shape::Ellipse {
                frame: EllipseFrame {
                    center: Point::new(n[1], n[2]),
                    rx: Length::from_mm(n[3]),
                    ry: Length::from_mm(n[4]),
                },
                arc: None,
                curve: Curve::ELLIPSE_DEFAULT,
            },
            Angle::from_radians(n[5]),
        ),
        3 => (
            Shape::Polygon {
                frame: star_frame(),
                point_count: points().ok_or(Malformed)?,
                curve: Curve::POLYGON_DEFAULT,
            },
            zero,
        ),
        4 => (
            Shape::Star {
                frame: star_frame(),
                point_count: points().ok_or(Malformed)?,
                inner_ratio: InnerRatio::new(n[6]).map_err(|_| Malformed)?,
                curve: Curve::POLYGON_DEFAULT,
            },
            zero,
        ),
        _ => return Err(Malformed),
    };
    Ok(Some(shape))
}

/// The shape section at the start of `tail` (it starts with [`SHAPE_MARK`]) for a result of
/// `outlines` outlines and the numbers it took; `None` unless it is well formed.
pub(crate) fn read_shapes(tail: &[f64], outlines: usize) -> Option<(Vec<TraceShapeRegion>, usize)> {
    let mut at = 1;
    let mut next = || -> Option<f64> {
        let value = *tail.get(at)?;
        at += 1;
        Some(value)
    };
    let regions = count(next()?)?;
    let mut out = Vec::with_capacity(regions.min(outlines));
    for _ in 0..regions {
        let k = count(next()?)?;
        if k == 0 || k > outlines {
            return None;
        }
        let indices: Vec<usize> = (0..k)
            .map(|_| count(next()?).filter(|&i| i < outlines))
            .collect::<Option<_>>()?;
        let mut shapes = Vec::with_capacity(k);
        for _ in 0..k {
            let numbers: Vec<f64> = (0..SHAPE_NUMBERS).map(|_| next()).collect::<Option<_>>()?;
            shapes.push(shape_of(&numbers).ok()?);
        }
        shapes.first()?.as_ref()?;
        let width = next()?;
        let band = if width == 0.0 {
            None
        } else {
            let numbers: Vec<f64> = (0..SHAPE_NUMBERS).map(|_| next()).collect::<Option<_>>()?;
            let (shape, rotation) = shape_of(&numbers).ok()??;
            if !(width > 0.0 && width.is_finite() && k == 2) {
                return None;
            }
            Some(TraceBand {
                shape,
                rotation,
                width: Length::from_mm(width),
            })
        };
        if indices
            .iter()
            .enumerate()
            .any(|(k, i)| indices[..k].contains(i))
        {
            return None;
        }
        out.push(TraceShapeRegion {
            outlines: indices,
            shapes,
            band,
        });
    }
    Some((out, at))
}

/// Adds the shape section to a trace result `wire` of a picture mapped by `pixel_map` (the
/// worker's numbers of `trace_job_with`, `trace_colour_job_with` or the text jobs), after its text
/// section; a result that was not traced is left as it is.
pub fn trace_add_shapes(wire: &mut Vec<f64>, pixel_map: [f64; 6]) {
    let Some((read, at)) = crate::trace_transfer::read_trace_body(wire) else {
        return;
    };
    if read.status != crate::TraceStatus::Traced || wire.get(at) == Some(&SHAPE_MARK) {
        return;
    }
    let [a, b, c, d, _, _] = pixel_map;
    let pixel_mm = (a * d - b * c).abs().sqrt();
    if !(pixel_mm > 0.0 && pixel_mm.is_finite()) {
        return;
    }
    let layers: Vec<usize> = read.layers.iter().map(|l| l.outlines).collect();
    let regions = recognise_shapes(&read.outlines, &layers, pixel_mm);
    let mut section = Vec::new();
    write_shapes(&mut section, &regions);
    wire.splice(at..at, section);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64, side: f64, clockwise: bool) -> BezPath {
        let mut p = BezPath::new();
        let c = [(x, x), (x + side, x), (x + side, x + side), (x, x + side)];
        let order: Vec<_> = if clockwise {
            c.to_vec()
        } else {
            c.iter().rev().copied().collect()
        };
        p.move_to(order[0]);
        for q in &order[1..] {
            p.line_to(*q);
        }
        p.close_path();
        p
    }

    /// A hole drawn the outer's way round would fill the frame: that is not the same fill.
    #[test]
    fn a_hole_that_no_longer_cuts_is_not_the_same_fill() {
        let (outer, hole) = (square(0.0, 10.0, true), square(2.0, 6.0, false));
        let traced = [&outer, &hole];
        assert!(keeps_fill(&traced, &[outer.clone(), hole.clone()]));
        assert!(keeps_fill(
            &traced,
            &[square(0.0, 10.01, true), square(2.0, 5.99, false)]
        ));
        assert!(!keeps_fill(
            &traced,
            &[outer.clone(), square(2.0, 6.0, true)]
        ));
    }
}
