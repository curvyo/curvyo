//! CPU budgets of Find centre lines (`specs/0117-centre-lines` criterion 18, ADR 0063 §10): the
//! native gates are half the wasm targets. The traced line-art fixture (174 nodes) is tiled to
//! about 5,000 nodes (Apply ≤ 0.5 s, an option change, which reruns only the finish, ≤ 0.2 s: the
//! slider-change gate is not half of the wasm 0.3 s, see `preview_budgets`) and to about 100,000 nodes (Apply ≤ 7.5 s, release builds only:
//! `cargo test --release -p curvyo-editor-wasm --test centre_lines_cpu_time`).
//! Debug builds get twelve times the budget, as the geometry crate's budget tests do.

#![allow(clippy::unwrap_used, missing_docs)]

mod cpu_time;

use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_document_core::{Length, Point, Vec2};
use curvyo_geometry_core::{CentreOptions, CentreSkeleton, Outline};

const WHALE: &str =
    include_str!("../../curvyo-geometry-core/tests/fixtures/centre_lines/whale_traced.txt");
const DOLPHIN: &str =
    include_str!("../../curvyo-geometry-core/tests/fixtures/centre_lines/dolphin_traced.txt");

type Triple = (Point, Vec2, Vec2);

fn whale(dx: f64, dy: f64) -> Vec<Vec<(Vec<Triple>, bool)>> {
    fixture(WHALE, dx, dy)
}

fn fixture(text: &str, dx: f64, dy: f64) -> Vec<Vec<(Vec<Triple>, bool)>> {
    let mut subjects: Vec<Vec<(Vec<Triple>, bool)>> = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.first() {
            Some(&"subject") => subjects.push(Vec::new()),
            Some(&"outline") => subjects
                .last_mut()
                .unwrap()
                .push((Vec::new(), words[1] == "true")),
            Some(_) => {
                let v: Vec<f64> = words.iter().map(|w| w.parse().unwrap()).collect();
                subjects.last_mut().unwrap().last_mut().unwrap().0.push((
                    Point::new(v[0] + dx, v[1] + dy),
                    Vec2::new(v[2], v[3]),
                    Vec2::new(v[4], v[5]),
                ));
            }
            None => {}
        }
    }
    subjects
}

/// `copies` whales side by side, one subject each.
fn herd(copies: usize) -> Vec<Vec<(Vec<Triple>, bool)>> {
    (0..copies)
        .flat_map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let (x, y) = ((i % 10) as f64 * 160.0, (i / 10) as f64 * 100.0);
            whale(x, y)
        })
        .collect()
}

fn nodes(subjects: &[Vec<(Vec<Triple>, bool)>]) -> usize {
    subjects.iter().flatten().map(|(a, _)| a.len()).sum()
}

fn build(subjects: &[Vec<(Vec<Triple>, bool)>]) -> Vec<CentreSkeleton> {
    subjects
        .iter()
        .map(|s| {
            let outlines: Vec<Outline<'_>> = s.iter().map(|(a, c)| Outline::new(a, *c)).collect();
            CentreSkeleton::build(&outlines).unwrap()
        })
        .collect()
}

fn finish(skeletons: &[CentreSkeleton], options: &CentreOptions) -> usize {
    skeletons
        .iter()
        .map(|s| s.finish(options, &[]).paths.len())
        .sum()
}

const fn budget(native_ms: u64) -> Duration {
    Duration::from_millis(if cfg!(debug_assertions) {
        12 * native_ms
    } else {
        native_ms
    })
}

#[test]
fn criterion_18_apply_and_an_option_change_on_5000_nodes() {
    let subjects = herd(29);
    let count = nodes(&subjects);
    let ((skeletons, paths), apply) = cpu_time_of(|| {
        let skeletons = build(&subjects);
        let paths = finish(&skeletons, &CentreOptions::default());
        (skeletons, paths)
    });
    let options = CentreOptions {
        overlap: -0.1,
        thick_parts: Some(curvyo_document_core::Length::from_mm(5.0)),
        inner_offset: Some(Length::from_mm(0.3)),
        ..CentreOptions::default()
    };
    let (_, change) = cpu_time_of(|| finish(&skeletons, &options));
    eprintln!("{count} nodes, {paths} paths: Apply {apply:?}, option change {change:?} thread CPU");
    assert!(apply <= budget(500), "Apply {apply:?}");
    assert!(change <= budget(200), "option change {change:?}");
}

#[test]
#[ignore = "release build only: 6.1 s CPU for Apply of 99,876 nodes since round A of 2026-11-18 (gate 7.5 s; it was 9.2 s with the fat parts)"]
fn criterion_18_apply_on_100000_nodes() {
    let subjects = herd(574);
    let count = nodes(&subjects);
    let (paths, apply) = cpu_time_of(|| finish(&build(&subjects), &CentreOptions::default()));
    eprintln!("{count} nodes, {paths} paths: Apply {apply:?} thread CPU");
    assert!(apply <= Duration::from_millis(7_500), "Apply {apply:?}");
}

/// The herd as an SVG of filled compound paths, in mm.
#[allow(clippy::many_single_char_names)] // point, handles and count of the SVG segment
fn svg(subjects: &[Vec<(Vec<Triple>, bool)>]) -> String {
    use std::fmt::Write as _;
    let mut out = String::from(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="1700mm" height="6000mm" viewBox="0 0 1700 6000">"#,
    );
    for subject in subjects {
        out.push_str(r#"<path fill="black" d=""#);
        for (anchors, _) in subject {
            let n = anchors.len();
            let _ = write!(out, "M{} {}", anchors[0].0.x, anchors[0].0.y);
            for i in 0..n {
                let (p, _, o) = anchors[i];
                let (q, h, _) = anchors[(i + 1) % n];
                let _ = write!(
                    out,
                    "C{} {} {} {} {} {}",
                    p.x + o.x,
                    p.y + o.y,
                    q.x + h.x,
                    q.y + h.y,
                    q.x,
                    q.y
                );
            }
            out.push('Z');
        }
        out.push_str(r#""/>"#);
    }
    out.push_str("</svg>");
    out
}

/// ADR 0063 Consequences: the main thread's part of Apply at the interim 20,000-node limit, the
/// commit of the result (native gate 50 ms per call). Apply here computes and commits on one
/// thread, so the commit is Apply minus the kernel's time for the same subjects.
#[test]
#[ignore = "measurement for docs/technical-debt.md, release build: about 30 s"]
fn criterion_18_the_commit_of_20000_nodes() {
    use curvyo_editor_wasm::{Session, Tool};
    // The import keeps a closing node per outline: 107 copies are 19,902 nodes there.
    let subjects = herd(107);
    let mut session = Session::new(1);
    session.import_begin(None);
    session.import_add(svg(&subjects).as_bytes(), "herd.svg");
    let _ = session.import_finish(true);
    session.set_tool(Tool::Select);
    let (_, kernel) = cpu_time_of(|| finish(&build(&subjects), &CentreOptions::default()));
    let (outcome, apply) = cpu_time_of(|| session.apply_centre_lines(CentreOptions::default()));
    eprintln!(
        "{outcome:?}: Apply in the session {apply:?}, kernel {kernel:?}, commit about {:?}",
        apply.saturating_sub(kernel)
    );
    // With Keep inner areas on every other subject is a cut-out candidate (ADR 0063 §7); only
    // those whose box meets the subject's are offset.
    let mut session = Session::new(1);
    session.import_begin(None);
    session.import_add(svg(&subjects).as_bytes(), "herd.svg");
    let _ = session.import_finish(true);
    session.set_tool(Tool::Select);
    let inner = CentreOptions {
        inner_offset: Some(Length::from_mm(0.3)),
        ..CentreOptions::default()
    };
    let (outcome, apply) = cpu_time_of(|| session.apply_centre_lines(inner));
    eprintln!("with inner areas {outcome:?}: Apply in the session {apply:?}");
}

/// One large region of about 20,000 nodes: a jagged spiral stroke (8 turns, 1.5 mm wide, outline
/// noise ±0.05 mm), as a single filled polygon.
fn spiral(nodes: usize) -> Vec<Triple> {
    let half = nodes / 2;
    let side = |offset: f64, k: usize| {
        #[allow(clippy::cast_precision_loss)]
        let t = k as f64 / half as f64 * 8.0 * std::f64::consts::TAU;
        let r = 10.0 + 4.0 * t / std::f64::consts::TAU + offset;
        #[allow(clippy::cast_precision_loss)]
        let noise = 0.05 * ((k as f64 * 12.9898).sin() * 43_758.545).fract();
        let r = r + noise;
        (
            Point::new(200.0 + r * t.cos(), 200.0 + r * t.sin()),
            Vec2::ZERO,
            Vec2::ZERO,
        )
    };
    let mut ring: Vec<Triple> = (0..half).map(|k| side(0.75, k)).collect();
    ring.extend((0..half).rev().map(|k| side(-0.75, k)));
    ring
}

/// The interim main-thread Apply (ADR 0063 amendment 2026-11-16): kernel and commit of one real
/// subject at the 20,000-node limit, in the session.
#[test]
#[ignore = "measurement for adrs.md and docs/technical-debt.md, release build"]
fn criterion_18_session_apply_of_one_20000_node_subject() {
    use curvyo_editor_wasm::{Session, Tool};
    let ring = spiral(19_990);
    let subjects = vec![vec![(ring.clone(), true)]];
    let mut session = Session::new(1);
    session.import_begin(None);
    session.import_add(svg(&subjects).as_bytes(), "spiral.svg");
    let _ = session.import_finish(true);
    session.set_tool(Tool::Select);
    let (paths, kernel) = cpu_time_of(|| finish(&build(&subjects), &CentreOptions::default()));
    let (outcome, apply) = cpu_time_of(|| session.apply_centre_lines(CentreOptions::default()));
    eprintln!(
        "{outcome:?}, {paths} paths: Apply in the session {apply:?}, kernel {kernel:?}, commit about {:?}",
        apply.saturating_sub(kernel)
    );
}

/// ADR 0063 amendment 2026-11-18, A step 1: where the main thread's commit of a 100,000-node
/// result goes (release build).
#[test]
#[ignore = "measurement for adrs.md, release build"]
fn criterion_18_the_commit_of_100000_nodes() {
    use curvyo_editor_wasm::{Session, Tool};
    // The import keeps a closing node per outline: 537 copies are 99,882 nodes there.
    let subjects = herd(
        std::env::var("COPIES")
            .ok()
            .and_then(|c| c.parse().ok())
            .unwrap_or(537),
    );
    let mut session = Session::new(1);
    session.import_begin(None);
    session.import_add(svg(&subjects).as_bytes(), "herd.svg");
    let _ = session.import_finish(true);
    session.set_tool(Tool::Select);
    let (_, kernel) = cpu_time_of(|| finish(&build(&subjects), &CentreOptions::default()));
    let (outcome, apply) = cpu_time_of(|| session.apply_centre_lines(CentreOptions::default()));
    eprintln!(
        "{outcome:?}: Apply in the session {apply:?}, kernel {kernel:?}, commit about {:?}",
        apply.saturating_sub(kernel)
    );
}

/// The preview in the worker (ADR 0063 amendment 2026-11-18, C): the first preview (subjects,
/// finish and painted outlines) and each slider's change, native thread CPU; gates at half the
/// wasm limits (0.5 s first preview). The native slider-change gate is 0.2 s, not half of the
/// wasm 0.3 s (Proposal, ADR 0063 §10): CI runners are about 25 % slower than the dev machine
/// (whale x29 Optimise 0.115 s locally, 0.177 s on CI, 0.15 s was too tight) and the wasm 0.3 s,
/// which holds at 0.17 s, is the binding limit. Prints the per-slider numbers and message sizes.
fn preview_budgets(name: &str, subjects: &[Vec<(Vec<Triple>, bool)>]) -> (Duration, Duration) {
    use curvyo_editor_wasm::centre_lines_glue::CentreLinesWorker;
    use curvyo_editor_wasm::centre_lines_wire::encode_subjects;
    let wire = encode_subjects(subjects);
    let mut worker = CentreLinesWorker::new();
    let base = CentreOptions::default();
    let (preview, first) = cpu_time_of(|| {
        worker.set_subjects(&wire, &mut |_| {}).unwrap();
        worker.finish(&base, true)
    });
    let steps: [(&str, CentreOptions); 6] = [
        (
            "Overlap",
            CentreOptions {
                overlap: 0.1,
                ..base
            },
        ),
        (
            "Thick parts",
            CentreOptions {
                overlap: 0.1,
                thick_parts: Some(Length::from_mm(4.0)),
                ..base
            },
        ),
        (
            "Optimise",
            CentreOptions {
                overlap: 0.1,
                thick_parts: Some(Length::from_mm(4.0)),
                optimise: 0.6,
                ..base
            },
        ),
        (
            "Keep inner areas",
            CentreOptions {
                overlap: 0.1,
                thick_parts: Some(Length::from_mm(4.0)),
                optimise: 0.6,
                inner_offset: Some(Length::from_mm(0.3)),
                join_tips: true,
            },
        ),
        (
            "Offset",
            CentreOptions {
                overlap: 0.1,
                thick_parts: Some(Length::from_mm(4.0)),
                optimise: 0.6,
                inner_offset: Some(Length::from_mm(0.5)),
                join_tips: true,
            },
        ),
        (
            "Join tips",
            CentreOptions {
                overlap: 0.1,
                thick_parts: Some(Length::from_mm(4.0)),
                optimise: 0.6,
                inner_offset: Some(Length::from_mm(0.5)),
                join_tips: false,
            },
        ),
    ];
    let mut worst = Duration::ZERO;
    for (slider, options) in steps {
        let (_, took) = cpu_time_of(|| worker.finish(&options, true));
        eprintln!("{name}: {slider} {took:?}");
        worst = worst.max(took);
    }
    let result = worker.result();
    eprintln!(
        "{name}: {} nodes, first preview {first:?}; subjects {} KB, preview {} KB, result {} KB",
        nodes(subjects),
        wire.len() * 8 / 1024,
        preview.len() * 8 / 1024,
        result.len() * 8 / 1024
    );
    (first, worst)
}

#[test]
fn criterion_18_the_preview_of_the_whale_and_the_dolphin_in_the_worker() {
    for (name, subjects) in [
        ("whale x29", herd(29)),
        ("dolphin", fixture(DOLPHIN, 0.0, 0.0)),
    ] {
        let (first, change) = preview_budgets(name, &subjects);
        assert!(first <= budget(500), "{name}: first preview {first:?}");
        assert!(change <= budget(200), "{name}: a slider change {change:?}");
    }
}

/// The 100,000-node preview and the main thread's Apply from it (ADR 0063 amendment 2026-11-18,
/// A and C): the worker's first preview, the message sizes, each Apply slice and the one batch.
#[test]
#[ignore = "measurement for adrs.md, release build"]
fn criterion_18_the_preview_and_apply_of_100000_nodes() {
    use curvyo_editor_wasm::centre_lines_glue::CentreLinesWorker;
    use curvyo_editor_wasm::{Session, Tool};
    let subjects = herd(537);
    let _ = preview_budgets("herd x537", &subjects);
    let mut session = Session::new(1);
    session.import_begin(None);
    session.import_add(svg(&subjects).as_bytes(), "herd.svg");
    let _ = session.import_finish(true);
    session.set_tool(Tool::Node);
    let _ = session.frame_draw_list();
    let (_, frame_before) = cpu_time_of(|| session.frame_draw_list());
    let (wire, open) = cpu_time_of(|| session.centre_lines_open().unwrap());
    // The subjects' wire for a measurement of the worker's memory in Node (by hand).
    if let Ok(path) = std::env::var("CURVYO_CENTRE_WIRE_OUT") {
        let bytes: Vec<u8> = wire.iter().flat_map(|v| v.to_le_bytes()).collect();
        std::fs::write(path, bytes).unwrap();
    }
    let mut worker = CentreLinesWorker::new();
    worker.set_subjects(&wire, &mut |_| {}).unwrap();
    let preview = worker.finish(&CentreOptions::default(), true);
    // ADR 0063 amendment 2026-11-18, W: one message at most 64 MB at 100,000 nodes.
    let preview_mb = preview.len() * 8 / 1_000_000;
    assert!(preview_mb < 64, "the preview message is {preview_mb} MB");
    let (_, show) = cpu_time_of(|| session.centre_lines_show(&preview));
    // The main thread's first frame with the preview (its tessellation), then a second one.
    let (_, first_preview_frame) = cpu_time_of(|| session.frame_draw_list());
    let (_, next_preview_frame) = cpu_time_of(|| session.frame_draw_list());
    let result = worker.result();
    let (_, begin) = cpu_time_of(|| session.centre_lines_apply_begin(&result).unwrap());
    let mut slices = Vec::new();
    let outcome = loop {
        let (outcome, took) = cpu_time_of(|| session.centre_lines_apply_step());
        slices.push(took);
        if let Some(outcome) = outcome {
            break outcome;
        }
    };
    let batch = slices.pop().unwrap_or_default();
    let (_, first_frame_after) = cpu_time_of(|| session.frame_draw_list());
    let (_, next_frame_after) = cpu_time_of(|| session.frame_draw_list());
    eprintln!(
        "{outcome:?}: open {open:?}, show {show:?}, begin {begin:?}, {} slices, the longest {:?}, the batch {batch:?}",
        slices.len(),
        slices.iter().max()
    );
    eprintln!(
        "main thread: a frame before {frame_before:?}; preview {preview_mb} MB, first preview \
         frame {first_preview_frame:?}, next {next_preview_frame:?}; first frame after Apply \
         {first_frame_after:?}, next {next_frame_after:?}"
    );
}
