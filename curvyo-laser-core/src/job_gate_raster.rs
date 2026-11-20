//! The raster checks of one page (ADR 0069 §6, `specs/0123` criteria 31 to 36): Grey on M3, the
//! dot limit, coarse, overlapping, reduced and unreadable images, Lowest power above Power.

use std::collections::HashSet;

use curvyo_document_core::{NodeId, Point, Tolerance};
use curvyo_geometry_core::flatten_outline;

use crate::job_build::Built;
use crate::job_flatten::FLATTEN_MM;
use crate::job_gate::{Issue, Targets};
use crate::job_gate_cause::Cause;
use crate::job_input::{BlockInput, JobInput, LeafGeometry, LeafInput, PointUm};
use crate::job_numbers::NumbersRead;
use crate::job_raster_grid::estimated_dots;
use crate::job_raster_plan::MAX_DOTS;
use crate::job_raster_plan::burns_grey;
use crate::job_raster_sample::pixel_pitch_mm;
use crate::operation_kind::OperationKind;

/// How far two frames must reach into each other to overlap, in mm (an edge shared within this
/// is no overlap).
const OVERLAP_MM: f64 = 0.001;

/// The raster leaves of `block` that run as raster items: images and gradient-filled leaves of
/// an Image operation (ADR 0069 §13).
pub(crate) fn raster_leaves<'a>(
    block: &'a BlockInput,
    read: &NumbersRead,
) -> impl Iterator<Item = &'a LeafInput> {
    let rasters = read
        .numbers()
        .is_some_and(|n| n.kind == OperationKind::Image);
    block.leaves.iter().filter(move |leaf| {
        rasters
            && matches!(
                leaf.geometry,
                LeafGeometry::Image { .. } | LeafGeometry::Gradient { .. }
            )
    })
}

/// The points that bound a raster leaf: an image's frame, a gradient leaf's outlines flattened
/// as the job flattens them.
pub(crate) fn bounding_points(leaf: &LeafInput) -> Vec<Point> {
    match &leaf.geometry {
        LeafGeometry::Image { frame, .. } => frame.to_vec(),
        LeafGeometry::Gradient { outlines, .. } | LeafGeometry::Outlines(outlines) => outlines
            .iter()
            .flat_map(|o| flatten_outline(&o.anchors, o.closed, Tolerance::from_mm(FLATTEN_MM)))
            .collect(),
    }
}

/// Appends the raster checks of `input` to `issues`.
pub(crate) fn raster_issues(input: &JobInput, built: &Built, issues: &mut Vec<Issue>) {
    let m4 = input.machine.values.laser_mode;
    let mut lists = Lists::default();
    for (index, block) in input.blocks.iter().enumerate() {
        let read = &built.numbers[index];
        let Some(numbers) = read.numbers().filter(|_| built.runs[index]) else {
            continue;
        };
        let leaves: Vec<&LeafInput> = raster_leaves(block, read).collect();
        if leaves.is_empty() {
            continue;
        }
        let image = &numbers.options.image;
        let operation = || Targets::Operations(vec![block.id]);
        // Any raster leaf burning Grey, a picture with its own dither included (ADR 0072
        // amendment A4); the lines stay the operation's (Frage 284 B, Lowest power is its field).
        let grey = burns_grey(image, leaves.iter().copied());
        if grey && !m4 {
            issues.push(issue(Cause::GreyNeedsM4 { op: block.id }, 1, operation()));
        }
        if grey && image.lowest_power_tenths > numbers.main.power.tenths() {
            let cause = Cause::LowestPowerAbovePower { op: block.id };
            issues.push(issue(cause, 1, operation()));
        }
        let interval_um = i64::from(numbers.interval.hundredths()) * 10;
        let angle = numbers.options.scan_angle.degrees();
        for leaf in &leaves {
            let ring: Vec<PointUm> = bounding_points(leaf)
                .into_iter()
                .map(PointUm::of_mm)
                .collect();
            let count = estimated_dots(&[ring], interval_um, angle);
            if count > MAX_DOTS {
                let cause = Cause::TooManyDots {
                    item: leaf.node,
                    count,
                };
                issues.push(issue(cause, 1, Targets::Items(vec![leaf.node])));
            }
            lists.image(leaf, f64::from(numbers.interval.hundredths()) / 100.0);
        }
        lists.overlaps(&leaves);
    }
    lists.issues(issues);
}

/// Items each listed once, in the order first added.
#[derive(Default)]
struct Once {
    list: Vec<NodeId>,
    seen: HashSet<NodeId>,
}

impl Once {
    fn add(&mut self, node: NodeId) {
        if self.seen.insert(node) {
            self.list.push(node);
        }
    }
}

/// The items of the page-wide raster warns, each once, in input order.
#[derive(Default)]
struct Lists {
    coarse: Once,
    overlapping: Once,
    reduced: Once,
    unreadable: Once,
    /// Pictures with steps that cannot run, each once with its count (`specs/0135` criterion 23).
    broken: Vec<(NodeId, usize)>,
}

impl Lists {
    fn image(&mut self, leaf: &LeafInput, interval_mm: f64) {
        let LeafGeometry::Image {
            frame,
            pixels,
            adjust,
            ..
        } = &leaf.geometry
        else {
            return;
        };
        let broken = adjust.broken_steps();
        if broken > 0 && !self.broken.iter().any(|(node, _)| *node == leaf.node) {
            self.broken.push((leaf.node, broken));
        }
        let Some(pixels) = pixels else {
            self.unreadable.add(leaf.node);
            return;
        };
        if pixel_pitch_mm(frame, pixels.width, pixels.height) > 2.0 * interval_mm {
            self.coarse.add(leaf.node);
        }
        if pixels.is_reduced() {
            self.reduced.add(leaf.node);
        }
    }

    /// Adds the raster items of one operation whose frames overlap with an area above 0: images
    /// by their parallelograms, gradient leaves by their boxes (Proposal). Only shapes whose
    /// boxes overlap are compared (disjoint boxes share no area), swept by their left edges.
    fn overlaps(&mut self, leaves: &[&LeafInput]) {
        let shapes: Vec<Vec<Point>> = leaves.iter().map(|leaf| footprint(leaf)).collect();
        let boxes: Vec<(Point, Point)> = shapes.iter().map(|s| bounds(s)).collect();
        let mut by_left: Vec<usize> = (0..shapes.len()).collect();
        by_left.sort_by(|&a, &b| boxes[a].0.x.total_cmp(&boxes[b].0.x));
        let mut open: Vec<usize> = Vec::new();
        let mut pairs = Vec::new();
        for &b in &by_left {
            let (lo, hi) = boxes[b];
            open.retain(|&a| boxes[a].1.x > lo.x);
            for &a in &open {
                let (alo, ahi) = boxes[a];
                let meet = alo.y < hi.y && lo.y < ahi.y;
                if meet && convex_overlap(&shapes[a], &shapes[b]) {
                    pairs.push((a.min(b), a.max(b)));
                }
            }
            open.push(b);
        }
        pairs.sort_unstable();
        for (a, b) in pairs {
            self.overlapping.add(leaves[a].node);
            self.overlapping.add(leaves[b].node);
        }
    }

    fn issues(self, issues: &mut Vec<Issue>) {
        for (cause, items) in [
            (Cause::CoarseImages, self.coarse),
            (Cause::OverlappingRasterItems, self.overlapping),
            (Cause::ReducedImages, self.reduced),
            (Cause::UnreadableImages, self.unreadable),
        ] {
            let items = items.list;
            if !items.is_empty() {
                issues.push(issue(cause, items.len(), Targets::Items(items)));
            }
        }
        for (item, count) in self.broken {
            issues.push(issue(
                Cause::BrokenAdjustments { item },
                count,
                Targets::Items(vec![item]),
            ));
        }
    }
}

/// The box of `points`: its lowest and highest corner.
fn bounds(points: &[Point]) -> (Point, Point) {
    points.iter().fold(
        (
            Point::new(f64::INFINITY, f64::INFINITY),
            Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(lo, hi), p| {
            (
                Point::new(lo.x.min(p.x), lo.y.min(p.y)),
                Point::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        },
    )
}

/// An image's frame, or a gradient leaf's box, as a convex polygon.
fn footprint(leaf: &LeafInput) -> Vec<Point> {
    let points = bounding_points(leaf);
    if matches!(leaf.geometry, LeafGeometry::Image { .. }) {
        return points;
    }
    let (lo, hi) = bounds(&points);
    vec![lo, Point::new(hi.x, lo.y), hi, Point::new(lo.x, hi.y)]
}

/// Whether two convex polygons overlap with an area above 0: no edge normal of either separates
/// them by more than [`OVERLAP_MM`] of shared edge.
fn convex_overlap(a: &[Point], b: &[Point]) -> bool {
    if a.len() < 3 || b.len() < 3 {
        return false;
    }
    let separated = |poly: &[Point]| {
        poly.iter().zip(poly.iter().cycle().skip(1)).any(|(p, q)| {
            let (nx, ny) = (q.y - p.y, p.x - q.x);
            let len = (nx * nx + ny * ny).sqrt();
            if len == 0.0 {
                return false;
            }
            let project = |poly: &[Point]| {
                poly.iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), r| {
                        let v = (r.x * nx + r.y * ny) / len;
                        (lo.min(v), hi.max(v))
                    })
            };
            let ((a0, a1), (b0, b1)) = (project(a), project(b));
            a1 <= b0 + OVERLAP_MM || b1 <= a0 + OVERLAP_MM
        })
    };
    !separated(a) && !separated(b)
}

const fn issue(cause: Cause, count: usize, targets: Targets) -> Issue {
    Issue {
        cause,
        count,
        targets,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64, y: f64, side: f64) -> Vec<Point> {
        vec![
            Point::new(x, y),
            Point::new(x + side, y),
            Point::new(x + side, y + side),
            Point::new(x, y + side),
        ]
    }

    #[test]
    fn frames_that_touch_do_not_overlap_and_frames_that_reach_in_do() {
        assert!(!convex_overlap(
            &square(0.0, 0.0, 10.0),
            &square(10.0, 0.0, 10.0)
        ));
        assert!(convex_overlap(
            &square(0.0, 0.0, 10.0),
            &square(9.0, 9.0, 10.0)
        ));
        assert!(!convex_overlap(
            &square(0.0, 0.0, 10.0),
            &square(20.0, 0.0, 5.0)
        ));
        let diamond = vec![
            Point::new(15.0, 10.0),
            Point::new(20.0, 15.0),
            Point::new(15.0, 20.0),
            Point::new(10.0, 15.0),
        ];
        assert!(
            !convex_overlap(&square(0.0, 0.0, 12.0), &diamond),
            "corners apart"
        );
    }
}
