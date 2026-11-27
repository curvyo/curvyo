//! The acceptance criteria of `specs/0144-qr-and-barcodes` slice 1 (QR) that the core crate can
//! check without a host: the numbers, the styles, the clear area, Inverted, the refusal, the
//! clamps and the stored encoding.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::len_zero,
    clippy::assert_is_empty,
    clippy::format_push_string,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::needless_pass_by_value
)]

mod support;

use curvyo_codes_core::{EcLevel, EncodeError, ModuleShape, clear_area, encode};
use curvyo_document_core::{Length, Point};
use curvyo_extension_core::{
    Bound, ClosedPath, Edit, ParamValue, ScaleFactor, ShownWhen, StoredParams,
};
use support::{choice, generate, generate_from, params, paths, schema, text};

fn mm(generated: &curvyo_extension_core::Generated, key: &str) -> f64 {
    generated.params[key]["mm"].as_f64().unwrap()
}

fn bounds(paths: &[ClosedPath]) -> (Point, Point) {
    let (mut low, mut high) = (
        Point::new(f64::MAX, f64::MAX),
        Point::new(f64::MIN, f64::MIN),
    );
    for path in paths {
        for point in std::iter::once(path.start).chain(path.segments.iter().map(|c| c.to)) {
            low = Point::new(low.x.min(point.x), low.y.min(point.y));
            high = Point::new(high.x.max(point.x), high.y.max(point.y));
        }
    }
    (low, high)
}

/// The signed area of the polygon of end points (exact for straight outlines).
fn area(path: &ClosedPath) -> f64 {
    let mut points = vec![path.start];
    points.extend(path.segments.iter().map(|c| c.to));
    points
        .windows(2)
        .map(|w| w[0].x * w[1].y - w[1].x * w[0].y)
        .sum::<f64>()
        / 2.0
}

fn dark_modules(content: &str, level: EcLevel) -> usize {
    let grid = encode(content, level).unwrap();
    (0..grid.width() * grid.width())
        .filter(|at| grid.is_dark(at % grid.width(), at / grid.width()))
        .count()
}

// Criteria 1 and 2

#[test]
fn hello_at_m_is_21_modules_and_the_width_includes_the_quiet_zone() {
    let generated = generate(&[("content", text("Hello"))]);
    assert_eq!(mm(&generated, "module_size"), 1.0);
    assert!(
        (mm(&generated, "width") - 29.0).abs() < 0.001,
        "21 + 2 x 4 modules"
    );
    let (low, high) = bounds(paths(&generated));
    assert!(
        (high.x - low.x - 21.0).abs() < 0.001 && (high.y - low.y - 21.0).abs() < 0.001,
        "the box is the symbol"
    );
    assert!(
        (low.x + high.x).abs() < 1e-9 && (low.y + high.y).abs() < 1e-9,
        "centred on the origin"
    );
}

#[test]
fn version_40_holds_2953_bytes_at_l_and_2954_are_refused_with_the_limit() {
    assert_eq!(encode(&"a".repeat(2953), EcLevel::L).unwrap().width(), 177);
    assert_eq!(
        encode(&"a".repeat(2954), EcLevel::L),
        Err(EncodeError::TooLong {
            bytes: 2954,
            limit: 2953
        })
    );
    let refused = generate(&[("content", text(&"a".repeat(2954))), ("ec", choice("l"))]);
    assert!(refused.parts.is_empty());
    assert_eq!(refused.warnings[0].id, "content_too_long");
    assert_eq!(refused.warnings[0].value, Some(ParamValue::Count(2954)));
    assert_eq!(refused.warnings[0].limit, Some(ParamValue::Count(2953)));
}

#[test]
fn the_same_input_gives_the_identical_output() {
    let asked = params(&[
        ("content", text("https://example.com/same")),
        ("module_shape", choice("dot")),
    ]);
    assert_eq!(generate_from(&asked, None), generate_from(&asked, None));
}

// Criterion 5

#[test]
fn a_new_code_has_the_spec_defaults_and_no_symbology_field() {
    let kind = curvyo_codes_core::manifest();
    let kind = kind.generators().next().unwrap().clone();
    assert_eq!(
        (kind.id, kind.label, kind.icon),
        ("qr", "QR code", "QrCode")
    );
    let groups: Vec<&str> = kind.groups.iter().map(|g| g.label).collect();
    assert_eq!(
        groups,
        [
            "Content",
            "Size",
            "Modules",
            "Finder",
            "Error correction",
            "Logo",
            "Colours",
            "Advanced"
        ]
    );
    assert!(kind.params().all(|p| p.key != "symbology"));
    let defaults = params(&[]);
    assert_eq!(
        defaults["content"],
        serde_json::json!({"text": "https://example.com"})
    );
    assert_eq!(defaults["ec"], serde_json::json!({"choice": "m"}));
    assert_eq!(defaults["quiet_zone"], serde_json::json!({"count": 4}));
    let generated = generate(&[]);
    assert_eq!(generated.parts.len(), 1);
    assert_eq!(generated.parts[0].key, "code");
    assert!(generated.clamps.is_empty() && generated.warnings.is_empty());
}

#[test]
fn turning_the_clear_area_on_raises_the_level_to_h_and_off_leaves_it() {
    let on = params(&[("logo", ParamValue::Switch(true))]);
    let generated = generate_from(&on, Some(Edit::Param("logo")));
    assert_eq!(generated.params["ec"], serde_json::json!({"choice": "h"}));
    assert!(generated.clamps.iter().all(|c| c.key != "ec"), "no notice");
    let off = params(&[("ec", choice("h")), ("logo", ParamValue::Switch(false))]);
    let generated = generate_from(&off, Some(Edit::Param("logo")));
    assert_eq!(generated.params["ec"], serde_json::json!({"choice": "h"}));
    // a text that fits only at Q: raised to Q, the highest it fits
    let text = "q".repeat(1300);
    let asked = params(&[
        ("content", support::text(&text)),
        ("ec", choice("l")),
        ("logo", ParamValue::Switch(true)),
    ]);
    let generated = generate_from(&asked, Some(Edit::Param("logo")));
    assert_eq!(generated.params["ec"], serde_json::json!({"choice": "q"}));
}

// Criterion 6, 7

#[test]
fn merged_square_modules_cover_exactly_the_dark_modules_and_share_no_edge() {
    let generated = generate(&[("content", text("Hello"))]);
    let covered: f64 = paths(&generated).iter().map(area).sum();
    #[allow(clippy::cast_precision_loss)]
    let dark = dark_modules("Hello", EcLevel::M) as f64;
    assert!(
        (covered - dark).abs() < 0.001,
        "{covered} against {dark} mm^2"
    );
    let mut edges: Vec<[i64; 4]> = Vec::new();
    for path in paths(&generated) {
        let mut from = path.start;
        for c in &path.segments {
            #[allow(clippy::cast_possible_truncation)]
            let key = |p: Point| ((p.x * 1000.0).round() as i64, (p.y * 1000.0).round() as i64);
            let (a, b) = (key(from), key(c.to));
            edges.push([a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1)]);
            from = c.to;
        }
    }
    let total = edges.len();
    edges.sort_unstable();
    edges.dedup();
    assert_eq!(edges.len(), total, "no two segments overlap");
}

#[test]
fn rounded_modules_round_convex_corners_only_and_never_add_ink() {
    let square = generate(&[("content", text("Hello"))]);
    let rounded = generate(&[
        ("content", text("Hello")),
        ("module_shape", choice("rounded")),
    ]);
    let (a, b): (f64, f64) = (
        paths(&square).iter().map(area).sum(),
        paths(&rounded).iter().map(area).sum(),
    );
    assert!(b < a && b > a * 0.8, "{b} against {a}");
    let full = generate(&[
        ("content", text("Hello")),
        ("module_shape", choice("rounded")),
        ("rounding", ParamValue::Percent(0)),
    ]);
    assert_eq!(
        paths(&full).iter().map(area).sum::<f64>(),
        a,
        "0 % rounding is square"
    );
}

#[test]
fn dot_and_diamond_and_a_fill_below_100_are_separate_one_subpath_per_module() {
    let dark = dark_modules("Hello", EcLevel::M);
    for (shape, fill) in [("dot", 100), ("diamond", 100), ("square", 80)] {
        let generated = generate(&[
            ("content", text("Hello")),
            ("module_shape", choice(shape)),
            ("module_fill", ParamValue::Percent(fill)),
        ]);
        // the three finders are drawn by the finder style: ring, hole and centre each
        let finder_modules = 3 * (49 - 25 + 9);
        let data = dark - finder_modules;
        assert_eq!(paths(&generated).len(), data + 9, "{shape} at {fill} %");
    }
    // the quiet and the stored output choice are hidden below 100 %
    let schema = schema();
    let output = schema.iter().find(|p| p.key == "output").unwrap();
    let hidden = params(&[("module_fill", ParamValue::Percent(90))]);
    let shown = output.shown_when.unwrap();
    assert!(!shown.holds(&schema, &hidden));
    assert!(shown.holds(&schema, &params(&[("module_shape", choice("rounded"))])));
    assert!(!shown.holds(&schema, &params(&[("module_shape", choice("dot"))])));
    let _ = ShownWhen::Full { key: "module_fill" };
}

#[test]
fn a_finder_at_the_style_keeps_its_ring_hole_and_centre() {
    for outer in ["square", "rounded"] {
        let generated = generate(&[
            ("content", text("Hello")),
            ("finder_outer", choice(outer)),
            ("module_shape", choice("dot")),
        ]);
        let negative = paths(&generated).iter().filter(|p| area(p) < 0.0).count();
        assert_eq!(negative, 3, "{outer}: one hole per finder");
    }
}

// Criterion 8

#[test]
fn a_clear_area_over_the_budget_is_limited_and_says_which_level_allows_it() {
    let asked = params(&[
        ("content", text("Hello")),
        ("logo", ParamValue::Switch(true)),
        ("logo_size", ParamValue::Percent(35)),
        ("logo_margin", ParamValue::Count(2)),
    ]);
    let generated = generate_from(&asked, Some(Edit::Param("logo_size")));
    let size = generated.params["logo_size"]["pct"].as_u64().unwrap();
    assert!(size < 35, "stored size is the limited value, {size}");
    let clamp = generated
        .clamps
        .iter()
        .find(|c| c.key == "logo_size" || c.key == "logo_margin")
        .unwrap();
    assert_eq!(clamp.bound, Bound::DependentFor("error correction M"));
}

#[test]
fn the_stored_clear_area_never_exceeds_the_budget_of_its_version_and_level() {
    for level in EcLevel::ALL {
        for version_text in [
            "Hello",
            "https://example.com/a/longer/path/that/needs/a/bigger/version/1234567890",
        ] {
            let grid = encode(version_text, level).unwrap();
            let generated = generate(&[
                ("content", text(version_text)),
                ("ec", choice(level.id())),
                ("logo", ParamValue::Switch(true)),
                ("logo_size", ParamValue::Percent(35)),
                ("logo_margin", ParamValue::Count(2)),
            ]);
            let area = clear_area::ClearArea {
                shape: clear_area::LogoShape::Circle,
                size: u32::try_from(generated.params["logo_size"]["pct"].as_u64().unwrap())
                    .unwrap(),
                margin: u32::try_from(generated.params["logo_margin"]["count"].as_u64().unwrap())
                    .unwrap(),
            };
            assert!(
                clear_area::count(&grid, area) <= clear_area::budget(&grid),
                "{level:?} {version_text}"
            );
        }
    }
}

#[test]
fn a_lower_level_that_shrinks_the_budget_is_a_knock_on_with_the_same_clamp() {
    let high = params(&[
        (
            "content",
            text("https://example.com/a/longer/path/to/fill/version/five"),
        ),
        ("ec", choice("h")),
        ("logo", ParamValue::Switch(true)),
        ("logo_size", ParamValue::Percent(30)),
    ]);
    let at_h = generate_from(&high, Some(Edit::Param("ec")));
    let mut low = at_h.params.clone();
    low.insert("ec".into(), choice("l").to_stored());
    let at_l = generate_from(&low, Some(Edit::Param("ec")));
    assert!(at_l.params["logo_size"]["pct"].as_u64() < at_h.params["logo_size"]["pct"].as_u64());
    assert_eq!(
        at_l.clamps[0].bound,
        Bound::DependentFor("error correction L")
    );
}

// Criterion 10

#[test]
fn an_inverted_code_is_one_rectangle_with_the_dark_modules_cut_out() {
    let generated = generate(&[
        ("content", text("Hello")),
        ("inverted", ParamValue::Switch(true)),
    ]);
    let covered: f64 = paths(&generated).iter().map(area).sum();
    #[allow(clippy::cast_precision_loss)]
    let expected = 29.0 * 29.0 - dark_modules("Hello", EcLevel::M) as f64;
    assert!(
        (covered - expected).abs() < 0.001,
        "{covered} against {expected}"
    );
    let (low, high) = bounds(paths(&generated));
    assert!(
        (high.x - low.x - 29.0).abs() < 0.001,
        "the box includes the quiet zone"
    );
}

// Criterion 13

#[test]
fn empty_and_too_long_content_are_refused_with_no_parts_and_the_params_kept() {
    let empty = generate(&[("content", text(""))]);
    assert!(empty.parts.is_empty());
    assert_eq!(empty.warnings[0].id, "content_empty");
    assert_eq!(
        empty.params["content"],
        serde_json::json!({"text": ""}),
        "the typed text is written"
    );
    let long = generate(&[("content", text(&"a".repeat(2332)))]);
    assert_eq!(
        (long.warnings[0].id, long.warnings[0].limit.clone()),
        ("content_too_long", Some(ParamValue::Count(2331)))
    );
    // changing the content never lowers the level; only the level edit does
    let asked = params(&[("content", text(&"a".repeat(2332)))]);
    assert!(
        generate_from(&asked, Some(Edit::Param("content")))
            .parts
            .is_empty()
    );
    let lowered = generate_from(&asked, Some(Edit::Param("ec")));
    assert!(!lowered.parts.is_empty());
    assert_eq!(lowered.params["ec"], serde_json::json!({"choice": "l"}));
    assert_eq!(lowered.clamps[0].bound, Bound::DependentFor("this content"));
}

// Criterion 14

#[test]
fn a_scale_changes_the_module_size_and_the_width_follows_within_the_bounds() {
    let asked = params(&[("content", text("Hello"))]);
    let doubled = generate_from(&asked, Some(Edit::Scale(ScaleFactor(2.0))));
    assert_eq!(mm(&doubled, "module_size"), 2.0);
    assert!((mm(&doubled, "width") - 58.0).abs() < 1e-9);
    let huge = generate_from(&asked, Some(Edit::Scale(ScaleFactor(100.0))));
    assert_eq!(mm(&huge, "module_size"), 20.0);
    assert_eq!(huge.clamps[0].bound, Bound::Maximum);
    let tiny = generate_from(&asked, Some(Edit::Scale(ScaleFactor(0.01))));
    assert_eq!(
        (mm(&tiny, "module_size"), tiny.clamps[0].bound),
        (0.1, Bound::Minimum)
    );
}

#[test]
fn typing_the_width_writes_the_module_size() {
    let mut asked = params(&[("content", text("Hello"))]);
    asked.insert(
        "width".into(),
        ParamValue::Length(Length::from_mm(58.0)).to_stored(),
    );
    let generated = generate_from(&asked, Some(Edit::Param("width")));
    assert!((mm(&generated, "module_size") - 2.0).abs() < 1e-9);
    asked.insert(
        "width".into(),
        ParamValue::Length(Length::from_mm(2.1)).to_stored(),
    );
    let least = generate_from(&asked, Some(Edit::Param("width")));
    assert!(
        (mm(&least, "module_size") - 0.1).abs() < 1e-9,
        "clamped through the module size"
    );
    assert!((mm(&least, "width") - 2.9).abs() < 1e-9);
    assert_eq!(least.clamps[0].key, "width");
    assert_eq!(least.clamps[0].bound, Bound::DependentMinimum);
}

#[test]
fn a_change_of_size_keeps_the_centre_in_place() {
    for edit in [
        ("quiet_zone", ParamValue::Count(8)),
        (
            "content",
            text("a longer text that needs another version of the code"),
        ),
    ] {
        let mut asked = params(&[]);
        asked.insert(edit.0.into(), edit.1.to_stored());
        let (low, high) = bounds(paths(&generate_from(&asked, Some(Edit::Param(edit.0)))));
        assert!(
            (low.x + high.x).abs() < 1e-9 && (low.y + high.y).abs() < 1e-9,
            "{}",
            edit.0
        );
    }
}

// Criterion 16 and 18

#[test]
fn every_stored_value_round_trips_through_the_schema() {
    let generated = generate(&[
        ("content", text("two\nlines")),
        ("module_shape", choice("rounded")),
        ("rounding", ParamValue::Percent(20)),
        ("logo", ParamValue::Switch(true)),
        ("inverted", ParamValue::Switch(true)),
    ]);
    let again: StoredParams = generated.params.clone();
    let regenerated = generate_from(&again, None);
    assert_eq!(
        regenerated.parts, generated.parts,
        "regenerating the stored params gives the stored paths"
    );
    assert_eq!(regenerated.params, generated.params);
    assert!(
        regenerated.clamps.is_empty(),
        "a stored, limited value is not limited again"
    );
    assert_eq!(again["content"], serde_json::json!({"text": "two\nlines"}));
}

#[test]
fn the_crate_has_no_logging_and_no_network_surface() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let source = std::fs::read_to_string(&path).unwrap();
                let production = source.split("#[cfg(test)]").next().unwrap();
                for banned in [
                    "println!",
                    "eprintln!",
                    "dbg!",
                    "log::",
                    "tracing::",
                    "std::net",
                    "reqwest",
                    "format!(\"{content",
                ] {
                    assert!(
                        !production.contains(banned),
                        "{} uses {banned}",
                        path.display()
                    );
                }
            }
        }
    }
}

#[test]
fn the_module_shapes_have_stable_ids() {
    for shape in [
        ModuleShape::Square,
        ModuleShape::Rounded,
        ModuleShape::Dot,
        ModuleShape::Diamond,
    ] {
        assert_eq!(ModuleShape::from_id(shape.id()), Some(shape));
    }
}

#[test]
fn the_ghost_is_the_symbol_square_and_the_three_finder_edges() {
    let generated = generate_from(&params(&[]), Some(Edit::Draft));
    assert_eq!(
        paths(&generated).len(),
        4,
        "the square and the three outer edges"
    );
    let (low, high) = bounds(paths(&generated));
    assert!(
        (high.x - low.x - 25.0).abs() < 0.001,
        "version 2 of the default text, 25 modules"
    );
}

#[test]
fn a_stored_circular_outer_finder_reads_as_the_default() {
    let asked = params(&[
        ("content", text("Hello")),
        ("finder_outer", choice("circle")),
    ]);
    let generated = generate_from(&asked, None);
    assert_eq!(
        generated.params["finder_outer"],
        serde_json::json!({"choice": "square"})
    );
    assert_eq!(
        paths(&generated),
        paths(&generate(&[("content", text("Hello"))]))
    );
}
