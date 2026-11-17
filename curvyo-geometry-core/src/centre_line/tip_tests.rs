//! The measures of Join tips on the output paths at the outline's own tips (`specs/0117-centre-lines`
//! criterion 9b, ADR 0063 amendment CENTRE-TIPS): reach at width 0, butts near, paint outside,
//! shape left unpainted.

use kurbo::Point;

use super::measure_tests::{painted_rings, traced_lines, traced_subjects};
use super::quality_tests::Rings;
use super::{CentreLines, CentreOptions, CentreRole, CentreSkeleton};
use crate::{Outline, OutlineTriple, PathMeasure};
use curvyo_document_core::{Length, Point as DocPoint, Vec2 as DocVec};

/// The outline is walked this far either side of a vertex to measure its turn, in millimetres.
const TURN_REACH_MM: f64 = 0.3;
/// A tip turns by more than this (its inner angle is below 150 degrees).
const TIP_TURN_DEGREES: f64 = 30.0;
/// Convex corners closer together than this many tip radii are a flat end's corners, no tip.
const FLAT_END_RADII: f64 = 2.5;

/// A tip of a shape: its point, the outline's inward bisector there and the shape's half-width
/// behind it (the largest clearance on the bisector within a few millimetres).
#[derive(Debug, Clone, Copy)]
pub(super) struct Tip {
    pub(super) point: Point,
    pub(super) radius: f64,
}

fn corner(x: f64, y: f64) -> OutlineTriple {
    (DocPoint::new(x, y), DocVec::ZERO, DocVec::ZERO)
}

/// The sharp convex corners of `subject` that are tips, with the shape's half-width behind each.
pub(super) fn tips(subject: &[(Vec<OutlineTriple>, bool)]) -> Vec<Tip> {
    let rings = Rings::new(subject);
    let mut corners: Vec<(Point, kurbo::Vec2)> = Vec::new();
    for (anchors, closed) in subject {
        let ring: Vec<Point> = crate::flatten::flatten(anchors, *closed, 0.005)
            .iter()
            .map(|p| Point::new(p.x, p.y))
            .collect();
        let n = ring.len();
        if n < 4 {
            continue;
        }
        // Arc length positions for the walk either side.
        let mut along = vec![0.0; n + 1];
        for i in 0..n {
            along[i + 1] = along[i] + (ring[(i + 1) % n] - ring[i]).hypot();
        }
        let total = along[n];
        let at = |s: f64| -> Point {
            let s = s.rem_euclid(total);
            let i = along.partition_point(|&a| a <= s).clamp(1, n) - 1;
            let span = along[i + 1] - along[i];
            let t = if span > 0.0 {
                (s - along[i]) / span
            } else {
                0.0
            };
            ring[i].lerp(ring[(i + 1) % n], t)
        };
        let turn = |i: usize| -> (f64, kurbo::Vec2) {
            let here = ring[i];
            let (back, ahead) = (
                here - at(along[i] - TURN_REACH_MM),
                at(along[i] + TURN_REACH_MM) - here,
            );
            let (lb, la) = (back.hypot(), ahead.hypot());
            if lb == 0.0 || la == 0.0 {
                return (0.0, kurbo::Vec2::ZERO);
            }
            let (b, a) = (back / lb, ahead / la);
            let cos = b.dot(a).clamp(-1.0, 1.0);
            let bisector = a - b;
            (
                cos.acos().to_degrees(),
                bisector / bisector.hypot().max(1e-12),
            )
        };
        let turns: Vec<(f64, kurbo::Vec2)> = (0..n).map(turn).collect();
        for i in 0..n {
            let (degrees, bisector) = turns[i];
            if degrees <= TIP_TURN_DEGREES {
                continue;
            }
            // The sharpest vertex of its neighbourhood.
            let here = along[i];
            let sharpest = (0..n).all(|k| {
                let gap = (along[k] - here).abs().min(total - (along[k] - here).abs());
                gap > TURN_REACH_MM || turns[k].0 < degrees || (turns[k].0 <= degrees && k >= i)
            });
            if !sharpest {
                continue;
            }
            // Convex: the shape lies inside the angle, along the inward bisector.
            let probe = ring[i] + bisector * 0.02;
            if rings.inside(probe) {
                corners.push((ring[i], bisector));
            }
        }
    }
    let radius_of = |(point, bisector): (Point, kurbo::Vec2)| {
        let mut best = 0.0_f64;
        for k in 1..=400 {
            let p = point + bisector * (f64::from(k) * 0.01);
            if !rings.inside(p) {
                break;
            }
            best = best.max(rings.distance(p, 5.0));
        }
        best
    };
    let measured: Vec<Tip> = corners
        .iter()
        .map(|&c| Tip {
            point: c.0,
            radius: radius_of(c),
        })
        .collect();
    measured
        .iter()
        .filter(|tip| {
            tip.radius > 0.0
                && !measured.iter().any(|other| {
                    let gap = (other.point - tip.point).hypot();
                    gap > 1e-9 && gap < FLAT_END_RADII * tip.radius
                })
        })
        .copied()
        .collect()
}

/// The measures of one tip of a result.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct TipMeasure {
    /// The nearest distance from the tip to a path's centre line where its width is at most
    /// a tenth of the tip's width (or 0.05 mm), in mm; 0 when a path ends within 2.5 tip radii
    /// (a corner of a stroke's own end, flat or hooked: the end's business, criterion 8).
    pub(super) reach: f64,
    /// Branch butt ends (width above the tolerance) within three tip radii.
    pub(super) butts: usize,
    /// The farthest painted point outside the shape within three tip radii, in mm.
    pub(super) over: f64,
    /// The share of the shape within two tip radii of the tip that no path paints.
    pub(super) under: f64,
}

/// Measures every tip of `subject` on `lines`.
pub(super) fn tip_measures(
    subject: &[(Vec<OutlineTriple>, bool)],
    lines: &CentreLines,
) -> Vec<(Tip, TipMeasure)> {
    let shape = Rings::new(subject);
    let painted: Vec<Rings> = lines
        .paths
        .iter()
        .map(|p| {
            Rings::new(
                &painted_rings(p)
                    .into_iter()
                    .map(|r| (r, true))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    // Each path's centre, sampled every 0.02 mm, with its width.
    let centres: Vec<Vec<(Point, f64)>> = lines
        .paths
        .iter()
        .map(|path| {
            let measure = PathMeasure::new(&path.anchors, path.closed);
            let total = measure.length().as_mm();
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let count = ((total / 0.02).ceil() as usize).max(2);
            (0..=count)
                .map(|k| {
                    #[allow(clippy::cast_precision_loss)]
                    let at = k as f64 / count as f64;
                    let p = measure.at(Length::from_mm(at * total)).point;
                    let w = path.width.as_mm() * path.profile.sample(at).unwrap_or(1.0);
                    (Point::new(p.x, p.y), w)
                })
                .collect()
        })
        .collect();
    tips(subject)
        .into_iter()
        .map(|tip| {
            let near = |p: Point, k: f64| (p - tip.point).hypot() <= k * tip.radius;
            let mut m = TipMeasure {
                reach: f64::INFINITY,
                ..TipMeasure::default()
            };
            for centre in &centres {
                for &(p, w) in centre {
                    if w <= (0.2 * tip.radius).max(0.05) {
                        m.reach = m.reach.min((p - tip.point).hypot());
                    }
                }
                // A stroke's own end (a flat end's corner): its path ends there.
                for &(p, _) in &[centre[0], centre[centre.len() - 1]] {
                    if (p - tip.point).hypot() <= 2.5 * tip.radius {
                        m.reach = 0.0;
                    }
                }
            }
            for (k, path) in lines.paths.iter().enumerate() {
                if path.role == CentreRole::Branch && !path.closed {
                    for &(p, w) in &[centres[k][0], centres[k][centres[k].len() - 1]] {
                        if near(p, 3.0) && w > super::fit::tolerance(w) {
                            m.butts += 1;
                        }
                    }
                }
                for ring in painted_rings(path) {
                    for (p, _, _) in ring {
                        let p = Point::new(p.x, p.y);
                        if near(p, 3.0) && !shape.inside(p) {
                            m.over = m.over.max(shape.distance(p, 5.0));
                        }
                    }
                }
            }
            let step = tip.radius / 10.0;
            let (mut inside, mut bare) = (0_u32, 0_u32);
            for i in -20..=20 {
                for j in -20..=20 {
                    let p = tip.point + kurbo::Vec2::new(f64::from(i), f64::from(j)) * step;
                    if !near(p, 2.0) || !shape.inside(p) || shape.distance(p, 1.0) < step {
                        continue;
                    }
                    inside += 1;
                    if !painted.iter().any(|r| r.inside(p)) {
                        bare += 1;
                    }
                }
            }
            m.under = f64::from(bare) / f64::from(inside.max(1));
            (tip, m)
        })
        .collect()
}

/// A Λ of two straight strokes `w` wide meeting at a sharp tip at the origin, apex `2 beta`.
pub(super) fn crest(beta_degrees: f64, w: f64) -> Vec<OutlineTriple> {
    let (s, c) = beta_degrees.to_radians().sin_cos();
    let length = 20.0;
    let left = (-length * s, length * c);
    let right = (length * s, length * c);
    vec![
        corner(0.0, 0.0),
        corner(left.0, left.1),
        corner(left.0 + w * c, left.1 + w * s),
        corner(0.0, w / s),
        corner(right.0 - w * c, right.1 + w * s),
        corner(right.0, right.1),
    ]
}

/// A drop: a ring of outer radius 5 and stroke `w` whose outline runs out into a pointed tail
/// at (0, -9), its hole a circle.
fn drop_shape(w: f64) -> Vec<(Vec<OutlineTriple>, bool)> {
    let r = 5.0;
    let tail = Point::new(0.0, -9.0);
    // The outer outline: tangents from the tail to the circle, then the circle's far arc.
    let d = (tail - Point::ZERO).hypot();
    let half = (r / d).acos();
    let start = -std::f64::consts::FRAC_PI_2 + half;
    let end = -std::f64::consts::FRAC_PI_2 - half + std::f64::consts::TAU;
    let mut outer = vec![corner(tail.x, tail.y)];
    for k in 0..=48 {
        let a = start + (end - start) * f64::from(k) / 48.0;
        outer.push(corner(r * a.cos(), r * a.sin()));
    }
    let hole_r = r - w;
    let hole: Vec<OutlineTriple> = (0..48)
        .rev()
        .map(|k| {
            let a = std::f64::consts::TAU * f64::from(k) / 48.0;
            corner(hole_r * a.cos(), hole_r * a.sin())
        })
        .collect();
    vec![(outer, true), (hole, true)]
}

fn lines_for(subject: &[(Vec<OutlineTriple>, bool)], join_tips: bool) -> CentreLines {
    let outlines: Vec<Outline<'_>> = subject.iter().map(|(a, c)| Outline::new(a, *c)).collect();
    CentreSkeleton::build(&outlines).unwrap().finish(
        &CentreOptions {
            join_tips,
            ..CentreOptions::default()
        },
        &[],
    )
}

/// Every tip of a crest from a 40 to a 120 degree apex: with Join tips on, one path runs into
/// the tip at width 0, no branch butt lies near it, no paint strays outside the shape and the
/// shape near it is painted.
#[test]
fn join_tips_runs_every_crest_into_its_tip() {
    for beta in [20.0, 30.0, 35.0, 45.0, 60.0] {
        let subject = vec![(crest(beta, 2.0), true)];
        let lines = lines_for(&subject, true);
        let measures = tip_measures(&subject, &lines);
        eprintln!("beta {beta}: {} paths, {measures:?}", lines.paths.len());
        assert_eq!(lines.paths.len(), 1, "beta {beta}");
        let (_, m) = measures
            .iter()
            .find(|(t, _)| t.point.to_vec2().hypot() < 0.05)
            .expect("the crest's tip is found");
        assert!(m.reach <= 0.06, "beta {beta}: reach {}", m.reach);
        assert_eq!(m.butts, 0, "beta {beta}");
        assert!(m.over <= 0.06, "beta {beta}: over {}", m.over);
        assert!(m.under <= 0.05, "beta {beta}: under {}", m.under);
    }
}

/// With Join tips off, a crest is the traditional pair: a main path and a branch whose butt
/// overlaps it near the tip (a wide crest whose spur is pruned stays one path round its corner).
#[test]
fn join_tips_off_keeps_the_branch() {
    for beta in [20.0, 25.0] {
        let subject = vec![(crest(beta, 2.0), true)];
        let lines = lines_for(&subject, false);
        let measures = tip_measures(&subject, &lines);
        eprintln!("off beta {beta}: {} paths, {measures:?}", lines.paths.len());
        assert_eq!(lines.paths.len(), 2, "beta {beta}");
        let (_, m) = measures
            .iter()
            .find(|(t, _)| t.point.to_vec2().hypot() < 0.05)
            .expect("the crest's tip is found");
        assert_eq!(m.butts, 1, "beta {beta}");
    }
}

/// A drop (a ring running out into a pointed tail) is one closed path that runs into the
/// tail's tip and out again with Join tips on; off, it stays the traditional closed ring
/// round its tail's corner (the tail's short spur is pruned).
#[test]
fn join_tips_opens_a_drop_at_its_tail() {
    let subject = drop_shape(1.6);
    let on = lines_for(&subject, true);
    let measures = tip_measures(&subject, &on);
    eprintln!("drop on: {} paths, {measures:?}", on.paths.len());
    assert_eq!(on.paths.len(), 1);
    assert!(on.paths[0].closed);
    let (_, m) = measures
        .iter()
        .find(|(t, _)| (t.point - Point::new(0.0, -9.0)).hypot() < 0.05)
        .expect("the drop's tail is found");
    assert!(m.reach <= 0.06, "reach {}", m.reach);
    assert_eq!(m.butts, 0);
    assert!(m.over <= 0.06, "over {}", m.over);
    assert!(m.under <= 0.05, "under {}", m.under);
    let off = lines_for(&subject, false);
    let measures = tip_measures(&subject, &off);
    eprintln!("drop off: {} paths, {measures:?}", off.paths.len());
    assert_eq!(off.paths.len(), 1);
    assert!(off.paths[0].closed);
}

/// Shapes without a tip give the same result with Join tips on and off: a T, a Y and an
/// oblique branch.
#[test]
fn join_tips_leaves_shapes_without_tips_alone() {
    use super::junction_tests::{bar, union};
    let shapes = [
        union(&[
            bar((-20.0, 0.0), (20.0, 0.0), 2.0),
            bar((0.0, 0.0), (0.0, 15.0), 2.0),
        ]),
        union(&[
            bar((-20.0, 0.0), (20.0, 0.0), 2.0),
            bar((0.0, 0.0), (10.6, 10.6), 2.0),
        ]),
    ];
    for subject in &shapes {
        let subject: Vec<(Vec<OutlineTriple>, bool)> =
            subject.iter().map(|r| (r.clone(), true)).collect();
        let (on, off) = (lines_for(&subject, true), lines_for(&subject, false));
        assert_eq!(on.paths.len(), off.paths.len());
        for (a, b) in on.paths.iter().zip(&off.paths) {
            assert_eq!(a.anchors, b.anchors);
        }
    }
}

/// The traced fixtures with Join tips on (criterion 9b): no tip keeps a branch butt near it,
/// every tip is reached by a path at a tenth of its width, and nothing near a tip is painted
/// outside the shape by more than 0.1 mm.
#[test]
fn join_tips_on_the_traced_fixtures() {
    for (name, text) in [
        (
            "dolphin",
            include_str!("../../tests/fixtures/centre_lines/dolphin_traced.txt"),
        ),
        (
            "whale",
            include_str!("../../tests/fixtures/centre_lines/whale_traced.txt"),
        ),
    ] {
        let report = tip_report(text, true);
        eprintln!("{name}: {report:?}");
        assert_eq!(report.butts, 0, "{name}: tips with a butt");
        assert_eq!(report.unreached, 0, "{name}: tips no path reaches");
        assert!(report.over <= 0.1, "{name}: over {}", report.over);
    }
}

/// A summary of [`tip_measures`] over a traced fixture: tips, those with a butt near them, those
/// no path reaches within 0.1 mm (or a tenth of their radius), the worst overpaint and the
/// mean unpainted share.
#[derive(Debug, Default)]
pub(super) struct TipReport {
    pub(super) tips: usize,
    pub(super) butts: usize,
    pub(super) unreached: usize,
    pub(super) over: f64,
    pub(super) under: f64,
}

/// [`TipReport`] of a traced fixture text with Join tips `join`.
pub(super) fn tip_report(text: &str, join: bool) -> TipReport {
    let options = CentreOptions {
        join_tips: join,
        ..CentreOptions::default()
    };
    let subjects = traced_subjects(text);
    let lines = traced_lines(text, &options);
    let mut report = TipReport::default();
    let mut under = 0.0;
    for (subject, lines) in subjects.iter().zip(&lines) {
        for (tip, m) in tip_measures(subject, lines) {
            // A tip inside a part kept filled is not a centre line's tip.
            let kept = lines.filled.iter().any(|region| {
                Rings::new(&region.iter().map(|r| (r.clone(), true)).collect::<Vec<_>>())
                    .distance(tip.point, 1.0)
                    < 0.05
            });
            if kept {
                continue;
            }
            report.tips += 1;
            if m.butts > 0 {
                report.butts += 1;
                if std::env::var("CURVYO_TIP_LIST").is_ok() {
                    eprintln!(
                        "butt at ({:.1}, {:.1}) r {:.2}",
                        tip.point.x, tip.point.y, tip.radius
                    );
                }
            }
            if m.reach > 0.1_f64.max(0.1 * tip.radius) {
                report.unreached += 1;
                if std::env::var("CURVYO_TIP_LIST").is_ok() {
                    eprintln!(
                        "unreached ({:.1}, {:.1}) r {:.2} by {:.2}",
                        tip.point.x, tip.point.y, tip.radius, m.reach
                    );
                }
            }
            if m.over > 0.1 && std::env::var("CURVYO_TIP_LIST").is_ok() {
                eprintln!(
                    "over ({:.1}, {:.1}) r {:.2} by {:.3}",
                    tip.point.x, tip.point.y, tip.radius, m.over
                );
            }
            report.over = report.over.max(m.over);
            under += m.under;
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let count = report.tips.max(1) as f64;
    report.under = under / count;
    report
}

/// Prints the tip report of a traced fixture (`CURVYO_CENTRE_INPUT`, default the dolphin) with
/// Join tips on and off; `CURVYO_TIP_LIST` lists each tip with a butt or not reached.
#[test]
#[ignore = "a measurement, run by hand"]
fn measure_tips() {
    let text = std::env::var("CURVYO_CENTRE_INPUT").map_or_else(
        |_| include_str!("../../tests/fixtures/centre_lines/dolphin_traced.txt").to_owned(),
        |path| std::fs::read_to_string(path).unwrap(),
    );
    for join in [true, false] {
        eprintln!("join {join}: {:?}", tip_report(&text, join));
    }
}

/// Paints `lines` over `subject` into the SVG file `path` (view box `view`, "x y w h" in mm).
#[allow(dead_code)]
pub(super) fn paint(
    subject: &[(Vec<OutlineTriple>, bool)],
    lines: &CentreLines,
    view: &str,
    path: &str,
) {
    use std::fmt::Write as _;
    let ring = |r: &[OutlineTriple], closed: bool| {
        let mut d = String::new();
        let n = r.len();
        let _ = write!(d, "M{} {}", r[0].0.x, r[0].0.y);
        let count = if closed { n } else { n - 1 };
        for i in 0..count {
            let (a, b) = (r[i], r[(i + 1) % n]);
            let _ = write!(
                d,
                " C{} {} {} {} {} {}",
                a.0.x + a.2.x,
                a.0.y + a.2.y,
                b.0.x + b.1.x,
                b.0.y + b.1.y,
                b.0.x,
                b.0.y
            );
        }
        if closed {
            d.push('Z');
        }
        d
    };
    let mut out = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='{view}' width='800' height='800'>\
         <rect x='-1000' y='-1000' width='3000' height='3000' fill='white'/>\n"
    );
    let d: String = subject.iter().map(|(a, c)| ring(a, *c)).collect();
    let _ = writeln!(
        out,
        "<path d='{d}' fill='#bbb' stroke='#000' stroke-width='0.02'/>"
    );
    for p in &lines.paths {
        let colour = if p.role == CentreRole::Main {
            "#d22"
        } else {
            "#22d"
        };
        let d: String = painted_rings(p).iter().map(|r| ring(r, true)).collect();
        let _ = writeln!(
            out,
            "<path d='{d}' fill='{colour}' fill-opacity='0.25' stroke='{colour}' stroke-width='0.02'/>"
        );
        let _ = writeln!(
            out,
            "<path d='{}' fill='none' stroke='{colour}' stroke-width='0.03'/>",
            ring(&p.anchors, p.closed)
        );
    }
    out.push_str("</svg>\n");
    std::fs::write(path, out).unwrap();
}

/// Paints a synthetic tip case to `CURVYO_TIP_SVG`: `CURVYO_TIP_CASE` "crest <beta>" or "drop".
#[test]
#[ignore = "a picture, made by hand"]
fn paint_tip_case() {
    let case = std::env::var("CURVYO_TIP_CASE").unwrap_or_else(|_| "crest 20".into());
    let join = std::env::var("CURVYO_TIP_OFF").is_err();
    let words: Vec<&str> = case.split_whitespace().collect();
    let (subject, view) = if words[0] == "drop" {
        (drop_shape(1.6), "-7 -11 14 14")
    } else {
        let beta: f64 = words[1].parse().unwrap();
        (vec![(crest(beta, 2.0), true)], "-6 -2 12 12")
    };
    let lines = lines_for(&subject, join);
    paint(
        &subject,
        &lines,
        view,
        &std::env::var("CURVYO_TIP_SVG").unwrap(),
    );
}
