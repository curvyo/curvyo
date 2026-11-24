//! `specs/0029-laser-job-preview-and-output` criteria 12 and 36 (d) to (f) at session level, in
//! thread CPU time (ADR 0064 §9): (d) the Output tab's view after a numbers edit on a page with 50
//! operations and 1,000 Cut items of 20 nodes within 16 ms; (e) the Output tab opening within
//! 100 ms; (f) a play slider move redrawing within 16 ms; (12) a pan with the Toolpath view
//! pressed within 16 ms. Budgets are asserted in release builds; a debug build prints the times
//! and checks the rest.

#![allow(clippy::unwrap_used, clippy::cast_precision_loss)]

mod cpu_time;

use std::time::Duration;

use curvyo_document_core::{
    AnchorId, Document, DocumentSize, NewAnchor, NewOperation, NewOutputPage, NodeId, Point,
    SettingsStep, SettingsWrite, pack,
};
use curvyo_editor_wasm::{Session, Tool};
use serde_json::{Value, json};

const FRAME: Duration = Duration::from_millis(16);
const TAB: Duration = Duration::from_millis(100);

/// Whether the times are asserted: release builds only.
const TIMED: bool = !cfg!(debug_assertions);

fn check(name: &str, took: Duration, budget: Duration) {
    eprintln!("{name}: {took:?} (CPU time, budget {budget:?})");
    if TIMED {
        assert!(took <= budget, "{name}: {took:?} > {budget:?}");
    }
}

/// A closed 20-gon of radius 3.5 mm at `(cx, cy)`, anchors numbered from `first`.
fn polygon(d: &Document, cx: f64, cy: f64, first: u64) -> NodeId {
    let anchors: Vec<NewAnchor> = (0..20_u32)
        .map(|k| {
            let a = f64::from(k) * std::f64::consts::TAU / 20.0;
            NewAnchor::corner(
                AnchorId::new(1, first + u64::from(k)),
                Point::new(cx + 3.5 * a.cos(), cy + 3.5 * a.sin()),
            )
        })
        .collect();
    d.create_path(&anchors, true)
}

/// A 400 x 300 mm laser page with 1,000 20-gons in 50 Cut operations with own numbers.
fn page_file() -> (Vec<u8>, String) {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let o = laser.origin;
    let items: Vec<NodeId> = (0..1_000_u64)
        .map(|i| {
            let (col, row) = ((i % 40) as f64, (i / 40) as f64);
            polygon(&d, o.x + 5.0 + col * 9.8, o.y + 5.0 + row * 11.5, i * 20)
        })
        .collect();
    let m = laser.kind.manufacturing().unwrap();
    for k in 0..50 {
        let members: Vec<Vec<NodeId>> = items
            .iter()
            .enumerate()
            .filter(|(i, _)| i % 50 == k)
            .map(|(_, id)| vec![*id])
            .collect();
        let name = format!("Cut {}", k + 1);
        let op = d
            .add_operation(
                m,
                &NewOperation {
                    kind: "cut",
                    name: &name,
                    settings: None,
                },
                k,
                &members,
            )
            .unwrap();
        let Value::Object(value) =
            json!({"main": {"power_pct": 60.0, "speed_mm_min": 3000, "passes": 1}})
        else {
            unreachable!()
        };
        d.write_operation_settings(
            SettingsStep::EditSettings,
            &[SettingsWrite {
                operation: op,
                value: Some(value),
            }],
        )
        .unwrap();
    }
    (pack(&d, "budget").unwrap(), laser.id)
}

fn open() -> (Session, String) {
    let (bytes, laser) = page_file();
    let mut s = Session::open(2, &bytes).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    assert!(s.activate_page(&laser));
    (s, laser)
}

fn first_operation(s: &Session) -> NodeId {
    let view: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    serde_json::from_value(view["rows"][0]["id"].clone()).unwrap()
}

#[test]
fn criterion_36_e_the_operations_tab_with_the_output_group_opens_within_100_ms() {
    // Superseded by 0029 rework (2026-11-17): 36 e is the Operations tab with the output group.
    let (s, _) = open();
    let _ = s.draw_list();
    let ((rows, group), took) =
        cpu_time::cpu_time_of(|| (s.operations_view_json(), s.output_group_json()));
    assert!(rows.contains("\"digit\":50"), "{rows}");
    assert!(group.contains("Runs 50 operations"), "{group}");
    check("criterion 36 e: Operations tab opens", took, TAB);
}

#[test]
fn criterion_36_d_the_gate_follows_a_numbers_edit_within_16_ms() {
    let (mut s, _) = open();
    let _ = s.output_group_json();
    let op = first_operation(&s);
    s.edit_operation_setting(op, "main", "power", 40.0).unwrap();
    let _ = s.draw_list();
    // Superseded by 0029 rework (2026-11-17): the numbers are the row's line 3, read with the
    // output group from the same gate.
    let (json, took) = cpu_time::cpu_time_of(|| s.operations_view_json() + &s.output_group_json());
    assert!(json.contains("40 %, 3,000 mm/min, 1 ×"));
    check("criterion 36 d: gate after a numbers edit", took, FRAME);
}

#[test]
fn criterion_36_f_a_slider_move_redraws_within_16_ms() {
    let (mut s, laser) = open();
    assert!(s.toggle_toolpath_view(&laser));
    let _ = s.draw_list();
    s.set_play_position(0.5);
    let (list, took) = cpu_time::cpu_time_of(|| s.draw_list());
    assert!(!list.triangles.is_empty(), "the frame draws");
    check("criterion 36 f: slider move redraw", took, FRAME);
}

#[test]
fn criterion_12_a_pan_with_the_toolpath_view_pressed_draws_within_16_ms() {
    let (mut s, laser) = open();
    assert!(s.toggle_toolpath_view(&laser));
    let _ = s.draw_list();
    s.pan_to(40.0, 25.0);
    let (_, took) = cpu_time::cpu_time_of(|| s.draw_list());
    check("criterion 12: pan with the Toolpath view", took, FRAME);
}
