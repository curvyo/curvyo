//! A library item put into a slot at session level (`specs/0154-library-into-slots` criteria 1 to
//! 10 and 17): the fit, one undo step each, the computed "filled", the refusal and the Replace,
//! the fill, the drag cue and the Into slot list.
#![allow(clippy::float_cmp)]

use curvyo_document_core::{
    Affine, Angle, CameFrom, DocumentSize, FixtureTurn, NodeId, Point, UnixMillis,
};
use curvyo_laser_core::FitMode;

use super::super::Session;
use super::super::align_test_support::draw_rect;
use super::add_fixture;
use super::tests::{chip, corner, laser, last_label, mm, place};
use super::{SlotSource, UnitBox};

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

/// A read-only library source whose first page is 60 x 40 mm with two rectangles on it.
fn source() -> Session {
    let mut s = Session::new(5);
    s.resize_viewport(1200.0, 800.0);
    s.document
        .resize_page(
            "first",
            DocumentSize::from_mm(60.0, 40.0),
            &curvyo_document_core::PageContents::default(),
        )
        .unwrap();
    let o = s.active_page().origin;
    draw_rect(&mut s, (o.x + 5.0, o.y + 5.0), (o.x + 25.0, o.y + 35.0));
    draw_rect(&mut s, (o.x + 35.0, o.y + 5.0), (o.x + 55.0, o.y + 35.0));
    let bytes = s.pack("t").unwrap();
    Session::open(6, &bytes).unwrap()
}

/// A laser page with one holder (320 x 240 at 10, 10) and slots: 30 x 30, 60 x 40, circle 25.
fn laser_with_slots() -> (Session, NodeId, [NodeId; 3]) {
    let mut s = laser();
    let holder = add_fixture(&mut s, "holder").unwrap();
    chip(&mut s, holder, 's', &["320", "240"]);
    place(&mut s, holder, 10.0, 10.0);
    let mut ids = Vec::new();
    for (cx, w, h) in [(50.0, 30.0, 30.0), (150.0, 60.0, 40.0), (250.0, 25.0, 25.0)] {
        let slot = s.add_slot(holder).unwrap();
        s.set_slot_length(slot, "cx_mm", mm(cx));
        s.set_slot_length(slot, "cy_mm", mm(100.0));
        s.set_slot_length(slot, "w_mm", mm(w));
        s.set_slot_length(slot, "h_mm", mm(h));
        ids.push(slot);
    }
    s.set_slot_shape(ids[2], "circle");
    s.clear_fixture_selection();
    (s, holder, [ids[0], ids[1], ids[2]])
}

fn page_source(source: &Session) -> SlotSource<'_> {
    SlotSource {
        source,
        roots: &[],
        page: Some("first"),
        came_from: Box::leak(Box::new(record())),
    }
}

/// The slot's centre in document space.
fn slot_centre(s: &Session, holder: NodeId, cx: f64) -> Point {
    let origin = s.active_page().origin;
    let frame = corner(s, holder);
    Point::new(origin.x + frame.x + cx, origin.y + frame.y + 100.0)
}

/// The joint outline box of the selection.
fn selection_box(s: &Session) -> (Point, Point) {
    let ids: Vec<NodeId> = s
        .selection
        .ids()
        .iter()
        .flat_map(|&id| {
            if s.document.is_group(id) {
                s.document.leaves_of(id)
            } else {
                vec![id]
            }
        })
        .collect();
    let objects = s.objects();
    let boxes: Vec<_> = objects
        .iter()
        .filter(|o| ids.contains(&o.id()))
        .map(curvyo_ui_core::object_outline_bounds)
        .collect();
    super::into_slot::joint(&boxes).unwrap()
}

fn assert_box(b: (Point, Point), centre: Point, w: f64, h: f64) {
    let (min, max) = b;
    let c = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
    assert!(
        c.vector_to(centre).length() < 1e-6,
        "centre {c:?} wanted {centre:?}"
    );
    assert!(
        ((max.x - min.x) - w).abs() < 1e-6,
        "width {}",
        max.x - min.x
    );
    assert!(
        ((max.y - min.y) - h).abs() < 1e-6,
        "height {}",
        max.y - min.y
    );
}

#[test]
fn a_page_into_a_smaller_slot_is_scaled_centred_wrapped_and_one_step() {
    let (mut s, holder, slots) = laser_with_slots();
    let source = source();
    let steps = s.document.own_steps().len();
    let done = s
        .put_into_slot(&page_source(&source), slots[0], FitMode::Fit, false)
        .unwrap();
    assert_eq!(done.refused, None);
    assert_eq!(
        done.notice.as_deref(),
        Some("Fitted into Slot 1 at 50\u{a0}%.")
    );
    assert_eq!(s.document.own_steps().len(), steps + 1, "one step");
    assert_eq!(last_label(&s), "put_into_slot");
    let selected = s.selection.ids().to_vec();
    assert_eq!(selected.len(), 1);
    assert!(s.document.is_group(selected[0]), "two members are wrapped");
    assert_eq!(
        s.document.came_from(selected[0]).unwrap().item.as_deref(),
        Some("Page 1")
    );
    // The page's frame (60 x 40 with margins) is the unit: the rectangles' box is 25 x 15.
    assert_box(selection_box(&s), slot_centre(&s, holder, 50.0), 25.0, 15.0);
    s.undo();
    assert!(s.selection.ids().is_empty(), "undo restores the selection");
    assert!(s.member_slots().is_empty(), "undo removes the copy");
}

#[test]
fn a_filled_slot_refuses_and_replace_swaps_its_content_in_one_step() {
    let (mut s, _, slots) = laser_with_slots();
    let source = source();
    s.put_into_slot(&page_source(&source), slots[1], FitMode::Fit, false)
        .unwrap();
    let first = s.selection.ids()[0];
    let steps = s.document.own_steps().len();
    let refused = s
        .put_into_slot(&page_source(&source), slots[1], FitMode::Fit, false)
        .unwrap();
    assert_eq!(refused.refused.as_deref(), Some("Slot 2 is filled."));
    assert_eq!(s.document.own_steps().len(), steps, "nothing is written");
    s.put_into_slot(&page_source(&source), slots[1], FitMode::Fit, true)
        .unwrap();
    assert_eq!(last_label(&s), "replace_slot_content");
    assert_eq!(s.document.own_steps().len(), steps + 1);
    assert!(!s.document.object_ids().contains(&first) && !s.document.is_group(first));
    assert_eq!(s.member_slots().len(), 1);
}

#[test]
fn fill_puts_one_copy_into_every_empty_slot_and_counts_the_filled_ones() {
    let (mut s, holder, slots) = laser_with_slots();
    let source = source();
    s.put_into_slot(&page_source(&source), slots[0], FitMode::Fit, false)
        .unwrap();
    let before = s.document.own_steps().len();
    let done = s
        .fill_empty_slots(&page_source(&source), holder, FitMode::Fit)
        .unwrap();
    assert_eq!(s.document.own_steps().len(), before + 1, "one undo step");
    assert_eq!(last_label(&s), "fill_slots");
    assert_eq!(s.selection.ids().len(), 2);
    assert_eq!(
        done.notice.as_deref(),
        Some("Filled 2 slots at 42 to 100\u{a0}%; 1 was already filled.")
    );
    let again = s
        .fill_empty_slots(&page_source(&source), holder, FitMode::Fit)
        .unwrap();
    assert_eq!(again.refused.as_deref(), Some("No empty slot in Holder 1."));
    let view = s.slot_targets_view(None, false);
    assert_eq!(view.holders[0].filled, 3);
    assert!(view.holders[0].slots.iter().all(|r| r.filled));
}

#[test]
fn a_page_matching_a_slot_turned_is_turned_and_a_circle_fits_its_diameter() {
    let (mut s, holder, slots) = laser_with_slots();
    s.set_slot_length(slots[1], "w_mm", mm(40.0));
    s.set_slot_length(slots[1], "h_mm", mm(60.0));
    s.clear_fixture_selection();
    let source = source();
    let done = s
        .put_into_slot(&page_source(&source), slots[1], FitMode::Fit, false)
        .unwrap();
    assert_eq!(done.notice.as_deref(), Some("Turned to fit Slot 2."));
    assert_box(
        selection_box(&s),
        slot_centre(&s, holder, 150.0),
        30.0,
        50.0,
    );
    s.put_into_slot(&page_source(&source), slots[2], FitMode::Fit, false)
        .unwrap();
    // 60 x 40 into the 25 mm square of the circle: 5/12, the rectangles' box 50 x 30 -> 20.83 x 12.5.
    assert_box(
        selection_box(&s),
        slot_centre(&s, holder, 250.0),
        50.0 * 25.0 / 60.0,
        30.0 * 25.0 / 60.0,
    );
}

#[test]
fn alt_keeps_the_real_size_and_says_when_it_is_larger() {
    let (mut s, holder, slots) = laser_with_slots();
    let source = source();
    let done = s
        .put_into_slot(&page_source(&source), slots[0], FitMode::RealSize, false)
        .unwrap();
    assert_eq!(
        done.notice.as_deref(),
        Some("Real size: larger than Slot 1.")
    );
    assert_box(selection_box(&s), slot_centre(&s, holder, 50.0), 50.0, 30.0);
}

#[test]
fn a_holder_turned_on_the_bed_turns_the_unit_with_it() {
    let (mut s, holder, slots) = laser_with_slots();
    let matrix = s.active_fixtures().unwrap().get(holder).unwrap().matrix;
    let turn = Affine::rotation_about(Point::new(0.0, 0.0), Angle::from_radians(0.5));
    s.document
        .set_fixture_matrix(holder, matrix.compose(turn), FixtureTurn::Rotate)
        .unwrap();
    let source = source();
    s.put_into_slot(&page_source(&source), slots[1], FitMode::Fit, false)
        .unwrap();
    let target = s
        .slot_targets()
        .into_iter()
        .find(|t| t.id == slots[1])
        .unwrap();
    let centre = target.board.apply_point(target.slot.centre);
    let leaf = s.document.leaves_of(s.selection.ids()[0])[0];
    let object = s.objects().iter().find(|o| o.id() == leaf).unwrap().clone();
    let along = match &object {
        curvyo_document_core::ObjectSnapshot::Primitive(p) => p.rotation.as_radians(),
        _ => panic!("a rectangle"),
    };
    assert!(
        (along - 0.5).abs() < 1e-6,
        "turned with the holder: {along}"
    );
    assert_eq!(
        s.member_slots().len(),
        1,
        "its centre lies in the slot {centre:?}"
    );
}

#[test]
fn the_drag_cue_names_the_slot_with_the_scale_and_shift_fills() {
    let (mut s, holder, slots) = laser_with_slots();
    let unit = UnitBox::between(Point::new(0.0, 0.0), Point::new(60.0, 40.0));
    s.begin_slot_drop(Some(unit));
    let cue = s.slot_drop_cue(slot_centre(&s, holder, 50.0), false, false);
    assert_eq!(cue.kind, "slot");
    assert_eq!(cue.slot, Some(slots[0].full_id()));
    assert_eq!(cue.badge.as_deref(), Some("Into Slot 1 · 50\u{a0}%"));
    let alt = s.slot_drop_cue(slot_centre(&s, holder, 50.0), true, false);
    assert_eq!(
        alt.badge.as_deref(),
        Some("Into Slot 1 · real size, too large")
    );
    let fill = s.slot_drop_cue(slot_centre(&s, holder, 150.0), false, true);
    assert_eq!(fill.kind, "fill");
    assert_eq!(
        fill.badge.as_deref(),
        Some("Fill 3 of 3 slots · 42 to 100\u{a0}%")
    );
    let off = s.slot_drop_cue(Point::new(-500.0, -500.0), false, false);
    assert_eq!(off.kind, "none");
    s.end_slot_drop();
    s.set_snapping_fixtures(false);
    s.begin_slot_drop(Some(unit));
    let cue = s.slot_drop_cue(slot_centre(&s, holder, 50.0), false, false);
    assert_eq!(cue.badge.as_deref(), Some("Fixture snapping is off."));
}

#[test]
fn the_list_shows_size_outcome_and_the_selected_slot() {
    let (mut s, _, slots) = laser_with_slots();
    let unit = UnitBox::between(Point::new(0.0, 0.0), Point::new(60.0, 40.0));
    let view = s.slot_targets_view(Some(unit), false);
    let rows = &view.holders[0].slots;
    assert_eq!(rows[0].size, "30 × 30 mm");
    assert_eq!(rows[0].outcome.as_deref(), Some("50\u{a0}%"));
    assert_eq!(rows[1].outcome, None, "a plain centre");
    assert_eq!(rows[2].size, "⌀ 25 mm");
    assert_eq!(view.selected, None);
    s.select_slot(slots[1]);
    assert_eq!(
        s.slot_targets_view(Some(unit), false).selected,
        Some(slots[1].full_id())
    );
}

#[test]
fn a_slot_of_another_laser_page_names_it_and_the_release_activates_and_puts_in_one_step() {
    let (mut s, holder, slots) = laser_with_slots();
    let first = s.active_page().id;
    let over = slot_centre(&s, holder, 50.0);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    assert_ne!(s.active_page().id, first);
    s.begin_slot_drop(Some(UnitBox::between(
        Point::new(0.0, 0.0),
        Point::new(60.0, 40.0),
    )));
    let cue = s.slot_drop_cue(over, false, false);
    assert_eq!(cue.kind, "slot");
    assert_eq!(cue.badge.as_deref(), Some("Into Slot 1 · 50\u{a0}%"));
    assert_eq!(cue.page.as_deref(), Some(first.as_str()));
    s.end_slot_drop();
    // The release (the frontend): activate that page, then the put.
    let steps = s.document.own_steps().len();
    assert!(s.activate_page(&first));
    let source = source();
    s.put_into_slot(&page_source(&source), slots[0], FitMode::Fit, false)
        .unwrap();
    assert_eq!(s.document.own_steps().len(), steps + 1, "one undo step");
    assert_eq!(last_label(&s), "put_into_slot");
}

#[test]
fn the_ghost_draws_the_content_outlines_and_the_status_says_the_scale() {
    let (mut s, holder, _) = laser_with_slots();
    let source = source();
    s.begin_slot_drop(None);
    s.set_slot_drop_source(&page_source(&source));
    let cue = s.slot_drop_cue(slot_centre(&s, holder, 50.0), false, false);
    assert_eq!(
        cue.status.as_deref(),
        Some("Into Slot 1 of Holder 1, scaled to 50 percent.")
    );
    let with_outlines = s.slot_drop_overlay().triangles.len();
    s.begin_slot_drop(Some(UnitBox::between(
        Point::new(0.0, 0.0),
        Point::new(60.0, 40.0),
    )));
    s.slot_drop_cue(slot_centre(&s, holder, 50.0), false, false);
    assert!(
        with_outlines > s.slot_drop_overlay().triangles.len(),
        "outlines add lines"
    );
}

#[test]
fn a_filled_slot_hides_its_name_label() {
    let (mut s, _, slots) = laser_with_slots();
    s.set_device_pixel_ratio(8.0);
    let names = |s: &Session| -> Vec<String> {
        s.fixture_labels()
            .into_iter()
            .filter(|l| l.kind == "slot")
            .map(|l| l.text)
            .collect()
    };
    assert!(names(&s).contains(&"Slot 1".to_owned()));
    let source = source();
    s.put_into_slot(&page_source(&source), slots[0], FitMode::Fit, false)
        .unwrap();
    assert!(!names(&s).contains(&"Slot 1".to_owned()));
    assert!(names(&s).contains(&"Slot 2".to_owned()));
}

#[test]
fn the_holder_line_counts_the_filled_slots() {
    let (mut s, _, slots) = laser_with_slots();
    let source = source();
    s.put_into_slot(&page_source(&source), slots[0], FitMode::Fit, false)
        .unwrap();
    let view = s.fixtures_view().unwrap();
    let line = &view.sets[0].fixtures[0].line2;
    assert!(line.starts_with("Holder · 3 slots · 1 filled"), "{line}");
}

#[test]
fn a_guide_in_a_slot_does_not_fill_it() {
    let (mut s, holder, slots) = laser_with_slots();
    let c = slot_centre(&s, holder, 50.0);
    let id = s
        .document
        .create_rect(curvyo_document_core::RectBounds::from_corners(
            Point::new(c.x - 5.0, c.y - 5.0),
            Point::new(c.x + 5.0, c.y + 5.0),
        ));
    assert!(s.slot_targets_view(None, false).holders[0].slots[0].filled);
    s.document.set_guide(&[id], true).unwrap();
    assert!(
        !s.slot_targets_view(None, false).holders[0].slots[0].filled,
        "Terms: guides never fill a slot"
    );
    let done = s
        .put_into_slot(&page_source(&source()), slots[0], FitMode::Fit, false)
        .unwrap();
    assert_eq!(done.refused, None);
}

#[test]
fn criterion_11_the_copy_is_a_not_assigned_work_item_outside_the_fixture_jobs() {
    let (mut s, _, slots) = laser_with_slots();
    let jobs_before = s.fixture_job_triangles(&s.active_fixtures().unwrap()).len();
    let source = source();
    s.put_into_slot(&page_source(&source), slots[0], FitMode::Fit, false)
        .unwrap();
    let group = s.selection.ids()[0];
    let leaves = s.document.leaves_of(group);
    let view = s.operations_view().expect("a laser page");
    for leaf in &leaves {
        assert!(
            view.items.not_assigned.contains(leaf),
            "Not assigned (Frage 155 A)"
        );
    }
    assert_eq!(
        s.fixture_job_triangles(&s.active_fixtures().unwrap()).len(),
        jobs_before,
        "fixture jobs take fixtures only"
    );
    assert!(
        s.document.parent_node(group) != Some(slots[0]),
        "not a child of the slot"
    );
}

#[test]
fn criterion_12_the_fixture_set_does_not_take_the_content() {
    let (mut s, _, slots) = laser_with_slots();
    let shape = |s: &Session| {
        let view = s.fixtures_view().unwrap();
        view.sets
            .iter()
            .map(|set| {
                set.fixtures
                    .iter()
                    .map(|f| {
                        (
                            f.id.clone(),
                            f.slots.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    let before = shape(&s);
    let source = source();
    s.put_into_slot(&page_source(&source), slots[0], FitMode::Fit, false)
        .unwrap();
    assert_eq!(
        shape(&s),
        before,
        "the set holds the same fixtures and slots"
    );
}
