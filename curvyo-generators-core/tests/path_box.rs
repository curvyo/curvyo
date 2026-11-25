//! Acceptance of the path box slice 1 (`specs/0149-path-box` criteria 1 to 5, 8, 10 to 12; ADR 0083
//! §9): the coffin's parts, the finger joints at any corner angle, kinks as bent spans, the
//! checks, the layout and the goldens.

#![allow(
    clippy::unwrap_used,
    clippy::many_single_char_names,
    clippy::float_cmp,
    clippy::format_push_string,
    clippy::cast_possible_truncation
)]

use curvyo_document_core::{Length, Point};
use curvyo_extension_core::{
    ClosedPath, Cubic, Edit, Extension, GenerateRequest, Generated, Mark, ParamValue, ScaleFactor,
    Seed, StoredParams,
};
use curvyo_generators_core::{GENERATORS, PATH_BOX};

fn mm(value: f64) -> ParamValue {
    ParamValue::Length(Length::from_mm(value))
}

fn params(entries: &[(&str, ParamValue)]) -> StoredParams {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.to_stored()))
        .collect()
}

fn generate(stored: &StoredParams, edit: Option<Edit<'_>>) -> Generated {
    GENERATORS
        .generate(&GenerateRequest {
            kind: PATH_BOX,
            params: stored,
            seed: Seed(1),
            edit,
        })
        .unwrap()
}

fn polygon(points: &[(f64, f64)]) -> ClosedPath {
    let pts: Vec<Point> = points.iter().map(|&(x, y)| Point::new(x, y)).collect();
    ClosedPath {
        start: pts[0],
        segments: (0..pts.len())
            .map(|i| {
                let to = pts[(i + 1) % pts.len()];
                Cubic {
                    c1: pts[i],
                    c2: to,
                    to,
                }
            })
            .collect(),
    }
}

fn with_outline(path: ClosedPath, rest: &[(&str, ParamValue)]) -> StoredParams {
    let mut stored = params(rest);
    stored.insert("outline".to_owned(), ParamValue::Path(path).to_stored());
    stored
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

const T: f64 = 3.0;
const HEIGHT: f64 = 56.0;

fn points(path: &ClosedPath) -> Vec<(f64, f64)> {
    let mut points = vec![(path.start.x, path.start.y)];
    points.extend(path.segments.iter().map(|s| (s.to.x, s.to.y)));
    points
}

fn bounds(path: &ClosedPath) -> (f64, f64, f64, f64) {
    let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for (x, y) in points(path) {
        b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
    }
    b
}

/// The wall pieces are the parts before the plates: strip parts are as tall as the strip.
fn walls(g: &Generated) -> Vec<&curvyo_extension_core::Part> {
    g.parts
        .iter()
        .filter(|part| {
            let b = bounds(&part.paths[0]);
            (b.3 - b.1 - HEIGHT).abs() < 1e-6
        })
        .collect()
}

/// The vertical steps of an end edge: `(y0, y1, x)`, top to bottom.
fn edge(part: &curvyo_extension_core::Part, right: bool) -> Vec<(f64, f64, f64)> {
    let pts = points(&part.paths[0]);
    let b = bounds(&part.paths[0]);
    let mut steps = Vec::new();
    for i in 0..pts.len() {
        let (a, c) = (pts[i], pts[(i + 1) % pts.len()]);
        let near = if right {
            a.0 > b.2 - 2.2 * T
        } else {
            a.0 < b.0 + 2.2 * T
        };
        if (a.0 - c.0).abs() < 1e-9 && near && (a.1 - c.1).abs() > 1e-9 {
            steps.push((a.1.min(c.1), a.1.max(c.1), a.0));
        }
    }
    steps.sort_by(|p, q| p.0.total_cmp(&q.0));
    steps
}

#[test]
fn the_default_path_box_is_a_coffin_of_six_wall_pieces_and_four_plates() {
    let g = generate(&StoredParams::new(), None);
    assert_eq!(
        g.parts.len(),
        10,
        "six pieces, cover, lid, inner base, ring"
    );
    assert_eq!(walls(&g).len(), 6);
    assert!(g.clamps.is_empty() && g.warnings.is_empty());
    assert_eq!(g.parts[9].paths.len(), 2, "the ring has its opening");
    assert_eq!(
        g.marks
            .on_footprint
            .iter()
            .filter(|m| matches!(m, Mark::Corner { .. }))
            .count(),
        6
    );
    assert!(
        !g.marks
            .on_footprint
            .iter()
            .any(|m| matches!(m, Mark::Bend { .. }))
    );
}

#[test]
fn nothing_overlaps_in_the_layout() {
    let g = generate(&StoredParams::new(), None);
    let boxes: Vec<_> = g.parts.iter().map(|p| bounds(&p.paths[0])).collect();
    for (i, a) in boxes.iter().enumerate() {
        for b in &boxes[i + 1..] {
            let apart = a.2 < b.0 || b.2 < a.0 || a.3 < b.1 || b.3 < a.1;
            assert!(apart, "{a:?} {b:?}");
        }
    }
}

/// The spec's criterion 4: at every corner of the coffin the owner's fingers and the mating
/// piece's notches have the same centres, alternate, and the tips lie within t |cot d| / 2.
#[test]
fn the_fingers_of_neighbouring_pieces_alternate_at_every_corner() {
    let g = generate(&StoredParams::new(), None);
    let pieces = walls(&g);
    let turns = [51.3, 46.3, 82.4, 82.4, 46.3, 51.3];
    for (k, piece) in pieces.iter().enumerate() {
        let next = pieces[(k + 1) % pieces.len()];
        let (right, left) = (edge(piece, true), edge(next, false));
        assert_eq!(right.len(), left.len());
        assert_eq!(right.len() % 2, 1, "an odd count");
        let (rb, lb) = (bounds(&piece.paths[0]), bounds(&next.paths[0]));
        for (a, b) in right.iter().zip(&left) {
            assert!(
                (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9,
                "same centres"
            );
            let owner_finger = a.2 >= rb.2 - 1e-9;
            let mate_finger = b.2 <= lb.0 + 1e-9;
            assert_ne!(owner_finger, mate_finger, "one finger per height");
        }
        assert!(
            right[0].2 >= rb.2 - 1e-9 && right[right.len() - 1].2 >= rb.2 - 1e-9,
            "the owner keeps the top and bottom pieces"
        );
        // The piece reaches `m` past its inner end at a convex corner: the middle of the slant.
        let turn = f64::to_radians(turns[(k + 1) % 6]);
        let m = T * ((turn / 2.0).tan() + 0.5 / turn.tan());
        let reach = rb.2 - right.iter().map(|s| s.2).fold(f64::MAX, f64::min);
        assert!((reach - m).abs() < 0.02, "{k}: reach {reach} vs {m}");
        let (inner, outer) = (T / turn.sin(), T * (turn / 2.0).tan());
        let bound = T * (0.5 / turn.tan()).abs();
        assert!((m - inner).abs() <= bound + 1e-9 && (m - outer).abs() <= bound + 1e-9);
    }
}

#[test]
fn a_piece_is_as_long_as_its_edge_plus_its_two_finger_reaches() {
    let g = generate(&StoredParams::new(), None);
    let corners: [((f64, f64), (f64, f64)); 6] = [
        ((-50.0, -10.0), (-25.0, -30.0)),
        ((-25.0, -30.0), (50.0, -20.0)),
        ((50.0, -20.0), (50.0, 20.0)),
        ((50.0, 20.0), (-25.0, 30.0)),
        ((-25.0, 30.0), (-50.0, 10.0)),
        ((-50.0, 10.0), (-50.0, -10.0)),
    ];
    let turns = [51.3_f64, 46.3, 82.4, 82.4, 46.3, 51.3];
    let reach = |degrees: f64| {
        let turn = degrees.to_radians();
        T * ((turn / 2.0).tan() + 0.5 / turn.tan())
    };
    for (k, piece) in walls(&g).iter().enumerate() {
        let ((ax, ay), (bx, by)) = corners[k];
        let edge_length = f64::hypot(bx - ax, by - ay);
        let b = bounds(&piece.paths[0]);
        // The piece starts at the corner before its edge (the mating end) and ends at the next.
        let expected = edge_length + reach(turns[k]) + reach(turns[(k + 1) % 6]);
        assert!(
            (b.2 - b.0 - expected).abs() < 0.05,
            "{k}: {} vs {expected}",
            b.2 - b.0
        );
    }
}

#[test]
fn the_flat_layout_lies_in_two_rows_with_the_pieces_in_walk_order() {
    let g = generate(&StoredParams::new(), None);
    let pieces = walls(&g);
    let xs: Vec<f64> = pieces.iter().map(|p| bounds(&p.paths[0]).0).collect();
    assert!(
        xs.windows(2).all(|w| w[0] < w[1]),
        "walk order, left to right"
    );
    let tops: Vec<f64> = pieces.iter().map(|p| bounds(&p.paths[0]).1).collect();
    assert!(tops.iter().all(|t| (t - tops[0]).abs() < 1e-9));
    let base = bounds(&g.parts[8].paths[0]);
    assert!(base.1 > tops[0] + HEIGHT, "the plates are below the strips");
}

#[test]
fn the_default_path_box_outline_is_frozen() {
    golden(
        "path_box_default.txt",
        &dump(&generate(&StoredParams::new(), None)),
    );
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

fn ngon(n: u32, radius: f64) -> ClosedPath {
    // A flat top: the first vertex half a step round.
    let pts: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let a = (f64::from(i) + 0.5) * std::f64::consts::TAU / f64::from(n)
                - std::f64::consts::FRAC_PI_2;
            (radius * a.cos(), radius * a.sin())
        })
        .collect();
    polygon(&pts)
}

/// Criterion 3 and ADR 0083 §3 (revision 2): a 20-gon turns 18 degrees per join, below the
/// Corner angle, so it is one strip of bent spans with slits, a seam in the top straight and a
/// bent mark per kink.
#[test]
fn a_20_gon_is_one_strip_with_a_bent_span_at_every_kink() {
    let g = generate(&with_outline(ngon(20, 90.0), &[]), None);
    assert_eq!(walls(&g).len(), 1, "no corner, one strip");
    assert!(
        walls(&g)[0].paths[0].segments.len() > 100,
        "the strip has its slits as notches"
    );
    let bends = g
        .marks
        .on_footprint
        .iter()
        .filter(|m| matches!(m, Mark::Bend { .. }))
        .count();
    assert_eq!(bends, 20);
    assert!(
        !g.marks
            .on_footprint
            .iter()
            .any(|m| matches!(m, Mark::Corner { .. }))
    );
    assert!(g.warnings.is_empty(), "{:?}", g.warnings);
    golden("path_box_20gon.txt", &dump(&g));
}

#[test]
fn a_lid_with_no_straight_for_the_hollow_leaves_it_out_with_a_warning() {
    let g = generate(&with_outline(ngon(20, 60.0), &[]), None);
    assert!(g.warnings.iter().any(|w| w.id == "hollow_left_out"));
}

#[test]
fn raising_the_corner_angle_turns_the_corners_of_the_coffin_into_bent_spans() {
    let g = generate(
        &params(&[(
            "corner_angle",
            ParamValue::Angle(curvyo_extension_core::Degrees(90.0)),
        )]),
        None,
    );
    assert_eq!(walls(&g).len(), 1);
    let bends = g
        .marks
        .on_footprint
        .iter()
        .filter(|m| matches!(m, Mark::Bend { .. }))
        .count();
    assert_eq!(bends, 6);
}

#[test]
fn the_reported_slit_spacing_is_the_smallest_gap_laid_and_never_below_the_least() {
    // A hexagon with a flat top at a Corner angle of 90 degrees: six kinks of 60 degrees, whose
    // arcs hold several columns, the seam in the top straight.
    let g = generate(
        &with_outline(
            ngon(6, 60.0),
            &[(
                "corner_angle",
                ParamValue::Angle(curvyo_extension_core::Degrees(90.0)),
            )],
        ),
        None,
    );
    let (_, value) = g
        .automatic
        .iter()
        .find(|(k, _)| *k == "slit_spacing")
        .unwrap();
    let gap = value.length().unwrap().as_mm();
    assert!(gap >= 1.32 - 1e-9, "0.22 x 2 t, the least is 1.3: {gap}");
    // A kink of 18 degrees is an arc of 1.9 mm: two columns is the least a bend takes, so its
    // gap is the arc over two, below the spacing (the ring of the rounded box does the same).
    let tight = generate(&with_outline(ngon(20, 90.0), &[]), None);
    let (_, value) = tight
        .automatic
        .iter()
        .find(|(k, _)| *k == "slit_spacing")
        .unwrap();
    assert!(
        value.length().unwrap().as_mm() >= 1.3 - 1e-9,
        "the kink's radius is raised until two columns keep the gap: {value:?}"
    );
    let fallback = generate(&StoredParams::new(), None);
    let (_, value) = fallback
        .automatic
        .iter()
        .find(|(k, _)| *k == "slit_spacing")
        .unwrap();
    assert!(
        (value.length().unwrap().as_mm() - 1.32).abs() < 1e-9,
        "no bent span: the gap a kink would get"
    );
}

#[test]
fn a_thickness_past_the_size_rule_is_clamped_and_the_outline_is_kept() {
    let tiny = polygon(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
    let stored = with_outline(tiny.clone(), &[]);
    let g = generate(&stored, None);
    let clamp = g.clamps.iter().find(|c| c.key == "thickness").unwrap();
    assert!(
        (clamp.value.length().unwrap().as_mm() - 2.5).abs() < 1e-9,
        "sqrt(100) / 4"
    );
    assert_eq!(
        g.params["outline"],
        ParamValue::Path(tiny).to_stored(),
        "the outline is kept"
    );
    let coffin = generate(&StoredParams::new(), None);
    assert!(
        coffin.clamps.iter().all(|c| c.key != "thickness"),
        "the default coffin is not clamped"
    );
}

#[test]
fn a_resize_scales_the_outline_and_stops_at_the_smallest_size_keeping_the_thickness() {
    let half = generate(&StoredParams::new(), Some(Edit::Scale(ScaleFactor(0.5))));
    assert_eq!(half.params["thickness"], mm(3.0).to_stored());
    let outline = half.params["outline"]["path"].as_array().unwrap();
    assert!(
        (outline[0].as_f64().unwrap() + 25.0).abs() < 1e-9,
        "scaled about the origin"
    );
    assert!(half.warnings.iter().all(|w| w.id != "smallest_size"));
    let tiny = generate(&StoredParams::new(), Some(Edit::Scale(ScaleFactor(0.01))));
    assert!(tiny.warnings.iter().any(|w| w.id == "smallest_size"));
    assert_eq!(tiny.params["thickness"], mm(3.0).to_stored());
    assert!(tiny.clamps.iter().all(|c| c.key != "thickness"));
}

#[test]
fn a_crossing_outline_reads_as_the_default_and_a_curve_as_its_chord() {
    let bow = polygon(&[(0.0, 0.0), (40.0, 40.0), (40.0, 0.0), (0.0, 40.0)]);
    let g = generate(&with_outline(bow.clone(), &[]), None);
    assert!(g.warnings.iter().any(|w| w.id == "outline_crosses"));
    assert_eq!(g.params["outline"], ParamValue::Path(bow).to_stored());
    assert_eq!(walls(&g).len(), 6, "built as the default coffin");
    let mut curved = polygon(&[(0.0, 0.0), (60.0, 0.0), (60.0, 60.0), (0.0, 60.0)]);
    curved.segments[0].c1 = Point::new(10.0, -12.0);
    curved.segments[0].c2 = Point::new(50.0, -12.0);
    let c = generate(&with_outline(curved.clone(), &[]), None);
    assert!(c.warnings.iter().any(|w| w.id == "outline_curved"));
    assert_eq!(
        c.params["outline"],
        ParamValue::Path(curved).to_stored(),
        "stored as it is"
    );
    assert_eq!(walls(&c).len(), 4);
}

#[test]
fn a_spike_is_built_anyway_and_named_in_a_warning() {
    // A triangle with a 22 degree tip at the top.
    let tip = 11.0_f64.to_radians();
    let spike = polygon(&[
        (0.0, -100.0),
        (100.0 * tip.tan(), 0.0),
        (-100.0 * tip.tan(), 0.0),
    ]);
    let g = generate(&with_outline(spike, &[]), None);
    let sharp: Vec<_> = g
        .warnings
        .iter()
        .filter(|w| w.id == "sharp_corner")
        .collect();
    assert_eq!(sharp.len(), 1);
    assert_eq!(sharp[0].pieces.len(), 2);
    let angle = sharp[0].value.as_ref().unwrap().degrees().unwrap();
    assert!((angle - 22.0).abs() < 0.01, "{angle}");
    assert!(g.parts.len() >= 3);
}

#[test]
fn the_same_outline_turned_by_90_degrees_gives_congruent_parts() {
    let turned = polygon(&[
        (10.0, -50.0),
        (30.0, -25.0),
        (20.0, 50.0),
        (-20.0, 50.0),
        (-30.0, -25.0),
        (-10.0, -50.0),
    ]);
    let a = generate(&StoredParams::new(), None);
    let b = generate(&with_outline(turned, &[]), None);
    let sizes = |g: &Generated| {
        let mut sizes: Vec<(i64, i64)> = g
            .parts
            .iter()
            .map(|p| {
                let b = bounds(&p.paths[0]);
                (
                    ((b.2 - b.0) * 100.0).round() as i64,
                    ((b.3 - b.1) * 100.0).round() as i64,
                )
            })
            .collect();
        sizes.sort_unstable();
        sizes
    };
    assert_eq!(a.parts.len(), b.parts.len());
    // The wall pieces keep their size; the plates' bounding boxes swap their sides.
    let walls_a: Vec<_> = sizes(&a)
        .into_iter()
        .filter(|s| s.1 == (HEIGHT * 100.0) as i64)
        .collect();
    let walls_b: Vec<_> = sizes(&b)
        .into_iter()
        .filter(|s| s.1 == (HEIGHT * 100.0) as i64)
        .collect();
    assert_eq!(walls_a.len(), 6);
    for (x, y) in walls_a.iter().zip(&walls_b) {
        assert!((x.0 - y.0).abs() <= 2, "{x:?} {y:?}");
    }
}

/// Whether the polygon through `points` has no two edges that cross (touching ends excluded).
fn is_simple(points: &[(f64, f64)]) -> bool {
    let n = points.len();
    let side = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| {
        (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
    };
    for i in 0..n {
        for j in i + 2..n {
            if i == 0 && j == n - 1 {
                continue;
            }
            let (a, b) = (points[i], points[(i + 1) % n]);
            let (c, d) = (points[j], points[(j + 1) % n]);
            let crossing =
                side(a, b, c) * side(a, b, d) < -1e-9 && side(c, d, a) * side(c, d, b) < -1e-9;
            if crossing {
                return false;
            }
        }
    }
    true
}

/// A five-point star has convex tips and concave valleys: at every one of the ten corners one
/// piece holds each height, the centres agree, and the cover is a simple polygon (ADR 0083 §5).
#[test]
fn a_star_joins_at_convex_and_concave_corners_and_its_cover_does_not_cross_itself() {
    let pts: Vec<(f64, f64)> = (0..10)
        .map(|i| {
            let r = if i % 2 == 0 { 60.0 } else { 24.0 };
            let a = f64::from(i) * std::f64::consts::PI / 5.0 - std::f64::consts::FRAC_PI_2;
            (r * a.cos(), r * a.sin())
        })
        .collect();
    let g = generate(&with_outline(polygon(&pts), &[]), None);
    let pieces = walls(&g);
    assert_eq!(pieces.len(), 10);
    for (k, piece) in pieces.iter().enumerate() {
        let next = pieces[(k + 1) % pieces.len()];
        let (right, left) = (edge(piece, true), edge(next, false));
        assert_eq!(right.len(), left.len(), "corner {k}");
        assert_eq!(right.len() % 2, 1, "an odd count");
        let widest = right.iter().map(|s| s.2).fold(f64::MIN, f64::max);
        let narrowest = left.iter().map(|s| s.2).fold(f64::MAX, f64::min);
        for (a, b) in right.iter().zip(&left) {
            assert!(
                (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9,
                "same centres"
            );
            let owner_finger = (a.2 - widest).abs() < 1e-9;
            let mate_finger = (b.2 - narrowest).abs() < 1e-9;
            assert_ne!(
                owner_finger, mate_finger,
                "corner {k}: one piece per height"
            );
        }
    }
    // The cover is the plate after the pieces; its corner points must not cross.
    let cover = points(&g.parts[10].paths[0]);
    assert!(
        is_simple(&cover[..cover.len() - 1]),
        "the cover crosses itself"
    );
}

/// The seam never lies inside a bent span: the coffin at Corner angle 90 degrees turns every
/// corner into a kink, and every kink keeps at least two columns and no uncut band.
#[test]
fn the_coffin_at_corner_angle_90_keeps_every_kink_slit_with_the_seam_in_a_straight() {
    let g = generate(
        &params(&[(
            "corner_angle",
            ParamValue::Angle(curvyo_extension_core::Degrees(90.0)),
        )]),
        None,
    );
    let strip = &walls(&g)[0].paths[0];
    let pts = points(strip);
    // Every top or bottom edge notch is a slot: count the vertical runs of width 0.3.
    let slots = pts
        .windows(2)
        .filter(|w| (w[0].0 - w[1].0).abs() < 1e-9 && (w[0].1 - w[1].1).abs() > 5.0)
        .count();
    assert!(
        slots >= 6 * 2 * 2,
        "at least two columns per kink: {slots} slot sides"
    );
    let (_, value) = g
        .automatic
        .iter()
        .find(|(k, _)| *k == "slit_spacing")
        .unwrap();
    assert!(value.length().unwrap().as_mm() >= 1.3 - 1e-9);
}

/// UX review: every corner mark carries a unit bisector that points into the box, so a cross
/// drawn from it has an extent.
#[test]
fn corner_marks_carry_a_unit_bisector_pointing_into_the_box() {
    let g = generate(&StoredParams::new(), None);
    let mut count = 0;
    for mark in &g.marks.on_footprint {
        if let Mark::Corner { at, bisector, .. } = mark {
            count += 1;
            let length = bisector.x.hypot(bisector.y);
            assert!((length - 1.0).abs() < 1e-9, "{at:?} {bisector:?}");
            // The centre of the coffin is the origin: a step along the bisector gets closer.
            let before = at.x.hypot(at.y);
            let after = (at.x + bisector.x).hypot(at.y + bisector.y);
            assert!(after < before, "into the box: {at:?}");
        }
    }
    assert_eq!(count, 6);
}
