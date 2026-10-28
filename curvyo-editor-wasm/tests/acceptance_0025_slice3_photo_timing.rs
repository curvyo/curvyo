//! Tester timing of a large photo through the Trace tool, step by step, as thread CPU time
//! (CLAUDE.md section 5): decode and import, the reduced copy, the worker's trace (reduced and
//! full size, Photo preset), the preview mesh, each view, create, undo and redo. Run with
//!
//! ```text
//! cargo test --release -p curvyo-editor-wasm --test acceptance_0025_slice3_photo_timing -- --ignored --nocapture
//! ```
//!
//! The photo is not committed: `target/photo/iceberg.jpg`, see `SOURCES.md` next to this file's
//! fixtures (`curvyo-image-core/tests/fixtures/tester/SOURCES.md`). Without it the test skips.

#![allow(clippy::unwrap_used, clippy::cast_precision_loss, missing_docs)]

mod cpu_time;

use cpu_time::cpu_time_of;
use curvyo_editor_wasm::{Session, Tool, TraceViewMode};
use curvyo_ui_core::{TRACE_PRESETS, TraceParams, trace_auto_threshold, trace_job_with};

fn say(step: &str, took: std::time::Duration) {
    println!("{step:<44} {:>9.1} ms CPU", took.as_secs_f64() * 1000.0);
}

#[test]
#[ignore = "needs target/photo/iceberg.jpg (Wikimedia Commons, public domain)"]
fn a_4000_px_photo_through_the_trace_tool_step_by_step() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/photo/iceberg.jpg");
    let Ok(bytes) = std::fs::read(&path) else {
        println!("skipped: {} is missing", path.display());
        return;
    };
    let (decoded, took) = cpu_time_of(|| image::load_from_memory(&bytes).unwrap());
    say("decode with the image crate (reference)", took);
    println!("photo: {} x {} px", decoded.width(), decoded.height());
    drop(decoded);

    let mut session = Session::new(1);
    session.resize_viewport(1280.0, 800.0);
    let ((), took) = cpu_time_of(|| {
        session.import_begin(None);
        session.import_add(&bytes, "iceberg.jpg");
        session.import_finish(false);
    });
    say("import (decode, working image, document step)", took);
    let ((), took) = cpu_time_of(|| session.set_tool(Tool::Trace));
    say("enter the Trace tool", took);
    let image = session.trace_view().shown.unwrap();

    let (reduced, took) = cpu_time_of(|| session.trace_source(image, true).unwrap());
    say("reduced copy (1000 px) for the worker", took);
    let (threshold, took) =
        cpu_time_of(|| trace_auto_threshold(&reduced.rgba, reduced.width, reduced.height));
    say("automatic Threshold", took);
    let photo = TRACE_PRESETS.iter().find(|p| p.key == "photo").unwrap();
    let params = photo.apply(TraceParams::with_threshold(threshold));

    let (wire, took) = cpu_time_of(|| {
        trace_job_with(
            &reduced.rgba,
            reduced.width,
            reduced.height,
            reduced.map,
            &params,
            &mut |_| {},
        )
    });
    say("worker: trace of the reduced copy", took);
    println!("  reduced result: {} paths, {} nodes", wire[1], wire[2]);
    let (ok, took) = cpu_time_of(|| session.set_trace_preview(image, &wire));
    assert!(ok);
    say("preview taken (main thread)", took);

    for mode in [
        TraceViewMode::Overlay,
        TraceViewMode::Trace,
        TraceViewMode::Image,
    ] {
        let ((), took) = cpu_time_of(|| session.set_trace_view_mode(mode));
        say(&format!("{mode:?}: switch"), took);
        session.set_trace_view_moving(false);
        let (_, first) = cpu_time_of(|| session.draw_list());
        say(&format!("{mode:?}: first frame"), first);
        let (_, again) = cpu_time_of(|| session.draw_list());
        say(&format!("{mode:?}: next frame"), again);
        session.set_trace_view_moving(true);
        session.wheel(0.0, -120.0, 600.0, 400.0, false, true);
        let (_, moving) = cpu_time_of(|| session.draw_list());
        say(&format!("{mode:?}: frame while zooming"), moving);
        session.set_trace_view_moving(false);
        let (_, rest) = cpu_time_of(|| session.draw_list());
        say(&format!("{mode:?}: frame when the view rests"), rest);
        session.wheel(0.0, 120.0, 600.0, 400.0, false, true);
    }
    session.set_trace_view_mode(TraceViewMode::Overlay);

    let (full, took) = cpu_time_of(|| session.trace_source(image, false).unwrap());
    say("full-size pixels for the worker", took);
    let (wire, took) = cpu_time_of(|| {
        trace_job_with(
            &full.rgba,
            full.width,
            full.height,
            full.map,
            &params,
            &mut |_| {},
        )
    });
    say("worker: trace at full size", took);
    println!("  full-size result: {} paths, {} nodes", wire[1], wire[2]);
    let (ok, took) = cpu_time_of(|| session.set_trace_result(image, &wire));
    assert!(ok);
    say("full result taken (main thread)", took);
    let (outcome, took) = cpu_time_of(|| session.create_traced_paths());
    say("Enter: create the group", took);
    println!("  {outcome:?}");
    let (_, took) = cpu_time_of(|| session.draw_list());
    say("first frame with the group", took);
    let (_, took) = cpu_time_of(|| session.undo());
    say("undo", took);
    let (_, took) = cpu_time_of(|| session.redo());
    say("redo", took);
    let (_, took) = cpu_time_of(|| session.undo());
    say("undo again", took);
}

/// The same photo at other Thresholds and presets: which combination makes a step slow.
#[test]
#[ignore = "needs target/photo/iceberg.jpg (Wikimedia Commons, public domain)"]
fn the_photo_at_other_thresholds_and_presets() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/photo/iceberg.jpg");
    let Ok(bytes) = std::fs::read(&path) else {
        println!("skipped: {} is missing", path.display());
        return;
    };
    println!(
        "{:<12} {:>4} | {:>8} {:>7} | {:>9} {:>8} {:>7} {:>8} {:>8} {:>8}",
        "preset",
        "thr",
        "paths",
        "nodes",
        "trace ms",
        "take ms",
        "frame1",
        "create",
        "undo",
        "redo"
    );
    for key in ["photo", "engraving", "logo"] {
        for threshold in [40u8, 90, 128, 170, 220] {
            let mut session = Session::new(1);
            session.resize_viewport(1280.0, 800.0);
            session.import_begin(None);
            session.import_add(&bytes, "iceberg.jpg");
            session.import_finish(false);
            session.set_tool(Tool::Trace);
            let image = session.trace_view().shown.unwrap();
            let full = session.trace_source(image, false).unwrap();
            let preset = TRACE_PRESETS.iter().find(|p| p.key == key).unwrap();
            let params = preset.apply(TraceParams::with_threshold(threshold));
            let (wire, trace) = cpu_time_of(|| {
                trace_job_with(
                    &full.rgba,
                    full.width,
                    full.height,
                    full.map,
                    &params,
                    &mut |_| {},
                )
            });
            let (_, take) = cpu_time_of(|| session.set_trace_result(image, &wire));
            let (_, frame) = cpu_time_of(|| session.draw_list());
            let (_, create) = cpu_time_of(|| session.create_traced_paths());
            let (_, undo) = cpu_time_of(|| session.undo());
            let (_, redo) = cpu_time_of(|| session.redo());
            let ms = |d: std::time::Duration| d.as_secs_f64() * 1000.0;
            println!(
                "{key:<12} {threshold:>4} | {:>8} {:>7} | {:>9.0} {:>8.1} {:>7.0} {:>8.0} {:>8.0} {:>8.0}",
                wire[1],
                wire[2],
                ms(trace),
                ms(take),
                ms(frame),
                ms(create),
                ms(undo),
                ms(redo)
            );
        }
    }
}
