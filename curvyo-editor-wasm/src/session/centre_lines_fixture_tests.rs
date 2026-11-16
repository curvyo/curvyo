//! The real line-art fixture of `specs/0117-centre-lines` criterion 19 (ADR 0063 §12): a
//! whale-like drawing made here (own work, CC0) is painted into a bitmap, traced once by
//! `vectrace` at fixed settings, and the traced outlines are written as text to
//! `curvyo-geometry-core/tests/fixtures/centre_lines/whale_traced.txt`. Run it by hand to
//! regenerate: `CURVYO_BLESS=1 cargo test -p curvyo-editor-wasm bless_traced_whale -- --ignored`.

use std::fmt::Write as _;

use curvyo_document_core::ObjectSnapshot;

use super::trace_tests::trace_job_with;
use super::{Session, Tool};

/// Pixels per millimetre of the drawing.
const PX_PER_MM: f64 = 8.0;
/// The tracer's smoothing, as the Trace tool's tests use it.
const SMOOTH: u8 = 80;
const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

/// One stroke: a polyline in millimetres with a half width at every point.
struct Line(Vec<(f64, f64, f64)>);

#[allow(clippy::too_many_arguments)] // centre, radii, angles and widths of one arc
fn arc(cx: f64, cy: f64, rx: f64, ry: f64, from: f64, to: f64, w0: f64, w1: f64) -> Line {
    Line(
        (0..=96)
            .map(|i| {
                let t = f64::from(i) / 96.0;
                let a = from + (to - from) * t;
                (
                    cx + rx * a.cos(),
                    cy + ry * a.sin(),
                    f64::midpoint(w0, (w1 - w0) * t),
                )
            })
            .collect(),
    )
}

fn line(points: &[(f64, f64)], w0: f64, w1: f64) -> Line {
    let n = points.len() - 1;
    Line(
        points
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| {
                #[allow(clippy::cast_precision_loss)]
                let t = i as f64 / n as f64;
                (x, y, f64::midpoint(w0, (w1 - w0) * t))
            })
            .collect(),
    )
}

/// The drawing: body ring, belly line, an eye with a pupil, a mouth with round ends, a tail Y,
/// a fin T, tapering throat stripes, a loop with a branch, a spout with two holes.
fn drawing() -> (Vec<Line>, Vec<(f64, f64, f64)>) {
    let pi = std::f64::consts::PI;
    let mut lines = vec![
        arc(60.0, 45.0, 48.0, 28.0, 0.0, 2.0 * pi, 2.0, 2.0),
        arc(60.0, 40.0, 44.0, 26.0, 0.35, pi - 0.35, 1.6, 0.6),
        arc(36.0, 36.0, 4.0, 4.0, 0.0, 2.0 * pi, 1.0, 1.0),
        arc(48.0, 50.0, 12.0, 5.0, 0.3, pi - 0.6, 0.8, 0.8),
        line(&[(108.0, 45.0), (122.0, 40.0)], 2.0, 2.0),
        line(&[(122.0, 40.0), (138.0, 34.0)], 2.0, 1.2),
        line(&[(122.0, 40.0), (132.0, 52.0)], 1.2, 0.5),
        line(&[(52.0, 66.0), (72.0, 66.0)], 1.5, 1.5),
        line(&[(62.0, 66.0), (62.0, 78.0)], 1.5, 0.6),
        arc(20.0, 12.0, 4.0, 4.0, 0.0, 2.0 * pi, 1.0, 1.0),
        line(&[(24.0, 12.0), (34.0, 12.0)], 1.0, 1.0),
    ];
    for k in 0..4 {
        let x = 40.0 + 8.0 * f64::from(k);
        lines.push(line(&[(x, 58.0), (x + 4.0, 68.0)], 1.0, 0.3));
    }
    lines.push(line(&[(84.0, 6.0), (100.0, 6.0)], 6.0, 6.0));
    let discs = vec![(36.0, 36.0, 1.8), (88.0, 6.0, 1.2), (96.0, 6.0, 1.2)];
    (lines, discs)
}

/// Whether the millimetre point `(x, y)` is dark: inside a stroke and not inside a white disc,
/// except the pupil (the first disc), which is dark.
fn dark(lines: &[Line], discs: &[(f64, f64, f64)], x: f64, y: f64) -> bool {
    let inside = |(cx, cy, r): (f64, f64, f64)| (x - cx).hypot(y - cy) <= r;
    if inside(discs[0]) {
        return true;
    }
    if discs[1..].iter().any(|&d| inside(d)) {
        return false;
    }
    lines.iter().any(|Line(points)| {
        points.windows(2).any(|pair| {
            let ((ax, ay, ar), (bx, by, br)) = (pair[0], pair[1]);
            let (dx, dy) = (bx - ax, by - ay);
            let len2 = dx * dx + dy * dy;
            let t = if len2 > 0.0 {
                (((x - ax) * dx + (y - ay) * dy) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (x - ax - t * dx).hypot(y - ay - t * dy) <= ar + (br - ar) * t
        })
    })
}

fn png() -> Vec<u8> {
    let (lines, discs) = drawing();
    let image = image::RgbaImage::from_fn(WIDTH, HEIGHT, |px, py| {
        let (x, y) = (
            (f64::from(px) + 0.5) / PX_PER_MM,
            (f64::from(py) + 0.5) / PX_PER_MM,
        );
        if dark(&lines, &discs, x, y) {
            image::Rgba([0, 0, 0, 255])
        } else {
            image::Rgba([255, 255, 255, 255])
        }
    });
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}

/// The traced outlines of `png` (read in as an image, as the app imports it) as fixture text
/// after `header`, and their node count; `params` are the trace settings.
fn traced_text(png: &[u8], params: &vectrace::TraceParams, header: &str) -> (String, usize) {
    let mut session = Session::new(1);
    session.import_begin(None);
    session.import_add(png, "line-art.png");
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
    let source = session.trace_source(image, false).unwrap();
    let wire = trace_job_with(
        &source.rgba,
        source.width,
        source.height,
        source.map,
        params,
    );
    assert!(session.set_trace_result(image, &wire));
    session.create_traced_paths();
    let mut text = String::from(header);
    let mut nodes = 0;
    for object in session.objects().iter() {
        let ObjectSnapshot::Path(path) = object else {
            continue;
        };
        text.push_str("subject\n");
        for subpath in path.subpaths() {
            let _ = writeln!(text, "outline {} {}", subpath.closed, subpath.anchors.len());
            nodes += subpath.anchors.len();
            for a in subpath.anchors {
                let _ = writeln!(
                    text,
                    "{:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
                    a.point.x,
                    a.point.y,
                    a.handle_in.x,
                    a.handle_in.y,
                    a.handle_out.x,
                    a.handle_out.y
                );
            }
        }
    }
    (text, nodes)
}

/// Writes `text` to the fixture `name` when `CURVYO_BLESS` is set.
fn bless(name: &str, text: &str) {
    if std::env::var("CURVYO_BLESS").is_ok() {
        let out = format!(
            "{}/../curvyo-geometry-core/tests/fixtures/centre_lines/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::create_dir_all(std::path::Path::new(&out).parent().unwrap()).unwrap();
        std::fs::write(out, text).unwrap();
    }
}

#[test]
#[ignore = "regenerates a reviewed fixture; run by hand with CURVYO_BLESS=1"]
fn bless_traced_whale() {
    let params = vectrace::TraceParams {
        smooth: SMOOTH,
        specks: vectrace::Mm::from_mm(0.0),
        ..vectrace::TraceParams::with_threshold(128)
    };
    let (text, nodes) = traced_text(
        &png(),
        &params,
        "# Whale-like line art made for Curvyo (own work, CC0), traced once by vectrace v0.1.0\n\
         # (threshold 128, smooth 80, no speck removal). Per subject: `subject`, then per outline\n\
         # `outline <closed> <anchors>` and one anchor per line: x y in_x in_y out_x out_y (mm).\n",
    );
    eprintln!("traced whale: {nodes} nodes");
    bless("whale_traced.txt", &text);
}

/// The customer's dolphin with thick wave bands (criteria 19 and 20), traced at the app's
/// default settings. `CURVYO_TRACE_PNG` and `CURVYO_TRACE_OUT` trace another picture to another
/// file instead, for measurements of pictures that are not checked in.
#[test]
#[ignore = "regenerates a reviewed fixture; run by hand with CURVYO_BLESS=1"]
fn bless_traced_dolphin() {
    let png = match std::env::var("CURVYO_TRACE_PNG") {
        Ok(path) => std::fs::read(path).unwrap(),
        Err(_) => {
            include_bytes!("../../../curvyo-geometry-core/tests/fixtures/centre_lines/dolphin.png")
                .to_vec()
        }
    };
    let (text, nodes) = traced_text(
        &png,
        &vectrace::TraceParams::with_threshold(128),
        "# The customer's own drawing of a dolphin over thick wave bands (dolphin.png, check-in\n\
         # approved 2026-11-18), traced once by vectrace at the app's default settings (threshold\n\
         # 128, smooth 80, tolerance 0.5 px, specks 0.5 mm, corners 70). Format as whale_traced.txt.\n",
    );
    eprintln!("traced: {nodes} nodes");
    match std::env::var("CURVYO_TRACE_OUT") {
        Ok(path) => std::fs::write(path, text).unwrap(),
        Err(_) => bless("dolphin_traced.txt", &text),
    }
}
