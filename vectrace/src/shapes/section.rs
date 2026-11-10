//! The shape section of the wire: the recognised regions as numbers after a result's text
//! section, and read back (`specs/0088-trace-recognition`).

use super::{Band, Join, POINTS, Primitive, ShapeRegion, valid_ratio};
use crate::shapes::fit::Fitted;
use crate::wire::count;
use crate::{Angle, Mm};
use kurbo::Point;

/// The number that opens the shape section (no count is negative; the text section uses -1 to -3
/// inside its own).
pub(crate) const SHAPE_MARK: f64 = -4.0;

/// Numbers per outline in the section: the kind and six parameters.
const SHAPE_NUMBERS: usize = 7;

/// The section of `regions` for the wire.
pub(super) fn write_shapes(wire: &mut Vec<f64>, regions: &[ShapeRegion]) {
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
