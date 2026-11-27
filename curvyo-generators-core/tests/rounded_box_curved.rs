//! Acceptance of the curved top (`specs/0147-rounded-box` slice 2, criteria 11 and 16 to 24; ADR 0082
//! §11): the parts and their numbers, the strip's length and slits, the joints, the clamps, the
//! rows each Top shape shows, the layout and the golden.

#![allow(
    clippy::assert_is_empty,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::many_single_char_names,
    clippy::float_cmp,
    clippy::format_push_string
)]

mod budget_factor;

use curvyo_document_core::Length;
use curvyo_extension_core::{
    Bound, ClosedPath, Contribution, Edit, Extension, GenerateRequest, Generated, GeneratorKind,
    Mark, ParamValue, Part, ScaleFactor, Seed, ShownWhen, StoredParams,
};
use curvyo_generators_core::{GENERATORS, PATH_BOX, ROUNDED_BOX, manifest};

fn mm(value: f64) -> ParamValue {
    ParamValue::Length(Length::from_mm(value))
}

fn choice(id: &str) -> ParamValue {
    ParamValue::Choice(id.to_owned())
}

fn params(entries: &[(&str, ParamValue)]) -> StoredParams {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.to_stored()))
        .collect()
}

/// The defaults of the curved box with `entries` on top.
fn curved(entries: &[(&str, ParamValue)]) -> StoredParams {
    let mut all = vec![("top_shape", choice("curved"))];
    all.extend(entries.iter().cloned());
    params(&all)
}

fn generate(stored: &StoredParams, edit: Option<Edit<'_>>) -> Generated {
    GENERATORS
        .generate(&GenerateRequest {
            kind: ROUNDED_BOX,
            params: stored,
            seed: Seed(1),
            edit,
            glyphs: None,
        })
        .unwrap()
}

fn kind_of(id: &str) -> GeneratorKind {
    manifest()
        .contributions
        .into_iter()
        .find_map(|contribution| match contribution {
            Contribution::Generator(kind) if kind.id == id => Some(kind),
            _ => None,
        })
        .unwrap()
}

/// The keys of the rows whose condition holds for `stored`.
fn shown(stored: &StoredParams) -> Vec<&'static str> {
    let kind = kind_of(ROUNDED_BOX);
    let schema = kind.schema();
    kind.groups
        .iter()
        .flat_map(|group| group.params.iter())
        .filter(|param| {
            param
                .shown_when
                .is_none_or(|condition| condition.holds(&schema, stored))
        })
        .map(|param| param.key)
        .collect()
}

// ---------- criterion 21: the rows each Top shape shows ----------

#[test]
fn criterion_21_curved_hides_the_flat_rows_and_keeps_their_values() {
    let flat = shown(&StoredParams::new());
    let curved_rows = shown(&curved(&[]));
    let hidden = [
        "lid",
        "lid_seat",
        "ring_thickness",
        "lid_thickness",
        "wall_slits",
        "base_cover",
        "cover_thickness",
        "base_inset",
    ];
    for key in hidden {
        assert!(flat.contains(&key), "{key} is shown in Flat");
        assert!(!curved_rows.contains(&key), "{key} is hidden in Curved");
    }
    for key in ["end_plate_thickness", "front_thickness"] {
        assert!(!flat.contains(&key), "{key} is hidden in Flat");
        assert!(curved_rows.contains(&key), "{key} is shown in Curved");
    }
    for key in [
        "inner_width",
        "inner_depth",
        "inner_height",
        "corner_radius",
        "top_shape",
        "thickness",
        "kerf",
        "slit_pattern",
        "slit_width",
        "slit_spacing",
        "plate_thickness",
        "finger_hollow",
        "hollow_width",
        "hollow_depth",
        "gap",
    ] {
        assert!(curved_rows.contains(&key), "{key} is shown in Curved");
        assert!(flat.contains(&key), "{key} is shown in Flat");
    }
    // The hollow's size follows its switch in Curved too.
    let off = shown(&curved(&[("finger_hollow", ParamValue::Switch(false))]));
    assert!(off.contains(&"finger_hollow"));
    assert!(!off.contains(&"hollow_width") && !off.contains(&"hollow_depth"));
    // Flat keeps today's conditions: no lid, no ring row and no hollow.
    let no_lid = shown(&params(&[("lid", ParamValue::Switch(false))]));
    assert!(!no_lid.contains(&"ring_thickness") && !no_lid.contains(&"finger_hollow"));
}

#[test]
fn the_path_box_rows_and_conditions_do_not_change() {
    fn mentions_top_shape(condition: ShownWhen) -> bool {
        match condition {
            ShownWhen::Is { key, .. }
            | ShownWhen::On { key }
            | ShownWhen::Off { key }
            | ShownWhen::Full { key }
            | ShownWhen::Positive { key } => key == "top_shape",
            ShownWhen::AllOn { .. } => false,
            ShownWhen::All { .. } | ShownWhen::Any { .. } => true,
        }
    }
    let path_box = kind_of(PATH_BOX);
    for param in path_box.groups.iter().flat_map(|g| g.params.iter()) {
        assert_ne!(param.key, "top_shape");
        assert!(
            !param.shown_when.is_some_and(mentions_top_shape),
            "{} carries a curved-top condition",
            param.key
        );
    }
    let keys: Vec<&str> = path_box
        .groups
        .iter()
        .flat_map(|g| g.params.iter())
        .map(|p| p.key)
        .collect();
    for key in [
        "lid",
        "lid_seat",
        "base_cover",
        "ring_thickness",
        "finger_hollow",
    ] {
        assert!(keys.contains(&key), "the path box still has {key}");
    }
    assert!(!keys.contains(&"end_plate_thickness"));
}

#[test]
fn an_unknown_top_shape_reads_flat_and_a_slice_1_file_has_none() {
    let flat = generate(&StoredParams::new(), None);
    let unknown = generate(&params(&[("top_shape", choice("domed"))]), None);
    assert_eq!(unknown.parts.len(), flat.parts.len());
    assert_eq!(unknown.params["top_shape"], choice("flat").to_stored());
    assert_eq!(flat.params["end_plate_thickness"], mm(0.0).to_stored());
}

// ---------- measuring helpers ----------

const PI: f64 = std::f64::consts::PI;

/// The defaults of the curved box (spec `0147`): 120 x 80 x 50 inside, radius 15, every plate 3 mm.
const W: f64 = 120.0;
const D: f64 = 80.0;
const H: f64 = 50.0;
const R: f64 = 15.0;
const T: f64 = 3.0;

fn points(path: &ClosedPath) -> Vec<(f64, f64)> {
    let mut points = vec![(path.start.x, path.start.y)];
    points.extend(path.segments.iter().map(|s| (s.to.x, s.to.y)));
    points
}

fn bounds(path: &ClosedPath) -> (f64, f64, f64, f64) {
    points(path)
        .into_iter()
        .fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |b, p| {
            (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1))
        })
}

/// Whether `(x, y)` lies in the material of a compound part (even-odd over all its paths).
fn in_material(part: &Part, x: f64, y: f64) -> bool {
    let mut inside = false;
    for path in &part.paths {
        let pts = points(path);
        let n = pts.len();
        for i in 0..n {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            if (a.1 > y) != (b.1 > y) && x < (b.0 - a.0) * (y - a.1) / (b.1 - a.1) + a.0 {
                inside = !inside;
            }
        }
    }
    inside
}

fn strip(g: &Generated) -> &Part {
    &g.parts[0]
}

/// The strip's length on the neutral line by an independent sum: the front run, the arc, the back
/// run (spec criterion 17).
fn strip_length(d: f64, h: f64, r: f64, t: f64, front: f64, base: f64) -> f64 {
    (d - r + front) + PI / 2.0 * (r + t / 2.0) + (h - r + base)
}

fn length_of(g: &Generated, key: &str) -> f64 {
    g.params[key]["mm"].as_f64().unwrap()
}

fn clamped(g: &Generated, key: &str) -> Option<Bound> {
    g.clamps.iter().find(|c| c.key == key).map(|c| c.bound)
}

// ---------- criterion 16: the parts and their numbers ----------

#[test]
fn criterion_16_the_five_parts_in_numbering_order_with_their_badges() {
    let g = generate(&curved(&[]), None);
    assert_eq!(g.parts.len(), 5, "strip, two end plates, front, base");
    assert!(g.clamps.is_empty());
    let numbers: Vec<u32> = g
        .marks
        .on_parts
        .iter()
        .filter_map(|m| match m {
            Mark::Piece { number, .. } => Some(*number),
            _ => None,
        })
        .collect();
    assert_eq!(numbers, [1, 2, 3, 4, 5]);
    let bends = g
        .marks
        .on_footprint
        .iter()
        .filter(|m| matches!(m, Mark::Bend { tight: false, .. }))
        .count();
    assert_eq!(bends, 2, "a bend band on each end plate's arc");
    // The two end plates are one outline.
    assert_eq!(
        g.parts[1].paths[0].segments.len(),
        g.parts[2].paths[0].segments.len()
    );
    let (b1, b2) = (bounds(&g.parts[1].paths[0]), bounds(&g.parts[2].paths[0]));
    assert!(
        (b1.2 - b1.0 - (b2.2 - b2.0)).abs() < 1e-9 && (b1.3 - b1.1 - (b2.3 - b2.1)).abs() < 1e-9
    );
}

#[test]
fn the_part_sizes_follow_the_outer_measures_of_the_adr() {
    // W = w + 2 t_e, D = d + t_f + t_w, H = h + t_b: the base is W by D, the front W by h + t_b,
    // an end plate d + t_f by h + t_b with its tabs t_w beyond.
    let g = generate(
        &curved(&[
            ("end_plate_thickness", mm(4.0)),
            ("front_thickness", mm(5.0)),
            ("plate_thickness", mm(2.0)),
        ]),
        None,
    );
    let size = |i: usize| {
        let b = bounds(&g.parts[i].paths[0]);
        (b.2 - b.0, b.3 - b.1)
    };
    assert!((size(0).1 - (W + 8.0)).abs() < 1e-9, "the strip is W wide");
    let (front, base) = (size(3), size(4));
    assert!((front.0 - (W + 8.0)).abs() < 1e-9 && (front.1 - (H + 2.0)).abs() < 1e-9);
    assert!((base.0 - (W + 8.0)).abs() < 1e-9 && (base.1 - (D + 5.0 + T)).abs() < 1e-9);
    let plate = size(1);
    assert!(
        (plate.0 - (D + 5.0 + T)).abs() < 1e-9,
        "the profile plus the tabs: {plate:?}"
    );
    assert!((plate.1 - (H + 2.0)).abs() < 1e-9);
}

// ---------- criterion 17: the strip's length ----------

#[test]
fn criterion_17_the_strip_is_as_long_as_the_profile_on_the_neutral_line() {
    for r in [15.0, 8.0, 20.0, 30.0] {
        let g = generate(&curved(&[("corner_radius", mm(r))]), None);
        let b = bounds(&strip(&g).paths[0]);
        let want = strip_length(D, H, r, T, T, T);
        assert!(
            (b.2 - b.0 - want).abs() < 0.1,
            "r {r}: {} against {want}",
            b.2 - b.0
        );
    }
    // Every part's own thickness counts: the front's in the run over the top, the base's below.
    let g = generate(
        &curved(&[("front_thickness", mm(5.0)), ("plate_thickness", mm(2.0))]),
        None,
    );
    let b = bounds(&strip(&g).paths[0]);
    assert!((b.2 - b.0 - strip_length(D, H, R, T, 5.0, 2.0)).abs() < 0.1);
}

// ---------- criterion 19: the slits of the arc ----------

/// The strip's arc on its neutral line: from the end of the lid run, a quarter circle long.
fn arc_range(r: f64) -> (f64, f64) {
    let from = D - r + T;
    (from, from + PI / 2.0 * (r + T / 2.0))
}

#[test]
fn criterion_19_only_the_arc_is_slit_and_no_line_across_it_stays_uncut() {
    for pattern in ["straight", "offset", "wave"] {
        for r in [15.0, 8.0] {
            let g = generate(
                &curved(&[("slit_pattern", choice(pattern)), ("corner_radius", mm(r))]),
                None,
            );
            let part = strip(&g);
            let outline = bounds(&part.paths[0]);
            let (from, to) = arc_range(r);
            let back = to + (H - r) + T;
            // No vertex between the free edge and the arc: the lid run is solid.
            for (x, y) in points(&part.paths[0]) {
                let u = x - outline.0;
                assert!(
                    u < 1e-6 || u > from - 1e-6,
                    "{pattern} r {r}: a vertex at {u} on the lid run"
                );
                // The back run holds nothing but the tabs' edge notches and the finger end.
                if u > to + 1e-6 && u < back - T - 1e-6 {
                    let v = y - outline.1;
                    let edge = (W + 2.0 * T) - v;
                    assert!(
                        v < T + 1e-6 || edge < T + 1e-6,
                        "{pattern} r {r}: a vertex at ({u}, {v}) on the back run"
                    );
                }
            }
            // Every line across the arc, along the strip, meets a slot.
            let height = outline.3 - outline.1;
            let mut y = 0.1;
            while y < height - 0.05 {
                let mut cut = false;
                let mut u = from + 0.2;
                while u < to - 0.2 {
                    cut |= !in_material(part, outline.0 + u, outline.1 + y);
                    u += 0.05;
                }
                assert!(
                    cut,
                    "{pattern} r {r}: the line at height {y} runs uncut across the arc"
                );
                y += 0.25;
            }
        }
    }
}

#[test]
fn criterion_19_the_arc_keeps_two_columns_and_the_default_has_eight() {
    let g = generate(&curved(&[]), None);
    let outline = bounds(&strip(&g).paths[0]);
    let (from, to) = arc_range(R);
    // Each Straight column is one edge notch: count the notch floors across the arc.
    let mut columns = 0;
    let mut u = from;
    let mut inside = false;
    while u < to {
        let cut = !in_material(strip(&g), outline.0 + u, outline.1 + 1.0)
            || !in_material(strip(&g), outline.0 + u, outline.3 - 1.0);
        if cut && !inside {
            columns += 1;
        }
        inside = cut;
        u += 0.02;
    }
    assert_eq!(
        columns, 8,
        "0.22 times the radius spaces eight columns over the arc"
    );
    // The smallest radius still holds two columns.
    let smallest = generate(&curved(&[("corner_radius", mm(6.0))]), None);
    assert!(smallest.clamps.is_empty());
    assert!(strip(&smallest).paths[0].segments.len() > 20);
}

#[test]
fn criterion_19_a_typed_spacing_the_arc_cannot_hold_twice_is_clamped_down() {
    let g = generate(&curved(&[("slit_spacing", mm(20.0))]), None);
    let want = PI / 8.0 * (2.0 * R + T);
    assert!((length_of(&g, "slit_spacing") - want).abs() < 1e-9);
    assert_eq!(clamped(&g, "slit_spacing"), Some(Bound::Dependent));
    assert_eq!(length_of(&g, "corner_radius"), R, "the radius stays");
}

// ---------- criterion 18: the joints ----------

/// The segments of a path as `(from, to)`.
fn segments_of(path: &ClosedPath) -> Vec<((f64, f64), (f64, f64))> {
    let pts = points(path);
    let n = pts.len();
    (0..n).map(|i| (pts[i], pts[(i + 1) % n])).collect()
}

#[test]
fn criterion_18_the_tabs_of_an_end_plate_meet_the_strips_edge_notches() {
    let h = 150.0;
    let g = generate(
        &curved(&[("inner_height", mm(h)), ("end_plate_thickness", mm(4.0))]),
        None,
    );
    assert!(g.clamps.is_empty(), "{:?}", g.clamps);
    let plate = &g.parts[1].paths[0];
    let pb = bounds(plate);
    // Tabs: vertical segments on the outer line, 2 t_e long, t_w beyond the plate's back edge.
    let mut tabs: Vec<f64> = segments_of(plate)
        .into_iter()
        .filter(|((x0, y0), (x1, y1))| {
            (x0 - pb.2).abs() < 1e-6
                && (x1 - pb.2).abs() < 1e-6
                && ((y1 - y0).abs() - 8.0).abs() < 1e-6
        })
        .map(|((_, y0), (_, y1))| f64::midpoint(y0, y1) - pb.1)
        .collect();
    tabs.sort_by(f64::total_cmp);
    assert!(
        tabs.len() >= 2,
        "{tabs:?}: a tall back holds more than one tab"
    );
    let back = (D - R + T) + PI / 2.0 * (R + T / 2.0);
    let strip_part = &g.parts[0].paths[0];
    let sb = bounds(strip_part);
    // The strip's edge notches: horizontal floors one t_e in from either side edge.
    let mut slots: Vec<f64> = segments_of(strip_part)
        .into_iter()
        .filter(|((x0, y0), (x1, y1))| {
            (y0 - sb.1 - 4.0).abs() < 1e-6
                && (y1 - sb.1 - 4.0).abs() < 1e-6
                && ((x1 - x0).abs() - 8.0).abs() < 1e-6
        })
        .map(|((x0, _), (x1, _))| f64::midpoint(x0, x1) - sb.0)
        .collect();
    slots.sort_by(f64::total_cmp);
    assert_eq!(slots.len(), tabs.len());
    for (tab, slot) in tabs.iter().zip(&slots) {
        // Distance from the plate's top edge against the neutral length from the arc's end.
        assert!(
            (back + (tab - R) - slot).abs() < 1e-6,
            "tab {tab} against slot {slot}"
        );
        assert!(
            tab - 4.0 >= R + 2.0 * T - 1e-6,
            "never closer to the arc than 2 t_w"
        );
        assert!(tab + 4.0 <= h - 2.0 * T + 1e-6, "2 t_w clear of the base");
    }
}

#[test]
fn criterion_18_the_base_and_the_front_hold_fingers_on_straights_only() {
    let g = generate(&curved(&[]), None);
    let (front, base) = (&g.parts[3].paths[0], &g.parts[4].paths[0]);
    // 21 pieces across the width (6 mm fingers on 120 mm): the base's back edge has 10 fingers.
    let fingers = |path: &ClosedPath, y: f64| {
        segments_of(path)
            .into_iter()
            .filter(|((_, y0), (_, y1))| (y0 - y).abs() < 1e-6 && (y1 - y).abs() < 1e-6)
            .count()
    };
    let bb = bounds(base);
    assert_eq!(
        fingers(base, bb.3),
        11,
        "the base keeps the even pieces of its back edge"
    );
    let fb = bounds(front);
    assert_eq!(
        fingers(front, fb.3),
        10,
        "the front keeps the odd pieces of its bottom edge"
    );
}

// ---------- criterion 20: clamps ----------

#[test]
fn criterion_20_the_corner_radius_stays_between_two_wall_thicknesses_and_the_room_it_needs() {
    // At the defaults 6 to 32 mm: depth minus 2 t_w is 74, height minus 2 t_e minus 4 t_w is 32.
    let high = generate(&curved(&[("corner_radius", mm(40.0))]), None);
    assert_eq!(length_of(&high, "corner_radius"), 32.0);
    assert_eq!(clamped(&high, "corner_radius"), Some(Bound::Dependent));
    let low = generate(&curved(&[("corner_radius", mm(2.0))]), None);
    assert_eq!(length_of(&low, "corner_radius"), 6.0);
    assert_eq!(
        clamped(&low, "corner_radius"),
        Some(Bound::DependentMinimum)
    );
    let short = generate(
        &curved(&[("inner_depth", mm(30.0)), ("corner_radius", mm(30.0))]),
        None,
    );
    assert_eq!(
        length_of(&short, "corner_radius"),
        24.0,
        "the lid keeps a flat run over the front"
    );
    let limit = high
        .limits
        .iter()
        .find(|(key, _)| *key == "corner_radius")
        .map(|(_, v)| v.clone());
    assert_eq!(limit, Some(mm(32.0)));
    assert_eq!(
        high.states,
        [("corner_radius", "max")],
        "at the bound it reads max"
    );
    assert!(generate(&curved(&[]), None).states.is_empty());
}

#[test]
fn criterion_20_a_low_box_is_raised_so_the_radius_range_is_never_empty() {
    let g = generate(&curved(&[("inner_height", mm(12.0))]), None);
    assert_eq!(length_of(&g, "inner_height"), 24.0, "2 t_e + 6 t_w");
    assert_eq!(clamped(&g, "inner_height"), Some(Bound::DependentMinimum));
    assert_eq!(
        length_of(&g, "corner_radius"),
        6.0,
        "the range is the single value 6 mm"
    );
}

#[test]
fn criterion_20_switching_top_shape_keeps_the_values_and_reclamps_the_shared_ones() {
    let flat = params(&[("corner_radius", mm(40.0))]);
    assert_eq!(length_of(&generate(&flat, None), "corner_radius"), 40.0);
    let to_curved = {
        let mut p = flat.clone();
        p.insert("top_shape".into(), choice("curved").to_stored());
        generate(&p, None)
    };
    assert_eq!(length_of(&to_curved, "corner_radius"), 32.0);
    // Back to Flat with what the box wrote: the radius stays at 32.
    let mut back = to_curved.params.clone();
    back.insert("top_shape".into(), choice("flat").to_stored());
    let flat_again = generate(&back, None);
    assert_eq!(length_of(&flat_again, "corner_radius"), 32.0);
    assert_eq!(flat_again.parts.len(), 5, "strip, cover, lid, base, ring");
}

#[test]
fn criterion_21_hidden_rows_keep_their_stored_values_and_are_never_clamped() {
    let hidden = [
        ("base_inset", mm(1.0)),
        ("ring_thickness", mm(40.0)),
        ("cover_thickness", mm(9.0)),
        ("lid_thickness", mm(7.0)),
        ("lid", ParamValue::Switch(false)),
        ("lid_seat", ParamValue::Switch(false)),
        ("base_cover", ParamValue::Switch(false)),
        ("wall_slits", choice("all")),
    ];
    let g = generate(&curved(&hidden), None);
    for (key, value) in &hidden {
        assert_eq!(
            g.params[*key],
            value.to_stored(),
            "{key} is saved back as it was"
        );
        assert!(
            clamped(&g, key).is_none(),
            "{key} is never clamped in Curved"
        );
    }
    assert_eq!(g.parts.len(), 5, "no cover, ring or loose lid in this mode");
}

#[test]
fn criterion_21_the_new_thicknesses_follow_the_wall_until_typed() {
    let g = generate(&curved(&[("thickness", mm(4.0))]), None);
    for key in ["end_plate_thickness", "front_thickness", "plate_thickness"] {
        assert!(
            g.automatic.iter().any(|(k, v)| *k == key && *v == mm(4.0)),
            "{key} follows Wall thickness"
        );
        assert_eq!(
            g.params[key],
            mm(0.0).to_stored(),
            "nothing automatic is written"
        );
    }
    let typed = generate(&curved(&[("front_thickness", mm(5.0))]), None);
    assert!(typed.automatic.iter().all(|(k, _)| *k != "front_thickness"));
}

#[test]
fn criterion_21_the_finger_hollow_is_a_u_notch_in_the_fronts_top_edge() {
    let g = generate(
        &curved(&[("hollow_width", mm(20.0)), ("hollow_depth", mm(12.0))]),
        None,
    );
    let front = &g.parts[3];
    assert_eq!(front.paths.len(), 1, "the front stays in one piece");
    let b = bounds(&front.paths[0]);
    // Centred across the width, 12 mm deep: material below the notch, none inside it.
    let centre = f64::midpoint(b.0, b.2);
    assert!(!in_material(front, centre, b.1 + 6.0), "inside the notch");
    assert!(in_material(front, centre, b.1 + 13.0), "below the notch");
    assert!(
        in_material(front, centre - 11.0, b.1 + 1.0),
        "beside the notch"
    );
    assert!(
        !in_material(front, centre + 9.0, b.1 + 1.0),
        "within the half width"
    );
    // The widest notch keeps 2 t_e to the side edge, where the finger joint is.
    let widest = generate(
        &curved(&[("hollow_width", mm(500.0)), ("inner_height", mm(200.0))]),
        None,
    );
    assert_eq!(length_of(&widest, "hollow_width"), W - 2.0 * T);
    assert_eq!(clamped(&widest, "hollow_width"), Some(Bound::Dependent));
    let fb = bounds(&widest.parts[3].paths[0]);
    let edge = (fb.2 - fb.0 - length_of(&widest, "hollow_width")) / 2.0;
    assert!((edge - 2.0 * T).abs() < 1e-9, "2 t_e from the edge: {edge}");
    let deep = generate(&curved(&[("hollow_depth", mm(90.0))]), None);
    assert_eq!(length_of(&deep, "hollow_depth"), 25.0, "half the height");
    let narrow = generate(
        &curved(&[("hollow_depth", mm(2.0)), ("hollow_width", mm(16.0))]),
        None,
    );
    assert_eq!(length_of(&narrow, "hollow_depth"), 8.0, "half the width");
    let off = generate(
        &curved(&[("finger_hollow", ParamValue::Switch(false))]),
        None,
    );
    assert!(off.parts[3].paths[0].segments.len() < g.parts[3].paths[0].segments.len());
}

#[test]
fn criterion_21_kerf_and_thickness_clamps_use_the_new_plates() {
    let g = generate(
        &curved(&[("kerf", mm(1.4)), ("front_thickness", mm(1.0))]),
        None,
    );
    assert_eq!(length_of(&g, "kerf"), 0.5, "half the thinnest part");
    let thick = generate(&curved(&[("thickness", mm(30.0))]), None);
    assert_eq!(
        length_of(&thick, "thickness"),
        80.0 / 6.0,
        "a sixth of the smaller inside side"
    );
}

// ---------- criterion 22: the lid rests ----------

#[test]
fn criterion_22_the_strip_the_end_plates_and_the_front_share_the_seat_plane() {
    // The strip's inner face starts at the front's outer face and reaches the base's underside;
    // the plates stand from the base's underside to the seat: all by the same sums (kerf 0).
    let g = generate(
        &curved(&[("front_thickness", mm(5.0)), ("plate_thickness", mm(2.0))]),
        None,
    );
    let height = H + 2.0;
    for i in [1, 2, 3] {
        let b = bounds(&g.parts[i].paths[0]);
        assert!(
            (b.3 - b.1 - height).abs() < 1e-9,
            "part {i} stands {height} high"
        );
    }
    let outline = bounds(strip(&g).paths.first().unwrap());
    let run = strip_length(D, H, R, T, 5.0, 2.0);
    assert!((outline.2 - outline.0 - run).abs() < 0.1);
    // The end plate's depth is the strip's lid run plus the arc's two ends over the plate:
    // d + t_f, so the strip's free edge and the plate's front edge are one plane.
    let plate = bounds(&g.parts[1].paths[0]);
    assert!((plate.2 - plate.0 - (D + 5.0 + T)).abs() < 1e-9);
}

// ---------- criterion 23: layout ----------

#[test]
fn criterion_23_the_strip_is_the_first_row_and_the_plates_follow_in_number_order() {
    let g = generate(&curved(&[("gap", mm(5.0))]), None);
    let bs: Vec<_> = g.parts.iter().map(|p| bounds(&p.paths[0])).collect();
    // First row: the strip alone; second row: left to right, 2, 3, 4, 5.
    assert!(bs[0].3 < bs[1].1 && bs[0].3 < bs[4].1);
    for i in 1..4 {
        assert!(
            bs[i].2 < bs[i + 1].0,
            "part {} lies left of part {}",
            i + 1,
            i + 2
        );
        assert!(((bs[i + 1].0 - bs[i].2) - 5.0).abs() < 1e-6, "gap {i}");
    }
    assert!(
        (bs[1].1 - bs[0].3 - 5.0).abs() < 1e-6,
        "the rows are one gap apart"
    );
    // A thicker front goes after the plates of equal thickness (the runs of section 8).
    let thick = generate(&curved(&[("front_thickness", mm(4.0))]), None);
    let widths: Vec<f64> = thick.parts[1..]
        .iter()
        .map(|p| {
            let b = bounds(&p.paths[0]);
            b.2 - b.0
        })
        .collect();
    assert!(
        (widths[0] - widths[1]).abs() < 1e-9,
        "the end plates come first"
    );
    assert!(
        (widths[2] - (W + 6.0)).abs() < 1e-9,
        "then the base, W wide: {widths:?}"
    );
}

#[test]
fn criterion_23_the_handles_sit_on_the_base_the_front_and_the_left_end_plate() {
    let g = generate(&curved(&[]), None);
    let bs: Vec<_> = g.parts.iter().map(|p| bounds(&p.paths[0])).collect();
    let handle = |key: &str| g.handles.iter().find(|h| h.key == key).copied().unwrap();
    let width = handle("inner_width");
    assert!(
        (width.anchor.x - (bs[4].2 - T)).abs() < 1e-6,
        "base right edge, on the finger root"
    );
    assert!((width.anchor.y - f64::midpoint(bs[4].1, bs[4].3)).abs() < 1e-6);
    let depth = handle("inner_depth");
    assert!(
        (depth.anchor.y - (bs[4].3 - T)).abs() < 1e-6,
        "base back edge"
    );
    let height = handle("inner_height");
    assert!((height.anchor.y - bs[3].1).abs() < 1e-6, "front top edge");
    assert!(
        height.anchor.x > f64::midpoint(bs[3].0, bs[3].2),
        "beside the hollow"
    );
    let radius = handle("corner_radius");
    let want_x = bs[1].0 + (D + T) - R * (1.0 - std::f64::consts::FRAC_1_SQRT_2);
    assert!(
        (radius.anchor.x - want_x).abs() < 1e-6,
        "the arc's middle point"
    );
    assert!(
        (radius.anchor.y - (bs[1].1 + R * (1.0 - std::f64::consts::FRAC_1_SQRT_2))).abs() < 1e-6
    );
    // The knob stays on the arc: at r + 1 the anchor moves along the inward diagonal.
    let bigger = generate(&curved(&[("corner_radius", mm(R + 1.0))]), None);
    let moved = bigger
        .handles
        .iter()
        .find(|h| h.key == "corner_radius")
        .unwrap();
    let along = (moved.anchor.x - radius.anchor.x) * radius.direction.x
        + (moved.anchor.y - radius.anchor.y) * radius.direction.y;
    assert!(
        (along * radius.rate - 1.0).abs() < 1e-6,
        "{along} mm of drag per mm of radius"
    );
}

// ---------- criterion 24: edits, files, time ----------

#[test]
fn criterion_24_a_resize_changes_the_three_inner_sizes_only() {
    let g = generate(
        &curved(&[("corner_radius", mm(20.0))]),
        Some(Edit::Scale(ScaleFactor(0.5))),
    );
    assert_eq!(length_of(&g, "inner_width"), 60.0);
    assert_eq!(length_of(&g, "inner_depth"), 40.0);
    assert_eq!(length_of(&g, "inner_height"), 25.0);
    assert_eq!(length_of(&g, "thickness"), 3.0);
    // Clamped to the room the halved height leaves: 25 - 2 t_e - 4 t_w = 7.
    assert_eq!(length_of(&g, "corner_radius"), 7.0);
    assert_eq!(clamped(&g, "corner_radius"), Some(Bound::Dependent));
    assert_eq!(g.params["top_shape"], choice("curved").to_stored());
}

#[test]
fn criterion_24_the_stored_values_survive_and_a_slice_1_file_opens_flat() {
    let g = generate(&curved(&[("end_plate_thickness", mm(4.0))]), None);
    assert_eq!(g.params["top_shape"], choice("curved").to_stored());
    assert_eq!(g.params["end_plate_thickness"], mm(4.0).to_stored());
    // A regenerated record reads back to the same outlines.
    let again = generate(&g.params, None);
    assert_eq!(again.parts, g.parts);
    // A stored record of slice 1 (none of the three new keys) is Flat.
    let mut old = generate(&StoredParams::new(), None).params;
    for key in ["top_shape", "end_plate_thickness", "front_thickness"] {
        old.remove(key);
    }
    let reopened = generate(&old, None);
    assert_eq!(reopened.params["top_shape"], choice("flat").to_stored());
    assert_eq!(reopened.parts.len(), 5);
}

#[test]
fn criterion_24_the_draft_is_the_plain_strip_and_the_plates_as_built() {
    let full = generate(&curved(&[]), None);
    let draft = generate(&curved(&[]), Some(Edit::Draft));
    assert_eq!(draft.parts.len(), 5);
    assert_eq!(
        draft.parts[0].paths[0].segments.len(),
        4,
        "a plain rectangle"
    );
    assert_eq!(draft.parts[1], full.parts[1], "the plates as they are");
    assert_eq!(draft.parts[4], full.parts[4]);
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
fn criterion_24_regenerating_a_curved_box_of_about_400_slots_costs_under_100_ms_of_thread_cpu() {
    // A wide, deep box with the least spacing: the arc alone carries hundreds of slots.
    let stored = curved(&[
        ("inner_width", mm(1000.0)),
        ("inner_depth", mm(400.0)),
        ("inner_height", mm(300.0)),
        ("corner_radius", mm(200.0)),
        ("slit_spacing", mm(2.5)),
        ("slit_pattern", choice("offset")),
        ("slit_rows", ParamValue::Count(2)),
    ]);
    let _warm = generate(&stored, None);
    let mut best = u64::MAX;
    let mut segments = 0;
    for _ in 0..3 {
        let before = thread_ticks().expect("the thread CPU clock is readable");
        let g = generate(&stored, None);
        let after = thread_ticks().expect("the thread CPU clock is readable");
        segments = strip(&g)
            .paths
            .iter()
            .map(|p| p.segments.len())
            .sum::<usize>();
        best = best.min(after - before);
    }
    eprintln!("curved 400 slots: {segments} strip segments, best run {best} ticks (10 ms each)");
    assert!(segments >= 1000, "about 400 slots: {segments} segments");
    let took = budget_factor::scaled(std::time::Duration::from_millis(best * 10));
    assert!(
        took <= std::time::Duration::from_millis(100),
        "took {} ms",
        best * 10
    );
}

// ---------- the golden ----------

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
fn the_default_curved_box_outline_is_frozen() {
    golden(
        "rounded_box_curved.txt",
        &dump(&generate(&curved(&[]), None)),
    );
}

#[test]
fn criterion_21_the_widest_and_deepest_hollow_leaves_the_front_in_one_piece() {
    for height in [50.0, 30.0, 24.0, 200.0] {
        let g = generate(
            &curved(&[
                ("inner_height", mm(height)),
                ("hollow_width", mm(500.0)),
                ("hollow_depth", mm(500.0)),
            ]),
            None,
        );
        let (hw, hd) = (length_of(&g, "hollow_width"), length_of(&g, "hollow_depth"));
        assert!(
            hw <= length_of(&g, "inner_height") + 1e-9,
            "no wider than the height: {hw}"
        );
        assert!(hd >= hw / 2.0 - 1e-9 && hd <= length_of(&g, "inner_height") / 2.0 + 1e-9);
        let front = &g.parts[3];
        assert_eq!(front.paths.len(), 1);
        let b = bounds(&front.paths[0]);
        let centre = f64::midpoint(b.0, b.2);
        assert!(
            in_material(front, centre, b.1 + hd + 0.5),
            "material under the notch"
        );
        // Above the bottom joint's fingers the front is solid under the notch.
        assert!(in_material(front, centre, b.3 - T - 0.5));
        assert!(b.1 + hd < b.3 - T, "the notch ends above the bottom joint");
    }
}
