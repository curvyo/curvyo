//! Independent acceptance tests for `specs/0102-laser-fixtures`, written from the specification
//! and ADR 0049 and ADR 0050 before the implementation was read. Covers the pure rules of
//! `curvyo-laser-core`: the fit rule (criterion 25), the parameter ranges (criteria 15, 18 to 23),
//! the kinds (criteria 1, 32), the grid's lines and tapes (criteria 16, 17), the snap points
//! (criterion 27), and the profile record format 2 with the fixture set (criteria 28 to 30).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::assert_is_empty,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::items_after_statements,
    clippy::len_zero,
    clippy::bool_assert_comparison,
    clippy::too_many_lines
)]

use curvyo_document_core::{DocumentSize, Length, Point};
use curvyo_laser_core::{
    FixtureKind, GridSpec, MAX_RECORD_BYTES, MAX_SET_FIXTURES, MAX_SET_SLOTS, MachineProfile,
    Placement, Slot, SlotShape, TapeEdge, TapeEdges, board_points, fit_slot, grid_spacing_range,
    layout, read_record, record_file_name, stop_points, write_record,
};
use serde_json::{Value, json};

const ID: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

fn mm(v: f64) -> Length {
    Length::from_mm(v)
}

fn rect(w: f64, h: f64) -> Slot {
    Slot {
        shape: SlotShape::Rectangle,
        centre: Point::new(0.0, 0.0),
        width: mm(w),
        height: mm(h),
        diameter: mm(0.0),
    }
}

fn circle(d: f64) -> Slot {
    Slot {
        shape: SlotShape::Circle,
        centre: Point::new(0.0, 0.0),
        width: mm(0.0),
        height: mm(0.0),
        diameter: mm(d),
    }
}

fn tol() -> Length {
    mm(0.1)
}

// ---- criterion 25: the fit rule (Frage 173 A, tolerance 0.1 mm) ---------------------------------

#[test]
fn ac25_a_size_within_0_1_mm_fits_and_0_11_does_not() {
    let slots = [rect(20.0, 10.0)];
    let fit = fit_slot((mm(20.09), mm(10.0)), &slots, tol()).expect("0.09 mm fits");
    assert_eq!(fit.slot, 0);
    assert!(!fit.turned);
    assert!(fit_slot((mm(20.11), mm(10.0)), &slots, tol()).is_none());
    assert!(fit_slot((mm(20.0), mm(9.89)), &slots, tol()).is_none());
    assert!(fit_slot((mm(19.91), mm(10.09)), &slots, tol()).is_some());
}

#[test]
fn ac25_a_turned_part_fits_by_a_quarter_turn_and_is_marked_turned() {
    let slots = [rect(20.0, 10.0)];
    let fit = fit_slot((mm(10.05), mm(20.05)), &slots, tol()).expect("turned fit");
    assert!(fit.turned, "the other orientation is the fit");
    assert!(fit_slot((mm(10.0), mm(20.11)), &slots, tol()).is_none());
    assert!(fit_slot((mm(10.11), mm(20.0)), &slots, tol()).is_none());
}

#[test]
fn ac25_a_square_in_a_square_slot_is_never_turned() {
    let slots = [rect(20.0, 20.0)];
    let fit = fit_slot((mm(20.0), mm(20.0)), &slots, tol()).expect("square fits");
    assert!(!fit.turned);
    let fit = fit_slot((mm(19.95), mm(20.05)), &slots, tol()).expect("within tolerance");
    assert!(
        !fit.turned,
        "a square item in a square slot is never turned"
    );
}

#[test]
fn ac25_no_fit_when_no_slot_matches_or_none_exists() {
    assert!(fit_slot((mm(20.0), mm(20.0)), &[], tol()).is_none());
    assert!(
        fit_slot(
            (mm(50.0), mm(50.0)),
            &[rect(20.0, 20.0), circle(12.0)],
            tol()
        )
        .is_none()
    );
    // Zero-size item against the smallest legal slot (0.5 mm).
    assert!(fit_slot((mm(0.0), mm(0.0)), &[rect(0.5, 0.5)], tol()).is_none());
}

#[test]
fn ac25_a_circle_slot_fits_by_its_diameter_square() {
    let fit = fit_slot((mm(12.05), mm(11.95)), &[circle(12.0)], tol()).expect("circle fits");
    assert!(!fit.turned);
    assert_eq!(fit.slot, 0);
}

#[test]
fn ac25_the_first_matching_slot_of_the_holder_is_chosen() {
    let slots = [rect(30.0, 30.0), rect(20.0, 10.0), rect(10.0, 20.0)];
    let fit = fit_slot((mm(20.0), mm(10.0)), &slots, tol()).unwrap();
    assert_eq!(fit.slot, 1);
    assert!(!fit.turned);
    // 10 x 20 fits slot 2 unturned; an unturned fit is taken before any turn.
    let fit = fit_slot((mm(10.0), mm(20.0)), &slots, tol()).unwrap();
    assert_eq!((fit.slot, fit.turned), (2, false));
}

// ---- criteria 15, 18 to 23: parameter ranges and their nearest limits ---------------------------

#[test]
fn ac15_grid_spacing_range_is_1_to_100_mm_with_default_10() {
    let r = grid_spacing_range();
    assert_eq!(r.min.as_mm(), 1.0);
    assert_eq!(r.max.as_mm(), 100.0);
    assert_eq!(r.default.as_mm(), 10.0);
    let (v, changed) = r.clamp(mm(0.0));
    assert_eq!(v.as_mm(), 1.0, "nearest limit");
    assert!(changed);
    let (v, changed) = r.clamp(mm(250.0));
    assert_eq!(v.as_mm(), 100.0);
    assert!(changed);
    let (v, changed) = r.clamp(mm(10.0));
    assert_eq!(v.as_mm(), 10.0);
    assert!(!changed, "an in-range value is not a notice");
}

#[test]
fn ac15_major_every_is_2_to_20_with_default_5() {
    let (lo, hi, default) = curvyo_laser_core::MAJOR_EVERY_RANGE;
    assert_eq!((lo, hi, default), (2, 20, 5));
}

#[test]
fn ac18_22_23_board_clamp_and_stop_ranges() {
    let board = curvyo_laser_core::board_side_range(100.0);
    assert_eq!(board.min.as_mm(), 1.0);
    assert_eq!(board.max.as_mm(), 1000.0);
    assert_eq!(board.clamp(mm(0.2)).0.as_mm(), 1.0);
    assert_eq!(board.clamp(mm(5000.0)).0.as_mm(), 1000.0);
    let leg = curvyo_laser_core::stop_leg_range();
    assert_eq!((leg.min.as_mm(), leg.max.as_mm()), (1.0, 1000.0));
    let thick = curvyo_laser_core::stop_thickness_range();
    assert_eq!((thick.min.as_mm(), thick.max.as_mm()), (0.5, 50.0));
    assert_eq!(thick.clamp(mm(60.0)).0.as_mm(), 50.0);
}

#[test]
fn ac19_20_slot_size_is_0_5_to_1000_mm() {
    let s = curvyo_laser_core::slot_size_range();
    assert_eq!((s.min.as_mm(), s.max.as_mm()), (0.5, 1000.0));
    assert_eq!(s.clamp(mm(0.1)).0.as_mm(), 0.5);
    assert_eq!(s.clamp(mm(1500.0)).0.as_mm(), 1000.0);
}

// ---- criterion 1, 32: kinds --------------------------------------------------------------------

#[test]
fn ac1_32_stored_kinds_read_and_an_unknown_suffix_is_unknown() {
    assert_eq!(
        FixtureKind::from_stored("curvyo.laser/engraved_grid"),
        FixtureKind::EngravedGrid
    );
    assert_eq!(
        FixtureKind::from_stored("curvyo.laser/holder"),
        FixtureKind::Holder
    );
    assert_eq!(
        FixtureKind::from_stored("curvyo.laser/clamp"),
        FixtureKind::Clamp
    );
    assert_eq!(
        FixtureKind::from_stored("curvyo.laser/stop"),
        FixtureKind::Stop
    );
    assert_eq!(
        FixtureKind::from_stored("curvyo.laser/turntable"),
        FixtureKind::Unknown
    );
    assert_eq!(
        FixtureKind::from_stored("curvyo.laser/"),
        FixtureKind::Unknown
    );
    assert_eq!(FixtureKind::from_stored("holder"), FixtureKind::Unknown);
}

// ---- criteria 16, 17: the grid's lines and tapes ------------------------------------------------

fn spec_600x400(major: u32, tapes: TapeEdges) -> GridSpec {
    GridSpec {
        spacing: mm(10.0),
        major_every: major,
        tapes,
        corner: curvyo_document_core::OriginCorner::BottomLeft,
        origin_x: mm(0.0),
        origin_y: mm(0.0),
    }
}

fn no_tapes() -> TapeEdges {
    TapeEdges {
        bottom: false,
        top: false,
        left: false,
        right: false,
    }
}

fn only_bottom() -> TapeEdges {
    TapeEdges {
        bottom: true,
        ..no_tapes()
    }
}

#[test]
fn ac16_grid_10_mm_major_every_5_has_61_and_41_lines_with_13_and_9_major() {
    let g = layout(
        spec_600x400(5, no_tapes()),
        DocumentSize::from_mm(600.0, 400.0),
    );
    let mut vertical: Vec<i64> = Vec::new();
    let mut horizontal: Vec<i64> = Vec::new();
    let mut major_v: Vec<i64> = Vec::new();
    let mut major_h: Vec<i64> = Vec::new();
    for (list, is_major) in [(&g.minor, false), (&g.major, true)] {
        for seg in list {
            if seg.start.x == seg.end.x {
                let x = (seg.start.x.round() * 1000.0) as i64;
                if !vertical.contains(&x) {
                    vertical.push(x);
                }
                if is_major && !major_v.contains(&x) {
                    major_v.push(x);
                }
            } else {
                let y = (seg.start.y.round() * 1000.0) as i64;
                if !horizontal.contains(&y) {
                    horizontal.push(y);
                }
                if is_major && !major_h.contains(&y) {
                    major_h.push(y);
                }
            }
        }
    }
    assert_eq!(
        vertical.len(),
        61,
        "vertical lines incl. both work-area edges"
    );
    assert_eq!(horizontal.len(), 41, "horizontal lines incl. both edges");
    assert_eq!(major_v.len(), 13, "every 5th vertical from the origin");
    assert_eq!(major_h.len(), 9, "every 5th horizontal from the origin");
    assert!(vertical.contains(&0) && vertical.contains(&600_000));
    assert!(horizontal.contains(&0) && horizontal.contains(&400_000));
}

#[test]
fn ac17_a_600_mm_bottom_tape_has_121_ticks_61_long_and_13_numbers() {
    let g = layout(
        spec_600x400(5, only_bottom()),
        DocumentSize::from_mm(600.0, 400.0),
    );
    assert_eq!(g.tapes.len(), 1);
    let tape = &g.tapes[0];
    assert_eq!(tape.edge, TapeEdge::Bottom);
    assert_eq!(tape.ticks.len(), 121, "a tick every 5 mm, both ends");
    assert_eq!(
        tape.ticks.iter().filter(|t| t.long).count(),
        61,
        "every 10 mm"
    );
    let numbers: Vec<i64> = tape.numbers.iter().map(|n| n.mm).collect();
    assert_eq!(numbers, (0..=12).map(|i| i * 50).collect::<Vec<_>>());
}

#[test]
fn ac17_the_tape_numbers_count_from_the_origin_offset() {
    let mut spec = spec_600x400(5, only_bottom());
    spec.origin_x = mm(20.0);
    let g = layout(spec, DocumentSize::from_mm(600.0, 400.0));
    let numbers: Vec<i64> = g.tapes[0].numbers.iter().map(|n| n.mm).collect();
    assert_eq!(
        numbers.first(),
        Some(&0),
        "numbers start at the grid origin"
    );
    assert!(numbers.iter().all(|n| n % 50 == 0));
}

#[test]
fn ac15_all_four_tapes_are_on_by_default_with_all_four_edges() {
    let all = TapeEdges::all();
    assert!(all.bottom && all.top && all.left && all.right);
    let g = layout(spec_600x400(5, all), DocumentSize::from_mm(600.0, 400.0));
    assert_eq!(g.tapes.len(), 4);
}

#[test]
fn ac16_no_tape_when_no_edge_is_on() {
    let g = layout(
        spec_600x400(5, no_tapes()),
        DocumentSize::from_mm(600.0, 400.0),
    );
    assert!(g.tapes.is_empty());
}

// ---- criterion 27: snap points of the fixtures --------------------------------------------------

#[test]
fn ac27_board_points_are_corners_and_edge_midpoints_under_the_placement() {
    let pts = board_points(mm(100.0), mm(60.0), Placement::IDENTITY);
    assert_eq!(pts.len(), 8, "four corners and four edge midpoints");
    for want in [(0.0, 0.0), (100.0, 60.0), (50.0, 0.0), (0.0, 30.0)] {
        assert!(
            pts.iter()
                .any(|p| (p.x - want.0).abs() < 1e-9 && (p.y - want.1).abs() < 1e-9),
            "missing {want:?}"
        );
    }
}

#[test]
fn ac27_stop_points_include_the_outer_corner() {
    let pts = stop_points(mm(60.0), mm(60.0), Placement::IDENTITY);
    assert!(
        pts.iter().any(|p| p.x.abs() < 1e-9 && p.y.abs() < 1e-9),
        "the outer corner is a snap point"
    );
    assert!(!pts.is_empty());
}

// ---- criteria 28 to 30: the profile record of format 2 (ADR 0050) -------------------------------

fn format1() -> Value {
    serde_json::from_str(include_str!(
        "fixtures/machine_profiles/format1_full_record.json"
    ))
    .unwrap()
}

fn record(v: &Value) -> Result<MachineProfile, curvyo_laser_core::RecordError> {
    read_record(&record_file_name(ID), &v.to_string())
}

fn grid_entry() -> Value {
    json!({
        "kind": "engraved_grid", "name": "Grid 1", "active": true,
        "params": { "spacing_mm": 10.0, "major_every": 5,
                    "tapes": ["bottom", "left", "top", "right"],
                    "origin_x_mm": 0.0, "origin_y_mm": 0.0 },
        "matrix": null, "slots": []
    })
}

fn holder_entry(name: &str) -> Value {
    json!({
        "kind": "holder", "name": name, "active": true,
        "params": { "width_mm": 100.0, "height_mm": 60.0 },
        "matrix": [1.0, 0.0, 0.0, 1.0, 40.0, 25.0],
        "slots": [
            { "name": "Slot 1", "shape": "circle", "cx_mm": 30.0, "cy_mm": 30.0,
              "w_mm": 0.0, "h_mm": 0.0, "d_mm": 12.0 },
            { "name": "Slot 2", "shape": "rect", "cx_mm": 70.0, "cy_mm": 30.0,
              "w_mm": 20.0, "h_mm": 10.0, "d_mm": 0.0 }
        ]
    })
}

fn format2_with(fixtures: Value) -> Value {
    let mut v = format1();
    v["format_version"] = json!(2);
    v["fixtures"] = fixtures;
    v
}

#[test]
fn ac28_30_a_format_1_record_reads_as_the_empty_set() {
    let p = record(&format1()).expect("format 1 reads");
    assert!(p.fixtures.is_empty(), "no set in format 1");
    assert_eq!(p.values.name, "K40 in the garage");
}

#[test]
fn ac30_any_write_writes_format_2_with_a_fixtures_array_even_when_empty() {
    let p = record(&format1()).unwrap();
    let text = write_record(&p);
    let back: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(back["format_version"], 2, "a write upgrades format 1");
    assert_eq!(
        back["fixtures"],
        json!([]),
        "the key is written, possibly empty"
    );
}

#[test]
fn ac30_format_2_reads_its_set_in_order_and_writes_it_back() {
    let v = format2_with(json!([grid_entry(), holder_entry("Holder 1")]));
    let p = record(&v).expect("format 2 reads");
    assert_eq!(p.fixtures.len(), 2);
    assert_eq!(p.fixtures[0].kind, "curvyo.laser/engraved_grid");
    assert_eq!(p.fixtures[1].name, "Holder 1");
    assert_eq!(p.fixtures[1].slots.len(), 2);
    let text = write_record(&p);
    let again = read_record(&record_file_name(ID), &text).expect("own write reads");
    assert_eq!(again.fixtures, p.fixtures, "round trip keeps the set");
    let back: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        back["fixtures"][1]["matrix"],
        json!([1.0, 0.0, 0.0, 1.0, 40.0, 25.0])
    );
}

#[test]
fn ac30_a_string_in_a_slot_size_damages_the_whole_record() {
    let mut holder = holder_entry("Holder 1");
    holder["slots"][0]["d_mm"] = json!("twelve");
    assert!(record(&format2_with(json!([holder]))).is_err());
}

#[test]
fn ac30_an_unknown_kind_damages_the_whole_record() {
    let v = format2_with(json!([{
        "kind": "turntable", "name": "T", "active": true, "params": {},
        "matrix": null, "slots": []
    }]));
    assert!(record(&v).is_err());
}

#[test]
fn ac30_33_fixtures_are_over_the_limit_and_32_are_not() {
    assert_eq!(MAX_SET_FIXTURES, 32);
    assert_eq!(MAX_SET_SLOTS, 200);
    let thirty_two: Vec<Value> = (0..32)
        .map(|i| {
            let mut g = grid_entry();
            g["kind"] = json!("clamp");
            g["params"] = json!({ "width_mm": 40.0, "height_mm": 20.0 });
            g["matrix"] = json!([1.0, 0.0, 0.0, 1.0, i as f64, 0.0]);
            g["name"] = json!(format!("Clamp {}", i + 1));
            g
        })
        .collect();
    assert!(
        record(&format2_with(json!(thirty_two))).is_ok(),
        "32 fixtures are the limit"
    );
    let mut thirty_three = thirty_two.clone();
    thirty_three.push(thirty_two[0].clone());
    assert!(
        record(&format2_with(json!(thirty_three))).is_err(),
        "33 is over the limit"
    );
}

#[test]
fn ac30_201_slots_in_the_set_are_over_the_limit() {
    // Eight holders of 25 slots are 200 slots: the limit. One more slot is over it.
    let slot = json!({ "name": "S", "shape": "circle", "cx_mm": 10.0, "cy_mm": 10.0,
                       "w_mm": 0.0, "h_mm": 0.0, "d_mm": 5.0 });
    let holder = |slots: usize| {
        let mut h = holder_entry("H");
        h["slots"] = Value::Array(vec![slot.clone(); slots]);
        h
    };
    let ok = format2_with(json!([
        holder(25),
        holder(25),
        holder(25),
        holder(25),
        holder(25),
        holder(25),
        holder(25),
        holder(25)
    ]));
    assert!(record(&ok).is_ok(), "200 slots are the limit");
    let over = format2_with(json!([
        holder(25),
        holder(25),
        holder(25),
        holder(25),
        holder(25),
        holder(25),
        holder(25),
        holder(26)
    ]));
    assert!(record(&over).is_err(), "201 slots damage the record");
}

#[test]
fn ac30_a_newer_record_format_is_skipped() {
    let mut v = format2_with(json!([]));
    v["format_version"] = json!(3);
    assert!(record(&v).is_err());
}

#[test]
fn ac30_a_fixtures_key_in_a_format_1_record_is_an_unknown_key_and_damages_it() {
    let mut v = format1();
    v["fixtures"] = json!([]);
    assert!(
        record(&v).is_err(),
        "ADR 0050 section 2: a format 1 record has no such key"
    );
}

#[test]
fn ac30_the_record_text_limit_is_64_kib() {
    assert_eq!(MAX_RECORD_BYTES, 64 * 1024);
}

#[test]
fn ac28_generic_laser_holds_an_empty_set() {
    let g = MachineProfile::generic_laser();
    assert!(
        g.fixtures.is_empty(),
        "a new laser with Generic laser starts with no fixture"
    );
    let _ = grid_spacing_range();
}
