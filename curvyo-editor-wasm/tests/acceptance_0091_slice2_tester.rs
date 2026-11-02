//! Tester cases for `specs/0091-trace-text-recognition` slice 2 (criteria 23 and 24): text
//! recognition in the multicolour Trace, on the tester's own colour posters
//! (`curvyo-ocr-core/tests/fixtures/tester/make_colour.py`): red, green (a colour only the text
//! has), blue and black text on white, white text on a blue box, a red disc, umlauts and ß.
//! Written from the specification and adrs.md decision 6; only public names were looked up.
//!
//! The native record of the colour poster is kept in
//! `tests/fixtures/acceptance_0091_slice2_tester_native.json` (`CURVYO_BLESS=1` rewrites it);
//! `frontend/tests/acceptance_0091_tester.test.ts` (slice 2 section) runs the same inputs through
//! the editor wasm in Chromium and compares (criterion 24 with 13). The plain multicolour trace
//! of `main` before this slice is compared by `acceptance_0091_slice2_off_tester.rs`
//! (criterion 24 with 12).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    missing_docs
)]

use std::path::PathBuf;

use curvyo_document_core::{
    Document, FontKind, NodeId, ObjectSnapshot, PathSnapshot, TextSnapshot, unpack,
};
use curvyo_editor_wasm::{
    FitWorker, Session, Tool, TraceCreateOutcome, TracedText, ocr_lines_json, read_ocr_lines,
    trace_colour_settings, trace_settings,
};
use curvyo_ui_core::{
    FontBlock, OcrLine, OcrModel, TraceRgb, TraceWire, read_trace_wire, text_pass,
    trace_colour_job_with, trace_read_text, trace_text_colour_job_with, trace_text_regions_with,
};
use kurbo::{Affine, BezPath, Rect, Shape};
use serde_json::Value;

const TOLERANCE_MM: f64 = 0.3;
/// The frontend's first-use Multicolour settings (`settingsJson`): Brightness, automatic count.
const MULTI: &str = r#"{"output":1,"colours":null,"fixed":[],"excluded":[],"merged":[],"method":0,"background":null,"swatches":[],"threshold":128}"#;
/// Multicolour with Remove background on the white paper.
const MULTI_RB: &str =
    r##"{"output":1,"colours":null,"method":1,"background":"#ffffff","threshold":128}"##;
/// Multicolour with a fixed count of 4.
const MULTI_4: &str = r#"{"output":1,"colours":4,"method":0,"threshold":128}"#;

const WHITE: TraceRgb = TraceRgb::new(255, 255, 255);
/// The blue box under "Straße 7" and the red disc, in source pixels (`make_colour.py`).
const BOX_PX: Rect = Rect::new(110.0, 830.0, 1211.0, 1091.0);
const DISC_PX: Rect = Rect::new(1500.0, 1150.0, 1881.0, 1531.0);

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn fonts_dir() -> PathBuf {
    root().join("frontend/public/fonts")
}

fn fixture(file: &str) -> PathBuf {
    root()
        .join("curvyo-ocr-core/tests/fixtures/tester")
        .join(file)
}

fn golden() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/acceptance_0091_slice2_tester_native.json")
}

fn model() -> OcrModel {
    let dir = root().join("frontend/public/models/paddleocr-latin");
    let read = |name: &str| std::fs::read(dir.join(name)).unwrap();
    let alphabet = String::from_utf8(read("dict.txt")).unwrap();
    OcrModel::load(&read("det.onnx"), &read("rec.onnx"), &alphabet).unwrap()
}

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

/// One line of `colour_boxes.toml`.
#[derive(Debug, Clone)]
struct Truth {
    file: String,
    text: String,
    colour: TraceRgb,
    ink: Rect,
}

fn truths() -> Vec<Truth> {
    let text = std::fs::read_to_string(fixture("colour_boxes.toml")).unwrap();
    let mut out = Vec::new();
    let (mut file, mut string, mut colour) = (String::new(), String::new(), WHITE);
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("file = ") {
            file = serde_json::from_str(v).unwrap();
        } else if let Some(v) = line.strip_prefix("text = ") {
            string = serde_json::from_str(v).unwrap();
        } else if let Some(v) = line.strip_prefix("colour = ") {
            let c: Vec<u8> = serde_json::from_str(v).unwrap();
            colour = TraceRgb::new(c[0], c[1], c[2]);
        } else if let Some(v) = line.strip_prefix("box = ") {
            let b: Vec<f64> = serde_json::from_str(v).unwrap();
            out.push(Truth {
                file: file.clone(),
                text: string.clone(),
                colour,
                ink: Rect::new(b[0], b[1], b[2], b[3]),
            });
        }
    }
    out
}

fn truths_of(file: &str) -> Vec<Truth> {
    truths().into_iter().filter(|t| t.file == file).collect()
}

fn load_fonts(s: &mut Session) {
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes), "{file} loads");
    }
}

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

fn traced(wire: &[f64]) -> TraceWire {
    read_trace_wire(wire).expect("a readable wire")
}

fn dist2(a: TraceRgb, b: TraceRgb) -> i32 {
    let d = |x: u8, y: u8| i32::from(x) - i32::from(y);
    d(a.r, b.r).pow(2) + d(a.g, b.g).pow(2) + d(a.b, b.b).pow(2)
}

fn rgb(c: curvyo_document_core::Color) -> TraceRgb {
    TraceRgb::new(c.r, c.g, c.b)
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
        let bytes = std::fs::read(fixture(file)).unwrap();
        let mut s = Session::new(91);
        s.resize_viewport(1600.0, 1000.0);
        s.set_font_list_with_kinds(FontBlock::Included, included(), Vec::new());
        load_fonts(&mut s);
        s.import_begin(None);
        s.import_add(&bytes, file);
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

    fn read(&self, model: &OcrModel) -> Vec<OcrLine> {
        trace_read_text(model, &self.rgba, self.width, self.height, &mut |_| {
            std::ops::ControlFlow::Continue(())
        })
        .unwrap()
    }

    /// The multicolour job with `lines` (`trace_text_job` of the wasm with Multicolour settings).
    fn wire(&self, settings: &str, lines: &[OcrLine]) -> Vec<f64> {
        let colour = trace_colour_settings(settings).expect("multicolour settings");
        trace_text_colour_job_with(
            &self.rgba,
            self.width,
            self.height,
            self.map,
            &colour,
            lines,
            &mut |_| {},
        )
    }

    /// The plain multicolour trace (Recognise text off: the host calls `trace_job`).
    fn plain_wire(&self, settings: &str) -> Vec<f64> {
        let colour = trace_colour_settings(settings).expect("multicolour settings");
        trace_colour_job_with(
            &self.rgba,
            self.width,
            self.height,
            self.map,
            &colour,
            &mut |_| {},
        )
    }

    /// The host's flow (`useTrace`): cache the lines, the text pass alone, the fit, then one
    /// multicolour trace with the lines the fit decided.
    fn trace_and_fit(&mut self, settings: &str, lines: &[OcrLine]) -> (String, String, Vec<f64>) {
        assert!(
            self.s
                .set_trace_ocr_lines(self.image, lines.to_vec(), (self.width, self.height))
        );
        let regions = trace_text_regions_with(
            &self.rgba,
            self.width,
            self.height,
            self.map,
            &trace_settings(settings),
            lines,
        );
        let (request, reply) =
            match self
                .s
                .trace_fit_request_for(self.image, &regions, TOLERANCE_MM, &[])
            {
                Some(request) => {
                    let reply = fit(&request);
                    assert!(self.s.set_trace_fit(&request, &reply));
                    (request, reply)
                }
                None => (String::new(), String::new()),
            };
        let (decided, _) = self.s.trace_lines_to_trace(self.image).expect("kept lines");
        let wire = self.wire(settings, &decided);
        assert!(self.s.set_trace_preview(self.image, &wire));
        assert!(self.s.set_trace_result(self.image, &wire));
        if !request.is_empty() {
            assert!(self.s.trace_text_fitted(), "the trace keeps the fit");
        }
        (request, reply, wire)
    }

    fn doc(&self) -> Document {
        unpack(77, &self.s.pack("0.1.0").unwrap()).expect("the session output reopens")
    }

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

    fn paths(&self) -> Vec<PathSnapshot> {
        self.leaves()
            .into_iter()
            .filter_map(|o| match o {
                ObjectSnapshot::Path(p) => Some(p),
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

struct Run {
    host: Host,
    lines: Vec<OcrLine>,
    /// The lines the fit decided, handed to the one multicolour trace.
    decided: Vec<OcrLine>,
    wire: Vec<f64>,
    reply: String,
    before: Vec<String>,
    created: Vec<TracedText>,
}

fn run_with(model: &OcrModel, file: &str, settings: &str) -> Run {
    let mut host = Host::new(file);
    let lines = host.read(model);
    let (_, reply, wire) = host.trace_and_fit(settings, &lines);
    let (decided, _) = host.s.trace_lines_to_trace(host.image).expect("kept lines");
    load_fonts(&mut host.s);
    let before = host.fingerprint();
    let outcome = host.create();
    assert!(
        matches!(outcome, TraceCreateOutcome::Created { images: 1, .. }),
        "{outcome:?}"
    );
    load_fonts(&mut host.s);
    let created = host.s.trace_created_texts().to_vec();
    Run {
        host,
        lines,
        decided,
        wire,
        reply,
        before,
        created,
    }
}

fn guessed_of(created: &[TracedText], text: &str) -> Vec<usize> {
    created
        .iter()
        .find(|c| c.string == text)
        .map(|c| c.guessed.iter().map(|g| g.1 - 1).collect())
        .unwrap_or_default()
}

/// The truth line of `text` (exact, else the same length with fewest differences).
fn truth_of<'a>(truths: &'a [Truth], text: &str) -> Option<&'a Truth> {
    truths.iter().find(|t| t.text == text).or_else(|| {
        truths
            .iter()
            .filter(|t| t.text.chars().count() == text.chars().count())
            .min_by_key(|t| {
                t.text
                    .chars()
                    .zip(text.chars())
                    .filter(|(a, b)| a != b)
                    .count()
            })
    })
}

/// Letters, sure, guessed and wrong sure letters of the run against `truths`.
fn letter_counts(r: &Run, truths: &[Truth]) -> (usize, usize, usize, usize) {
    let (mut letters, mut sure, mut guessed, mut wrong) = (0, 0, 0, 0);
    for t in r.host.texts() {
        let g = guessed_of(&r.created, &t.string);
        let truth = truth_of(truths, &t.string);
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
            if truth.is_none_or(|t| t.text.chars().nth(i) != Some(c)) {
                wrong += 1;
                eprintln!("wrong sure letter {c:?} at {} of {:?}", i + 1, t.string);
            }
        }
    }
    (letters, sure, guessed, wrong)
}

fn px_rect_to_mm(map: [f64; 6], r: Rect) -> Rect {
    let a = Affine::new(map);
    Rect::from_points(
        a * kurbo::Point::new(r.x0, r.y0),
        a * kurbo::Point::new(r.x1, r.y1),
    )
}

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

/// The outlines of each layer of a multicolour wire, with the layer's colour.
fn layer_outlines(w: &TraceWire) -> Vec<(TraceRgb, Vec<BezPath>)> {
    let mut at = 0;
    w.layers
        .iter()
        .map(|l| {
            let o = w.outlines[at..at + l.outlines].to_vec();
            at += l.outlines;
            (l.colour, o)
        })
        .collect()
}

/// The candidates of "nearest" per criterion 23 and adrs.md decision 6 (as amended by the lead
/// 2026-11-02): the palette, and the trace's background (here the white paper) only for a text
/// that lies on a traced shape.
fn candidates(w: &TraceWire, on_shape: bool) -> Vec<TraceRgb> {
    let mut c: Vec<TraceRgb> = w.palette.iter().map(|p| p.colour).collect();
    if on_shape {
        c.push(WHITE);
    }
    c
}

/// The colour poster's only line on a traced shape (the blue box).
const ON_SHAPE: &str = "Straße 7";

fn nearest(cands: &[TraceRgb], c: TraceRgb) -> TraceRgb {
    *cands.iter().min_by_key(|p| dist2(**p, c)).unwrap()
}

fn describe(w: &TraceWire) -> String {
    format!(
        "palette {:?}; layers {:?}",
        w.palette
            .iter()
            .map(|p| (p.colour, p.pixels, p.fixed))
            .collect::<Vec<_>>(),
        w.layers
            .iter()
            .map(|l| (l.colour, l.outlines))
            .collect::<Vec<_>>()
    )
}

// ---------------------------------------------------------------------------------------------
// Criterion 23: the fill of each text.
// ---------------------------------------------------------------------------------------------

/// Criterion 23 with the first-use Multicolour settings, Remove background and a fixed count of
/// 4: every text object's fill is the nearest candidate (palette or the trace's background) to
/// the colour its line was rendered in, and the wire's text section carries the same fill.
#[test]
fn s2_c23_each_text_takes_the_nearest_palette_colour_of_its_pixels() {
    let model = model();
    let truths = truths_of("colour_poster.png");
    for settings in [MULTI, MULTI_RB, MULTI_4] {
        let r = run_with(&model, "colour_poster.png", settings);
        let w = traced(&r.wire);
        eprintln!("{settings}: {}", describe(&w));
        let texts = r.host.texts();
        assert!(
            texts.len() >= 5,
            "{settings}: {:?}",
            texts.iter().map(|t| &t.string).collect::<Vec<_>>()
        );
        for t in &texts {
            let truth = truth_of(&truths, &t.string).expect("a poster line");
            let want = nearest(&candidates(&w, t.string == ON_SHAPE), truth.colour);
            let got = rgb(t.style.fill.color);
            eprintln!(
                "{settings}: {:?} rendered {:?}, fill {:?}, nearest candidate {:?}",
                t.string, truth.colour, got, want
            );
            assert!(t.style.fill.enabled, "{:?}", t.string);
            assert_eq!(got, want, "{settings}: fill of {:?}", t.string);
            let wired = w
                .texts
                .lines
                .iter()
                .find(|l| l.text == t.string)
                .and_then(|l| l.fill);
            assert_eq!(
                wired,
                Some(want),
                "{settings}: the wire's fill of {:?}",
                t.string
            );
        }
    }
}

/// Criterion 23, second half: white text on the blue box is a white text above the whole box:
/// the blue layer has one outline over the box area, no letter-shaped hole, the box edges within
/// 2 px, and the text lies above every traced path.
#[test]
fn s2_c23_white_text_on_a_coloured_box_is_white_above_the_whole_box() {
    let model = model();
    for settings in [MULTI, MULTI_RB] {
        let r = run_with(&model, "colour_poster.png", settings);
        let w = traced(&r.wire);
        let texts = r.host.texts();
        let on_box = texts
            .iter()
            .find(|t| t.string == "Straße 7")
            .unwrap_or_else(|| panic!("{settings}: \"Straße 7\" is text"));
        assert_eq!(
            rgb(on_box.style.fill.color),
            WHITE,
            "{settings}: white text"
        );
        let boxed = px_rect_to_mm(r.host.map, BOX_PX);
        let px = (r.host.map[0].powi(2) + r.host.map[1].powi(2)).sqrt();
        let inside: Vec<(TraceRgb, Rect)> = layer_outlines(&w)
            .into_iter()
            .flat_map(|(c, o)| {
                o.iter()
                    .flat_map(subpath_boxes)
                    .map(move |b| (c, b))
                    .collect::<Vec<_>>()
            })
            .filter(|(_, b)| boxed.inflate(0.5, 0.5).contains_rect(*b))
            .collect();
        eprintln!("{settings}: outlines inside the box {inside:?}");
        assert_eq!(
            inside.len(),
            1,
            "{settings}: only the box itself, no letter hole or letter"
        );
        let (c, b) = inside[0];
        assert!(
            dist2(c, TraceRgb::new(30, 60, 200)) < 30 * 30,
            "{settings}: the box is blue: {c:?}"
        );
        for (got, want) in [
            (b.x0, boxed.x0),
            (b.y0, boxed.y0),
            (b.x1, boxed.x1),
            (b.y1, boxed.y1),
        ] {
            assert!(
                (got - want).abs() <= 2.0 * px,
                "{settings}: box edge {got} vs {want}"
            );
        }
        let leaves = r.host.leaves();
        let last_path = leaves
            .iter()
            .rposition(|o| matches!(o, ObjectSnapshot::Path(_)))
            .unwrap();
        let first_text = leaves
            .iter()
            .position(|o| matches!(o, ObjectSnapshot::Text(_)))
            .unwrap();
        assert!(first_text > last_path, "{settings}: texts above the paths");
    }
}

/// Criterion 23 with adrs.md decision 6 (slice 2 note, Proposal a): green is a colour only the
/// text "Übermaß" has. It stays a palette colour, so the text is green. The case also shows what
/// palette the filled picture alone would have given (the alternative: palette after in-painting)
/// and checks that Create paths writes no empty path for a colour left without pixels.
#[test]
fn s2_c23_a_colour_only_the_text_has() {
    let model = model();
    let green = TraceRgb::new(30, 150, 60);
    let r = run_with(&model, "colour_poster.png", MULTI);
    let w = traced(&r.wire);
    eprintln!("with text: {}", describe(&w));
    let in_palette = w.palette.iter().any(|p| dist2(p.colour, green) < 40 * 40);
    let as_layer = w.layers.iter().any(|l| dist2(l.colour, green) < 40 * 40);
    let text = r
        .host
        .texts()
        .into_iter()
        .find(|t| t.string == "Übermaß")
        .expect("\"Übermaß\" is text");
    eprintln!(
        "green in the palette {in_palette}, as a layer {as_layer}, text fill {:?}",
        text.style.fill.color
    );
    assert!(
        in_palette,
        "a colour only the text has stays a palette colour (decision 6 note)"
    );
    assert!(
        dist2(rgb(text.style.fill.color), green) < 40 * 40,
        "the text is green"
    );
    // The alternative of Proposal (a): the palette of the filled picture.
    let pass = text_pass(
        &r.host.rgba,
        r.host.width,
        r.host.height,
        curvyo_document_core::Affine::from_array(r.host.map),
        &trace_settings(MULTI),
        &r.decided,
    );
    let filled = traced(&trace_colour_job_with(
        &pass.rgba,
        r.host.width,
        r.host.height,
        r.host.map,
        &trace_colour_settings(MULTI).unwrap(),
        &mut |_| {},
    ));
    eprintln!("palette of the filled picture alone: {}", describe(&filled));
    let alt = nearest(&candidates(&filled, false), green);
    eprintln!("with that palette the green text would be {alt:?}");
    // No empty path in the document, no layer without outlines.
    for (c, o) in layer_outlines(&w) {
        assert!(!o.is_empty(), "layer {c:?} has outlines");
    }
    for p in r.host.paths() {
        assert!(
            p.subpaths().next().is_some(),
            "an empty path was written: {:?}",
            p.style
        );
    }
}

/// Proposal (b) on a light grey text on white (`grey_text.png`): reported, and held to the spec's
/// rule (nearest candidate).
#[test]
fn s2_c23_light_grey_text_near_the_paper() {
    let model = model();
    let r = run_with(&model, "grey_text.png", MULTI);
    let w = traced(&r.wire);
    eprintln!("grey: {}", describe(&w));
    eprintln!(
        "OCR {:?}",
        r.lines
            .iter()
            .map(|l| (&l.text, l.accepted()))
            .collect::<Vec<_>>()
    );
    for t in r.host.texts() {
        let got = rgb(t.style.fill.color);
        eprintln!(
            "{:?} fill {got:?} (paper white: {})",
            t.string,
            got == WHITE
        );
        // Criterion 23 as amended: text on the paper takes the nearest palette colour, never
        // the paper's. (Before the amendment this case asserted the background as a candidate
        // and saw #FFFFFF.)
        assert_ne!(got, WHITE, "pale text on paper is never white");
        assert_eq!(
            got,
            nearest(&candidates(&w, false), TraceRgb::new(190, 190, 190))
        );
    }
    assert_eq!(r.host.texts().len(), 1, "\"Hellgrau\" is text");
}

// ---------------------------------------------------------------------------------------------
// Criterion 24: slice 1 criteria 3, 6, 11, 12 and 13 in the multicolour trace.
// ---------------------------------------------------------------------------------------------

/// Criterion 24 with 3: no wrong sure letter on the colour poster in all three settings; the
/// strings with umlauts and ß reported.
#[test]
fn s2_c24_c3_no_wrong_sure_letter_in_multicolour() {
    let model = model();
    let truths = truths_of("colour_poster.png");
    for settings in [MULTI, MULTI_RB, MULTI_4] {
        let r = run_with(&model, "colour_poster.png", settings);
        let strings: Vec<String> = r.host.texts().into_iter().map(|t| t.string).collect();
        let (letters, sure, guessed, wrong) = letter_counts(&r, &truths);
        eprintln!(
            "{settings}: texts {strings:?}; letters {letters}, sure {sure}, guessed {guessed}, wrong sure {wrong}"
        );
        assert_eq!(wrong, 0, "{settings}: criterion 3");
        for t in &strings {
            assert!(
                truths.iter().any(|x| &x.text == t),
                "{settings}: {t:?} is a rendered line exactly"
            );
        }
    }
}

/// Criterion 24 with 6: the box and the disc stay whole (one outline each, no letter hole in any
/// layer within a text's ink box), the texts lie above the traced paths.
#[test]
fn s2_c24_c6_shapes_under_text_stay_whole_and_texts_lie_on_top() {
    let model = model();
    let r = run_with(&model, "colour_poster.png", MULTI);
    let w = traced(&r.wire);
    let texts: Vec<String> = r.host.texts().into_iter().map(|t| t.string).collect();
    // No outline of any layer lies inside the ink box of a line that became text.
    let truths = truths_of("colour_poster.png");
    for t in truths.iter().filter(|t| texts.contains(&t.text)) {
        let ink = px_rect_to_mm(r.host.map, t.ink).inflate(0.3, 0.3);
        let inner: Vec<(TraceRgb, Rect)> = layer_outlines(&w)
            .into_iter()
            .flat_map(|(c, o)| {
                o.iter()
                    .flat_map(subpath_boxes)
                    .map(move |b| (c, b))
                    .collect::<Vec<_>>()
            })
            .filter(|(_, b)| ink.contains_rect(*b))
            .collect();
        assert!(
            inner.is_empty(),
            "{:?}: letter outlines left in the trace: {inner:?}",
            t.text
        );
    }
    let disc = px_rect_to_mm(r.host.map, DISC_PX);
    let in_disc: Vec<Rect> = w
        .outlines
        .iter()
        .flat_map(subpath_boxes)
        .filter(|b| disc.inflate(0.5, 0.5).contains_rect(*b))
        .collect();
    assert_eq!(in_disc.len(), 1, "the disc is one outline: {in_disc:?}");
    let leaves = r.host.leaves();
    let last_path = leaves
        .iter()
        .rposition(|o| matches!(o, ObjectSnapshot::Path(_)))
        .unwrap();
    let first_text = leaves
        .iter()
        .position(|o| matches!(o, ObjectSnapshot::Text(_)))
        .unwrap();
    assert!(
        first_text > last_path,
        "texts above the paths ({first_text} vs {last_path})"
    );
}

/// Criterion 24 with 11: one step, one undo leaves the document as before, redo brings it back.
#[test]
fn s2_c24_c11_one_undo_removes_the_colour_group_and_all_texts() {
    let model = model();
    let mut r = run_with(&model, "colour_poster.png", MULTI);
    let after = r.host.fingerprint();
    assert!(r.host.texts().len() >= 5);
    let layers = traced(&r.wire).layers.len();
    assert!(layers >= 2, "a multicolour result");
    assert_eq!(r.host.paths().len(), layers, "one path per colour layer");
    r.host.s.undo();
    assert_eq!(r.host.fingerprint(), r.before, "the document as before");
    r.host.s.redo();
    assert_eq!(r.host.fingerprint(), after, "redo brings the same result");
}

/// Criterion 24 with 12: without lines, and with only unaccepted lines, the multicolour job with
/// text is the plain multicolour trace number for number.
#[test]
fn s2_c24_c12_no_lines_or_unaccepted_lines_trace_as_the_plain_multicolour() {
    let host = Host::new("colour_poster.png");
    for settings in [MULTI, MULTI_RB, MULTI_4] {
        assert_eq!(
            host.wire(settings, &[]),
            host.plain_wire(settings),
            "{settings}: no lines"
        );
        let weak = vec![
            OcrLine {
                quad: [
                    [1490.0, 1140.0],
                    [1890.0, 1140.0],
                    [1890.0, 1540.0],
                    [1490.0, 1540.0],
                ],
                text: "O".to_owned(),
                confidence: vec![0.2],
                positions: Vec::new(),
            },
            OcrLine {
                quad: [
                    [100.0, 120.0],
                    [700.0, 120.0],
                    [700.0, 300.0],
                    [100.0, 300.0],
                ],
                text: "- .".to_owned(),
                confidence: vec![0.99, 0.99, 0.99],
                positions: Vec::new(),
            },
        ];
        assert!(weak.iter().all(|l| !l.accepted()));
        let on = traced(&host.wire(settings, &weak));
        let off = traced(&host.plain_wire(settings));
        assert_eq!(
            on.outlines, off.outlines,
            "{settings}: unaccepted lines trace as usual"
        );
        assert_eq!(on.palette, off.palette, "{settings}: the palette");
        assert_eq!(on.layers, off.layers, "{settings}: the layers");
        assert_eq!((on.paths, on.nodes), (off.paths, off.nodes), "{settings}");
        assert_eq!(on.texts.lines.len(), 0);
    }
}

/// Criterion 24 with 13: two runs give the same texts (content, face, place, size, fill, guessed
/// marks) and the same wire.
#[test]
fn s2_c24_c13_two_multicolour_runs_are_identical() {
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
                        rgb(t.style.fill.color),
                        guessed_of(&r.created, &t.string),
                    )
                })
                .collect::<Vec<_>>(),
            r.wire.clone(),
            r.host
                .paths()
                .iter()
                .map(|p| format!("{p:?}"))
                .collect::<Vec<_>>(),
        )
    };
    let a = run_with(&model, "colour_poster.png", MULTI);
    let b = run_with(&model, "colour_poster.png", MULTI);
    assert_eq!(a.lines, b.lines);
    assert_eq!(a.reply, b.reply);
    assert!(summary(&a) == summary(&b), "two runs differ");
}

fn numbers_of(v: &Value) -> Vec<f64> {
    match v {
        Value::Number(n) => vec![n.as_f64().unwrap()],
        Value::Array(list) => list.iter().flat_map(numbers_of).collect(),
        _ => Vec::new(),
    }
}

fn assert_ocr_alike(kept: &str, now: &str) {
    let (a, b): (Value, Value) = (
        serde_json::from_str(kept).unwrap(),
        serde_json::from_str(now).unwrap(),
    );
    let (la, lb) = (
        a["lines"].as_array().unwrap(),
        b["lines"].as_array().unwrap(),
    );
    assert_eq!(la.len(), lb.len(), "the line count");
    for (x, y) in la.iter().zip(lb) {
        assert_eq!(x["text"], y["text"], "the strings");
        for key in ["quad", "confidence"] {
            let (p, q) = (numbers_of(&x[key]), numbers_of(&y[key]));
            assert_eq!(p.len(), q.len());
            for (u, v) in p.iter().zip(&q) {
                assert!((u - v).abs() <= 1e-4, "{key} of {}: {u} vs {v}", x["text"]);
            }
        }
    }
}

/// Criterion 24 with 13 across builds: the OCR lines, the regions, the fit and the multicolour
/// wire of the colour poster natively, for the browser case to compare with. The trace and fit
/// start from the recorded OCR lines and must match exactly.
#[test]
fn s2_native_record_of_the_colour_poster_matches_the_golden_file() {
    let model = model();
    let mut host = Host::new("colour_poster.png");
    let read = host.read(&model);
    let ocr = ocr_lines_json(host.width, host.height, &read);
    let bless = std::env::var_os("CURVYO_BLESS").is_some();
    let kept: Option<Value> = (!bless)
        .then(|| serde_json::from_str(&std::fs::read_to_string(golden()).unwrap()).unwrap());
    let lines = kept.as_ref().map_or_else(
        || read.clone(),
        |k| read_ocr_lines(k["ocr"].as_str().unwrap(), host.width),
    );
    let (request, reply, wire) = host.trace_and_fit(MULTI, &lines);
    let (decided, _) = host.s.trace_lines_to_trace(host.image).unwrap();
    let fnv = host.rgba.iter().fold(0x811c_9dc5_u32, |h, b| {
        (h ^ u32::from(*b)).wrapping_mul(0x0100_0193)
    });
    let record = serde_json::json!({
        "width": host.width,
        "height": host.height,
        "rgba_fnv32": fnv,
        "settings": MULTI,
        "map": host.map,
        "ocr": ocr,
        "fit_request": request,
        "fit_reply": reply,
        "decided": ocr_lines_json(host.width, host.height, &decided),
        "wire": wire,
    });
    let Some(kept) = kept else {
        std::fs::write(golden(), serde_json::to_string(&record).unwrap()).unwrap();
        return;
    };
    assert_ocr_alike(kept["ocr"].as_str().unwrap(), &ocr);
    for key in ["fit_request", "fit_reply", "decided", "wire"] {
        assert_eq!(kept[key], record[key], "{key}");
    }
}

fn fill_of(r: &Run, s: &str) -> Option<TraceRgb> {
    r.host
        .texts()
        .into_iter()
        .find(|t| t.string == s)
        .map(|t| rgb(t.style.fill.color))
}

/// Decision 6 note: the maker's Merge into applies to texts as to paths (green merged into blue
/// sets "Übermaß" in blue).
#[test]
fn s2_c23_merge_into_applies_to_texts() {
    let model = model();
    let merged =
        r##"{"output":1,"colours":4,"method":0,"threshold":128,"merged":[["#1e963c","#1e3cc8"]]}"##;
    let r = run_with(&model, "colour_poster.png", merged);
    assert_eq!(
        fill_of(&r, "Übermaß"),
        Some(TraceRgb::new(30, 60, 200)),
        "green merged into blue"
    );
}

/// Criterion 23 as amended (lead 2026-11-02): a text whose fill colour the maker excluded is left
/// out, as the shapes of that colour are. Black excluded on the colour poster drops the two black
/// lines; on `grey_text.png` (grey text filled black, a black box) it drops the box and the text.
#[test]
fn s2_c23_an_excluded_colour_leaves_out_its_texts_and_shapes() {
    let model = model();
    let excluded =
        r##"{"output":1,"colours":4,"method":0,"threshold":128,"excluded":["#000000"]}"##;
    let r = run_with(&model, "colour_poster.png", excluded);
    let texts: Vec<String> = r.host.texts().into_iter().map(|t| t.string).collect();
    eprintln!(
        "black excluded: {}; texts {texts:?}",
        describe(&traced(&r.wire))
    );
    for gone in ["Äpfel & Öl", "Brücke über Täler"] {
        assert!(!texts.iter().any(|t| t == gone), "{gone:?} is left out");
    }
    for kept in ["Größe", "Übermaß", "Straße 7", "Süße Grüße"] {
        assert!(texts.iter().any(|t| t == kept), "{kept:?} stays");
    }
    for p in r.host.paths() {
        assert_ne!(
            p.style.fill.color,
            curvyo_document_core::Color::BLACK,
            "no black path"
        );
    }
    let mut host = Host::new("grey_text.png");
    let lines = host.read(&model);
    let (_, _, wire) = host.trace_and_fit(excluded, &lines);
    let w = traced(&wire);
    load_fonts(&mut host.s);
    let out = host.s.create_traced_paths();
    eprintln!("grey_text, black excluded: {}; {out:?}", describe(&w));
    assert_eq!(
        host.texts().len(),
        0,
        "the grey text filled black is left out"
    );
    assert!(
        host.paths()
            .iter()
            .all(|p| p.style.fill.color != curvyo_document_core::Color::BLACK),
        "the black box is left out"
    );
}

/// Criterion 23 with Pick colours red and blue: the palette is the picked colours; the black
/// texts on the paper take the nearest of them (red), the white text on the blue box stays white.
#[test]
fn s2_c23_pick_colours_follows_the_rule() {
    let model = model();
    let pick = r##"{"output":1,"method":2,"background":"#ffffff","swatches":["#c81e1e","#1e3cc8"],"threshold":128}"##;
    let r = run_with(&model, "colour_poster.png", pick);
    let w = traced(&r.wire);
    eprintln!(
        "pick: {}; fills {:?}",
        describe(&w),
        r.host
            .texts()
            .into_iter()
            .map(|t| (t.string, rgb(t.style.fill.color)))
            .collect::<Vec<_>>()
    );
    let truths = truths_of("colour_poster.png");
    for t in r.host.texts() {
        let truth = truth_of(&truths, &t.string).unwrap();
        let want = nearest(&candidates(&w, t.string == ON_SHAPE), truth.colour);
        assert_eq!(rgb(t.style.fill.color), want, "{:?}", t.string);
    }
    assert_eq!(fill_of(&r, ON_SHAPE), Some(WHITE));
    assert_eq!(fill_of(&r, "Äpfel & Öl"), Some(TraceRgb::new(200, 30, 30)));
}
