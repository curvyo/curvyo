//! Tests of the Print and cut writes (`specs/0121-print-and-cut` criteria 9, 16, 19).
//!
//! Test-only: declared under `#[cfg(test)]` in `print_and_cut_commands.rs`.

use super::*;
use crate::DocumentSize;
use crate::manufacturing::NewOutputPage;
use crate::print_and_cut_codec::KEY_PRINT_AND_CUT;
use crate::units::{Point, Vec2};

fn sheet() -> (Document, NodeId) {
    let d = Document::new(1);
    let drawing = d.pages()[0].clone();
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    d.insert_registration_marks(crate::registration_mark_corners(
        (Point::new(0.0, 0.0), Point::new(210.0, 297.0)),
        None,
    ))
    .unwrap();
    let placement = d
        .place_page(
            &laser.id,
            &drawing.id,
            Affine::translation(Vec2::new(20.0, 0.0)),
        )
        .unwrap();
    (d, placement)
}

fn measures(d: &Document, id: NodeId) -> PrintAndCutMeasures {
    match d.print_and_cut(id).unwrap() {
        PrintAndCutRegister::Measures(m) => m,
        other => panic!("{other:?}"),
    }
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn undo(d: &Document) {
    let last = d.own_steps().last().unwrap().step;
    d.undo_step(last).unwrap();
}

#[test]
fn a_placement_without_measures_reads_absent() {
    let (d, placement) = sheet();
    assert_eq!(
        d.print_and_cut(placement),
        Some(PrintAndCutRegister::Absent)
    );
    assert_eq!(d.print_and_cut(NodeId::from_parts(9, 9)), None);
}

#[test]
fn each_measure_and_clear_is_one_step_and_undoes() {
    let (d, placement) = sheet();
    let before = d.own_step_count();
    d.set_print_and_cut_measure(placement, 1, Some([83_400, 41_200]))
        .unwrap();
    d.set_print_and_cut_measure(placement, 3, Some([-5, 7]))
        .unwrap();
    assert_eq!(d.own_step_count(), before + 2);
    assert_eq!(last_label(&d), PRINT_AND_CUT_MEASURE_COMMIT_LABEL);
    assert_eq!(measures(&d, placement).marks.len(), 2);
    d.set_print_and_cut_measure(placement, 3, None).unwrap();
    assert_eq!(
        measures(&d, placement)
            .marks
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        [1]
    );
    undo(&d);
    assert_eq!(measures(&d, placement).marks[&3], [-5, 7]);
    d.set_print_and_cut_measure(placement, 1, None).unwrap();
    d.set_print_and_cut_measure(placement, 3, None).unwrap();
    assert_eq!(
        d.print_and_cut(placement),
        Some(PrintAndCutRegister::Absent)
    );
    assert!(matches!(
        d.set_print_and_cut_measure(placement, 5, Some([0, 0])),
        Err(ManufacturingError::OutOfRange)
    ));
    assert!(matches!(
        d.set_print_and_cut_measure(placement, 1, Some([10_000_001, 0])),
        Err(ManufacturingError::OutOfRange)
    ));
}

#[test]
fn an_unreadable_register_is_replaced_by_the_next_write() {
    let (d, placement) = sheet();
    let meta = d.placement_meta(placement).unwrap();
    meta.insert(KEY_PRINT_AND_CUT, "garbage").unwrap();
    d.commit_with_label("test");
    assert_eq!(
        d.print_and_cut(placement),
        Some(PrintAndCutRegister::Unreadable)
    );
    d.set_print_and_cut_measure(placement, 2, Some([1, 2]))
        .unwrap();
    assert_eq!(measures(&d, placement).marks[&2], [1, 2]);
}

#[test]
fn tolerance_is_one_step_within_50_to_5000_um() {
    let (d, placement) = sheet();
    d.set_print_and_cut_tolerance(placement, 400).unwrap();
    assert_eq!(last_label(&d), PRINT_AND_CUT_TOLERANCE_COMMIT_LABEL);
    assert_eq!(measures(&d, placement).tolerance_um, 400);
    for bad in [49, 5001] {
        assert!(matches!(
            d.set_print_and_cut_tolerance(placement, bad),
            Err(ManufacturingError::OutOfRange)
        ));
    }
}

#[test]
fn apply_sets_the_matrix_keeps_the_drawing_and_reset_restores_exactly() {
    let (d, placement) = sheet();
    d.set_print_and_cut_measure(placement, 1, Some([1, 1]))
        .unwrap();
    let drawing = serde_json::to_string(&d.read_objects().objects).unwrap();
    let before = d.placed_page(placement).unwrap().matrix;
    let fit = Affine::translation(Vec2::new(30.0, 12.5)).compose(Affine::rotation_about(
        Point::new(0.0, 0.0),
        crate::Angle::from_radians(0.03),
    ));
    let steps = d.own_step_count();
    d.apply_print_and_cut(placement, fit).unwrap();
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(last_label(&d), APPLY_PRINT_AND_CUT_COMMIT_LABEL);
    assert_eq!(d.placed_page(placement).unwrap().matrix, fit);
    assert_eq!(measures(&d, placement).reset, Some(before));
    assert_eq!(
        serde_json::to_string(&d.read_objects().objects).unwrap(),
        drawing,
        "the drawing page is not changed"
    );
    d.reset_print_and_cut(placement).unwrap();
    assert_eq!(last_label(&d), RESET_PRINT_AND_CUT_COMMIT_LABEL);
    assert_eq!(d.placed_page(placement).unwrap().matrix, before);
    assert_eq!(measures(&d, placement).reset, None);
    assert!(matches!(
        d.reset_print_and_cut(placement),
        Err(ManufacturingError::NotFound)
    ));
    undo(&d);
    assert_eq!(
        d.placed_page(placement).unwrap().matrix,
        fit,
        "undo of Reset"
    );
    assert_eq!(measures(&d, placement).reset, Some(before));
}

#[test]
fn a_fit_off_the_laser_page_is_refused() {
    let (d, placement) = sheet();
    let steps = d.own_step_count();
    assert!(matches!(
        d.apply_print_and_cut(placement, Affine::translation(Vec2::new(5000.0, 0.0))),
        Err(ManufacturingError::OffPage)
    ));
    assert_eq!(d.own_step_count(), steps);
}

#[test]
fn a_duplicate_of_the_placement_copies_no_measures() {
    let (d, placement) = sheet();
    d.set_print_and_cut_measure(placement, 1, Some([1, 1]))
        .unwrap();
    let copy = d
        .duplicate_placements(&[placement], Vec2::new(5.0, 5.0), "duplicate")
        .unwrap()[0];
    assert_eq!(d.print_and_cut(copy), Some(PrintAndCutRegister::Absent));
}
