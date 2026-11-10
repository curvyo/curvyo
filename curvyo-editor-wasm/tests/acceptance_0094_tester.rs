//! Tester cases for `specs/0094-masked-composite-trace` slice 1 at the session (criteria 1, 2, 5
//! to 8, 12 to 15, 17): an SVG with the tester's M1 picture (`curvyo-ui-core/tests/support_0094`)
//! and mask paths is imported, so image and paths come in selected, as the maker selects them;
//! the trace runs as the worker runs it, from the session's own settings JSON
//! (`trace_settings`, `trace_mask_region`) over the pixels the session decoded. Written from the
//! specification before the implementation was read.

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
use curvyo_editor_wasm::{Session, Tool, TraceCreateOutcome, trace_mask_region, trace_settings};
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

/// R1 over the left half of the picture.
const R1: &str =
    r#"<rect id="r1" x="0" y="0" width="50" height="100" fill="none" stroke="black"/>"#;

// ---------------------------------------------------------------------------------------------

#[test]
fn c1_image_and_path_are_masked_inside_by_default_image_alone_is_not() {
    let mut s = import(&svg("", R1));
    assert_eq!(
        s.selected_object_count(),
        2,
        "image and path come in selected"
    );
    assert!(
        s.trace_available(),
        "the Select bar offers Trace for one image plus a path"
    );
    s.set_tool(Tool::Trace);
    let view = s.trace_view();
    assert_eq!(view.mask.len(), 1, "one mask path");
    assert!(!view.mask_outside_image && !view.mask_failed);
    assert!(!s.trace_mask_outside(), "Inside is selected");
    let json: serde_json::Value = serde_json::from_str(&s.trace_mask_json()).unwrap();
    assert_eq!(json["outside"], false);
    assert!(s.trace_region().is_some());

    let mut plain = import(&svg("", ""));
    plain.set_tool(Tool::Trace);
    assert!(plain.trace_view().mask.is_empty(), "no mask row");
    assert_eq!(plain.trace_mask_json(), "", "the settings carry no mask");
    assert!(plain.trace_region().is_none());
}

#[test]
fn c2_c12_create_paths_inside_r1_keeps_the_mask_path_and_is_one_undo() {
    let mut s = import(&svg("", R1));
    let before = doc(&s);
    let (image, corners) = image_of(&before);
    let mask_id = before
        .object_ids()
        .into_iter()
        .find(|&id| id != image)
        .unwrap();
    let mask_before = before.object(mask_id).unwrap();
    s.set_tool(Tool::Trace);
    let wire = traced(&mut s, image, 50);
    let r1 = [
        at(&corners, 0.0, 0.0),
        at(&corners, 100.0, 0.0),
        at(&corners, 100.0, 200.0),
        at(&corners, 0.0, 200.0),
    ];
    let all = nodes(&wire);
    assert_ne!(all.len(), 0);
    for p in &all {
        assert!(convex_distance(&r1, *p) <= EDGE_MM, "node {p:?} outside R1");
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
        after.object(mask_id),
        Some(mask_before),
        "the mask path is unchanged"
    );
    assert_eq!(
        after.parent_group(mask_id),
        before.parent_group(mask_id),
        "not moved into the group"
    );
    let groups: Vec<NodeId> = after
        .object_ids()
        .into_iter()
        .filter_map(|id| after.parent_group(id))
        .filter(|g| before.parent_group(mask_id) != Some(*g))
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
    assert_eq!(undone.object(mask_id), before.object(mask_id));
}

#[test]
fn c15_the_session_flow_twice_is_identical() {
    let mut s = import(&svg("", R1));
    let (image, _) = image_of(&doc(&s));
    s.set_tool(Tool::Trace);
    let a = traced(&mut s, image, 50);
    let b = traced(&mut s, image, 50);
    assert_eq!(a, b);
    let mut t = import(&svg("", R1));
    let (image_t, _) = image_of(&doc(&t));
    t.set_tool(Tool::Trace);
    assert_eq!(
        traced(&mut t, image_t, 50),
        a,
        "a second session (worker) gives the same numbers"
    );
}

#[test]
fn c5_a_compound_ring_is_a_region_with_a_hole_and_overlaps_are_one_union() {
    // A ring around the disc's centre (25, 50), even-odd, outer 20 mm, hole 6 mm.
    let ring = r#"<path fill-rule="evenodd" d="M 5 50 A 20 20 0 1 0 45 50 A 20 20 0 1 0 5 50 Z M 19 50 A 6 6 0 1 0 31 50 A 6 6 0 1 0 19 50 Z"/>"#;
    let mut s = import(&svg("", ring));
    let (image, corners) = image_of(&doc(&s));
    s.set_tool(Tool::Trace);
    let region = s.trace_region().expect("a region");
    let centre = at(&corners, 50.0, 100.0);
    assert!(!region.keeps(centre), "the hole is no part of the region");
    assert!(region.keeps(at(&corners, 50.0, 70.0)), "the ring is");
    let wire = traced(&mut s, image, 50);
    for p in nodes(&wire) {
        let r = (p.x - centre.x).hypot(p.y - centre.y);
        assert!(
            (6.0 - EDGE_MM..=20.0 + EDGE_MM).contains(&r),
            "node {p:?} at {r} mm"
        );
    }

    let two = r#"<rect x="10" y="30" width="30" height="40"/><rect x="30" y="40" width="40" height="20"/>"#;
    let mut s = import(&svg("", two));
    let (_, corners) = image_of(&doc(&s));
    s.set_tool(Tool::Trace);
    assert_eq!(s.trace_view().mask.len(), 2);
    let region = s.trace_region().unwrap();
    for (x, y) in [(30.0, 70.0), (70.0, 100.0), (130.0, 110.0)] {
        assert!(
            region.keeps(at(&corners, x, y)),
            "({x}, {y}) px is in the union"
        );
    }
    assert!(!region.keeps(at(&corners, 30.0, 150.0)));
}

#[test]
fn c6_an_open_path_is_closed_by_a_straight_line() {
    let open = r#"<path fill="none" stroke="black" d="M 0 0 L 50 0 L 50 100"/>"#;
    let mut s = import(&svg("", open));
    let (image, corners) = image_of(&doc(&s));
    s.set_tool(Tool::Trace);
    let view = s.trace_view();
    assert_eq!(view.mask.len(), 1, "the open path is a mask path");
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
fn c7_a_mask_beyond_the_image_is_clipped_to_it() {
    let big = r#"<rect x="-40" y="-40" width="90" height="180"/>"#;
    let mut s = import(&svg("", big));
    let (image, corners) = image_of(&doc(&s));
    s.set_tool(Tool::Trace);
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
fn c7_a_mask_outside_the_image_is_flagged_and_sends_no_region() {
    let far = r#"<rect x="150" y="150" width="30" height="30"/>"#;
    let mut s = import(&svg("", far));
    s.set_tool(Tool::Trace);
    let view = s.trace_view();
    assert_eq!(view.mask.len(), 1);
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
    let r1 = format!(
        r#"<rect x="0" y="0" width="50" height="100" fill="none" stroke="black" transform="{transform}"/>"#
    );
    let mut s = import(&svg(&format!(r#"transform="{transform}""#), &r1));
    let (image, corners) = image_of(&doc(&s));
    s.set_tool(Tool::Trace);
    assert_eq!(s.trace_view().mask.len(), 1);
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

#[test]
fn c8_rotated_scaled_and_skewed_image_with_the_same_mask_transform() {
    c8_case("translate(10 20) rotate(30) scale(0.6)");
    c8_case("translate(10 20) rotate(30) skewX(12) scale(0.6)");
}

/// Terms ("Mask path"): "A group counts as the paths in it", and Frage 130 A: the image and the
/// paths are selected together with the Select tool (re-test 2026-11-05: fixed, no longer ignored).
#[test]
fn grouped_mask_paths_count_as_the_paths_in_the_group() {
    let group = r#"<g><rect x="5" y="5" width="20" height="20"/><rect x="60" y="60" width="20" height="20"/></g>"#;
    let mut s = import(&svg("", group));
    let (_, corners) = image_of(&doc(&s));
    s.set_tool(Tool::Trace);
    let view = s.trace_view();
    assert!(!view.mask.is_empty(), "a group gives mask paths");
    assert!(!view.mask_failed && !view.mask_outside_image);
    let region = s.trace_region().expect("a region");
    assert!(
        region.keeps(at(&corners, 30.0, 30.0)),
        "the first rectangle"
    );
    assert!(
        region.keeps(at(&corners, 140.0, 140.0)),
        "the second rectangle"
    );
    assert!(!region.keeps(at(&corners, 100.0, 100.0)), "between them");
}

/// Re-test 2026-11-05: a group inside a group still counts as its paths, at any depth.
#[test]
fn a_nested_group_counts_as_its_leaf_paths() {
    let group = r#"<g><rect x="5" y="5" width="20" height="20"/><g><rect x="60" y="60" width="20" height="20"/></g></g>"#;
    let mut s = import(&svg("", group));
    let (image, corners) = image_of(&doc(&s));
    s.set_tool(Tool::Trace);
    let view = s.trace_view();
    assert_eq!(format!("{:?}", view.selection), "Images");
    assert_eq!(view.mask.len(), 2, "both leaves, the nested one too");
    let region = s.trace_region().expect("a region");
    assert!(region.keeps(at(&corners, 30.0, 30.0)));
    assert!(
        region.keeps(at(&corners, 140.0, 140.0)),
        "the nested rectangle"
    );
    assert!(!region.keeps(at(&corners, 100.0, 100.0)));
    // Every node of the masked trace lies in one of the two rectangles.
    let wire = traced(&mut s, image, 50);
    let boxes = [
        [
            at(&corners, 10.0, 10.0),
            at(&corners, 50.0, 10.0),
            at(&corners, 50.0, 50.0),
            at(&corners, 10.0, 50.0),
        ],
        [
            at(&corners, 120.0, 120.0),
            at(&corners, 160.0, 120.0),
            at(&corners, 160.0, 160.0),
            at(&corners, 120.0, 160.0),
        ],
    ];
    for p in nodes(&wire) {
        let d = boxes
            .iter()
            .map(|b| convex_distance(b, p))
            .fold(f64::INFINITY, f64::min);
        assert!(
            d <= EDGE_MM,
            "node {p:?} lies {d} mm outside both rectangles"
        );
    }
}

/// Criterion 14 with groups (re-test item 1): a group holding an image next to its paths is
/// not a group of mask paths (a text takes the same leaf rule); the image with it is the mixed row, no region.
#[test]
fn a_group_holding_an_image_is_the_mixed_row() {
    let group = format!(
        r#"<g><rect x="5" y="5" width="20" height="20"/><image x="110" y="0" width="20" height="20" xlink:href="data:image/png;base64,{}"/></g>"#,
        base64(&png())
    );
    let mut s = import(&svg("", &group));
    s.set_tool(Tool::Trace);
    let view = s.trace_view();
    assert_eq!(format!("{:?}", view.selection), "Mixed");
    assert_eq!(view.mask.len(), 0, "no mask paths");
    assert!(s.trace_mask_json().is_empty(), "no region is sent");
}

#[test]
fn c14_a_second_image_with_a_mask_path_is_refused() {
    let second = format!(
        r#"<image x="110" y="0" width="20" height="20" xlink:href="data:image/png;base64,{}"/>{R1}"#,
        base64(&png())
    );
    let mut s = import(&svg("", &second));
    assert_eq!(s.selected_object_count(), 3);
    s.set_tool(Tool::Trace);
    let view = s.trace_view();
    assert_eq!(
        format!("{:?}", view.selection),
        "Mixed",
        "two images and a path: \"Use Object to path for text; one image at a time.\""
    );
    assert!(s.trace_mask_json().is_empty(), "no region is sent");
}

#[test]
fn c13_shift_click_adds_and_removes_a_mask_path_empty_canvas_changes_nothing() {
    // The path lies over the image's right part and reaches beyond it, filled, so a click
    // inside it hits the path, not the image.
    let path = r#"<rect x="60" y="10" width="60" height="30" fill="black"/>"#;
    let mut s = import(&svg("", path));
    let (_, corners) = image_of(&doc(&s));
    s.set_tool(Tool::Trace);
    assert_eq!(s.trace_view().mask.len(), 1);
    let on_path = at(&corners, 220.0, 50.0);
    let empty = at(&corners, 400.0, 380.0);
    let shift_click = |s: &mut Session, p: Point| {
        s.pointer_down(p, true);
        s.pointer_up(p, true, false);
    };
    shift_click(&mut s, on_path);
    assert!(
        s.trace_view().mask.is_empty(),
        "Shift+click removes the mask path"
    );
    assert_eq!(s.trace_mask_json(), "");
    shift_click(&mut s, on_path);
    assert_eq!(s.trace_view().mask.len(), 1, "Shift+click adds it again");
    let images = s.trace_view().images;
    shift_click(&mut s, empty);
    let view = s.trace_view();
    assert_eq!(view.mask.len(), 1, "empty canvas changes nothing");
    assert_eq!(view.images, images, "the image stays chosen");
}

#[test]
fn c17_outside_is_forgotten_when_the_tool_is_left() {
    let mut s = import(&svg("", R1));
    s.set_tool(Tool::Trace);
    s.set_trace_mask_outside(true);
    assert!(s.trace_mask_outside());
    let json: serde_json::Value = serde_json::from_str(&s.trace_mask_json()).unwrap();
    assert_eq!(json["outside"], true);
    s.set_tool(Tool::Select);
    s.set_tool(Tool::Trace);
    assert!(!s.trace_mask_outside(), "Inside again on entry");
    let json: serde_json::Value = serde_json::from_str(&s.trace_mask_json()).unwrap();
    assert_eq!(json["outside"], false);
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
    // R1 on an image rotated by 30 degrees and skewed, as the session makes its region.
    let transform = "translate(10 20) rotate(30) skewX(12) scale(0.6)";
    let r1 = format!(r#"<rect x="0" y="0" width="50" height="100" transform="{transform}"/>"#);
    let mut s = import(&svg(&format!(r#"transform="{transform}""#), &r1));
    let (image, _) = image_of(&doc(&s));
    s.set_tool(Tool::Trace);
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
