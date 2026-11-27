//! The path box through its generate entry: curves, plates and notices (ADR 0083 §13).

use curvyo_extension_core::{
    ClosedPath, Edit, GenerateRequest, Generated, Mark, ParamValue, Seed, StoredParams,
};
use curvyo_geometry_core::{Outline, cut_parameters};

use super::check::{curve_area, triples};
use super::fixtures::{circle, ellipse, heart, peanut, rounded_rect};

fn build(path: &ClosedPath, edit: Option<Edit<'_>>) -> Generated {
    let mut params = StoredParams::new();
    params.insert(
        "outline".to_owned(),
        ParamValue::Path(path.clone()).to_stored(),
    );
    super::generate(&GenerateRequest {
        kind: crate::PATH_BOX,
        params: &params,
        seed: Seed(1),
        edit,
        glyphs: None,
    })
}

fn crosses(path: &ClosedPath) -> bool {
    let anchors = triples(path);
    cut_parameters(&[Outline::new(&anchors, true)])
        .iter()
        .any(|cuts| !cuts.is_empty())
}

fn ids(g: &Generated) -> Vec<&'static str> {
    g.warnings.iter().map(|w| w.id).collect()
}

/// The first plate after the wall pieces is the cover.
fn cover(g: &Generated, walls: usize) -> &ClosedPath {
    &g.parts[walls].paths[0]
}

#[test]
fn a_peanut_gets_a_round_cover_with_the_notice_and_no_hollow_notice() {
    let g = build(&peanut(), None);
    let notice = g.warnings.iter().find(|w| w.id == "cover_rounded").unwrap();
    assert_eq!(
        notice.value,
        Some(ParamValue::Length(curvyo_document_core::Length::from_mm(
            3.0
        )))
    );
    assert!(!ids(&g).contains(&"hollow_left_out"));
    assert!(!crosses(cover(&g, 1)), "the cover does not cross itself");
}

#[test]
fn a_draft_skips_the_plate_tests_and_sends_no_cover_notice() {
    let g = build(&peanut(), Some(Edit::Draft));
    assert!(!ids(&g).contains(&"cover_rounded"));
    assert!(!ids(&g).contains(&"ring_closed"));
}

#[test]
fn a_heart_has_the_exact_cover_and_no_cover_notice() {
    let g = build(&heart(), None);
    assert!(!ids(&g).contains(&"cover_rounded"));
}

/// The cover is the part after the wall pieces, the seat lid the next one.
fn lid(g: &Generated, walls: usize) -> &ClosedPath {
    &g.parts[walls + 1].paths[0]
}

#[test]
fn the_lid_of_a_heart_a_circle_and_an_ellipse_carries_the_hollow_on_a_curved_stretch() {
    for (name, path, walls) in [
        ("heart", heart(), 2),
        ("circle", circle(30.0), 1),
        ("ellipse", ellipse(45.0, 30.0), 1),
    ] {
        let g = build(&path, None);
        assert!(
            !ids(&g).contains(&"hollow_left_out"),
            "{name}: hollow built"
        );
        let (plain, cut) = (
            curve_area(cover(&g, walls)).abs(),
            curve_area(lid(&g, walls)).abs(),
        );
        assert!(
            (80.0..200.0).contains(&(plain - cut)),
            "{name}: the lid is the cover less the notch: {plain} against {cut}"
        );
        assert!(
            !crosses(lid(&g, walls)),
            "{name}: the lid does not cross itself"
        );
    }
}

fn seams(g: &Generated) -> usize {
    g.marks
        .on_footprint
        .iter()
        .filter(|m| matches!(m, Mark::Seam { .. }))
        .count()
}

fn bends(
    g: &Generated,
) -> Vec<(
    &Vec<curvyo_extension_core::Cubic>,
    curvyo_document_core::Point,
    bool,
)> {
    g.marks
        .on_footprint
        .iter()
        .filter_map(|m| match m {
            Mark::Bend {
                start,
                segments,
                tight,
            } => Some((segments, *start, *tight)),
            _ => None,
        })
        .collect()
}

/// ADR 0083 §13.16: an outline with no corner carries one seam mark; one with corners none.
#[test]
fn a_loop_without_a_corner_carries_one_seam_mark_and_the_heart_and_coffin_none() {
    for (name, path, want) in [
        ("circle", circle(30.0), 1),
        ("ellipse", ellipse(45.0, 30.0), 1),
        ("rounded rectangle", rounded_rect(80.0, 50.0, 10.0), 1),
        ("heart", heart(), 0),
        ("coffin", super::check::default_outline(), 0),
    ] {
        assert_eq!(seams(&build(&path, None)), want, "{name}");
    }
}

/// ADR 0083 §13.12, §13.16: a radius below twice the wall thickness is built with a notice naming
/// the piece and a tight stretch; nothing else is red.
#[test]
fn a_tight_corner_is_built_with_tight_bend_and_a_tight_stretch_only_there() {
    let g = build(&rounded_rect(80.0, 50.0, 3.0), None);
    let warning = g.warnings.iter().find(|w| w.id == "tight_bend").unwrap();
    assert_eq!(warning.pieces, vec![1]);
    let mm = |v: &Option<ParamValue>| v.as_ref().unwrap().length().unwrap().as_mm();
    assert!((mm(&warning.value) - 3.0).abs() < 1e-6);
    assert!((mm(&warning.limit) - 6.0).abs() < 1e-6);
    let marks = bends(&g);
    assert_eq!(marks.len(), 4, "one stretch per corner arc");
    assert!(marks.iter().all(|(_, _, tight)| *tight));
}

/// ADR 0083 §13.16: where the tightness changes inside a band, consecutive marks share their end
/// point; the ends of a long thin ellipse are tight, its sides are not.
#[test]
fn touching_stretches_of_one_band_share_an_end_point_and_only_the_tight_arcs_are_red() {
    let g = build(&ellipse(60.0, 14.0), None);
    let marks = bends(&g);
    assert!(marks.iter().any(|(_, _, tight)| *tight));
    assert!(marks.iter().any(|(_, _, tight)| !*tight));
    let mut joined = 0;
    for pair in marks.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        let end = a.0.last().unwrap().to;
        if (end.x - b.1.x).abs() < 1e-9 && (end.y - b.1.y).abs() < 1e-9 && a.2 != b.2 {
            joined += 1;
        }
    }
    assert!(
        joined >= 2,
        "tight and fine stretches meet exactly: {joined}"
    );
}

/// ADR 0083 §13.16: a draft gives the same corner, seam and tight stretches as the full build.
#[test]
fn a_draft_has_the_same_corners_seam_and_tight_stretches_as_the_full_build() {
    for path in [heart(), rounded_rect(80.0, 50.0, 3.0), circle(30.0)] {
        let (full, draft) = (build(&path, None), build(&path, Some(Edit::Draft)));
        let keep = |g: &Generated| -> Vec<String> {
            g.marks
                .on_footprint
                .iter()
                .filter(|m| !matches!(m, Mark::Piece { .. }))
                .map(|m| format!("{m:?}"))
                .collect()
        };
        assert_eq!(keep(&full), keep(&draft));
    }
}

/// Criterion 16: a heart whose notch is a 53 degree V (a narrower one than the golden heart's)
/// crosses itself in the exact cover at 3 mm walls: the round-offset fallback and its notice.
#[test]
fn a_heart_with_a_narrow_v_notch_gets_the_round_cover_and_no_hollow() {
    let mut narrow = heart();
    narrow.segments[1].c2 = curvyo_document_core::Point::new(-10.0, -42.0);
    narrow.segments[1].to = curvyo_document_core::Point::new(0.0, -22.0);
    narrow.segments[2].c1 = curvyo_document_core::Point::new(10.0, -42.0);
    let g = build(&narrow, None);
    assert!(ids(&g).contains(&"cover_rounded"));
    assert!(!ids(&g).contains(&"hollow_left_out"));
    assert!(!crosses(cover(&g, 2)));
}

/// The tabs and the inner footprint of the box of `path` at the default values.
fn tabs_of(path: &ClosedPath) -> (crate::footprint::Footprint, Vec<crate::footprint::Tab>) {
    let mut params = StoredParams::new();
    params.insert(
        "outline".to_owned(),
        ParamValue::Path(path.clone()).to_stored(),
    );
    let schema = super::params();
    let mut values = super::Values::read(&schema, &params);
    let checked = super::check(path);
    let m = super::measures(&mut values, checked.area, false);
    let (_, inner, tabs) = super::build::build(&checked, &m);
    (inner, tabs)
}

/// The 0147-style mating on heart and shield (criterion 16), white box: every tab's centre on the
/// strip is its slot's centre (the middle of the span it stands on, windows included), the tab
/// lies at the middle of its chord, and where the tab stands on a window the plate's engaged edge
/// departs from the wall's outer face by at most the window sag l^2 / (8 r) (the kerf comes on
/// top in the parts).
#[test]
fn the_tabs_of_the_heart_and_the_shield_stand_at_their_slots_within_the_window_sag() {
    use crate::footprint::Shape;
    use curvyo_geometry_core::PathMeasure;
    for (name, path) in [("heart", heart()), ("shield", super::fixtures::shield())] {
        let (inner, tabs) = tabs_of(&path);
        assert!(!tabs.is_empty(), "{name}: tabs");
        let measure = PathMeasure::new(&triples(&path), true);
        for tab in &tabs {
            let (index, span) = inner
                .spans
                .iter()
                .enumerate()
                .find(|(_, s)| tab.s >= s.s0 - 1e-9 && tab.s <= s.s0 + s.length + 1e-9)
                .unwrap();
            let Shape::Straight { from, to } = span.shape else {
                panic!("{name}: a tab stands on a straight or a window");
            };
            let mid = (f64::midpoint(from.x, to.x), f64::midpoint(from.y, to.y));
            if span.window {
                assert!(
                    (tab.s - (span.s0 + span.length / 2.0)).abs() < 1e-6,
                    "{name}: the slot is centred on its window"
                );
                assert!(
                    (tab.centre.x - mid.0).abs() < 1e-6 && (tab.centre.y - mid.1).abs() < 1e-6,
                    "{name}: the tab is at the middle of its chord"
                );
            } else {
                let l = libm::hypot(to.x - from.x, to.y - from.y);
                let u = ((tab.centre.x - from.x) * (to.x - from.x)
                    + (tab.centre.y - from.y) * (to.y - from.y))
                    / l;
                assert!(
                    (tab.s - span.s0 - u).abs() < 1e-6,
                    "{name}: the slot is at the tab's place on its straight"
                );
            }
            if span.window {
                let l = libm::hypot(to.x - from.x, to.y - from.y);
                let radius = [index.checked_sub(1), Some(index + 1)]
                    .into_iter()
                    .flatten()
                    .filter_map(|i| inner.spans.get(i))
                    .filter_map(|s| match s.shape {
                        Shape::Bend { radius, .. } => Some(radius),
                        Shape::Straight { .. } => None,
                    })
                    .fold(f64::MAX, f64::min);
                let s = measure.nearest(curvyo_document_core::Point::new(mid.0, mid.1));
                let on = measure.at(s).point;
                let sag = libm::hypot(on.x - mid.0, on.y - mid.1);
                assert!(
                    sag <= l * l / (8.0 * radius) + 0.02,
                    "{name}: sag {sag} over a chord of {l} at radius {radius}"
                );
            }
        }
    }
}

/// An imported outline may end with a zero-length cubic back onto its start (`C` to the start
/// point, then `Z`): it is no corner. The imported shapes read as the drawn ones.
#[test]
fn a_zero_length_closing_cubic_is_no_corner_and_the_imported_shapes_equal_the_drawn_ones() {
    for (name, path) in [
        ("ellipse", ellipse(50.0, 30.0)),
        ("circle", circle(30.0)),
        ("heart", heart()),
    ] {
        let mut imported = path.clone();
        let s = imported.start;
        imported.segments.push(curvyo_extension_core::Cubic {
            c1: s,
            c2: s,
            to: s,
        });
        let (drawn, got) = (build(&path, None), build(&imported, None));
        let corners = |g: &Generated| {
            g.marks
                .on_footprint
                .iter()
                .filter(|m| matches!(m, Mark::Corner { .. }))
                .count()
        };
        assert_eq!(corners(&drawn), corners(&got), "{name}: corners");
        assert_eq!(seams(&drawn), seams(&got), "{name}: seam");
        assert_eq!(got.parts.len(), drawn.parts.len(), "{name}: parts");
    }
}

/// Lead decision on ADR 0083 §13.10: the hollow sits on the longest convex stretch of a run; at
/// Corner angle 90 the heart's notch is a kink inside its single run and no longer hides the lobes.
#[test]
fn the_heart_at_corner_angle_90_still_gets_the_hollow_on_a_lobe() {
    let mut params = StoredParams::new();
    params.insert("outline".to_owned(), ParamValue::Path(heart()).to_stored());
    params.insert(
        "corner_angle".to_owned(),
        ParamValue::Angle(curvyo_extension_core::Degrees(90.0)).to_stored(),
    );
    let g = super::generate(&GenerateRequest {
        kind: crate::PATH_BOX,
        params: &params,
        seed: Seed(1),
        edit: None,
        glyphs: None,
    });
    assert!(!ids(&g).contains(&"hollow_left_out"));
    let (plain, cut) = (curve_area(cover(&g, 1)).abs(), curve_area(lid(&g, 1)).abs());
    assert!((80.0..200.0).contains(&(plain - cut)), "{plain} {cut}");
}
