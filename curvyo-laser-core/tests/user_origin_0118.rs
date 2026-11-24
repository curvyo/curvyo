//! The user origin of `specs/0118-machine-control` slice 2c, criteria 29 to 31 (`adrs.md` O1, O4
//! to O7): the page's point read from the format 60 goldens, the job shifted so its anchor sits on
//! the point (per origin corner: every burn line is Machine origin's plus the offset), the frame,
//! and the work-area check on the placed job. `CURVYO_UPDATE_GOLDEN=1` rewrites the files.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

mod job_fixtures;

use std::path::PathBuf;

use curvyo_document_core::{ManufacturingSnapshot, Point, unpack};
use curvyo_laser_core::OriginCorner;
use curvyo_laser_core::job::{
    Cause, FrameShape, GateFacts, JobInput, Names, PlanCache, PointUm, Start, Timeline, build_job,
    check_text, frame_outline, gate, read_output_settings, write, write_frame,
};
use curvyo_laser_core::operations::OperationKind::Cut;
use job_fixtures::{Page, leaf, line, own, plain, rect};

fn dir(sub: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/output")
        .join(sub)
}

/// A line and a rectangle, so the box's corner is no burn start.
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

fn user_origin(mut page: Page, point: PointUm) -> Page {
    page.input.start = Start::UserOrigin;
    page.input.user_origin = Some(point);
    page
}

fn text(input: &JobInput) -> String {
    let built = build_job(input, &[], &mut PlanCache::default());
    String::from_utf8(write(input.machine.values.output_format, &built.job)).unwrap()
}

fn golden(sub: &str, name: &str, text: &str) {
    let path = dir(sub).join(name);
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, text).unwrap();
    }
    let stored = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing golden {name}"));
    assert_eq!(text, stored, "{name} differs from its golden");
}

/// The X and Y words of a G0 or G1 line in µm.
fn xy(line: &str) -> Option<(i64, i64)> {
    let word = |letter: char| {
        line.split_whitespace()
            .find_map(|w| w.strip_prefix(letter))
            .and_then(|v| v.parse::<f64>().ok())
    };
    #[allow(clippy::cast_possible_truncation)]
    let um = |mm: f64| (mm * 1000.0).round() as i64;
    Some((um(word('X')?), um(word('Y')?)))
}

fn moves(text: &str) -> Vec<(&str, (i64, i64))> {
    text.lines()
        .filter_map(|l| {
            let kind = l.split_whitespace().next()?;
            matches!(kind, "G0" | "G1").then_some(())?;
            Some((kind, xy(l)?))
        })
        .collect()
}

/// The anchor in machine µm: the min corner of the Machine origin job's burn box.
fn anchor(input: &JobInput) -> PointUm {
    let built = build_job(input, &[], &mut PlanCache::default());
    let ring = frame_outline(&built.job, FrameShape::Rectangle).unwrap();
    PointUm::new(
        ring.iter().map(|p| p.x).min().unwrap(),
        ring.iter().map(|p| p.y).min().unwrap(),
    )
}

const U: PointUm = PointUm::new(150_000, 100_000);

#[test]
fn c31_per_corner_every_burn_line_is_machine_origins_plus_the_offset() {
    for (corner, name) in [
        (OriginCorner::BottomLeft, "start_user_origin_bl.gcode"),
        (OriginCorner::BottomRight, "start_user_origin_br.gcode"),
        (OriginCorner::TopLeft, "start_user_origin_tl.gcode"),
        (OriginCorner::TopRight, "start_user_origin_tr.gcode"),
    ] {
        let base = page(corner);
        let a = anchor(&base.input);
        let shifted = user_origin(page(corner), U);
        let ours = text(&shifted.input);
        golden("grbl", name, &ours);
        assert!(
            !ours.contains("G10") && !ours.contains("G92") && !ours.contains("G54"),
            "{name}"
        );
        assert!(ours.contains("G90\n") && !ours.contains("$H"), "{name}");
        let theirs = text(&base.input);
        let (dx, dy) = (U.x - a.x, U.y - a.y);
        let burns = |t: &str| -> Vec<(i64, i64)> {
            moves(t)
                .into_iter()
                .filter(|(kind, _)| *kind == "G1")
                .map(|(_, p)| p)
                .collect()
        };
        let expected: Vec<_> = burns(&theirs)
            .into_iter()
            .map(|(x, y)| (x + dx, y + dy))
            .collect();
        assert_eq!(burns(&ours), expected, "{name}");
        // The file starts and ends on the user origin: the first move leaves it for the first
        // burn start (no travel when the burn starts on it), the last comes back to it.
        let all = moves(&ours);
        let built = build_job(&shifted.input, &[], &mut PlanCache::default());
        assert_eq!(built.job.machine_point(built.job.home), U, "{name}");
        assert_eq!(all.last().unwrap(), &("G0", (U.x, U.y)), "{name}");
        // The anchor sits on the user origin: no move goes below U on either axis.
        assert!(all.iter().all(|(_, p)| p.0 >= U.x && p.1 >= U.y), "{name}");
    }
}

#[test]
fn c31_the_frame_traces_the_shifted_outline() {
    let base = page(OriginCorner::BottomLeft);
    let a = anchor(&base.input);
    let shifted = user_origin(page(OriginCorner::BottomLeft), U);
    let built = build_job(&shifted.input, &[], &mut PlanCache::default());
    let ring = frame_outline(&built.job, FrameShape::Rectangle).unwrap();
    let plain = build_job(&base.input, &[], &mut PlanCache::default());
    let expected: Vec<_> = frame_outline(&plain.job, FrameShape::Rectangle)
        .unwrap()
        .into_iter()
        .map(|p| PointUm::new(p.x + U.x - a.x, p.y + U.y - a.y))
        .collect();
    assert_eq!(ring, expected);
    let frame = write_frame(
        shifted.input.machine.values.output_format,
        &built.job,
        FrameShape::Rectangle,
    )
    .unwrap();
    let frame = String::from_utf8(frame).unwrap();
    assert!(frame.contains("G90\n") && !frame.contains("G91"));
    golden("grbl_frame", "frame_user_origin.gcode", &frame);
}

#[test]
fn c31_the_timeline_equals_the_files_lengths() {
    let shifted = user_origin(page(OriginCorner::TopRight), U);
    let built = build_job(&shifted.input, &[], &mut PlanCache::default());
    let base = build_job(
        &page(OriginCorner::TopRight).input,
        &[],
        &mut PlanCache::default(),
    );
    let (ours, theirs) = (Timeline::of(&built.job), Timeline::of(&base.job));
    assert!(
        (ours.burn_mm() - theirs.burn_mm()).abs() < 1e-9,
        "the same burns"
    );
}

fn bed_issue(input: &JobInput) -> Option<(Cause, String)> {
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
fn c31_the_work_area_check_measures_the_placed_job_and_names_the_distance() {
    // Inside: the box (100 x 50 mm) at (150, 100) stays in the 400 x 300 mm bed.
    assert_eq!(
        bed_issue(&user_origin(page(OriginCorner::BottomLeft), U).input),
        None
    );
    // The box's right edge lands at 400 + 11.5 mm: blocked, "by 12 mm".
    let far = user_origin(
        page(OriginCorner::BottomLeft),
        PointUm::new(311_500, 100_000),
    );
    let (cause, words) = bed_issue(&far.input).expect("outside the bed");
    assert_eq!(
        cause,
        Cause::OutsideBed {
            width_um: 400_000,
            height_um: 300_000,
            over_um: Some(11_500),
        }
    );
    assert_eq!(words, "1 item lies outside the 400 × 300 mm bed by 12 mm.");
    let near = user_origin(
        page(OriginCorner::BottomLeft),
        PointUm::new(300_350, 100_000),
    );
    let (_, words) = bed_issue(&near.input).expect("outside the bed");
    assert_eq!(words, "1 item lies outside the 400 × 300 mm bed by 0.4 mm.");
    // Machine origin keeps its words.
    let mut base = page(OriginCorner::BottomLeft);
    base.operation(
        Cut,
        "Cut 2",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![rect(390.0, 10.0, 30.0, 20.0)])],
    );
    let (cause, words) = bed_issue(&base.input).expect("outside the bed");
    assert!(matches!(cause, Cause::OutsideBed { over_um: None, .. }));
    assert_eq!(words, "1 item lies outside the 400 × 300 mm bed.");
}

fn pages(name: &str) -> Vec<ManufacturingSnapshot> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name);
    unpack(2, &std::fs::read(path).unwrap())
        .unwrap()
        .manufacturings()
}

#[test]
fn c30_the_golden_reads_user_origin_and_its_point() {
    let pages = pages("user_origin_v61.curvyo");
    let first = read_output_settings(&pages[0]);
    assert_eq!(first.start, Start::UserOrigin);
    assert_eq!(first.user_origin, Some(PointUm::new(123_456, 78_900)));
    assert!(!first.start_damaged);
    assert_eq!(first.damaged, 0);
    let second = read_output_settings(&pages[1]);
    assert_eq!(second.start, Start::MachineOrigin);
    assert_eq!(second.user_origin, Some(PointUm::new(1_000, 2_000)));
    let third = read_output_settings(&pages[2]);
    assert_eq!(third.user_origin, None);
    assert_eq!(third.damaged, 0);
}

#[test]
fn c30_a_damaged_point_or_user_origin_without_one_is_a_damaged_start() {
    for page in pages("user_origin_damaged_v61.curvyo") {
        let settings = read_output_settings(&page);
        assert_eq!(settings.user_origin, None);
        assert!(settings.start_damaged, "never a guessed start");
        assert_eq!(settings.start, Start::MachineOrigin);
        assert_eq!(settings.damaged, 1, "counts one");
    }
}
