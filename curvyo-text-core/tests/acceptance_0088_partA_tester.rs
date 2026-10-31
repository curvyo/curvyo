//! Independent tester cases for `specs/0088-trace-recognition` Part A at the text engine
//! (criteria 1 to 5 and 7), written from the specification before the implementation was read;
//! only the public names (`Recogniser`, `LineMatch`, `RecognisedRun`) were looked up.
//!
//! The traced lines are the included faces laid out with `layout()` and turned into outlines
//! (what Object to path writes). Deviations are measured here with an own two-sided distance
//! (dense samples against polyline segments), never with the engine's.
//!
//! `zero_wrong_sure_letters_full_sweep` and `cpu_numbers` are `#[ignore]` and print numbers:
//! `cargo test --release -p curvyo-text-core --test acceptance_0088_partA_tester -- --ignored --nocapture`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::needless_range_loop,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unchecked_time_subtraction,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names,
    missing_docs
)]

mod common;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use common::font_bytes;
use curvyo_text_core::{Face, FaceStore, LayoutOptions, LineMatch, Mm, Recogniser, layout};
use kurbo::{Affine, BezPath, PathEl, Point};

/// The included outline faces (single-line faces never match), with their names.
const FACES: [(&str, &str); 13] = [
    ("Great Vibes Regular", "great-vibes/GreatVibes-Regular.ttf"),
    (
        "Source Sans 3 Regular",
        "source-sans-3/SourceSans3-Regular.otf",
    ),
    ("Source Sans 3 Italic", "source-sans-3/SourceSans3-It.otf"),
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
    ("Source Sans 3 Light", "source-sans-3/SourceSans3-Light.otf"),
    (
        "Source Sans 3 Light Italic",
        "source-sans-3/SourceSans3-LightIt.otf",
    ),
    (
        "Source Serif 4 Regular",
        "source-serif-4/SourceSerif4-Regular.ttf",
    ),
    (
        "Source Serif 4 Italic",
        "source-serif-4/SourceSerif4-It.ttf",
    ),
    (
        "Source Serif 4 Bold",
        "source-serif-4/SourceSerif4-Bold.ttf",
    ),
    (
        "Source Serif 4 Bold Italic",
        "source-serif-4/SourceSerif4-BoldIt.ttf",
    ),
];
const GREAT_VIBES: usize = 0;
const SANS_REGULAR: usize = 1;
const SANS_BOLD: usize = 3;
const SANS_BOLD_ITALIC: usize = 4;
const SANS_LIGHT: usize = 7;
const SERIF_REGULAR: usize = 9;
const SERIF_ITALIC: usize = 10;
const SERIF_BOLD: usize = 11;

const TOL: Mm = Mm(0.3);

/// Every included outline face, each from its own store entry (a cold descriptor cache).
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

fn pick(all: &[Arc<Face>], which: &[usize]) -> Vec<Arc<Face>> {
    which.iter().map(|&i| all[i].clone()).collect()
}

fn refs(faces: &[Arc<Face>]) -> Vec<&Face> {
    faces.iter().map(AsRef::as_ref).collect()
}

/// The closed subpaths of `path`, each its own path.
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

/// Moves every point of `path` by `f`.
fn map_points(path: &BezPath, f: &dyn Fn(Point) -> Point) -> BezPath {
    let mut moved = BezPath::new();
    for element in path.elements() {
        moved.push(match *element {
            PathEl::MoveTo(p) => PathEl::MoveTo(f(p)),
            PathEl::LineTo(p) => PathEl::LineTo(f(p)),
            PathEl::QuadTo(a, b) => PathEl::QuadTo(f(a), f(b)),
            PathEl::CurveTo(a, b, c) => PathEl::CurveTo(f(a), f(b), f(c)),
            PathEl::ClosePath => PathEl::ClosePath,
        });
    }
    moved
}

/// How a test line is made.
#[derive(Clone, Copy)]
struct Setup {
    size: f64,
    degrees: f64,
    mirror: bool,
    wobble: f64,
}

impl Setup {
    fn plain(size: f64) -> Self {
        Self {
            size,
            degrees: 0.0,
            mirror: false,
            wobble: 0.0,
        }
    }
}

/// The glyph outlines of `text` in `face`, one entry per laid-out glyph (its subpaths), turned
/// about the anchor at (30, 200) mm; `stretch` scales glyph `k` by (x, y) about its box centre.
fn glyphs(
    face: &Face,
    text: &str,
    setup: Setup,
    stretch: &dyn Fn(usize) -> (f64, f64),
) -> Vec<Vec<BezPath>> {
    let laid = layout(face, text, Mm(setup.size), &LayoutOptions::default()).unwrap();
    let (sin, cos) = setup.degrees.to_radians().sin_cos();
    let mirror = if setup.mirror { -1.0 } else { 1.0 };
    let placement = Affine::new([cos, sin, -sin, cos, 30.0, 200.0])
        * Affine::new([mirror, 0.0, 0.0, 1.0, 0.0, 0.0]);
    let amount = setup.wobble;
    let mut out = Vec::new();
    for (k, glyph) in laid.lines.iter().flat_map(|l| l.glyphs.iter()).enumerate() {
        let local = Affine::translate(glyph.origin.to_vec2()) * Affine::scale(laid.mm_per_em);
        let path = local * face.outline(glyph.glyph).path.clone();
        let (sx, sy) = stretch(k);
        let path = if (sx - 1.0).abs() > 1e-12 || (sy - 1.0).abs() > 1e-12 {
            let b = kurbo::Shape::bounding_box(&path);
            let c = b.center();
            map_points(&path, &|p| {
                Point::new(c.x + (p.x - c.x) * sx, c.y + (p.y - c.y) * sy)
            })
        } else {
            path
        };
        let path = placement * path;
        let path = map_points(&path, &|p| {
            Point::new(
                p.x + amount * (p.y * 1.9 + p.x * 0.7).sin(),
                p.y + amount * (p.x * 1.1 - p.y * 0.5).cos(),
            )
        });
        out.push(subpaths(&path));
    }
    out
}

fn outlines(face: &Face, text: &str, setup: Setup) -> Vec<BezPath> {
    glyphs(face, text, setup, &|_| (1.0, 1.0))
        .into_iter()
        .flatten()
        .collect()
}

/// Every line matched against `tier`.
fn run_lines(outlines: &[BezPath], tier: &[&Face], tol: Mm) -> Vec<LineMatch> {
    let r = Recogniser::new(outlines, tol);
    (0..r.line_count()).map(|l| r.match_line(l, tier)).collect()
}

// ---------------------------------------------------------------------------------------------
// An own two-sided distance: dense samples of one side against the polyline segments of the
// other, bucketed in a 0.5 mm grid.

struct Polyline {
    points: Vec<Point>,
    segments: Vec<(Point, Point)>,
}

fn polyline(paths: &[BezPath]) -> Polyline {
    let mut points = Vec::new();
    let mut segments = Vec::new();
    for path in paths {
        let mut last: Option<Point> = None;
        let mut start: Option<Point> = None;
        kurbo::flatten(path.iter(), 0.002, |el| match el {
            PathEl::MoveTo(p) => {
                last = Some(p);
                start = Some(p);
            }
            PathEl::LineTo(p) => {
                if let Some(a) = last {
                    segments.push((a, p));
                }
                last = Some(p);
            }
            PathEl::ClosePath => {
                if let (Some(a), Some(s)) = (last, start) {
                    segments.push((a, s));
                }
                last = start;
            }
            _ => {}
        });
    }
    for &(a, b) in &segments {
        let n = ((a.distance(b) / 0.02).ceil() as usize).max(1);
        for i in 0..n {
            points.push(a.lerp(b, i as f64 / n as f64));
        }
    }
    Polyline { points, segments }
}

fn seg_dist(p: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let len2 = ab.hypot2();
    let t = if len2 > 0.0 {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + ab * t)
}

/// The largest distance of a point of `from` to the outline `to`.
fn directed(from: &Polyline, to: &Polyline) -> f64 {
    const CELL: f64 = 0.5;
    let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, &(a, b)) in to.segments.iter().enumerate() {
        let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
        let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
        for gx in (x0 / CELL).floor() as i64..=(x1 / CELL).floor() as i64 {
            for gy in (y0 / CELL).floor() as i64..=(y1 / CELL).floor() as i64 {
                grid.entry((gx, gy)).or_default().push(i);
            }
        }
    }
    let mut worst = 0.0_f64;
    for &p in &from.points {
        let (cx, cy) = ((p.x / CELL).floor() as i64, (p.y / CELL).floor() as i64);
        let mut best = f64::INFINITY;
        let mut ring = 0_i64;
        loop {
            for gx in cx - ring..=cx + ring {
                for gy in cy - ring..=cy + ring {
                    if (gx - cx).abs() != ring && (gy - cy).abs() != ring {
                        continue;
                    }
                    if let Some(list) = grid.get(&(gx, gy)) {
                        for &i in list {
                            let (a, b) = to.segments[i];
                            best = best.min(seg_dist(p, a, b));
                        }
                    }
                }
            }
            // Every segment within `ring * CELL` has been seen.
            if best <= ring as f64 * CELL || ring > 400 {
                break;
            }
            ring += 1;
        }
        worst = worst.max(best);
    }
    worst
}

fn two_sided(a: &[BezPath], b: &[BezPath]) -> f64 {
    let (pa, pb) = (polyline(a), polyline(b));
    directed(&pa, &pb).max(directed(&pb, &pa))
}

/// The run's text laid out at its size, spacing and placement, as outlines.
fn laid_out(face: &Face, run: &curvyo_text_core::RecognisedRun) -> Vec<BezPath> {
    let options = LayoutOptions {
        letter_spacing: run.letter_spacing,
        word_spacing: run.word_spacing,
        ..LayoutOptions::default()
    };
    let laid = layout(face, &run.text, run.size, &options).unwrap();
    let mut out = Vec::new();
    for glyph in laid.lines.iter().flat_map(|l| l.glyphs.iter()) {
        let local = Affine::translate(glyph.origin.to_vec2()) * Affine::scale(laid.mm_per_em);
        out.extend(subpaths(
            &(run.placement * local * face.outline(glyph.glyph).path.clone()),
        ));
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Criteria

/// Criterion 1: "Curvyo 2026" at 24 mm cap height becomes one text with that content and face,
/// searched against every included outline face (faces other than the branch tests use).
#[test]
fn c1_curvyo_2026_at_24_mm_is_one_text_in_its_own_face_among_all_included() {
    let all = faces();
    let tier = refs(&all);
    for source in [SERIF_ITALIC, SANS_LIGHT, SANS_BOLD_ITALIC] {
        let traced = outlines(&all[source], "Curvyo 2026", Setup::plain(24.0));
        let lines = run_lines(&traced, &tier, TOL);
        let runs: Vec<_> = lines.iter().filter_map(|l| l.run.clone()).collect();
        assert_eq!(
            runs.len(),
            1,
            "{}: one text, got {lines:?}",
            FACES[source].0
        );
        let run = &runs[0];
        assert_eq!(run.text, "Curvyo 2026", "{}", FACES[source].0);
        assert_eq!(
            run.face, source,
            "{}: its face, got {}",
            FACES[source].0, FACES[run.face].0
        );
        assert!((run.size.get() - 24.0).abs() <= 0.1, "size {:?}", run.size);
        let mut used = run.outlines.clone();
        used.sort_unstable();
        assert_eq!(
            used,
            (0..traced.len()).collect::<Vec<_>>(),
            "every outline replaced"
        );
    }
}

/// Criterion 2: the laid-out text deviates from the traced outlines by at most the tolerance
/// everywhere, by an own measurement (rotated 15 degrees, 10 mm, slight wobble).
#[test]
fn c2_the_laid_out_text_is_within_the_tolerance_everywhere_by_own_measure() {
    let all = faces();
    let tier = refs(&all);
    let setup = Setup {
        size: 10.0,
        degrees: 15.0,
        mirror: false,
        wobble: 0.03,
    };
    for source in [SERIF_BOLD, SANS_REGULAR] {
        let traced = outlines(&all[source], "Curvyo 2026", setup);
        let lines = run_lines(&traced, &tier, TOL);
        let run = lines.iter().find_map(|l| l.run.clone()).expect("a text");
        assert!(run.guessed.is_empty(), "clean input, all sure: {run:?}");
        let got = laid_out(&all[run.face], &run);
        let d = two_sided(&got, &traced);
        assert!(
            d <= TOL.get() + 0.01,
            "{}: own two-sided distance {d:.3} mm over the tolerance",
            FACES[source].0
        );
        assert!(
            run.max_deviation.get() <= TOL.get(),
            "reported {:?}",
            run.max_deviation
        );
        assert!(
            run.max_deviation.get() + 0.05 >= d,
            "reported largest deviation {:.3} under the measured {d:.3}",
            run.max_deviation.get()
        );
    }
}

/// Criterion 3: text in a face the tier lacks is set in the nearest face within the tolerance
/// (if any), and the run names its largest deviation, which holds by own measurement.
#[test]
fn c3_a_missing_face_gives_the_nearest_face_within_the_tolerance() {
    let all = faces();
    // Source Sans 3 Regular is not searched; its neighbours Light and Bold and the serif are. At
    // 5 mm no face is within 0.3 mm (the nearest is about 0.57 mm off), so the maker's tolerance
    // is 0.7 mm here (criterion 6: the tolerance is a setting).
    let tier_faces = pick(&all, &[SANS_LIGHT, SANS_BOLD, SERIF_REGULAR]);
    let tier = refs(&tier_faces);
    let traced = outlines(&all[SANS_REGULAR], "Curvyo 2026", Setup::plain(5.0));
    let lines = run_lines(&traced, &tier, Mm(0.7));
    let run = lines.iter().find_map(|l| l.run.clone());
    let Some(run) = run else {
        let near = lines
            .iter()
            .find_map(|l| l.nearest)
            .expect("a nearest face is named");
        panic!("no face within 0.7 mm at 5 mm; nearest {near:?}");
    };
    assert_ne!(
        run.face, 2,
        "the serif is not the nearest to a sans: {run:?}"
    );
    // Nearest: no other face of the tier, alone, has more sure letters.
    for (i, f) in tier.iter().enumerate() {
        let alone = run_lines(&traced, &[*f], Mm(0.7));
        let sure = alone.iter().map(|l| l.sure).max().unwrap_or(0);
        assert!(
            sure <= run.sure,
            "face {i} alone has {sure} sure > the chosen {}",
            run.sure
        );
    }
    assert!(run.max_deviation.get() <= 0.7, "{:?}", run.max_deviation);
    if run.guessed.is_empty() {
        let d = two_sided(&laid_out(tier[run.face], &run), &traced);
        assert!(d <= 0.71, "own measure {d:.3} mm");
    }
}

/// Criterion 4: no face within the tolerance leaves the line as paths and names the nearest face
/// with its LARGEST per-letter deviation: one letter far off raises the named deviation.
#[test]
fn c4_nothing_within_tolerance_names_the_largest_per_letter_deviation() {
    let all = faces();
    let tier_faces = pick(&all, &[SANS_BOLD]);
    let tier = refs(&tier_faces);
    let text = "Curvyo 2026";
    let mild = |_k: usize| (1.07, 1.0);
    let heavy = |k: usize| if k == 3 { (1.07, 1.25) } else { (1.07, 1.0) };
    let mut named = Vec::new();
    for stretch in [&mild as &dyn Fn(usize) -> (f64, f64), &heavy] {
        let g = glyphs(&all[SANS_BOLD], text, Setup::plain(24.0), stretch);
        let traced: Vec<BezPath> = g.into_iter().flatten().collect();
        let lines = run_lines(&traced, &tier, TOL);
        assert!(
            lines.iter().all(|l| l.run.is_none()),
            "nothing within the tolerance stays paths: {lines:?}"
        );
        let near = lines
            .iter()
            .find_map(|l| l.nearest)
            .expect("the nearest face is named");
        assert_eq!(near.face, 0);
        assert!(near.max_deviation.get() > TOL.get(), "{near:?}");
        named.push(near.max_deviation.get());
    }
    // Every letter is 7 % wider; in the second line the 'v' (k = 3) is also 25 % taller about
    // its centre, so its top and foot move by an eighth of its height, which no translation and
    // no line-wide size change of 3 % can take back. The named deviation must cover that letter.
    let plain = glyphs(&all[SANS_BOLD], text, Setup::plain(24.0), &|_| (1.0, 1.0));
    let v_height = plain[3]
        .iter()
        .map(kurbo::Shape::bounding_box)
        .reduce(|a, b| a.union(b))
        .unwrap()
        .height();
    let v_off = 0.125 * v_height - 0.03 * 24.0;
    assert!(
        named[1] >= v_off,
        "the named deviation {:.2} is under the 'v' letter's own {v_off:.2} (mild line {:.2}): \
         not the largest per-letter deviation",
        named[1],
        named[0]
    );
    assert!(
        named[1] > named[0] + 0.5,
        "mild {:.2}, one heavy {:.2}",
        named[0],
        named[1]
    );
}

/// Criterion 4 (lead's mirrored case): mirrored text is out of scope and stays paths, with a
/// nearest face to name; no letter is taken as sure.
#[test]
fn c4_mirrored_text_stays_paths_with_a_nearest_face() {
    let all = faces();
    let tier = refs(&all);
    for source in [SANS_BOLD, SERIF_REGULAR] {
        let setup = Setup {
            mirror: true,
            ..Setup::plain(24.0)
        };
        let traced = outlines(&all[source], "Curvyo 2026", setup);
        let lines = run_lines(&traced, &tier, TOL);
        assert!(
            lines.iter().all(|l| l.run.is_none()),
            "{}: mirrored text became text: {:?}",
            FACES[source].0,
            lines
                .iter()
                .filter_map(|l| l.run.as_ref())
                .map(|r| &r.text)
                .collect::<Vec<_>>()
        );
        assert!(
            lines.iter().any(|l| l.nearest.is_some()),
            "a nearest face is named"
        );
    }
}

/// Criterion 5: the recognition code reads no file, opens no socket and starts no thread; the
/// engine has no network dependency (it works with the network off by construction).
#[test]
fn c5_recognition_has_no_network_file_or_thread_access() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("src/recognise");
    let mut files = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let src = std::fs::read_to_string(&path).unwrap();
        files += 1;
        for banned in [
            "std::net",
            "TcpStream",
            "UdpSocket",
            "std::fs",
            "std::thread",
            "std::process",
            "std::env",
            "Instant",
            "SystemTime",
            "reqwest",
            "ureq",
            "hyper",
            "fetch(",
        ] {
            assert!(!src.contains(banned), "{} uses {banned}", path.display());
        }
    }
    assert!(files >= 5, "the recognise module folder is read");
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let deps = manifest
        .split("[dependencies]")
        .nth(1)
        .unwrap()
        .split("\n[")
        .next()
        .unwrap();
    for banned in [
        "reqwest", "ureq", "hyper", "tokio", "surf", "curl", "ocrs", "ort",
    ] {
        assert!(
            !deps.lines().any(|l| l.trim_start().starts_with(banned)),
            "text-core depends on {banned}"
        );
    }
}

/// Criterion 7: one letter far off is guessed (its position listed), the others are sure, the
/// text keeps a letter in its place, and the largest deviation covers the sure letters only.
#[test]
fn c7_one_far_letter_is_guessed_in_place_and_left_out_of_the_deviation() {
    let all = faces();
    let tier = refs(&all);
    let text = "Curvyo 2026";
    // Glyph k = 4 is the 'y' (character position 4), widened 8 % (about 0.6 mm per side; it
    // still keeps clear of the 'v').
    let g = glyphs(&all[SERIF_REGULAR], text, Setup::plain(24.0), &|k| {
        if k == 4 { (1.08, 1.0) } else { (1.0, 1.0) }
    });
    let traced: Vec<BezPath> = g.into_iter().flatten().collect();
    let lines = run_lines(&traced, &tier, TOL);
    let run = lines
        .iter()
        .find_map(|l| l.run.clone())
        .expect("10 of 11 sure: a text");
    assert_eq!(run.face, SERIF_REGULAR, "{run:?}");
    assert_eq!(run.text.chars().count(), 11, "{:?}", run.text);
    assert_eq!(run.guessed, vec![4], "the 'y' is the guessed one: {run:?}");
    for (i, (a, b)) in run.text.chars().zip(text.chars()).enumerate() {
        if i != 4 {
            assert_eq!(a, b, "sure letter {i}");
        }
    }
    assert_eq!(run.sure, 9, "10 letters, 1 guessed");
    assert!(
        run.max_deviation.get() <= TOL.get(),
        "sure letters only: {:?}",
        run.max_deviation
    );
}

/// Criterion 4, closing sentence (lead): a joined script face whose letters touch stays paths:
/// merged letters are not split, so no wrong letter is taken as sure.
#[test]
fn joined_script_never_gives_a_wrong_sure_letter() {
    let all = faces();
    let tier = refs(&all);
    let truth = "minimum";
    let traced = outlines(&all[GREAT_VIBES], truth, Setup::plain(24.0));
    let lines = run_lines(&traced, &tier, TOL);
    let mut wrong = 0;
    for run in lines.iter().filter_map(|l| l.run.as_ref()) {
        wrong += wrong_sure(truth, run);
    }
    assert_eq!(wrong, 0, "{lines:?}");
}

// ---------------------------------------------------------------------------------------------
// Zero wrong sure letters on synthetic text

/// The sure letters of `run` that an alignment with `truth` (longest common subsequence over
/// letters, spaces ignored) does not account for.
fn wrong_sure(truth: &str, run: &curvyo_text_core::RecognisedRun) -> usize {
    let t: Vec<char> = truth.chars().filter(|c| !c.is_whitespace()).collect();
    let got: Vec<(char, bool)> = run
        .text
        .chars()
        .enumerate()
        .filter(|(_, c)| !c.is_whitespace())
        .map(|(i, c)| (c, !run.guessed.contains(&i)))
        .collect();
    // LCS that prefers matching sure letters.
    let (n, m) = (got.len(), t.len());
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if got[i].0 == t[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let (mut i, mut j, mut matched_sure) = (0, 0, 0);
    while i < n && j < m {
        if got[i].0 == t[j] {
            if got[i].1 {
                matched_sure += 1;
            }
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    got.iter().filter(|g| g.1).count() - matched_sure
}

#[derive(Debug, Default, Clone, Copy)]
struct Totals {
    lines: usize,
    letters: usize,
    sure: usize,
    guessed: usize,
    unrecognised: usize,
    wrong_sure: usize,
    wrong_face: usize,
}

const STRINGS: [&str; 2] = ["Curvyo 2026", "Jackdaws love 389 quiz"];

fn sweep(
    all: &[Arc<Face>],
    sources: &[usize],
    sizes: &[f64],
    angles: &[f64],
) -> (Totals, Vec<String>) {
    let tier = refs(all);
    let mut t = Totals::default();
    let mut wrong = Vec::new();
    for &source in sources {
        for &size in sizes {
            for &degrees in angles {
                for truth in STRINGS {
                    let setup = Setup {
                        size,
                        degrees,
                        mirror: false,
                        wobble: 0.0,
                    };
                    let traced = outlines(&all[source], truth, setup);
                    let letters = truth.chars().filter(|c| !c.is_whitespace()).count();
                    t.lines += 1;
                    t.letters += letters;
                    let mut covered = 0;
                    for line in run_lines(&traced, &tier, TOL) {
                        let Some(run) = line.run else { continue };
                        let n = run.text.chars().filter(|c| !c.is_whitespace()).count();
                        covered += n;
                        t.guessed += run.guessed.len();
                        t.sure += n - run.guessed.len();
                        let w = wrong_sure(truth, &run);
                        if w > 0 {
                            wrong.push(format!(
                                "{} {size} mm {degrees} deg: {truth:?} -> {:?} guessed {:?} ({w} wrong sure)",
                                FACES[source].0, run.text, run.guessed
                            ));
                        }
                        t.wrong_sure += w;
                        if run.face != source {
                            t.wrong_face += 1;
                        }
                    }
                    t.unrecognised += letters.saturating_sub(covered);
                }
            }
        }
    }
    (t, wrong)
}

/// Zero wrong letters accepted as sure, a fast subset: two faces, two sizes, two angles.
#[test]
fn zero_wrong_sure_letters_fast_subset() {
    let all = faces();
    let (t, wrong) = sweep(
        &all,
        &[SANS_REGULAR, SERIF_BOLD],
        &[10.0, 36.0],
        &[0.0, 33.0],
    );
    println!("fast subset: {t:?}");
    assert_eq!(t.wrong_sure, 0, "{wrong:#?}");
    assert!(t.sure > t.letters / 2, "most letters recognised: {t:?}");
}

/// Zero wrong letters accepted as sure: every included outline face at 4, 10, 36 and 120 mm,
/// turned 0, 15, 33, 90 and 180 degrees; prints the totals.
#[test]
#[ignore = "release sweep, prints numbers"]
fn zero_wrong_sure_letters_full_sweep() {
    let all = faces();
    let sizes = [4.0, 10.0, 36.0, 120.0];
    let angles = [0.0, 15.0, 33.0, 90.0, 180.0];
    let mut total = Totals::default();
    let mut by_size = [Totals::default(); 4];
    let mut by_angle = [Totals::default(); 5];
    let mut by_face = [Totals::default(); 13];
    let mut wrong = Vec::new();
    for source in 0..FACES.len() {
        for (si, &size) in sizes.iter().enumerate() {
            for (ai, &degrees) in angles.iter().enumerate() {
                let (t, w) = sweep(&all, &[source], &[size], &[degrees]);
                for into in [
                    &mut total,
                    &mut by_size[si],
                    &mut by_angle[ai],
                    &mut by_face[source],
                ] {
                    add(into, &t);
                }
                wrong.extend(w);
            }
        }
    }
    println!("full sweep: {total:?}");
    for w in &wrong {
        println!("  {w}");
    }
    for (size, t) in sizes.iter().zip(&by_size) {
        println!("  {size:>5} mm: {t:?}");
    }
    for (degrees, t) in angles.iter().zip(&by_angle) {
        println!("  {degrees:>5} deg: {t:?}");
    }
    for ((name, _), t) in FACES.iter().zip(&by_face) {
        println!("  {name}: {t:?}");
    }
    assert_eq!(total.wrong_sure, 0, "{wrong:#?}");
}

fn add(into: &mut Totals, t: &Totals) {
    into.lines += t.lines;
    into.letters += t.letters;
    into.sure += t.sure;
    into.guessed += t.guessed;
    into.unrecognised += t.unrecognised;
    into.wrong_sure += t.wrong_sure;
    into.wrong_face += t.wrong_face;
}

// ---------------------------------------------------------------------------------------------
// CPU numbers against adrs.md decision 8 (thread CPU time, release)

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

fn cpu<T>(f: impl FnOnce() -> T) -> (T, Duration) {
    let start = cpu_now();
    let out = f();
    (out, cpu_now() - start)
}

/// `count` cold faces, the included outline faces repeated, each its own store entry.
fn cold_faces(count: usize) -> Vec<Arc<Face>> {
    let mut store = FaceStore::new();
    let mut out = Vec::new();
    while out.len() < count {
        for (_, path) in FACES.iter().skip(1) {
            if out.len() == count {
                break;
            }
            let mut bytes = font_bytes(path);
            // A distinct file per copy, so each copy is its own face with its own cache.
            bytes.extend_from_slice(&(out.len() as u32).to_le_bytes());
            let key = store.add(bytes, 0).unwrap();
            out.push(store.get(&key).unwrap().clone());
        }
    }
    out
}

#[test]
#[ignore = "release timing, prints numbers"]
fn cpu_numbers() {
    let all = faces();
    // 30 glyphs on one line.
    let line30 = "Jackdaws love my big sphinx 39";
    let traced = outlines(&all[SANS_REGULAR], line30, Setup::plain(10.0));

    // Tier 4 hit cold: the included faces, fresh caches.
    let cold = faces();
    let tier4 = refs(&cold);
    let (m, t_tier4) = cpu(|| run_lines(&traced, &tier4, TOL));
    assert!(m.iter().any(|l| l.run.is_some()));
    println!("tier 4 hit, 13 included faces, cold, 30 glyphs: {t_tier4:?} (target <= 0.3 s)");

    // 64 faces: first (cold) and later (cached) run.
    let many = cold_faces(64);
    let tier64 = refs(&many);
    let (_, first) = cpu(|| run_lines(&traced, &tier64, TOL));
    let (_, later) = cpu(|| run_lines(&traced, &tier64, TOL));
    println!(
        "64 faces first run: {first:?} (target <= 1.3 s); cached: {later:?} (target <= 300 ms)"
    );
    let per_face = first.saturating_sub(later) / 64;
    println!("descriptors per face (first minus cached, / 64): {per_face:?} (target <= 15 ms)");

    // Worst case: about 200 faces searched cold, a line that matches none (mirrored).
    let worst = cold_faces(200);
    let tier200 = refs(&worst);
    let mirrored = outlines(
        &all[SANS_REGULAR],
        line30,
        Setup {
            mirror: true,
            ..Setup::plain(10.0)
        },
    );
    let (_, t_worst) = cpu(|| run_lines(&mirrored, &tier200, TOL));
    println!("worst case, 200 faces cold, no match: {t_worst:?} (target <= 4 s)");
}

/// Zero wrong letters accepted as sure when the line's face is missing and a wide tolerance (the
/// bar allows up to 2 mm) lets a neighbouring family through: criterion 9 lets an earlier tier
/// with such a face win, so its letters must still be the right ones.
#[test]
fn a_wide_tolerance_across_families_takes_no_wrong_letter_as_sure() {
    let all = faces();
    let sans = pick(&all, &[1, 2, 3, 4, 5, 6, 7, 8]);
    let tier = refs(&sans);
    let truth = "Curvyo 2026";
    let mut wrong = Vec::new();
    for (size, tol) in [(3.0, 0.8), (5.0, 1.0), (10.0, 2.0)] {
        let traced = outlines(&all[SERIF_REGULAR], truth, Setup::plain(size));
        for line in run_lines(&traced, &tier, Mm(tol)) {
            if let Some(run) = line.run {
                let w = wrong_sure(truth, &run);
                if w > 0 {
                    wrong.push(format!(
                        "{size} mm at {tol} mm: {:?} guessed {:?}",
                        run.text, run.guessed
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "wrong letters taken as sure: {wrong:#?}");
}

/// Criterion 4: the named deviation is the largest per-letter deviation even when that letter is
/// far off (here the 'v' is 60 % taller, its top and foot about 3.5 mm off): no ceiling.
#[test]
fn c4_a_far_letter_is_named_with_its_whole_deviation() {
    let all = faces();
    let tier_faces = pick(&all, &[SANS_BOLD]);
    let tier = refs(&tier_faces);
    let text = "Curvyo 2026";
    let far = |k: usize| if k == 3 { (1.07, 1.6) } else { (1.07, 1.0) };
    let g = glyphs(&all[SANS_BOLD], text, Setup::plain(24.0), &far);
    let traced: Vec<BezPath> = g.into_iter().flatten().collect();
    let lines = run_lines(&traced, &tier, TOL);
    assert!(lines.iter().all(|l| l.run.is_none()), "{lines:?}");
    let near = lines
        .iter()
        .find_map(|l| l.nearest)
        .expect("a nearest face");
    let plain = glyphs(&all[SANS_BOLD], text, Setup::plain(24.0), &|_| (1.0, 1.0));
    let v_height = plain[3]
        .iter()
        .map(kurbo::Shape::bounding_box)
        .reduce(|a, b| a.union(b))
        .unwrap()
        .height();
    let v_off = 0.3 * v_height - 0.03 * 24.0;
    assert!(
        near.max_deviation.get() >= v_off,
        "named {:.2} mm, the 'v' alone is at least {v_off:.2} mm off",
        near.max_deviation.get()
    );
}

/// Criterion 1 beyond its example: a short word in an included face at 24 mm cap height becomes
/// a text in that face too.
#[test]
fn c1_a_short_word_at_24_mm_becomes_a_text() {
    let all = faces();
    let tier = refs(&all);
    for (truth, size) in [("Wolf", 24.0), ("Wolf 2026", 36.0)] {
        let traced = outlines(&all[SANS_REGULAR], truth, Setup::plain(size));
        let lines = run_lines(&traced, &tier, TOL);
        let texts: Vec<&str> = lines
            .iter()
            .filter_map(|l| l.run.as_ref())
            .map(|r| r.text.as_str())
            .collect();
        assert_eq!(texts, [truth], "{truth} at {size} mm");
    }
}
