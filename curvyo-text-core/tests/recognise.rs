//! Recognise text on synthetic traces (`specs/0088-trace-recognition` criteria 1 to 5 and 7,
//! `adrs.md` decision 9): strings laid out with the included faces at 5, 10 and 24 mm cap height,
//! turned 0, 7 and -30 degrees, with a deterministic wobble of up to 0.05 mm, against ground truth
//! (string, face, size, placement).
//!
//! The fast tests run in every build; `synthetic_harness` is `#[ignore]` and prints the numbers of
//! the PR (`cargo test --release -p curvyo-text-core --test recognise -- --ignored --nocapture`).
#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::unchecked_time_subtraction,
    missing_docs
)]

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::font_bytes;
use curvyo_text_core::{
    Face, FaceStore, LayoutOptions, Mm, NearestFace, RecognisedRun, Recogniser, layout,
};
use kurbo::{Affine, BezPath, PathEl, Point};

/// The included outline faces in library order, with their names.
const FACES: [(&str, &str); 13] = [
    ("Great Vibes Regular", "great-vibes/GreatVibes-Regular.ttf"),
    ("Source Sans 3 Bold", "source-sans-3/SourceSans3-Bold.otf"),
    (
        "Source Sans 3 Bold Italic",
        "source-sans-3/SourceSans3-BoldIt.otf",
    ),
    (
        "Source Sans 3 ExtraLight",
        "source-sans-3/SourceSans3-ExtraLight.otf",
    ),
    (
        "Source Sans 3 ExtraLight Italic",
        "source-sans-3/SourceSans3-ExtraLightIt.otf",
    ),
    ("Source Sans 3 Italic", "source-sans-3/SourceSans3-It.otf"),
    ("Source Sans 3 Light", "source-sans-3/SourceSans3-Light.otf"),
    (
        "Source Sans 3 Light Italic",
        "source-sans-3/SourceSans3-LightIt.otf",
    ),
    (
        "Source Sans 3 Regular",
        "source-sans-3/SourceSans3-Regular.otf",
    ),
    (
        "Source Serif 4 Bold",
        "source-serif-4/SourceSerif4-Bold.ttf",
    ),
    (
        "Source Serif 4 Bold Italic",
        "source-serif-4/SourceSerif4-BoldIt.ttf",
    ),
    (
        "Source Serif 4 Italic",
        "source-serif-4/SourceSerif4-It.ttf",
    ),
    (
        "Source Serif 4 Regular",
        "source-serif-4/SourceSerif4-Regular.ttf",
    ),
];

const SANS_REGULAR: usize = 8;
const SANS_BOLD: usize = 1;
const SERIF_REGULAR: usize = 12;
const GREAT_VIBES: usize = 0;

/// All lines matched against one tier.
#[derive(Debug, Clone, PartialEq, Default)]
struct Recognition {
    runs: Vec<RecognisedRun>,
    left_as_paths: Vec<usize>,
    nearest: Option<NearestFace>,
}

/// Every line of `outlines` matched against one tier through [`Recogniser`].
fn recognise(outlines: &[BezPath], tier: &[&Face], tolerance: Mm) -> Recognition {
    let recogniser = Recogniser::new(outlines, tolerance);
    let mut recognition = Recognition::default();
    let mut used = vec![false; outlines.len()];
    for line in 0..recogniser.line_count() {
        let found = recogniser.match_line(line, tier);
        if let Some(run) = found.run {
            for &o in &run.outlines {
                used[o] = true;
            }
            recognition.runs.push(run);
        } else if let Some(near) = found.nearest
            && recognition
                .nearest
                .is_none_or(|n| near.max_deviation.get() < n.max_deviation.get())
        {
            recognition.nearest = Some(near);
        }
    }
    recognition.left_as_paths = (0..outlines.len()).filter(|&i| !used[i]).collect();
    recognition
}

fn faces() -> Vec<Arc<Face>> {
    let mut store = FaceStore::new();
    FACES
        .iter()
        .map(|(_, path)| {
            let key = store.add(font_bytes(path), 0).unwrap();
            store.get(&key).unwrap().clone()
        })
        .collect()
}

/// A traced line: its outlines (one closed subpath each) and its truth.
struct Traced {
    outlines: Vec<BezPath>,
    placement: Affine,
}

/// The subpaths of `path`, each its own path.
fn subpaths(path: &BezPath) -> Vec<BezPath> {
    let mut out: Vec<BezPath> = Vec::new();
    for element in path.elements() {
        if let PathEl::MoveTo(_) = element {
            out.push(BezPath::new());
        }
        if let Some(last) = out.last_mut() {
            last.push(*element);
        }
    }
    out
}

/// A smooth deterministic wobble of at most `amount` mm.
fn wobble(p: Point, amount: f64) -> Point {
    Point::new(
        p.x + amount * (p.y * 1.7 + p.x * 0.3).sin(),
        p.y + amount * (p.x * 1.3 - p.y * 0.4).cos(),
    )
}

/// `text` in `face` at cap height `size`, turned `degrees` about its anchor at `at`, every
/// point moved by the wobble.
fn trace(face: &Face, text: &str, size: f64, degrees: f64, at: Point, amount: f64) -> Traced {
    let laid = layout(face, text, Mm(size), &LayoutOptions::default()).unwrap();
    let (sin, cos) = degrees.to_radians().sin_cos();
    let placement = Affine::new([cos, sin, -sin, cos, at.x, at.y]);
    let mut outlines = Vec::new();
    for line in &laid.lines {
        for glyph in &line.glyphs {
            let outline = face.outline(glyph.glyph);
            let local = Affine::translate(glyph.origin.to_vec2()) * Affine::scale(laid.mm_per_em);
            let mut path = (placement * local) * outline.path.clone();
            path.apply_affine(Affine::IDENTITY);
            let mut moved = BezPath::new();
            for element in path.elements() {
                moved.push(match *element {
                    PathEl::MoveTo(p) => PathEl::MoveTo(wobble(p, amount)),
                    PathEl::LineTo(p) => PathEl::LineTo(wobble(p, amount)),
                    PathEl::QuadTo(a, b) => PathEl::QuadTo(wobble(a, amount), wobble(b, amount)),
                    PathEl::CurveTo(a, b, c) => {
                        PathEl::CurveTo(wobble(a, amount), wobble(b, amount), wobble(c, amount))
                    }
                    PathEl::ClosePath => PathEl::ClosePath,
                });
            }
            outlines.extend(subpaths(&moved));
        }
    }
    Traced {
        outlines,
        placement,
    }
}

fn tier(all: &[Arc<Face>], pick: &[usize]) -> Vec<Arc<Face>> {
    pick.iter().map(|&i| all[i].clone()).collect()
}

fn refs(faces: &[Arc<Face>]) -> Vec<&Face> {
    faces.iter().map(AsRef::as_ref).collect()
}

/// The thread's CPU time (wall time off Unix).
fn cpu_now() -> Duration {
    #[cfg(unix)]
    {
        let t = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
        Duration::new(
            u64::try_from(t.tv_sec).unwrap(),
            u32::try_from(t.tv_nsec).unwrap(),
        )
    }
    #[cfg(not(unix))]
    {
        use std::sync::OnceLock;
        static START: OnceLock<std::time::Instant> = OnceLock::new();
        START.get_or_init(std::time::Instant::now).elapsed()
    }
}

#[test]
fn criterion_1_a_traced_line_becomes_one_text_in_its_face() {
    let all = faces();
    let traced = trace(
        &all[SANS_REGULAR],
        "Curvyo 2026",
        24.0,
        0.0,
        Point::new(20.0, 50.0),
        0.05,
    );
    let candidates = tier(&all, &[SERIF_REGULAR, SANS_BOLD, SANS_REGULAR]);
    let result = recognise(&traced.outlines, &refs(&candidates), Mm(0.3));
    assert_eq!(result.runs.len(), 1, "{result:#?}");
    let run = &result.runs[0];
    assert_eq!(run.text, "Curvyo 2026");
    assert_eq!(run.face, 2);
    assert!(run.guessed.is_empty(), "{run:#?}");
    assert!((run.size.get() - 24.0).abs() < 0.1, "{run:#?}");
    assert!(run.max_deviation.get() <= 0.3);
    let anchor = run.placement * Point::ORIGIN;
    let truth = traced.placement * Point::ORIGIN;
    assert!((anchor - truth).hypot() < 0.3, "{anchor:?} {truth:?}");
    assert_eq!(result.left_as_paths, Vec::<usize>::new());
}

/// Points along the outlines every `step` mm (kurbo's flattening: the test's own measure, not the
/// engine's).
fn points_of(paths: &[BezPath], step: f64) -> Vec<Point> {
    let mut out = Vec::new();
    for path in paths {
        let mut last = Point::ORIGIN;
        let mut start = Point::ORIGIN;
        kurbo::flatten(path, 0.001, |el| {
            let to = match el {
                PathEl::MoveTo(p) => {
                    out.push(p);
                    last = p;
                    start = p;
                    return;
                }
                PathEl::LineTo(p) => p,
                PathEl::ClosePath => start,
                _ => return,
            };
            let n = ((to - last).hypot() / step).ceil().max(1.0) as usize;
            for k in 1..=n {
                out.push(last.lerp(to, k as f64 / n as f64));
            }
            last = to;
        });
    }
    out
}

/// The symmetric Hausdorff distance of two point sets, brute force.
fn hausdorff(a: &[Point], b: &[Point]) -> f64 {
    let one = |from: &[Point], to: &[Point]| {
        from.iter()
            .map(|p| {
                to.iter()
                    .map(|q| (*p - *q).hypot2())
                    .fold(f64::INFINITY, f64::min)
            })
            .fold(0.0, f64::max)
            .sqrt()
    };
    one(a, b).max(one(b, a))
}

/// The run laid out as the document draws it, in the outlines' space.
fn laid_out(face: &Face, run: &curvyo_text_core::RecognisedRun) -> Vec<BezPath> {
    let options = LayoutOptions {
        letter_spacing: run.letter_spacing,
        word_spacing: run.word_spacing,
        ..LayoutOptions::default()
    };
    let laid = layout(face, &run.text, run.size, &options).unwrap();
    let mut out = Vec::new();
    for line in &laid.lines {
        for glyph in &line.glyphs {
            let local = Affine::translate(glyph.origin.to_vec2()) * Affine::scale(laid.mm_per_em);
            out.push((run.placement * local) * face.outline(glyph.glyph).path.clone());
        }
    }
    out
}

#[test]
fn criterion_2_the_text_lies_within_the_tolerance_everywhere() {
    let all = faces();
    let traced = trace(
        &all[SERIF_REGULAR],
        "Laser 3 mm",
        10.0,
        7.0,
        Point::new(10.0, 20.0),
        0.05,
    );
    let candidates = tier(&all, &[SANS_REGULAR, SERIF_REGULAR]);
    let result = recognise(&traced.outlines, &refs(&candidates), Mm(0.3));
    let run = &result.runs[0];
    assert_eq!(run.text, "Laser 3 mm");
    assert_eq!(run.guessed, Vec::<usize>::new());
    let drawn = points_of(&laid_out(&candidates[run.face], run), 0.05);
    let input = points_of(&traced.outlines, 0.05);
    let distance = hausdorff(&drawn, &input);
    assert!(distance <= 0.3, "{distance} mm");
    assert!(run.max_deviation.get() <= 0.3);
}

#[test]
fn criterion_3_the_nearest_face_within_the_tolerance_is_used() {
    let all = faces();
    // Source Sans 3 Regular at 5 mm; the tier holds its Light and Bold but not itself.
    let traced = trace(
        &all[SANS_REGULAR],
        "Laser 3 mm",
        5.0,
        0.0,
        Point::new(10.0, 20.0),
        0.0,
    );
    let candidates = tier(&all, &[SANS_BOLD, 6, SERIF_REGULAR]);
    let result = recognise(&traced.outlines, &refs(&candidates), Mm(0.3));
    assert_eq!(result.runs.len(), 1, "{result:#?}");
    let run = &result.runs[0];
    assert_eq!(run.face, 1, "Source Sans 3 Light is the nearest");
    assert_eq!(run.text, "Laser 3 mm");
    assert!(run.max_deviation.get() <= 0.3);
}

#[test]
fn criterion_4_no_face_within_the_tolerance_changes_nothing_and_names_the_nearest() {
    let all = faces();
    let traced = trace(
        &all[GREAT_VIBES],
        "Curvyo",
        10.0,
        0.0,
        Point::new(10.0, 20.0),
        0.05,
    );
    let candidates = tier(&all, &[SANS_REGULAR, SERIF_REGULAR]);
    let result = recognise(&traced.outlines, &refs(&candidates), Mm(0.3));
    assert!(result.runs.is_empty(), "{result:#?}");
    assert_eq!(result.left_as_paths.len(), traced.outlines.len());
    let nearest = result.nearest.unwrap();
    assert!(nearest.max_deviation.get() > 0.3);
}

/// `traced` with the outlines of glyph `index` (by order of the layout) grown by `factor`
/// about their box centre.
fn distort(
    face: &Face,
    text: &str,
    size: f64,
    at: Point,
    index: usize,
    factor: f64,
) -> Vec<BezPath> {
    let laid = layout(face, text, Mm(size), &LayoutOptions::default()).unwrap();
    let mut outlines = Vec::new();
    for (k, glyph) in laid.lines[0].glyphs.iter().enumerate() {
        let local = Affine::translate(at.to_vec2() + glyph.origin.to_vec2())
            * Affine::scale(laid.mm_per_em);
        let mut path = local * face.outline(glyph.glyph).path.clone();
        if k == index {
            let centre = kurbo::Shape::bounding_box(&path).center();
            path = (Affine::translate(centre.to_vec2())
                * Affine::scale(factor)
                * Affine::translate(-centre.to_vec2()))
                * path;
        }
        outlines.extend(subpaths(&path));
    }
    outlines
}

#[test]
fn criterion_7_an_uncertain_letter_is_guessed_and_listed() {
    let all = faces();
    // "Recognise": the fifth letter (g, position 5 one-based) drawn 12 % too large.
    let outlines = distort(
        &all[SANS_REGULAR],
        "Recognise",
        10.0,
        Point::new(10.0, 20.0),
        4,
        1.12,
    );
    let candidates = tier(&all, &[SANS_REGULAR, SERIF_REGULAR]);
    let result = recognise(&outlines, &refs(&candidates), Mm(0.3));
    assert_eq!(result.runs.len(), 1, "{result:#?}");
    let run = &result.runs[0];
    assert_eq!(run.text, "Recognise");
    assert_eq!(run.guessed, vec![4]);
    // The largest deviation covers the sure letters only.
    assert!(run.max_deviation.get() <= 0.3);
    assert_eq!(run.sure, 8);
}

#[test]
fn a_line_with_fewer_than_half_its_letters_sure_stays_paths() {
    let all = faces();
    let face = &all[SANS_REGULAR];
    // Every letter stretched 30 % across: no face draws such letters.
    let laid = layout(face, "Laser", Mm(10.0), &LayoutOptions::default()).unwrap();
    let mut outlines = Vec::new();
    for glyph in &laid.lines[0].glyphs {
        let local = Affine::translate(glyph.origin.to_vec2())
            * Affine::scale_non_uniform(laid.mm_per_em * 1.3, laid.mm_per_em);
        outlines.extend(subpaths(&(local * face.outline(glyph.glyph).path.clone())));
    }
    let result = recognise(&outlines, &refs(&tier(&all, &[SANS_REGULAR])), Mm(0.3));
    assert!(result.runs.is_empty(), "{result:#?}");
    assert_eq!(result.left_as_paths.len(), outlines.len());
}

#[test]
fn the_same_input_gives_the_same_result() {
    let all = faces();
    let traced = trace(
        &all[SANS_BOLD],
        "Bäckerei 42",
        8.0,
        -30.0,
        Point::new(10.0, 20.0),
        0.05,
    );
    let candidates = tier(&all, &[SANS_BOLD, SERIF_REGULAR]);
    let first = recognise(&traced.outlines, &refs(&candidates), Mm(0.3));
    let second = recognise(&traced.outlines, &refs(&candidates), Mm(0.3));
    assert_eq!(first, second);
    assert_eq!(first.runs[0].text, "Bäckerei 42");
}

/// The letters of `got` that are right, by their longest common subsequence with `truth`
/// (spaces left out), and the sure letters that are wrong.
fn score(truth: &str, got: &str, guessed: &[usize]) -> (usize, usize) {
    let t: Vec<char> = truth.chars().filter(|c| *c != ' ').collect();
    let g: Vec<(usize, char)> = got.chars().enumerate().filter(|(_, c)| *c != ' ').collect();
    let mut table = vec![vec![0_usize; g.len() + 1]; t.len() + 1];
    for i in 0..t.len() {
        for j in 0..g.len() {
            table[i + 1][j + 1] = if t[i] == g[j].1 {
                table[i][j] + 1
            } else {
                table[i][j + 1].max(table[i + 1][j])
            };
        }
    }
    let mut matched = vec![false; g.len()];
    let (mut i, mut j) = (t.len(), g.len());
    while i > 0 && j > 0 {
        if t[i - 1] == g[j - 1].1 {
            matched[j - 1] = true;
            i -= 1;
            j -= 1;
        } else if table[i - 1][j] >= table[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    let wrong_sure = g
        .iter()
        .zip(&matched)
        .filter(|((position, _), ok)| !**ok && !guessed.contains(position))
        .count();
    (table[t.len()][g.len()], wrong_sure)
}

#[derive(Debug, Default, Clone, Copy)]
struct Totals {
    cases: usize,
    letters: usize,
    right: usize,
    wrong_sure: usize,
    guessed: usize,
    lines_left: usize,
    face_right: usize,
    size_error_max: f64,
    deviation_max: f64,
    cpu_max: Duration,
    cpu_sum: Duration,
}

const STRINGS: [&str; 4] = [
    "Curvyo 2026",
    "B\u{e4}ckerei M\u{fc}ller",
    "Hello, World!",
    "Laser 3 mm",
];
const SIZES: [f64; 3] = [5.0, 10.0, 24.0];
const ANGLES: [f64; 3] = [0.0, 7.0, -30.0];
const TRUTH_FACES: [usize; 5] = [SANS_REGULAR, SANS_BOLD, SERIF_REGULAR, 11, GREAT_VIBES];

fn run_set(all: &[Arc<Face>], tolerance: Mm) -> (Totals, Vec<(String, Totals)>, Vec<Recognition>) {
    let tier_faces = refs(all);
    let mut totals = Totals::default();
    let mut per_face = Vec::new();
    let mut results = Vec::new();
    for &truth_face in &TRUTH_FACES {
        let mut face_totals = Totals::default();
        for text in STRINGS {
            for size in SIZES {
                for angle in ANGLES {
                    let traced = trace(
                        &all[truth_face],
                        text,
                        size,
                        angle,
                        Point::new(30.0, 40.0),
                        0.05,
                    );
                    let start = cpu_now();
                    let result = recognise(&traced.outlines, &tier_faces, tolerance);
                    let took = cpu_now().saturating_sub(start);
                    let letters = text.chars().filter(|c| *c != ' ').count();
                    let mut t = Totals {
                        cases: 1,
                        letters,
                        cpu_max: took,
                        cpu_sum: took,
                        ..Totals::default()
                    };
                    match result.runs.as_slice() {
                        [run] => {
                            let (right, wrong) = score(text, &run.text, &run.guessed);
                            t.right = right;
                            t.wrong_sure = wrong;
                            t.guessed = run.guessed.len();
                            t.face_right = usize::from(run.face == truth_face);
                            t.size_error_max = (run.size.get() - size).abs();
                            t.deviation_max = run.max_deviation.get();
                            if wrong > 0 || std::env::var_os("RECOGNISE_VERBOSE").is_some() {
                                println!(
                                    "  {} {text:?} {size} mm {angle} deg -> {:?} in {} guessed {:?} wrong {wrong}",
                                    FACES[truth_face].0, run.text, FACES[run.face].0, run.guessed
                                );
                            }
                        }
                        runs => {
                            t.lines_left = 1;
                            println!(
                                "  {} {text:?} {size} mm {angle} deg -> {} runs: {:?}",
                                FACES[truth_face].0,
                                runs.len(),
                                runs.iter()
                                    .map(|r| (&r.text, FACES[r.face].0))
                                    .collect::<Vec<_>>()
                            );
                            for run in runs {
                                let (_, wrong) = score(text, &run.text, &run.guessed);
                                t.wrong_sure += wrong;
                            }
                        }
                    }
                    add(&mut face_totals, &t);
                    results.push(result);
                }
            }
        }
        add(&mut totals, &face_totals);
        per_face.push((FACES[truth_face].0.to_owned(), face_totals));
    }
    (totals, per_face, results)
}

fn add(into: &mut Totals, t: &Totals) {
    into.cases += t.cases;
    into.letters += t.letters;
    into.right += t.right;
    into.wrong_sure += t.wrong_sure;
    into.guessed += t.guessed;
    into.lines_left += t.lines_left;
    into.face_right += t.face_right;
    into.size_error_max = into.size_error_max.max(t.size_error_max);
    into.deviation_max = into.deviation_max.max(t.deviation_max);
    into.cpu_max = into.cpu_max.max(t.cpu_max);
    into.cpu_sum += t.cpu_sum;
}

fn print(label: &str, t: &Totals) {
    println!(
        "{label}: cases {}, letters right {}/{} ({:.1} %), wrong sure {}, guessed {}, lines left {}, face right {}/{}, size error max {:.3} mm, deviation max {:.3} mm, cpu max {:?}, cpu mean {:?}",
        t.cases,
        t.right,
        t.letters,
        100.0 * t.right as f64 / t.letters.max(1) as f64,
        t.wrong_sure,
        t.guessed,
        t.lines_left,
        t.face_right,
        t.cases,
        t.size_error_max,
        t.deviation_max,
        t.cpu_max,
        t.cpu_sum / u32::try_from(t.cases.max(1)).unwrap()
    );
}

#[test]
#[ignore = "the synthetic harness: release only, prints the numbers of the PR"]
fn synthetic_harness() {
    let all = faces();
    let (totals, per_face, first) = run_set(&all, Mm(0.3));
    for (name, t) in &per_face {
        print(name, t);
    }
    print("all", &totals);
    // Determinism (`adrs.md` decision 7): the same input gives the same result.
    let (_, _, second) = run_set(&all, Mm(0.3));
    assert_eq!(first, second);
    assert_eq!(totals.wrong_sure, 0, "wrongly accepted letters must stay 0");
    // The widest tolerance the bar allows lets more faces through; still no wrong letter.
    let (wide, _, _) = run_set(&all, Mm(2.0));
    print("all at 2.0 mm", &wide);
    assert_eq!(wide.wrong_sure, 0, "wrongly accepted letters at 2.0 mm");
}

/// A face that is not in the tier, across the tolerances the bar allows (0.05 to 2.00 mm): a
/// neighbouring family fits within a wide tolerance, but no wrong letter is taken as sure
/// (tester finding 2026-10-31: Source Serif 4 against the Source Sans 3 faces).
#[test]
#[ignore = "release sweep: prints the numbers of the PR"]
fn cross_family_at_every_tolerance_takes_no_wrong_letter_as_sure() {
    let all = faces();
    let sans: Vec<usize> = (0..FACES.len())
        .filter(|&i| FACES[i].0.starts_with("Source Sans"))
        .collect();
    let serif: Vec<usize> = (0..FACES.len())
        .filter(|&i| FACES[i].0.starts_with("Source Serif"))
        .collect();
    let mut wrong_total = 0;
    for (truths, tier_pick) in [(&serif, &sans), (&sans, &serif)] {
        let tier_faces = tier(&all, tier_pick);
        let tier_refs = refs(&tier_faces);
        for &truth in truths {
            for text in &STRINGS[..2] {
                let text = *text;
                for size in [3.0, 5.0, 10.0] {
                    let traced = trace(&all[truth], text, size, 0.0, Point::new(30.0, 40.0), 0.0);
                    for tolerance in [0.05, 0.1, 0.2, 0.3, 0.5, 0.8, 1.0, 1.5, 2.0] {
                        let result = recognise(&traced.outlines, &tier_refs, Mm(tolerance));
                        for run in &result.runs {
                            let (_, wrong) = score(text, &run.text, &run.guessed);
                            if wrong > 0 {
                                println!(
                                    "  {} {text:?} {size} mm at {tolerance:.2} mm -> {:?} guessed {:?}",
                                    FACES[truth].0, run.text, run.guessed
                                );
                            }
                            wrong_total += wrong;
                        }
                    }
                }
            }
        }
    }
    println!("cross family, 0.05 to 2.00 mm: wrong sure {wrong_total}");
    assert_eq!(wrong_total, 0);
}

/// `count` distinct faces (each file parsed again, so each has its own cache), cycling through
/// the included ones.
fn many_faces(count: usize) -> Vec<Arc<Face>> {
    let mut out = Vec::new();
    while out.len() < count {
        let mut store = FaceStore::new();
        for (_, path) in FACES {
            if out.len() == count {
                break;
            }
            let key = store.add(font_bytes(path), 0).unwrap();
            out.push(store.get(&key).unwrap().clone());
        }
    }
    out
}

#[test]
#[ignore = "the CPU budgets of adrs.md decision 8: release only, prints the numbers of the PR"]
fn budgets() {
    let text = "Recognise text: 30 glyphs, ok!";
    let truth = faces();
    let traced = trace(
        &truth[SANS_REGULAR],
        text,
        10.0,
        0.0,
        Point::new(10.0, 20.0),
        0.05,
    );
    let recogniser = Recogniser::new(&traced.outlines, Mm(0.3));
    assert_eq!(recogniser.line_count(), 1);
    // The included tier cold (the usual hit), then warm.
    let included = faces();
    let start = cpu_now();
    let cold = recogniser.match_line(0, &refs(&included));
    let cold_time = cpu_now() - start;
    let start = cpu_now();
    let warm = recogniser.match_line(0, &refs(&included));
    let warm_time = cpu_now() - start;
    assert_eq!(cold, warm);
    println!(
        "included tier (13 faces), {} glyphs: cold {cold_time:?}, warm {warm_time:?}, text {:?}",
        cold.letters,
        cold.run.map(|r| r.text)
    );
    println!(
        "descriptors per face: {:?}",
        (cold_time.saturating_sub(warm_time)) / 13
    );
    // A tier of 64 faces, cold and warm.
    let tier64 = many_faces(64);
    let start = cpu_now();
    let _ = recogniser.match_line(0, &refs(&tier64));
    let cold64 = cpu_now() - start;
    let start = cpu_now();
    let found = recogniser.match_line(0, &refs(&tier64));
    let warm64 = cpu_now() - start;
    println!(
        "64 faces: cold {cold64:?}, warm {warm64:?}, sure {} of {}",
        found.sure, found.letters
    );
}

/// `specs/0091` decision 4: a misread umlaut in the OCR hint is fitted right by the matcher; the
/// hint narrows the characters, the outlines decide.
#[test]
fn a_hint_with_a_misread_letter_is_fitted_right() {
    let all = faces();
    let traced = trace(
        &all[SANS_BOLD],
        "BÄCKEREI",
        24.0,
        0.0,
        Point::new(20.0, 50.0),
        0.05,
    );
    let candidates = tier(&all, &[SERIF_REGULAR, SANS_BOLD, SANS_REGULAR]);
    let recogniser = Recogniser::with_hint(&traced.outlines, Mm(0.3), "BÂCKEREI");
    assert_eq!(recogniser.line_count(), 1);
    let found = recogniser.match_line(0, &refs(&candidates));
    let run = found.run.unwrap();
    assert_eq!(run.text, "BÄCKEREI");
    assert!(run.guessed.is_empty(), "{run:#?}");
}

/// `specs/0091` Frage 114 B: `fit_text` sets the string over its own outlines within the
/// tolerance in the same face, rotated with the line, every letter guessed.
#[test]
fn fit_text_lays_the_string_over_the_outlines() {
    let all = faces();
    for degrees in [0.0, 15.0] {
        let traced = trace(
            &all[SANS_REGULAR],
            "Open daily",
            10.0,
            degrees,
            Point::new(20.0, 50.0),
            0.0,
        );
        let recogniser = Recogniser::with_hint(&traced.outlines, Mm(0.3), "Open daily");
        let direction =
            (traced.placement * Point::new(1.0, 0.0)) - (traced.placement * Point::ORIGIN);
        let run = recogniser
            .fit_text(&all[SANS_REGULAR], "Open daily", direction)
            .unwrap();
        assert_eq!(run.guessed, vec![0, 1, 2, 3, 5, 6, 7, 8, 9]);
        assert!((run.size.get() - 10.0).abs() < 0.2, "{run:#?}");
        let d = hausdorff(
            &points_of(&traced.outlines, 0.2),
            &points_of(&laid_out(&all[SANS_REGULAR], &run), 0.2),
        );
        assert!(d < 0.3, "{degrees}: {d}");
    }
}
