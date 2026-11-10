//! `specs/0102-laser-fixtures` criterion 35 in thread CPU time: (a) a pointer move of a drag on a
//! laser page with 4 holders of 15 slots each and 1,000 own items, fit and snap included, within
//! 16 ms; (b) a press resolved on a page with 50 fixtures within 2 ms; (c) the Fixtures block of
//! 50 fixtures within 100 ms; (d) the draw list of a grid at 1 mm over 1,000 x 1,000 mm, on a
//! cache miss, within 16 ms. Budgets are asserted in release builds (the `performance-budgets`
//! job); a debug build prints the times and checks the rest.

#![allow(clippy::unwrap_used, clippy::cast_precision_loss)]

mod cpu_time;

use std::time::Duration;

use curvyo_document_core::{
    Affine, Document, DocumentSize, DocumentSnapping, FixtureAdd, FixtureParams, NewFixture,
    NewOutputPage, NodeId, Point, RectBounds, Vec2, pack,
};
use curvyo_editor_wasm::{Session, Tool};
use serde_json::{Value, json};

const FRAME: Duration = Duration::from_millis(16);
const PRESS: Duration = Duration::from_millis(2);
const BLOCK: Duration = Duration::from_millis(100);

/// Whether the times are asserted: release builds only.
const TIMED: bool = !cfg!(debug_assertions);

fn params(value: Value) -> FixtureParams {
    let Value::Object(map) = value else {
        panic!("an object")
    };
    map
}

/// A laser page of `w` by `h` mm; the document, its page id and its manufacturing.
fn laser(w: f64, h: f64) -> (Document, String, NodeId) {
    let d = Document::new(1);
    let page = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(w, h),
        })
        .unwrap();
    let m = page.kind.manufacturing().unwrap();
    (d, page.id, m)
}

fn holder(d: &Document, m: NodeId, at: (f64, f64), slots: u32) -> NodeId {
    let id = d
        .add_fixture(
            m,
            &NewFixture {
                add: FixtureAdd::Holder,
                kind: "curvyo.laser/holder",
                label: "Holder",
                params: &params(json!({"width_mm": 150.0, "height_mm": 100.0})),
                matrix: Some(Affine::translation(Vec2::new(at.0, at.1))),
            },
        )
        .unwrap();
    for k in 0..slots {
        let (cx, cy) = (
            15.0 + f64::from(k % 5) * 30.0,
            15.0 + f64::from(k / 5) * 30.0,
        );
        let slot = json!({"shape": "rect", "cx_mm": cx, "cy_mm": cy, "w_mm": 20.0, "h_mm": 20.0});
        d.add_slot(id, "Slot", &params(slot)).unwrap();
    }
    id
}

fn open_on(d: &Document, page: &str) -> Session {
    let mut s = Session::open(2, &pack(d, "budget").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    assert!(s.activate_page(page));
    s.show_default_view();
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

#[test]
fn criterion_35_a_a_drag_move_with_sixty_slots_and_a_thousand_items() {
    let (d, page, m) = laser(800.0, 600.0);
    for k in 0..4 {
        holder(&d, m, (20.0 + f64::from(k) * 190.0, 20.0), 15);
    }
    let origin = d.pages().into_iter().find(|p| p.id == page).unwrap().origin;
    for i in 0..1000 {
        let (x, y) = (
            20.0 + f64::from(i % 50) * 15.0,
            200.0 + f64::from(i / 50) * 15.0,
        );
        let at = Point::new(origin.x + x, origin.y + y);
        let _ = d.create_rect(RectBounds::from_corners(
            at,
            Point::new(at.x + 10.0, at.y + 10.0),
        ));
    }
    d.set_snapping(DocumentSnapping {
        enabled: true,
        ..d.snapping()
    });
    let grip = Point::new(origin.x + 20.0, origin.y + 205.0);
    let cost = cheapest(
        || {
            let mut s = open_on(&d, &page);
            s.pointer_down(grip, false);
            s.pointer_hover(Point::new(grip.x + 5.0, grip.y - 5.0), false, false);
            assert_eq!(s.selected_object_count(), 1);
            s
        },
        |s| s.pointer_hover(Point::new(grip.x + 40.0, grip.y - 150.0), false, false),
    );
    // The measured move is a real move: the release lands the square elsewhere.
    let mut s = open_on(&d, &page);
    let steps = s.undo_len();
    s.pointer_down(grip, false);
    s.pointer_hover(Point::new(grip.x + 5.0, grip.y - 5.0), false, false);
    s.pointer_hover(Point::new(grip.x + 40.0, grip.y - 150.0), false, false);
    s.pointer_up(Point::new(grip.x + 40.0, grip.y - 150.0), false, false);
    assert_eq!(s.undo_len(), steps + 1, "the drag moved the square");
    eprintln!("criterion 35 a: one drag move {cost:?} (CPU time)");
    assert!(!TIMED || cost < FRAME, "{cost:?}");
}

#[test]
fn criterion_35_b_a_press_on_a_page_with_fifty_fixtures() {
    let (d, page, m) = laser(1000.0, 1000.0);
    for k in 0..50 {
        holder(
            &d,
            m,
            (f64::from(k % 7) * 140.0, f64::from(k / 7) * 120.0),
            2,
        );
    }
    let origin = d.pages().into_iter().find(|p| p.id == page).unwrap().origin;
    // The top edge of the last holder.
    let edge = Point::new(origin.x + 75.0, origin.y + 7.0 * 120.0);
    let cost = cheapest(|| open_on(&d, &page), |s| s.pointer_down(edge, false));
    let mut s = open_on(&d, &page);
    s.pointer_down(edge, false);
    assert!(s.selected_fixture().is_some(), "the press hit a holder");
    eprintln!("criterion 35 b: a press among 50 fixtures {cost:?} (CPU time)");
    assert!(!TIMED || cost < PRESS, "{cost:?}");
}

#[test]
fn criterion_35_c_the_fixtures_block_with_fifty_fixtures() {
    let (d, page, m) = laser(1000.0, 1000.0);
    for k in 0..50 {
        holder(
            &d,
            m,
            (f64::from(k % 7) * 140.0, f64::from(k / 7) * 120.0),
            2,
        );
    }
    let cost = cheapest(
        || open_on(&d, &page),
        |s| assert!(s.fixtures_view_json().len() > 1000),
    );
    eprintln!("criterion 35 c: the Fixtures block of 50 fixtures {cost:?} (CPU time)");
    assert!(!TIMED || cost < BLOCK, "{cost:?}");
}

#[test]
fn criterion_35_d_the_draw_list_of_a_grid_at_one_mm_over_a_square_metre() {
    let (d, page, m) = laser(1000.0, 1000.0);
    let grid = d
        .add_fixture(
            m,
            &NewFixture {
                add: FixtureAdd::Grid,
                kind: "curvyo.laser/engraved_grid",
                label: "Grid",
                params: &params(json!({"spacing_mm": 2.0})),
                matrix: None,
            },
        )
        .unwrap();
    let cost = cheapest(
        || {
            let mut s = open_on(&d, &page);
            // Enough device pixels per mm that every minor line is drawn.
            s.set_device_pixel_ratio(6.0);
            let _ = s.frame_draw_list();
            assert!(s.set_fixture_length(
                grid,
                "spacing_mm",
                curvyo_document_core::Length::from_mm(1.0)
            ));
            s
        },
        |s| assert_ne!(s.frame_draw_list().triangles.len(), 0),
    );
    eprintln!("criterion 35 d: grid draw list at 1 mm, cache miss {cost:?} (CPU time)");
    assert!(!TIMED || cost < FRAME, "{cost:?}");
}

/// `adrs.md` §8 (e): the commit of a typed size about the centre of a holder with 50 slots (its
/// sizes, matrix and 50 slot centres in one step) within 16 ms.
#[test]
fn adrs_8_e_a_typed_resize_of_a_holder_with_fifty_slots_commits_in_a_frame() {
    let (d, page, m) = laser(800.0, 600.0);
    let id = holder(&d, m, (100.0, 100.0), 50);
    let cost = cheapest(
        || {
            let mut s = open_on(&d, &page);
            assert!(s.select_fixture(id));
            let opened = s.key_down(curvyo_editor_wasm::KeyInput {
                key: "s",
                ..curvyo_editor_wasm::KeyInput::default()
            });
            assert_eq!(opened, curvyo_editor_wasm::KeyOutcome::EntryOpened);
            s
        },
        |s| {
            let outcome = s.commit_transform_entry("200", "140", 0);
            assert_eq!(outcome, curvyo_ui_core::EntryOutcome::Committed);
        },
    );
    eprintln!("adrs 8 e: one resize commit with 50 slots {cost:?} (CPU time)");
    assert!(!TIMED || cost < FRAME, "{cost:?}");
}
