//! Independent acceptance tests for `specs/0154-library-into-slots` at session level (criteria 1
//! to 5, 7 to 10): the drag cue, the put as one undo step, the refused filled slot and the armed
//! Replace, the fill with its count, the Into slot list, the non-target drop. Written from the
//! specification before the implementation was read; the set-up follows the public API used by
//! the golden tests of 0154 (a laser page, a holder with slots, a library page of 60 x 40 mm).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]

use curvyo_document_core::{
    Affine, AnchorId, CameFrom, Document, DocumentSize, FixtureAdd, FixtureParams, NewAnchor,
    NewFixture, NewOutputPage, NodeId, Point, UnixMillis, Vec2, pack, unpack,
};
use curvyo_editor_wasm::{Session, SlotSource, UnitBox};
use curvyo_laser_core::FitMode;
use serde_json::{Value, json};

fn params(value: Value) -> FixtureParams {
    let Value::Object(map) = value else {
        panic!("an object")
    };
    map
}

/// A laser page with a holder at (10, 10) and three target slots (30 x 30 at x 50, 60 x 40 at
/// x 150, circle 25 at x 250, all at y 100 mm from the board) and a damaged fourth slot.
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
                params: &params(json!({"width_mm": 320.0, "height_mm": 240.0})),
                matrix: Some(Affine::translation(Vec2::new(10.0, 10.0))),
            },
        )
        .unwrap();
    for slot in [
        json!({"shape": "rect", "cx_mm": 50.0, "cy_mm": 100.0, "w_mm": 30.0, "h_mm": 30.0}),
        json!({"shape": "rect", "cx_mm": 150.0, "cy_mm": 100.0, "w_mm": 60.0, "h_mm": 40.0}),
        json!({"shape": "circle", "cx_mm": 250.0, "cy_mm": 100.0, "d_mm": 25.0}),
        json!({"shape": "rect", "cx_mm": 300.0, "cy_mm": 200.0, "w_mm": "wide", "h_mm": 20.0}),
    ] {
        d.add_slot(holder, "Slot", &params(slot)).unwrap();
    }
    let mut s = Session::open(2, &pack(&d, "accept").unwrap()).unwrap();
    s.resize_viewport(1200.0, 800.0);
    assert!(s.activate_page(&page.id));
    (s, page.id)
}

/// A library document whose 60 x 40 mm page holds two 20 x 20 squares.
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
    Session::open(10, &pack(&d, "accept").unwrap()).unwrap()
}

fn record() -> CameFrom {
    CameFrom {
        document: "Box job".into(),
        page: "Coaster".into(),
        page_id: "first".into(),
        item: None,
        at: UnixMillis::new(0),
        online: None,
    }
}

fn source<'a>(library: &'a Session, came_from: &'a CameFrom) -> SlotSource<'a> {
    SlotSource {
        source: library,
        roots: &[],
        page: Some("first"),
        came_from,
    }
}

/// The unit of a 60 x 40 mm library page, as the drag and the list see it.
fn unit_60x40() -> UnitBox {
    UnitBox {
        size: (
            curvyo_document_core::Length::from_mm(60.0),
            curvyo_document_core::Length::from_mm(40.0),
        ),
        centre: Point::new(0.0, 0.0),
    }
}

/// The absolute centre of the slot `cx`, `cy` (mm from the board corner): the page origin, the
/// holder's offset (10, 10), then the slot's centre.
fn slot_point(s: &Session, cx: f64, cy: f64) -> Point {
    let o = s.active_page().origin;
    Point::new(o.x + 10.0 + cx, o.y + 10.0 + cy)
}

/// The top-level members of the active laser page.
fn members(s: &Session) -> Vec<NodeId> {
    s.member_slots().into_iter().map(|(id, _)| id).collect()
}

/// Text with the no-break spaces the views print for "50 %" read as plain spaces.
fn plain(text: &str) -> String {
    text.replace('\u{a0}', " ")
}

fn holder_rows(s: &Session) -> Vec<Value> {
    let view = s.slot_targets_view(None, false);
    assert_eq!(view.holders.len(), 1, "one holder with target slots");
    view.holders[0]
        .slots
        .iter()
        .map(|r| json!({"name": r.name, "filled": r.filled, "outcome": r.outcome}))
        .collect()
}

#[test]
fn ac1_the_cue_names_the_slot_with_its_scale_and_the_real_size_word() {
    let (mut s, _) = laser();
    s.begin_slot_drop(Some(unit_60x40()));
    let at = slot_point(&s, 50.0, 100.0);
    let cue = serde_json::to_value(s.slot_drop_cue(at, false, false)).unwrap();
    assert_eq!(cue["kind"], "slot", "the slot lights: {cue}");
    let badge = cue["badge"].as_str().unwrap();
    assert!(
        plain(badge).contains("Into Slot"),
        "badge names the slot: {badge}"
    );
    assert!(
        plain(badge).contains("50 %"),
        "badge carries the scale: {badge}"
    );
    let alt = serde_json::to_value(s.slot_drop_cue(at, true, false)).unwrap();
    assert!(
        alt["badge"].as_str().unwrap().contains("real size"),
        "Alt says real size"
    );
    s.end_slot_drop();
}

#[test]
fn ac1_over_no_slot_the_cue_is_none_and_the_damaged_slot_is_not_a_target() {
    let (mut s, _) = laser();
    s.begin_slot_drop(Some(unit_60x40()));
    let open_board = slot_point(&s, 10.0, 10.0);
    let cue = serde_json::to_value(s.slot_drop_cue(open_board, false, false)).unwrap();
    assert_eq!(
        cue["kind"], "none",
        "inside the board outside every slot: {cue}"
    );
    let damaged = slot_point(&s, 300.0, 200.0);
    let cue = serde_json::to_value(s.slot_drop_cue(damaged, false, false)).unwrap();
    assert_eq!(
        cue["kind"], "none",
        "a damaged slot is no target (criterion 9): {cue}"
    );
    s.end_slot_drop();
    assert_eq!(holder_rows(&s).len(), 3, "the damaged slot is not listed");
}

#[test]
fn ac1_shift_over_a_slot_is_the_fill_cue() {
    let (mut s, _) = laser();
    s.begin_slot_drop(Some(unit_60x40()));
    let at = slot_point(&s, 50.0, 100.0);
    let cue = serde_json::to_value(s.slot_drop_cue(at, false, true)).unwrap();
    assert_eq!(
        cue["kind"], "fill",
        "Shift over a slot fills the holder: {cue}"
    );
    assert!(
        cue["badge"].as_str().unwrap().contains("Fill 3 of 3 slots"),
        "{cue}"
    );
    s.end_slot_drop();
}

#[test]
fn ac4_and_ac5_a_drop_is_one_undo_step_named_put_into_slot_with_a_scale_notice() {
    let (mut s, page) = laser();
    let library = library();
    let came = record();
    let before = members(&s).len();
    let undo_before = s.undo_len();
    let drop = s
        .put_into_slot(
            &source(&library, &came),
            slot_id(&s, 0),
            FitMode::Fit,
            false,
        )
        .unwrap();
    assert!(drop.refused.is_none(), "an empty slot takes the drop");
    let notice = drop.notice.unwrap();
    assert!(
        plain(&notice).contains("Fitted into Slot 1 at 50 %."),
        "criterion 5: the notice says what happened: {notice}"
    );
    assert!(
        notice.chars().count() <= 70,
        "notice of at most 70 characters"
    );
    assert_eq!(s.undo_len(), undo_before + 1, "one undo step");
    assert_eq!(
        s.history_target_name(curvyo_document_core::Direction::Undo)
            .as_deref(),
        Some("Put into slot")
    );
    let after = members(&s);
    assert_eq!(after.len(), before + 1, "the two squares are one unit");
    s.undo();
    assert_eq!(members(&s).len(), before, "undo removes the copies");
    assert!(s.activate_page(&page));
}

#[test]
fn ac2_two_picked_members_are_wrapped_in_one_group_that_carries_came_from() {
    let (mut s, _) = laser();
    let library = library();
    let came = record();
    let drop = s
        .put_into_slot(
            &source(&library, &came),
            slot_id(&s, 0),
            FitMode::Fit,
            false,
        )
        .unwrap();
    assert!(drop.refused.is_none());
    let d = unpack(3, &s.pack("accept").unwrap()).unwrap();
    let group = *members(&s).last().unwrap();
    assert_eq!(
        d.children(Some(group)).len(),
        2,
        "the whole page is one wrapped unit"
    );
    let cf = d.came_from(group).expect("Came from on the wrapping group");
    assert_eq!(cf.page_id, "first", "the source page is the item");
    assert_eq!(cf.document, "Box job");
}

#[test]
fn ac7_a_filled_slot_refuses_with_nothing_written_then_replace_is_one_step() {
    let (mut s, _) = laser();
    let library = library();
    let came = record();
    let first = s
        .put_into_slot(
            &source(&library, &came),
            slot_id(&s, 1),
            FitMode::Fit,
            false,
        )
        .unwrap();
    assert!(first.refused.is_none());
    let undo_before = s.undo_len();
    let members_before = members(&s).len();
    let refused = s
        .put_into_slot(
            &source(&library, &came),
            slot_id(&s, 1),
            FitMode::Fit,
            false,
        )
        .unwrap();
    let text = refused.refused.expect("a filled slot refuses");
    assert!(
        text.ends_with("is filled."),
        "the notice says the slot is filled: {text}"
    );
    assert_eq!(s.undo_len(), undo_before, "nothing is written");
    assert_eq!(members(&s).len(), members_before);

    let replaced = s
        .put_into_slot(&source(&library, &came), slot_id(&s, 1), FitMode::Fit, true)
        .unwrap();
    assert!(replaced.refused.is_none());
    assert_eq!(s.undo_len(), undo_before + 1, "Replace is one undo step");
    assert_eq!(
        s.history_target_name(curvyo_document_core::Direction::Undo)
            .as_deref(),
        Some("Replace slot content")
    );
    assert_eq!(
        members(&s).len(),
        members_before,
        "the old content is removed, the new one in"
    );
}

#[test]
fn ac8_fill_skips_filled_slots_counts_them_and_is_one_step_named_fill_slots() {
    let (mut s, _) = laser();
    let library = library();
    let came = record();
    let holder = s.slot_targets_view(None, false).holders[0].id.clone();
    let holder = s.fixture_part_named(&holder).unwrap();
    s.put_into_slot(
        &source(&library, &came),
        slot_id(&s, 0),
        FitMode::Fit,
        false,
    )
    .unwrap();
    let undo_before = s.undo_len();
    let members_before = members(&s).len();
    let drop = s
        .fill_empty_slots(&source(&library, &came), holder, FitMode::Fit)
        .unwrap();
    let notice = drop.notice.expect("the fill says what it did");
    assert!(
        notice.contains("already filled"),
        "counts the filled slots: {notice}"
    );
    assert!(notice.contains('1'), "one slot was filled before: {notice}");
    assert_eq!(s.undo_len(), undo_before + 1, "one undo step");
    assert_eq!(
        s.history_target_name(curvyo_document_core::Direction::Undo)
            .as_deref(),
        Some("Fill slots")
    );
    assert_eq!(
        members(&s).len(),
        members_before + 2,
        "one copy into each of the two empty slots"
    );
    let rows = holder_rows(&s);
    assert!(
        rows.iter().all(|r| r["filled"] == true),
        "all three slots are filled: {rows:?}"
    );
}

#[test]
fn ac8_a_holder_with_no_empty_slot_writes_nothing_and_says_so() {
    let (mut s, _) = laser();
    let library = library();
    let came = record();
    let holder_id = s.slot_targets_view(None, false).holders[0].id.clone();
    let holder = s.fixture_part_named(&holder_id).unwrap();
    s.fill_empty_slots(&source(&library, &came), holder, FitMode::Fit)
        .unwrap();
    let undo_before = s.undo_len();
    let drop = s
        .fill_empty_slots(&source(&library, &came), holder, FitMode::Fit)
        .unwrap();
    let said = drop
        .notice
        .clone()
        .or(drop.refused.clone())
        .unwrap_or_default();
    assert!(
        said.starts_with("No empty slot in "),
        "notice {:?} refused {:?}",
        drop.notice,
        drop.refused
    );
    assert_eq!(s.undo_len(), undo_before, "nothing is written");
}

#[test]
fn ac10_the_list_shows_the_outcome_per_slot_and_the_filled_rows() {
    let (mut s, _) = laser();
    let library = library();
    let came = record();
    s.put_into_slot(
        &source(&library, &came),
        slot_id(&s, 1),
        FitMode::Fit,
        false,
    )
    .unwrap();
    let view = s.slot_targets_view(Some(unit_60x40()), false);
    let rows = &view.holders[0].slots;
    assert_eq!(
        rows[0].outcome.as_deref().map(plain).as_deref(),
        Some("50 %"),
        "30 x 30 takes 60 x 40 at 50 %"
    );
    assert!(!rows[0].filled);
    assert!(rows[1].filled, "the slot with the copy is filled");
}

#[test]
fn ac10_the_list_is_built_with_the_switch_off_and_the_unit_known_or_not() {
    let (s, _) = laser();
    let unknown = s.slot_targets_view(None, false);
    assert_eq!(unknown.holders.len(), 1, "the list works without a unit");
    assert_eq!(unknown.holders[0].slots.len(), 3);
}

#[test]
fn ac3_criterion_3_default_is_a_copy_so_the_source_stays_independent() {
    let (mut s, _) = laser();
    let library = library();
    let came = record();
    let before = objects_of(&library);
    s.put_into_slot(
        &source(&library, &came),
        slot_id(&s, 2),
        FitMode::Fit,
        false,
    )
    .unwrap();
    assert_eq!(
        objects_of(&library),
        before,
        "the library source is not changed"
    );
}

/// The id of the `index`-th target slot of the holder, from the Into slot list.
fn slot_id(s: &Session, index: usize) -> NodeId {
    let view = s.slot_targets_view(None, false);
    let row = &view.holders[0].slots[index];
    s.fixture_part_named(&row.id).unwrap()
}

/// Keeps `slot_point` used for the centres of the other slots and `Point` in scope.
#[allow(dead_code)]
fn centres(s: &Session) -> [Point; 3] {
    [
        slot_point(s, 50.0, 100.0),
        slot_point(s, 150.0, 100.0),
        slot_point(s, 250.0, 100.0),
    ]
}

/// The number of objects in the library's saved document, a read of its content.
fn objects_of(s: &Session) -> usize {
    unpack(4, &s.pack("accept").unwrap())
        .unwrap()
        .object_ids()
        .len()
}
