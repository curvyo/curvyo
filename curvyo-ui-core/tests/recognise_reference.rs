//! The chain harness of Recognise text (`specs/0088-trace-recognition` criteria 1 and 3,
//! `adrs.md` decision 9): text rendered to pixels by a test-only even-odd scanline filler at 300
//! and 600 dpi, traced with the Logo preset, recognised over the included faces as one tier and
//! scored against the truth; a hold-out run without the true family; and the customer's
//! reference logos with text (`tests/assets/trace-references/`), traced whole and selected whole.
//! `#[ignore]`: release only, prints the numbers of the PR. The one hard rule: no wrong letter is
//! accepted as sure.
#![allow(clippy::unwrap_used, missing_docs, clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss, clippy::cast_precision_loss)]

use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::FontKind;
use curvyo_text_core::{Face, FaceStore, LayoutOptions, Mm, layout};
use curvyo_ui_core::{FaceState, FontBlock, JobStep, ListedFace, RecogniseJob, recognise_tiers};
use curvyo_vectorize_core::{PRESETS, Raster, TraceParams, auto_threshold, trace};
use kurbo::{Affine, BezPath, PathEl, Point};

const FACES: &[(&str, &str, &str)] = &[
    (
        "Source Sans 3",
        "Regular",
        "source-sans-3/SourceSans3-Regular.otf",
    ),
    (
        "Source Sans 3",
        "Bold",
        "source-sans-3/SourceSans3-Bold.otf",
    ),
    (
        "Source Sans 3",
        "Light",
        "source-sans-3/SourceSans3-Light.otf",
    ),
    (
        "Source Serif 4",
        "Regular",
        "source-serif-4/SourceSerif4-Regular.ttf",
    ),
    (
        "Source Serif 4",
        "Italic",
        "source-serif-4/SourceSerif4-It.ttf",
    ),
    (
        "Source Serif 4",
        "Bold",
        "source-serif-4/SourceSerif4-Bold.ttf",
    ),
    (
        "Great Vibes",
        "Regular",
        "great-vibes/GreatVibes-Regular.ttf",
    ),
];

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

struct Library {
    listed: Vec<ListedFace>,
    faces: Vec<Arc<Face>>,
}

fn library() -> Library {
    let mut store = FaceStore::new();
    let mut listed = Vec::new();
    let mut faces = Vec::new();
    for (family, style, file) in FACES {
        let bytes = std::fs::read(repo().join("frontend/public/fonts").join(file)).unwrap();
        let key = store.add(bytes, 0).unwrap();
        faces.push(store.get(&key).unwrap().clone());
        listed.push(ListedFace {
            block: FontBlock::Included,
            family: (*family).to_owned(),
            style: (*style).to_owned(),
            host_id: (*file).to_owned(),
            index: 0,
            key: Some(key),
            kind: FontKind::Outline,
        });
    }
    Library { listed, faces }
}

/// `text` laid out in `face` at cap height `size` mm with its first baseline at `at`.
fn rendered(face: &Face, text: &str, size: f64, at: Point) -> Vec<BezPath> {
    let laid = layout(face, text, Mm(size), &LayoutOptions::default()).unwrap();
    let mut out = Vec::new();
    for glyph in &laid.lines[0].glyphs {
        let local = Affine::translate((at.x + glyph.origin.x, at.y + glyph.origin.y))
            * Affine::scale(laid.mm_per_em);
        out.push(local * face.outline(glyph.glyph).path.clone());
    }
    out
}

/// The paths filled even-odd into white RGBA with black ink, 4 x 4 samples a pixel; `map` takes
/// pixel corners to millimetres.
fn rasterised(
    paths: &[BezPath],
    width: u32,
    height: u32,
    mm_per_px: f64,
    origin: Point,
) -> Vec<u8> {
    let mut edges: Vec<(Point, Point)> = Vec::new();
    for path in paths {
        let mut start = Point::ORIGIN;
        let mut last = Point::ORIGIN;
        kurbo::flatten(path.elements().iter().copied(), 0.002, |el| match el {
            PathEl::MoveTo(p) => {
                start = p;
                last = p;
            }
            PathEl::LineTo(p) => {
                edges.push((last, p));
                last = p;
            }
            PathEl::ClosePath => {
                edges.push((last, start));
                last = start;
            }
            _ => {}
        });
    }
    let to_px = |p: Point| Point::new((p.x - origin.x) / mm_per_px, (p.y - origin.y) / mm_per_px);
    let edges: Vec<(Point, Point)> = edges
        .into_iter()
        .map(|(a, b)| (to_px(a), to_px(b)))
        .collect();
    let mut cover = vec![0.0_f64; (width * height) as usize];
    let mut xs: Vec<f64> = Vec::new();
    for row in 0..height {
        for sub in 0..4 {
            let y = f64::from(row) + (f64::from(sub) + 0.5) / 4.0;
            xs.clear();
            for (a, b) in &edges {
                if (a.y <= y) != (b.y <= y) {
                    xs.push(a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y));
                }
            }
            xs.sort_by(f64::total_cmp);
            for [start, end] in xs.as_chunks::<2>().0 {
                let (x0, x1) = (start.max(0.0), end.min(f64::from(width)));
                if x1 <= x0 {
                    continue;
                }
                let (first, last) = (x0.floor() as u32, (x1.ceil() as u32).min(width));
                for x in first..last {
                    let lo = x0.max(f64::from(x));
                    let hi = x1.min(f64::from(x) + 1.0);
                    if hi > lo {
                        cover[(row * width + x) as usize] += (hi - lo) / 4.0;
                    }
                }
            }
        }
    }
    let mut rgba = Vec::with_capacity(cover.len() * 4);
    for c in cover {
        let v = (255.0 * (1.0 - c.clamp(0.0, 1.0))).round() as u8;
        rgba.extend_from_slice(&[v, v, v, 255]);
    }
    rgba
}

/// The closed outlines of a trace, one subpath each.
fn traced(rgba: &[u8], width: u32, height: u32, mm_per_px: f64, auto: bool) -> Vec<BezPath> {
    let raster = Raster {
        width,
        height,
        rgba,
    };
    let threshold = if auto { auto_threshold(raster) } else { 128 };
    let params = PRESETS[0].apply(TraceParams::with_threshold(threshold));
    let mut go = |_| ControlFlow::Continue(());
    let map = curvyo_document_core::Affine::scale(mm_per_px, mm_per_px);
    let result = trace(raster, map, &params, &mut go).unwrap();
    let mut out: Vec<BezPath> = Vec::new();
    for outline in &result.outlines {
        for element in outline.elements() {
            if let PathEl::MoveTo(_) = element {
                out.push(BezPath::new());
            }
            if let Some(last) = out.last_mut() {
                last.push(*element);
            }
        }
    }
    out
}

/// The lines a job accepted over `outlines` with every face of `faces` in the included tier.
fn recognised(
    outlines: &[BezPath],
    lib: &Library,
    faces: &[usize],
) -> Vec<(usize, String, Vec<usize>, f64)> {
    recognised_with_lines(outlines, lib, faces).0
}

/// [`recognised`], with the number of lines found and the nearest face's deviation.
#[allow(clippy::type_complexity)]
fn recognised_with_lines(
    outlines: &[BezPath],
    lib: &Library,
    faces: &[usize],
) -> (
    Vec<(usize, String, Vec<usize>, f64)>,
    usize,
    Option<(usize, f64)>,
) {
    let listed: Vec<ListedFace> = faces.iter().map(|&i| lib.listed[i].clone()).collect();
    let mut job = RecogniseJob::new(outlines, Mm(0.3), recognise_tiers(&listed, &[], &[]));
    loop {
        match job.step(&|index| FaceState::Ready(lib.faces[faces[index]].clone())) {
            JobStep::Finished => break,
            JobStep::NeedFaces(_) => unreachable!("every face is ready"),
            JobStep::Running { .. } => {}
        }
    }
    let accepted = job
        .accepted()
        .iter()
        .map(|line| {
            (
                faces[line.face],
                line.run.text.clone(),
                line.run.guessed.clone(),
                line.run.max_deviation.get(),
            )
        })
        .collect();
    let nearest = job
        .nearest()
        .map(|(face, deviation)| (faces[face], deviation));
    (accepted, job.lines(), nearest)
}

/// The sure letters of `text` that are not in order in the truth line that fits best: the sure
/// letters (spaces and guessed letters left out) less their longest common subsequence with the
/// truth (spaces left out). Letters left as paths in between are no error, a wrong one is.
fn wrong_sure(text: &str, guessed: &[usize], truths: &[&str]) -> usize {
    let sure: Vec<char> = text
        .chars()
        .enumerate()
        .filter(|(i, c)| *c != ' ' && !guessed.contains(i))
        .map(|(_, c)| c)
        .collect();
    truths
        .iter()
        .map(|truth| {
            let truth: Vec<char> = truth.chars().filter(|c| *c != ' ').collect();
            let mut row = vec![0_usize; truth.len() + 1];
            for &a in &sure {
                let mut diagonal = 0;
                for (j, &b) in truth.iter().enumerate() {
                    let above = row[j + 1];
                    row[j + 1] = if a == b {
                        diagonal + 1
                    } else {
                        above.max(row[j])
                    };
                    diagonal = above;
                }
            }
            sure.len() - row[truth.len()]
        })
        .min()
        .unwrap_or(sure.len())
}

#[derive(Default)]
struct Score {
    cases: usize,
    letters: usize,
    right: usize,
    wrong_sure: usize,
    guessed: usize,
    left: usize,
    face_right: usize,
    deviation: f64,
}

impl Score {
    fn print(&self, name: &str) {
        println!(
            "{name}: cases {}, letters right {}/{} ({:.1} %), wrong sure {}, guessed {}, lines left {}, face right {}/{}, deviation max {:.3} mm",
            self.cases,
            self.right,
            self.letters,
            100.0 * self.right as f64 / self.letters.max(1) as f64,
            self.wrong_sure,
            self.guessed,
            self.left,
            self.face_right,
            self.cases,
            self.deviation,
        );
    }
}

const STRINGS: &[&str] = &["Curvyo 2026", "Laser 3 mm", "Hello, World!"];

fn chain(
    lib: &Library,
    truth_face: usize,
    candidates: &[usize],
    dpi: f64,
    size: f64,
    score: &mut Score,
) {
    let mm_per_px = 25.4 / dpi;
    for text in STRINGS {
        let paths = rendered(
            &lib.faces[truth_face],
            text,
            size,
            Point::new(5.0, 5.0 + size * 1.5),
        );
        let width_mm = 10.0 + size * text.len() as f64 * 0.9;
        let height_mm = 10.0 + size * 2.5;
        let (width, height) = (
            (width_mm / mm_per_px).ceil() as u32,
            (height_mm / mm_per_px).ceil() as u32,
        );
        let rgba = rasterised(&paths, width, height, mm_per_px, Point::ORIGIN);
        let outlines = traced(&rgba, width, height, mm_per_px, false);
        let lines = recognised(&outlines, lib, candidates);
        let letters = text.chars().filter(|c| *c != ' ').count();
        score.cases += 1;
        score.letters += letters;
        if let Some((face, found, guessed, deviation)) = lines.first() {
            // Letters in the text that are the truth's, in order (the longest common
            // subsequence): letters left as paths are not counted as right.
            let shown = found.chars().filter(|c| *c != ' ').count();
            score.right += shown - wrong_sure(found, &[], &[text]);
            score.wrong_sure += wrong_sure(found, guessed, &[text]);
            score.guessed += guessed.len();
            score.face_right += usize::from(*face == truth_face);
            score.deviation = score.deviation.max(*deviation);
            if found != text || *face != truth_face {
                println!(
                    "  {} {} {dpi} dpi {size} mm {text:?} -> {:?} in {} {}, guessed {guessed:?}",
                    FACES[truth_face].0, FACES[truth_face].1, found, FACES[*face].0, FACES[*face].1
                );
            }
        } else {
            score.left += 1;
            println!(
                "  {} {} {dpi} dpi {size} mm {text:?} -> left as paths",
                FACES[truth_face].0, FACES[truth_face].1
            );
        }
    }
}

#[test]
#[ignore = "the chain harness: release only, prints the numbers of the PR"]
fn chain_harness() {
    let lib = library();
    let all: Vec<usize> = (0..FACES.len()).collect();
    let mut total = Score::default();
    let mut hold_out = Score::default();
    for truth in [0, 1, 3, 4, 6] {
        let mut score = Score::default();
        for dpi in [300.0, 600.0] {
            for size in [5.0, 10.0] {
                chain(&lib, truth, &all, dpi, size, &mut score);
            }
        }
        score.print(&format!("{} {}", FACES[truth].0, FACES[truth].1));
        total.cases += score.cases;
        total.letters += score.letters;
        total.right += score.right;
        total.wrong_sure += score.wrong_sure;
        total.guessed += score.guessed;
        total.left += score.left;
        total.face_right += score.face_right;
        total.deviation = total.deviation.max(score.deviation);
        // Hold-out (criterion 3): the true family is not among the candidates.
        let family = FACES[truth].0;
        let others: Vec<usize> = all
            .iter()
            .copied()
            .filter(|&i| FACES[i].0 != family)
            .collect();
        chain(&lib, truth, &others, 600.0, 10.0, &mut hold_out);
    }
    total.print("chain all");
    hold_out
        .print("hold-out (true family removed; wrong sure must be 0, face right is 0 by design)");
    assert_eq!(total.wrong_sure, 0, "a wrong letter was accepted as sure");
    assert_eq!(
        hold_out.wrong_sure, 0,
        "a wrong letter was accepted as sure in the hold-out"
    );
}

/// The customer's reference logos with text and the text they show (the images are in the
/// repository, CC0, `tests/assets/trace-references/`).
const LOGOS: &[(&str, &[&str])] = &[
    (
        "precision.png",
        &["PRÄZISION", "MIT SYSTEM", "BERATUNG · PLANUNG · LÖSUNGEN"],
    ),
    (
        "balance.png",
        &[
            "BALANCE",
            "FÜR MEHR KLARHEIT",
            "COACHING · BERATUNG · ENTWICKLUNG",
        ],
    ),
    (
        "blütenzeit.png",
        &["Blütenzeit", "NATÜRLICH ♡ SCHÖN ♡ EINZIGARTIG"],
    ),
];

#[test]
#[ignore = "the customer's logos: release only, prints the numbers of the PR"]
fn customer_logos() {
    let lib = library();
    let all: Vec<usize> = (0..FACES.len()).collect();
    let mut wrong = 0;
    for (file, truths) in LOGOS {
        let bytes = std::fs::read(repo().join("tests/assets/trace-references").join(file)).unwrap();
        let image = curvyo_image_core::decode_working(&bytes).unwrap();
        // 0.1 mm a pixel: the large lines are 4 to 5 mm cap height.
        let outlines = traced(&image.pixels, image.width, image.height, 0.1, true);
        let (lines, found, nearest) = recognised_with_lines(&outlines, &lib, &all);
        let mut letters = 0;
        let mut guessed_total = 0;
        let mut wrong_here = 0;
        for (face, text, guessed, deviation) in &lines {
            letters += text.chars().filter(|c| *c != ' ').count();
            guessed_total += guessed.len();
            let w = wrong_sure(text, guessed, truths);
            wrong_here += w;
            println!(
                "  {file}: {text:?} in {} {}, guessed {guessed:?}, deviation {deviation:.3} mm, wrong sure {w}",
                FACES[*face].0, FACES[*face].1
            );
        }
        println!(
            "{file}: {} outlines, {found} lines, {} texts, {letters} letters, {guessed_total} guessed, wrong sure {wrong_here}, nearest {}",
            outlines.len(),
            lines.len(),
            nearest.map_or_else(String::new, |(face, d)| format!(
                "{} {} {d:.2} mm",
                FACES[face].0, FACES[face].1
            )),
        );
        wrong += wrong_here;
    }
    assert_eq!(wrong, 0, "a wrong letter was accepted as sure");
}
