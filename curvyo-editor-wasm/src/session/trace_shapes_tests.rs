//! Recognise shapes in Trace end to end (`specs/0088-trace-recognition` Part B criteria 1, 6, 8,
//! 10, 12, 14 and 15): the worker's job with the shape section, the preview's count, Create paths
//! in one step and one undo; the text poster with Recognise text on too.

use curvyo_document_core::{Color, ObjectSnapshot, Shape};
use curvyo_ui_core::read_trace_wire;
use vectrace::job::{Job as TraceJob, Mode as TraceMode, run as run_trace_job};
use vectrace::ocr::read_text;
use vectrace::shapes::add_shapes;
use vectrace::{PixelMap as TracePixelMap, Raster as TraceRaster, TraceParams};

use super::trace_text_tests::{cpu_seconds, fit, model, session_with, session_with_bytes};
use super::{Session, TraceCreateOutcome};

/// A PNG of `width` x `height` whose pixel (centre `x`, `y`) has the colour `paint` gives, white
/// elsewhere.
fn png(width: u32, height: u32, paint: impl Fn(f64, f64) -> Option<[u8; 3]>) -> Vec<u8> {
    use image::ImageEncoder;
    let mut rgba = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let c = paint(f64::from(x) + 0.5, f64::from(y) + 0.5).unwrap_or([255; 3]);
            rgba.extend([c[0], c[1], c[2], 255]);
        }
    }
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&rgba, width, height, image::ExtendedColorType::Rgba8)
        .unwrap();
    out
}

/// What Create paths left besides the images: primitives (kind, fill), paths (subpaths, fill),
/// texts.
#[derive(Debug, Default)]
struct Made {
    primitives: Vec<(&'static str, Color)>,
    paths: Vec<(usize, Color)>,
    texts: Vec<String>,
}

fn made(session: &Session) -> Made {
    let mut out = Made::default();
    for object in session.objects().iter() {
        match object {
            ObjectSnapshot::Primitive(p) => {
                let kind = match p.shape {
                    Shape::Ellipse { frame, .. } if frame.rx == frame.ry => "circle",
                    Shape::Ellipse { .. } => "ellipse",
                    Shape::Rect { .. } => "rect",
                    Shape::Polygon { .. } => "polygon",
                    Shape::Star { .. } => "star",
                };
                out.primitives.push((kind, p.style.fill.color));
            }
            ObjectSnapshot::Path(p) => out
                .paths
                .push((1 + p.extra_subpaths.len(), p.style.fill.color)),
            ObjectSnapshot::Text(t) => out.texts.push(t.string.clone()),
            _ => {}
        }
    }
    out
}

/// Traces the imported picture with `settings`-like values and Recognise shapes on, keeps the
/// result and shows it; the session as the host leaves it before Create paths.
fn traced(png: &[u8], multicolour: bool) -> Session {
    let (mut session, image) = session_with_bytes(png, "shapes.png");
    let source = session.trace_source(image, false).unwrap();
    let wire = if multicolour {
        let params = crate::trace_glue::trace_colour_settings(
            r##"{"output":1,"method":1,"background":"#FFFFFF","threshold":128,"specks":0}"##,
        )
        .unwrap();
        run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: source.width,
                    height: source.height,
                    rgba: &source.rgba,
                },
                map: TracePixelMap::new(source.map),
                mode: TraceMode::Colours(&params),
                lines: &[],
                mask: None,
                shapes: true,
            },
            &mut |_| {},
        )
    } else {
        run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: source.width,
                    height: source.height,
                    rgba: &source.rgba,
                },
                map: TracePixelMap::new(source.map),
                mode: TraceMode::Mono(&TraceParams::with_threshold(128)),
                lines: &[],
                mask: None,
                shapes: true,
            },
            &mut |_| {},
        )
    };
    assert!(session.set_trace_result(image, &wire));
    assert!(session.set_trace_preview(image, &wire));
    session
}

/// Criteria 1, 8 and 14: a circle, a rectangle and a ring become a circle primitive, a rectangle
/// primitive and one compound of two circles, in the "Trace of" group, black, in one step that
/// one undo takes back; the preview counts four shapes (UX notes B3).
#[test]
fn c1_c8_c14_create_paths_sets_primitives_in_one_step() {
    let bytes = png(800, 400, |x, y| {
        let ring = (x - 600.0).hypot(y - 200.0);
        let dark = (x - 100.0).hypot(y - 100.0) <= 60.0
            || ((150.0..350.0).contains(&x) && (250.0..350.0).contains(&y))
            || (40.0..=100.0).contains(&ring);
        dark.then_some([0, 0, 0])
    });
    let mut session = traced(&bytes, false);
    let image = session.trace_view().images[0];
    let map = session.trace_source(image, false).unwrap().map;
    let px = (map[0] * map[3] - map[1] * map[2]).abs().sqrt();
    // Criterion 16: the ring is one stroked circle, so 3 shapes.
    assert_eq!(session.trace_shape_count(), 3);
    let steps = session.document.own_steps().len();
    let TraceCreateOutcome::Created { .. } = session.create_traced_paths() else {
        panic!("nothing created");
    };
    assert_eq!(session.document.own_steps().len(), steps + 1, "one step");
    let out = made(&session);
    let mut kinds: Vec<&str> = out.primitives.iter().map(|p| p.0).collect();
    kinds.sort_unstable();
    assert_eq!(kinds, ["circle", "circle", "rect"], "{out:?}");
    assert_eq!(out.paths, [], "{out:?}");
    let strokes: Vec<(bool, f64, Color, bool)> = session
        .objects()
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Primitive(p) => Some((
                p.style.stroke.enabled,
                p.style.stroke.width.as_mm(),
                p.style.stroke.color,
                p.style.fill.enabled,
            )),
            _ => None,
        })
        .filter(|s| s.0)
        .collect();
    assert_eq!(strokes.len(), 1, "one stroked primitive: {strokes:?}");
    let (_, width, colour, filled) = strokes[0];
    assert!(
        (width - 60.0 * px).abs() <= 2.0 * px,
        "the ring's 60 px width: {width} mm"
    );
    assert_eq!((colour, filled), (Color::BLACK, false));
    // The selection is the result (the group's leaves), as before.
    assert_eq!(session.selection.ids().len(), 3);
    session.undo();
    assert_eq!(session.objects().len(), 1, "only the image is left");
}

/// Criterion 10: in Multicolour a red circle and a blue rectangle keep their fills.
#[test]
fn c10_multicolour_primitives_keep_their_fill() {
    let (red, blue) = ([208, 34, 42], [35, 78, 184]);
    let bytes = png(600, 300, |x, y| {
        if (x - 100.0).hypot(y - 150.0) <= 80.0 {
            Some(red)
        } else if (300.0..550.0).contains(&x) && (50.0..250.0).contains(&y) {
            Some(blue)
        } else {
            None
        }
    });
    let mut session = traced(&bytes, true);
    assert!(matches!(
        session.create_traced_paths(),
        TraceCreateOutcome::Created { .. }
    ));
    let out = made(&session);
    let colour = |c: [u8; 3]| Color {
        r: c[0],
        g: c[1],
        b: c[2],
    };
    let mut primitives = out.primitives.clone();
    primitives.sort_by_key(|p| p.0);
    assert_eq!(
        primitives,
        [("circle", colour(red)), ("rect", colour(blue))],
        "{out:?}"
    );
    assert_eq!(out.paths, []);
}

/// Criteria 12 and 15: the poster (text lines, a circle, a triangle, a square and two boxes
/// behind text) with Recognise text and Recognise shapes on: the lines become texts, the shapes
/// primitives, and no letter (not even an "o") becomes a shape. The OCR and the fit run
/// in memory with no network by construction.
#[test]
fn c12_letters_never_become_shapes() {
    let model = model();
    let (mut session, image) = session_with("poster.png");
    let source = session.trace_source(image, false).unwrap();
    let lines = read_text(
        &model,
        &source.rgba,
        source.width,
        source.height,
        &mut |_| std::ops::ControlFlow::Continue(()),
    )
    .unwrap();
    assert!(session.set_trace_ocr_lines(image, lines.clone(), (source.width, source.height)));
    let threshold = vectrace::auto_threshold(vectrace::Raster {
        width: source.width,
        height: source.height,
        rgba: &source.rgba,
    });
    let mut wire = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: source.width,
                height: source.height,
                rgba: &source.rgba,
            },
            map: TracePixelMap::new(source.map),
            mode: TraceMode::Mono(&TraceParams::with_threshold(threshold)),
            lines: &lines,
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    );
    let started = cpu_seconds();
    add_shapes(&mut wire, TracePixelMap::new(source.map));
    let shape_seconds = cpu_seconds() - started;
    let read = read_trace_wire(&wire).unwrap();
    eprintln!(
        "poster: {} outlines, {} regions, shape pass {shape_seconds:.3} s thread CPU",
        read.outlines.len(),
        read.shapes.len()
    );
    assert!(session.set_trace_result(image, &wire));
    assert!(session.set_trace_preview(image, &wire));
    let request = session.trace_fit_request(0.3, &[]).unwrap();
    assert!(session.set_trace_fit(&request, &fit(&request)));
    assert!(matches!(
        session.create_traced_paths(),
        TraceCreateOutcome::Created { .. }
    ));
    let out = made(&session);
    eprintln!("{out:?}");
    assert!(out.texts.iter().any(|t| t.contains("Wedding")), "{out:?}");
    let mut kinds: Vec<&str> = out.primitives.iter().map(|p| p.0).collect();
    kinds.sort_unstable();
    // The circle, the triangle, the square and the SALE and FRESH boxes (their text left out).
    assert_eq!(
        kinds,
        ["circle", "polygon", "rect", "rect", "rect"],
        "{out:?}"
    );
    let objects = session.objects();
    // The tallest letters (BÄCKEREI) are about 11 mm high; the smallest shape, the SALE box, 17 mm.
    for object in objects.iter() {
        if let ObjectSnapshot::Primitive(p) = object {
            let outline = curvyo_document_core::drawing_outline_of_rotated(&p.shape, p.rotation);
            let (xs, ys): (Vec<f64>, Vec<f64>) = outline
                .anchors
                .iter()
                .map(|a| (a.point.x, a.point.y))
                .unzip();
            let span = |v: &[f64]| {
                v.iter().copied().fold(f64::MIN, f64::max)
                    - v.iter().copied().fold(f64::MAX, f64::min)
            };
            assert!(
                span(&xs).min(span(&ys)) > 15.0,
                "no letter-sized shape: {:?}",
                p.shape
            );
        }
    }
}

/// Criterion 6: 500 traced closed paths (circles, rectangles, stars, hexagons and irregular
/// blobs, 6 mm each) through the shape pass in under 2 s thread CPU, inside the trace job.
#[test]
#[allow(clippy::many_single_char_names)] // the picture's coordinates
fn c6_five_hundred_paths_take_under_two_seconds() {
    let (cols, rows, cell) = (25_u32, 20_u32, 80.0);
    let bytes = png(cols * 80, rows * 80, |x, y| {
        let (i, j) = ((x / cell).floor(), (y / cell).floor());
        let (dx, dy) = (x - (i + 0.5) * cell, y - (j + 0.5) * cell);
        let (r, t) = (dx.hypot(dy), dy.atan2(dx));
        // invariant: the cell index is below 500.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let dark = match (i + j * f64::from(cols)) as u32 % 5 {
            0 => r <= 28.0,
            1 => dx.abs() <= 30.0 && dy.abs() <= 20.0,
            2 => {
                let k = (t * 5.0 / std::f64::consts::TAU).rem_euclid(1.0);
                r <= 14.0 + 16.0 * (2.0 * k - 1.0).abs()
            }
            3 => {
                r * ((t.rem_euclid(std::f64::consts::FRAC_PI_3) - std::f64::consts::FRAC_PI_6)
                    .cos())
                    <= 26.0
            }
            _ => r <= 24.0 * (1.0 + 0.15 * (3.0 * t).sin() + 0.08 * (7.0 * t + 1.0).sin()),
        };
        dark.then_some([0, 0, 0])
    });
    let (mut session, image) = session_with_bytes(&bytes, "many.png");
    let source = session.trace_source(image, false).unwrap();
    let plain = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: source.width,
                height: source.height,
                rgba: &source.rgba,
            },
            map: TracePixelMap::new(source.map),
            mode: TraceMode::Mono(&TraceParams::with_threshold(128)),
            lines: &[],
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    );
    let mut wire = plain.clone();
    let started = cpu_seconds();
    add_shapes(&mut wire, TracePixelMap::new(source.map));
    let seconds = cpu_seconds() - started;
    let read = read_trace_wire(&wire).unwrap();
    eprintln!(
        "{} paths, {} recognised, shape pass {seconds:.3} s thread CPU",
        read.outlines.len(),
        read.shapes.len()
    );
    assert_eq!(read.outlines.len(), 500);
    assert_eq!(
        read.shapes.len(),
        400,
        "the blobs stay paths, the rest are shapes"
    );
    assert!(seconds < 2.0, "{seconds} s");
    assert!(session.set_trace_result(image, &wire));
}

/// The `vectrace` golden pictures (binary PGM).
fn pgm(file: &str) -> (u32, u32, Vec<u8>) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../vectrace/tests/fixtures/trace")
        .join(file);
    let bytes = std::fs::read(path).unwrap();
    let text = String::from_utf8_lossy(&bytes[..20]).to_string();
    let fields: Vec<&str> = text.split_whitespace().collect();
    let (w, h): (u32, u32) = (fields[1].parse().unwrap(), fields[2].parse().unwrap());
    let grey = &bytes[bytes.len() - (w * h) as usize..];
    (w, h, grey.iter().flat_map(|&g| [g, g, g, 255]).collect())
}

/// Criterion 13 across builds: the shape sections of the golden pictures (disc, rectangle, ring
/// with a dot) as the worker's `trace_job` makes them with `"shapes": true`, recorded natively;
/// `CURVYO_BLESS=1` rewrites the record, else they must be equal bit for bit.
/// `frontend/tests/traceWasm.test.ts` runs the editor wasm on the same pictures and compares.
#[test]
fn native_record_of_the_shape_sections_matches_the_golden_file() {
    let record_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/trace_shapes_native.json");
    let map = [
        0.264_583_333_333_333_34,
        0.0,
        0.0,
        0.264_583_333_333_333_34,
        0.0,
        0.0,
    ];
    let settings = r#"{"threshold":128,"smooth":80,"specks":0,"shapes":true}"#;
    assert!(crate::trace_glue::recognise_shapes(settings));
    let mut record = serde_json::Map::new();
    for file in ["disc.pgm", "rectangle.pgm", "ring-with-dot.pgm"] {
        let (w, h, rgba) = pgm(file);
        let plain = run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: w,
                    height: h,
                    rgba: &rgba,
                },
                map: TracePixelMap::new(map),
                mode: TraceMode::Mono(&crate::trace_glue::trace_settings(settings)),
                lines: &[],
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        );
        let wire = run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: w,
                    height: h,
                    rgba: &rgba,
                },
                map: TracePixelMap::new(map),
                mode: TraceMode::Mono(&crate::trace_glue::trace_settings(settings)),
                lines: &[],
                mask: None,
                shapes: true,
            },
            &mut |_| {},
        );
        let read = read_trace_wire(&wire).unwrap();
        assert!(!read.shapes.is_empty(), "{file}");
        record.insert(file.to_owned(), serde_json::json!(wire[plain.len()..]));
    }
    let record = serde_json::json!({ "map": map, "settings": settings, "sections": record });
    if std::env::var_os("CURVYO_BLESS").is_some() {
        std::fs::write(&record_file, serde_json::to_string_pretty(&record).unwrap()).unwrap();
    }
    let kept: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&record_file).unwrap()).unwrap();
    assert_eq!(kept, record);
}
