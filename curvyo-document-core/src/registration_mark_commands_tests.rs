//! Tests of the marks object (`specs/0121-print-and-cut` criteria 1 to 3, 6).
//!
//! Test-only: declared under `#[cfg(test)]` in `registration_mark_commands.rs`.

use super::*;
use crate::primitive_model::{ObjectSnapshot, RectBounds};
use crate::units::Vec2;

const A4: (Point, Point) = (Point::new(0.0, 0.0), Point::new(210.0, 297.0));

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn undo(d: &Document) {
    let last = d.own_steps().last().unwrap().step;
    d.undo_step(last).unwrap();
}

fn marks(d: &Document) -> MarksObject {
    d.marks_objects_on(&d.pages()[0]).remove(0)
}

#[test]
fn an_empty_page_puts_the_marks_10_mm_inside_its_corners_clockwise() {
    let corners = registration_mark_corners(A4, None);
    assert_eq!(
        corners,
        [
            Point::new(10.0, 10.0),
            Point::new(200.0, 10.0),
            Point::new(200.0, 287.0),
            Point::new(10.0, 287.0),
        ]
    );
}

#[test]
fn a_small_design_keeps_the_page_corners() {
    let art = Some((Point::new(60.0, 80.0), Point::new(150.0, 200.0)));
    assert_eq!(
        registration_mark_corners(A4, art),
        registration_mark_corners(A4, None)
    );
}

#[test]
fn a_full_page_design_grows_its_box_by_10_mm_clamped_inside_the_page() {
    let art = Some((Point::new(5.0, 15.0), Point::new(190.0, 292.0)));
    let corners = registration_mark_corners(A4, art);
    assert_eq!(
        corners,
        [
            Point::new(10.0, 10.0),
            Point::new(200.0, 10.0),
            Point::new(200.0, 287.0),
            Point::new(10.0, 287.0),
        ],
        "every grown corner leaves the page, so each is clamped"
    );
    let art = Some((Point::new(15.0, 15.0), Point::new(150.0, 150.0)));
    let corners = registration_mark_corners(A4, art);
    assert_eq!(corners[0], Point::new(10.0, 10.0));
    assert_eq!(corners[2], Point::new(160.0, 160.0));
    for c in corners {
        assert!(!(c.x > 5.0 && c.x < 160.0 && c.y > 5.0 && c.y < 160.0) || c == corners[0]);
    }
}

#[test]
fn insert_adds_four_numbered_marks_as_one_group_in_one_step() {
    let d = Document::new(1);
    let _ = d.create_rect(RectBounds::from_corners(
        Point::new(50.0, 50.0),
        Point::new(80.0, 80.0),
    ));
    let before = d.own_step_count();
    let group = d
        .insert_registration_marks(registration_mark_corners(A4, None))
        .unwrap();
    assert_eq!(d.own_step_count(), before + 1);
    assert_eq!(last_label(&d), INSERT_REGISTRATION_MARKS_COMMIT_LABEL);
    let object = d.marks_object(group).unwrap();
    let numbers: Vec<u8> = object.marks.iter().map(|m| m.number).collect();
    assert_eq!(numbers, [1, 2, 3, 4]);
    assert!(
        object
            .marks
            .iter()
            .all(|m| (m.diameter.as_mm() - 6.0).abs() < 1e-12)
    );
    assert_eq!(d.marks_object_of(object.marks[2].id).unwrap().id, group);
    undo(&d);
    assert!(d.marks_object(group).is_none());
}

#[test]
fn a_move_moves_the_centre_only_and_the_number_stays() {
    let d = Document::new(1);
    let group = d
        .insert_registration_marks(registration_mark_corners(A4, None))
        .unwrap();
    let mark = d.marks_object(group).unwrap().marks[1].clone();
    d.translate_objects(&[mark.id], Vec2::new(5.0, -3.0))
        .unwrap();
    let Some(ObjectSnapshot::RegistrationMark(moved)) = d.object(mark.id) else {
        panic!("a mark");
    };
    assert_eq!(moved.centre, Point::new(205.0, 7.0));
    assert_eq!(moved.number, 2);
    assert_eq!(moved.diameter, mark.diameter);
}

#[test]
fn size_writes_every_mark_in_one_step_and_refuses_out_of_range() {
    let d = Document::new(1);
    let group = d
        .insert_registration_marks(registration_mark_corners(A4, None))
        .unwrap();
    let before = d.own_step_count();
    d.set_registration_mark_size(group, Length::from_mm(10.0), Length::from_mm(0.5))
        .unwrap();
    assert_eq!(d.own_step_count(), before + 1);
    assert!(
        marks(&d)
            .marks
            .iter()
            .all(|m| m.diameter == Length::from_mm(10.0))
    );
    for (dia, line) in [(2.9, 0.25), (20.1, 0.25), (6.0, 0.04), (6.0, 1.1)] {
        assert_eq!(
            d.set_registration_mark_size(group, Length::from_mm(dia), Length::from_mm(line)),
            Err(ObjectEditError::InvalidStrokeWidth)
        );
    }
}

#[test]
fn delete_keeps_two_marks_and_add_refills_the_lowest_number() {
    let d = Document::new(1);
    let corners = registration_mark_corners(A4, None);
    let group = d.insert_registration_marks(corners).unwrap();
    let ids: Vec<NodeId> = marks(&d).marks.iter().map(|m| m.id).collect();
    d.delete_objects(&[ids[2]]).unwrap();
    assert_eq!(
        d.delete_objects(&[ids[0], ids[1]]),
        Err(ObjectEditError::MarksBelowTwo)
    );
    assert_eq!(marks(&d).marks.len(), 3);
    d.delete_objects(&[ids[3]]).unwrap();
    assert_eq!(
        d.delete_objects(&[ids[0]]),
        Err(ObjectEditError::MarksBelowTwo)
    );
    let added = d.add_registration_mark(group, corners).unwrap();
    assert_eq!(last_label(&d), ADD_REGISTRATION_MARK_COMMIT_LABEL);
    let object = marks(&d);
    let new = object.marks.iter().find(|m| m.id == added).unwrap();
    assert_eq!((new.number, new.centre), (3, corners[2]));
    d.add_registration_mark(group, corners).unwrap();
    assert_eq!(
        d.add_registration_mark(group, corners),
        Err(ObjectEditError::TooDeep)
    );
    assert!(!d.delete_leaves_one_mark(&[added]));
    let all: Vec<NodeId> = marks(&d).marks.iter().map(|m| m.id).collect();
    assert!(d.delete_leaves_one_mark(&all[..3]));
    assert!(
        !d.delete_leaves_one_mark(&all),
        "all marks delete the object"
    );
    d.delete_objects(&[group]).unwrap();
    assert!(
        d.marks_object(group).is_none(),
        "the object deletes as a whole"
    );
}

#[test]
fn a_marks_object_cannot_be_ungrouped() {
    let d = Document::new(1);
    let group = d
        .insert_registration_marks(registration_mark_corners(A4, None))
        .unwrap();
    assert_eq!(d.ungroup(&[group]), Err(ObjectEditError::NotAGroup));
}
