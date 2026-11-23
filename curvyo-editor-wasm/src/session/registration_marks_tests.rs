//! Session tests of the marks object (`specs/0121-print-and-cut` criteria 1 to 4).
#![allow(clippy::unwrap_used)]

use curvyo_document_core::{FIRST_PAGE_ID, NodeId, ObjectSnapshot};

use super::MARKS_BELOW_TWO_NOTICE;
use crate::session::align_test_support::draw_rect;
use crate::session::{Session, Tool};

fn marks_view(s: &Session) -> serde_json::Value {
    serde_json::from_str(&s.registration_marks_view().unwrap()).unwrap()
}

fn marks(s: &Session) -> Vec<NodeId> {
    s.document
        .read_objects()
        .objects
        .iter()
        .filter(|o| matches!(o, ObjectSnapshot::RegistrationMark(_)))
        .map(ObjectSnapshot::id)
        .collect()
}

#[test]
fn criterion_1_add_registration_marks_adds_four_selected_marks_in_one_step() {
    let mut s = Session::new(121_001);
    assert!(s.registration_marks_offered());
    let steps = s.undo_len();
    assert!(s.add_registration_marks());
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(s.tool(), Tool::Select);
    assert_eq!(s.selection.ids().len(), 4);
    let v = marks_view(&s);
    assert_eq!(v["count"], 4);
    assert!(v["add"].is_null());
    assert_eq!(v["diameter"], 6.0);
    let _ = s.undo();
    assert_eq!(marks(&s), Vec::<NodeId>::new());
}

#[test]
fn criterion_1_a_small_design_keeps_the_marks_off_its_box() {
    let mut s = Session::new(121_002);
    draw_rect(&mut s, (80.0, 100.0), (130.0, 180.0));
    assert!(s.add_registration_marks());
    let centres: Vec<_> = s
        .document
        .marks_objects_on(&s.active_page())
        .remove(0)
        .marks
        .iter()
        .map(|m| (m.centre.x, m.centre.y))
        .collect();
    assert_eq!(centres[0], (10.0, 10.0), "the page corner");
    for (x, y) in centres {
        assert!(!(70.0..=140.0).contains(&x) || !(90.0..=190.0).contains(&y));
    }
}

#[test]
fn criterion_1_the_button_is_absent_on_a_laser_page() {
    let mut s = Session::new(121_003);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    assert!(!s.registration_marks_offered());
    assert!(!s.add_registration_marks());
}

#[test]
fn criteria_2_and_3_size_add_mark_and_the_two_mark_refusal() {
    let mut s = Session::new(121_004);
    s.add_registration_marks();
    assert!(s.set_registration_mark_size(10.0, 0.5));
    assert_eq!(marks_view(&s)["diameter"], 10.0);
    assert!(!s.set_registration_mark_size(25.0, 0.5), "out of range");
    let ids = marks(&s);
    s.selection.set(&ids[..2]);
    s.delete_selected();
    assert_eq!(marks(&s).len(), 2);
    s.selection.set(&[marks(&s)[0]]);
    s.delete_selected();
    assert_eq!(marks(&s).len(), 2, "refused");
    assert_eq!(s.history.notice.as_deref(), Some(MARKS_BELOW_TWO_NOTICE));
    s.selection.set(&marks(&s));
    assert_eq!(marks_view(&s)["add"], 1);
    assert!(s.add_registration_mark());
    assert_eq!(marks(&s).len(), 3);
    s.selection.set(&marks(&s));
    s.delete_selected();
    assert_eq!(
        marks(&s),
        Vec::<NodeId>::new(),
        "the object deletes as a whole"
    );
}

/// Criterion 4: a Cut over everything burns nothing of the marks, on the sheet and placed.
#[test]
fn criterion_4_no_mark_contributes_a_burn_move() {
    let mut s = Session::new(121_005);
    draw_rect(&mut s, (60.0, 60.0), (100.0, 100.0));
    s.add_registration_marks();
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    assert!(s.place_page_on_active(FIRST_PAGE_ID));
    let placement = s.selected_placement().unwrap();
    s.selection.set(&[placement]);
    let id = s.press_operation_kind("cut", true).unwrap();
    s.selection.set(&[]);
    s.edit_operation_setting(id, "main", "power", 60.0).unwrap();
    s.edit_operation_setting(id, "main", "speed", 600.0)
        .unwrap();
    let burns = |s: &mut Session| {
        let file = s.export_job("").unwrap();
        String::from_utf8(file.bytes)
            .unwrap()
            .lines()
            .filter(|l| l.starts_with("G1 "))
            .count()
    };
    let with_marks = burns(&mut s);
    assert!(with_marks > 0, "the square burns");
    let group = s
        .document
        .marks_objects_on(&s.document.pages()[0])
        .remove(0)
        .id;
    s.document.delete_objects(&[group]).unwrap();
    assert_eq!(burns(&mut s), with_marks, "the marks added no burn move");
}

/// UX notes P6: the marks object and a mark inside it only move (no resize or rotate handles).
#[test]
fn p6_the_marks_object_only_moves() {
    let mut s = Session::new(121_006);
    s.add_registration_marks();
    assert!(s.transform_handle_tolerances().move_only);
    let first = marks(&s)[0];
    s.selection.set(&[first]);
    assert!(s.transform_handle_tolerances().move_only);
    draw_rect(&mut s, (60.0, 60.0), (100.0, 100.0));
    assert!(
        !s.transform_handle_tolerances().move_only,
        "a rectangle resizes"
    );
}

/// UX notes §3.2: the Layers tab shows "Registration marks" with its own glyph and "Mark n" rows.
#[test]
fn the_layers_tab_names_the_marks_object_and_each_mark() {
    let mut s = Session::new(121_007);
    s.add_registration_marks();
    let group = s.document.marks_objects_on(&s.active_page()).remove(0).id;
    assert!(s.set_layer_expanded(&serde_json::to_string(&group).unwrap(), true));
    let rows: serde_json::Value = serde_json::from_str(&s.layers_view_json(0, 50)).unwrap();
    let text = rows.to_string();
    assert!(text.contains("\"registration-marks\""), "{text}");
    for n in 1..=4 {
        assert!(text.contains(&format!("\"Mark {n}\"")), "{text}");
    }
    assert!(text.contains("\"Registration marks\""));
}

/// Criterion 4 (PNG): the page picture the thumbnail and the PNG of a page are drawn from holds
/// black ink at every mark (the same drawing path as the canvas, `build_artwork`).
#[test]
fn criterion_4_the_page_picture_shows_every_mark() {
    let mut s = Session::new(121_008);
    s.resize_viewport(1200.0, 800.0);
    let empty = s.thumbnail_frame(400.0, 560.0).0.triangles.len();
    s.add_registration_marks();
    let (list, _) = s.thumbnail_frame(400.0, 560.0);
    assert!(list.triangles.len() > empty);
    let centres = s
        .document
        .marks_objects_on(&s.active_page())
        .remove(0)
        .marks;
    for mark in centres {
        let near = list.triangles.iter().any(|v| {
            v.color.r == 0
                && v.color.a == 255
                && (v.position.x - mark.centre.x).abs() < 4.5
                && (v.position.y - mark.centre.y).abs() < 4.5
        });
        assert!(near, "ink at mark {}", mark.number);
    }
}

/// UX review: the Style tab names a marks object "Registration marks", a mark "Mark 2".
#[test]
fn the_style_tab_names_the_marks_object() {
    let mut s = Session::new(121_009);
    s.add_registration_marks();
    assert_eq!(s.style_scope().subject, "Registration marks");
    assert_eq!(s.style_panel_view().subject, "Registration marks");
    let second = marks(&s)[1];
    s.selection.set(&[second]);
    assert_eq!(s.style_scope().subject, "Registration marks");
}

/// UX notes §3.2 (UX review): the Select bar on a marks object keeps X, Y and the reference
/// points only: no scale switches, Stroke to path, Linked clone, Flip or Guide.
#[test]
fn the_select_bar_of_a_marks_object_moves_only() {
    let mut s = Session::new(121_010);
    s.add_registration_marks();
    assert!(!s.select_bar_state().scale_switches_shown);
    assert!(!s.select_bar_state().object_to_path);
    assert!(!s.stroke_to_path_shown());
    assert!(!s.linked_clone_shown());
    assert!(s.select_bar_hides_grid_commands(), "Flip and Guide hidden");
    draw_rect(&mut s, (60.0, 60.0), (100.0, 100.0));
    assert!(s.select_bar_state().scale_switches_shown);
    assert!(!s.select_bar_hides_grid_commands());
}
