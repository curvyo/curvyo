//! The stretches of the inner face under the slit bands, one mark per stretch of one tightness
//! (ADR 0083 §13.12, §13.16).

use curvyo_extension_core::{Cubic, Mark};

use crate::footprint::Shape;
use crate::outline::Builder;
use crate::shape_ops::{between, length, start, turn};
use crate::wall_bands::BandSpec;

/// The marks of the bands of one piece and the tightest arc under them.
pub(crate) struct Stretches {
    pub(crate) marks: Vec<Mark>,
    /// The least `radius / limit` among the tight arcs, with that radius and limit.
    pub(crate) tightest: Option<(f64, f64)>,
}

fn cubics(parts: &[Shape]) -> Vec<Cubic> {
    let Some(first) = parts.first() else {
        return Vec::new();
    };
    let mut builder = Builder::new(start(first));
    for part in parts {
        if let Shape::Bend {
            centre,
            radius,
            from,
            sweep,
        } = *part
        {
            builder.arc(centre, radius, from, from + sweep);
        }
    }
    builder.segments()
}

/// The bend marks of `specs` for a wall of thickness `wall`: the stretch of every band, split
/// where the tightness changes (a radius below twice the wall thickness convex, once concave).
pub(crate) fn bend_marks(specs: &[BandSpec], wall: f64) -> Stretches {
    let mut out = Stretches {
        marks: Vec::new(),
        tightest: None,
    };
    for spec in specs {
        let (from, to) = (spec.band.from.as_mm(), spec.band.to.as_mm());
        let mut group: Vec<Shape> = Vec::new();
        let mut group_tight = false;
        let flush = |group: &mut Vec<Shape>, tight: bool, marks: &mut Vec<Mark>| {
            if let Some(first) = group.first() {
                marks.push(Mark::Bend {
                    start: start(first),
                    segments: cubics(group),
                    tight,
                });
            }
            group.clear();
        };
        for (&(s0, s1, radius), shape) in spec.arcs.iter().zip(&spec.shapes) {
            let (a, b) = (s0.max(from), s1.min(to));
            if b - a <= 1e-9 || s1 - s0 <= 1e-12 {
                continue;
            }
            let inner = length(shape);
            let part = between(
                shape,
                (a - s0) / (s1 - s0) * inner,
                (b - s0) / (s1 - s0) * inner,
            );
            let limit = if turn(shape) > 0.0 { 2.0 * wall } else { wall };
            let tight = radius < limit - 1e-9;
            if tight && out.tightest.is_none_or(|(r, l)| radius / limit < r / l) {
                out.tightest = Some((radius, limit));
            }
            if !group.is_empty() && tight != group_tight {
                flush(&mut group, group_tight, &mut out.marks);
            }
            group_tight = tight;
            group.push(part);
        }
        flush(&mut group, group_tight, &mut out.marks);
    }
    out
}
