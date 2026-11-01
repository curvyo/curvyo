//! The pure align and distribute translations
//! (`specs/0061-align-and-distribute/` criteria 2 to 4, 6 to 9): worked
//! examples, the reference rules, the no-ops and the typed gap.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]

use curvyo_document_core::{Length, Point, Vec2};
use curvyo_ui_core::{
    AlignAxis, AlignBox, AlignCommand, DistributeMode, align_translations,
    distribute_on_frame_translations, distribute_translations, grid_translations,
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

/// The customer's amendment of 2026-10-30: a typed gap spaces the second of
/// two objects from the first; without one, two change nothing.
#[test]
fn a_typed_gap_distributes_two_objects_from_the_first() {
    let boxes = [rect(0.0, 0.0, 10.0, 10.0), rect(30.0, 0.0, 40.0, 10.0)];
    let moves = distribute_translations(
        &boxes,
        AlignAxis::Horizontal,
        DistributeMode::Gaps,
        Some(Length::from_mm(5.0)),
    );
    assert_eq!(dx(moves[0]), 0.0);
    assert_eq!(dx(moves[1]), 15.0 - 30.0);
    // Centres and the end-relative gaps still need three.
    let moves = distribute_translations(
        &boxes,
        AlignAxis::Horizontal,
        DistributeMode::Centres,
        Some(Length::from_mm(5.0)),
    );
    assert!(moves.iter().all(|m| *m == Vec2::ZERO));
    let moves = distribute_translations(&boxes, AlignAxis::Horizontal, DistributeMode::Gaps, None);
    assert!(moves.iter().all(|m| *m == Vec2::ZERO));
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
        Some(Length::from_mm(5.0)),
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
        Some(Length::from_mm(0.0)),
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
/// The customer's extension of 2026-10-30: with nothing selected, align acts
/// on the active page's objects with the page as the reference, whatever the
/// count.
#[test]
fn the_page_align_puts_every_page_object_on_the_page_edge() {
    // Two objects, both right of the A4 page's left edge.
    let page = rect(0.0, 0.0, 210.0, 297.0);
    let objects = [rect(40.0, 30.0, 50.0, 44.0), rect(90.0, 20.0, 110.0, 50.0)];
    let moves = align_translations(&objects, &page, AlignCommand::Left);
    assert_eq!(dx(moves[0]), -40.0);
    assert_eq!(dx(moves[1]), -90.0);
}

/// Criterion 16: on the page the n boxes leave n + 1 equal gaps (page edge to
/// first, between them, last to page edge); with centres the n centres cut
/// the page into n + 1 equal parts.
#[test]
fn c16_the_page_distribute_leaves_n_plus_one_equal_gaps() {
    let page = rect(10.0, 0.0, 110.0, 100.0);
    // Three boxes of 10 on a 100 mm page off the origin: 70 mm free, 17.5
    // per gap, so the left edges land on 27.5, 55 and 82.5.
    let boxes = [
        rect(70.0, 0.0, 80.0, 10.0),
        rect(20.0, 0.0, 30.0, 10.0),
        rect(45.0, 0.0, 55.0, 10.0),
    ];
    let moves = distribute_on_frame_translations(
        &boxes,
        &page,
        AlignAxis::Horizontal,
        DistributeMode::Gaps,
        None,
    );
    assert!((dx(moves[1]) - 7.5).abs() < 1e-9);
    assert!((dx(moves[2]) - 10.0).abs() < 1e-9);
    assert!((dx(moves[0]) - 12.5).abs() < 1e-9);
    // Centres: the page cut into four parts, centres at 35, 60, 85.
    let moves = distribute_on_frame_translations(
        &boxes,
        &page,
        AlignAxis::Horizontal,
        DistributeMode::Centres,
        None,
    );
    assert!((dx(moves[1]) - 10.0).abs() < 1e-9);
    assert!((dx(moves[2]) - 10.0).abs() < 1e-9);
    assert!((dx(moves[0]) - 10.0).abs() < 1e-9);
}

/// Criterion 16: one object is centred across the page in both modes; two
/// objects sit in thirds.
#[test]
fn c16_one_object_is_centred_and_two_sit_in_thirds() {
    let page = rect(0.0, 0.0, 210.0, 297.0);
    let one = [rect(10.0, 20.0, 30.0, 60.0)];
    for mode in [DistributeMode::Gaps, DistributeMode::Centres] {
        let moves = distribute_on_frame_translations(&one, &page, AlignAxis::Vertical, mode, None);
        // Centre 40 onto 148.5.
        assert!((dy(moves[0]) - 108.5).abs() < 1e-9);
    }
    let two = [rect(0.0, 0.0, 30.0, 10.0), rect(100.0, 0.0, 130.0, 10.0)];
    let moves = distribute_on_frame_translations(
        &two,
        &page,
        AlignAxis::Horizontal,
        DistributeMode::Gaps,
        None,
    );
    // 150 free over three gaps: 50, so the left edges land on 50 and 130.
    assert!((dx(moves[0]) - 50.0).abs() < 1e-9);
    assert!((dx(moves[1]) - 30.0).abs() < 1e-9);
    let moves = distribute_on_frame_translations(
        &two,
        &page,
        AlignAxis::Horizontal,
        DistributeMode::Centres,
        None,
    );
    // Centres on 70 and 140.
    assert!((dx(moves[0]) - 55.0).abs() < 1e-9);
    assert!((dx(moves[1]) - 25.0).abs() < 1e-9);
}

/// Criterion 16 with a typed gap: the first box starts that gap from the
/// page's start edge, the rest are spaced by it and may reach past the end.
#[test]
fn c16_a_typed_gap_on_the_page_starts_one_gap_from_the_edge() {
    let page = rect(0.0, 0.0, 100.0, 100.0);
    let boxes = [rect(20.0, 0.0, 30.0, 10.0), rect(45.0, 0.0, 55.0, 10.0)];
    let moves = distribute_on_frame_translations(
        &boxes,
        &page,
        AlignAxis::Horizontal,
        DistributeMode::Gaps,
        Some(Length::from_mm(5.0)),
    );
    assert_eq!(dx(moves[0]), -15.0);
    assert_eq!(dx(moves[1]), -25.0);
    // Nothing on the page is no distribute.
    let none = distribute_on_frame_translations(
        &[],
        &page,
        AlignAxis::Horizontal,
        DistributeMode::Gaps,
        None,
    );
    assert_eq!(none, Vec::<Vec2>::new());
}

/// Criterion 20's worked example: six boxes of different sizes, 2 rows, 3
/// columns, gap 5: cells as large as the largest box, each box centred in
/// its cell, reading order by the positions before, the block's top-left
/// on the selection box's top-left.
#[test]
fn c20_six_boxes_fill_two_rows_of_three_in_reading_order() {
    // Scrambled input. Reading order: the top line (boxes 4, 2, 0 by
    // their left edges; box 4 starts lower but its centre lies within the
    // line), then the bottom line (1, 5, 3).
    let boxes = [
        rect(100.0, 0.0, 110.0, 10.0),
        rect(0.0, 40.0, 20.0, 50.0),
        rect(50.0, 0.0, 70.0, 20.0),
        rect(90.0, 40.0, 96.0, 44.0),
        rect(0.0, 2.0, 8.0, 32.0),
        rect(40.0, 40.0, 52.0, 48.0),
    ];
    // Selection box top-left (0, 0); cells 20 x 30; gap 5.
    let moves = grid_translations(&boxes, 2, 3, 5.0).unwrap();
    let centre = |i: usize| {
        let b = boxes[i];
        (
            f64::midpoint(b.min.x, b.max.x) + moves[i].x,
            f64::midpoint(b.min.y, b.max.y) + moves[i].y,
        )
    };
    let expected = [
        (4, (10.0, 15.0)),
        (2, (35.0, 15.0)),
        (0, (60.0, 15.0)),
        (1, (10.0, 50.0)),
        (5, (35.0, 50.0)),
        (3, (60.0, 50.0)),
    ];
    for (index, (x, y)) in expected {
        let (cx, cy) = centre(index);
        assert!(
            (cx - x).abs() < 1e-6 && (cy - y).abs() < 1e-6,
            "box {index}: ({cx}, {cy})"
        );
    }
    // Criterion 21: a second arrange of the result changes nothing.
    let arranged: Vec<AlignBox> = boxes
        .iter()
        .zip(&moves)
        .map(|(b, m)| AlignBox::new(b.min.translated(*m), b.max.translated(*m)))
        .collect();
    let again = grid_translations(&arranged, 2, 3, 5.0).unwrap();
    assert!(again.iter().all(|m| *m == Vec2::ZERO), "{again:?}");
}

/// Criterion 19: a grid with fewer cells than boxes is refused; an extra
/// row is allowed (the last row partly empty).
#[test]
fn c19_too_few_cells_are_refused() {
    let boxes = [
        rect(0.0, 0.0, 10.0, 10.0),
        rect(20.0, 0.0, 30.0, 10.0),
        rect(40.0, 0.0, 50.0, 10.0),
    ];
    assert!(grid_translations(&boxes, 1, 2, 0.0).is_none());
    assert!(grid_translations(&boxes, 0, 3, 0.0).is_none());
    assert!(grid_translations(&boxes, 3, 3, 0.0).is_some());
}

/// Criterion 21: a grid that is already in place moves nothing.
#[test]
fn c21_a_grid_already_in_place_is_all_zero() {
    let boxes = [rect(0.0, 0.0, 10.0, 10.0), rect(12.0, 0.0, 22.0, 10.0)];
    let moves = grid_translations(&boxes, 1, 2, 2.0).unwrap();
    assert!(moves.iter().all(|m| *m == Vec2::ZERO));
}

/// Criterion 21 (the lead's decision on finding 0061-F3): boxes of unequal
/// size, whose arranged union corner lies inside the first cells, stay put
/// on a second press with the same values.
#[test]
fn c21_a_second_press_of_a_grid_of_unequal_boxes_moves_nothing() {
    let boxes = [
        rect(0.0, 0.0, 10.0, 10.0),
        rect(30.0, 0.0, 50.0, 10.0),
        rect(0.0, 50.0, 6.0, 80.0),
    ];
    let moves = grid_translations(&boxes, 2, 2, 5.0).unwrap();
    let arranged: Vec<AlignBox> = boxes
        .iter()
        .zip(&moves)
        .map(|(b, m)| AlignBox::new(b.min.translated(*m), b.max.translated(*m)))
        .collect();
    let again = grid_translations(&arranged, 2, 2, 5.0).unwrap();
    assert!(again.iter().all(|m| *m == Vec2::ZERO), "{again:?}");
}

/// Finding 0061-F1: a top-aligned row of different heights reads from left
/// to right.
#[test]
fn c20_a_top_aligned_row_of_different_heights_reads_left_to_right() {
    let boxes = [rect(0.0, 0.0, 10.0, 10.0), rect(30.0, 0.0, 50.0, 4.0)];
    let moves = grid_translations(&boxes, 1, 2, 0.0).unwrap();
    assert!(boxes[0].min.x + moves[0].x < boxes[1].min.x + moves[1].x);
}
