//! The font preview's redraw budget (`specs/0077-font-preview` criteria 10 and 12): one
//! keystroke redraws 30 visible tiles, each one line of the sample clipped at the tile's edge, and
//! the wasm side of that stays inside 100 ms, measured as thread CPU time so a busy runner does
//! not fail it.

#![allow(clippy::unwrap_used)]

mod cpu_time;

use curvyo_editor_wasm::PreviewFacesGlue;

fn font(path: &str) -> Vec<u8> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read(dir.join("../frontend/public/fonts").join(path)).unwrap()
}

/// The widest tile at the default size: a column stays under twice its 240 px minimum, at a
/// 32 px cap height that is under 16 cap heights.
const TILE_CAPS: f64 = 16.0;

/// The budget holds for the release build the app ships; the unoptimised test build only prints
/// the time (CI runs this file in release, `performance budgets`).
#[test]
fn thirty_tiles_per_keystroke_fit_the_budget() {
    thirty_tiles_fit("Anna & Jonas 2027");
}

/// The longest sample (200 characters, criterion 12) costs no more, as tiles stop at their edge.
#[test]
fn thirty_tiles_of_the_longest_sample_fit_the_budget() {
    let long: String = "Anna & Jonas 2027 ".chars().cycle().take(200).collect();
    thirty_tiles_fit(&long);
}

fn thirty_tiles_fit(sample: &str) {
    let mut glue = PreviewFacesGlue::default();
    let files = [
        "source-sans-3/SourceSans3-Regular.otf",
        "source-serif-4/SourceSerif4-Regular.ttf",
        "great-vibes/GreatVibes-Regular.ttf",
        "hershey-script-1/HersheyScript1.svg",
        "ems-allure/EMSAllure.svg",
    ];
    for (at, file) in files.iter().enumerate() {
        assert!(glue.add(&at.to_string(), &font(file), 0));
    }
    // Outlines are cached per face after the first draw, as in the running preview.
    for at in 0..files.len() {
        let _ = glue.tile_within(&at.to_string(), "Anna & Jonas 2026", TILE_CAPS);
    }
    let ((), took) = cpu_time::cpu_time_of(|| {
        for i in 0..30 {
            let json = glue.tile_within(&(i % files.len()).to_string(), sample, TILE_CAPS);
            assert_ne!(json, "");
        }
    });
    println!("30 tiles: {took:?} (budget 100 ms)");
    if !cfg!(debug_assertions) {
        assert!(took.as_millis() < 100, "30 tiles took {took:?}");
    }
}
