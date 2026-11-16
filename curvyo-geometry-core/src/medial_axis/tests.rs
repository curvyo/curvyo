//! Step 0 of ADR 0063 Decision 10: the axis of strokes drawn from known centre lines lies within
//! `T(w)/2` of the centre line, its width within `T(w)/2` of the drawn width, and the skeleton of
//! a 20-shape drawing stays within its CPU budget.

use curvyo_document_core::{
    DashPattern, Length, LineCap, LineJoin, Markers, Point as DocPoint, Tolerance, Vec2 as DocVec,
};
use kurbo::{CubicBez, ParamCurve, Point, Vec2};

use super::{MedialAxis, Region, medial_axis, regions};
use crate::boolean_grid::{Path64, Point64, snap_polygon};
use crate::{Outline, OutlineTriple, StrokeShape, WidthPoint, WidthProfile};

/// `T(w) = max(0.05 mm, 3 % of w)`.
fn tolerance(width: f64) -> f64 {
    0.05_f64.max(0.03 * width)
}

/// A stroke drawn along cubic pieces with a width profile.
pub(crate) struct Stroke {
    pub(crate) anchors: Vec<OutlineTriple>,
    pub(crate) closed: bool,
    pub(crate) width: f64,
    pub(crate) profile: Vec<(f64, f64)>,
}

/// A dense sample of a true centre line.
#[derive(Clone, Copy)]
pub(crate) struct Truth {
    pub(crate) point: Point,
    pub(crate) width: f64,
}

pub(crate) fn corner(x: f64, y: f64) -> OutlineTriple {
    (DocPoint::new(x, y), DocVec::ZERO, DocVec::ZERO)
}

/// Anchors of a circular arc around `centre` from `start` to `end` radians, in pieces of at most
/// 45 degrees.
pub(crate) fn arc(centre: Point, radius: f64, start: f64, end: f64) -> Vec<OutlineTriple> {
    let pieces = ((end - start).abs() / std::f64::consts::FRAC_PI_4)
        .ceil()
        .max(1.0);
    let step = (end - start) / pieces;
    let k = 4.0 / 3.0 * (step / 4.0).tan() * radius;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    (0..=pieces as usize)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let angle = start + step * i as f64;
            let (s, c) = angle.sin_cos();
            let tangent = DocVec::new(-s * k, c * k);
            (
                DocPoint::new(centre.x + radius * c, centre.y + radius * s),
                DocVec::new(-tangent.x, -tangent.y),
                tangent,
            )
        })
        .collect()
}

impl Stroke {
    pub(crate) fn line(from: Point, to: Point, width: f64) -> Self {
        Self {
            anchors: vec![corner(from.x, from.y), corner(to.x, to.y)],
            closed: false,
            width,
            profile: vec![(0.0, 1.0), (1.0, 1.0)],
        }
    }

    fn cubics(&self) -> Vec<CubicBez> {
        let n = self.anchors.len();
        let count = if self.closed { n } else { n - 1 };
        (0..count)
            .map(|i| {
                let (a, b) = (self.anchors[i], self.anchors[(i + 1) % n]);
                let p = |q: DocPoint| Point::new(q.x, q.y);
                CubicBez::new(
                    p(a.0),
                    p(a.0) + Vec2::new(a.2.x, a.2.y),
                    p(b.0) + Vec2::new(b.1.x, b.1.y),
                    p(b.0),
                )
            })
            .collect()
    }

    fn width_profile(&self) -> WidthProfile {
        let points = self
            .profile
            .iter()
            .enumerate()
            .map(|(i, &(at, m))| WidthPoint::new(i as u128 + 1, at, m, self.closed).unwrap())
            .collect();
        WidthProfile::new(points, self.closed).unwrap()
    }

    /// The true centre line, every 0.005 mm, with its drawn width.
    pub(crate) fn truth(&self) -> Vec<Truth> {
        let mut dense = Vec::new();
        for cubic in self.cubics() {
            for i in 0..2000 {
                dense.push(cubic.eval(f64::from(i) / 2000.0));
            }
        }
        if let Some(last) = self.cubics().last() {
            dense.push(last.p3);
        }
        let mut lengths = vec![0.0];
        for pair in dense.windows(2) {
            lengths.push(lengths[lengths.len() - 1] + (pair[1] - pair[0]).hypot());
        }
        let total = lengths[lengths.len() - 1];
        let profile = self.width_profile();
        dense
            .iter()
            .zip(&lengths)
            .map(|(&point, &s)| {
                let at = if self.closed {
                    (s / total) % 1.0
                } else {
                    s / total
                };
                Truth {
                    point,
                    width: self.width * profile.sample(at).unwrap(),
                }
            })
            .collect()
    }

    /// The painted outline on the grid.
    pub(crate) fn paths(&self) -> Vec<Path64> {
        let style = StrokeShape {
            width: Length::from_mm(self.width),
            join: LineJoin::Round,
            cap: LineCap::Round,
            dash: DashPattern::default(),
            markers: Markers::default(),
        };
        let result = crate::variable_stroke_outline(
            Outline::new(&self.anchors, self.closed),
            &style,
            &self.width_profile(),
            Tolerance::from_mm(0.002),
        )
        .unwrap();
        result
            .outlines()
            .iter()
            .map(|ring| {
                let triples: Vec<OutlineTriple> = ring
                    .iter()
                    .map(|a| (a.point, a.handle_in, a.handle_out))
                    .collect();
                snap_polygon(&crate::flatten::flatten(
                    &triples,
                    true,
                    super::region::FLATTEN_MM - crate::boolean_grid::GRID_MM,
                ))
            })
            .collect()
    }
}

/// The regions of the union of `strokes`.
pub(crate) fn union(strokes: &[Stroke]) -> Vec<Region> {
    let paths: Vec<Path64> = strokes.iter().flat_map(Stroke::paths).collect();
    super::region::regions_of_paths(&paths)
}

/// Every axis sample, outside the junction zones.
fn samples(axis: &MedialAxis) -> Vec<super::AxisSample> {
    let junctions: Vec<super::AxisSample> = axis
        .edges
        .iter()
        .flat_map(|e| [e.from, e.to])
        .filter(|&v| {
            axis.edges
                .iter()
                .filter(|e| e.from == v || e.to == v)
                .count()
                >= 3
        })
        .map(|v| axis.vertices[v])
        .collect();
    axis.edges
        .iter()
        .flat_map(|e| e.samples.iter().copied())
        .chain(axis.rings.iter().flatten().copied())
        .filter(|s| {
            junctions
                .iter()
                .all(|j| (s.point - j.point).hypot() >= j.radius)
        })
        .collect()
}

/// The worst axis and width error over all samples, as shares of `T(w)/2`.
fn errors(axis: &MedialAxis, truth: &[Truth]) -> (f64, f64) {
    let (mut worst_axis, mut worst_width) = (0.0_f64, 0.0_f64);
    for sample in samples(axis) {
        let nearest = truth
            .iter()
            .min_by(|a, b| {
                (a.point - sample.point)
                    .hypot2()
                    .total_cmp(&(b.point - sample.point).hypot2())
            })
            .unwrap();
        let budget = tolerance(nearest.width) / 2.0;
        worst_axis = worst_axis.max((nearest.point - sample.point).hypot() / budget);
        worst_width = worst_width.max((2.0 * sample.radius - nearest.width).abs() / budget);
    }
    (worst_axis, worst_width)
}

fn single_axis(strokes: &[Stroke]) -> MedialAxis {
    let regions = union(strokes);
    assert_eq!(regions.len(), 1);
    medial_axis(&regions[0]).unwrap()
}

fn check(name: &str, strokes: &[Stroke], edges: usize, rings: usize) {
    let axis = single_axis(strokes);
    let truth: Vec<Truth> = strokes.iter().flat_map(Stroke::truth).collect();
    let (axis_error, width_error) = errors(&axis, &truth);
    eprintln!(
        "{name}: {} edges, {} rings, axis error {:.2}, width error {:.2} of T(w)/2",
        axis.edges.len(),
        axis.rings.len(),
        axis_error,
        width_error
    );
    assert_eq!(
        (axis.edges.len(), axis.rings.len()),
        (edges, rings),
        "{name}"
    );
    assert!(axis_error <= 1.0, "{name}: axis error {axis_error}");
    assert!(width_error <= 1.0, "{name}: width error {width_error}");
}

#[test]
fn a_straight_stroke_has_its_centre_line_as_axis() {
    check(
        "straight",
        &[Stroke::line(
            Point::new(0.0, 0.0),
            Point::new(40.0, 0.0),
            2.0,
        )],
        1,
        0,
    );
}

#[test]
fn an_arc_stroke_has_its_arc_as_axis() {
    let stroke = Stroke {
        anchors: arc(Point::new(0.0, 0.0), 15.0, 0.0, 2.0),
        closed: false,
        width: 2.0,
        profile: vec![(0.0, 1.0), (1.0, 1.0)],
    };
    check("arc", &[stroke], 1, 0);
}

#[test]
fn an_s_stroke_has_its_s_as_axis() {
    let mut anchors = arc(Point::new(0.0, 0.0), 10.0, std::f64::consts::PI, 0.0);
    let second = arc(
        Point::new(20.0, 0.0),
        10.0,
        std::f64::consts::PI,
        2.0 * std::f64::consts::PI,
    );
    let joint = anchors.pop().unwrap();
    anchors.push((joint.0, joint.1, second[0].2));
    anchors.extend_from_slice(&second[1..]);
    let stroke = Stroke {
        anchors,
        closed: false,
        width: 2.0,
        profile: vec![(0.0, 1.0), (1.0, 1.0)],
    };
    check("s", &[stroke], 1, 0);
}

#[test]
fn a_taper_keeps_its_width_along_the_axis() {
    let profile = (0..=30)
        .map(|i| {
            let at = f64::from(i) / 30.0;
            (at, 1.0 - at * (2.5 / 3.0))
        })
        .collect();
    let stroke = Stroke {
        anchors: vec![corner(0.0, 0.0), corner(30.0, 0.0)],
        closed: false,
        width: 3.0,
        profile,
    };
    check("taper", &[stroke], 1, 0);
}

#[test]
fn a_ring_is_one_closed_axis() {
    let mut anchors = arc(Point::new(0.0, 0.0), 5.0, 0.0, 2.0 * std::f64::consts::PI);
    anchors.pop();
    let stroke = Stroke {
        anchors,
        closed: true,
        width: 2.0,
        profile: vec![(0.0, 1.0)],
    };
    check("ring", &[stroke], 0, 1);
}

/// The thinnest curved case: at 0.5 mm the flattening's clearance error is closest to `T(w)/2`.
#[test]
fn a_thin_ring_is_one_closed_axis() {
    for radius in [0.6, 2.0] {
        let mut anchors = arc(
            Point::new(0.0, 0.0),
            radius,
            0.0,
            2.0 * std::f64::consts::PI,
        );
        anchors.pop();
        let stroke = Stroke {
            anchors,
            closed: true,
            width: 0.5,
            profile: vec![(0.0, 1.0)],
        };
        check("thin ring", &[stroke], 0, 1);
    }
}

#[test]
fn a_t_has_three_edges_at_one_junction() {
    check(
        "t",
        &[
            Stroke::line(Point::new(-20.0, 0.0), Point::new(20.0, 0.0), 2.0),
            Stroke::line(Point::new(0.0, 0.0), Point::new(0.0, 20.0), 2.0),
        ],
        3,
        0,
    );
}

#[test]
fn a_rounded_end_stops_at_its_end_circle_centre() {
    let axis = single_axis(&[Stroke::line(
        Point::new(0.0, 0.0),
        Point::new(40.0, 0.0),
        2.0,
    )]);
    let samples = &axis.edges[0].samples;
    let ends = [samples[0].point, samples[samples.len() - 1].point];
    let (lo, hi) = if ends[0].x < ends[1].x {
        (ends[0], ends[1])
    } else {
        (ends[1], ends[0])
    };
    eprintln!("ends {lo:?} {hi:?}");
    assert!((lo - Point::new(0.0, 0.0)).hypot() <= tolerance(2.0));
    assert!((hi - Point::new(40.0, 0.0)).hypot() <= tolerance(2.0));
}

#[test]
fn two_holes_touching_at_a_vertex_triangulate() {
    let square = |x0: i64, y0: i64, size: i64, hole: bool| -> Path64 {
        let mut ring = vec![
            Point64::new(x0, y0),
            Point64::new(x0 + size, y0),
            Point64::new(x0 + size, y0 + size),
            Point64::new(x0, y0 + size),
        ];
        if hole {
            ring.reverse();
        }
        ring
    };
    let region = Region {
        outer: square(0, 0, 20_000, false),
        holes: vec![
            square(2_000, 2_000, 8_000, true),
            square(10_000, 10_000, 8_000, true),
        ],
    };
    let axis = medial_axis(&region).unwrap();
    assert!(!axis.edges.is_empty() || !axis.rings.is_empty());
}

/// The CPU time this thread has used.
fn thread_time() -> std::time::Duration {
    #[cfg(unix)]
    {
        let now = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
        std::time::Duration::new(
            u64::try_from(now.tv_sec).unwrap(),
            u32::try_from(now.tv_nsec).unwrap(),
        )
    }
    #[cfg(not(unix))]
    {
        use std::sync::OnceLock;
        static START: OnceLock<std::time::Instant> = OnceLock::new();
        START.get_or_init(std::time::Instant::now).elapsed()
    }
}

/// Twenty strokes laid out like a line-art whale: body arcs, S curves, tapers, rings, a T.
pub(crate) fn synthetic_whale() -> Vec<Stroke> {
    let mut strokes = Vec::new();
    for i in 0..4 {
        let y = f64::from(i) * 30.0;
        strokes.push(Stroke {
            anchors: arc(
                Point::new(60.0, y + 40.0),
                50.0 - f64::from(i) * 5.0,
                3.4,
                6.0,
            ),
            closed: false,
            width: 1.5 + f64::from(i) * 0.4,
            profile: vec![(0.0, 0.5), (0.5, 1.0), (1.0, 0.4)],
        });
    }
    for i in 0..4 {
        let x = 150.0 + f64::from(i) * 25.0;
        let mut ring = arc(
            Point::new(x, 20.0),
            6.0 + f64::from(i),
            0.0,
            2.0 * std::f64::consts::PI,
        );
        ring.pop();
        strokes.push(Stroke {
            anchors: ring,
            closed: true,
            width: 1.2,
            profile: vec![(0.0, 1.0), (0.5, 0.6)],
        });
    }
    for i in 0..8 {
        let x = 150.0 + f64::from(i) * 12.0;
        strokes.push(Stroke {
            anchors: arc(Point::new(x, 80.0), 8.0, 0.2, 2.8),
            closed: false,
            width: 1.0 + 0.1 * f64::from(i),
            profile: vec![(0.0, 0.2), (0.5, 1.0), (1.0, 0.2)],
        });
    }
    for i in 0..2 {
        let x = 20.0 + f64::from(i) * 50.0;
        strokes.push(Stroke::line(
            Point::new(x, 160.0),
            Point::new(x + 40.0, 160.0),
            2.0,
        ));
        strokes.push(Stroke::line(
            Point::new(x + 20.0, 160.0),
            Point::new(x + 20.0, 185.0),
            2.0,
        ));
    }
    strokes
}

#[test]
fn the_skeleton_of_a_twenty_shape_drawing_stays_in_budget() {
    let regions = union(&synthetic_whale());
    let vertices: usize = regions.iter().map(Region::vertex_count).sum();
    let mut best = std::time::Duration::MAX;
    for _ in 0..3 {
        let start = thread_time();
        for region in &regions {
            medial_axis(region).unwrap();
        }
        best = best.min(thread_time().saturating_sub(start));
    }
    eprintln!(
        "whale: {} regions, {vertices} outline vertices, skeleton {best:?} thread CPU",
        regions.len()
    );
    let budget = if cfg!(debug_assertions) { 3_000 } else { 300 };
    assert!(best <= std::time::Duration::from_millis(budget), "{best:?}");
}

#[test]
fn regions_refuse_nonfinite_coordinates() {
    let bad = [corner(f64::NAN, 0.0), corner(1.0, 0.0), corner(1.0, 1.0)];
    assert_eq!(
        regions(&[Outline::new(&bad, true)]),
        Err(super::CentreLineError::NonFinite)
    );
}

/// A 40 x 2 mm bar with round ends whose sides carry jaggies of up to 0.05 mm every 0.2 mm
/// from a fixed-seed generator.
pub(crate) fn noisy_bar() -> Region {
    let mut seed: u64 = 0x5eed;
    let mut jag = || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        #[allow(clippy::cast_precision_loss)]
        let unit = (seed >> 11) as f64 / (1_u64 << 53) as f64;
        (unit - 0.5) * 0.1
    };
    let mut ring: Vec<Point> = Vec::new();
    for i in 0..=200 {
        ring.push(Point::new(f64::from(i) * 0.2, -1.0 + jag()));
    }
    for i in 1..32 {
        let a = -std::f64::consts::FRAC_PI_2 + f64::from(i) * std::f64::consts::PI / 32.0;
        ring.push(Point::new(40.0 + a.cos(), a.sin()));
    }
    for i in (0..=200).rev() {
        ring.push(Point::new(f64::from(i) * 0.2, 1.0 + jag()));
    }
    for i in 1..32 {
        let a = std::f64::consts::FRAC_PI_2 + f64::from(i) * std::f64::consts::PI / 32.0;
        ring.push(Point::new(a.cos(), a.sin()));
    }
    let doc: Vec<DocPoint> = ring.iter().map(|p| DocPoint::new(p.x, p.y)).collect();
    let regions = super::region::regions_of_paths(&[snap_polygon(&doc)]);
    regions.into_iter().next().unwrap()
}

#[test]
fn a_noisy_bar_keeps_one_axis_near_its_centre() {
    let mut axis = medial_axis(&noisy_bar()).unwrap();
    let ends = axis.edges.len();
    // No spur into a jaggy: every sample keeps most of the bar's clearance.
    let thinnest = axis
        .edges
        .iter()
        .flat_map(|e| &e.samples)
        .map(|s| s.radius)
        .fold(f64::INFINITY, f64::min);
    assert!(
        thinnest > 0.8,
        "a spur reaches the outline: clearance {thinnest}"
    );
    for edge in &mut axis.edges {
        edge.samples.retain(|s| (1.5..=38.5).contains(&s.point.x));
    }
    let truth: Vec<Truth> = Stroke::line(Point::new(0.0, 0.0), Point::new(40.0, 0.0), 2.0).truth();
    let (axis_error, width_error) = errors(&axis, &truth);
    eprintln!(
        "noisy bar: {ends} edges, inner axis error {axis_error:.2}, width error {width_error:.2} of T(w)/2"
    );
    // Recorded, not gated by step 0: the jaggies reach the raw axis; the fit smooths them.
    assert_eq!(ends, 1);
}
