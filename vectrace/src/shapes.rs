//! The shape pass of Trace (`specs/0088-trace-recognition` Part B): which traced regions are
//! primitives within the picture-relative tolerance, written as a section of the worker's numbers
//! after the text section and read back.

pub mod band;
pub mod fit;
pub mod hole_band;
pub mod join;
pub mod math;
pub mod sample;
pub mod star;

use std::ops::RangeInclusive;

use crate::{Angle, Mm};
use kurbo::{BezPath, Point};

use crate::shapes::band::{band_of, stroked_area};
use crate::shapes::fit::{Fitted, fit_outline};
use crate::shapes::hole_band::{BAND_AREA_SHARE, band_from_hole, touching_bands};
use crate::shapes::sample::{Sampled, sample_closed};
use crate::wire::count;

/// The number that opens the shape section (no count is negative; the text section uses -1 to -3
/// inside its own).
pub(crate) const SHAPE_MARK: f64 = -4.0;

/// The least tolerance, in picture pixels (`specs/0088` Frage 73: 0.2 mm at 10 px/mm; the rule of
/// `specs/0091` criterion 26).
pub(crate) const SHAPE_TOLERANCE_PX: f64 = 2.0;

/// The tolerance as a share of the square root of the outline's area (Proposal, `adrs.md`
/// Part B): large shapes may stray a little more than the pixel floor.
pub(crate) const SHAPE_TOLERANCE_SHARE: f64 = 0.01;

/// Numbers per outline in the section: the kind and six parameters.
const SHAPE_NUMBERS: usize = 7;

/// A primitive the shape pass recognised, as the wire carries it: plain numbers in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Primitive {
    /// A rectangle with one corner radius, before its rotation.
    Rect {
        /// Its top-left corner before the rotation.
        origin: Point,
        /// Its width.
        width: Mm,
        /// Its height.
        height: Mm,
        /// The radius of all four corners.
        corner_radius: Mm,
    },
    /// An ellipse (a circle when the radii are equal), before its rotation.
    Ellipse {
        /// Its centre.
        centre: Point,
        /// The radius along its x axis.
        rx: Mm,
        /// The radius along its y axis.
        ry: Mm,
    },
    /// A regular polygon.
    Polygon {
        /// Its centre.
        centre: Point,
        /// The distance of a corner from the centre.
        radius: Mm,
        /// Where the first corner points, from the x axis.
        angle: Angle,
        /// The corners, in [`POINTS`].
        points: u32,
    },
    /// A star.
    Star {
        /// Its centre.
        centre: Point,
        /// The distance of a tip from the centre.
        radius: Mm,
        /// Where the first tip points, from the x axis.
        angle: Angle,
        /// The tips, in [`POINTS`].
        points: u32,
        /// The valley radius over the tip radius, in the open interval (0, 1).
        inner_ratio: f64,
    },
}

/// The corners of a polygon and the tips of a star (the document's `PointCount`, tested).
pub const POINTS: RangeInclusive<u32> = 3..=1024;

/// The inner ratio of a star when it lies in the open interval (0, 1).
#[must_use]
pub fn valid_ratio(ratio: f64) -> Option<f64> {
    (ratio.is_finite() && ratio > 0.0 && ratio < 1.0).then_some(ratio)
}

/// How the corners of a stroke are joined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Join {
    /// Pointed to the miter limit.
    Miter,
    /// Rounded.
    Round,
    /// Cut off.
    Bevel,
}

/// One recognised region of a trace result: its outer outline and its holes (indices into the
/// result's outlines, outer first) and each one's primitive, `None` for a hole that stays a path.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeRegion {
    /// The outer outline, then its holes, as indices into the outlines.
    pub outlines: Vec<usize>,
    /// The primitive of each, with its rotation; the outer's is there unless the region is
    /// bands (a bevelled stroke's or touching strokes' outer outline is no primitive).
    pub shapes: Vec<Option<(Primitive, Angle)>>,
    /// A band of constant width (one hole, outer and hole parallel offsets of one primitive,
    /// `specs/0088` Part B criterion 16): the centre-line primitive and the stroke width it is
    /// written as; `None` for a filled region.
    pub band: Option<Band>,
    /// Touching bands whose outer outlines the trace merged into the region's one (`adrs.md`
    /// decision 22): one stroked primitive per hole; empty otherwise.
    pub bands: Vec<Band>,
}

/// The centre-line primitive of a band and its stroke width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    /// The primitive at the band's centre line.
    pub primitive: Primitive,
    /// Its rotation about its frame's centre.
    pub rotation: Angle,
    /// The band's width: the stroke width.
    pub width: Mm,
    /// The join the traced corners show (`adrs.md` decision 22): mitred or bevelled.
    pub join: Join,
}

impl ShapeRegion {
    /// How many primitives it is written as: one per band, else one per recognised outline.
    #[must_use]
    pub fn primitives(&self) -> usize {
        if self.band.is_some() {
            1
        } else if !self.bands.is_empty() {
            self.bands.len()
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
pub fn recognise_shapes(outlines: &[BezPath], layers: &[usize], pixel_mm: f64) -> Vec<ShapeRegion> {
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
    regions: &mut Vec<ShapeRegion>,
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
        let holes: Vec<(usize, &Sampled)> = sampled
            .iter()
            .enumerate()
            .filter(|&(h, _)| parents[h] == Some(k))
            .filter_map(|(h, s)| Some((h, s.as_ref()?)))
            .collect();
        let shape = fit(outer);
        if shape.is_none() && holes.is_empty() {
            continue;
        }
        let mut region = ShapeRegion {
            outlines: vec![first + k],
            shapes: vec![shape],
            band: None,
            bands: Vec::new(),
        };
        for &(h, hole) in &holes {
            region.outlines.push(first + h);
            region.shapes.push(fit(hole));
        }
        let tol = tolerance(outer.area(), pixel_mm);
        let traced_holes: Vec<&Sampled> = holes.iter().map(|&(_, s)| s).collect();
        match (&region.shapes[..], &traced_holes[..]) {
            ([o, Some(i)], [hole]) => {
                region.band = o
                    .as_ref()
                    .and_then(|o| band(outer, hole, o, i, tol))
                    .or_else(|| band_from_hole(outer, hole, i, tol));
            }
            ([None, inners @ ..], _) if traced_holes.len() >= 2 => {
                if let Some(inners) = inners
                    .iter()
                    .map(Option::as_ref)
                    .collect::<Option<Vec<_>>>()
                {
                    region.bands =
                        touching_bands(outer, &traced_holes, &inners, tol).unwrap_or_default();
                }
            }
            _ => {}
        }
        if region.shapes[0].is_some() || region.band.is_some() || !region.bands.is_empty() {
            regions.push(region);
        }
    }
}

/// The band between the traced `outer` and its one `hole`, recognised as `o` and `i`: their
/// centre-line primitive when they are parallel offsets of one, and the stroked area it covers is
/// the traced band's within [`BAND_AREA_SHARE`] (the area guard of decision 16 for the stroke).
fn band(outer: &Sampled, hole: &Sampled, o: &Fitted, i: &Fitted, tol: f64) -> Option<Band> {
    let traced = outer.area() - hole.area();
    let ((primitive, rotation), width) = band_of(o, i, traced, tol)?;
    let stroked = stroked_area(&primitive, width);
    ((stroked - traced).abs() <= BAND_AREA_SHARE * traced).then_some(Band {
        primitive,
        rotation,
        width: Mm::from_mm(width),
        join: Join::Miter,
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

/// The section of `regions` for the wire.
pub(crate) fn write_shapes(wire: &mut Vec<f64>, regions: &[ShapeRegion]) {
    #[allow(clippy::cast_precision_loss)] // counts far below 2^52
    {
        wire.extend([SHAPE_MARK, regions.len() as f64]);
        for region in regions {
            wire.push(region.outlines.len() as f64);
            wire.extend(region.outlines.iter().map(|&i| i as f64));
            for shape in &region.shapes {
                wire.extend(shape_numbers(shape.as_ref()));
            }
            // The band: its width (0 for none), its join and its centre-line primitive; touching
            // bands: minus their count, then each band so.
            match (&region.band, &region.bands[..]) {
                (Some(band), _) => write_band(wire, band),
                (None, []) => wire.push(0.0),
                (None, bands) => {
                    wire.push(-(bands.len() as f64));
                    for band in bands {
                        write_band(wire, band);
                    }
                }
            }
        }
    }
}

/// The join's number on the wire.
fn join_number(join: Join) -> f64 {
    match join {
        Join::Miter => 0.0,
        Join::Bevel => 1.0,
        Join::Round => 2.0,
    }
}

/// One band's width, join and centre-line primitive.
fn write_band(wire: &mut Vec<f64>, band: &Band) {
    wire.push(band.width.as_mm());
    wire.push(join_number(band.join));
    wire.extend(shape_numbers(Some(&(band.primitive, band.rotation))));
}

/// The kind and six parameters of one outline's primitive (kind 0: none).
fn shape_numbers(shape: Option<&Fitted>) -> [f64; SHAPE_NUMBERS] {
    let Some((shape, rotation)) = shape else {
        return [0.0; SHAPE_NUMBERS];
    };
    let turn = rotation.as_radians();
    match shape {
        Primitive::Rect {
            origin,
            width,
            height,
            corner_radius,
        } => {
            let (w, h) = (width.as_mm(), height.as_mm());
            [
                1.0,
                origin.x + w / 2.0,
                origin.y + h / 2.0,
                w,
                h,
                corner_radius.as_mm(),
                turn,
            ]
        }
        Primitive::Ellipse { centre, rx, ry } => {
            [2.0, centre.x, centre.y, rx.as_mm(), ry.as_mm(), turn, 0.0]
        }
        Primitive::Polygon {
            centre,
            radius,
            angle,
            points,
        } => [
            3.0,
            centre.x,
            centre.y,
            radius.as_mm(),
            angle.as_radians(),
            f64::from(*points),
            0.0,
        ],
        Primitive::Star {
            centre,
            radius,
            angle,
            points,
            inner_ratio,
        } => [
            4.0,
            centre.x,
            centre.y,
            radius.as_mm(),
            angle.as_radians(),
            f64::from(*points),
            *inner_ratio,
        ],
    }
}

/// Numbers of the shape section that are no primitive.
struct Malformed;

/// The primitive of one outline's numbers (`None` for kind 0).
fn primitive_of(n: &[f64]) -> Result<Option<Fitted>, Malformed> {
    if n.iter().any(|v| !v.is_finite()) {
        return Err(Malformed);
    }
    let zero = Angle::from_radians(0.0);
    let points = || {
        u32::try_from(count(n[5])?)
            .ok()
            .filter(|p| POINTS.contains(p))
    };
    let shape = match count(n[0]).ok_or(Malformed)? {
        0 => return Ok(None),
        1 => (
            Primitive::Rect {
                origin: Point::new(n[1] - n[3] / 2.0, n[2] - n[4] / 2.0),
                width: Mm::from_mm(n[3]),
                height: Mm::from_mm(n[4]),
                corner_radius: Mm::from_mm(n[5]),
            },
            Angle::from_radians(n[6]),
        ),
        2 => (
            Primitive::Ellipse {
                centre: Point::new(n[1], n[2]),
                rx: Mm::from_mm(n[3]),
                ry: Mm::from_mm(n[4]),
            },
            Angle::from_radians(n[5]),
        ),
        3 => (
            Primitive::Polygon {
                centre: Point::new(n[1], n[2]),
                radius: Mm::from_mm(n[3]),
                angle: Angle::from_radians(n[4]),
                points: points().ok_or(Malformed)?,
            },
            zero,
        ),
        4 => (
            Primitive::Star {
                centre: Point::new(n[1], n[2]),
                radius: Mm::from_mm(n[3]),
                angle: Angle::from_radians(n[4]),
                points: points().ok_or(Malformed)?,
                inner_ratio: valid_ratio(n[6]).ok_or(Malformed)?,
            },
            zero,
        ),
        _ => return Err(Malformed),
    };
    Ok(Some(shape))
}

/// The shape section at the start of `tail` (it starts with [`SHAPE_MARK`]) for a result of
/// `outlines` outlines and the numbers it took; `None` unless it is well formed.
pub(crate) fn read_shapes(tail: &[f64], outlines: usize) -> Option<(Vec<ShapeRegion>, usize)> {
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
            shapes.push(primitive_of(&numbers).ok()?);
        }
        let width = next()?;
        let (band, bands) = if width == 0.0 {
            (None, Vec::new())
        } else if width > 0.0 {
            if k != 2 {
                return None;
            }
            (Some(read_band(&mut next, width)?), Vec::new())
        } else {
            // Touching bands: one per hole.
            let m = count(-width).filter(|&m| m + 1 == k && m >= 2)?;
            let bands = (0..m)
                .map(|_| {
                    let width = next()?;
                    read_band(&mut next, width)
                })
                .collect::<Option<Vec<_>>>()?;
            (None, bands)
        };
        if shapes.first()?.is_none() && band.is_none() && bands.is_empty() {
            return None;
        }
        if indices
            .iter()
            .enumerate()
            .any(|(k, i)| indices[..k].contains(i))
        {
            return None;
        }
        out.push(ShapeRegion {
            outlines: indices,
            shapes,
            band,
            bands,
        });
    }
    Some((out, at))
}

/// One band after its width: its join and centre-line primitive.
fn read_band(next: &mut impl FnMut() -> Option<f64>, width: f64) -> Option<Band> {
    let join = match count(next()?)? {
        0 => Join::Miter,
        1 => Join::Bevel,
        2 => Join::Round,
        _ => return None,
    };
    let numbers: Vec<f64> = (0..SHAPE_NUMBERS).map(|_| next()).collect::<Option<_>>()?;
    let (primitive, rotation) = primitive_of(&numbers).ok()??;
    (width > 0.0 && width.is_finite()).then_some(Band {
        primitive,
        rotation,
        width: Mm::from_mm(width),
        join,
    })
}

/// Adds the shape section to a trace result `wire` of a picture mapped by `pixel_map` (the
/// worker's numbers of `trace_job_in`, `trace_colour_job_in` or the text jobs), after its text
/// section; a result that was not traced is left as it is.
pub fn trace_add_shapes(wire: &mut Vec<f64>, pixel_map: [f64; 6]) {
    let Some((read, at)) = crate::wire::read_trace_body(wire) else {
        return;
    };
    if read.status != crate::wire::Status::Traced || wire.get(at) == Some(&SHAPE_MARK) {
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
