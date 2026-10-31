//! Nests closed outlines into filled shapes, each an outer outline with its holes.

use kurbo::{Point, Rect};

use super::sample::{Ring, inside, signed_area};

/// One sampled closed outline and the input outline it came from.
#[derive(Debug, Clone)]
pub(crate) struct Loop {
    /// The points in document millimetres.
    pub points: Ring,
    /// The index of the input outline.
    pub outline: usize,
    /// The enclosed area (unsigned).
    pub area: f64,
    /// The bounding box.
    pub bbox: Rect,
}

impl Loop {
    pub(crate) fn new(points: Ring, outline: usize) -> Self {
        let area = signed_area(&points).abs();
        let bbox = bounds(&points);
        Self {
            points,
            outline,
            area,
            bbox,
        }
    }
}

/// One filled shape: an outer loop and the loops of its holes.
#[derive(Debug, Clone)]
pub(crate) struct Blob {
    /// The outer loop first, then the holes, as indices into the loops.
    pub loops: Vec<usize>,
    /// The outer loop's bounding box.
    pub bbox: Rect,
}

impl Blob {
    /// The number of holes.
    pub(crate) fn holes(&self) -> usize {
        self.loops.len() - 1
    }

    /// The centre of the bounding box.
    pub(crate) fn centre(&self) -> Point {
        self.bbox.center()
    }
}

/// The bounding box of points (empty at the origin for none).
pub(crate) fn bounds(points: &[Point]) -> Rect {
    let Some(first) = points.first() else {
        return Rect::ZERO;
    };
    let mut rect = Rect::from_points(*first, *first);
    for p in points {
        rect.x0 = rect.x0.min(p.x);
        rect.y0 = rect.y0.min(p.y);
        rect.x1 = rect.x1.max(p.x);
        rect.y1 = rect.y1.max(p.y);
    }
    rect
}

fn contains(outer: &Loop, inner: &Loop) -> bool {
    outer.area > inner.area
        && outer.bbox.x0 <= inner.bbox.x0
        && outer.bbox.y0 <= inner.bbox.y0
        && outer.bbox.x1 >= inner.bbox.x1
        && outer.bbox.y1 >= inner.bbox.y1
        && inside(&outer.points, inner.points[0])
}

/// Nests `loops` by even-odd containment: a loop inside an even number of others is an outer
/// loop, inside an odd number a hole of the smallest loop around it. Blobs come in the order of
/// their outer loops.
pub(crate) fn nest(loops: &[Loop]) -> Vec<Blob> {
    let n = loops.len();
    let mut depth = vec![0_usize; n];
    let mut parent: Vec<Option<usize>> = vec![None; n];
    for (i, inner) in loops.iter().enumerate() {
        for (j, outer) in loops.iter().enumerate() {
            if i != j && contains(outer, inner) {
                depth[i] += 1;
                let smaller = parent[i].is_none_or(|p| loops[j].area < loops[p].area);
                if smaller {
                    parent[i] = Some(j);
                }
            }
        }
    }
    let mut blob_of: Vec<Option<usize>> = vec![None; n];
    let mut blobs: Vec<Blob> = Vec::new();
    for i in 0..n {
        if depth[i].is_multiple_of(2) {
            blob_of[i] = Some(blobs.len());
            blobs.push(Blob {
                loops: vec![i],
                bbox: loops[i].bbox,
            });
        }
    }
    for i in 0..n {
        if !depth[i].is_multiple_of(2)
            && let Some(blob) = parent[i].and_then(|p| blob_of[p])
        {
            blobs[blob].loops.push(i);
        }
    }
    blobs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64, y: f64, side: f64) -> Ring {
        vec![
            Point::new(x, y),
            Point::new(x + side, y),
            Point::new(x + side, y + side),
            Point::new(x, y + side),
        ]
    }

    #[test]
    fn holes_and_islands_nest_by_even_odd() {
        let loops = vec![
            Loop::new(square(0.0, 0.0, 10.0), 0),
            Loop::new(square(2.0, 2.0, 6.0), 1),
            Loop::new(square(4.0, 4.0, 2.0), 2),
            Loop::new(square(20.0, 0.0, 5.0), 3),
        ];
        let blobs = nest(&loops);
        assert_eq!(blobs.len(), 3);
        assert_eq!(blobs[0].loops, vec![0, 1]);
        assert_eq!(blobs[0].holes(), 1);
        assert_eq!(blobs[1].loops, vec![2]);
        assert_eq!(blobs[2].loops, vec![3]);
    }
}
