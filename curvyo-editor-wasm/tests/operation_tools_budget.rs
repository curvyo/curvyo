//! `specs/0111-laser-operation-tools` criterion 26 in thread CPU time (`adrs.md` "Goldens and
//! budgets"): a kind press assigning 1,000 items, a kind press assigning a group of 1,000 leaves,
//! entering a group of 1,000 leaves, and unfolding a row on a page with 50 operations (the press
//! and the tab's JSON the host reads after it), each within 16 ms. Budgets are asserted in release
//! builds (the `performance-budgets` job); a debug build prints the times and checks the rest.

#![allow(clippy::unwrap_used)]

mod cpu_time;

use std::time::Duration;

use curvyo_document_core::{
    Document, DocumentSize, NewOperation, NewOutputPage, NodeId, Point, RectBounds, pack,
};
use curvyo_editor_wasm::{KeyInput, Session, Tool};

const BUDGET: Duration = Duration::from_millis(16);

/// Whether the times are asserted: release builds only.
const TIMED: bool = !cfg!(debug_assertions);

/// A laser page of 400 x 300 mm with 1,000 squares, grouped into one group when `grouped`, and
/// `operations` empty Cut operations. Returns the file and the laser page id.
fn page_file(grouped: bool, operations: usize) -> (Vec<u8>, String) {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let origin = laser.origin;
    let squares: Vec<NodeId> = (0..1000_u32)
        .map(|i| {
            let (col, row) = (f64::from(i % 40), f64::from(i / 40));
            let x = origin.x + 2.0 + col * 9.0;
            let y = origin.y + 2.0 + row * 9.0;
            d.create_rect(RectBounds::from_corners(
                Point::new(x, y),
                Point::new(x + 6.0, y + 6.0),
            ))
        })
        .collect();
    if grouped {
        d.group_objects(&squares).unwrap();
    }
    let m = laser.kind.manufacturing().unwrap();
    for (k, square) in squares.iter().take(operations).enumerate() {
        let index = d.operations(m).unwrap().len();
        d.add_operation(
            m,
            &NewOperation {
                kind: "cut",
                name: &format!("Cut {}", k + 1),
                settings: None,
            },
            index,
            &[vec![*square]],
        )
        .unwrap();
    }
    (pack(&d, "budget").unwrap(), laser.id)
}

fn open_on(bytes: &[u8], laser: &str) -> Session {
    let mut s = Session::open(2, bytes).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    assert!(s.activate_page(laser));
    s
}

/// The cheapest of five runs of `work` on fresh sessions made by `make`.
fn cheapest<S>(mut make: impl FnMut() -> S, mut work: impl FnMut(&mut S)) -> Duration {
    (0..5)
        .map(|_| {
            let mut s = make();
            cpu_time::cpu_time_of(|| work(&mut s)).1
        })
        .min()
        .unwrap()
}

/// The page's 1,000 squares selected (the group's unit when they are grouped).
fn select_squares(s: &mut Session) {
    assert!(s.select_operation_items("not_assigned"));
    assert_eq!(s.selected_object_count(), 1000);
}

#[test]
fn criterion_26_a_kind_press_assigning_a_thousand_items() {
    let (bytes, laser) = page_file(false, 0);
    let cost = cheapest(
        || {
            let mut s = open_on(&bytes, &laser);
            select_squares(&mut s);
            let _ = s.operations_view_json();
            s
        },
        |s| assert!(s.press_operation_kind("engrave", false).is_some()),
    );
    eprintln!("criterion 26: kind press on 1000 items {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}

#[test]
fn criterion_26_a_kind_press_assigning_a_group_of_a_thousand_leaves() {
    let (bytes, laser) = page_file(true, 0);
    let cost = cheapest(
        || {
            let mut s = open_on(&bytes, &laser);
            select_squares(&mut s);
            let _ = s.operations_view_json();
            s
        },
        |s| assert!(s.press_operation_kind("cut", false).is_some()),
    );
    eprintln!("criterion 26: kind press on a group of 1000 leaves {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}

#[test]
fn criterion_26_entering_a_group_of_a_thousand_leaves() {
    let (bytes, laser) = page_file(true, 0);
    let cost = cheapest(
        || {
            let mut s = open_on(&bytes, &laser);
            select_squares(&mut s);
            s
        },
        |s| {
            s.key_down(KeyInput {
                key: "Enter",
                ..KeyInput::default()
            });
            assert_eq!(s.context_depth(), 1);
        },
    );
    eprintln!("criterion 26: entering a group of 1000 leaves {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}

#[test]
fn criterion_26_unfolding_a_row_on_a_page_with_fifty_operations() {
    let (bytes, laser) = page_file(false, 50);
    let cost = cheapest(
        || {
            let s = open_on(&bytes, &laser);
            let id = s.operations_view().unwrap().rows[25].id;
            (s, id)
        },
        |(s, id)| {
            assert!(s.unfold_operation_row(*id));
            assert!(s.operations_view_json().contains("\"unfolded\":true"));
        },
    );
    eprintln!("criterion 26: unfolding a row among 50 operations {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}

/// `specs/0139` criterion 25 (b), adrs L13: a selected 20-gon with 20 own tabs and a set start
/// redraws its tab handles and start marker within 16 ms per pointer move.
#[test]
fn criterion_0139_25_the_tab_handles_of_a_selected_item_redraw_within_16_ms() {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let origin = laser.origin;
    let corners: Vec<curvyo_document_core::NewAnchor> = (0..20_u32)
        .map(|k| {
            let a = f64::from(k) * std::f64::consts::TAU / 20.0;
            curvyo_document_core::NewAnchor::corner(
                curvyo_document_core::AnchorId::new(1, u64::from(k) + 1),
                Point::new(
                    origin.x + 100.0 + 60.0 * a.cos(),
                    origin.y + 100.0 + 60.0 * a.sin(),
                ),
            )
        })
        .collect();
    let path = d.create_path(&corners, true);
    let m = laser.kind.manufacturing().unwrap();
    let cut = d
        .add_operation(
            m,
            &NewOperation {
                kind: "cut",
                name: "Cut 1",
                settings: None,
            },
            0,
            &[vec![path]],
        )
        .unwrap();
    let tabs: Vec<serde_json::Value> = (0..20_u64)
        .map(|k| serde_json::json!({ "at": k * 50_000_000 + 10_000_000 }))
        .collect();
    let record = serde_json::json!({ "0": tabs });
    d.write_operation_tabs(
        cut,
        &curvyo_document_core::member_key_text(&[path]),
        record.as_object(),
        curvyo_document_core::ADD_TAB_COMMIT_LABEL,
    )
    .unwrap();
    let entry = serde_json::json!({ "0": { "start": 30_000_000 } });
    d.write_operation_records(
        cut,
        &[(
            curvyo_document_core::ItemRecord::Entry,
            curvyo_document_core::member_key_text(&[path]),
            entry.as_object().cloned(),
        )],
        curvyo_document_core::MOVE_START_POINT_COMMIT_LABEL,
    )
    .unwrap();
    let bytes = pack(&d, "budget").unwrap();
    let centre = Point::new(origin.x + 100.0, origin.y + 100.0);
    let cost = cheapest(
        || {
            let mut s = open_on(&bytes, &laser.id);
            s.pointer_down(Point::new(centre.x + 60.0, centre.y), false);
            s.pointer_up(Point::new(centre.x + 60.0, centre.y), false, false);
            assert_eq!(s.selected_object_count(), 1);
            let _ = s.draw_list();
            s
        },
        |s| {
            s.pointer_hover(Point::new(centre.x + 30.0, centre.y + 5.0), false, false);
            let _ = s.draw_list();
        },
    );
    eprintln!("0139 criterion 25 (b): tab handles redraw {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}
