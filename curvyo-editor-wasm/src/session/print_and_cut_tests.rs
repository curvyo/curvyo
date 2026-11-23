//! Session tests of the Print and cut group (`specs/0121-print-and-cut` criteria 7 to 19).
#![allow(clippy::unwrap_used)]

use curvyo_document_core::{FIRST_PAGE_ID, NodeId, Point};
use curvyo_laser_core::print_and_cut::machine_mm;

use crate::session::Session;

/// A Drawing page with marks placed on a laser page; the session and the placement.
fn sheet(peer: u64) -> (Session, NodeId) {
    let mut s = Session::new(peer);
    s.add_registration_marks();
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    assert!(s.place_page_on_active(FIRST_PAGE_ID));
    let placement = s.selected_placement().unwrap();
    (s, placement)
}

fn view(s: &Session) -> serde_json::Value {
    serde_json::from_str(&s.print_and_cut_view().unwrap()).unwrap()
}

/// Measures every mark where `map` (sheet page-local to laser page-local) puts it.
fn measure_all(s: &mut Session, map: curvyo_document_core::Affine) {
    let (frame, _) = s.print_and_cut_frame().unwrap();
    let source = s.document.pages()[0].clone();
    for mark in s.document.marks_objects_on(&source).remove(0).marks {
        let local = Point::new(
            mark.centre.x - source.origin.x,
            mark.centre.y - source.origin.y,
        );
        let m = machine_mm(frame, map.apply_point(local));
        assert!(s.print_and_cut_measure(mark.number, m.x, m.y));
    }
}

#[test]
fn criterion_7_the_group_lists_the_marks_and_is_absent_without_them() {
    let mut s = Session::new(121_101);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    assert!(s.place_page_on_active(FIRST_PAGE_ID));
    assert!(s.print_and_cut_view().is_none(), "absent, not disabled");
    let (s, _) = sheet(121_102);
    let v = view(&s);
    assert_eq!(v["rows"].as_array().unwrap().len(), 4);
    assert_eq!(v["state"], "Measure at least 2 marks");
    assert_eq!(v["summary"], "Not measured");
    assert_eq!(v["not_cut"], "Registration marks (not cut)", "criterion 5");
    assert_eq!(v["apply"]["reason"], "Measure at least 2 marks");
}

#[test]
fn criteria_9_to_16_measure_fit_apply_and_reset_each_one_step() {
    let (mut s, placement) = sheet(121_103);
    let before = s.document.placed_page(placement).unwrap().matrix;
    let fit = curvyo_document_core::Affine::translation(curvyo_document_core::Vec2::new(30.0, 5.0))
        .compose(curvyo_document_core::Affine::rotation_about(
            Point::new(0.0, 0.0),
            curvyo_document_core::Angle::from_radians(0.02),
        ));
    let steps = s.undo_len();
    measure_all(&mut s, fit);
    assert_eq!(s.undo_len(), steps + 4, "one step per row");
    let v = view(&s);
    assert!(
        v["fit"]["error"]
            .as_str()
            .unwrap()
            .starts_with("Largest 0.00\u{a0}mm"),
        "{v:#}"
    );
    assert_eq!(v["apply"]["label"], "Apply");
    assert!(v["apply"]["reason"].is_null());
    assert_eq!(s.print_and_cut_apply(), "");
    assert!(
        s.document
            .placed_page(placement)
            .unwrap()
            .matrix
            .approx_eq(fit, 1e-3)
    );
    let v = view(&s);
    assert_eq!(v["apply"]["reason"], "The sheet is on the fit");
    assert_eq!(v["placement_line"], "On the fit");
    assert_eq!(v["reset"]["enabled"], true);
    assert!(s.print_and_cut_reset());
    assert_eq!(s.document.placed_page(placement).unwrap().matrix, before);
    assert!(s.print_and_cut_clear(1));
    assert!(view(&s)["rows"][0]["x"].is_null());
    assert!(s.print_and_cut_tolerance(0.4));
    assert_eq!(view(&s)["tolerance"], 0.4);
}

#[test]
fn criterion_8_take_head_position_rounds_to_a_micrometre() {
    let (mut s, placement) = sheet(121_104);
    assert!(s.print_and_cut_take_head_position(2, 12.345_4, 7.000_6));
    let Some(curvyo_document_core::PrintAndCutRegister::Measures(m)) =
        s.document.print_and_cut(placement)
    else {
        panic!("measures");
    };
    assert_eq!(m.marks[&2], [12_345, 7_001]);
}

/// UX review: the crosses carry their numbers, the rings theirs, the fitted frame "Fit".
#[test]
fn the_canvas_tags_name_marks_rings_and_the_fit() {
    let (mut s, _) = sheet(121_105);
    s.resize_viewport(1200.0, 800.0);
    let kinds = |s: &Session| -> Vec<(&'static str, String)> {
        s.print_and_cut_labels()
            .into_iter()
            .map(|l| (l.kind, l.text))
            .collect()
    };
    let marks: Vec<_> = kinds(&s)
        .into_iter()
        .filter(|(k, _)| *k == "mark")
        .collect();
    assert_eq!(marks.len(), 4);
    assert!(marks.iter().any(|(_, t)| t == "3"));
    let fit = curvyo_document_core::Affine::translation(curvyo_document_core::Vec2::new(30.0, 5.0));
    measure_all(&mut s, fit);
    let all = kinds(&s);
    assert_eq!(all.iter().filter(|(k, _)| *k == "ring").count(), 4);
    assert!(all.contains(&("fit", "Fit".to_owned())));
    assert_eq!(s.print_and_cut_apply(), "");
    assert!(
        !kinds(&s).iter().any(|(k, _)| *k == "fit"),
        "gone on the fit"
    );
}
