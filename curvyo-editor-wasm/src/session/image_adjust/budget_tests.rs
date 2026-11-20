//! Session budgets of the Adjustments group (`specs/0135` criterion 55, ADR 0073 §10; Proposal
//! for the session numbers) in thread CPU time, release only (`cargo nextest run --release
//! --run-ignored only`): the group opened on a 4 Mpx picture, the edit frame of a preset pick,
//! the same frame with the reduced Before and after, the worst tick that works the adjustment, and
//! the group and the pane on a page of 1,000 pictures, and the Levels histogram after an edit.

use std::sync::Arc;
use std::time::Duration;

use curvyo_document_core::{Affine, NewImage, NodeId};
use curvyo_image_core::{WorkingImage, encode_png};

use crate::session::Session;
use crate::session::sync_test_support::thread_cpu;

fn check(name: &str, took: Duration, ms: u64) {
    println!("{name}: {took:?} (budget {ms} ms)");
    if !cfg!(debug_assertions) {
        assert!(took <= Duration::from_millis(ms), "{name}: {took:?}");
    }
}

fn png(side: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        for x in 0..side {
            let g = u8::try_from((x / 3 + y / 5 + (x ^ y) % 17) % 256).unwrap();
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

/// A laser page with `count` pictures of `side` px (the first 200 x 200 mm, the rest 2 mm) in
/// one Image operation at 0.1 mm; the session and the first picture.
fn page(side: u32, count: usize) -> (Session, NodeId) {
    let mut s = Session::new(290_155);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let origin = s.active_manufacturing().unwrap().page.unwrap().origin;
    let big: Arc<[u8]> = Arc::from(png(side).as_slice());
    let small: Arc<[u8]> = Arc::from(png(8).as_slice());
    let images: Vec<NewImage> = (0..count)
        .map(|i| NewImage {
            bytes: Arc::clone(if i == 0 { &big } else { &small }),
            placement: if i == 0 {
                Affine::new(200.0, 0.0, 0.0, 200.0, origin.x, origin.y)
            } else {
                #[allow(clippy::cast_precision_loss)]
                let (x, y) = ((i % 40) as f64 * 3.0, 205.0 + (i / 40) as f64 * 3.0);
                Affine::new(2.0, 0.0, 0.0, 2.0, origin.x + x, origin.y + y)
            },
            name: format!("Photo {i}"),
        })
        .collect();
    let ids = s.document.add_images(&images).unwrap();
    s.selection.set(&ids);
    let op = s.press_operation_kind("image", true).unwrap();
    s.edit_operation_setting(op, "main", "power", 50.0).unwrap();
    s.edit_operation_setting(op, "main", "speed", 6000.0)
        .unwrap();
    s.selection.set(&ids[..1]);
    let _ = s.laser_job();
    (s, ids[0])
}

#[test]
#[ignore = "release budget"]
fn c55_the_group_opens_and_a_preset_pick_frame_fits_on_a_4_mpx_picture() {
    let (mut s, _) = page(2_000, 1);
    let mut view = None;
    let first = thread_cpu(|| view = s.adjustments_view());
    assert!(view.is_some());
    check("group open, Expert", first, 50);
    let pick = thread_cpu(|| {
        assert!(s.pick_adjust_preset("photo"));
        let _ = s.job_pending();
        view = s.adjustments_view();
    });
    check("preset pick frame, analysis included", pick, 50 + 150);
    let again = thread_cpu(|| {
        assert!(s.pick_adjust_preset("graphics"));
        let _ = s.job_pending();
        view = s.adjustments_view();
    });
    check("preset pick frame", again, 50);
    // The Dither on the picture (2026-11-20): a press in the picture's Dither strip re-dithers
    // without a new adjusted image, in the same edit frame.
    let dither = thread_cpu(|| {
        assert!(s.set_picture_dither("stucki"));
        let _ = s.job_pending();
        view = s.adjustments_view();
    });
    check("picture's Dither pick frame", dither, 50);
    assert!(s.toggle_before_after());
    let pane = thread_cpu(|| {
        assert!(s.pick_adjust_preset("photo"));
        let after = s.before_after_view().after.unwrap();
        assert!(after.reduced);
        assert_ne!(s.before_after_pixels("after"), Vec::<u8>::new());
    });
    // Criterion 13: the live After of the edit's frame, the reduced adjustment included.
    check(
        "preset pick frame with the reduced Before and after",
        pane,
        100,
    );
    let mut ticks = 0;
    let mut worst = Duration::ZERO;
    while s.job_pending() {
        let took = thread_cpu(|| {
            s.advance_job();
        });
        worst = worst.max(took);
        ticks += 1;
        assert!(ticks < 10_000);
    }
    // Criterion 13: the adjustment is worked a band per tick, never whole in one.
    check("worst tick (a band of the stack)", worst, 50);
}

#[test]
#[ignore = "release budget"]
fn c55_the_group_on_a_page_of_1000_pictures() {
    let (s, _) = page(64, 1_000);
    let mut view = None;
    let took = thread_cpu(|| view = s.adjustments_view());
    assert!(view.is_some());
    check("group open, 1,000 pictures", took, 50);
    let pane = thread_cpu(|| {
        let _ = s.before_after_view();
    });
    check("Before and after view, 1,000 pictures", pane, 35);
}

/// Criteria 16, 55: a field edit above a Levels row on a 4 Mpx picture: the group's view reads the
/// Levels input once (Denoise on the 256 Kpx mip) inside an edit frame, then from the memo.
#[test]
#[ignore = "release budget"]
fn c16_the_levels_histogram_after_an_edit_above_it_fits_a_frame() {
    let (mut s, _) = page(2_000, 1);
    assert!(s.add_adjust_step("denoise"));
    assert!(s.add_adjust_step("levels"));
    assert_eq!(s.edit_adjust_field(0, "strength_pct", 40.0), Ok(true));
    let mut view = None;
    let first = thread_cpu(|| view = s.adjustments_view());
    assert!(view.as_ref().unwrap().steps[1].histogram.is_some());
    check("Levels histogram after an edit above it", first, 50);
    let again = thread_cpu(|| view = s.adjustments_view());
    check("the group again, the histogram kept", again, 5);
}

/// A6: a preset that reaches a picture without the group's frame (a peer's edit, an open
/// document whose job is built): the first tick reads the picture analysis, and no tick is over
/// the 50 ms of criterion 13.
#[test]
#[ignore = "release budget"]
fn c13_the_first_tick_of_a_preset_picture_reads_the_analysis_within_a_tick() {
    let (s, image) = page(2_000, 1);
    let write = curvyo_document_core::AdjustmentsWrite {
        preset: Some("photo".to_owned()),
        ..curvyo_document_core::AdjustmentsWrite::default()
    };
    assert!(s.document.edit_image_adjustments(image, &write).unwrap());
    let frame = thread_cpu(|| assert!(s.job_pending()));
    check("the frame of the edit, no analysis", frame, 50);
    let mut ticks = 0;
    let mut worst = Duration::ZERO;
    while s.job_pending() {
        let took = thread_cpu(|| {
            s.advance_job();
        });
        if ticks == 0 {
            println!("first tick, the analysis included: {took:?}");
        }
        worst = worst.max(took);
        ticks += 1;
        assert!(ticks < 10_000);
    }
    assert!(ticks > 1, "the adjustment is worked in ticks");
    check("worst tick, the analysis included", worst, 50);
}
