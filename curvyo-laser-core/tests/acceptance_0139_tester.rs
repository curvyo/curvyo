//! Tester acceptance cases for `specs/0139-laser-cut-path-options` slice 1 (tabs), written from the
//! specification: the job's GRBL read back by the tester's own reader (`tester_grbl_0029`), so the
//! gaps, the pieces and the burn and travel lengths are measured on the file, not on the generator.
//! Covered here: criteria 1 (value ranges), 2 (placement and the k rule), 3 (skip inner shapes), 8
//! (gap lengths on the file), 9 (order and passes), 10 (own tabs cover a path), 11 (damaged own
//! tabs), 24 (defaults change nothing). Not covered here: the editor (handles, Tabs tool, Operations
//! view, undo names, copies, Duplicate, save and reopen): they need the browser or the wasm layer.
//!
//! A gap is a laser-off travel move between two burnt moves of at least 0.05 mm. A piece of a
//! closed path that wraps through its entry point is burnt as two runs with no gap between them;
//! `closed_pieces` joins them, so a piece is counted once.

#![allow(
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    missing_docs
)]

mod job_fixtures;
mod tester_grbl_0029;

use std::panic::{AssertUnwindSafe, catch_unwind};

use curvyo_document_core::Point;
use curvyo_laser_core::job::{GateFacts, LeafOutline, PlanCache, Timeline, build_job, gate, write};
use curvyo_laser_core::operations::OperationKind::Cut;
use curvyo_laser_core::settings::{CutLength, CutSettings, OperationOptions, TabCount};
use curvyo_laser_core::tabs::{LeafTabs, OwnTab, PathFraction};
use job_fixtures::{Page, circle, leaf, line, own, plain, rect};
use tester_grbl_0029::{Kind, Program, read};

/// A point in mm.
type Pt = (f64, f64);

/// The tolerance of the criteria, in mm.
const TOL: f64 = 0.005;

fn options(count: u64, width: f64, skip_inner: bool) -> OperationOptions {
    OperationOptions {
        cut: CutSettings {
            tab_count: TabCount::new(count).unwrap(),
            tab_width: CutLength::tab_of_mm(width).unwrap(),
            tab_skip_inner: skip_inner,
        },
        ..plain()
    }
}

/// One Cut operation at 80 % and 600 mm/min with `options`, holding the leaves given.
fn cut_page(options: OperationOptions, leaves: Vec<Vec<LeafOutline>>, passes: u8) -> Page {
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 80.0, 600, passes),
        options,
        leaves.into_iter().map(leaf).collect(),
    );
    page
}

/// The job built from `page`, written as GRBL, read back, and its Timeline.
fn run(page: &Page) -> (Vec<u8>, Program, Timeline) {
    let built = build_job(&page.input, &[], &mut PlanCache::default());
    let format = page.input.machine.values.output_format;
    let bytes = write(format, &built.job);
    let program = read(&bytes).unwrap();
    let timeline = Timeline::of(&built.job);
    (bytes, program, timeline)
}

fn mm(p: (i64, i64)) -> Pt {
    (p.0 as f64 / 1000.0, p.1 as f64 / 1000.0)
}

fn dist(a: Pt, b: Pt) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

fn piece_len(points: &[Pt]) -> f64 {
    points.windows(2).map(|w| dist(w[0], w[1])).sum()
}

/// The gaps of the file in file order: the laser-off travel between two burnt moves, as
/// (end of the burn before, start of the burn after).
fn gaps(p: &Program) -> Vec<(Pt, Pt)> {
    let s = &p.segs;
    let mut out = Vec::new();
    for i in 1..s.len().saturating_sub(1) {
        let (before, mid, after) = (s[i - 1], s[i], s[i + 1]);
        if mid.kind == Kind::Travel
            && before.kind == Kind::Burn
            && after.kind == Kind::Burn
            && mid.length_mm() >= 0.05
        {
            out.push((mm(mid.from), mm(mid.to)));
        }
    }
    out
}

/// The tab gaps of the file: the gaps whose length is the tab width `width` (0.005 mm), so a
/// travel between two paths of the same operation is not counted as a tab.
fn tab_gaps(p: &Program, width: f64) -> Vec<(Pt, Pt)> {
    gaps(p)
        .into_iter()
        .filter(|(a, b)| (dist(*a, *b) - width).abs() <= TOL)
        .collect()
}

/// The burnt chunks of the file: consecutive burn moves, split at any travel move.
fn chunks(p: &Program) -> Vec<Vec<Pt>> {
    let mut out: Vec<Vec<Pt>> = Vec::new();
    let mut cur: Vec<Pt> = Vec::new();
    for s in &p.segs {
        if s.kind == Kind::Burn {
            if cur.is_empty() {
                cur.push(mm(s.from));
            }
            cur.push(mm(s.to));
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// The pieces of a closed path: the chunks, with the last joined to the first when the path wraps
/// through its entry point (the last chunk ends where the first one starts).
fn closed_pieces(p: &Program) -> Vec<Vec<Pt>> {
    let mut c = chunks(p);
    if c.len() >= 2 && dist(*c[c.len() - 1].last().unwrap(), c[0][0]) <= 0.001 {
        let last = c.pop().unwrap();
        let mut joined = last;
        joined.extend_from_slice(&c[0][1..]);
        c[0] = joined;
    }
    c
}

fn burn_total(p: &Program) -> f64 {
    p.segs
        .iter()
        .filter(|s| s.kind == Kind::Burn)
        .map(tester_grbl_0029::Seg::length_mm)
        .sum()
}

// ---- criterion 1: the value ranges of the Tabs block --------------------------------------------

#[test]
fn c1_count_21_is_refused_and_0_to_20_are_accepted() {
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _ = TabCount::new(21).unwrap();
        }))
        .is_err()
    );
    let _ = TabCount::new(0).unwrap();
    let _ = TabCount::new(20).unwrap();
}

#[test]
fn c1_width_0_09_and_20_01_are_refused_and_0_1_and_20_are_accepted() {
    for refused in [0.09_f64, 20.01] {
        assert!(
            catch_unwind(AssertUnwindSafe(|| {
                let _ = CutLength::tab_of_mm(refused).unwrap();
            }))
            .is_err(),
            "width {refused} mm must be refused"
        );
    }
    let _ = CutLength::tab_of_mm(0.1).unwrap();
    let _ = CutLength::tab_of_mm(20.0).unwrap();
}

// ---- criterion 2: placement, centres, widths and the k rule -------------------------------------

/// Count 4 on a 40 mm square drawn from its bottom-left corner, width 2 mm: four gaps on the four
/// side midpoints, each 2 mm along the path.
#[test]
fn c2_square_four_tabs_sit_on_the_side_midpoints_and_are_2mm() {
    let page = cut_page(
        options(4, 2.0, false),
        vec![vec![rect(20.0, 20.0, 40.0, 40.0)]],
        1,
    );
    let (_, p, _) = run(&page);
    let g = tab_gaps(&p, 2.0);
    assert_eq!(g.len(), 4, "4 tabs on a closed path make 4 gaps");

    let all: Vec<Pt> = chunks(&p).into_iter().flatten().collect();
    let min_x = all.iter().map(|q| q.0).fold(f64::INFINITY, f64::min);
    let min_y = all.iter().map(|q| q.1).fold(f64::INFINITY, f64::min);
    let max_x = all.iter().map(|q| q.0).fold(f64::NEG_INFINITY, f64::max);
    assert!((max_x - min_x - 40.0).abs() <= TOL, "square is 40 mm wide");

    let expected = [(20.0, 0.0), (40.0, 20.0), (20.0, 40.0), (0.0, 20.0)];
    for (i, (a, b)) in g.iter().enumerate() {
        assert!(
            (dist(*a, *b) - 2.0).abs() <= TOL,
            "gap {i} is {} mm, want 2.000 +/- 0.005",
            dist(*a, *b)
        );
        let mid = (
            f64::midpoint(a.0, b.0) - min_x,
            f64::midpoint(a.1, b.1) - min_y,
        );
        assert!(
            expected
                .iter()
                .any(|e| (e.0 - mid.0).hypot(e.1 - mid.1) <= 0.01),
            "gap {i} centre {mid:?} is not on a side midpoint"
        );
    }
    for corner in [(0.0, 0.0), (40.0, 0.0), (40.0, 40.0), (0.0, 40.0)] {
        assert!(
            all.iter().any(|q| {
                (q.0 - min_x - corner.0).abs() <= 0.01 && (q.1 - min_y - corner.1).abs() <= 0.01
            }),
            "corner {corner:?} is burnt"
        );
    }
}

/// Count 1 on a 100 mm open line: one gap centred at 50 mm, so both pieces are 49 mm.
#[test]
fn c2_open_line_one_tab_is_centred_at_50mm() {
    let page = cut_page(
        options(1, 2.0, false),
        vec![vec![line(
            Point::new(20.0, 280.0),
            Point::new(120.0, 280.0),
        )]],
        1,
    );
    let (_, p, _) = run(&page);
    let c = chunks(&p);
    assert_eq!(c.len(), 2, "one tab on an open path makes two pieces");
    for piece in &c {
        assert!(
            (piece_len(piece) - 49.0).abs() <= TOL,
            "piece is {} mm, want 49.000",
            piece_len(piece)
        );
    }
}

/// A 10 mm diameter circle (31.416 mm), width 2.5 mm, Count 20: floor(31.4 / 10) = 3 tabs. The
/// burn is the circumference less three 2.5 mm arcs (the gaps are straight travel).
#[test]
fn c2_circle_count_20_places_three_tabs_and_burns_the_arc_length() {
    let page = cut_page(
        options(20, 2.5, false),
        vec![vec![circle(50.0, 50.0, 5.0)]],
        1,
    );
    let (_, p, _) = run(&page);
    assert_eq!(
        tab_gaps(&p, 2.4740).len(),
        3,
        "k = floor(31.4 / 10) = 3; a 2.5 mm arc of r 5 has a 2.4740 mm chord"
    );
    let want = 2.0 * std::f64::consts::PI * 5.0 - 3.0 * 2.5;
    let burn = burn_total(&p);
    assert!(
        (burn - want).abs() <= 0.03,
        "burn {burn} mm, want {want:.3} +/- 0.03 (flattened arc)"
    );
}

/// The k rule with the 1 mm piece limit binding: a 2.5 mm square (perimeter 10 mm) at width 0.1 mm
/// has floor(10 / 1.1) = 9 tabs, so nine pieces of 10 / 9 - 0.1 = 1.011 mm, each at least 1 mm.
#[test]
fn c2_closed_path_pieces_keep_at_least_1mm_when_the_1mm_limit_binds() {
    let page = cut_page(
        options(20, 0.1, false),
        vec![vec![rect(0.0, 0.0, 2.5, 2.5)]],
        1,
    );
    let (_, p, _) = run(&page);
    let pieces = closed_pieces(&p);
    assert_eq!(
        pieces.len(),
        9,
        "k = min(20, 25, floor(10 / 1.1)) = 9; pieces {:?}",
        pieces.iter().map(|q| piece_len(q)).collect::<Vec<_>>()
    );
    for piece in &pieces {
        assert!(
            piece_len(piece) >= 1.0 - TOL,
            "piece {} mm < 1 mm",
            piece_len(piece)
        );
    }
}

/// Criterion 2 and its "every burnt piece at least 1 mm" intent, on an open path. Spec rule as
/// written: k = min(20, floor(10 / 0.4), floor(10 / 1.1)) = 9 for a 10 mm line at 0.1 mm. The two
/// end pieces are only L / (2k) - W / 2 = 0.506 mm long, so the rule as written does not keep every
/// piece at least 1 mm on an open path. Expected to FAIL until the spec or the rule is fixed.
#[test]
fn c2_open_path_end_pieces_keep_at_least_1mm() {
    let page = cut_page(
        options(20, 0.1, false),
        vec![vec![line(Point::new(0.0, 0.0), Point::new(10.0, 0.0))]],
        1,
    );
    let (_, p, _) = run(&page);
    for piece in chunks(&p) {
        assert!(
            piece_len(&piece) >= 1.0 - TOL,
            "piece {} mm < 1 mm",
            piece_len(&piece)
        );
    }
}

// ---- criterion 3: skip inner shapes ------------------------------------------------------------

/// A plate 80 x 80 mm with a round hole of radius 10 mm, Count 4, width 2 mm. Skip on: the hole has
/// no tabs (4 gaps, the hole whole); Skip off: four tabs on the hole too (8 gaps). The burn differs
/// by exactly the hole's 8 mm of tab gaps.
#[test]
fn c3_skip_inner_gives_the_hole_no_tabs() {
    let plate_and_hole = || vec![rect(0.0, 0.0, 80.0, 80.0), circle(40.0, 40.0, 10.0)];
    let on = run(&cut_page(options(4, 2.0, true), vec![plate_and_hole()], 1)).1;
    let off = run(&cut_page(options(4, 2.0, false), vec![plate_and_hole()], 1)).1;
    assert_eq!(
        tab_gaps(&on, 2.0).len(),
        4,
        "4 gaps on the outline, none on the hole"
    );
    assert_eq!(
        tab_gaps(&off, 2.0).len(),
        8,
        "4 gaps on the outline and 4 on the hole"
    );
    let hole_whole = chunks(&on)
        .iter()
        .any(|c| (piece_len(c) - 2.0 * std::f64::consts::PI * 10.0).abs() <= 0.03);
    assert!(hole_whole, "the hole is one whole piece with Skip on");
    let diff = burn_total(&on) - burn_total(&off);
    assert!((diff - 8.0).abs() <= 0.02, "hole gaps are 8 mm, got {diff}");
}

// ---- criterion 8: the file's gaps and burn against the Timeline --------------------------------

/// A straight 100 mm line, Count 3, width 2 mm: burn 94 mm, three gaps of 2 mm, read from the file;
/// the Timeline's burn and travel change by the same amounts as the file. The file has no dwell.
#[test]
fn c8_straight_line_three_tabs_burn_94_and_gap_6() {
    let line_100 = || vec![vec![line(Point::new(0.0, 0.0), Point::new(100.0, 0.0))]];
    let (bytes, p, t) = run(&cut_page(options(3, 2.0, false), line_100(), 1));
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.lines().any(|l| l.starts_with("G4")), "no dwell");
    let g = tab_gaps(&p, 2.0);
    assert_eq!(g.len(), 3, "3 tabs make 3 gaps");
    assert_eq!(chunks(&p).len(), 4, "3 tabs on an open path make 4 pieces");
    for (i, (a, b)) in g.iter().enumerate() {
        assert!(
            (dist(*a, *b) - 2.0).abs() <= TOL,
            "gap {i} is {} mm",
            dist(*a, *b)
        );
    }
    let burn = burn_total(&p);
    assert!(
        (burn - 94.0).abs() <= 0.015,
        "burn {burn} mm, want 94.000 +/- 0.015"
    );
    let (_, plain_p, plain_t) = run(&cut_page(plain(), line_100(), 1));
    assert!((burn_total(&plain_p) - 100.0).abs() <= 0.005);
    let d_burn = t.burn_mm() - plain_t.burn_mm();
    let d_travel = t.travel_mm() - plain_t.travel_mm();
    assert!(
        (d_burn + 6.0).abs() <= 0.015,
        "Timeline burn changes by {d_burn}, want -6"
    );
    assert!(
        (d_travel - 6.0).abs() <= 0.015,
        "Timeline travel changes by {d_travel}, want +6"
    );
}

/// Criterion 8: "no S above 0 in the gap (GRBL M4)" means no beam in the gap (lead decision
/// 2026-11-27, Proposal): under M4 a G0 never fires the laser, so the modal S of the burn before it
/// may stay in force. Every gap is a G0 in a file that runs in M4.
#[test]
fn c8_gap_travel_is_a_g0_under_m4() {
    let (bytes, p, _) = run(&cut_page(
        options(3, 2.0, false),
        vec![vec![line(Point::new(0.0, 0.0), Point::new(100.0, 0.0))]],
        1,
    ));
    let text = String::from_utf8(bytes).unwrap();
    assert!(
        text.lines().any(|l| l.starts_with("M4")),
        "the file runs in M4"
    );
    assert!(
        !text.lines().any(|l| l.starts_with("M3")),
        "no constant-power M3"
    );
    let lines: Vec<&str> = text.lines().collect();
    for s in p.segs.iter().filter(|s| s.kind == Kind::Travel) {
        let line = lines[s.line - 1];
        assert!(
            line.starts_with("G0"),
            "travel at line {} is {line}",
            s.line
        );
    }
}

/// Criterion 8, every tab a gap: a plate with a hole, Count 4 on both, width 2 mm. The plate's entry
/// is the far end of its bottom tab (the nearest piece start to the head after the hole), so the
/// tab that the path wraps through has no G0 between its piece before and its piece after: the
/// file has 8 tab gaps, not 4 + 4 = 8 counted between burns. Expected to FAIL on the current output.
#[test]
fn c8_every_tab_is_one_g0_between_its_pieces() {
    let page = cut_page(
        options(4, 2.0, false),
        vec![vec![rect(0.0, 0.0, 80.0, 80.0), circle(40.0, 40.0, 10.0)]],
        1,
    );
    let (_, p, _) = run(&page);
    assert_eq!(tab_gaps(&p, 2.0).len(), 8, "every tab is a G0 of 2 mm");
}

// ---- criterion 9: order and passes -------------------------------------------------------------

/// A plate with a hole, tabs on both: the hole's four gaps come before the plate's four, so the
/// hole's pieces are cut first (inner before outer).
#[test]
fn c9_the_hole_pieces_run_before_the_plate_pieces() {
    let page = cut_page(
        options(4, 2.0, false),
        vec![vec![rect(0.0, 0.0, 80.0, 80.0), circle(40.0, 40.0, 10.0)]],
        1,
    );
    let (_, p, _) = run(&page);
    let g = tab_gaps(&p, 2.0);
    assert_eq!(g.len(), 8, "4 hole gaps and 4 plate gaps; got {g:?}");
    let all: Vec<Pt> = chunks(&p).into_iter().flatten().collect();
    let min_x = all.iter().map(|q| q.0).fold(f64::INFINITY, f64::min);
    let min_y = all.iter().map(|q| q.1).fold(f64::INFINITY, f64::min);
    let centre = (min_x + 40.0, min_y + 40.0);
    for (a, b) in &g[0..4] {
        for q in [a, b] {
            let r = dist(*q, centre);
            assert!((r - 10.0).abs() <= 0.01, "hole gap end at radius {r}");
        }
    }
    for (a, b) in &g[4..8] {
        for q in [a, b] {
            let m = (q.0 - centre.0).abs().max((q.1 - centre.1).abs());
            assert!(
                (m - 40.0).abs() <= 0.01,
                "plate gap end at {m} from the centre"
            );
        }
    }
}

/// Two passes: every pass writes the same gaps at the same coordinates, and the burn is twice one
/// pass.
#[test]
fn c9_two_passes_repeat_the_same_gaps() {
    let page = cut_page(
        options(4, 2.0, false),
        vec![vec![rect(20.0, 20.0, 40.0, 40.0)]],
        2,
    );
    let (_, p, t) = run(&page);
    let g = tab_gaps(&p, 2.0);
    assert_eq!(g.len(), 8, "4 gaps per pass, 2 passes");
    for i in 0..4 {
        for (a, b) in [(g[i].0, g[i + 4].0), (g[i].1, g[i + 4].1)] {
            assert!(
                dist(a, b) <= TOL,
                "pass 2 gap {i} is {} mm from pass 1's",
                dist(a, b)
            );
        }
    }
    let (_, one_p, one_t) = run(&cut_page(
        options(4, 2.0, false),
        vec![vec![rect(20.0, 20.0, 40.0, 40.0)]],
        1,
    ));
    assert!(burn_total(&one_p) > 150.0, "one pass burns 152 mm");
    assert!(
        (t.burn_mm() - 2.0 * one_t.burn_mm()).abs() <= 0.015,
        "two passes burn twice"
    );
}

// ---- criterion 10: own tabs that cover a whole path --------------------------------------------

/// A 3 mm line whose own tab (3 mm wide, at its middle) covers all of it, next to a 10 mm square
/// that burns. The Checks show one warn line for the covered path; the square still runs.
#[test]
fn c10_a_path_covered_by_own_tabs_is_one_warning_and_cuts_nothing() {
    let mut page = cut_page(
        plain(),
        vec![
            vec![line(Point::new(10.0, 10.0), Point::new(13.0, 10.0))],
            vec![rect(30.0, 30.0, 10.0, 10.0)],
        ],
        1,
    );
    let tab = OwnTab {
        at: PathFraction::of(0.5),
        width: Some(CutLength::tab_of_mm(3.0).unwrap()),
    };
    page.input.blocks[0].leaves[0].cut = LeafTabs::Own(vec![(0, vec![tab])]);
    let built = build_job(&page.input, &[], &mut PlanCache::default());
    let timeline = Timeline::of(&built.job);
    assert!(
        (timeline.burn_mm() - 40.0).abs() <= 0.01,
        "only the square burns"
    );
    let checks = gate(&page.input, &built, &timeline, &GateFacts::default());
    let covered: Vec<_> = checks
        .issues
        .iter()
        .filter(|i| format!("{:?}", i.cause).contains("Tab"))
        .collect();
    assert_eq!(covered.len(), 1, "one warn line for the covered path");
    assert_eq!(covered[0].count, 1, "the count is 1 path");
}

// ---- criterion 11: damaged own tabs ------------------------------------------------------------

/// Two own tabs that overlap (0.30 and 0.31 of a 100 mm line, 5 mm wide): the item runs nothing in
/// that operation, so the file has no burnt move at all.
#[test]
fn c11_overlapping_own_tabs_make_the_item_run_nothing() {
    let mut page = cut_page(
        plain(),
        vec![vec![line(Point::new(0.0, 0.0), Point::new(100.0, 0.0))]],
        1,
    );
    let tab = |f: f64| OwnTab {
        at: PathFraction::of(f),
        width: Some(CutLength::tab_of_mm(5.0).unwrap()),
    };
    page.input.blocks[0].leaves[0].cut = LeafTabs::Own(vec![(0, vec![tab(0.30), tab(0.31)])]);
    let (_, p, t) = run(&page);
    assert!(chunks(&p).is_empty(), "no burnt move");
    assert!(t.burn_mm().abs() <= 1e-9, "Timeline burn is 0");
}

// ---- criterion 24: the defaults change nothing -------------------------------------------------

/// Count 0 and the default width write the same bytes as an operation with no tabs at all.
#[test]
fn c24_count_0_writes_the_same_bytes_as_no_tabs() {
    let square = || vec![vec![rect(20.0, 20.0, 40.0, 40.0)]];
    let (a, _, _) = run(&cut_page(plain(), square(), 1));
    let (b, _, _) = run(&cut_page(options(0, 0.5, false), square(), 1));
    assert_eq!(a, b, "defaults are byte-identical to today");
}
