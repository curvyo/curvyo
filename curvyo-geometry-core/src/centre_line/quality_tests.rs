//! The junction and tip quality of a result (ADR 0063 amendment 2026-11-20, CENTRE-THROUGH A):
//! how far the painted paths stray outside the subject, how far a branch's butt end pokes out of
//! the path it joins, and how many paths a shape gives. Measured on the output paths as `0085`
//! paints them, on synthetic shapes and on the traced fixtures.

use kurbo::{Point, Vec2};

use super::measure_tests::{painted_rings, traced_lines, traced_subjects};
use super::{CentreLines, CentreOptions, CentreRole, CentreSkeleton};
use crate::{Outline, OutlineTriple, PathMeasure};

/// Flattening of subjects and painted outlines for the measures, in millimetres.
const FLATTEN_MM: f64 = 0.005;
/// The cell of the segment grid, in millimetres.
const CELL_MM: f64 = 1.0;

/// Closed polylines with a grid of their segments, for inside and distance queries.
pub(super) struct Rings {
    rings: Vec<Vec<Point>>,
    cells: std::collections::HashMap<(i64, i64), Vec<(usize, usize)>>,
}

#[allow(clippy::cast_possible_truncation)]
fn cell(p: Point) -> (i64, i64) {
    (
        (p.x / CELL_MM).floor() as i64,
        (p.y / CELL_MM).floor() as i64,
    )
}

impl Rings {
    pub(super) fn new(outlines: &[(Vec<OutlineTriple>, bool)]) -> Self {
        let rings: Vec<Vec<Point>> = outlines
            .iter()
            .map(|(anchors, closed)| {
                crate::flatten::flatten(anchors, *closed, FLATTEN_MM)
                    .iter()
                    .map(|p| Point::new(p.x, p.y))
                    .collect()
            })
            .filter(|r: &Vec<Point>| r.len() >= 2)
            .collect();
        let mut cells: std::collections::HashMap<(i64, i64), Vec<(usize, usize)>> =
            std::collections::HashMap::new();
        for (r, ring) in rings.iter().enumerate() {
            for i in 0..ring.len() {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                let (ca, cb) = (cell(a), cell(b));
                for x in ca.0.min(cb.0)..=ca.0.max(cb.0) {
                    for y in ca.1.min(cb.1)..=ca.1.max(cb.1) {
                        cells.entry((x, y)).or_default().push((r, i));
                    }
                }
            }
        }
        Self { rings, cells }
    }

    /// Nonzero winding.
    pub(super) fn inside(&self, p: Point) -> bool {
        let mut winding = 0;
        for ring in &self.rings {
            for i in 0..ring.len() {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                if a.y <= p.y {
                    if b.y > p.y && (b - a).cross(p - a) > 0.0 {
                        winding += 1;
                    }
                } else if b.y <= p.y && (b - a).cross(p - a) < 0.0 {
                    winding -= 1;
                }
            }
        }
        winding != 0
    }

    /// The distance to the nearest segment, searched up to `limit` mm (`limit` when none).
    pub(super) fn distance(&self, p: Point, limit: f64) -> f64 {
        let mut best = limit;
        #[allow(clippy::cast_possible_truncation)]
        let reach = (limit / CELL_MM).ceil() as i64;
        let c = cell(p);
        for x in c.0 - reach..=c.0 + reach {
            for y in c.1 - reach..=c.1 + reach {
                for &(r, i) in self.cells.get(&(x, y)).into_iter().flatten() {
                    let ring = &self.rings[r];
                    let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                    best = best.min(segment_distance(p, a, b));
                }
            }
        }
        best
    }

    fn vertices(&self) -> impl Iterator<Item = Point> + '_ {
        self.rings.iter().flatten().copied()
    }
}

fn segment_distance(point: Point, from: Point, to: Point) -> f64 {
    let along = to - from;
    let len2 = along.hypot2();
    let t = if len2 > 0.0 {
        ((point - from).dot(along) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (from + along * t - point).hypot()
}

/// The measures of one result.
#[derive(Debug, Default)]
pub(super) struct Quality {
    /// Paths.
    pub(super) paths: usize,
    /// The farthest a painted outline point lies outside the subject, in mm, and where.
    pub(super) outside: (f64, Point),
    /// Branch ends inside another path's paint (junction ends).
    pub(super) junction_ends: usize,
    /// Of those, ends whose butt is at least `T` wide.
    pub(super) butt_ends: usize,
    /// The farthest a butt corner of a junction end lies outside the other paths' paint, in mm,
    /// and where.
    pub(super) poke: (f64, Point),
    /// Every spot outside the subject by more than 0.1 mm or poking by more than 0.1 mm, one per
    /// 2 mm: (kind, distance, where).
    pub(super) spots: Vec<(&'static str, f64, Point)>,
}

impl Quality {
    fn spot(&mut self, kind: &'static str, d: f64, at: Point) {
        if d <= 0.1 {
            return;
        }
        if let Some(s) = self
            .spots
            .iter_mut()
            .find(|s| s.0 == kind && (s.2 - at).hypot() < 2.0)
        {
            if d > s.1 {
                *s = (kind, d, at);
            }
        } else {
            self.spots.push((kind, d, at));
        }
    }
}

/// Measures `lines` against the subject `outlines`.
pub(super) fn quality(outlines: &[(Vec<OutlineTriple>, bool)], lines: &CentreLines) -> Quality {
    let subject = Rings::new(outlines);
    let painted: Vec<Rings> = lines
        .paths
        .iter()
        .map(|path| {
            Rings::new(
                &painted_rings(path)
                    .into_iter()
                    .map(|r| (r, true))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    let mut q = Quality {
        paths: lines.paths.len(),
        ..Quality::default()
    };
    for rings in &painted {
        for v in rings.vertices() {
            if !subject.inside(v) {
                let d = subject.distance(v, 5.0);
                q.spot("outside", d, v);
                if d > q.outside.0 {
                    q.outside = (d, v);
                }
            }
        }
    }
    for (k, path) in lines.paths.iter().enumerate() {
        if path.role != CentreRole::Branch || path.closed {
            continue;
        }
        let measure = PathMeasure::new(&path.anchors, false);
        let total = measure.length().as_mm();
        for at in [0.0, 1.0] {
            let sample = measure.at(curvyo_document_core::Length::from_mm(at * total));
            let end = Point::new(sample.point.x, sample.point.y);
            let others: Vec<&Rings> = painted
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != k)
                .map(|(_, r)| r)
                .collect();
            let width = path.width.as_mm() * path.profile.sample(at).unwrap_or(0.0);
            let near = others.iter().any(|r| r.inside(end))
                || others
                    .iter()
                    .any(|r| r.distance(end, 1.0) <= 0.1_f64.max(0.5 * width));
            if !near {
                continue;
            }
            q.junction_ends += 1;
            if width >= super::fit::tolerance(width) {
                q.butt_ends += 1;
            }
            let t = Vec2::new(sample.tangent.x, sample.tangent.y);
            let n = Vec2::new(-t.y, t.x) / t.hypot().max(1e-12);
            for corner in [end + n * (width / 2.0), end - n * (width / 2.0)] {
                if others.iter().any(|r| r.inside(corner)) {
                    continue;
                }
                let d = others
                    .iter()
                    .map(|r| r.distance(corner, 5.0))
                    .fold(5.0, f64::min);
                q.spot("poke", d, corner);
                if d > q.poke.0 {
                    q.poke = (d, corner);
                }
            }
        }
    }
    q
}

/// The quality of a traced fixture text at `options`, all subjects together.
fn traced_quality(text: &str, options: &CentreOptions) -> Vec<Quality> {
    let subjects = traced_subjects(text);
    traced_lines(text, options)
        .iter()
        .zip(&subjects)
        .map(|(lines, subject)| quality(subject, lines))
        .collect()
}

#[test]
#[ignore = "a measurement, run by hand"]
fn measure_quality() {
    let text = std::env::var("CURVYO_CENTRE_INPUT").map_or_else(
        |_| include_str!("../../tests/fixtures/centre_lines/dolphin_traced.txt").to_owned(),
        |path| std::fs::read_to_string(path).unwrap(),
    );
    for (s, q) in traced_quality(&text, &CentreOptions::default())
        .iter()
        .enumerate()
    {
        eprintln!(
            "subject {s}: {} paths, outside {:.3} at {:?}, {} junction ends, {} butt, poke {:.3} at {:?}",
            q.paths, q.outside.0, q.outside.1, q.junction_ends, q.butt_ends, q.poke.0, q.poke.1
        );
        let mut spots = q.spots.clone();
        spots.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (kind, d, at) in spots {
            eprintln!("  {kind:8} {d:.3} at ({:.1}, {:.1})", at.x, at.y);
        }
    }
}

/// A shape's result, its own outline as the subject.
pub(super) fn polygon_quality(
    outline: &[OutlineTriple],
    options: &CentreOptions,
) -> (CentreLines, Quality) {
    let lines = CentreSkeleton::build(&[Outline::new(outline, true)])
        .unwrap()
        .finish(options, &[]);
    let q = quality(&[(outline.to_vec(), true)], &lines);
    (lines, q)
}

/// The traced fixtures (criterion 19) after CENTRE-THROUGH A: the dolphin's wave crests run
/// through their tips (69 paths before, 55 after), no butt end pokes out of the path it joins
/// by more than 0.2 mm (0.77 mm before, at the crests), and nothing new is painted outside;
/// the whale stays as it was.
#[test]
fn the_traced_fixtures_keep_their_junctions_inside() {
    let dolphin = traced_quality(
        include_str!("../../tests/fixtures/centre_lines/dolphin_traced.txt"),
        &CentreOptions::default(),
    );
    let q = &dolphin[0];
    eprintln!(
        "dolphin: {} paths, outside {:?}, poke {:?}",
        q.paths, q.outside, q.poke
    );
    assert!(q.paths <= 55, "dolphin: {} paths", q.paths);
    assert!(q.poke.0 <= 0.2, "dolphin: poke {:?}", q.poke);
    // The flipper's corner (0.51 mm at 121.0, 199.6) is older than this round.
    assert!(q.outside.0 <= 0.52, "dolphin: outside {:?}", q.outside);
    let pokes = q.spots.iter().filter(|s| s.0 == "poke").count();
    assert!(
        pokes <= 2,
        "dolphin: {pokes} ends poke out by more than 0.1 mm"
    );
    let whale = traced_quality(
        include_str!("../../tests/fixtures/centre_lines/whale_traced.txt"),
        &CentreOptions::default(),
    );
    for q in &whale {
        assert!(q.poke.0 <= 0.06, "whale: poke {:?}", q.poke);
        assert!(q.outside.0 <= 0.12, "whale: outside {:?}", q.outside);
    }
}
