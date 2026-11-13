//! `specs/0114-image-trace-mask` criterion 24 as thread CPU time (`tests/cpu_time/`): 8 parts of
//! 20 regions each (rectangles and lassos, added and subtracted, overlapping) on one picture;
//! drawing a frame during a region drag, the release of a move and the parts' clips the retrace
//! reads stay under 50 ms each, without a panic; invalid geometry gives the notices of Behaviour
//! 12. Budgets are asserted in release builds (the `performance-budgets` job); a debug build prints
//! the times:
//!
//! ```text
//! cargo test --release -p curvyo-editor-wasm --test trace_mask_budget -- --nocapture
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::many_single_char_names,
    missing_docs
)]

mod cpu_time;

use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_document_core::{NodeId, Point};
use curvyo_editor_wasm::{KeyInput, Session, Tool, TraceRegionTool};
use image::ImageEncoder;

const HANDLER: Duration = Duration::from_millis(50);

fn check(what: &str, took: Duration) {
    println!("{what}: {:.2} ms CPU", took.as_secs_f64() * 1000.0);
    if !cfg!(debug_assertions) {
        assert!(took < HANDLER, "{what} took {took:?}, budget {HANDLER:?}");
    }
}

/// A 600 x 600 px picture of discs, imported over 100 mm and traced with the Trace tool.
fn picture() -> (Session, NodeId, [f64; 6]) {
    let side = 600_u32;
    let mut rgba = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        for x in 0..side {
            let (dx, dy) = (f64::from(x % 40) - 20.0, f64::from(y % 40) - 20.0);
            let v = if dx * dx + dy * dy < 150.0 { 0 } else { 255 };
            rgba.extend([v, v, v, 255]);
        }
    }
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&rgba, side, side, image::ExtendedColorType::Rgba8)
        .unwrap();
    let mut s = Session::new(1);
    s.resize_viewport(1200.0, 800.0);
    s.import_begin(None);
    s.import_add(&png, "discs.png");
    s.import_finish(false);
    s.set_tool(Tool::Trace);
    let image = s.trace_view().shown.unwrap();
    let map = s.trace_source(image, false).unwrap().map;
    (s, image, map)
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

/// The document point of picture pixel `(x, y)` by the picture's pixel map `m`.
fn px(m: [f64; 6], (x, y): (f64, f64)) -> Point {
    Point::new(m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

/// 8 parts, each with 10 rectangles and 10 lassos spread over the picture, every third one
/// subtracted, overlapping their neighbours.
fn eight_parts_of_twenty(s: &mut Session, m: [f64; 6]) {
    for part in 0..8 {
        if part > 0 {
            assert!(s.add_trace_part().is_some(), "part {part}");
        }
        for n in 0..20_u32 {
            let (cx, cy) = (
                f64::from((n * 37 + part * 61) % 520) + 40.0,
                f64::from((n * 53 + part * 29) % 520) + 40.0,
            );
            s.set_trace_subtract(n % 3 == 2);
            if n % 2 == 0 {
                s.set_trace_region_tool(TraceRegionTool::Rect);
                let (a, b) = (px(m, (cx - 35.0, cy - 25.0)), px(m, (cx + 35.0, cy + 25.0)));
                s.pointer_down(a, false);
                s.pointer_hover(b, false, false);
                s.pointer_up(b, false, false);
            } else {
                s.set_trace_region_tool(TraceRegionTool::Lasso);
                for k in 0..7_u32 {
                    let t = f64::from(k) / 7.0 * std::f64::consts::TAU;
                    let r = if k % 2 == 0 { 40.0 } else { 18.0 };
                    let p = px(m, (cx + r * t.cos(), cy + r * t.sin()));
                    s.pointer_down(p, false);
                    s.pointer_up(p, false, false);
                }
                key(s, "Enter");
            }
        }
    }
    s.set_trace_subtract(false);
    s.set_trace_region_tool(TraceRegionTool::None);
}

#[test]
fn c24_eight_parts_of_twenty_regions_edit_and_retrace_within_50_ms() {
    let (mut s, _, m) = picture();
    eight_parts_of_twenty(&mut s, m);
    assert_eq!(s.trace_part_count(), 8);
    assert!(s.trace_parts().iter().all(|p| p.regions == 20));
    let ((), took) = cpu_time_of(|| {
        let _ = s.trace_part_clip_list();
    });
    check("the 8 parts' clips, first", took);
    // The last part active: press its first rectangle (centre (467, 243) px) and drag it, a
    // frame per move.
    let (from, to) = (px(m, (467.0, 243.0)), px(m, (497.0, 253.0)));
    s.pointer_down(from, false);
    assert!(
        s.trace_selected_region().is_some(),
        "the press took a region"
    );
    let steps = s.undo_len();
    let mut worst = Duration::ZERO;
    for step in 1..=10 {
        let t = f64::from(step) / 10.0;
        let p = Point::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
        s.pointer_hover(p, false, false);
        let (_, took) = cpu_time_of(|| s.draw_list());
        worst = worst.max(took);
    }
    check("the worst frame during a region drag", worst);
    let ((), took) = cpu_time_of(|| s.pointer_up(to, false, false));
    check("the release of the drag (one step)", took);
    assert_eq!(s.undo_len(), steps + 1, "one step");
    let ((), took) = cpu_time_of(|| {
        let _ = s.trace_parts();
        for index in 0..8 {
            let _ = s.trace_part_mask_json(index);
        }
    });
    check("the parts and their clips for the retrace", took);
    let (_, took) = cpu_time_of(|| s.draw_list());
    check("a frame after the edit", took);
    // Every part picked, a frame each.
    for index in 0..8 {
        let ((), took) = cpu_time_of(|| {
            assert!(s.set_trace_active_part(index));
            let _ = s.draw_list();
        });
        check(&format!("picking part {}", index + 1), took);
    }
}

/// Behaviour 12 with many regions: a mask wholly outside the picture says so, a flat lasso adds
/// nothing, and nothing panics.
#[test]
fn c24_invalid_geometry_gives_the_notices() {
    let (mut s, _, m) = picture();
    s.set_trace_region_tool(TraceRegionTool::Rect);
    for n in 0..20 {
        let off = 700.0 + f64::from(n) * 10.0;
        let (a, b) = (px(m, (off, off)), px(m, (off + 30.0, off + 30.0)));
        s.pointer_down(a, false);
        s.pointer_hover(b, false, false);
        s.pointer_up(b, false, false);
    }
    s.set_trace_region_tool(TraceRegionTool::Lasso);
    for p in [(10.0, 10.0), (20.0, 20.0), (30.0, 30.0)] {
        let q = px(m, p);
        s.pointer_down(q, false);
        s.pointer_up(q, false, false);
    }
    key(&mut s, "Enter");
    let view = s.trace_view();
    assert_eq!(view.regions, 20, "the flat lasso is not stored");
    assert!(
        view.mask_outside_image,
        "0 paths: the mask is outside the image."
    );
    assert_eq!(s.trace_mask_json(), "");
    s.add_trace_part();
    assert!(
        s.trace_parts().iter().all(|p| !p.contributes),
        "0 paths: no part has anything to trace."
    );
}
