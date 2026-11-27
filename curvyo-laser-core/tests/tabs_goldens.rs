//! The tab goldens of `specs/0139-laser-cut-path-options` slice 1 (criteria 2 to 10): each file
//! is `write(GRBL, job)`, built twice and compared byte for byte, and read back by a reader of its
//! own that sums the burnt pieces and the gaps between them. `CURVYO_UPDATE_GOLDEN=1` rewrites
//! the files; review the diff.

#![allow(clippy::unwrap_used, clippy::missing_panics_doc)]

mod job_fixtures;

use std::path::PathBuf;

use curvyo_document_core::{Point, Vec2};
use curvyo_laser_core::job::{
    Cause, GateFacts, JobInput, LeafOutline, PlanCache, Timeline, build_job, gate, write,
};
use curvyo_laser_core::operations::OperationKind::{Cut, CutOut};
use curvyo_laser_core::settings::{CutLength, CutSettings, OperationOptions, TabCount};
use curvyo_laser_core::tabs::{LeafTabs, OwnTab, PathFraction};
use job_fixtures::{Page, circle, leaf, line, own, plain, rect};

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/output/grbl")
}

fn tabs(count: u64, width: f64, skip_inner: bool) -> OperationOptions {
    OperationOptions {
        cut: CutSettings {
            tab_count: TabCount::new(count).unwrap(),
            tab_width: CutLength::tab_of_mm(width).unwrap(),
            tab_skip_inner: skip_inner,
        },
        ..plain()
    }
}

/// A point in machine mm.
type At = (f64, f64);

/// One piece or gap read back from the file.
#[derive(Debug, Default)]
struct Read {
    /// The burnt runs: their lengths, and their first and last points.
    pieces: Vec<(f64, At, At)>,
    /// The straight travel between two burnt runs.
    gaps: Vec<f64>,
    burn: f64,
    travel: f64,
    /// Whether a G0 named an S above 0 (M4) or ran without an M5 before it (M3).
    hot_travel: bool,
}

fn word(line: &str, letter: char) -> Option<f64> {
    line.split_whitespace()
        .find_map(|w| w.strip_prefix(letter).and_then(|v| v.parse().ok()))
}

/// A reader independent of the generator: G0 is travel, G1 with S above 0 and the laser on burns.
fn read(text: &str) -> Read {
    let (mut x, mut y, mut s) = (0.0_f64, 0.0_f64, 0.0_f64);
    let (mut m3, mut on) = (false, true);
    let mut out = Read::default();
    let mut run: Option<(f64, (f64, f64))> = None;
    let mut travel_since = None::<f64>;
    for line in text.lines().filter(|l| !l.starts_with(';')) {
        match line.split_whitespace().next() {
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
                let (Some(nx), Some(ny)) = (word(line, 'X'), word(line, 'Y')) else {
                    continue;
                };
                let length = (nx - x).hypot(ny - y);
                if g == "G1" && s > 0.0 && on {
                    out.burn += length;
                    if let Some(t) = travel_since.take()
                        && run.is_none()
                        && !out.pieces.is_empty()
                    {
                        out.gaps.push(t);
                    }
                    let (sum, start) = run.unwrap_or((0.0, (x, y)));
                    run = Some((sum + length, start));
                } else {
                    // M4: a gap's G0 names no S above 0; M3: an M5 comes before it.
                    let s_word = word(line, 'S').unwrap_or(0.0);
                    if g == "G0" && ((!m3 && s_word > 0.0) || (m3 && on)) {
                        out.hot_travel = true;
                    }
                    out.travel += length;
                    if let Some((sum, start)) = run.take() {
                        out.pieces.push((sum, start, (x, y)));
                        travel_since = Some(0.0);
                    }
                    if let Some(t) = travel_since.as_mut() {
                        *t += length;
                    }
                }
                (x, y) = (nx, ny);
            }
            _ => {}
        }
    }
    if let Some((sum, start)) = run {
        out.pieces.push((sum, start, (x, y)));
    }
    out
}

/// The golden `name` of `input` with its read-back, checked against the Timeline.
fn golden(name: &str, input: &JobInput) -> (String, Read) {
    let built = build_job(input, &[], &mut PlanCache::default());
    let format = input.machine.values.output_format;
    let bytes = write(format, &built.job);
    let again = write(
        format,
        &build_job(input, &[], &mut PlanCache::default()).job,
    );
    assert_eq!(bytes, again, "two exports are byte-identical");
    let path = dir().join(name);
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &bytes).unwrap();
    }
    let stored = std::fs::read(&path).unwrap_or_else(|_| panic!("missing golden {name}"));
    let text = String::from_utf8(bytes).unwrap();
    assert_eq!(text, String::from_utf8(stored).unwrap(), "{name} differs");
    let back = read(&text);
    let timeline = Timeline::of(&built.job);
    assert!((back.burn - timeline.burn_mm()).abs() < 0.01, "{name} burn");
    assert!(
        (back.travel - timeline.travel_mm()).abs() < 0.01,
        "{name} travel"
    );
    assert!(!back.hot_travel, "{name}: no laser in a gap (criterion 8)");
    (text, back)
}

fn cut_page(options: OperationOptions, outlines: Vec<Vec<LeafOutline>>, passes: u8) -> Page {
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 80.0, 600, passes),
        options,
        outlines.into_iter().map(leaf).collect(),
    );
    page
}

fn corner(x: f64, y: f64) -> (Point, Vec2, Vec2) {
    (Point::new(x, y), Vec2::ZERO, Vec2::ZERO)
}

/// A 40 x 40 mm square drawn from its bottom-left corner.
fn square_from_bottom_left(x: f64, y: f64) -> LeafOutline {
    LeafOutline {
        anchors: vec![
            corner(x, y + 40.0),
            corner(x + 40.0, y + 40.0),
            corner(x + 40.0, y),
            corner(x, y),
        ],
        closed: true,
    }
}

fn own_tab(fraction: f64, width: Option<f64>) -> OwnTab {
    OwnTab {
        at: PathFraction::of(fraction),
        width: width.map(|w| CutLength::tab_of_mm(w).unwrap()),
    }
}

/// Criterion 2: four gaps on the four side midpoints, each 2 mm.
#[test]
fn tabs_square_even() {
    let page = cut_page(
        tabs(4, 2.0, false),
        vec![vec![square_from_bottom_left(10.0, 10.0)]],
        1,
    );
    let (_, back) = golden("tabs_square_even.gcode", &page.input);
    assert!((back.burn - 152.0).abs() < 0.015, "burn {}", back.burn);
    assert_eq!(back.gaps.len(), 4);
    for gap in &back.gaps {
        assert!((gap - 2.0).abs() < 0.005, "gap {gap}");
    }
    // Machine Y = 300 - page Y: the bottom side lies at page Y 50, machine Y 250.
    let mids = [(30.0, 250.0), (50.0, 270.0), (30.0, 290.0), (10.0, 270.0)];
    for (piece, next) in back.pieces.iter().zip(back.pieces.iter().skip(1)) {
        let centre = (
            f64::midpoint(piece.2.0, next.1.0),
            f64::midpoint(piece.2.1, next.1.1),
        );
        assert!(
            mids.iter()
                .any(|m| (m.0 - centre.0).abs() < 0.01 && (m.1 - centre.1).abs() < 0.01),
            "gap centre {centre:?}"
        );
    }
}

/// Criterion 8: a 100 mm line with three 2 mm tabs burns 94 mm with 6 mm of gaps; Count 1 sits
/// at 50 mm.
#[test]
fn tabs_open_line() {
    let a_line = || vec![line(Point::new(20.0, 280.0), Point::new(120.0, 280.0))];
    let plain_page = cut_page(plain(), vec![a_line()], 1);
    let page = cut_page(tabs(3, 2.0, false), vec![a_line()], 1);
    let (_, back) = golden("tabs_open_line.gcode", &page.input);
    assert!((back.burn - 94.0).abs() < 0.015, "burn {}", back.burn);
    let gaps: f64 = back.gaps.iter().sum();
    assert!((gaps - 6.0).abs() < 0.015, "gaps {gaps}");
    let with = Timeline::of(&build_job(&page.input, &[], &mut PlanCache::default()).job);
    let without = Timeline::of(&build_job(&plain_page.input, &[], &mut PlanCache::default()).job);
    assert!((without.burn_mm() - with.burn_mm() - 6.0).abs() < 0.015);
    assert!((with.travel_mm() - without.travel_mm() - 6.0).abs() < 0.015);
    let one = cut_page(tabs(1, 2.0, false), vec![a_line()], 1);
    let built = build_job(&one.input, &[], &mut PlanCache::default());
    let back = read(
        &String::from_utf8(write(one.input.machine.values.output_format, &built.job)).unwrap(),
    );
    assert_eq!(back.pieces.len(), 2);
    let centre = f64::midpoint(back.pieces[0].2.0, back.pieces[1].1.0);
    assert!(
        (centre - 70.0).abs() < 0.01,
        "centre at 50 mm along the line: {centre}"
    );
}

/// Criteria 2 and 8: a 10 mm circle, Width 2.5 mm, Count 20 places 3 tabs; the gaps are measured
/// by arc length, so the burn is the flattened length less 7.5 mm.
#[test]
fn tabs_circle_arc_length() {
    let whole = cut_page(plain(), vec![vec![circle(50.0, 50.0, 5.0)]], 1);
    let page = cut_page(tabs(20, 2.5, false), vec![vec![circle(50.0, 50.0, 5.0)]], 1);
    let (_, back) = golden("tabs_circle_arc_length.gcode", &page.input);
    assert_eq!(
        back.pieces.len(),
        4,
        "three gaps, the entry piece split in two"
    );
    let full = Timeline::of(&build_job(&whole.input, &[], &mut PlanCache::default()).job).burn_mm();
    assert!(
        (full - back.burn - 7.5).abs() < 0.015,
        "{full} vs {}",
        back.burn
    );
}

/// Criterion 3: a plate with a round hole, Count 4, Skip on: four gaps on the outline and none on
/// the hole; off: four on each.
#[test]
fn tabs_skip_inner() {
    let plate = || vec![rect(10.0, 10.0, 60.0, 60.0), circle(40.0, 40.0, 10.0)];
    let page = cut_page(tabs(4, 1.0, true), vec![plate()], 1);
    let (_, back) = golden("tabs_skip_inner.gcode", &page.input);
    assert_eq!(
        back.gaps.len(),
        5,
        "the plate entered at a burnt corner: four gaps and the travel from the hole"
    );
    let off = cut_page(tabs(4, 1.0, false), vec![plate()], 1);
    let built = build_job(&off.input, &[], &mut PlanCache::default());
    let plan = built.plans[0].as_ref().unwrap();
    assert_eq!(plan.marks.len(), 2);
    assert!(plan.marks.iter().all(|m| m.gaps.len() == 4));
    let skip = build_job(&page.input, &[], &mut PlanCache::default());
    let marks = &skip.plans[0].as_ref().unwrap().marks;
    assert_eq!(marks.len(), 1, "the hole gets none: {marks:?}");
}

/// Criteria 5, 6: own tabs and an own width replace the operation's on that item only.
#[test]
fn tabs_own_item() {
    let mut page = cut_page(
        tabs(4, 1.0, false),
        vec![
            vec![rect(10.0, 10.0, 40.0, 40.0)],
            vec![rect(60.0, 10.0, 40.0, 40.0)],
        ],
        1,
    );
    page.input.blocks[0].leaves[0].cut =
        LeafTabs::Own(vec![(0, vec![own_tab(0.1, Some(3.0)), own_tab(0.6, None)])]);
    let (_, back) = golden("tabs_own_item.gcode", &page.input);
    let gaps: f64 = back.gaps.iter().filter(|g| **g < 5.0).sum();
    assert!((gaps - (3.0 + 1.0 + 4.0 * 1.0)).abs() < 0.02, "gaps {gaps}");
    // Back to the operation's: the output is the operation's byte for byte.
    let mut back_page = page;
    back_page.input.blocks[0].leaves[0].cut = LeafTabs::Operation;
    let plain = cut_page(
        tabs(4, 1.0, false),
        vec![
            vec![rect(10.0, 10.0, 40.0, 40.0)],
            vec![rect(60.0, 10.0, 40.0, 40.0)],
        ],
        1,
    );
    let bytes = |input: &JobInput| {
        write(
            input.machine.values.output_format,
            &build_job(input, &[], &mut PlanCache::default()).job,
        )
    };
    // The fixture ids differ only by the order they were taken, which the file never names.
    assert_eq!(bytes(&back_page.input), bytes(&plain.input));
}

/// Criterion 9: every pass writes the same gaps at the same coordinates.
#[test]
fn tabs_passes() {
    let page = cut_page(
        tabs(2, 2.0, false),
        vec![vec![rect(10.0, 10.0, 20.0, 20.0)]],
        2,
    );
    let (text, back) = golden("tabs_passes.gcode", &page.input);
    let g0: Vec<&str> = text.lines().filter(|l| l.starts_with("G0 ")).collect();
    assert_eq!(
        back.pieces.len(),
        5,
        "entered at a burnt corner: three pieces per pass, the second pass joining the first"
    );
    assert_eq!(
        g0[1], g0[3],
        "the first gap is at the same place in both passes"
    );
}

/// Criterion 4: a Cut-out item's tabs sit on its contour.
#[test]
fn tabs_cutout() {
    let mut page = Page::new();
    page.cut_out(
        "Cut-out 1",
        own(CutOut, 80.0, 600, 1),
        0.0,
        vec![vec![vec![rect(10.0, 10.0, 30.0, 30.0)]]],
    );
    if let curvyo_laser_core::job::BlockSource::Operation {
        options: Ok(options),
        ..
    } = &mut page.input.blocks[0].source
    {
        options.cut = tabs(3, 1.5, false).cut;
    }
    let (_, back) = golden("tabs_cutout.gcode", &page.input);
    assert!(
        (back.burn - (120.0 - 4.5)).abs() < 0.02,
        "burn {}",
        back.burn
    );
}

/// Criterion 8: with M3 every gap is `M5`, the travel, `M3 S<n>`.
#[test]
fn tabs_m3_constant_power() {
    let mut page = cut_page(
        tabs(2, 2.0, false),
        vec![vec![rect(10.0, 10.0, 20.0, 20.0)]],
        1,
    );
    page.values().laser_mode = false;
    let (text, back) = golden("tabs_m3_constant_power.gcode", &page.input);
    assert_eq!(back.pieces.len(), 3);
    let lines: Vec<&str> = text.lines().collect();
    for (i, l) in lines.iter().enumerate().skip(1) {
        if l.starts_with("G0 ") && lines[i - 1].starts_with("G1 ") {
            assert_eq!(
                lines[..i].iter().rev().find(|l| l.starts_with('M')),
                Some(&"M5")
            );
        }
    }
    assert!(!text.contains("G4"), "no dwell");
}

/// Criterion 10: own tabs that cover a path cut nothing; the job still runs the other path and
/// the Checks warn once.
#[test]
fn tabs_cover_path() {
    let mut page = cut_page(
        plain(),
        vec![
            vec![line(Point::new(10.0, 10.0), Point::new(13.0, 10.0))],
            vec![rect(30.0, 30.0, 10.0, 10.0)],
        ],
        1,
    );
    page.input.blocks[0].leaves[0].cut = LeafTabs::Own(vec![(0, vec![own_tab(0.5, Some(3.0))])]);
    let covered = page.input.blocks[0].leaves[0].node;
    let (_, back) = golden("tabs_cover_path.gcode", &page.input);
    assert!((back.burn - 40.0).abs() < 0.01);
    let built = build_job(&page.input, &[], &mut PlanCache::default());
    let timeline = Timeline::of(&built.job);
    let checks = gate(&page.input, &built, &timeline, &GateFacts::default());
    let issue = checks
        .issues
        .iter()
        .find(|i| matches!(i.cause, Cause::CoveredByTabs { .. }))
        .unwrap();
    assert_eq!(issue.count, 1);
    assert_eq!(
        issue.targets,
        curvyo_laser_core::job::Targets::Items(vec![covered])
    );
    assert!(!checks.has(&Cause::NothingRuns));
}

/// Criterion 11: own tabs that overlap make the item run nothing and count as damaged settings.
#[test]
fn overlapping_own_tabs_run_nothing() {
    let mut page = cut_page(
        plain(),
        vec![
            vec![rect(10.0, 10.0, 10.0, 10.0)],
            vec![rect(30.0, 10.0, 10.0, 10.0)],
        ],
        1,
    );
    page.input.blocks[0].leaves[0].cut = LeafTabs::Own(vec![(
        0,
        vec![own_tab(0.1, Some(5.0)), own_tab(0.15, Some(5.0))],
    )]);
    let built = build_job(&page.input, &[], &mut PlanCache::default());
    let back = read(
        &String::from_utf8(write(page.input.machine.values.output_format, &built.job)).unwrap(),
    );
    assert!(
        (back.burn - 40.0).abs() < 0.01,
        "only the second square burns"
    );
    let checks = gate(
        &page.input,
        &built,
        &Timeline::of(&built.job),
        &GateFacts::default(),
    );
    assert!(checks.has(&Cause::DamagedSettings));
    let mut damaged = page;
    damaged.input.blocks[0].leaves[0].cut = LeafTabs::Damaged;
    let built = build_job(&damaged.input, &[], &mut PlanCache::default());
    assert_eq!(built.plans[0].as_ref().unwrap().damaged.len(), 1);
}

/// Criterion 9: a plate with a hole and tabs on both cuts the hole's pieces before the plate's.
#[test]
fn the_hole_with_tabs_runs_before_the_plate() {
    let page = cut_page(
        tabs(4, 1.0, false),
        vec![vec![
            rect(10.0, 10.0, 60.0, 60.0),
            rect(30.0, 30.0, 20.0, 20.0),
        ]],
        1,
    );
    let built = build_job(&page.input, &[], &mut PlanCache::default());
    let run = &built.plans[0].as_ref().unwrap().runs[0];
    let inside = |p: &curvyo_laser_core::job::PointUm| {
        (30_000..=50_000).contains(&p.x) && (30_000..=50_000).contains(&p.y)
    };
    let holes = run
        .paths
        .iter()
        .take_while(|p| p.points.iter().all(inside))
        .count();
    assert_eq!(holes, 5, "the hole's five pieces first");
    assert!(
        run.paths[holes..]
            .iter()
            .all(|p| !p.points.iter().all(inside))
    );
}
