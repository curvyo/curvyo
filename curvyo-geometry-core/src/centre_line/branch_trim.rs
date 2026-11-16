//! The start of a branch (ADR 0063 §4, branch start): where its distance to the parent's fitted
//! centre curve equals `(0.5 - p) w`, inside the junction zone along the straight extension of
//! its edge measure.

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
        let mut best = (f64::INFINITY, 0.0);
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
                best = (distance, wa + (wb - wa) * t);
            }
        }
        best
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

/// The branch's samples from its cut on: `samples` start at the junction side (zone samples
/// already left out); inside the zone the branch is the straight line from the first sample
/// back along `direction` (pointing away from the junction) with width `width`.
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
    let back = (first.point - centre).dot(direction).max(0.0);
    // Position along the extended branch, from the junction centre outward.
    let total: f64 = back
        + samples
            .windows(2)
            .map(|w| (w[1].point - w[0].point).hypot())
            .sum::<f64>();
    let at = |u: f64| -> AxisSample {
        if u <= back {
            AxisSample {
                point: first.point - direction * (back - u),
                radius: width / 2.0,
            }
        } else {
            let mut left = u - back;
            for pair in samples.windows(2) {
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
            samples[samples.len() - 1]
        }
    };
    // The start lies within the junction zone or a parent width beyond it.
    let reach = total.min(back + 2.0 * width + 4.0 * SCAN_MM);
    // Every point searched lies within `reach` of the first sample, which lies `back` from
    // the centre along the branch.
    let parent = parent.near(centre, reach + (first.point - centre).hypot());
    let gap = |u: f64| {
        let (distance, parent_width) = parent.nearest(at(u).point);
        distance - (0.5 - overlap) * parent_width
    };
    let mut low = 0.0;
    let mut high = None;
    let mut u = 0.0;
    while u <= reach {
        if gap(u) >= 0.0 {
            high = Some(u);
            break;
        }
        low = u;
        u += SCAN_MM;
    }
    let Some(mut high) = high else {
        return Vec::new();
    };
    while high - low > PRECISION_MM {
        let mid = f64::midpoint(low, high);
        if gap(mid) >= 0.0 {
            high = mid;
        } else {
            low = mid;
        }
    }
    let start = at(high);
    let mut out = vec![start];
    let mut travelled = back;
    for pair in samples.windows(2) {
        if travelled > high {
            out.push(pair[0]);
        }
        travelled += (pair[1].point - pair[0].point).hypot();
    }
    if travelled > high {
        out.push(samples[samples.len() - 1]);
    } else if samples.len() == 1 && back > high {
        out.push(samples[0]);
    }
    out.dedup_by(|a, b| (a.point - b.point).hypot() < 1e-9);
    out
}
