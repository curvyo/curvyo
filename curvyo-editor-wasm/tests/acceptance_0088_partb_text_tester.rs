//! Tester case for `specs/0088-trace-recognition` Part B criterion 12 (interplay with text): a picture
//! with a line of text and a circle, Recognise text and Recognise shapes both on, gives one text
//! object and one circle primitive, and no letter (not even the "o" of "Größe") becomes a circle.
//! Runs the host's flow of the 0091 tester (OCR, fit, one trace) and adds the shape section as the
//! worker does after the text. Needs the shipped OCR weights and the included fonts.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

use std::path::PathBuf;

use curvyo_document_core::{FontKind, ObjectSnapshot, Shape, unpack};
use curvyo_editor_wasm::{
    FitWorker, Session, Tool, TraceCreateOutcome, trace_recognise_shapes, trace_settings,
};
use curvyo_ui_core::FontBlock;
use serde_json::Value;
use vectrace::job::{
    Job as TraceJob, Mode as TraceMode, run as run_trace_job, text_regions as trace_text_regions,
};
use vectrace::ocr::{OcrModel, read_text};
use vectrace::{PixelMap as TracePixelMap, Raster as TraceRaster};

const TOLERANCE_MM: f64 = 0.3;
const TEXT_ON: &str = r#"{"threshold":128}"#;
const BOTH_ON: &str = r#"{"threshold":128,"shapes":true}"#;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn fonts_dir() -> PathBuf {
    root().join("frontend/public/fonts")
}

fn poster(file: &str) -> PathBuf {
    root()
        .join("tests/assets/vectrace-fixtures/tester")
        .join(file)
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

/// Hands the session every font file it asks for.
fn load_fonts(s: &mut Session) {
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes), "{file} loads");
    }
}

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

/// `clean.png` ("Größe", 1200 x 500 px) with a black disc of 100 px radius added at (930, 250).
fn text_and_circle() -> Vec<u8> {
    use image::ImageEncoder;
    let mut img = image::open(poster("clean.png")).unwrap().to_rgb8();
    for y in 0..img.height() {
        for x in 0..img.width() {
            let (dx, dy) = (f64::from(x) + 0.5 - 930.0, f64::from(y) + 0.5 - 250.0);
            if dx.hypot(dy) <= 100.0 {
                img.put_pixel(x, y, image::Rgb([0, 0, 0]));
            }
        }
    }
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    out
}

struct Out {
    texts: usize,
    ellipses: Vec<(f64, f64)>,
    other_prims: usize,
    paths: usize,
    undo_ok: bool,
}

fn run(model: &OcrModel, settings: &str) -> Out {
    let mut s = Session::new(88);
    s.resize_viewport(1600.0, 1000.0);
    s.set_font_list_with_kinds(FontBlock::Included, included(), Vec::new());
    load_fonts(&mut s);
    s.import_begin(None);
    s.import_add(&text_and_circle(), "text_circle.png");
    s.import_finish(false);
    s.set_tool(Tool::Trace);
    let image = s.trace_view().shown.expect("the image is shown");
    let src = s.trace_source(image, false).unwrap();
    let lines = read_text(model, &src.rgba, src.width, src.height, &mut |_| {
        std::ops::ControlFlow::Continue(())
    })
    .unwrap();
    assert!(s.set_trace_ocr_lines(image, lines.clone(), (src.width, src.height)));
    let params = trace_settings(settings);
    let regions = trace_text_regions(&TraceJob {
        raster: TraceRaster {
            width: src.width,
            height: src.height,
            rgba: &src.rgba,
        },
        map: TracePixelMap::new(src.map),
        mode: TraceMode::Mono(&params),
        lines: &lines,
        mask: None,
        shapes: false,
    });
    if let Some(request) = s.trace_fit_request_for(image, &regions, TOLERANCE_MM, &[]) {
        let reply = fit(&request);
        assert!(s.set_trace_fit(&request, &reply));
    }
    let (decided, _) = s.trace_lines_to_trace(image).expect("kept lines");
    let wire = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: src.width,
                height: src.height,
                rgba: &src.rgba,
            },
            map: TracePixelMap::new(src.map),
            mode: TraceMode::Mono(&params),
            lines: &decided,
            mask: None,
            shapes: trace_recognise_shapes(settings),
        },
        &mut |_| {},
    );
    assert!(s.set_trace_preview(image, &wire));
    assert!(s.set_trace_result(image, &wire));
    load_fonts(&mut s);
    let before = unpack(1, &s.pack("0.1.0").unwrap()).unwrap();
    let steps = s.undo_len();
    let out = s.create_traced_paths();
    assert!(
        matches!(out, TraceCreateOutcome::Created { images: 1, .. }),
        "{out:?}"
    );
    assert_eq!(s.undo_len(), steps + 1);
    let d = unpack(77, &s.pack("0.1.0").unwrap()).unwrap();
    let leaves: Vec<ObjectSnapshot> = d
        .object_ids()
        .into_iter()
        .filter_map(|id| d.object(id))
        .collect();
    let mut o = Out {
        texts: 0,
        ellipses: Vec::new(),
        other_prims: 0,
        paths: 0,
        undo_ok: false,
    };
    for l in &leaves {
        match l {
            ObjectSnapshot::Text(_) => o.texts += 1,
            ObjectSnapshot::Path(_) => o.paths += 1,
            ObjectSnapshot::Primitive(p) => match p.shape {
                Shape::Ellipse { frame, .. } => {
                    o.ellipses.push((frame.rx.as_mm(), frame.ry.as_mm()));
                }
                _ => o.other_prims += 1,
            },
            _ => {}
        }
    }
    s.undo();
    o.undo_ok = unpack(1, &s.pack("0.1.0").unwrap())
        .unwrap()
        .object_ids()
        .len()
        == before.object_ids().len();
    o
}

/// Criterion 12: one text object and one circle primitive; no letter became a circle (or any shape).
#[test]
fn c12_text_and_circle_with_both_switches_on() {
    let model = model();
    let on = run(&model, BOTH_ON);
    eprintln!(
        "both on: {} texts, {:?} ellipses, {} other primitives, {} paths",
        on.texts, on.ellipses, on.other_prims, on.paths
    );
    assert_eq!(on.texts, 1, "one text object");
    assert_eq!(
        on.ellipses.len(),
        1,
        "exactly one ellipse: the circle, no letter"
    );
    assert_eq!(on.other_prims, 0, "no letter became another shape");
    assert_eq!(on.ellipses[0].0, on.ellipses[0].1, "a circle");
    assert!(on.undo_ok, "one undo restores");
}

/// Control: with Recognise text on and shapes off, the same picture gives the text and the circle as a path.
#[test]
fn c12_control_shapes_off_keeps_the_circle_a_path() {
    let model = model();
    let off = run(&model, TEXT_ON);
    assert_eq!(off.texts, 1);
    assert!(off.ellipses.is_empty() && off.other_prims == 0);
    assert!(off.paths >= 1, "the circle stays a traced path");
}
