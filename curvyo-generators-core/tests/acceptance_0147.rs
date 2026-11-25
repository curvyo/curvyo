//! Acceptance tests of `specs/0147-rounded-box/specification.md`, slice 1 (criteria 1 to 10 and 12
//! to 13; criterion 11 is slice 2, criterion 14 is later, criterion 15 is slice 3), through the
//! public generator contract (ADR 0080). Black-box: manifest, stored parameters and outlines only.
//!
//! Coordinates: the strip is measured in its own frame (`u` along its long axis from one butt end,
//! `v` across the wall); plates are found by area and path count, not by part order.
//! Not covered here: criterion 1's panel placement and undo (ui-core), criterion 5's tab-and-slot
//! centre mating (no tab geometry seam was found in the public outlines, see the report), criterion
//! 7's drag and undo, criterion 8's `format_version` (document-core).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names
)]

mod budget_factor;

use curvyo_extension_core::{
    ClosedPath, Contribution, Edit, Extension, GenerateRequest, Generated, Param, ParamKind,
    ParamValue, Part, ScaleFactor, Seed, StoredParams,
};
use curvyo_generators_core::{BOX, GEAR, GENERATORS, ROUNDED_BOX, manifest};
use serde_json::{Map, Value, json};
use std::f64::consts::PI;

type Pt = (f64, f64);

// ---------- parameter helpers (stored forms as the host writes them) ----------

fn mm(value: f64) -> Value {
    json!({ "mm": value })
}

fn sw(value: bool) -> Value {
    json!({ "on": value })
}

fn choice(id: &str) -> Value {
    json!({ "choice": id })
}

/// The stored values of a box `w` x `d` x `h` (inside), corner radius `r`, wall `t`, plus `extra`.
fn stored_at(w: f64, d: f64, h: f64, r: f64, t: f64, extra: &[(&str, Value)]) -> StoredParams {
    let mut map = Map::new();
    map.insert("inner_width".to_owned(), mm(w));
    map.insert("inner_depth".to_owned(), mm(d));
    map.insert("inner_height".to_owned(), mm(h));
    map.insert("corner_radius".to_owned(), mm(r));
    map.insert("thickness".to_owned(), mm(t));
    for (key, value) in extra {
        map.insert((*key).to_owned(), value.clone());
    }
    map
}

/// The spec's example box: 120 x 80 x 50 inside, corner radius 15, 3 mm wall.
fn stored(extra: &[(&str, Value)]) -> StoredParams {
    stored_at(120.0, 80.0, 50.0, 15.0, 3.0, extra)
}

fn generate_with(params: &StoredParams, edit: Option<Edit<'_>>) -> Generated {
    GENERATORS
        .generate(&GenerateRequest {
            kind: ROUNDED_BOX,
            params,
            seed: Seed(7),
            edit,
        })
        .expect("the rounded box answers its own kind")
}

fn boxed(extra: &[(&str, Value)]) -> Generated {
    generate_with(&stored(extra), None)
}

fn boxed_at(w: f64, d: f64, h: f64, r: f64, t: f64, extra: &[(&str, Value)]) -> Generated {
    generate_with(&stored_at(w, d, h, r, t, extra), None)
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

/// The automatic value the generator used for a stored-0 parameter, if it reported one.
fn automatic_length(generated: &Generated, key: &str) -> Option<f64> {
    generated
        .automatic
        .iter()
        .find(|(k, _)| *k == key)
        .and_then(|(_, value)| value.clone().length())
        .map(curvyo_document_core::Length::as_mm)
}

fn is_automatic(generated: &Generated, key: &str) -> bool {
    generated.automatic.iter().any(|(k, _)| *k == key)
}

fn clamped(generated: &Generated, key: &str) -> bool {
    generated.clamps.iter().any(|clamp| clamp.key == key)
}

fn assert_close(actual: f64, expected: f64, tolerance: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: expected {expected} within {tolerance}, got {actual}"
    );
}

// ---------- outline sampling and geometry ----------

fn sample(path: &ClosedPath) -> Vec<Pt> {
    const STEPS: usize = 16;
    let mut points = Vec::new();
    let mut from = path.start;
    for segment in &path.segments {
        for step in 0..STEPS {
            let u =
                f64::from(u32::try_from(step).unwrap()) / f64::from(u32::try_from(STEPS).unwrap());
            let v = 1.0 - u;
            let x = v * v * v * from.x
                + 3.0 * v * v * u * segment.c1.x
                + 3.0 * v * u * u * segment.c2.x
                + u * u * u * segment.to.x;
            let y = v * v * v * from.y
                + 3.0 * v * v * u * segment.c1.y
                + 3.0 * v * u * u * segment.c2.y
                + u * u * u * segment.to.y;
            points.push((x, y));
        }
        from = segment.to;
    }
    points
}

fn all_points(points: &[Pt]) -> (f64, f64, f64, f64) {
    let mut x0 = f64::INFINITY;
    let mut y0 = f64::INFINITY;
    let mut x1 = f64::NEG_INFINITY;
    let mut y1 = f64::NEG_INFINITY;
    for &(x, y) in points {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    (x0, y0, x1, y1)
}

fn part_points(part: &Part) -> Vec<Pt> {
    part.paths.iter().flat_map(sample).collect()
}

fn part_bbox(part: &Part) -> (f64, f64, f64, f64) {
    all_points(&part_points(part))
}

/// The area of a closed outline (shoelace, absolute value).
fn area(points: &[Pt]) -> f64 {
    let n = points.len();
    let mut twice = 0.0;
    for i in 0..n {
        let (x0, y0) = points[i];
        let (x1, y1) = points[(i + 1) % n];
        twice += x0 * y1 - x1 * y0;
    }
    twice.abs() / 2.0
}

fn outer_area(generated: &Generated, index: usize) -> f64 {
    area(&sample(&generated.parts[index].paths[0]))
}

/// The index of the strip: the part whose outer outline has the largest area.
fn strip_index(generated: &Generated) -> usize {
    (0..generated.parts.len())
        .max_by(|&a, &b| outer_area(generated, a).total_cmp(&outer_area(generated, b)))
        .expect("a box has parts")
}

/// The strip in its own frame: `u` along the long axis from its minimum, `v` across the wall.
struct Strip {
    paths: Vec<Vec<Pt>>,
    length: f64,
    height: f64,
}
fn strip_of(generated: &Generated) -> Strip {
    let index = strip_index(generated);
    let paths: Vec<Vec<Pt>> = generated.parts[index].paths.iter().map(sample).collect();
    let flat: Vec<Pt> = paths.iter().flatten().copied().collect();
    let (x0, y0, x1, y1) = all_points(&flat);
    // The length runs along the axis whose extent is not the strip height (inside height plus the
    // cover and the lid plate as used); a small circle has its length shorter than its height.
    let (_, _, _, _, h) = effective(generated);
    let used =
        |key: &str| automatic_length(generated, key).unwrap_or_else(|| length_of(generated, key));
    let height = h + used("cover_thickness") + used("lid_thickness");
    let along_x = (x1 - x0 - height).abs() > (y1 - y0 - height).abs();
    let mapped = paths
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

/// The value a plate thickness is built with: the automatic one, or the typed one.
fn used_length(generated: &Generated, key: &str) -> f64 {
    automatic_length(generated, key).unwrap_or_else(|| length_of(generated, key))
}

/// The normalised box values (inner w, d, corner r, wall t, inner h) the generator used.
fn effective(generated: &Generated) -> (f64, f64, f64, f64, f64) {
    (
        length_of(generated, "inner_width"),
        length_of(generated, "inner_depth"),
        length_of(generated, "corner_radius"),
        length_of(generated, "thickness"),
        length_of(generated, "inner_height"),
    )
}

/// The strip's length on the neutral line (spec criterion 3): `2(w - 2r) + 2(d - 2r) + pi (2r + t)`.
fn neutral_length(w: f64, d: f64, r: f64, t: f64) -> f64 {
    2.0 * (w - 2.0 * r) + 2.0 * (d - 2.0 * r) + PI * (2.0 * r + t)
}

/// The strip's expected length (spec criterion 3, ADR 0082 §4 and §5, lead decision 83f3835c): the
/// inner face follows the plate outline, and the bends add `pi t`. Each of the `windows` flat tab
/// windows on a bent span replaces its inner arc by its chord, so
/// `expected = 2(w - 2r) + 2(d - 2r) + pi (2r + t) - N (r theta - l)`, with `l = 2 t_p + 2 t_w` and
/// `theta = 2 asin(l / (2 r))`.
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

/// The centres `u` of the tab notches at one edge of the strip: flat runs at the notch bottom
/// `v = level`, each of the length of a tab (`2 t_p`, sampled), counted once per run.
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

/// The tab windows of the box (criterion 3): the tab notches of the inner base row on the bent
/// spans (their centres inside the corner arcs). The ring row (top edge, depth `t_w`) carries the
/// same windows, so both rows must agree.
fn tab_windows(strip: &Strip, w: f64, d: f64, r: f64, t_w: f64, t_p: f64) -> usize {
    let arcs = corner_windows(w, d, r, t_w);
    let on_bends = |centres: &[f64]| {
        centres
            .iter()
            .filter(|&&u| arcs.iter().any(|&(a, b)| u >= a - 0.5 && u <= b + 0.5))
            .count()
    };
    let bottom = on_bends(&notch_centres(strip, t_w, t_p));
    let top = on_bends(&notch_centres(strip, strip.height - t_w, t_p));
    assert_eq!(bottom, top, "the base and ring rows carry the same windows");
    bottom
}

/// The four corner arcs along the strip's `u` (spec criterion 3): the strip starts in the middle of
/// the longer straight, so the windows are symmetric under `u -> length - u`.
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
    assert_close(
        cursor,
        neutral_length(w, d, r, t),
        1e-9,
        "the windows cover the strip",
    );
    windows
}

fn in_any(u: f64, windows: &[(f64, f64)], tolerance: f64) -> bool {
    windows
        .iter()
        .any(|&(a, b)| u >= a - tolerance && u <= b + tolerance)
}

/// Whether the horizontal line `v = height` crosses an edge of any outline of the strip with `u`
/// inside `[a, b]` (a slot or an edge notch cuts the line there).
fn line_is_cut(strip: &Strip, v: f64, a: f64, b: f64) -> bool {
    for path in &strip.paths {
        let n = path.len();
        for i in 0..n {
            let (u0, v0) = path[i];
            let (u1, v1) = path[(i + 1) % n];
            if (v0 - v) * (v1 - v) < 0.0 {
                let s = (v - v0) / (v1 - v0);
                let u = u0 + s * (u1 - u0);
                if u >= a && u <= b {
                    return true;
                }
            }
        }
    }
    false
}

/// Every line of constant height across the strip on each bent span meets a slot or a notch.
/// Returns the first failing (height, arc start) pair and the number of failures.
fn uncut_bands(strip: &Strip, windows: &[(f64, f64)]) -> (usize, Option<(f64, f64)>) {
    let mut failures = 0;
    let mut first = None;
    let mut v = 0.05;
    while v < strip.height - 0.05 {
        for &(a, b) in windows {
            if !line_is_cut(strip, v, a, b) {
                failures += 1;
                first.get_or_insert((v, a));
            }
        }
        v += 0.1;
    }
    (failures, first)
}

// ---------- criterion 1: the segment and the default outline ----------

#[test]
fn criterion_1_rounded_box_is_a_third_generator_segment_after_box_and_gear() {
    let kinds: Vec<(&str, &str)> = manifest()
        .contributions
        .iter()
        .filter_map(|contribution| match contribution {
            // The path box is built from a selected outline, not placed (`0149`).
            Contribution::Generator(kind) if kind.source.is_none() => Some((kind.id, kind.label)),
            Contribution::Generator(_) | Contribution::Workbench(_) => None,
        })
        .collect();
    let ids: Vec<&str> = kinds.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, [BOX, GEAR, ROUNDED_BOX], "Box, Gear, then Rounded box");
    assert_eq!(
        kinds[2].1, "Rounded box",
        "the segment is labelled Rounded box"
    );
    assert_eq!(ROUNDED_BOX, "rounded-box", "the kind id is rounded-box");
    let defaults = generate_with(&StoredParams::new(), None);
    assert!(
        !defaults.parts.is_empty(),
        "a rounded box has outlines at its defaults"
    );
}

// ---------- criterion 2: parameters, defaults, clamps ----------

fn rounded_kind() -> curvyo_extension_core::GeneratorKind {
    manifest()
        .contributions
        .into_iter()
        .find_map(|contribution| match contribution {
            Contribution::Generator(kind) if kind.id == ROUNDED_BOX => Some(kind),
            _ => None,
        })
        .expect("the manifest declares the rounded box")
}

#[test]
fn criterion_2_declared_parameters_have_the_spec_labels_and_defaults() {
    let kind = rounded_kind();
    let params: Vec<&Param> = kind.groups.iter().flat_map(|g| g.params.iter()).collect();
    let find = |key: &str| -> &Param {
        params
            .iter()
            .copied()
            .find(|p| p.key == key)
            .unwrap_or_else(|| panic!("the parameter {key} is declared"))
    };
    let labels = [
        ("slit_pattern", "Slit pattern"),
        ("wall_slits", "Wall slits"),
        ("slit_width", "Slit width"),
        ("slit_spacing", "Slit spacing"),
        ("slit_rows", "Slit rows"),
        ("thickness", "Wall thickness"),
        ("kerf", "Kerf compensation"),
        ("plate_thickness", "Inner base thickness"),
        ("base_inset", "Base inset"),
        ("base_cover", "Base cover"),
        ("cover_thickness", "Cover thickness"),
        ("lid", "Lid"),
        ("lid_thickness", "Lid thickness"),
        ("lid_seat", "Lid seat"),
        ("ring_thickness", "Ring thickness"),
        ("finger_hollow", "Finger hollow"),
        ("hollow_width", "Hollow width"),
        ("hollow_depth", "Hollow depth"),
        ("gap", "Gap"),
    ];
    for (key, label) in labels {
        let found = find(key).label;
        assert_eq!(found, label, "the label of {key}");
    }
    for key in [
        "inner_width",
        "inner_depth",
        "inner_height",
        "corner_radius",
    ] {
        find(key);
    }
    for key in [
        "plate_thickness",
        "cover_thickness",
        "lid_thickness",
        "ring_thickness",
    ] {
        assert!(
            matches!(find(key).default, ParamValue::Length(l) if l.as_mm() == 0.0),
            "{key} defaults to automatic (0, following the wall)"
        );
    }
    for key in ["base_cover", "lid", "lid_seat", "finger_hollow"] {
        assert!(
            matches!(find(key).default, ParamValue::Switch(true)),
            "{key} is on by default"
        );
    }
    assert!(
        matches!(find("base_inset").default, ParamValue::Length(l) if l.as_mm() == 0.0),
        "Base inset defaults to 0"
    );
    assert!(
        matches!(find("kerf").default, ParamValue::Length(l) if l.as_mm() == 0.0),
        "Kerf defaults to 0"
    );
    let options = |key: &str| -> Vec<&str> {
        match &find(key).kind {
            ParamKind::Choice { options } => options.iter().map(|o| o.id).collect(),
            other => panic!("{key} is a choice, got {other:?}"),
        }
    };
    assert_eq!(options("slit_pattern"), ["straight", "offset", "wave"]);
    assert_eq!(options("wall_slits"), ["corners", "all"]);
    assert!(
        matches!(&find("wall_slits").default, ParamValue::Choice(c) if c == "corners"),
        "Wall slits defaults to Corners only"
    );
    assert!(
        !params
            .iter()
            .any(|p| matches!(p.key, "finger_width" | "clearance")),
        "no Finger width and no Clearance (derived); Top shape arrived with slice 2"
    );
}

#[test]
fn criterion_2_corner_radius_below_two_wall_thicknesses_is_clamped_with_a_notice() {
    let generated = boxed(&[("corner_radius", mm(1.0))]);
    assert!(
        clamped(&generated, "corner_radius"),
        "a notice names the corner radius"
    );
    assert!(
        length_of(&generated, "corner_radius") >= 2.0 * WALL_T - 1e-9,
        "the radius is raised to at least 2 times the wall"
    );
    assert!(
        !generated.parts.is_empty(),
        "the box is still produced, no refusal"
    );
}

const WALL_T: f64 = 3.0;

#[test]
fn criterion_2_full_round_is_no_notice_and_no_refusal() {
    // Exact circles and stadia at the maximum radius (half the smaller side): no special case.
    for (w, d, r, t) in [
        (120.0, 80.0, 40.0, 3.0), // stadium at the maximum
        (60.0, 60.0, 30.0, 3.0),  // exact circle
        (12.0, 12.0, 6.0, 3.0),   // small circle at the minimum (= 2 times the wall)
        (20.0, 20.0, 10.0, 3.0),  // small circle
        (40.0, 200.0, 20.0, 3.0), // deep stadium
    ] {
        let generated = boxed_at(w, d, 50.0, r, t, &[]);
        assert!(
            !clamped(&generated, "corner_radius"),
            "no notice for radius {r} in {w} x {d}"
        );
        assert!(!generated.parts.is_empty(), "outlines for {w} x {d} r {r}");
    }
}

#[test]
fn criterion_2_kerf_zero_and_point_two_are_valid_without_notice() {
    for kerf in [0.0, 0.2] {
        let generated = boxed(&[("kerf", mm(kerf))]);
        assert!(
            !clamped(&generated, "kerf"),
            "kerf {kerf} is inside its range"
        );
        assert_close(
            length_of(&generated, "kerf"),
            kerf,
            1e-9,
            "the kerf is stored as typed",
        );
    }
}

#[test]
fn criterion_2_part_thickness_follows_the_wall_until_typed_and_then_overrides() {
    let following = boxed(&[]);
    assert_eq!(
        automatic_length(&following, "plate_thickness"),
        Some(3.0),
        "Inner base thickness shows the wall thickness as its used value"
    );
    assert!(is_automatic(&following, "cover_thickness"));
    let typed = boxed(&[("plate_thickness", mm(5.0))]);
    assert!(
        !is_automatic(&typed, "plate_thickness"),
        "a typed thickness is not automatic any more"
    );
    assert_close(
        length_of(&typed, "plate_thickness"),
        5.0,
        1e-9,
        "the typed value is kept",
    );
    assert_close(
        length_of(&typed, "thickness"),
        3.0,
        1e-9,
        "the wall thickness is unchanged",
    );
}

#[test]
fn criterion_2_flat_layout_keeps_the_plates_apart() {
    let generated = boxed(&[("gap", mm(5.0))]);
    let boxes: Vec<(f64, f64, f64, f64)> = generated.parts.iter().map(part_bbox).collect();
    assert!(
        boxes.len() >= 5,
        "strip, inner base, cover, ring and lid: {}",
        boxes.len()
    );
    for i in 0..boxes.len() {
        for j in (i + 1)..boxes.len() {
            let (a, b) = (boxes[i], boxes[j]);
            let overlap_x = a.2.min(b.2) - a.0.max(b.0);
            let overlap_y = a.3.min(b.3) - a.1.max(b.1);
            assert!(
                overlap_x <= 0.01 || overlap_y <= 0.01,
                "plates {i} and {j} overlap in the flat layout"
            );
        }
    }
}

// ---------- criterion 3: the strip ----------

#[test]
fn criterion_3_strip_length_is_the_adr_length_within_a_tenth_of_a_mm() {
    let mut misses: Vec<String> = Vec::new();
    // (inner w, inner d, corner r, wall t): the example box, an exact circle, a deep and a wide
    // stadium, a radius at twice the wall, and a 5 mm wall. Small circles are in their own test.
    for (w, d, r, t) in [
        (120.0, 80.0, 15.0, 3.0),
        (60.0, 60.0, 30.0, 3.0),
        (40.0, 200.0, 20.0, 3.0),
        (120.0, 60.0, 30.0, 3.0),
        (120.0, 80.0, 6.0, 3.0),
        (100.0, 60.0, 10.0, 5.0),
    ] {
        let generated = boxed_at(w, d, 30.0, r, t, &[]);
        let (w, d, r, t, _) = effective(&generated);
        let strip = strip_of(&generated);
        let windows = tab_windows(&strip, w, d, r, t, t);
        let expected = expected_strip_length(w, d, r, t, t, windows);
        if (strip.length - expected).abs() > 0.1 {
            misses.push(format!(
                "{w} x {d} r {r} t {t}: strip {:.3} vs ADR {expected:.3}",
                strip.length
            ));
        }
        // The strip is the inside height plus the cover and the lid plate (inside of the wall).
        let plates =
            used_length(&generated, "cover_thickness") + used_length(&generated, "lid_thickness");
        assert_close(
            strip.height,
            30.0 + plates,
            0.1,
            "strip height = inside height + cover + lid",
        );
    }
    assert!(
        misses.is_empty(),
        "strip length misses the neutral perimeter: {misses:#?}"
    );
}

#[test]
fn criterion_3_small_circles_strip_length_follows_the_normalised_wall() {
    // 12 and 20 mm inside with a typed 3 mm wall: the wall is first clamped to a sixth of the
    // smaller inside side (criterion 2), so the 12 mm circle runs on 2 mm and the 20 mm on 3 mm.
    let mut misses: Vec<String> = Vec::new();
    for (w, r) in [(12.0, 6.0), (20.0, 10.0)] {
        let generated = boxed_at(w, w, 30.0, r, 3.0, &[]);
        let (w, d, r, t, _) = effective(&generated);
        assert_close(
            t,
            3.0f64.min(w / 6.0),
            1e-9,
            "the wall after the sixth clamp",
        );
        let strip = strip_of(&generated);
        let windows = tab_windows(&strip, w, d, r, t, t);
        assert!(windows >= 1, "a small circle has tab windows");
        let expected = expected_strip_length(w, d, r, t, t, windows);
        if (strip.length - expected).abs() > 0.1 {
            misses.push(format!(
                "{w} mm circle: strip {:.3} vs ADR {expected:.3}",
                strip.length
            ));
        }
    }
    assert!(
        misses.is_empty(),
        "small circles miss the ADR strip length: {misses:#?}"
    );
}

#[test]
fn criterion_2_wall_is_clamped_to_a_sixth_of_the_smaller_inside_side_with_a_notice() {
    // Revision 6 (Proposal): the wall is at most a sixth of the smaller inside side, and the clamp
    // is reported. A 12 mm circle is clamped to 2 mm; a 20 mm circle keeps 3 mm (20 / 6 >= 3).
    for (w, r) in [(12.0, 6.0), (20.0, 10.0)] {
        let generated = boxed_at(w, w, 50.0, r, 3.0, &[]);
        let limit = w / 6.0;
        assert_close(
            length_of(&generated, "thickness"),
            3.0f64.min(limit),
            1e-9,
            &format!("the wall of a {w} mm circle"),
        );
        assert_eq!(
            clamped(&generated, "thickness"),
            3.0 > limit,
            "a notice on the wall of a {w} mm circle exactly when the wall is clamped"
        );
    }
}

#[test]
fn criterion_3_slits_sit_in_the_corner_arcs_by_default() {
    for (w, d, r, t) in [(120.0, 80.0, 15.0, 3.0), (40.0, 200.0, 20.0, 3.0)] {
        for pattern in ["straight", "offset", "wave"] {
            let generated = boxed_at(w, d, 50.0, r, t, &[("slit_pattern", choice(pattern))]);
            let strip = strip_of(&generated);
            let windows = corner_windows(w, d, r, t);
            let (_, _, _, t, _) = effective(&generated);
            let mut outside = 0;
            let mut slit_points = 0;
            for path in &strip.paths {
                for &(u, v) in path {
                    // Tab-slot notches (depth = wall) at the edges are joints, not slits.
                    let slit_zone = v > t + 0.5 && v < strip.height - t - 0.5;
                    if !slit_zone || u <= 1.0 || u >= strip.length - 1.0 {
                        continue;
                    }
                    slit_points += 1;
                    if !in_any(u, &windows, 1.0) {
                        outside += 1;
                    }
                }
            }
            assert!(slit_points > 0, "{pattern}: slits exist");
            assert_eq!(
                outside, 0,
                "{pattern} in {w} x {d}: no slit point on a straight wall part"
            );
        }
    }
}

#[test]
fn criterion_3_all_round_puts_slits_on_the_straight_walls_too() {
    let generated = boxed(&[("wall_slits", choice("all"))]);
    let (w, d, r, t, _) = effective(&generated);
    let strip = strip_of(&generated);
    let windows = corner_windows(w, d, r, t);
    let (_, _, _, t, _) = effective(&generated);
    let straight_points = strip
        .paths
        .iter()
        .flatten()
        .filter(|&&(_, v)| v > t + 0.5 && v < strip.height - t - 0.5)
        .filter(|&&(u, _)| u > 1.0 && u < strip.length - 1.0 && !in_any(u, &windows, 1.0))
        .count();
    assert!(
        straight_points > 0,
        "the all-round wall has slit points on the straight parts"
    );
}

// ---------- criterion 4: slits, the uncut-band rule and the automatic values ----------

#[test]
fn criterion_4_every_slot_lies_inside_the_strip_for_each_pattern_and_wall() {
    for pattern in ["straight", "offset", "wave"] {
        for walls in ["corners", "all"] {
            let generated = boxed(&[
                ("slit_pattern", choice(pattern)),
                ("wall_slits", choice(walls)),
            ]);
            let strip = strip_of(&generated);
            for path in strip.paths.iter().skip(1) {
                for &(u, v) in path {
                    assert!(
                        (-0.01..=strip.length + 0.01).contains(&u)
                            && (-0.01..=strip.height + 0.01).contains(&v),
                        "{pattern}/{walls}: a slot point ({u:.3}, {v:.3}) lies outside the strip"
                    );
                }
            }
        }
    }
}

#[test]
fn criterion_4_no_uncut_band_across_a_bent_span_for_the_default_patterns() {
    for pattern in ["straight", "offset", "wave"] {
        let generated = boxed(&[("slit_pattern", choice(pattern))]);
        let strip = strip_of(&generated);
        let windows = corner_windows(120.0, 80.0, 15.0, 3.0);
        let (failures, first) = uncut_bands(&strip, &windows);
        assert_eq!(
            failures, 0,
            "{pattern}: an uncut band across a bend, first at (height, arc start) {first:?}"
        );
    }
}

#[test]
fn criterion_4_no_uncut_band_on_a_small_circle_and_a_deep_stadium() {
    for (w, d, r, t) in [
        (12.0, 12.0, 6.0, 3.0),
        (20.0, 20.0, 10.0, 3.0),
        (40.0, 200.0, 20.0, 3.0),
    ] {
        for pattern in ["straight", "offset", "wave"] {
            let generated = boxed_at(w, d, 50.0, r, t, &[("slit_pattern", choice(pattern))]);
            let (w, d, r, t, _) = effective(&generated);
            let strip = strip_of(&generated);
            let windows = corner_windows(w, d, r, t);
            let (failures, first) = uncut_bands(&strip, &windows);
            assert_eq!(
                failures, 0,
                "{pattern} in {w} x {d}: uncut band, first at {first:?}"
            );
        }
    }
}

#[test]
fn criterion_4_no_uncut_band_with_wave_at_extreme_spacing() {
    for spacing in [0.5, 50.0] {
        let generated = boxed(&[
            ("slit_pattern", choice("wave")),
            ("slit_spacing", mm(spacing)),
        ]);
        let strip = strip_of(&generated);
        let windows = corner_windows(120.0, 80.0, 15.0, 3.0);
        let (failures, first) = uncut_bands(&strip, &windows);
        assert_eq!(
            failures, 0,
            "wave at spacing {spacing}: uncut band at {first:?}"
        );
        assert!(
            (strip.length - neutral_length(120.0, 80.0, 15.0, 3.0)).abs() <= 0.1,
            "the strip length does not depend on the slits"
        );
    }
}

#[test]
fn criterion_4_automatic_slit_width_is_kerf_plus_0_3_and_closed_slots_are_drawn_0_3() {
    for kerf in [0.0, 0.2] {
        let generated = boxed(&[("kerf", mm(kerf))]);
        assert_close(
            automatic_length(&generated, "slit_width").expect("slit width is automatic"),
            kerf + 0.3,
            1e-6,
            "the automatic slit width is kerf plus 0.3 mm",
        );
        // The drawn closed slot is its width minus the kerf: 0.3 mm at both kerf values. Slit walls
        // (holes or edge notches) sit at u-positions; the smallest gap between two of them is the
        // drawn width.
        let strip = strip_of(&boxed(&[
            ("kerf", mm(kerf)),
            ("slit_pattern", choice("straight")),
        ]));
        let (_, _, _, t, _) = effective(&boxed(&[("kerf", mm(kerf))]));
        let mut us: Vec<f64> = strip
            .paths
            .iter()
            .flatten()
            .filter(|&&(_, v)| v > t + 0.5 && v < strip.height - t - 0.5)
            .map(|&(u, _)| (u * 1000.0).round() / 1000.0)
            .collect();
        us.sort_by(f64::total_cmp);
        us.dedup();
        let narrowest = us
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .filter(|gap| *gap > 0.01)
            .fold(f64::INFINITY, f64::min);
        assert!(narrowest.is_finite(), "slit walls exist");
        assert_close(
            narrowest,
            0.3,
            0.02,
            &format!("drawn slot width at kerf {kerf}"),
        );
    }
}

#[test]
fn criterion_4_automatic_spacing_is_0_22_times_the_corner_radius() {
    for r in [15.0, 20.0] {
        let generated = boxed_at(120.0, 80.0, 50.0, r, 3.0, &[]);
        assert_close(
            automatic_length(&generated, "slit_spacing").expect("slit spacing is automatic"),
            0.22 * r,
            1e-6,
            &format!("automatic spacing at radius {r}"),
        );
    }
}

// ---------- criterion 6: timing, thread CPU time only ----------

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
fn criterion_6_regenerating_a_box_of_about_400_slots_costs_under_100_ms_of_thread_cpu() {
    let params = stored_at(
        200.0,
        100.0,
        100.0,
        20.0,
        3.0,
        &[("wall_slits", choice("all")), ("slit_spacing", mm(1.5))],
    );
    let _warm = generate_with(&params, None);
    let mut best = u64::MAX;
    let mut segments = 0;
    for _ in 0..3 {
        let before = thread_ticks().expect("the thread CPU clock is readable");
        let generated = generate_with(&params, None);
        let after = thread_ticks().expect("the thread CPU clock is readable");
        // A notch or a closed slot adds 3 to 4 segments to the strip outline.
        segments = generated.parts[strip_index(&generated)]
            .paths
            .iter()
            .map(|p| p.segments.len())
            .sum::<usize>();
        best = best.min(after - before);
    }
    eprintln!("criterion 6: {segments} strip segments, best run {best} clock ticks (10 ms each)");
    // A notch is 3 segments: about 385 notches in this box.
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

// ---------- criterion 7: proportional resize and regeneration ----------

#[test]
fn criterion_7_proportional_resize_changes_only_the_three_dimensions() {
    let stored_values = stored(&[("kerf", mm(0.2)), ("gap", mm(5.0))]);
    let scaled = generate_with(&stored_values, Some(Edit::Scale(ScaleFactor(2.0))));
    assert_close(
        length_of(&scaled, "inner_width"),
        240.0,
        1e-9,
        "Inner width doubles",
    );
    assert_close(
        length_of(&scaled, "inner_depth"),
        160.0,
        1e-9,
        "Depth doubles",
    );
    assert_close(
        length_of(&scaled, "inner_height"),
        100.0,
        1e-9,
        "Height doubles",
    );
    assert_close(
        length_of(&scaled, "corner_radius"),
        15.0,
        1e-9,
        "Corner radius stays",
    );
    assert_close(
        length_of(&scaled, "thickness"),
        3.0,
        1e-9,
        "the wall thickness stays",
    );
    assert_close(length_of(&scaled, "kerf"), 0.2, 1e-9, "the kerf stays");
    assert_close(length_of(&scaled, "gap"), 5.0, 1e-9, "the gap stays");
    assert!(
        is_automatic(&scaled, "slit_spacing"),
        "the automatic spacing stays automatic"
    );
    // The perimeter follows the new size; kerf 0 here, so the formula holds at 0.1 mm.
    let zero_kerf = generate_with(&stored(&[]), Some(Edit::Scale(ScaleFactor(2.0))));
    let strip = strip_of(&zero_kerf);
    assert_close(
        strip.length,
        neutral_length(240.0, 160.0, 15.0, 3.0),
        0.1,
        "the slit count follows the new perimeter (strip length)",
    );
}

// ---------- criterion 8: save, reopen and the registry ----------

#[test]
fn criterion_8_regeneration_from_the_stored_values_is_identical_and_ignores_the_seed() {
    let stored_values = stored(&[("slit_pattern", choice("offset")), ("lid_seat", sw(false))]);
    let first = generate_with(&stored_values, None);
    let again = generate_with(&first.params, None);
    assert_eq!(
        first.parts, again.parts,
        "reopening gives the same outlines"
    );
    assert_eq!(
        first.params, again.params,
        "the normalised values are stable"
    );
    let other_seed = GENERATORS
        .generate(&GenerateRequest {
            kind: ROUNDED_BOX,
            params: &stored_values,
            seed: Seed(9_999),
            edit: None,
        })
        .expect("answers");
    assert_eq!(first, other_seed, "the seed does not change a rounded box");
}

#[test]
fn criterion_8_an_undeclared_kind_is_not_generated() {
    let answer = GENERATORS.generate(&GenerateRequest {
        kind: "rounded-box-v2",
        params: &StoredParams::new(),
        seed: Seed(0),
        edit: None,
    });
    assert!(answer.is_none(), "an unknown kind yields no outline");
}

// ---------- criterion 9: the first build's parts ----------

fn non_strip_parts(generated: &Generated) -> Vec<usize> {
    let strip = strip_index(generated);
    (0..generated.parts.len()).filter(|&i| i != strip).collect()
}

#[test]
fn criterion_9_base_and_lid_combinations_give_the_expected_plates() {
    let cases: [(&str, StoredParams, usize); 5] = [
        ("default: base cover, seat lid", stored(&[]), 4),
        ("tabbed lid", stored(&[("lid_seat", sw(false))]), 3),
        ("no lid", stored(&[("lid", sw(false))]), 2),
        (
            "no base cover, seat lid",
            stored(&[("base_cover", sw(false))]),
            3,
        ),
        (
            "no base cover, no lid",
            stored(&[("base_cover", sw(false)), ("lid", sw(false))]),
            1,
        ),
    ];
    for (name, params, plates) in cases {
        let generated = generate_with(&params, None);
        assert_eq!(
            non_strip_parts(&generated).len(),
            plates,
            "{name}: the plates besides the strip"
        );
    }
    let tabbed = generate_with(&stored(&[("lid_seat", sw(false))]), None);
    assert!(
        non_strip_parts(&tabbed)
            .iter()
            .all(|&i| tabbed.parts[i].paths.len() == 1),
        "the tabbed lid and the other plates carry no seat ring (no second path)"
    );
}

// ---------- criterion 10: base inset and the plain cover ----------

#[test]
fn criterion_10_base_inset_below_one_wall_is_clamped_and_larger_values_are_not() {
    let small = boxed(&[("base_inset", mm(1.0))]);
    assert!(
        clamped(&small, "base_inset"),
        "1 mm inset (below the 3 mm wall) carries a notice"
    );
    for inset in [0.0, 10.0] {
        let generated = boxed(&[("base_inset", mm(inset))]);
        assert!(
            !clamped(&generated, "base_inset"),
            "inset {inset} is accepted without notice"
        );
    }
}

#[test]
fn criterion_10_cover_is_the_strip_outer_outline_as_a_plain_rounded_plate() {
    // Lid off, so the base cover is the largest plate after the strip.
    let generated = boxed(&[("lid", sw(false))]);
    let cover = non_strip_parts(&generated)
        .into_iter()
        .max_by(|&a, &b| outer_area(&generated, a).total_cmp(&outer_area(&generated, b)))
        .expect("the cover exists");
    let part = &generated.parts[cover];
    assert_eq!(
        part.paths.len(),
        1,
        "the cover has no holes and no tab outlines of its own"
    );
    let (x0, y0, x1, y1) = part_bbox(part);
    assert_close(
        x1 - x0,
        120.0 + 2.0 * WALL_T,
        0.1,
        "the cover is the footprint plus the wall, width",
    );
    assert_close(
        y1 - y0,
        80.0 + 2.0 * WALL_T,
        0.1,
        "the cover is the footprint plus the wall, depth",
    );
    // Plain rounded outline with radius r + t: area within 2 % of the ideal rounded rectangle.
    let (w, d, r) = (126.0, 86.0, 15.0 + WALL_T);
    let ideal = w * d - (4.0 - PI) * r * r;
    let actual = outer_area(&generated, cover);
    assert!(
        (actual - ideal).abs() / ideal <= 0.02,
        "the cover is a plain rounded rectangle: area {actual:.2} against {ideal:.2}"
    );
}

// ---------- criterion 12: the seat ring ----------

#[test]
fn criterion_12_seat_ring_opening_is_the_footprint_offset_by_twice_the_ring_thickness() {
    let generated = boxed(&[]);
    let ring: Vec<usize> = non_strip_parts(&generated)
        .into_iter()
        .filter(|&i| generated.parts[i].paths.len() == 2)
        .collect();
    assert_eq!(
        ring.len(),
        1,
        "exactly one plate carries an opening: the seat ring"
    );
    let part = &generated.parts[ring[0]];
    let (ox0, oy0, ox1, oy1) = all_points(&sample(&part.paths[1]));
    // Ring thickness follows the wall (3 mm); the ring width is 2 times it, so 4 mm per side is
    // the opening's inset from the footprint's 120 x 80: 120 - 2 x 6 = 108 wide.
    let ring_width = 2.0 * 3.0;
    assert_close(ox1 - ox0, 120.0 - 2.0 * ring_width, 0.1, "opening width");
    assert_close(oy1 - oy0, 80.0 - 2.0 * ring_width, 0.1, "opening depth");
}

// ---------- criterion 13: the finger hollow and the lid ----------

#[test]
fn criterion_13_finger_hollow_is_a_u_notch_centred_on_the_front_edge_and_the_lid_stays_whole() {
    let hollow_width = 16.0;
    let hollow_depth = 8.0;
    let with = boxed(&[
        ("base_cover", sw(false)),
        ("hollow_width", mm(hollow_width)),
        ("hollow_depth", mm(hollow_depth)),
    ]);
    let without = boxed(&[("base_cover", sw(false)), ("finger_hollow", sw(false))]);
    assert_eq!(
        with.parts.len(),
        without.parts.len(),
        "the same plates either way"
    );
    let differing: Vec<usize> = (0..with.parts.len())
        .filter(|&i| (outer_area(&with, i) - outer_area(&without, i)).abs() > 1.0)
        .collect();
    assert_eq!(differing.len(), 1, "exactly one plate changes: the lid");
    let lid = &with.parts[differing[0]];
    assert_eq!(lid.paths.len(), 1, "the lid stays one piece with no hole");
    let points = sample(&lid.paths[0]);
    let (x0, y0, x1, y1) = all_points(&points);
    let cx = f64::midpoint(x0, x1);
    // The front edge is the straight edge that the notch reaches into: at the centre line the
    // outline is nearest to that edge at the notch bottom, the other edge stays at its full line.
    let nearest_at_centre = |edge: f64| -> f64 {
        points
            .iter()
            .filter(|(x, _)| (x - cx).abs() <= 0.5)
            .map(|(_, y)| (y - edge).abs())
            .fold(f64::INFINITY, f64::min)
    };
    let (edge, depth) = [y0, y1]
        .into_iter()
        .map(|e| (e, nearest_at_centre(e)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .expect("two edges");
    assert_close(
        depth,
        hollow_depth,
        0.5,
        "the notch reaches the hollow depth from the front edge",
    );
    // Only the notch walls and bottom: the corner arcs and side walls lie outside this band.
    let band: Vec<f64> = points
        .iter()
        .filter(|(x, y)| {
            let dy = (y - edge).abs();
            dy > 0.5 && dy < hollow_depth - 0.5 && (x - cx).abs() < 0.5 * hollow_width + 2.0
        })
        .map(|(x, _)| *x)
        .collect();
    let (min_x, max_x) = band
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &x| {
            (lo.min(x), hi.max(x))
        });
    assert_close(
        f64::midpoint(min_x, max_x),
        cx,
        0.5,
        "the notch is centred on the front edge",
    );
    assert_close(
        max_x - min_x,
        hollow_width,
        1.0,
        "the notch has the hollow width",
    );
    let removed = outer_area(&without, differing[0]) - outer_area(&with, differing[0]);
    let rectangle = hollow_width * hollow_depth;
    assert!(
        removed >= 0.5 * rectangle && removed <= 1.05 * rectangle,
        "the removed area {removed:.2} is about the notch ({rectangle:.2})"
    );
}

#[test]
fn criterion_13_finger_hollow_defaults_on_with_the_seat_lid_and_is_absent_on_the_tabbed_lid() {
    // Base cover off: strip, inner base, seat ring and lid; tabbed lid: strip, inner base, lid.
    let seat = boxed(&[("base_cover", sw(false))]);
    assert_eq!(seat.parts.len(), 4, "strip, inner base, seat ring and lid");
    let no_hollow = boxed(&[("base_cover", sw(false)), ("finger_hollow", sw(false))]);
    let lid_index = (0..seat.parts.len())
        .find(|&i| (outer_area(&seat, i) - outer_area(&no_hollow, i)).abs() > 1.0)
        .expect("the default seat lid carries the finger hollow");
    assert!(outer_area(&seat, lid_index) < outer_area(&no_hollow, lid_index));
    let tabbed = boxed(&[("base_cover", sw(false)), ("lid_seat", sw(false))]);
    assert_eq!(
        tabbed.parts.len(),
        3,
        "strip, inner base and the tabbed lid"
    );
    assert!(
        !tabbed.parts.iter().any(|p| p.paths.len() == 2),
        "the tabbed lid has no seat ring"
    );
}

// ---------- undeclared behaviour check: the gear and box kinds are untouched ----------

#[test]
fn the_rounded_box_does_not_change_the_gear_or_box_kinds() {
    for kind in [GEAR, BOX] {
        let generated = GENERATORS
            .generate(&GenerateRequest {
                kind,
                params: &StoredParams::new(),
                seed: Seed(0),
                edit: None,
            })
            .expect("answers at its defaults");
        assert!(!generated.parts.is_empty(), "{kind} still has outlines");
    }
}

// ---------- slice 2: the curved top (criteria 11 and 16 to 24, ADR 0082 §11) ----------
//
// Black-box, as the rest of this file: manifest, stored parameters and outlines. The strip is read
// in its own flat frame: `u` from the free front edge (the lid) to the base end, `v` across the
// width. The orientation is found from the outline (the lid is the plain end), not assumed.

/// The sizes and thicknesses of a curved box: inside `w`, `d`, `h`, corner radius `r`, wall `t`,
/// end plate `te`, front `tf`, base `tb` (the inner base thickness).
#[derive(Clone, Copy, Debug)]
struct Dims {
    w: f64,
    d: f64,
    h: f64,
    r: f64,
    t: f64,
    te: f64,
    tf: f64,
    tb: f64,
}

/// The spec's curved box: 120 x 80 x 50 inside, corner radius 15, every thickness 3 mm.
const SLICE2: Dims = Dims {
    w: 120.0,
    d: 80.0,
    h: 50.0,
    r: 15.0,
    t: 3.0,
    te: 3.0,
    tf: 3.0,
    tb: 3.0,
};

impl Dims {
    /// The strip's flat width: the inside width plus both end plates (their faces flush with it).
    fn width(self) -> f64 {
        self.w + 2.0 * self.te
    }

    /// The lid run on the neutral line: from the front's outer face to the arc (criterion 17).
    fn front_run(self) -> f64 {
        self.d - self.r + self.tf
    }

    /// The quarter arc on the neutral line, `pi/2 (r + t/2)` (criterion 17).
    fn arc(self) -> f64 {
        PI / 2.0 * (self.r + self.t / 2.0)
    }

    /// The back run: from the arc's end down to the base's underside (criterion 17).
    fn back_run(self) -> f64 {
        self.h - self.r + self.tb
    }

    /// The strip's length on the neutral line (criterion 17).
    fn length(self) -> f64 {
        self.front_run() + self.arc() + self.back_run()
    }
}

/// A curved box: the stored values of `dm` plus `extra` (which win), `top_shape` curved.
fn curved_box(dm: Dims, extra: &[(&str, Value)]) -> Generated {
    let mut all: Vec<(&str, Value)> = vec![
        ("top_shape", choice("curved")),
        ("end_plate_thickness", mm(dm.te)),
        ("front_thickness", mm(dm.tf)),
        ("plate_thickness", mm(dm.tb)),
    ];
    all.extend(extra.iter().cloned());
    generate_with(&stored_at(dm.w, dm.d, dm.h, dm.r, dm.t, &all), None)
}

/// A flat plate in its own frame: the outlines as sampled points and their bounding box.
struct Plate {
    paths: Vec<Vec<Pt>>,
    bbox: (f64, f64, f64, f64),
}

fn plate_of(generated: &Generated, index: usize) -> Plate {
    let paths: Vec<Vec<Pt>> = generated.parts[index].paths.iter().map(sample).collect();
    let flat: Vec<Pt> = paths.iter().flatten().copied().collect();
    Plate {
        bbox: all_points(&flat),
        paths,
    }
}

impl Plate {
    fn extent_x(&self) -> f64 {
        self.bbox.2 - self.bbox.0
    }

    fn extent_y(&self) -> f64 {
        self.bbox.3 - self.bbox.1
    }

    fn min_of(&self, is_x: bool) -> f64 {
        if is_x { self.bbox.0 } else { self.bbox.1 }
    }

    fn max_of(&self, is_x: bool) -> f64 {
        if is_x { self.bbox.2 } else { self.bbox.3 }
    }

    /// Whether the axis whose extent is `extent` is x (a plate may come out turned or mirrored).
    fn axis_is_x(&self, extent: f64) -> bool {
        if (self.extent_x() - extent).abs() <= 0.1 {
            true
        } else if (self.extent_y() - extent).abs() <= 0.1 {
            false
        } else {
            panic!(
                "no side of {:.3} x {:.3} is {:.3}",
                self.extent_x(),
                self.extent_y(),
                extent
            )
        }
    }
}

/// The runs of outline points on the line where the coordinate on `axis_is_x` equals `level`,
/// each as the range of the other coordinate.
fn runs_on(paths: &[Vec<Pt>], axis_is_x: bool, level: f64) -> Vec<(f64, f64)> {
    let mut runs = Vec::new();
    for path in paths {
        let mut run: Option<(f64, f64)> = None;
        for &(x, y) in path {
            let (c, o) = if axis_is_x { (x, y) } else { (y, x) };
            if (c - level).abs() < 1e-3 {
                run = Some(match run {
                    Some((lo, hi)) => (lo.min(o), hi.max(o)),
                    None => (o, o),
                });
            } else if let Some(finished) = run.take() {
                runs.push(finished);
            }
        }
        runs.extend(run);
    }
    runs
}

/// Whether a run of `width` (to the sampling) lies on the line `level` of the given axis.
fn has_run_of_width(plate: &Plate, axis_is_x: bool, level: f64, width: f64) -> bool {
    runs_on(&plate.paths, axis_is_x, level)
        .iter()
        .any(|&(lo, hi)| (hi - lo - width).abs() <= 1.0)
}

/// The outline points with `u` in `[lo, hi]` that lie inside the strip's width: slot and notch walls.
fn cut_pts(paths: &[Vec<Pt>], lo: f64, hi: f64, width: f64) -> Vec<Pt> {
    paths
        .iter()
        .flatten()
        .copied()
        .filter(|&(u, v)| (lo..=hi).contains(&u) && v > 0.05 && v < width - 0.05)
        .collect()
}

/// The strip in its own frame, with the lid at `u = 0`.
struct Frame {
    strip: Strip,
    lid: f64,
    arc: f64,
}

/// The strip's frame. The flat outline may come out mirrored or turned: the lid is the end whose
/// first straight run carries no cut, so `u` is flipped when the front window holds one.
fn frame_of(generated: &Generated, dm: Dims) -> Frame {
    let raw: Vec<Vec<Pt>> = generated.parts[0].paths.iter().map(sample).collect();
    let flat: Vec<Pt> = raw.iter().flatten().copied().collect();
    let (x0, y0, x1, y1) = all_points(&flat);
    let (ex, ey) = (x1 - x0, y1 - y0);
    let (length, width) = (dm.length(), dm.width());
    let along_x = if (ex - length).abs() <= 0.1 && (ey - width).abs() <= 0.1 {
        true
    } else if (ey - length).abs() <= 0.1 && (ex - width).abs() <= 0.1 {
        false
    } else {
        panic!("the strip's flat extents are {ex:.3} x {ey:.3}, not {length:.3} x {width:.3}")
    };
    let map = |flip: bool| -> Vec<Vec<Pt>> {
        raw.iter()
            .map(|path| {
                path.iter()
                    .map(|&(x, y)| {
                        let (u, v) = if along_x {
                            (x - x0, y - y0)
                        } else {
                            (y - y0, x - x0)
                        };
                        (if flip { length - u } else { u }, v)
                    })
                    .collect()
            })
            .collect()
    };
    let plain = map(false);
    let front_cuts = cut_pts(&plain, 0.5, dm.front_run() - 0.5, width).len();
    let paths = if front_cuts > 0 { map(true) } else { plain };
    Frame {
        strip: Strip {
            paths,
            length,
            height: width,
        },
        lid: dm.front_run(),
        arc: dm.arc(),
    }
}

/// The slit columns among cut points: `(first u, last u)` of each group of points less than `gap`
/// apart in `u`.
fn columns(points: &[Pt], gap: f64) -> Vec<(f64, f64)> {
    let mut us: Vec<f64> = points.iter().map(|p| p.0).collect();
    us.sort_by(f64::total_cmp);
    let mut groups: Vec<(f64, f64)> = Vec::new();
    for u in us {
        match groups.last_mut() {
            Some(group) if u - group.1 <= gap => group.1 = u,
            _ => groups.push((u, u)),
        }
    }
    groups
}

/// Whether two lists of positions agree pairwise within `tolerance`, in any order.
fn same_positions(a: &[f64], b: &[f64], tolerance: f64) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut a = a.to_vec();
    let mut b = b.to_vec();
    a.sort_by(f64::total_cmp);
    b.sort_by(f64::total_cmp);
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= tolerance)
}

/// Criteria 17 and 19 on the strip: its length on the neutral line, a plain lid (no slot, notch or
/// slit on the top straight), a back straight with end plate notches of depth `te` only, and no
/// uncut band across the arc.
fn check_strip_and_back(generated: &Generated, dm: Dims) -> Frame {
    let f = frame_of(generated, dm);
    assert_close(
        f.strip.length,
        dm.length(),
        0.1,
        "the strip length on the neutral line",
    );
    let lid = cut_pts(&f.strip.paths, 0.5, f.lid - 0.5, f.strip.height);
    assert!(
        lid.is_empty(),
        "the top straight (the lid) carries no slot, notch or slit: {lid:?}"
    );
    let back = cut_pts(
        &f.strip.paths,
        f.lid + f.arc + 0.5,
        f.strip.length - dm.tb - 2.0 * dm.t - 0.5,
        f.strip.height,
    );
    let notch_only = back
        .iter()
        .all(|&(_, v)| v <= dm.te + 0.05 || v >= f.strip.height - dm.te - 0.05);
    assert!(
        notch_only,
        "the back straight holds only the end plate notches of depth te: {:?}",
        back.iter()
            .filter(|p| !(p.1 <= dm.te + 0.05 || p.1 >= f.strip.height - dm.te - 0.05))
            .take(12)
            .collect::<Vec<_>>()
    );
    let (failures, first) = uncut_bands(&f.strip, &[(f.lid, f.lid + f.arc)]);
    assert_eq!(
        failures, 0,
        "no uncut band across the arc; first at {first:?}"
    );
    f
}

/// The boxes the mating and length checks run on: the defaults, and two sets of five-ish
/// thicknesses with other radii.
fn curved_cases() -> [Dims; 4] {
    [
        SLICE2,
        Dims {
            te: 4.0,
            tf: 5.0,
            tb: 6.0,
            ..SLICE2
        },
        Dims {
            w: 150.0,
            d: 100.0,
            h: 60.0,
            r: 12.0,
            t: 2.5,
            te: 3.5,
            tf: 4.5,
            tb: 5.5,
        },
        Dims {
            r: 25.0,
            tf: 4.0,
            ..SLICE2
        },
    ]
}

fn assert_extents(plate: &Plate, expected: (f64, f64), what: &str) {
    let (a, b) = (plate.extent_x(), plate.extent_y());
    assert_close(a.min(b), expected.0, 0.1, &format!("{what}: short side"));
    assert_close(a.max(b), expected.1, 0.1, &format!("{what}: long side"));
}

// ---------- criterion 11: the curved top is a choice, and it is a curved box ----------

#[test]
fn s2_criterion_11_top_shape_is_a_choice_flat_by_default_with_a_curved_option() {
    let kind = rounded_kind();
    let params: Vec<&Param> = kind.groups.iter().flat_map(|g| g.params.iter()).collect();
    let find = |key: &str| -> &Param {
        params
            .iter()
            .copied()
            .find(|p| p.key == key)
            .unwrap_or_else(|| panic!("the parameter {key} is declared"))
    };
    let top = find("top_shape");
    assert_eq!(top.label, "Top shape");
    match &top.kind {
        ParamKind::Choice { options } => assert_eq!(
            options.iter().map(|o| o.id).collect::<Vec<_>>(),
            ["flat", "curved"]
        ),
        other => panic!("top_shape is a choice, got {other:?}"),
    }
    assert!(
        matches!(&top.default, ParamValue::Choice(c) if c == "flat"),
        "Top shape defaults to Flat"
    );
    for (key, label) in [
        ("end_plate_thickness", "End plate thickness"),
        ("front_thickness", "Front thickness"),
        ("plate_thickness", "Inner base thickness"),
    ] {
        assert_eq!(find(key).label, label, "{key} carries its label");
    }
}

#[test]
fn s2_criterion_11_curved_top_gives_five_parts_that_differ_from_flat() {
    let flat_points = |g: &Generated| -> Vec<Vec<Pt>> { g.parts.iter().map(part_points).collect() };
    let curved = curved_box(SLICE2, &[]);
    let flat = boxed(&[]);
    assert_eq!(curved.parts.len(), 5, "strip, left, right, front, base");
    assert_ne!(
        flat_points(&curved),
        flat_points(&flat),
        "a curved top is not the flat box"
    );
}

// ---------- criterion 16: five parts in numbering order, with their sizes ----------

#[test]
fn s2_criterion_16_five_parts_strip_then_end_plates_front_and_base_with_their_sizes() {
    let dm = SLICE2;
    let g = curved_box(dm, &[]);
    assert_eq!(g.parts.len(), 5);
    // Strip: the flat outline is the length on the neutral line by the width (criterion 17).
    assert_extents(
        &plate_of(&g, 0),
        (dm.width().min(dm.length()), dm.width().max(dm.length())),
        "1 strip",
    );
    // End plates: height `h + tb`, depth `d + tf` plus the back tabs of `t` (ADR 0082 §11).
    for index in [1, 2] {
        assert_extents(
            &plate_of(&g, index),
            (dm.h + dm.tb, dm.d + dm.tf + dm.t),
            "end plate",
        );
    }
    // Front: the inside width plus the end plates, by its height and the base's thickness.
    assert_extents(&plate_of(&g, 3), (dm.h + dm.tb, dm.width()), "4 front");
    // Base: the width by the depth from the front's outer face to the strip's outer face.
    assert_extents(
        &plate_of(&g, 4),
        (dm.d + dm.tf + dm.t, dm.width()),
        "5 base",
    );
}

// ---------- criteria 17 and 19: the strip, the lid, the arc ----------

#[test]
fn s2_criterion_17_strip_length_is_the_profile_edges_on_the_neutral_line() {
    for dm in curved_cases() {
        let g = curved_box(dm, &[]);
        check_strip_and_back(&g, dm);
    }
    // The defaults with the thickness fields left out (automatic, following the wall).
    let g = generate_with(
        &stored_at(
            120.0,
            80.0,
            50.0,
            15.0,
            3.0,
            &[("top_shape", choice("curved"))],
        ),
        None,
    );
    check_strip_and_back(&g, SLICE2);
}

#[test]
fn s2_criterion_19_arc_is_slit_across_at_kerf_plus_point_three_and_pitch_point_two_two_r() {
    for pattern in ["straight", "offset", "wave"] {
        let g = curved_box(SLICE2, &[("slit_pattern", choice(pattern))]);
        let f = check_strip_and_back(&g, SLICE2);
        let arc = cut_pts(
            &f.strip.paths,
            f.lid - 0.01,
            f.lid + f.arc + 0.01,
            f.strip.height,
        );
        assert!(
            columns(&arc, 0.6).len() >= 2,
            "{pattern}: the arc keeps at least two slit columns"
        );
    }
    // Straight at the defaults: the slot width and the pitch are the automatic values.
    let g = curved_box(SLICE2, &[]);
    let f = frame_of(&g, SLICE2);
    let cols = columns(
        &cut_pts(
            &f.strip.paths,
            f.lid - 0.01,
            f.lid + f.arc + 0.01,
            f.strip.height,
        ),
        0.6,
    );
    assert!(cols.len() >= 2, "the arc has slit columns");
    for &(a, b) in &cols {
        assert_close(
            b - a,
            0.3,
            0.02,
            "a slot is kerf plus 0.3 mm (0.3 at kerf 0)",
        );
    }
    for pair in cols.windows(2) {
        let pitch = f64::midpoint(pair[1].0, pair[1].1) - f64::midpoint(pair[0].0, pair[0].1);
        assert_close(
            pitch,
            0.22 * SLICE2.r,
            0.1,
            "the automatic spacing is 0.22 r",
        );
    }
}

#[test]
fn s2_criterion_19_all_round_wall_slits_is_not_offered_so_the_back_stays_solid() {
    // Wall slits has no meaning on a curved top: the stored value is kept, the back straight
    // still carries only the end plate notches (check_strip_and_back).
    let g = curved_box(SLICE2, &[("wall_slits", choice("all"))]);
    check_strip_and_back(&g, SLICE2);
}

// ---------- criterion 18: the joints ----------

#[test]
fn s2_criterion_18_end_plate_tabs_meet_the_strip_notches_of_the_back_straight_only() {
    for dm in curved_cases() {
        let g = curved_box(dm, &[]);
        let f = check_strip_and_back(&g, dm);
        let back_start = f.lid + f.arc;
        let mut sides: Vec<Vec<f64>> = Vec::new();
        for level in [dm.te, f.strip.height - dm.te] {
            let centres = notch_centres(&f.strip, level, dm.te);
            for &u in &centres {
                assert!(
                    u - dm.te >= back_start + 2.0 * dm.t - 0.2,
                    "a notch at u {u:.2} sits closer than 2 t_w to the arc"
                );
            }
            sides.push(centres.iter().map(|u| f.strip.length - u).collect());
        }
        assert!(!sides[0].is_empty(), "the back straight has notches");
        for index in [1, 2] {
            let plate = plate_of(&g, index);
            let h_is_x = plate.axis_is_x(dm.h + dm.tb);
            let d_is_x = !h_is_x;
            // The tabs stand t_w beyond the back edge: the side whose tips are all one tab wide
            // (2 t_e); the front edge's finger tips are another width at these sizes. (The first
            // version looked for a long back edge one t_w in, which a tall tab leaves in pieces
            // shorter than 10 mm at radius 25.)
            let tab_wide = |level: f64| {
                let runs = runs_on(&plate.paths, d_is_x, level);
                !runs.is_empty()
                    && runs
                        .iter()
                        .all(|&(lo, hi)| (hi - lo - 2.0 * dm.te).abs() < 0.05)
            };
            let tab_level = if tab_wide(plate.max_of(d_is_x)) {
                plate.max_of(d_is_x)
            } else {
                plate.min_of(d_is_x)
            };
            let tips: Vec<f64> = runs_on(&plate.paths, d_is_x, tab_level)
                .iter()
                .map(|&(lo, hi)| f64::midpoint(lo, hi))
                .collect();
            assert!(
                !tips.is_empty(),
                "end plate {index} has tabs at its back edge"
            );
            let (hmin, hmax) = (plate.min_of(h_is_x), plate.max_of(h_is_x));
            let from_bottom: Vec<f64> = tips.iter().map(|c| c - hmin).collect();
            let from_top: Vec<f64> = tips.iter().map(|c| hmax - c).collect();
            let mates = sides.iter().any(|side| {
                same_positions(side, &from_bottom, 0.25) || same_positions(side, &from_top, 0.25)
            });
            assert!(
                mates,
                "end plate {index}: its tab centres {from_bottom:.2?} meet no notch row {sides:.2?}"
            );
        }
    }
}

#[test]
fn s2_criterion_18_base_and_front_fingers_are_as_deep_as_the_mating_plate_and_twice_the_thinner() {
    let dm = Dims {
        te: 4.0,
        tf: 5.0,
        tb: 6.0,
        ..SLICE2
    };
    let g = curved_box(dm, &[("finger_hollow", sw(false))]);
    // Base (5): left and right to the end plates (depth te), front to the front (depth tf).
    let base = plate_of(&g, 4);
    let w_is_x = base.axis_is_x(dm.width());
    assert!(
        has_run_of_width(
            &base,
            w_is_x,
            base.min_of(w_is_x) + dm.te,
            2.0 * dm.tb.min(dm.te)
        ) && has_run_of_width(
            &base,
            w_is_x,
            base.max_of(w_is_x) - dm.te,
            2.0 * dm.tb.min(dm.te)
        ),
        "the base's sides carry fingers of depth te and width 2 min(tb, te)"
    );
    assert!(
        has_run_of_width(
            &base,
            !w_is_x,
            base.min_of(!w_is_x) + dm.tf,
            2.0 * dm.tb.min(dm.tf)
        ) || has_run_of_width(
            &base,
            !w_is_x,
            base.max_of(!w_is_x) - dm.tf,
            2.0 * dm.tb.min(dm.tf)
        ),
        "the base's front carries fingers of depth tf and width 2 min(tb, tf)"
    );
    // Front (4): sides to the end plates (depth te), bottom to the base (depth tb).
    let front = plate_of(&g, 3);
    let fw_is_x = front.axis_is_x(dm.width());
    assert!(
        has_run_of_width(
            &front,
            fw_is_x,
            front.min_of(fw_is_x) + dm.te,
            2.0 * dm.tf.min(dm.te)
        ) && has_run_of_width(
            &front,
            fw_is_x,
            front.max_of(fw_is_x) - dm.te,
            2.0 * dm.tf.min(dm.te)
        ),
        "the front's sides carry fingers of depth te and width 2 min(tf, te)"
    );
    assert!(
        has_run_of_width(
            &front,
            !fw_is_x,
            front.min_of(!fw_is_x) + dm.tb,
            2.0 * dm.tf.min(dm.tb)
        ) || has_run_of_width(
            &front,
            !fw_is_x,
            front.max_of(!fw_is_x) - dm.tb,
            2.0 * dm.tf.min(dm.tb)
        ),
        "the front's bottom carries fingers of depth tb and width 2 min(tf, tb)"
    );
    // End plate (2): bottom to the base (depth tb), front edge to the front (depth tf).
    let end = plate_of(&g, 1);
    let eh_is_x = end.axis_is_x(dm.h + dm.tb);
    assert!(
        has_run_of_width(
            &end,
            eh_is_x,
            end.min_of(eh_is_x) + dm.tb,
            2.0 * dm.te.min(dm.tb)
        ) || has_run_of_width(
            &end,
            eh_is_x,
            end.max_of(eh_is_x) - dm.tb,
            2.0 * dm.te.min(dm.tb)
        ),
        "the end plate's bottom carries fingers of depth tb"
    );
    assert!(
        has_run_of_width(
            &end,
            !eh_is_x,
            end.min_of(!eh_is_x) + dm.tf,
            2.0 * dm.te.min(dm.tf)
        ) || has_run_of_width(
            &end,
            !eh_is_x,
            end.max_of(!eh_is_x) - dm.tf,
            2.0 * dm.te.min(dm.tf)
        ),
        "the end plate's front edge carries fingers of depth tf"
    );
}

// ---------- criterion 20: the corner radius clamp ----------

#[test]
fn s2_criterion_20_corner_radius_is_clamped_to_the_most_the_box_allows_and_to_two_walls() {
    // The most is min(d - 2t, h - 2 te - 4 tw): 74 and 32 at the defaults.
    let g = curved_box(Dims { r: 40.0, ..SLICE2 }, &[]);
    assert_close(
        length_of(&g, "corner_radius"),
        32.0,
        1e-9,
        "the most at the defaults",
    );
    assert!(clamped(&g, "corner_radius"), "the clamp is reported");
    // A thicker end plate takes its own 2 te from the height: 50 - 8 - 12 = 30.
    let g = curved_box(
        Dims {
            r: 40.0,
            te: 4.0,
            ..SLICE2
        },
        &[],
    );
    assert_close(
        length_of(&g, "corner_radius"),
        30.0,
        1e-9,
        "the most with te 4",
    );
    // A shorter box: 30 - 6 - 12 = 12.
    let g = curved_box(
        Dims {
            r: 20.0,
            h: 30.0,
            ..SLICE2
        },
        &[],
    );
    assert_close(
        length_of(&g, "corner_radius"),
        12.0,
        1e-9,
        "the most in a 30 mm high box",
    );
    // The lower bound is two walls.
    let g = curved_box(Dims { r: 2.0, ..SLICE2 }, &[]);
    assert_close(
        length_of(&g, "corner_radius"),
        6.0,
        1e-9,
        "the least is 2 t_w",
    );
    assert!(clamped(&g, "corner_radius"));
    // Inside the range nothing is clamped.
    let g = curved_box(SLICE2, &[]);
    assert_close(length_of(&g, "corner_radius"), 15.0, 1e-9, "15 mm is kept");
    assert!(!clamped(&g, "corner_radius"));
}

// ---------- criterion 21: the rows that do not apply keep their values; the thicknesses ----------

#[test]
fn s2_criterion_21_hidden_values_are_saved_back_unread_and_the_thickness_fields_follow_the_wall() {
    let kept = [
        ("lid", sw(false)),
        ("lid_seat", sw(false)),
        ("base_cover", sw(false)),
        ("wall_slits", choice("all")),
        ("base_inset", mm(4.0)),
        ("ring_thickness", mm(2.5)),
        ("cover_thickness", mm(2.5)),
        ("lid_thickness", mm(2.5)),
    ];
    let g = curved_box(SLICE2, &kept);
    for (key, value) in &kept {
        assert_eq!(
            g.params.get(*key),
            Some(value),
            "{key} is saved back unread"
        );
    }
    assert_eq!(g.parts.len(), 5, "the hidden switches add no part");
    // A stored 0 follows the wall and is shown muted.
    let g = curved_box(
        SLICE2,
        &[
            ("end_plate_thickness", mm(0.0)),
            ("front_thickness", mm(0.0)),
            ("plate_thickness", mm(0.0)),
        ],
    );
    for key in ["end_plate_thickness", "front_thickness", "plate_thickness"] {
        assert!(is_automatic(&g, key), "{key} is automatic at 0");
        assert_close(used_length(&g, key), 3.0, 1e-9, key);
    }
    // A typed value overrides the wall.
    let g = curved_box(SLICE2, &[("front_thickness", mm(4.0))]);
    assert!(
        !is_automatic(&g, "front_thickness"),
        "a typed front thickness is not automatic"
    );
    assert_close(
        length_of(&g, "front_thickness"),
        4.0,
        1e-9,
        "front thickness 4",
    );
}

#[test]
fn s2_criterion_21_finger_hollow_is_a_u_notch_in_the_front_top_edge_one_piece_with_margin() {
    let dm = SLICE2;
    let g = curved_box(
        dm,
        &[
            ("finger_hollow", sw(true)),
            ("hollow_width", mm(20.0)),
            ("hollow_depth", mm(10.0)),
        ],
    );
    let front = plate_of(&g, 3);
    assert_eq!(g.parts[3].paths.len(), 1, "the front stays in one piece");
    let w_is_x = front.axis_is_x(dm.width());
    let h_is_x = !w_is_x;
    // The top edge is the high or the low end of the height. Measure the notch by depth, so a U
    // with a flat bottom and a rounded one both count: the points 0.5 to 10.5 mm down from the top,
    // clear of the side fingers (depth te), span the hollow's width and reach 10 mm deep.
    let mut found: Option<(f64, f64, f64)> = None;
    for top in [front.max_of(h_is_x), front.min_of(h_is_x)] {
        let sign = if top == front.max_of(h_is_x) {
            -1.0
        } else {
            1.0
        };
        let inside: Vec<Pt> = front
            .paths
            .iter()
            .flatten()
            .copied()
            .filter(|&(x, y)| {
                let (w, h) = if w_is_x { (x, y) } else { (y, x) };
                let depth = sign * (h - top);
                depth > 0.5
                    && depth < 10.5
                    && w > front.min_of(w_is_x) + dm.te + 0.5
                    && w < front.max_of(w_is_x) - dm.te - 0.5
            })
            .map(|(x, y)| if w_is_x { (x, y) } else { (y, x) })
            .collect();
        if inside.is_empty() {
            continue;
        }
        let lo = inside.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let hi = inside.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let deepest = inside
            .iter()
            .map(|p| sign * (p.1 - top))
            .fold(0.0_f64, f64::max);
        if (deepest - 10.0).abs() <= 0.2 {
            found = Some((lo, hi, deepest));
        }
    }
    let (lo, hi, _) = found.expect("a hollow 10 mm deep in the front's top edge");
    assert!(
        (hi - lo - 20.0).abs() <= 0.6,
        "the hollow is 20 mm wide: {:.2}",
        hi - lo
    );
    let centre_across = f64::midpoint(front.min_of(w_is_x), front.max_of(w_is_x));
    assert_close(
        f64::midpoint(lo, hi),
        centre_across,
        0.2,
        "the hollow is centred across the width",
    );
    // Revision 9 (spec 0147): the hollow stays within Inner width minus 2 times the End plate
    // thickness, measured from the front's side edges (where the side fingers of depth te sit).
    let inner_span = dm.w - 2.0 * dm.te;
    assert!(
        hi - lo <= inner_span + 0.2,
        "the hollow is at most the inner width less 2 te ({inner_span} mm): {:.2}",
        hi - lo
    );
    let left_from_edge = lo - (front.min_of(w_is_x) + dm.te);
    let right_from_edge = (front.max_of(w_is_x) - dm.te) - hi;
    assert!(
        left_from_edge >= -0.2 && right_from_edge >= -0.2,
        "the hollow lies between the side edges: {left_from_edge:.2}, {right_from_edge:.2}"
    );
}

#[test]
fn criterion_19_typed_slit_spacing_the_arc_cannot_hold_twice_is_clamped_with_a_notice() {
    // The arc at r 15 holds about 26 mm: at most pi/8 (2r + t_w) = 12.95 mm per slit spacing
    // (architect review, revision 9). A typed 20 mm must be clamped, not kept.
    let g = curved_box(SLICE2, &[("slit_spacing", mm(20.0))]);
    assert!(
        clamped(&g, "slit_spacing"),
        "the spacing is clamped with a notice"
    );
    assert_close(
        length_of(&g, "slit_spacing"),
        PI / 8.0 * (2.0 * SLICE2.r + SLICE2.t),
        0.05,
        "the clamp value pi/8 (2r + t_w)",
    );
}

// ---------- criterion 23: the flat layout ----------

#[test]
fn s2_criterion_23_flat_layout_keeps_all_five_parts_apart_by_the_gap_in_order() {
    let g = curved_box(SLICE2, &[("gap", mm(5.0))]);
    let boxes: Vec<(f64, f64, f64, f64)> = (0..5).map(|i| part_bbox(&g.parts[i])).collect();
    for i in 0..5 {
        for j in (i + 1)..5 {
            let (a, b) = (boxes[i], boxes[j]);
            let sep_x = (b.0 - a.2).max(a.0 - b.2);
            let sep_y = (b.1 - a.3).max(a.1 - b.3);
            assert!(
                sep_x.max(sep_y) >= 5.0 - 0.1,
                "parts {} and {} are {:.2} apart, less than the gap",
                i + 1,
                j + 1,
                sep_x.max(sep_y)
            );
        }
    }
    // The plates in their row: left, right, front, base in order along one axis.
    let in_x = (1..4).all(|i| boxes[i].0 < boxes[i + 1].0);
    let in_y = (1..4).all(|i| boxes[i].1 < boxes[i + 1].1);
    assert!(
        in_x || in_y,
        "left, right, front and base follow each other in one row"
    );
}

// ---------- criterion 24: a box of about 400 slots, the regeneration cost, the file ----------

#[test]
fn s2_criterion_24_a_curved_box_of_about_400_slots_regenerates_within_100_ms_of_thread_cpu() {
    let dm = Dims {
        w: 200.0,
        d: 900.0,
        h: 450.0,
        r: 400.0,
        ..SLICE2
    };
    let extra = [("slit_spacing", mm(1.5))];
    let _warm = curved_box(dm, &extra);
    let mut best = u64::MAX;
    let mut last = None;
    for _ in 0..3 {
        let before = thread_ticks().expect("the thread CPU clock is readable");
        let generated = curved_box(dm, &extra);
        let after = thread_ticks().expect("the thread CPU clock is readable");
        best = best.min(after - before);
        last = Some(generated);
    }
    let g = last.expect("three runs");
    let f = frame_of(&g, dm);
    let arc = cut_pts(
        &f.strip.paths,
        f.lid - 0.01,
        f.lid + f.arc + 0.01,
        f.strip.height,
    );
    let slots = columns(&arc, 0.6).len();
    eprintln!("slice 2 timing: {slots} slit columns on the arc, best run {best} clock ticks");
    assert!(slots >= 400, "the box carries about 400 slots: {slots}");
    let took = budget_factor::scaled(std::time::Duration::from_millis(best * 10));
    assert!(
        took <= std::time::Duration::from_millis(100),
        "regeneration took {} ms of thread CPU (scaled by the CI factor: {took:?})",
        best * 10
    );
}

// ---------- criterion 8 and 24: an older file opens as Flat; regeneration is stable ----------

#[test]
fn s2_criterion_24_a_file_without_top_shape_opens_as_flat_and_regenerates_identically() {
    let points = |g: &Generated| -> Vec<Vec<Pt>> { g.parts.iter().map(part_points).collect() };
    let absent = generate_with(&stored(&[]), None);
    let explicit_flat = generate_with(&stored(&[("top_shape", choice("flat"))]), None);
    assert_eq!(
        points(&absent),
        points(&explicit_flat),
        "no top_shape reads as Flat"
    );
    let first = curved_box(SLICE2, &[]);
    let second = curved_box(SLICE2, &[]);
    assert_eq!(
        points(&first),
        points(&second),
        "the curved box regenerates identically"
    );
    assert_eq!(first.params.get("top_shape"), Some(&choice("curved")));
}
