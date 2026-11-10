//! One closed traced outline as points about one picture pixel apart around its area centroid,
//! with its area moments (`specs/0088-trace-recognition` Part B: the fit samples at a step
//! relative to the picture pixel, so the result does not depend on the dpi).

// The formulas' own names (x, y, u, v, a, b, r, ...) read better than long ones here.
#![allow(clippy::many_single_char_names, clippy::similar_names)]

use kurbo::{BezPath, PathEl, Point};

/// The most points one outline is sampled with; a longer outline gets a coarser step.
const MOST_POINTS: f64 = 16_384.0;

/// A closed outline sampled for the shape fit.
#[derive(Debug, Clone)]
pub(crate) struct Sampled {
    /// The area centroid in document millimetres.
    pub(crate) centre: [f64; 2],
    /// The points about the centroid, in outline order (the last joins the first).
    pub(crate) points: Vec<[f64; 2]>,
    /// The signed area (the sign is the winding).
    pub(crate) signed_area: f64,
    /// The central second moments over the area: ∫x², ∫y², ∫xy divided by the area.
    pub(crate) sxx: f64,
    pub(crate) syy: f64,
    pub(crate) sxy: f64,
    /// The outline's length.
    pub(crate) perimeter: f64,
    /// The box of the points (document millimetres): min x, min y, max x, max y.
    pub(crate) bounds: [f64; 4],
}

impl Sampled {
    /// The area.
    pub(crate) fn area(&self) -> f64 {
        self.signed_area.abs()
    }

    /// Point `i` and the one after it.
    pub(crate) fn segment(&self, i: usize) -> ([f64; 2], [f64; 2]) {
        let n = self.points.len();
        (self.points[i], self.points[(i + 1) % n])
    }

    /// Whether the document point `p` lies inside the outline (even-odd over its points).
    pub(crate) fn contains(&self, p: [f64; 2]) -> bool {
        let [minx, miny, maxx, maxy] = self.bounds;
        if p[0] < minx || p[0] > maxx || p[1] < miny || p[1] > maxy {
            return false;
        }
        let (x, y) = (p[0] - self.centre[0], p[1] - self.centre[1]);
        let mut inside = false;
        for i in 0..self.points.len() {
            let (a, b) = self.segment(i);
            if (a[1] > y) != (b[1] > y) {
                let at = a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
                if x < at {
                    inside = !inside;
                }
            }
        }
        inside
    }
}

/// The length of (`x`, `y`) by `sqrt` alone.
pub(crate) fn length(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

/// How many pieces a span of `length` gets at `step`.
fn pieces(length: f64, step: f64) -> u32 {
    // invariant: the count is capped by `MOST_POINTS` before the cast.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = (length / step).ceil().clamp(1.0, MOST_POINTS) as u32;
    n
}

/// `path` sampled about every `step` millimetres, when it is one closed outline with an area.
pub(crate) fn sample_closed(path: &BezPath, step: f64) -> Option<Sampled> {
    let elements = path.elements();
    let mut segments: Vec<PathEl> = Vec::with_capacity(elements.len());
    let mut start = None;
    let mut closed = false;
    for element in elements {
        match *element {
            PathEl::MoveTo(p) if start.is_none() => start = Some(p),
            PathEl::ClosePath if !closed => closed = true,
            PathEl::LineTo(_) | PathEl::QuadTo(..) | PathEl::CurveTo(..) if !closed => {
                segments.push(*element);
            }
            _ => return None,
        }
    }
    let start = start?;
    if !closed || segments.is_empty() {
        return None;
    }
    // The control polygon bounds the length; the step grows for very long outlines.
    let mut total = 0.0;
    let mut at = start;
    for segment in &segments {
        let (len, end) = control_length(at, segment);
        total += len;
        at = end;
    }
    total += length(start.x - at.x, start.y - at.y);
    let step = step.max(total / MOST_POINTS);
    let mut points: Vec<[f64; 2]> = vec![[start.x, start.y]];
    let mut at = start;
    for segment in &segments {
        at = push_segment(&mut points, at, segment, step);
    }
    push_segment(&mut points, at, &PathEl::LineTo(start), step);
    // The closing point repeats the first (twice when the last segment ends at the start).
    while points.len() > 1 && points.last() == points.first() {
        points.pop();
    }
    moments(points)
}

/// The control polygon's length of `segment` from `from`, and its end.
fn control_length(from: Point, segment: &PathEl) -> (f64, Point) {
    let d = |a: Point, b: Point| length(b.x - a.x, b.y - a.y);
    match *segment {
        PathEl::LineTo(p) => (d(from, p), p),
        PathEl::QuadTo(c, p) => (d(from, c) + d(c, p), p),
        PathEl::CurveTo(c1, c2, p) => (d(from, c1) + d(c1, c2) + d(c2, p), p),
        PathEl::MoveTo(p) => (0.0, p),
        PathEl::ClosePath => (0.0, from),
    }
}

/// Pushes the points of `segment` after `from` (its end included) and returns its end.
fn push_segment(points: &mut Vec<[f64; 2]>, from: Point, segment: &PathEl, step: f64) -> Point {
    let (len, end) = control_length(from, segment);
    let n = pieces(len, step);
    for k in 1..=n {
        let t = f64::from(k) / f64::from(n);
        let u = 1.0 - t;
        let p = match *segment {
            PathEl::QuadTo(c, p) => [
                u * u * from.x + 2.0 * u * t * c.x + t * t * p.x,
                u * u * from.y + 2.0 * u * t * c.y + t * t * p.y,
            ],
            PathEl::CurveTo(c1, c2, p) => {
                let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                [
                    a * from.x + b * c1.x + c * c2.x + d * p.x,
                    a * from.y + b * c1.y + c * c2.y + d * p.y,
                ]
            }
            _ => [u * from.x + t * end.x, u * from.y + t * end.y],
        };
        points.push(p);
    }
    end
}

/// The centroid, the moments and the points about the centroid of the closed polygon `points`.
fn moments(points: Vec<[f64; 2]>) -> Option<Sampled> {
    let n = points.len();
    if n < 3 {
        return None;
    }
    // About the first point, so large coordinates do not cancel.
    let o = points[0];
    let (mut a2, mut cx, mut cy, mut xx, mut yy, mut xy, mut perimeter) =
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    let mut bounds = [o[0], o[1], o[0], o[1]];
    for i in 0..n {
        let (p, q) = (points[i], points[(i + 1) % n]);
        let (x0, y0, x1, y1) = (p[0] - o[0], p[1] - o[1], q[0] - o[0], q[1] - o[1]);
        let cross = x0 * y1 - x1 * y0;
        a2 += cross;
        cx += (x0 + x1) * cross;
        cy += (y0 + y1) * cross;
        xx += (x0 * x0 + x0 * x1 + x1 * x1) * cross;
        yy += (y0 * y0 + y0 * y1 + y1 * y1) * cross;
        xy += (x0 * y1 + 2.0 * x0 * y0 + 2.0 * x1 * y1 + x1 * y0) * cross;
        perimeter += length(x1 - x0, y1 - y0);
        bounds = [
            bounds[0].min(p[0]),
            bounds[1].min(p[1]),
            bounds[2].max(p[0]),
            bounds[3].max(p[1]),
        ];
    }
    let area = a2 / 2.0;
    if area == 0.0 || !area.is_finite() {
        return None;
    }
    let (mx, my) = (cx / (6.0 * area), cy / (6.0 * area));
    let sxx = xx / (12.0 * area) - mx * mx;
    let syy = yy / (12.0 * area) - my * my;
    let sxy = xy / (24.0 * area) - mx * my;
    let centre = [o[0] + mx, o[1] + my];
    let points = points
        .into_iter()
        .map(|p| [p[0] - centre[0], p[1] - centre[1]])
        .collect();
    Some(Sampled {
        centre,
        points,
        signed_area: area,
        sxx,
        syy,
        sxy,
        perimeter,
        bounds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rectangle_has_its_area_centroid_and_moments() {
        let mut p = BezPath::new();
        p.move_to((10.0, 20.0));
        p.line_to((50.0, 20.0));
        p.line_to((50.0, 40.0));
        p.line_to((10.0, 40.0));
        p.close_path();
        let s = sample_closed(&p, 0.1).unwrap();
        assert!((s.area() - 800.0).abs() < 1e-9);
        assert!((s.centre[0] - 30.0).abs() < 1e-9 && (s.centre[1] - 30.0).abs() < 1e-9);
        assert!((s.sxx - 1600.0 / 12.0).abs() < 1e-9);
        assert!((s.syy - 400.0 / 12.0).abs() < 1e-9);
        assert!(s.sxy.abs() < 1e-9);
        assert!((s.perimeter - 120.0).abs() < 1e-9);
        assert_eq!(s.points.len(), 1200);
        assert!(s.contains([30.0, 30.0]) && !s.contains([5.0, 30.0]));
    }

    #[test]
    fn open_or_several_outlines_are_not_sampled() {
        let mut open = BezPath::new();
        open.move_to((0.0, 0.0));
        open.line_to((1.0, 0.0));
        open.line_to((1.0, 1.0));
        assert!(sample_closed(&open, 0.1).is_none());
        let mut two = open.clone();
        two.close_path();
        two.move_to((5.0, 5.0));
        two.line_to((6.0, 5.0));
        two.line_to((6.0, 6.0));
        two.close_path();
        assert!(sample_closed(&two, 0.1).is_none());
    }
}
