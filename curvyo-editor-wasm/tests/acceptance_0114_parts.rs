//! `specs/0114-image-trace-mask` slices 3 and 4 at the session: parts persist with their
//! settings (criteria 12, 17, 19), the mask follows the image's placement (13), a linked clone
//! shows and edits its root's mask (15), and several selected images each keep their own mask
//! (Frage 11 A).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names
)]

#[path = "../../curvyo-ui-core/tests/support_0094/mod.rs"]
mod support;

use curvyo_document_core::{Document, NodeId, ObjectSnapshot, Point, unpack};
use curvyo_editor_wasm::{RowPress, Session, Tool, TraceRegionTool};
use image::ImageEncoder;
use support::{M1, base64, draw};

const PX: u32 = 300;

fn svg(count: usize) -> String {
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&draw(M1, PX), PX, PX, image::ExtendedColorType::Rgba8)
        .unwrap();
    let pic = base64(&png);
    let images: String = (0..count).fold(String::new(), |mut all, i| {
        use std::fmt::Write;
        let _ = write!(
            all,
            r#"<image x="{}" y="0" width="100" height="100" preserveAspectRatio="none" xlink:href="data:image/png;base64,{pic}"/>"#,
            i * 150
        );
        all
    });
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="400mm" height="300mm" viewBox="-100 -100 400 300">{images}</svg>"#
    )
}

fn doc(s: &Session) -> Document {
    unpack(9, &s.pack("t").unwrap()).unwrap()
}

fn images_of(s: &Session) -> Vec<NodeId> {
    let d = doc(s);
    d.object_ids()
        .into_iter()
        .filter(|&id| matches!(d.object(id), Some(ObjectSnapshot::Image(_))))
        .collect()
}

fn corners(s: &Session, id: NodeId) -> [Point; 4] {
    doc(s)
        .read_objects()
        .objects
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Image(i) if i.id == id => Some(i.corners()),
            ObjectSnapshot::Clone(c) if c.id() == id => {
                c.chain.picture.as_deref().and_then(|p| match p {
                    [ObjectSnapshot::Image(i)] => Some(i.corners()),
                    _ => None,
                })
            }
            _ => None,
        })
        .unwrap()
}

/// The point at fractions (`u`, `v`) of the picture `id`'s frame.
fn at(s: &Session, id: NodeId, (u, v): (f64, f64)) -> Point {
    let c = corners(s, id);
    Point::new(
        c[0].x + (c[1].x - c[0].x) * u + (c[3].x - c[0].x) * v,
        c[0].y + (c[1].y - c[0].y) * u + (c[3].y - c[0].y) * v,
    )
}

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

/// Draws a rectangle over fractions `a`..`b` of the picture `id` with the Rectangle tool.
fn rectangle(s: &mut Session, id: NodeId, a: (f64, f64), b: (f64, f64)) {
    s.set_trace_region_tool(TraceRegionTool::Rect);
    let (p, q) = (at(s, id, a), at(s, id, b));
    s.pointer_down(p, false);
    s.pointer_hover(q, false, false);
    s.pointer_up(q, false, false);
    s.set_trace_region_tool(TraceRegionTool::None);
}

fn traced(count: usize) -> (Session, Vec<NodeId>) {
    let mut s = Session::new(1);
    s.resize_viewport(1200.0, 800.0);
    s.import_svg(svg(count).as_bytes(), "m.svg").unwrap();
    let ids = images_of(&s);
    select(&mut s, &ids[..1]);
    s.set_tool(Tool::Trace);
    (s, ids)
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

fn setting(s: &Session, index: usize, key: &str) -> serde_json::Value {
    let json: serde_json::Value = serde_json::from_str(
        &s.trace_part_settings(index)
            .map(serde_json::Value::Object)
            .unwrap()
            .to_string(),
    )
    .unwrap();
    json[key].clone()
}

/// Criteria 12 and 17: two parts with their own settings; a settings edit is one step "Change
/// trace settings" (none when unchanged); undo and redo while the tool is open bring the values
/// back; after save and reopen the parts, modes, settings and clips are as left.
#[test]
fn c12_c17_parts_keep_their_settings_through_undo_and_reopen() {
    let (mut s, ids) = traced(1);
    let image = ids[0];
    s.set_trace_tool_settings(r#"{"threshold": 60, "smooth": 30}"#);
    assert_eq!(
        s.trace_part_settings(0),
        None,
        "an untouched image stores no settings"
    );
    assert!(
        !s.set_trace_part_settings(0, r#"{"smooth": 40}"#),
        "tool state, no step"
    );
    rectangle(&mut s, image, (0.0, 0.0), (0.5, 1.0));
    assert_eq!(
        setting(&s, 0, "smooth"),
        serde_json::json!(40),
        "the first gesture stored the bar's"
    );
    s.add_trace_part();
    rectangle(&mut s, image, (0.5, 0.0), (1.0, 1.0));
    s.set_trace_mask_outside(true);
    let steps = s.undo_len();
    assert!(s.set_trace_part_settings(1, r#"{"smooth": 75, "tolerance": 1.5, "threshold": 120}"#));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(labels(&s).last().unwrap(), "edit_trace_settings");
    assert!(
        !s.set_trace_part_settings(1, r#"{"smooth": 75, "tolerance": 1.5, "threshold": 120}"#),
        "unchanged: no step"
    );
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(setting(&s, 1, "smooth"), serde_json::json!(75));
    assert_eq!(
        setting(&s, 0, "smooth"),
        serde_json::json!(40),
        "part 1 keeps its own"
    );
    s.undo();
    assert_ne!(
        setting(&s, 1, "smooth"),
        serde_json::json!(75),
        "undo reshows the earlier value"
    );
    s.redo();
    assert_eq!(setting(&s, 1, "smooth"), serde_json::json!(75));
    let clips: Vec<String> = (0..2).map(|i| s.trace_part_mask_json(i)).collect();
    let settings: Vec<_> = (0..2).map(|i| s.trace_part_settings(i)).collect();
    let mut reopened = Session::open(3, &s.pack("t").unwrap()).unwrap();
    reopened.resize_viewport(1200.0, 800.0);
    select(&mut reopened, &[image]);
    reopened.set_tool(Tool::Trace);
    assert_eq!(reopened.trace_part_count(), 2);
    assert_eq!(reopened.trace_active_part(), 0, "part 1 active on entry");
    let again: Vec<String> = (0..2).map(|i| reopened.trace_part_mask_json(i)).collect();
    assert_eq!(again, clips, "the same clips");
    let read: Vec<_> = (0..2).map(|i| reopened.trace_part_settings(i)).collect();
    assert_eq!(read, settings, "the same settings");
    assert!(reopened.trace_parts()[1].outside);
}

/// Criterion 13 at the session: after a move and a mirror of the image the region still lies on
/// the same picture content.
#[test]
fn c13_the_region_moves_and_mirrors_with_the_image() {
    let (mut s, ids) = traced(1);
    let image = ids[0];
    rectangle(&mut s, image, (0.0, 0.0), (0.5, 1.0));
    let inside = (0.25, 0.5);
    let beyond = (0.75, 0.5);
    s.set_tool(Tool::Select);
    select(&mut s, &[image]);
    s.flip_selection(curvyo_ui_core::FlipAxis::Horizontal);
    for _ in 0..3 {
        s.key_down(curvyo_editor_wasm::KeyInput {
            key: "ArrowRight",
            shift: true,
            ctrl: false,
            alt: false,
            repeat: false,
            dom_blocked: false,
            mac: false,
            time_ms: 0.0,
        });
    }
    s.set_tool(Tool::Trace);
    let region = s.trace_region().unwrap();
    assert!(
        region.keeps(at(&s, image, inside)),
        "the picture's left half, wherever it is now"
    );
    assert!(!region.keeps(at(&s, image, beyond)));
}

/// Criterion 15 (CLONE-MASK A): Trace on a linked clone shows the root's regions in the clone's
/// placement; a region drawn on the clone is stored on the root; deleting the clone keeps them.
#[test]
fn c15_a_clone_shows_and_edits_its_root_s_mask() {
    let (mut s, ids) = traced(1);
    let image = ids[0];
    rectangle(&mut s, image, (0.0, 0.0), (0.5, 1.0));
    s.set_tool(Tool::Select);
    select(&mut s, &[image]);
    s.linked_clone();
    let clone = doc(&s)
        .read_objects()
        .objects
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Clone(c) => Some(c.id()),
            _ => None,
        })
        .unwrap();
    assert_ne!(clone, image);
    // Move the clone away so its frame differs from the root's.
    for _ in 0..12 {
        s.key_down(curvyo_editor_wasm::KeyInput {
            key: "ArrowDown",
            shift: true,
            ctrl: false,
            alt: false,
            repeat: false,
            dom_blocked: false,
            mac: false,
            time_ms: 0.0,
        });
    }
    s.set_tool(Tool::Trace);
    assert_eq!(s.trace_view().images, [clone]);
    assert_eq!(s.trace_view().regions, 1, "the root's region");
    let region = s.trace_region().unwrap();
    assert!(
        region.keeps(at(&s, clone, (0.25, 0.5))),
        "on the clone's picture"
    );
    assert!(
        !region.keeps(at(&s, image, (0.25, 0.5)))
            || at(&s, clone, (0.25, 0.5)) == at(&s, image, (0.25, 0.5))
    );
    rectangle(&mut s, clone, (0.6, 0.1), (0.9, 0.4));
    assert_eq!(
        doc(&s).trace_mask(image).unwrap().parts[0].regions.len(),
        2,
        "the edit is visible on the original"
    );
    assert_eq!(doc(&s).trace_mask(clone), None);
    s.set_tool(Tool::Select);
    select(&mut s, &[clone]);
    s.delete_selected();
    assert_eq!(doc(&s).trace_mask(image).unwrap().parts[0].regions.len(), 2);
}

/// Frage 11 A: with two images selected the parts are not edited (no region tool draws), and each
/// image is traced with its own stored parts.
#[test]
fn several_images_keep_their_own_masks() {
    let (mut s, ids) = traced(2);
    rectangle(&mut s, ids[0], (0.0, 0.0), (0.5, 1.0));
    select(&mut s, &[ids[1]]);
    assert_eq!(s.trace_view().images, [ids[1]]);
    assert!(s.add_trace_part().is_some(), "{:?}", s.trace_view());
    rectangle(&mut s, ids[1], (0.5, 0.0), (1.0, 1.0));
    select(&mut s, &ids);
    assert_eq!(s.trace_view().images.len(), 2);
    assert_eq!(s.trace_part_count(), 1, "no parts to edit for two pictures");
    s.set_trace_region_tool(TraceRegionTool::Rect);
    let steps = s.undo_len();
    let (p, q) = (at(&s, ids[0], (0.6, 0.6)), at(&s, ids[0], (0.9, 0.9)));
    s.pointer_down(p, false);
    s.pointer_up(q, false, false);
    assert_eq!(s.undo_len(), steps, "nothing drawn");
    s.set_trace_region_tool(TraceRegionTool::None);
    select(&mut s, &ids);
    let first = s.for_trace_image(ids[0], |s| {
        (s.trace_part_count(), s.trace_part_mask_json(0))
    });
    let second = s.for_trace_image(ids[1], Session::trace_part_count);
    assert_eq!((first.0, second), (1, 2), "each image's own parts");
    assert!(s.for_trace_image(ids[0], Session::trace_region).is_some());
    let source = s.trace_part_source(ids[1], 1, false, 0.0, false);
    assert!(
        source.is_some(),
        "a part of the second image has its pixels"
    );
}
