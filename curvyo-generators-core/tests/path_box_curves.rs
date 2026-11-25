//! Acceptance of the path box slice 2, curves (`specs/0149-path-box` criteria 13 to 16, ADR 0083
//! §13): a heart, a shield, an ellipse and a circle build with no refusal, as the pieces, corners
//! and bent spans the spec names, with the goldens of their parts.

#![allow(
    clippy::unwrap_used,
    clippy::many_single_char_names,
    clippy::float_cmp,
    clippy::format_push_string,
    clippy::cast_possible_truncation
)]

use curvyo_document_core::{Length, Point};
use curvyo_extension_core::{
    ClosedPath, Cubic, Edit, Extension, GenerateRequest, Generated, Mark, ParamValue, Part, Seed,
    StoredParams,
};
use curvyo_generators_core::{GENERATORS, PATH_BOX};

/// The strip height with the default values: inner height 50, base 3 and lid 3.
const STRIP: f64 = 56.0;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

type Triple = ((f64, f64), (f64, f64), (f64, f64));

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

fn shield() -> ClosedPath {
    curve(
        p(-30.0, -40.0),
        &[
            ((-30.0, -40.0), (30.0, -40.0), (30.0, -40.0)),
            ((30.0, -40.0), (30.0, 10.0), (30.0, 10.0)),
            ((30.0, 25.0), (10.0, 40.0), (0.0, 40.0)),
            ((-10.0, 40.0), (-30.0, 25.0), (-30.0, 10.0)),
            ((-30.0, 10.0), (-30.0, -40.0), (-30.0, -40.0)),
        ],
    )
}

/// An ellipse of half axes `a` and `b` from four cubics.
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

fn generate_path(path: ClosedPath, edit: Option<Edit<'_>>) -> Generated {
    let mut stored = StoredParams::new();
    stored.insert("outline".to_owned(), ParamValue::Path(path).to_stored());
    GENERATORS
        .generate(&GenerateRequest {
            kind: PATH_BOX,
            params: &stored,
            seed: Seed(1),
            edit,
        })
        .unwrap()
}

fn height(part: &Part) -> f64 {
    let mut lo = f64::MAX;
    let mut hi = f64::MIN;
    for path in &part.paths {
        for y in std::iter::once(path.start.y).chain(path.segments.iter().map(|c| c.to.y)) {
            lo = lo.min(y);
            hi = hi.max(y);
        }
    }
    hi - lo
}

fn walls(g: &Generated) -> Vec<&Part> {
    g.parts
        .iter()
        .filter(|part| (height(part) - STRIP).abs() < 1e-6)
        .collect()
}

fn corners(g: &Generated) -> usize {
    g.marks
        .on_footprint
        .iter()
        .filter(|m| matches!(m, Mark::Corner { .. }))
        .count()
}

#[test]
fn the_four_drawings_build_with_no_refusal_as_the_pieces_the_spec_names() {
    let _ = Length::from_mm(0.0);
    for (name, path, pieces, corner_count) in [
        ("heart", heart(), 2, 2),
        ("shield", shield(), 2, 2),
        ("ellipse", ellipse(45.0, 30.0), 1, 0),
        ("circle", ellipse(30.0, 30.0), 1, 0),
    ] {
        let g = generate_path(path, None);
        assert!(
            g.warnings.iter().all(|w| w.id != "outline_curved"),
            "{name}: no curve refusal"
        );
        assert_eq!(walls(&g).len(), pieces, "{name}: wall pieces");
        assert_eq!(corners(&g), corner_count, "{name}: corners");
    }
}

fn dump(generated: &Generated) -> String {
    let mut text = String::new();
    for (index, part) in generated.parts.iter().enumerate() {
        for path in &part.paths {
            text.push_str(&format!(
                "part {index} start {:.6} {:.6}\n",
                path.start.x, path.start.y
            ));
            for s in &path.segments {
                text.push_str(&format!(
                    "{:.6} {:.6} {:.6} {:.6} {:.6} {:.6}\n",
                    s.c1.x, s.c1.y, s.c2.x, s.c2.y, s.to.x, s.to.y
                ));
            }
        }
    }
    text
}

fn golden(name: &str, actual: &str) {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    if std::env::var_os("UPDATE_GOLDENS").is_some() {
        std::fs::write(&path, actual).unwrap();
    }
    let expected = std::fs::read_to_string(&path).unwrap();
    assert!(
        expected == actual,
        "{name} differs; rerun with UPDATE_GOLDENS=1 after review"
    );
}

/// ADR 0083 §13.15: the goldens of the four drawings, so a change of the fit, the spacing, the
/// flats or the plates shows as a diff.
#[test]
fn the_parts_of_the_heart_shield_ellipse_and_circle_are_frozen() {
    for (name, path) in [
        ("path_box_heart.txt", heart()),
        ("path_box_shield.txt", shield()),
        ("path_box_ellipse.txt", ellipse(45.0, 30.0)),
        ("path_box_circle.txt", ellipse(30.0, 30.0)),
    ] {
        golden(name, &dump(&generate_path(path, None)));
    }
}

fn generate_with(path: ClosedPath, rest: &[(&str, ParamValue)]) -> Generated {
    let mut stored = StoredParams::new();
    stored.insert("outline".to_owned(), ParamValue::Path(path).to_stored());
    for (key, value) in rest {
        stored.insert((*key).to_owned(), value.to_stored());
    }
    GENERATORS
        .generate(&GenerateRequest {
            kind: PATH_BOX,
            params: &stored,
            seed: Seed(1),
            edit: None,
        })
        .unwrap()
}

/// A circle of radius `r` from `n` equal cubics.
fn circle_of(r: f64, n: u32) -> ClosedPath {
    let step = std::f64::consts::TAU / f64::from(n);
    let k = 4.0 / 3.0 * (step / 4.0).tan() * r;
    let at = |a: f64| p(r * a.cos(), r * a.sin());
    let tangent = |a: f64| (-a.sin(), a.cos());
    let segments = (0..n)
        .map(|i| {
            let (a, b) = (f64::from(i) * step, f64::from(i + 1) * step);
            let (ta, tb) = (tangent(a), tangent(b));
            Cubic {
                c1: p(at(a).x + k * ta.0, at(a).y + k * ta.1),
                c2: p(at(b).x - k * tb.0, at(b).y - k * tb.1),
                to: at(b),
            }
        })
        .collect();
    ClosedPath {
        start: at(0.0),
        segments,
    }
}

fn width(part: &Part) -> f64 {
    let xs = part.paths[0]
        .segments
        .iter()
        .map(|c| c.to.x)
        .chain(std::iter::once(part.paths[0].start.x));
    let (lo, hi) = xs.fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(x), b.max(x)));
    hi - lo
}

/// The slits of a wall piece: a slot has two long vertical edges, as outline notch or as a hole.
fn slits(part: &Part) -> usize {
    let mut edges = 0;
    for path in &part.paths {
        let mut from = path.start;
        for c in &path.segments {
            if (c.to.x - from.x).abs() < 1e-9 && (c.to.y - from.y).abs() > 20.0 {
                edges += 1;
            }
            from = c.to;
        }
    }
    edges / 2
}

/// Criterion 14: a circle drawn as four cubics and as eight gives the same strip length within
/// 0.1 mm and the same slit count: the node count of the drawing does not matter.
#[test]
fn a_circle_of_four_and_of_eight_cubics_gives_the_same_strip_and_slits() {
    let (four, eight) = (
        generate_path(circle_of(30.0, 4), None),
        generate_path(circle_of(30.0, 8), None),
    );
    let (a, b) = (&walls(&four)[0], &walls(&eight)[0]);
    assert!(
        (width(a) - width(b)).abs() < 0.1,
        "{} {}",
        width(a),
        width(b)
    );
    assert_eq!(slits(a), slits(b));
    assert!(slits(a) > 10);
}

/// Criterion 15: the heart's corner counts at Corner angle 5, 25 and 90 (recorded here): the tip
/// turns 112 degrees and the notch 80, so at 90 the notch is a kink, built as a bent span.
#[test]
fn the_heart_has_the_recorded_corner_counts_at_corner_angle_5_25_and_90() {
    for (degrees, corner_count, pieces) in [(5.0, 2, 2), (25.0, 2, 2), (90.0, 1, 1)] {
        let g = generate_with(
            heart(),
            &[(
                "corner_angle",
                ParamValue::Angle(curvyo_extension_core::Degrees(degrees)),
            )],
        );
        assert_eq!(corners(&g), corner_count, "corners at {degrees}");
        assert_eq!(walls(&g).len(), pieces, "pieces at {degrees}");
    }
}

/// Criterion 6, 14: the automatic spacing of the heart is the smallest gap laid and never below
/// the least; a tight lobe is built, not clamped.
#[test]
fn the_heart_reports_its_smallest_slit_gap_and_builds_every_wall_piece() {
    let g = generate_path(heart(), None);
    let (_, value) = g
        .automatic
        .iter()
        .find(|(key, _)| *key == "slit_spacing")
        .unwrap();
    assert!(value.length().unwrap().as_mm() >= 1.3 - 1e-9);
    for wall in walls(&g) {
        assert!(
            (height(wall) - STRIP).abs() < 0.5,
            "within 0.5 mm of the wall height"
        );
        assert!(slits(wall) > 5, "the heart's pieces are slit");
    }
}

/// The centres of the slits of a wall piece along the strip, left to right.
fn slit_centres(part: &Part) -> Vec<f64> {
    let mut xs: Vec<f64> = Vec::new();
    for path in &part.paths {
        let mut from = path.start;
        let mut open: Option<f64> = None;
        for c in &path.segments {
            if (c.to.x - from.x).abs() < 1e-9 && (c.to.y - from.y).abs() > 20.0 {
                match open.take() {
                    Some(x0) => xs.push(f64::midpoint(x0, from.x)),
                    None => open = Some(from.x),
                }
            }
            from = c.to;
        }
    }
    xs.sort_by(f64::total_cmp);
    xs
}

/// Criterion 6 (slit gaps): on a bent run the solid wood between neighbouring slit columns stays
/// within the run's gap g (at most 1.5 g: the columns divide the integral of 1/g evenly), except
/// across a flat tab window (its chord 2 w + 2 t = 12 mm, two bridges of 3 mm and the half steps
/// to the next columns) and at the ends of a piece.
#[test]
fn solid_wood_between_slit_columns_on_a_bent_run_stays_within_the_gap_except_at_windows() {
    for (name, path, g_max) in [
        ("heart", heart(), 6.0),
        ("ellipse", ellipse(45.0, 30.0), 10.5),
    ] {
        let g = generate_path(path, None);
        for (k, wall) in walls(&g).iter().enumerate() {
            let xs = slit_centres(wall);
            let gaps: Vec<f64> = xs.windows(2).map(|w| w[1] - w[0]).collect();
            // Gaps this long hold a window; the rest lie inside one slit band.
            let window = 12.0 + 2.0 * 3.0;
            for (i, gap) in gaps.iter().enumerate() {
                let inside = *gap < 14.0;
                if inside {
                    assert!(
                        *gap <= g_max,
                        "{name} piece {k} gap {i}: {gap} over {g_max}"
                    );
                } else {
                    assert!(
                        *gap <= window + 1.5 * g_max,
                        "{name} piece {k} gap {i}: {gap} across a window"
                    );
                }
            }
        }
    }
}

/// The heart with the narrow 53 degree V notch.
fn narrow_heart() -> ClosedPath {
    let mut h = heart();
    h.segments[1].c2 = p(-10.0, -42.0);
    h.segments[2].c1 = p(10.0, -42.0);
    h
}

/// Lead decision on criterion 6: the gap rule (at least the slit width plus 1 mm) holds
/// everywhere; a stretch too short for two columns at that gap gets fewer and a `tight_bend`.
#[test]
fn no_two_slits_of_the_narrow_v_heart_are_closer_than_the_least_gap() {
    let g = generate_path(narrow_heart(), None);
    let (_, value) = g
        .automatic
        .iter()
        .find(|(key, _)| *key == "slit_spacing")
        .unwrap();
    assert!(value.length().unwrap().as_mm() >= 1.3 - 1e-9, "{value:?}");
    for (k, wall) in walls(&g).iter().enumerate() {
        let xs = slit_centres(wall);
        for pair in xs.windows(2) {
            assert!(pair[1] - pair[0] >= 1.3 - 1e-6, "piece {k}: {pair:?}");
        }
    }
}

/// The narrow-V heart of the browser review (`M100 140 C90 125 60 110 60 90 C60 60 96 62 100 80
/// C104 62 140 60 140 90 C140 110 110 125 100 140 Z`): both pieces have a bend too short for two
/// slits, which is told, and no two neighbouring slits are closer than the slit width plus 1 mm.
#[test]
fn the_reviewed_narrow_v_heart_tells_bend_too_short_and_keeps_the_slit_gap() {
    let outline = curve(
        p(100.0, 140.0),
        &[
            ((90.0, 125.0), (60.0, 110.0), (60.0, 90.0)),
            ((60.0, 60.0), (96.0, 62.0), (100.0, 80.0)),
            ((104.0, 62.0), (140.0, 60.0), (140.0, 90.0)),
            ((140.0, 110.0), (110.0, 125.0), (100.0, 140.0)),
        ],
    );
    let g = generate_path(outline, None);
    let mut short: Vec<u32> = g
        .warnings
        .iter()
        .filter(|w| w.id == "bend_too_short")
        .flat_map(|w| w.pieces.clone())
        .collect();
    short.sort_unstable();
    assert_eq!(short, vec![1, 2]);
    for (k, wall) in walls(&g).iter().enumerate() {
        for pair in slit_centres(wall).windows(2) {
            assert!(pair[1] - pair[0] >= 1.3 - 1e-6, "piece {}: {pair:?}", k + 1);
        }
    }
}
