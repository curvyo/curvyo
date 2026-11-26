//! The floating-point measures Bridge takes on grid polygons: nearest points, normals, arc length
//! and containment, in grid units with `+ - * /` and `sqrt` only, so they are the same on every
//! target (`specs/0156-fused-lettering/adrs.md` decision 4).

use crate::boolean_grid::{Path64, Point64};

/// A point in grid units, as floating point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct P {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

impl P {
    pub(crate) const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    #[allow(clippy::cast_precision_loss)] // |units| <= 10^10, exact in an f64
    pub(crate) fn of(point: Point64) -> Self {
        Self::new(point.x as f64, point.y as f64)
    }

    pub(crate) fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }

    pub(crate) fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }

    pub(crate) fn scale(self, factor: f64) -> Self {
        Self::new(self.x * factor, self.y * factor)
    }

    pub(crate) fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    pub(crate) fn cross(self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }

    pub(crate) fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    /// The vector scaled to length one; zero stays zero.
    pub(crate) fn unit(self) -> Self {
        let length = self.length();
        if length > 0.0 {
            self.scale(1.0 / length)
        } else {
            self
        }
    }

    /// The point snapped to the grid.
    // Within the kernel's range after the operand checks, so the cast is exact.
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn snap(self) -> Point64 {
        Point64::new(self.x.round() as i64, self.y.round() as i64)
    }

    /// Orders by x, then y (`total_cmp`, so the order is total).
    pub(crate) fn cmp_xy(self, other: Self) -> std::cmp::Ordering {
        self.x
            .total_cmp(&other.x)
            .then_with(|| self.y.total_cmp(&other.y))
    }
}

/// The point of a polygon nearest to a query: on edge `edge` (from vertex `edge` to the next) at
/// parameter `t`, and its squared distance.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Near {
    pub(crate) point: P,
    pub(crate) edge: usize,
    pub(crate) t: f64,
    pub(crate) distance_squared: f64,
}

/// The parameter in `0..=1` of the point of `a b` nearest to `q`.
pub(crate) fn segment_parameter(q: P, a: P, b: P) -> f64 {
    let d = b.sub(a);
    let length_squared = d.dot(d);
    if length_squared == 0.0 {
        return 0.0;
    }
    (q.sub(a).dot(d) / length_squared).clamp(0.0, 1.0)
}

/// Vertex `index` of `polygon`, wrapping around.
pub(crate) fn vertex(polygon: &Path64, index: usize) -> P {
    P::of(polygon[index % polygon.len()])
}

/// The point of the closed `polygon` nearest to `q`; the first edge wins a tie.
#[allow(clippy::many_single_char_names)] // the usual names of plane geometry
pub(crate) fn nearest_on(polygon: &Path64, q: P) -> Near {
    let mut best = Near {
        point: vertex(polygon, 0),
        edge: 0,
        t: 0.0,
        distance_squared: f64::INFINITY,
    };
    for edge in 0..polygon.len() {
        let (a, b) = (vertex(polygon, edge), vertex(polygon, edge + 1));
        let t = segment_parameter(q, a, b);
        let point = a.add(b.sub(a).scale(t));
        let d = point.sub(q);
        let distance_squared = d.dot(d);
        if distance_squared < best.distance_squared {
            best = Near {
                point,
                edge,
                t,
                distance_squared,
            };
        }
    }
    best
}

/// The normal of edge `edge` pointing to its right in a Y-up plane: out of a region the polygon
/// bounds with positive area, into the region of a hole.
fn edge_normal(polygon: &Path64, edge: usize) -> P {
    let d = vertex(polygon, edge + 1).sub(vertex(polygon, edge));
    P::new(d.y, -d.x).unit()
}

/// The outward normal of `polygon` (positive area) at the nearest point `near`: the edge normal,
/// or at a vertex the bisector of the two edge normals.
pub(crate) fn normal_at(polygon: &Path64, near: &Near) -> P {
    let n = polygon.len();
    let at_vertex = if near.t <= 0.0 {
        Some(near.edge)
    } else if near.t >= 1.0 {
        Some((near.edge + 1) % n)
    } else {
        None
    };
    match at_vertex {
        Some(v) => edge_normal(polygon, (v + n - 1) % n)
            .add(edge_normal(polygon, v))
            .unit(),
        None => edge_normal(polygon, near.edge),
    }
}

/// The cumulative arc length at each vertex of the closed `polygon`, and the perimeter last.
pub(crate) fn arc_lengths(polygon: &Path64) -> Vec<f64> {
    let mut lengths = Vec::with_capacity(polygon.len() + 1);
    let mut total = 0.0;
    lengths.push(0.0);
    for edge in 0..polygon.len() {
        total += vertex(polygon, edge + 1)
            .sub(vertex(polygon, edge))
            .length();
        lengths.push(total);
    }
    lengths
}

/// The point at arc length `s` (taken modulo the perimeter) along the closed `polygon`.
pub(crate) fn point_at(polygon: &Path64, lengths: &[f64], s: f64) -> P {
    let perimeter = lengths[lengths.len() - 1];
    let s = if perimeter > 0.0 {
        s.rem_euclid(perimeter)
    } else {
        0.0
    };
    let edge = lengths
        .partition_point(|&l| l <= s)
        .saturating_sub(1)
        .min(polygon.len() - 1);
    let (a, b) = (vertex(polygon, edge), vertex(polygon, edge + 1));
    let span = lengths[edge + 1] - lengths[edge];
    let t = if span > 0.0 {
        (s - lengths[edge]) / span
    } else {
        0.0
    };
    a.add(b.sub(a).scale(t))
}

/// The arc length of the point at `t` on edge `edge`.
pub(crate) fn arc_at(lengths: &[f64], edge: usize, t: f64) -> f64 {
    lengths[edge] + (lengths[edge + 1] - lengths[edge]) * t
}

/// The distance between two arc lengths along a closed outline of perimeter `perimeter`, the
/// shorter way round.
pub(crate) fn along_distance(a: f64, b: f64, perimeter: f64) -> f64 {
    let d = (a - b).abs().rem_euclid(perimeter);
    d.min(perimeter - d)
}

/// The winding number of the closed `polygon` around `q`.
pub(crate) fn winding(polygon: &Path64, q: P) -> i32 {
    let mut winding = 0;
    for edge in 0..polygon.len() {
        let (a, b) = (vertex(polygon, edge), vertex(polygon, edge + 1));
        let side = b.sub(a).cross(q.sub(a));
        if a.y <= q.y {
            if b.y > q.y && side > 0.0 {
                winding += 1;
            }
        } else if b.y <= q.y && side < 0.0 {
            winding -= 1;
        }
    }
    winding
}

/// Whether the open segments `a b` and `c d` cross or touch.
pub(crate) fn segments_meet(a: P, b: P, c: P, d: P) -> bool {
    let d1 = b.sub(a).cross(c.sub(a));
    let d2 = b.sub(a).cross(d.sub(a));
    let d3 = d.sub(c).cross(a.sub(c));
    let d4 = d.sub(c).cross(b.sub(c));
    (d1 * d2 <= 0.0) && (d3 * d4 <= 0.0) && !(d1 == 0.0 && d2 == 0.0)
}

/// A rectangle polygon from the centre line `a b` with half width `half`, wound with positive
/// area.
pub(crate) fn ribbon_quad(a: P, b: P, half: f64) -> Path64 {
    let along = b.sub(a).unit();
    let side = P::new(-along.y, along.x).scale(half);
    let quad = vec![
        a.add(side).snap(),
        a.sub(side).snap(),
        b.sub(side).snap(),
        b.add(side).snap(),
    ];
    positive(quad)
}

/// `path` wound with a positive area.
pub(crate) fn positive(mut path: Path64) -> Path64 {
    if crate::boolean_grid::double_area(&path) < 0 {
        path.reverse();
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Path64 {
        vec![
            Point64::new(0, 0),
            Point64::new(10, 0),
            Point64::new(10, 10),
            Point64::new(0, 10),
        ]
    }

    #[test]
    fn the_nearest_point_and_its_outward_normal() {
        let near = nearest_on(&square(), P::new(15.0, 4.0));
        assert_eq!(near.point, P::new(10.0, 4.0));
        assert_eq!(normal_at(&square(), &near), P::new(1.0, 0.0));
        let corner = nearest_on(&square(), P::new(12.0, 12.0));
        let n = normal_at(&square(), &corner);
        assert!((n.x - n.y).abs() < 1e-12 && n.x > 0.0);
    }

    #[test]
    fn arc_length_walks_round_the_outline() {
        let lengths = arc_lengths(&square());
        assert_eq!(lengths[4], 40.0);
        assert_eq!(point_at(&square(), &lengths, 15.0), P::new(10.0, 5.0));
        assert_eq!(point_at(&square(), &lengths, 45.0), P::new(5.0, 0.0));
        assert_eq!(along_distance(1.0, 39.0, 40.0), 2.0);
    }

    #[test]
    fn winding_and_crossings() {
        assert_eq!(winding(&square(), P::new(5.0, 5.0)), 1);
        assert_eq!(winding(&square(), P::new(15.0, 5.0)), 0);
        let (a, b) = (P::new(0.0, 0.0), P::new(10.0, 10.0));
        assert!(segments_meet(a, b, P::new(0.0, 10.0), P::new(10.0, 0.0)));
        assert!(!segments_meet(a, b, P::new(20.0, 0.0), P::new(30.0, 0.0)));
    }
}
