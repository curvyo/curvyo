//! The scan lines of one Engrave area (ADR 0064 §6; criteria 14, 15): lines Interval apart at the
//! Scan angle, the first half an Interval inside the area's box, cut to the area by the nonzero
//! rule over all its rings.

use curvyo_geometry_core::direction_of_degrees;

use crate::job_input::PointUm;

/// One burn span of a scan line, from `0` to `1` along the line's base direction.
pub type Span = (PointUm, PointUm);

/// The spans of each scan line of one area, lines in order across the area, spans in the base
/// direction along each line. Empty lines are left out.
pub type HatchLines = Vec<Vec<Span>>;

/// The scan lines of the area bounded by `rings` (each a closed polygon, the closing edge
/// implicit) at `interval_um` apart and `angle_deg` (0: along the page's X axis; positive turns
/// counter-clockwise as seen on the page). Intersections are computed in f64 from the integer
/// points and rounded to µm at once (ADR 0064 §2).
#[must_use]
pub fn hatch(rings: &[Vec<PointUm>], interval_um: i64, angle_deg: f64) -> HatchLines {
    if interval_um <= 0 {
        return Vec::new();
    }
    // The page's Y grows down, so a counter-clockwise turn as seen is a negative Y component.
    let turn = direction_of_degrees(angle_deg);
    let dir = (turn.x, -turn.y);
    let normal = (-dir.1, dir.0);
    let edges = scan_edges(rings, dir, normal);
    let Some((low, high)) = edges.iter().fold(None, |range: Option<(f64, f64)>, edge| {
        let (a, b) = (edge.s0.min(edge.s1), edge.s0.max(edge.s1));
        Some(range.map_or((a, b), |(lo, hi)| (lo.min(a), hi.max(b))))
    }) else {
        return Vec::new();
    };
    let step = as_f64(interval_um);
    let first = low + step / 2.0;
    if first > high {
        return Vec::new();
    }
    let count = line_count(first, high, step);
    let mut crossings: Vec<Vec<(f64, i32)>> = vec![Vec::new(); count];
    for edge in &edges {
        edge.cross(first, step, count, &mut crossings);
    }
    let point = |s: f64, t: f64| {
        PointUm::new(
            round_um(s * normal.0 + t * dir.0),
            round_um(s * normal.1 + t * dir.1),
        )
    };
    crossings
        .into_iter()
        .enumerate()
        .filter_map(|(k, mut list)| {
            #[allow(clippy::cast_precision_loss)]
            // exact: line counts are far below 2^53
            let s = first + k as f64 * step;
            list.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let spans = spans_of(&list)
                .into_iter()
                .map(|(t0, t1)| (point(s, t0), point(s, t1)))
                .filter(|(a, b)| a != b)
                .collect::<Vec<_>>();
            (!spans.is_empty()).then_some(spans)
        })
        .collect()
}

/// The edges of `rings` (each closed, at least three points) in the scan frame of `dir` and
/// `normal`, without the edges along a scan line.
fn scan_edges(rings: &[Vec<PointUm>], dir: (f64, f64), normal: (f64, f64)) -> Vec<Edge> {
    let along = |p: PointUm| as_f64(p.x) * dir.0 + as_f64(p.y) * dir.1;
    let across = |p: PointUm| as_f64(p.x) * normal.0 + as_f64(p.y) * normal.1;
    rings
        .iter()
        .filter(|ring| ring.len() >= 3)
        .flat_map(|ring| {
            ring.iter()
                .zip(ring.iter().cycle().skip(1))
                .map(|(&a, &b)| Edge {
                    s0: across(a),
                    s1: across(b),
                    t0: along(a),
                    t1: along(b),
                })
        })
        // exact: an edge along a scan line crosses none, by the half-open rule
        .filter(|edge| {
            #[allow(clippy::float_cmp)]
            let crosses = edge.s0 != edge.s1;
            crosses
        })
        .collect()
}

/// One polygon edge in the scan frame: `s` across the lines, `t` along them.
struct Edge {
    s0: f64,
    s1: f64,
    t0: f64,
    t1: f64,
}

impl Edge {
    /// Adds this edge's crossing with every line `first + k * step` it spans, by the half-open
    /// rule (`min <= s < max`), with its winding (+1 when `s` rises).
    fn cross(&self, first: f64, step: f64, count: usize, crossings: &mut [Vec<(f64, i32)>]) {
        let (lo, hi) = (self.s0.min(self.s1), self.s0.max(self.s1));
        let from = index_at_or_after(first, lo, step).saturating_sub(1);
        let wind = if self.s1 > self.s0 { 1 } else { -1 };
        for (k, line) in crossings.iter_mut().enumerate().take(count).skip(from) {
            #[allow(clippy::cast_precision_loss)]
            // exact: line counts are far below 2^53
            let s = first + k as f64 * step;
            if s >= hi {
                break;
            }
            if s < lo {
                continue;
            }
            let t = self.t0 + (s - self.s0) / (self.s1 - self.s0) * (self.t1 - self.t0);
            line.push((t, wind));
        }
    }
}

/// The spans of one line where the winding number is not zero, from its sorted crossings.
fn spans_of(crossings: &[(f64, i32)]) -> Vec<(f64, f64)> {
    let mut spans = Vec::new();
    let mut winding = 0;
    let mut start = 0.0;
    for &(t, wind) in crossings {
        let before = winding;
        winding += wind;
        if before == 0 && winding != 0 {
            start = t;
        } else if before != 0 && winding == 0 {
            spans.push((start, t));
        }
    }
    spans
}

/// The number of lines from `first` to `high` inclusive at `step`.
fn line_count(first: f64, high: f64, step: f64) -> usize {
    let n = ((high - first) / step).floor();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    // invariant: `first <= high`, so `n >= 0`; `as` saturates for absurd ranges
    let n = n as usize;
    n + 1
}

/// The first line index whose position is at or after `s`.
fn index_at_or_after(first: f64, s: f64, step: f64) -> usize {
    let k = ((s - first) / step).ceil().max(0.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    // invariant: `k >= 0`; `as` saturates
    let k = k as usize;
    k
}

fn as_f64(um: i64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    // exact: page coordinates in µm are far below 2^53
    let value = um as f64;
    value
}

fn round_um(value: f64) -> i64 {
    #[allow(clippy::cast_possible_truncation)]
    // invariant: `as` saturates; values are page coordinates in µm
    let um = value.round() as i64;
    um
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: i64, y: i64, side: i64) -> Vec<PointUm> {
        vec![
            PointUm::new(x, y),
            PointUm::new(x + side, y),
            PointUm::new(x + side, y + side),
            PointUm::new(x, y + side),
        ]
    }

    #[test]
    fn lines_at_zero_degrees_run_along_x_half_an_interval_inside() {
        let lines = hatch(&[square(0, 0, 10_000)], 1_000, 0.0);
        assert_eq!(lines.len(), 10);
        assert_eq!(
            lines[0],
            vec![(PointUm::new(0, 500), PointUm::new(10_000, 500))]
        );
        assert_eq!(lines[9][0].0.y, 9_500);
        assert!(lines.iter().all(|line| line.len() == 1));
    }

    #[test]
    fn a_hole_stays_empty_and_its_line_gets_two_spans() {
        // The inner square runs the other way: a hole by the nonzero rule.
        let mut hole = square(4_000, 4_000, 2_000);
        hole.reverse();
        let lines = hatch(&[square(0, 0, 10_000), hole], 1_000, 0.0);
        let through = &lines[4];
        assert_eq!(through[0].0.y, 4_500);
        assert_eq!(
            through,
            &vec![
                (PointUm::new(0, 4_500), PointUm::new(4_000, 4_500)),
                (PointUm::new(6_000, 4_500), PointUm::new(10_000, 4_500)),
            ]
        );
    }

    #[test]
    fn an_island_drawn_the_same_way_is_filled_by_nonzero() {
        let lines = hatch(
            &[square(0, 0, 10_000), square(4_000, 4_000, 2_000)],
            1_000,
            0.0,
        );
        assert!(lines.iter().all(|line| line.len() == 1));
    }

    #[test]
    fn ninety_degrees_runs_lines_along_y() {
        let lines = hatch(&[square(0, 0, 10_000)], 2_500, 90.0);
        assert_eq!(lines.len(), 4);
        for line in &lines {
            assert_eq!(line[0].0.x, line[0].1.x, "vertical");
        }
    }

    #[test]
    fn positive_angles_turn_counter_clockwise_as_seen() {
        let lines = hatch(&[square(0, 0, 10_000)], 1_000, 45.0);
        let (a, b) = lines[lines.len() / 2][0];
        // Along +X and up the page (Y decreasing).
        assert!(b.x > a.x && b.y < a.y, "{a:?} {b:?}");
    }

    #[test]
    fn a_narrow_area_or_no_interval_gives_no_line() {
        assert_eq!(hatch(&[square(0, 0, 400)], 1_000, 0.0).len(), 0);
        assert_eq!(hatch(&[square(0, 0, 10_000)], 0, 0.0).len(), 0);
        assert_eq!(hatch(&[], 1_000, 0.0).len(), 0);
    }

    #[test]
    fn hatching_is_deterministic() {
        let ring = vec![
            PointUm::new(0, 0),
            PointUm::new(7_331, 1_002),
            PointUm::new(5_000, 9_999),
        ];
        assert_eq!(
            hatch(std::slice::from_ref(&ring), 333, 17.5),
            hatch(&[ring], 333, 17.5)
        );
    }
}
