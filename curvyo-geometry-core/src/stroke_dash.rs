//! The dashes of one outline on its exact arc length (`specs/0085-stroke-to-path` `adrs.md`
//! decision 4): the semantics of the canvas's dash walk in `curvyo-render-core` (`dash_walk.rs`):
//! the pattern in multiples of the width, an odd list walked twice, the walk starting afresh at
//! the first node, a zero-length "on" entry a dot unless the cap is Butt.

use curvyo_document_core::DashPattern;
use kurbo::{ParamCurve, ParamCurveArclen, PathSeg, Point, Vec2};

use crate::offset_pieces::{end_tangent, start_tangent};
use crate::stroke_loops::Cap;

/// The accuracy of every arc length, millimetres.
const ARCLEN_ACCURACY_MM: f64 = 1e-6;

/// More dashes than this on one outline draw it solid, as the canvas does past its own limit; it
/// keeps a crafted pattern from exhausting memory.
const MAX_DASHES_PER_OUTLINE: f64 = 100_000.0;

/// One "on" part of a dashed outline.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DashPiece {
    /// A dash: the outline's segments between two arc lengths.
    Dash(Vec<PathSeg>),
    /// A zero-length dash: a dot at a point, heading in the outline's direction there.
    Dot {
        /// Where.
        at: Point,
        /// The unit direction of travel.
        ahead: Vec2,
    },
}

/// The outline `segments` measured: each segment's length.
pub(crate) struct Measured<'a> {
    segments: &'a [PathSeg],
    lengths: Vec<f64>,
    total: f64,
}

impl<'a> Measured<'a> {
    pub(crate) fn new(segments: &'a [PathSeg]) -> Self {
        let lengths: Vec<f64> = segments
            .iter()
            .map(|segment| segment.arclen(ARCLEN_ACCURACY_MM))
            .collect();
        let total = lengths.iter().sum();
        Self {
            segments,
            lengths,
            total,
        }
    }

    /// The whole length.
    pub(crate) const fn total(&self) -> f64 {
        self.total
    }

    /// The segment and parameter at arc length `s` (clamped to the outline).
    fn place(&self, s: f64) -> (usize, f64) {
        let mut start = 0.0;
        for (index, &length) in self.lengths.iter().enumerate() {
            if s <= start + length || index + 1 == self.lengths.len() {
                let local = (s - start).clamp(0.0, length);
                let t = if length > 0.0 {
                    self.segments[index].inv_arclen(local, ARCLEN_ACCURACY_MM)
                } else {
                    0.0
                };
                return (index, t);
            }
            start += length;
        }
        (0, 0.0)
    }

    /// The point and unit direction at arc length `s`.
    pub(crate) fn sample(&self, s: f64) -> (Point, Vec2) {
        let (index, t) = self.place(s);
        let segment = &self.segments[index];
        let ahead = if t >= 1.0 {
            end_tangent(segment)
        } else if t <= 0.0 {
            start_tangent(segment)
        } else {
            let (before, after) = (segment.subsegment(0.0..t), segment.subsegment(t..1.0));
            let u = end_tangent(&before);
            if u == Vec2::ZERO {
                start_tangent(&after)
            } else {
                u
            }
        };
        (segment.eval(t), ahead)
    }

    /// The segments between the arc lengths `a` and `b`, cut at both.
    fn between(&self, a: f64, b: f64) -> Vec<PathSeg> {
        let mut out = Vec::new();
        let mut start = 0.0;
        for (index, &length) in self.lengths.iter().enumerate() {
            let end = start + length;
            if length > 0.0 && end > a && start < b {
                let segment = &self.segments[index];
                let t0 = if a > start {
                    segment.inv_arclen(a - start, ARCLEN_ACCURACY_MM)
                } else {
                    0.0
                };
                let t1 = if b < end {
                    segment.inv_arclen(b - start, ARCLEN_ACCURACY_MM)
                } else {
                    1.0
                };
                if t1 - t0 > 1e-12 {
                    out.push(if t0 <= 0.0 && t1 >= 1.0 {
                        *segment
                    } else {
                        segment.subsegment(t0..t1)
                    });
                }
            }
            start = end;
        }
        out
    }
}

/// The dashes of the outline `segments` (at least one, positive length) for `pattern` at `width`
/// millimetres, or `None` when it draws solid: a solid pattern, or more dashes than
/// [`MAX_DASHES_PER_OUTLINE`].
pub(crate) fn dash_pieces(
    measured: &Measured<'_>,
    pattern: &DashPattern,
    width: f64,
    cap: Cap,
) -> Option<Vec<DashPiece>> {
    let stored = pattern.as_slice();
    if stored.is_empty() {
        return None;
    }
    let ratios: Vec<f64> = if stored.len() % 2 == 1 {
        [stored, stored].concat()
    } else {
        stored.to_vec()
    };
    let period = ratios.iter().sum::<f64>() * width;
    let length = measured.total();
    if !(period > 0.0 && period.is_finite() && length > 0.0) {
        return None;
    }
    let dots = cap != Cap::Butt;
    let per_period = ratios
        .chunks(2)
        .filter(|pair| pair[0] > 0.0 || dots)
        .count();
    #[allow(clippy::cast_precision_loss)] // a handful of entries
    let estimate = (length / period).ceil() * per_period as f64;
    if estimate.is_nan() || estimate > MAX_DASHES_PER_OUTLINE {
        return None;
    }
    let mut pieces = Vec::new();
    let mut distance = 0.0_f64;
    // invariant: the estimate bounds the walk; the step cap also stops it if rounding ever failed
    // to advance the distance.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let max_steps = ratios.len() * (estimate as usize + 2);
    for step in 0..max_steps {
        if distance >= length {
            break;
        }
        let run = ratios[step % ratios.len()] * width;
        if step % 2 == 0 {
            if run > 0.0 {
                let dash = measured.between(distance, (distance + run).min(length));
                if !dash.is_empty() {
                    pieces.push(DashPiece::Dash(dash));
                }
            } else if dots {
                let (at, ahead) = measured.sample(distance);
                if ahead != Vec2::ZERO {
                    pieces.push(DashPiece::Dot { at, ahead });
                }
            }
        }
        distance += run;
    }
    Some(pieces)
}

#[cfg(test)]
mod tests {
    use kurbo::Line;

    use super::*;

    fn line(length: f64) -> Vec<PathSeg> {
        vec![PathSeg::Line(Line::new(
            Point::ORIGIN,
            Point::new(length, 0.0),
        ))]
    }

    fn dashes(pattern: &[f64], cap: Cap, length: f64) -> Vec<DashPiece> {
        let segments = line(length);
        let measured = Measured::new(&segments);
        dash_pieces(
            &measured,
            &DashPattern::new(pattern.to_vec()).unwrap(),
            1.0,
            cap,
        )
        .unwrap()
    }

    #[test]
    fn an_odd_list_is_walked_twice() {
        // `1 2 4` walks as `1 2 4 1 2 4`: three dashes per 14 mm.
        assert_eq!(dashes(&[1.0, 2.0, 4.0], Cap::Butt, 28.0).len(), 6);
    }

    #[test]
    fn a_zero_entry_is_a_dot_unless_the_cap_is_butt() {
        assert_eq!(dashes(&[0.0, 3.0], Cap::Round, 30.0).len(), 10);
        assert_eq!(dashes(&[0.0, 3.0], Cap::Butt, 30.0), Vec::new());
    }

    #[test]
    fn the_last_dash_is_cut_at_the_end() {
        let pieces = dashes(&[6.0, 4.0], Cap::Butt, 13.0);
        assert_eq!(pieces.len(), 2);
        let DashPiece::Dash(last) = &pieces[1] else {
            panic!("a dash");
        };
        assert!((last[0].eval(1.0).x - 13.0).abs() < 1e-9);
    }
}
