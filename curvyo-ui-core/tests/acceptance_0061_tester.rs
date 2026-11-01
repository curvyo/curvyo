//! Independent pure-plan checks for specification 0061, from the criteria.
#![allow(
    clippy::unwrap_used,
    clippy::many_single_char_names,
    clippy::manual_midpoint,
    clippy::assert_is_empty
)]
use curvyo_document_core::{Point, Vec2};
use curvyo_ui_core::{
    AlignAxis, AlignBox, AlignCommand, DistributeMode, align_translations,
    distribute_on_frame_translations, distribute_translations, grid_translations,
};
use proptest::prelude::*;

const EPS: f64 = 1e-6;
fn b(x: f64, y: f64, w: f64, h: f64) -> AlignBox {
    AlignBox::new(Point::new(x, y), Point::new(x + w, y + h))
}
fn moved(bx: &AlignBox, t: Vec2) -> AlignBox {
    AlignBox::new(
        Point::new(bx.min.x + t.x, bx.min.y + t.y),
        Point::new(bx.max.x + t.x, bx.max.y + t.y),
    )
}

#[test]
fn ac2_align_all_six_onto_reference() {
    let r = b(10.0, 20.0, 100.0, 60.0);
    let boxes = [b(0.0, 0.0, 10.0, 10.0), b(200.0, 300.0, 30.0, 8.0)];
    for (cmd, f) in [
        (
            AlignCommand::Left,
            (|x: &AlignBox| x.min.x) as fn(&AlignBox) -> f64,
        ),
        (AlignCommand::Right, |x| x.max.x),
        (AlignCommand::Top, |x| x.min.y),
        (AlignCommand::Bottom, |x| x.max.y),
        (AlignCommand::CentreH, |x| (x.min.x + x.max.x) / 2.0),
        (AlignCommand::CentreV, |x| (x.min.y + x.max.y) / 2.0),
    ] {
        let t = align_translations(&boxes, &r, cmd);
        let want = f(&r);
        for (bx, t) in boxes.iter().zip(&t) {
            assert!((f(&moved(bx, *t)) - want).abs() < EPS, "{cmd:?}");
        }
    }
}

#[test]
fn ac3_object_on_reference_gets_zero_translation_and_other_axis_untouched() {
    let r = b(0.0, 0.0, 50.0, 50.0);
    let t = align_translations(&[b(0.0, 7.0, 10.0, 10.0)], &r, AlignCommand::Left);
    assert_eq!(t, vec![Vec2::ZERO]);
    let t = align_translations(&[b(9.0, 7.0, 10.0, 10.0)], &r, AlignCommand::Left);
    assert!(t[0].y == 0.0 && (t[0].x + 9.0).abs() < EPS);
}

#[test]
fn ac7_fewer_than_three_without_gap_changes_nothing() {
    for n in 0..3 {
        let boxes: Vec<AlignBox> = (0..n)
            .map(|i| b(f64::from(i) * 17.0, 0.0, 5.0, 5.0))
            .collect();
        for mode in [DistributeMode::Gaps, DistributeMode::Centres] {
            let t = distribute_translations(&boxes, AlignAxis::Horizontal, mode, None);
            assert!(t.iter().all(|v| *v == Vec2::ZERO), "n={n}");
        }
    }
}

#[test]
fn ac16_frame_distribution_zero_one_two_boxes() {
    let f = b(100.0, 0.0, 300.0, 100.0);
    assert!(
        distribute_on_frame_translations(
            &[],
            &f,
            AlignAxis::Horizontal,
            DistributeMode::Gaps,
            None
        )
        .is_empty()
    );
    let one = [b(110.0, 5.0, 20.0, 10.0)];
    let t = distribute_on_frame_translations(
        &one,
        &f,
        AlignAxis::Horizontal,
        DistributeMode::Gaps,
        None,
    );
    assert!((moved(&one[0], t[0]).min.x - 240.0).abs() < EPS);
    let two = [b(350.0, 0.0, 30.0, 10.0), b(110.0, 0.0, 30.0, 10.0)];
    let t = distribute_on_frame_translations(
        &two,
        &f,
        AlignAxis::Horizontal,
        DistributeMode::Gaps,
        None,
    );
    let (a, c) = (moved(&two[1], t[1]), moved(&two[0], t[0]));
    assert!(
        (a.min.x - 180.0).abs() < EPS && (c.min.x - 290.0).abs() < EPS,
        "ordered by position: {a:?} {c:?}"
    );
}

#[test]
fn ac19_ac20_grid_refusal_and_cells() {
    let boxes = [
        b(0.0, 0.0, 10.0, 10.0),
        b(30.0, 0.0, 20.0, 10.0),
        b(0.0, 50.0, 6.0, 30.0),
    ];
    assert!(grid_translations(&boxes, 1, 2, 0.0).is_none());
    assert!(grid_translations(&boxes, 0, 5, 0.0).is_none());
    let t = grid_translations(&boxes, 2, 2, 5.0).unwrap();
    // cell 20 x 30, corner (0,0)
    for (i, want) in [(0, (10.0, 15.0)), (1, (35.0, 15.0)), (2, (10.0, 50.0))] {
        let c = moved(&boxes[i], t[i]);
        let got = ((c.min.x + c.max.x) / 2.0, (c.min.y + c.max.y) / 2.0);
        assert!(
            (got.0 - want.0).abs() < EPS && (got.1 - want.1).abs() < EPS,
            "box {i}: {got:?}"
        );
    }
    assert_eq!(grid_translations(&[], 1, 1, 0.0).map(|v| v.len()), Some(0));
}

/// Finding 0061-F1 (fixed): two top-aligned objects of different heights keep their
/// left-to-right order.
#[test]
fn ac20_top_aligned_row_of_unequal_heights_keeps_left_to_right_order() {
    let boxes = [b(0.0, 0.0, 10.0, 10.0), b(30.0, 0.0, 20.0, 4.0)];
    let t = grid_translations(&boxes, 1, 2, 0.0).unwrap();
    let first = moved(&boxes[0], t[0]);
    let second = moved(&boxes[1], t[1]);
    assert!(
        first.min.x < second.min.x,
        "left object stays first: {first:?} {second:?}"
    );
}

proptest! {
    #[test]
    fn distribute_gaps_equal_and_ends_stay(
        raw in proptest::collection::vec((0.0f64..500.0, 1.0f64..50.0), 3..8)
    ) {
        let boxes: Vec<AlignBox> = raw.iter().map(|&(x, w)| b(x, 0.0, w, 5.0)).collect();
        let t = distribute_translations(&boxes, AlignAxis::Horizontal, DistributeMode::Gaps, None);
        let mut order: Vec<usize> = (0..boxes.len()).collect();
        order.sort_by(|&i, &j| boxes[i].min.x.total_cmp(&boxes[j].min.x));
        let m: Vec<AlignBox> = order.iter().map(|&i| moved(&boxes[i], t[i])).collect();
        prop_assert!((m[0].min.x - boxes[order[0]].min.x).abs() < 1e-3);
        let g0 = m[1].min.x - m[0].max.x;
        for w in m.windows(2) {
            prop_assert!((w[1].min.x - w[0].max.x - g0).abs() < 1e-6);
        }
        prop_assert!(t.iter().all(|v| v.y == 0.0));
    }

    #[test]
    fn grid_boxes_do_not_overlap_when_gap_nonnegative(
        raw in proptest::collection::vec((0.0f64..300.0, 0.0f64..300.0, 1.0f64..40.0, 1.0f64..40.0), 1..12),
        gap in 0.0f64..10.0
    ) {
        let boxes: Vec<AlignBox> = raw.iter().map(|&(x, y, w, h)| b(x, y, w, h)).collect();
        let n = u32::try_from(boxes.len()).unwrap();
        let cols = (1..=n).find(|c| c * c >= n).unwrap();
        let rows = n.div_ceil(cols);
        let t = grid_translations(&boxes, rows, cols, gap).unwrap();
        let m: Vec<AlignBox> = boxes.iter().zip(&t).map(|(x, t)| moved(x, *t)).collect();
        for i in 0..m.len() { for j in i + 1..m.len() {
            let ox = m[i].min.x < m[j].max.x - 1e-9 && m[j].min.x < m[i].max.x - 1e-9;
            let oy = m[i].min.y < m[j].max.y - 1e-9 && m[j].min.y < m[i].max.y - 1e-9;
            prop_assert!(!(ox && oy));
        }}
    }
}
