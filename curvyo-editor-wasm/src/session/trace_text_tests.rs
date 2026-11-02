//! Text recognition in Trace end to end (`specs/0091-trace-text-recognition` criteria 1 to 4, 6,
//! 8, 10, 11 and 13): the reference set read with the shipped weights, traced with the text pass,
//! the lines fitted to the included faces as the host loads them, and Create paths. The rotated
//! line runs in every build; the posters are the `#[ignore]` harness that prints the numbers of
//! the PR:
//!
//! ```text
//! cargo test --release -p curvyo-editor-wasm --lib trace_text_tests -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};

use super::{Session, Tool, TraceCreateOutcome};
use curvyo_document_core::{NodeId, ObjectSnapshot, TextSnapshot};
use curvyo_ui_core::{OcrModel, TraceParams, trace_read_text, trace_text_job_with};

/// The thread's CPU time in seconds (`CLAUDE.md` §5: timings are thread CPU time).
#[allow(clippy::cast_precision_loss)] // seconds and nanoseconds of a test run
fn cpu_seconds() -> f64 {
    #[cfg(unix)]
    {
        let t = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
        t.tv_sec as f64 + t.tv_nsec as f64 * 1e-9
    }
    #[cfg(not(unix))]
    {
        0.0
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn fonts() -> PathBuf {
    root().join("frontend/public/fonts")
}

pub(super) fn model() -> OcrModel {
    let dir = root().join("frontend/public/models/paddleocr-latin");
    let read = |name: &str| std::fs::read(dir.join(name)).unwrap();
    let alphabet = String::from_utf8(read("dict.txt")).unwrap();
    OcrModel::load(&read("det.onnx"), &read("rec.onnx"), &alphabet).unwrap()
}

fn load_wanted(session: &mut Session) {
    for file in session.wanted_font_files() {
        let bytes = std::fs::read(fonts().join(&file)).unwrap();
        assert!(session.add_font_file(&file, &bytes));
    }
}

/// A session with the included fonts listed and `file` imported, selected, in the Trace tool.
pub(super) fn session_with(file: &str) -> (Session, NodeId) {
    let bytes = std::fs::read(root().join("curvyo-ocr-core/tests/fixtures").join(file)).unwrap();
    session_with_bytes(&bytes, file)
}

/// A session with the included fonts listed and the picture `bytes` imported as `file`.
fn session_with_bytes(bytes: &[u8], file: &str) -> (Session, NodeId) {
    let mut session = Session::new(1);
    let index: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fonts().join("index.json")).unwrap())
            .unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|face| {
            (
                face["file"].as_str().unwrap().to_owned(),
                face["family"].as_str().unwrap().to_owned(),
                face["style"].as_str().unwrap().to_owned(),
                0,
                if face["kind"] == "single_line" {
                    curvyo_document_core::FontKind::SingleLine
                } else {
                    curvyo_document_core::FontKind::Outline
                },
            )
        })
        .collect();
    session.set_font_list_with_kinds(curvyo_ui_core::FontBlock::Included, faces, Vec::new());
    load_wanted(&mut session);
    session.import_begin(None);
    session.import_add(bytes, file);
    session.import_finish(false);
    let image = session
        .objects()
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Image(image) => Some(image.id),
            _ => None,
        })
        .unwrap();
    session.set_tool(Tool::Trace);
    (session, image)
}

/// What one run produced.
#[derive(Debug, PartialEq)]
struct Run {
    texts: Vec<(String, Vec<usize>, f64, [f64; 6])>,
    paths: usize,
    ocr_seconds: f64,
    pass_seconds: f64,
    fit_seconds: f64,
}

fn texts(session: &Session) -> Vec<TextSnapshot> {
    session
        .objects()
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Text(text) => Some((**text).clone()),
            _ => None,
        })
        .collect()
}

/// The fit of `request` as the trace worker runs it (`trace_fit_glue.rs`).
pub(super) fn fit(request: &str) -> String {
    let mut worker = crate::trace_fit_glue::FitWorker::default();
    let mut reply = worker.start(request, &mut |_| {});
    loop {
        let value: serde_json::Value = serde_json::from_str(&reply).unwrap();
        if value["kind"] == "finished" {
            return reply;
        }
        for face in value["faces"].as_array().unwrap() {
            let file = face[0].as_str().unwrap();
            let bytes = std::fs::read(fonts().join(file)).unwrap();
            assert!(worker.add_face(
                file,
                u32::try_from(face[1].as_u64().unwrap()).unwrap(),
                &bytes
            ));
        }
        reply = worker.step(&mut |_| {});
    }
}

/// OCR, text pass and trace, the fit, Create paths, one undo; checks the one step (criterion 11).
fn run(model: &OcrModel, file: &str) -> Run {
    let (mut session, image) = session_with(file);
    let source = session.trace_source(image, false).unwrap();
    let started = cpu_seconds();
    let lines = trace_read_text(
        model,
        &source.rgba,
        source.width,
        source.height,
        &mut |_| std::ops::ControlFlow::Continue(()),
    )
    .unwrap();
    let ocr_seconds = cpu_seconds() - started;
    assert!(session.set_trace_ocr_lines(image, lines.clone(), (source.width, source.height)));
    let threshold = curvyo_ui_core::trace_auto_threshold(&source.rgba, source.width, source.height);
    let started = cpu_seconds();
    let wire = trace_text_job_with(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        &TraceParams::with_threshold(threshold),
        &lines,
        &mut |_| {},
    );
    let pass_seconds = cpu_seconds() - started;
    assert!(session.set_trace_result(image, &wire));
    assert!(session.set_trace_preview(image, &wire));
    let before = session.trace.preview.as_ref().unwrap().outlines.clone();
    let started = cpu_seconds();
    let request = session.trace_fit_request(0.3, &[]).unwrap();
    let reply = fit(&request);
    assert!(session.set_trace_fit(&request, &reply));
    let fit_seconds = cpu_seconds() - started;
    // UX notes §2: the preview now draws the texts in their faces, not the lines' outlines.
    assert!(session.trace_text_fitted());
    assert_ne!(session.trace.preview.as_ref().unwrap().outlines, before);
    let steps = session.document.own_steps().len();
    let TraceCreateOutcome::Created { paths, .. } = session.create_traced_paths() else {
        panic!("nothing created");
    };
    assert_eq!(session.document.own_steps().len(), steps + 1, "one step");
    let created = session.trace_created_texts().to_vec();
    let written = texts(&session);
    // Criterion 10: black fill.
    assert!(
        written
            .iter()
            .all(|t| t.style.fill.enabled
                && t.style.fill.color == curvyo_document_core::Color::BLACK)
    );
    let out = Run {
        texts: written
            .iter()
            .map(|t| {
                let guessed = created
                    .iter()
                    .find(|c| c.string == t.string)
                    .map(|c| c.guessed.iter().map(|g| g.1 - 1).collect())
                    .unwrap_or_default();
                (
                    t.string.clone(),
                    guessed,
                    t.size.as_mm(),
                    t.placement.to_array(),
                )
            })
            .collect(),
        paths,
        ocr_seconds,
        pass_seconds,
        fit_seconds,
    };
    // Criterion 11: one undo removes the group and every text.
    session.undo();
    assert_eq!(texts(&session).len(), 0);
    assert_eq!(session.objects().len(), 1, "only the image is left");
    out
}

fn levenshtein(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let next = (prev + usize::from(ca != cb))
                .min(row[j] + 1)
                .min(row[j + 1] + 1);
            prev = row[j + 1];
            row[j + 1] = next;
        }
    }
    row[b.len()]
}

/// Wrong sure letters of `text` against its nearest truth line: positions that differ and are
/// not guessed (a different length counts every unguessed letter of the difference).
fn wrong_sure(text: &str, guessed: &[usize], truth: &str) -> usize {
    let (t, r): (Vec<char>, Vec<char>) = (text.chars().collect(), truth.chars().collect());
    if t.len() != r.len() {
        return levenshtein(text, truth).min(t.len() - guessed.len().min(t.len()));
    }
    t.iter()
        .zip(&r)
        .enumerate()
        .filter(|(i, (a, b))| a != b && !guessed.contains(i))
        .count()
}

fn report(file: &str, truth: &[&str]) -> (usize, usize, Run) {
    let model = model();
    let run1 = run(&model, file);
    let exact = truth
        .iter()
        .filter(|line| run1.texts.iter().any(|t| t.0 == **line))
        .count();
    let mut wrong = 0;
    for (text, guessed, _, _) in &run1.texts {
        let nearest = truth.iter().min_by_key(|l| levenshtein(text, l)).unwrap();
        wrong += wrong_sure(text, guessed, nearest);
    }
    println!(
        "{file}: {exact} of {} lines exact, {wrong} wrong sure letters, {} texts, {} paths; \
         thread CPU: OCR {:.2} s (rten threads not counted), text pass and trace {:.2} s, fit {:.2} s",
        truth.len(),
        run1.texts.len(),
        run1.paths,
        run1.ocr_seconds,
        run1.pass_seconds,
        run1.fit_seconds
    );
    for t in &run1.texts {
        println!("  {:?} guessed {:?} size {:.2} mm", t.0, t.1, t.2);
    }
    (exact, wrong, run1)
}

const POSTER: [&str; 8] = [
    "BÄCKEREI",
    "SALE",
    "Müller & Söhne",
    "Größe 42 mm, Laser 0.2",
    "Wedding",
    "FRESH",
    "Art.-Nr. 0815/ÄÖÜ äöü ß",
    "small print: est. 1998 in Hamburg",
];

/// Criteria 18 and 25 (Frage 115 B / 117 B): the `0025` ring, which the OCR reads as "O", fails
/// the garbage filter and has no sure letter, so the fit, run on the text pass alone before the
/// trace, sets no text; the one trace then takes it as a shape, equals the trace without text, and
/// keeps the fit.
#[test]
fn a_ring_read_as_o_is_decided_before_the_one_trace() {
    use image::ImageEncoder;
    let model = model();
    let mut rgba = Vec::new();
    for y in 0..200 {
        for x in 0..200 {
            let r = (f64::from(x) + 0.5 - 100.0).hypot(f64::from(y) + 0.5 - 100.0);
            let g = if r < 60.0 && r > 20.0 { 0 } else { 255 };
            rgba.extend([g, g, g, 255]);
        }
    }
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&rgba, 200, 200, image::ExtendedColorType::Rgba8)
        .unwrap();
    let (mut session, image) = session_with_bytes(&png, "ring.png");
    let source = session.trace_source(image, false).unwrap();
    let lines = trace_read_text(
        &model,
        &source.rgba,
        source.width,
        source.height,
        &mut |_| std::ops::ControlFlow::Continue(()),
    )
    .unwrap();
    assert!(
        lines.iter().any(curvyo_ui_core::OcrLine::accepted),
        "{lines:?}"
    );
    assert!(session.set_trace_ocr_lines(image, lines.clone(), (source.width, source.height)));
    let params = TraceParams::with_threshold(128);
    let regions = curvyo_ui_core::trace_text_regions_with(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        &params,
        &lines,
    );
    let request = session
        .trace_fit_request_for(image, &regions, 0.3, &[])
        .unwrap();
    assert!(session.set_trace_fit(&request, &fit(&request)));
    assert_eq!(session.trace_fit_dropped(), 1);
    let (decided, _) = session.trace_lines_to_trace(image).unwrap();
    assert!(decided.iter().all(|l| !l.accepted()));
    let wire = trace_text_job_with(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        &params,
        &decided,
        &mut |_| {},
    );
    let plain = curvyo_ui_core::trace_job_with(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        &params,
        &mut |_| {},
    );
    let (on, off) = (
        curvyo_ui_core::read_trace_wire(&wire).unwrap(),
        curvyo_ui_core::read_trace_wire(&plain).unwrap(),
    );
    let plain_paths = off.paths;
    assert_eq!((on.outlines, on.nodes), (off.outlines, off.nodes));
    assert!(session.set_trace_preview(image, &wire));
    assert!(session.set_trace_result(image, &wire));
    assert!(session.trace_text_fitted(), "the one trace keeps the fit");
    let TraceCreateOutcome::Created { paths, .. } = session.create_traced_paths() else {
        panic!("nothing created");
    };
    assert_eq!(texts(&session).len(), 0);
    assert_eq!(
        paths, plain_paths,
        "the ring once, not again from the fit's leftovers"
    );
}

/// Criterion 25: a result whose text lines are not the ones the fit decided drops the fit.
#[test]
fn a_trace_with_other_lines_drops_the_fit() {
    let model = model();
    let (mut session, image) = session_with("rotated.png");
    let source = session.trace_source(image, false).unwrap();
    let lines = trace_read_text(
        &model,
        &source.rgba,
        source.width,
        source.height,
        &mut |_| std::ops::ControlFlow::Continue(()),
    )
    .unwrap();
    assert!(session.set_trace_ocr_lines(image, lines.clone(), (source.width, source.height)));
    let params = TraceParams::with_threshold(128);
    let regions = curvyo_ui_core::trace_text_regions_with(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        &params,
        &lines,
    );
    let request = session
        .trace_fit_request_for(image, &regions, 0.3, &[])
        .unwrap();
    assert!(session.set_trace_fit(&request, &fit(&request)));
    let plain = curvyo_ui_core::trace_job_with(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        &params,
        &mut |_| {},
    );
    assert!(session.set_trace_result(image, &plain));
    assert!(!session.trace_text_fitted());
}

#[test]
fn the_rotated_line_is_one_turned_text_and_runs_repeat() {
    let model = model();
    let first = run(&model, "rotated.png");
    assert_eq!(first.texts.len(), 1, "{first:?}");
    assert_eq!(first.texts[0].0, "Tilted 15");
    let m = first.texts[0].3;
    let degrees = m[1].atan2(m[0]).to_degrees();
    assert!((degrees + 15.0).abs() <= 1.0, "{degrees}");
    let second = run(&model, "rotated.png");
    assert_eq!(
        (first.texts, first.paths),
        (second.texts, second.paths),
        "criterion 13"
    );
}

/// Criteria 1 to 3 on the posters: at least 7 of 8 lines exact, 0 wrong sure letters.
#[test]
#[ignore = "harness: prints the numbers of the PR"]
fn the_posters_read_right() {
    let (exact, wrong, run) = report("poster.png", &POSTER);
    assert!(exact >= 7 && wrong == 0);
    // Criterion 6: the five shapes whole, the boxes under SALE and FRESH among them, no letter
    // left as a path.
    assert_eq!(run.paths, 5);
    // Criterion 4: the line in Liberation Mono (not included) is set in the nearest face, every
    // letter guessed.
    let mono = run.texts.iter().find(|t| t.0.starts_with("Art.")).unwrap();
    assert_eq!(mono.1.len(), mono.0.chars().filter(|c| *c != ' ').count());
    let (exact, wrong, _) = report("poster_noisy.jpg", &POSTER);
    assert!(exact >= 6 && wrong == 0);
}
