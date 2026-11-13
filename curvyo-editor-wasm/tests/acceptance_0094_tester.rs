//! Tester cases for `specs/0094-masked-composite-trace` slice 1 at the session (criteria 1, 2, 5
//! to 8, 12, 15, 17), as `specs/0114-image-trace-mask` keeps them: the mask is no longer a set of
//! selected paths but the regions drawn on the picture with the Trace tool's Rectangle and Lasso
//! (`0114` Behaviour 1 to 3), here drawn by the session's own pointer and keys; the trace runs as
//! the worker runs it, from the session's own settings JSON (`trace_settings`,
//! `trace_mask_region`) over the pixels the session decoded. The cases that existed only for mask
//! paths (groups as mask paths, the mixed-selection row, Shift+click) went with them (`0114`
//! criterion 21).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]

#[path = "../../curvyo-ui-core/tests/support_0094/mod.rs"]
mod support;

use curvyo_document_core::{Document, NodeId, ObjectSnapshot, Point, unpack};
use curvyo_editor_wasm::{
    KeyInput, Session, Tool, TraceCreateOutcome, TraceRegionTool, trace_mask_region, trace_settings,
};
use curvyo_ui_core::read_trace_wire;
use image::ImageEncoder;
use kurbo::PathEl;
use support::{M1, base64, draw};
use vectrace::job::{Job as TraceJob, Mode as TraceMode, run as run_trace_job};
use vectrace::mask::Mask;
use vectrace::{PixelMap as TracePixelMap, Raster as TraceRaster};

/// The picture's size: 200 px over 100 mm.
const PX: u32 = 200;
const EDGE_MM: f64 = 0.01;

fn png() -> Vec<u8> {
    let rgba = draw(M1, PX);
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&rgba, PX, PX, image::ExtendedColorType::Rgba8)
        .unwrap();
    out
}

/// An SVG in mm with the M1 picture at 0..100 mm and `extra` after it.
fn svg(image_attrs: &str, extra: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="300mm" height="300mm" viewBox="-100 -100 300 300"><image id="m1" x="0" y="0" width="100" height="100" preserveAspectRatio="none" {image_attrs} xlink:href="data:image/png;base64,{}"/>{extra}</svg>"#,
        base64(&png())
    )
}

fn import(svg: &str) -> Session {
    let mut s = Session::new(1);
    s.resize_viewport(1200.0, 800.0);
    let outcome = s.import_svg(svg.as_bytes(), "masked.svg").unwrap();
    assert!(
        format!("{outcome:?}").starts_with("Imported"),
        "{outcome:?}"
    );
    s
}

fn doc(s: &Session) -> Document {
    unpack(9, &s.pack("t").unwrap()).unwrap()
}

fn image_of(d: &Document) -> (NodeId, [Point; 4]) {
    d.object_ids()
        .into_iter()
        .find_map(|id| match d.object(id) {
            Some(ObjectSnapshot::Image(i)) => Some((id, i.corners())),
            _ => None,
        })
        .expect("an image")
}

/// The image's pixel (x, y) of the picture's own grid in document millimetres.
fn at(corners: &[Point; 4], x: f64, y: f64) -> Point {
    let (u, v) = (x / f64::from(PX), y / f64::from(PX));
    let (o, a, c) = (corners[0], corners[1], corners[3]);
    Point::new(
        o.x + (a.x - o.x) * u + (c.x - o.x) * v,
        o.y + (a.y - o.y) * u + (c.y - o.y) * v,
    )
}

/// The worker's job from the session's settings with `threshold`.
fn traced(s: &mut Session, image: NodeId, threshold: u8) -> Vec<f64> {
    let mask = s.trace_mask_json();
    let settings = if mask.is_empty() {
        format!(r#"{{"threshold":{threshold}}}"#)
    } else {
        format!(r#"{{"threshold":{threshold},"mask":{mask}}}"#)
    };
    let source = s.trace_source(image, false).expect("pixels");
    let region = trace_mask_region(&settings);
    assert_eq!(
        region.is_some(),
        !mask.is_empty(),
        "the region crosses in the settings"
    );
    run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: source.width,
                height: source.height,
                rgba: &source.rgba,
            },
            map: TracePixelMap::new(source.map),
            mode: TraceMode::Mono(&trace_settings(&settings)),
            lines: &[],
            mask: region.as_ref().map(|r| r as &dyn Mask),
            shapes: false,
        },
        &mut |_| {},
    )
}

fn nodes(wire: &[f64]) -> Vec<Point> {
    let mut out = Vec::new();
    for p in read_trace_wire(wire).unwrap().outlines {
        for el in p.elements() {
            match *el {
                PathEl::MoveTo(q)
                | PathEl::LineTo(q)
                | PathEl::CurveTo(_, _, q)
                | PathEl::QuadTo(_, q) => {
                    out.push(Point::new(q.x, q.y));
                }
                PathEl::ClosePath => {}
            }
        }
    }
    out
}

/// The signed distance of `p` from the convex `poly`, positive outside.
fn convex_distance(poly: &[Point], p: Point) -> f64 {
    let twice: f64 = (0..poly.len())
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    let mut worst = f64::NEG_INFINITY;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let (nx, ny) = if twice > 0.0 { (dy, -dx) } else { (-dy, dx) };
        worst = worst.max(((p.x - a.x) * nx + (p.y - a.y) * ny) / dx.hypot(dy));
    }
    worst
}

/// The picture alone, imported and selected, in the Trace tool.
fn picture(image_attrs: &str, extra: &str) -> (Session, NodeId, [Point; 4]) {
    let mut s = import(&svg(image_attrs, extra));
    let (image, corners) = image_of(&doc(&s));
    s.set_tool(Tool::Trace);
    (s, image, corners)
}

/// A rectangle region dragged with the Rectangle tool from picture pixel `a` to `b`; with
/// `subtract` taken out.
fn rectangle(s: &mut Session, corners: &[Point; 4], a: (f64, f64), b: (f64, f64), subtract: bool) {
    s.set_trace_region_tool(TraceRegionTool::Rect);
    s.set_trace_subtract(subtract);
    let (from, to) = (at(corners, a.0, a.1), at(corners, b.0, b.1));
    s.pointer_down(from, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
    s.set_trace_subtract(false);
    s.set_trace_region_tool(TraceRegionTool::None);
}

fn key(s: &mut Session, key: &str) {
    s.key_down(KeyInput {
        key,
        shift: false,
        ctrl: false,
        alt: false,
        repeat: false,
        dom_blocked: false,
        mac: false,
        time_ms: 0.0,
    });
}

/// A lasso clicked corner by corner with the Lasso tool at picture pixels `points`, finished by
/// Enter.
fn lasso(s: &mut Session, corners: &[Point; 4], points: &[(f64, f64)]) {
    s.set_trace_region_tool(TraceRegionTool::Lasso);
    for &(x, y) in points {
        let p = at(corners, x, y);
        s.pointer_down(p, false);
        s.pointer_up(p, false, false);
    }
    key(s, "Enter");
    s.set_trace_region_tool(TraceRegionTool::None);
}

/// R1 over the left half of the picture: picture pixels 0..100 by 0..200.
fn r1(s: &mut Session, corners: &[Point; 4]) {
    rectangle(s, corners, (0.0, 0.0), (100.0, 200.0), false);
}

// ---------------------------------------------------------------------------------------------

#[test]
fn c1_a_region_masks_inside_by_default_the_image_alone_is_not() {
    let (mut s, _, corners) = picture("", "");
    assert!(s.trace_region().is_none());
    assert_eq!(s.trace_mask_json(), "", "the settings carry no mask");
    r1(&mut s, &corners);
    let view = s.trace_view();
    assert_eq!(view.regions, 1, "one region");
    assert!(!view.mask_outside_image && !view.mask_failed);
    assert!(!s.trace_mask_outside(), "Inside is selected");
    let json: serde_json::Value = serde_json::from_str(&s.trace_mask_json()).unwrap();
    assert_eq!(json["outside"], false);
    assert!(s.trace_region().is_some());
}

#[test]
fn c2_c12_create_paths_inside_r1_keeps_the_region_and_is_one_undo() {
    let (mut s, image, corners) = picture("", "");
    r1(&mut s, &corners);
    let before = doc(&s);
    let mask_before = before.trace_mask(image);
    let wire = traced(&mut s, image, 50);
    let r1q = [
        at(&corners, 0.0, 0.0),
        at(&corners, 100.0, 0.0),
        at(&corners, 100.0, 200.0),
        at(&corners, 0.0, 200.0),
    ];
    let all = nodes(&wire);
    assert_ne!(all.len(), 0);
    for p in &all {
        assert!(
            convex_distance(&r1q, *p) <= EDGE_MM,
            "node {p:?} outside R1"
        );
    }
    assert!(s.set_trace_preview(image, &wire));
    assert!(s.set_trace_result(image, &wire));
    let undo_before = s.undo_len();
    let outcome = s.create_traced_paths();
    assert!(
        matches!(outcome, TraceCreateOutcome::Created { images: 1, .. }),
        "{outcome:?}"
    );
    assert_eq!(s.undo_len(), undo_before + 1, "one history step");
    let after = doc(&s);
    assert_eq!(
        after.trace_mask(image),
        mask_before,
        "the region is unchanged"
    );
    let groups: Vec<NodeId> = after
        .object_ids()
        .into_iter()
        .filter_map(|id| after.parent_group(id))
        .collect();
    let group = *groups.first().expect("a trace group");
    let name = after
        .read_nodes()
        .into_iter()
        .find(|n| n.id == group)
        .and_then(|n| n.registers.name);
    assert!(
        name.as_deref().is_some_and(|n| n.starts_with("Trace of")),
        "one group \"Trace of ...\": {name:?}"
    );
    s.undo();
    let undone = doc(&s);
    assert_eq!(
        undone.object_ids(),
        before.object_ids(),
        "one undo restores the document"
    );
    assert_eq!(undone.trace_mask(image), mask_before);
}

#[test]
fn c15_the_session_flow_twice_is_identical() {
    let (mut s, image, corners) = picture("", "");
    r1(&mut s, &corners);
    let a = traced(&mut s, image, 50);
    let b = traced(&mut s, image, 50);
    assert_eq!(a, b);
    let (mut t, image_t, corners_t) = picture("", "");
    r1(&mut t, &corners_t);
    assert_eq!(
        traced(&mut t, image_t, 50),
        a,
        "a second session (worker) gives the same numbers"
    );
}

/// `0094` criterion 5 with regions: a subtracted region is a hole of the added one, and two
/// overlapping regions are one union.
#[test]
fn c5_a_subtracted_region_is_a_hole_and_overlaps_are_one_union() {
    let (mut s, image, corners) = picture("", "");
    // Around the disc's centre (50, 100) px: outer 40 px each way, hole 12 px each way.
    rectangle(&mut s, &corners, (10.0, 60.0), (90.0, 140.0), false);
    rectangle(&mut s, &corners, (38.0, 88.0), (62.0, 112.0), true);
    let region = s.trace_region().expect("a region");
    let centre = at(&corners, 50.0, 100.0);
    assert!(!region.keeps(centre), "the hole is no part of the region");
    assert!(region.keeps(at(&corners, 50.0, 70.0)), "the ring is");
    let outer = [
        at(&corners, 10.0, 60.0),
        at(&corners, 90.0, 60.0),
        at(&corners, 90.0, 140.0),
        at(&corners, 10.0, 140.0),
    ];
    let hole = [
        at(&corners, 38.0, 88.0),
        at(&corners, 62.0, 88.0),
        at(&corners, 62.0, 112.0),
        at(&corners, 38.0, 112.0),
    ];
    for p in nodes(&traced(&mut s, image, 50)) {
        assert!(
            convex_distance(&outer, p) <= EDGE_MM,
            "node {p:?} off the ring"
        );
        assert!(
            convex_distance(&hole, p) >= -EDGE_MM,
            "node {p:?} in the hole"
        );
    }

    let (mut s, _, corners) = picture("", "");
    rectangle(&mut s, &corners, (20.0, 60.0), (80.0, 140.0), false);
    rectangle(&mut s, &corners, (60.0, 80.0), (140.0, 120.0), false);
    assert_eq!(s.trace_view().regions, 2);
    let region = s.trace_region().unwrap();
    for (x, y) in [(30.0, 70.0), (70.0, 100.0), (130.0, 110.0)] {
        assert!(
            region.keeps(at(&corners, x, y)),
            "({x}, {y}) px is in the union"
        );
    }
    assert!(!region.keeps(at(&corners, 30.0, 150.0)));
}

/// `0094` criterion 6 as the lasso keeps it (`0114` Behaviour 3): corners set and finished are
/// closed by a straight line.
#[test]
fn c6_a_lasso_is_closed_by_a_straight_line() {
    let (mut s, image, corners) = picture("", "");
    lasso(
        &mut s,
        &corners,
        &[(0.0, 0.0), (100.0, 0.0), (100.0, 200.0)],
    );
    let view = s.trace_view();
    assert_eq!(view.regions, 1, "the lasso is a region");
    assert!(!view.mask_failed && !view.mask_outside_image, "no refusal");
    let region = s.trace_region().expect("a region");
    assert!(
        region.keeps(at(&corners, 90.0, 40.0)),
        "inside the closed triangle"
    );
    assert!(
        !region.keeps(at(&corners, 10.0, 160.0)),
        "beyond the closing line"
    );
    let tri = [
        at(&corners, 0.0, 0.0),
        at(&corners, 100.0, 0.0),
        at(&corners, 100.0, 200.0),
    ];
    for p in nodes(&traced(&mut s, image, 50)) {
        assert!(convex_distance(&tri, p) <= EDGE_MM, "node {p:?}");
    }
}

#[test]
fn c7_a_region_beyond_the_image_is_clipped_to_it() {
    let (mut s, image, corners) = picture("", "");
    rectangle(&mut s, &corners, (-80.0, -80.0), (100.0, 280.0), false);
    assert!(!s.trace_view().mask_outside_image);
    let quad = [corners[0], corners[1], corners[2], corners[3]];
    for p in nodes(&traced(&mut s, image, 50)) {
        assert!(
            convex_distance(&quad, p) <= EDGE_MM,
            "node {p:?} off the image"
        );
    }
}

#[test]
fn c7_a_region_outside_the_image_is_flagged_and_sends_no_region() {
    let (mut s, _, corners) = picture("", "");
    rectangle(&mut s, &corners, (300.0, 300.0), (360.0, 360.0), false);
    let view = s.trace_view();
    assert_eq!(view.regions, 1);
    assert!(
        view.mask_outside_image,
        "\"0 paths: the mask is outside the image.\""
    );
    // The host dims the Trace button and creates nothing on this flag (`useTrace`); the
    // settings carry no region, so the session sends nothing that could be traced as a mask.
    assert!(
        s.trace_region().is_none(),
        "no region for a mask outside the image"
    );
    assert_eq!(s.trace_mask_json(), "", "no region is sent");
}

fn c8_case(transform: &str) {
    let (mut s, image, corners) = picture(&format!(r#"transform="{transform}""#), "");
    r1(&mut s, &corners);
    assert_eq!(s.trace_view().regions, 1);
    let r1q = [
        at(&corners, 0.0, 0.0),
        at(&corners, 100.0, 0.0),
        at(&corners, 100.0, 200.0),
        at(&corners, 0.0, 200.0),
    ];
    let inside = traced(&mut s, image, 50);
    assert_ne!(nodes(&inside).len(), 0);
    for p in nodes(&inside) {
        assert!(
            convex_distance(&r1q, p) <= EDGE_MM,
            "{transform}: node {p:?} outside R1"
        );
    }
    s.set_trace_mask_outside(true);
    for p in nodes(&traced(&mut s, image, 50)) {
        assert!(
            -convex_distance(&r1q, p) <= EDGE_MM,
            "{transform}: node {p:?} inside R1"
        );
    }
}

/// `0094` criterion 8 and `0114` criterion 13: on a rotated, scaled and skewed image the region
/// lies in the picture's frame.
#[test]
fn c8_rotated_scaled_and_skewed_image_keeps_the_region_on_the_picture() {
    c8_case("translate(10 20) rotate(30) scale(0.6)");
    c8_case("translate(10 20) rotate(30) skewX(12) scale(0.6)");
}

/// `0114` criterion 21 (was `0094` criterion 14): a second image and a path beside the image are
/// no refusal; the images trace, the path is no mask.
#[test]
fn c21_a_second_image_and_a_path_trace_as_the_images() {
    let second = format!(
        r#"<image x="110" y="0" width="20" height="20" xlink:href="data:image/png;base64,{}"/><rect x="0" y="0" width="50" height="100"/>"#,
        base64(&png())
    );
    let mut s = import(&svg("", &second));
    assert_eq!(s.selected_object_count(), 3);
    s.set_tool(Tool::Trace);
    let view = s.trace_view();
    assert_eq!(format!("{:?}", view.selection), "Images");
    assert_eq!(view.images.len(), 2);
    assert!(s.trace_mask_json().is_empty(), "no region is sent");
}

/// `0094` criterion 17 with ADR 0061 §3: on an untouched image Outside is tool state and Inside
/// again on entry; once a region stores the part, its mode is the part's and comes back.
#[test]
fn c17_outside_is_tool_state_until_the_part_is_stored() {
    let (mut s, _, corners) = picture("", "");
    s.set_trace_mask_outside(true);
    assert!(s.trace_mask_outside());
    s.set_tool(Tool::Select);
    s.set_tool(Tool::Trace);
    assert!(!s.trace_mask_outside(), "Inside again on entry");
    r1(&mut s, &corners);
    s.set_trace_mask_outside(true);
    let json: serde_json::Value = serde_json::from_str(&s.trace_mask_json()).unwrap();
    assert_eq!(json["outside"], true);
    s.set_tool(Tool::Select);
    s.set_tool(Tool::Trace);
    assert!(s.trace_mask_outside(), "the stored part keeps Outside");
}

// ---------------------------------------------------------------------------------------------
// Criterion 15 across builds: the native record the browser test compares the editor wasm with
// (`frontend/tests/acceptance_0094_tester.test.ts`). The picture is drawn there by the same
// arithmetic; its FNV-1a hash proves the same pixels.

const NATIVE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/acceptance_0094_tester_native.json"
);

fn fnv(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5_u32, |h, &b| {
        (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
    })
}

/// The worker's `trace_job` as the wasm module runs it, natively.
fn worker_job(rgba: &[u8], n: u32, map: [f64; 6], settings: &str) -> Vec<f64> {
    let region = trace_mask_region(settings);
    match curvyo_editor_wasm::trace_colour_settings(settings) {
        Some(colour) => run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: n,
                    height: n,
                    rgba,
                },
                map: TracePixelMap::new(map),
                mode: TraceMode::Colours(&colour),
                lines: &[],
                mask: region.as_ref().map(|r| r as &dyn Mask),
                shapes: curvyo_editor_wasm::trace_recognise_shapes(settings),
            },
            &mut |_| {},
        ),
        None => run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: n,
                    height: n,
                    rgba,
                },
                map: TracePixelMap::new(map),
                mode: TraceMode::Mono(&trace_settings(settings)),
                lines: &[],
                mask: region.as_ref().map(|r| r as &dyn Mask),
                shapes: curvyo_editor_wasm::trace_recognise_shapes(settings),
            },
            &mut |_| {},
        ),
    }
}

#[test]
fn c15_native_record_for_the_editor_wasm() {
    // R1 drawn on an image rotated by 30 degrees and skewed, as the session makes its region.
    let transform = "translate(10 20) rotate(30) skewX(12) scale(0.6)";
    let (mut s, image, corners) = picture(&format!(r#"transform="{transform}""#), "");
    r1(&mut s, &corners);
    let source = s.trace_source(image, false).unwrap();
    assert_eq!(
        source.rgba,
        draw(M1, PX),
        "the session decodes the tester's pixels"
    );
    let inside = s.trace_mask_json();
    s.set_trace_mask_outside(true);
    let outside = s.trace_mask_json();
    let cases = [
        format!(r#"{{"threshold":50,"mask":{inside}}}"#),
        format!(r#"{{"threshold":50,"shapes":true,"mask":{inside}}}"#),
        format!(
            r##"{{"threshold":128,"output":1,"method":1,"background":"#FFFFFF","colour_tolerance":40,"specks":0,"mask":{outside}}}"##
        ),
    ];
    let colour = draw(support::COLOUR, PX);
    let records: Vec<serde_json::Value> = cases
        .iter()
        .enumerate()
        .map(|(i, settings)| {
            let rgba = if i == 2 { &colour } else { &source.rgba };
            let wire = worker_job(rgba, PX, source.map, settings);
            assert!(
                read_trace_wire(&wire).unwrap().paths > 0,
                "case {i} traces something"
            );
            serde_json::json!({
                "palette": if i == 2 { "colour" } else { "m1" },
                "rgba_fnv32": fnv(rgba),
                "settings": settings,
                "wire": wire,
            })
        })
        .collect();
    let record = serde_json::json!({ "n": PX, "map": source.map, "cases": records });
    if std::env::var_os("CURVYO_UPDATE_0094_NATIVE").is_some()
        || !std::path::Path::new(NATIVE).exists()
    {
        std::fs::write(NATIVE, serde_json::to_string(&record).unwrap()).unwrap();
    }
    let kept: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(NATIVE).unwrap()).unwrap();
    assert_eq!(kept, record, "the native record is this build's");
}
