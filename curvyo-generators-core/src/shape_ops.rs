//! Measures and cuts of one footprint shape: its ends, its length on the inner face and the part
//! between two distances.

use curvyo_document_core::Point;

use crate::footprint::{Shape, split};
use crate::outline::polar;

/// Where the walk of `shape` starts.
pub(crate) fn start(shape: &Shape) -> Point {
    match *shape {
        Shape::Straight { from, .. } => from,
        Shape::Bend {
            centre,
            radius,
            from,
            ..
        } => polar(centre, radius, from),
    }
}

/// Where the walk of `shape` ends.
pub(crate) fn end(shape: &Shape) -> Point {
    match *shape {
        Shape::Straight { to, .. } => to,
        Shape::Bend {
            centre,
            radius,
            from,
            sweep,
        } => polar(centre, radius, from + sweep),
    }
}

/// The length of `shape` on the inner face.
pub(crate) fn length(shape: &Shape) -> f64 {
    match *shape {
        Shape::Straight { from, to } => libm::hypot(to.x - from.x, to.y - from.y),
        Shape::Bend { radius, sweep, .. } => radius * sweep.abs(),
    }
}

/// The unit direction of the walk at the start or the end of `shape`.
pub(crate) fn direction(shape: &Shape, at_end: bool) -> (f64, f64) {
    match *shape {
        Shape::Straight { from, to } => {
            let l = length(shape).max(1e-12);
            ((to.x - from.x) / l, (to.y - from.y) / l)
        }
        Shape::Bend { from, sweep, .. } => {
            let angle = if at_end { from + sweep } else { from };
            let s = sweep.signum();
            (-s * libm::sin(angle), s * libm::cos(angle))
        }
    }
}

/// The part of `shape` from distance `a` to distance `b` along it.
pub(crate) fn between(shape: &Shape, a: f64, b: f64) -> Shape {
    let after = if a > 1e-12 {
        split(shape, a).1.unwrap_or(*shape)
    } else {
        *shape
    };
    let keep = b - a;
    if keep >= length(&after) - 1e-12 {
        after
    } else {
        split(&after, keep.max(0.0)).0
    }
}

/// The point at distance `at` along `shape`.
pub(crate) fn point_at(shape: &Shape, at: f64) -> Point {
    if at <= 1e-12 {
        return start(shape);
    }
    end(&between(shape, 0.0, at))
}

/// The signed turn of the walk along `shape` (radians, positive clockwise on screen).
pub(crate) fn turn(shape: &Shape) -> f64 {
    match *shape {
        Shape::Straight { .. } => 0.0,
        Shape::Bend { sweep, .. } => sweep,
    }
}

/// Consecutive shapes measured as one: their start distances and the total length.
#[derive(Debug, Clone)]
pub(crate) struct Chain {
    pub(crate) shapes: Vec<Shape>,
    starts: Vec<f64>,
    pub(crate) total: f64,
}

impl Chain {
    pub(crate) fn new(shapes: Vec<Shape>) -> Self {
        let mut starts = Vec::with_capacity(shapes.len());
        let mut total = 0.0;
        for shape in &shapes {
            starts.push(total);
            total += length(shape);
        }
        Self {
            shapes,
            starts,
            total,
        }
    }

    /// The point at distance `u` along the chain.
    pub(crate) fn at(&self, u: f64) -> Point {
        let u = u.clamp(0.0, self.total);
        let index = self
            .starts
            .partition_point(|start| *start <= u)
            .saturating_sub(1)
            .min(self.shapes.len() - 1);
        point_at(&self.shapes[index], u - self.starts[index])
    }

    /// The shapes between the distances `a` and `b`.
    pub(crate) fn part(&self, a: f64, b: f64) -> Vec<Shape> {
        self.shapes
            .iter()
            .zip(&self.starts)
            .filter_map(|(shape, &start)| {
                let (lo, hi) = (a.max(start), b.min(start + length(shape)));
                (hi - lo > 1e-9).then(|| between(shape, lo - start, hi - start))
            })
            .collect()
    }

    /// The signed turn of the walk between the distances `a` and `b`.
    pub(crate) fn turn_between(&self, a: f64, b: f64) -> f64 {
        self.part(a, b).iter().map(turn).sum()
    }
}
