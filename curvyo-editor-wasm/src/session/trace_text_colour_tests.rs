//! Text recognition in the multicolour Trace end to end (`specs/0091-trace-text-recognition`
//! slice 2, criteria 23 and 24): the multicolour posters read with the shipped weights, the lines
//! decided by the text pass alone and the fit, traced once in colours, Create paths and one undo.
//! The small poster runs in every build; the full posters are the `#[ignore]` harness that prints
//! the numbers of the PR:
//!
//! ```text
//! cargo test --release -p curvyo-editor-wasm --lib trace_text_colour_tests -- --ignored --nocapture
//! ```

use curvyo_document_core::{Color, NodeId, ObjectSnapshot};
use curvyo_ui_core::{
    OcrModel, TRACE_DEFAULT_BACKGROUND_TOLERANCE, TraceColourParams, TraceMethod, TraceParams,
    TraceRgb, trace_read_text, trace_text_colour_job_in, trace_text_regions_in,
};

use super::trace_text_tests::{
    POSTER, cpu_seconds, fit, levenshtein, model, session_with, wrong_sure,
};
use super::{Session, TraceCreateOutcome};

/// Multicolour with Remove background on white paper, the rest at its first-use values.
pub(super) fn colour_params() -> TraceColourParams {
    TraceColourParams::new(TraceParams {
        selection: TraceMethod::RemoveBackground {
            background: Some(TraceRgb::new(255, 255, 255)),
            tolerance: TRACE_DEFAULT_BACKGROUND_TOLERANCE,
        },
        ..TraceParams::with_threshold(128)
    })
}

/// One written text: string, fill, guessed positions, size in mm, placement.
pub(super) type WrittenColourText = (String, Color, Vec<usize>, f64, [f64; 6]);

/// What one run wrote: the texts (string, fill, guessed positions, size, placement) and the
/// colour paths (fill, outline count), in the document's order, and where the texts lie.
#[derive(Debug, PartialEq)]
pub(super) struct ColourRun {
    pub(super) texts: Vec<WrittenColourText>,
    pub(super) paths: Vec<(Color, usize)>,
    /// Every text lies above every traced path.
    pub(super) texts_on_top: bool,
}

/// Seconds of thread CPU per stage of one run.
#[derive(Debug, Default)]
pub(super) struct Times {
    ocr: f64,
    regions: f64,
    fit: f64,
    trace: f64,
    /// The same multicolour trace with Recognise text off.
    plain: f64,
}

/// OCR, the text pass alone, the fit, the one multicolour trace (criterion 25), Create paths,
/// one undo; checks the one step (criterion 24/11).
pub(super) fn run(model: &OcrModel, file: &str, params: &TraceColourParams) -> (ColourRun, Times) {
    let (mut session, image) = session_with(file);
    let source = session.trace_source(image, false).unwrap();
    let mut times = Times::default();
    let started = cpu_seconds();
    let lines = trace_read_text(
        model,
        &source.rgba,
        source.width,
        source.height,
        &mut |_| std::ops::ControlFlow::Continue(()),
    )
    .unwrap();
    times.ocr = cpu_seconds() - started;
    assert!(session.set_trace_ocr_lines(image, lines.clone(), (source.width, source.height)));
    let started = cpu_seconds();
    let regions = trace_text_regions_in(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        &params.trace,
        &lines,
        None,
    );
    times.regions = cpu_seconds() - started;
    let started = cpu_seconds();
    if let Some(request) = session.trace_fit_request_for(image, &regions, 0.3, &[]) {
        assert!(session.set_trace_fit(&request, &fit(&request)));
    }
    times.fit = cpu_seconds() - started;
    let (decided, _) = session.trace_lines_to_trace(image).unwrap();
    let started = cpu_seconds();
    let wire = trace_text_colour_job_in(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        params,
        &decided,
        None,
        &mut |_| {},
    );
    times.trace = cpu_seconds() - started;
    let started = cpu_seconds();
    let plain = curvyo_ui_core::trace_colour_job_in(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        params,
        None,
        &mut |_| {},
    );
    times.plain = cpu_seconds() - started;
    assert_ne!(plain, wire);
    assert!(session.set_trace_preview(image, &wire));
    assert!(session.set_trace_result(image, &wire));
    assert!(session.trace_text_fitted(), "the one trace keeps the fit");
    let steps = session.document.own_steps().len();
    let TraceCreateOutcome::Created { .. } = session.create_traced_paths() else {
        panic!("nothing created");
    };
    assert_eq!(session.document.own_steps().len(), steps + 1, "one step");
    let out = written(&session, image);
    // Criterion 24/11: one undo removes the group and every text.
    session.undo();
    assert_eq!(session.objects().len(), 1, "only the image is left");
    (out, times)
}

fn written(session: &Session, image: NodeId) -> ColourRun {
    let created = session.trace_created_texts().to_vec();
    let objects = session.objects();
    let mut texts = Vec::new();
    let mut paths = Vec::new();
    let (mut last_path, mut first_text) = (None, None);
    for (k, object) in objects.iter().enumerate() {
        match object {
            ObjectSnapshot::Text(t) => {
                first_text.get_or_insert(k);
                let guessed = created
                    .iter()
                    .find(|c| c.string == t.string)
                    .map(|c| c.guessed.iter().map(|g| g.1 - 1).collect())
                    .unwrap_or_default();
                texts.push((
                    t.string.clone(),
                    t.style.fill.color,
                    guessed,
                    t.size.as_mm(),
                    t.placement.to_array(),
                ));
            }
            ObjectSnapshot::Path(p) => {
                last_path = Some(k);
                paths.push((p.style.fill.color, 1 + p.extra_subpaths.len()));
            }
            ObjectSnapshot::Image(i) => assert_eq!(i.id, image),
            _ => {}
        }
    }
    ColourRun {
        texts,
        paths,
        texts_on_top: first_text.is_none_or(|t| last_path.is_none_or(|p| t > p)),
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color { r, g, b }
}

const RED: Color = Color {
    r: 208,
    g: 34,
    b: 42,
};
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

/// Criteria 23, 24/6, 24/11 and 24/13 on the small multicolour poster: red text on white is a red
/// text; white text on the red box is a white text above the whole box (one outline, no letter
/// hole); one step, one undo; the same twice.
#[test]
fn the_small_colour_poster_gives_coloured_texts_above_whole_shapes() {
    let model = model();
    let (first, _) = run(&model, "colour/colour_small.png", &colour_params());
    let fill_of = |s: &str| first.texts.iter().find(|t| t.0 == s).map(|t| t.1);
    assert_eq!(fill_of("Rote Farbe"), Some(RED), "{first:?}");
    assert_eq!(fill_of("OFFEN"), Some(WHITE), "{first:?}");
    assert_eq!(first.paths, vec![(RED, 1)], "the box whole: {first:?}");
    assert!(first.texts_on_top, "{first:?}");
    let (second, _) = run(&model, "colour/colour_small.png", &colour_params());
    assert_eq!(first, second, "criterion 24/13");
}

/// The truth of the multicolour poster, top to bottom, with each line's colour.
const COLOUR_POSTER: [(&str, Color); 5] = [
    ("Rote Farbe", RED),
    ("OFFEN", WHITE),
    (
        "Laser 42",
        Color {
            r: 250,
            g: 204,
            b: 21,
        },
    ),
    (
        "Blaue Größe",
        Color {
            r: 35,
            g: 78,
            b: 184,
        },
    ),
    (
        "Grün auf Weiß",
        Color {
            r: 31,
            g: 122,
            b: 58,
        },
    ),
];

fn print(file: &str, run: &ColourRun, times: &Times) {
    println!(
        "{file} (Multicolour): {} texts, paths {:?}; thread CPU: OCR {:.2} s, text pass {:.2} s, \
         fit {:.2} s, multicolour trace with text {:.2} s (off: {:.2} s)",
        run.texts.len(),
        run.paths,
        times.ocr,
        times.regions,
        times.fit,
        times.trace,
        times.plain
    );
    for t in &run.texts {
        println!("  {:?} fill {:?} guessed {:?}", t.0, t.1, t.2);
    }
}

/// Criteria 23 and 24/3, 24/6 and 24/13 on the full posters in Multicolour: each line in its own
/// colour (white on the red box, yellow on the blue box), 0 wrong sure letters, the shapes whole,
/// the same twice; the research poster's SALE white on the whole green box, FRESH black on the
/// whole grey box.
#[test]
#[ignore = "harness: prints the numbers of the PR"]
fn the_colour_posters_read_right_in_multicolour() {
    let model = model();
    let (run1, times) = run(&model, "colour/colour_poster.png", &colour_params());
    print("colour_poster.png", &run1, &times);
    let mut wrong = 0;
    for (truth, colour) in COLOUR_POSTER {
        let text = run1.texts.iter().find(|t| t.0 == truth);
        let text = text.unwrap_or_else(|| panic!("{truth} not read: {run1:?}"));
        assert_eq!(text.1, colour, "{truth}");
        wrong += wrong_sure(&text.0, &text.2, truth);
    }
    assert_eq!(wrong, 0, "criterion 24/3");
    // Red box, blue box, green circle, yellow bar: each one whole outline.
    let mut paths = run1.paths.clone();
    paths.sort_by_key(|p| (p.0.r, p.0.g, p.0.b));
    assert!(paths.iter().all(|p| p.1 == 1), "{paths:?}");
    assert_eq!(paths.len(), 4, "{paths:?}");
    assert!(run1.texts_on_top);
    let (run2, _) = run(&model, "colour/colour_poster.png", &colour_params());
    assert_eq!(run1, run2, "criterion 24/13");

    let (poster, times) = run(&model, "poster.png", &colour_params());
    print("poster.png", &poster, &times);
    let mut wrong = 0;
    for (text, _, guessed, _, _) in &poster.texts {
        let nearest = POSTER.iter().min_by_key(|l| levenshtein(text, l)).unwrap();
        wrong += wrong_sure(text, guessed, nearest);
    }
    assert_eq!(wrong, 0, "criterion 24/3 on the research poster");
    let fill_of = |s: &str| poster.texts.iter().find(|t| t.0 == s).map(|t| t.1);
    assert_eq!(fill_of("SALE"), Some(WHITE));
    assert_eq!(fill_of("FRESH"), Some(rgb(0, 0, 0)));
    let green = poster.paths.iter().find(|p| p.0 == rgb(31, 122, 58));
    assert_eq!(green.map(|p| p.1), Some(1), "the box under SALE whole");
    assert!(poster.texts_on_top);
}

/// The settings the worker gets for the native record: Multicolour, Remove background on white.
const RECORD_SETTINGS: &str = r##"{"output":1,"method":1,"background":"#FFFFFF","threshold":128}"##;

fn record_file() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/trace_text_colour_native.json")
}

/// Criterion 24/13 across builds (ADR 0028 §6): the native record of the small multicolour
/// poster, its OCR lines and the multicolour job with text as the worker runs it
/// (`trace_text_job`). `CURVYO_BLESS=1` rewrites it; else the job, run from the recorded OCR
/// lines, must give the recorded numbers exactly. `frontend/tests/trace_text_browser.test.ts` runs
/// the editor wasm on the same lines and compares.
#[test]
fn native_record_of_the_small_colour_poster_matches_the_golden_file() {
    let (mut session, image) = session_with("colour/colour_small.png");
    let source = session.trace_source(image, false).unwrap();
    let bless = std::env::var_os("CURVYO_BLESS").is_some();
    let kept: Option<serde_json::Value> = (!bless)
        .then(|| serde_json::from_str(&std::fs::read_to_string(record_file()).unwrap()).unwrap());
    let ocr = if let Some(k) = &kept {
        k["ocr"].as_str().unwrap().to_owned()
    } else {
        let lines = trace_read_text(
            &model(),
            &source.rgba,
            source.width,
            source.height,
            &mut |_| std::ops::ControlFlow::Continue(()),
        )
        .unwrap();
        crate::trace_glue::ocr_lines_json(source.width, source.height, &lines)
    };
    let params = crate::trace_glue::trace_colour_settings(RECORD_SETTINGS).unwrap();
    let wire = trace_text_colour_job_in(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        &params,
        &crate::trace_glue::read_ocr_lines(&ocr, source.width),
        None,
        &mut |_| {},
    );
    let read = curvyo_ui_core::read_trace_wire(&wire).unwrap();
    assert_eq!(read.texts.lines.len(), 2, "both lines in the record");
    let fnv = source.rgba.iter().fold(0x811c_9dc5_u32, |h, b| {
        (h ^ u32::from(*b)).wrapping_mul(0x0100_0193)
    });
    let record = serde_json::json!({
        "width": source.width,
        "height": source.height,
        "rgba_fnv32": fnv,
        "settings": RECORD_SETTINGS,
        "map": source.map,
        "ocr": ocr,
        "wire": wire,
    });
    match kept {
        None => std::fs::write(record_file(), serde_json::to_string(&record).unwrap()).unwrap(),
        Some(kept) => {
            assert_eq!(kept["rgba_fnv32"], record["rgba_fnv32"]);
            assert_eq!(kept["wire"], record["wire"]);
        }
    }
}

/// Lead 2026-11-02: a text whose colour the maker excluded is left out, as the shapes of that
/// colour are; the other text stays.
#[test]
fn a_text_in_an_excluded_colour_is_not_written() {
    let model = model();
    let mut params = colour_params();
    params.excluded = vec![TraceRgb::new(208, 34, 42)];
    let (out, _) = run(&model, "colour/colour_small.png", &params);
    assert!(out.texts.iter().all(|t| t.0 != "Rote Farbe"), "{out:?}");
    assert!(out.texts.iter().any(|t| t.0 == "OFFEN"), "{out:?}");
    assert!(out.paths.iter().all(|p| p.0 != RED), "{out:?}");
}

/// Lead 2026-11-02: the colour count names the palette colours only texts fill too, so it agrees
/// with the strip (a red text on white beside a blue box: two colours, one path).
#[test]
fn the_colour_count_includes_colours_only_texts_use() {
    let (w, h) = (60_u32, 30_u32);
    let rgba: Vec<u8> = (0..(w * h) as usize)
        .flat_map(|i| {
            let (x, y) = (i % w as usize, i / w as usize);
            if (10..50).contains(&x) && (12..18).contains(&y) {
                [208, 34, 42, 255]
            } else if (54..59).contains(&x) && (4..26).contains(&y) {
                [35, 78, 184, 255]
            } else {
                [255, 255, 255, 255]
            }
        })
        .collect();
    let line = curvyo_ui_core::OcrLine {
        quad: [[8.0, 10.0], [52.0, 10.0], [52.0, 20.0], [8.0, 20.0]],
        text: "Il".to_owned(),
        confidence: vec![0.9, 0.9],
        positions: Vec::new(),
    };
    let mut params = colour_params();
    params.trace.specks = curvyo_ui_core::TraceMm::from_mm(0.0);
    let wire = trace_text_colour_job_in(
        &rgba,
        w,
        h,
        [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        &params,
        &[line],
        None,
        &mut |_| {},
    );
    let (mut session, image) = session_with("colour/colour_small.png");
    assert!(session.set_trace_preview(image, &wire));
    let json: serde_json::Value = serde_json::from_str(&session.trace_palette_json()).unwrap();
    assert_eq!(json["colours"].as_array().unwrap().len(), 2, "{json}");
    assert_eq!(
        json["layers"],
        serde_json::json!(["#234EB8", "#D0222A"]),
        "{json}"
    );
}
