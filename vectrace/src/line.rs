//! Fits straight lines to runs of chain points by total least squares (`specs/0089` adrs.md §5),
//! with the vector helpers of the geometry modules in basic operations and `sqrt` only (§8).

use kurbo::{Point, Vec2};

/// Lines whose directions differ by less than this sine do not intersect usefully.
const MIN_SINE: f64 = 0.1;

/// The length of `v`.
pub(crate) fn norm(v: Vec2) -> f64 {
    (v.x * v.x + v.y * v.y).sqrt()
}

/// The distance from `a` to `b`.
pub(crate) fn distance(a: Point, b: Point) -> f64 {
    norm(b - a)
}

/// `v` scaled to length 1, or `None` for a (nearly) zero vector.
pub(crate) fn unit(v: Vec2) -> Option<Vec2> {
    let n = norm(v);
    (n > 1e-12).then(|| Vec2::new(v.x / n, v.y / n))
}

/// The distance from `point` to the segment `from`–`to`.
pub(crate) fn segment_distance(point: Point, from: Point, to: Point) -> f64 {
    let along = to - from;
    let length_sq = along.dot(along);
    if length_sq == 0.0 {
        return distance(point, from);
    }
    let share = ((point - from).dot(along) / length_sq).clamp(0.0, 1.0);
    distance(point, from + along * share)
}

/// An infinite line through `origin` along the unit vector `direction`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Line {
    pub origin: Point,
    pub direction: Vec2,
}

impl Line {
    /// The unsigned distance of `p` from the line.
    pub(crate) fn offset(&self, p: Point) -> f64 {
        (p - self.origin).cross(self.direction).abs()
    }

    /// The foot of `p` on the line.
    pub(crate) fn project(&self, p: Point) -> Point {
        self.origin + self.direction * (p - self.origin).dot(self.direction)
    }

    /// Where the two lines cross, or `None` when they are (nearly) parallel.
    pub(crate) fn intersect(&self, other: &Self) -> Option<Point> {
        let denominator = self.direction.cross(other.direction);
        if denominator.abs() < MIN_SINE {
            return None;
        }
        let t = (other.origin - self.origin).cross(other.direction) / denominator;
        Some(self.origin + self.direction * t)
    }

    /// The largest offset of `points` from the line.
    pub(crate) fn max_offset(&self, points: &[Point]) -> f64 {
        points.iter().map(|&p| self.offset(p)).fold(0.0, f64::max)
    }
}

/// The unit eigenvector of the larger eigenvalue of `[[xx, xy], [xy, yy]]`, or `None` when the
/// matrix has no single main direction.
fn principal(xx: f64, xy: f64, yy: f64) -> Option<Vec2> {
    let half = (xx - yy) / 2.0;
    let root = (half * half + xy * xy).sqrt();
    if root <= 1e-12 * (xx + yy).max(1e-300) {
        return None;
    }
    let lambda = f64::midpoint(xx, yy) + root;
    let a = Vec2::new(xy, lambda - xx);
    let b = Vec2::new(lambda - yy, xy);
    unit(if a.dot(a) >= b.dot(b) { a } else { b })
}

/// `direction` pointing the way of `first` → `last`.
fn along(direction: Vec2, first: Point, last: Point) -> Vec2 {
    if direction.dot(last - first) < 0.0 {
        -direction
    } else {
        direction
    }
}

/// The total-least-squares line of `points` (at least two, not all equal), directed from the
/// first towards the last.
pub(crate) fn fit_line(points: &[Point]) -> Option<Line> {
    let (&first, &last) = (points.first()?, points.last()?);
    // Relative to the first point, so a whole-pixel shift changes nothing but the origin.
    #[allow(clippy::cast_precision_loss)] // chains are far shorter than 2^52
    let n = points.len() as f64;
    let (mut sx, mut sy) = (0.0, 0.0);
    for p in points {
        sx += p.x - first.x;
        sy += p.y - first.y;
    }
    let mean = Vec2::new(sx / n, sy / n);
    let (mut xx, mut xy, mut yy) = (0.0, 0.0, 0.0);
    for p in points {
        let d = (*p - first) - mean;
        xx += d.x * d.x;
        xy += d.x * d.y;
        yy += d.y * d.y;
    }
    let direction = principal(xx, xy, yy)?;
    Some(Line {
        origin: first + mean,
        direction: along(direction, first, last),
    })
}

/// The line through `anchor` closest to `points` in the least-squares sense, directed from the
/// first point towards the last.
pub(crate) fn fit_line_through(anchor: Point, points: &[Point]) -> Option<Line> {
    let (&first, &last) = (points.first()?, points.last()?);
    let (mut xx, mut xy, mut yy) = (0.0, 0.0, 0.0);
    for p in points {
        let d = *p - anchor;
        xx += d.x * d.x;
        xy += d.x * d.y;
        yy += d.y * d.y;
    }
    let direction = principal(xx, xy, yy)?;
    Some(Line {
        origin: anchor,
        direction: along(direction, first, last),
    })
}

/// `points` with the points left out that lie within their own tolerance of the simplified
/// polyline (`tols`, one per point; Douglas and Peucker splitting at the point furthest beyond
/// its tolerance; the ends stay).
pub(crate) fn simplify(points: &[Point], tols: &[f64]) -> Vec<Point> {
    let n = points.len();
    if n < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;
    let mut stack = vec![(0, n - 1)];
    while let Some((first, last)) = stack.pop() {
        let (mut worst, mut at) = (1.0, first);
        for k in first + 1..last {
            let d = segment_distance(points[k], points[first], points[last]) / tols[k].max(1e-12);
            if d > worst {
                (worst, at) = (d, k);
            }
        }
        if at > first {
            keep[at] = true;
            stack.push((first, at));
            stack.push((at, last));
        }
    }
    points
        .iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(*p))
        .collect()
}

/// A straight run of a chain: points `first..=last` lie within the run tolerance of the segment
/// `from`–`to`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Run {
    pub first: usize,
    pub last: usize,
    pub from: Point,
    pub to: Point,
}

/// Where a chain's ends are held: a pinned end is a node every line through it must keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Pins {
    pub start: bool,
    pub end: bool,
}

/// The line of `q[i..=j]` honouring the pins, if its points lie within `tolerance` (and the
/// smallest of the run's segment tolerances `tols`) of it.
fn run_line(
    q: &[Point],
    tols: &[f64],
    i: usize,
    j: usize,
    pins: Pins,
    tolerance: f64,
) -> Option<Line> {
    let last = q.len() - 1;
    let points = &q[i..=j];
    let tolerance = tols[i..j].iter().copied().fold(tolerance, f64::min);
    let line = match (i == 0 && pins.start, j == last && pins.end) {
        (true, true) => Line {
            origin: q[0],
            direction: unit(q[last] - q[0])?,
        },
        (true, false) => fit_line_through(q[0], points)?,
        (false, true) => fit_line_through(q[last], points)?,
        (false, false) => fit_line(points)?,
    };
    (line.max_offset(points) <= tolerance).then_some(line)
}

/// The run of `q[i..=j]` on `line`: pinned ends stay, free ends are the feet of their points.
fn run_of(q: &[Point], i: usize, j: usize, pins: Pins, line: &Line) -> Run {
    let last = q.len() - 1;
    let end = |k: usize, pinned: bool| if pinned { q[k] } else { line.project(q[k]) };
    Run {
        first: i,
        last: j,
        from: end(i, i == 0 && pins.start),
        to: end(j, j == last && pins.end),
    }
}

/// The maximal straight runs of the open chain `q` at least `min_length` long (arc length along
/// `q`) whose points lie within `tolerance` (and their segments' `tols`) of their line, found
/// greedily from the start; runs do not share points.
pub(crate) fn straight_runs(
    q: &[Point],
    tols: &[f64],
    tolerance: f64,
    min_length: f64,
    pins: Pins,
) -> Vec<Run> {
    let mut arc = Vec::with_capacity(q.len());
    let mut total = 0.0;
    for (k, p) in q.iter().enumerate() {
        if k > 0 {
            total += distance(q[k - 1], *p);
        }
        arc.push(total);
    }
    let last = q.len().saturating_sub(1);
    let mut runs = Vec::new();
    let mut i = 0;
    let mut shortest = 0;
    while i < last {
        // The first end far enough from `i`.
        shortest = shortest.max(i + 1);
        while shortest <= last && arc[shortest] - arc[i] < min_length {
            shortest += 1;
        }
        if shortest > last {
            break;
        }
        if run_line(q, tols, i, shortest, pins, tolerance).is_none() {
            i += 1;
            continue;
        }
        // Grow by doubling, then bisect between the last good and the first bad end.
        let (mut good, mut step) = (shortest, 1);
        let mut bad = None;
        while good < last {
            let j = (good + step).min(last);
            if run_line(q, tols, i, j, pins, tolerance).is_some() {
                good = j;
                step *= 2;
            } else {
                bad = Some(j);
                break;
            }
        }
        if let Some(mut bad) = bad {
            while bad - good > 1 {
                let mid = good + (bad - good) / 2;
                if run_line(q, tols, i, mid, pins, tolerance).is_some() {
                    good = mid;
                } else {
                    bad = mid;
                }
            }
        }
        if let Some(line) = run_line(q, tols, i, good, pins, tolerance) {
            runs.push(run_of(q, i, good, pins, &line));
        }
        i = good + 1;
    }
    runs
}

#[cfg(test)]
// Test shapes are small, named by their formulas (x, y, w, h).
#[allow(clippy::cast_precision_loss, clippy::many_single_char_names)]
mod tests {
    use super::*;

    #[test]
    fn a_staircase_fits_its_true_line() {
        // The edge midpoints of a digital line of slope 1/4.
        let points: Vec<Point> = (0..40)
            .map(|x| Point::new(f64::from(x) + 0.5, (f64::from(x) / 4.0).floor()))
            .collect();
        let line = fit_line(&points).unwrap();
        assert!(
            (line.direction.y / line.direction.x - 0.25).abs() < 0.01,
            "{line:?}"
        );
        assert!(line.max_offset(&points) < 0.5);
    }

    #[test]
    fn two_lines_cross_where_expected() {
        let a = Line {
            origin: Point::new(0.0, 1.0),
            direction: Vec2::new(1.0, 0.0),
        };
        let b = Line {
            origin: Point::new(3.0, 5.0),
            direction: Vec2::new(0.0, 1.0),
        };
        assert_eq!(a.intersect(&b), Some(Point::new(3.0, 1.0)));
        assert_eq!(a.intersect(&a), None);
    }

    #[test]
    fn runs_are_found_on_the_straight_parts_only() {
        // 40 px straight, a quarter circle of radius 10, 40 px straight.
        let mut q: Vec<Point> = (0..=40).map(|x| Point::new(f64::from(x), 0.0)).collect();
        for k in 1..16 {
            let a = f64::from(k) / 16.0 * std::f64::consts::FRAC_PI_2;
            q.push(Point::new(40.0 + 10.0 * a.sin(), 10.0 - 10.0 * a.cos()));
        }
        q.extend((0..=40).map(|y| Point::new(50.0, 10.0 + f64::from(y))));
        let pins = Pins {
            start: true,
            end: true,
        };
        let tols = vec![1.0; q.len() - 1];
        let runs = straight_runs(&q, &tols, 0.25, 20.0, pins);
        assert_eq!(runs.len(), 2, "{runs:?}");
        assert_eq!(runs[0].from, q[0]);
        assert_eq!(*q.last().unwrap(), runs[1].to);
        assert!(
            runs[0].last >= 40 && runs[1].first <= q.len() - 41,
            "{runs:?}"
        );
    }
}
