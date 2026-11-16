//! The golden centre lines of traced line drawings (`specs/0117-centre-lines` criteria 19 and
//! 20, ADR 0063 §12): the whale-like drawing and the customer's dolphin with thick wave bands.
//! The expected centre nodes, profile points, branches, kept parts and inner areas are stored
//! as text and compared within 0.01 mm. Embedded with `include_str!`, so the same tests run
//! natively and in Node on `wasm32-unknown-unknown`:
//!
//! ```text
//! cargo test --target wasm32-unknown-unknown -p curvyo-geometry-core --test centre_lines_golden
//! ```
//!
//! Regenerate (reviewed in the PR): `CURVYO_BLESS=1 cargo test -p curvyo-geometry-core --test
//! centre_lines_golden`.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::fmt::Write as _;

use curvyo_document_core::{
    DashPattern, Length, LineCap, LineJoin, Markers, Point, Tolerance, Vec2,
};
use curvyo_geometry_core::{
    BooleanOp, CentreLines, CentreOptions, CentreRole, CentreSkeleton, Outline, PathMeasure,
    StrokeShape, boolean, signed_area_mm2, variable_stroke_outline,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const INPUT: &str = include_str!("fixtures/centre_lines/whale_traced.txt");
const EXPECTED: &str = include_str!("fixtures/centre_lines/whale_expected.txt");
const DOLPHIN: &str = include_str!("fixtures/centre_lines/dolphin_traced.txt");
const DOLPHIN_EXPECTED: &str = include_str!("fixtures/centre_lines/dolphin_expected.txt");

type Triple = (Point, Vec2, Vec2);

/// The subjects of the fixture: per subject its outlines `(anchors, closed)`.
fn subjects(text: &str) -> Vec<Vec<(Vec<Triple>, bool)>> {
    let mut subjects: Vec<Vec<(Vec<Triple>, bool)>> = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.first() {
            Some(&"subject") => subjects.push(Vec::new()),
            Some(&"outline") => subjects
                .last_mut()
                .unwrap()
                .push((Vec::new(), words[1] == "true")),
            Some(_) => {
                let v: Vec<f64> = words.iter().map(|w| w.parse().unwrap()).collect();
                let outline = subjects.last_mut().unwrap().last_mut().unwrap();
                outline.0.push((
                    Point::new(v[0], v[1]),
                    Vec2::new(v[2], v[3]),
                    Vec2::new(v[4], v[5]),
                ));
            }
            None => {}
        }
    }
    subjects
}

fn describe(lines: &CentreLines, out: &mut String) {
    let _ = writeln!(
        out,
        "result paths {} filled {} inner {} failed {}",
        lines.paths.len(),
        lines.filled.len(),
        lines.inner_areas.len(),
        lines.failed
    );
    for path in &lines.paths {
        let role = match path.role {
            CentreRole::Main => "main",
            CentreRole::Branch => "branch",
        };
        let _ = writeln!(
            out,
            "path {role} {} {:.4} {}",
            path.closed,
            path.width.as_mm(),
            path.anchors.len()
        );
        for (p, i, o) in &path.anchors {
            let _ = writeln!(
                out,
                "{:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
                p.x, p.y, i.x, i.y, o.x, o.y
            );
        }
        let profile: Vec<String> = path
            .profile
            .positions()
            .map(|at| format!("{at:.4}:{:.4}", path.profile.sample(at).unwrap()))
            .collect();
        let _ = writeln!(out, "profile {}", profile.join(" "));
    }
    for region in &lines.filled {
        let rings: Vec<String> = region.iter().map(|r| r.len().to_string()).collect();
        let _ = writeln!(out, "filled {}", rings.join(" "));
        for (p, i, o) in region.iter().flatten() {
            let _ = writeln!(
                out,
                "{:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
                p.x, p.y, i.x, i.y, o.x, o.y
            );
        }
    }
    for area in &lines.inner_areas {
        let (sx, sy) = area.iter().fold((0.0, 0.0), |(x, y), p| (x + p.x, y + p.y));
        #[allow(clippy::cast_precision_loss)]
        let n = area.len() as f64;
        let _ = writeln!(out, "inner centroid {:.4} {:.4}", sx / n, sy / n);
    }
}

fn actual(input: &str) -> String {
    let mut out = String::new();
    for subject in subjects(input) {
        let outlines: Vec<Outline<'_>> = subject.iter().map(|(a, c)| Outline::new(a, *c)).collect();
        let skeleton = CentreSkeleton::build(&outlines).unwrap();
        describe(&skeleton.finish(&CentreOptions::default(), &[]), &mut out);
        // Without a Thick parts limit, so the two-hole spout (7.9 mm wide) gives its inner
        // areas (criterion 13).
        let inner = CentreOptions {
            inner_offset: Some(Length::from_mm(0.3)),
            thick_parts: None,
            ..CentreOptions::default()
        };
        describe(&skeleton.finish(&inner, &[]), &mut out);
    }
    out
}

/// Compares word by word: words equal, or numbers within 0.01.
fn same(expected: &str, actual: &str) -> Result<(), String> {
    let (e, a): (Vec<&str>, Vec<&str>) = (
        expected.split_whitespace().collect(),
        actual.split_whitespace().collect(),
    );
    if e.len() != a.len() {
        return Err(format!("{} words expected, {} found", e.len(), a.len()));
    }
    for (i, (x, y)) in e.iter().zip(&a).enumerate() {
        let near = |p: &str, q: &str| match (p.parse::<f64>(), q.parse::<f64>()) {
            (Ok(p), Ok(q)) => (p - q).abs() <= 0.01,
            _ => p == q,
        };
        let pair = x.split(':').zip(y.split(':')).all(|(p, q)| near(p, q));
        if !pair {
            return Err(format!("word {i}: expected {x}, found {y}"));
        }
    }
    Ok(())
}

/// Compares with the golden file `name`, or writes it under `CURVYO_BLESS`.
fn check(name: &str, expected: &str, actual: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var("CURVYO_BLESS").is_ok() {
        let out = format!(
            "{}/tests/fixtures/centre_lines/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::write(out, actual).unwrap();
        return;
    }
    let _ = name;
    same(expected, actual).unwrap();
}

#[test]
fn the_traced_whale_gives_its_golden_centre_lines() {
    check("whale_expected.txt", EXPECTED, &actual(INPUT));
}

#[test]
fn the_traced_dolphin_gives_its_golden_centre_lines() {
    check("dolphin_expected.txt", DOLPHIN_EXPECTED, &actual(DOLPHIN));
}

#[test]
fn the_result_is_bit_identical_on_a_second_run() {
    assert_eq!(actual(INPUT), actual(INPUT));
}

/// The dolphin's subject and its result at the defaults.
fn dolphin() -> (Vec<(Vec<Triple>, bool)>, CentreLines) {
    let subject = subjects(DOLPHIN).remove(0);
    let outlines: Vec<Outline<'_>> = subject.iter().map(|(a, c)| Outline::new(a, *c)).collect();
    let lines = CentreSkeleton::build(&outlines)
        .unwrap()
        .finish(&CentreOptions::default(), &[]);
    (subject, lines)
}

/// The thick bands of a result: paths at least 10 mm long whose median width is at least
/// 1.5 mm (the dolphin's strokes and wave bands; its fine lines are under 1.4 mm), as length,
/// nodes, width points and pointed ends (open ends whose width falls to 0).
fn bands(lines: &CentreLines) -> Vec<(f64, usize, usize, usize)> {
    lines
        .paths
        .iter()
        .filter_map(|path| {
            let length = PathMeasure::new(&path.anchors, path.closed)
                .length()
                .as_mm();
            let width = |t: f64| path.width.as_mm() * path.profile.sample(t).unwrap();
            let mut widths: Vec<f64> = (0..=20).map(|i| width(f64::from(i) / 20.0)).collect();
            widths.sort_by(f64::total_cmp);
            let pointed = if path.closed {
                0
            } else {
                [0.0, 1.0].iter().filter(|&&t| width(t) < 0.05).count()
            };
            (length >= 10.0 && widths[10] >= 1.5).then(|| {
                (
                    length,
                    path.anchors.len(),
                    path.profile.positions().count(),
                    pointed,
                )
            })
        })
        .collect()
}

/// Criterion 20 (ADR 0063 amendment 2026-11-18, B): each of the dolphin's thick bands stays
/// within its own budget of `2 + ceil(0.12 L)` nodes and `6 + 4 p + ceil(0.12 L)` width points
/// (`p` its pointed ends), and none is kept filled.
#[test]
fn criterion_20_the_dolphins_bands_stay_within_the_node_budget() {
    let (_, lines) = dolphin();
    let bands = bands(&lines);
    assert!(bands.len() >= 15, "{} bands", bands.len());
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let per = |l: f64| (0.12 * l).ceil() as usize;
    let over: Vec<_> = bands
        .iter()
        .filter(|b| b.1 > 2 + per(b.0) || b.2 > 6 + 4 * b.3 + per(b.0))
        .collect();
    let nodes: usize = bands.iter().map(|b| b.1).sum();
    let points: usize = bands.iter().map(|b| b.2).sum();
    eprintln!(
        "{} bands: {nodes} nodes, {points} width points, over their own budget: {over:?}",
        bands.len()
    );
    assert!(over.is_empty(), "bands over budget: {over:?}");
    let kept: usize = lines.filled.iter().flatten().map(Vec::len).sum();
    assert!(kept <= 20, "only the eye stays filled: {kept} nodes");
}

/// Criterion 20: the painted result (the paths' variable-width outlines and the kept parts)
/// differs from the dolphin by at most 4 % of its area.
#[test]
fn criterion_20_the_painted_dolphin_matches_its_area() {
    let (subject, lines) = dolphin();
    let tolerance = Tolerance::from_mm(0.01);
    let mut painted: Vec<Vec<Triple>> = lines.filled.iter().flatten().cloned().collect();
    for path in &lines.paths {
        let style = StrokeShape {
            width: path.width,
            join: LineJoin::Round,
            cap: match path.role {
                CentreRole::Main => LineCap::Round,
                CentreRole::Branch => LineCap::Butt,
            },
            dash: DashPattern::default(),
            markers: Markers::default(),
        };
        let outline = variable_stroke_outline(
            Outline::new(&path.anchors, path.closed),
            &style,
            &path.profile,
            tolerance,
        )
        .unwrap();
        painted.extend(outline.outlines().iter().map(|r| {
            r.iter()
                .map(|a| (a.point, a.handle_in, a.handle_out))
                .collect()
        }));
    }
    let area = |rings: &[Outline<'_>]| -> f64 {
        boolean(BooleanOp::Union, &[rings], tolerance)
            .unwrap()
            .outlines()
            .iter()
            .map(|r| signed_area_mm2(r))
            .sum()
    };
    let subject: Vec<Outline<'_>> = subject.iter().map(|(a, c)| Outline::new(a, *c)).collect();
    let painted: Vec<Outline<'_>> = painted.iter().map(|r| Outline::new(r, true)).collect();
    let (want, got) = (area(&subject), area(&painted));
    let both: Vec<Outline<'_>> = subject.iter().chain(&painted).copied().collect();
    let union = area(&both);
    // Painted outside the subject plus subject left unpainted.
    let differ = (union - want) + (union - got);
    eprintln!("painted {got:.1} mm2 of {want:.1} mm2, {differ:.1} mm2 differ");
    assert!(
        differ <= 0.04 * want,
        "painted {got:.1} mm2 of {want:.1} mm2, {differ:.1} mm2 differ"
    );
}

/// Criterion 12 (customer 2026-11-18, point 1): at Thick parts 2.3 mm the dolphin's wave bands
/// are cut partway, and each kept part runs along the subject's own Béziers: every curved
/// segment of a kept outline and every anchor that is not inside a straight run (a chord across
/// a stroke) lies on the subject's outline.
#[test]
fn criterion_12_kept_parts_run_along_the_subjects_own_curves() {
    let subject = subjects(DOLPHIN).remove(0);
    let outlines: Vec<Outline<'_>> = subject.iter().map(|(a, c)| Outline::new(a, *c)).collect();
    let lines = CentreSkeleton::build(&outlines).unwrap().finish(
        &CentreOptions {
            thick_parts: Some(Length::from_mm(2.3)),
            ..CentreOptions::default()
        },
        &[],
    );
    assert!(lines.filled.len() > 1, "bands are cut partway");
    assert!(!lines.paths.is_empty(), "the rest gets centre lines");
    // The subject's outline, finely flattened.
    let mut outline: Vec<kurbo::Point> = Vec::new();
    for (anchors, _) in &subject {
        let n = anchors.len();
        for i in 0..n {
            let (a, b) = (anchors[i], anchors[(i + 1) % n]);
            let cubic = kurbo::CubicBez::new(
                (a.0.x, a.0.y),
                (a.0.x + a.2.x, a.0.y + a.2.y),
                (b.0.x + b.1.x, b.0.y + b.1.y),
                (b.0.x, b.0.y),
            );
            for k in 0..=200 {
                outline.push(kurbo::ParamCurve::eval(&cubic, f64::from(k) / 200.0));
            }
        }
    }
    let off = |p: kurbo::Point| {
        outline
            .windows(2)
            .map(|w| {
                let line = kurbo::Line::new(w[0], w[1]);
                kurbo::ParamCurveNearest::nearest(&line, p, 1e-9)
                    .distance_sq
                    .sqrt()
            })
            .fold(f64::INFINITY, f64::min)
    };
    let straight = |o: Vec2, i: Vec2| o.x == 0.0 && o.y == 0.0 && i.x == 0.0 && i.y == 0.0;
    let mut curves = 0;
    for ring in lines.filled.iter().flatten() {
        let n = ring.len();
        for k in 0..n {
            let (a, b) = (ring[k], ring[(k + 1) % n]);
            let previous = ring[(k + n - 1) % n];
            let at = kurbo::Point::new(a.0.x, a.0.y);
            let inside_a_run = straight(previous.2, a.1) && straight(a.2, b.1);
            if !inside_a_run {
                assert!(off(at) <= 0.02, "anchor {at:?} is {:.3} off", off(at));
            }
            if !straight(a.2, b.1) {
                curves += 1;
                let cubic = kurbo::CubicBez::new(
                    (a.0.x, a.0.y),
                    (a.0.x + a.2.x, a.0.y + a.2.y),
                    (b.0.x + b.1.x, b.0.y + b.1.y),
                    (b.0.x, b.0.y),
                );
                let middle = kurbo::ParamCurve::eval(&cubic, 0.5);
                assert!(
                    off(middle) <= 0.01,
                    "curve at {middle:?} is {:.3} off",
                    off(middle)
                );
            }
        }
    }
    assert!(curves > 10, "{curves} curved segments kept");
}
