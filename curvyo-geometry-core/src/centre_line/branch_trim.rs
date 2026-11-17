//! The start of a branch (ADR 0063 §4, branch start, amended 2026-11-20): where its butt end's
//! farther corner (nearer corner for a gap) lies `(0.5 - p) w` from the parent's fitted centre
//! curve, inside the junction zone along the straight extension of its edge measure.

use kurbo::{Point, Vec2};

use super::fit::Fitted;
use crate::medial_axis::AxisSample;

/// The bisection stops at this precision, in millimetres.
const PRECISION_MM: f64 = 0.001;
/// The parent's flattening tolerance, in millimetres.
const PARENT_FLATTEN_MM: f64 = 0.002;
/// The longest piece of the flattened parent, along which its width is interpolated, in mm.
const PARENT_PIECE_MM: f64 = 0.1;
/// The step of the outward scan for the first root, in millimetres.
const SCAN_MM: f64 = 0.02;
/// The step of the scan back across the parent for an oblique branch's corner, in millimetres.
const BACK_SCAN_MM: f64 = 0.1;

/// The parent's fitted centre curve, flattened, with its width at every point.
pub(crate) struct ParentCurve {
    /// The flattened pieces, each with the width at both ends.
    pieces: Vec<[(Point, f64); 2]>,
}

impl ParentCurve {
    /// The flattened curve of a fitted path.
    pub(crate) fn new(fitted: &Fitted) -> Self {
        let doc: Vec<_> = fitted.anchors.clone();
        let coarse = crate::flatten::flatten(&doc, fitted.closed, PARENT_FLATTEN_MM);
        // The width is interpolated linearly along a piece: a straight parent flattens to one
        // piece, over which its profile is anything but linear, so the pieces are cut short.
        let mut flat = Vec::with_capacity(coarse.len());
        for pair in coarse.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let length = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let pieces = (length / PARENT_PIECE_MM).ceil().max(1.0) as usize;
            for k in 0..pieces {
                #[allow(clippy::cast_precision_loss)]
                let t = k as f64 / pieces as f64;
                flat.push(curvyo_document_core::Point::new(
                    a.x + (b.x - a.x) * t,
                    a.y + (b.y - a.y) * t,
                ));
            }
        }
        if let Some(&last) = coarse.last() {
            flat.push(last);
        }
        let mut lengths = vec![0.0];
        for pair in flat.windows(2) {
            let step = ((pair[1].x - pair[0].x).powi(2) + (pair[1].y - pair[0].y).powi(2)).sqrt();
            lengths.push(lengths[lengths.len() - 1] + step);
        }
        let total = lengths[lengths.len() - 1].max(1e-12);
        let base = fitted.width.as_mm();
        let points: Vec<(Point, f64)> = flat
            .iter()
            .zip(&lengths)
            .map(|(p, s)| {
                let at = (s / total).clamp(0.0, 1.0);
                let at = if fitted.closed && at >= 1.0 { 0.0 } else { at };
                (
                    Point::new(p.x, p.y),
                    base * fitted.profile.sample(at).unwrap_or(1.0),
                )
            })
            .collect();
        Self {
            pieces: points.windows(2).map(|pair| [pair[0], pair[1]]).collect(),
        }
    }

    /// The pieces that can hold the nearest point of any query within `reach` of `centre`.
    fn near(&self, centre: Point, reach: f64) -> Self {
        // A query `q` has its nearest piece within `|q - centre| + d(centre)` of itself, so
        // within `2 reach + d(centre)` of the centre.
        let radius = 2.0 * reach + self.nearest(centre).0;
        Self {
            pieces: self
                .pieces
                .iter()
                .copied()
                .filter(|&[(a, _), (b, _)]| segment_distance(centre, a, b) <= radius)
                .collect(),
        }
    }

    /// The distance from `point` to the curve and the curve's width at the nearest point.
    fn nearest(&self, point: Point) -> (f64, f64) {
        let (distance, width, _) = self.nearest_side(point);
        (distance, width)
    }

    /// [`Self::nearest`] with the side of the curve `point` lies on (+1 left, -1 right).
    fn nearest_side(&self, point: Point) -> (f64, f64, f64) {
        let mut best = (f64::INFINITY, 0.0, 1.0);
        for &[(a, wa), (b, wb)] in &self.pieces {
            let d = b - a;
            let len2 = d.hypot2();
            let t = if len2 > 0.0 {
                ((point - a).dot(d) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let distance = (a + d * t - point).hypot();
            if distance < best.0 {
                let side = if d.cross(point - a) < 0.0 { -1.0 } else { 1.0 };
                best = (distance, wa + (wb - wa) * t, side);
            }
        }
        best
    }

    /// How deep `point` lies inside the curve's paint (the union of its discs): the most any
    /// disc's radius exceeds its distance; negative outside.
    fn depth(&self, point: Point) -> f64 {
        let mut best = f64::NEG_INFINITY;
        for &[(a, wa), (b, wb)] in &self.pieces {
            let d = b - a;
            let len2 = d.hypot2();
            let t = if len2 > 0.0 {
                ((point - a).dot(d) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let width = wa + (wb - wa) * t;
            best = best.max(width / 2.0 - (a + d * t - point).hypot());
        }
        best
    }

    /// The distance of `point` from the curve, positive on `side`.
    fn signed(&self, point: Point, side: f64) -> f64 {
        let (distance, _, here) = self.nearest_side(point);
        distance * here * side
    }
}

/// The distance from `point` to the segment from `a` to `b`.
fn segment_distance(point: Point, a: Point, b: Point) -> f64 {
    let d = b - a;
    let len2 = d.hypot2();
    let t = if len2 > 0.0 {
        ((point - a).dot(d) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + d * t - point).hypot()
}

/// A branch's samples extended straight back through the junction: inside the zone the
/// branch is the line from its first sample back along `direction` (pointing away from the
/// junction) with width `width`, and on past the junction's centre.
struct Extension<'a> {
    samples: &'a [AxisSample],
    direction: Vec2,
    width: f64,
    /// How far the first sample lies from the junction's centre along `direction`.
    back: f64,
}

impl Extension<'_> {
    /// The sample `u` along the extended branch, from the junction centre outward.
    fn at(&self, u: f64) -> AxisSample {
        let first = self.samples[0];
        if u <= self.back {
            return AxisSample {
                point: first.point - self.direction * (self.back - u),
                radius: self.width / 2.0,
            };
        }
        let mut left = u - self.back;
        for pair in self.samples.windows(2) {
            let step = (pair[1].point - pair[0].point).hypot();
            if left <= step && step > 0.0 {
                let t = left / step;
                return AxisSample {
                    point: pair[0].point.lerp(pair[1].point, t),
                    radius: pair[0].radius + (pair[1].radius - pair[0].radius) * t,
                };
            }
            left -= step;
        }
        self.samples[self.samples.len() - 1]
    }

    /// The branch's normal at `u`, `radius` long, to its left.
    fn normal(&self, u: f64, radius: f64) -> Vec2 {
        let ahead = self.at(u + SCAN_MM).point - self.at((u - SCAN_MM).max(0.0)).point;
        let tangent = if u <= self.back || ahead.hypot() == 0.0 {
            self.direction
        } else {
            ahead / ahead.hypot()
        };
        Vec2::new(-tangent.y, tangent.x) * radius
    }

    /// The samples from `u` on.
    fn from(&self, u: f64) -> Vec<AxisSample> {
        let mut out = vec![self.at(u)];
        let mut travelled = self.back;
        for pair in self.samples.windows(2) {
            if travelled > u {
                out.push(pair[0]);
            }
            travelled += (pair[1].point - pair[0].point).hypot();
        }
        if travelled > u {
            out.push(self.samples[self.samples.len() - 1]);
        } else if self.samples.len() == 1 && self.back > u {
            out.push(self.samples[0]);
        }
        out.dedup_by(|a, b| (a.point - b.point).hypot() < 1e-9);
        out
    }
}

/// The butt of a branch against its parent: which side of the parent the branch lies on and
/// which of its corners lies farther out there.
struct Butt<'a> {
    branch: &'a Extension<'a>,
    parent: &'a ParentCurve,
    /// +1 when the branch lies left of its parent.
    side: f64,
    /// +1 when the butt's left corner is the farther one.
    far_sign: f64,
}

impl Butt<'_> {
    /// The butt's corners at `u` for `radius`, farther one first.
    fn corners(&self, u: f64, radius: f64) -> (Point, Point) {
        let normal = self.branch.normal(u, radius) * self.far_sign;
        let point = self.branch.at(u).point;
        (point + normal, point - normal)
    }

    /// How far the butt at `u` lies short of its target: the farther corner `p/2 w` inside the
    /// parent's paint (an overlap) or the nearer corner on its edge (a gap).
    fn gap(&self, u: f64, overlap: f64) -> f64 {
        let sample = self.branch.at(u);
        let (far, near) = self.corners(u, sample.radius);
        if overlap >= 0.0 {
            0.5 * overlap * self.parent.nearest(sample.point).1 - self.parent.depth(far)
        } else {
            -self.parent.depth(near)
        }
    }

    /// Narrows the samples from the start (at `u`) where the nearer corner would reach out of
    /// the parent's far side (an acute junction), so that it stays inside.
    fn narrow(&self, out: &mut [AxisSample], mut u: f64) {
        for k in 0..out.len() {
            if k > 0 {
                u += (out[k].point - out[k - 1].point).hypot();
            }
            let limit = -0.5 * self.parent.nearest(out[k].point).1;
            let near = |radius: f64| self.parent.signed(self.corners(u, radius).1, self.side);
            if near(out[k].radius) >= limit {
                break;
            }
            let (mut low, mut high) = (0.0, out[k].radius);
            for _ in 0..NARROW_STEPS {
                let mid = f64::midpoint(low, high);
                if near(mid) >= limit {
                    low = mid;
                } else {
                    high = mid;
                }
            }
            out[k].radius = low;
        }
    }
}

/// The branch's samples from its cut on: `samples` start at the junction side (zone samples
/// already left out); inside the zone the branch is the straight line from the first sample
/// back along `direction` (pointing away from the junction) with width `width`. The cut puts
/// the centre `(0.5 - p) w` from the parent, unless the butt then reaches out of the parent (an
/// oblique branch): then deeper, its farther corner `p/2 w` inside the parent's paint. A gap
/// also keeps the nearer corner clear of the parent.
pub(crate) fn cut_branch(
    samples: &[AxisSample],
    centre: Point,
    direction: Vec2,
    width: f64,
    parent: &ParentCurve,
    overlap: f64,
) -> Vec<AxisSample> {
    let Some(first) = samples.first() else {
        return Vec::new();
    };
    let branch = Extension {
        samples,
        direction,
        width,
        back: (first.point - centre).dot(direction).max(0.0),
    };
    let total: f64 = branch.back
        + samples
            .windows(2)
            .map(|w| (w[1].point - w[0].point).hypot())
            .sum::<f64>();
    // The start lies within the junction zone or a parent width beyond it.
    let reach = total.min(branch.back + 2.0 * width + 4.0 * SCAN_MM);
    // The extension runs on past the junction's centre, across the parent, by at most one
    // parent width: the farther corner of an oblique branch may need it. Every point searched
    // lies within `reach - before` of the first sample, `back` from the centre.
    let before = -(2.0 * width).min(parent.nearest(centre).1);
    let parent = parent.near(centre, reach - before + (first.point - centre).hypot());
    let close = parent.near(centre, reach + (first.point - centre).hypot());
    let centre_gap = |u: f64| {
        let (distance, parent_width) = close.nearest(branch.at(u).point);
        distance - (0.5 - overlap) * parent_width
    };
    let Some(centred) = first_root(&centre_gap, 0.0, (0.0, reach), false) else {
        return Vec::new();
    };
    let mut butt = Butt {
        branch: &branch,
        parent: &parent,
        side: side_of(&parent, &branch, centred, reach),
        far_sign: 1.0,
    };
    let (left, right) = butt.corners(centred, branch.at(centred).radius);
    if parent.signed(right, butt.side) > parent.signed(left, butt.side) {
        butt.far_sign = -1.0;
    }
    let gap = |u: f64| butt.gap(u, overlap);
    let start = if overlap >= 0.0 {
        if gap(centred) >= 0.0 {
            first_root(&gap, centred, (before, reach), true).unwrap_or(centred)
        } else {
            centred
        }
    } else if gap(centred) >= 0.0 {
        centred
    } else {
        first_root(&gap, centred, (centred, reach), false).unwrap_or(centred)
    };
    let mut out = branch.from(start);
    butt.narrow(&mut out, start);
    out
}

/// The branch's side of its parent (+1 left): that of its centre-rule start at `centred`, else
/// of its first sample clear of the parent's paint.
fn side_of(parent: &ParentCurve, branch: &Extension<'_>, centred: f64, reach: f64) -> f64 {
    let (distance, _, side) = parent.nearest_side(branch.at(centred).point);
    if distance > PRECISION_MM {
        return side;
    }
    let first = branch.samples[0].point;
    branch
        .samples
        .iter()
        .take_while(|s| (s.point - first).hypot() <= reach)
        .map(|s| {
            let (distance, width, side) = parent.nearest_side(s.point);
            (distance / width.max(1e-12), side)
        })
        .find(|&(share, _)| share >= 0.5)
        .map_or_else(
            || {
                parent
                    .nearest_side(branch.samples[branch.samples.len() - 1].point)
                    .2
            },
            |f| f.1,
        )
}

/// The first `u` from `origin` outward to `to` where `gap` turns non-negative, to
/// [`PRECISION_MM`]. With `backward`, a `gap` already non-negative at `origin` is followed back
/// towards `from` to where it turns; without, `origin` is the answer then.
fn first_root(
    gap: &dyn Fn(f64) -> f64,
    origin: f64,
    (from, to): (f64, f64),
    backward: bool,
) -> Option<f64> {
    let (mut low, mut high) = if gap(origin) >= 0.0 {
        if !backward {
            return Some(origin);
        }
        let mut u = origin;
        loop {
            let next = u - BACK_SCAN_MM;
            if next < from {
                // Never inside within reach (a branch running along its parent): the origin.
                return Some(origin);
            }
            if gap(next) < 0.0 {
                break (next, u);
            }
            u = next;
        }
    } else {
        let mut low = origin;
        let mut u = origin;
        loop {
            if u > to {
                return None;
            }
            if gap(u) >= 0.0 {
                break (low, u);
            }
            low = u;
            u += SCAN_MM;
        }
    };
    while high - low > PRECISION_MM {
        let mid = f64::midpoint(low, high);
        if gap(mid) >= 0.0 {
            high = mid;
        } else {
            low = mid;
        }
    }
    Some(high)
}

/// Bisection steps of the narrowed width at an acute start.
const NARROW_STEPS: usize = 24;
