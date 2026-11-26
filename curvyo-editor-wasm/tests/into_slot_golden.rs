//! Golden cases of a library page put into slots (`specs/0154-library-into-slots` criteria 4 to
//! 8, 14 and 16): a laser page with a holder of four slots (rectangle 30 x 30, rectangle 60 x 40,
//! circle 25 and a damaged one) and a library page of 60 x 40 mm. Each case is saved, reopened
//! and its laser page content compared with `tests/fixtures/into_slot/<case>.json`; undo of each
//! leaves the page empty. The fixtures are written only when `CURVYO_UPDATE_GOLDEN` is set.

#![allow(clippy::unwrap_used, missing_docs)]

use std::path::PathBuf;

use curvyo_document_core::{
    Affine, AnchorId, CameFrom, Document, DocumentSize, FixtureAdd, FixtureParams, NewAnchor,
    NewFixture, NewOutputPage, NodeId, Point, UnixMillis, Vec2, pack, unpack,
};
use curvyo_editor_wasm::{Session, SlotDrop, SlotSource};
use curvyo_laser_core::FitMode;
use serde_json::{Value, json};

fn params(value: Value) -> FixtureParams {
    let Value::Object(map) = value else {
        panic!("an object")
    };
    map
}

/// The laser page with its holder; the session on it and the laser page's id.
fn laser() -> (Session, String) {
    let d = Document::new(1);
    let page = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
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
                params: &params(json!({"width_mm": 320.0, "height_mm": 160.0})),
                matrix: Some(Affine::translation(Vec2::new(20.0, 30.0))),
            },
        )
        .unwrap();
    for slot in [
        json!({"shape": "rect", "cx_mm": 40.0, "cy_mm": 60.0, "w_mm": 30.0, "h_mm": 30.0}),
        json!({"shape": "rect", "cx_mm": 120.0, "cy_mm": 60.0, "w_mm": 60.0, "h_mm": 40.0}),
        json!({"shape": "circle", "cx_mm": 200.0, "cy_mm": 60.0, "d_mm": 25.0}),
        json!({"shape": "rect", "cx_mm": 280.0, "cy_mm": 60.0, "w_mm": "wide", "h_mm": 20.0}),
    ] {
        d.add_slot(holder, "Slot", &params(slot)).unwrap();
    }
    let mut s = Session::open(2, &pack(&d, "golden").unwrap()).unwrap();
    s.resize_viewport(1200.0, 800.0);
    assert!(s.activate_page(&page.id));
    (s, page.id)
}

/// A library document whose 60 x 40 mm page holds two squares.
fn library() -> Session {
    let d = Document::new(9);
    d.resize_page(
        "first",
        DocumentSize::from_mm(60.0, 40.0),
        &curvyo_document_core::PageContents::default(),
    )
    .unwrap();
    let o = d.pages()[0].origin;
    let mut n = 0;
    let mut square = |x: f64, y: f64| {
        let mut c = |dx: f64, dy: f64| {
            n += 1;
            NewAnchor::corner(AnchorId::new(9, n), Point::new(o.x + x + dx, o.y + y + dy))
        };
        let anchors = [c(0.0, 0.0), c(20.0, 0.0), c(20.0, 20.0), c(0.0, 20.0)];
        d.create_path(&anchors, true)
    };
    square(5.0, 10.0);
    square(35.0, 10.0);
    Session::open(10, &pack(&d, "golden").unwrap()).unwrap()
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

/// The slot ids of the holder, in creation order, from the Into slot list.
fn slots(s: &Session) -> (NodeId, Vec<NodeId>) {
    let view = s.slot_targets_view(None, false);
    assert_eq!(view.holders.len(), 1);
    let holder = &view.holders[0];
    assert_eq!(holder.slots.len(), 3, "the damaged slot is no target");
    let id = |text: &str| s.fixture_part_named(text).unwrap();
    (
        id(&holder.id),
        holder.slots.iter().map(|r| id(&r.id)).collect(),
    )
}

fn round(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// The laser page's content after save and reopen: the box of every leaf on it.
#[allow(clippy::many_single_char_names)]
fn reopened(s: &Session, page: &str) -> Value {
    let d = unpack(3, &s.pack("golden").unwrap()).unwrap();
    let p = d.pages().into_iter().find(|p| p.id == page).unwrap();
    let inside = |x: f64, y: f64| {
        x >= p.origin.x
            && y >= p.origin.y
            && x <= p.origin.x + p.size.width.as_mm()
            && y <= p.origin.y + p.size.height.as_mm()
    };
    let mut rows: Vec<Value> = Vec::new();
    for id in d.object_ids() {
        let Some(object) = d.object(id) else { continue };
        let (min, max) = curvyo_ui_core::object_outline_bounds(&object);
        if !inside(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y)) {
            continue;
        }
        let x = |v: f64| round(v - p.origin.x);
        let y = |v: f64| round(v - p.origin.y);
        rows.push(json!([x(min.x), y(min.y), x(max.x), y(max.y)]));
    }
    rows.sort_by_key(ToString::to_string);
    json!({ "leaves": rows })
}

fn check(name: &str, drop: &SlotDrop, s: &Session, page: &str) {
    let json = json!({
        "refused": drop.refused,
        "notice": drop.notice,
        "page": reopened(s, page),
    });
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/into_slot")
        .join(format!("{name}.json"));
    let text = serde_json::to_string_pretty(&json).unwrap() + "\n";
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &text).unwrap();
    }
    let want: Value = serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("{} exists (CURVYO_UPDATE_GOLDEN=1)", path.display())),
    )
    .unwrap();
    assert_eq!(json, want, "{name} drifted from {}", path.display());
}

fn source<'a>(library: &'a Session, came_from: &'a CameFrom) -> SlotSource<'a> {
    SlotSource {
        source: library,
        roots: &[],
        page: Some("first"),
        came_from,
    }
}

/// Undo of the last step leaves the laser page as it was.
fn undone(s: &mut Session, page: &str, before: &Value) {
    s.undo();
    assert_eq!(&reopened(s, page), before, "undo restores everything");
}

#[test]
fn single_drop_turned_circle_and_real_size() {
    let library = library();
    let came_from = record();
    let from = source(&library, &came_from);
    let cases = [
        ("single_drop", 0, FitMode::Fit),
        ("matched_drop", 1, FitMode::Fit),
        ("circle_fit", 2, FitMode::Fit),
        ("real_size", 0, FitMode::RealSize),
    ];
    for (name, slot, mode) in cases {
        let (mut s, page) = laser();
        let (_, ids) = slots(&s);
        let before = reopened(&s, &page);
        let drop = s.put_into_slot(&from, ids[slot], mode, false).unwrap();
        check(name, &drop, &s, &page);
        undone(&mut s, &page, &before);
    }
}

#[test]
fn turned_fit_into_a_portrait_slot() {
    let library = library();
    let came_from = record();
    let (mut s, page) = laser();
    let (_, ids) = slots(&s);
    // The 60 x 40 slot made 20 x 30: the page is turned and scaled to 50 %.
    s.set_slot_length(ids[1], "w_mm", curvyo_document_core::Length::from_mm(20.0));
    s.set_slot_length(ids[1], "h_mm", curvyo_document_core::Length::from_mm(30.0));
    let drop = s
        .put_into_slot(&source(&library, &came_from), ids[1], FitMode::Fit, false)
        .unwrap();
    check("turned_fit", &drop, &s, &page);
}

#[test]
fn fill_then_refusal_then_replace() {
    let library = library();
    let came_from = record();
    let from = source(&library, &came_from);
    let (mut s, page) = laser();
    let (holder, ids) = slots(&s);
    s.put_into_slot(&from, ids[0], FitMode::Fit, false).unwrap();
    let before = reopened(&s, &page);
    let fill = s.fill_empty_slots(&from, holder, FitMode::Fit).unwrap();
    check("fill_empty_slots", &fill, &s, &page);
    undone(&mut s, &page, &before);
    s.fill_empty_slots(&from, holder, FitMode::Fit).unwrap();
    let refused = s
        .put_into_slot(&from, ids[0], FitMode::RealSize, false)
        .unwrap();
    assert_eq!(refused.refused.as_deref(), Some("Slot 1 is filled."));
    let replaced = s
        .put_into_slot(&from, ids[0], FitMode::RealSize, true)
        .unwrap();
    check("replace", &replaced, &s, &page);
}
