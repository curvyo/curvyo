//! The AI steps end to end in the session (`specs/0135` slice 3, criteria 24, 47, 48, 63 to 66):
//! a step not run yet skips the picture; the consent line before the first download; a Run with a
//! tiny test net (the real models are not in this build) stores its result in one step "Run
//! Remove background"; the project burns the same dots after a save and reopen with no model;
//! an upstream change marks the result Out of date; Remove stored result.

use curvyo_imageai_core::{ModelFile, ModelKind};
use sha2::{Digest, Sha256};

use super::tests::{dots, image_op, laser_page, place, undo_label};
use crate::session::Session;

#[path = "../../../../curvyo-imageai-core/tests/support/onnx.rs"]
mod onnx;

fn group(s: &Session) -> serde_json::Value {
    serde_json::from_str(&s.adjustments_view_json()).unwrap()
}

fn checks(s: &Session) -> Vec<String> {
    let view: serde_json::Value = serde_json::from_str(&s.output_group_json()).unwrap();
    view["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["text"].as_str().unwrap_or_default().to_owned())
        .collect()
}

/// The fake download: the channel-mean test net, loaded into this worker under its own pin.
fn load_test_net() {
    let net = onnx::channel_mean_net();
    let file = ModelFile {
        kind: ModelKind::Background,
        name: "test-net",
        version: "t",
        sha256: Sha256::digest(&net).into(),
        bytes: net.len() as u64,
        placeholder: false,
        licence: "MIT",
        copyright: "",
        upstream: "",
        training_data: "",
    };
    assert_eq!(crate::ai_glue::load_pinned(&file, &net), "");
}

fn picture() -> (Session, curvyo_document_core::NodeId) {
    let mut s = laser_page();
    let pic = place(&s, 40.0, 20.0);
    image_op(&mut s, &[pic], 0.1);
    s.selection.set(&[pic]);
    (s, pic)
}

/// Runs row `row` as the host does: input, the worker's run, the stored result.
fn run(s: &mut Session, row: usize) -> Result<bool, String> {
    let input = s.ai_run_input(row)?;
    s.stash_ai_pixels(input.image.pixels.clone());
    let pixels = s.take_ai_pixels();
    let out = crate::ai_glue::run_model(
        input.kind,
        pixels,
        (input.image.width, input.image.height),
        input.factor,
        &mut |done, total| {
            s.set_ai_progress(row, done, total);
            true
        },
    )?;
    s.store_ai_result(
        row,
        &input.hash,
        (out.width, out.height),
        out.pixels,
        out.grey,
    )
}

#[test]
fn c47_c48_c63_c65_a_run_stores_its_result_and_the_file_burns_without_the_model() {
    let (mut s, _) = picture();
    let plain = dots(&s);
    assert!(s.add_adjust_step("remove_background"));
    let row = &group(&s)["steps"][0];
    assert_eq!(row["kind"], "remove_background");
    let body = &row["body"];
    assert_eq!(body["kind"], "ai");
    assert_eq!(body["run"], "Run");
    assert_eq!(body["state"], "Model not downloaded yet (4.4 MB).");
    // ADR 0077 §4: not run yet, the picture does not burn; the check says so.
    assert!(dots(&s).is_empty(), "skipped until Run");
    assert!(
        checks(&s)
            .iter()
            .any(|c| c.contains("cannot be read and is skipped")),
        "{:?}",
        checks(&s)
    );
    // Criterion 63: the consent line before the first download.
    assert!(s.set_ai_model_state("background", "asking", 0));
    assert_eq!(
        group(&s)["steps"][0]["body"]["consent"],
        "Downloads the Remove background model (u2netp, 4.4 MB) once from models.curvyo.org. Your picture does not leave this computer."
    );
    assert!(s.set_ai_model_state("background", "unreachable", 0));
    let body = group(&s)["steps"][0]["body"].clone();
    assert_eq!(body["run"], "Try again");
    assert_eq!(
        body["state"],
        "Model host not reachable. Everything else keeps working and stored results still burn."
    );
    assert!(s.set_ai_model_state("background", "ready", 0));
    load_test_net();
    assert_eq!(run(&mut s, 0), Ok(true));
    assert_eq!(undo_label(&mut s), "Run Remove background", "criterion 24");
    let body = group(&s)["steps"][0]["body"].clone();
    assert!(
        body["stored"]
            .as_str()
            .unwrap()
            .starts_with("Stored result ")
    );
    assert!(body["out_of_date"].is_null());
    assert!(
        group(&s)["stored_results"]["line"]
            .as_str()
            .unwrap()
            .starts_with("Stored results ")
    );
    let masked = dots(&s);
    assert!(!masked.is_empty(), "the stored result burns");
    assert_ne!(masked, plain, "the mask changes the dots");
    // Criterion 48's test: another device without the model and with no network opens the file
    // and burns the same dots; the job never calls the model runtime.
    let page = s.active_page().id;
    let bytes = curvyo_document_core::pack(&s.document, "t").unwrap();
    let mut other = Session::open(3, &bytes).unwrap();
    assert!(other.activate_page(&page) || other.active_page().id == page);
    assert_eq!(dots(&other), masked, "byte-identical from the stored PNG");
}

#[test]
fn c48_c66_out_of_date_still_burns_and_remove_stored_result_skips_again() {
    let (mut s, _) = picture();
    assert!(s.add_adjust_step("remove_background"));
    load_test_net();
    assert_eq!(run(&mut s, 0), Ok(true));
    let before = dots(&s);
    // A Crop above it changes the mask's input: Out of date, still burning.
    assert!(s.add_adjust_step("crop"));
    s.close_crop_bar();
    let _ = s.edit_adjust_field(0, "width_pct", 50.0);
    let mask_row = 1;
    let row = group(&s)["steps"][mask_row].clone();
    assert_eq!(
        row["body"]["out_of_date"],
        "Out of date. The stored result still burns."
    );
    assert!(row["line"].as_str().unwrap().starts_with("Out of date · "));
    assert!(!dots(&s).is_empty(), "the job is built and still burns");
    assert!(
        checks(&s).contains(&"1 stored AI result of Photo 1 is out of date.".to_owned()),
        "{:?}",
        checks(&s)
    );
    assert_ne!(dots(&s), Vec::<Vec<u8>>::new());
    assert!(s.remove_adjust_step(0));
    assert_eq!(dots(&s), before, "the same stack burns the same dots");
    // Remove stored result: one step; the picture does not burn until the next Run.
    assert!(s.remove_ai_result(0));
    assert_eq!(undo_label(&mut s), "Edit adjustments");
    assert_eq!(dots(&s), Vec::<Vec<u8>>::new());
    assert!(group(&s)["steps"][0]["body"]["stored"].is_null());
}

#[test]
fn c47_upscale_states_its_size_and_runs_by_the_factor() {
    let (mut s, _) = picture();
    assert!(s.add_adjust_step("upscale"));
    let body = group(&s)["steps"][0]["body"].clone();
    assert_eq!(body["size_line"], "40 × 20 px to 80 × 40 px");
    assert_eq!(body["size_refused"], false);
    let input = s.ai_run_input(0).unwrap();
    assert_eq!((input.kind, input.factor), ("upscale", 2));
    assert_eq!((input.image.width, input.image.height), (40, 20));
    assert_eq!(input.hash.len(), 64);
    s.end_ai_run();
    let add: Vec<String> = group(&s)["add_geometry"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["kind"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        add,
        [
            "crop",
            "resize",
            "upscale",
            "ai_denoise",
            "remove_background"
        ]
    );
}

/// `specs/0143` criteria 5, 17 (ADR 0078 §3): the tiles of a Run computed one by one, in reverse
/// order as a pool may finish them, and assembled on the worker give the same pixels as the
/// whole Run, for AI denoise and Upscale x2; Remove background is not tiled.
#[test]
fn c0143_5_tiles_assembled_on_the_worker_equal_the_whole_run() {
    let pin = |kind, net: &[u8]| ModelFile {
        kind,
        name: "test-net",
        version: "t",
        sha256: Sha256::digest(net).into(),
        bytes: net.len() as u64,
        placeholder: false,
        licence: "MIT",
        copyright: "",
        upstream: "",
        training_data: "",
    };
    for (kind, key, net, factor, scale) in [
        (ModelKind::Denoise, "denoise", onnx::blur_net(), 1_u8, 1_u32),
        (ModelKind::Upscale, "upscale", onnx::blur_x4_net(), 2, 2),
    ] {
        assert_eq!(crate::ai_glue::load_pinned(&pin(kind, &net), &net), "");
        let (w, h) = (300_u32, 250_u32);
        let rgba: Vec<u8> = (0..w * h * 4)
            .map(|i| u8::try_from((i * 37 + i / 1200) % 251).unwrap())
            .collect();
        let whole =
            crate::ai_glue::run_model(key, rgba.clone(), (w, h), factor, &mut |_, _| true).unwrap();
        crate::ai_glue::set_tile_input(rgba, (w, h));
        let count = curvyo_imageai_core::tile_count(w, h);
        assert_eq!(count, 4);
        for index in (0..count).rev() {
            let tile = crate::ai_glue::run_tile(key, factor, index).unwrap();
            assert_eq!(tile.index, index);
            crate::ai_glue::add_tile(tile);
        }
        let assembled = crate::ai_glue::assemble_tiles(w * scale, h * scale).unwrap();
        assert_eq!(assembled, whole.pixels, "{key}");
        assert!(
            crate::ai_glue::run_tile(key, factor, 0).is_err(),
            "the input is forgotten after assembly"
        );
    }
    load_test_net();
    crate::ai_glue::set_tile_input(vec![255; 16], (2, 2));
    assert!(crate::ai_glue::run_tile("background", 1, 0).is_err());
}

/// `specs/0143` criterion 10: a Run on a pool of one worker says "Running on one core" in its
/// progress line until it ends; the next Run starts without it.
#[test]
fn c0143_10_a_pool_of_one_says_running_on_one_core() {
    let (mut s, _) = picture();
    assert!(s.add_adjust_step("remove_background"));
    s.set_ai_model_state("background", "ready", 4);
    let progress = |s: &Session| group(s)["steps"][0]["body"]["progress"]["line"].clone();
    s.ai_run_input(0).unwrap();
    s.set_ai_one_core(0);
    s.set_ai_progress(0, 1, 4);
    assert_eq!(progress(&s), "Running on one core \u{b7} 25 %");
    s.set_ai_progress(0, 2, 4);
    assert_eq!(progress(&s), "Running on one core \u{b7} 50 %");
    s.end_ai_run();
    assert!(progress(&s).is_null());
    s.ai_run_input(0).unwrap();
    s.set_ai_progress(0, 1, 4);
    assert_eq!(progress(&s), "Running \u{b7} 25 %");
}
