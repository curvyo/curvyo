//! The bands of a traced region found from its holes when its outer outline is no primitive
//! (`specs/0088-trace-recognition` Part B criterion 16, `adrs.md` decision 22): a stroke whose
//! joins are bevelled, or touching strokes whose outer outlines the trace merged into one.

// The formulas' own names (h, k, o, p, t, ...) read better than long ones here.
#![allow(clippy::many_single_char_names)]

use curvyo_document_core::{Angle, Length, LineJoin, Shape};

use crate::trace_shape_fit::Fitted;
use crate::trace_shape_join::{
    corners, distance, inside, offset, stroke_area, widened, within_miter_limit,
};
use crate::trace_shape_sample::Sampled;
use crate::trace_shapes::TraceBand;

/// The joins looked for, in the order they are tried: a mitred stroke before a bevelled one.
const JOINS: [LineJoin; 2] = [LineJoin::Miter, LineJoin::Bevel];

/// Halvings of a half width's search.
const STEPS: u32 = 60;

/// The fewest outer points a hole's band must explain.
const FEWEST_POINTS: usize = 8;

/// A stroke along a star or polygon: its centre line, half width and join.
struct Stroke {
    centre: Shape,
    half: f64,
    join: LineJoin,
}

impl Stroke {
    /// The stroke whose inner outline has the sides of `inner`, `half` wide on each side.
    fn of(inner: &Shape, half: f64, join: LineJoin) -> Option<Self> {
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
        self.join != LineJoin::Miter
            || corners(&self.centre).is_some_and(|c| within_miter_limit(&c))
    }

    fn band(&self) -> TraceBand {
        TraceBand {
            shape: self.centre,
            rotation: Angle::from_radians(0.0),
            width: Length::from_mm(2.0 * self.half),
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
/// when the outer is no primitive: the first join whose stroke covers the traced band's area
/// with every traced point within `tolerance` of its outlines (a bevelled stroke's outer outline
/// is cut at the tips, so it is no star).
pub(crate) fn band_from_hole(
    outer: &Sampled,
    hole: &Sampled,
    inner: &Fitted,
    tolerance: f64,
) -> Option<TraceBand> {
    corners(&inner.0)?;
    let traced = outer.area() - hole.area();
    let (o, h) = (absolute(outer), absolute(hole));
    // The widest stroke looked for has the band's area along the hole's length alone.
    let hi = traced / hole.perimeter;
    JOINS.iter().find_map(|&join| {
        let covers =
            |half: f64| Stroke::of(&inner.0, half, join).is_some_and(|s| s.area() >= traced);
        if !covers(hi) {
            return None;
        }
        let stroke = Stroke::of(&inner.0, search(hi, covers), join)?;
        (stroke.drawable()
            && all_within(&o, &stroke.outline(1.0), tolerance)
            && all_within(&h, &stroke.outline(-1.0), tolerance))
        .then(|| stroke.band())
    })
}

/// The median of `values` (empty: 0).
fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values.get(values.len() / 2).copied().unwrap_or(0.0)
}

/// The stroke along hole `inner` that `points` (the outer points nearest to it) lie on: for each
/// join the half width where as many of them lie outside its outer outline as inside, the join
/// whose outline they lie closer to.
fn stroke_through(points: &[[f64; 2]], inner: &Shape) -> Option<Stroke> {
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
/// areas add up to the traced region's within the tolerance along its outlines; `None` when the
/// region cannot be split into them exactly.
pub(crate) fn touching_bands(
    outer: &Sampled,
    holes: &[&Sampled],
    inners: &[&Fitted],
    tolerance: f64,
) -> Option<Vec<TraceBand>> {
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
    let explained = o
        .iter()
        .all(|&p| outers.iter().any(|ring| distance(p, ring) <= tolerance));
    let on_holes = strokes
        .iter()
        .zip(holes)
        .all(|(s, h)| s.drawable() && all_within(&absolute(h), &s.outline(-1.0), tolerance));
    let traced = outer.area() - holes.iter().map(|h| h.area()).sum::<f64>();
    let covered: f64 = strokes.iter().map(Stroke::area).sum();
    let length = outer.perimeter + holes.iter().map(|h| h.perimeter).sum::<f64>();
    (explained && on_holes && (covered - traced).abs() <= tolerance * length)
        .then(|| strokes.iter().map(Stroke::band).collect())
}
