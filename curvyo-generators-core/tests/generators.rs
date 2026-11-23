//! Acceptance of the generators contract (`specs/0141-parametric-generators`, criteria 2 to 4):
//! closed outlines, clamps with their notices, the box's plate ownership and the golden outlines.

#![allow(
    clippy::unwrap_used,
    clippy::many_single_char_names,
    clippy::format_push_string,
    clippy::len_zero
)]

use curvyo_document_core::Length;
use curvyo_extension_core::{
    Bound, ClosedPath, Edit, Extension, GenerateRequest, Generated, ParamValue, ScaleFactor, Seed,
    StoredParams,
};
use curvyo_generators_core::{BOX, GEAR, GENERATORS};

fn params(entries: &[(&str, ParamValue)]) -> StoredParams {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.to_stored()))
        .collect()
}

fn mm(value: f64) -> ParamValue {
    ParamValue::Length(Length::from_mm(value))
}

fn generate(kind: &str, stored: &StoredParams, edit: Option<Edit<'_>>) -> Generated {
    GENERATORS
        .generate(&GenerateRequest {
            kind,
            params: stored,
            seed: Seed(7),
            edit,
        })
        .unwrap()
}

fn points(path: &ClosedPath) -> Vec<(f64, f64)> {
    let mut out = vec![(path.start.x, path.start.y)];
    out.extend(path.segments.iter().map(|s| (s.to.x, s.to.y)));
    out
}

/// The signed area of a path, every cubic sampled at 16 points (exact for straight lines).
fn area(path: &ClosedPath) -> f64 {
    let mut p = vec![(path.start.x, path.start.y)];
    let mut from = path.start;
    for s in &path.segments {
        for k in 1..=16 {
            let u = f64::from(k) / 16.0;
            let v = 1.0 - u;
            let (a, b, c, d) = (v * v * v, 3.0 * v * v * u, 3.0 * v * u * u, u * u * u);
            p.push((
                a * from.x + b * s.c1.x + c * s.c2.x + d * s.to.x,
                a * from.y + b * s.c1.y + c * s.c2.y + d * s.to.y,
            ));
        }
        from = s.to;
    }
    (0..p.len() - 1)
        .map(|i| p[i].0 * p[i + 1].1 - p[i + 1].0 * p[i].1)
        .sum::<f64>()
        / 2.0
}

fn assert_closed(generated: &Generated) {
    for part in &generated.parts {
        for path in &part.paths {
            let last = path.segments.last().unwrap().to;
            assert!((last.x - path.start.x).abs() < 1e-9 && (last.y - path.start.y).abs() < 1e-9);
            for segment in &path.segments {
                for p in [segment.c1, segment.c2, segment.to] {
                    assert!(p.x.is_finite() && p.y.is_finite());
                }
            }
        }
    }
}

#[test]
fn every_declared_kind_answers_at_its_defaults_and_is_closed() {
    for kind in GENERATORS.manifest().generators() {
        let defaults = curvyo_extension_core::Param::defaults(&kind.schema());
        let generated = generate(kind.id, &defaults, None);
        assert!(!generated.parts.is_empty(), "{}", kind.id);
        assert_closed(&generated);
    }
    assert!(
        GENERATORS
            .generate(&GenerateRequest {
                kind: "hinge",
                params: &StoredParams::new(),
                seed: Seed(0),
                edit: None
            })
            .is_none()
    );
}

#[test]
fn a_gear_has_one_compound_part_and_a_matching_pitch_diameter() {
    let generated = generate(GEAR, &StoredParams::new(), None);
    assert_eq!(generated.parts.len(), 1);
    // Outline and bore.
    assert_eq!(generated.parts[0].paths.len(), 2);
    let outer = area(&generated.parts[0].paths[0]);
    let bore = area(&generated.parts[0].paths[1]);
    assert!(
        outer > 0.0 && bore < 0.0,
        "the hole winds against the outline"
    );
    assert!((bore.abs() - std::f64::consts::PI * 9.0).abs() < 0.05);
    let pitch = ParamValue::from_stored(&generated.params["pitch_diameter"]).unwrap();
    assert!((pitch.length().unwrap().as_mm() - 48.0).abs() < 1e-9);
}

#[test]
fn typing_the_pitch_diameter_changes_the_module() {
    let stored = params(&[("pitch_diameter", mm(50.0))]);
    let generated = generate(GEAR, &stored, Some(Edit::Param("pitch_diameter")));
    let module = ParamValue::from_stored(&generated.params["module"]).unwrap();
    assert!((module.length().unwrap().as_mm() - 50.0 / 24.0).abs() < 1e-9);
    let pitch = ParamValue::from_stored(&generated.params["pitch_diameter"]).unwrap();
    assert!((pitch.length().unwrap().as_mm() - 50.0).abs() < 1e-9);
    assert_eq!(generated.clamps.len(), 0);
}

#[test]
fn invalid_values_are_clamped_with_their_bound_never_refused() {
    let stored = params(&[("teeth", ParamValue::Count(3)), ("bore", mm(500.0))]);
    let generated = generate(GEAR, &stored, None);
    let teeth = generated.clamps.iter().find(|c| c.key == "teeth").unwrap();
    assert_eq!(teeth.bound, Bound::Minimum);
    assert_eq!(teeth.value, ParamValue::Count(6));
    let bore = generated.clamps.iter().find(|c| c.key == "bore").unwrap();
    assert_eq!(bore.bound, Bound::Dependent);
    // module 2, 6 teeth: root radius 3.5, so the bore is at most 5 mm.
    assert!((bore.value.length().unwrap().as_mm() - 5.0).abs() < 1e-9);
    assert_closed(&generated);
}

#[test]
fn spokes_leave_rounded_windows_between_hub_and_rim() {
    let stored = params(&[
        ("web", ParamValue::Choice("spokes".into())),
        ("module", mm(3.0)),
        ("teeth", ParamValue::Count(40)),
        ("bore", mm(10.0)),
        ("keyway", ParamValue::Switch(true)),
    ]);
    let generated = generate(GEAR, &stored, None);
    assert_closed(&generated);
    let paths = &generated.parts[0].paths;
    // Outline, five windows, bore.
    assert_eq!(paths.len(), 7);
    assert!(paths[1..6].iter().all(|window| area(window) < 0.0));
    let hub = ParamValue::from_stored(&generated.params["hub_diameter"])
        .unwrap()
        .length()
        .unwrap()
        .as_mm()
        / 2.0;
    let rim = ParamValue::from_stored(&generated.params["rim_width"])
        .unwrap()
        .length()
        .unwrap()
        .as_mm();
    let root = 60.0 - 3.75;
    for window in &paths[1..6] {
        for (x, y) in points(window) {
            let r = x.hypot(y);
            assert!(
                r >= hub - 1e-6 && r <= root - rim + 1e-6,
                "{r} outside the web"
            );
        }
    }
}

#[test]
fn a_200_tooth_gear_is_closed_and_every_tooth_is_drawn() {
    let stored = params(&[("teeth", ParamValue::Count(200)), ("module", mm(1.0))]);
    let generated = generate(GEAR, &stored, None);
    assert_closed(&generated);
    let outer = &generated.parts[0].paths[0];
    let rmax = points(outer)
        .iter()
        .map(|(x, y)| x.hypot(*y))
        .fold(0.0, f64::max);
    assert!((rmax - 101.0).abs() < 1e-6, "tip radius {rmax}");
}

#[test]
fn a_resize_scales_the_lengths_and_keeps_the_counts() {
    let stored = StoredParams::new();
    let scaled = generate(GEAR, &stored, Some(Edit::Scale(ScaleFactor(2.0))));
    let module = ParamValue::from_stored(&scaled.params["module"])
        .unwrap()
        .length()
        .unwrap()
        .as_mm();
    let bore = ParamValue::from_stored(&scaled.params["bore"])
        .unwrap()
        .length()
        .unwrap()
        .as_mm();
    assert!((module - 4.0).abs() < 1e-9 && (bore - 12.0).abs() < 1e-9);
    assert_eq!(
        ParamValue::from_stored(&scaled.params["teeth"]),
        Some(ParamValue::Count(24))
    );
    let boxed = generate(BOX, &stored, Some(Edit::Scale(ScaleFactor(0.5))));
    let width = ParamValue::from_stored(&boxed.params["inner_width"])
        .unwrap()
        .length()
        .unwrap()
        .as_mm();
    let thick = ParamValue::from_stored(&boxed.params["thickness"])
        .unwrap()
        .length()
        .unwrap()
        .as_mm();
    assert!((width - 50.0).abs() < 1e-9 && (thick - 3.0).abs() < 1e-9);
}

/// The material of the shell in mm^3 for `lid` and the inner sizes, thickness `t`.
fn shell_volume(iw: f64, id: f64, ih: f64, t: f64, lid: bool) -> f64 {
    let top = if lid { t } else { 0.0 };
    (iw + 2.0 * t) * (id + 2.0 * t) * (ih + t + top) - iw * id * ih
}

#[test]
fn the_box_plates_partition_the_material_exactly_for_every_lid() {
    for lid in ["none", "flat", "fingers"] {
        let stored = params(&[
            ("lid", ParamValue::Choice(lid.into())),
            ("finger_width", mm(7.0)),
        ]);
        let generated = generate(BOX, &stored, None);
        assert_closed(&generated);
        assert_eq!(generated.parts.len(), if lid == "none" { 5 } else { 6 });
        let sum: f64 = generated
            .parts
            .iter()
            .map(|part| part.paths.iter().map(area).sum::<f64>().abs())
            .sum();
        let expected = shell_volume(100.0, 80.0, 50.0, 3.0, lid != "none") / 3.0;
        assert!(
            (sum - expected).abs() < 1e-6,
            "{lid}: {sum} against {expected}"
        );
    }
}

#[test]
fn a_box_with_no_joint_is_plain_rectangles() {
    let stored = params(&[("joint", ParamValue::Choice("none".into()))]);
    let generated = generate(BOX, &stored, None);
    assert!(
        generated
            .parts
            .iter()
            .all(|part| points(&part.paths[0]).len() == 5)
    );
}

#[test]
fn kerf_widens_every_plate_by_the_kerf_and_clamps_to_the_material() {
    let plain = generate(BOX, &StoredParams::new(), None);
    let kerfed = generate(BOX, &params(&[("kerf", mm(0.2))]), None);
    let extent = |g: &Generated| {
        let p = points(&g.parts[0].paths[0]);
        let xs = p.iter().map(|q| q.0);
        xs.clone().fold(f64::MIN, f64::max) - xs.fold(f64::MAX, f64::min)
    };
    assert!((extent(&kerfed) - extent(&plain) - 0.2).abs() < 1e-9);
    let huge = generate(
        BOX,
        &params(&[("kerf", mm(2.0)), ("finger_width", mm(2.0))]),
        None,
    );
    assert!(
        huge.clamps
            .iter()
            .any(|c| c.key == "kerf" && c.bound == Bound::Dependent)
    );
}

#[test]
fn finger_width_longer_than_the_edge_is_clamped() {
    let stored = params(&[("finger_width", mm(150.0))]);
    let generated = generate(BOX, &stored, None);
    let clamp = generated
        .clamps
        .iter()
        .find(|c| c.key == "finger_width")
        .unwrap();
    assert_eq!(clamp.bound, Bound::Dependent);
    assert!((clamp.value.length().unwrap().as_mm() - 50.0).abs() < 1e-9);
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

#[test]
fn the_default_gear_outline_is_frozen() {
    golden(
        "gear_default.txt",
        &dump(&generate(GEAR, &StoredParams::new(), None)),
    );
}

#[test]
fn the_default_box_outline_is_frozen() {
    golden(
        "box_default.txt",
        &dump(&generate(BOX, &StoredParams::new(), None)),
    );
}

#[test]
fn generation_is_deterministic() {
    let a = dump(&generate(GEAR, &StoredParams::new(), None));
    let b = dump(&generate(GEAR, &StoredParams::new(), None));
    assert_eq!(a, b);
}
