//! The pure align and distribute translations
//! (`specs/0061-align-and-distribute/` criteria 2 to 4, 6 to 9): worked
//! examples, the reference rules, the no-ops and the typed gap.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]

use curvyo_document_core::{Point, Vec2};
use curvyo_ui_core::{
    AlignAxis, AlignBox, AlignCommand, DistributeMode, align_translations, distribute_translations,
};

/// A box from its two extreme corners, in y-down document millimetres.
fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> AlignBox {
    AlignBox::new(Point::new(x0, y0), Point::new(x1, y1))
}

fn dx(translation: Vec2) -> f64 {
    assert_eq!(translation.y, 0.0, "a horizontal command moves x only");
    translation.x
}

fn dy(translation: Vec2) -> f64 {
    assert_eq!(translation.x, 0.0, "a vertical command moves y only");
    translation.y
}

/// Criterion 2's worked example, widened from the flip's: two rectangles
/// 10 apart, align left puts B's left edge on A's.
#[test]
fn c2_align_left_moves_every_box_onto_the_reference_left_edge() {
    let boxes = [rect(0.0, 0.0, 10.0, 10.0), rect(20.0, 5.0, 30.0, 15.0)];
    let reference = AlignBox::union(&boxes).unwrap();
    let moves = align_translations(&boxes, &reference, AlignCommand::Left);
    assert_eq!(dx(moves[0]), 0.0);
    assert_eq!(dx(moves[1]), -20.0);
}

/// Criterion 3: the reference is the selection box, an object already on it
/// does not move, and a centre align moves by the centre distance.
#[test]
fn c3_centre_and_right_align_onto_the_selection_box() {
    let boxes = [rect(0.0, 0.0, 10.0, 10.0), rect(20.0, 0.0, 26.0, 4.0)];
    let reference = AlignBox::union(&boxes).unwrap();
    let moves = align_translations(&boxes, &reference, AlignCommand::CentreH);
    assert_eq!(dx(moves[0]), 13.0 - 5.0);
    assert_eq!(dx(moves[1]), 13.0 - 23.0);
    let moves = align_translations(&boxes, &reference, AlignCommand::Right);
    assert_eq!(dx(moves[0]), 26.0 - 10.0);
    assert_eq!(dx(moves[1]), 0.0);
}

/// Criterion 3: an alignment that changes nothing gives every zero.
#[test]
fn c3_an_alignment_that_changes_nothing_is_all_zero() {
    let boxes = [rect(0.0, 0.0, 10.0, 10.0), rect(0.0, 20.0, 10.0, 30.0)];
    let reference = AlignBox::union(&boxes).unwrap();
    for command in [
        AlignCommand::Left,
        AlignCommand::CentreH,
        AlignCommand::Right,
    ] {
        let moves = align_translations(&boxes, &reference, command);
        assert!(moves.iter().all(|m| *m == Vec2::ZERO), "{command:?}");
    }
}

/// Criterion 4: with one object the reference is the active page's rectangle.
#[test]
fn c4_one_object_aligns_to_the_page() {
    let page = rect(0.0, 0.0, 210.0, 297.0);
    let object = rect(40.0, 30.0, 50.0, 44.0);
    let moves = align_translations(&[object], &page, AlignCommand::CentreH);
    assert_eq!(dx(moves[0]), 105.0 - 45.0);
    let moves = align_translations(&[object], &page, AlignCommand::Bottom);
    assert_eq!(dy(moves[0]), 297.0 - 44.0);
}

/// Criterion 6, centres: the outermost stay, the middle centre is exact.
#[test]
fn c6_distribute_centres_keeps_the_ends_and_spaces_the_centres() {
    let boxes = [
        rect(0.0, 0.0, 10.0, 10.0),
        rect(12.0, 0.0, 20.0, 10.0),
        rect(40.0, 0.0, 60.0, 10.0),
    ];
    let moves =
        distribute_translations(&boxes, AlignAxis::Horizontal, DistributeMode::Centres, None);
    assert_eq!(dx(moves[0]), 0.0);
    assert_eq!(dx(moves[1]), 27.5 - 16.0);
    assert_eq!(dx(moves[2]), 0.0);
}

/// Criterion 6, gaps: the ends stay, the free space is shared evenly.
#[test]
fn c6_distribute_gaps_keeps_the_ends_and_shares_the_space() {
    // Extents 10, 10, 10: from 0 to 60 is 60 mm, 30 mm of it free, so each
    // gap is 15: the middle box's left edge goes to 25.
    let boxes = [
        rect(0.0, 0.0, 10.0, 10.0),
        rect(12.0, 0.0, 22.0, 10.0),
        rect(50.0, 0.0, 60.0, 10.0),
    ];
    let moves = distribute_translations(&boxes, AlignAxis::Horizontal, DistributeMode::Gaps, None);
    assert_eq!(dx(moves[0]), 0.0);
    assert_eq!(dx(moves[1]), 25.0 - 12.0);
    assert_eq!(dx(moves[2]), 0.0);
}

/// Criterion 6: a vertical distribute moves y only and orders by y.
#[test]
fn c6_a_vertical_distribute_orders_by_y() {
    let boxes = [
        rect(0.0, 40.0, 10.0, 50.0),
        rect(0.0, 0.0, 10.0, 10.0),
        rect(0.0, 80.0, 10.0, 100.0),
    ];
    let moves = distribute_translations(&boxes, AlignAxis::Vertical, DistributeMode::Centres, None);
    assert_eq!(dy(moves[0]), 47.5 - 45.0);
    assert_eq!(dy(moves[1]), 0.0);
    assert_eq!(dy(moves[2]), 0.0);
}

/// Criterion 7: fewer than three objects change nothing, both modes.
#[test]
fn c7_a_distribute_with_fewer_than_three_objects_is_all_zero() {
    let boxes = [rect(0.0, 0.0, 10.0, 10.0), rect(30.0, 0.0, 40.0, 10.0)];
    for mode in [DistributeMode::Gaps, DistributeMode::Centres] {
        for axis in [AlignAxis::Horizontal, AlignAxis::Vertical] {
            let moves = distribute_translations(&boxes, axis, mode, None);
            assert!(moves.iter().all(|m| *m == Vec2::ZERO), "{mode:?} {axis:?}");
        }
    }
}

/// Criterion 8: a typed gap replaces the ends; the first stays.
#[test]
fn c8_a_typed_gap_spaces_from_the_first_object() {
    let boxes = [
        rect(0.0, 0.0, 10.0, 10.0),
        rect(14.0, 0.0, 22.0, 10.0),
        rect(50.0, 0.0, 60.0, 10.0),
    ];
    let moves = distribute_translations(
        &boxes,
        AlignAxis::Horizontal,
        DistributeMode::Gaps,
        Some(5.0),
    );
    assert_eq!(dx(moves[0]), 0.0);
    assert_eq!(dx(moves[1]), 15.0 - 14.0);
    assert_eq!(dx(moves[2]), 28.0 - 50.0);
}

/// Criterion 8: a typed gap of zero touches the objects.
#[test]
fn c8_a_typed_gap_of_zero_touches() {
    let boxes = [
        rect(0.0, 0.0, 10.0, 10.0),
        rect(14.0, 0.0, 22.0, 10.0),
        rect(50.0, 0.0, 60.0, 10.0),
    ];
    let moves = distribute_translations(
        &boxes,
        AlignAxis::Horizontal,
        DistributeMode::Gaps,
        Some(0.0),
    );
    assert_eq!(dx(moves[1]), 10.0 - 14.0);
    assert_eq!(dx(moves[2]), 18.0 - 50.0);
}

/// Criterion 9: the boxes are the caller's true boxes — a rotated object's
/// document-axis box decides, not its oriented one; the function only sees
/// `AlignBox`es, so this holds by construction. The worked check: a narrow
/// tall box aligns by its document-axis left edge.
#[test]
fn c9_the_document_axis_box_decides() {
    let boxes = [rect(0.0, 0.0, 10.0, 40.0), rect(20.0, 0.0, 24.0, 4.0)];
    let reference = AlignBox::union(&boxes).unwrap();
    let moves = align_translations(&boxes, &reference, AlignCommand::Right);
    assert_eq!(dx(moves[0]), 24.0 - 10.0);
    assert_eq!(dx(moves[1]), 0.0);
}

/// Criterion 3: a difference under the move tolerance is no move, so a box
/// a micrometre off the reference stays.
#[test]
fn a_difference_under_the_tolerance_is_no_move() {
    let boxes = [rect(0.0, 0.0, 10.0, 10.0), rect(1e-9, 0.0, 10.0, 10.0)];
    let reference = AlignBox::union(&boxes).unwrap();
    let moves = align_translations(&boxes, &reference, AlignCommand::Left);
    assert_eq!(moves[1], Vec2::ZERO);
}

/// Ties in distribute order keep the input order; equal boxes overlap evenly.
#[test]
fn ties_keep_the_input_order() {
    let boxes = [
        rect(0.0, 0.0, 10.0, 10.0),
        rect(0.0, 0.0, 10.0, 10.0),
        rect(40.0, 0.0, 50.0, 10.0),
    ];
    let moves =
        distribute_translations(&boxes, AlignAxis::Horizontal, DistributeMode::Centres, None);
    assert_eq!(dx(moves[0]), 0.0);
    // The second of the tied pair is the middle in rank and moves to the
    // even centre; its twin keeps the first rank and stays.
    assert_eq!(dx(moves[1]), 20.0);
    assert_eq!(dx(moves[2]), 0.0);
}
