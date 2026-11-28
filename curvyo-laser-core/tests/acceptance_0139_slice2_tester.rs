//! Acceptance tests of `specs/0139-laser-cut-path-options` slice 2 (criteria 13 to 24), written
//! from the specification before the implementation was read. Black box: a job is built through
//! `build_job`, written as GRBL with `write`, and read back by a reader of its own that knows only
//! the G-code words (G0 travel, G1 burn at S above 0, M3/M4/M5 laser state).

#![allow(clippy::unwrap_used, clippy::missing_panics_doc, clippy::float_cmp)]

mod job_fixtures;

use curvyo_laser_core::job::{
    GateFacts, JobInput, LeafOutline, PlanCache, Timeline, build_job, gate, write,
};
use curvyo_laser_core::operations::OperationKind::Cut;
use curvyo_laser_core::settings::{CutLength, LeadKind, OperationOptions, Overcut, PathOptions};
use curvyo_laser_core::tabs::{Direction, LeafPath, OutlineEntry, PathFraction};
use job_fixtures::{Page, circle, leaf, line, own, plain, rect};

/// One G-code move read back: from `a` to `b`, burning when `burn`.
#[derive(Clone, Copy, Debug)]
struct Seg {
    a: (f64, f64),
    b: (f64, f64),
    burn: bool,
}

fn len(s: &Seg) -> f64 {
    (s.b.0 - s.a.0).hypot(s.b.1 - s.a.1)
}

fn dist(p: (f64, f64), q: (f64, f64)) -> f64 {
    (p.0 - q.0).hypot(p.1 - q.1)
}

/// The moves of a GRBL file: G1 with S above 0 under the laser on burns, every other move travels.
fn segments(text: &str) -> Vec<Seg> {
    let (mut cur_x, mut cur_y, mut power) = (0.0_f64, 0.0_f64, 0.0_f64);
    let (mut g1, mut m3, mut on) = (false, false, true);
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('(') {
            continue;
        }
        let (mut nx, mut ny, mut moved) = (cur_x, cur_y, false);
        for word in line.split_whitespace() {
            match word {
                "G0" => g1 = false,
                "G1" => g1 = true,
                "M3" => (m3, on) = (true, true),
                "M4" => (m3, on) = (false, true),
                "M5" => {
                    if m3 {
                        on = false;
                    }
                }
                _ => match (word.get(..1), word.get(1..).map(str::parse::<f64>)) {
                    (Some("X"), Some(Ok(value))) => {
                        nx = value;
                        moved = true;
                    }
                    (Some("Y"), Some(Ok(value))) => {
                        ny = value;
                        moved = true;
                    }
                    (Some("S"), Some(Ok(value))) => power = value,
                    _ => {}
                },
            }
        }
        if moved {
            out.push(Seg {
                a: (cur_x, cur_y),
                b: (nx, ny),
                burn: g1 && power > 0.0 && on,
            });
            (cur_x, cur_y) = (nx, ny);
        }
    }
    out
}

struct Run {
    segs: Vec<Seg>,
    /// Burn length summed from the G-code.
    burn: f64,
    /// Travel length summed from the G-code.
    travel: f64,
    timeline_burn: f64,
    export_allowed: bool,
    /// The debug text of each gate issue's cause.
    issues: Vec<String>,
}

const BED_H: f64 = 300.0;

fn run(input: &JobInput) -> Run {
    let built = build_job(input, &[], &mut PlanCache::default());
    let text = String::from_utf8(write(input.machine.values.output_format, &built.job)).unwrap();
    let timeline = Timeline::of(&built.job);
    let verdict = gate(input, &built, &timeline, &GateFacts::default());
    // GRBL Y runs up from the bed bottom, the document Y down: the bed of `Page::new` is 300 mm.
    let segs: Vec<Seg> = segments(&text)
        .into_iter()
        .map(|s| Seg {
            a: (s.a.0, BED_H - s.a.1),
            b: (s.b.0, BED_H - s.b.1),
            burn: s.burn,
        })
        .collect();
    Run {
        burn: segs.iter().filter(|s| s.burn).map(len).sum(),
        travel: segs.iter().filter(|s| !s.burn).map(len).sum(),
        timeline_burn: timeline.burn_mm(),
        export_allowed: verdict.export_allowed(),
        issues: verdict
            .issues
            .iter()
            .map(|i| format!("{:?} {}", i.cause, i.count))
            .collect(),
        segs,
    }
}

/// The burn length from the first burn move until the burn first reaches `q`.
fn burn_until(r: &Run, q: (f64, f64)) -> f64 {
    let mut sum = 0.0;
    for s in r.segs.iter().filter(|s| s.burn) {
        sum += len(s);
        if dist(s.b, q) < 0.005 {
            return sum;
        }
    }
    f64::NAN
}

fn opts(path: PathOptions) -> OperationOptions {
    OperationOptions { path, ..plain() }
}

fn lead(kind: LeadKind, mm: f64) -> PathOptions {
    PathOptions {
        lead_in: kind,
        lead_in_length: CutLength::tab_of_mm(mm).unwrap(),
        lead_out: kind,
        lead_out_length: CutLength::tab_of_mm(mm).unwrap(),
        ..PathOptions::DEFAULT
    }
}

fn lead_in_only(kind: LeadKind, mm: f64) -> PathOptions {
    PathOptions {
        lead_in: kind,
        lead_in_length: CutLength::tab_of_mm(mm).unwrap(),
        ..PathOptions::DEFAULT
    }
}

fn lead_out_only(kind: LeadKind, mm: f64) -> PathOptions {
    PathOptions {
        lead_out: kind,
        lead_out_length: CutLength::tab_of_mm(mm).unwrap(),
        ..PathOptions::DEFAULT
    }
}

/// A Cut page of one leaf with `outlines`, its operation `path` options, `passes` passes, and the
/// leaf's entries and own path options set on the built block.
fn cut_page(
    path: PathOptions,
    passes: u8,
    outlines: Vec<LeafOutline>,
    entries: Vec<(u32, OutlineEntry)>,
    leaf_own: Option<PathOptions>,
) -> Page {
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 50.0, 1000, passes),
        opts(path),
        vec![leaf(outlines)],
    );
    let block = page.input.blocks.last_mut().unwrap();
    block.leaves[0].path = LeafPath {
        own: leaf_own,
        entries,
        damaged: false,
    };
    page
}

fn start_at(ppb: u64) -> Vec<(u32, OutlineEntry)> {
    vec![(0, OutlineEntry::Start(PathFraction::new(ppb).unwrap()))]
}

/// A 40 mm square at (10, 10), drawn from its bottom-left corner.
fn square40() -> Vec<LeafOutline> {
    vec![rect(10.0, 10.0, 40.0, 40.0)]
}

// ---------------------------------------------------------------- criterion 13: ranges

#[test]
fn c13_lead_overcut_ranges_refuse_out_of_range_values() {
    assert!(Overcut::typed(10.0).is_ok(), "Overcut 10 mm is allowed");
    assert!(
        Overcut::typed(10.01).is_err(),
        "Overcut 10.01 mm is refused"
    );
    assert!(
        Overcut::typed(-0.01).is_err(),
        "a negative Overcut is refused"
    );
    assert!(CutLength::typed_tab(0.1).is_ok());
    assert!(CutLength::typed_tab(20.0).is_ok());
    assert!(
        CutLength::typed_tab(0.09).is_err(),
        "lead 0.09 mm is refused"
    );
    assert!(
        CutLength::typed_tab(20.01).is_err(),
        "lead 20.01 mm is refused"
    );
    assert_eq!(PathOptions::DEFAULT.lead_in, LeadKind::None);
    assert_eq!(PathOptions::DEFAULT.lead_out, LeadKind::None);
    assert_eq!(PathOptions::DEFAULT.overcut.mm(), 0.0);
    assert_eq!(
        PathOptions::DEFAULT.lead_in_length.mm(),
        1.0,
        "default lead 1 mm"
    );
}

// ---------------------------------------------------------------- criterion 14: lead geometry

#[test]
fn c14_line_lead_in_on_outer_square_starts_2mm_below_the_side() {
    let entry = start_at(125_000_000); // 0.125 of 160 mm = 20 mm along the bottom side: (30, 10)
    let base = cut_page(PathOptions::DEFAULT, 1, square40(), entry.clone(), None);
    let with = cut_page(
        lead_in_only(LeadKind::Line, 2.0),
        1,
        square40(),
        entry,
        None,
    );
    let plain_run = run(&base.input);
    let lead_run = run(&with.input);
    let first = *lead_run.segs.iter().find(|s| s.burn).unwrap();
    // The lead is one straight burn of 2.000 mm that ends on the side at (30, 10).
    assert!(
        (len(&first) - 2.0).abs() <= 0.005,
        "lead length {}",
        len(&first)
    );
    assert!(
        dist(first.b, (30.0, 10.0)) <= 0.005,
        "lead ends at the start point"
    );
    // It starts 2.000 mm below the side (the waste side of a depth 0 outline is outside).
    assert!(
        dist(first.a, (30.0, 8.0)) <= 0.005,
        "lead starts at (30, 8): {:?}",
        first.a
    );
    // The lead is travelled to: the move before it ends where the lead starts.
    let idx = lead_run.segs.iter().position(|s| s.burn).unwrap();
    assert!(!lead_run.segs[idx - 1].burn);
    assert!(dist(lead_run.segs[idx - 1].b, first.a) <= 0.005);
    // Criterion 17 on the same job: the lead adds exactly 2 mm of burn.
    assert!((lead_run.burn - plain_run.burn - 2.0).abs() <= 0.01);
    assert!((lead_run.burn - lead_run.timeline_burn).abs() <= 0.01);
}

#[test]
fn c14_line_lead_in_inside_a_hole_starts_1mm_from_its_edge() {
    // A plate of 100 mm with a 10 mm hole (depth 1): the hole's lead starts inside the hole.
    let outlines = vec![rect(0.0, 0.0, 100.0, 100.0), circle(50.0, 50.0, 5.0)];
    let page = cut_page(
        lead_in_only(LeadKind::Line, 1.0),
        1,
        outlines.clone(),
        vec![],
        None,
    );
    let plain = cut_page(PathOptions::DEFAULT, 1, outlines, vec![], None);
    let plain_run = run(&plain.input);
    let lead_run = run(&page.input);
    let centre = (50.0, 50.0);
    let found = lead_run
        .segs
        .iter()
        .filter(|s| s.burn)
        .any(|s| (len(s) - 1.0).abs() <= 0.005 && (dist(s.a, centre) - 4.0).abs() <= 0.01);
    assert!(
        found,
        "a 1.000 mm lead starts 4.000 mm from the hole centre (1 mm inside the edge)"
    );
    // The plate's own lead (outward, 1 mm) is not inside the hole.
    assert!(
        (lead_run.burn - plain_run.burn - 2.0).abs() <= 0.01,
        "one lead per outline"
    );
}

#[test]
fn c14_arc_lead_in_is_a_quarter_circle_tangent_to_the_side() {
    let entry = start_at(125_000_000);
    let base = cut_page(PathOptions::DEFAULT, 1, square40(), entry.clone(), None);
    let with = cut_page(lead_in_only(LeadKind::Arc, 2.0), 1, square40(), entry, None);
    let plain_run = run(&base.input);
    let arc_run = run(&with.input);
    let quarter = std::f64::consts::FRAC_PI_2 * 2.0;
    let lead = burn_until(&arc_run, (30.0, 10.0));
    assert!(
        (lead - quarter).abs() <= 0.01,
        "arc lead length {lead} vs {quarter}"
    );
    // The arc starts at (28, 8): centre (30, 8), radius 2, tangent to the side at (30, 10).
    let first = arc_run.segs.iter().find(|s| s.burn).unwrap();
    assert!(
        dist(first.a, (28.0, 8.0)) <= 0.01,
        "arc starts at (28, 8): {:?}",
        first.a
    );
    assert!((arc_run.burn - plain_run.burn - quarter).abs() <= 0.01);
}

#[test]
fn c14_arc_lead_out_adds_a_quarter_circle_after_the_last_burnt_point() {
    let entry = start_at(125_000_000);
    let base = cut_page(PathOptions::DEFAULT, 1, square40(), entry.clone(), None);
    let with = cut_page(
        lead_out_only(LeadKind::Arc, 2.0),
        1,
        square40(),
        entry,
        None,
    );
    let plain_run = run(&base.input);
    let out_run = run(&with.input);
    let quarter = std::f64::consts::FRAC_PI_2 * 2.0;
    assert!(
        (out_run.burn - plain_run.burn - quarter).abs() <= 0.01,
        "arc lead-out adds {} vs {quarter}",
        out_run.burn - plain_run.burn
    );
}

// ---------------------------------------------------------------- criterion 15: overcut

#[test]
fn c15_overcut_5mm_on_a_closed_circle_adds_5mm_of_burn() {
    let r = 60.0 / (2.0 * std::f64::consts::PI);
    let outlines = || vec![circle(50.0, 50.0, r)];
    let base = cut_page(PathOptions::DEFAULT, 1, outlines(), vec![], None);
    let over = cut_page(
        PathOptions {
            overcut: Overcut::of_mm(5.0).unwrap(),
            ..PathOptions::DEFAULT
        },
        1,
        outlines(),
        vec![],
        None,
    );
    let b0 = run(&base.input);
    let b1 = run(&over.input);
    assert!(
        (b0.burn - 60.0).abs() <= 0.05,
        "circle circumference {}",
        b0.burn
    );
    assert!(
        (b1.burn - 65.0).abs() <= 0.01,
        "overcut burn {} vs 65",
        b1.burn
    );
    assert!((b1.burn - b0.burn - 5.0).abs() <= 0.01);
}

// ---------------------------------------------------------------- criterion 16: passes

#[test]
fn c16_each_pass_writes_its_own_lead_in_and_lead_out() {
    let one = cut_page(lead(LeadKind::Line, 2.0), 1, square40(), vec![], None);
    let two = cut_page(lead(LeadKind::Line, 2.0), 2, square40(), vec![], None);
    let r1 = run(&one.input);
    let r2 = run(&two.input);
    assert!(
        (r1.burn - 164.0).abs() <= 0.01,
        "one pass: 160 + 2 + 2 = {}",
        r1.burn
    );
    assert!(
        (r2.burn - 2.0 * r1.burn).abs() <= 0.01,
        "two passes: {} vs 2 x {}",
        r2.burn,
        r1.burn
    );
}

// ---------------------------------------------------------------- criterion 17: timeline, gate

#[test]
fn c17_bed_edge_lead_outside_the_work_area_blocks_export() {
    // A 40 mm square at the bed corner (0, 0); the start on its left side (0, 20), lead outward.
    let at_corner = vec![rect(0.0, 0.0, 40.0, 40.0)];
    let entry = start_at(875_000_000); // 0.875 of 160 mm = 140 mm: the left side's midpoint
    let inside = cut_page(
        PathOptions::DEFAULT,
        1,
        at_corner.clone(),
        entry.clone(),
        None,
    );
    let leaving = cut_page(lead_in_only(LeadKind::Line, 2.0), 1, at_corner, entry, None);
    assert!(
        run(&inside.input).export_allowed,
        "control: the square alone exports"
    );
    assert!(
        !run(&leaving.input).export_allowed,
        "a lead at x = -2 mm blocks Export"
    );
}

// ---------------------------------------------------------------- criterion 18: lead crossings

#[test]
fn c18_lead_that_crosses_another_cut_warns_and_the_job_still_runs() {
    // Square A (0..10) with its start on the right side and a lead outward to x = 12 crosses the
    // left edge of square B (11..21).
    let a = rect(0.0, 0.0, 10.0, 10.0);
    let b = rect(11.0, 0.0, 10.0, 10.0);
    let entry = start_at(375_000_000); // 15 mm of 40 along: the right side's midpoint (10, 5)
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 50.0, 1000, 1),
        opts(lead_in_only(LeadKind::Line, 2.0)),
        vec![leaf(vec![a]), leaf(vec![b])],
    );
    let block = page.input.blocks.last_mut().unwrap();
    block.leaves[0].path = LeafPath {
        own: None,
        entries: entry,
        damaged: false,
    };
    let with = run(&page.input);
    assert!(
        with.export_allowed,
        "a crossing lead warns and does not block"
    );
    assert!(
        with.issues
            .iter()
            .any(|i| i.to_lowercase().contains("lead")),
        "a lead warning is shown: {:?}",
        with.issues
    );
}

// ---------------------------------------------------------------- criterion 19: start point

#[test]
fn c19_set_start_point_begins_the_burn_there_and_goes_round_in_the_drawn_direction() {
    let set = cut_page(
        PathOptions::DEFAULT,
        1,
        square40(),
        start_at(125_000_000),
        None,
    );
    let set_run = run(&set.input);
    let burns: Vec<&Seg> = set_run.segs.iter().filter(|s| s.burn).collect();
    let target = (30.0, 10.0);
    assert!(
        dist(burns[0].a, target) <= 0.005,
        "burn begins at (30, 10): {:?}",
        burns[0].a
    );
    // Drawn direction: the bottom side runs towards (50, 10) first.
    assert!(
        dist(burns[0].b, (50.0, 10.0)) <= 0.005,
        "next node (50, 10)"
    );
    let last = burns.last().unwrap();
    assert!(
        dist(last.b, target) <= 0.005,
        "the cut returns to the start point"
    );
    assert!(
        (set_run.burn - 160.0).abs() <= 0.01,
        "one full outline, {}",
        set_run.burn
    );
}

// ---------------------------------------------------------------- criterion 20: direction

#[test]
fn c20_open_direction_sets_the_end_the_cut_starts_at() {
    let drawn = vec![line(
        curvyo_document_core::Point::new(0.0, 0.0),
        curvyo_document_core::Point::new(100.0, 0.0),
    )];
    let as_drawn = cut_page(
        PathOptions::DEFAULT,
        1,
        drawn.clone(),
        vec![(0, OutlineEntry::Direction(Direction::AsDrawn))],
        None,
    );
    let reversed = cut_page(
        PathOptions::DEFAULT,
        1,
        drawn,
        vec![(0, OutlineEntry::Direction(Direction::Reversed))],
        None,
    );
    let a = run(&as_drawn.input);
    let r = run(&reversed.input);
    let fa = a.segs.iter().find(|s| s.burn).unwrap();
    let fr = r.segs.iter().find(|s| s.burn).unwrap();
    assert!(
        dist(fa.a, (0.0, 0.0)) <= 0.005,
        "as drawn starts at the first end: {:?}",
        fa.a
    );
    assert!(
        dist(fr.a, (100.0, 0.0)) <= 0.005,
        "reversed starts at the last end: {:?}",
        fr.a
    );
    assert!((r.burn - 100.0).abs() <= 0.01 && (a.burn - 100.0).abs() <= 0.01);
}

// ---------------------------------------------------------------- criterion 21: own options

#[test]
fn c21_own_path_options_of_a_leaf_replace_the_operations() {
    let own_lead = cut_page(
        PathOptions::DEFAULT,
        1,
        square40(),
        vec![],
        Some(lead_in_only(LeadKind::Line, 2.0)),
    );
    let r = run(&own_lead.input);
    assert!(
        (r.burn - 162.0).abs() <= 0.01,
        "the leaf's own lead-in adds 2 mm: {}",
        r.burn
    );
}

#[test]
fn c21_damaged_leaf_record_runs_nothing_and_warns() {
    let mut page = cut_page(PathOptions::DEFAULT, 1, square40(), vec![], None);
    let block = page.input.blocks.last_mut().unwrap();
    block.leaves[0].path.damaged = true;
    let r = run(&page.input);
    assert!(
        r.burn <= 0.01,
        "a damaged leaf burns nothing, burn {}",
        r.burn
    );
    assert!(
        r.issues
            .iter()
            .any(|i| i.to_lowercase().contains("damaged")),
        "the damaged-settings warning counts it: {:?}",
        r.issues
    );
}

// ---------------------------------------------------------------- criterion 24: defaults

#[test]
fn c24_defaults_add_nothing_to_the_output() {
    let plain_page = cut_page(PathOptions::DEFAULT, 1, square40(), vec![], None);
    let explicit = cut_page(PathOptions::default(), 1, square40(), vec![], None);
    let a = run(&plain_page.input);
    let b = run(&explicit.input);
    assert!(
        (a.burn - 160.0).abs() <= 0.01,
        "a plain square burns its perimeter, {}",
        a.burn
    );
    assert!(
        (a.burn - b.burn).abs() <= 0.0,
        "defaults equal the plain path"
    );
    assert!(a.travel >= 0.0);
}

// ---------------------------------------------------------------- criterion 25 and format

#[test]
fn c_format_version_is_at_least_68() {
    const { assert!(curvyo_document_core::CURRENT_FORMAT_VERSION >= 68) };
}
