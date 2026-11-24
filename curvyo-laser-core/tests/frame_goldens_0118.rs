//! The frame text of `specs/0118-machine-control` slice 2b, criteria 22 to 25 and 28: goldens per
//! shape, start pick and origin corner, and the laser-off rules read from the bytes.
//! `CURVYO_UPDATE_GOLDEN=1` rewrites the files; review the diff.

#![allow(clippy::unwrap_used, clippy::missing_panics_doc)]

mod job_fixtures;

use std::path::PathBuf;

use curvyo_laser_core::OriginCorner;
use curvyo_laser_core::job::{
    FrameShape, JobInput, PlanCache, Start, build_frame_job, build_job, frame_outline, write_frame,
};
use curvyo_laser_core::operations::OperationKind::Cut;
use curvyo_laser_core::settings::SettingsRead;
use job_fixtures::{Page, leaf, own, plain, rect};

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/output/grbl_frame")
}

/// Two rectangles apart, so the hull and the box differ.
fn two_rects(corner: OriginCorner, start: Start) -> Page {
    two_rects_with(corner, start, own(Cut, 90.0, 300, 1))
}

/// [`two_rects`] with the Cut's settings `numbers`.
fn two_rects_with(corner: OriginCorner, start: Start, numbers: SettingsRead) -> Page {
    let mut page = Page::new().corner(corner).start(start);
    page.operation(
        Cut,
        "Cut 1",
        numbers,
        plain(),
        vec![leaf(vec![
            rect(20.0, 20.0, 30.0, 20.0),
            rect(100.0, 60.0, 40.0, 30.0),
        ])],
    );
    page
}

fn frame_text(input: &JobInput, shape: FrameShape) -> String {
    let built = build_frame_job(input, &[], &mut PlanCache::default());
    let bytes = write_frame(input.machine.values.output_format, &built.job, shape).unwrap();
    String::from_utf8(bytes).unwrap()
}

fn golden(name: &str, input: &JobInput, shape: FrameShape) -> String {
    let text = frame_text(input, shape);
    assert_eq!(text, frame_text(input, shape), "two exports are identical");
    let path = dir().join(name);
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let stored = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing golden {name}"));
    assert_eq!(text, stored, "{name} differs from its golden");
    assert!(
        text.is_ascii() && !text.contains('\r'),
        "ASCII, LF line ends"
    );
    laser_stays_off(&text);
    text
}

/// Criterion 25: `M5` first, only `G1 ... S0`, no `G0`, `M3`, `M4`, `$H`, no `S` above 0.
fn laser_stays_off(text: &str) {
    let code: Vec<&str> = text.lines().filter(|l| !l.starts_with(';')).collect();
    let m5 = code.iter().position(|l| *l == "M5").unwrap();
    let first_move = code.iter().position(|l| l.starts_with("G1 ")).unwrap();
    assert!(m5 < first_move, "M5 before the first move: {text}");
    for line in &code {
        assert!(!line.starts_with("G0"), "{line}");
        assert!(!line.contains("M3") && !line.contains("M4"), "{line}");
        assert!(!line.contains('$'), "{line}");
        if line.starts_with("G1 ") {
            assert!(line.ends_with(" S0") && line.contains(" F"), "{line}");
        }
    }
}

#[test]
fn rectangle_starts_at_the_corner_nearest_the_origin() {
    let page = two_rects(OriginCorner::BottomLeft, Start::MachineOrigin);
    let text = golden("frame_rectangle.gcode", &page.input, FrameShape::Rectangle);
    // The page box 20..140 x 20..90 on a 300 mm tall page: machine y 210..280.
    let moves: Vec<&str> = text.lines().filter(|l| l.starts_with("G1 ")).collect();
    assert_eq!(moves.len(), 5, "four corners and back to the first");
    assert_eq!(moves[0], "G1 X20.000 Y210.000 F3000 S0");
    assert_eq!(moves[1], "G1 X140.000 Y210.000 F3000 S0", "X first");
    assert_eq!(moves[4], moves[0]);
}

#[test]
fn rubber_band_is_the_convex_hull() {
    let page = two_rects(OriginCorner::BottomLeft, Start::MachineOrigin);
    let text = golden(
        "frame_rubber_band.gcode",
        &page.input,
        FrameShape::RubberBand,
    );
    let moves = text.lines().filter(|l| l.starts_with("G1 ")).count();
    assert_eq!(moves, 7, "a hexagon and back to the first");
}

#[test]
fn head_position_is_relative_and_returns_to_the_start() {
    let page = two_rects(OriginCorner::BottomLeft, Start::HeadPosition);
    let text = golden(
        "frame_head_position.gcode",
        &page.input,
        FrameShape::Rectangle,
    );
    assert!(text.lines().any(|l| l == "G91"), "{text}");
    assert!(!text.lines().any(|l| l == "G90"), "{text}");
    // The relative moves of a closed path and the way back to the start add up to nothing.
    let (mut x, mut y) = (0.0_f64, 0.0_f64);
    for line in text.lines().filter(|l| l.starts_with("G1 ")) {
        let word = |c: char| -> f64 {
            line.split(' ')
                .find_map(|w| w.strip_prefix(c))
                .unwrap()
                .parse()
                .unwrap()
        };
        x += word('X');
        y += word('Y');
    }
    assert!(x.abs() < 1e-6 && y.abs() < 1e-6, "ends where it began");
}

#[test]
fn home_then_origin_frames_as_machine_origin_and_never_homes() {
    let a = two_rects(OriginCorner::BottomLeft, Start::HomeThenOrigin);
    let b = two_rects(OriginCorner::BottomLeft, Start::MachineOrigin);
    let text = frame_text(&a.input, FrameShape::Rectangle);
    assert_eq!(text, frame_text(&b.input, FrameShape::Rectangle));
    assert!(!text.contains("$H"));
}

#[test]
fn every_origin_corner_has_its_golden() {
    for (corner, name, first) in [
        (
            OriginCorner::BottomLeft,
            "frame_corners_bl.gcode",
            "G1 X20.000 Y210.000 F3000 S0",
        ),
        (
            OriginCorner::BottomRight,
            "frame_corners_br.gcode",
            "G1 X260.000 Y210.000 F3000 S0",
        ),
        (
            OriginCorner::TopLeft,
            "frame_corners_tl.gcode",
            "G1 X20.000 Y20.000 F3000 S0",
        ),
        (
            OriginCorner::TopRight,
            "frame_corners_tr.gcode",
            "G1 X260.000 Y20.000 F3000 S0",
        ),
    ] {
        let page = two_rects(corner, Start::MachineOrigin);
        let text = golden(name, &page.input, FrameShape::Rectangle);
        let first_move = text.lines().find(|l| l.starts_with("G1 ")).unwrap();
        assert_eq!(first_move, first, "{name}");
    }
}

#[test]
fn the_feed_is_capped_by_the_machine() {
    let mut page = two_rects(OriginCorner::BottomLeft, Start::MachineOrigin);
    page.values().max_feed = curvyo_laser_core::FeedRate::new(1_200).unwrap();
    let text = frame_text(&page.input, FrameShape::Rectangle);
    assert!(
        text.contains(" F1200 S0") && !text.contains("F3000"),
        "{text}"
    );
}

#[test]
fn a_job_without_a_burn_has_nothing_to_frame() {
    let page = Page::new();
    let built = build_frame_job(&page.input, &[], &mut PlanCache::default());
    assert!(frame_outline(&built.job, FrameShape::Rectangle).is_none());
    assert!(
        write_frame(
            page.input.machine.values.output_format,
            &built.job,
            FrameShape::RubberBand
        )
        .is_none()
    );
}

#[test]
fn frage_335_a_cut_without_power_or_speed_frames_its_geometry() {
    // A Cut without `op_settings` reads own numbers from the copy's pair (`specs/0150`
    // criterion 25): the job runs, and its frame is the outline it had as "No settings".
    let page = two_rects_with(OriginCorner::BottomLeft, Start::MachineOrigin, absent());
    let built = build_job(&page.input, &[], &mut PlanCache::default());
    assert!(built.job.burns(), "the derived numbers run");
    assert!(!built.unpriced());
    for (shape, name) in [
        (FrameShape::Rectangle, "frame_rectangle.gcode"),
        (FrameShape::RubberBand, "frame_rubber_band.gcode"),
    ] {
        let text = frame_text(&page.input, shape);
        let stored = std::fs::read_to_string(dir().join(name)).unwrap();
        assert_eq!(text, stored, "{name}: the same frame as with numbers");
        laser_stays_off(&text);
        assert!(text.contains(" F3000 S0"), "the frame feed: {text}");
    }
}

#[test]
fn frage_335_head_position_without_numbers_frames_as_with_them() {
    let page = two_rects_with(OriginCorner::BottomLeft, Start::HeadPosition, absent());
    let text = frame_text(&page.input, FrameShape::Rectangle);
    let stored = std::fs::read_to_string(dir().join("frame_head_position.gcode")).unwrap();
    assert_eq!(text, stored);
}

/// What a Cut without `op_settings` reads (`specs/0150` criterion 15).
fn absent() -> SettingsRead {
    curvyo_laser_core::settings::read_settings(
        None,
        curvyo_laser_core::operations::OperationKind::Cut,
        curvyo_laser_core::DefaultNumbers::BUILT_IN,
    )
}
