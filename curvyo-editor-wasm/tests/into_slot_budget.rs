//! The budgets of `specs/0154-library-into-slots` criterion 8 (ADR 0088 decision 9), thread CPU
//! time (`CLAUDE.md` §5): a fill of 50 slots with a page of 200 objects within 500 ms, a fill of
//! 15 slots within 150 ms, and the Into slot list of 50 filled slots (10,000 objects) within
//! 50 ms. Asserted in a release build; a debug build only prints.

#![allow(clippy::unwrap_used, clippy::cast_precision_loss, missing_docs)]

mod cpu_time;

use curvyo_document_core::{
    Affine, AnchorId, CameFrom, Document, DocumentSize, FixtureAdd, FixtureParams, NewAnchor,
    NewFixture, NewOutputPage, Point, UnixMillis, Vec2, pack,
};
use curvyo_editor_wasm::{Session, SlotSource};
use curvyo_laser_core::FitMode;
use serde_json::{Value, json};

fn params(value: Value) -> FixtureParams {
    let Value::Object(map) = value else {
        panic!("an object")
    };
    map
}

/// A laser page with one holder of `slots` slots of 30 x 30 mm.
fn laser(slots: u32) -> Session {
    let d = Document::new(1);
    let page = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(600.0, 400.0),
        })
        .unwrap();
    let m = page.kind.manufacturing().unwrap();
    let holder = d
        .add_fixture(
            m,
            &NewFixture {
                add: FixtureAdd::Holder,
                kind: "curvyo.laser/holder",
                label: "Holder",
                params: &params(json!({"width_mm": 500.0, "height_mm": 250.0})),
                matrix: Some(Affine::translation(Vec2::new(20.0, 20.0))),
            },
        )
        .unwrap();
    for n in 0..slots {
        let (col, row) = (f64::from(n % 10), f64::from(n / 10));
        let slot = json!({"shape": "rect", "cx_mm": 25.0 + col * 48.0, "cy_mm": 25.0 + row * 48.0,
            "w_mm": 30.0, "h_mm": 30.0});
        d.add_slot(holder, "Slot", &params(slot)).unwrap();
    }
    let mut s = Session::open(2, &pack(&d, "budget").unwrap()).unwrap();
    s.resize_viewport(1200.0, 800.0);
    assert!(s.activate_page(&page.id));
    s
}

/// A 60 x 40 mm library page with 200 small triangles.
#[allow(clippy::many_single_char_names)]
fn library() -> Session {
    let d = Document::new(9);
    d.resize_page(
        "first",
        DocumentSize::from_mm(60.0, 40.0),
        &curvyo_document_core::PageContents::default(),
    )
    .unwrap();
    let o = d.pages()[0].origin;
    for n in 0..200_u64 {
        let (x, y) = (
            o.x + 1.0 + (n % 20) as f64 * 2.9,
            o.y + 1.0 + (n / 20) as f64 * 3.8,
        );
        let c = |k: u64, dx: f64, dy: f64| {
            NewAnchor::corner(AnchorId::new(9, n * 3 + k), Point::new(x + dx, y + dy))
        };
        let _ = d.create_path(&[c(0, 0.0, 0.0), c(1, 2.0, 0.0), c(2, 1.0, 2.0)], true);
    }
    Session::open(10, &pack(&d, "budget").unwrap()).unwrap()
}

fn record() -> CameFrom {
    CameFrom {
        document: "Box job".into(),
        page: "Page 1".into(),
        page_id: "first".into(),
        item: None,
        at: UnixMillis::new(0),
        online: None,
    }
}

/// `seconds` against `budget`: asserted in a release build, printed always.
fn report(name: &str, seconds: f64, budget: f64) {
    eprintln!("{name}: {seconds:.4} s CPU (budget {budget} s)");
    if !cfg!(debug_assertions) {
        assert!(seconds <= budget, "{name}: {seconds:.4} s > {budget} s");
    }
}

/// Fills a holder of `slots` slots with the library page; the session and the fill's CPU time.
fn filled(slots: u32) -> (Session, f64) {
    let library = library();
    let came_from = record();
    let from = SlotSource {
        source: &library,
        roots: &[],
        page: Some("first"),
        came_from: &came_from,
    };
    let mut s = laser(slots);
    let view = s.slot_targets_view(None, false);
    assert_eq!(view.holders[0].slots.len(), slots as usize);
    let holder = s.fixture_part_named(&view.holders[0].id).unwrap();
    let (done, time) = cpu_time::cpu_time_of(|| s.fill_empty_slots(&from, holder, FitMode::Fit));
    assert_eq!(done.unwrap().refused, None);
    (s, time.as_secs_f64())
}

#[test]
fn a_fill_of_50_slots_with_a_page_of_200_objects_takes_within_500_ms() {
    let (_, seconds) = filled(50);
    report("fill of 50 slots x 200 objects", seconds, 0.500);
}

#[test]
fn a_fill_of_15_slots_with_a_page_of_200_objects_takes_within_150_ms() {
    let (_, seconds) = filled(15);
    report("fill of 15 slots x 200 objects", seconds, 0.150);
}

#[test]
fn the_into_slot_list_of_50_filled_slots_is_built_within_50_ms() {
    let (s, _) = filled(50);
    let (view, time) = cpu_time::cpu_time_of(|| s.slot_targets_view(None, false));
    assert_eq!(view.holders[0].filled, 50);
    report(
        "Into slot list of 50 filled slots",
        time.as_secs_f64(),
        0.050,
    );
}
