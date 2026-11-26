//! Acceptance of `specs/0155-generator-joint-options`, slice 3 (criteria 18 to 24): the Advanced
//! group and the joint layout of the tabs. Black-box: measured on the generated parts.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::many_single_char_names
)]

mod budget_factor;

use curvyo_document_core::Length;
use curvyo_extension_core::{
    Bound, ClosedPath, Extension, GenerateRequest, Generated, Param, ParamKind, ParamValue, Part,
    Seed, ShownWhen, StoredParams,
};
use curvyo_generators_core::{GENERATORS, PATH_BOX, ROUNDED_BOX};

type Pt = (f64, f64);

const T: f64 = 3.0;

fn mm(value: f64) -> ParamValue {
    ParamValue::Length(Length::from_mm(value))
}

fn choice(id: &str) -> ParamValue {
    ParamValue::Choice(id.to_owned())
}

fn count(n: u32) -> ParamValue {
    ParamValue::Count(n)
}

fn stored(entries: &[(&str, ParamValue)]) -> StoredParams {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.to_stored()))
        .collect()
}

fn generate(kind: &str, params: &StoredParams) -> Generated {
    GENERATORS
        .generate(&GenerateRequest {
            kind,
            params,
            seed: Seed(1),
            edit: None,
        })
        .unwrap()
}

/// A 200 x 100 rounded box (radius 15, so every straight carries tabs) with `extra`.
fn rounded(extra: &[(&str, ParamValue)]) -> Generated {
    let mut entries = vec![
        ("inner_width", mm(200.0)),
        ("inner_depth", mm(100.0)),
        ("corner_radius", mm(15.0)),
        ("thickness", mm(T)),
    ];
    entries.extend(extra.iter().cloned());
    generate(ROUNDED_BOX, &stored(&entries))
}

fn sample(path: &ClosedPath) -> Vec<Pt> {
    const STEPS: u32 = 8;
    let mut points = Vec::new();
    let mut from = path.start;
    for c in &path.segments {
        for step in 0..STEPS {
            let u = f64::from(step) / f64::from(STEPS);
            let v = 1.0 - u;
            let f = |a: f64, b: f64, c1: f64, d: f64| {
                v * v * v * a + 3.0 * v * v * u * b + 3.0 * v * u * u * c1 + u * u * u * d
            };
            points.push((
                f(from.x, c.c1.x, c.c2.x, c.to.x),
                f(from.y, c.c1.y, c.c2.y, c.to.y),
            ));
        }
        from = c.to;
    }
    points
}

fn bounds(part: &Part) -> (f64, f64, f64, f64) {
    let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for (x, y) in part.paths.iter().flat_map(sample) {
        b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
    }
    b
}

/// The strip: the longest part.
fn strip(generated: &Generated) -> &Part {
    generated
        .parts
        .iter()
        .max_by(|a, b| {
            let (p, q) = (bounds(a), bounds(b));
            (p.2 - p.0).total_cmp(&(q.2 - q.0))
        })
        .unwrap()
}

/// The sorted x of every crossing of the horizontal line at `y` with the part's outlines.
fn crossings(part: &Part, y: f64) -> Vec<f64> {
    let mut cross = Vec::new();
    for path in &part.paths {
        let pts = sample(path);
        for i in 0..pts.len() {
            let ((x0, y0), (x1, y1)) = (pts[i], pts[(i + 1) % pts.len()]);
            if (y0 > y) != (y1 > y) {
                cross.push(x0 + (y - y0) / (y1 - y0) * (x1 - x0));
            }
        }
    }
    cross.sort_by(f64::total_cmp);
    cross
}

/// `(centre, width)` of every cut wider than `least` that the line at `y` meets.
fn notches(part: &Part, y: f64, least: f64) -> Vec<(f64, f64)> {
    crossings(part, y)[1..]
        .chunks(2)
        .filter(|pair| pair.len() == 2 && pair[1] - pair[0] > least)
        .map(|pair| (f64::midpoint(pair[0], pair[1]), pair[1] - pair[0]))
        .collect()
}

/// The tab notches of the base joint along the strip's bottom edge.
fn base_notches(generated: &Generated) -> Vec<(f64, f64)> {
    let part = strip(generated);
    let b = bounds(part);
    notches(part, b.3 - 1.5, 4.0)
}

fn middle(generated: &Generated) -> f64 {
    let b = bounds(strip(generated));
    f64::midpoint(b.0, b.2)
}

fn kind_groups(id: &str) -> Vec<curvyo_extension_core::ParamGroup> {
    let manifest = GENERATORS.manifest();
    manifest
        .generators()
        .find(|k| k.id == id)
        .unwrap()
        .groups
        .clone()
}

fn dump(generated: &Generated) -> String {
    format!("{:?}", generated.parts)
}

// ---------- criterion 18: the group, the keys, the defaults ----------

#[test]
fn criterion_18_both_boxes_end_with_an_advanced_group_of_the_five_keys() {
    for id in [ROUNDED_BOX, PATH_BOX] {
        let groups = kind_groups(id);
        let last = groups.last().unwrap();
        assert_eq!(last.label, "Advanced", "{id}");
        let keys: Vec<&str> = last.params.iter().map(|p| p.key).collect();
        assert_eq!(
            keys,
            [
                "joint_pattern",
                "joint_count",
                "joint_width",
                "joint_gap",
                "joint_spread"
            ]
        );
        let labels: Vec<&str> = last.params.iter().map(|p| p.label).collect();
        assert_eq!(
            labels,
            [
                "Joint pattern",
                "Joint count",
                "Joint size",
                "Joint gap",
                "Joint spread"
            ]
        );
        // Gap and spread only for Centred.
        for param in &last.params[3..] {
            assert!(param.shown_when.is_some(), "{} is conditional", param.key);
        }
        assert!(matches!(
            last.params[4].kind,
            ParamKind::Count {
                min: 0,
                max: 100,
                automatic: false
            }
        ));
        assert_eq!(last.params[4].default, count(0));
    }
}

#[test]
fn criterion_18_the_rounded_boxs_advanced_rows_hang_on_the_flat_top() {
    let groups = kind_groups(ROUNDED_BOX);
    for param in &groups.last().unwrap().params {
        let shown = param.shown_when.expect("conditional on the top shape");
        let text = format!("{shown:?}");
        assert!(text.contains("flat"), "{} {text}", param.key);
    }
    assert!(matches!(
        groups.last().unwrap().params[0].shown_when,
        Some(ShownWhen::Is { .. })
    ));
}

#[test]
fn criterion_18_the_defaults_equal_the_output_before() {
    let plain = rounded(&[]);
    let explicit = rounded(&[
        ("joint_pattern", choice("even")),
        ("joint_count", count(0)),
        ("joint_width", mm(0.0)),
        ("joint_gap", mm(0.0)),
        ("joint_spread", count(0)),
    ]);
    assert_eq!(dump(&plain), dump(&explicit));
    let path_plain = generate(PATH_BOX, &StoredParams::new());
    let path_explicit = generate(
        PATH_BOX,
        &stored(&[("joint_pattern", choice("even")), ("joint_count", count(0))]),
    );
    assert_eq!(dump(&path_plain), dump(&path_explicit));
}

#[test]
fn criterion_19_an_unknown_pattern_id_reads_even() {
    let unknown = rounded(&[("joint_pattern", choice("zigzag"))]);
    assert_eq!(dump(&unknown), dump(&rounded(&[])));
}

// ---------- criteria 19, 21: the patterns ----------

#[test]
fn criterion_21_count_one_centred_is_the_middle_of_the_edge_and_the_seam_edge_takes_two() {
    let g = rounded(&[
        ("joint_pattern", choice("centred")),
        ("joint_count", count(1)),
    ]);
    let found = base_notches(&g);
    // Front (seam) 2, back 1, left 1, right 1.
    assert_eq!(found.len(), 5, "{found:?}");
    let mid = middle(&g);
    assert!(
        found.iter().any(|(c, _)| (c - mid).abs() < 0.01),
        "the back tab sits at the middle of the back straight: {found:?} (middle {mid})"
    );
    for (c, _) in &found {
        assert!(
            found.iter().any(|(d, _)| (c + d - 2.0 * mid).abs() < 0.01),
            "every tab has its mirror image: {c}"
        );
    }
}

#[test]
fn criterion_19_each_gap_outward_adds_the_joint_gap() {
    let three = rounded(&[
        ("joint_pattern", choice("centred")),
        ("joint_count", count(3)),
        ("joint_gap", mm(8.0)),
    ]);
    let mid = middle(&three);
    let near = |g: &Generated, at: f64| base_notches(g).iter().any(|(c, _)| (c - at).abs() < 0.01);
    for at in [mid - 14.0, mid, mid + 14.0] {
        assert!(near(&three, at), "{at} in {:?}", base_notches(&three));
    }
    let four = rounded(&[
        ("joint_pattern", choice("centred")),
        ("joint_count", count(4)),
        ("joint_gap", mm(7.0)),
    ]);
    let mid = middle(&four);
    for at in [mid - 26.5, mid - 6.5, mid + 6.5, mid + 26.5] {
        assert!(near(&four, at), "{at} in {:?}", base_notches(&four));
    }
}

#[test]
fn criterion_19_spread_blends_from_centred_to_even_and_100_percent_is_even() {
    let make = |pattern: &str, spread: u32| {
        rounded(&[
            ("joint_pattern", choice(pattern)),
            ("joint_count", count(3)),
            ("joint_gap", mm(8.0)),
            ("joint_spread", count(spread)),
        ])
    };
    let (full, even) = (make("centred", 100), make("even", 0));
    let (a, b) = (base_notches(&full), base_notches(&even));
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(&b) {
        assert!(
            (x.0 - y.0).abs() < 1e-6 && (x.1 - y.1).abs() < 1e-6,
            "{x:?} {y:?}"
        );
    }
    let half = make("centred", 50);
    let mid = middle(&half);
    // Back straight 170 mm, three tabs: Even at 56.67 from the middle, Centred at 14.
    let want = f64::midpoint(14.0, 170.0 / 3.0);
    assert!(
        base_notches(&half)
            .iter()
            .any(|(c, _)| (c - (mid + want)).abs() < 0.01),
        "{:?} wants {}",
        base_notches(&half),
        mid + want
    );
}

#[test]
fn criterion_21_every_pattern_and_count_builds_without_overlap() {
    for pattern in ["even", "centred"] {
        for n in 1..=12 {
            for spread in [0, 40, 100] {
                let g = rounded(&[
                    ("joint_pattern", choice(pattern)),
                    ("joint_count", count(n)),
                    ("joint_spread", count(spread)),
                ]);
                let found = base_notches(&g);
                assert!(!found.is_empty(), "{pattern} {n} {spread}");
                for pair in found.windows(2) {
                    let gap = pair[1].0 - pair[0].0 - f64::midpoint(pair[0].1, pair[1].1);
                    assert!(
                        gap > 0.0,
                        "neighbours overlap: {pattern} {n} {spread} {pair:?}"
                    );
                }
            }
        }
    }
}

// ---------- criterion 20: size, count, auto values ----------

#[test]
fn criterion_20_joint_size_is_the_tab_width_in_the_strip_and_the_plates() {
    let g = rounded(&[("joint_width", mm(10.0))]);
    let found = base_notches(&g);
    assert!(found.len() >= 4, "{found:?}");
    for (_, width) in &found {
        assert!((width - 10.0).abs() < 0.01, "slot {width}");
    }
    // The base plate's back tabs: material 10 mm wide just above the footprint's back edge.
    let base = &g.parts[3];
    let b = bounds(base);
    for y in [b.1 + 1.5, b.3 - 1.5] {
        let cross = crossings(base, y);
        for pair in cross.chunks(2) {
            assert!(
                (pair[1] - pair[0] - 10.0).abs() < 0.01,
                "tab {} at y {y}",
                pair[1] - pair[0]
            );
        }
    }
}

#[test]
fn criterion_20_auto_count_and_size_report_the_used_values() {
    let g = rounded(&[]);
    let used = |key: &str| {
        g.automatic
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.clone())
    };
    assert_eq!(used("joint_count"), Some(count(6)));
    assert_eq!(used("joint_width"), Some(mm(2.0 * T)));
    let typed = rounded(&[("joint_count", count(2))]);
    assert!(typed.automatic.iter().all(|(k, _)| *k != "joint_count"));
}

#[test]
fn criterion_20_a_typed_count_replaces_the_automatic_one() {
    let g = rounded(&[("joint_count", count(2))]);
    // Front (seam) 2, back 2, left 2, right 2.
    assert_eq!(base_notches(&g).len(), 8);
    let odd = rounded(&[("joint_count", count(3))]);
    // The seam straight takes four.
    assert_eq!(base_notches(&odd).len(), 13);
}

#[test]
fn criterion_23_a_typed_count_of_two_or_more_per_edge_drops_the_bend_windows() {
    // 100 x 60, radius 30: two straights of 40 mm, too few tabs automatic, so the bends carry windows.
    let build = |extra: &[(&str, ParamValue)]| {
        let mut entries = vec![
            ("inner_width", mm(100.0)),
            ("inner_depth", mm(60.0)),
            ("corner_radius", mm(30.0)),
            ("bend_tabs", choice("window")),
        ];
        entries.extend(extra.iter().cloned());
        generate(ROUNDED_BOX, &stored(&entries))
    };
    let auto = base_notches(&build(&[])).len();
    let typed = base_notches(&build(&[("joint_count", count(1))])).len();
    assert!(auto > 3, "windows add tabs: {auto}");
    assert_eq!(typed, 3, "front (seam) 2 and back 1, no windows");
}

// ---------- criterion 23: clamps ----------

fn clamp<'a>(g: &'a Generated, key: &str) -> Option<&'a curvyo_extension_core::Clamp> {
    g.clamps.iter().find(|c| c.key == key)
}

#[test]
fn criterion_23_a_size_too_big_for_the_shortest_edge_is_lowered_with_a_notice() {
    let g = rounded(&[("joint_width", mm(150.0))]);
    let c = clamp(&g, "joint_width").expect("a clamp");
    assert_eq!(c.bound, Bound::Dependent);
    // The left and right straights are 70 mm: 70 less two wall thicknesses.
    assert_eq!(c.value, mm(64.0));
    assert_eq!(g.parts.len(), rounded(&[]).parts.len());
}

#[test]
fn criterion_23_a_size_below_the_least_is_raised() {
    let g = rounded(&[("joint_width", mm(1.0)), ("kerf", mm(0.2))]);
    let c = clamp(&g, "joint_width").expect("a clamp");
    assert_eq!(c.bound, Bound::DependentMinimum);
    assert_eq!(c.value, mm(1.3));
}

#[test]
fn criterion_23_a_count_too_high_is_lowered_for_both_boxes() {
    let g = rounded(&[("joint_count", count(150))]);
    let c = clamp(&g, "joint_count").expect("a clamp");
    assert_eq!(c.bound, Bound::Dependent);
    let ParamValue::Count(n) = c.value else {
        panic!("a count");
    };
    assert!((1..150).contains(&n));
    let path = generate(PATH_BOX, &stored(&[("joint_count", count(200))]));
    assert!(clamp(&path, "joint_count").is_some());
}

#[test]
fn criterion_23_a_gap_too_wide_is_lowered_and_a_gap_too_narrow_raised() {
    let wide = rounded(&[
        ("joint_pattern", choice("centred")),
        ("joint_count", count(3)),
        ("joint_gap", mm(300.0)),
    ]);
    let c = clamp(&wide, "joint_gap").expect("a clamp");
    assert_eq!(c.bound, Bound::Dependent);
    let narrow = rounded(&[
        ("joint_pattern", choice("centred")),
        ("joint_count", count(3)),
        ("joint_gap", mm(1.0)),
    ]);
    let c = clamp(&narrow, "joint_gap").expect("a clamp");
    assert_eq!(c.bound, Bound::DependentMinimum);
    assert_eq!(c.value, mm(6.0));
}

#[test]
fn criterion_23_a_clamped_box_has_every_tab_inside_the_margins() {
    let g = rounded(&[("joint_count", count(150))]);
    let found = base_notches(&g);
    assert!(found.len() > 8);
    for pair in found.windows(2) {
        assert!(pair[1].0 - pair[0].0 > pair[0].1, "no overlap {pair:?}");
    }
}

// ---------- criterion 24: speed, round trip ----------

#[test]
fn criterion_24_a_generated_box_stores_its_keys_and_regenerates_identically() {
    let first = rounded(&[
        ("joint_pattern", choice("centred")),
        ("joint_count", count(3)),
        ("joint_gap", mm(9.0)),
        ("joint_spread", count(25)),
        ("joint_width", mm(5.0)),
    ]);
    for key in [
        "joint_pattern",
        "joint_count",
        "joint_gap",
        "joint_spread",
        "joint_width",
    ] {
        assert!(first.params.contains_key(key), "{key} stored");
    }
    let again = generate(ROUNDED_BOX, &first.params);
    assert_eq!(dump(&first), dump(&again));
    assert_eq!(first.params, again.params);
}

#[cfg(target_os = "linux")]
fn thread_ticks() -> Option<u64> {
    let stat = std::fs::read_to_string("/proc/thread-self/stat").ok()?;
    let after = stat.rsplit_once(')')?.1;
    let fields: Vec<&str> = after.split_whitespace().collect();
    let user: u64 = fields.get(11)?.parse().ok()?;
    let system: u64 = fields.get(12)?.parse().ok()?;
    Some(user + system)
}

#[cfg(target_os = "linux")]
#[test]
fn criterion_24_regenerating_100_joints_per_edge_costs_under_100_ms_of_thread_cpu() {
    let build = || {
        generate(
            ROUNDED_BOX,
            &stored(&[
                ("inner_width", mm(1200.0)),
                ("inner_depth", mm(1100.0)),
                ("corner_radius", mm(15.0)),
                ("joint_count", count(100)),
                ("joint_width", mm(4.0)),
            ]),
        )
    };
    let g = build();
    assert!(
        clamp(&g, "joint_count").is_none(),
        "100 fit: {:?}",
        g.clamps
    );
    let mut best = u64::MAX;
    for _ in 0..3 {
        let before = thread_ticks().unwrap();
        let _generated = build();
        best = best.min(thread_ticks().unwrap() - before);
    }
    let took = budget_factor::scaled(std::time::Duration::from_millis(best * 10));
    assert!(
        took <= std::time::Duration::from_millis(100),
        "regeneration took {} ms of thread CPU (scaled: {took:?})",
        best * 10
    );
}

#[test]
fn schema_rows_are_known_params() {
    let _: Vec<Param> = GENERATORS
        .manifest()
        .generators()
        .find(|k| k.id == ROUNDED_BOX)
        .unwrap()
        .schema();
}

#[test]
fn criterion_22_the_path_box_takes_the_layout_on_its_straights() {
    let plain = generate(PATH_BOX, &StoredParams::new());
    let one = generate(
        PATH_BOX,
        &stored(&[
            ("joint_pattern", choice("centred")),
            ("joint_count", count(1)),
        ]),
    );
    assert_eq!(plain.parts.len(), one.parts.len());
    assert_ne!(dump(&plain), dump(&one), "one centred tab per straight");
    let widened = generate(PATH_BOX, &stored(&[("joint_width", mm(5.0))]));
    assert_ne!(dump(&plain), dump(&widened));
    assert!(widened.clamps.is_empty(), "{:?}", widened.clamps);
}
