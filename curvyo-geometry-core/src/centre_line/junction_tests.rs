//! Junctions and tips (ADR 0063 amendment 2026-11-20, CENTRE-THROUGH A): a sharp crest is one
//! path through its tip, a fin continues the arm it lines up with, and a branch's butt end
//! stays inside the path it joins.

use std::fmt::Write as _;

use curvyo_document_core::{Point as DocPoint, Tolerance, Vec2 as DocVec};

use super::quality_tests::{Quality, polygon_quality, quality};
use super::{CentreLines, CentreOptions, CentreRole, CentreSkeleton};
use crate::{BooleanOp, Outline, OutlineTriple, PathMeasure, boolean};

fn corner(x: f64, y: f64) -> OutlineTriple {
    (DocPoint::new(x, y), DocVec::ZERO, DocVec::ZERO)
}

/// A Λ of two straight strokes `w` wide and 20 mm long meeting at a sharp tip at the origin,
/// the apex angle `2 beta` degrees, cut square at the feet.
fn crest(beta_degrees: f64, w: f64) -> Vec<OutlineTriple> {
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

/// The union of convex polygons, as the subject's outlines.
fn union(parts: &[Vec<OutlineTriple>]) -> Vec<Vec<OutlineTriple>> {
    let outlines: Vec<Vec<Outline<'_>>> =
        parts.iter().map(|p| vec![Outline::new(p, true)]).collect();
    let operands: Vec<&[Outline<'_>]> = outlines.iter().map(Vec::as_slice).collect();
    boolean(BooleanOp::Union, &operands, Tolerance::from_mm(0.01))
        .unwrap()
        .outlines()
        .iter()
        .map(|ring| ring.iter().map(|p| corner(p.x, p.y)).collect())
        .collect()
}

/// A rectangle `w` wide along the segment from `a` to `b`.
fn bar(a: (f64, f64), b: (f64, f64), w: f64) -> Vec<OutlineTriple> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l = dx.hypot(dy);
    let (nx, ny) = (-dy / l * w / 2.0, dx / l * w / 2.0);
    vec![
        corner(a.0 + nx, a.1 + ny),
        corner(b.0 + nx, b.1 + ny),
        corner(b.0 - nx, b.1 - ny),
        corner(a.0 - nx, a.1 - ny),
    ]
}

fn lines_of(subject: &[Vec<OutlineTriple>], options: &CentreOptions) -> (CentreLines, Quality) {
    if let Ok(dir) = std::env::var("CURVYO_JUNCTION_DUMP") {
        let mut text = String::from("subject\n");
        for ring in subject {
            text.push_str("outline true\n");
            for (p, i, o) in ring {
                let _ = writeln!(text, "{} {} {} {} {} {}", p.x, p.y, i.x, i.y, o.x, o.y);
            }
        }
        let name = format!("{dir}/{:x}.txt", text.len());
        std::fs::write(name, text).unwrap();
    }
    let outlines: Vec<Outline<'_>> = subject.iter().map(|r| Outline::new(r, true)).collect();
    let lines = CentreSkeleton::build(&outlines)
        .unwrap()
        .finish(options, &[]);
    let rings: Vec<(Vec<OutlineTriple>, bool)> =
        subject.iter().map(|r| (r.clone(), true)).collect();
    let q = quality(&rings, &lines);
    (lines, q)
}

/// The width of `path` at its point nearest `target`, and that distance.
fn nearest(path: &super::CentrePath, target: kurbo::Point) -> (f64, f64) {
    let measure = PathMeasure::new(&path.anchors, path.closed);
    let total = measure.length().as_mm();
    (0..=2000)
        .map(|i| {
            let at = f64::from(i) / 2000.0;
            let p = measure
                .at(curvyo_document_core::Length::from_mm(at * total))
                .point;
            (
                (kurbo::Point::new(p.x, p.y) - target).hypot(),
                path.width.as_mm() * path.profile.sample(at).unwrap(),
            )
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap()
}

/// A sharp crest (screenshots 20, the dolphin's wave crests) is one path running through its
/// tip, where its width falls to 0; no branch, nothing painted outside the crest.
#[test]
#[ignore = "a measurement of the crest range, run by hand"]
fn measure_crest_range() {
    for beta in [25.0, 28.0, 30.0, 35.0, 40.0, 45.0] {
        let (lines, q) = polygon_quality(&crest(beta, 2.0), &CentreOptions::default());
        let (d, w) = nearest(&lines.paths[0], kurbo::Point::ZERO);
        eprintln!(
            "CREST beta {beta}: {} paths, tip at {d:.3} width {w:.3}, outside {:.3}",
            lines.paths.len(),
            q.outside.0
        );
    }
}

#[test]
fn a_sharp_crest_is_one_path_through_its_tip() {
    for beta in [15.0, 20.0, 25.0] {
        let (lines, q) = polygon_quality(&crest(beta, 2.0), &CentreOptions::default());
        eprintln!("beta {beta}: {q:?}");
        assert_eq!(
            lines.paths.len(),
            1,
            "beta {beta}: {} paths",
            lines.paths.len()
        );
        let (distance, width) = nearest(&lines.paths[0], kurbo::Point::ZERO);
        assert!(
            distance <= 0.06 && width <= 0.06,
            "beta {beta}: tip missed by {distance}, width {width}"
        );
        assert!(
            q.outside.0 <= 0.06,
            "beta {beta}: outside by {:?}",
            q.outside
        );
    }
}

/// A branch leaving a trunk at an angle ends with its butt inside the trunk's paint (screenshots
/// 21, 22): no corner pokes out of the trunk, nothing is painted outside the shape.
#[test]
fn an_oblique_branch_ends_inside_its_trunk() {
    for degrees in [35.0_f64, 45.0, 60.0, 75.0, 90.0] {
        let (s, c) = degrees.to_radians().sin_cos();
        let subject = union(&[
            bar((-20.0, 0.0), (20.0, 0.0), 2.0),
            bar((0.0, 0.0), (15.0 * c, 15.0 * s), 2.0),
        ]);
        for overlap in [0.2, 0.0] {
            let options = CentreOptions {
                overlap,
                ..CentreOptions::default()
            };
            let (lines, q) = lines_of(&subject, &options);
            eprintln!("{degrees}° p {overlap}: {q:?}");
            assert_eq!(lines.paths.len(), 2, "{degrees}°");
            assert_eq!(q.junction_ends, 1, "{degrees}°");
            assert!(
                q.poke.0 <= 0.06,
                "{degrees}° p {overlap}: poke {:?}",
                q.poke
            );
            assert!(
                q.outside.0 <= 0.06,
                "{degrees}° p {overlap}: outside {:?}",
                q.outside
            );
        }
    }
}

/// A thin fin on an obtuse crest (screenshot 23) continues the arm it lines up with as one path
/// to the fin's end; the other arm is a branch whose butt stays inside that path.
#[test]
fn a_fin_on_a_crest_continues_an_arm() {
    let subject = union(&[crest(45.0, 2.0), bar((0.0, 0.5), (0.0, -8.0), 0.5)]);
    let (lines, q) = lines_of(&subject, &CentreOptions::default());
    eprintln!("fin: {q:?}");
    assert_eq!(lines.paths.len(), 2);
    let main = lines
        .paths
        .iter()
        .find(|p| p.role == CentreRole::Main)
        .unwrap();
    let (distance, _) = nearest(main, kurbo::Point::new(0.0, -8.0));
    assert!(
        distance <= 0.3,
        "the main path ends {distance} from the fin's end"
    );
    assert!(q.poke.0 <= 0.06, "poke {:?}", q.poke);
}
