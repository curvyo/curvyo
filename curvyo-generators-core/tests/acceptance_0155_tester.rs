//! Acceptance tests of `specs/0155-generator-joint-options/specification.md`, slice 1 (criteria 1
//! to 9, ADR 0087 decision 1: tighter tab windows), through the public generator contract (ADR
//! 0080). Black-box: stored parameters and generated outlines only. Written from the spec and the
//! `0147` / `0149` acceptance helpers, before the implementation was read.
//!
//! Not covered here: criterion 4's tab-and-slot centre agreement and the engaged-edge line (the
//! public outlines carry no tab geometry, as `0147` criterion 5 found); criterion 9 (the demo cut
//! on the customer's laser); criterion 7 checks only that the box builds without an error.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::similar_names,
    clippy::needless_range_loop,
    clippy::items_after_statements,
    clippy::suboptimal_flops,
    clippy::imprecise_flops
)]

mod budget_factor;

use curvyo_document_core::Point;
use curvyo_extension_core::{
    ClosedPath, Cubic, Extension, GenerateRequest, Generated, ParamValue, Part, Seed, StoredParams,
};
use curvyo_generators_core::{GENERATORS, PATH_BOX, ROUNDED_BOX};
use serde_json::{Map, Value, json};
use std::f64::consts::{PI, TAU};
use std::fmt::Write as _;

type Pt = (f64, f64);

const PATTERNS: [&str; 3] = ["straight", "offset", "wave"];

// ---------- stored parameters, as the host writes them ----------

fn mm(value: f64) -> Value {
    json!({ "mm": value })
}

fn choice(id: &str) -> Value {
    json!({ "choice": id })
}

fn run(kind: &'static str, params: &StoredParams) -> Generated {
    GENERATORS
        .generate(&GenerateRequest {
            kind,
            params,
            seed: Seed(1),
            edit: None,
        })
        .expect("the generator answers its own kind")
}

/// A rounded box, inside `w` x `d` x `h`, corner radius `r`, 3 mm wall, plus `extra`.
fn rounded_at(w: f64, d: f64, h: f64, r: f64, extra: &[(&str, Value)]) -> Generated {
    let mut map = Map::new();
    map.insert("inner_width".to_owned(), mm(w));
    map.insert("inner_depth".to_owned(), mm(d));
    map.insert("inner_height".to_owned(), mm(h));
    map.insert("corner_radius".to_owned(), mm(r));
    map.insert("thickness".to_owned(), mm(3.0));
    for (key, value) in extra {
        map.insert((*key).to_owned(), value.clone());
    }
    run(ROUNDED_BOX, &map)
}

/// A path box with only its outline stored (the defaults of every other key).
fn outline_map(path: &ClosedPath) -> StoredParams {
    let mut map = Map::new();
    map.insert(
        "outline".to_owned(),
        ParamValue::Path(path.clone()).to_stored(),
    );
    map
}

/// A path box with a 30 mm inside height, a 3 mm wall, plus `extra`.
fn path_built(path: &ClosedPath, extra: &[(&str, Value)]) -> Generated {
    let mut map = outline_map(path);
    map.insert("inner_height".to_owned(), mm(30.0));
    map.insert("thickness".to_owned(), mm(3.0));
    for (key, value) in extra {
        map.insert((*key).to_owned(), value.clone());
    }
    run(PATH_BOX, &map)
}

// ---------- the shapes ----------

fn cubic(c1: Pt, c2: Pt, to: Pt) -> Cubic {
    Cubic {
        c1: Point::new(c1.0, c1.1),
        c2: Point::new(c2.0, c2.1),
        to: Point::new(to.0, to.1),
    }
}

/// An ellipse of semi-axes `rx`, `ry` as `segments` cubics, CCW from (rx, 0) (as `0149`).
fn ellipse_cubics(rx: f64, ry: f64, segments: usize) -> ClosedPath {
    let theta = TAU / segments as f64;
    let k = 4.0 / 3.0 * (theta / 4.0).tan();
    let point = |i: usize| {
        let a = (i % segments) as f64 * theta;
        (rx * a.cos(), ry * a.sin())
    };
    let tangent = |i: usize| {
        let a = (i % segments) as f64 * theta;
        (-rx * a.sin(), ry * a.cos())
    };
    let curves = (0..segments)
        .map(|i| {
            let (p0, p1) = (point(i), point(i + 1));
            let (t0, t1) = (tangent(i), tangent(i + 1));
            cubic(
                (p0.0 + k * t0.0, p0.1 + k * t0.1),
                (p1.0 - k * t1.0, p1.1 - k * t1.1),
                p1,
            )
        })
        .collect();
    ClosedPath {
        start: Point::new(rx, 0.0),
        segments: curves,
    }
}

/// The heart of `0149` (two lobes, a notch at the top and a tip at the bottom).
fn heart() -> ClosedPath {
    ClosedPath {
        start: Point::new(0.0, 20.0),
        segments: vec![
            cubic((10.0, 32.0), (30.0, 28.0), (30.0, 10.0)),
            cubic((30.0, -8.0), (6.0, -29.6), (0.0, -40.0)),
            cubic((-6.0, -29.6), (-30.0, -8.0), (-30.0, 10.0)),
            cubic((-30.0, 28.0), (-10.0, 32.0), (0.0, 20.0)),
        ],
    }
}

/// A closed outline from cubics given as (c1, c2, to) triples (as `path_box_curves`).
fn curve(start: Pt, cubics: &[(Pt, Pt, Pt)]) -> ClosedPath {
    ClosedPath {
        start: Point::new(start.0, start.1),
        segments: cubics
            .iter()
            .map(|&(c1, c2, to)| cubic(c1, c2, to))
            .collect(),
    }
}

/// The heart of `path_box_curves` (its golden is `path_box_heart.txt`).
fn heart_golden() -> ClosedPath {
    curve(
        (0.0, 40.0),
        &[
            ((-10.0, 25.0), (-40.0, 10.0), (-40.0, -10.0)),
            ((-40.0, -35.0), (-12.0, -32.0), (0.0, -22.0)),
            ((12.0, -32.0), (40.0, -35.0), (40.0, -10.0)),
            ((40.0, 10.0), (10.0, 25.0), (0.0, 40.0)),
        ],
    )
}

/// The shield of `path_box_curves` (golden `path_box_shield.txt`).
fn shield_golden() -> ClosedPath {
    curve(
        (-30.0, -40.0),
        &[
            ((-30.0, -40.0), (30.0, -40.0), (30.0, -40.0)),
            ((30.0, -40.0), (30.0, 10.0), (30.0, 10.0)),
            ((30.0, 25.0), (10.0, 40.0), (0.0, 40.0)),
            ((-10.0, 40.0), (-30.0, 25.0), (-30.0, 10.0)),
            ((-30.0, 10.0), (-30.0, -40.0), (-30.0, -40.0)),
        ],
    )
}

/// The ellipse of `path_box_curves` with half axes `a`, `b` (goldens `path_box_ellipse.txt`,
/// `path_box_circle.txt`).
fn ellipse_golden(a: f64, b: f64) -> ClosedPath {
    let k = 0.552_284_75;
    curve(
        (a, 0.0),
        &[
            ((a, k * b), (k * a, b), (0.0, b)),
            ((-k * a, b), (-a, k * b), (-a, 0.0)),
            ((-a, -k * b), (-k * a, -b), (0.0, -b)),
            ((k * a, -b), (a, -k * b), (a, 0.0)),
        ],
    )
}

/// A regular n-gon of radius `radius`, first vertex half a step round (as `path_box.rs`).
fn ngon(n: u32, radius: f64) -> ClosedPath {
    let pts: Vec<Pt> = (0..n)
        .map(|i| {
            let a = (f64::from(i) + 0.5) * TAU / f64::from(n) - std::f64::consts::FRAC_PI_2;
            (radius * a.cos(), radius * a.sin())
        })
        .collect();
    ClosedPath {
        start: Point::new(pts[0].0, pts[0].1),
        segments: (0..pts.len())
            .map(|i| {
                let to = pts[(i + 1) % pts.len()];
                cubic(pts[i], to, to)
            })
            .collect(),
    }
}

/// The cases of the spec's criterion 1, with their build.
#[derive(Clone, Copy, Debug)]
enum Case {
    Circle,
    Stadium,
    Heart,
    Ellipse,
}

impl Case {
    fn name(self) -> &'static str {
        match self {
            Self::Circle => "circle 60 x 60 r 30",
            Self::Stadium => "short stadium 60 x 30 r 15",
            Self::Heart => "heart",
            Self::Ellipse => "ellipse 40 x 25",
        }
    }

    /// Width, depth and corner radius of a rounded box; `None` for a path box.
    fn rounded(self) -> Option<(f64, f64, f64)> {
        match self {
            Self::Circle => Some((60.0, 60.0, 30.0)),
            Self::Stadium => Some((60.0, 30.0, 15.0)),
            Self::Heart | Self::Ellipse => None,
        }
    }

    fn build(self, pattern: &str, kerf: f64) -> Generated {
        let extra = [("slit_pattern", choice(pattern)), ("kerf", mm(kerf))];
        match self {
            Self::Circle => rounded_at(60.0, 60.0, 30.0, 30.0, &extra),
            Self::Stadium => rounded_at(60.0, 30.0, 30.0, 15.0, &extra),
            Self::Heart => path_built(&heart(), &extra),
            Self::Ellipse => path_built(&ellipse_cubics(40.0, 25.0, 4), &extra),
        }
    }
}

// ---------- outlines, parts and the strip ----------

fn sample(path: &ClosedPath) -> Vec<Pt> {
    const STEPS: usize = 16;
    let mut points = Vec::new();
    let mut from = path.start;
    for c in &path.segments {
        for step in 0..STEPS {
            let t = step as f64 / STEPS as f64;
            let u = 1.0 - t;
            points.push((
                u * u * u * from.x
                    + 3.0 * u * u * t * c.c1.x
                    + 3.0 * u * t * t * c.c2.x
                    + t * t * t * c.to.x,
                u * u * u * from.y
                    + 3.0 * u * u * t * c.c1.y
                    + 3.0 * u * t * t * c.c2.y
                    + t * t * t * c.to.y,
            ));
        }
        from = c.to;
    }
    points
}

fn bbox(points: &[Pt]) -> (f64, f64, f64, f64) {
    let mut b = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for &(x, y) in points {
        b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
    }
    b
}

fn sampled_points(paths: &[ClosedPath]) -> Vec<Pt> {
    paths.iter().flat_map(sample).collect()
}

/// Width and height of a part's outlines.
fn extents(paths: &[ClosedPath]) -> (f64, f64) {
    let (x0, y0, x1, y1) = bbox(&sampled_points(paths));
    (x1 - x0, y1 - y0)
}

/// The strip in its own frame: `u` along the long axis from its minimum, `v` across the wall
/// (`v = 0` the base edge, `v = height` the ring edge), as `0147` and `0149` map it.
struct Strip {
    paths: Vec<Vec<Pt>>,
    length: f64,
    height: f64,
}

fn strip_from(paths: &[ClosedPath], height: f64) -> Strip {
    let sampled: Vec<Vec<Pt>> = paths.iter().map(sample).collect();
    let flat: Vec<Pt> = sampled.iter().flatten().copied().collect();
    let (x0, y0, x1, y1) = bbox(&flat);
    let along_x = (x1 - x0 - height).abs() > (y1 - y0 - height).abs();
    let mapped: Vec<Vec<Pt>> = sampled
        .iter()
        .map(|path| {
            path.iter()
                .map(|&(x, y)| {
                    if along_x {
                        (x - x0, y - y0)
                    } else {
                        (y - y0, x - x0)
                    }
                })
                .collect()
        })
        .collect();
    Strip {
        paths: mapped,
        length: if along_x { x1 - x0 } else { y1 - y0 },
        height: if along_x { y1 - y0 } else { x1 - x0 },
    }
}

fn length_of(generated: &Generated, key: &str) -> f64 {
    generated
        .params
        .get(key)
        .and_then(ParamValue::from_stored)
        .and_then(|value| value.length())
        .unwrap_or_else(|| panic!("no length stored under {key}"))
        .as_mm()
}

/// The value a plate thickness is built with: the automatic one, or the typed one.
fn used(generated: &Generated, key: &str) -> f64 {
    generated
        .automatic
        .iter()
        .find(|(k, _)| *k == key)
        .and_then(|(_, value)| value.clone().length())
        .map_or_else(
            || length_of(generated, key),
            curvyo_document_core::Length::as_mm,
        )
}

/// The wall height: inside height plus the cover and the lid plate.
fn wall_height(generated: &Generated) -> f64 {
    length_of(generated, "inner_height")
        + used(generated, "cover_thickness")
        + used(generated, "lid_thickness")
}

/// (wall thickness `t`, plate thickness `t_w`) as the spec's tab window uses them.
fn wall_context(generated: &Generated) -> (f64, f64) {
    (
        length_of(generated, "thickness"),
        used(generated, "plate_thickness"),
    )
}

/// Whether a part's outlines have the wall height in one of their two extents.
fn wall_like(paths: &[ClosedPath], height: f64) -> bool {
    let (w, d) = extents(paths);
    (w - height).abs() <= 0.5 || (d - height).abs() <= 0.5
}

fn wall_parts(generated: &Generated) -> Vec<&Part> {
    let height = wall_height(generated);
    generated
        .parts
        .iter()
        .filter(|part| wall_like(&part.paths, height))
        .collect()
}

/// The strips of a build: for a rounded box the largest wall-height part, for a path box every
/// wall-height part (one strip, or one per side of a joint).
fn strips_of(generated: &Generated, rounded: bool) -> Vec<Strip> {
    let height = wall_height(generated);
    let mut parts = wall_parts(generated);
    if rounded {
        parts.sort_by(|a, b| {
            let (aw, ad) = extents(&a.paths);
            let (bw, bd) = extents(&b.paths);
            (bw * bd).total_cmp(&(aw * ad))
        });
        parts.truncate(1);
    }
    parts
        .iter()
        .map(|part| strip_from(&part.paths, height))
        .collect()
}

// ---------- the geometry of the strip (copied from 0147 and 0149) ----------

/// The strip's length on the neutral line: `2(w - 2r) + 2(d - 2r) + pi (2r + t)`.
fn neutral_length(w: f64, d: f64, r: f64, t: f64) -> f64 {
    2.0 * (w - 2.0 * r) + 2.0 * (d - 2.0 * r) + PI * (2.0 * r + t)
}

/// The expected strip length with `windows` flat tab windows on bent spans (`0147` criterion 3):
/// `neutral - N (r theta - l)`, `l = 2 t_p + 2 t_w`, `theta = 2 asin(l / (2 r))`.
fn expected_strip_length(w: f64, d: f64, r: f64, t_w: f64, t_p: f64, windows: usize) -> f64 {
    let neutral = neutral_length(w, d, r, t_w);
    if windows == 0 {
        return neutral;
    }
    let l = 2.0 * t_p + 2.0 * t_w;
    let theta = 2.0 * (l / (2.0 * r)).asin();
    let per_window = r * theta - l;
    neutral - f64::from(u32::try_from(windows).unwrap()) * per_window
}

/// The four bent spans of a rounded box along `u` (`0147` criterion 3).
fn corner_windows(w: f64, d: f64, r: f64, t: f64) -> Vec<(f64, f64)> {
    let long = w.max(d) - 2.0 * r;
    let short = w.min(d) - 2.0 * r;
    let arc = PI / 2.0 * (r + t / 2.0);
    let mut cursor = long / 2.0;
    let mut windows = Vec::new();
    windows.push((cursor, cursor + arc));
    cursor += arc + short;
    windows.push((cursor, cursor + arc));
    cursor += arc + long;
    windows.push((cursor, cursor + arc));
    cursor += arc + short;
    windows.push((cursor, cursor + arc));
    cursor += arc + long / 2.0;
    assert!(
        (cursor - neutral_length(w, d, r, t)).abs() <= 1e-9,
        "the windows cover the strip"
    );
    windows
}

fn in_any(u: f64, windows: &[(f64, f64)], tolerance: f64) -> bool {
    windows
        .iter()
        .any(|&(a, b)| u >= a - tolerance && u <= b + tolerance)
}

/// The centres `u` of the tab notches at one edge of the strip: flat runs at `v = level` of the
/// length of a tab (`2 t_p`), counted once per run.
fn notch_centres(strip: &Strip, level: f64, t_p: f64) -> Vec<f64> {
    let mut centres = Vec::new();
    for path in &strip.paths {
        let mut run: Vec<f64> = Vec::new();
        let close = |run: &mut Vec<f64>, centres: &mut Vec<f64>| {
            if run.len() >= 2 {
                let lo = run.iter().copied().fold(f64::INFINITY, f64::min);
                let hi = run.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                if hi - lo > 1.5 * t_p && hi - lo <= 2.0 * t_p + 0.01 {
                    centres.push(f64::midpoint(lo, hi));
                }
            }
            run.clear();
        };
        for &(u, v) in path {
            if (v - level).abs() < 1e-3 {
                run.push(u);
            } else {
                close(&mut run, &mut centres);
            }
        }
        close(&mut run, &mut centres);
    }
    centres
}

/// The tab windows of a strip: the notches of the base row, on the bent spans of a rounded box
/// (the flats of a straight are not bent spans), or all of them for a path box.
fn bend_tabs(rounded: Option<(f64, f64, f64)>, strip: &Strip, t_w: f64, t_p: f64) -> Vec<f64> {
    let centres = notch_centres(strip, t_w, t_p);
    match rounded {
        Some((w, d, r)) => {
            let arcs = corner_windows(w, d, r, t_w);
            centres
                .into_iter()
                .filter(|&u| in_any(u, &arcs, 0.5))
                .collect()
        }
        None => centres,
    }
}

/// The bent runs to be checked for uncut bands: the bent spans of a rounded box, or the bent
/// runs of a path box between its tab windows, each with a margin of `2 t` at both ends.
fn bent_run_list(
    rounded: Option<(f64, f64, f64)>,
    strip: &Strip,
    t_w: f64,
    t_p: f64,
) -> Vec<(f64, f64)> {
    match rounded {
        Some((w, d, r)) => corner_windows(w, d, r, t_w),
        None => bent_runs(
            strip.length,
            &notch_centres(strip, t_w, t_p),
            t_p + t_w,
            2.0 * t_w,
        ),
    }
}

fn bent_runs(length: f64, centres: &[f64], half: f64, margin: f64) -> Vec<(f64, f64)> {
    let mut cuts: Vec<(f64, f64)> = centres.iter().map(|&c| (c - half, c + half)).collect();
    cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut runs = Vec::new();
    let mut cursor = margin;
    for (a, b) in cuts {
        if a > cursor {
            runs.push((cursor, a.min(length - margin)));
        }
        cursor = cursor.max(b);
    }
    if cursor < length - margin {
        runs.push((cursor, length - margin));
    }
    runs.retain(|&(a, b)| b > a);
    runs
}

/// The u positions where the horizontal line `v` crosses an edge of the strip, sorted.
fn crossings(strip: &Strip, v: f64) -> Vec<f64> {
    let mut us = Vec::new();
    for path in &strip.paths {
        let n = path.len();
        for i in 0..n {
            let (u0, v0) = path[i];
            let (u1, v1) = path[(i + 1) % n];
            if (v0 - v) * (v1 - v) < 0.0 {
                let s = (v - v0) / (v1 - v0);
                us.push(u0 + s * (u1 - u0));
            }
        }
    }
    us.sort_by(f64::total_cmp);
    us
}

/// The unslit stretch across the window at `c` on one row: between the nearest crossings either
/// side of `c`, if there are crossings on both sides.
fn gap_at(us: &[f64], c: f64) -> Option<f64> {
    let left = us.iter().rev().find(|&&u| u < c)?;
    let right = us.iter().find(|&&u| u > c)?;
    Some(right - left)
}

/// The flat across the window at `c` is a property of the strip: the nearest slit edge on each
/// side of `c` over ALL rows `lo..=hi` (for Offset and Wave the nearest column may sit in another
/// row). Returns the drawn distance between those edges, `None` if there is no edge on one side.
fn strip_flat_drawn(strip: &Strip, c: f64, lo: f64, hi: f64) -> Option<f64> {
    let mut us = Vec::new();
    let mut v = lo;
    while v <= hi + 1e-9 {
        us.extend(crossings(strip, v));
        v += 0.1;
    }
    us.sort_by(f64::total_cmp);
    gap_at(&us, c)
}

/// The drawn width of a slit: the smallest gap between two edges of one slit on a mid row.
fn slit_width_at(strip: &Strip, row: f64) -> f64 {
    crossings(strip, row)
        .windows(2)
        .map(|p| p[1] - p[0])
        .filter(|&d| d > 0.01 && d < 1.0)
        .fold(f64::INFINITY, f64::min)
}

/// The number of uncut height lines across the runs, and the first (height, run start).
fn uncut_bands(strip: &Strip, runs: &[(f64, f64)]) -> (usize, Option<(f64, f64)>) {
    let mut failures = 0;
    let mut first = None;
    let mut v = 0.05;
    while v < strip.height - 0.05 {
        let us = crossings(strip, v);
        for &(a, b) in runs {
            if !us.iter().any(|&u| u >= a && u <= b) {
                failures += 1;
                first.get_or_insert((v, a));
            }
        }
        v += 0.1;
    }
    (failures, first)
}

fn seg_point_distance(p: Pt, a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    };
    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
}

fn orient(p: Pt, q: Pt, r: Pt) -> f64 {
    (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0)
}

fn segments_distance(a0: Pt, a1: Pt, b0: Pt, b1: Pt) -> f64 {
    let crosses = orient(b0, b1, a0) * orient(b0, b1, a1) < 0.0
        && orient(a0, a1, b0) * orient(a0, a1, b1) < 0.0;
    if crosses {
        return 0.0;
    }
    seg_point_distance(a0, b0, b1)
        .min(seg_point_distance(a1, b0, b1))
        .min(seg_point_distance(b0, a0, a1))
        .min(seg_point_distance(b1, a0, a1))
}

/// The smallest distance between two different contours of the strip (sampled polylines), only
/// segment pairs within 1 mm of each other are compared (a kerf is 0.2 mm or less).
fn closest_contours(strip: &Strip) -> f64 {
    let segments: Vec<Vec<(Pt, Pt)>> = strip
        .paths
        .iter()
        .map(|p| (0..p.len()).map(|i| (p[i], p[(i + 1) % p.len()])).collect())
        .collect();
    let boxes: Vec<(f64, f64, f64, f64)> = strip.paths.iter().map(|p| bbox(p)).collect();
    let mut best = f64::INFINITY;
    for i in 0..strip.paths.len() {
        for j in (i + 1)..strip.paths.len() {
            let (a, b) = (boxes[i], boxes[j]);
            let dx = (a.0 - b.2).max(b.0 - a.2).max(0.0);
            let dy = (a.1 - b.3).max(b.1 - a.3).max(0.0);
            if dx.hypot(dy) > 1.0 {
                continue;
            }
            for &(a0, a1) in &segments[i] {
                let sa = (
                    a0.0.min(a1.0),
                    a0.1.min(a1.1),
                    a0.0.max(a1.0),
                    a0.1.max(a1.1),
                );
                for &(b0, b1) in &segments[j] {
                    let sb = (
                        b0.0.min(b1.0),
                        b0.1.min(b1.1),
                        b0.0.max(b1.0),
                        b0.1.max(b1.1),
                    );
                    let dx = (sa.0 - sb.2).max(sb.0 - sa.2).max(0.0);
                    let dy = (sa.1 - sb.3).max(sb.1 - sa.3).max(0.0);
                    if dx.hypot(dy) > 1.0 {
                        continue;
                    }
                    best = best.min(segments_distance(a0, a1, b0, b1));
                }
            }
        }
    }
    best
}

// ---------- the goldens (as the existing tests dump them) ----------

fn dump_part(index: usize, paths: &[ClosedPath]) -> String {
    let mut text = String::new();
    for path in paths {
        writeln!(
            text,
            "part {index} start {:.6} {:.6}",
            path.start.x, path.start.y
        )
        .unwrap();
        for s in &path.segments {
            writeln!(
                text,
                "{:.6} {:.6} {:.6} {:.6} {:.6} {:.6}",
                s.c1.x, s.c1.y, s.c2.x, s.c2.y, s.to.x, s.to.y
            )
            .unwrap();
        }
    }
    text
}

fn dump(generated: &Generated) -> String {
    generated
        .parts
        .iter()
        .enumerate()
        .map(|(index, part)| dump_part(index, &part.paths))
        .collect()
}

fn golden_text(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path).unwrap()
}

/// The parts of a golden file, each as its outlines (the holes follow the outer outline).
fn golden_parts(name: &str) -> Vec<Vec<ClosedPath>> {
    let mut parts: Vec<Vec<ClosedPath>> = Vec::new();
    for line in golden_text(name).lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.first() == Some(&"part") {
            let index: usize = fields[1].parse().unwrap();
            if index == parts.len() {
                parts.push(Vec::new());
            }
            assert_eq!(index + 1, parts.len(), "{name}: parts in order");
            let x: f64 = fields[3].parse().unwrap();
            let y: f64 = fields[4].parse().unwrap();
            parts[index].push(ClosedPath {
                start: Point::new(x, y),
                segments: Vec::new(),
            });
        } else {
            let v: Vec<f64> = fields.iter().map(|s| s.parse().unwrap()).collect();
            let path = parts.last_mut().unwrap().last_mut().unwrap();
            path.segments
                .push(cubic((v[0], v[1]), (v[2], v[3]), (v[4], v[5])));
        }
    }
    parts
}

// ---------- criterion 1: every tab window on a bent span has a flat of at most l + 0.1 mm ----------

#[test]
fn criterion_1_the_flat_across_every_tab_window_on_a_bent_span_is_at_most_l_plus_a_tenth() {
    let mut misses = Vec::new();
    for case in [Case::Circle, Case::Stadium, Case::Heart, Case::Ellipse] {
        for pattern in PATTERNS {
            for kerf in [0.0, 0.2] {
                let generated = case.build(pattern, kerf);
                let (t, t_w) = wall_context(&generated);
                let l = 2.0 * t + 2.0 * t_w;
                let height = wall_height(&generated);
                let mut windows = 0;
                for strip in strips_of(&generated, case.rounded().is_some()) {
                    for c in bend_tabs(case.rounded(), &strip, t, t_w) {
                        windows += 1;
                        // The cut flat is the drawn distance less the kerf (as the lead ruled).
                        match strip_flat_drawn(&strip, c, t + 0.5, height - t - 0.5) {
                            None => misses.push(format!(
                                "{} {pattern} kerf {kerf}: the window at u {c:.2} has no slit on one side",
                                case.name()
                            )),
                            Some(drawn) if drawn - kerf > l + 0.1 => misses.push(format!(
                                "{} {pattern} kerf {kerf}: window at u {c:.2}: drawn {drawn:.3} mm, slit width {:.3} mm, kerf {kerf}, cut flat {:.3} mm, l = {l}",
                                case.name(),
                                slit_width_at(&strip, height / 2.0),
                                drawn - kerf
                            )),
                            Some(_) => {}
                        }
                    }
                }
                assert!(
                    windows >= 2,
                    "{} {pattern} kerf {kerf}: the tab windows on the bends were found ({windows})",
                    case.name()
                );
            }
        }
    }
    assert!(misses.is_empty(), "flats above l + 0.1 mm: {misses:#?}");
}

// ---------- criterion 2: the bands start at the window ends and hold their columns ----------

#[test]
fn criterion_2_the_circle_band_holds_18_to_24_columns_in_even_groups_of_at_least_two() {
    let generated = Case::Circle.build("straight", 0.0);
    let (t, t_w) = wall_context(&generated);
    let l = 2.0 * t + 2.0 * t_w;
    let strips = strips_of(&generated, true);
    assert_eq!(strips.len(), 1, "the circle is one strip");
    let strip = &strips[0];
    let r: f64 = 30.0;
    let row = strip.height / 2.0;
    let us: Vec<f64> = crossings(strip, row)
        .into_iter()
        .filter(|&u| u > 0.05 && u < strip.length - 0.05)
        .collect();
    assert_eq!(us.len() % 2, 0, "the slit edges pair up into columns");
    let centres: Vec<f64> = us.chunks(2).map(|p| f64::midpoint(p[0], p[1])).collect();
    assert!(
        (18..=24).contains(&centres.len()),
        "the circle has {} columns, the spec says 18 to 24",
        centres.len()
    );
    // A band is the run of columns between two tab windows: a gap of about l + w between the
    // centres of two bands, and a pitch of at most the radius pitch 0.22 r inside a band.
    let mut bands: Vec<Vec<f64>> = vec![vec![centres[0]]];
    for pair in centres.windows(2) {
        if pair[1] - pair[0] > l - 1.0 {
            bands.push(Vec::new());
        }
        bands.last_mut().unwrap().push(pair[1]);
    }
    // Spec revision 2 (lead ruling): the pitch inside a band is at least g = max(0.22 r, slit width
    // + 1 mm) and at most 2 g (N = max(2, floor(L / g)) columns per band).
    let width = us[1] - us[0];
    let g = (0.22 * r).max(width + 1.0);
    for band in &bands {
        assert!(
            band.len() >= 2,
            "a curved stretch keeps two columns: {band:?}"
        );
        for pair in band.windows(2) {
            let pitch = pair[1] - pair[0];
            assert!(
                (g - 0.05..=2.0 * g + 0.05).contains(&pitch),
                "the pitch {pitch:.3} inside a band is outside [{g:.3}, {:.3}] (slit width {width:.3})",
                2.0 * g
            );
        }
    }
}

// ---------- criterion 3: no uncut band across a bent run, no contours closer than the kerf ----------

#[test]
fn criterion_3_no_band_across_a_bent_run_is_uncut_and_no_two_contours_are_closer_than_the_kerf() {
    let mut failures = Vec::new();
    for case in [Case::Circle, Case::Stadium, Case::Heart, Case::Ellipse] {
        for pattern in PATTERNS {
            for kerf in [0.0, 0.2] {
                let generated = case.build(pattern, kerf);
                let (t, t_w) = wall_context(&generated);
                for strip in strips_of(&generated, case.rounded().is_some()) {
                    let runs = bent_run_list(case.rounded(), &strip, t, t_w);
                    let (uncut, first) = uncut_bands(&strip, &runs);
                    if uncut > 0 {
                        failures.push(format!(
                            "{} {pattern} kerf {kerf}: {uncut} uncut bands, first at {first:?}",
                            case.name()
                        ));
                    }
                    let gap = closest_contours(&strip);
                    if gap < kerf - 0.02 {
                        failures.push(format!(
                            "{} {pattern} kerf {kerf}: two contours {gap:.3} mm apart",
                            case.name()
                        ));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "criterion 3 failures: {failures:#?}");
}

// ---------- criterion 4: the strip length and the tab windows ----------

#[test]
fn criterion_4_the_strip_length_is_the_neutral_line_less_the_tab_windows_within_a_tenth() {
    let mut misses = Vec::new();
    for (w, d, r) in [(120.0, 80.0, 15.0), (60.0, 60.0, 30.0), (60.0, 30.0, 15.0)] {
        let generated = rounded_at(w, d, 30.0, r, &[]);
        let (t, t_w) = wall_context(&generated);
        let strip = &strips_of(&generated, true)[0];
        let at_ring = bend_tabs(Some((w, d, r)), strip, t, t_w).len();
        let windows = bend_tabs(Some((w, d, r)), strip, t, t_w).len();
        let ring = notch_centres(strip, strip.height - t, t_w)
            .iter()
            .filter(|&&u| in_any(u, &corner_windows(w, d, r, t), 0.5))
            .count();
        assert_eq!(
            at_ring, ring,
            "the base and ring rows carry the same windows"
        );
        let expected = expected_strip_length(w, d, r, t, t_w, windows);
        if (strip.length - expected).abs() > 0.1 {
            misses.push(format!(
                "{w} x {d} r {r}: strip {:.3} vs expected {expected:.3}",
                strip.length
            ));
        }
    }
    // A path box circle: the same neutral line, every notch of the base row a window.
    let generated = path_built(&ellipse_cubics(30.0, 30.0, 4), &[]);
    let (t, t_w) = wall_context(&generated);
    let strip = &strips_of(&generated, false)[0];
    let windows = notch_centres(strip, t_w, t_w).len();
    let expected = expected_strip_length(60.0, 60.0, 30.0, t, t_w, windows);
    if (strip.length - expected).abs() > 0.1 {
        misses.push(format!(
            "path circle: strip {:.3} vs expected {expected:.3}",
            strip.length
        ));
    }
    assert!(misses.is_empty(), "strip length misses: {misses:#?}");
}

// ---------- criterion 5: the default outputs stay byte-identical ----------

#[test]
fn criterion_5_default_rounded_box_curved_top_coffin_and_20_gon_are_byte_identical_to_goldens() {
    let rounded = run(ROUNDED_BOX, &StoredParams::new());
    let mut curved = Map::new();
    curved.insert("top_shape".to_owned(), choice("curved"));
    let curved = run(ROUNDED_BOX, &curved);
    let coffin = run(PATH_BOX, &StoredParams::new());
    let gon = run(PATH_BOX, &outline_map(&ngon(20, 90.0)));
    for (name, generated) in [
        ("rounded_box_default.txt", &rounded),
        ("rounded_box_curved.txt", &curved),
        ("path_box_default.txt", &coffin),
        ("path_box_20gon.txt", &gon),
    ] {
        assert_eq!(
            dump(generated),
            golden_text(name),
            "{name}: the default output is unchanged"
        );
    }
}

// ---------- criterion 6: the re-blessed goldens move only slit columns ----------

/// The plates of a golden are unchanged, and on each wall the tab and slot positions (the notch
/// centres at the base row and the ring row) are unchanged; only the slit columns may move.
fn assert_only_slit_columns_moved(name: &str, generated: &Generated) {
    let old = golden_parts(name);
    assert_eq!(
        old.len(),
        generated.parts.len(),
        "{name}: the same number of parts"
    );
    let height = wall_height(generated);
    let (t, t_w) = wall_context(generated);
    for (index, part) in generated.parts.iter().enumerate() {
        if wall_like(&part.paths, height) {
            let before = strip_from(&old[index], height);
            let after = strip_from(&part.paths, height);
            for level in [t_w, height - t_w] {
                let a = notch_centres(&before, level, t);
                let b = notch_centres(&after, level, t);
                assert_eq!(a.len(), b.len(), "{name}: wall {index} keeps its tabs");
                for (x, y) in a.iter().zip(&b) {
                    assert!(
                        (x - y).abs() <= 0.01,
                        "{name}: wall {index} moves a tab or slot from {x:.3} to {y:.3}"
                    );
                }
            }
        } else {
            assert_eq!(
                dump_part(index, &part.paths),
                dump_part(index, &old[index]),
                "{name}: plate {index} is unchanged"
            );
        }
    }
}

#[test]
fn criterion_6_the_re_blessed_goldens_change_only_the_slit_columns_on_the_walls() {
    let mut circle = Map::new();
    circle.insert("inner_width".to_owned(), mm(60.0));
    circle.insert("inner_depth".to_owned(), mm(60.0));
    circle.insert("corner_radius".to_owned(), mm(30.0));
    assert_only_slit_columns_moved("rounded_box_circle.txt", &run(ROUNDED_BOX, &circle));
    for (name, path) in [
        ("path_box_heart.txt", heart_golden()),
        ("path_box_shield.txt", shield_golden()),
        ("path_box_ellipse.txt", ellipse_golden(45.0, 30.0)),
        ("path_box_circle.txt", ellipse_golden(30.0, 30.0)),
    ] {
        assert_only_slit_columns_moved(name, &run(PATH_BOX, &outline_map(&path)));
    }
}

// ---------- criterion 7: a radius too small for two columns builds with a notice, no error ----------

#[test]
fn criterion_7_a_corner_radius_too_small_for_two_columns_still_builds_without_an_error() {
    for (w, r) in [(12.0, 6.0), (20.0, 10.0), (12.0, 2.5)] {
        for pattern in PATTERNS {
            let generated = rounded_at(
                w,
                w,
                30.0,
                r,
                &[("slit_pattern", choice(pattern)), ("kerf", mm(0.2))],
            );
            assert!(
                !generated.parts.is_empty(),
                "a {w} mm box with radius {r} ({pattern}) builds"
            );
        }
    }
}

// ---------- criterion 8: regeneration costs under 100 ms of thread CPU ----------

#[cfg(target_os = "linux")]
fn thread_ticks() -> Option<u64> {
    let stat = std::fs::read_to_string("/proc/thread-self/stat").ok()?;
    let after = stat.rsplit_once(')')?.1;
    let fields: Vec<&str> = after.split_whitespace().collect();
    // Field 3 is fields[0]: utime is field 14 and stime field 15 (clock ticks, 100 per second).
    let user: u64 = fields.get(11)?.parse().ok()?;
    let system: u64 = fields.get(12)?.parse().ok()?;
    Some(user + system)
}

#[cfg(target_os = "linux")]
#[test]
fn criterion_8_regenerating_a_box_of_about_400_slots_costs_under_100_ms_of_thread_cpu() {
    let build = || {
        rounded_at(
            200.0,
            100.0,
            100.0,
            20.0,
            &[("wall_slits", choice("all")), ("slit_spacing", mm(1.5))],
        )
    };
    let _warm = build();
    let mut best = u64::MAX;
    let mut segments = 0;
    for _ in 0..3 {
        let before = thread_ticks().expect("the thread CPU clock is readable");
        let generated = build();
        let after = thread_ticks().expect("the thread CPU clock is readable");
        segments = generated
            .parts
            .iter()
            .flat_map(|p| p.paths.iter())
            .map(|p| p.segments.len())
            .sum::<usize>();
        best = best.min(after - before);
    }
    eprintln!("criterion 8: {segments} segments, best run {best} clock ticks (10 ms each)");
    assert!(
        segments >= 1000,
        "the box carries about 400 slots: {segments} segments"
    );
    let took = budget_factor::scaled(std::time::Duration::from_millis(best * 10));
    assert!(
        took <= std::time::Duration::from_millis(100),
        "regeneration took {} ms of thread CPU (scaled by the CI factor: {took:?})",
        best * 10
    );
}
