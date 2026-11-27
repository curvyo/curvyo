//! Acceptance of the rounded box (`specs/0147-rounded-box` criteria 2 to 10, 12, 13; ADR 0082 §9):
//! the strip's length, the slots, the mating of tabs and slots, the layout and the goldens.

#![allow(
    clippy::unwrap_used,
    clippy::many_single_char_names,
    clippy::float_cmp,
    clippy::format_push_string
)]

use curvyo_document_core::Length;
use curvyo_extension_core::{
    ClosedPath, Edit, Extension, GenerateRequest, Generated, ParamValue, ScaleFactor, Seed,
    StoredParams,
};
use curvyo_generators_core::{GENERATORS, ROUNDED_BOX};

fn mm(value: f64) -> ParamValue {
    ParamValue::Length(Length::from_mm(value))
}

fn params(entries: &[(&str, ParamValue)]) -> StoredParams {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.to_stored()))
        .collect()
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

/// The points a path passes (line segments only: end points).
fn points(path: &ClosedPath) -> Vec<(f64, f64)> {
    let mut points = vec![(path.start.x, path.start.y)];
    points.extend(path.segments.iter().map(|s| (s.to.x, s.to.y)));
    points
}

fn bounds(path: &ClosedPath) -> (f64, f64, f64, f64) {
    let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for (x, y) in points(path) {
        b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
    }
    b
}

const PI: f64 = std::f64::consts::PI;

/// The inner box of the defaults: 120 x 80 x 50, radius 15, wall 3.
const W: f64 = 120.0;
const D: f64 = 80.0;
const H: f64 = 50.0;
const R: f64 = 15.0;
const T: f64 = 3.0;

fn strip(g: &Generated) -> &curvyo_extension_core::Part {
    &g.parts[0]
}

fn height_of_strip(inset: f64, base: f64, ring: f64) -> f64 {
    inset + base + H + ring
}

/// Whether `(x, y)` lies in the material of a compound part (even-odd over all its paths).
fn in_material(part: &curvyo_extension_core::Part, x: f64, y: f64) -> bool {
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

/// The bent spans on the strip's neutral line, by an independent formula: the seam half of the
/// back straight, then arc, right straight, arc, front straight, arc, left straight, arc.
fn bend_ranges() -> Vec<(f64, f64)> {
    bend_ranges_of(W, D, R, T)
}

fn bend_ranges_of(w: f64, d: f64, r: f64, t: f64) -> Vec<(f64, f64)> {
    let arc = PI / 2.0 * (r + t / 2.0);
    let (back, side) = (w - 2.0 * r, d - 2.0 * r);
    let mut s = back / 2.0;
    let mut ranges = Vec::new();
    for straight in [side, back, side] {
        ranges.push((s, s + arc));
        s += arc + straight;
    }
    ranges.push((s, s + arc));
    ranges
}

#[test]
fn the_default_box_has_the_five_parts_in_the_order_of_the_layout() {
    let g = generate(&StoredParams::new(), None);
    assert_eq!(g.parts.len(), 5, "strip, cover, lid, inner base, ring");
    assert_eq!(g.parts[4].paths.len(), 2, "the ring has its opening");
    assert_eq!(g.clamps.len(), 0);
    let b = bounds(&strip(&g).paths[0]);
    assert!(
        (b.3 - b.1 - height_of_strip(0.0, T, T)).abs() < 1e-9,
        "strip height"
    );
}

#[test]
fn the_strip_is_as_long_as_the_perimeter_on_the_neutral_line() {
    // Independent of the footprint's own spans (ADR 0082 §9).
    let expected = 2.0 * (W - 2.0 * R) + 2.0 * (D - 2.0 * R) + PI * (2.0 * R + T);
    let g = generate(&StoredParams::new(), None);
    let b = bounds(&strip(&g).paths[0]);
    assert!(
        (b.2 - b.0 - expected).abs() < 0.1,
        "{} vs {expected}",
        b.2 - b.0
    );
}

#[test]
fn the_plates_lie_on_the_footprint_outline() {
    let g = generate(&StoredParams::new(), None);
    // The cover is the outer outline, the lid too; the inner base and the ring reach the outer
    // face with their tabs.
    for part in &g.parts[1..] {
        let b = bounds(&part.paths[0]);
        assert!((b.2 - b.0 - (W + 2.0 * T)).abs() < 1e-6, "{b:?}");
        assert!((b.3 - b.1 - (D + 2.0 * T)).abs() < 1e-6, "{b:?}");
    }
}

#[test]
fn no_band_of_material_runs_uncut_across_a_bend() {
    let g = generate(&StoredParams::new(), None);
    let h = height_of_strip(0.0, T, T);
    let top = bounds(&strip(&g).paths[0]);
    for (from, to) in bend_ranges() {
        let mut y = 0.2;
        while y < h - 0.1 {
            let mut met = false;
            let mut x = from + 0.05;
            while x < to - 0.05 {
                if !in_material(strip(&g), top.0 + x, top.1 + y) {
                    met = true;
                    break;
                }
                x += 0.02;
            }
            assert!(met, "an uncut band at height {y} on the bend {from}..{to}");
            y += 0.25;
        }
    }
}

#[test]
fn straight_walls_stay_solid_with_the_default_corners_only_slits() {
    let g = generate(&StoredParams::new(), None);
    let top = bounds(&strip(&g).paths[0]);
    // The middle of the right straight at mid height (no tab slot there: two tabs at 1/4, 3/4).
    let arc = PI / 2.0 * (R + T / 2.0);
    let s = (W - 2.0 * R) / 2.0 + arc + (D - 2.0 * R) / 2.0;
    assert!(in_material(strip(&g), top.0 + s, top.1 + 25.0));
}

#[test]
fn slit_values_are_automatic_by_default_and_reported() {
    let g = generate(&StoredParams::new(), None);
    let used = |key: &str| {
        g.automatic
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.clone())
    };
    assert_eq!(used("slit_width"), Some(mm(0.3)));
    assert_eq!(used("slit_spacing"), Some(mm(3.3)));
    assert_eq!(used("lid_thickness"), Some(mm(3.0)));
    // Nothing automatic is written into the record.
    assert_eq!(g.params["slit_width"], mm(0.0).to_stored());
    let typed = generate(&params(&[("slit_width", mm(0.5))]), None);
    assert!(typed.automatic.iter().all(|(k, _)| *k != "slit_width"));
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
fn the_default_rounded_box_outline_is_frozen() {
    golden(
        "rounded_box_default.txt",
        &dump(&generate(&StoredParams::new(), None)),
    );
}

#[test]
fn a_resize_scales_the_three_inner_sizes_and_keeps_the_radius_and_thicknesses() {
    let scaled = generate(&StoredParams::new(), Some(Edit::Scale(ScaleFactor(0.5))));
    let length = |key: &str| scaled.params[key]["mm"].as_f64().unwrap();
    assert_eq!(length("inner_width"), 60.0);
    assert_eq!(length("inner_depth"), 40.0);
    assert_eq!(length("inner_height"), 25.0);
    assert_eq!(length("corner_radius"), 15.0);
    assert_eq!(length("thickness"), 3.0);
}

/// The centres of the horizontal segments of length `length` at height `y` of a path.
fn horizontal_centres(path: &ClosedPath, y: f64, length: f64) -> Vec<f64> {
    let pts = points(path);
    let n = pts.len();
    let mut centres: Vec<f64> = (0..n)
        .filter_map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            ((a.1 - y).abs() < 1e-6
                && (b.1 - y).abs() < 1e-6
                && ((a.0 - b.0).abs() - length).abs() < 1e-6)
                .then_some(f64::midpoint(a.0, b.0))
        })
        .collect();
    centres.sort_by(f64::total_cmp);
    centres
}

/// The tab centres on the strip from the footprint's straights, by an independent formula.
fn expected_slot_centres() -> Vec<f64> {
    let arc = PI / 2.0 * (R + T / 2.0);
    let (back, side) = (W - 2.0 * R, D - 2.0 * R);
    let mut centres = Vec::new();
    // Back straight, second half: tabs 56.25 and 78.75 of 4 in 90 mm; the seam is at 45.
    centres.extend([56.25 - back / 2.0, 78.75 - back / 2.0]);
    let mut s = back / 2.0 + arc;
    for (straight, tabs) in [
        (side, [12.5, 37.5].to_vec()),
        (back, vec![15.0, 45.0, 75.0]),
        (side, vec![12.5, 37.5]),
    ] {
        centres.extend(tabs.iter().map(|u| s + u));
        s += straight + arc;
    }
    // The first half of the back straight.
    centres.extend([s + 11.25, s + 33.75]);
    centres
}

#[test]
fn the_tabs_of_the_inner_base_meet_the_slots_of_the_strip_within_the_kerf() {
    for kerf in [0.0, 0.2] {
        let g = generate(&params(&[("kerf", mm(kerf))]), None);
        let top = bounds(&strip(&g).paths[0]);
        let h = top.3 - top.1;
        // A slot is drawn smaller by the kerf; its centre stays.
        let slots = horizontal_centres(&strip(&g).paths[0], top.3 - T, 2.0 * T - kerf);
        let ring_slots = horizontal_centres(&strip(&g).paths[0], top.1 + T, 2.0 * T - kerf);
        let expected = expected_slot_centres();
        assert_eq!(
            slots.len(),
            expected.len(),
            "kerf {kerf}: base slots {slots:?}"
        );
        assert_eq!(ring_slots.len(), expected.len(), "kerf {kerf}: ring slots");
        let mut sorted = expected.clone();
        sorted.sort_by(f64::total_cmp);
        for (found, want) in slots.iter().zip(&sorted) {
            assert!(
                (found - top.0 - want).abs() <= kerf + 1e-6,
                "{found} against {want}"
            );
        }
        let _ = h;
        // The base plate: 4 tabs on the back edge, 3 on the front, 2 on each side.
        let base = &g.parts[3].paths[0];
        let b = bounds(base);
        let back = horizontal_centres(base, b.1, 2.0 * T + kerf);
        assert_eq!(back.len(), 4, "kerf {kerf}: back tabs {back:?}");
        let mid = f64::midpoint(b.0, b.2);
        for (found, want) in back.iter().zip([-33.75, -11.25, 11.25, 33.75]) {
            assert!((found - mid - want).abs() <= 1e-6, "{found} against {want}");
        }
    }
}

#[test]
fn a_thinner_inner_base_makes_narrower_tabs_and_slots() {
    let g = generate(&params(&[("plate_thickness", mm(2.0))]), None);
    let top = bounds(&strip(&g).paths[0]);
    let slots = horizontal_centres(&strip(&g).paths[0], top.3 - 2.0, 4.0);
    assert_eq!(slots.len(), 11, "one slot per tab, 2 t wide");
    let h = top.3 - top.1;
    assert!(
        (h - height_of_strip(0.0, 2.0, T)).abs() < 1e-9,
        "strip height follows the thicknesses"
    );
}

fn choice(id: &str) -> ParamValue {
    ParamValue::Choice(id.to_owned())
}

fn on(value: bool) -> ParamValue {
    ParamValue::Switch(value)
}

/// Every height across every bend meets a slot or an edge notch.
fn assert_no_uncut_band(g: &Generated, ranges: &[(f64, f64)], what: &str) {
    let top = bounds(&strip(g).paths[0]);
    let h = top.3 - top.1;
    for &(from, to) in ranges {
        let mut y = 0.2;
        while y < h - 0.1 {
            let mut x = from + 0.05;
            let mut met = false;
            while x < to - 0.05 {
                if !in_material(strip(g), top.0 + x, top.1 + y) {
                    met = true;
                    break;
                }
                x += 0.02;
            }
            assert!(
                met,
                "{what}: an uncut band at height {y} on the bend {from}..{to}"
            );
            y += 0.25;
        }
    }
}

#[test]
fn every_pattern_leaves_no_uncut_band_over_a_range_of_radii_and_thicknesses() {
    for pattern in ["straight", "offset", "wave"] {
        for (r, t) in [
            (6.0, 3.0),
            (10.0, 3.0),
            (25.0, 3.0),
            (40.0, 3.0),
            (15.0, 1.5),
            (20.0, 6.0),
        ] {
            let g = generate(
                &params(&[
                    ("slit_pattern", choice(pattern)),
                    ("corner_radius", mm(r)),
                    ("thickness", mm(t)),
                ]),
                None,
            );
            assert_no_uncut_band(
                &g,
                &bend_ranges_of(W, D, r, t),
                &format!("{pattern} r{r} t{t}"),
            );
        }
    }
}

#[test]
fn all_round_slits_cover_the_straights_but_keep_a_bridge_round_every_tab_slot() {
    let corners = generate(&StoredParams::new(), None);
    let all = generate(&params(&[("wall_slits", choice("all"))]), None);
    assert!(
        all.parts[0].paths.len() > corners.parts[0].paths.len() || {
            // Notches are not paths of their own: compare the outline's size instead.
            all.parts[0].paths[0].segments.len() > corners.parts[0].paths[0].segments.len()
        }
    );
    let top = bounds(&all.parts[0].paths[0]);
    // The middle of the right straight is slit now (no tab slot there).
    let arc = PI / 2.0 * (R + T / 2.0);
    let s = (W - 2.0 * R) / 2.0 + arc + (D - 2.0 * R) / 2.0;
    let mut met = false;
    let mut x = s - 3.0;
    while x < s + 3.0 {
        met |= !in_material(strip(&all), top.0 + x, top.1 + 20.0);
        x += 0.02;
    }
    assert!(met, "the straight carries slits with Wall slits All round");
}

#[test]
fn a_corner_radius_outside_its_limits_is_clamped_with_a_dependent_notice() {
    use curvyo_extension_core::Bound;
    let low = generate(&params(&[("corner_radius", mm(2.0))]), None);
    let clamp = low
        .clamps
        .iter()
        .find(|c| c.key == "corner_radius")
        .unwrap();
    assert_eq!(clamp.bound, Bound::DependentMinimum);
    assert_eq!(clamp.value, mm(6.0), "two wall thicknesses");
    let high = generate(&params(&[("corner_radius", mm(90.0))]), None);
    let clamp = high
        .clamps
        .iter()
        .find(|c| c.key == "corner_radius")
        .unwrap();
    assert_eq!(clamp.bound, Bound::Dependent);
    assert_eq!(clamp.value, mm(40.0), "half the shorter side");
}

#[test]
fn a_base_inset_between_zero_and_one_wall_thickness_is_raised_to_one_wall_thickness() {
    let g = generate(&params(&[("base_inset", mm(1.0))]), None);
    let clamp = g.clamps.iter().find(|c| c.key == "base_inset").unwrap();
    assert_eq!(clamp.value, mm(3.0));
    let b = bounds(&strip(&g).paths[0]);
    assert!(
        (b.3 - b.1 - height_of_strip(3.0, T, T)).abs() < 1e-9,
        "the strip is taller by the inset"
    );
    let top = bounds(&strip(&g).paths[0]);
    // The base slots lie `base_inset` from the bottom edge: a closed hole now, not a notch.
    let holes: Vec<_> = g.parts[0].paths[1..]
        .iter()
        .filter(|path| {
            let hb = bounds(path);
            (hb.2 - hb.0 - 2.0 * T).abs() < 1e-6 && (top.3 - hb.3 - 3.0).abs() < 1e-6
        })
        .collect();
    assert_eq!(holes.len(), 11);
}

#[test]
fn five_distinct_thicknesses_give_the_strip_height_and_five_layout_runs() {
    let g = generate(
        &params(&[
            ("thickness", mm(3.0)),
            ("plate_thickness", mm(2.0)),
            ("cover_thickness", mm(4.0)),
            ("ring_thickness", mm(5.0)),
            ("lid_thickness", mm(6.0)),
        ]),
        None,
    );
    let b = bounds(&strip(&g).paths[0]);
    assert!((b.3 - b.1 - height_of_strip(0.0, 2.0, 5.0)).abs() < 1e-9);
    // One run per thickness: cover (4), lid (6), inner base (2), ring (5), in this order.
    let widths: Vec<f64> = g.parts[1..].iter().map(|p| bounds(&p.paths[0]).0).collect();
    assert!(
        widths.windows(2).all(|pair| pair[0] < pair[1]),
        "left to right"
    );
    assert_eq!(g.parts.len(), 5);
    // Typed equal thicknesses share a run: the lid after the ring when they differ from the base.
    let shared = generate(
        &params(&[("lid_thickness", mm(2.0)), ("plate_thickness", mm(4.0))]),
        None,
    );
    let x = |index: usize| bounds(&shared.parts[index].paths[0]).0;
    // Runs: 3 mm [cover, ring], 2 mm [lid], 4 mm [inner base]: strip, cover, ring, lid, base.
    assert!(
        x(1) < x(2) && x(2) < x(3) && x(3) < x(4),
        "runs of equal thickness sit together"
    );
}

#[test]
fn the_ring_is_flush_with_the_top_and_its_opening_is_the_footprint_offset_by_two_ring_thicknesses()
{
    let g = generate(&params(&[("ring_thickness", mm(2.0))]), None);
    let ring = &g.parts[4];
    let opening = bounds(&ring.paths[1]);
    assert!(
        (opening.2 - opening.0 - (W - 8.0)).abs() < 1e-6,
        "{opening:?}"
    );
    assert!((opening.3 - opening.1 - (D - 8.0)).abs() < 1e-6);
    // Flush: the strip's top slot for the ring is as deep as the ring is thick.
    let top = bounds(&strip(&g).paths[0]);
    assert_eq!(
        horizontal_centres(&strip(&g).paths[0], top.1 + 2.0, 4.0).len(),
        11
    );
}

#[test]
fn the_finger_hollow_lies_inside_the_lid_and_leaves_it_in_one_piece() {
    let g = generate(&StoredParams::new(), None);
    let lid = &g.parts[2];
    let b = bounds(&lid.paths[0]);
    let mid = f64::midpoint(b.0, b.2);
    assert_eq!(lid.paths.len(), 1, "one piece");
    // The hollow is open at the front edge (bottom) and 8 mm deep.
    assert!(!in_material(lid, mid, b.3 - 1.0));
    assert!(!in_material(lid, mid, b.3 - 7.0));
    assert!(in_material(lid, mid, b.3 - 9.0));
    assert!(in_material(lid, mid + 9.0, b.3 - 1.0), "beside the hollow");
    let without = generate(&params(&[("finger_hollow", on(false))]), None);
    let lid = &without.parts[2];
    assert!(in_material(lid, mid, b.3 - 1.0));
}

#[test]
fn the_lid_options_make_the_parts_the_switches_say() {
    let open = generate(&params(&[("lid", on(false))]), None);
    assert_eq!(open.parts.len(), 3, "strip, cover, inner base");
    let b = bounds(&strip(&open).paths[0]);
    assert!((b.3 - b.1 - height_of_strip(0.0, T, 0.0)).abs() < 1e-9);
    let tabbed = generate(
        &params(&[("lid_seat", on(false)), ("lid_thickness", mm(4.0))]),
        None,
    );
    assert_eq!(
        tabbed.parts.len(),
        4,
        "strip, cover, tabbed lid, inner base"
    );
    let b = bounds(&strip(&tabbed).paths[0]);
    assert!((b.3 - b.1 - height_of_strip(0.0, T, 4.0)).abs() < 1e-9);
    let bare = generate(&params(&[("base_cover", on(false))]), None);
    assert_eq!(bare.parts.len(), 4, "no cover");
}

fn circle() -> StoredParams {
    params(&[
        ("inner_width", mm(60.0)),
        ("inner_depth", mm(60.0)),
        ("corner_radius", mm(30.0)),
    ])
}

/// Every point of `path` that is not on a tab lies on the circle of `radius` round `centre`, or
/// inside it by at most `sag`.
fn assert_on_circle(path: &ClosedPath, centre: (f64, f64), radius: f64, sag: f64, tab_radius: f64) {
    for (x, y) in points(path) {
        let d = libm::hypot(x - centre.0, y - centre.1);
        if d > radius + 0.5 && d > tab_radius - 1.0 {
            continue;
        }
        assert!(
            d <= radius + 1e-6 && d >= radius - sag - 1e-6,
            "({x}, {y}) is {d} from the centre, circle {radius}, sag {sag}"
        );
    }
}

#[test]
fn a_full_round_box_is_an_exact_circle_with_tabs_in_flat_windows() {
    let g = generate(&circle(), None);
    assert_eq!(g.clamps.len(), 0, "no notice at full round");
    // Six windows of 12 mm chords: the strip is the inner perimeter with chords plus pi t.
    let (r, l): (f64, f64) = (30.0, 12.0);
    let window_arc = r * 2.0 * (l / (2.0 * r)).asin();
    let expected = 2.0 * PI * r - 6.0 * window_arc + 6.0 * l + PI * T;
    let b = bounds(&strip(&g).paths[0]);
    assert!(
        (b.2 - b.0 - expected).abs() < 0.1,
        "{} vs {expected}",
        b.2 - b.0
    );
    // Six slots for the inner base, the first half a pitch from the seam.
    let slots = horizontal_centres(&strip(&g).paths[0], b.3 - T, 2.0 * T);
    assert_eq!(slots.len(), 6);
    let length = b.2 - b.0;
    let nearest = slots
        .iter()
        .map(|x| (x - b.0).min(b.2 - x))
        .fold(f64::MAX, f64::min);
    assert!(
        (nearest - length / 12.0).abs() < 1.0,
        "half a pitch from the seam: {nearest}"
    );
    // The plates lie on the circle within the window sag l^2 / 8 r.
    let sag = l * l / (8.0 * r);
    let base = &g.parts[3].paths[0];
    // The outline starts at the seam, the circle's back point.
    let centre = (base.start.x, base.start.y + r);
    assert_on_circle(base, centre, r, sag, r + T);
    // The cover is the full circle; the lid is too, its front edge only has the hollow.
    let ob = bounds(&g.parts[1].paths[0]);
    assert!(
        (ob.2 - ob.0 - 2.0 * (r + T)).abs() < 1e-6,
        "the cover is the full circle"
    );
    assert_no_uncut_band_on_circle(&g, length);
}

/// On the circle every bent stretch (outside the windows) is cut through; the windows are flat.
fn assert_no_uncut_band_on_circle(g: &Generated, length: f64) {
    let top = bounds(&strip(g).paths[0]);
    let slots = horizontal_centres(&strip(g).paths[0], top.3 - T, 2.0 * T);
    let h = top.3 - top.1;
    let mut x = 0.3;
    while x < length - 0.3 {
        let near_window = slots.iter().any(|c| (c - top.0 - x).abs() < 6.0);
        if !near_window {
            let mut y = 0.2;
            let mut met_all = true;
            while y < h - 0.1 {
                // A line across the strip at this height must meet a slot within a pitch (7 mm at r 30).
                let mut met = false;
                let mut dx = -10.0;
                while dx < 10.0 && x + dx < length - 0.3 {
                    if x + dx < 0.0 {
                        dx += 0.05;
                        continue;
                    }
                    if !in_material(strip(g), top.0 + x + dx, top.1 + y) {
                        met = true;
                        break;
                    }
                    dx += 0.05;
                }
                met_all &= met;
                y += 2.0;
            }
            assert!(met_all, "an uncut band near {x}");
        }
        x += 4.0;
    }
}

#[test]
fn a_stadium_keeps_straight_tabs_and_the_strip_length_formula() {
    let g = generate(
        &params(&[
            ("inner_width", mm(100.0)),
            ("inner_depth", mm(40.0)),
            ("corner_radius", mm(20.0)),
        ]),
        None,
    );
    let expected = 2.0 * (100.0 - 40.0) + PI * (2.0 * 20.0 + T);
    let b = bounds(&strip(&g).paths[0]);
    assert!((b.2 - b.0 - expected).abs() < 0.1);
    let slots = horizontal_centres(&strip(&g).paths[0], b.3 - T, 2.0 * T);
    assert_eq!(slots.len(), 4, "two tabs on each straight");
}

#[test]
fn the_circle_outline_is_frozen() {
    golden("rounded_box_circle.txt", &dump(&generate(&circle(), None)));
}

#[test]
fn a_circle_with_five_different_thicknesses_still_mates() {
    let g = generate(
        &{
            let mut p = circle();
            for (key, value) in [
                ("plate_thickness", 2.0),
                ("ring_thickness", 4.0),
                ("lid_thickness", 5.0),
                ("cover_thickness", 6.0),
            ] {
                p.insert(key.to_owned(), mm(value).to_stored());
            }
            p
        },
        None,
    );
    let b = bounds(&strip(&g).paths[0]);
    // The windows use the widest tab: 4 mm half width: 8 + 6 = 14 mm chords; 6 windows.
    let slots_base = horizontal_centres(&strip(&g).paths[0], b.3 - 2.0, 4.0);
    let slots_ring = horizontal_centres(&strip(&g).paths[0], b.1 + 4.0, 8.0);
    assert_eq!(slots_base.len(), slots_ring.len());
    for (a, c) in slots_base.iter().zip(&slots_ring) {
        assert!(
            (a - c).abs() < 1e-6,
            "base and ring slots share their centres"
        );
    }
}

#[test]
fn four_handles_the_corner_radius_one_on_the_inner_base_arc() {
    let g = generate(&StoredParams::new(), None);
    let keys: Vec<&str> = g.handles.iter().map(|h| h.key).collect();
    assert_eq!(
        keys,
        [
            "inner_width",
            "inner_depth",
            "inner_height",
            "corner_radius"
        ]
    );
    // The radius knob sits on the arc's middle point, which is a vertex of the base outline.
    let knob = g.handles.iter().find(|h| h.key == "corner_radius").unwrap();
    let on_outline = points(&g.parts[3].paths[0])
        .iter()
        .any(|p| libm::hypot(p.0 - knob.anchor.x, p.1 - knob.anchor.y) < 1e-6);
    assert!(on_outline, "{:?}", knob.anchor);
    // Dragging along the diagonal by one mm changes the radius by `rate` mm, as the arc moves.
    assert!((knob.rate - 2.414_213_562).abs() < 1e-6, "{}", knob.rate);
}

#[test]
fn a_wave_box_has_curved_slots_clamps_its_spacing_and_stays_closed_with_a_kerf() {
    use curvyo_extension_core::Bound;
    let wave = params(&[("slit_pattern", choice("wave"))]);
    let g = generate(&wave, None);
    assert_eq!(g.parts.len(), 5);
    let straight = generate(&StoredParams::new(), None);
    assert!(
        strip(&g).paths[0].segments.len() > strip(&straight).paths[0].segments.len(),
        "the wave notches are arcs, more cubics than lines"
    );
    // The same slot count and the strip length are those of the Straight pattern.
    let (a, b) = (
        bounds(&strip(&g).paths[0]),
        bounds(&strip(&straight).paths[0]),
    );
    assert!((a.2 - a.0 - (b.2 - b.0)).abs() < 1e-9);
    // Spacing below (width + 1 mm) / 0.88 is raised to it.
    let tight = generate(
        &params(&[("slit_pattern", choice("wave")), ("slit_spacing", mm(1.3))]),
        None,
    );
    let clamp = tight
        .clamps
        .iter()
        .find(|c| c.key == "slit_spacing")
        .unwrap();
    assert_eq!(clamp.bound, Bound::DependentMinimumFor("Wave pattern"));
    let least = clamp.value.length().unwrap().as_mm();
    assert!((least - 1.3 / 0.88).abs() < 1e-9, "{least}");
    // With a kerf every contour still closes and no part is empty.
    let kerfed = generate(
        &params(&[("slit_pattern", choice("wave")), ("kerf", mm(0.2))]),
        None,
    );
    for part in &kerfed.parts {
        for path in &part.paths {
            let end = path.segments.last().unwrap().to;
            assert!(libm::hypot(end.x - path.start.x, end.y - path.start.y) < 1e-6);
        }
    }
}

#[test]
fn the_wave_outline_is_frozen() {
    golden(
        "rounded_box_wave.txt",
        &dump(&generate(
            &params(&[("slit_pattern", choice("wave"))]),
            None,
        )),
    );
}

/// The lid of `g`: the top point of the plan and the middle of the front edge (bottom).
fn lid_front(g: &Generated, plan_height: f64) -> (f64, f64) {
    let b = bounds(&g.parts[2].paths[0]);
    (f64::midpoint(b.0, b.2), b.1 + plan_height)
}

#[test]
fn the_finger_hollow_sits_on_the_front_point_of_a_circle_and_of_a_deep_stadium() {
    let cases = [
        (circle(), 66.0),
        (
            params(&[
                ("inner_width", mm(60.0)),
                ("inner_depth", mm(100.0)),
                ("corner_radius", mm(30.0)),
            ]),
            106.0,
        ),
    ];
    for (stored, plan_height) in cases {
        let g = generate(&stored, None);
        let lid = &g.parts[2];
        assert_eq!(lid.paths.len(), 1, "one piece");
        let (x, front) = lid_front(&g, plan_height);
        assert!(!in_material(lid, x, front - 1.0), "open at the front edge");
        assert!(!in_material(lid, x, front - 7.0), "8 mm deep");
        assert!(in_material(lid, x, front - 9.0), "closed beyond the depth");
        assert!(
            in_material(lid, x + 9.0, front - 2.5),
            "beside the hollow (width 16)"
        );
        assert!(in_material(lid, x - 9.0, front - 2.5));
        let mut off = stored.clone();
        off.insert("finger_hollow".to_owned(), on(false).to_stored());
        let plain = generate(&off, None);
        assert!(
            in_material(&plain.parts[2], x, front - 1.0),
            "no hollow, a plain lid"
        );
    }
}

#[test]
fn a_hollow_wider_than_a_circle_allows_is_clamped_to_the_radius() {
    let mut stored = circle();
    stored.insert("hollow_width".to_owned(), mm(50.0).to_stored());
    let g = generate(&stored, None);
    let clamp = g.clamps.iter().find(|c| c.key == "hollow_width").unwrap();
    assert_eq!(clamp.value, mm(30.0), "the most on a circle is its radius");
}

/// On a circle every height meets a slot within two spacings of every point that is not within a
/// bridge of a window (the check does not skip any window).
fn assert_circle_covered(g: &Generated, t: f64) {
    let top = bounds(&strip(g).paths[0]);
    let length = top.2 - top.0;
    let slots = horizontal_centres(&strip(g).paths[0], top.3 - t, 2.0 * t);
    let spacing = g
        .automatic
        .iter()
        .find(|(key, _)| *key == "slit_spacing")
        .and_then(|(_, value)| value.length())
        .unwrap()
        .as_mm();
    let h = top.3 - top.1;
    let mut x = 0.3;
    while x < length - 0.3 {
        // Windows are 4 t long (2 t each side of the slot centre), plus a bridge of t.
        let near_window = slots.iter().any(|c| (c - top.0 - x).abs() < 3.0 * t);
        // The strip's two ends are free edges: a bridge there is no band across a bend.
        if !near_window && x > t && x < length - t {
            let mut y = 0.2;
            while y < h - 0.1 {
                let mut met = false;
                let mut dx = -2.0 * spacing;
                while dx <= 2.0 * spacing {
                    let at = x + dx;
                    if at > 0.0 && at < length && !in_material(strip(g), top.0 + at, top.1 + y) {
                        met = true;
                        break;
                    }
                    dx += 0.1;
                }
                assert!(met, "an uncut band at x {x}, height {y}");
                y += 2.0;
            }
        }
        x += 2.5;
    }
}

#[test]
fn small_circles_keep_a_tab_and_slit_columns_between_their_windows() {
    for side in [12.0, 20.0, 30.0] {
        let g = generate(
            &params(&[
                ("inner_width", mm(side)),
                ("inner_depth", mm(side)),
                ("corner_radius", mm(side / 2.0)),
            ]),
            None,
        );
        let t = g.params["thickness"]["mm"].as_f64().unwrap();
        let top = bounds(&strip(&g).paths[0]);
        let slots = horizontal_centres(&strip(&g).paths[0], top.3 - t, 2.0 * t);
        assert!(
            !slots.is_empty(),
            "side {side}: the inner base has a tab and a slot"
        );
        assert_circle_covered(&g, t);
    }
}

#[test]
fn all_round_slits_leave_no_uncut_band_anywhere_but_beside_the_tab_slots() {
    for pattern in ["straight", "offset", "wave"] {
        let g = generate(
            &params(&[
                ("wall_slits", choice("all")),
                ("slit_pattern", choice(pattern)),
            ]),
            None,
        );
        assert_circle_covered(&g, T);
    }
}

#[test]
fn the_hollow_may_be_as_wide_as_the_front_straight_and_no_wider() {
    // 120 x 80 at the full corner radius 40: the front straight is 120 - 80 = 40 mm.
    let base = [("corner_radius", mm(40.0))];
    let fits = generate(
        &params(&[base[0].clone(), ("hollow_width", mm(30.0))]),
        None,
    );
    assert!(
        fits.clamps.iter().all(|c| c.key != "hollow_width"),
        "30 mm fits a 40 mm straight: {:?}",
        fits.clamps
    );
    let default = generate(&params(&base), None);
    assert!(
        default.clamps.iter().all(|c| c.key != "hollow_width"),
        "the default 16 mm fits"
    );
    let wide = generate(
        &params(&[base[0].clone(), ("hollow_width", mm(50.0))]),
        None,
    );
    let clamp = wide
        .clamps
        .iter()
        .find(|c| c.key == "hollow_width")
        .unwrap();
    assert_eq!(
        clamp.value,
        mm(40.0),
        "the most is the front straight, once"
    );
}

#[test]
fn the_ghost_draft_has_the_outer_outlines_and_the_ring_opening_but_no_slits() {
    let full = generate(&StoredParams::new(), None);
    let draft = generate(&StoredParams::new(), Some(Edit::Draft));
    assert_eq!(draft.parts.len(), full.parts.len());
    // The strip is its plain rectangle: no slit notch, no slot for a tab.
    assert_eq!(draft.parts[0].paths.len(), 1);
    assert_eq!(draft.parts[0].paths[0].segments.len(), 4);
    // The plates are those of the real box, the ring with its opening.
    for index in 1..5 {
        assert_eq!(draft.parts[index], full.parts[index], "part {index}");
    }
    assert_eq!(draft.parts[4].paths.len(), 2);
    let (a, b) = (
        bounds(&draft.parts[0].paths[0]),
        bounds(&full.parts[0].paths[0]),
    );
    assert!((a.2 - a.0 - (b.2 - b.0)).abs() < 1e-9, "the same length");
}
