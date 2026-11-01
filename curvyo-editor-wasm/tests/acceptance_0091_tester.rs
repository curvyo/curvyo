//! Tester cases for `specs/0091-trace-text-recognition` slice 1 in the editor session: the
//! tester's own posters (`curvyo-ocr-core/tests/fixtures/tester/`, `make_posters.py`) read with
//! the shipped weights, traced with the text pass, fitted to the included faces by the worker's
//! fit (`FitWorker`, as the trace worker runs it) and written by Create paths. Written from the
//! specification; only public names were looked up.
//!
//! The native numbers of the poster are kept in `tests/fixtures/acceptance_0091_tester_native.json`
//! (`CURVYO_BLESS=1` rewrites it); `frontend/tests/acceptance_0091_tester.test.ts` runs the same
//! inputs through the editor wasm in Chromium and compares (criterion 13, ADR 0027 §6).
//!
//! Timings (release, `#[ignore]`):
//! `RAYON_NUM_THREADS=1 cargo test --release -p curvyo-editor-wasm --test acceptance_0091_tester
//! -- --ignored --nocapture`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

mod cpu_time;

use std::path::PathBuf;

use curvyo_document_core::{
    Color, Document, FontKind, NodeId, ObjectSnapshot, TextSnapshot, unpack,
};
use curvyo_editor_wasm::{
    FitWorker, ObjectToPathOutcome, RowPress, Session, Tool, TraceCreateOutcome, TracedText,
    ocr_lines_json, read_ocr_lines, trace_settings,
};
use curvyo_ui_core::{
    FontBlock, OcrLine, OcrModel, TraceWire, read_trace_wire, trace_job_with, trace_read_text,
    trace_text_job_with,
};
use kurbo::{Affine, BezPath, Rect, Shape};
use serde_json::Value;

const TOLERANCE_MM: f64 = 0.3;
const SETTINGS: &str = r#"{"threshold":128}"#;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn fonts_dir() -> PathBuf {
    root().join("frontend/public/fonts")
}

fn poster(file: &str) -> PathBuf {
    root()
        .join("curvyo-ocr-core/tests/fixtures/tester")
        .join(file)
}

fn golden() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/acceptance_0091_tester_native.json")
}

fn model() -> OcrModel {
    let dir = root().join("frontend/public/models/paddleocr-latin");
    let read = |name: &str| std::fs::read(dir.join(name)).unwrap();
    let alphabet = String::from_utf8(read("dict.txt")).unwrap();
    OcrModel::load(&read("det.onnx"), &read("rec.onnx"), &alphabet).unwrap()
}

/// Every included face as `index.json` lists it.
fn included() -> Vec<(String, String, String, u32, FontKind)> {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            let kind = if f["kind"] == "single_line" {
                FontKind::SingleLine
            } else {
                FontKind::Outline
            };
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
                kind,
            )
        })
        .collect()
}

/// The truth lines of the tester poster, top to bottom.
const TRUTH: [&str; 8] = [
    "Größe",
    "Übermaß",
    "Äpfel & Öl",
    "Tel. 0421 98765",
    "Fräsen ab 3 Stück, 12.50 EUR",
    "Straße 7",
    "Mono 314 Zoll",
    "Brücke über Täler",
];
/// The line in Liberation Mono Bold, a face Curvyo does not include.
const FOREIGN: &str = "Mono 314 Zoll";
/// The black box under "Straße 7", in source pixels (`make_posters.py`).
const BOX_PX: Rect = Rect::new(110.0, 1130.0, 1211.0, 1391.0);

/// Hands the session every font file it asks for.
fn load_fonts(s: &mut Session) {
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes), "{file} loads");
    }
}

/// A path snapshot's outlines as one `BezPath` in document mm.
fn bez(p: &curvyo_document_core::PathSnapshot) -> BezPath {
    let mut out = BezPath::new();
    for sub in p.subpaths() {
        let pt = |a: &curvyo_document_core::AnchorSnapshot| kurbo::Point::new(a.point.x, a.point.y);
        let Some(first) = sub.anchors.first() else {
            continue;
        };
        out.move_to(pt(first));
        let mut segs: Vec<(usize, usize)> = (1..sub.anchors.len()).map(|i| (i - 1, i)).collect();
        if sub.closed {
            segs.push((sub.anchors.len() - 1, 0));
        }
        for (i, j) in segs {
            let (a, b) = (&sub.anchors[i], &sub.anchors[j]);
            out.curve_to(
                pt(a) + kurbo::Vec2::new(a.handle_out.x, a.handle_out.y),
                pt(b) + kurbo::Vec2::new(b.handle_in.x, b.handle_in.y),
                pt(b),
            );
        }
        if sub.closed {
            out.close_path();
        }
    }
    out
}

struct Host {
    s: Session,
    image: NodeId,
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    map: [f64; 6],
}

impl Host {
    fn new(file: &str) -> Self {
        Self::with_bytes(&std::fs::read(poster(file)).unwrap(), file)
    }

    fn with_bytes(bytes: &[u8], file: &str) -> Self {
        let mut s = Session::new(91);
        s.resize_viewport(1600.0, 1000.0);
        s.set_font_list_with_kinds(FontBlock::Included, included(), Vec::new());
        load_fonts(&mut s);
        s.import_begin(None);
        s.import_add(bytes, file);
        s.import_finish(false);
        s.set_tool(Tool::Trace);
        let image = s.trace_view().shown.expect("the image is shown");
        let source = s.trace_source(image, false).unwrap();
        Self {
            s,
            image,
            rgba: source.rgba,
            width: source.width,
            height: source.height,
            map: source.map,
        }
    }

    fn load_fonts(&mut self) {
        load_fonts(&mut self.s);
    }

    fn read(&self, model: &OcrModel) -> Vec<OcrLine> {
        trace_read_text(model, &self.rgba, self.width, self.height, &mut |_| {
            std::ops::ControlFlow::Continue(())
        })
        .unwrap()
    }

    fn wire(&self, settings: &str, lines: &[OcrLine]) -> Vec<f64> {
        trace_text_job_with(
            &self.rgba,
            self.width,
            self.height,
            self.map,
            &trace_settings(settings),
            lines,
            &mut |_| {},
        )
    }

    fn plain_wire(&self, settings: &str) -> Vec<f64> {
        trace_job_with(
            &self.rgba,
            self.width,
            self.height,
            self.map,
            &trace_settings(settings),
            &mut |_| {},
        )
    }

    /// One trace with `lines` cached and shown as preview and result, then the fit as the trace
    /// worker runs it; returns the fit's request and reply (empty without an accepted line).
    fn trace_once(&mut self, wire: &[f64]) -> (String, String) {
        assert!(self.s.set_trace_preview(self.image, wire));
        assert!(self.s.set_trace_result(self.image, wire));
        let Some(request) = self.s.trace_fit_request(TOLERANCE_MM, &[]) else {
            return (String::new(), String::new());
        };
        let reply = fit(&request);
        assert!(self.s.set_trace_fit(&request, &reply));
        assert!(self.s.trace_text_fitted());
        (request, reply)
    }

    /// The host's whole flow (`useTrace`): cache `lines`, trace with `settings`, fit; when the fit
    /// set no text for some lines (lead decision 1), trace again with the session's lines and fit
    /// again. Returns the last fit's request and reply and the last wire.
    fn trace_and_fit(&mut self, settings: &str, lines: &[OcrLine]) -> (String, String, Vec<f64>) {
        assert!(
            self.s
                .set_trace_ocr_lines(self.image, lines.to_vec(), (self.width, self.height))
        );
        let mut wire = self.wire(settings, lines);
        let (mut request, mut reply) = self.trace_once(&wire);
        if !request.is_empty() && self.s.trace_fit_dropped() > 0 {
            let (again, size) = self.s.trace_lines_to_trace(self.image).expect("kept lines");
            assert_eq!(size, (self.width, self.height));
            wire = self.wire(settings, &again);
            (request, reply) = self.trace_once(&wire);
        }
        (request, reply, wire)
    }

    fn doc(&self) -> Document {
        unpack(77, &self.s.pack("0.1.0").unwrap()).expect("the session output reopens")
    }

    /// Every leaf in z-order (bottom first, as the layer tree's leaves).
    fn leaves(&self) -> Vec<ObjectSnapshot> {
        let d = self.doc();
        d.object_ids()
            .into_iter()
            .filter_map(|id| d.object(id))
            .collect()
    }

    fn texts(&self) -> Vec<TextSnapshot> {
        self.leaves()
            .into_iter()
            .filter_map(|o| match o {
                ObjectSnapshot::Text(t) => Some(*t),
                _ => None,
            })
            .collect()
    }

    fn fingerprint(&self) -> Vec<String> {
        let d = self.doc();
        d.object_ids()
            .into_iter()
            .map(|id| format!("{id:?} {:?}", d.object(id)))
            .collect()
    }

    fn create(&mut self) -> TraceCreateOutcome {
        let steps = self.s.undo_len();
        let out = self.s.create_traced_paths();
        assert_eq!(
            self.s.undo_len(),
            steps + 1,
            "criterion 11: one history step"
        );
        out
    }
}

/// The worker's side of the fit (`trace_fit_glue.rs`): start, hand in the faces it asks for, step
/// until finished.
fn fit(request: &str) -> String {
    let mut worker = FitWorker::default();
    let mut reply = worker.start(request, &mut |_| {});
    for _ in 0..64 {
        let value: Value = serde_json::from_str(&reply).unwrap();
        if value["kind"] == "finished" {
            return reply;
        }
        for face in value["faces"].as_array().unwrap() {
            let file = face[0].as_str().unwrap();
            let bytes = std::fs::read(fonts_dir().join(file)).unwrap();
            assert!(worker.add_face(
                file,
                u32::try_from(face[1].as_u64().unwrap()).unwrap(),
                &bytes
            ));
        }
        reply = worker.step(&mut |_| {});
    }
    panic!("the fit never finished");
}

/// One whole run on `file`: OCR, text pass with `settings`, fit, Create paths.
struct Run {
    host: Host,
    lines: Vec<OcrLine>,
    wire: Vec<f64>,
    reply: String,
    before: Vec<String>,
    created: Vec<TracedText>,
    outcome: TraceCreateOutcome,
}

fn run_with(model: &OcrModel, file: &str, settings: &str) -> Run {
    let mut host = Host::new(file);
    let lines = host.read(model);
    let (_, reply, wire) = host.trace_and_fit(settings, &lines);
    host.load_fonts();
    let before = host.fingerprint();
    let outcome = host.create();
    host.load_fonts();
    let created = host.s.trace_created_texts().to_vec();
    Run {
        host,
        lines,
        wire,
        reply,
        before,
        created,
        outcome,
    }
}

fn run(model: &OcrModel, file: &str) -> Run {
    run_with(model, file, SETTINGS)
}

fn traced(wire: &[f64]) -> TraceWire {
    read_trace_wire(wire).expect("a readable wire")
}

/// The positions (0-based, in characters) of `text`'s guessed letters.
fn guessed_of(created: &[TracedText], text: &str) -> Vec<usize> {
    created
        .iter()
        .find(|c| c.string == text)
        .map(|c| c.guessed.iter().map(|g| g.1 - 1).collect())
        .unwrap_or_default()
}

/// The truth line closest to `text` (same length, fewest differing characters).
fn truth_of(text: &str) -> Option<&'static str> {
    TRUTH
        .iter()
        .filter(|t| t.chars().count() == text.chars().count())
        .min_by_key(|t| t.chars().zip(text.chars()).filter(|(a, b)| a != b).count())
        .copied()
}

/// The letters (not spaces) of the run, how many are sure and guessed, and how many sure ones
/// are wrong against the truth. A text with no truth line of its length counts all its sure
/// letters as wrong.
fn letter_counts(texts: &[TextSnapshot], created: &[TracedText]) -> (usize, usize, usize, usize) {
    let (mut letters, mut sure, mut guessed, mut wrong) = (0, 0, 0, 0);
    for t in texts {
        let g = guessed_of(created, &t.string);
        let truth = truth_of(&t.string);
        for (i, c) in t.string.chars().enumerate() {
            if c.is_whitespace() {
                continue;
            }
            letters += 1;
            if g.contains(&i) {
                guessed += 1;
                continue;
            }
            sure += 1;
            if truth.is_none_or(|t| t.chars().nth(i) != Some(c)) {
                wrong += 1;
                eprintln!("wrong sure letter {c:?} at {} of {:?}", i + 1, t.string);
            }
        }
    }
    (letters, sure, guessed, wrong)
}

fn bbox(paths: &[BezPath]) -> Rect {
    paths
        .iter()
        .map(Shape::bounding_box)
        .reduce(|a, b| a.union(b))
        .unwrap_or(Rect::ZERO)
}

fn px_rect_to_mm(map: [f64; 6], r: Rect) -> Rect {
    let a = Affine::new(map);
    let p = [
        a * kurbo::Point::new(r.x0, r.y0),
        a * kurbo::Point::new(r.x1, r.y1),
    ];
    Rect::from_points(p[0], p[1])
}

fn quad_rect_mm(map: [f64; 6], line: &OcrLine) -> Rect {
    let a = Affine::new(map);
    line.quad
        .iter()
        .map(|q| a * kurbo::Point::new(q[0], q[1]))
        .fold(Rect::new(f64::MAX, f64::MAX, f64::MIN, f64::MIN), |r, p| {
            Rect::new(r.x0.min(p.x), r.y0.min(p.y), r.x1.max(p.x), r.y1.max(p.y))
        })
}

/// The ink box of the poster line `text` (case-insensitive, `boxes.toml`) in document mm: the
/// pixels inside the renderer's box grown by 6 px whose grey differs from the box's corner by more
/// than half, mapped by the picture's pixel map.
fn ink_box_mm(host: &Host, text: &str) -> Rect {
    let boxes: Value = toml_lines();
    let entry = boxes
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["text"].as_str().unwrap().to_lowercase() == text.to_lowercase())
        .unwrap_or_else(|| panic!("{text:?} is a poster line"));
    let b: Vec<i64> = entry["box"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect();
    let (w, h) = (i64::from(host.width), i64::from(host.height));
    let (x0, y0) = ((b[0] - 6).max(0), (b[1] - 6).max(0));
    let (x1, y1) = ((b[2] + 6).min(w - 1), (b[3] + 6).min(h - 1));
    let grey = |x: i64, y: i64| i64::from(host.rgba[((y * w + x) * 4) as usize]);
    let ground = grey(x0, y0);
    let mut ink = Rect::new(f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for y in y0..=y1 {
        for x in x0..=x1 {
            if (grey(x, y) - ground).abs() > 128 {
                ink = Rect::new(
                    ink.x0.min(x as f64),
                    ink.y0.min(y as f64),
                    ink.x1.max(x as f64 + 1.0),
                    ink.y1.max(y as f64 + 1.0),
                );
            }
        }
    }
    px_rect_to_mm(host.map, ink)
}

fn toml_lines() -> Value {
    let text = std::fs::read_to_string(poster("boxes.toml")).unwrap();
    let mut lines = Vec::new();
    let mut current: Option<(String, Value)> = None;
    for line in text.lines() {
        if let Some(t) = line.strip_prefix("text = ") {
            current = Some((serde_json::from_str::<String>(t).unwrap(), Value::Null));
        } else if let Some(b) = line.strip_prefix("box = ") {
            let (t, _) = current.take().unwrap();
            lines.push(
                serde_json::json!({ "text": t, "box": serde_json::from_str::<Value>(b).unwrap() }),
            );
        }
    }
    Value::Array(lines)
}

/// The largest distance between matching edges of two boxes.
fn edge_gap(a: Rect, b: Rect) -> f64 {
    [
        (a.x0 - b.x0).abs(),
        (a.y0 - b.y0).abs(),
        (a.x1 - b.x1).abs(),
        (a.y1 - b.y1).abs(),
    ]
    .into_iter()
    .fold(0.0, f64::max)
}

/// The box of each subpath of `path`.
fn subpath_boxes(path: &BezPath) -> Vec<Rect> {
    let mut subs = Vec::new();
    let mut cur = BezPath::new();
    for el in path.elements() {
        if matches!(el, kurbo::PathEl::MoveTo(_)) && !cur.elements().is_empty() {
            subs.push(std::mem::take(&mut cur).bounding_box());
        }
        cur.push(*el);
    }
    if !cur.elements().is_empty() {
        subs.push(cur.bounding_box());
    }
    subs
}

/// Criteria 1 and 2 on the five-text poster (lead decision 1 as amended: a line is text only with
/// a sure letter in its face): every text is its rendered line exactly, at least 5 lines become
/// text, and over those lines every umlaut and ß is at its place, each of ä ö ü ß Ä Ö Ü at least
/// once ("Brücke über Täler" carries ä and ü now that "Fräsen…" stays paths).
#[test]
fn c1_c2_poster_lines_become_exact_texts_with_umlauts_and_sharp_s() {
    let model = model();
    let r = run(&model, "poster.png");
    let texts = r.host.texts();
    let strings: Vec<&str> = texts.iter().map(|t| t.string.as_str()).collect();
    eprintln!("texts: {strings:?}");
    for t in &strings {
        assert!(TRUTH.contains(t), "{t:?} is not a rendered line exactly");
    }
    assert!(strings.len() >= 5, "{strings:?}");
    let mut seen = String::new();
    for t in &texts {
        for c in t.string.chars().filter(|c| "äöüßÄÖÜ".contains(*c)) {
            seen.push(c);
        }
    }
    eprintln!("umlauts and ß set: {seen:?}");
    for c in "äöüßÄÖÜ".chars() {
        assert!(seen.contains(c), "{c} is set in no text: {strings:?}");
    }
    assert!(seen.chars().count() >= 9, "{seen:?}");
}

/// Criterion 3: at the default tolerance no sure letter is wrong; prints the letter counts.
#[test]
fn c3_poster_has_no_wrong_sure_letter() {
    let model = model();
    let r = run(&model, "poster.png");
    for c in &r.created {
        eprintln!(
            "  {:?}: guessed {:?}, nearest {}",
            c.string, c.guessed, c.nearest
        );
    }
    let (letters, sure, guessed, wrong) = letter_counts(&r.host.texts(), &r.created);
    eprintln!("poster: {letters} letters, {sure} sure, {guessed} guessed, {wrong} wrong sure");
    assert_eq!(wrong, 0);
}

/// Criterion 4 as amended (lead decision 1): the line in a face Curvyo does not include has no
/// sure letter in any included face, so it becomes no text and stays traced paths (its letters are
/// in the final trace); every text set in the nearest face (114 B) has at least one sure letter,
/// its other letters guessed and held by the Glyph table / Text tool record.
#[test]
fn c4_a_foreign_line_without_a_sure_letter_stays_paths_and_nearest_texts_have_one() {
    let model = model();
    let r = run(&model, "poster.png");
    let texts = r.host.texts();
    assert!(
        !texts
            .iter()
            .any(|t| t.string.to_lowercase() == FOREIGN.to_lowercase()),
        "the foreign line is no text: {texts:?}"
    );
    let ink = ink_box_mm(&r.host, FOREIGN);
    let wire = traced(&r.wire);
    let inside = wire
        .outlines
        .iter()
        .flat_map(subpath_boxes)
        .filter(|b| ink.inflate(0.5, 0.5).contains_rect(*b))
        .count();
    eprintln!("foreign line: {inside} traced outlines inside its ink box");
    assert!(inside >= 10, "its letters stay traced paths");
    let mut nearest = 0;
    for t in &texts {
        let created = r.created.iter().find(|c| c.string == t.string).unwrap();
        if !created.nearest {
            continue;
        }
        nearest += 1;
        let letters = t.string.chars().filter(|c| !c.is_whitespace()).count();
        eprintln!(
            "nearest-face text {:?} in {} {}: {} of {letters} guessed",
            t.string,
            t.font.family,
            t.font.style,
            created.guessed.len()
        );
        assert!(created.guessed.len() < letters, "at least one sure letter");
        assert_eq!(
            r.host.s.recognise_guessed(t.id).map(|h| h.len()),
            Some(created.guessed.len()),
            "the Glyph table / Text tool record"
        );
    }
    eprintln!("{nearest} texts set in the nearest face");
}

/// Lead decision 1: every text of two or more letters has at least one sure letter (a sure single
/// letter may be marked guessed, 0088 `MIN_SURE_LETTERS`).
#[test]
fn c4_every_text_has_a_sure_letter() {
    let model = model();
    let r = run(&model, "poster.png");
    for t in r.host.texts() {
        let letters = t.string.chars().filter(|c| !c.is_whitespace()).count();
        let guessed = guessed_of(&r.created, &t.string).len();
        if letters >= 2 {
            assert!(guessed < letters, "{:?} has no sure letter", t.string);
        }
    }
}

/// Criterion 5 (native part): a line whose letters are all sure creates no guessed letter and no
/// session record, so there is nothing to warn about (the notice itself: the Chromium file).
#[test]
fn c5_a_clean_line_leaves_nothing_guessed() {
    let model = model();
    let r = run(&model, "clean.png");
    assert_eq!(r.created.len(), 1, "{:?}", r.created);
    assert_eq!(r.created[0].string, "Größe");
    assert_eq!(r.created[0].guessed, Vec::new(), "{:?}", r.created);
    assert!(!r.created[0].nearest);
    let text = r.host.texts().pop().unwrap();
    assert!(
        r.host
            .s
            .recognise_guessed(text.id)
            .is_none_or(|g| g.is_empty()),
        "no highlight record"
    );
}

/// Criterion 6 (113 A): the box under "Straße 7" is traced whole, one outline with no
/// letter-shaped holes inside it, and every text lies above the traced paths.
#[test]
fn c6_the_shape_under_text_stays_whole_and_the_text_lies_on_top() {
    let model = model();
    let r = run(&model, "poster.png");
    let wire = traced(&r.wire);
    let boxed = px_rect_to_mm(r.host.map, BOX_PX);
    let inside: Vec<Rect> = wire
        .outlines
        .iter()
        .flat_map(|p| {
            // Each subpath of a compound outline on its own.
            let mut subs = Vec::new();
            let mut cur = BezPath::new();
            for el in p.elements() {
                if matches!(el, kurbo::PathEl::MoveTo(_)) && !cur.elements().is_empty() {
                    subs.push(std::mem::take(&mut cur));
                }
                cur.push(*el);
            }
            if !cur.elements().is_empty() {
                subs.push(cur);
            }
            subs.into_iter().map(|s| s.bounding_box())
        })
        .filter(|b| boxed.inflate(0.5, 0.5).contains_rect(*b))
        .collect();
    eprintln!("outlines inside the box: {inside:?} (box {boxed:?})");
    assert_eq!(inside.len(), 1, "only the box itself, no letter hole");
    let b = inside[0];
    let px = (r.host.map[0].powi(2) + r.host.map[1].powi(2)).sqrt();
    for (got, want) in [
        (b.x0, boxed.x0),
        (b.y0, boxed.y0),
        (b.x1, boxed.x1),
        (b.y1, boxed.y1),
    ] {
        assert!((got - want).abs() <= 2.0 * px, "box edge {got} vs {want}");
    }
    let leaves = r.host.leaves();
    let last_path = leaves
        .iter()
        .rposition(|o| matches!(o, ObjectSnapshot::Path(_)))
        .expect("traced paths");
    let first_text = leaves
        .iter()
        .position(|o| matches!(o, ObjectSnapshot::Text(_)))
        .expect("texts");
    assert!(
        first_text > last_path,
        "texts lie above the traced paths (z order {first_text} vs {last_path})"
    );
}

/// Criterion 7: at a global threshold where the small line breaks up, the text still comes out
/// right, and every outline away from the text boxes is the outline of the trace with Recognise
/// text off.
#[test]
fn c7_text_uses_its_own_threshold_and_the_rest_of_the_trace_is_unchanged() {
    let model = model();
    let dark = r#"{"threshold":40}"#;
    let r = run_with(&model, "poster.png", dark);
    let strings: Vec<String> = r.host.texts().into_iter().map(|t| t.string).collect();
    eprintln!("at threshold 40: {strings:?}");
    // The smallest line that becomes text (72 px) and the largest.
    assert!(strings.iter().any(|s| s == "Tel. 0421 98765"));
    assert!(strings.iter().any(|s| s == "Größe"));
    let on = traced(&r.wire);
    let off = traced(&r.host.plain_wire(dark));
    let boxes: Vec<Rect> = r
        .lines
        .iter()
        .filter(|l| l.accepted())
        .map(|l| quad_rect_mm(r.host.map, l).inflate(0.5, 0.5))
        .collect();
    let away = |paths: &[BezPath]| -> Vec<String> {
        let mut v: Vec<String> = paths
            .iter()
            .filter(|p| {
                boxes
                    .iter()
                    .all(|b| b.intersect(p.bounding_box()).area() <= 0.0)
            })
            .map(|p| format!("{:?}", p.elements()))
            .collect();
        v.sort();
        v
    };
    let (a, b) = (away(&on.outlines), away(&off.outlines));
    assert!(!b.is_empty(), "the circle is away from the text");
    assert_eq!(a, b, "outlines away from the text boxes are identical");
}

/// Criterion 8, at 7 and 30 degrees: one text, turned within 1 degree.
#[test]
fn c8_rotated_lines_are_turned_texts_within_one_degree() {
    let model = model();
    for (file, want, string) in [
        ("rotated_7.png", -7.0_f64, "Winkel sieben 7"),
        ("rotated_30.png", -30.0, "Winkel 30 Grad"),
    ] {
        let r = run(&model, file);
        let texts = r.host.texts();
        assert_eq!(texts.len(), 1, "{file}: {texts:?}");
        assert_eq!(texts[0].string, string);
        let m = texts[0].placement.to_array();
        let got = m[1].atan2(m[0]).to_degrees();
        eprintln!("{file}: text turned {got:.2} degrees");
        assert!((got - want).abs() <= 1.0, "{file}: {got} vs {want}");
    }
}

/// A line turned by 90 degrees: outside criterion 8 (15 degrees) but a likely real case. Whatever
/// is read must not create sure letters that are wrong (criterion 3's rule).
#[test]
fn a_line_turned_ninety_degrees_makes_no_wrong_sure_letter() {
    let model = model();
    let r = run(&model, "rotated_90.png");
    let texts = r.host.texts();
    eprintln!(
        "rotated_90: OCR {:?}, accepted {}, texts {}",
        r.lines
            .iter()
            .map(|l| (&l.text, l.accepted()))
            .collect::<Vec<_>>(),
        r.lines.iter().filter(|l| l.accepted()).count(),
        texts.len()
    );
    for t in &texts {
        let g = guessed_of(&r.created, &t.string);
        let sure: String = t
            .string
            .chars()
            .enumerate()
            .filter(|(i, c)| !c.is_whitespace() && !g.contains(i))
            .map(|(_, c)| c)
            .collect();
        eprintln!("rotated_90: text {:?}, sure letters {sure:?}", t.string);
        if t.string != "Neunzig 90" {
            assert!(sure.is_empty(), "a misread line has sure letters: {sure:?}");
        }
    }
}

/// Criterion 9: each all-sure text's outline matches its line's traced outlines within the
/// tolerance (box of the glyphs after Object to path against the box of the line's outlines).
#[test]
fn c9_texts_sit_on_their_traced_letters_within_the_tolerance() {
    let model = model();
    let mut r = run(&model, "poster.png");
    let wire = traced(&r.wire);
    let texts = r.host.texts();
    let mut checked = 0;
    for t in texts {
        if !guessed_of(&r.created, &t.string).is_empty() {
            continue;
        }
        let Some(line) = wire.texts.lines.iter().find(|l| l.text == t.string) else {
            continue;
        };
        let want = bbox(&line.outlines);
        r.host.s.set_tool(Tool::Select);
        r.host
            .s
            .press_layer_row(&serde_json::to_string(&t.id).unwrap(), RowPress::Plain);
        let before: Vec<NodeId> = r.host.doc().object_ids();
        assert_eq!(r.host.s.object_to_path(), ObjectToPathOutcome::Converted);
        let d = r.host.doc();
        let glyphs: Vec<BezPath> = d
            .object_ids()
            .into_iter()
            .filter(|id| !before.contains(id))
            .filter_map(|id| match d.object(id) {
                Some(ObjectSnapshot::Path(p)) => Some(bez(&p)),
                _ => None,
            })
            .collect();
        let got = bbox(&glyphs);
        let worst = [
            (got.x0 - want.x0).abs(),
            (got.y0 - want.y0).abs(),
            (got.x1 - want.x1).abs(),
            (got.y1 - want.y1).abs(),
        ]
        .into_iter()
        .fold(0.0, f64::max);
        eprintln!("{:?}: worst box edge {worst:.3} mm", t.string);
        assert!(worst <= TOLERANCE_MM, "{:?}: {got:?} vs {want:?}", t.string);
        checked += 1;
    }
    // Only "Größe" and "Straße 7" are all sure on this poster (c3 prints the guesses).
    assert!(checked >= 2, "{checked} all-sure lines checked");
}

/// Criterion 9's ground: each accepted line's own traced outlines (the text section of the
/// result) lie on the line's letters in the picture: their box matches the line's ink box (taken
/// from the poster's pixels, umlaut dots included) within the tolerance.
#[test]
fn c9_line_outlines_lie_on_the_picture_letters() {
    let model = model();
    let host = Host::new("poster.png");
    let lines = host.read(&model);
    let wire = traced(&host.wire(SETTINGS, &lines));
    let mut bad = Vec::new();
    for l in &wire.texts.lines {
        let want = ink_box_mm(&host, &l.text);
        let got = bbox(&l.outlines);
        let worst = edge_gap(got, want);
        eprintln!("{:?}: worst edge {worst:.3} mm from the ink", l.text);
        if worst > TOLERANCE_MM {
            bad.push((l.text.clone(), worst));
        }
    }
    assert!(wire.texts.lines.len() >= 6);
    assert!(
        bad.is_empty(),
        "line outlines away from their letters: {bad:?}"
    );
}

/// Criterion 9 for every text, guessed ones included: its glyphs lie on its line, inside the line's
/// ink box grown by 2 mm (a text set in the nearest face may differ in width, never in place).
#[test]
fn c9_every_text_lies_on_its_line() {
    let model = model();
    let mut r = run(&model, "poster.png");
    let mut bad = Vec::new();
    for t in r.host.texts() {
        let want = ink_box_mm(&r.host, &t.string);
        r.host.s.set_tool(Tool::Select);
        r.host
            .s
            .press_layer_row(&serde_json::to_string(&t.id).unwrap(), RowPress::Plain);
        let before: Vec<NodeId> = r.host.doc().object_ids();
        assert_eq!(r.host.s.object_to_path(), ObjectToPathOutcome::Converted);
        let d = r.host.doc();
        let glyphs: Vec<BezPath> = d
            .object_ids()
            .into_iter()
            .filter(|id| !before.contains(id))
            .filter_map(|id| match d.object(id) {
                Some(ObjectSnapshot::Path(p)) => Some(bez(&p)),
                _ => None,
            })
            .collect();
        let got = bbox(&glyphs);
        eprintln!(
            "{:?} ({} {}, size {:.2} mm): glyphs {got:?}, line {want:?}",
            t.string,
            t.font.family,
            t.font.style,
            t.size.as_mm()
        );
        if !want.inflate(2.0, 2.0).contains_rect(got) {
            bad.push(t.string.clone());
        }
    }
    assert!(bad.is_empty(), "texts away from their line: {bad:?}");
}

/// Criterion 10: every text is filled black, the white "Straße 7" on its black box included.
#[test]
fn c10_texts_are_filled_black_including_white_text_on_a_dark_box() {
    let model = model();
    let r = run(&model, "poster.png");
    let texts = r.host.texts();
    assert!(texts.iter().any(|t| t.string == "Straße 7"), "{texts:?}");
    for t in &texts {
        assert!(t.style.fill.enabled, "{:?}", t.string);
        assert_eq!(t.style.fill.color, Color::BLACK, "{:?}", t.string);
    }
}

/// Criterion 11: Create paths with several lines is one step; one undo leaves the document as it
/// was, one redo brings the same result back.
#[test]
fn c11_one_undo_removes_group_and_all_texts() {
    let model = model();
    let mut r = run(&model, "poster.png");
    let after = r.host.fingerprint();
    assert!(r.host.texts().len() >= 5);
    assert!(matches!(
        r.outcome,
        TraceCreateOutcome::Created { images: 1, .. }
    ));
    r.host.s.undo();
    assert_eq!(r.host.fingerprint(), r.before, "the document as before");
    assert_eq!(r.host.texts().len(), 0);
    r.host.s.redo();
    assert_eq!(r.host.fingerprint(), after, "redo brings the same result");
}

/// Criteria 12 and 18: a plain picture reads no line, and its trace with the (empty) lines is the
/// trace with Recognise text off; lines that are not accepted change nothing either.
#[test]
fn c12_c18_no_text_and_unaccepted_lines_trace_as_before() {
    let model = model();
    let host = Host::new("plain.png");
    let lines = host.read(&model);
    assert!(lines.is_empty(), "{lines:?}");
    assert_eq!(host.wire(SETTINGS, &lines), host.plain_wire(SETTINGS));
    let weak = vec![
        OcrLine {
            quad: [[90.0, 90.0], [410.0, 90.0], [410.0, 410.0], [90.0, 410.0]],
            text: "O".to_owned(),
            confidence: vec![0.2],
        },
        OcrLine {
            quad: [
                [480.0, 100.0],
                [780.0, 100.0],
                [780.0, 530.0],
                [480.0, 530.0],
            ],
            text: "- .".to_owned(),
            confidence: vec![0.99, 0.99, 0.99],
        },
    ];
    assert!(weak.iter().all(|l| !l.accepted()));
    let on = traced(&host.wire(SETTINGS, &weak));
    let off = traced(&host.plain_wire(SETTINGS));
    assert_eq!(on.outlines, off.outlines, "unaccepted lines trace as usual");
    assert_eq!((on.paths, on.nodes), (off.paths, off.nodes));
    assert_eq!(on.texts.lines.len(), 0);
}

/// A grey PNG of `width` x `height` with `grey(x, y)`.
fn grey_png(width: u32, height: u32, grey: impl Fn(u32, u32) -> u8) -> Vec<u8> {
    use image::ImageEncoder;
    let mut rgba = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let g = grey(x, y);
            rgba.extend([g, g, g, 255]);
        }
    }
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&rgba, width, height, image::ExtendedColorType::Rgba8)
        .unwrap();
    out
}

/// Criterion 18 as amended, judged on what the host ends with (trace, fit, trace again without
/// the lines the fit set no text for): a ring (the `0025` browser tests' black annulus, radii 20
/// and 60 px in 200 x 200 px) and a dot grid (the `0025` c55 picture, 800 x 800 px of 3 px dots
/// every 8 px) set no text, and their trace is the trace with Recognise text off.
#[test]
fn c18_a_plain_ring_and_a_dot_grid_stay_shapes_after_the_fit() {
    let model = model();
    let ring = grey_png(200, 200, |x, y| {
        let r = (f64::from(x) + 0.5 - 100.0).hypot(f64::from(y) + 0.5 - 100.0);
        if r < 60.0 && r > 20.0 { 0 } else { 255 }
    });
    let dots = grey_png(800, 800, |x, y| {
        let (dx, dy) = (f64::from(x % 8) - 3.5, f64::from(y % 8) - 3.5);
        if dx * dx + dy * dy < 9.0 { 0 } else { 255 }
    });
    for (name, png) in [("ring.png", ring), ("dots.png", dots)] {
        let mut host = Host::with_bytes(&png, name);
        let lines = host.read(&model);
        eprintln!(
            "{name}: OCR {:?}",
            lines
                .iter()
                .map(|l| (&l.text, l.accepted()))
                .collect::<Vec<_>>()
        );
        let (_, _, wire) = host.trace_and_fit(SETTINGS, &lines);
        let on = traced(&wire);
        let off = traced(&host.plain_wire(SETTINGS));
        assert_eq!(on.texts.lines.len(), 0, "{name}: no text line set");
        assert_eq!(
            (on.outlines.len(), on.nodes),
            (off.outlines.len(), off.nodes),
            "{name}"
        );
        assert_eq!(on.outlines, off.outlines, "{name}");
        let out = host.create();
        eprintln!("{name}: {out:?}");
        assert_eq!(host.texts().len(), 0, "{name}: no text object");
    }
}

/// Criterion 18 on a plain disc (control for the ring).
#[test]
fn c18_a_plain_disc_is_traced_as_a_shape() {
    let model = model();
    let disc = grey_png(200, 200, |x, y| {
        let r = (f64::from(x) + 0.5 - 100.0).hypot(f64::from(y) + 0.5 - 100.0);
        if r < 50.0 { 0 } else { 255 }
    });
    let host = Host::with_bytes(&disc, "disc.png");
    let lines = host.read(&model);
    eprintln!(
        "disc: {:?}",
        lines
            .iter()
            .map(|l| (&l.text, l.accepted()))
            .collect::<Vec<_>>()
    );
    let on = traced(&host.wire(SETTINGS, &lines));
    let off = traced(&host.plain_wire(SETTINGS));
    assert_eq!(on.texts.lines.len(), 0);
    assert_eq!(on.outlines, off.outlines);
}

/// Criterion 13: the same picture and settings twice give the same texts (content, face, place,
/// size, guessed marks) and the same paths.
#[test]
fn c13_two_runs_are_identical() {
    let model = model();
    let summary = |r: &Run| {
        (
            r.host
                .texts()
                .into_iter()
                .map(|t| {
                    (
                        t.string.clone(),
                        t.font.family,
                        t.font.style,
                        t.placement.to_array(),
                        t.size.as_mm(),
                        guessed_of(&r.created, &t.string),
                    )
                })
                .collect::<Vec<_>>(),
            r.wire.clone(),
        )
    };
    let a = run(&model, "poster.png");
    let b = run(&model, "poster.png");
    assert_eq!(a.lines, b.lines);
    assert_eq!(a.reply, b.reply);
    assert!(summary(&a) == summary(&b), "two runs differ");
}

/// Criterion 16 (session part): the read lines stay cached for the image across a preview with
/// other settings, so a second trace needs no OCR.
#[test]
fn c16_lines_stay_cached_across_a_settings_change() {
    let model = model();
    let mut host = Host::new("rotated_30.png");
    let lines = host.read(&model);
    host.trace_and_fit(SETTINGS, &lines);
    let other = host.wire(r#"{"threshold":90,"smooth":3}"#, &lines);
    assert!(host.s.set_trace_preview(host.image, &other));
    let (kept, size) = host.s.trace_ocr_lines(host.image).expect("cached lines");
    assert_eq!(kept, lines.as_slice());
    assert_eq!(size, (host.width, host.height));
}

/// The determinism record for the browser (criterion 13 across builds, ADR 0027 §6): the OCR
/// lines, the trace wire and the fit of the poster natively. `CURVYO_BLESS=1` rewrites it; else it
/// must match.
#[test]
fn native_record_of_the_poster_matches_the_golden_file() {
    let model = model();
    let host = Host::new("poster.png");
    let lines = host.read(&model);
    let ocr = ocr_lines_json(host.width, host.height, &lines);
    assert_eq!(
        read_ocr_lines(&ocr, host.width),
        lines,
        "the JSON round trip"
    );
    let wire = host.wire(SETTINGS, &lines);
    let mut host = host;
    // The first pass only: what the wasm functions in the browser reproduce from the OCR lines.
    assert!(
        host.s
            .set_trace_ocr_lines(host.image, lines.clone(), (host.width, host.height))
    );
    let (request, reply) = host.trace_once(&wire);
    let fnv = host.rgba.iter().fold(0x811c_9dc5_u32, |h, b| {
        (h ^ u32::from(*b)).wrapping_mul(0x0100_0193)
    });
    let record = serde_json::json!({
        "width": host.width,
        "height": host.height,
        "rgba_fnv32": fnv,
        "settings": SETTINGS,
        "map": host.map,
        "ocr": ocr,
        "wire": wire,
        "fit_request": request,
        "fit_reply": reply,
    });
    let text = serde_json::to_string(&record).unwrap();
    if std::env::var_os("CURVYO_BLESS").is_some() {
        std::fs::write(golden(), &text).unwrap();
    }
    let kept: Value = serde_json::from_str(&std::fs::read_to_string(golden()).unwrap()).unwrap();
    assert_eq!(kept["ocr"], record["ocr"]);
    assert_eq!(kept["wire"], record["wire"]);
    assert_eq!(kept["fit_request"], record["fit_request"]);
    assert_eq!(kept["fit_reply"], record["fit_reply"]);
}

/// Criteria 20 and 21 natively (Proposal targets are for the wasm worker): OCR of the tester
/// poster as process CPU with one `rayon` thread, the text pass with trace and the font fit as
/// thread CPU; best of 3.
#[test]
#[ignore = "timing, release only"]
fn timings_of_the_tester_poster() {
    let model = model();
    let mut host = Host::new("poster.png");
    let process = || {
        let text = std::fs::read_to_string("/proc/self/stat").unwrap();
        let fields: Vec<&str> = text
            .rsplit(')')
            .next()
            .unwrap()
            .split_whitespace()
            .collect();
        let ticks: u64 = fields[11].parse::<u64>().unwrap() + fields[12].parse::<u64>().unwrap();
        ticks as f64 / 100.0
    };
    let (mut ocr, mut pass, mut fit_s) = (f64::MAX, f64::MAX, f64::MAX);
    let mut lines = Vec::new();
    for _ in 0..3 {
        let start = process();
        lines = host.read(&model);
        ocr = ocr.min(process() - start);
        let (wire, t) = cpu_time::cpu_time_of(|| host.wire(SETTINGS, &lines));
        pass = pass.min(t.as_secs_f64());
        assert!(
            host.s
                .set_trace_ocr_lines(host.image, lines.clone(), (host.width, host.height))
        );
        assert!(host.s.set_trace_result(host.image, &wire));
        assert!(host.s.set_trace_preview(host.image, &wire));
        let request = host.s.trace_fit_request(TOLERANCE_MM, &[]).unwrap();
        let (_, t) = cpu_time::cpu_time_of(|| fit(&request));
        fit_s = fit_s.min(t.as_secs_f64());
    }
    let accepted = lines.iter().filter(|l| l.accepted()).count();
    eprintln!(
        "tester poster native: OCR {ocr:.2} s (process CPU, RAYON_NUM_THREADS={:?}), text pass with \
         trace {pass:.2} s for {accepted} lines, font fit {fit_s:.2} s (thread CPU)",
        std::env::var("RAYON_NUM_THREADS").ok()
    );
}
