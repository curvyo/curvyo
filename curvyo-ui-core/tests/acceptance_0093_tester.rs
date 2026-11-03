//! Independent acceptance tests for `specs/0093-laser-workbench-shell` at the extension contract
//! the shell dispatches through (ADR 0034): the compiled-in Laser manifest (criteria 2, 9, 11b),
//! the per-user disable list (11b), the Engraved grid outline as a property over its whole
//! parameter range (9) and the overhang notice text (8). No network, no G-code (15).
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use curvyo_document_core::{DocumentSize, Point};
use curvyo_extension_core::{Block, NoticeContext, Param, StoredParams};
use curvyo_ui_core::{ExtensionSettings, fixture_drawing, manifest, manifests, notice};
use proptest::prelude::*;

const LASER: &str = "curvyo.laser";
const GRID: &str = "curvyo.laser/engraved_grid";
const TOL: f64 = 1e-6;

fn grid_defaults() -> StoredParams {
    let m = manifest(LASER).unwrap();
    let kind = m
        .workbench()
        .unwrap()
        .fixture_kind("engraved_grid")
        .unwrap();
    Param::defaults(&kind.params)
}

fn params(spacing: f64, origin_x: f64, origin_y: f64) -> StoredParams {
    serde_json::from_value(serde_json::json!({
        "spacing": {"mm": spacing},
        "line_width": {"mm": 0.1},
        "origin_x": {"mm": origin_x},
        "origin_y": {"mm": origin_y},
    }))
    .unwrap()
}

fn area(w: f64, h: f64) -> DocumentSize {
    DocumentSize::from_mm(w, h)
}

fn overhang(min: (f64, f64), max: (f64, f64)) -> Option<String> {
    notice(
        LASER,
        "overhang",
        &NoticeContext {
            work_area: area(400.0, 300.0),
            placed_bounds: Some((Point::new(min.0, min.1), Point::new(max.0, max.1))),
        },
    )
}

// ---- criteria 2, 9, 11b: the manifest -----------------------------------------------------------

#[test]
fn ac11b_the_extension_list_holds_the_laser_workbench_with_version_and_licence() {
    let all = manifests();
    let laser = all.iter().find(|m| m.id == LASER).unwrap();
    assert_eq!(laser.name, "Laser workbench");
    assert_ne!(laser.version, "");
    assert_eq!(laser.version.split('.').count(), 3, "a semver version");
    assert_eq!(laser.licence, "AGPL-3.0-or-later");
}

#[test]
fn ac2_the_laser_workbench_brings_generic_laser_at_400_by_300() {
    let m = manifest(LASER).unwrap();
    let w = m.workbench().unwrap();
    assert_eq!(w.label, "Laser");
    assert_eq!(w.machine_name, "Generic laser");
    assert_eq!((w.work_width_mm, w.work_height_mm), (400.0, 300.0));
    assert_eq!(w.tabs.len(), 1);
    assert_eq!(w.tabs[0].label, "Laser");
    assert!(
        w.tabs[0]
            .blocks
            .iter()
            .any(|b| matches!(b, Block::FixtureList { .. })),
        "the Laser tab lists the fixtures"
    );
}

#[test]
fn ac9_the_one_fixture_kind_is_the_engraved_grid_with_its_defaults() {
    let m = manifest(LASER).unwrap();
    let kinds = &m.workbench().unwrap().fixture_kinds;
    assert_eq!(kinds.len(), 1, "one fixture kind in this slice");
    assert_eq!(kinds[0].label, "Engraved grid");
    assert_eq!(grid_defaults(), params(10.0, 0.0, 0.0));
}

// ---- criterion 11b: enable switch ---------------------------------------------------------------

#[test]
fn ac11b_everything_is_enabled_by_default_and_the_switch_round_trips_through_the_file() {
    let fresh = ExtensionSettings::parse("");
    assert!(fresh.is_enabled(LASER), "permit: default on");
    let mut s = ExtensionSettings::default();
    assert!(s.is_enabled(LASER));
    assert!(s.set_enabled(LASER, false));
    assert!(!s.is_enabled(LASER));
    let text = s.to_text();
    assert!(text.contains("disabled"), "{text}");
    let back = ExtensionSettings::parse(&text);
    assert!(!back.is_enabled(LASER));
    assert_eq!(back, s);
    assert!(s.set_enabled(LASER, true));
    assert!(ExtensionSettings::parse(&s.to_text()).is_enabled(LASER));
}

#[test]
fn ac11b_a_damaged_settings_file_falls_back_to_everything_enabled() {
    for text in ["disabled = 3", "{{{", "disabled = [1, 2]", "\u{0}"] {
        assert!(ExtensionSettings::parse(text).is_enabled(LASER), "{text:?}");
    }
    let s = ExtensionSettings::parse("disabled = [\"curvyo.laser\"]\nfuture = true\n");
    assert!(
        !s.is_enabled(LASER),
        "an unknown key does not lose the disable list"
    );
}

// ---- criterion 9: the Engraved grid -------------------------------------------------------------

#[test]
fn ac9_the_default_grid_over_400_by_300_is_41_by_31_lines_of_0_1_mm() {
    let d = fixture_drawing(GRID, &grid_defaults(), area(400.0, 300.0));
    let verticals = d.lines.iter().filter(|l| l.start.x == l.end.x).count();
    let horizontals = d.lines.iter().filter(|l| l.start.y == l.end.y).count();
    assert_eq!((verticals, horizontals), (41, 31));
    assert!((d.line_width.as_mm() - 0.1).abs() < TOL);
}

#[test]
fn ac9_an_unknown_kind_or_process_draws_nothing() {
    for kind in [
        "curvyo.laser/pocket",
        "acme.cnc/engraved_grid",
        "engraved_grid",
        "",
        "/",
    ] {
        assert!(
            fixture_drawing(kind, &grid_defaults(), area(400.0, 300.0))
                .lines
                .is_empty(),
            "{kind:?}"
        );
    }
}

#[test]
fn ac9_damaged_parameters_read_as_defaults_or_bounds_and_never_explode() {
    let damaged: Vec<StoredParams> = vec![
        StoredParams::new(),
        serde_json::from_value(serde_json::json!({"spacing": {"mm": 0.0}})).unwrap(),
        serde_json::from_value(serde_json::json!({"spacing": {"mm": -10.0}})).unwrap(),
        serde_json::from_value(serde_json::json!({"spacing": "dense"})).unwrap(),
        serde_json::from_value(serde_json::json!({"spacing": {"mm": 1e-300}})).unwrap(),
        serde_json::from_value(serde_json::json!({"spacing": {"in": 1.0}})).unwrap(),
        serde_json::from_value(serde_json::json!({"origin_x": {"mm": 1e300}})).unwrap(),
    ];
    for p in &damaged {
        let d = fixture_drawing(GRID, p, area(400.0, 300.0));
        // At most the minimum spacing's count (0.5 mm): 801 + 601.
        assert!(d.lines.len() <= 801 + 601, "{p:?}: {} lines", d.lines.len());
        for l in &d.lines {
            for q in [l.start, l.end] {
                assert!(q.x.is_finite() && q.y.is_finite(), "{p:?}");
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Every line lies on the output page, spans it whole, sits on the origin's lattice, and no
    /// lattice line inside the area is missing.
    #[test]
    fn ac9_grid_lines_cover_the_area_on_the_lattice_and_stay_inside(
        spacing in 0.5f64..500.0,
        ox in -2000.0f64..2000.0,
        oy in -2000.0f64..2000.0,
        w in 1.0f64..2000.0,
        h in 1.0f64..2000.0,
    ) {
        let d = fixture_drawing(GRID, &params(spacing, ox, oy), area(w, h));
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        for l in &d.lines {
            let vertical = (l.start.x - l.end.x).abs() < TOL;
            let horizontal = (l.start.y - l.end.y).abs() < TOL;
            prop_assert!(vertical ^ horizontal, "axis-parallel: {l:?}");
            for q in [l.start, l.end] {
                prop_assert!(q.x >= -TOL && q.x <= w + TOL && q.y >= -TOL && q.y <= h + TOL,
                    "inside the work area: {q:?}");
            }
            let (at, origin) = if vertical {
                prop_assert!((l.start.y.min(l.end.y)).abs() < TOL && (l.start.y.max(l.end.y) - h).abs() < TOL);
                xs.push(l.start.x);
                (l.start.x, ox)
            } else {
                prop_assert!((l.start.x.min(l.end.x)).abs() < TOL && (l.start.x.max(l.end.x) - w).abs() < TOL);
                ys.push(l.start.y);
                (l.start.y, oy)
            };
            let k = (at - origin) / spacing;
            prop_assert!((k - k.round()).abs() < 1e-6, "on the lattice: {at} from {origin} by {spacing}");
        }
        let expected = |origin: f64, extent: f64| {
            let first = ((0.0 - origin) / spacing).ceil();
            let last = ((extent - origin) / spacing).floor();
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let n = if last >= first { (last - first) as usize + 1 } else { 0 };
            n
        };
        // Lines exactly on an edge may fall either way within the tolerance.
        prop_assert!(xs.len().abs_diff(expected(ox, w)) <= 1, "{} vs {}", xs.len(), expected(ox, w));
        prop_assert!(ys.len().abs_diff(expected(oy, h)) <= 1, "{} vs {}", ys.len(), expected(oy, h));
    }
}

// ---- criterion 8: overhang notice ---------------------------------------------------------------

#[test]
fn ac8_the_overhang_notice_names_the_actual_overhang() {
    assert_eq!(
        overhang((0.0, 0.0), (440.0, 312.0)).as_deref(),
        Some("Extends past the work area by 40 x 12 mm")
    );
    assert!(
        overhang((0.0, 0.0), (400.0, 300.0)).is_none(),
        "exactly fits"
    );
    assert!(overhang((10.0, 10.0), (200.0, 100.0)).is_none());
    assert_eq!(
        overhang((-5.0, 0.0), (400.0, 300.0)).as_deref(),
        Some("Extends past the work area by 5 x 0 mm"),
        "past the left edge counts too, a zero axis reads 0"
    );
    assert_eq!(
        overhang((0.0, 0.0), (412.26, 300.0)).as_deref(),
        Some("Extends past the work area by 12.3 x 0 mm"),
        "one decimal at most"
    );
    assert!(
        overhang((0.0, 0.0), (400.005, 300.0)).is_none(),
        "within the 0.01 mm tolerance"
    );
}

#[test]
fn ac8_a_shown_overhang_notice_never_reads_zero_by_zero() {
    // Above the 0.01 mm tolerance, so the notice shows; its numbers must say something.
    let text = overhang((0.0, 0.0), (400.04, 300.0)).unwrap();
    assert_ne!(
        text, "Extends past the work area by 0 x 0 mm",
        "a shown notice states the actual overhang"
    );
}

#[test]
fn ac8_no_placed_content_no_notice_and_other_processes_have_none() {
    let ctx = NoticeContext {
        work_area: area(400.0, 300.0),
        placed_bounds: None,
    };
    assert!(notice(LASER, "overhang", &ctx).is_none());
    let past = NoticeContext {
        work_area: area(400.0, 300.0),
        placed_bounds: Some((Point::new(0.0, 0.0), Point::new(900.0, 900.0))),
    };
    assert!(notice("acme.cnc", "overhang", &past).is_none());
    assert!(notice(LASER, "unknown_notice", &past).is_none());
}

#[test]
fn ac7_ac8_notices_fit_in_70_characters() {
    let text = overhang((-9999.9, -9999.9), (19999.9, 19999.9)).unwrap();
    assert!(text.chars().count() <= 70, "{text}");
}

// ---- criterion 15 -------------------------------------------------------------------------------

#[test]
fn ac15_the_extension_crates_have_no_io_network_or_gcode() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for krate in ["curvyo-laser-core", "curvyo-extension-core"] {
        let toml = std::fs::read_to_string(root.join(krate).join("Cargo.toml")).unwrap();
        let deps = toml.split("[dependencies]").nth(1).unwrap_or("");
        for line in deps.lines().filter(|l| l.contains('=')) {
            let name = line
                .split('=')
                .next()
                .unwrap()
                .trim()
                .trim_end_matches(".workspace");
            assert!(
                [
                    "curvyo-extension-core",
                    "curvyo-document-core",
                    "curvyo-geometry-core",
                    "serde",
                    "serde_json"
                ]
                .contains(&name),
                "{krate} depends on {name}"
            );
        }
        for entry in std::fs::read_dir(root.join(krate).join("src")).unwrap() {
            let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
            for banned in [
                "std::net",
                "std::fs",
                "std::process",
                "TcpStream",
                "reqwest",
                "G-code",
                "G0 ",
                "G1 ",
                "telemetry",
            ] {
                assert!(!text.contains(banned), "{krate} contains {banned}");
            }
        }
        let lib = std::fs::read_to_string(root.join(krate).join("src/lib.rs")).unwrap();
        assert!(lib.contains("#![forbid(unsafe_code)]"), "{krate}");
    }
}
