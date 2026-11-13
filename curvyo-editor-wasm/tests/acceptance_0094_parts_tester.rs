//! Tester cases for `specs/0094-masked-composite-trace` slice 2 at the session (criteria 18, 19,
//! 21, 22, 24, 27 to 29, 31 to 33 and 42; the cut-out Proposal), as `specs/0114-image-trace-mask`
//! keeps them: the tester's M1 picture (`curvyo-ui-core/tests/support_0094`) is imported, the
//! parts are built with Add part, chip picks and rectangles drawn with the Trace tool's Rectangle
//! (they replaced the mask paths, Shift+click and the pick mode), and each part is traced as the
//! worker traces it, from the session's part pixels (`trace_part_source`) and its clip
//! (`trace_parts` mask JSON), then joined by `set_trace_joined`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]

#[path = "../../curvyo-ui-core/tests/support_0094/mod.rs"]
mod support;

mod cpu_time;

use curvyo_document_core::{Document, NodeId, ObjectSnapshot, Point, unpack};
use curvyo_editor_wasm::{
    Session, Tool, TraceCreateOutcome, TraceRegionTool, trace_mask_region, trace_settings,
};
use curvyo_ui_core::{TraceRegion, read_trace_wire};
use image::ImageEncoder;
use kurbo::{BezPath, PathEl};
use support::{M1, base64, draw};
use vectrace::job::{Job as TraceJob, Mode as TraceMode, run as run_trace_job};
use vectrace::mask::Mask;
use vectrace::{PixelMap as TracePixelMap, Raster as TraceRaster};

/// The picture's size: 300 px over 100 mm.
const PX: u32 = 300;

fn png_of(rgba: &[u8], n: u32) -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(rgba, n, n, image::ExtendedColorType::Rgba8)
        .unwrap();
    out
}

/// An SVG in mm with `rgba` (`n` px) at 0..100 mm and `extra` after it.
fn svg_of(rgba: &[u8], n: u32, extra: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="300mm" height="300mm" viewBox="-100 -100 300 300"><image id="m1" x="0" y="0" width="100" height="100" preserveAspectRatio="none" xlink:href="data:image/png;base64,{}"/>{extra}</svg>"#,
        base64(&png_of(rgba, n))
    )
}

fn svg(extra: &str) -> String {
    svg_of(&draw(M1, PX), PX, extra)
}

fn import(svg: &str) -> Session {
    let mut s = Session::new(1);
    s.resize_viewport(1200.0, 800.0);
    let outcome = s.import_svg(svg.as_bytes(), "parts.svg").unwrap();
    assert!(
        format!("{outcome:?}").starts_with("Imported"),
        "{outcome:?}"
    );
    s
}

fn doc(s: &Session) -> Document {
    unpack(9, &s.pack("t").unwrap()).unwrap()
}

fn image_of(d: &Document) -> NodeId {
    d.object_ids()
        .into_iter()
        .find(|&id| matches!(d.object(id), Some(ObjectSnapshot::Image(_))))
        .expect("an image")
}

fn parts(s: &Session) -> serde_json::Value {
    let v: Vec<serde_json::Value> = s
        .trace_parts()
        .iter()
        .enumerate()
        .map(|(i, p)| {
            serde_json::json!({
                "i": i, "id": p.id, "regions": p.regions, "outside": p.outside, "whole": p.whole,
                "muted": p.muted, "active": p.active, "contributes": p.contributes,
            })
        })
        .collect();
    serde_json::Value::Array(v)
}

/// Rectangles in the picture's mm (its first corner at 0, 0): A over the left half (x -20..50),
/// B over the right half (x 50..120), C over the top band.
type MmBox = ((f64, f64), (f64, f64));
const A: MmBox = ((-20.0, 0.0), (50.0, 100.0));
const B: MmBox = ((50.0, 0.0), (120.0, 100.0));
const C: MmBox = ((0.0, -20.0), (100.0, 25.0));
/// D and E inside the picture, away from its edges.
const D: MmBox = ((12.0, 14.0), (42.0, 54.0));
const E: MmBox = ((55.0, 40.0), (88.0, 85.0));

/// The document point of the SVG's mm `(x, y)`: the picture's first corner is the SVG's origin
/// (the import may move the drawing).
fn pt(s: &Session, (x, y): (f64, f64)) -> Point {
    let d = doc(s);
    let image = image_of(&d);
    let Some(ObjectSnapshot::Image(i)) = d.object(image) else {
        unreachable!()
    };
    let c = i.corners();
    Point::new(c[0].x + x, c[0].y + y)
}

/// Draws the rectangle `area` (the picture's mm) into the active part with the Rectangle tool.
fn region(s: &mut Session, area: MmBox) {
    s.set_trace_region_tool(TraceRegionTool::Rect);
    let (from, to) = (pt(s, area.0), pt(s, area.1));
    s.pointer_down(from, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
    s.set_trace_region_tool(TraceRegionTool::None);
}

/// Imports M1, Trace tool on, region A in part 1.
fn part_a() -> (Session, NodeId) {
    let mut s = import(&svg(""));
    let image = image_of(&doc(&s));
    s.set_tool(Tool::Trace);
    region(&mut s, A);
    assert_eq!(s.trace_view().regions, 1, "A is the mask");
    (s, image)
}

// ---------------------------------------------------------------------------------------------
// The worker's jobs and the join.

/// Part `index` traced as the worker traces it with `settings` (no mask key; the part's clip is
/// added from `trace_parts`), from the session's part pixels when `cut`, else the whole picture.
fn part_wire(s: &mut Session, image: NodeId, index: usize, settings: &str, cut: bool) -> Vec<f64> {
    let mask = s.trace_part_mask_json(index);
    let settings = if mask.is_empty() {
        settings.to_owned()
    } else {
        format!("{},\"mask\":{mask}}}", settings.trim_end_matches('}'))
    };
    let params = trace_settings(&settings);
    let source = if cut {
        let auto = params.upscale == vectrace::Upscale::Auto;
        s.trace_part_source(image, index, false, params.specks.as_mm(), auto)
    } else {
        s.trace_source(image, false)
    }
    .expect("pixels");
    if let Some(colour) = curvyo_editor_wasm::trace_colour_settings(&settings) {
        return run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: source.width,
                    height: source.height,
                    rgba: &source.rgba,
                },
                map: TracePixelMap::new(source.map),
                mode: TraceMode::Colours(&colour),
                lines: &[],
                mask: trace_mask_region(&settings)
                    .as_ref()
                    .map(|r| r as &dyn Mask),
                shapes: false,
            },
            &mut |_| {},
        );
    }
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

/// Every part traced with its `settings` and joined; `full` keeps it for Create paths. The
/// joined wire's header.
fn join(s: &mut Session, image: NodeId, settings: &[&str], full: bool) -> Vec<f64> {
    let views = s.trace_parts();
    assert_eq!(views.len(), settings.len());
    let wires: Vec<Option<Vec<f64>>> = views
        .iter()
        .enumerate()
        .map(|(i, v)| {
            v.contributes
                .then(|| part_wire(s, image, i, settings[i], true))
        })
        .collect();
    let refs: Vec<Option<&[f64]>> = wires.iter().map(Option::as_deref).collect();
    s.set_trace_joined(image, &refs, full)
        .expect("the wires are results")
}

// Geometry (this crate has no geometry kernel dependency; areas and differences by kurbo) -----

fn inside_paths(paths: &[(BezPath, kurbo::Rect)], p: kurbo::Point) -> bool {
    use kurbo::Shape;
    paths
        .iter()
        .filter(|(_, b)| b.contains(p))
        .map(|(path, _)| path.winding(p))
        .sum::<i32>()
        != 0
}

fn with_boxes(paths: &[BezPath]) -> Vec<(BezPath, kurbo::Rect)> {
    use kurbo::Shape;
    paths
        .iter()
        .map(|p| (p.clone(), p.bounding_box()))
        .collect()
}

/// The painted area of `paths` (nonzero, disjoint shapes with their holes), mm².
fn painted(paths: &[BezPath]) -> f64 {
    paths.iter().map(kurbo::Shape::area).sum::<f64>().abs()
}

/// Each outline of `got` matched to the outline of `want` with the nearest box: the sum of the
/// area differences and of the box moves times the box perimeter (a bound of the symmetric
/// difference of a matched pair that only moved), relative to `want`'s area; `None` when an
/// outline has no partner within 0.05 mm.
fn matched_ratio(got: &[BezPath], want: &[BezPath]) -> Option<f64> {
    use kurbo::Shape;
    let w = with_boxes(want);
    let mut used = vec![false; w.len()];
    let mut diff = 0.0;
    for g in got {
        let gb = g.bounding_box();
        let off = |b: &kurbo::Rect| {
            (gb.x0 - b.x0)
                .abs()
                .max((gb.y0 - b.y0).abs())
                .max((gb.x1 - b.x1).abs())
                .max((gb.y1 - b.y1).abs())
        };
        let (k, d) = w
            .iter()
            .enumerate()
            .filter(|(k, _)| !used[*k])
            .map(|(k, (_, b))| (k, off(b)))
            .min_by(|a, b| a.1.total_cmp(&b.1))?;
        if d > 0.05 {
            return None;
        }
        used[k] = true;
        diff += (g.area() - w[k].0.area()).abs() + d * 2.0 * (gb.width() + gb.height());
    }
    Some(diff / painted(want).max(1e-9))
}

fn covers(paths: &[BezPath], p: Point) -> bool {
    inside_paths(&with_boxes(paths), kurbo::Point::new(p.x, p.y))
}

fn outlines(wire: &[f64]) -> Vec<BezPath> {
    read_trace_wire(wire).expect("a readable wire").outlines
}

// ---------------------------------------------------------------------------------------------
// Criteria 18, 19, 42: the Parts row's state.

#[test]
fn c18_one_masked_image_has_chip_1_active_and_add_part_stops_at_8() {
    let (mut s, _) = part_a();
    let p = parts(&s);
    assert_eq!(p.as_array().unwrap().len(), 1, "{p}");
    assert_eq!(p[0]["active"], true);
    assert_eq!(p[0]["regions"], 1);
    assert_eq!(p[0]["whole"], false);
    for n in 2..=8 {
        assert!(s.add_trace_part().is_some(), "part {n} is added");
    }
    assert_eq!(s.trace_part_count(), 8);
    assert!(s.add_trace_part().is_none(), "a 9th part is refused");
    assert_eq!(s.trace_part_count(), 8);
    let ids: Vec<u64> = s.trace_parts().iter().map(|p| u64::from(p.id)).collect();
    let mut sorted = ids.clone();
    sorted.dedup();
    assert_eq!(sorted.len(), 8, "every part has its own id: {ids:?}");
}

#[test]
fn c18_an_image_alone_has_one_whole_part() {
    let mut s = import(&svg(""));
    s.set_tool(Tool::Trace);
    let p = parts(&s);
    assert_eq!(p.as_array().unwrap().len(), 1, "{p}");
    assert_eq!(p[0]["whole"], true);
    assert_eq!(p[0]["contributes"], true);
    assert_eq!(p[0]["muted"], false);
}

#[test]
fn c19_add_part_is_active_muted_and_takes_the_next_paths_a_chip_pick_loads_its_mask() {
    let (mut s, image) = part_a();
    assert!(s.add_trace_part().is_some());
    let p = parts(&s);
    assert_eq!(p[1]["active"], true, "{p}");
    assert_eq!(p[0]["active"], false);
    assert_eq!(p[1]["regions"], 0);
    assert_eq!(p[1]["muted"], true, "a new part without paths is muted");
    assert_eq!(p[1]["contributes"], false);
    assert_eq!(s.selected_object_count(), 1, "the image alone is selected");
    assert_eq!(s.trace_view().regions, 0);
    // A rectangle drawn now is part 2's.
    region(&mut s, B);
    let p = parts(&s);
    assert_eq!(p[1]["regions"], 1, "{p}");
    assert_eq!(p[1]["muted"], false);
    assert_eq!(p[0]["regions"], 1, "part 1 keeps A");
    assert_eq!(s.trace_view().regions, 1);
    // A chip pick: part 1 active with its region; the selection stays the image.
    assert!(s.set_trace_active_part(0));
    assert_eq!(s.trace_active_part(), 0);
    assert_eq!(s.selected_object_count(), 1, "the image alone");
    assert_eq!(s.trace_view().images, vec![image]);
    assert!(
        s.trace_region()
            .is_some_and(|r| r.keeps(pt(&s, (10.0, 50.0))))
    );
    assert!(s.set_trace_active_part(1));
    assert!(
        s.trace_region()
            .is_some_and(|r| r.keeps(pt(&s, (90.0, 50.0))))
    );
    assert!(!s.set_trace_active_part(5), "no such part");
}

#[test]
fn c19_mask_mode_is_per_part() {
    let (mut s, _) = part_a();
    s.set_trace_mask_outside(true);
    assert!(s.add_trace_part().is_some());
    assert!(!s.trace_mask_outside(), "a new part starts Inside");
    region(&mut s, B);
    let p = parts(&s);
    assert_eq!(p[0]["outside"], true, "{p}");
    assert_eq!(p[1]["outside"], false);
    assert!(s.set_trace_active_part(0));
    assert!(
        s.trace_mask_outside(),
        "part 1's Outside comes back with its chip"
    );
}

/// Criterion 42 with regions: drawing and Clear mask touch the active part only; Add part and a
/// chip pick keep the pressed region tool (`0114` UX notes §9).
#[test]
fn c42_drawing_and_clear_mask_touch_the_active_part_only_and_keep_the_tool() {
    let (mut s, image) = part_a();
    assert!(s.add_trace_part().is_some());
    region(&mut s, B);
    region(&mut s, C);
    let p = parts(&s);
    assert_eq!(p[1]["regions"], 2, "the rectangles go to part 2: {p}");
    assert_eq!(p[0]["regions"], 1, "part 1 keeps A only");
    s.set_trace_region_tool(TraceRegionTool::Rect);
    assert!(s.set_trace_active_part(0));
    assert_eq!(
        s.trace_region_tool(),
        TraceRegionTool::Rect,
        "a chip pick keeps the tool"
    );
    s.set_trace_region_tool(TraceRegionTool::None);
    region(&mut s, C);
    let p = parts(&s);
    assert_eq!(p[0]["regions"], 2, "C drawn into part 1: {p}");
    assert_eq!(p[1]["regions"], 2, "part 2 unchanged");
    assert!(s.set_trace_active_part(1));
    s.clear_trace_mask();
    let p = parts(&s);
    assert_eq!(p[1]["regions"], 0, "Clear mask empties part 2: {p}");
    assert_eq!(p[0]["regions"], 2, "part 1 untouched");
    assert_eq!(
        doc(&s).trace_mask(image).unwrap().parts.len(),
        2,
        "the part stays"
    );
}

// ---------------------------------------------------------------------------------------------
// Criteria 27, 28, 29.

/// Criterion 27 with regions: a part's region deleted while the tool is open (selected and Delete)
/// leaves the part without an error; part 1 without regions is the whole image again.
#[test]
fn c27_a_region_deleted_while_the_tool_is_open_leaves_its_part_without_an_error() {
    let (mut s, image) = part_a();
    assert!(s.add_trace_part().is_some());
    region(&mut s, B);
    let delete_at = |s: &mut Session, at: (f64, f64)| {
        let p = pt(s, at);
        s.pointer_down(p, false);
        s.pointer_up(p, false, false);
        s.key_down(curvyo_editor_wasm::KeyInput {
            key: "Delete",
            shift: false,
            ctrl: false,
            alt: false,
            repeat: false,
            dom_blocked: false,
            mac: false,
            time_ms: 0.0,
        });
    };
    delete_at(&mut s, (110.0, 50.0));
    assert_eq!(s.trace_part_count(), 2, "the part stays");
    let p = parts(&s);
    assert_eq!(p[1]["regions"], 0, "{p}");
    assert_eq!(p[1]["muted"], true);
    assert_eq!(p[1]["contributes"], false);
    let view = s.trace_view();
    assert!(!view.mask_failed && !view.mask_outside_image, "no error");
    assert_eq!(view.images, vec![image], "the image is still traced");
    // The join with a muted part is part 1's trace.
    let head = join(
        &mut s,
        image,
        &[r#"{"threshold":50}"#, r#"{"threshold":50}"#],
        false,
    );
    assert!(read_trace_wire(&head).is_some() || !head.is_empty());
    // Part 1's A deleted: the first part covers the whole image again.
    assert!(s.set_trace_active_part(0));
    delete_at(&mut s, (-10.0, 50.0));
    let p = parts(&s);
    assert_eq!(p[0]["regions"], 0, "{p}");
    assert_eq!(
        p[0]["whole"], true,
        "the first part is the whole image again"
    );
    assert_eq!(p[0]["contributes"], true);
}

#[test]
fn c28_removing_a_part_renumbers_and_one_part_cannot_be_removed() {
    let (mut s, _) = part_a();
    assert!(!s.remove_trace_part(0), "one part cannot be removed");
    let first = s.add_trace_part().unwrap();
    region(&mut s, B);
    let third = s.add_trace_part().unwrap();
    region(&mut s, C);
    let ids: Vec<u32> = s.trace_parts().iter().map(|p| p.id).collect();
    assert_eq!(ids[1..], [first, third]);
    assert!(s.remove_trace_part(1), "part 2 goes");
    let views = s.trace_parts();
    assert_eq!(views.len(), 2);
    assert_eq!(views[1].id, third, "part 3 is now part 2");
    assert_eq!(views[1].regions, 1);
    assert!(
        views.iter().filter(|v| v.active).count() == 1,
        "one active part"
    );
    assert!(!s.remove_trace_part(7), "no such part");
}

/// Replaced by `specs/0114-image-trace-mask` criterion 19 and Behaviour 9 (customer 2026-11-12,
/// ADR 0061 accepted 2026-11-13): the parts are stored on the image, so leaving the tool keeps
/// them and entering it again shows them with part 1 active; Add part is one step each.
#[test]
fn c29_leaving_the_tool_keeps_the_stored_parts_with_part_one_active() {
    let (mut s, _) = part_a();
    let before = s.pack("t").unwrap();
    assert!(s.add_trace_part().is_some());
    region(&mut s, B);
    assert!(s.add_trace_part().is_some());
    assert_eq!(s.trace_part_count(), 3);
    assert_ne!(
        s.pack("t").unwrap(),
        before,
        "the parts are in the document"
    );
    for round in 0..2 {
        s.set_tool(Tool::Select);
        s.set_tool(Tool::Trace);
        assert_eq!(s.trace_part_count(), 3, "the parts after leaving {round}");
        let p = parts(&s);
        assert_eq!(p[0]["active"], true, "part 1 is active on entry");
    }
}

// ---------------------------------------------------------------------------------------------
// Criteria 21, 22, 24 and 33 through the session's join.

#[test]
fn c24_create_paths_with_parts_is_one_group_one_step_and_one_undo() {
    let (mut s, image) = part_a();
    assert!(s.add_trace_part().is_some());
    region(&mut s, B);
    let before = doc(&s);
    let undo_before = s.undo_len();
    join(
        &mut s,
        image,
        &[
            r#"{"threshold":50,"smooth":20}"#,
            r#"{"threshold":50,"smooth":80}"#,
        ],
        true,
    );
    let outcome = s.create_traced_paths();
    assert!(
        matches!(outcome, TraceCreateOutcome::Created { images: 1, .. }),
        "{outcome:?}"
    );
    assert_eq!(s.undo_len(), undo_before + 1, "one history step");
    let after = doc(&s);
    assert_eq!(
        after.trace_mask(image),
        before.trace_mask(image),
        "the regions are unchanged"
    );
    let groups: Vec<NodeId> = after
        .object_ids()
        .into_iter()
        .filter(|id| !before.object_ids().contains(id))
        .filter_map(|id| after.parent_group(id))
        .fold(Vec::new(), |mut v, g| {
            if !v.contains(&g) {
                v.push(g);
            }
            v
        });
    assert_eq!(groups.len(), 1, "one group: {groups:?}");
    let name = after
        .read_nodes()
        .into_iter()
        .find(|n| n.id == groups[0])
        .and_then(|n| n.registers.name);
    assert!(
        name.as_deref().is_some_and(|n| n.starts_with("Trace of")),
        "{name:?}"
    );
    // C22 through the session: the join of the two halves at different Smooth is one path.
    let new: Vec<NodeId> = after
        .object_ids()
        .into_iter()
        .filter(|id| !before.object_ids().contains(id))
        .collect();
    assert_eq!(
        new.len(),
        1,
        "disc, bar and square are one compound path: {new:?}"
    );
    s.undo();
    let undone = doc(&s);
    assert_eq!(
        undone.object_ids(),
        before.object_ids(),
        "one undo restores"
    );
    assert_eq!(undone.object(image), before.object(image), "and the image");
}

#[test]
fn c21_c33_session_join_matches_the_clip_rules_and_one_part_is_slice_1() {
    let (mut s, image) = part_a();
    // One part: the joined preview is the slice 1 masked trace.
    let one = part_wire(&mut s, image, 0, r#"{"threshold":50}"#, false);
    let mask = s.trace_mask_json();
    let slice1 = {
        let settings = format!(r#"{{"threshold":50,"mask":{mask}}}"#);
        let src = s.trace_source(image, false).unwrap();
        run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: src.width,
                    height: src.height,
                    rgba: &src.rgba,
                },
                map: TracePixelMap::new(src.map),
                mode: TraceMode::Mono(&trace_settings(&settings)),
                lines: &[],
                mask: trace_mask_region(&settings)
                    .as_ref()
                    .map(|r| r as &dyn Mask),
                shapes: false,
            },
            &mut |_| {},
        )
    };
    assert_eq!(one, slice1, "one part sends slice 1's mask");
    let head = join(&mut s, image, &[r#"{"threshold":50}"#], true);
    assert_eq!(
        head[..7],
        slice1[..7],
        "one part's joined header is slice 1's"
    );
    // Overlap: part 1 = A (x < 50) at 150, part 2 = C (the top band y < 25, over A's top and
    // the grey square at 90, 10) at 50: part 2 decides the grey (paper at 50), and part 1 is cut
    // at C's edge.
    assert!(s.add_trace_part().is_some());
    region(&mut s, C);
    let views = s.trace_parts();
    assert!(views.iter().all(|v| v.contributes), "{views:?}");
    let a = part_wire(&mut s, image, 0, r#"{"threshold":150}"#, true);
    let c = part_wire(&mut s, image, 1, r#"{"threshold":50}"#, true);
    let o = pt(&s, (0.0, 0.0));
    let grey = pt(&s, (90.0, 10.0));
    assert!(!covers(&outlines(&c), grey), "part 2 at 50 leaves the grey");
    // A's clip (A minus C): no node of part 1 lies in C's band (y < 25 mm).
    for p in outlines(&a) {
        for el in p.elements() {
            if let PathEl::MoveTo(q) | PathEl::LineTo(q) | PathEl::CurveTo(_, _, q) = *el {
                assert!(
                    q.y - o.y >= 25.0 - 0.01 && q.x - o.x <= 50.0 + 0.01,
                    "part 1 is cut at C's edge: {q:?}"
                );
            }
        }
    }
}

#[test]
fn the_session_join_is_deterministic() {
    let run = || {
        let (mut s, image) = part_a();
        assert!(s.add_trace_part().is_some());
        region(&mut s, C);
        let head = join(
            &mut s,
            image,
            &[
                r#"{"threshold":60,"smooth":30}"#,
                r#"{"threshold":90,"smooth":70}"#,
            ],
            true,
        );
        let wires: Vec<Vec<f64>> = (0..2)
            .map(|i| {
                part_wire(
                    &mut s,
                    image,
                    i,
                    [
                        r#"{"threshold":60,"smooth":30}"#,
                        r#"{"threshold":90,"smooth":70}"#,
                    ][i],
                    true,
                )
            })
            .collect();
        (head, wires)
    };
    assert_eq!(run(), run());
}

// ---------------------------------------------------------------------------------------------
// The cut-out Proposal: a part traced from its cut-out matches its whole-picture trace within
// 0.1 %, across Ignore below and Inside / Outside; and four quarter parts are cut at all.

/// A busier picture: M1 plus a grid of 3 mm discs every 8 mm and thin 0.4 mm lines, 400 px.
fn busy(n: u32) -> Vec<u8> {
    let mut rgba = draw(M1, n);
    let mm = 100.0 / f64::from(n);
    for y in 0..n {
        for x in 0..n {
            let (u, v) = ((f64::from(x) + 0.5) * mm, (f64::from(y) + 0.5) * mm);
            let (cx, cy) = ((u / 8.0).floor() * 8.0 + 4.0, (v / 8.0).floor() * 8.0 + 4.0);
            let disc = (u - cx).hypot(v - cy) <= 1.5;
            let line = (v - 70.3).abs() <= 0.2 || (u - 33.3).abs() <= 0.2;
            if disc || line {
                let i = ((y * n + x) * 4) as usize;
                rgba[i..i + 3].copy_from_slice(&[0, 0, 0]);
            }
        }
    }
    rgba
}

#[test]
fn a_cut_out_part_traces_as_the_whole_picture_within_0_1_percent() {
    let n = 400;
    // D and E inside the picture, both away from the edges.
    let mut s = import(&svg_of(&busy(n), n, ""));
    let image = image_of(&doc(&s));
    s.set_tool(Tool::Trace);
    region(&mut s, D);
    assert!(s.add_trace_part().is_some());
    region(&mut s, E);
    let p = parts(&s);
    assert!(
        p[0]["regions"] == 1 && p[1]["regions"] == 1,
        "part 1 is D, part 2 is E: {p}"
    );
    let whole_px = s.trace_source(image, false).unwrap();
    let mut cut_any = false;
    for outside in [false, true] {
        s.set_trace_mask_outside(outside);
        for settings in [
            r#"{"threshold":128}"#,
            r#"{"threshold":128,"specks":1.0}"#,
            r#"{"threshold":128,"specks":2.5,"smooth":80}"#,
            r#"{"threshold":128,"denoise":2}"#,
        ] {
            for index in 0..2 {
                let params = trace_settings(settings);
                let auto = params.upscale == vectrace::Upscale::Auto;
                let cut_px = s
                    .trace_part_source(image, index, false, params.specks.as_mm(), auto)
                    .unwrap();
                if cut_px.width * cut_px.height < whole_px.width * whole_px.height {
                    cut_any = true;
                }
                let from_cut = part_wire(&mut s, image, index, settings, true);
                let from_whole = part_wire(&mut s, image, index, settings, false);
                let (wc, ww) = (
                    read_trace_wire(&from_cut).unwrap(),
                    read_trace_wire(&from_whole).unwrap(),
                );
                assert_eq!(wc.status, ww.status);
                let r = matched_ratio(&wc.outlines, &ww.outlines).unwrap_or(f64::INFINITY);
                assert!(
                    r <= 0.001,
                    "part {index} outside={outside} {settings}: cut-out differs by {r} ({} vs {} px, {} vs {} outlines)",
                    cut_px.width,
                    whole_px.width,
                    wc.outlines.len(),
                    ww.outlines.len()
                );
                assert_eq!(
                    wc.outlines.len(),
                    ww.outlines.len(),
                    "part {index} outside={outside} {settings}: the same outlines"
                );
            }
        }
    }
    assert!(cut_any, "at least one part is traced from a cut-out");
}

/// The colours of the layers of a multicolour wire.
fn layer_colours(wire: &[f64]) -> Vec<(u8, u8, u8)> {
    read_trace_wire(wire)
        .unwrap()
        .layers
        .iter()
        .map(|l| (l.colour.r, l.colour.g, l.colour.b))
        .collect()
}

#[test]
fn a_cut_out_multicolour_part_with_colours_auto_traces_as_the_whole_picture() {
    // 300 px over 100 mm (3 px per mm): the crop's 16 px margin is 5.3 mm. Inside D (20..60 mm)
    // a red disc and a blue square; a yellow band at x 61..64 mm lies just outside D, inside the
    // margin; magenta far away.
    let n = 300_u32;
    let mut rgba = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let (u, v) = ((f64::from(x) + 0.5) / 3.0, (f64::from(y) + 0.5) / 3.0);
            let c = if (u - 32.0).hypot(v - 32.0) <= 8.0 {
                [200, 30, 30]
            } else if (42.0..56.0).contains(&u) && (40.0..56.0).contains(&v) {
                [30, 60, 200]
            } else if (61.0..64.0).contains(&u) && (10.0..90.0).contains(&v) {
                [230, 210, 20]
            } else if (75.0..95.0).contains(&u) && (70.0..95.0).contains(&v) {
                [200, 20, 200]
            } else {
                [255, 255, 255]
            };
            rgba.extend([c[0], c[1], c[2], 255]);
        }
    }
    let mut s = import(&svg_of(&rgba, n, ""));
    let image = image_of(&doc(&s));
    s.set_tool(Tool::Trace);
    assert!(s.add_trace_part().is_some());
    region(&mut s, ((20.0, 20.0), (60.0, 60.0)));
    let p = parts(&s);
    assert_eq!(p[1]["regions"], 1, "part 2 is D: {p}");
    let cut = s.trace_part_source(image, 1, false, 0.0, false).unwrap();
    assert!(cut.width < n, "part 2 is cut out: {} px", cut.width);
    for settings in [
        r#"{"threshold":128,"output":1,"specks":0}"#,
        r#"{"threshold":128,"output":1,"colours":3,"specks":0}"#,
    ] {
        let from_cut = part_wire(&mut s, image, 1, settings, true);
        let from_whole = part_wire(&mut s, image, 1, settings, false);
        assert_eq!(
            layer_colours(&from_cut),
            layer_colours(&from_whole),
            "{settings}: the same colours from the cut-out and the whole picture"
        );
        let r =
            matched_ratio(&outlines(&from_cut), &outlines(&from_whole)).unwrap_or(f64::INFINITY);
        assert!(r <= 0.001, "{settings}: the cut-out differs by {r}");
    }
}

/// A 2000 x 2000 px picture over 200 mm: 400 discs on a 10 mm grid.
fn big() -> Vec<u8> {
    let side = 2000_u32;
    (0..side * side)
        .flat_map(|i| {
            let (x, y) = (f64::from(i % side) + 0.5, f64::from(i / side) + 0.5);
            let (cx, cy) = (
                (x / 100.0).floor() * 100.0 + 50.0,
                (y / 100.0).floor() * 100.0 + 50.0,
            );
            let v = if (x - cx).hypot(y - cy) <= 35.0 {
                0
            } else {
                255
            };
            [v, v, v, 255]
        })
        .collect()
}

/// The four quarter parts of a 2000 px picture in the session, Trace tool open.
fn quarters() -> (Session, NodeId) {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="400mm" height="400mm" viewBox="-100 -100 400 400"><image x="0" y="0" width="200" height="200" preserveAspectRatio="none" xlink:href="data:image/png;base64,{}"/></svg>"#,
        base64(&png_of(&big(), 2000))
    );
    let mut s = Session::new(1);
    s.resize_viewport(1200.0, 800.0);
    s.import_svg(svg.as_bytes(), "big.svg").unwrap();
    let d = doc(&s);
    let image = image_of(&d);
    s.set_tool(Tool::Trace);
    // One part per quarter, drawn with the Rectangle tool.
    let corners = [(0.0, 0.0), (100.0, 0.0), (0.0, 100.0), (100.0, 100.0)];
    for (n, &(x, y)) in corners.iter().enumerate() {
        if n > 0 {
            assert!(s.add_trace_part().is_some());
        }
        region(&mut s, ((x, y), (x + 100.0, y + 100.0)));
    }
    assert_eq!(s.trace_part_count(), 4);
    assert!(
        s.trace_parts()
            .iter()
            .all(|p| p.regions == 1 && p.contributes)
    );
    (s, image)
}

#[test]
fn c31_each_quarter_part_is_traced_from_a_cut_out() {
    let (mut s, image) = quarters();
    let whole = s.trace_source(image, false).unwrap();
    for index in 0..4 {
        let cut = s
            .trace_part_source(image, index, false, 0.0, false)
            .unwrap();
        assert!(
            u64::from(cut.width) * u64::from(cut.height)
                <= u64::from(whole.width) * u64::from(whole.height) * 3 / 10,
            "quarter {index}: {} x {} px of {} x {}",
            cut.width,
            cut.height,
            whole.width,
            whole.height
        );
    }
}

#[test]
#[ignore = "criterion 31 (reported, not blocking): run in release with --ignored --nocapture"]
fn c31_four_quarter_parts_cost_at_most_1_3_whole_traces_in_thread_cpu() {
    let (mut s, image) = quarters();
    let settings = r#"{"threshold":128}"#;
    let src = s.trace_source(image, false).unwrap();
    let (_, whole) = cpu_time::cpu_time_of(|| {
        run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: src.width,
                    height: src.height,
                    rgba: &src.rgba,
                },
                map: TracePixelMap::new(src.map),
                mode: TraceMode::Mono(&trace_settings(settings)),
                lines: &[],
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        )
    });
    let (head, parts) = cpu_time::cpu_time_of(|| join(&mut s, image, &[settings; 4], false));
    let ratio = parts.as_secs_f64() / whole.as_secs_f64();
    println!(
        "criterion 31 (session route): whole {:.3} s, four parts with sources and join {:.3} s, ratio {ratio:.2}",
        whole.as_secs_f64(),
        parts.as_secs_f64()
    );
    assert_ne!(head, Vec::<f64>::new());
    assert!(ratio <= 1.3, "ratio {ratio:.2}");
}

// ---------------------------------------------------------------------------------------------
// Determinism across builds: the native record the browser test replays in the editor wasm on
// two own instances (`frontend/tests/acceptance_0094_parts_tester.test.ts`). Clicks are made in
// screen pixels from the session's own conversion, so the same steps work in both builds.

const PARTS_NATIVE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/acceptance_0094_parts_tester_native.json"
);

/// The screen pixel of document point `p` (the inverse of `screen_to_document`).
fn screen_of(s: &Session, p: Point) -> (f64, f64) {
    let o = s.screen_to_document(0.0, 0.0);
    let ex = s.screen_to_document(1.0, 0.0);
    let ey = s.screen_to_document(0.0, 1.0);
    let (a, b, c, d) = (ex.x - o.x, ex.y - o.y, ey.x - o.x, ey.y - o.y);
    let det = a * d - b * c;
    let (dx, dy) = (p.x - o.x, p.y - o.y);
    ((d * dx - c * dy) / det, (a * dy - b * dx) / det)
}

/// The settings of the three parts of the record.
const RECORD_SETTINGS: [&str; 3] = [
    r#"{"threshold":60,"smooth":30}"#,
    r#"{"threshold":150,"smooth":70,"specks":0.5}"#,
    r#"{"threshold":90,"tolerance":1.5}"#,
];

#[test]
fn determinism_native_record_for_the_editor_wasm() {
    let svg = svg("");
    // As `WasmSession::new` and the page's `resize(1200, 800, 1)` make it.
    let mut s = Session::new(1);
    s.show_default_view();
    s.resize_viewport(1200.0, 800.0);
    s.import_svg(svg.as_bytes(), "parts.svg").unwrap();
    let image = image_of(&doc(&s));
    s.set_tool(Tool::Trace);
    let map = s.trace_source(image, false).unwrap().map;
    // The picture's pixel of SVG mm (u, v), then its screen pixel.
    let at = |s: &Session, (u, v): (f64, f64)| {
        let (x, y) = (u * f64::from(PX) / 100.0, v * f64::from(PX) / 100.0);
        screen_of(
            s,
            Point::new(
                map[0] * x + map[2] * y + map[4],
                map[1] * x + map[3] * y + map[5],
            ),
        )
    };
    // Steps: A drawn with Rectangle into part 1; Add part, C; Add part, B. A drag runs from its
    // first corner to its second, in screen pixels.
    let mut steps: Vec<serde_json::Value> = Vec::new();
    let mut step = |s: &mut Session, what: &str, area: Option<MmBox>| {
        if let Some((a, b)) = area {
            let ((x0, y0), (x1, y1)) = (at(s, a), at(s, b));
            s.set_trace_region_tool(TraceRegionTool::Rect);
            let (from, to) = (s.screen_to_document(x0, y0), s.screen_to_document(x1, y1));
            s.pointer_down(from, false);
            s.pointer_hover(to, false, false);
            s.pointer_up(to, false, false);
            s.set_trace_region_tool(TraceRegionTool::None);
            steps.push(serde_json::json!({ "do": what, "x0": x0, "y0": y0, "x1": x1, "y1": y1 }));
        } else {
            assert!(s.add_trace_part().is_some());
            steps.push(serde_json::json!({ "do": what }));
        }
    };
    step(&mut s, "rectangle", Some(A));
    step(&mut s, "add_part", None);
    step(&mut s, "rectangle", Some(C));
    step(&mut s, "add_part", None);
    step(&mut s, "rectangle", Some(B));
    let views = s.trace_parts();
    assert_eq!(views.len(), 3);
    assert!(
        views.iter().all(|v| v.regions == 1 && v.contributes),
        "{views:?}"
    );
    let mut parts_out = Vec::new();
    let mut wires = Vec::new();
    for (i, settings) in RECORD_SETTINGS.iter().enumerate() {
        let mask = s.trace_part_mask_json(i);
        let full = if mask.is_empty() {
            (*settings).to_owned()
        } else {
            format!("{},\"mask\":{mask}}}", settings.trim_end_matches('}'))
        };
        let specks = trace_settings(&full).specks.as_mm();
        let src = s.trace_part_source(image, i, false, specks, false).unwrap();
        let wire = part_wire(&mut s, image, i, settings, true);
        parts_out.push(serde_json::json!({
            "settings": full,
            "specks": specks,
            "width": src.width,
            "height": src.height,
            "map": src.map,
            "rgba_fnv32": src.rgba.iter().fold(0x811c_9dc5_u32, |h, &b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193)),
            "wire": wire,
        }));
        wires.push(wire);
    }
    let refs: Vec<Option<&[f64]>> = wires.iter().map(|w| Some(w.as_slice())).collect();
    let header = s.set_trace_joined(image, &refs, true).unwrap();
    let record = serde_json::json!({
        "svg": svg,
        "viewport": [1200, 800],
        "steps": steps,
        "parts": parts_out,
        "header": header,
    });
    if std::env::var_os("CURVYO_UPDATE_0094_PARTS_NATIVE").is_some()
        || !std::path::Path::new(PARTS_NATIVE).exists()
    {
        std::fs::write(PARTS_NATIVE, serde_json::to_string(&record).unwrap()).unwrap();
    }
    let kept: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(PARTS_NATIVE).unwrap()).unwrap();
    assert_eq!(kept, record, "the native record is this build's");
}

#[test]
fn a_cut_out_part_with_upscale_auto_traces_as_the_whole_picture() {
    // Engraving's Upscale Auto on a picture under 1 Mpx: the whole picture and its cut-out are
    // both traced from a 2x copy.
    let n = 400;
    let mut s = import(&svg_of(&busy(n), n, ""));
    let image = image_of(&doc(&s));
    s.set_tool(Tool::Trace);
    region(&mut s, D);
    assert!(s.add_trace_part().is_some());
    assert!(s.set_trace_active_part(0));
    let settings = r#"{"threshold":128,"upscale":0}"#;
    let cut = s.trace_part_source(image, 0, false, 0.5, true).unwrap();
    let whole = s.trace_source(image, false).unwrap();
    assert!(cut.width < whole.width, "part 1 is cut out");
    let region = trace_mask_region(&format!(
        "{},\"mask\":{}}}",
        settings.trim_end_matches('}'),
        s.trace_part_mask_json(0)
    ));
    let params = trace_settings(settings);
    let run = |src: &curvyo_editor_wasm::TraceSource, region: Option<&TraceRegion>| {
        read_trace_wire(&run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: src.width,
                    height: src.height,
                    rgba: &src.rgba,
                },
                map: TracePixelMap::new(src.map),
                mode: TraceMode::Mono(&params),
                lines: &[],
                mask: region.map(|r| r as &dyn Mask),
                shapes: false,
            },
            &mut |_| {},
        ))
        .unwrap()
    };
    // Unclipped, the cut-out's outlines must lie where the whole picture's do (D's box plus the
    // margin, 8..46 mm from the picture's left edge).
    let left_of = |w: &vectrace::Output| {
        use kurbo::Shape;
        w.outlines
            .iter()
            .map(|o| o.bounding_box().x0)
            .fold(f64::INFINITY, f64::min)
    };
    let origin = pt(&s, (0.0, 0.0)).x;
    let unclipped = run(&cut, None);
    assert!(
        left_of(&unclipped) - origin >= 8.0 - 0.3,
        "the cut-out's outlines start {:.3} mm from the picture's edge, before the cut (8 mm)",
        left_of(&unclipped) - origin
    );
    let (a, b) = (run(&cut, region.as_ref()), run(&whole, region.as_ref()));
    let r = matched_ratio(&a.outlines, &b.outlines).unwrap_or(f64::INFINITY);
    assert_eq!(a.outlines.len(), b.outlines.len(), "the same outlines");
    assert!(r <= 0.001, "the cut-out differs by {r}");
}

/// A part with Denoise is traced from the whole picture (adrs.md slice 2, Crop: the page's
/// `mayCrop` in `frontend/src/lib/traceParts.ts` sends no cut-out with Denoise, because the
/// session's cut-out of part E came back 0.83 % off). So part E with Denoise Low, traced from the
/// whole picture with its clip and joined, equals the trace of one part with the same mask
/// (slice 1), wire for wire and in the joined result.
#[test]
fn a_part_with_denoise_low_is_traced_from_the_whole_picture_and_equals_its_whole_picture_trace() {
    let n = 400;
    let settings = r#"{"threshold":128,"denoise":1}"#;
    // Two parts: part 1 is E, part 2 has no regions yet.
    let mut two = import(&svg_of(&busy(n), n, ""));
    let image = image_of(&doc(&two));
    two.set_tool(Tool::Trace);
    region(&mut two, E);
    assert!(two.add_trace_part().is_some());
    assert!(two.set_trace_active_part(0));
    let views = two.trace_parts();
    assert!(
        views[0].contributes && !views[1].contributes,
        "{:?}",
        parts(&two)
    );
    // One part with E (slice 1).
    let mut one = import(&svg_of(&busy(n), n, ""));
    let image_one = image_of(&doc(&one));
    one.set_tool(Tool::Trace);
    region(&mut one, E);
    assert_eq!(one.trace_parts().len(), 1);
    assert_eq!(
        two.trace_part_mask_json(0),
        one.trace_part_mask_json(0),
        "part 1's clip is E, as the one part's"
    );
    // The page's choice for Denoise: the whole picture, never the cut-out.
    let whole = two.trace_source(image, false).unwrap();
    assert_eq!(whole.width, n, "the whole picture is sent");
    let part = part_wire(&mut two, image, 0, settings, false);
    let single = part_wire(&mut one, image_one, 0, settings, false);
    assert_eq!(part, single, "the Denoise part = the whole-picture trace");
    let w = read_trace_wire(&part).unwrap();
    assert!(!w.outlines.is_empty(), "something is traced inside E");
    let joined_two = two
        .set_trace_joined(image, &[Some(part.as_slice()), None], true)
        .expect("the wire is a result");
    let joined_one = one
        .set_trace_joined(image_one, &[Some(single.as_slice())], true)
        .expect("the wire is a result");
    assert_eq!(
        joined_two, joined_one,
        "the joined result is the one part's"
    );
}

/// An image away from the document origin, traced whole with Upscale Auto (a 400 px picture is
/// traced from a 2x copy), lands where the same trace with Upscale Off lands and on the picture:
/// the upscaled pixel map's offset (PR #282) is right for the whole picture too.
#[test]
fn a_whole_picture_with_upscale_auto_away_from_the_origin_lands_on_the_picture() {
    use kurbo::Shape;
    let n = 400;
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="300mm" height="200mm" viewBox="0 0 300 200"><image x="137" y="61" width="100" height="100" preserveAspectRatio="none" xlink:href="data:image/png;base64,{}"/></svg>"#,
        base64(&png_of(&draw(M1, n), n))
    );
    let mut s = import(&svg);
    let d = doc(&s);
    let image = image_of(&d);
    let Some(ObjectSnapshot::Image(img)) = d.object(image) else {
        unreachable!()
    };
    let corner = img.corners()[0];
    assert!(
        corner.x.abs() > 50.0 && corner.y.abs() > 20.0,
        "the image is away from the origin: {corner:?}"
    );
    s.set_tool(Tool::Trace);
    let src = s.trace_source(image, false).unwrap();
    assert!(vectrace::Upscale::Auto.applies(src.width, src.height));
    let trace = |settings: &str| {
        read_trace_wire(&run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: src.width,
                    height: src.height,
                    rgba: &src.rgba,
                },
                map: TracePixelMap::new(src.map),
                mode: TraceMode::Mono(&trace_settings(settings)),
                lines: &[],
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        ))
        .unwrap()
    };
    let auto = trace(r#"{"threshold":100,"upscale":0}"#);
    let off = trace(r#"{"threshold":100,"upscale":1}"#);
    let bbox = |w: &vectrace::Output| {
        w.outlines
            .iter()
            .map(Shape::bounding_box)
            .reduce(|a, b| a.union(b))
            .expect("outlines")
    };
    let (a, o) = (bbox(&auto), bbox(&off));
    for (got, want, what) in [
        (a.x0, o.x0, "left"),
        (a.y0, o.y0, "top"),
        (a.x1, o.x1, "right"),
        (a.y1, o.y1, "bottom"),
    ] {
        assert!(
            (got - want).abs() <= 0.3,
            "{what}: Upscale Auto {got:.3} mm, Off {want:.3} mm"
        );
    }
    // M1's bar spans the picture's full width at 48..52 % of its height; the disc reaches 35 %.
    assert!(
        (a.x0 - corner.x).abs() <= 0.3,
        "left edge {:.3} vs the picture's {:.3}",
        a.x0,
        corner.x
    );
    assert!(
        (a.x1 - (corner.x + 100.0)).abs() <= 0.3,
        "right edge {:.3} vs {:.3}",
        a.x1,
        corner.x + 100.0
    );
    assert!(
        a.y0 >= corner.y - 0.3 && a.y1 <= corner.y + 100.3,
        "inside the picture: {a:?}, corner {corner:?}"
    );
}
