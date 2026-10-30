//! Spatial pruning of finite snapping geometry.
use super::{Point, SnapTarget, TargetAxis};

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct Index {
    vertical: Vec<(f64, usize)>,
    horizontal: Vec<(f64, usize)>,
    points: Option<Box<Branch>>,
}
#[derive(Debug, Clone, PartialEq)]
struct Branch {
    bounds: (Point, Point),
    content: Content,
}
#[derive(Debug, Clone, PartialEq)]
enum Content {
    Leaf(Vec<usize>),
    Split(Box<Branch>, Box<Branch>),
}
impl Branch {
    fn build(mut ids: Vec<usize>, entries: &[SnapTarget]) -> Self {
        let bounds = ids.iter().fold(
            (
                Point::new(f64::INFINITY, f64::INFINITY),
                Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
            ),
            |(lo, hi), &id| {
                let t = &entries[id];
                let (a, b) = if t.curve.is_some() {
                    t.extent
                } else {
                    (t.point, t.point)
                };
                (
                    Point::new(lo.x.min(a.x), lo.y.min(a.y)),
                    Point::new(hi.x.max(b.x), hi.y.max(b.y)),
                )
            },
        );
        let content = if ids.len() <= 8 {
            Content::Leaf(ids)
        } else {
            let x = bounds.1.x - bounds.0.x >= bounds.1.y - bounds.0.y;
            ids.sort_unstable_by(|&a, &b| {
                let pa = entries[a].point;
                let pb = entries[b].point;
                (if x { pa.x } else { pa.y })
                    .total_cmp(&if x { pb.x } else { pb.y })
                    .then(a.cmp(&b))
            });
            let right = ids.split_off(ids.len() / 2);
            Content::Split(
                Box::new(Self::build(ids, entries)),
                Box::new(Self::build(right, entries)),
            )
        };
        Self { bounds, content }
    }
    fn query(&self, p: Point, radius: f64, result: &mut Vec<usize>) {
        if self.bounds.1.x < p.x - radius
            || self.bounds.0.x > p.x + radius
            || self.bounds.1.y < p.y - radius
            || self.bounds.0.y > p.y + radius
        {
            return;
        }
        match &self.content {
            Content::Leaf(ids) => result.extend(ids),
            Content::Split(left, right) => {
                left.query(p, radius, result);
                right.query(p, radius, result);
            }
        }
    }
}
impl Index {
    pub(super) fn build(entries: &[SnapTarget]) -> Self {
        let mut index = Self::default();
        let mut points = Vec::new();
        for (id, target) in entries.iter().enumerate() {
            match target.axis {
                TargetAxis::Vertical => index.vertical.push((target.point.x, id)),
                TargetAxis::Horizontal => index.horizontal.push((target.point.y, id)),
                TargetAxis::Both => points.push(id),
            }
        }
        for lines in [&mut index.vertical, &mut index.horizontal] {
            lines.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        }
        if !points.is_empty() {
            index.points = Some(Box::new(Branch::build(points, entries)));
        }
        index
    }
    pub(super) fn lines(&self, axis: TargetAxis, value: f64, radius: f64) -> &[(f64, usize)] {
        let lines = if axis == TargetAxis::Vertical {
            &self.vertical
        } else {
            &self.horizontal
        };
        let start = lines.partition_point(|p| p.0 < value - radius);
        let end = lines.partition_point(|p| p.0 <= value + radius);
        &lines[start..end]
    }
    pub(super) fn points(&self, point: Point, radius: f64) -> Vec<usize> {
        let mut result = Vec::new();
        if let Some(root) = &self.points {
            root.query(point, radius, &mut result);
        }
        result
    }
}
