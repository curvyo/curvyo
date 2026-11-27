//! The stitch plan of `specs/0160-embroidery` criteria 7 to 10, 15 in the DST frame (0.1 mm from
//! the hoop centre, Y up): running stitch, tatami fill and satin through the port, ties, trims,
//! jumps, colour changes and the end.

#![allow(
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::items_after_statements,
    clippy::type_complexity,
    missing_docs
)]

use curvyo_document_core::{Point, Vec2};
use curvyo_embroidery_core::{
    DST_MAX_STITCH, EmbroiderySettings, ItemOutline, Mm, OpKind, PlanItem, PlanOperation,
    StitchKind, StitchPlan, ThreadColour, build_plan, edit_setting,
};

const CENTRE: Point = Point { x: 95.0, y: 145.0 };

fn line(points: &[(f64, f64)], closed: bool) -> ItemOutline {
    ItemOutline {
        anchors: points
            .iter()
            .map(|&(x, y)| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO))
            .collect(),
        closed,
    }
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> ItemOutline {
    line(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h)], true)
}

fn item(outlines: Vec<ItemOutline>, stroke: f64) -> PlanItem {
    PlanItem {
        outlines,
        stroke_width: Mm::new(stroke).unwrap(),
        stacking: 0,
    }
}

fn op(kind: OpKind, thread: u32, items: Vec<PlanItem>) -> PlanOperation {
    PlanOperation {
        kind,
        settings: EmbroiderySettings::default(),
        thread: ThreadColour::parse(&format!("#{thread:06X}")).unwrap(),
        items,
    }
}

fn plan(ops: &[PlanOperation]) -> StitchPlan {
    build_plan(ops, CENTRE, DST_MAX_STITCH)
}

/// The needle points of block `b` without its ties (3 lock points each end beyond the run).
fn needles(plan: &StitchPlan, b: usize) -> Vec<(i32, i32)> {
    plan.stitches[plan.blocks[b].stitches.clone()]
        .iter()
        .filter(|s| s.kind == StitchKind::Normal)
        .map(|s| (s.at.x.get(), s.at.y.get()))
        .collect()
}

#[test]
fn criterion_7_a_100_mm_line_gives_41_needle_points_2_5_mm_apart() {
    let p = plan(&[op(
        OpKind::Running,
        0x00C8_102E,
        vec![item(
            vec![line(&[(45.0, 145.0), (145.0, 145.0)], false)],
            0.3,
        )],
    )]);
    let points = needles(&p, 0);
    // The tie-in ends on the run's first point; the tie-off adds four points after its last.
    let run = &points[4..points.len() - 4];
    assert_eq!(run.len(), 41, "criterion 7");
    assert_eq!(run[0], (-500, 0));
    assert_eq!(run[40], (500, 0));
    for pair in run.windows(2) {
        assert!((pair[1].0 - pair[0].0 - 25).abs() <= 1, "{pair:?}");
    }
    assert_eq!(
        &points[..5],
        [(-500, 0), (-493, 0), (-485, 0), (-493, 0), (-500, 0)],
        "Ink/Stitch's half stitch: 0.75, 1.5, 0.75 mm and home"
    );
}

#[test]
fn criterion_8_a_tatami_rectangle_stays_inside_with_short_stitches() {
    let p = plan(&[op(
        OpKind::Tatami,
        0x0030_60A0,
        vec![item(vec![rect(70.0, 130.0, 50.0, 30.0)], 0.3)],
    )]);
    let points = needles(&p, 0);
    assert!(points.len() > 1_000);
    for &(x, y) in &points {
        assert!(
            (-251..=251).contains(&x) && (-151..=151).contains(&y),
            "{x} {y}"
        );
    }
    for pair in points.windows(2) {
        let d = f64::from(pair[1].0 - pair[0].0).hypot(f64::from(pair[1].1 - pair[0].1));
        assert!(d <= 31.5, "criterion 8: {d}");
    }
}

#[test]
fn criterion_9_satin_alternates_1_5_mm_each_side_and_a_narrow_one_runs() {
    let satin = item(vec![line(&[(75.0, 145.0), (115.0, 145.0)], false)], 3.0);
    let p = plan(&[op(OpKind::Satin, 0x0000_0000, vec![satin])]);
    let points = needles(&p, 0);
    let column: Vec<_> = points.iter().filter(|p| p.1.abs() == 15).collect();
    assert!(column.len() >= 200, "{}", column.len());
    assert_eq!(p.narrow, [0]);
    let narrow = item(vec![line(&[(75.0, 145.0), (115.0, 145.0)], false)], 0.3);
    let p = plan(&[op(OpKind::Satin, 0x0000_0000, vec![narrow])]);
    assert_eq!(p.narrow, [1], "criterion 9: runs as running stitch");
    assert!(needles(&p, 0).iter().all(|p| p.1 == 0));
}

#[test]
fn criterion_10_trims_jumps_colour_changes_and_the_end() {
    let near = |x| item(vec![line(&[(x, 145.0), (x + 10.0, 145.0)], false)], 0.3);
    let p = plan(&[
        op(
            OpKind::Running,
            0x00C8_102E,
            vec![near(80.0), near(93.0), near(106.0)],
        ),
        op(OpKind::Running, 0x0030_60A0, vec![near(120.0)]),
    ]);
    let kinds: Vec<StitchKind> = p
        .stitches
        .iter()
        .map(|s| s.kind)
        .filter(|k| *k != StitchKind::Normal)
        .collect();
    use StitchKind::{ColourChange, End, Jump, Trim};
    // From the centre (x 95) the nearest start is 93; from 103 the line at 106 (3 mm, a jump);
    // from 116 back to 80 (36 mm, a trim); then the blue line after a colour change.
    assert_eq!(kinds.first(), Some(&Jump));
    assert_eq!(kinds.last(), Some(&End));
    assert_eq!(kinds.iter().filter(|k| **k == ColourChange).count(), 1);
    assert_eq!(kinds.iter().filter(|k| **k == Trim).count(), 1);
    assert_eq!(p.blocks.len(), 4);
    let order: Vec<usize> = p.blocks.iter().map(|b| b.item).collect();
    assert_eq!(order, [1, 2, 0, 0], "nearest next");
    let jumps_between = kinds.iter().filter(|k| **k == Jump).count();
    assert!(jumps_between >= 4, "{kinds:?}");
}

#[test]
fn criterion_6_triple_stitch_runs_forward_back_forward() {
    let mut s = op(
        OpKind::Running,
        0x00C8_102E,
        vec![item(
            vec![line(&[(85.0, 145.0), (105.0, 145.0)], false)],
            0.3,
        )],
    );
    s.settings = edit_setting(OpKind::Running, s.settings, "repeats", "3").unwrap();
    let points = needles(&plan(&[s]), 0);
    let at_end = points.iter().filter(|p| p.0 == 100).count();
    assert!(at_end >= 2, "turns at the far end: {points:?}");
    assert_eq!(
        points.iter().filter(|p| p.0 == -100).count(),
        3,
        "start, turn, tie"
    );
}
