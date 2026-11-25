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

/// Superseded by slice 2 (spec revision 7 criterion 1, ADR 0083 section 13.14): a curved cubic was
/// built as its chord with the notice `outline_curved`; it is now built as it is drawn and no
/// notice is sent. The outline is still kept as stored.
#[test]
fn criterion_1_a_curved_cubic_is_built_with_no_notice_and_the_outline_is_kept() {
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
        0,
        "no outline_curved notice from slice 2 on"
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

// ============================================================================
// Slice 2: curves (criteria 6, 7, 9, 10, 11, 13 to 16). Written before the slice-2 implementation
// was read. The outlines are drawn here from cubics (a heart, a shield, ellipses, circles and a
// circle with one 20 degree node), not read from fixtures. The strip model (neutral length with
// the flat tab windows, the notch centres, the uncut-band line test) is copied from
// `acceptance_0147.rs` (ADR 0082 sections 4 and 5), since the two crates' test files do not share
// helpers.
// ============================================================================

use std::f64::consts::PI;

fn cubic(c1: Pt, c2: Pt, to: Pt) -> Cubic {
    Cubic {
        c1: Point::new(c1.0, c1.1),
        c2: Point::new(c2.0, c2.1),
        to: Point::new(to.0, to.1),
    }
}

/// A straight edge as a cubic with its handles on the chord.
fn line_cubic(from: Pt, to: Pt) -> Cubic {
    let at = |f: f64| (from.0 + (to.0 - from.0) * f, from.1 + (to.1 - from.1) * f);
    cubic(at(1.0 / 3.0), at(2.0 / 3.0), to)
}

/// An ellipse of semi-axes `rx` and `ry` about the origin, as `segments` cubics (CCW, from (rx, 0)).
fn ellipse_cubics(rx: f64, ry: f64, segments: usize) -> ClosedPath {
    let theta = 2.0 * PI / segments as f64;
    let k = 4.0 / 3.0 * (theta / 4.0).tan();
    let point = |i: usize| {
        let a = (i % segments) as f64 * theta;
        (rx * a.cos(), ry * a.sin())
    };
    let tangent = |i: usize| {
        let a = (i % segments) as f64 * theta;
        (-rx * a.sin(), ry * a.cos())
    };
    let curves = (0..segments)
        .map(|i| {
            let (p0, p1) = (point(i), point(i + 1));
            let (t0, t1) = (tangent(i), tangent(i + 1));
            cubic(
                (p0.0 + k * t0.0, p0.1 + k * t0.1),
                (p1.0 - k * t1.0, p1.1 - k * t1.1),
                p1,
            )
        })
        .collect();
    ClosedPath {
        start: Point::new(rx, 0.0),
        segments: curves,
    }
}

/// A circle of four cubics with one node turned by 20 degrees (its first handle rotated).
fn kinked_circle() -> ClosedPath {
    let mut path = ellipse_cubics(30.0, 30.0, 4);
    let start = path.start;
    let (vx, vy) = (
        path.segments[0].c1.x - start.x,
        path.segments[0].c1.y - start.y,
    );
    let (s, c) = 20.0_f64.to_radians().sin_cos();
    path.segments[0].c1 = Point::new(start.x + vx * c - vy * s, start.y + vx * s + vy * c);
    path
}

/// A heart: two smooth lobes meeting at a notch (top, turn 100 degrees) and a tip (bottom, 120).
fn heart() -> ClosedPath {
    ClosedPath {
        start: Point::new(0.0, 20.0),
        segments: vec![
            cubic((10.0, 32.0), (30.0, 28.0), (30.0, 10.0)),
            cubic((30.0, -8.0), (6.0, -29.6), (0.0, -40.0)),
            cubic((-6.0, -29.6), (-30.0, -8.0), (-30.0, 10.0)),
            cubic((-30.0, 28.0), (-10.0, 32.0), (0.0, 20.0)),
        ],
    }
}

/// A shield: a flat top, two straight sides and a bottom of two cubics, tangent at every join
/// but the two top corners (turn 90 degrees).
fn shield() -> ClosedPath {
    ClosedPath {
        start: Point::new(-40.0, 60.0),
        segments: vec![
            line_cubic((-40.0, 60.0), (40.0, 60.0)),
            line_cubic((40.0, 60.0), (40.0, 0.0)),
            cubic((40.0, -22.0), (22.0, -40.0), (0.0, -40.0)),
            cubic((-22.0, -40.0), (-40.0, -22.0), (-40.0, 0.0)),
            line_cubic((-40.0, 0.0), (-40.0, 60.0)),
        ],
    }
}

/// A rectangle 100 x 60 mm whose top edge has a narrow V notch (1 mm half width, 2 mm deep, an
/// apex angle of about 53 degrees).
const NOTCHED: [Pt; 7] = [
    (0.0, 0.0),
    (100.0, 0.0),
    (100.0, 60.0),
    (51.0, 60.0),
    (50.0, 58.0),
    (49.0, 60.0),
    (0.0, 60.0),
];

/// Points of the cubics from `start`, `steps` per cubic, each cubic's end left out.
fn sample_from(start: Point, cubics: &[Cubic], steps: usize) -> Vec<Pt> {
    let mut points = Vec::new();
    let mut from = start;
    for cubic in cubics {
        for step in 0..steps {
            let t = step as f64 / steps as f64;
            let u = 1.0 - t;
            points.push((
                u * u * u * from.x
                    + 3.0 * u * u * t * cubic.c1.x
                    + 3.0 * u * t * t * cubic.c2.x
                    + t * t * t * cubic.to.x,
                u * u * u * from.y
                    + 3.0 * u * u * t * cubic.c1.y
                    + 3.0 * u * t * t * cubic.c2.y
                    + t * t * t * cubic.to.y,
            ));
        }
        from = cubic.to;
    }
    points
}

/// A closed outline as 256 points per cubic (for lengths and distances).
fn sample_dense(path: &ClosedPath) -> Vec<Pt> {
    sample_from(path.start, &path.segments, 256)
}

/// The distance from `p` to the closed polyline `poly`.
fn distance_to_polyline(p: Pt, poly: &[Pt]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let len2 = dx * dx + dy * dy;
            let t = if len2 == 0.0 {
                0.0
            } else {
                (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
            };
            (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
        })
        .fold(f64::MAX, f64::min)
}

/// Stored values for a curved outline: wall height 30 mm, wall thickness 3 mm, the rest default.
fn curve_params(path: &ClosedPath) -> StoredParams {
    stored_with(path, &[("inner_height", mm(30.0)), ("thickness", mm(3.0))])
}

/// A value the generator used: the automatic one, or the typed one (as `0147` uses it).
fn used(generated: &Generated, key: &str) -> f64 {
    generated
        .automatic
        .iter()
        .find(|(k, _)| *k == key)
        .and_then(|(_, value)| value.clone().length())
        .map_or_else(
            || length_of(generated, key),
            curvyo_document_core::Length::as_mm,
        )
}

/// The strip height the wall must have: inside height plus the cover and the lid plate.
fn wall_height(generated: &Generated) -> f64 {
    length_of(generated, "inner_height")
        + used(generated, "cover_thickness")
        + used(generated, "lid_thickness")
}

fn part_width(part: &Part) -> f64 {
    let (x0, _, x1, _) = bbox(&part_points(part));
    x1 - x0
}

fn part_height(part: &Part) -> f64 {
    let (_, y0, _, y1) = bbox(&part_points(part));
    y1 - y0
}

/// The widest part: the wall strip of an outline without corners (plates are narrower).
fn widest_part(generated: &Generated) -> &Part {
    generated
        .parts
        .iter()
        .max_by(|a, b| part_width(a).total_cmp(&part_width(b)))
        .expect("the box has parts")
}

/// The wall pieces: the parts with a piece badge inside their bounding box.
fn wall_parts(generated: &Generated) -> Vec<&Part> {
    let badges: Vec<Pt> = piece_badges(generated)
        .into_iter()
        .map(|(_, at)| at)
        .collect();
    generated
        .parts
        .iter()
        .filter(|part| {
            let (x0, y0, x1, y1) = bbox(&part_points(part));
            badges
                .iter()
                .any(|&(x, y)| x >= x0 && x <= x1 && y >= y0 && y <= y1)
        })
        .collect()
}

fn auto_spacing(generated: &Generated) -> f64 {
    generated
        .automatic
        .iter()
        .find(|(key, _)| *key == "slit_spacing")
        .and_then(|(_, value)| value.clone().length())
        .expect("slit spacing is reported as automatic")
        .as_mm()
}

fn value_mm(value: Option<ParamValue>) -> Option<f64> {
    value
        .and_then(|v| v.length())
        .map(curvyo_document_core::Length::as_mm)
}

fn assert_no_overlap(generated: &Generated) {
    let boxes: Vec<(f64, f64, f64, f64)> = generated
        .parts
        .iter()
        .map(|part| bbox(&part_points(part)))
        .collect();
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

/// Every wall piece is the wall height (inside height plus cover and lid) within 0.5 mm.
fn assert_pieces_at_wall_height(generated: &Generated) {
    let height = wall_height(generated);
    let pieces = wall_parts(generated);
    assert_eq!(
        pieces.len(),
        piece_badges(generated).len(),
        "every badge sits on a wall piece"
    );
    for part in pieces {
        assert!(
            (part_height(part) - height).abs() <= 0.5,
            "a wall piece is {} mm high, the wall height is {height} mm",
            part_height(part)
        );
    }
}

/// The strip in its own frame, as `0147` maps it: `u` along the long axis from its minimum, `v`
/// across the wall (`v = 0` the base edge, `v = height` the ring edge).
struct Strip {
    paths: Vec<Vec<Pt>>,
    length: f64,
    height: f64,
}

fn strip_of_part(part: &Part, height: f64) -> Strip {
    let paths: Vec<Vec<Pt>> = part.paths.iter().map(sample).collect();
    let flat: Vec<Pt> = paths.iter().flatten().copied().collect();
    let (x0, y0, x1, y1) = bbox(&flat);
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

/// The neutral line of a rounded box (ADR 0082 section 4), copied from `0147`.
fn neutral_length(w: f64, d: f64, r: f64, t: f64) -> f64 {
    2.0 * (w - 2.0 * r) + 2.0 * (d - 2.0 * r) + PI * (2.0 * r + t)
}

/// The expected length with `windows` flat tab windows on bent spans (copied from `0147`, criterion
/// 3): `neutral - N (r theta - l)`, `l = 2 t_p + 2 t_w`, `theta = 2 asin(l / (2 r))`.
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

/// The centres `u` of the tab notches at one edge of the strip (copied from `0147`): flat runs at
/// the notch bottom `v = level`, each of the length of a tab (`2 t_p`), counted once per run.
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

/// The bent runs of a strip (copied from the idea of `0147`'s windows): the length minus the
/// margin at both ends (the solid seam or joint margin, `2 t`) and minus each window of half
/// length `half` around a tab centre.
fn bent_runs(length: f64, centres: &[f64], half: f64, margin: f64) -> Vec<(f64, f64)> {
    let mut cuts: Vec<(f64, f64)> = centres.iter().map(|&c| (c - half, c + half)).collect();
    cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut runs = Vec::new();
    let mut cursor = margin;
    for (a, b) in cuts {
        if a > cursor {
            runs.push((cursor, a.min(length - margin)));
        }
        cursor = cursor.max(b);
    }
    if cursor < length - margin {
        runs.push((cursor, length - margin));
    }
    runs.retain(|&(a, b)| b > a);
    runs
}

/// Whether the horizontal line `v` crosses an edge of the strip with `u` inside `[a, b]` (copied
/// from `0147`: a slot or an edge notch cuts the line there).
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

/// The number of uncut height lines across the given bent runs (copied from `0147`), and the first.
fn uncut_bands(strip: &Strip, runs: &[(f64, f64)]) -> (usize, Option<(f64, f64)>) {
    let mut failures = 0;
    let mut first = None;
    let mut v = 0.05;
    while v < strip.height - 0.05 {
        for &(a, b) in runs {
            if !line_is_cut(strip, v, a, b) {
                failures += 1;
                first.get_or_insert((v, a));
            }
        }
        v += 0.1;
    }
    (failures, first)
}

/// Half the length of a tab window: `l / 2` with `l = 2 t_p + 2 t_w`.
fn half_window(t_p: f64, t_w: f64) -> f64 {
    t_p + t_w
}

/// The wall thickness and the plate thickness of a wall.
fn wall_context(generated: &Generated) -> (f64, f64) {
    (
        length_of(generated, "thickness"),
        used(generated, "plate_thickness"),
    )
}

// ---------- criterion 13: heart, shield, ellipse and circle build ----------

#[test]
fn criterion_13_a_heart_builds_with_two_joints_two_pieces_and_two_bent_spans() {
    let generated = build(&curve_params(&heart()), None);
    assert!(
        !generated.parts.is_empty(),
        "the heart is built, no refusal"
    );
    assert_eq!(
        warnings(&generated, "outline_curved"),
        0,
        "no curve refusal"
    );
    assert_eq!(
        corner_marks(&generated).len(),
        2,
        "the notch and the tip are joints"
    );
    assert_eq!(
        piece_badges(&generated).len(),
        2,
        "two wall pieces, one per side of the heart"
    );
    assert!(bend_marks(&generated).len() >= 2, "two bent spans");
    assert_pieces_at_wall_height(&generated);
    assert_no_overlap(&generated);
}

#[test]
fn criterion_13_a_shield_builds_as_two_pieces_between_two_corners() {
    let generated = build(&curve_params(&shield()), None);
    assert!(
        !generated.parts.is_empty(),
        "the shield is built, no refusal"
    );
    assert_eq!(
        corner_marks(&generated).len(),
        2,
        "the two top corners are joints, the sides meet the bottom tangentially"
    );
    assert_eq!(
        piece_badges(&generated).len(),
        2,
        "the flat top and the piece through the sides and the bottom"
    );
    assert!(
        !bend_marks(&generated).is_empty(),
        "the bottom is a bent span"
    );
    assert_pieces_at_wall_height(&generated);
    assert_no_overlap(&generated);
}

#[test]
fn criterion_13_an_ellipse_and_a_circle_build_as_one_strip_with_no_corner() {
    for outline in [ellipse_cubics(40.0, 25.0, 4), ellipse_cubics(30.0, 30.0, 4)] {
        let generated = build(&curve_params(&outline), None);
        assert!(!generated.parts.is_empty(), "built, no refusal");
        assert!(corner_marks(&generated).is_empty(), "no corner");
        assert!(
            piece_badges(&generated).is_empty(),
            "one strip: no piece badge"
        );
        assert!(!bend_marks(&generated).is_empty(), "the strip is bent");
        assert!(
            (part_height(widest_part(&generated)) - wall_height(&generated)).abs() <= 0.5,
            "the strip is the inside height plus cover and lid, within 0.5 mm"
        );
        assert_no_overlap(&generated);
    }
}

// ---------- criterion 14: the arc fit and the junctions ----------

#[test]
fn criterion_14_every_bent_span_lies_within_0_01_mm_of_its_drawn_curve() {
    for path in [
        heart(),
        shield(),
        ellipse_cubics(40.0, 25.0, 4),
        ellipse_cubics(30.0, 30.0, 4),
    ] {
        let generated = build(&curve_params(&path), None);
        let dense = sample_dense(&path);
        let mut checked = 0;
        for mark in &generated.marks.on_footprint {
            if let Mark::Bend {
                start, segments, ..
            } = mark
            {
                for point in sample_from(*start, segments, 32) {
                    let off = distance_to_polyline(point, &dense);
                    assert!(
                        off <= 0.011,
                        "a bent span point is {off:.4} mm off the drawn curve"
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 0, "the bent spans are marked and sampled");
    }
}

#[test]
fn criterion_14_a_circle_of_four_cubics_and_one_of_eight_give_the_same_strip_and_slit_count() {
    let four = build(&curve_params(&ellipse_cubics(30.0, 30.0, 4)), None);
    let eight = build(&curve_params(&ellipse_cubics(30.0, 30.0, 8)), None);
    let (a, b) = (widest_part(&four), widest_part(&eight));
    assert!(
        (part_width(a) - part_width(b)).abs() <= 0.1,
        "the strip lengths agree within 0.1 mm: {} and {}",
        part_width(a),
        part_width(b)
    );
    assert_eq!(
        a.paths.len(),
        b.paths.len(),
        "the same number of slit paths in the strip"
    );
    assert!(corner_marks(&four).is_empty() && corner_marks(&eight).is_empty());
}

// ---------- criterion 6: the strip length and the slit spacing ----------

#[test]
fn criterion_6_a_circle_strip_is_its_neutral_line_less_the_flat_tab_windows() {
    // ADR 0082 section 4 as `0147` criterion 3: each flat tab window on the bent run replaces its
    // arc by its chord; the windows are the tab notches of the base row.
    let circle = ellipse_cubics(30.0, 30.0, 4);
    let generated = build(&curve_params(&circle), None);
    let (t_w, t_p) = wall_context(&generated);
    let strip = strip_of_part(widest_part(&generated), wall_height(&generated));
    let windows = notch_centres(&strip, t_w, t_p).len();
    let expected = expected_strip_length(60.0, 60.0, 30.0, t_w, t_p, windows);
    assert!(
        (strip.length - expected).abs() <= 0.1,
        "the strip is {:.3} mm, expected {expected:.3} mm with {windows} windows",
        strip.length
    );
}

#[test]
fn criterion_6_the_reported_slit_spacing_is_the_smallest_gap_within_the_automatic_bound() {
    // The reported value is the smallest gap used (spec revision 7, criterion 2). Columns sit at
    // equal steps with N = max(2, floor(integral ds / g)), so a gap is L / N >= g, slightly above
    // g = 0.22 r; N is not observable, so the upper bound is taken as twice the bound.
    for radius in [15.0, 60.0] {
        let generated = build(&curve_params(&ellipse_cubics(radius, radius, 4)), None);
        let reported = auto_spacing(&generated);
        let slit_width = used(&generated, "slit_width");
        let bound = (0.22 * radius).max(slit_width + 1.0);
        // Short stretches between windows still carry 2 columns, so the smallest gap may be well
        // below 0.22 r; only the gap rule is hard (final ruling).
        assert!(
            reported >= slit_width + 1.0 - 1e-6,
            "radius {radius}: reported {reported:.3} mm is below the slit width plus 1 mm"
        );
        assert!(
            reported <= 2.0 * bound + 1e-6,
            "radius {radius}: reported {reported:.3} mm is above twice the bound {bound:.3} mm"
        );
    }
}

// ---------- criterion 7: a radius below 2 t is built with a notice ----------

#[test]
fn criterion_7_a_radius_below_twice_the_wall_is_built_with_a_tight_bend_notice_and_no_clamp() {
    // An ellipse 120 x 16 mm: its ends have a radius of 16 squared over 120, about 1.07 mm.
    let thin = ellipse_cubics(60.0, 8.0, 4);
    let generated = build(&curve_params(&thin), None);
    assert!(!generated.parts.is_empty(), "built anyway, no refusal");
    assert!(
        warnings(&generated, "tight_bend") >= 1,
        "a tight_bend notice"
    );
    let notice = generated
        .warnings
        .iter()
        .find(|w| w.id == "tight_bend")
        .expect("the notice");
    let radius = value_mm(notice.value.clone()).expect("the notice carries the radius");
    assert!(
        (radius - 64.0 / 60.0).abs() <= 0.05,
        "the smallest radius is about 1.07 mm, got {radius:.3}"
    );
    let limit = value_mm(notice.limit.clone()).expect("the notice carries the limit");
    assert!((limit - 6.0).abs() <= 1e-6, "the limit is 2 t = 6 mm");
    assert!(
        clamped(&generated, "thickness").is_none(),
        "no value clamped"
    );
    assert_eq!(
        stored_outline(&generated).flat(),
        thin.flat(),
        "the path is not changed"
    );
    let round = build(&curve_params(&ellipse_cubics(30.0, 30.0, 4)), None);
    assert_eq!(
        warnings(&round, "tight_bend"),
        0,
        "a circle of 30 mm has no tight bend"
    );
}

// ---------- criterion 9: the seam of a smooth outline ----------

#[test]
fn criterion_9_a_circle_or_ellipse_has_one_strip_and_its_seam_at_the_least_y_point() {
    for (outline, expected_y) in [
        (ellipse_cubics(30.0, 30.0, 4), -30.0),
        (ellipse_cubics(40.0, 25.0, 4), -25.0),
        (ellipse_cubics(25.0, 40.0, 4), -40.0),
    ] {
        let generated = build(&curve_params(&outline), None);
        let seams: Vec<Point> = generated
            .marks
            .on_footprint
            .iter()
            .chain(generated.marks.on_parts.iter())
            .filter_map(|mark| match mark {
                Mark::Seam { at, .. } => Some(*at),
                _ => None,
            })
            .collect();
        assert_eq!(seams.len(), 1, "one seam mark");
        assert!(
            seams[0].x.abs() <= 0.01 && (seams[0].y - expected_y).abs() <= 0.01,
            "the seam at ({}, {}), expected (0, {expected_y})",
            seams[0].x,
            seams[0].y
        );
        assert!(piece_badges(&generated).is_empty(), "one strip");
    }
}

#[test]
fn criterion_9_no_slit_lies_within_2_t_of_the_seam_on_a_circle_or_an_ellipse() {
    for outline in [
        ellipse_cubics(30.0, 30.0, 4),
        ellipse_cubics(40.0, 25.0, 4),
        ellipse_cubics(25.0, 40.0, 4),
    ] {
        let generated = build(&curve_params(&outline), None);
        let t = length_of(&generated, "thickness");
        let strip = strip_of_part(widest_part(&generated), wall_height(&generated));
        // The seam is the butt at both ends of the strip (u = 0 and u = length); an interior
        // point of any outline (a slot, an edge notch) within 2 t of either end is a slit there.
        let mut hits = Vec::new();
        for path in &strip.paths {
            for &(u, v) in path {
                let interior = v > 0.01 && v < strip.height - 0.01;
                let near = (u > 1e-3 && u < 2.0 * t)
                    || (u > strip.length - 2.0 * t && u < strip.length - 1e-3);
                if interior && near {
                    hits.push((u, v));
                }
            }
        }
        assert!(
            hits.is_empty(),
            "slit points within 2 t of the seam: {hits:?}"
        );
    }
}

#[test]
fn criterion_9_6_no_uncut_band_across_a_bent_run_of_the_circle() {
    // The circle is one bent run; the seam margin (2 t at both butt ends) and the tab windows are
    // the only places without a slit (ADR 0083 section 6, ADR 0082 section 5).
    let generated = build(&curve_params(&ellipse_cubics(30.0, 30.0, 4)), None);
    let (t_w, t_p) = wall_context(&generated);
    let strip = strip_of_part(widest_part(&generated), wall_height(&generated));
    let centres = notch_centres(&strip, t_w, t_p);
    let runs = bent_runs(strip.length, &centres, half_window(t_p, t_w), 2.0 * t_w);
    assert!(!runs.is_empty(), "the circle has bent runs");
    let (failures, first) = uncut_bands(&strip, &runs);
    assert_eq!(
        failures, 0,
        "an uncut band across a bent run, first at {first:?}"
    );
}

#[test]
fn criterion_6_no_uncut_band_across_a_bent_piece_of_the_heart() {
    // Every heart wall piece is one lobe, bent from one corner to the next; its ends carry the
    // solid margin of 2 t and its tab windows are the notches of the base row.
    let generated = build(&curve_params(&heart()), None);
    let (t_w, t_p) = wall_context(&generated);
    let height = wall_height(&generated);
    let pieces = wall_parts(&generated);
    assert_eq!(pieces.len(), 2, "two lobes");
    for part in pieces {
        let strip = strip_of_part(part, height);
        let centres = notch_centres(&strip, t_w, t_p);
        let runs = bent_runs(strip.length, &centres, half_window(t_p, t_w), 2.0 * t_w);
        let (failures, first) = uncut_bands(&strip, &runs);
        assert_eq!(
            failures, 0,
            "an uncut band across a heart lobe, first at {first:?}"
        );
    }
}

// ---------- criterion 10: the plates follow the outline; the hollow on the lid ----------

#[test]
fn criterion_10_a_circle_cover_and_base_follow_the_outline_offset_by_the_wall() {
    let generated = build(&curve_params(&ellipse_cubics(30.0, 30.0, 4)), None);
    let strip_width = part_width(widest_part(&generated));
    let plates: Vec<&Part> = generated
        .parts
        .iter()
        .filter(|part| part_width(part) < 0.5 * strip_width)
        .collect();
    assert!(plates.len() >= 2, "an inner base and a cover at least");
    let base_area = PI * 30.0 * 30.0;
    let cover_area = PI * 33.0 * 33.0;
    let close = |a: f64, b: f64| (a - b).abs() <= 0.05 * b;
    assert!(
        plates.iter().any(|p| close(outer_area(p), base_area)),
        "a plate of the outline's area {base_area:.1} mm2"
    );
    assert!(
        plates.iter().any(|p| close(outer_area(p), cover_area)),
        "a plate of the outline offset outward by the wall, {cover_area:.1} mm2"
    );
    assert_eq!(
        warnings(&generated, "hollow_left_out"),
        0,
        "the hollow is placed on the circle"
    );
}

#[test]
fn criterion_10_the_default_heart_keeps_its_exact_cover_and_its_finger_hollow() {
    // Spec revision 8 (32fc30e2): the default heart's notch (about 100 degrees) is not narrow.
    let generated = build(&curve_params(&heart()), None);
    assert_eq!(
        warnings(&generated, "hollow_left_out"),
        0,
        "the heart gets the hollow on its longest convex curve"
    );
    assert_eq!(
        warnings(&generated, "cover_rounded"),
        0,
        "the heart's cover is the exact offset"
    );
}

// ---------- criterion 11: the heart's edit budget (thread CPU time) ----------

#[cfg(target_os = "linux")]
#[test]
fn criterion_11_a_heart_edit_regenerates_within_300_ms_of_thread_cpu() {
    let base = build(&curve_params(&heart()), None);
    let started = thread_ticks();
    let changed = build(&base.params, Some(Edit::Param("inner_height")));
    let ended = thread_ticks();
    assert!(!changed.parts.is_empty(), "the regeneration has parts");
    if let (Some(start), Some(end)) = (started, ended) {
        // Clock ticks are 10 ms each: anything under 300 ms reads as at most 30 ticks.
        assert!(
            (end - start) * 10 < 300 + 10,
            "the regeneration took {} ms of thread CPU",
            (end - start) * 10
        );
    }
}

// ---------- criterion 15: Corner angle on curved outlines ----------

#[test]
fn criterion_15_the_heart_keeps_its_two_joints_at_corner_angles_5_25_and_90() {
    for angle in [5.0, 25.0, 90.0] {
        let generated = build(
            &stored_with(
                &heart(),
                &[
                    ("inner_height", mm(30.0)),
                    ("thickness", mm(3.0)),
                    ("corner_angle", deg(angle)),
                ],
            ),
            None,
        );
        assert_eq!(
            corner_marks(&generated).len(),
            2,
            "the notch and the tip are joints at Corner angle {angle}"
        );
        assert!(
            bend_marks(&generated).len() >= 2,
            "the lobes are bent spans at Corner angle {angle}"
        );
    }
}

#[test]
fn criterion_15_a_node_turning_by_20_degrees_is_a_corner_at_5_and_a_kink_at_25_and_90() {
    let outline = kinked_circle();
    let at = |angle: f64| {
        build(
            &stored_with(
                &outline,
                &[
                    ("inner_height", mm(30.0)),
                    ("thickness", mm(3.0)),
                    ("corner_angle", deg(angle)),
                ],
            ),
            None,
        )
    };
    let five = at(5.0);
    assert_eq!(
        corner_marks(&five).len(),
        1,
        "a 20 degree turn is a corner at 5 degrees"
    );
    for angle in [25.0, 90.0] {
        let generated = at(angle);
        assert!(
            corner_marks(&generated).is_empty(),
            "no corner at {angle} degrees"
        );
        assert!(
            !bend_marks(&generated).is_empty(),
            "a kink, built as a bent span, at {angle}"
        );
    }
}

// ---------- criterion 16: the narrow V notch falls back to a rounded cover ----------

#[test]
fn criterion_16_a_narrow_v_notch_gives_the_cover_notice_and_no_finger_hollow() {
    let outline = polygon(&NOTCHED);
    let generated = build(&stored_with(&outline, &[("thickness", mm(3.0))]), None);
    assert!(!generated.parts.is_empty(), "built anyway, no refusal");
    assert_eq!(
        warnings(&generated, "cover_rounded"),
        1,
        "the cover is rounded at the notch"
    );
    assert_eq!(
        warnings(&generated, "hollow_left_out"),
        0,
        "the rounded cover carries no finger hollow notice"
    );
    assert_eq!(
        stored_outline(&generated).flat(),
        outline.flat(),
        "the path is not changed"
    );
}
