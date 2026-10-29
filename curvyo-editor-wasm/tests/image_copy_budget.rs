//! Criterion 14 of `specs/0087-image-clipboard-and-drop`: the main thread's part of a copy (the
//! stored pixels of a 16 Mpx image read out of the session) costs at most 50 ms of thread CPU time;
//! the PNG is made in the worker, and its time for a 2000 x 2000 px photo-like picture and for
//! 16 Mpx is printed for the PR, not gated. The budget is asserted in release builds (the
//! `performance-budgets` job); a debug build prints the times.

#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    missing_docs
)]

mod cpu_time;

use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_editor_wasm::{ImportOrigin, Session, encode_rgba_png};

/// A photo-like picture: smooth gradients with noise, opaque.
fn photo(width: u32, height: u32) -> Vec<u8> {
    let mut seed: u32 = 0x1234_5678;
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let noise = (seed >> 27) as u8;
            let r = ((x * 255) / width) as u8;
            let g = ((y * 255) / height) as u8;
            let b = (((x + y) * 127) / (width + height)) as u8;
            pixels.extend_from_slice(&[
                r.saturating_add(noise),
                g.saturating_add(noise),
                b.saturating_add(noise),
                255,
            ]);
        }
    }
    pixels
}

fn report(what: &str, took: Duration) {
    eprintln!(
        "{what}: {:.1} ms of thread CPU time",
        took.as_secs_f64() * 1000.0
    );
}

#[test]
fn copying_a_16_mpx_image_costs_the_main_thread_at_most_50_ms() {
    let (side_w, side_h) = (4000, 4000);
    let pixels = photo(2000, 2000);
    let (small, small_took) = cpu_time_of(|| encode_rgba_png(pixels, 2000, 2000).unwrap());
    report("encode_png 2000 x 2000 px (worker)", small_took);
    eprintln!("  PNG {:.1} MB", small.len() as f64 / 1_048_576.0);
    let pixels = photo(side_w, side_h);
    let (png, big_took) = cpu_time_of(|| encode_rgba_png(pixels, side_w, side_h).unwrap());
    report("encode_png 4000 x 4000 px (worker)", big_took);

    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    session.import_begin_from(ImportOrigin::Paste, None);
    session.import_add(&png, "");
    assert_eq!(session.import_finish(false).imported, 1);
    // The first read after the import is the one a Ctrl+C right after a paste makes.
    let (copy, took) = cpu_time_of(|| session.image_copy().unwrap());
    assert_eq!((copy.width, copy.height), (side_w, side_h));
    report("image_copy 16 Mpx (main thread)", took);
    if !cfg!(debug_assertions) {
        assert!(
            took <= Duration::from_millis(50),
            "the main thread's part took {took:?}"
        );
    }
}
