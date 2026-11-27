//! The path goldens of `specs/0139-laser-cut-path-options` slice 2 (criteria 14 to 20): each file
//! is `write(GRBL, job)`, built twice and compared byte for byte, and read back by a reader of its
//! own that sums the burnt runs and keeps their points, so the lead lengths, the overcut and the
//! start are measured on the written file. `CURVYO_UPDATE_GOLDEN=1` rewrites the files; review
//! the diff.

#![allow(clippy::unwrap_used, clippy::missing_panics_doc)]

mod job_fixtures;

use std::path::PathBuf;

use curvyo_document_core::{Point, Vec2};
use curvyo_laser_core::job::{
    BlockSource, Cause, GateFacts, JobInput, LeafOutline, PlanCache, Targets, Timeline, build_job,
    gate, write,
};
use curvyo_laser_core::operations::OperationKind::{Cut, CutOut};
use curvyo_laser_core::settings::{
    CutLength, CutSettings, LeadKind, OperationOptions, Overcut, PathOptions, TabCount,
};
use curvyo_laser_core::tabs::{Direction, LeafPath, LeafTabs, OutlineEntry, OwnTab, PathFraction};
use job_fixtures::{Page, circle, leaf, line, own, plain, rect};

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/output/grbl")
}

/// A point in machine mm (machine Y = 300 - page Y).
type At = (f64, f64);

/// The burnt runs read back from a file, with their points.
#[derive(Debug, Default)]
struct Read {
    runs: Vec<Vec<At>>,
    burn: f64,
    travel: f64,
}

fn word(line: &str, letter: char) -> Option<f64> {
    line.split_whitespace()
        .find_map(|w| w.strip_prefix(letter).and_then(|v| v.parse().ok()))
}

/// A reader independent of the generator: G0 is travel, G1 with S above 0 burns (M4 only here).
fn read(text: &str) -> Read {
    let (mut x, mut y, mut s) = (0.0_f64, 0.0_f64, 0.0_f64);
    let mut out = Read::default();
    let mut run: Vec<At> = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with(';')) {
        let Some(g @ ("G0" | "G1")) = line.split_whitespace().next() else {
            continue;
        };
        if let Some(v) = word(line, 'S') {
            s = v;
        }
        let (Some(nx), Some(ny)) = (word(line, 'X'), word(line, 'Y')) else {
            continue;
        };
        let length = (nx - x).hypot(ny - y);
        if g == "G1" && s > 0.0 {
            out.burn += length;
            if run.is_empty() {
                run.push((x, y));
            }
            run.push((nx, ny));
        } else {
            out.travel += length;
            if !run.is_empty() {
                out.runs.push(std::mem::take(&mut run));
            }
        }
        (x, y) = (nx, ny);
    }
    if !run.is_empty() {
        out.runs.push(run);
    }
    out
}

/// The golden `name` of `input` read back, checked against the Timeline.
fn golden(name: &str, input: &JobInput) -> Read {
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
    back
}

/// The burn length of `input` without writing a golden.
fn burn_of(input: &JobInput) -> f64 {
    let built = build_job(input, &[], &mut PlanCache::default());
    Timeline::of(&built.job).burn_mm()
}

fn path(edit: impl FnOnce(&mut PathOptions)) -> PathOptions {
    let mut options = PathOptions::DEFAULT;
    edit(&mut options);
    options
}

fn mm(value: f64) -> CutLength {
    CutLength::tab_of_mm(value).unwrap()
}

fn with_path(options: PathOptions) -> OperationOptions {
    OperationOptions {
        path: options,
        ..plain()
    }
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

/// A 40 x 40 mm square from `(x, y)` (page, Y down) drawn from the midpoint of its side at page
/// Y `y + 40` (machine bottom), along +X first.
fn square_from_midpoint(x: f64, y: f64) -> LeafOutline {
    LeafOutline {
        anchors: vec![
            corner(x + 20.0, y + 40.0),
            corner(x + 40.0, y + 40.0),
            corner(x + 40.0, y),
            corner(x, y),
            corner(x, y + 40.0),
        ],
        closed: true,
    }
}

fn start_at(fraction: f64) -> LeafPath {
    LeafPath {
        entries: vec![(0, OutlineEntry::Start(PathFraction::of(fraction)))],
        ..LeafPath::default()
    }
}

fn distance(a: At, b: At) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// Criterion 14: a Line lead-in of 2 mm starts 2 mm below the bottom side of a part.
#[test]
fn lead_in_line_outer() {
    let options = with_path(path(|p| {
        p.lead_in = LeadKind::Line;
        p.lead_in_length = mm(2.0);
    }));
    let mut page = cut_page(options, vec![vec![square_from_midpoint(10.0, 10.0)]], 1);
    page.input.blocks[0].leaves[0].path = start_at(0.0);
    let back = golden("lead_in_line_outer.gcode", &page.input);
    assert_eq!(back.runs.len(), 1);
    // The side lies at page Y 50, machine Y 250; the waste side is outside, machine Y below 250.
    let first = back.runs[0][0];
    assert!(
        (first.0 - 30.0).abs() < 0.005 && (first.1 - 248.0).abs() < 0.005,
        "{first:?}"
    );
    assert!((back.runs[0][1].1 - 250.0).abs() < 0.005);
    assert!((back.burn - 162.0).abs() < 0.01, "burn {}", back.burn);
}

/// Criterion 14: an Arc lead-in is a quarter circle of radius 2 mm tangent at the junction.
#[test]
fn lead_in_arc_outer() {
    let options = with_path(path(|p| {
        p.lead_in = LeadKind::Arc;
        p.lead_in_length = mm(2.0);
    }));
    let mut page = cut_page(options, vec![vec![square_from_midpoint(10.0, 10.0)]], 1);
    page.input.blocks[0].leaves[0].path = start_at(0.0);
    let back = golden("lead_in_arc_outer.gcode", &page.input);
    let run = &back.runs[0];
    let junction = run
        .iter()
        .position(|p| distance(*p, (30.0, 250.0)) < 0.002)
        .unwrap();
    let centre = (30.0, 248.0);
    for p in &run[..=junction] {
        assert!(
            (distance(*p, centre) - 2.0).abs() < 0.01,
            "{p:?} off the circle"
        );
    }
    assert!(
        distance(run[0], (28.0, 248.0)) < 0.005,
        "starts a quarter turn back"
    );
    let quarter = std::f64::consts::FRAC_PI_2 * 2.0;
    assert!(
        (back.burn - (160.0 + quarter)).abs() < 0.01,
        "burn {}",
        back.burn
    );
}

/// Criteria 9, 14: a hole runs before its plate and its lead-in starts 1 mm inside the hole.
#[test]
fn lead_in_hole() {
    let options = with_path(path(|p| {
        p.lead_in = LeadKind::Line;
        p.lead_in_length = mm(1.0);
    }));
    let page = cut_page(
        options,
        vec![
            vec![rect(10.0, 10.0, 60.0, 60.0)],
            vec![circle(40.0, 40.0, 5.0)],
        ],
        1,
    );
    let back = golden("lead_in_hole.gcode", &page.input);
    assert_eq!(back.runs.len(), 2);
    // The hole (machine centre 40, 260) first: its lead starts inside, 1 mm from its edge.
    let hole = &back.runs[0];
    assert!(distance(hole[0], (40.0, 260.0)) < 4.01, "inside the hole");
    assert!((distance(hole[0], hole[1]) - 1.0).abs() < 0.005);
    assert!(
        (distance(hole[1], (40.0, 260.0)) - 5.0).abs() < 0.01,
        "ends on the edge"
    );
    // The plate's lead lies outside the plate.
    let plate = back.runs[1][0];
    let inside = |p: At| p.0 > 10.0 && p.0 < 70.0 && p.1 > 230.0 && p.1 < 290.0;
    assert!(!inside(plate), "{plate:?}");
}

/// Criterion 14: a lead-out Arc leaves at the last burnt point in the path direction.
#[test]
fn lead_out_arc() {
    let options = with_path(path(|p| {
        p.lead_out = LeadKind::Arc;
        p.lead_out_length = mm(1.0);
    }));
    let mut page = cut_page(options, vec![vec![square_from_midpoint(10.0, 10.0)]], 1);
    page.input.blocks[0].leaves[0].path = start_at(0.0);
    let back = golden("lead_out_arc.gcode", &page.input);
    let run = &back.runs[0];
    let end = *run.last().unwrap();
    // Arriving along machine +X at (30, 250): it ends 1 mm on and 1 mm out (below).
    assert!(distance(end, (31.0, 249.0)) < 0.005, "{end:?}");
    let quarter = std::f64::consts::FRAC_PI_2;
    assert!((back.burn - (160.0 + quarter)).abs() < 0.01);
}

/// Criterion 15: a circle with Overcut 5 mm burns its length and 5 mm more.
#[test]
fn overcut_closed() {
    let r = 60.0 / std::f64::consts::TAU;
    let ring = || vec![vec![circle(100.0, 100.0, r)]];
    let base = burn_of(&cut_page(plain(), ring(), 1).input);
    assert!((base - 60.0).abs() < 0.05, "the flattened circle {base}");
    let options = with_path(path(|p| p.overcut = Overcut::of_mm(5.0).unwrap()));
    let back = golden("overcut_closed.gcode", &cut_page(options, ring(), 1).input);
    assert!(
        (back.burn - (base + 5.0)).abs() < 0.01,
        "burn {}",
        back.burn
    );
    assert_eq!(back.runs.len(), 1);
}

/// Criterion 15: one 2 mm tab starting 1 mm after the start point stops the overcut at 1 mm.
#[test]
fn overcut_stops_at_tab() {
    let r = 60.0 / std::f64::consts::TAU;
    let ring = || vec![vec![circle(100.0, 100.0, r)]];
    let base = burn_of(&cut_page(plain(), ring(), 1).input);
    let options = with_path(path(|p| p.overcut = Overcut::of_mm(5.0).unwrap()));
    let mut page = cut_page(options, ring(), 1);
    let leaf = &mut page.input.blocks[0].leaves[0];
    leaf.path = start_at(0.0);
    leaf.cut = LeafTabs::Own(vec![(
        0,
        vec![OwnTab {
            at: PathFraction::of(2.0 / base),
            width: Some(mm(2.0)),
        }],
    )]);
    let back = golden("overcut_stops_at_tab.gcode", &page.input);
    assert!(
        (back.burn - (base - 2.0 + 1.0)).abs() < 0.01,
        "burn {}",
        back.burn
    );
}

/// Criterion 16: every pass writes its own lead-in, overcut and lead-out.
#[test]
fn path_options_passes() {
    let options = with_path(path(|p| {
        p.lead_in = LeadKind::Line;
        p.lead_out = LeadKind::Line;
        p.overcut = Overcut::of_mm(2.0).unwrap();
    }));
    let mut page = cut_page(options, vec![vec![square_from_midpoint(10.0, 10.0)]], 2);
    page.input.blocks[0].leaves[0].path = start_at(0.0);
    let back = golden("path_options_passes.gcode", &page.input);
    assert_eq!(back.runs.len(), 2);
    assert!((back.burn - 2.0 * (160.0 + 1.0 + 2.0 + 1.0)).abs() < 0.02);
    assert_eq!(back.runs[0], back.runs[1], "the same moves each pass");
}

/// Criterion 21: an item's own path options replace the operation's, the others keep them.
#[test]
fn path_options_own_item() {
    let mut page = cut_page(
        plain(),
        vec![
            vec![square_from_midpoint(10.0, 10.0)],
            vec![square_from_midpoint(80.0, 10.0)],
        ],
        1,
    );
    page.input.blocks[0].leaves[1].path = LeafPath {
        own: Some(path(|p| {
            p.lead_in = LeadKind::Arc;
            p.lead_in_length = mm(1.0);
        })),
        entries: vec![(0, OutlineEntry::Start(PathFraction::of(0.0)))],
        damaged: false,
    };
    let back = golden("path_options_own_item.gcode", &page.input);
    let quarter = std::f64::consts::FRAC_PI_2;
    assert!(
        (back.burn - (320.0 + quarter)).abs() < 0.01,
        "burn {}",
        back.burn
    );
}

/// Criterion 19: a set start is written exactly and the burn returns to it.
#[test]
fn start_point_set() {
    let mut page = cut_page(plain(), vec![vec![square_from_midpoint(10.0, 10.0)]], 1);
    // 0.3 of 160 mm = 48 mm from the midpoint: 20 mm to the corner, 28 mm up the right side.
    page.input.blocks[0].leaves[0].path = start_at(0.3);
    let back = golden("start_point_set.gcode", &page.input);
    let run = &back.runs[0];
    let expected = (50.0, 250.0 + 28.0);
    assert!(distance(run[0], expected) < 0.005, "{:?}", run[0]);
    assert!(distance(*run.last().unwrap(), expected) < 0.005);
}

/// Criterion 20: a reversed open path starts at its last point.
#[test]
fn open_direction_reversed() {
    let mut page = cut_page(
        plain(),
        vec![vec![line(
            Point::new(20.0, 280.0),
            Point::new(120.0, 280.0),
        )]],
        1,
    );
    page.input.blocks[0].leaves[0].path = LeafPath {
        entries: vec![(0, OutlineEntry::Direction(Direction::Reversed))],
        ..LeafPath::default()
    };
    let back = golden("open_direction_reversed.gcode", &page.input);
    assert!(distance(back.runs[0][0], (120.0, 20.0)) < 0.005);
    assert!(distance(*back.runs[0].last().unwrap(), (20.0, 20.0)) < 0.005);
}

/// Criteria 13 to 15 on a Cut-out: tabs, both leads and an overcut together; tabs win.
#[test]
fn tabs_leads_overcut() {
    let mut page = Page::new();
    page.cut_out(
        "Cut-out 1",
        own(CutOut, 80.0, 600, 1),
        0.0,
        vec![vec![vec![rect(10.0, 10.0, 30.0, 30.0)]]],
    );
    if let BlockSource::Operation {
        options: Ok(options),
        ..
    } = &mut page.input.blocks[0].source
    {
        options.cut = CutSettings {
            tab_count: TabCount::new(2).unwrap(),
            tab_width: mm(1.5),
            tab_skip_inner: false,
        };
        options.path = path(|p| {
            p.lead_in = LeadKind::Arc;
            p.lead_out = LeadKind::Line;
            p.overcut = Overcut::of_mm(3.0).unwrap();
        });
    }
    let back = golden("tabs_leads_overcut.gcode", &page.input);
    assert_eq!(
        back.runs.len(),
        3,
        "two tabs: three runs from a corner entry"
    );
    let quarter = std::f64::consts::FRAC_PI_2;
    // The overcut runs on from the corner along the first piece, which is 15 - 0.75 mm long.
    let expected = 120.0 - 3.0 + quarter + 1.0 + 3.0;
    assert!((back.burn - expected).abs() < 0.02, "burn {}", back.burn);
}

/// Criterion 17: a Line lead-in of 2 mm adds 2 mm to the burn; a lead outside the work area
/// blocks like any burn move.
#[test]
fn leads_count_as_burn_and_block_outside_the_work_area() {
    let lead = with_path(path(|p| {
        p.lead_in = LeadKind::Line;
        p.lead_in_length = mm(2.0);
    }));
    let plain_page = cut_page(plain(), vec![vec![square_from_midpoint(10.0, 10.0)]], 1);
    let mut page = cut_page(lead, vec![vec![square_from_midpoint(10.0, 10.0)]], 1);
    page.input.blocks[0].leaves[0].path = start_at(0.0);
    assert!((burn_of(&page.input) - burn_of(&plain_page.input) - 2.0).abs() < 0.01);
    // A part on the bed edge (page Y 300 is machine Y 0): its lead leaves the area.
    let mut edge = cut_page(lead, vec![vec![square_from_midpoint(10.0, 260.0)]], 1);
    edge.input.blocks[0].leaves[0].path = start_at(0.0);
    let built = build_job(&edge.input, &[], &mut PlanCache::default());
    let checks = gate(
        &edge.input,
        &built,
        &Timeline::of(&built.job),
        &GateFacts::default(),
    );
    assert!(
        checks
            .issues
            .iter()
            .any(|i| matches!(i.cause, Cause::OutsideBed { .. })),
        "{:?}",
        checks.issues
    );
    let plain_edge = cut_page(plain(), vec![vec![square_from_midpoint(10.0, 260.0)]], 1);
    let built = build_job(&plain_edge.input, &[], &mut PlanCache::default());
    let checks = gate(
        &plain_edge.input,
        &built,
        &Timeline::of(&built.job),
        &GateFacts::default(),
    );
    assert!(
        !checks
            .issues
            .iter()
            .any(|i| matches!(i.cause, Cause::OutsideBed { .. }))
    );
}

/// Criterion 18: a lead that crosses another cut of the operation warns once and still runs.
#[test]
fn a_lead_that_crosses_a_cut_warns() {
    let lead = with_path(path(|p| {
        p.lead_in = LeadKind::Line;
        p.lead_in_length = mm(5.0);
    }));
    // A line 2 mm below the square's bottom side crosses its 5 mm lead-in.
    let mut page = cut_page(
        lead,
        vec![
            vec![square_from_midpoint(10.0, 10.0)],
            vec![line(Point::new(20.0, 52.0), Point::new(40.0, 52.0))],
        ],
        1,
    );
    page.input.blocks[0].leaves[0].path = start_at(0.0);
    let item = page.input.blocks[0].leaves[0].node;
    let built = build_job(&page.input, &[], &mut PlanCache::default());
    let timeline = Timeline::of(&built.job);
    let checks = gate(&page.input, &built, &timeline, &GateFacts::default());
    let issue = checks
        .issues
        .iter()
        .find(|i| matches!(i.cause, Cause::LeadsTouchCut { .. }))
        .unwrap();
    assert_eq!(issue.count, 1);
    assert_eq!(issue.targets, Targets::Items(vec![item]));
    let names = curvyo_laser_core::job::Names::default();
    assert_eq!(
        curvyo_laser_core::job::check_text(issue, &names).whole(),
        "1 lead crosses or touches a cut."
    );
    assert!(timeline.burn_mm() > 180.0, "the job still runs");
    // Without the line nothing warns.
    page.input.blocks[0].leaves.truncate(1);
    let built = build_job(&page.input, &[], &mut PlanCache::default());
    let checks = gate(
        &page.input,
        &built,
        &Timeline::of(&built.job),
        &GateFacts::default(),
    );
    assert!(
        !checks
            .issues
            .iter()
            .any(|i| matches!(i.cause, Cause::LeadsTouchCut { .. }))
    );
}

/// Criterion 22: damaged own records make the item run nothing and count as damaged settings.
#[test]
fn damaged_path_records_run_nothing() {
    let mut page = cut_page(
        plain(),
        vec![
            vec![square_from_midpoint(10.0, 10.0)],
            vec![square_from_midpoint(80.0, 10.0)],
        ],
        1,
    );
    page.input.blocks[0].leaves[0].path = LeafPath {
        damaged: true,
        ..LeafPath::default()
    };
    assert!((burn_of(&page.input) - 160.0).abs() < 0.01);
    let built = build_job(&page.input, &[], &mut PlanCache::default());
    let checks = gate(
        &page.input,
        &built,
        &Timeline::of(&built.job),
        &GateFacts::default(),
    );
    assert!(checks.has(&Cause::DamagedSettings));
}
