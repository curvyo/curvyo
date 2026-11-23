//! Acceptance tests of `specs/0141-parametric-generators`, criteria 2 to 5 and 7 to 8, through the
//! public generator contract (ADR 0079 §1 to §3). Black-box: outlines and stored values only.
//! Criteria 1 and 6 and the document, sketch and UI layers are covered outside this crate.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use curvyo_extension_core::{
    Bound, ClosedPath, Edit, Extension, GenerateRequest, Generated, ParamValue, ScaleFactor, Seed,
    StoredParams,
};
use curvyo_generators_core::{BOX, GEAR, GENERATORS, PROCESS_ID, manifest};
use serde_json::{Map, Value, json};
use std::f64::consts::PI;

fn mm(value: f64) -> Value {
    json!({ "mm": value })
}

fn count(value: u32) -> Value {
    json!({ "count": value })
}

fn on(value: bool) -> Value {
    json!({ "on": value })
}

fn choice(id: &str) -> Value {
    json!({ "choice": id })
}

fn params(pairs: &[(&str, Value)]) -> StoredParams {
    let mut map = Map::new();
    for (key, value) in pairs {
        map.insert((*key).to_owned(), value.clone());
    }
    map
}

fn generate(kind: &str, stored: &StoredParams, edit: Option<Edit<'_>>) -> Generated {
    GENERATORS
        .generate(&GenerateRequest {
            kind,
            params: stored,
            seed: Seed(7),
            edit,
        })
        .expect("the generator answers its own kind")
}

fn length(generated: &Generated, key: &str) -> f64 {
    generated
        .params
        .get(key)
        .and_then(ParamValue::from_stored)
        .and_then(|value| value.length())
        .unwrap_or_else(|| panic!("no length stored under {key}"))
        .as_mm()
}

fn count_of(generated: &Generated, key: &str) -> u32 {
    generated
        .params
        .get(key)
        .and_then(ParamValue::from_stored)
        .and_then(|value| value.count())
        .unwrap_or_else(|| panic!("no count stored under {key}"))
}

fn assert_close(actual: f64, expected: f64, tolerance: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{what}: expected {expected} within {tolerance}, got {actual}"
    );
}

fn assert_all_closed(generated: &Generated) {
    for part in &generated.parts {
        for path in &part.paths {
            let last = path.segments.last().expect("an outline has segments");
            assert!(
                (last.to.x - path.start.x).abs() < 1e-9 && (last.to.y - path.start.y).abs() < 1e-9,
                "every outline is closed by construction"
            );
        }
    }
}

// ---------- outline sampling and the exact involute ----------

fn sample_outline(path: &ClosedPath) -> Vec<(f64, f64)> {
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

/// The exact involute flanks of a standard tooth (zero backlash, no profile shift), sampled on a
/// fine grid of the involute parameter `t`. Each entry is (radius, x, y) in the tooth frame where
/// the tooth centre is at angle 0; the right flank is first, the left flank second.
struct FlankGrid {
    right: Vec<(f64, f64, f64)>,
    left: Vec<(f64, f64, f64)>,
    teeth: u32,
}

fn flank_grid(module: f64, teeth: u32, pressure_angle_deg: f64, outer_radius: f64) -> FlankGrid {
    const N: usize = 40_000;
    let alpha = pressure_angle_deg.to_radians();
    let pitch_radius = module * f64::from(teeth) / 2.0;
    let base_radius = pitch_radius * alpha.cos();
    let psi_pitch = alpha.tan() - alpha;
    let half_tooth = PI / (2.0 * f64::from(teeth));
    let t_max = (((outer_radius + module) / base_radius).powi(2) - 1.0).sqrt();
    let mut right = Vec::with_capacity(N + 1);
    let mut left = Vec::with_capacity(N + 1);
    for i in 0..=N {
        let t = t_max * f64::from(u32::try_from(i).unwrap()) / f64::from(u32::try_from(N).unwrap());
        let psi = t - t.atan();
        let radius = base_radius * (1.0 + t * t).sqrt();
        let phi_right = -half_tooth - psi_pitch + psi;
        let phi_left = half_tooth + psi_pitch - psi;
        right.push((radius, radius * phi_right.cos(), radius * phi_right.sin()));
        left.push((radius, radius * phi_left.cos(), radius * phi_left.sin()));
    }
    FlankGrid { right, left, teeth }
}

/// The distance from `(x, y)` to the nearest sampled flank point at about the same radius, over
/// the neighbouring tooth positions. Upper bound of the true distance to the exact involute: the
/// grid is 40 000 steps, and a point within 0.01 mm of the curve has its nearest grid point within
/// the search window.
fn distance_to_involute(grid: &FlankGrid, x: f64, y: f64, phase: f64) -> f64 {
    let rho = x.hypot(y);
    let step = 2.0 * PI / f64::from(grid.teeth);
    let raw = y.atan2(x) - phase;
    let theta = raw - step * (raw / step).round();
    let mut best = f64::INFINITY;
    for shift in [-1.0, 0.0, 1.0] {
        let phi = theta + shift * step;
        let (px, py) = (rho * phi.cos(), rho * phi.sin());
        for flank in [&grid.right, &grid.left] {
            let index = flank.partition_point(|entry| entry.0 < rho);
            let from = index.saturating_sub(400);
            let to = (index + 400).min(flank.len());
            for &(_, cx, cy) in &flank[from..to] {
                let d = (cx - px).hypot(cy - py);
                if d < best {
                    best = d;
                }
            }
        }
    }
    best
}

fn check_involute_band(generated: &Generated, module: f64, teeth: u32, pressure_angle: f64) {
    let pitch_radius = module * f64::from(teeth) / 2.0;
    let base_radius = pitch_radius * pressure_angle.to_radians().cos();
    let grid = flank_grid(module, teeth, pressure_angle, pitch_radius + module);
    let mut band: Vec<(f64, f64)> = Vec::new();
    for part in &generated.parts {
        for path in &part.paths {
            for (x, y) in sample_outline(path) {
                let rho = x.hypot(y);
                // Only the flank band: the root and tip arcs and the bore sit outside it.
                if rho <= (pitch_radius - 0.9 * module).max(base_radius)
                    || rho >= pitch_radius + 0.9 * module
                {
                    continue;
                }
                band.push((x, y));
            }
        }
    }
    assert!(
        band.len() >= 2 * teeth as usize,
        "enough flank points were checked: {}",
        band.len()
    );
    // The generator may place tooth 0 at any angle: the one common phase offset is searched on a
    // subset of the points (coarse, then fine), then every point is checked against it.
    let pitch = 2.0 * PI / f64::from(teeth);
    let subset: Vec<(f64, f64)> = band
        .iter()
        .step_by((band.len() / 40).max(1))
        .copied()
        .collect();
    let cost = |phase: f64| -> f64 {
        subset
            .iter()
            .map(|&(x, y)| distance_to_involute(&grid, x, y, phase))
            .sum()
    };
    let mut best_phase = 0.0;
    let mut best_cost = f64::INFINITY;
    for i in 0..720 {
        let phase = pitch * f64::from(i) / 720.0;
        let c = cost(phase);
        if c < best_cost {
            best_cost = c;
            best_phase = phase;
        }
    }
    let coarse = pitch / 720.0;
    for i in -360..=360 {
        let phase = best_phase + coarse * f64::from(i) / 360.0;
        let c = cost(phase);
        if c < best_cost {
            best_cost = c;
            best_phase = phase;
        }
    }
    let worst = band
        .iter()
        .map(|&(x, y)| distance_to_involute(&grid, x, y, best_phase))
        .fold(0.0_f64, f64::max);
    assert!(
        worst <= 0.01,
        "every flank point lies within 0.01 mm of the exact involute; worst {worst:.6} mm at phase {best_phase:.6} rad"
    );
}

// ---------- criterion 1 and 8: kinds and the registry ----------

#[test]
fn criterion_8_every_declared_generator_kind_answers_at_its_defaults() {
    let declared = manifest();
    assert_eq!(
        declared.id, PROCESS_ID,
        "the generators extension has its manifest id"
    );
    let empty = StoredParams::new();
    for kind in [GEAR, BOX] {
        let generated = GENERATORS
            .generate(&GenerateRequest {
                kind,
                params: &empty,
                seed: Seed(0),
                edit: None,
            })
            .unwrap_or_else(|| panic!("kind {kind} answers generate at its defaults"));
        assert!(
            !generated.parts.is_empty(),
            "{kind} has at least one outline at its defaults"
        );
        assert_all_closed(&generated);
    }
}

#[test]
fn criterion_8_an_unknown_kind_is_not_generated() {
    let empty = StoredParams::new();
    let answer = GENERATORS.generate(&GenerateRequest {
        kind: "hinge",
        params: &empty,
        seed: Seed(0),
        edit: None,
    });
    assert!(
        answer.is_none(),
        "a kind this extension does not declare yields no outline"
    );
}

#[test]
fn criterion_7_regenerating_from_stored_numbers_gives_the_same_outline() {
    let stored = params(&[("module", mm(2.0)), ("teeth", count(24)), ("bore", mm(6.0))]);
    let first = generate(GEAR, &stored, None);
    let again = generate(GEAR, &first.params, None);
    assert_eq!(
        first.parts, again.parts,
        "regeneration from the stored values is identical"
    );
    assert_eq!(
        first.params, again.params,
        "the normalised values are stable"
    );
}

#[test]
fn criterion_7_the_seed_does_not_change_a_gear_or_a_box() {
    let empty = StoredParams::new();
    for kind in [GEAR, BOX] {
        let a = GENERATORS
            .generate(&GenerateRequest {
                kind,
                params: &empty,
                seed: Seed(1),
                edit: None,
            })
            .unwrap();
        let b = GENERATORS
            .generate(&GenerateRequest {
                kind,
                params: &empty,
                seed: Seed(99_999),
                edit: None,
            })
            .unwrap();
        assert_eq!(a, b, "{kind} ignores the seed");
    }
}

// ---------- criterion 2: the gear ----------

#[test]
fn criterion_2_default_gear_is_one_closed_compound_outline_with_its_bore() {
    let generated = generate(GEAR, &StoredParams::new(), None);
    assert_eq!(generated.parts.len(), 1, "a gear is one part");
    assert_all_closed(&generated);
    assert_eq!(
        generated.parts[0].paths.len(),
        2,
        "outer tooth outline plus the centre bore hole"
    );
    assert_close(
        length(&generated, "pitch_diameter"),
        48.0,
        1e-9,
        "default pitch diameter is module times teeth",
    );
    assert_eq!(count_of(&generated, "teeth"), 24);
}

#[test]
fn criterion_2_default_gear_flanks_are_within_0_01_mm_of_the_exact_involute() {
    let generated = generate(GEAR, &StoredParams::new(), None);
    check_involute_band(&generated, 2.0, 24, 20.0);
}

#[test]
fn criterion_2_gear_of_200_teeth_flanks_are_within_0_01_mm_of_the_exact_involute() {
    let stored = params(&[("module", mm(1.0)), ("teeth", count(200))]);
    let generated = generate(GEAR, &stored, None);
    check_involute_band(&generated, 1.0, 200, 20.0);
}

#[test]
fn criterion_2_pitch_diameter_typed_changes_the_module() {
    let stored = params(&[
        ("module", mm(2.0)),
        ("teeth", count(24)),
        ("pitch_diameter", mm(60.0)),
    ]);
    let generated = generate(GEAR, &stored, Some(Edit::Param("pitch_diameter")));
    assert_close(
        length(&generated, "module"),
        2.5,
        1e-9,
        "module follows the typed pitch diameter",
    );
    assert_close(
        length(&generated, "pitch_diameter"),
        60.0,
        1e-9,
        "the typed pitch diameter is kept",
    );
}

#[test]
fn criterion_2_spokes_make_a_web_with_one_rounded_window_per_spoke() {
    let stored = params(&[
        ("module", mm(2.0)),
        ("teeth", count(24)),
        ("bore", mm(6.0)),
        ("web", choice("spokes")),
        ("spokes", count(5)),
        ("rim_width", mm(4.0)),
        ("hub_diameter", mm(16.0)),
    ]);
    let generated = generate(GEAR, &stored, None);
    assert_eq!(generated.parts.len(), 1, "still one part");
    assert_all_closed(&generated);
    assert_eq!(
        generated.parts[0].paths.len(),
        1 + 1 + 5,
        "outer outline, the bore and one window per spoke, all holes of one compound path"
    );
}

#[test]
fn criterion_2_keyway_is_stored_and_the_outline_stays_closed() {
    let stored = params(&[
        ("bore", mm(8.0)),
        ("keyway", on(true)),
        ("keyway_width", mm(3.0)),
        ("keyway_depth", mm(1.5)),
    ]);
    let generated = generate(GEAR, &stored, None);
    assert_all_closed(&generated);
    assert_close(
        length(&generated, "keyway_width"),
        3.0,
        1e-9,
        "keyway width kept",
    );
    assert_close(
        length(&generated, "keyway_depth"),
        1.5,
        1e-9,
        "keyway depth kept",
    );
}

// ---------- criterion 3: the box ----------

#[test]
fn criterion_3_default_box_is_five_plates_side_by_side_with_the_gap() {
    let generated = generate(BOX, &StoredParams::new(), None);
    assert_eq!(generated.parts.len(), 5, "five plates without a lid");
    assert_all_closed(&generated);
    let bounds: Vec<(f64, f64, f64, f64)> = generated
        .parts
        .iter()
        .map(|part| {
            let points: Vec<(f64, f64)> = part.paths.iter().flat_map(sample_outline).collect();
            let min_x = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
            let max_x = points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
            let min_y = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
            let max_y = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
            (min_x, min_y, max_x, max_y)
        })
        .collect();
    for (i, a) in bounds.iter().enumerate() {
        for b in bounds.iter().skip(i + 1) {
            let horizontal_gap = (b.0 - a.2).max(a.0 - b.2);
            let vertical_gap = (b.1 - a.3).max(a.1 - b.3);
            assert!(
                horizontal_gap.max(vertical_gap) >= 5.0 - 1e-6,
                "plates are apart by at least the gap (5 mm default)"
            );
        }
    }
}

#[test]
fn criterion_3_flat_lid_adds_a_plate() {
    let stored = params(&[("lid", choice("flat"))]);
    let generated = generate(BOX, &stored, None);
    assert_eq!(generated.parts.len(), 6, "a flat lid is one more plate");
}

#[test]
fn criterion_3_box_parameters_are_kept_as_stored() {
    let stored = params(&[
        ("inner_width", mm(120.0)),
        ("inner_depth", mm(80.0)),
        ("inner_height", mm(60.0)),
        ("thickness", mm(3.0)),
        ("joint", choice("fingers")),
        ("finger_width", mm(6.0)),
        ("kerf", mm(0.1)),
        ("clearance", mm(0.05)),
        ("gap", mm(10.0)),
    ]);
    let generated = generate(BOX, &stored, None);
    assert_close(
        length(&generated, "inner_width"),
        120.0,
        1e-9,
        "inner width",
    );
    assert_close(length(&generated, "inner_height"), 60.0, 1e-9, "height");
    assert_close(
        length(&generated, "kerf"),
        0.1,
        1e-9,
        "kerf is only a number",
    );
    assert_close(length(&generated, "clearance"), 0.05, 1e-9, "clearance");
    assert_all_closed(&generated);
}

// ---------- criterion 4: clamping, never an error ----------

#[test]
fn criterion_4_negative_thickness_is_clamped_to_the_minimum_with_a_clamp_record() {
    let stored = params(&[("thickness", mm(-5.0))]);
    let generated = generate(BOX, &stored, None);
    let clamp = generated
        .clamps
        .iter()
        .find(|clamp| clamp.key == "thickness")
        .expect("a clamp record names the field");
    assert_eq!(clamp.bound, Bound::Minimum, "the schema minimum");
    assert!(
        clamp.value.length().unwrap().as_mm() > 0.0,
        "clamped to a valid, positive value"
    );
    assert!(length(&generated, "thickness") > 0.0);
}

#[test]
fn criterion_4_teeth_below_six_are_clamped_to_six() {
    let stored = params(&[("teeth", count(5))]);
    let generated = generate(GEAR, &stored, None);
    assert_eq!(count_of(&generated, "teeth"), 6, "clamped to the minimum");
    let clamp = generated
        .clamps
        .iter()
        .find(|clamp| clamp.key == "teeth")
        .expect("clamp record");
    assert_eq!(clamp.bound, Bound::Minimum);
    assert_all_closed(&generated);
}

#[test]
fn criterion_4_bore_at_or_above_the_root_circle_is_clamped_below_it() {
    let stored = params(&[
        ("module", mm(2.0)),
        ("teeth", count(24)),
        ("bore", mm(45.0)),
    ]);
    let generated = generate(GEAR, &stored, None);
    let bore = length(&generated, "bore");
    assert!(
        bore < 43.0,
        "the bore stays inside the root circle (root diameter 43 mm): {bore}"
    );
    assert!(
        generated.clamps.iter().any(|clamp| clamp.key == "bore"),
        "the dependent clamp names the bore"
    );
    assert_all_closed(&generated);
}

#[test]
fn criterion_4_hub_and_keyway_dependent_values_never_error() {
    let stored = params(&[
        ("web", choice("spokes")),
        ("spokes", count(4)),
        ("rim_width", mm(20.0)),
        ("hub_diameter", mm(400.0)),
        ("bore", mm(6.0)),
        ("keyway", on(true)),
        ("keyway_width", mm(90.0)),
        ("keyway_depth", mm(40.0)),
    ]);
    let generated = generate(GEAR, &stored, None);
    assert!(
        !generated.parts.is_empty(),
        "an outline comes back, never an error"
    );
    assert_all_closed(&generated);
    assert!(
        generated
            .clamps
            .iter()
            .any(|clamp| clamp.key == "hub_diameter" || clamp.key == "keyway_width"),
        "a dependent value is clamped with a record"
    );
}

#[test]
fn criterion_4_a_box_of_100_fingers_per_edge_generates_closed_outlines() {
    // 100 fingers of 6 mm along a 600 mm edge, with a finger lid.
    let stored = params(&[
        ("inner_width", mm(600.0)),
        ("finger_width", mm(6.0)),
        ("lid", choice("fingers")),
    ]);
    let generated = generate(BOX, &stored, None);
    assert!(
        !generated.parts.is_empty(),
        "a box of 100 fingers per edge generates"
    );
    assert_all_closed(&generated);
}

// ---------- criterion 5: resize changes size parameters only ----------

#[test]
fn criterion_5_proportional_resize_of_a_gear_scales_its_size_parameters_only() {
    let stored = params(&[
        ("module", mm(2.0)),
        ("teeth", count(24)),
        ("bore", mm(6.0)),
        ("keyway", on(true)),
        ("keyway_width", mm(2.0)),
        ("keyway_depth", mm(1.0)),
        ("web", choice("spokes")),
        ("spokes", count(5)),
        ("rim_width", mm(4.0)),
        ("hub_diameter", mm(16.0)),
    ]);
    let generated = generate(GEAR, &stored, Some(Edit::Scale(ScaleFactor(2.0))));
    assert_close(length(&generated, "module"), 4.0, 1e-9, "module doubles");
    assert_close(
        length(&generated, "pitch_diameter"),
        96.0,
        1e-9,
        "pitch diameter doubles",
    );
    assert_close(length(&generated, "bore"), 12.0, 1e-9, "bore doubles");
    assert_close(
        length(&generated, "keyway_width"),
        4.0,
        1e-9,
        "keyway width doubles",
    );
    assert_close(
        length(&generated, "keyway_depth"),
        2.0,
        1e-9,
        "keyway depth doubles",
    );
    assert_close(
        length(&generated, "rim_width"),
        8.0,
        1e-9,
        "rim width doubles",
    );
    assert_close(
        length(&generated, "hub_diameter"),
        32.0,
        1e-9,
        "hub diameter doubles",
    );
    assert_eq!(count_of(&generated, "teeth"), 24, "teeth stay");
    assert_eq!(count_of(&generated, "spokes"), 5, "spokes stay");
    let angle = generated
        .params
        .get("pressure_angle")
        .and_then(ParamValue::from_stored);
    assert_eq!(
        angle.and_then(|value| value.degrees()),
        Some(20.0),
        "pressure angle stays"
    );
}

#[test]
fn criterion_5_proportional_resize_of_a_box_scales_inner_dimensions_only() {
    let stored = params(&[
        ("inner_width", mm(100.0)),
        ("inner_depth", mm(80.0)),
        ("inner_height", mm(50.0)),
        ("thickness", mm(3.0)),
        ("kerf", mm(0.2)),
        ("finger_width", mm(6.0)),
        ("clearance", mm(0.1)),
        ("gap", mm(5.0)),
    ]);
    let generated = generate(BOX, &stored, Some(Edit::Scale(ScaleFactor(2.0))));
    assert_close(
        length(&generated, "inner_width"),
        200.0,
        1e-9,
        "width doubles",
    );
    assert_close(
        length(&generated, "inner_depth"),
        160.0,
        1e-9,
        "depth doubles",
    );
    assert_close(
        length(&generated, "inner_height"),
        100.0,
        1e-9,
        "height doubles",
    );
    assert_close(
        length(&generated, "thickness"),
        3.0,
        1e-9,
        "thickness stays",
    );
    assert_close(length(&generated, "kerf"), 0.2, 1e-9, "kerf stays");
    assert_close(
        length(&generated, "finger_width"),
        6.0,
        1e-9,
        "finger width stays",
    );
    assert_close(
        length(&generated, "clearance"),
        0.1,
        1e-9,
        "clearance stays",
    );
    assert_close(length(&generated, "gap"), 5.0, 1e-9, "gap stays");
}

// Criterion 4 CPU budget (100 ms thread CPU for 200 teeth and 100 fingers) is NOT tested here:
// it belongs in curvyo-editor-wasm, measured on thread CPU time (CLAUDE.md §5).

#[test]
fn criterion_2_negative_control_the_involute_check_rejects_a_wrong_pressure_angle() {
    let generated = generate(GEAR, &StoredParams::new(), None);
    let result = std::panic::catch_unwind(|| check_involute_band(&generated, 2.0, 24, 25.0));
    assert!(
        result.is_err(),
        "the oracle rejects a 25 degree involute for a 20 degree gear"
    );
}
