//! Acceptance of `specs/0155-generator-joint-options`, slice 1 (criteria 1 to 3): on a bent run the
//! unslit flat across a tab window is the window's chord, `l = 2 t + 2 t_w`, plus 0.1 mm at most.
//! Black-box: measured on the generated strips, for every tab window, for each pattern.

#![allow(clippy::unwrap_used, clippy::many_single_char_names)]

use curvyo_document_core::Point;
use curvyo_extension_core::{
    ClosedPath, Cubic, Extension, GenerateRequest, Generated, ParamValue, Part, Seed, StoredParams,
};
use curvyo_generators_core::{GENERATORS, PATH_BOX, ROUNDED_BOX};
use serde_json::{Value, json};

type Pt = (f64, f64);
type Triple = ((f64, f64), (f64, f64), (f64, f64));

const THICKNESS: f64 = 3.0;
/// The chord of a tab window at 3 mm material and 3 mm plates: `2 t + 2 t_w`.
const CHORD: f64 = 2.0 * THICKNESS + 2.0 * THICKNESS;
/// A slit column nearer than this to a tab slot on both sides belongs to a bent run.
const NEAR: f64 = 40.0;

fn sample(path: &ClosedPath) -> Vec<Pt> {
    const STEPS: u32 = 16;
    let mut points = Vec::new();
    let mut from = path.start;
    for c in &path.segments {
        for step in 0..STEPS {
            let u = f64::from(step) / f64::from(STEPS);
            let v = 1.0 - u;
            let f = |a: f64, b: f64, c1: f64, d: f64| {
                v * v * v * a + 3.0 * v * v * u * b + 3.0 * v * u * u * c1 + u * u * u * d
            };
            points.push((
                f(from.x, c.c1.x, c.c2.x, c.to.x),
                f(from.y, c.c1.y, c.c2.y, c.to.y),
            ));
        }
        from = c.to;
    }
    points
}

/// Every gap (cut) a line along the strip meets, over all heights: `(from, to)` along the strip.
fn gaps(part: &Part, along_x: bool) -> Vec<(f64, f64)> {
    let paths: Vec<Vec<Pt>> = part
        .paths
        .iter()
        .map(|p| {
            sample(p)
                .into_iter()
                .map(|(x, y)| if along_x { (x, y) } else { (y, x) })
                .collect()
        })
        .collect();
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for &(_, v) in paths.iter().flatten() {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    let mut out = Vec::new();
    let mut v = lo + 0.05;
    while v < hi {
        let mut cross: Vec<f64> = Vec::new();
        for path in &paths {
            for i in 0..path.len() {
                let (u0, v0) = path[i];
                let (u1, v1) = path[(i + 1) % path.len()];
                if (v0 - v) * (v1 - v) < 0.0 {
                    cross.push(u0 + (v - v0) / (v1 - v0) * (u1 - u0));
                }
            }
        }
        cross.sort_by(f64::total_cmp);
        for pair in cross[1..].chunks(2) {
            if let [a, b] = pair {
                out.push((*a, *b));
            }
        }
        v += 0.1;
    }
    out
}

/// The flat of every tab window of a wall strip: the distance between the nearest slit cuts either
/// side of a tab slot, for the windows that have slit columns near on both sides.
fn flats(part: &Part, along_x: bool) -> Vec<f64> {
    let all = gaps(part, along_x);
    let (tabs, slits): (Vec<_>, Vec<_>) = all.into_iter().partition(|(a, b)| b - a > 4.0);
    let mut windows: Vec<(f64, f64)> = Vec::new();
    let mut tabs = tabs;
    tabs.sort_by(|p, q| p.0.total_cmp(&q.0));
    for (a, b) in tabs {
        match windows.last_mut() {
            Some(last) if a <= last.1 => last.1 = last.1.max(b),
            _ => windows.push((a, b)),
        }
    }
    let mut out = Vec::new();
    for (a, b) in windows {
        let left = slits
            .iter()
            .filter(|s| s.1 <= a + 1e-6)
            .map(|s| s.1)
            .fold(f64::MIN, f64::max);
        let right = slits
            .iter()
            .filter(|s| s.0 >= b - 1e-6)
            .map(|s| s.0)
            .fold(f64::MAX, f64::min);
        if a - left < NEAR && right - b < NEAR {
            out.push(right - left);
        }
    }
    out
}

fn strips(generated: &Generated) -> Vec<(&Part, bool)> {
    generated
        .parts
        .iter()
        .filter_map(|part| {
            let points: Vec<Pt> = part.paths.iter().flat_map(sample).collect();
            let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
            for &(x, y) in &points {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
            // The strip is the thin part: 56 mm high (50 inside, two 3 mm plates), long.
            let (w, h) = (x1 - x0, y1 - y0);
            ((h - 56.0).abs() < 0.5 && w > 100.0)
                .then_some((part, true))
                .or_else(|| ((w - 56.0).abs() < 0.5 && h > 100.0).then_some((part, false)))
        })
        .collect()
}

fn mm(value: f64) -> Value {
    json!({ "mm": value })
}

fn rounded(w: f64, d: f64, r: f64, pattern: &str, kerf: f64) -> Generated {
    let mut stored = StoredParams::new();
    for (k, v) in [
        ("inner_width", mm(w)),
        ("inner_depth", mm(d)),
        ("corner_radius", mm(r)),
        ("kerf", mm(kerf)),
        ("slit_pattern", json!({ "choice": pattern })),
    ] {
        stored.insert(k.to_owned(), v);
    }
    GENERATORS
        .generate(&GenerateRequest {
            kind: ROUNDED_BOX,
            params: &stored,
            seed: Seed(1),
            edit: None,
        })
        .unwrap()
}

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn curve(start: Point, cubics: &[Triple]) -> ClosedPath {
    ClosedPath {
        start,
        segments: cubics
            .iter()
            .map(|&(c1, c2, to)| Cubic {
                c1: p(c1.0, c1.1),
                c2: p(c2.0, c2.1),
                to: p(to.0, to.1),
            })
            .collect(),
    }
}

fn heart() -> ClosedPath {
    curve(
        p(0.0, 40.0),
        &[
            ((-10.0, 25.0), (-40.0, 10.0), (-40.0, -10.0)),
            ((-40.0, -35.0), (-12.0, -32.0), (0.0, -22.0)),
            ((12.0, -32.0), (40.0, -35.0), (40.0, -10.0)),
            ((40.0, 10.0), (10.0, 25.0), (0.0, 40.0)),
        ],
    )
}

fn ellipse(a: f64, b: f64) -> ClosedPath {
    let k = 0.552_284_75;
    curve(
        p(a, 0.0),
        &[
            ((a, k * b), (k * a, b), (0.0, b)),
            ((-k * a, b), (-a, k * b), (-a, 0.0)),
            ((-a, -k * b), (-k * a, -b), (0.0, -b)),
            ((k * a, -b), (a, -k * b), (a, 0.0)),
        ],
    )
}

fn path_box(path: ClosedPath, pattern: &str, kerf: f64) -> Generated {
    let mut stored = StoredParams::new();
    stored.insert("outline".to_owned(), ParamValue::Path(path).to_stored());
    stored.insert("kerf".to_owned(), mm(kerf));
    stored.insert("slit_pattern".to_owned(), json!({ "choice": pattern }));
    GENERATORS
        .generate(&GenerateRequest {
            kind: PATH_BOX,
            params: &stored,
            seed: Seed(1),
            edit: None,
        })
        .unwrap()
}

/// The flat is measured on the drawn outlines, which are the cut's centre lines: each drawn slit
/// edge stands kerf / 2 inside the cut, so the flat of the cut is the drawn flat less the kerf.
fn assert_tight(name: &str, generated: &Generated, kerf: f64) -> usize {
    let mut count = 0;
    for (part, along_x) in strips(generated) {
        for flat in flats(part, along_x) {
            count += 1;
            assert!(
                flat - kerf <= CHORD + 0.1,
                "{name}: the flat across a tab is {flat:.2} mm, chord {CHORD}"
            );
        }
    }
    count
}

#[test]
fn criterion_1_the_flat_across_a_tab_on_a_bend_is_the_window_chord_plus_a_tenth() {
    for pattern in ["straight", "offset", "wave"] {
        for kerf in [0.0, 0.2] {
            let mut seen = 0;
            seen += assert_tight(
                &format!("circle {pattern} {kerf}"),
                &rounded(60.0, 60.0, 30.0, pattern, kerf),
                kerf,
            );
            seen += assert_tight(
                &format!("stadium {pattern} {kerf}"),
                &rounded(60.0, 66.0, 30.0, pattern, kerf),
                kerf,
            );
            seen += assert_tight(
                &format!("heart {pattern} {kerf}"),
                &path_box(heart(), pattern, kerf),
                kerf,
            );
            seen += assert_tight(
                &format!("ellipse {pattern} {kerf}"),
                &path_box(ellipse(45.0, 30.0), pattern, kerf),
                kerf,
            );
            assert!(seen >= 12, "{pattern} {kerf}: only {seen} windows measured");
        }
    }
}

/// The slit columns of a strip: the distinct narrow cuts along it, whatever their height.
fn columns(part: &Part, along_x: bool) -> usize {
    let mut cuts: Vec<(f64, f64)> = gaps(part, along_x)
        .into_iter()
        .filter(|(a, b)| b - a < 4.0)
        .collect();
    cuts.sort_by(|p, q| p.0.total_cmp(&q.0));
    cuts.dedup_by(|a, b| (a.0 - b.0).abs() < 0.2);
    cuts.len()
}

/// Criterion 2 (pinned figures): the circle golden had 14 columns on its strip (twelve between the
/// windows and one at each seam end); each of the five full bands between two windows now holds
/// four, the two seam bands two each, 24 in all.
#[test]
fn criterion_2_the_circle_strip_holds_24_columns_with_an_even_pitch_from_the_band_ends() {
    let generated = rounded(60.0, 60.0, 30.0, "straight", 0.0);
    let strips = strips(&generated);
    assert_eq!(strips.len(), 1);
    assert_eq!(columns(strips[0].0, strips[0].1), 24);
}

/// Criterion 2 (revision 2): inside a band the pitch is at least `g = max(0.22 r, slit width + 1)`,
/// 6.6 mm on the circle; across a window the columns stand one chord apart.
#[test]
fn criterion_2_no_pitch_of_the_circle_is_below_the_radius_gap() {
    let generated = rounded(60.0, 60.0, 30.0, "straight", 0.0);
    let (part, along_x) = strips(&generated)[0];
    let mut cuts: Vec<(f64, f64)> = gaps(part, along_x)
        .into_iter()
        .filter(|(a, b)| b - a < 4.0)
        .collect();
    cuts.sort_by(|p, q| p.0.total_cmp(&q.0));
    cuts.dedup_by(|a, b| (a.0 - b.0).abs() < 0.2);
    let centres: Vec<f64> = cuts.iter().map(|c| f64::midpoint(c.0, c.1)).collect();
    for pair in centres.windows(2) {
        let pitch = pair[1] - pair[0];
        assert!(
            pitch >= 6.6 - 0.01,
            "a pitch of {pitch:.3} mm is below the gap 6.6 mm"
        );
    }
}
