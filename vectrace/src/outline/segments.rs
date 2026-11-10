//! Turns the smoothed points of one outline into segments: straight runs as lines, the rest as
//! cubics within the tolerance, the closed polygon fall-back, and the finished segments as a path.

use kurbo::{BezPath, Point, Vec2};

use super::Span;
use crate::fit::{Piece, Seg, arc_length, fit_cubics, point_at};
use crate::line::{Pins, Run, segment_distance, straight_runs, unit};

/// The fitting constants of one outline, in traced pixels.
pub(super) struct Rules {
    pub(super) tolerance: f64,
    pub(super) run_tolerance: f64,
    pub(super) run_length: f64,
    pub(super) arc: f64,
}

/// The smoothed `span` between two corners: one line when it is straight within the tolerance
/// (and `chords` allows it), else its straight runs as lines and cubics between them, the
/// cubics at the corners along the corners' straight sides; `tols` is the tolerance per
/// segment.
pub(super) fn fit_span(span: &Span, tols: &[f64], rules: &Rules, chords: bool, out: &mut Vec<Seg>) {
    let q = &span.smoothed;
    let (first, last) = (q[0], q[q.len() - 1]);
    let at = |k: usize| -> f64 {
        let before = tols[k.saturating_sub(1)];
        let after = tols[k.min(tols.len() - 1)];
        before.min(after)
    };
    if q.len() == 2
        || (chords
            && first != last
            && q.iter()
                .enumerate()
                .all(|(k, &p)| segment_distance(p, first, last) <= at(k)))
    {
        out.push(Seg::Line(first, last));
        return;
    }
    let runs = if chords {
        let pins = Pins {
            start: true,
            end: true,
        };
        straight_runs(q, tols, rules.run_tolerance, rules.run_length, pins)
    } else {
        // Two corners only (a thin bar's two ends): a straight side is a line off the chord,
        // joined to the corners by short lines.
        let last = q.len() - 1;
        let pins = Pins {
            start: false,
            end: false,
        };
        let length = arc_length(q);
        straight_runs(
            &q[1..last],
            &tols[1..last - 1],
            rules.tolerance,
            0.6 * length,
            pins,
        )
        .into_iter()
        .map(|r| Run {
            first: r.first + 1,
            last: r.last + 1,
            ..r
        })
        .collect()
    };
    assemble(q, tols, &runs, span.tangents, rules, out);
}

/// The closed corner-free chain `q` (last point = first): its straight runs as lines and
/// cubics between them, tangent-continuous at the start.
pub(super) fn fit_closed(q: &[Point], tols: &[f64], rules: &Rules, out: &mut Vec<Seg>) {
    let last = q.len() - 1;
    let length = arc_length(q);
    let reach = rules.arc.min(length / 4.0);
    let Some(seam) = unit(point_at(q, reach) - point_at(q, length - reach)) else {
        out.extend(polygon(&q[..last]));
        return;
    };
    let pins = Pins {
        start: false,
        end: false,
    };
    let runs: Vec<Run> = straight_runs(
        &q[1..last],
        &tols[1..last - 1],
        rules.run_tolerance,
        rules.run_length,
        pins,
    )
    .into_iter()
    .map(|r| Run {
        first: r.first + 1,
        last: r.last + 1,
        ..r
    })
    .collect();
    if runs.is_empty() {
        // Two halves meeting tangent-continuously at the middle point.
        let mid = last / 2;
        let at = arc_length(&q[..=mid]);
        let Some(centre) = unit(point_at(q, at + reach) - point_at(q, at - reach)) else {
            out.extend(polygon(&q[..last]));
            return;
        };
        let halves = [
            (0, mid, Some(seam), Some(-centre)),
            (mid, last, Some(centre), Some(-seam)),
        ];
        for (i, j, t0, t1) in halves {
            let piece = Piece {
                q: &q[i..=j],
                tols: &tols[i..j],
                start: q[i],
                end: q[j],
                start_tangent: t0,
                end_tangent: t1,
            };
            fit_cubics(piece, rules.tolerance, rules.arc, out);
        }
        return;
    }
    assemble(q, tols, &runs, (Some(seam), Some(-seam)), rules, out);
}

/// Lines for `runs` of `q` and cubics for the gaps, tangent-continuous at every run end;
/// `(start, end)` are the tangents at `q`'s ends (`None`: free, at a corner).
fn assemble(
    q: &[Point],
    tols: &[f64],
    runs: &[Run],
    (start, end): (Option<Vec2>, Option<Vec2>),
    rules: &Rules,
    out: &mut Vec<Seg>,
) {
    let last = q.len() - 1;
    let (mut cursor, mut node, mut tangent) = (0, q[0], start);
    for run in runs {
        let direction = unit(run.to - run.from);
        if run.first > cursor {
            let piece = Piece {
                q: &q[cursor..=run.first],
                tols: &tols[cursor..run.first],
                start: node,
                end: run.from,
                start_tangent: tangent,
                end_tangent: direction.map(|d| -d),
            };
            fit_cubics(piece, rules.tolerance, rules.arc, out);
        }
        out.push(Seg::Line(run.from, run.to));
        (cursor, node, tangent) = (run.last, run.to, direction);
    }
    if cursor < last {
        let piece = Piece {
            q: &q[cursor..],
            tols: &tols[cursor..last],
            start: node,
            end: q[last],
            start_tangent: tangent,
            end_tangent: end,
        };
        fit_cubics(piece, rules.tolerance, rules.arc, out);
    }
}

/// The closed polygon through `points`, points on a straight line between their neighbours
/// left out.
pub(super) fn polygon(points: &[Point]) -> Vec<Seg> {
    let n = points.len();
    let kept: Vec<Point> = (0..n)
        .filter(|&i| {
            let (a, b, c) = (points[(i + n - 1) % n], points[i], points[(i + 1) % n]);
            (b - a).cross(c - b) != 0.0 || (b - a).dot(c - b) < 0.0
        })
        .map(|i| points[i])
        .collect();
    let kept = if kept.len() >= 3 {
        kept
    } else {
        points.to_vec()
    };
    let m = kept.len();
    (0..m)
        .map(|i| Seg::Line(kept[i], kept[(i + 1) % m]))
        .collect()
}

/// The lines through `points`, in order (an open polyline).
pub(super) fn open_polyline(points: &[Point]) -> Vec<Seg> {
    points.windows(2).map(|w| Seg::Line(w[0], w[1])).collect()
}

/// `segments` (a closed outline) as a closed path.
pub(crate) fn to_path(segments: &[Seg]) -> BezPath {
    let mut path = BezPath::new();
    let Some(first) = segments.first() else {
        return path;
    };
    path.move_to(match *first {
        Seg::Line(p, _) | Seg::Cubic(p, _, _, _) => p,
    });
    for seg in segments {
        match *seg {
            // A line of no length is no node.
            Seg::Line(a, b) if a == b => {}
            Seg::Line(_, p) => path.line_to(p),
            Seg::Cubic(_, c1, c2, p) => path.curve_to(c1, c2, p),
        }
    }
    path.close_path();
    path
}

/// The anchors of a closed outline: one per segment, the closing one not counted twice. This is
/// what `Output::nodes` counts; Curvyo's join of trace parts recomputes it with the same rule.
#[must_use]
pub fn node_count(path: &BezPath) -> usize {
    use kurbo::PathEl;
    let mut start = None;
    let mut last = None;
    let mut segments = 0;
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(p) => {
                start = Some(p);
                last = Some(p);
            }
            PathEl::LineTo(p) | PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => {
                segments += 1;
                last = Some(p);
            }
            PathEl::ClosePath => {}
        }
    }
    // The last segment ends on the start: its end is the first anchor again.
    if segments > 0 && start == last {
        segments
    } else {
        segments + 1
    }
}
