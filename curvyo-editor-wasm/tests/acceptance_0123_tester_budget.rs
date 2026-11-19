//! Tester budgets for `specs/0123-raster-engraving` criteria 12 and 37 at the session, in thread
//! CPU time, measured as the app pays them: the frontend re-reads the Operations view, the
//! output group (mounted in `App`, read on every sync) and the canvas after every command, so one
//! Image setting edit costs the edit plus those three reads in the same frame. Criterion 37 (d)
//! sets 50 ms for the preview after an edit; (b) 250 ms for the writer of a worst-case dither;
//! (f) 16 ms for checks after an edit that keeps the dots. Asserted in release builds only (run
//! one `--test` at a time, `--test-threads=1`); a debug build prints the times.

#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    missing_docs
)]

mod cpu_time;

use std::sync::Arc;
use std::time::Duration;

use curvyo_document_core::{
    Affine, Document, DocumentSize, NewImage, NewOperation, NewOutputPage, NodeId, pack,
};
use curvyo_editor_wasm::Session;
use curvyo_image_core::{WorkingImage, encode_png};

const TIMED: bool = !cfg!(debug_assertions);
const REPEATS: usize = 5;

fn check(name: &str, times: &[Duration], budget_ms: u64) {
    let budget = Duration::from_millis(budget_ms);
    let worst = times.iter().max().copied().unwrap_or_default();
    let mut sorted = times.to_vec();
    sorted.sort();
    let median = sorted[sorted.len() / 2];
    println!("{name}: median {median:?}, worst {worst:?} (budget {budget:?}) {times:?}");
    if TIMED {
        assert!(median <= budget, "{name}: median {median:?} > {budget:?}");
    }
}

/// A `side` px picture: a two-axis shade with a fine noise on top (many runs in every dither).
fn picture(side: u32) -> Vec<u8> {
    let mut seed = 1u64;
    let mut pixels = Vec::with_capacity(side as usize * side as usize * 4);
    for y in 0..side {
        for x in 0..side {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let shade = (x + y) * 200 / (2 * side);
            let noise = (seed >> 58) as u32;
            let g = u8::try_from((shade + noise * 4).min(255)).unwrap();
            pixels.extend_from_slice(&[g, g, g, 255]);
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

/// The 200 x 200 mm image at Interval 0.1 (4 million dots) in an Image operation; the opened
/// session with the Operations tab's row unfolded, and the operation. Superseded by the Image
/// kind rework (2026-11-19): it was an Engrave.
fn session() -> (Session, NodeId) {
    let side = if TIMED { 2_000 } else { 200 };
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let m = laser.kind.manufacturing().unwrap();
    let o = laser.origin;
    let image = d
        .add_images(&[NewImage {
            bytes: Arc::from(picture(side).as_slice()),
            placement: Affine::new(200.0, 0.0, 0.0, 200.0, o.x + 20.0, o.y + 20.0),
            name: "Photo 1".to_owned(),
        }])
        .unwrap()[0];
    let engrave = d
        .add_operation(
            m,
            &NewOperation {
                kind: "image",
                name: "Image 1",
            },
            0,
            &[vec![image]],
        )
        .unwrap();
    let mut s = Session::open(3, &pack(&d, "budget").unwrap()).unwrap();
    s.load_machine_profiles_json("[]");
    assert!(s.activate_page(&laser.id));
    s.resize_viewport(1600.0, 1000.0);
    assert!(
        s.edit_operation_setting(engrave, "main", "power", 50.0)
            .is_ok()
    );
    assert!(
        s.edit_operation_setting(engrave, "main", "speed", 6000.0)
            .is_ok()
    );
    assert_eq!(s.edit_operation_option(engrave, "interval", 0.1), Ok(true));
    assert!(s.unfold_operation_row(engrave));
    // Warm: the picture is decoded, the job and the preview are built once.
    let _ = s.operations_view_json();
    let _ = s.output_group_json();
    let _ = s.frame_draw_list();
    (s, engrave)
}

/// What the app does after one command: the Operations tab, the output group and the canvas.
fn frame(s: &Session) {
    let _ = s.operations_view_json();
    let _ = s.output_group_json();
    let _ = s.frame_draw_list();
}

#[test]
fn c12_c37d_one_image_setting_edit_in_the_app_frame_takes_50_ms() {
    let (mut s, e) = session();
    let mut times = Vec::new();
    let mut preview_only = Vec::new();
    for k in 0..REPEATS {
        let gamma = 1.1 + 0.1 * k as f64;
        let ((), took) = cpu_time::cpu_time_of(|| {
            assert_eq!(s.edit_operation_option(e, "gamma", gamma), Ok(true));
            frame(&s);
        });
        times.push(took);
        let ((), took) = cpu_time::cpu_time_of(|| {
            assert_eq!(
                s.edit_operation_option(e, "brightness", k as f64 + 1.0),
                Ok(true)
            );
            let _ = s.frame_draw_list();
        });
        preview_only.push(took);
    }
    check(
        "37 d edit + preview only (the implementer's measure)",
        &preview_only,
        50,
    );
    check(
        "12/37 d edit + full app frame (views, checks, canvas)",
        &times,
        50,
    );
}

#[test]
fn c12_c37d_a_dither_pick_and_its_undo_in_the_app_frame_take_50_ms() {
    let (mut s, e) = session();
    let modes = ["ordered", "jarvis", "atkinson", "stucki", "threshold"];
    let mut picks = Vec::new();
    let mut undos = Vec::new();
    for mode in modes {
        let ((), took) = cpu_time::cpu_time_of(|| {
            assert!(s.set_operation_dither(e, mode));
            frame(&s);
        });
        picks.push(took);
        let ((), took) = cpu_time::cpu_time_of(|| {
            let _ = s.undo();
            frame(&s);
        });
        undos.push(took);
        assert!(s.set_operation_dither(e, mode));
        frame(&s);
    }
    check("12/37 d dither pick + app frame", &picks, 50);
    check("12/37 d undo + app frame", &undos, 50);
}

#[test]
fn c37b_the_writer_of_a_worst_case_dither_takes_250_ms() {
    let (mut s, e) = session();
    assert!(s.set_operation_dither(e, "ordered"));
    let mut times = Vec::new();
    for _ in 0..REPEATS {
        let _ = s.output_group_json();
        let (bytes, took) =
            cpu_time::cpu_time_of(|| s.export_job("Untitled.curvyo").unwrap().bytes);
        assert!(bytes.len() > 1_000);
        times.push(took);
    }
    check("37 b export of a built job (writer)", &times, 250);
}

#[test]
fn c37f_checks_after_a_power_edit_take_16_ms() {
    let (mut s, e) = session();
    let mut times = Vec::new();
    for k in 0..REPEATS {
        let (_, took) = cpu_time::cpu_time_of(|| {
            assert!(
                s.edit_operation_setting(e, "main", "power", 40.0 + k as f64)
                    .is_ok()
            );
            s.output_group_json()
        });
        times.push(took);
    }
    check("37 f power edit + checks", &times, 16);
}

/// Where the app frame's time after an Image setting edit goes (printed, the budget is above).
#[test]
fn c12_where_the_edit_frame_goes() {
    let (mut s, e) = session();
    for k in 0..3 {
        let (_, edit) =
            cpu_time::cpu_time_of(|| s.edit_operation_option(e, "contrast", 5.0 + f64::from(k)));
        let (_, ops) = cpu_time::cpu_time_of(|| s.operations_view_json());
        let (_, outp) = cpu_time::cpu_time_of(|| s.output_group_json());
        let (_, canvas) = cpu_time::cpu_time_of(|| s.frame_draw_list());
        println!(
            "edit {edit:?} · operations_view_json {ops:?} · output_group_json {outp:?} · frame_draw_list {canvas:?}"
        );
    }
}
