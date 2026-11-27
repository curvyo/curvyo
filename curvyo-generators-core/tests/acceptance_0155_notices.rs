//! Acceptance of `specs/0155-generator-joint-options`, the two notices left open by PR #506:
//! criterion 16 (a tab on a bend that does not fit one arc keeps its flat window, with a notice)
//! and criterion 19 (an unknown Joint pattern id reads Even, with a notice).

#![allow(clippy::unwrap_used)]

use curvyo_document_core::{Length, Point};
use curvyo_extension_core::{
    ClosedPath, Cubic, Extension, GenerateRequest, Generated, ParamValue, Seed, StoredParams,
};
use curvyo_generators_core::{BOX, GENERATORS, PATH_BOX, ROUNDED_BOX};

fn mm(value: f64) -> ParamValue {
    ParamValue::Length(Length::from_mm(value))
}

fn choice(id: &str) -> ParamValue {
    ParamValue::Choice(id.to_owned())
}

fn stored(entries: &[(&str, ParamValue)]) -> StoredParams {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.to_stored()))
        .collect()
}

fn generate(kind: &str, entries: &[(&str, ParamValue)]) -> Generated {
    GENERATORS
        .generate(&GenerateRequest {
            kind,
            params: &stored(entries),
            seed: Seed(1),
            edit: None,
            glyphs: None,
        })
        .unwrap()
}

fn told(g: &Generated, id: &str) -> usize {
    g.warnings.iter().filter(|w| w.id == id).count()
}

fn rounded(w: f64, r: f64, bend: &str) -> Generated {
    generate(
        ROUNDED_BOX,
        &[
            ("inner_width", mm(w)),
            ("inner_depth", mm(w)),
            ("corner_radius", mm(r)),
            ("thickness", mm(3.0)),
            ("bend_tabs", choice(bend)),
        ],
    )
}

fn heart() -> ClosedPath {
    let p = |x: f64, y: f64| Point::new(x, y);
    let c = |c1: (f64, f64), c2: (f64, f64), to: (f64, f64)| Cubic {
        c1: p(c1.0, c1.1),
        c2: p(c2.0, c2.1),
        to: p(to.0, to.1),
    };
    ClosedPath {
        start: p(0.0, 40.0),
        segments: vec![
            c((-10.0, 25.0), (-40.0, 10.0), (-40.0, -10.0)),
            c((-40.0, -35.0), (-12.0, -32.0), (0.0, -22.0)),
            c((12.0, -32.0), (40.0, -35.0), (40.0, -10.0)),
            c((40.0, 10.0), (10.0, 25.0), (0.0, 40.0)),
        ],
    }
}

fn heart_box(bend: &str) -> Generated {
    generate(
        PATH_BOX,
        &[
            ("outline", ParamValue::Path(heart())),
            ("thickness", mm(3.0)),
            ("bend_tabs", choice(bend)),
        ],
    )
}

#[test]
fn criterion_16_a_tab_that_keeps_its_window_on_a_bend_tells_tab_window() {
    // The heart's bends are chains of short arcs: some tabs do not sit wholly on one of them.
    let curve = heart_box("curve");
    assert_eq!(
        told(&curve, "tab_window"),
        1,
        "one notice, the count is its value"
    );
    let w = curve
        .warnings
        .iter()
        .find(|w| w.id == "tab_window")
        .unwrap();
    assert!(w.value.as_ref().and_then(ParamValue::count) > Some(0));
    assert_eq!(
        told(&heart_box("window"), "tab_window"),
        0,
        "window asked: no notice"
    );
}

#[test]
fn criterion_16_a_box_whose_tabs_all_fit_says_nothing() {
    assert_eq!(told(&rounded(60.0, 30.0, "curve"), "tab_window"), 0);
    assert_eq!(told(&generate(ROUNDED_BOX, &[]), "tab_window"), 0);
}

#[test]
fn criterion_19_an_unknown_joint_pattern_reads_even_with_a_notice() {
    let curved = [("top_shape", choice("curved"))];
    for (kind, extra) in [
        (BOX, &[][..]),
        (ROUNDED_BOX, &[][..]),
        (ROUNDED_BOX, &curved[..]),
        (PATH_BOX, &[][..]),
    ] {
        let mut odd = extra.to_vec();
        odd.push(("joint_pattern", choice("sideways")));
        let mut even = extra.to_vec();
        even.push(("joint_pattern", choice("even")));
        let (odd, even) = (generate(kind, &odd), generate(kind, &even));
        assert_eq!(told(&odd, "unknown_joint_pattern"), 1, "{kind}");
        assert_eq!(told(&even, "unknown_joint_pattern"), 0, "{kind}");
        assert_eq!(odd.parts, even.parts, "{kind}: reads Even");
    }
}

#[test]
fn criterion_19_a_known_or_missing_pattern_says_nothing() {
    for pattern in [None, Some("even"), Some("centred")] {
        let entries: Vec<(&str, ParamValue)> = pattern
            .map(|p| ("joint_pattern", choice(p)))
            .into_iter()
            .collect();
        assert_eq!(
            told(&generate(ROUNDED_BOX, &entries), "unknown_joint_pattern"),
            0
        );
    }
}
