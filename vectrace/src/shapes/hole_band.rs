//! The bands of a traced region found from its holes when its outer outline is no primitive
//! (`specs/0088-trace-recognition` Part B criterion 16, `adrs.md` decision 22): a stroke whose
//! joins are bevelled, or touching strokes whose outer outlines the trace merged into one.

// The formulas' own names (h, k, o, p, t, ...) read better than long ones here.
#![allow(clippy::many_single_char_names)]

use crate::{Angle, Mm};

use crate::shapes::fit::Fitted;
use crate::shapes::join::{
    corners, distance, inside, offset, stroke_area, widened, within_miter_limit,
};
use crate::shapes::sample::Sampled;
use crate::shapes::{Band, Join, Primitive};

/// The joins looked for; on a tie the earlier one wins.
const JOINS: [Join; 3] = [Join::Miter, Join::Bevel, Join::Round];

/// The radius about each corner of the centre line, in band widths, whose traced points only
/// choose the join and are left out of the deviation check (customer 2026-11-04: the tracer
/// rounds, flattens or clips the tips; Proposal, calibrated on his ten-star page).
const TIP_ZONE: f64 = 1.5;

/// The most a band's stroked area may differ from the traced band's, as a share of it (Proposal).
pub(crate) const BAND_AREA_SHARE: f64 = 0.03;

/// Halvings of a half width's search.
const STEPS: u32 = 60;

/// The fewest outer points a hole's band must explain.
const FEWEST_POINTS: usize = 8;

/// A stroke along a star or polygon: its centre line, half width and join.
struct Stroke {
    centre: Primitive,
    half: f64,
    join: Join,
}

impl Stroke {
    /// The stroke whose inner outline has the sides of `inner`, `half` wide on each side.
    fn of(inner: &Primitive, half: f64, join: Join) -> Option<Self> {
        let centre = widened(inner, half)?;
        Some(Self { centre, half, join })
    }

    /// Its outer (`side` 1) or inner (`side` −1) outline.
    fn outline(&self, side: f64) -> Vec<[f64; 2]> {
        corners(&self.centre).map_or_else(Vec::new, |c| offset(&c, side * self.half, self.join))
    }

    fn area(&self) -> f64 {
        corners(&self.centre).map_or(0.0, |c| stroke_area(&c, 2.0 * self.half, self.join))
    }

    /// Whether the renderer draws it as it is (a miter longer than the limit is bevelled).
    fn drawable(&self) -> bool {
        self.join != Join::Miter || corners(&self.centre).is_some_and(|c| within_miter_limit(&c))
    }

    fn band(&self) -> Band {
        Band {
            primitive: self.centre,
            rotation: Angle::from_radians(0.0),
            width: Mm::from_mm(2.0 * self.half),
            join: self.join,
        }
    }
}

/// The points of `s` in document millimetres.
fn absolute(s: &Sampled) -> Vec<[f64; 2]> {
    s.points
        .iter()
        .map(|p| [p[0] + s.centre[0], p[1] + s.centre[1]])
        .collect()
}

/// Whether every point is within `tolerance` of `outline`.
fn all_within(points: &[[f64; 2]], outline: &[[f64; 2]], tolerance: f64) -> bool {
    !outline.is_empty() && points.iter().all(|&p| distance(p, outline) <= tolerance)
}

/// `points` split into those within the tip zone of a corner of `stroke`'s centre line (at most a
/// quarter of its shortest side) and those on its sides.
fn by_zone(points: &[[f64; 2]], stroke: &Stroke) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
    let corners = corners(&stroke.centre).unwrap_or_default();
    // At most a quarter of the shortest side, so half of every side decides.
    let shortest = (0..corners.len())
        .map(|i| {
            let (a, b) = (corners[i], corners[(i + 1) % corners.len()]);
            crate::shapes::sample::length(b[0] - a[0], b[1] - a[1])
        })
        .fold(f64::INFINITY, f64::min);
    let zone = (TIP_ZONE * 2.0 * stroke.half).min(shortest / 4.0);
    points.iter().partition(|p| {
        corners
            .iter()
            .any(|c| crate::shapes::sample::length(p[0] - c[0], p[1] - c[1]) <= zone)
    })
}

/// How far, as a root mean square, `points` lie from `outline` (0 for none).
fn spread(points: &[[f64; 2]], outline: &[[f64; 2]]) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)] // point counts far below 2^52
    let n = points.len() as f64;
    (points
        .iter()
        .map(|&p| distance(p, outline).powi(2))
        .sum::<f64>()
        / n)
        .sqrt()
}

/// Whether the side points of `outer` and `hole` lie on `stroke`'s outlines within `tolerance`
/// (enough of them), and how far its tip-zone points lie from them.
fn sides_fit(
    stroke: &Stroke,
    outer: &[[f64; 2]],
    hole: &[[f64; 2]],
    tolerance: f64,
) -> Option<f64> {
    let (out_line, in_line) = (stroke.outline(1.0), stroke.outline(-1.0));
    let (outer_tips, outer_sides) = by_zone(outer, stroke);
    let (hole_tips, hole_sides) = by_zone(hole, stroke);
    (stroke.drawable()
        && outer_sides.len() >= FEWEST_POINTS
        && hole_sides.len() >= FEWEST_POINTS
        && all_within(&outer_sides, &out_line, tolerance)
        && all_within(&hole_sides, &in_line, tolerance))
    .then(|| spread(&outer_tips, &out_line).max(spread(&hole_tips, &in_line)))
}

/// The half width in (0, `hi`] where `rises` turns true, by halving (`rises` false at 0).
fn search(hi: f64, rises: impl Fn(f64) -> bool) -> f64 {
    let (mut lo, mut hi) = (0.0, hi);
    for _ in 0..STEPS {
        let mid = f64::midpoint(lo, hi);
        if rises(mid) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    f64::midpoint(lo, hi)
}

/// The band of the traced `outer` and its one `hole`, recognised as `inner` (a star or polygon),
/// when the outer is no primitive: for each join the width whose stroke covers the traced band's
/// area, kept when the sides lie on its outlines within `tolerance` (the tips are left to the
/// join), the join whose outlines the tips lie closest to.
pub(crate) fn band_from_hole(
    outer: &Sampled,
    hole: &Sampled,
    inner: &Fitted,
    tolerance: f64,
) -> Option<Band> {
    corners(&inner.0)?;
    let traced = outer.area() - hole.area();
    let (o, h) = (absolute(outer), absolute(hole));
    // The widest stroke looked for has the band's area along the hole's length alone.
    let hi = traced / hole.perimeter;
    let mut best: Option<(f64, Stroke)> = None;
    for join in JOINS {
        let covers =
            |half: f64| Stroke::of(&inner.0, half, join).is_some_and(|s| s.area() >= traced);
        if !covers(hi) {
            continue;
        }
        let Some(stroke) = Stroke::of(&inner.0, search(hi, covers), join) else {
            continue;
        };
        let Some(tips) = sides_fit(&stroke, &o, &h, tolerance) else {
            continue;
        };
        if best.as_ref().is_none_or(|(b, _)| tips < *b) {
            best = Some((tips, stroke));
        }
    }
    let (_, stroke) = best?;
    ((stroke.area() - traced).abs() <= BAND_AREA_SHARE * traced).then(|| stroke.band())
}

/// The median of `values` (empty: 0).
fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values.get(values.len() / 2).copied().unwrap_or(0.0)
}

/// The stroke along hole `inner` that `points` (the outer points nearest to it) lie on: for each
/// join the half width where as many of them lie outside its outer outline as inside, the join
/// whose outline they lie closer to.
fn stroke_through(points: &[[f64; 2]], inner: &Primitive) -> Option<Stroke> {
    let outline = corners(inner)?;
    let hi = median(points.iter().map(|&p| distance(p, &outline)).collect());
    let mut best: Option<(f64, Stroke)> = None;
    for join in JOINS {
        let signed_median = |half: f64| {
            Stroke::of(inner, half, join).map_or(f64::INFINITY, |s| {
                let o = s.outline(1.0);
                median(
                    points
                        .iter()
                        .map(|&p| {
                            let d = distance(p, &o);
                            if inside(p, &o) { -d } else { d }
                        })
                        .collect(),
                )
            })
        };
        let stroke = Stroke::of(inner, search(hi, |half| signed_median(half) <= 0.0), join)?;
        let o = stroke.outline(1.0);
        let spread: f64 = points.iter().map(|&p| distance(p, &o).powi(2)).sum();
        if best.as_ref().is_none_or(|(s, _)| spread < *s) {
            best = Some((spread, stroke));
        }
    }
    best.map(|(_, stroke)| stroke)
}

/// The bands of touching strokes whose outer outlines the trace merged into `outer`, one per
/// hole (each recognised in `inners` as a star or polygon), when every outer point lies on one of
/// their outer outlines within `tolerance`, every hole on its stroke's inner outline, and their
/// areas add up to the traced region's within [`BAND_AREA_SHARE`]; `None` when the
/// region cannot be split into them exactly.
pub(crate) fn touching_bands(
    outer: &Sampled,
    holes: &[&Sampled],
    inners: &[&Fitted],
    tolerance: f64,
) -> Option<Vec<Band>> {
    let insides: Vec<Vec<[f64; 2]>> = inners
        .iter()
        .map(|i| corners(&i.0))
        .collect::<Option<_>>()?;
    let o = absolute(outer);
    let mut nearest: Vec<Vec<[f64; 2]>> = vec![Vec::new(); insides.len()];
    for &p in &o {
        let k = (0..insides.len())
            .min_by(|&a, &b| distance(p, &insides[a]).total_cmp(&distance(p, &insides[b])))?;
        nearest[k].push(p);
    }
    let strokes: Vec<Stroke> = nearest
        .iter()
        .zip(inners)
        .map(|(points, inner)| {
            (points.len() >= FEWEST_POINTS)
                .then(|| stroke_through(points, &inner.0))
                .flatten()
        })
        .collect::<Option<_>>()?;
    let outers: Vec<Vec<[f64; 2]>> = strokes.iter().map(|s| s.outline(1.0)).collect();
    // A point in a tip zone is left to the join (decision 23).
    let in_tip = |p: [f64; 2]| strokes.iter().any(|s| by_zone(&[p], s).0.len() == 1);
    let explained = o
        .iter()
        .filter(|&&p| !in_tip(p))
        .all(|&p| outers.iter().any(|ring| distance(p, ring) <= tolerance));
    let on_holes = strokes.iter().zip(holes).all(|(s, h)| {
        s.drawable() && all_within(&by_zone(&absolute(h), s).1, &s.outline(-1.0), tolerance)
    });
    let traced = outer.area() - holes.iter().map(|h| h.area()).sum::<f64>();
    let covered: f64 = strokes.iter().map(Stroke::area).sum();
    (explained && on_holes && (covered - traced).abs() <= BAND_AREA_SHARE * traced)
        .then(|| strokes.iter().map(Stroke::band).collect())
}
