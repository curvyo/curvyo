//! `specs/0123-raster-engraving` criterion 37 (c) to (f) at the session (ADR 0068 §8, ADR 0069
//! §11), in thread CPU time: (c) the job of a 200 x 200 mm image at 0.1 mm (16 Mpx picture, its
//! grey pyramid included) and 50 Cut items builds within 37 (a) plus `0029` 36 (a), 450 ms;
//! (d) the app frame after an Image setting edit, and after its undo, within 50 ms (the Operations
//! tab, the output group and the canvas; also zoomed in to one pixel per dot), the job's rest worked in ticks of 16 ms each;
//! (e) a pan of the Toolpath view with that job within 16 ms; (f) the checks after an edit that
//! keeps the dots within 16 ms; and opening the Operations tab with the Image settings group
//! unfolded within 16 ms; (g) the same with 1,000 images in one Image operation. Budgets are asserted in release builds, one `--test` at a time; a debug
//! build prints the times (with a smaller picture) and checks the rest.

#![allow(clippy::unwrap_used, clippy::many_single_char_names)]

mod cpu_time;

use std::sync::Arc;
use std::time::Duration;

use curvyo_document_core::{
    Affine, Document, DocumentSize, NewImage, NewOperation, NewOutputPage, NodeId, Point,
    RectBounds, SettingsStep, SettingsWrite, pack,
};
use curvyo_editor_wasm::Session;
use curvyo_image_core::{WorkingImage, encode_png};
use serde_json::{Value, json};

/// Whether the times are asserted: release builds only.
const TIMED: bool = !cfg!(debug_assertions);

fn check(name: &str, took: Duration, budget_ms: u64) {
    let budget = Duration::from_millis(budget_ms);
    println!("{name}: {took:?} (budget {budget:?})");
    if TIMED {
        assert!(took <= budget, "{name}: {took:?} > {budget:?}");
    }
}

/// A smooth two-axis shade with a fine checker on top, `side` pixels square (4,000: 16 Mpx).
fn picture() -> Vec<u8> {
    let side: u32 = if TIMED { 4_000 } else { 400 };
    let mut pixels = Vec::with_capacity(side as usize * side as usize * 4);
    for y in 0..side {
        for x in 0..side {
            let shade = (x + y) * 255 / (2 * side);
            let checker = if (x / 2 + y / 2) % 2 == 0 { 0 } else { 40 };
            let g = u8::try_from((shade + checker).min(255)).unwrap();
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

fn object(value: &Value) -> serde_json::Map<String, Value> {
    let Value::Object(map) = value.clone() else {
        panic!("an object")
    };
    map
}

/// A 400 x 300 mm laser page with the image at (50, 50) in an Image operation (Floyd-Steinberg,
/// 50 %, 6,000 mm/min, 0.1 mm) and 50 squares in a Cut: the opened session, the Image operation
/// and the Cut (an Engrave before the Image kind rework, 2026-11-19).
fn session() -> (Session, NodeId, NodeId) {
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
            bytes: Arc::from(picture().as_slice()),
            placement: Affine::new(200.0, 0.0, 0.0, 200.0, o.x + 50.0, o.y + 50.0),
            name: "Photo 1".to_owned(),
        }])
        .unwrap()[0];
    let squares: Vec<Vec<NodeId>> = (0..50)
        .map(|i| {
            let x = o.x + 260.0 + f64::from(i % 10) * 12.0;
            let y = o.y + 20.0 + f64::from(i / 10) * 12.0;
            vec![d.create_rect(RectBounds::from_corners(
                Point::new(x, y),
                Point::new(x + 10.0, y + 10.0),
            ))]
        })
        .collect();
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
    let cut = d
        .add_operation(
            m,
            &NewOperation {
                kind: "cut",
                name: "Cut 1",
            },
            1,
            &squares,
        )
        .unwrap();
    d.write_operation_settings(
        SettingsStep::EditSettings,
        &[
            SettingsWrite {
                operation: engrave,
                value: Some(object(&json!({"main": {"power_pct": 50.0,
                    "speed_mm_min": 6000, "passes": 1, "interval_mm": 0.1}}))),
            },
            SettingsWrite {
                operation: cut,
                value: Some(object(&json!({"main": {"power_pct": 60.0,
                    "speed_mm_min": 600, "passes": 1}}))),
            },
        ],
    )
    .unwrap();
    let bytes = pack(&d, "raster budget").unwrap();
    let mut s = Session::open(3, &bytes).unwrap();
    s.load_machine_profiles_json("[]");
    assert!(s.activate_page(&laser.id));
    s.resize_viewport(1600.0, 1000.0);
    assert!(s.restore_view_text("1 200 150"));
    // The canvas decodes the picture once with the Operations view released (the drawn look), as
    // opening the page does; the job takes it from there.
    assert!(s.toggle_operations_view(&laser.id));
    let mut list = s.frame_draw_list();
    let _ = s.frame_images(&mut list);
    assert!(s.toggle_operations_view(&laser.id));
    (s, engrave, cut)
}

fn page(s: &Session) -> String {
    s.active_page().id
}

#[test]
fn c_the_job_of_a_200_mm_image_and_50_cut_items_builds_within_450_ms() {
    let (s, _, _) = session();
    let (json, took) = cpu_time::cpu_time_of(|| s.output_group_json());
    let view: Value = serde_json::from_str(&json).unwrap();
    let lengths = view["total"]["lengths"].as_str().unwrap();
    let burn_m: f64 = lengths
        .trim_start_matches("Burn ")
        .split(' ')
        .next()
        .unwrap()
        .replace(',', "")
        .parse()
        .unwrap();
    assert!(
        lengths.split(' ').nth(2) == Some("m") && burn_m > 100.0,
        "the image's rows burn: {lengths}"
    );
    check("37 c", took, 450);
}

/// What the app reads after one command: the Operations tab, the output group and the canvas.
fn app_frame(s: &Session) {
    let _ = s.operations_view_json();
    let _ = s.output_group_json();
    let _ = s.frame_draw_list();
}

/// Works the pending job in ticks as the host does, one per animation frame: the worst tick.
fn ticks(s: &Session) -> Duration {
    let mut worst = Duration::ZERO;
    let mut count = 0;
    while s.job_pending() {
        let (_, took) = cpu_time::cpu_time_of(|| s.advance_job());
        worst = worst.max(took);
        count += 1;
        assert!(count < 1_000, "the job finishes");
    }
    worst
}

#[test]
fn d_the_dots_after_an_image_setting_edit_and_its_undo_are_ready_within_50_ms() {
    let (mut s, engrave, _) = session();
    // The first frame builds the job and the preview's grey pyramid once.
    app_frame(&s);
    let ((), took) = cpu_time::cpu_time_of(|| {
        assert_eq!(s.edit_operation_option(engrave, "gamma", 1.4), Ok(true));
        app_frame(&s);
    });
    check("37 d edit + app frame", took, 50);
    let pending = s.job_pending();
    check("37 d tick", ticks(&s), 16);
    assert!(!s.job_pending());
    if TIMED {
        assert!(pending, "a 4 million dot item is worked in ticks");
    }
    let ((), took) = cpu_time::cpu_time_of(|| {
        let _ = s.undo();
        app_frame(&s);
    });
    check("37 d undo + app frame", took, 50);
    check("37 d undo tick", ticks(&s), 16);
    let (_, took) = cpu_time::cpu_time_of(|| s.operations_view_json());
    check("37 d finished tab", took, 16);
}

/// 37 (d) zoomed in to one CSS pixel per 0.1 mm dot (preview step 1, architect review S2): the
/// Operations view's preview of the 4 million dot item is capped, so the app frame after an Image
/// setting edit stays within 50 ms.
#[test]
fn d_zoomed_to_one_pixel_per_dot_the_app_frame_after_an_edit_stays_within_50_ms() {
    let (mut s, engrave, _) = session();
    assert!(s.restore_view_text("10 150 150"));
    app_frame(&s);
    let ((), took) = cpu_time::cpu_time_of(|| {
        assert_eq!(s.edit_operation_option(engrave, "gamma", 1.4), Ok(true));
        app_frame(&s);
    });
    check("37 d edit + app frame at preview step 1", took, 50);
    check("37 d tick at preview step 1", ticks(&s), 16);
}

#[test]
fn e_a_pan_of_the_toolpath_view_takes_16_ms() {
    let (mut s, _, _) = session();
    let laser = page(&s);
    assert!(s.toggle_toolpath_view(&laser));
    let _ = s.frame_draw_list();
    s.begin_pan(800.0, 500.0);
    let mut worst = Duration::ZERO;
    for step in 1..=10 {
        let (_, took) = cpu_time::cpu_time_of(|| {
            s.pan_to(800.0 + f64::from(step) * 7.0, 500.0);
            s.frame_draw_list()
        });
        worst = worst.max(took);
    }
    s.end_pan();
    check("37 e pan", worst, 16);
}

#[test]
fn f_the_checks_after_an_edit_that_keeps_the_dots_and_opening_the_tab_take_16_ms() {
    let (mut s, engrave, cut) = session();
    let _ = s.output_group_json();
    let _ = s.operations_view_json();
    let (_, took) = cpu_time::cpu_time_of(|| {
        s.edit_operation_setting(cut, "main", "power", 55.0)
            .unwrap();
        s.output_group_json()
    });
    check("37 f checks", took, 16);
    let (json, took) = cpu_time::cpu_time_of(|| {
        assert!(s.unfold_operation_row(engrave));
        s.operations_view_json()
    });
    // Superseded by the Dither on the picture (2026-11-20): a picture-only Image row shows its
    // tone fields and no Dither of its own.
    assert!(json.contains("\"invert\":false"), "the group is open");
    check("37 f tab open", took, 16);
}

/// A 16 x 16 px picture with a shade that differs per `index`, so every image is its own asset.
fn small_picture(index: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(16 * 16 * 4);
    for y in 0..16_u32 {
        for x in 0..16_u32 {
            let g = u8::try_from((x * 8 + y * 4 + index) % 256).unwrap();
            pixels.extend_from_slice(&[g, g, g, 255]);
        }
    }
    encode_png(&WorkingImage {
        width: 16,
        height: 16,
        pixels,
        original_size: (16, 16),
        truncated: false,
        animated: false,
    })
    .unwrap()
}

/// A 400 x 300 mm laser page with 1,000 images of 5 x 5 mm in one Image operation (0.1 mm): the
/// opened session and the operation.
fn thousand_images() -> (Session, NodeId) {
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
    let new: Vec<NewImage> = (0..1_000_u32)
        .map(|i| NewImage {
            bytes: Arc::from(small_picture(i).as_slice()),
            placement: Affine::new(
                5.0,
                0.0,
                0.0,
                5.0,
                o.x + 10.0 + f64::from(i % 40) * 6.0,
                o.y + 10.0 + f64::from(i / 40) * 6.0,
            ),
            name: format!("Photo {}", i + 1),
        })
        .collect();
    let images = d.add_images(&new).unwrap();
    let items: Vec<Vec<NodeId>> = images.iter().map(|&id| vec![id]).collect();
    let engrave = d
        .add_operation(
            m,
            &NewOperation {
                kind: "image",
                name: "Image 1",
            },
            0,
            &items,
        )
        .unwrap();
    d.write_operation_settings(
        SettingsStep::EditSettings,
        &[SettingsWrite {
            operation: engrave,
            value: Some(object(&json!({"main": {"power_pct": 50.0,
                "speed_mm_min": 6000, "passes": 1, "interval_mm": 0.1}}))),
        }],
    )
    .unwrap();
    let bytes = pack(&d, "raster budget").unwrap();
    let mut s = Session::open(3, &bytes).unwrap();
    s.load_machine_profiles_json("[]");
    assert!(s.activate_page(&laser.id));
    s.resize_viewport(1600.0, 1000.0);
    assert!(s.restore_view_text("4 130 85"));
    (s, engrave)
}

/// 37 (d) with 1,000 images in one Image operation (architect review S7): the app frame after an Image
/// setting edit within 50 ms, each tick within 16 ms and the canvas's pictures of the frame
/// within 16 ms, so no step walks the items once per item.
#[test]
fn g_a_thousand_images_edit_tick_and_draw_within_their_budgets() {
    let (mut s, engrave) = thousand_images();
    app_frame(&s);
    let _ = ticks(&s);
    let ((), took) = cpu_time::cpu_time_of(|| {
        assert_eq!(s.edit_operation_option(engrave, "gamma", 1.4), Ok(true));
        app_frame(&s);
    });
    check("37 d 1,000 images edit + app frame", took, 50);
    check("37 d 1,000 images tick", ticks(&s), 16);
    let mut list = s.frame_draw_list();
    let (images, took) = cpu_time::cpu_time_of(|| s.frame_images(&mut list));
    assert!(images.len() >= 1_000, "{} pictures", images.len());
    check("37 d 1,000 images canvas pictures", took, 16);
}
