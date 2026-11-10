//! `specs/0106-move-versus-edit` criteria 20 and 22 in thread CPU time: classifying 1,000
//! selected objects on 10 pages, a press, a marquee step and a double-click within 16 ms each, a
//! drag of 1,000 out-of-context objects costing what the same drag costs in context, and the
//! laser page's tree painting its first rows within 100 ms with at most 60 rows crossing over.
//! Budgets are asserted in release builds (the `performance-budgets` job); a debug build prints
//! the times and checks the rest.

#![allow(clippy::unwrap_used, clippy::cast_precision_loss)]

mod cpu_time;

use std::time::Duration;

use curvyo_document_core::{
    Affine, Document, DocumentSize, NewOutputPage, Page, Point, RectBounds, pack,
};
use curvyo_editor_wasm::{KeyInput, Session, Tool};

const BUDGET: Duration = Duration::from_millis(16);

/// Whether the times are asserted: release builds only.
const TIMED: bool = !cfg!(debug_assertions);

fn square(d: &Document, x: f64, y: f64) {
    let _ = d.create_rect(RectBounds::from_corners(
        Point::new(x, y),
        Point::new(x + 4.0, y + 4.0),
    ));
}

/// 10 drawing pages of 100 small squares each, and the pages.
fn ten_pages() -> (Vec<u8>, Vec<Page>) {
    let d = Document::new(1);
    for _ in 1..10 {
        d.add_page();
    }
    for page in d.pages() {
        for i in 0..100 {
            let (col, row) = (f64::from(i % 10), f64::from(i / 10));
            square(
                &d,
                page.origin.x + 10.0 + col * 8.0,
                page.origin.y + 10.0 + row * 8.0,
            );
        }
    }
    (pack(&d, "budget").unwrap(), d.pages())
}

fn session(bytes: &[u8], active: &str) -> Session {
    let mut s = Session::open(2, bytes).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    assert!(s.activate_page(active));
    s
}

/// A marquee from far above-left to far below-right: every object, of every context.
fn select_everything(s: &mut Session) {
    let from = Point::new(-5000.0, -5000.0);
    let to = Point::new(50_000.0, 50_000.0);
    s.pointer_down(from, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
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
fn criterion_22_a_classifying_a_thousand_selected_objects_on_ten_pages() {
    let (bytes, _) = ten_pages();
    let cost = cheapest(
        || {
            let mut s = session(&bytes, "first");
            select_everything(&mut s);
            assert_eq!(s.selected_object_count(), 1000);
            s
        },
        |s| {
            assert!(s.selection_move_only());
        },
    );
    eprintln!("classify 1,000 objects: {cost:?}");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}

#[test]
fn criterion_22_b_a_press_a_marquee_step_and_a_double_click() {
    let (bytes, pages) = ten_pages();
    // A square of the second page, pressed while the first is active.
    let second = &pages[1];
    let on_square = Point::new(second.origin.x + 10.0, second.origin.y + 12.0);
    // Warm: a first press and release on empty canvas of the active page reads the document.
    let warm = |s: &mut Session| {
        let empty = Point::new(150.0, 250.0);
        s.pointer_down(empty, false);
        s.pointer_up(empty, false, false);
    };
    let press = cheapest(
        || {
            let mut s = session(&bytes, "first");
            warm(&mut s);
            s
        },
        |s| {
            s.pointer_down(on_square, false);
        },
    );
    let step = cheapest(
        || {
            let mut s = session(&bytes, "first");
            warm(&mut s);
            s.pointer_down(Point::new(-5000.0, -5000.0), false);
            s
        },
        |s| s.pointer_hover(Point::new(50_000.0, 50_000.0), false, false),
    );
    let double = cheapest(
        || {
            let mut s = session(&bytes, "first");
            s.pointer_down(on_square, false);
            s.pointer_up(on_square, false, false);
            s
        },
        |s| {
            s.double_click(on_square, false, false);
        },
    );
    let inside = cheapest(
        || {
            let mut s = session(&bytes, "first");
            warm(&mut s);
            s
        },
        |s| {
            s.pointer_down(Point::new(10.0, 12.0), false);
        },
    );
    eprintln!(
        "press {press:?} (in context {inside:?}), marquee step {step:?}, double-click {double:?}"
    );
    assert!(!TIMED || press < BUDGET, "press {press:?}");
    assert!(!TIMED || step < BUDGET, "marquee step {step:?}");
    assert!(!TIMED || double < BUDGET, "double-click {double:?}");
}

#[test]
fn criterion_22_c_a_drag_out_of_context_costs_what_it_costs_in_context() {
    let (bytes, pages) = ten_pages();
    let drag = |active: &str| {
        cheapest(
            || {
                let mut s = session(&bytes, active);
                select_everything(&mut s);
                // A press on a square of the first page starts the move of all of them.
                s.pointer_down(Point::new(10.0, 12.0), false);
                s.pointer_hover(Point::new(14.0, 12.0), false, false);
                s
            },
            |s| {
                for i in 0..10 {
                    s.pointer_hover(Point::new(20.0 + f64::from(i), 20.0), false, false);
                }
            },
        )
    };
    // In context: the first page active, every object of the other pages out of context in
    // both runs; the drag path is the same, the scope is read once and cached.
    let inside = drag("first");
    let outside = drag(&pages[5].id);
    eprintln!("10 pointer moves of 1,000 objects: in context {inside:?}, out {outside:?}");
    let allowed = inside.mul_f64(1.10) + Duration::from_micros(500);
    assert!(!TIMED || outside <= allowed, "{outside:?} > {allowed:?}");
}

#[test]
fn criterion_20_the_laser_tree_of_a_thousand_items_and_twenty_placed_pages() {
    let d = Document::new(3);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(2000.0, 2000.0),
        })
        .unwrap();
    // Add page takes the size of the last drawing page: Page 1 becomes 60 x 60 mm first, so no
    // later resize moves the laser page and its items along (`specs/0109` criterion 2).
    d.resize_page(
        "first",
        DocumentSize::from_mm(60.0, 60.0),
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    let laser = d.pages().into_iter().find(|q| q.id == laser.id).unwrap();
    for i in 0..1000 {
        let (col, row) = (f64::from(i % 40), f64::from(i / 40));
        square(
            &d,
            laser.origin.x + 10.0 + col * 8.0,
            laser.origin.y + 1500.0 + row * 8.0,
        );
    }
    for p in 0..20 {
        let page = d.add_page();
        for i in 0..50 {
            let (col, row) = (f64::from(i % 10), f64::from(i / 10));
            square(
                &d,
                page.origin.x + 2.0 + col * 5.0,
                page.origin.y + 2.0 + row * 5.0,
            );
        }
        let at = Affine::translation(curvyo_document_core::Vec2::new(
            10.0 + f64::from(p % 10) * 70.0,
            10.0 + f64::from(p / 10) * 70.0,
        ));
        d.place_page(&laser.id, &page.id, at).unwrap();
    }
    let bytes = pack(&d, "budget").unwrap();
    let first = |expand: bool| {
        cheapest(
            || {
                let mut s = session(&bytes, &laser.id);
                if expand {
                    let view: serde_json::Value =
                        serde_json::from_str(&s.layers_view_json(0, 2000)).unwrap();
                    for row in view["rows"].as_array().unwrap() {
                        if row["kind"] == "placed-page" {
                            s.set_layer_expanded(&row["id"].to_string(), true);
                        }
                    }
                    s.key_down(KeyInput {
                        key: "Escape",
                        ..KeyInput::default()
                    });
                }
                s
            },
            |s| {
                let view: serde_json::Value =
                    serde_json::from_str(&s.layers_view_json(0, 60)).unwrap();
                assert!(view["rows"].as_array().unwrap().len() <= 60);
                let count = view["count"].as_u64().unwrap();
                assert!(
                    count >= if expand { 1000 + 20 * 51 } else { 1022 },
                    "{count}"
                );
            },
        )
    };
    let collapsed = first(false);
    let expanded = first(true);
    eprintln!("laser tree first rows: collapsed {collapsed:?}, expanded {expanded:?}");
    assert!(
        !TIMED || collapsed < Duration::from_millis(100),
        "{collapsed:?}"
    );
    assert!(
        !TIMED || expanded < Duration::from_millis(100),
        "{expanded:?}"
    );
}
