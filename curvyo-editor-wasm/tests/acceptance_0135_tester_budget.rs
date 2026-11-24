//! Tester budgets for `specs/0135-image-optimisation` criterion 55 at the session, in thread CPU
//! time, release only (`cargo test --release -p curvyo-editor-wasm --test
//! acceptance_0135_tester_budget -- --nocapture --test-threads=1`; debug only prints), five
//! repeats, the median against the budget: a Strength drag's reduced update (100 ms), the edit
//! frame of a preset pick with the reduced Before and after (100 ms), the worst tick while the
//! exact dots settle (50 ms), the Before and after view on a page of 1,000 pictures (35 ms), the
//! exact job of a 200 x 200 mm picture at Interval 0.1 with the Photo preset against the same job
//! without adjustments (the stack's share below 1.2 s), and the screen modes' job against
//! Ordered (ADR 0074 §6).
#![allow(
    clippy::assert_is_empty,
    clippy::many_single_char_names,
    clippy::trivially_copy_pass_by_ref,
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    missing_docs
)]

#[path = "cpu_time/mod.rs"]
mod cpu_time;

use std::sync::Arc;
use std::time::Duration;

use curvyo_document_core::{
    Affine, Document, DocumentSize, NewImage, NewOperation, NewOutputPage, NodeId, Point, pack,
};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_image_core::{WorkingImage, encode_png};

const TIMED: bool = !cfg!(debug_assertions);
const REPEATS: usize = 5;

fn check(name: &str, times: &[Duration], budget_ms: u64) {
    let budget = Duration::from_millis(budget_ms);
    let mut sorted = times.to_vec();
    sorted.sort();
    let median = sorted[sorted.len() / 2];
    let worst = sorted[sorted.len() - 1];
    let margin = budget.as_secs_f64() / median.as_secs_f64().max(1e-9);
    println!(
        "BUDGET {name}: median {median:?}, worst {worst:?}, budget {budget:?}, margin {margin:.1}x"
    );
    if TIMED {
        assert!(median <= budget, "{name}: median {median:?} > {budget:?}");
    }
}

fn picture(side: u32) -> Vec<u8> {
    let mut seed = 1u64;
    let mut pixels = Vec::with_capacity(side as usize * side as usize * 4);
    for y in 0..side {
        for x in 0..side {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let n = (seed >> 59).cast_signed() - 16;
            let g = ((i64::from(x) * 200 / i64::from(side))
                + (i64::from(y) * 55 / i64::from(side))
                + n)
                .clamp(0, 255) as u8;
            pixels.extend_from_slice(&[g, g.saturating_add(10), g, 255]);
        }
    }
    encode_png(&WorkingImage {
        width: side,
        height: side,
        pixels,
        original_size: (side, side),
        truncated: false,
        animated: false,
    })
    .unwrap()
}

/// `count` pictures of `side` px, each `mm` mm, the first in one Image operation at `interval`.
fn session(side: u32, mm: f64, count: usize, interval: f64) -> (Session, NodeId, NodeId, Point) {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let page = d.pages().into_iter().find(|p| p.id == made.id).unwrap();
    let m = page.kind.manufacturing().unwrap();
    let o = page.origin;
    let bytes: Arc<[u8]> = Arc::from(picture(side));
    let images: Vec<NewImage> = (0..count)
        .map(|k| {
            let (cx, cy) = ((k % 40) as f64, (k / 40) as f64);
            NewImage {
                bytes: bytes.clone(),
                placement: Affine::new(
                    mm,
                    0.0,
                    0.0,
                    mm,
                    o.x + 10.0 + cx * (mm + 0.5),
                    o.y + 10.0 + cy * (mm + 0.5),
                ),
                name: format!("Photo {}", k + 1),
            }
        })
        .collect();
    let ids = d.add_images(&images).unwrap();
    let op = d
        .add_operation(
            m,
            &NewOperation {
                kind: "image",
                name: "Image 1",
                settings: None,
            },
            0,
            &[vec![ids[0]]],
        )
        .unwrap();
    let mut s = Session::open(3, &pack(&d, "budget").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    assert!(s.activate_page(&page.id));
    s.load_materials(&[], None);
    let _ = s.save_machine_profile([13; 16], 1_000);
    let _ = s.set_machine_name("Diode 10 W");
    assert!(s.edit_operation_setting(op, "main", "power", 60.0).is_ok());
    assert!(
        s.edit_operation_setting(op, "main", "speed", 3000.0)
            .is_ok()
    );
    let _ = s.edit_operation_option(op, "interval", interval);
    s.wipe_history();
    let at = Point::new(o.x + 10.0 + mm * 0.1, o.y + 10.0 + mm * 0.1);
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    assert!(s.adjustments_view().is_some(), "picture selected");
    (s, ids[0], op, o)
}

fn settle(s: &Session) -> Duration {
    let mut worst = Duration::ZERO;
    let mut n = 0;
    while s.job_pending() || s.ticks_pending() {
        let ((), took) = cpu_time::cpu_time_of(|| {
            s.advance_tick();
        });
        worst = worst.max(took);
        n += 1;
        assert!(n < 100_000);
    }
    worst
}

#[test]
fn c55_a_strength_drag_updates_the_reduced_view_within_100_ms() {
    let (mut s, _, _, _) = session(2_000, 100.0, 1, 0.1);
    assert!(s.pick_adjust_preset("photo"));
    assert!(s.toggle_before_after());
    let _ = settle(&s);
    let mut times = Vec::new();
    for k in 0..REPEATS {
        let ((), took) = cpu_time::cpu_time_of(|| {
            s.hold_adjust_strength(55.0 + 7.0 * k as f64);
            let v = s.before_after_view();
            let _ = s.before_after_pixels("after");
            let _ = s.adjustments_view();
            let _ = v;
        });
        times.push(took);
    }
    let _ = s.release_adjust_strength();
    check(
        "Strength drag, reduced Before and after, 4 Mpx",
        &times,
        100,
    );
}

#[test]
fn c55_a_preset_pick_frame_and_the_worst_settling_tick() {
    let (mut s, _, _, _) = session(2_000, 100.0, 1, 0.1);
    assert!(s.toggle_before_after());
    let _ = settle(&s);
    let presets = ["photo", "graphics", "black_white", "line_art", "dot_raster"];
    let mut frames = Vec::new();
    let mut ticks = Vec::new();
    for p in presets {
        let ((), took) = cpu_time::cpu_time_of(|| {
            assert!(s.pick_adjust_preset(p));
            let _ = s.adjustments_view();
            let _ = s.before_after_view();
            let _ = s.before_after_pixels("after");
        });
        frames.push(took);
        ticks.push(settle(&s));
    }
    check(
        "preset pick frame with the reduced Before and after",
        &frames,
        100,
    );
    check("worst tick while the exact dots settle", &ticks, 50);
}

#[test]
fn c55_the_before_and_after_view_on_a_page_of_1000_pictures() {
    let (mut s, _, _, _) = session(64, 4.0, 1_000, 0.1);
    assert!(s.pick_adjust_preset("photo"));
    assert!(s.toggle_before_after());
    let _ = settle(&s);
    let mut times = Vec::new();
    let mut group = Vec::new();
    for _ in 0..REPEATS {
        let (_, took) = cpu_time::cpu_time_of(|| s.before_after_view());
        times.push(took);
        let (_, took) = cpu_time::cpu_time_of(|| s.adjustments_view());
        group.push(took);
    }
    check("Before and after view, 1,000 pictures", &times, 35);
    check("Adjustments group, 1,000 pictures", &group, 50);
}

#[test]
fn c55_the_exact_job_of_200_mm_at_0_1_with_the_photo_stack() {
    let (mut s, _, _, _) = session(2_000, 200.0, 1, 0.1);
    let mut plain = Vec::new();
    for _ in 0..3 {
        let (_, took) = cpu_time::cpu_time_of(|| s.export_job("Untitled.curvyo").unwrap());
        plain.push(took);
        // A new picture state each time so nothing is reused.
        let _ = s.set_adjust_strength(50.0);
    }
    let mut adjusted = Vec::new();
    assert!(s.pick_adjust_preset("photo"));
    for k in 0..3 {
        assert!(s.set_adjust_strength(40.0 + f64::from(k)).is_ok());
        let (_, took) = cpu_time::cpu_time_of(|| s.export_job("Untitled.curvyo").unwrap());
        adjusted.push(took);
    }
    let med = |v: &mut Vec<Duration>| {
        v.sort();
        v[v.len() / 2]
    };
    let (p, a) = (med(&mut plain), med(&mut adjusted));
    let stack = a.saturating_sub(p);
    println!("BUDGET exact job 200 mm at 0.1: plain {p:?}, Photo {a:?}, stack share {stack:?}");
    check(
        "stack share of the exact job (whole stack budget)",
        &[stack],
        1_200,
    );
}

#[test]
fn c55_screen_modes_cost_about_an_ordered_job() {
    // Superseded by the Dither on the picture (2026-11-20): the mode is the picture's (selected by `session`).
    let (mut s, _, op, _) = session(1_000, 100.0, 1, 0.1);
    let mut cost = |mode: &str| {
        assert!(s.set_picture_dither(mode));
        let mut t = Vec::new();
        for k in 0..3 {
            let _ = s.edit_operation_option(op, "gamma", 1.0 + 0.01 * f64::from(k));
            let (_, took) = cpu_time::cpu_time_of(|| s.export_job("Untitled.curvyo").unwrap());
            t.push(took);
        }
        t.sort();
        t[1]
    };
    let ordered = cost("ordered");
    let halftone = cost("halftone");
    let lines = cost("line_screen");
    println!(
        "BUDGET screen modes 100 mm at 0.1: ordered {ordered:?}, halftone {halftone:?}, lines {lines:?}"
    );
    let cap = ordered.mul_f64(1.5) + Duration::from_millis(20);
    if TIMED {
        assert!(
            halftone <= cap && lines <= cap,
            "screens within Ordered's budget"
        );
    }
}
