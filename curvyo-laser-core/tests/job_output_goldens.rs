//! The GRBL goldens of `specs/0029-laser-job-preview-and-output` criterion 31, built from
//! `job_input` fixtures with no session (ADR 0064 Consequences): each file is
//! `write(GRBL, job)`, built twice and compared byte for byte, and read back by a small reader
//! that checks the view's lengths equal the file's (criterion 9). `CURVYO_UPDATE_GOLDEN=1`
//! rewrites the files; review the diff.

#![allow(
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::many_single_char_names
)]

mod job_fixtures;

use std::path::PathBuf;

use curvyo_document_core::Point;
use curvyo_laser_core::OriginCorner;
use curvyo_laser_core::job::{JobInput, PlanCache, Start, Timeline, build_job, extension, write};
use curvyo_laser_core::library::MaterialRead;
use curvyo_laser_core::operations::OperationKind::{Cut, CutOut, Engrave, EngraveOutline, Score};
use curvyo_laser_core::settings::SetKind;
use job_fixtures::{
    BIRCH, Page, circle, leaf, line, material, options, own, own_engrave, plain, rect,
    rect_reversed, settings,
};
use serde_json::json;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/output/grbl")
}

/// The bytes of `input`, checked for determinism and against the reader.
fn bytes(input: &JobInput, materials: &[MaterialRead]) -> Vec<u8> {
    let built = build_job(input, materials, &mut PlanCache::default());
    let format = input.machine.values.output_format;
    let first = write(format, &built.job);
    let again = write(
        format,
        &build_job(input, materials, &mut PlanCache::default()).job,
    );
    assert_eq!(
        first, again,
        "two exports are byte-identical (criterion 31)"
    );
    assert_eq!(extension(format), "gcode");
    let timeline = Timeline::of(&built.job);
    let read = reader::read(std::str::from_utf8(&first).unwrap());
    assert!(
        (read.burn_mm - timeline.burn_mm()).abs() < 0.01,
        "burn {} vs {}",
        read.burn_mm,
        timeline.burn_mm()
    );
    assert!(
        (read.travel_mm - timeline.travel_mm()).abs() < 0.01,
        "travel {} vs {}",
        read.travel_mm,
        timeline.travel_mm()
    );
    first
}

fn golden(name: &str, input: &JobInput, materials: &[MaterialRead]) -> String {
    let bytes = bytes(input, materials);
    let path = dir().join(name);
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(dir()).unwrap();
        std::fs::write(&path, &bytes).unwrap();
    }
    let stored = std::fs::read(&path).unwrap_or_else(|_| panic!("missing golden {name}"));
    let text = String::from_utf8(bytes).unwrap();
    assert_eq!(
        text,
        String::from_utf8(stored).unwrap(),
        "{name} differs from its golden"
    );
    assert!(text.is_ascii(), "ASCII only");
    assert!(!text.contains('\r'), "LF line ends");
    text
}

fn cut_line_page() -> Page {
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 50.0, 600, 1),
        plain(),
        vec![leaf(vec![line(
            Point::new(20.0, 280.0),
            Point::new(120.0, 280.0),
        )])],
    );
    page
}

#[test]
fn cut_line() {
    let text = golden("cut_line.gcode", &cut_line_page().input, &[]);
    assert!(text.contains("G1 X120.000 Y20.000 F600 S500"), "{text}");
    assert!(
        !text
            .lines()
            .any(|l| l.starts_with("G2 ") || l.starts_with("G3 ")),
        "no arcs"
    );
}

/// `specs/0121-print-and-cut` criterion 17: a 40 mm square on a sheet with marks at its corners,
/// measured on the bed as placed by a turn of 90 degrees and a move to (150, 60) mm; the fit
/// places the square, and the job cuts it where the measures say.
#[test]
fn print_and_cut_cut_line() {
    use curvyo_document_core::{Affine, PrintAndCutMeasures, PrintAndCutRegister};
    use curvyo_laser_core::print_and_cut::{
        FitState, SheetInput, head_position_um, machine_mm, read_print_and_cut,
    };
    let marks = [
        (1, Point::new(10.0, 10.0)),
        (2, Point::new(50.0, 10.0)),
        (3, Point::new(50.0, 50.0)),
        (4, Point::new(10.0, 50.0)),
    ];
    let placed = Affine::new(0.0, 1.0, -1.0, 0.0, 150.0, 60.0);
    let mut page = cut_line_page();
    let frame = curvyo_laser_core::job::Frame {
        corner: page.input.machine.values.origin,
        width: 400_000,
        height: 300_000,
    };
    let measured = marks
        .iter()
        .map(|&(n, c)| {
            let m = machine_mm(frame, placed.apply_point(c));
            (n, head_position_um(m.x, m.y))
        })
        .collect();
    let register = PrintAndCutRegister::Measures(PrintAndCutMeasures {
        marks: measured,
        ..PrintAndCutMeasures::default()
    });
    let reading = read_print_and_cut(&SheetInput {
        marks: &marks,
        register: &register,
        frame,
        matrix: Affine::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
        sheet: (100.0, 100.0),
        start: Start::default(),
    });
    let FitState::Fit(fit) = reading.fit else {
        panic!("a fit");
    };
    assert!(fit.largest_mm < 0.001);
    let square = rect(10.0, 10.0, 40.0, 40.0);
    let mapped = curvyo_laser_core::job::LeafOutline {
        anchors: square
            .anchors
            .iter()
            .map(|&(p, i, o)| (fit.map.apply_point(p), i, o))
            .collect(),
        closed: true,
    };
    page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 50.0, 600, 1),
        plain(),
        vec![leaf(vec![mapped])],
    );
    let text = golden("print_and_cut_cut_line.gcode", &page.input, &[]);
    // The square's first corner (10, 10) lands at page (140, 70): machine (140, 230).
    assert!(text.contains("X140.000 Y230.000"), "{text}");
}

/// A page with a Score rectangle, an Engrave square and a Cut rectangle.
fn shapes(page: &mut Page) {
    page.operation(
        Score,
        "Score 1",
        own(Score, 20.0, 3000, 1),
        plain(),
        vec![leaf(vec![rect(40.0, 40.0, 30.0, 20.0)])],
    );
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![rect(30.0, 30.0, 50.0, 40.0)])],
    );
}

#[test]
fn score_and_cut_order() {
    let mut page = Page::new();
    shapes(&mut page);
    let text = golden("score_and_cut_order.gcode", &page.input, &[]);
    assert!(text.find("S200").unwrap() < text.find("S900").unwrap());
}

#[test]
fn engrave_bidirectional() {
    let mut page = Page::new();
    page.operation(
        Engrave,
        "Engrave 1",
        own_engrave(Engrave, 30.0, 6000, 1.0),
        plain(),
        vec![leaf(vec![rect(10.0, 10.0, 20.0, 6.0)])],
    );
    golden("engrave_bidirectional.gcode", &page.input, &[]);
}

#[test]
fn engrave_crosshatch_scan_angle() {
    let mut page = Page::new();
    page.operation(
        Engrave,
        "Engrave 1",
        own_engrave(Engrave, 30.0, 6000, 1.0),
        options(true, 30.0, true, 0, 0.0),
        vec![leaf(vec![rect(10.0, 10.0, 6.0, 6.0)])],
    );
    golden("engrave_crosshatch_scan_angle.gcode", &page.input, &[]);
}

#[test]
fn engrave_overscan() {
    let mut page = Page::new();
    page.operation(
        Engrave,
        "Engrave 1",
        own_engrave(Engrave, 30.0, 6000, 1.0),
        options(true, 0.0, false, 50, 0.0),
        vec![leaf(vec![rect(10.0, 10.0, 20.0, 3.0)])],
    );
    let text = golden("engrave_overscan.gcode", &page.input, &[]);
    // 6,000 mm/min at 50 %: 2.5 mm run-out, laser off (S0).
    assert!(text.contains("G1 X32.500 Y287.500 S0"), "{text}");
}

#[test]
fn engrave_outline_set() {
    let mut page = Page::new();
    let read = settings(
        EngraveOutline,
        json!({"main": {"power_pct": 30.0, "speed_mm_min": 6000, "passes": 1, "interval_mm": 1.0},
               "outline": {"power_pct": 15.0, "speed_mm_min": 3000, "passes": 1}}),
    );
    page.operation(
        EngraveOutline,
        "Engrave + Outline 1",
        read,
        plain(),
        vec![leaf(vec![
            rect(10.0, 10.0, 10.0, 6.0),
            rect_reversed(13.0, 12.0, 4.0, 2.0),
        ])],
    );
    let text = golden("engrave_outline_set.gcode", &page.input, &[]);
    assert!(
        text.rfind("S300").unwrap() < text.find("S150").unwrap(),
        "hatch, then outline"
    );
}

#[test]
fn cutout_margin() {
    let mut page = Page::new();
    page.cut_out(
        "Cut-out 1",
        own(CutOut, 90.0, 300, 1),
        2.0,
        vec![vec![
            vec![rect(20.0, 20.0, 20.0, 10.0)],
            vec![rect(35.0, 25.0, 20.0, 10.0)],
        ]],
    );
    golden("cutout_margin.gcode", &page.input, &[]);
}

#[test]
fn inner_first() {
    // A rectangle with a circular hole and a text-like square inside the hole (criterion 22).
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![
            leaf(vec![rect(10.0, 10.0, 60.0, 60.0), circle(40.0, 40.0, 15.0)]),
            leaf(vec![rect(37.0, 37.0, 6.0, 6.0)]),
        ],
    );
    let text = golden("inner_first.gcode", &page.input, &[]);
    let first_burn = text.lines().find(|l| l.starts_with("G1 ")).unwrap();
    assert!(
        first_burn.contains("X37.000") || first_burn.contains("X43.000"),
        "{first_burn}"
    );
}

#[test]
fn passes() {
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 3),
        plain(),
        vec![
            leaf(vec![rect(10.0, 10.0, 10.0, 10.0)]),
            leaf(vec![rect(30.0, 10.0, 10.0, 10.0)]),
        ],
    );
    let text = golden("passes.gcode", &page.input, &[]);
    assert_eq!(
        text.matches("G0 ").count(),
        7,
        "1 + 2 per pass x 3 passes - 1 + return"
    );
}

#[test]
fn m3_constant_power() {
    let mut page = Page::new();
    page.values().laser_mode = false;
    shapes(&mut page);
    let text = golden("m3_constant_power.gcode", &page.input, &[]);
    assert!(text.contains("M3 S0") && text.contains("M3 S200") && !text.contains("M4"));
}

#[test]
fn z_offset() {
    let mut page = Page::new();
    page.values().motorised_z = true;
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 2),
        options(true, 0.0, false, 0, -1.5),
        vec![leaf(vec![rect(10.0, 10.0, 10.0, 10.0)])],
    );
    let text = golden("z_offset.gcode", &page.input, &[]);
    assert_eq!(
        text.matches(" Z").count(),
        2,
        "exactly two Z moves (criterion 19)"
    );
    assert!(text.contains("G1 Z-1.500 F300\n") && text.contains("G1 Z0.000 F300 S0"));
}

fn corner_page(corner: OriginCorner) -> Page {
    let mut page = Page::new().corner(corner);
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![line(
            Point::new(10.0, 20.0),
            Point::new(60.0, 40.0),
        )])],
    );
    page
}

#[test]
fn origin_corners() {
    for (corner, name) in [
        (OriginCorner::BottomLeft, "origin_corners_bl.gcode"),
        (OriginCorner::BottomRight, "origin_corners_br.gcode"),
        (OriginCorner::TopLeft, "origin_corners_tl.gcode"),
        (OriginCorner::TopRight, "origin_corners_tr.gcode"),
    ] {
        let text = golden(name, &corner_page(corner).input, &[]);
        assert!(!text.contains("G10") && !text.contains("G92") && !text.contains("G54"));
        assert!(
            !text.contains("X-") && !text.contains("Y-"),
            "inside the bed: {name}"
        );
    }
}

#[test]
fn fixture_jobs_first() {
    let mut page = Page::new();
    page.input.fixture_material = Some(BIRCH.to_owned());
    page.fixture(Score, vec![leaf(vec![rect(5.0, 5.0, 10.0, 10.0)])]);
    page.operation(
        Engrave,
        "Engrave 1",
        own_engrave(Engrave, 30.0, 6000, 1.0),
        plain(),
        vec![leaf(vec![rect(30.0, 10.0, 6.0, 3.0)])],
    );
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![rect(50.0, 10.0, 10.0, 10.0)])],
    );
    let birch = material(BIRCH, "Birch 3 mm", &[(SetKind::Score, 12.0, 4000, 1)]);
    let text = golden("fixture_jobs_first.gcode", &page.input, &[birch]);
    let (score, engrave, cut) = (
        text.find("S120").unwrap(),
        text.find("S300").unwrap(),
        text.find("S900").unwrap(),
    );
    assert!(score < engrave && engrave < cut, "criterion 20");
}

#[test]
fn home_first() {
    // Superseded by 0029 rework (2026-11-17): the start is the one value Home, then machine
    // origin (was Head position with Home first on); the golden keeps its bytes.
    let page = cut_line_page().start(Start::HomeThenOrigin);
    let text = golden("home_first.gcode", &page.input, &[]);
    let body: Vec<&str> = text.lines().filter(|l| !l.starts_with(';')).collect();
    assert_eq!(
        &body[..3],
        ["$H", "G21", "G90"],
        "Home, then machine origin runs from the machine origin"
    );
    let base = String::from_utf8(bytes(&cut_line_page().input, &[])).unwrap();
    let without_home: String = text.replacen("$H\n", "", 1);
    assert_eq!(without_home, base, "then exactly the Machine origin file");
}

#[test]
fn start_head_position() {
    // Superseded by 0029 rework (2026-11-17): the start is the one value Head position.
    let page = cut_line_page().start(Start::HeadPosition);
    let text = golden("start_head_position.gcode", &page.input, &[]);
    assert!(text.contains("G91") && !text.contains("G90"));
    let base = String::from_utf8(bytes(&cut_line_page().input, &[])).unwrap();
    assert_eq!(
        base,
        std::fs::read_to_string(dir().join("cut_line.gcode")).unwrap()
    );
}

#[test]
fn air_assist() {
    let mut page = cut_line_page();
    page.values().air_assist = true;
    let text = golden("air_assist.gcode", &page.input, &[]);
    let lines: Vec<&str> = text.lines().collect();
    let m8 = lines.iter().position(|l| *l == "M8").unwrap();
    let first_burn = lines.iter().position(|l| l.starts_with("G1 ")).unwrap();
    assert_eq!(
        m8 + 1,
        first_burn,
        "air on right before the first burn move"
    );
    assert_eq!(&lines[lines.len() - 3..lines.len() - 1], ["M5", "M9"]);
}

#[test]
fn every_golden_of_criterion_31_exists() {
    for name in [
        "cut_line",
        "score_and_cut_order",
        "engrave_bidirectional",
        "engrave_crosshatch_scan_angle",
        "engrave_overscan",
        "engrave_outline_set",
        "cutout_margin",
        "inner_first",
        "passes",
        "m3_constant_power",
        "z_offset",
        "origin_corners_bl",
        "origin_corners_br",
        "origin_corners_tl",
        "origin_corners_tr",
        "fixture_jobs_first",
        "home_first",
        "start_head_position",
        "air_assist",
    ] {
        assert!(dir().join(format!("{name}.gcode")).is_file(), "{name}");
    }
}

/// A test-only GRBL reader (the implementer's; the tester writes an independent one): burn and
/// travel lengths in mm, absolute or relative, M3 or M4.
mod reader {
    pub struct Read {
        pub burn_mm: f64,
        pub travel_mm: f64,
    }

    fn word(line: &str, letter: char) -> Option<f64> {
        line.split_whitespace()
            .find_map(|w| w.strip_prefix(letter).and_then(|v| v.parse().ok()))
    }

    pub fn read(text: &str) -> Read {
        let (mut x, mut y, mut s) = (0.0_f64, 0.0_f64, 0.0_f64);
        let (mut relative, mut m3, mut on) = (false, false, true);
        let mut out = Read {
            burn_mm: 0.0,
            travel_mm: 0.0,
        };
        for line in text.lines().filter(|l| !l.starts_with(';')) {
            match line.split_whitespace().next() {
                Some("G91") => relative = true,
                Some("G90") => relative = false,
                Some("M3") => {
                    m3 = true;
                    on = true;
                    if let Some(v) = word(line, 'S') {
                        s = v;
                    }
                }
                Some("M5") => on = !m3 && on,
                Some(g @ ("G0" | "G1")) => {
                    if let Some(v) = word(line, 'S') {
                        s = v;
                    }
                    if word(line, 'Z').is_some() {
                        continue;
                    }
                    let (nx, ny) = match (word(line, 'X'), word(line, 'Y')) {
                        (Some(a), Some(b)) if relative => (x + a, y + b),
                        (Some(a), Some(b)) => (a, b),
                        _ => continue,
                    };
                    let length = (nx - x).hypot(ny - y);
                    if g == "G1" && s > 0.0 && on {
                        out.burn_mm += length;
                    } else {
                        out.travel_mm += length;
                    }
                    (x, y) = (nx, ny);
                }
                _ => {}
            }
        }
        out
    }
}
