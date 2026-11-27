//! Acceptance of `specs/0155-generator-joint-options`, slice 4 (criteria 25 to 29): the Advanced
//! group and the joint layout of the finger joints of the Box and of the curved top's plates.
//! Black-box: measured on the generated parts.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::many_single_char_names,
    clippy::cast_precision_loss
)]

mod budget_factor;

use curvyo_document_core::Length;
use curvyo_extension_core::{
    Bound, ClosedPath, Extension, GenerateRequest, Generated, ParamKind, ParamValue, Part, Seed,
    StoredParams,
};
use curvyo_generators_core::{BOX, GENERATORS, ROUNDED_BOX};

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
            glyphs: None,
        })
        .unwrap()
}

/// The 120 x 80 x 50 mm box of criterion 29 in 3 mm material with `extra`.
fn the_box(extra: &[(&str, ParamValue)]) -> Generated {
    let mut entries = vec![
        ("inner_width", mm(120.0)),
        ("inner_depth", mm(80.0)),
        ("inner_height", mm(50.0)),
        ("thickness", mm(T)),
    ];
    entries.extend(extra.iter().cloned());
    generate(BOX, &stored(&entries))
}

/// A rounded box with the curved top and `extra`.
fn curved(extra: &[(&str, ParamValue)]) -> Generated {
    let mut entries = vec![
        ("top_shape", choice("curved")),
        ("inner_width", mm(120.0)),
        ("inner_depth", mm(80.0)),
        ("inner_height", mm(60.0)),
        ("corner_radius", mm(20.0)),
        ("thickness", mm(T)),
    ];
    entries.extend(extra.iter().cloned());
    generate(ROUNDED_BOX, &stored(&entries))
}

fn points(path: &ClosedPath) -> Vec<Pt> {
    let mut points = vec![(path.start.x, path.start.y)];
    points.extend(path.segments.iter().map(|c| (c.to.x, c.to.y)));
    points
}

fn dump(generated: &Generated) -> String {
    format!("{:?}", generated.parts)
}

/// The plate's outline points (the box's plates are polygons of straight steps).
fn outline(part: &Part) -> Vec<Pt> {
    points(&part.paths[0])
}

#[derive(Clone, Copy, Debug)]
enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

/// The merged intervals, along `side`, where the outline lies on the extreme line of that side,
/// measured from the plate's own bounding box.
fn extreme(part: &Part, side: Side) -> Vec<(f64, f64)> {
    let pts = outline(part);
    let (x0, x1) = pts
        .iter()
        .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
    let (y0, y1) = pts
        .iter()
        .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.1), a.1.max(p.1)));
    // (coordinate across, line value, origin along)
    let (across, line, origin): (usize, f64, f64) = match side {
        Side::Top => (1, y0, x0),
        Side::Bottom => (1, y1, x0),
        Side::Left => (0, x0, y0),
        Side::Right => (0, x1, y0),
    };
    let get = |p: Pt, axis: usize| if axis == 0 { p.0 } else { p.1 };
    let along = 1 - across;
    let mut runs: Vec<(f64, f64)> = Vec::new();
    for pair in pts.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if (get(a, across) - line).abs() < 1e-6 && (get(b, across) - line).abs() < 1e-6 {
            let (lo, hi) = (
                get(a, along).min(get(b, along)),
                get(a, along).max(get(b, along)),
            );
            if hi - lo > 1e-9 {
                runs.push((lo - origin, hi - origin));
            }
        }
    }
    runs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for run in runs {
        match merged.last_mut() {
            Some(last) if run.0 <= last.1 + 1e-6 => last.1 = last.1.max(run.1),
            _ => merged.push(run),
        }
    }
    merged
}

/// The fingers of one side: the plate keeping the odd pieces shows them as its extreme runs, the
/// owner shows them as the gaps between its extreme runs.
fn fingers(part: &Part, side: Side, owner: bool) -> Vec<(f64, f64)> {
    let runs = extreme(part, side);
    if owner {
        runs.windows(2).map(|w| (w[0].1, w[1].0)).collect()
    } else {
        runs
    }
}

/// `(plate, side, owner, mate plate, mate side, mate owner)` of the 12 joints of the five-plate
/// box, the lid plate (5) mating the walls' top edges.
fn box_joints(lid: bool) -> Vec<(usize, Side, bool, usize, Side, bool)> {
    let mut joints = vec![
        (0, Side::Top, true, 1, Side::Bottom, false),
        (0, Side::Bottom, true, 2, Side::Bottom, false),
        (0, Side::Left, true, 3, Side::Bottom, false),
        (0, Side::Right, true, 4, Side::Bottom, false),
        (1, Side::Left, true, 3, Side::Right, false),
        (1, Side::Right, true, 4, Side::Left, false),
        (2, Side::Right, true, 3, Side::Left, false),
        (2, Side::Left, true, 4, Side::Right, false),
    ];
    if lid {
        joints.extend([
            (5, Side::Top, true, 1, Side::Top, false),
            (5, Side::Bottom, true, 2, Side::Top, false),
            (5, Side::Left, true, 3, Side::Top, false),
            (5, Side::Right, true, 4, Side::Top, false),
        ]);
    }
    joints
}

fn assert_same(a: &[(f64, f64)], b: &[(f64, f64)], what: &str) {
    assert_eq!(a.len(), b.len(), "{what}: {a:?} against {b:?}");
    for (x, y) in a.iter().zip(b) {
        assert!(
            (x.0 - y.0).abs() < 1e-6 && (x.1 - y.1).abs() < 1e-6,
            "{what}: {a:?} against {b:?}"
        );
    }
}

/// Every joint of the box: finger and slot centres agree, the fingers do not overlap and the
/// owner's end pieces lie outside them.
fn assert_box_mates(g: &Generated, lid: bool, what: &str) -> Vec<usize> {
    let mut counts = Vec::new();
    for (p, ps, po, m, ms, mo) in box_joints(lid) {
        let a = fingers(&g.parts[p], ps, po);
        let b = fingers(&g.parts[m], ms, mo);
        assert_same(&a, &b, &format!("{what}: plate {p} {ps:?} / {m} {ms:?}"));
        for w in a.windows(2) {
            assert!(w[1].0 > w[0].1 + 1e-6, "{what}: fingers overlap {a:?}");
        }
        assert!(!a.is_empty(), "{what}: plate {p} {ps:?} has no finger");
        counts.push(a.len());
    }
    counts
}

/// The middle of the span a joint of plate `p` and side `side` runs along.
fn box_middle(p: usize, side: Side, lid: bool) -> f64 {
    let wall_h = 50.0 + T + if lid { T } else { 0.0 };
    match (p, side) {
        (0 | 1 | 2 | 5, Side::Top | Side::Bottom) => f64::midpoint(120.0, 2.0 * T),
        (3 | 4, Side::Top | Side::Bottom) | (0 | 5, _) => f64::midpoint(80.0, 2.0 * T),
        _ => {
            let from = if lid { T } else { 0.0 };
            f64::midpoint(from, wall_h - T)
        }
    }
}

// ---------- criterion 25: the group, the keys, the defaults ----------

fn kind_groups(id: &str) -> Vec<curvyo_extension_core::ParamGroup> {
    GENERATORS
        .manifest()
        .generators()
        .find(|k| k.id == id)
        .unwrap()
        .groups
        .clone()
}

#[test]
fn criterion_25_the_box_has_an_advanced_group_with_finger_width_as_the_joint_size() {
    let groups = kind_groups(BOX);
    let last = groups.last().unwrap();
    assert_eq!(last.label, "Advanced");
    let keys: Vec<&str> = last.params.iter().map(|p| p.key).collect();
    assert_eq!(
        keys,
        [
            "joint_pattern",
            "joint_count",
            "finger_width",
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
    for param in &last.params[3..] {
        assert!(param.shown_when.is_some(), "{} is conditional", param.key);
    }
    // Shown once: Finger width is in no other group, nor is the old Joint choice anywhere.
    let all: Vec<&str> = groups
        .iter()
        .flat_map(|g| g.params.iter().map(|p| p.key))
        .collect();
    assert_eq!(all.iter().filter(|k| **k == "finger_width").count(), 1);
    assert!(!all.contains(&"joint"), "the Joint choice is gone");
    assert!(!all.contains(&"joint_width"));
    let size = last.params[2].default.clone();
    assert_eq!(size, mm(6.0), "finger_width keeps its default");
    assert!(matches!(
        last.params[1].kind,
        ParamKind::Count {
            automatic: true,
            ..
        }
    ));
}

#[test]
fn criterion_25_the_curved_tops_rows_show_for_both_tops() {
    let groups = kind_groups(ROUNDED_BOX);
    let last = groups.last().unwrap();
    assert_eq!(last.label, "Advanced");
    for param in &last.params {
        let text = format!("{:?}", param.shown_when);
        assert!(
            !text.contains("flat") && !text.contains("curved"),
            "{} hangs on the top shape: {text}",
            param.key
        );
    }
}

#[test]
fn criterion_25_the_defaults_equal_the_output_before() {
    let plain = the_box(&[]);
    let explicit = the_box(&[
        ("joint_pattern", choice("even")),
        ("joint_count", count(0)),
        ("joint_gap", mm(0.0)),
        ("joint_spread", count(0)),
    ]);
    assert_eq!(dump(&plain), dump(&explicit));
    let plain = curved(&[]);
    let explicit = curved(&[
        ("joint_pattern", choice("even")),
        ("joint_count", count(0)),
        ("joint_width", mm(0.0)),
    ]);
    assert_eq!(dump(&plain), dump(&explicit));
    assert!(plain.clamps.is_empty(), "no notice at the defaults");
    assert_eq!(the_box(&[]).clamps, []);
}

#[test]
fn criterion_25_a_stored_joint_none_reads_as_fingers() {
    let old = the_box(&[("joint", choice("none"))]);
    assert_eq!(dump(&old), dump(&the_box(&[])));
}

// ---------- criteria 26 and 27: centres agree, the rest is the owner's ----------

#[test]
fn criterion_26_count_one_is_the_middle_of_every_edge_for_every_pattern() {
    for lid in ["none", "fingers"] {
        for pattern in ["even", "centred"] {
            let g = the_box(&[
                ("joint_pattern", choice(pattern)),
                ("joint_count", count(1)),
                ("lid", choice(lid)),
            ]);
            let with_lid = lid == "fingers";
            let counts = assert_box_mates(&g, with_lid, &format!("{pattern} {lid}"));
            assert!(counts.iter().all(|n| *n == 1), "{counts:?}");
            for (p, ps, po, _, _, _) in box_joints(with_lid) {
                let f = fingers(&g.parts[p], ps, po);
                let at = f64::midpoint(f[0].0, f[0].1);
                let want = box_middle(p, ps, with_lid);
                assert!(
                    (at - want).abs() < 0.01,
                    "{pattern} {lid}: plate {p} {ps:?} finger at {at}, middle {want}"
                );
                assert!((f[0].1 - f[0].0 - 6.0).abs() < 1e-6, "Joint size 6: {f:?}");
            }
        }
    }
}

#[test]
fn criterion_26_every_pattern_and_count_mates_without_overlap() {
    for pattern in ["even", "centred"] {
        for n in 1..=5 {
            for (gap, spread) in [(0.0, 0), (12.0, 0), (0.0, 50), (0.0, 100)] {
                let g = the_box(&[
                    ("joint_pattern", choice(pattern)),
                    ("joint_count", count(n)),
                    ("joint_gap", mm(gap)),
                    ("joint_spread", count(spread)),
                    ("lid", choice("fingers")),
                ]);
                let what = format!("{pattern} {n} gap {gap} spread {spread}");
                let counts = assert_box_mates(&g, true, &what);
                assert!(counts.iter().all(|c| *c >= 1), "{what}");
                // The 50 mm edges hold what the tightest edge allows, never more than asked.
                assert!(
                    counts.iter().all(|c| *c <= n as usize),
                    "{what}: {counts:?}"
                );
            }
        }
    }
}

#[test]
fn criterion_26_the_owners_end_pieces_stay_solid() {
    let g = the_box(&[
        ("joint_pattern", choice("centred")),
        ("joint_count", count(3)),
    ]);
    for (p, ps, po, _, _, _) in box_joints(false) {
        if !po {
            continue;
        }
        let runs = extreme(&g.parts[p], ps);
        assert!(runs.len() >= 2, "plate {p} {ps:?}: {runs:?}");
        // First and last run (the corner cube and the first piece) are longer than the thickness.
        assert!(runs[0].1 - runs[0].0 > T + 1e-6, "{runs:?}");
        let last = runs[runs.len() - 1];
        assert!(last.1 - last.0 > T + 1e-6, "{runs:?}");
    }
}

fn segment_distance(a: (Pt, Pt), b: (Pt, Pt)) -> f64 {
    fn cross(o: Pt, p: Pt, q: Pt) -> f64 {
        (p.0 - o.0) * (q.1 - o.1) - (p.1 - o.1) * (q.0 - o.0)
    }
    fn point_to(p: Pt, s: (Pt, Pt)) -> f64 {
        let (dx, dy) = (s.1.0 - s.0.0, s.1.1 - s.0.1);
        let len2 = dx * dx + dy * dy;
        let u = if len2 < 1e-18 {
            0.0
        } else {
            (((p.0 - s.0.0) * dx + (p.1 - s.0.1) * dy) / len2).clamp(0.0, 1.0)
        };
        (p.0 - (s.0.0 + u * dx)).hypot(p.1 - (s.0.1 + u * dy))
    }
    let crossing = cross(a.0, a.1, b.0) * cross(a.0, a.1, b.1) < 0.0
        && cross(b.0, b.1, a.0) * cross(b.0, b.1, a.1) < 0.0;
    if crossing {
        return 0.0;
    }
    point_to(a.0, b)
        .min(point_to(a.1, b))
        .min(point_to(b.0, a))
        .min(point_to(b.1, a))
}

/// The least distance between two segments that are not neighbours on the outline.
fn least_clearance(part: &Part) -> f64 {
    let pts = outline(part);
    let n = pts.len() - 1;
    let mut least = f64::MAX;
    for i in 0..n {
        for j in i + 2..n {
            if i == 0 && j == n - 1 {
                continue;
            }
            least = least.min(segment_distance(
                (pts[i], pts[i + 1]),
                (pts[j], pts[(j + 1) % (n + 1)]),
            ));
        }
    }
    least
}

#[test]
fn criterion_27_every_pattern_builds_closed_plates_with_no_contour_closer_than_the_kerf() {
    let kerf = 0.2;
    for pattern in ["even", "centred"] {
        for n in [0, 1, 2, 3, 5] {
            for lid in ["none", "flat", "fingers"] {
                let g = the_box(&[
                    ("joint_pattern", choice(pattern)),
                    ("joint_count", count(n)),
                    ("lid", choice(lid)),
                    ("kerf", mm(kerf)),
                ]);
                for (index, part) in g.parts.iter().enumerate() {
                    assert_eq!(part.paths.len(), 1, "one closed outline per plate");
                    let pts = outline(part);
                    assert_eq!(pts.first(), pts.last(), "plate {index} is closed");
                    let least = least_clearance(part);
                    assert!(
                        least >= kerf - 1e-9,
                        "{pattern} {n} {lid}: plate {index} has contours {least} apart"
                    );
                }
            }
        }
    }
}

// ---------- criterion 28: clamps, state, speed ----------

fn clamp<'a>(g: &'a Generated, key: &str) -> Option<&'a curvyo_extension_core::Clamp> {
    g.clamps.iter().find(|c| c.key == key)
}

#[test]
fn criterion_28_a_count_too_high_is_lowered_with_a_notice() {
    for pattern in ["even", "centred"] {
        let g = the_box(&[
            ("joint_pattern", choice(pattern)),
            ("joint_count", count(150)),
        ]);
        let c = clamp(&g, "joint_count").expect("a clamp");
        assert_eq!(c.bound, Bound::Dependent);
        let ParamValue::Count(n) = c.value else {
            panic!("a count");
        };
        assert!((1..150).contains(&n));
        let counts = assert_box_mates(&g, false, pattern);
        assert!(
            counts.iter().all(|c| *c == n as usize),
            "{counts:?} against {n}"
        );
    }
}

#[test]
fn criterion_28_a_gap_too_wide_is_lowered_and_the_curved_top_clamps_too() {
    let g = the_box(&[
        ("joint_pattern", choice("centred")),
        ("joint_count", count(3)),
        ("joint_gap", mm(300.0)),
    ]);
    let c = clamp(&g, "joint_gap").expect("a clamp");
    assert_eq!(c.bound, Bound::Dependent);
    assert_box_mates(&g, false, "wide gap");
    let g = curved(&[("joint_count", count(150))]);
    assert!(clamp(&g, "joint_count").is_some(), "{:?}", g.clamps);
}

#[test]
fn criterion_28_auto_count_reports_the_used_value_and_a_stored_box_regenerates_identically() {
    let g = the_box(&[("joint_pattern", choice("centred"))]);
    assert!(g.automatic.iter().any(|(k, _)| *k == "joint_count"));
    let first = the_box(&[
        ("joint_pattern", choice("centred")),
        ("joint_count", count(2)),
        ("joint_gap", mm(14.0)),
        ("joint_spread", count(30)),
    ]);
    let again = generate(BOX, &first.params);
    assert_eq!(dump(&first), dump(&again));
    assert_eq!(first.params, again.params);
    for key in [
        "joint_pattern",
        "joint_count",
        "finger_width",
        "joint_gap",
        "joint_spread",
    ] {
        assert!(first.params.contains_key(key), "{key} is stored");
    }
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
fn criterion_28_regenerating_100_fingers_per_edge_costs_under_100_ms_of_thread_cpu() {
    let build = || {
        generate(
            BOX,
            &stored(&[
                ("inner_width", mm(1200.0)),
                ("inner_depth", mm(1100.0)),
                ("inner_height", mm(1000.0)),
                ("joint_count", count(100)),
                ("finger_width", mm(4.0)),
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

// ---------- criterion 29: the demo box ----------

#[test]
fn criterion_29_the_demo_box_has_exactly_one_finger_per_edge() {
    for lid in ["none", "fingers"] {
        let g = the_box(&[
            ("joint_pattern", choice("centred")),
            ("joint_count", count(1)),
            ("lid", choice(lid)),
        ]);
        let with_lid = lid == "fingers";
        assert_eq!(g.parts.len(), if with_lid { 6 } else { 5 });
        let counts = assert_box_mates(&g, with_lid, lid);
        assert!(counts.iter().all(|n| *n == 1), "{counts:?}");
        for part in &g.parts {
            assert_eq!(part.paths.len(), 1);
            let pts = outline(part);
            assert_eq!(pts.first(), pts.last(), "closed");
            assert!(least_clearance(part) > 1.0);
        }
    }
}

// ---------- the curved top ----------

/// `(plate, side, owner, mate, mate side, mate owner)` of the curved top's joints; the parts are
/// the strip (0), the two end plates (1, 2), the front (3) and the base (4).
fn curved_joints() -> Vec<(usize, Side, bool, usize, Side, bool)> {
    vec![
        (3, Side::Left, true, 1, Side::Left, false),
        (3, Side::Right, true, 2, Side::Left, false),
        (4, Side::Top, true, 3, Side::Bottom, false),
        (4, Side::Left, true, 1, Side::Bottom, false),
        (4, Side::Right, true, 2, Side::Bottom, false),
    ]
}

#[test]
fn criterion_25_curved_top_plates_read_the_layout_and_mate() {
    for pattern in ["even", "centred"] {
        for n in [1, 2, 3] {
            let g = curved(&[
                ("joint_pattern", choice(pattern)),
                ("joint_count", count(n)),
            ]);
            assert_eq!(g.parts.len(), 5);
            // The tightest joint (the end plate's tab run) may lower the count, with a notice.
            let want = match clamp(&g, "joint_count").map(|c| c.value.clone()) {
                Some(ParamValue::Count(lowered)) => lowered,
                _ => n,
            };
            assert!(want >= 1);
            for (p, ps, po, m, ms, mo) in curved_joints() {
                let a = fingers(&g.parts[p], ps, po);
                let b = fingers(&g.parts[m], ms, mo);
                assert_same(&a, &b, &format!("{pattern} {n}: {p} {ps:?} / {m} {ms:?}"));
                assert_eq!(a.len(), want as usize, "{pattern} {n}: {p} {ps:?}");
            }
        }
    }
}

#[test]
fn criterion_26_curved_top_count_one_is_the_middle_of_each_joint() {
    let g = curved(&[
        ("joint_pattern", choice("centred")),
        ("joint_count", count(1)),
    ]);
    // The base's top edge: across the width between the end plates, then the depth between the
    // front and the strip, the front's side: the height above the base.
    let base = &g.parts[4];
    let top = fingers(base, Side::Top, true);
    let outer = 120.0 + 2.0 * T;
    assert!(
        (f64::midpoint(top[0].0, top[0].1) - outer / 2.0).abs() < 0.01,
        "{top:?}"
    );
    let side = fingers(base, Side::Left, true);
    let depth = 80.0 + 2.0 * T + 0.0;
    let at = f64::midpoint(side[0].0, side[0].1);
    // Between the front's thickness and the strip's: the middle of that span.
    assert!(
        (at - (T + (depth - T - T) / 2.0)).abs() < 0.01,
        "{side:?} {depth}"
    );
}

#[test]
fn criterion_28_curved_top_plates_stay_closed_with_clearance_for_every_pattern() {
    for pattern in ["even", "centred"] {
        for n in [0, 1, 3] {
            let g = curved(&[
                ("joint_pattern", choice(pattern)),
                ("joint_count", count(n)),
                ("kerf", mm(0.2)),
            ]);
            for (index, part) in g.parts.iter().enumerate().skip(1) {
                let pts = outline(part);
                assert_eq!(pts.first(), pts.last(), "plate {index} is closed");
                assert!(
                    least_clearance(part) >= 0.2 - 1e-9,
                    "{pattern} {n} plate {index}"
                );
            }
        }
    }
}

#[test]
fn criterion_25_curved_top_end_plate_tabs_and_strip_end_follow_the_layout() {
    let g = curved(&[
        ("joint_pattern", choice("centred")),
        ("joint_count", count(1)),
    ]);
    // The end plate's back edge carries the tabs; one tab, in the middle of the back straight
    // (from two wall thicknesses below the arc to two above the base).
    for plate in [1, 2] {
        let tabs = fingers(&g.parts[plate], Side::Right, false);
        assert_eq!(tabs.len(), 1, "{tabs:?}");
        let (from, to) = (20.0 + 2.0 * T, 60.0 - 2.0 * T);
        let at = f64::midpoint(tabs[0].0, tabs[0].1);
        assert!((at - f64::midpoint(from, to)).abs() < 0.01, "{tabs:?}");
        assert!((tabs[0].1 - tabs[0].0 - 2.0 * T).abs() < 1e-6, "{tabs:?}");
    }
    // The strip's end has the same single finger as the base's back edge.
    let base = fingers(&g.parts[4], Side::Bottom, true);
    let strip = fingers(&g.parts[0], Side::Right, false);
    assert_eq!(base.len(), 1);
    assert_same(&base, &strip, "strip end against base back edge");
}
