//! Independent acceptance tests for `specs/0118-machine-control` slice 2c, laser-core part
//! (criteria 29 to 32; `adrs.md` O4 to O7), written from the specification and the ADR before
//! reading the implementation. They read only the text the job and the frame write, so no golden
//! is needed here.
#![allow(clippy::unwrap_used, clippy::missing_panics_doc, clippy::expect_used)]

mod job_fixtures;

use curvyo_document_core::Point;
use curvyo_laser_core::OriginCorner;
use curvyo_laser_core::job::{
    Cause, FrameShape, GateFacts, JobInput, Names, PlanCache, PointUm, Start, Timeline, build_job,
    check_text, gate, write, write_frame,
};
use curvyo_laser_core::operations::OperationKind::Cut;
use job_fixtures::{Page, leaf, line, own, plain, rect};

/// A line from (10, 20) and a rectangle at (80, 50): the burn box is 100 x 50 mm, from (10, 20).
fn page(corner: OriginCorner) -> Page {
    let mut page = Page::new().corner(corner);
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![
            line(Point::new(10.0, 20.0), Point::new(60.0, 40.0)),
            rect(80.0, 50.0, 30.0, 20.0),
        ])],
    );
    page
}

fn with_user_origin(mut page: Page, x_um: i64, y_um: i64) -> JobInput {
    page.input.start = Start::UserOrigin;
    page.input.user_origin = Some(PointUm::new(x_um, y_um));
    page.input
}

fn job_text(input: &JobInput) -> String {
    let built = build_job(input, &[], &mut PlanCache::default());
    String::from_utf8(write(input.machine.values.output_format, &built.job)).unwrap()
}

/// A G0/G1 move of GRBL text: where it ends and whether it burns.
#[derive(Debug, Clone, Copy)]
struct Move {
    to: (f64, f64),
    burns: bool,
}

/// The moves of `text`, from `start`. Absolute positions only (the job's `G90`); the spindle and
/// the power are modal as GRBL keeps them.
fn moves(text: &str, start: (f64, f64)) -> Vec<Move> {
    let mut out = Vec::new();
    let mut cur = start;
    let mut on = false;
    let mut power = 0.0_f64;
    for line in text.lines() {
        if line.starts_with(';') {
            continue;
        }
        if line.starts_with("M5") {
            on = false;
        }
        if line.starts_with("M3") || line.starts_with("M4") {
            on = true;
        }
        let is_move = line.starts_with("G0 ") || line.starts_with("G1 ");
        if !is_move {
            continue;
        }
        let mut to = cur;
        for word in line.split_whitespace().skip(1) {
            let (key, value) = word.split_at(1);
            match key {
                "X" => to.0 = value.parse().unwrap(),
                "Y" => to.1 = value.parse().unwrap(),
                "S" => power = value.parse().unwrap(),
                _ => {}
            }
        }
        out.push(Move {
            to,
            burns: line.starts_with("G1 ") && on && power > 0.0,
        });
        cur = to;
    }
    out
}

/// Both ends of every burning move: the start of a burn is a point of the burn too (the first
/// burn of a User origin job starts on the point itself).
fn burn_ends(text: &str, start: (f64, f64)) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut from = start;
    for m in moves(text, start) {
        if m.burns {
            out.push(from);
            out.push(m.to);
        }
        from = m.to;
    }
    out
}

fn near(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6
}

fn min_of(points: &[(f64, f64)]) -> (f64, f64) {
    (
        points.iter().map(|p| p.0).fold(f64::MAX, f64::min),
        points.iter().map(|p| p.1).fold(f64::MAX, f64::min),
    )
}

const USER_MM: (f64, f64) = (150.0, 100.0);

// Criterion 31: the job is the machine-origin job shifted by one constant offset, per corner.

#[test]
fn c31_per_origin_corner_every_burn_line_is_machine_origins_plus_one_constant_offset() {
    for corner in [
        OriginCorner::BottomLeft,
        OriginCorner::BottomRight,
        OriginCorner::TopLeft,
        OriginCorner::TopRight,
    ] {
        let base = job_text(&page(corner).input);
        let user = job_text(&with_user_origin(page(corner), 150_000, 100_000));
        let base_burns = burn_ends(&base, (0.0, 0.0));
        let user_burns = burn_ends(&user, USER_MM);
        assert_eq!(base_burns.len(), user_burns.len(), "{corner:?}: same burns");
        assert!(!base_burns.is_empty(), "{corner:?}: the job burns");
        let delta = (
            user_burns[0].0 - base_burns[0].0,
            user_burns[0].1 - base_burns[0].1,
        );
        for (b, u) in base_burns.iter().zip(&user_burns) {
            assert!(
                near((u.0 - b.0, u.1 - b.1), delta),
                "{corner:?}: offset differs at {b:?} -> {u:?}"
            );
        }
    }
}

#[test]
fn c31_the_anchor_sits_on_the_user_origin_for_every_corner() {
    for corner in [
        OriginCorner::BottomLeft,
        OriginCorner::BottomRight,
        OriginCorner::TopLeft,
        OriginCorner::TopRight,
    ] {
        let user = job_text(&with_user_origin(page(corner), 150_000, 100_000));
        let burns = burn_ends(&user, USER_MM);
        // The box's min corner (the anchor, Machine origin's box corner) lands on the point.
        assert!(
            near(min_of(&burns), USER_MM),
            "{corner:?}: min {:?}",
            min_of(&burns)
        );
    }
}

#[test]
fn c31_the_file_starts_and_ends_on_the_user_origin() {
    let text = job_text(&with_user_origin(
        page(OriginCorner::BottomLeft),
        150_000,
        100_000,
    ));
    let all = moves(&text, USER_MM);
    let last = all.last().expect("moves");
    assert!(
        near(last.to, USER_MM),
        "ends on the user origin, got {:?}",
        last.to
    );
    assert!(
        !text.contains("G10") && !text.contains("G92") && !text.contains("G54"),
        "no offset is written"
    );
}

#[test]
fn c31_user_origin_is_absolute_and_writes_no_homing() {
    let text = job_text(&with_user_origin(
        page(OriginCorner::BottomLeft),
        150_000,
        100_000,
    ));
    assert!(text.contains("G90"), "absolute coordinates");
    assert!(!text.contains("$H"), "User origin writes no $H");
}

// Criterion 31: the frame follows the shift and stays laser-off.

#[test]
fn c31_the_frame_traces_the_shifted_outline_and_stays_laser_off() {
    let input = with_user_origin(page(OriginCorner::BottomLeft), 150_000, 100_000);
    let built = build_job(&input, &[], &mut PlanCache::default());
    let format = input.machine.values.output_format;
    let frame =
        String::from_utf8(write_frame(format, &built.job, FrameShape::Rectangle).unwrap()).unwrap();
    assert!(
        !frame.contains("M3") && !frame.contains("M4"),
        "no spindle on"
    );
    for word in frame.split_whitespace().filter(|w| w.starts_with('S')) {
        assert_eq!(word, "S0", "no power above 0 in the frame");
    }
    let traced: Vec<(f64, f64)> = moves(&frame, USER_MM).iter().map(|m| m.to).collect();
    let (lo, hi) = traced.iter().fold(
        ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN)),
        |(lo, hi), p| {
            (
                (lo.0.min(p.0), lo.1.min(p.1)),
                (hi.0.max(p.0), hi.1.max(p.1)),
            )
        },
    );
    // The burn box is 100 x 50 mm from (10, 20); shifted to start at (150, 100).
    assert!(
        (lo.0 - 150.0).abs() < 1e-6 && (lo.1 - 100.0).abs() < 1e-6,
        "lo {lo:?}"
    );
    assert!(
        (hi.0 - 250.0).abs() < 1e-6 && (hi.1 - 150.0).abs() < 1e-6,
        "hi {hi:?}"
    );
}

// Criterion 31: the work-area check measures the placed job and names the distance.

fn bed_text(input: &JobInput) -> Option<(Cause, String)> {
    let built = build_job(input, &[], &mut PlanCache::default());
    let timeline = Timeline::of(&built.job);
    let g = gate(input, &built, &timeline, &GateFacts::default());
    let names = Names::default();
    g.issues
        .iter()
        .find(|i| matches!(i.cause, Cause::OutsideBed { .. }))
        .map(|i| (i.cause.clone(), check_text(i, &names).whole()))
}

#[test]
fn c31_a_user_origin_inside_the_bed_passes_the_work_area_check() {
    assert_eq!(
        bed_text(&with_user_origin(
            page(OriginCorner::BottomLeft),
            100_000,
            100_000
        )),
        None
    );
}

#[test]
fn c31_a_user_origin_that_pushes_the_job_past_the_bed_blocks_and_names_the_distance() {
    // The shift is (300, 80): the burn box's right edge lands at 410 mm, 10 mm past the 400 mm bed.
    let (cause, words) = bed_text(&with_user_origin(
        page(OriginCorner::BottomLeft),
        310_000,
        100_000,
    ))
    .expect("outside the bed");
    assert!(matches!(cause, Cause::OutsideBed { .. }));
    assert!(
        words.contains("outside the 400 × 300 mm bed by 10 mm."),
        "text was: {words}"
    );
}

#[test]
fn c31_machine_origin_keeps_the_bed_text_without_a_distance() {
    let mut base = page(OriginCorner::BottomLeft);
    base.operation(
        Cut,
        "Cut 2",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![rect(390.0, 10.0, 30.0, 20.0)])],
    );
    let (cause, words) = bed_text(&base.input).expect("outside the bed");
    assert!(matches!(cause, Cause::OutsideBed { over_um: None, .. }));
    assert!(
        words.ends_with("outside the 400 × 300 mm bed."),
        "text was: {words}"
    );
}
