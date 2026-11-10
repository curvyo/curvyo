//! Tester cases for `specs/0091-trace-text-recognition` slice 3 (criteria 30 to 35): upside-down
//! and steep text through Trace, on the tester's own fixtures
//! (`curvyo-ocr-core/tests/fixtures/tester/make_turned.py`): the slice 1 poster and the slice 2
//! colour poster turned by 180, a picture with a line at +90 and one at -90, a mixed picture and a
//! line at 195. Written from the specification before the implementation was read; only public
//! names were looked up.
//!
//! Angles follow the document's y-down matrix `atan2(b, a)`: a line Pillow turns counter-clockwise
//! by d degrees (it reads bottom to top for d = 90) has the matrix angle -d, as in the slice 1
//! rotated cases (criterion 8: `rotated_30.png` is -30).

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

use curvyo_document_core::{Document, FontKind, NodeId, ObjectSnapshot, TextSnapshot, unpack};
use curvyo_editor_wasm::{
    FitWorker, Session, Tool, TracedText, trace_colour_settings, trace_settings,
};
use curvyo_ui_core::{
    FontBlock, OcrLine, OcrModel, TraceJob, TraceMode, TracePixelMap, TraceRaster, run_trace_job,
    trace_read_text, trace_text_regions,
};
use serde_json::Value;

const TOLERANCE_MM: f64 = 0.3;
const BW: &str = r#"{"threshold":128}"#;
const MULTI: &str = r#"{"output":1,"colours":null,"fixed":[],"excluded":[],"method":0,"background":null,"swatches":[],"threshold":128}"#;

const POSTER: [&str; 8] = [
    "Größe",
    "Übermaß",
    "Äpfel & Öl",
    "Tel. 0421 98765",
    "Fräsen ab 3 Stück, 12.50 EUR",
    "Straße 7",
    "Mono 314 Zoll",
    "Brücke über Täler",
];
const COLOUR_POSTER: [&str; 6] = [
    "Größe",
    "Übermaß",
    "Äpfel & Öl",
    "Straße 7",
    "Süße Grüße",
    "Brücke über Täler",
];

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

    fn wire(&self, settings: &str, lines: &[OcrLine]) -> Vec<f64> {
        if settings == MULTI {
            let colour = trace_colour_settings(settings).expect("multicolour settings");
            run_trace_job(
                &TraceJob {
                    raster: TraceRaster {
                        width: self.width,
                        height: self.height,
                        rgba: &self.rgba,
                    },
                    map: TracePixelMap::new(self.map),
                    mode: TraceMode::Colours(&colour),
                    lines,
                    mask: None,
                    shapes: false,
                },
                &mut |_| {},
            )
        } else {
            run_trace_job(
                &TraceJob {
                    raster: TraceRaster {
                        width: self.width,
                        height: self.height,
                        rgba: &self.rgba,
                    },
                    map: TracePixelMap::new(self.map),
                    mode: TraceMode::Mono(&trace_settings(settings)),
                    lines,
                    mask: None,
                    shapes: false,
                },
                &mut |_| {},
            )
        }
    }

    /// The host's flow (`useTrace`): cache the lines, the text pass alone, the fit, one trace.
    fn trace_and_fit(&mut self, settings: &str, lines: &[OcrLine]) -> Vec<f64> {
        assert!(
            self.s
                .set_trace_ocr_lines(self.image, lines.to_vec(), (self.width, self.height))
        );
        let regions = trace_text_regions(&TraceJob {
            raster: TraceRaster {
                width: self.width,
                height: self.height,
                rgba: &self.rgba,
            },
            map: TracePixelMap::new(self.map),
            mode: TraceMode::Mono(&trace_settings(settings)),
            lines,
            mask: None,
            shapes: false,
        });
        let fitted = self
            .s
            .trace_fit_request_for(self.image, &regions, TOLERANCE_MM, &[])
            .map(|request| {
                let reply = fit(&request);
                assert!(self.s.set_trace_fit(&request, &reply));
            })
            .is_some();
        let (decided, _) = self.s.trace_lines_to_trace(self.image).expect("kept lines");
        let wire = self.wire(settings, &decided);
        assert!(self.s.set_trace_preview(self.image, &wire));
        assert!(self.s.set_trace_result(self.image, &wire));
        if fitted {
            assert!(self.s.trace_text_fitted(), "the trace keeps the fit");
        }
        wire
    }

    fn doc(&self) -> Document {
        unpack(77, &self.s.pack("0.1.0").unwrap()).expect("the session output reopens")
    }

    fn texts(&self) -> Vec<TextSnapshot> {
        let d = self.doc();
        d.object_ids()
            .into_iter()
            .filter_map(|id| d.object(id))
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
}

struct Run {
    host: Host,
    lines: Vec<OcrLine>,
    created: Vec<TracedText>,
}

fn run_with(model: &OcrModel, file: &str, settings: &str) -> Run {
    let mut host = Host::new(file);
    let lines = host.read(model);
    host.trace_and_fit(settings, &lines);
    load_fonts(&mut host.s);
    let steps = host.s.undo_len();
    let outcome = host.s.create_traced_paths();
    assert_eq!(
        host.s.undo_len(),
        steps + 1,
        "criterion 11: one history step"
    );
    eprintln!("{file}: {outcome:?}");
    load_fonts(&mut host.s);
    let created = host.s.trace_created_texts().to_vec();
    Run {
        host,
        lines,
        created,
    }
}

fn run(model: &OcrModel, file: &str) -> Run {
    run_with(model, file, BW)
}

fn guessed_of(created: &[TracedText], text: &str) -> Vec<usize> {
    created
        .iter()
        .find(|c| c.string == text)
        .map(|c| c.guessed.iter().map(|g| g.1 - 1).collect())
        .unwrap_or_default()
}

/// Edit distance, for "exact or within 1 letter".
fn distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != cb))
                .min(row[j] + 1)
                .min(row[j + 1] + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

/// The truth line nearest to `text`, if within 1 letter.
fn truth_near<'a>(truth: &[&'a str], text: &str) -> Option<&'a str> {
    truth
        .iter()
        .copied()
        .min_by_key(|t| distance(t, text))
        .filter(|t| distance(t, text) <= 1)
}

/// The matrix angle of a text in degrees, `atan2(b, a)`.
fn angle_of(t: &TextSnapshot) -> f64 {
    let m = t.placement.to_array();
    m[1].atan2(m[0]).to_degrees()
}

/// The difference of two angles in degrees, in 0..=180.
fn angle_gap(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// Sure letters that are wrong against the truth (a sure letter of a line with no truth within
/// 1 letter counts as wrong; a line within 1 letter is compared position by position when the
/// lengths agree).
fn wrong_sure(r: &Run, truth: &[&str]) -> usize {
    let mut wrong = 0;
    for t in r.host.texts() {
        let g = guessed_of(&r.created, &t.string);
        let want = truth_near(truth, &t.string);
        for (i, c) in t.string.chars().enumerate() {
            if c.is_whitespace() || g.contains(&i) {
                continue;
            }
            let ok = want.is_some_and(|w| {
                w.chars().count() != t.string.chars().count() || w.chars().nth(i) == Some(c)
            });
            if !ok {
                wrong += 1;
                eprintln!("wrong sure letter {c:?} at {} of {:?}", i + 1, t.string);
            }
        }
    }
    wrong
}

fn sure_letters(r: &Run) -> usize {
    r.host
        .texts()
        .iter()
        .map(|t| {
            let g = guessed_of(&r.created, &t.string);
            t.string
                .chars()
                .enumerate()
                .filter(|(i, c)| !c.is_whitespace() && !g.contains(i))
                .count()
        })
        .sum()
}

fn describe(r: &Run) -> Vec<(String, String, f64)> {
    r.host
        .texts()
        .iter()
        .map(|t| (t.string.clone(), t.font.family.clone(), angle_of(t)))
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Criterion 30: upside-down.
// ---------------------------------------------------------------------------------------------

/// Criterion 30: the 180 degree poster gives the upright poster's strings (exact or within 1
/// letter), faces and sure letters, 0 wrong sure letters, rotation 180 within 1 degree.
#[test]
fn c30_the_poster_upside_down_reads_like_the_upright_one() {
    let model = model();
    let up = run(&model, "poster.png");
    let down = run(&model, "poster_180.png");
    eprintln!("up   {:?}", describe(&up));
    eprintln!("down {:?}", describe(&down));
    let (ut, dt) = (up.host.texts(), down.host.texts());
    assert_eq!(dt.len(), ut.len(), "the same number of text objects");
    for u in &ut {
        let d = dt
            .iter()
            .find(|d| distance(&d.string, &u.string) <= 1)
            .unwrap_or_else(|| panic!("no upside-down text for {:?}", u.string));
        assert_eq!(d.font.family, u.font.family, "same face for {:?}", u.string);
        assert_eq!(d.font.style, u.font.style, "same style for {:?}", u.string);
    }
    for d in &dt {
        let gap = angle_gap(angle_of(d), 180.0);
        assert!(gap <= 1.0, "{:?} turned {} not 180", d.string, angle_of(d));
    }
    for u in &ut {
        assert!(angle_gap(angle_of(u), 0.0) <= 1.0, "upright stays upright");
    }
    assert_eq!(wrong_sure(&down, &POSTER), 0, "0 wrong sure letters");
    let (su, sd) = (sure_letters(&up), sure_letters(&down));
    eprintln!("sure letters up {su}, down {sd}");
    assert!(
        sd + 2 >= su,
        "about the same sure letters: {su} up, {sd} down"
    );
}

/// Criterion 30 with a line at 180 plus 15: the rotation is 180 plus the line angle.
#[test]
fn c30_a_line_at_195_degrees_is_turned_by_the_sum() {
    let model = model();
    let r = run(&model, "turned_195.png");
    let texts = r.host.texts();
    assert_eq!(texts.len(), 1, "{:?}", describe(&r));
    assert!(distance(&texts[0].string, "Winkel 195 Grad") <= 1);
    let got = angle_of(&texts[0]);
    eprintln!("turned_195: {got:.2}");
    assert!(angle_gap(got, -195.0) <= 1.0, "{got} vs -195 (= 165)");
    assert_eq!(wrong_sure(&r, &["Winkel 195 Grad"]), 0);
}

// ---------------------------------------------------------------------------------------------
// Criterion 31: steep text.
// ---------------------------------------------------------------------------------------------

/// Criterion 31: a line reading bottom to top and one reading top to bottom, both read.
#[test]
fn c31_steep_lines_read_at_plus_and_minus_ninety() {
    let model = model();
    let r = run(&model, "steep.png");
    eprintln!("steep: {:?}", describe(&r));
    let texts = r.host.texts();
    assert_eq!(texts.len(), 2, "{:?}", describe(&r));
    for (want, up_to_down) in [("Hochkant hinauf 90", false), ("Hochkant hinab 90", true)] {
        let t = texts
            .iter()
            .find(|t| distance(&t.string, want) <= 1)
            .unwrap_or_else(|| panic!("{want:?} not read: {:?}", describe(&r)));
        // Bottom to top is Pillow +90, matrix -90; top to bottom is matrix +90.
        let expect = if up_to_down { 90.0 } else { -90.0 };
        assert!(
            angle_gap(angle_of(t), expect) <= 1.0,
            "{want:?} turned {} not {expect}",
            angle_of(t)
        );
    }
    assert_eq!(
        wrong_sure(&r, &["Hochkant hinauf 90", "Hochkant hinab 90"]),
        0
    );
}

/// The existing 90 degree fixture of slice 1 now reads, and still has no wrong sure letter.
#[test]
fn c31_the_slice_one_ninety_degree_fixture() {
    let model = model();
    let r = run(&model, "rotated_90.png");
    eprintln!("rotated_90: {:?}", describe(&r));
    let texts = r.host.texts();
    assert_eq!(texts.len(), 1);
    assert!(distance(&texts[0].string, "Neunzig 90") <= 1);
    assert!(angle_gap(angle_of(&texts[0]), -90.0) <= 1.0);
    assert_eq!(wrong_sure(&r, &["Neunzig 90"]), 0);
}

// ---------------------------------------------------------------------------------------------
// Criterion 33: mixed.
// ---------------------------------------------------------------------------------------------

/// Criterion 33: an upright and an upside-down line in one picture, each with its own rotation.
#[test]
fn c33_mixed_upright_and_upside_down() {
    let model = model();
    let r = run(&model, "mixed.png");
    eprintln!("mixed: {:?}", describe(&r));
    let texts = r.host.texts();
    assert_eq!(texts.len(), 2, "{:?}", describe(&r));
    let up = texts
        .iter()
        .find(|t| distance(&t.string, "Oben aufrecht 11") <= 1)
        .expect("the upright line");
    let down = texts
        .iter()
        .find(|t| distance(&t.string, "Unten kopfüber 22") <= 1)
        .expect("the upside-down line");
    assert!(angle_gap(angle_of(up), 0.0) <= 1.0, "{}", angle_of(up));
    assert!(
        angle_gap(angle_of(down), 180.0) <= 1.0,
        "{}",
        angle_of(down)
    );
    assert_eq!(
        wrong_sure(&r, &["Oben aufrecht 11", "Unten kopfüber 22"]),
        0
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 35: slices 1 and 2 hold; Multicolour with the turned poster.
// ---------------------------------------------------------------------------------------------

/// Criterion 35 (Multicolour): the turned colour poster has the upright one's strings and fills,
/// 180 degrees, 0 wrong sure letters.
#[test]
fn c35_multicolour_poster_upside_down() {
    let model = model();
    let up = run_with(&model, "colour_poster.png", MULTI);
    let down = run_with(&model, "colour_poster_180.png", MULTI);
    eprintln!("up   {:?}", describe(&up));
    eprintln!("down {:?}", describe(&down));
    let (ut, dt) = (up.host.texts(), down.host.texts());
    assert_eq!(dt.len(), ut.len());
    for u in &ut {
        let d = dt
            .iter()
            .find(|d| distance(&d.string, &u.string) <= 1)
            .unwrap_or_else(|| panic!("no turned text for {:?}", u.string));
        let (fu, fd) = (u.style.fill.color, d.style.fill.color);
        let gap = (i32::from(fu.r) - i32::from(fd.r)).abs()
            + (i32::from(fu.g) - i32::from(fd.g)).abs()
            + (i32::from(fu.b) - i32::from(fd.b)).abs();
        assert!(gap <= 6, "fill of {:?}: {fu:?} up, {fd:?} down", u.string);
        assert!(angle_gap(angle_of(d), 180.0) <= 1.0);
    }
    assert_eq!(wrong_sure(&down, &COLOUR_POSTER), 0);
}

/// Criterion 35, determinism (13): two runs on the turned poster are identical.
#[test]
fn c35_two_runs_on_the_turned_poster_are_identical() {
    let model = model();
    for (file, settings) in [("poster_180.png", BW), ("colour_poster_180.png", MULTI)] {
        let a = run_with(&model, file, settings);
        let b = run_with(&model, file, settings);
        assert_eq!(a.host.fingerprint(), b.host.fingerprint(), "{file}");
    }
}

/// Criterion 35: a picture without text gets no text object.
#[test]
fn c35_a_picture_without_text_gets_no_text_object() {
    let model = model();
    let file = "plain.png";
    let r = run(&model, file);
    assert!(r.host.texts().is_empty(), "{file}: {:?}", describe(&r));
    assert!(r.lines.iter().all(|l| !l.accepted()), "{file}");
}

// ---------------------------------------------------------------------------------------------
// Criterion 34: cost (reported, not asserted).
// ---------------------------------------------------------------------------------------------

/// Criterion 34: thread CPU time of the read on the upright poster, the 180 poster and the
/// steep picture. Only reported; the PR carries the numbers.
#[test]
fn c34_cost_of_the_read_is_reported() {
    let model = model();
    for file in ["poster.png", "poster_180.png", "steep.png", "mixed.png"] {
        let host = Host::new(file);
        let (lines, t) = cpu_time::cpu_time_of(|| host.read(&model));
        eprintln!(
            "cost {file}: read {:.2} s thread CPU, {} lines",
            t.as_secs_f64(),
            lines.len()
        );
    }
}

/// White box: three tight upside-down lines with marks. Each is read, none swallows the shapes of
/// its neighbour (the lines grow above their text; the text pass must not take the next line's
/// letters), and no sure letter is wrong.
#[test]
fn c30_tight_upside_down_lines_keep_their_neighbours() {
    let model = model();
    let truth = ["Äpfel Öl Ärger", "Ölfässer Übung", "Ärmel Öfen Äste"];
    let r = run(&model, "tight_180.png");
    eprintln!("tight: {:?}", describe(&r));
    let texts = r.host.texts();
    for t in truth {
        let found = texts.iter().find(|x| distance(&x.string, t) <= 1);
        eprintln!("{t:?}: {:?}", found.map(|f| &f.string));
    }
    assert_eq!(wrong_sure(&r, &truth), 0);
    assert_eq!(texts.len(), 3, "{:?}", describe(&r));
    for t in &texts {
        assert!(angle_gap(angle_of(t), 180.0) <= 1.0);
    }
}
