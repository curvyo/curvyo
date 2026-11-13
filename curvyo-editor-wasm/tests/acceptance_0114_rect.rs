//! `specs/0114-image-trace-mask` slice 1 at the session: the Rectangle tool draws stored regions
//! on M1 (`curvyo-ui-core/tests/support_0094`), a press selects, moves and resizes them, the keys
//! move and delete them, Subtract takes them out, Create paths reads them instead of selected
//! paths, and the mask comes back after save and reopen (criteria 2, 3, 8, 9, 11, 12, 17, 19, 20).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::many_single_char_names
)]

#[path = "../../curvyo-ui-core/tests/support_0094/mod.rs"]
mod support;

use curvyo_document_core::{Document, NodeId, ObjectSnapshot, Point, TraceRegionKind, unpack};
use curvyo_editor_wasm::{
    KeyInput, RowPress, Session, Tool, TraceCreateOutcome, TraceRegionTool, trace_mask_region,
    trace_settings,
};
use image::ImageEncoder;
use support::{M1, base64, draw};
use vectrace::job::{Job as TraceJob, Mode as TraceMode, run as run_trace_job};
use vectrace::mask::Mask;
use vectrace::{PixelMap as TracePixelMap, Raster as TraceRaster};

/// The picture: 300 px over 100 mm.
const PX: u32 = 300;

fn svg(extra: &str) -> String {
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&draw(M1, PX), PX, PX, image::ExtendedColorType::Rgba8)
        .unwrap();
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="300mm" height="300mm" viewBox="-100 -100 300 300"><image id="m1" x="0" y="0" width="100" height="100" preserveAspectRatio="none" xlink:href="data:image/png;base64,{}"/>{extra}</svg>"#,
        base64(&png)
    )
}

fn doc(s: &Session) -> Document {
    unpack(9, &s.pack("t").unwrap()).unwrap()
}

fn image_of(d: &Document) -> NodeId {
    d.object_ids()
        .into_iter()
        .find(|&id| matches!(d.object(id), Some(ObjectSnapshot::Image(_))))
        .unwrap()
}

/// The document point of the picture's mm `(x, y)`.
fn pt(s: &Session, (x, y): (f64, f64)) -> Point {
    let d = doc(s);
    let Some(ObjectSnapshot::Image(i)) = d.object(image_of(&d)) else {
        unreachable!()
    };
    let c = i.corners();
    Point::new(c[0].x + x, c[0].y + y)
}

/// M1 imported (with `extra`), the image alone selected, the Trace tool on.
fn traced(extra: &str) -> (Session, NodeId) {
    let mut s = Session::new(1);
    s.resize_viewport(1200.0, 800.0);
    s.import_svg(svg(extra).as_bytes(), "m1.svg").unwrap();
    let image = image_of(&doc(&s));
    select(&mut s, &[image]);
    s.set_tool(Tool::Trace);
    (s, image)
}

fn drag(s: &mut Session, from: (f64, f64), to: (f64, f64), shift: bool, ctrl: bool) {
    let (a, b) = (pt(s, from), pt(s, to));
    s.pointer_down(a, shift);
    s.pointer_hover(b, shift, ctrl);
    s.pointer_up(b, shift, ctrl);
}

fn key(s: &mut Session, key: &str, shift: bool) {
    s.key_down(KeyInput {
        key,
        shift,
        ctrl: false,
        alt: false,
        repeat: false,
        dom_blocked: false,
        mac: false,
        time_ms: 0.0,
    });
}

/// The stored regions of the image's first part as (kind, subtract, pixel corners).
/// A stored region as (kind, subtract, pixel corners).
type Stored = (TraceRegionKind, bool, Vec<(f64, f64)>);

fn regions(s: &Session, image: NodeId) -> Vec<Stored> {
    let mask = doc(s).trace_mask(image).unwrap();
    mask.parts.first().map_or_else(Vec::new, |part| {
        part.regions
            .iter()
            .map(|r| {
                let px = r
                    .points
                    .iter()
                    .map(|p| (whole_px(p.u), whole_px(p.v)))
                    .collect();
                (r.kind, r.subtract, px)
            })
            .collect()
    })
}

/// A unit coordinate as picture pixels, which must be whole (spec Behaviour 2).
fn whole_px(unit: f64) -> f64 {
    let px = unit * f64::from(PX);
    assert!((px - px.round()).abs() < 1e-9, "{px} is not a whole pixel");
    px.round()
}

fn labels(s: &Session) -> Vec<String> {
    let mut rows: Vec<String> = doc(s)
        .history_page(0, 1000)
        .iter()
        .map(|row| row.label.clone())
        .collect();
    rows.reverse();
    rows
}

/// Selects `ids` as Layers panel presses do: the first plainly, the others added.
fn select(s: &mut Session, ids: &[NodeId]) {
    for (n, id) in ids.iter().enumerate() {
        let press = if n == 0 {
            RowPress::Plain
        } else {
            RowPress::Toggle
        };
        s.press_layer_row(&serde_json::to_string(id).unwrap(), press);
    }
}

/// Criterion 2: a drag with Rectangle pressed stores one rectangle on whole pixels in one step;
/// the region limits the trace; under 2 px draws nothing; Shift from the centre; Ctrl a square.
#[test]
fn c2_a_drag_with_rectangle_stores_a_rectangle_on_whole_pixels() {
    let (mut s, image) = traced("");
    s.set_trace_region_tool(TraceRegionTool::Rect);
    assert_eq!(s.cursor_hint(), "crosshair");
    let steps = s.undo_len();
    drag(&mut s, (0.1, 0.2), (50.05, 99.9), false, false);
    assert_eq!(s.undo_len(), steps + 1, "one step");
    assert_eq!(
        regions(&s, image),
        [(
            TraceRegionKind::Rect,
            false,
            vec![(0.0, 1.0), (150.0, 300.0)]
        )]
    );
    assert!(labels(&s).iter().any(|l| l == "draw_trace_rectangle"));
    let view = s.trace_view();
    assert_eq!(view.regions, 1);
    let region = s.trace_region().expect("a region");
    assert!(region.keeps(pt(&s, (10.0, 50.0))));
    assert!(!region.keeps(pt(&s, (75.0, 50.0))));
    assert!(!s.trace_mask_json().is_empty(), "the worker gets the mask");
    // Under 2 px: nothing.
    drag(&mut s, (60.0, 60.0), (60.3, 60.3), false, false);
    assert_eq!(s.undo_len(), steps + 1);
    // Shift from the centre, Ctrl a square.
    drag(&mut s, (70.0, 70.0), (80.0, 75.0), true, false);
    drag(&mut s, (10.0, 10.0), (20.0, 15.0), false, true);
    let stored = regions(&s, image);
    assert_eq!(stored[1].2, vec![(180.0, 195.0), (240.0, 225.0)]);
    assert_eq!(stored[2].2, vec![(30.0, 30.0), (60.0, 60.0)]);
}

/// Criteria 3 and 17: with no tool a press selects the topmost rectangle, a drag moves it, a
/// handle resizes it, the arrows move it 1 px (Shift 10 px), Delete removes it; one step each.
#[test]
fn c3_a_selected_rectangle_moves_resizes_nudges_and_goes() {
    let (mut s, image) = traced("");
    s.set_trace_region_tool(TraceRegionTool::Rect);
    drag(&mut s, (10.0, 10.0), (40.0, 40.0), false, false);
    s.set_trace_region_tool(TraceRegionTool::None);
    // A drag inside moves it by whole pixels.
    drag(&mut s, (20.0, 20.0), (30.0, 25.0), false, false);
    assert!(s.trace_selected_region().is_some(), "the press selected it");
    assert_eq!(regions(&s, image)[0].2, vec![(60.0, 45.0), (150.0, 135.0)]);
    // The bottom-right handle resizes it.
    drag(&mut s, (50.0, 45.0), (60.0, 50.0), false, false);
    assert_eq!(regions(&s, image)[0].2, vec![(60.0, 45.0), (180.0, 150.0)]);
    key(&mut s, "ArrowRight", false);
    key(&mut s, "ArrowDown", true);
    assert_eq!(regions(&s, image)[0].2, vec![(61.0, 55.0), (181.0, 160.0)]);
    let tail: Vec<String> = labels(&s).into_iter().rev().take(4).collect();
    assert_eq!(
        tail,
        [
            "move_trace_region",
            "move_trace_region",
            "resize_trace_region",
            "move_trace_region"
        ]
    );
    key(&mut s, "Delete", false);
    assert_eq!(regions(&s, image).len(), 0);
    assert_eq!(labels(&s).last().unwrap(), "delete_trace_region");
    assert_eq!(s.tool(), Tool::Trace, "Delete keeps the tool");
}

/// Escape (UX notes §7, steps 5 to 7; reverses slice 1): a pressed region tool is released
/// before a selected region is deselected, then the Trace tool is left.
#[test]
fn escape_releases_the_tool_then_drops_the_region_selection() {
    let (mut s, _) = traced("");
    s.set_trace_region_tool(TraceRegionTool::Rect);
    drag(&mut s, (10.0, 10.0), (40.0, 40.0), false, false);
    s.set_trace_region_tool(TraceRegionTool::None);
    drag(&mut s, (20.0, 20.0), (20.0, 20.0), false, false);
    assert!(s.trace_selected_region().is_some());
    s.set_trace_region_tool(TraceRegionTool::Rect);
    key(&mut s, "Escape", false);
    assert_eq!(
        s.trace_region_tool(),
        TraceRegionTool::None,
        "the tool first"
    );
    assert!(s.trace_selected_region().is_some(), "the selection stays");
    key(&mut s, "Escape", false);
    assert!(s.trace_selected_region().is_none());
    assert_eq!(s.tool(), Tool::Trace);
    key(&mut s, "Escape", false);
    assert_eq!(s.tool(), Tool::Select, "then the tool is left");
}

/// Criterion 8: a rectangle drawn with Subtract is taken out, whichever was drawn first.
#[test]
fn c8_subtract_takes_a_rectangle_out_in_any_order() {
    let (mut s, image) = traced("");
    s.set_trace_region_tool(TraceRegionTool::Rect);
    drag(&mut s, (0.0, 0.0), (50.0, 100.0), false, false);
    s.set_trace_subtract(true);
    drag(&mut s, (0.0, 0.0), (100.0, 50.0), false, false);
    assert!(regions(&s, image)[1].1, "stored as subtracted");
    let one = s.trace_mask_json();
    let (mut t, _) = traced("");
    t.set_trace_region_tool(TraceRegionTool::Rect);
    t.set_trace_subtract(true);
    drag(&mut t, (0.0, 0.0), (100.0, 50.0), false, false);
    t.set_trace_subtract(false);
    drag(&mut t, (0.0, 0.0), (50.0, 100.0), false, false);
    let region = s.trace_region().unwrap();
    assert!(region.keeps(pt(&s, (25.0, 75.0))));
    assert!(!region.keeps(pt(&s, (25.0, 25.0))));
    let other = t.trace_region().unwrap();
    assert_eq!(region.polygons().len(), other.polygons().len());
    let area = |r: &curvyo_ui_core::TraceRegion| -> f64 {
        r.polygons()
            .iter()
            .map(|p| curvyo_geometry_core::signed_area_mm2(p))
            .sum::<f64>()
            .abs()
    };
    assert!((area(&region) - area(&other)).abs() < 1e-6);
    assert!((area(&region) - 50.0 * 50.0).abs() < 0.01);
    assert_ne!(one, "");
}

/// One trace of `image` as the worker runs it, with the session's mask and `settings`.
fn trace_wire(s: &mut Session, image: NodeId, settings: &str) -> Vec<f64> {
    let mask = s.trace_mask_json();
    let settings = if mask.is_empty() {
        settings.to_owned()
    } else {
        format!("{},\"mask\":{mask}}}", settings.trim_end_matches('}'))
    };
    let params = trace_settings(&settings);
    let source = s.trace_source(image, false).unwrap();
    run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: source.width,
                height: source.height,
                rgba: &source.rgba,
            },
            map: TracePixelMap::new(source.map),
            mode: TraceMode::Mono(&params),
            lines: &[],
            mask: trace_mask_region(&settings)
                .as_ref()
                .map(|r| r as &dyn Mask),
            shapes: false,
        },
        &mut |_| {},
    )
}

/// Criterion 9: R1 stored as a region gives the worker exactly the mask `0094` made of the R1
/// mask path (the kernel union of its outline in document mm), so the trace is exactly `0094`
/// criterion 2's (and with Outside, criterion 3's).
#[test]
fn c9_a_stored_r1_is_the_mask_of_the_r1_outline() {
    for outside in [false, true] {
        let (mut stored, image) = traced("");
        stored.set_trace_region_tool(TraceRegionTool::Rect);
        drag(&mut stored, (0.0, 0.0), (50.0, 100.0), false, false);
        stored.set_trace_mask_outside(outside);
        let corner = |p: Point| {
            (
                p,
                curvyo_document_core::Vec2::ZERO,
                curvyo_document_core::Vec2::ZERO,
            )
        };
        let r1 = vec![
            corner(pt(&stored, (0.0, 0.0))),
            corner(pt(&stored, (50.0, 0.0))),
            corner(pt(&stored, (50.0, 100.0))),
            corner(pt(&stored, (0.0, 100.0))),
        ];
        let path_region =
            curvyo_ui_core::TraceRegion::of_operands(&[vec![(r1, true)]], outside).unwrap();
        let want = serde_json::json!({
            "outside": outside,
            "region": path_region.to_numbers(),
        });
        let got: serde_json::Value = serde_json::from_str(&stored.trace_mask_json()).unwrap();
        assert_eq!(got, want, "outside {outside}");
        let settings = r#"{"threshold":50}"#;
        let wire = trace_wire(&mut stored, image, settings);
        assert!(curvyo_ui_core::read_trace_wire(&wire).unwrap().paths > 0);
    }
}

/// Criteria 11 and 19: Create paths reads the stored region, writes one step, changes no region
/// and twice gives byte-identical results; leaving and entering the tool shows the same part.
#[test]
fn c19_create_paths_keeps_the_mask_and_is_deterministic() {
    let run = || {
        let (mut s, image) = traced("");
        s.set_trace_region_tool(TraceRegionTool::Rect);
        drag(&mut s, (0.0, 0.0), (50.0, 100.0), false, false);
        let before = doc(&s).trace_mask(image).unwrap();
        let wire = trace_wire(&mut s, image, r#"{"threshold":50}"#);
        assert!(s.set_trace_result(image, &wire));
        let steps = s.undo_len();
        let outcome = s.create_traced_paths();
        assert!(
            matches!(outcome, TraceCreateOutcome::Created { images: 1, .. }),
            "{outcome:?}"
        );
        assert_eq!(s.undo_len(), steps + 1);
        assert_eq!(
            doc(&s).trace_mask(image).unwrap(),
            before,
            "the regions stay"
        );
        select(&mut s, &[image]);
        s.set_tool(Tool::Trace);
        let parts = s.trace_parts();
        assert_eq!(parts.len(), 1);
        assert!(parts[0].active && parts[0].regions == 1);
        doc(&s).export_json().unwrap()
    };
    assert_eq!(run(), run(), "byte-identical document serialisation");
}

/// Criteria 12 and 17: the mask is back after save and reopen, and undo and redo while the tool
/// is open take the region away and bring it back.
#[test]
fn c12_c17_the_mask_survives_reopen_and_undo_redo_reshow_it() {
    let (mut s, image) = traced("");
    s.set_trace_tool_settings(r#"{"threshold": 50, "smooth": 30, "unknown": 1}"#);
    s.set_trace_region_tool(TraceRegionTool::Rect);
    drag(&mut s, (0.0, 0.0), (50.0, 100.0), false, false);
    let json = s.trace_mask_json();
    let stored = doc(&s).trace_mask(image).unwrap();
    assert_eq!(stored.parts[0].settings["smooth"], serde_json::json!(30));
    assert!(!stored.parts[0].settings.contains_key("unknown"));
    let mut reopened = Session::open(3, &s.pack("t").unwrap()).unwrap();
    reopened.resize_viewport(1200.0, 800.0);
    select(&mut reopened, &[image]);
    reopened.set_tool(Tool::Trace);
    assert_eq!(reopened.trace_view().regions, 1);
    assert_eq!(reopened.trace_mask_json(), json);
    s.undo();
    assert_eq!(s.trace_view().regions, 0);
    assert_eq!(s.trace_mask_json(), "");
    s.redo();
    assert_eq!(s.trace_mask_json(), json);
}

/// Criterion 20: Clear mask removes the active part's regions in one step; the part stays.
#[test]
fn c20_clear_mask_removes_the_regions_in_one_step() {
    let (mut s, image) = traced("");
    s.set_trace_region_tool(TraceRegionTool::Rect);
    drag(&mut s, (0.0, 0.0), (50.0, 100.0), false, false);
    drag(&mut s, (60.0, 0.0), (90.0, 30.0), false, false);
    let steps = s.undo_len();
    s.clear_trace_mask();
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(regions(&s, image).len(), 0);
    assert_eq!(doc(&s).trace_mask(image).unwrap().parts.len(), 1);
    assert!(
        s.trace_parts()[0].whole,
        "the preview shows the whole image"
    );
}

/// Criterion 19 (no write without a gesture): opening the tool, choosing the mode on an untouched
/// image and the bar's settings write nothing.
#[test]
fn c19_no_write_without_a_gesture() {
    let (mut s, image) = traced("");
    let before = s.pack("t").unwrap();
    s.set_trace_mask_outside(true);
    s.set_trace_tool_settings(r#"{"smooth": 3}"#);
    s.set_trace_region_tool(TraceRegionTool::Rect);
    s.set_trace_subtract(true);
    let _ = s.trace_parts();
    assert_eq!(s.pack("t").unwrap(), before);
    // The first gesture stores the mode chosen before it.
    s.set_trace_subtract(false);
    drag(&mut s, (0.0, 0.0), (50.0, 100.0), false, false);
    assert!(doc(&s).trace_mask(image).unwrap().parts[0].outside);
}

/// Criterion 16: deleting the image deletes its mask; one undo brings both back exactly.
#[test]
fn c16_delete_and_undo_bring_the_image_back_with_its_mask() {
    let (mut s, image) = traced("");
    s.set_trace_region_tool(TraceRegionTool::Rect);
    drag(&mut s, (0.0, 0.0), (50.0, 100.0), false, false);
    let before = doc(&s).trace_mask(image).unwrap();
    s.set_tool(Tool::Select);
    select(&mut s, &[image]);
    s.delete_selected();
    assert_eq!(doc(&s).trace_mask(image), None);
    assert!(
        !String::from_utf8(doc(&s).export_json().unwrap())
            .unwrap()
            .contains("trace_mask")
    );
    s.undo();
    assert_eq!(doc(&s).trace_mask(image).unwrap(), before);
}
