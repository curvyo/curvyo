//! The outline a job's frame traces: the box or the convex hull of the burn moves, in machine
//! coordinates (`specs/0118` criteria 23, 24; adrs F1).

use crate::job_input::PointUm;
use crate::job_moves::{Job, MoveKind};

/// The shape a frame traces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrameShape {
    /// The bounding box of every burn move (the default).
    #[default]
    Rectangle,
    /// The convex hull of every burn move (the rubber band outline of other programs).
    RubberBand,
}

/// The points a frame of `job` traces in machine micrometres, counter-clockwise, starting at the
/// point nearest the machine origin; the caller closes the loop. `None` when no move burns
/// (nothing to frame). Travel, run-out and Z moves are ignored, fixture blocks count.
#[must_use]
pub fn outline(job: &Job, shape: FrameShape) -> Option<Vec<PointUm>> {
    let points = burn_points(job);
    if points.is_empty() {
        return None;
    }
    let ring = match shape {
        FrameShape::Rectangle => rectangle(&points),
        FrameShape::RubberBand => hull(&points),
    };
    Some(from_nearest(ring))
}

/// Both ends of every burn and sweep move, in machine coordinates.
fn burn_points(job: &Job) -> Vec<PointUm> {
    let mut head = job.home;
    let mut points = Vec::new();
    for m in &job.moves {
        if matches!(m.kind, MoveKind::Burn { .. } | MoveKind::Sweep { .. }) {
            points.push(job.frame.machine_point(head));
            points.push(job.frame.machine_point(m.to));
        }
        head = m.to;
    }
    points
}

fn rectangle(points: &[PointUm]) -> Vec<PointUm> {
    let (mut lo, mut hi) = (points[0], points[0]);
    for p in points {
        lo = PointUm::new(lo.x.min(p.x), lo.y.min(p.y));
        hi = PointUm::new(hi.x.max(p.x), hi.y.max(p.y));
    }
    let mut ring = vec![lo, PointUm::new(hi.x, lo.y), hi, PointUm::new(lo.x, hi.y)];
    ring.dedup();
    if ring.len() > 1 && ring.first() == ring.last() {
        ring.pop();
    }
    ring
}

fn cross(o: PointUm, a: PointUm, b: PointUm) -> i128 {
    i128::from(a.x - o.x) * i128::from(b.y - o.y) - i128::from(a.y - o.y) * i128::from(b.x - o.x)
}

/// Andrew's monotone chain with exact integer cross products; collinear points are dropped.
fn hull(points: &[PointUm]) -> Vec<PointUm> {
    let mut sorted: Vec<PointUm> = points.to_vec();
    sorted.sort_by_key(|p| (p.x, p.y));
    sorted.dedup();
    if sorted.len() < 3 {
        return sorted;
    }
    let mut ring: Vec<PointUm> = Vec::with_capacity(sorted.len() * 2);
    for pass in 0..2 {
        let floor = if pass == 0 { 2 } else { ring.len() + 1 };
        let iter: Box<dyn Iterator<Item = &PointUm>> = if pass == 0 {
            Box::new(sorted.iter())
        } else {
            Box::new(sorted.iter().rev().skip(1))
        };
        for &p in iter {
            while ring.len() >= floor && cross(ring[ring.len() - 2], ring[ring.len() - 1], p) <= 0 {
                ring.pop();
            }
            ring.push(p);
        }
    }
    ring.pop();
    ring
}

/// `ring` rotated to start at the point nearest the machine origin (ties: smaller X, then Y).
fn from_nearest(mut ring: Vec<PointUm>) -> Vec<PointUm> {
    let nearest = ring
        .iter()
        .enumerate()
        .min_by_key(|(_, p)| (i128::from(p.x).pow(2) + i128::from(p.y).pow(2), p.x, p.y))
        .map_or(0, |(i, _)| i);
    ring.rotate_left(nearest);
    ring
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i64, y: i64) -> PointUm {
        PointUm::new(x, y)
    }

    #[test]
    fn a_rectangle_starts_at_the_corner_nearest_the_origin_and_goes_x_first() {
        let ring = from_nearest(rectangle(&[p(30, 40), p(10, 20), p(50, 5)]));
        assert_eq!(ring, [p(10, 5), p(50, 5), p(50, 40), p(10, 40)]);
    }

    #[test]
    fn a_line_or_a_point_is_a_short_ring() {
        assert_eq!(rectangle(&[p(5, 5), p(5, 5)]), [p(5, 5)]);
        assert_eq!(rectangle(&[p(0, 5), p(9, 5)]), [p(0, 5), p(9, 5)]);
    }

    #[test]
    fn the_hull_is_counter_clockwise_without_inner_or_collinear_points() {
        let points = [
            p(0, 0),
            p(10, 0),
            p(5, 0),
            p(10, 10),
            p(0, 10),
            p(5, 5),
            p(3, 2),
        ];
        let ring = from_nearest(hull(&points));
        assert_eq!(ring, [p(0, 0), p(10, 0), p(10, 10), p(0, 10)]);
        let triangle = from_nearest(hull(&[p(20, 20), p(40, 20), p(30, 40), p(30, 25)]));
        assert_eq!(triangle, [p(20, 20), p(40, 20), p(30, 40)]);
    }

    #[test]
    fn the_hull_starts_at_the_point_nearest_the_origin() {
        let ring = from_nearest(hull(&[p(50, 50), p(90, 50), p(70, 90), p(60, 52)]));
        assert_eq!(ring[0], p(50, 50));
        let ring = from_nearest(hull(&[p(100, 100), p(10, 100), p(10, 10), p(60, 60)]));
        assert_eq!(ring[0], p(10, 10));
        assert_eq!(ring, [p(10, 10), p(100, 100), p(10, 100)]);
    }

    #[test]
    fn two_points_are_a_hull() {
        assert_eq!(hull(&[p(1, 1), p(9, 9), p(1, 1)]), [p(1, 1), p(9, 9)]);
    }
}
