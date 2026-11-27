//! Acceptance of `specs/0155-generator-joint-options`, slice 2 (criteria 10 to 17): Bend through
//! tabs. The plate keeps its true arc, the slit columns run on through the tab, and a column near a
//! slot is clipped one bridge from the slot's rows. Black-box: measured on the generated parts.

#![allow(clippy::unwrap_used, clippy::many_single_char_names)]

mod budget_factor;

use curvyo_document_core::{Length, Point};
use curvyo_extension_core::{
    ClosedPath, Cubic, Extension, GenerateRequest, Generated, Param, ParamKind, ParamValue, Part,
    Seed, StoredParams,
};
use curvyo_generators_core::{BOX, GENERATORS, PATH_BOX, ROUNDED_BOX};

type Pt = (f64, f64);

const T: f64 = 3.0;
const PI: f64 = std::f64::consts::PI;

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

fn generate(kind: &str, params: &StoredParams) -> Generated {
    GENERATORS
        .generate(&GenerateRequest {
            kind,
            params,
            seed: Seed(1),
            edit: None,
            glyphs: None,
        })
        .unwrap()
}

/// A rounded box of `w x d` with radius `r`, thickness `t` and the bend option `bend`.
fn rounded(w: f64, d: f64, r: f64, t: f64, bend: &str, extra: &[(&str, ParamValue)]) -> Generated {
    let mut entries = vec![
        ("inner_width", mm(w)),
        ("inner_depth", mm(d)),
        ("corner_radius", mm(r)),
        ("thickness", mm(t)),
        ("bend_tabs", choice(bend)),
    ];
    entries.extend(extra.iter().cloned());
    generate(ROUNDED_BOX, &stored(&entries))
}

fn circle(bend: &str) -> Generated {
    rounded(60.0, 60.0, 30.0, T, bend, &[])
}

fn sample(path: &ClosedPath) -> Vec<Pt> {
    const STEPS: u32 = 16;
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

/// The cuts (gaps of material) a horizontal line at height `y` meets in the strip, as `(from, to)`.
fn cuts_at(part: &Part, y: f64) -> Vec<(f64, f64)> {
    let mut cross: Vec<f64> = Vec::new();
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
    // Material is between the 1st and 2nd crossing (outline), cuts between the 2nd and 3rd, ...
    cross[1..]
        .chunks(2)
        .filter(|pair| pair.len() == 2)
        .map(|pair| (pair[0], pair[1]))
        .collect()
}

/// The distinct centres of the tab notches (cuts wider than 4 mm) in the strip.
fn notch_centres(part: &Part, y: f64, thickness: f64) -> Vec<f64> {
    let mut centres: Vec<f64> = cuts_at(part, y)
        .into_iter()
        .filter(|(a, b)| b - a > thickness)
        .map(|(a, b)| f64::midpoint(a, b))
        .collect();
    centres.sort_by(f64::total_cmp);
    centres
}

/// The tabbed plates in the order the rounded box lays its parts out: strip, cover, lid (seat,
/// plain), base, ring.
fn tabbed_plates(generated: &Generated) -> Vec<&Part> {
    vec![&generated.parts[3], &generated.parts[4]]
}

// ---------- criterion 10: the option and its default ----------

fn schema_of(id: &str) -> Vec<Param> {
    let manifest = GENERATORS.manifest();
    let kind = manifest.generators().find(|k| k.id == id).unwrap();
    kind.schema()
}

#[test]
fn criterion_10_bend_tabs_is_a_choice_in_the_slits_group_of_both_boxes_and_not_in_the_box() {
    for id in [ROUNDED_BOX, PATH_BOX] {
        let param = schema_of(id)
            .into_iter()
            .find(|p| p.key == "bend_tabs")
            .unwrap();
        assert_eq!(param.label, "Bend through tabs");
        let ParamKind::Choice { options } = &param.kind else {
            panic!("a choice");
        };
        assert_eq!(
            options.iter().map(|o| o.id).collect::<Vec<_>>(),
            ["window", "curve"]
        );
        assert_eq!(param.default, choice("window"));
        assert_eq!(param.initial, Some(choice("curve")));
    }
    assert!(schema_of(BOX).iter().all(|p| p.key != "bend_tabs"));
}

#[test]
fn criterion_10_a_new_box_stores_curve_and_a_stored_box_without_the_key_reads_window() {
    for id in [ROUNDED_BOX, PATH_BOX] {
        let schema = schema_of(id);
        let fresh = Param::defaults(&schema);
        assert_eq!(fresh["bend_tabs"], choice("curve").to_stored());
        let bend = schema.iter().find(|p| p.key == "bend_tabs").unwrap();
        assert_eq!(bend.read(&StoredParams::new()), choice("window"), "no key");
        assert_eq!(
            bend.read(&stored(&[("bend_tabs", choice("sideways"))])),
            choice("window"),
            "an unknown id"
        );
    }
}

#[test]
fn criterion_10_a_missing_key_builds_the_window_and_an_unknown_id_too() {
    let missing = generate(
        ROUNDED_BOX,
        &stored(&[
            ("inner_width", mm(60.0)),
            ("inner_depth", mm(60.0)),
            ("corner_radius", mm(30.0)),
        ]),
    );
    assert_eq!(dump(&missing), dump(&circle("window")));
    assert_eq!(dump(&circle("sideways")), dump(&circle("window")));
}

// ---------- criterion 11: the plate keeps its arc ----------

#[test]
fn criterion_11_every_plate_point_is_on_the_arc_or_on_a_tab_rectangle() {
    let g = circle("curve");
    let centre = (0.0, 0.0);
    let mut tabs = 0;
    for plate in tabbed_plates(&g) {
        // The plate's own centre is its bounding box centre.
        let b = bounds(plate);
        let c = (f64::midpoint(b.0, b.2), f64::midpoint(b.1, b.3));
        let _ = centre;
        let mut over = Vec::new();
        for (x, y) in sample(&plate.paths[0]) {
            let d = libm::hypot(x - c.0, y - c.1);
            // Inside the outer arc r + t_w, never inside the arc r by more than 0.01.
            assert!((30.0 - 3.5..=30.0 + T + 0.01).contains(&d), "d = {d}");
            if d > 30.0 + 0.01 {
                over.push(libm::atan2(y - c.1, x - c.0));
            }
        }
        if !over.is_empty() {
            tabs += 1;
            // Six tabs at 60 degree spacing: the over-points cluster in 6 groups.
            over.sort_by(f64::total_cmp);
            let gaps = over.windows(2).filter(|w| w[1] - w[0] > 0.15).count();
            assert!(gaps + 1 >= 6 && gaps <= 8, "tab groups {}", gaps + 1);
        }
    }
    assert!(tabs >= 2, "the base and the lid carry tabs");
}

#[test]
fn criterion_11_a_tab_is_a_rectangle_on_the_normal_with_a_concentric_tip() {
    let g = circle("curve");
    let plate = tabbed_plates(&g)[0];
    let b = bounds(plate);
    let c = (f64::midpoint(b.0, b.2), f64::midpoint(b.1, b.3));
    // The tab tips are on the circle r + t (flush outside): the farthest point is at 33 mm.
    let far = plate
        .paths
        .iter()
        .flat_map(sample)
        .map(|(x, y)| libm::hypot(x - c.0, y - c.1))
        .fold(0.0, f64::max);
    assert!((far - (30.0 + T)).abs() < 0.2, "far {far}");
}

#[test]
fn criterion_11_the_strip_has_no_window_so_columns_stand_across_the_tabs() {
    let g = circle("curve");
    let strip = strip(&g);
    let (_, y0, _, y1) = bounds(strip);
    let mid = f64::midpoint(y0, y1);
    let narrow: Vec<f64> = cuts_at(strip, mid)
        .into_iter()
        .filter(|(a, b)| b - a < 4.0)
        .map(|(a, b)| f64::midpoint(a, b))
        .collect();
    assert!(narrow.len() >= 20, "{} columns", narrow.len());
    let widest = narrow.windows(2).map(|w| w[1] - w[0]).fold(0.0, f64::max);
    // The window box has a flat of about 12 mm across each tab; here no pitch beyond a step.
    assert!(widest < 9.0, "widest pitch {widest}");
}

// ---------- criterion 12: bridges round the slot ----------

#[test]
fn criterion_12_every_column_keeps_a_bridge_t_w_from_the_tab_slots() {
    for pattern in ["straight", "offset", "wave"] {
        let g = rounded(
            60.0,
            60.0,
            30.0,
            T,
            "curve",
            &[("slit_pattern", choice(pattern))],
        );
        let strip = strip(&g);
        let (_, y0, _, y1) = bounds(strip);
        // Along the rows of a slot and the bridge beside them (two thicknesses from either edge),
        // no cut comes nearer than a bridge to the slot.
        for (lo, hi) in [(y0, y0 + 2.0 * T), (y1 - 2.0 * T, y1)] {
            let mut y = lo + 0.05;
            while y < hi {
                let cuts = cuts_at(strip, y);
                for slot in cuts.iter().filter(|(a, b)| b - a > 4.0) {
                    for cut in cuts.iter().filter(|(a, b)| b - a <= 4.0) {
                        let gap = (cut.0 - slot.1).max(slot.0 - cut.1);
                        assert!(
                            gap >= T - 0.06,
                            "{pattern}: a cut is {gap:.3} mm from a slot at height {y:.2}"
                        );
                    }
                }
                y += 0.1;
            }
        }
    }
}

#[test]
fn criterion_12_a_clipped_column_is_kept_not_dropped() {
    // Across the slot's width, below its bridge, the column is still there (a window would drop it).
    let g = circle("curve");
    let strip = strip(&g);
    let (_, y0, _, y1) = bounds(strip);
    let notches = notch_centres(strip, y1 - T / 2.0, T);
    assert!(notches.len() >= 6);
    let mid = f64::midpoint(y0, y1);
    let narrow: Vec<f64> = cuts_at(strip, mid)
        .into_iter()
        .filter(|(a, b)| b - a < 4.0)
        .map(|(a, b)| f64::midpoint(a, b))
        .collect();
    for centre in notches {
        assert!(
            narrow.iter().any(|x| (x - centre).abs() < 8.0),
            "a column stands within a step of the slot at {centre:.1}"
        );
    }
}

#[test]
fn criterion_12_no_band_across_a_bend_is_left_uncut() {
    for pattern in ["straight", "offset", "wave"] {
        let g = rounded(
            60.0,
            60.0,
            30.0,
            T,
            "curve",
            &[("slit_pattern", choice(pattern))],
        );
        let strip = strip(&g);
        let (_, y0, _, y1) = bounds(strip);
        let mut y = y0 + 0.05;
        while y < y1 {
            let narrow = cuts_at(strip, y).into_iter().any(|(a, b)| b - a < 4.0);
            assert!(narrow, "{pattern}: a line at height {y:.2} meets no slit");
            y += 0.1;
        }
    }
}

// ---------- criterion 13: the strip length without a window shortening ----------

#[test]
fn criterion_13_the_circle_strip_is_the_neutral_perimeter_within_a_tenth() {
    for t in [1.5, 2.0, 3.0, 4.0, 5.0] {
        let g = rounded(60.0, 60.0, 30.0, t, "curve", &[]);
        let b = bounds(strip(&g));
        let expected = PI * (2.0 * 30.0 + t);
        assert!(
            ((b.2 - b.0) - expected).abs() < 0.1,
            "t {t}: {} against {expected}",
            b.2 - b.0
        );
    }
}

// ---------- criterion 14: mating ----------

#[test]
fn criterion_14_tab_and_slot_centres_agree_by_arc_length_at_five_thicknesses() {
    for t in [1.5, 2.0, 3.0, 4.0, 5.0] {
        let g = rounded(60.0, 60.0, 30.0, t, "curve", &[]);
        let strip = strip(&g);
        let (x0, _, x1, y1) = bounds(strip);
        let length = x1 - x0;
        let notches = notch_centres(strip, y1 - t / 2.0, t);
        assert!(!notches.is_empty(), "t {t}");
        let count = notches.len();
        // Equal spacing along the neutral line, the first half a step from the seam.
        #[allow(clippy::cast_precision_loss)]
        for (k, centre) in notches.iter().enumerate() {
            let want = x0 + length / count as f64 * (k as f64 + 0.5);
            assert!(
                (centre - want).abs() < 0.05,
                "t {t}: slot {k} at {centre} not {want}"
            );
        }
        // The base plate's tabs stand at the same fractions of the circle's turn (the seam is at
        // the top, `-90` degrees; the plate's walk is clockwise on screen).
        // The ring (part 4) has a round opening whose bounding box centre is the circle's centre.
        let plate = &g.parts[4];
        let hole = bounds(&Part {
            key: "ring".into(),
            paths: vec![plate.paths[1].clone()],
            paint: None,
        });
        let c = (f64::midpoint(hole.0, hole.2), f64::midpoint(hole.1, hole.3));
        let angles: Vec<f64> = plate
            .paths
            .iter()
            .flat_map(sample)
            .filter(|&(x, y)| libm::hypot(x - c.0, y - c.1) > 30.0 + t - 0.05)
            .map(|(x, y)| libm::atan2(y - c.1, x - c.0))
            .collect();
        // The seam is at the top (-90 degrees) and the walk runs clockwise on screen, so tab k
        // stands at -90 degrees + (k + 1/2) of a turn / count (arc length on the neutral line).
        assert!(
            angles.len() >= count,
            "t {t}: plate tabs are as many as slots"
        );
        #[allow(clippy::cast_precision_loss)]
        for k in 0..count {
            let want = -PI / 2.0 + 2.0 * PI * (k as f64 + 0.5) / count as f64;
            let near: Vec<f64> = angles
                .iter()
                .map(|a| (a - want + PI).rem_euclid(2.0 * PI) - PI)
                .filter(|d| d.abs() < 0.3)
                .collect();
            assert!(!near.is_empty(), "t {t}: no plate tab near {want:.3}");
            let lo = near.iter().copied().fold(f64::MAX, f64::min);
            let hi = near.iter().copied().fold(f64::MIN, f64::max);
            let mean = f64::midpoint(lo, hi);
            // 0.01 rad is 0.3 mm of arc at r 30: the kerf is 0 here.
            assert!(
                mean.abs() < 0.01,
                "t {t}: plate tab {k} is {mean:.4} rad off its slot"
            );
        }
    }
}

// ---------- criterion 15: nothing else changes ----------

#[test]
fn criterion_15_a_box_without_a_tab_on_a_bend_is_the_same_with_either_choice() {
    let default = StoredParams::new();
    let with = |id: &str| {
        let mut params = default.clone();
        params.insert("bend_tabs".to_owned(), choice(id).to_stored());
        dump(&generate(ROUNDED_BOX, &params))
    };
    assert_eq!(with("curve"), with("window"));
    // The curved top has tabs on its straight back only.
    let curved = |id: &str| {
        dump(&generate(
            ROUNDED_BOX,
            &stored(&[("top_shape", choice("curved")), ("bend_tabs", choice(id))]),
        ))
    };
    assert_eq!(curved("curve"), curved("window"));
}

#[test]
fn criterion_15_window_is_slice_one_byte_for_byte_and_curve_differs_on_the_circle() {
    let window = dump(&circle("window"));
    let golden = std::fs::read_to_string(format!(
        "{}/tests/fixtures/rounded_box_circle.txt",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    assert_eq!(window, golden);
    assert_ne!(dump(&circle("curve")), window);
}

#[test]
fn criterion_15_the_curve_goldens_are_frozen() {
    golden("rounded_box_circle_curve.txt", &dump(&circle("curve")));
    let heart = heart();
    let g = generate(
        PATH_BOX,
        &stored(&[
            ("outline", ParamValue::Path(heart)),
            ("bend_tabs", choice("curve")),
        ]),
    );
    golden("path_box_heart_curve.txt", &dump(&g));
}

// ---------- criterion 16: a small radius never errors ----------

#[test]
fn criterion_16_small_radii_build_with_closed_parts_and_never_fail() {
    for (w, r) in [
        (20.0, 6.0),
        (24.0, 12.0),
        (30.0, 15.0),
        (40.0, 20.0),
        (70.0, 35.0),
    ] {
        for bend in ["curve", "window"] {
            let g = rounded(w, w, r, T, bend, &[]);
            assert!(!g.parts.is_empty(), "{w} x {r} {bend}");
            for path in g.parts.iter().flat_map(|p| p.paths.iter()) {
                let last = path.segments.last().unwrap().to;
                assert!(
                    libm::hypot(last.x - path.start.x, last.y - path.start.y) < 1e-6,
                    "{w} x {r} {bend}: a path is not closed"
                );
            }
        }
    }
    let g = generate(
        PATH_BOX,
        &stored(&[
            ("outline", ParamValue::Path(heart())),
            ("bend_tabs", choice("curve")),
            ("thickness", mm(6.0)),
        ]),
    );
    assert_ne!(g.parts.len(), 0);
}

// ---------- criterion 17: thread CPU time ----------

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
fn criterion_17_regenerating_a_curve_box_of_about_400_slots_costs_under_100_ms_of_thread_cpu() {
    let build = || {
        rounded(
            200.0,
            100.0,
            20.0,
            T,
            "curve",
            &[
                ("inner_height", mm(100.0)),
                ("wall_slits", choice("all")),
                ("slit_spacing", mm(1.5)),
            ],
        )
    };
    let _warm = build();
    let mut best = u64::MAX;
    for _ in 0..3 {
        let before = thread_ticks().unwrap();
        let _generated = build();
        let after = thread_ticks().unwrap();
        best = best.min(after - before);
    }
    let took = budget_factor::scaled(std::time::Duration::from_millis(best * 10));
    assert!(
        took <= std::time::Duration::from_millis(100),
        "regeneration took {} ms of thread CPU (scaled: {took:?})",
        best * 10
    );
}

// ---------- helpers: dump, golden, heart ----------

fn dump(generated: &Generated) -> String {
    use std::fmt::Write;
    let mut text = String::new();
    for (index, part) in generated.parts.iter().enumerate() {
        for path in &part.paths {
            writeln!(
                text,
                "part {index} start {:.6} {:.6}",
                path.start.x, path.start.y
            )
            .unwrap();
            for s in &path.segments {
                writeln!(
                    text,
                    "{:.6} {:.6} {:.6} {:.6} {:.6} {:.6}",
                    s.c1.x, s.c1.y, s.c2.x, s.c2.y, s.to.x, s.to.y
                )
                .unwrap();
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

fn heart() -> ClosedPath {
    let p = |x: f64, y: f64| Point::new(x, y);
    let c = |c1: Pt, c2: Pt, to: Pt| Cubic {
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
