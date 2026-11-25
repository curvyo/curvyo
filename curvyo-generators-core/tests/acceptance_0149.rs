//! Acceptance tests of `specs/0149-path-box/specification.md`, slice 1 (straight outlines, kinks
//! included; curved outlines refused in ui-core), through the public generator contract (ADR 0080,
//! ADR 0083). Black-box: manifest, stored parameters, warnings, marks and outlines only.
//!
//! Written before the implementation was read. Coordinates: outlines in frame-local mm, pieces
//! found by their badge marks (`Mark::Piece`) and bounding boxes, not by part order.
//! Not covered here (named in the report): criterion 4's finger mating (the finger geometry is not
//! exposed as labelled outlines), criteria 6, 7 and 9 for curved outlines (slice 2), criterion 11's
//! heart budget (slice 2), criterion 11's Detach part (`0147` slice 3), criterion 1's create,
//! placement and refusals (ui-core acceptance file).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::cast_precision_loss
)]

use curvyo_document_core::Point;
use curvyo_extension_core::{
    Bound, ClosedPath, Contribution, Cubic, Edit, Extension, GenerateRequest, Generated, Mark,
    Param, ParamKind, ParamValue, Part, ScaleFactor, Seed, StoredParams,
};
use curvyo_generators_core::{GENERATORS, PATH_BOX, manifest};
use serde_json::{Map, Value, json};

type Pt = (f64, f64);

// ---------- outlines ----------

/// A polygon as straight edges, closed by construction.
fn polygon(points: &[Pt]) -> ClosedPath {
    let segments = (0..points.len())
        .map(|i| {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            Cubic {
                c1: Point::new(a.0, a.1),
                c2: Point::new(b.0, b.1),
                to: Point::new(b.0, b.1),
            }
        })
        .collect();
    let first = points[0];
    ClosedPath {
        start: Point::new(first.0, first.1),
        segments,
    }
}

/// The default-like coffin, 100 x 60 mm, six straight edges; its turns are 56.3 and 67.4 degrees.
const COFFIN: [Pt; 6] = [
    (0.0, 30.0),
    (20.0, 0.0),
    (80.0, 0.0),
    (100.0, 30.0),
    (80.0, 60.0),
    (20.0, 60.0),
];

/// A five-point star, outer radius 60 mm, inner radius 24 mm (ten corners, concave inner ones).
fn star() -> Vec<Pt> {
    (0..10)
        .map(|i| {
            let radius = if i % 2 == 0 { 60.0 } else { 24.0 };
            let angle = std::f64::consts::PI / 2.0 + f64::from(i) * std::f64::consts::PI / 5.0;
            (radius * angle.cos(), radius * angle.sin())
        })
        .collect()
}

/// A regular 20-gon of radius 50 mm: every turn is 18 degrees, kinks only at Corner angle 25.
fn twenty_gon() -> Vec<Pt> {
    (0..20)
        .map(|i| {
            let angle = f64::from(i) * std::f64::consts::TAU / 20.0;
            (50.0 * angle.cos(), 50.0 * angle.sin())
        })
        .collect()
}

/// A 20 degree spike: an isosceles triangle with its apex at the origin (interior angle 20).
fn spike() -> Vec<Pt> {
    let ang = 20.0_f64.to_radians();
    vec![
        (0.0, 0.0),
        (100.0, 0.0),
        (100.0 * ang.cos(), 100.0 * ang.sin()),
    ]
}

/// A rhombus with two 20 degree corners (opposite), so two sharp corners.
fn two_spikes() -> Vec<Pt> {
    let ang = 20.0_f64.to_radians();
    let (l, a) = (100.0, ang);
    let b = (l, 0.0);
    let d = (l * a.cos(), l * a.sin());
    vec![(0.0, 0.0), b, (b.0 + d.0, b.1 + d.1), d]
}

/// A bow tie: a self-intersecting outline (its two edges cross at the middle).
fn bow_tie() -> Vec<Pt> {
    vec![(0.0, 0.0), (60.0, 40.0), (60.0, 0.0), (0.0, 40.0)]
}

// ---------- parameter helpers (stored forms as the host writes them) ----------

fn mm(value: f64) -> Value {
    json!({ "mm": value })
}

fn deg(value: f64) -> Value {
    json!({ "deg": value })
}

fn switch(value: bool) -> Value {
    json!({ "on": value })
}

/// The stored values of a path box: the outline, plus `extra` (the other keys as the host writes
/// them, `{"mm": n}` etc.).
fn stored_with(path: &ClosedPath, extra: &[(&str, Value)]) -> StoredParams {
    let mut map = Map::new();
    map.insert(
        "outline".to_owned(),
        ParamValue::Path(path.clone()).to_stored(),
    );
    for (key, value) in extra {
        map.insert((*key).to_owned(), value.clone());
    }
    map
}

fn build(params: &StoredParams, edit: Option<Edit<'_>>) -> Generated {
    GENERATORS
        .generate(&GenerateRequest {
            kind: PATH_BOX,
            params,
            seed: Seed(7),
            edit,
        })
        .expect("the path box answers its own kind")
}

fn build_coffin(extra: &[(&str, Value)]) -> Generated {
    build(&stored_with(&polygon(&COFFIN), extra), None)
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

fn stored_outline(generated: &Generated) -> ClosedPath {
    generated
        .params
        .get("outline")
        .and_then(ParamValue::from_stored)
        .and_then(|value| match value {
            ParamValue::Path(path) => Some(path),
            _ => None,
        })
        .expect("the outline is stored under outline")
}

fn clamped(generated: &Generated, key: &str) -> Option<Bound> {
    generated
        .clamps
        .iter()
        .find(|clamp| clamp.key == key)
        .map(|clamp| clamp.bound)
}

fn warnings(generated: &Generated, id: &str) -> usize {
    generated.warnings.iter().filter(|w| w.id == id).count()
}

fn corner_marks(generated: &Generated) -> Vec<(Point, bool)> {
    generated
        .marks
        .on_footprint
        .iter()
        .filter_map(|mark| match mark {
            Mark::Corner { at, sharp, .. } => Some((*at, *sharp)),
            _ => None,
        })
        .collect()
}

fn bend_marks(generated: &Generated) -> Vec<bool> {
    generated
        .marks
        .on_footprint
        .iter()
        .filter_map(|mark| match mark {
            Mark::Bend { tight, .. } => Some(*tight),
            _ => None,
        })
        .collect()
}

/// The badges on the wall pieces: (number, middle point in the frame).
fn piece_badges(generated: &Generated) -> Vec<(u32, Pt)> {
    generated
        .marks
        .on_parts
        .iter()
        .filter_map(|mark| match mark {
            Mark::Piece { number, at, .. } => Some((*number, (at.x, at.y))),
            _ => None,
        })
        .collect()
}

// ---------- geometry of the outlines ----------

/// Samples one closed path: its end points, cubic curves in 16 steps.
fn sample(path: &ClosedPath) -> Vec<Pt> {
    const STEPS: usize = 16;
    let mut points = Vec::new();
    let mut from = path.start;
    for cubic in &path.segments {
        for step in 0..STEPS {
            let t = step as f64 / STEPS as f64;
            let u = 1.0 - t;
            let x = u * u * u * from.x
                + 3.0 * u * u * t * cubic.c1.x
                + 3.0 * u * t * t * cubic.c2.x
                + t * t * t * cubic.to.x;
            let y = u * u * u * from.y
                + 3.0 * u * u * t * cubic.c1.y
                + 3.0 * u * t * t * cubic.c2.y
                + t * t * t * cubic.to.y;
            points.push((x, y));
        }
        from = cubic.to;
    }
    points
}

fn shoelace(points: &[Pt]) -> f64 {
    let n = points.len();
    let mut twice = 0.0;
    for i in 0..n {
        let (x0, y0) = points[i];
        let (x1, y1) = points[(i + 1) % n];
        twice += x0 * y1 - x1 * y0;
    }
    twice / 2.0
}

fn bbox(points: &[Pt]) -> (f64, f64, f64, f64) {
    let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &(x, y) in points {
        b.0 = b.0.min(x);
        b.1 = b.1.min(y);
        b.2 = b.2.max(x);
        b.3 = b.3.max(y);
    }
    b
}

fn part_points(part: &Part) -> Vec<Pt> {
    part.paths.iter().flat_map(sample).collect()
}

/// The outer area of a part: its first path, holes not subtracted.
fn outer_area(part: &Part) -> f64 {
    shoelace(&sample(&part.paths[0])).abs()
}

/// The outline offset outward by `t` with miter joins, as a polygon (for a straight outline).
fn miter_offset(points: &[Pt], t: f64) -> Vec<Pt> {
    let n = points.len();
    let area = shoelace(points);
    // Outward is to the right of the travel direction for a counter-clockwise (y up) outline.
    let sign = if area > 0.0 { 1.0 } else { -1.0 };
    let lines: Vec<(Pt, Pt)> = (0..n)
        .map(|i| {
            let a = points[i];
            let b = points[(i + 1) % n];
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let len = dx.hypot(dy);
            let normal = (sign * dy / len, -sign * dx / len);
            ((a.0 + normal.0 * t, a.1 + normal.1 * t), (dx, dy))
        })
        .collect();
    (0..n)
        .map(|i| {
            let (p, d1) = lines[(i + n - 1) % n];
            let (q, d2) = lines[i];
            // Intersection of p + s d1 and q + u d2.
            let cross = d1.0 * d2.1 - d1.1 * d2.0;
            let s = ((q.0 - p.0) * d2.1 - (q.1 - p.1) * d2.0) / cross;
            (p.0 + s * d1.0, p.1 + s * d1.1)
        })
        .collect()
}

// ---------- criterion 1: the kind, the command, the default build, refusals at generator level ----------

fn path_box_kind() -> curvyo_extension_core::GeneratorKind {
    manifest()
        .contributions
        .into_iter()
        .find_map(|contribution| match contribution {
            Contribution::Generator(kind) if kind.id == PATH_BOX => Some(kind),
            _ => None,
        })
        .expect("the manifest declares the path box")
}

#[test]
fn criterion_1_the_path_box_is_a_kind_built_from_a_selected_outline_with_the_box_from_path_command()
{
    let kind = path_box_kind();
    assert_eq!(kind.label, "Path box", "the object is named Path box");
    let source = kind
        .source
        .as_ref()
        .expect("the kind is built from a selection");
    assert_eq!(
        source.param, "outline",
        "the outline is the outline parameter"
    );
    assert_eq!(source.command, "Box from path", "the command text");
    assert_eq!(
        kind.segment(),
        "Box from path",
        "the segment shows the command"
    );
}

#[test]
fn criterion_1_default_build_of_a_coffin_has_outlines_and_no_notice_or_clamp() {
    let generated = build(&StoredParams::new(), None);
    assert!(
        !generated.parts.is_empty(),
        "the default path box has outlines"
    );
    assert!(generated.warnings.is_empty(), "no notice at the defaults");
    assert!(
        generated.clamps.is_empty(),
        "nothing clamped at the defaults"
    );
}

#[test]
fn criterion_1_a_curved_cubic_is_built_as_its_chord_with_a_notice_and_the_outline_is_kept() {
    let mut segments = polygon(&COFFIN).segments;
    segments[0] = Cubic {
        c1: Point::new(0.0, 25.0),
        c2: Point::new(10.0, 0.0),
        to: Point::new(20.0, 0.0),
    };
    let curved = ClosedPath {
        start: Point::new(0.0, 30.0),
        segments,
    };
    let params = stored_with(&curved, &[]);
    let generated = build(&params, None);
    assert_eq!(
        warnings(&generated, "outline_curved"),
        1,
        "one outline_curved notice"
    );
    assert_eq!(
        stored_outline(&generated).flat(),
        curved.flat(),
        "the stored outline is not rewritten"
    );
}

#[test]
fn criterion_1_a_crossing_outline_is_replaced_by_the_default_with_a_notice() {
    let generated = build(&stored_with(&polygon(&bow_tie()), &[]), None);
    assert_eq!(
        warnings(&generated, "outline_crosses"),
        1,
        "one outline_crosses notice"
    );
}

#[test]
fn criterion_1_an_outline_too_small_for_three_mm_walls_is_clamped_and_kept() {
    // 10 x 10 mm: area 100 mm^2 is below (4 t)^2 = 144 mm^2 at the default 3 mm wall.
    let small = polygon(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
    let generated = build(&stored_with(&small, &[]), None);
    assert_eq!(
        clamped(&generated, "thickness"),
        Some(Bound::Dependent),
        "the wall thickness is lowered to the most the outline allows"
    );
    assert!(
        length_of(&generated, "thickness") <= 2.5 + 1e-9,
        "the wall is at most sqrt(100) / 4 = 2.5 mm"
    );
    assert_eq!(
        stored_outline(&generated).flat(),
        small.flat(),
        "the outline is never replaced on a size check"
    );
}

// ---------- criterion 2: the parameter block ----------

#[test]
fn criterion_2_the_block_is_the_rounded_box_block_plus_corner_angle_without_radius_or_wall_slits() {
    let kind = path_box_kind();
    let params: Vec<&Param> = kind.groups.iter().flat_map(|g| g.params.iter()).collect();
    let find = |key: &str| -> Option<&Param> { params.iter().copied().find(|p| p.key == key) };
    for (key, label) in [
        ("inner_height", "Inner height"),
        ("thickness", "Wall thickness"),
        ("kerf", "Kerf compensation"),
        ("slit_pattern", "Slit pattern"),
        ("slit_width", "Slit width"),
        ("slit_spacing", "Slit spacing"),
        ("slit_rows", "Slit rows"),
        ("gap", "Gap"),
        ("base_inset", "Base inset"),
        ("base_cover", "Base cover"),
        ("lid", "Lid"),
        ("lid_seat", "Lid seat"),
        ("finger_hollow", "Finger hollow"),
    ] {
        let found = find(key).unwrap_or_else(|| panic!("{key} is declared"));
        assert_eq!(found.label, label, "the label of {key}");
    }
    let corner = find("corner_angle").expect("Corner angle is declared");
    assert_eq!(corner.label, "Corner angle");
    assert!(
        matches!(corner.default, ParamValue::Angle(d) if d.0 == 25.0),
        "Corner angle defaults to 25 degrees"
    );
    match &corner.kind {
        ParamKind::Angle { min, max, .. } => {
            assert_eq!(min.0, 5.0, "Corner angle starts at 5 degrees");
            assert_eq!(max.0, 90.0, "Corner angle ends at 90 degrees");
        }
        other => panic!("Corner angle is an angle, got {other:?}"),
    }
    assert!(
        find("corner_radius").is_none(),
        "no Corner radius: the radii are the path's"
    );
    assert!(find("wall_slits").is_none(), "Wall slits is not offered");
    assert!(
        matches!(
            find("base_cover").map(|p| &p.default),
            Some(ParamValue::Switch(true))
        ),
        "Base cover is on by default"
    );
    let outline = find("outline").expect("the outline is a parameter");
    assert!(
        matches!(outline.kind, ParamKind::Path),
        "the outline is a path parameter"
    );
}

#[test]
fn criterion_2_corner_angle_outside_five_to_ninety_is_clamped_with_a_notice() {
    let high = build_coffin(&[("corner_angle", deg(120.0))]);
    assert_eq!(
        clamped(&high, "corner_angle"),
        Some(Bound::Maximum),
        "clamped to 90"
    );
    let low = build_coffin(&[("corner_angle", deg(2.0))]);
    assert_eq!(
        clamped(&low, "corner_angle"),
        Some(Bound::Minimum),
        "clamped to 5"
    );
}

#[test]
fn criterion_2_kerf_zero_and_point_two_build_without_a_notice() {
    for kerf in [0.0, 0.2] {
        let generated = build_coffin(&[("kerf", mm(kerf))]);
        assert!(!generated.parts.is_empty(), "kerf {kerf} builds");
        assert!(
            clamped(&generated, "kerf").is_none(),
            "kerf {kerf} is not clamped"
        );
        assert!(generated.warnings.is_empty(), "kerf {kerf} gives no notice");
    }
}

#[test]
fn criterion_2_the_automatic_slit_spacing_is_reported_as_a_positive_length() {
    let generated = build_coffin(&[]);
    let spacing = generated
        .automatic
        .iter()
        .find(|(key, _)| *key == "slit_spacing")
        .and_then(|(_, value)| value.clone().length())
        .expect("slit spacing is reported as automatic");
    assert!(
        spacing.as_mm() > 0.0,
        "the reported spacing is a positive gap"
    );
}

// ---------- criterion 3: corners and spans ----------

#[test]
fn criterion_3_a_coffin_has_six_corners_and_no_bent_span_at_the_default_angle() {
    let generated = build_coffin(&[]);
    assert_eq!(
        corner_marks(&generated).len(),
        6,
        "six corners, one per node"
    );
    assert!(
        bend_marks(&generated).is_empty(),
        "no curved span on a coffin"
    );
}

#[test]
fn criterion_3_a_triangle_has_three_corners_and_a_five_point_star_has_ten() {
    let triangle = build(
        &stored_with(&polygon(&[(0.0, 0.0), (90.0, 0.0), (40.0, 70.0)]), &[]),
        None,
    );
    assert_eq!(
        corner_marks(&triangle).len(),
        3,
        "a triangle has three corners"
    );
    let five = build(&stored_with(&polygon(&star()), &[]), None);
    assert_eq!(
        corner_marks(&five).len(),
        10,
        "a five-point star has ten corners"
    );
}

#[test]
fn criterion_3_a_coffin_at_corner_angle_ninety_has_no_corner_only_kinks() {
    let generated = build_coffin(&[("corner_angle", deg(90.0))]);
    assert!(
        corner_marks(&generated).is_empty(),
        "every turn is at most 90 degrees: no corner"
    );
    let bends = bend_marks(&generated);
    assert!(!bends.is_empty(), "the kinks are bent spans");
    assert!(
        bends.iter().all(|tight| !tight),
        "no kink is too tight to bend"
    );
}

// ---------- criterion 5: a sharp corner is built with a notice ----------

#[test]
fn criterion_5_a_twenty_degree_spike_is_built_anyway_with_one_sharp_corner_notice() {
    let generated = build(&stored_with(&polygon(&spike()), &[]), None);
    assert!(!generated.parts.is_empty(), "the spike is built anyway");
    assert_eq!(
        warnings(&generated, "sharp_corner"),
        1,
        "one sharp corner notice"
    );
    let warning = generated
        .warnings
        .iter()
        .find(|w| w.id == "sharp_corner")
        .expect("the notice");
    assert_eq!(
        warning.pieces.len(),
        2,
        "the notice names the two wall pieces of the corner"
    );
    match &warning.value {
        Some(ParamValue::Angle(angle)) => assert!(
            (angle.0 - 20.0).abs() <= 0.5,
            "the interior angle is 20 degrees, got {}",
            angle.0
        ),
        other => panic!("the notice carries the interior angle, got {other:?}"),
    }
    let sharp = corner_marks(&generated)
        .iter()
        .filter(|(_, sharp)| *sharp)
        .count();
    assert_eq!(sharp, 1, "the sharp corner carries the cross mark");
}

#[test]
fn criterion_5_two_sharp_corners_give_two_notices_and_no_refusal() {
    let generated = build(&stored_with(&polygon(&two_spikes()), &[]), None);
    assert!(!generated.parts.is_empty(), "the rhombus is built anyway");
    assert_eq!(
        warnings(&generated, "sharp_corner"),
        2,
        "two sharp corner notices"
    );
}

// ---------- criterion 8: the outline is a stored parameter; resize is proportional ----------

#[test]
fn criterion_8_the_box_keeps_its_own_copy_of_the_outline_as_stored() {
    let coffin = polygon(&COFFIN);
    let generated = build(&stored_with(&coffin, &[]), None);
    let kept = stored_outline(&generated);
    assert_eq!(
        kept.segments.len(),
        coffin.segments.len(),
        "the same number of edges"
    );
    let (a, b) = (bbox(&sample(&kept)), bbox(&sample(&coffin)));
    for (x, y) in [(a.0, b.0), (a.1, b.1), (a.2, b.2), (a.3, b.3)] {
        assert!((x - y).abs() < 1e-6, "the stored outline keeps its extent");
    }
}

#[test]
fn criterion_8_a_proportional_resize_scales_the_outline_and_height_and_keeps_thicknesses() {
    let base = build_coffin(&[
        ("inner_height", mm(50.0)),
        ("thickness", mm(3.0)),
        ("kerf", mm(0.2)),
    ]);
    let scaled = build(&base.params, Some(Edit::Scale(ScaleFactor(2.0))));
    let (before, after) = (
        bbox(&sample(&stored_outline(&base))),
        bbox(&sample(&stored_outline(&scaled))),
    );
    assert!(
        ((after.2 - after.0) - 2.0 * (before.2 - before.0)).abs() < 1e-6,
        "the outline's width doubles"
    );
    assert!(
        ((after.3 - after.1) - 2.0 * (before.3 - before.1)).abs() < 1e-6,
        "the outline's height doubles"
    );
    assert!(
        (length_of(&scaled, "inner_height") - 100.0).abs() < 1e-6,
        "the height doubles"
    );
    assert!(
        (length_of(&scaled, "thickness") - 3.0).abs() < 1e-9,
        "the wall thickness stays"
    );
    assert!(
        (length_of(&scaled, "kerf") - 0.2).abs() < 1e-9,
        "the kerf stays"
    );
}

// ---------- criterion 9 (slice 1 part): a loop with no corner is one strip ----------

#[test]
fn criterion_9_a_twenty_gon_at_twenty_five_degrees_is_one_strip_with_no_corner() {
    let generated = build(&stored_with(&polygon(&twenty_gon()), &[]), None);
    assert!(
        corner_marks(&generated).is_empty(),
        "every turn of 18 degrees is a kink"
    );
    // One wall piece carries no badge (ADR 0083 section 11: "The wall" instead of "Piece 1").
    assert!(
        piece_badges(&generated).is_empty(),
        "one wall strip: no piece badge"
    );
    // The spec's notices (sharp corner, too tight, outline) do not apply to kinks. The finger
    // hollow notice (`hollow_left_out`, ADR 0083 section 7) is not one of them and is not asserted.
    for id in [
        "sharp_corner",
        "tight_bend",
        "outline_crosses",
        "outline_curved",
    ] {
        assert_eq!(
            warnings(&generated, id),
            0,
            "no {id} notice for a loop of kinks"
        );
    }
}

// ---------- criterion 11: thread CPU time, parameter changes ----------

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
fn criterion_11_regenerating_the_coffin_after_a_change_costs_under_100_ms_of_thread_cpu() {
    let base = build_coffin(&[]);
    let started = thread_ticks();
    let changed = build(&base.params, Some(Edit::Param("inner_height")));
    let ended = thread_ticks();
    assert_ne!(changed.parts.len(), 0, "the regeneration has outlines");
    if let (Some(start), Some(end)) = (started, ended) {
        // Clock ticks are 10 ms each: anything under 100 ms reads as at most 10 ticks.
        assert!(
            (end - start) * 10 < 100 + 10,
            "the regeneration took {} ms of thread CPU",
            (end - start) * 10
        );
    }
}

// ---------- criterion 10: the plates follow the outline ----------

#[test]
fn criterion_10_the_inner_base_and_the_cover_follow_the_outline_and_the_wall() {
    let coffin = polygon(&COFFIN);
    let generated = build_coffin(&[("finger_hollow", switch(false)), ("thickness", mm(3.0))]);
    let wall_numbers: Vec<Pt> = piece_badges(&generated)
        .into_iter()
        .map(|(_, at)| at)
        .collect();
    let plates: Vec<&Part> = generated
        .parts
        .iter()
        .filter(|part| {
            let points = part_points(part);
            let (x0, y0, x1, y1) = bbox(&points);
            !wall_numbers
                .iter()
                .any(|&(x, y)| x >= x0 && x <= x1 && y >= y0 && y <= y1)
        })
        .collect();
    assert!(plates.len() >= 2, "an inner base and a cover at least");
    let outline_area = shoelace(&sample(&coffin)).abs();
    let cover_area = shoelace(&miter_offset(&COFFIN, 3.0)).abs();
    let close = |a: f64, b: f64| (a - b).abs() <= 0.05 * b;
    assert!(
        plates.iter().any(|p| close(outer_area(p), outline_area)),
        "a plate of the outline's own area (the inner base, within 5 percent for its tabs)"
    );
    assert!(
        plates.iter().any(|p| close(outer_area(p), cover_area)),
        "a plate of the outline offset outward by the wall (the cover, {cover_area:.1} mm2 within 5 percent): plate areas {:?}",
        plates
            .iter()
            .map(|p| outer_area(p).round())
            .collect::<Vec<_>>()
    );
}

// ---------- criterion 12: the flat layout ----------

#[test]
fn criterion_12_six_wall_pieces_for_a_coffin_are_numbered_in_order_along_the_row() {
    let generated = build_coffin(&[]);
    let mut badges = piece_badges(&generated);
    assert_eq!(badges.len(), 6, "six wall pieces, one badge each");
    badges.sort_by(|a, b| a.1.0.total_cmp(&b.1.0));
    let numbers: Vec<u32> = badges.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        numbers,
        [1, 2, 3, 4, 5, 6],
        "walk order from the outline's start"
    );
    let gap = length_of(&generated, "gap");
    let wall_parts: Vec<&Part> = generated
        .parts
        .iter()
        .filter(|part| {
            let (x0, y0, x1, y1) = bbox(&part_points(part));
            badges
                .iter()
                .any(|(_, (x, y))| *x >= x0 && *x <= x1 && *y >= y0 && *y <= y1)
        })
        .collect();
    assert_eq!(
        wall_parts.len(),
        6,
        "each wall piece is a part with its badge inside"
    );
    let mut boxes: Vec<(f64, f64, f64, f64)> = wall_parts
        .iter()
        .map(|part| bbox(&part_points(part)))
        .collect();
    boxes.sort_by(|a, b| a.0.total_cmp(&b.0));
    for pair in boxes.windows(2) {
        let (left, right) = (pair[0], pair[1]);
        assert!(
            left.2 <= right.0 + 1e-6,
            "wall pieces do not overlap in the row"
        );
        assert!(
            (right.0 - left.2 - gap).abs() <= 0.05,
            "wall pieces lie Gap apart, got {}",
            right.0 - left.2
        );
        assert!(
            (left.1 - right.1).abs() < 1e-6,
            "the pieces share their bottom edge"
        );
        assert!(
            (left.3 - right.3).abs() < 1e-6,
            "the pieces share their top edge"
        );
    }
}

#[test]
fn criterion_12_plates_lie_apart_from_the_row_and_no_two_parts_overlap() {
    let generated = build_coffin(&[]);
    let badges: Vec<Pt> = piece_badges(&generated)
        .into_iter()
        .map(|(_, at)| at)
        .collect();
    let boxes: Vec<(f64, f64, f64, f64)> = generated
        .parts
        .iter()
        .map(|part| bbox(&part_points(part)))
        .collect();
    let is_wall = |b: &(f64, f64, f64, f64)| {
        badges
            .iter()
            .any(|&(x, y)| x >= b.0 && x <= b.2 && y >= b.1 && y <= b.3)
    };
    let row: Vec<&(f64, f64, f64, f64)> = boxes.iter().filter(|b| is_wall(b)).collect();
    let plates: Vec<&(f64, f64, f64, f64)> = boxes.iter().filter(|b| !is_wall(b)).collect();
    assert_eq!(row.len(), 6, "six wall pieces in the row");
    assert!(plates.len() >= 2, "the plates are laid out too");
    let row_lo = row.iter().map(|b| b.1).fold(f64::MAX, f64::min);
    let row_hi = row.iter().map(|b| b.3).fold(f64::MIN, f64::max);
    for plate in &plates {
        assert!(
            plate.3 <= row_lo + 1e-6 || plate.1 >= row_hi - 1e-6,
            "a plate lies beside the wall row, not on it: {plate:?}"
        );
    }
    for (i, a) in boxes.iter().enumerate() {
        for b in boxes.iter().skip(i + 1) {
            let overlap_x = a.2.min(b.2) - a.0.max(b.0);
            let overlap_y = a.3.min(b.3) - a.1.max(b.1);
            assert!(
                overlap_x <= 1e-6 || overlap_y <= 1e-6,
                "two parts overlap: {a:?} and {b:?}"
            );
        }
    }
}
