//! Tester acceptance cases for `specs/0135-image-optimisation` slice 3 through the editor
//! session, written from the specification (criteria 3, 24, 47, 48, 59, 63 to 66) before the
//! implementation was read. The real models are not in this build: tiny ONNX test nets
//! (`curvyo-imageai-core/tests/support/onnx.rs`) are loaded under their own pins as the fake
//! download. Covered: the AI kinds at their pipeline place, their fields and ranges, each Run one
//! undo step that undo takes back exactly, the stored result in `assets/derived/<sha>.png`
//! burning the same G-code after save and reopen in a thread that never loaded a model, Out of
//! date on a model field or an upstream change (the stored result still burns), the non-model
//! fields applied without the model, Remove stored result, pruning at save, Upscale over the
//! working limit refused before any run, Cancel, the model state lines of criterion 65, the
//! placeholder pins refusing every download, Restore original.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::case_sensitive_file_extension_comparisons,
    dead_code,
    missing_docs
)]

#[path = "../../curvyo-imageai-core/tests/support/onnx.rs"]
mod onnx;

use std::sync::Arc;

use curvyo_document_core::{
    Affine, AssetHash, Direction, Document, DocumentSize, NewImage, NewOperation, NewOutputPage,
    Point, Timestamp, pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool, ai_glue};
use curvyo_image_core::{WorkingImage, encode_png};
use curvyo_imageai_core::{ModelFile, ModelKind};
use serde_json::Value;
use sha2::{Digest, Sha256};

const LASER: &str = "curvyo.laser";

fn png(w: u32, h: u32, mut px: impl FnMut(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            pixels.extend_from_slice(&px(x, y));
        }
    }
    encode_png(&WorkingImage {
        width: w,
        height: h,
        pixels,
        original_size: (w, h),
        truncated: false,
        animated: false,
    })
    .unwrap()
}

/// A dark disc (the subject) on a light, grainy background.
fn photo(w: u32, h: u32) -> Vec<u8> {
    let mut seed = 11u32;
    png(w, h, move |x, y| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let n = (seed >> 24) as i32 % 21 - 10;
        let (dx, dy) = (x as i32 - w as i32 / 2, y as i32 - h as i32 / 2);
        let r = (h as i32 / 3).max(2);
        if dx * dx + dy * dy < r * r {
            [40, 30, 35, 255]
        } else {
            let g = (180 + n + (x as i32 % 7)).clamp(0, 255) as u8;
            // A few translucent pixels, so a model's opaque output is never the original PNG.
            let a = if (x + y) % 5 == 0 { 200 } else { 255 };
            [g, g, g, a]
        }
    })
}

struct Fx {
    s: Session,
    origin: Point,
    page: String,
}

fn configure(s: &mut Session, page: &str) {
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    s.set_clock(|| Timestamp::from_secs(1_762_948_800));
    assert!(s.activate_page(page));
    s.load_materials(&[], None);
    let _ = s.save_machine_profile([13; 16], 1_000);
    let _ = s.set_machine_name("Diode 10 W");
}

/// A laser page with "Photo 1" (`w` x `h` px, 16 x 8 mm at page (20, 20)) in one Image
/// operation at Interval 0.5; the picture selected.
fn setup_with(w: u32, h: u32) -> Fx {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let page = d.pages().into_iter().find(|p| p.id == made.id).unwrap();
    let m = page.kind.manufacturing().unwrap();
    let o = page.origin;
    let ids = d
        .add_images(&[NewImage {
            bytes: Arc::from(photo(w, h).as_slice()),
            placement: Affine::new(16.0, 0.0, 0.0, 8.0, o.x + 20.0, o.y + 20.0),
            name: "Photo 1".to_owned(),
        }])
        .unwrap();
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
    let mut s = Session::open(7, &pack(&d, "0135-s3-tester").unwrap()).unwrap();
    configure(&mut s, &page.id);
    assert!(s.edit_operation_setting(op, "main", "power", 60.0).is_ok());
    assert!(
        s.edit_operation_setting(op, "main", "speed", 3000.0)
            .is_ok()
    );
    let _ = s.edit_operation_option(op, "interval", 0.5);
    let _ = s.edit_operation_option(op, "overscan", 0.0);
    s.wipe_history();
    let mut fx = Fx {
        s,
        origin: o,
        page: page.id.clone(),
    };
    select(&mut fx);
    fx
}

fn setup() -> Fx {
    setup_with(40, 20)
}

fn select(fx: &mut Fx) {
    let at = Point::new(fx.origin.x + 21.0, fx.origin.y + 21.0);
    fx.s.pointer_hover(at, false, false);
    fx.s.pointer_down(at, false);
    fx.s.pointer_up(at, false, false);
    assert!(view(&fx.s).is_object(), "the picture is selected");
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.adjustments_view_json()).unwrap()
}

fn kinds(s: &Session) -> Vec<String> {
    view(s)["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["kind"].as_str().unwrap().to_owned())
        .collect()
}

fn row_of(s: &Session, kind: &str) -> usize {
    kinds(s).iter().position(|k| k == kind).unwrap()
}

fn body(s: &Session, row: usize) -> Value {
    view(s)["steps"][row]["body"].clone()
}

fn field(s: &Session, row: usize, key: &str) -> f64 {
    view(s)["steps"][row]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["key"] == key)
        .unwrap_or_else(|| panic!("field {key} in row {row}: {}", view(s)["steps"][row]))["value"]
        .as_f64()
        .unwrap()
}

fn checks(s: &Session) -> Vec<String> {
    let v: Value = serde_json::from_str(&s.output_group_json()).unwrap();
    v["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["text"].as_str().unwrap_or_default().to_owned())
        .collect()
}

/// The exported G-code, or `None` when the export is blocked.
fn gcode(s: &mut Session) -> Option<Vec<u8>> {
    s.export_job("Untitled.curvyo").map(|e| e.bytes)
}

/// The G-code without its comment lines (the burned moves and powers).
fn moves(s: &mut Session) -> Vec<String> {
    let bytes = gcode(s).unwrap_or_else(|| panic!("export blocked: {:?}", checks(s)));
    String::from_utf8(bytes)
        .unwrap()
        .lines()
        .filter(|l| !l.trim_start().starts_with(';') && !l.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

/// The burning moves: lines with a positive power.
fn burns(s: &mut Session) -> usize {
    moves(s)
        .iter()
        .filter(|l| {
            l.split_whitespace()
                .any(|w| w.starts_with('S') && w[1..].parse::<f64>().is_ok_and(|v| v > 0.0))
        })
        .count()
}

fn regs(s: &Session) -> Value {
    let d = unpack(99, &s.pack("0.1.0").unwrap()).unwrap();
    let objects = d.read_objects();
    let id = objects
        .objects
        .iter()
        .find_map(|o| match o {
            curvyo_document_core::ObjectSnapshot::Image(i) => Some(i.id),
            _ => None,
        })
        .unwrap();
    serde_json::to_value(d.shown_adjustments(id).unwrap()).unwrap()
}

fn ulen(s: &mut Session) -> usize {
    let _ = s.history_target_name(Direction::Undo);
    s.undo_len()
}

fn undo_name(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}

/// The derived members `assets/derived/<64 hex>.png` named in a packed file.
fn derived_members(bytes: &[u8]) -> Vec<String> {
    let tag = b"assets/derived/";
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i + tag.len() + 68 <= bytes.len() {
        if &bytes[i..i + tag.len()] == tag {
            let rest = &bytes[i + tag.len()..i + tag.len() + 68];
            if rest[..64].iter().all(u8::is_ascii_hexdigit) && &rest[64..] == b".png" {
                let name = String::from_utf8(rest[..64].to_vec()).unwrap();
                if !out.contains(&name) {
                    out.push(name);
                }
            }
        }
        i += 1;
    }
    out
}

fn pin(kind: ModelKind, net: &[u8]) -> ModelFile {
    ModelFile {
        kind,
        name: "tester-net",
        version: "t",
        sha256: Sha256::digest(net).into(),
        bytes: net.len() as u64,
        placeholder: false,
        licence: "MIT",
        copyright: "",
        upstream: "",
        training_data: "",
    }
}

/// The fake download: the three test nets in this thread's worker.
fn load_test_nets() {
    for (kind, net) in [
        (ModelKind::Background, onnx::channel_mean_net()),
        (ModelKind::Upscale, onnx::nearest_x4_net()),
        (ModelKind::Denoise, onnx::identity_net()),
    ] {
        assert_eq!(ai_glue::load_pinned(&pin(kind, &net), &net), "");
    }
}

/// A Run of row `row` as the host does it; `go` answers each progress call.
fn run_with(
    s: &mut Session,
    row: usize,
    go: &mut dyn FnMut(u32, u32) -> bool,
) -> Result<bool, String> {
    let input = s.ai_run_input(row)?;
    s.stash_ai_pixels(input.image.pixels.clone());
    let pixels = s.take_ai_pixels();
    let out = match ai_glue::run_model(
        input.kind,
        pixels,
        (input.image.width, input.image.height),
        input.factor,
        &mut |done, total| {
            s.set_ai_progress(row, done, total);
            go(done, total)
        },
    ) {
        Ok(out) => out,
        Err(e) => {
            s.end_ai_run();
            return Err(e);
        }
    };
    s.store_ai_result(
        row,
        &input.hash,
        (out.width, out.height),
        out.pixels,
        out.grey,
    )
}

fn run(s: &mut Session, row: usize) {
    assert_eq!(run_with(s, row, &mut |_, _| true), Ok(true));
}

// ---- criterion 3, 47: the kinds, their place and fields ---------------------------------------

#[test]
fn c3_c47_ai_steps_take_their_pipeline_place() {
    let mut fx = setup();
    for kind in [
        "remove_background",
        "sharpen",
        "levels",
        "ai_denoise",
        "upscale",
        "crop",
    ] {
        assert!(fx.s.add_adjust_step(kind), "{kind}");
        fx.s.close_crop_bar();
    }
    assert_eq!(
        kinds(&fx.s),
        [
            "crop",
            "upscale",
            "ai_denoise",
            "levels",
            "sharpen",
            "remove_background"
        ]
    );
    // Adding Denoise and Resize later still keeps Remove background last.
    assert!(fx.s.add_adjust_step("resize"));
    assert!(fx.s.add_adjust_step("denoise"));
    let k = kinds(&fx.s);
    assert_eq!(k.last().unwrap(), "remove_background");
    assert!(k.iter().position(|x| x == "resize").unwrap() < row_of(&fx.s, "upscale"));
    assert!(row_of(&fx.s, "ai_denoise") < row_of(&fx.s, "denoise"));
}

#[test]
fn c47_fields_defaults_and_ranges() {
    let mut fx = setup();
    for kind in ["upscale", "ai_denoise", "remove_background"] {
        assert!(fx.s.add_adjust_step(kind));
    }
    let (up, dn, bg) = (
        row_of(&fx.s, "upscale"),
        row_of(&fx.s, "ai_denoise"),
        row_of(&fx.s, "remove_background"),
    );
    assert_eq!(field(&fx.s, up, "factor"), 2.0);
    assert_eq!(field(&fx.s, dn, "strength_pct"), 50.0);
    assert_eq!(field(&fx.s, bg, "softness_pct"), 5.0);
    assert_eq!(field(&fx.s, bg, "threshold_pct"), 50.0);
    for (row, key, bad) in [
        (bg, "softness_pct", 21.0),
        (bg, "softness_pct", -1.0),
        (bg, "threshold_pct", 101.0),
        (dn, "strength_pct", 101.0),
        (dn, "strength_pct", -1.0),
        (up, "factor", 3.0),
        (up, "factor", 8.0),
    ] {
        let before = ulen(&mut fx.s);
        let r = fx.s.edit_adjust_field(row, key, bad);
        assert!(r.is_err(), "{key} {bad} refused: {r:?}");
        assert!(!r.unwrap_err().is_empty(), "the range in the message");
        assert_eq!(ulen(&mut fx.s), before, "nothing written");
    }
    assert_eq!(fx.s.edit_adjust_field(bg, "softness_pct", 20.0), Ok(true));
    assert_eq!(fx.s.edit_adjust_field(bg, "threshold_pct", 0.0), Ok(true));
    assert_eq!(fx.s.edit_adjust_field(dn, "strength_pct", 100.0), Ok(true));
    assert_eq!(fx.s.edit_adjust_field(up, "factor", 4.0), Ok(true));
    assert_eq!(field(&fx.s, up, "factor"), 4.0);
}

// ---- criteria 24, 47, 48: Run, undo, the stored result, reopen without the model ---------------

#[test]
fn c24_c48_each_run_is_one_undo_step_and_undo_takes_it_back_exactly() {
    let mut fx = setup();
    let plain = moves(&mut fx.s);
    assert!(fx.s.add_adjust_step("ai_denoise"));
    let dn = row_of(&fx.s, "ai_denoise");
    assert_eq!(body(&fx.s, dn)["run"], "Run");
    // Not run yet: the picture is skipped with a warning (ADR 0077 §4).
    let blocked_or_empty = gcode(&mut fx.s).is_none() || burns(&mut fx.s) == 0;
    assert!(blocked_or_empty, "a step without a result does not burn");
    assert!(!checks(&fx.s).is_empty(), "the checks say why");
    let before_regs = regs(&fx.s);
    load_test_nets();
    assert!(fx.s.set_ai_model_state("denoise", "ready", 0));
    let n = ulen(&mut fx.s);
    run(&mut fx.s, dn);
    assert_eq!(ulen(&mut fx.s), n + 1, "one undo step");
    assert_eq!(undo_name(&mut fx.s), "Run AI denoise");
    let ran = moves(&mut fx.s);
    assert!(burns(&mut fx.s) > 0, "the stored result burns");
    // The identity "model" at Strength 50 gives the input back: the same moves as no step.
    assert_eq!(ran, plain, "identity output burns the plain dots");
    let result = &regs(&fx.s)["adjustments"];
    let text = result.to_string();
    assert!(text.contains("input_hash"), "{text}");
    assert!(
        text.contains("ffdnet-color"),
        "the pinned model name: {text}"
    );
    let _ = fx.s.undo();
    assert_eq!(regs(&fx.s), before_regs, "registers exactly as before");
    let _ = fx.s.redo();
    assert_eq!(moves(&mut fx.s), ran);

    // Upscale and Remove background name their own steps.
    assert!(fx.s.add_adjust_step("upscale"));
    let up = row_of(&fx.s, "upscale");
    run(&mut fx.s, up);
    assert_eq!(undo_name(&mut fx.s), "Run Upscale");
    let text = regs(&fx.s)["adjustments"].to_string();
    assert!(text.contains("realesr-animevideov3"), "{text}");
    assert!(
        text.contains("80") && text.contains("40"),
        "2x of 40 x 20 px: {text}"
    );
    assert!(fx.s.add_adjust_step("remove_background"));
    let bg = row_of(&fx.s, "remove_background");
    // Upscale is above AI denoise now: denoise is out of date (its input changed).
    run(&mut fx.s, bg);
    assert_eq!(undo_name(&mut fx.s), "Run Remove background");
    assert!(regs(&fx.s)["adjustments"].to_string().contains("u2netp"));
}

#[test]
fn c48_stored_results_burn_the_same_gcode_after_reopen_in_a_thread_without_a_model() {
    let mut fx = setup();
    let plain = moves(&mut fx.s);
    load_test_nets();
    for kind in ["upscale", "ai_denoise", "remove_background"] {
        assert!(fx.s.add_adjust_step(kind));
        let row = row_of(&fx.s, kind);
        run(&mut fx.s, row);
    }
    let _ =
        fx.s.edit_adjust_field(row_of(&fx.s, "ai_denoise"), "strength_pct", 80.0);
    for row in 0..3 {
        assert!(
            body(&fx.s, row)["out_of_date"].is_null(),
            "row {row}: {}",
            body(&fx.s, row)
        );
        assert!(
            body(&fx.s, row)["stored"]
                .as_str()
                .unwrap()
                .starts_with("Stored result ")
        );
    }
    let burned = moves(&mut fx.s);
    assert_ne!(burned, plain, "the mask changes the dots");
    let bytes = fx.s.pack("0.1.0").unwrap();
    let mut members = derived_members(&bytes);
    let text = regs(&fx.s)["adjustments"].to_string();
    let mut named: Vec<String> = derived_members(text.replace('"', "").as_bytes());
    if named.is_empty() {
        // The registers name the asset by its hash only.
        named = members
            .iter()
            .filter(|m| text.contains(m.as_str()))
            .cloned()
            .collect();
    }
    members.sort();
    named.sort();
    assert_ne!(members.len(), 0);
    assert_eq!(
        members, named,
        "exactly the results the steps name (equal outputs share one PNG)"
    );
    assert_eq!(
        text.matches("input_hash").count(),
        3,
        "three result records: {text}"
    );
    let d = unpack(5, &bytes).unwrap();
    for m in &members {
        let hash = AssetHash::parse(m).unwrap();
        assert!(d.has_derived_asset(&hash));
        let png = d.asset(&hash).unwrap();
        assert_eq!(
            AssetHash::of(&png),
            hash,
            "the name is the SHA-256 of the bytes"
        );
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }
    let page = fx.page.clone();
    // Another "device": a fresh thread whose worker never loaded a model.
    let other = std::thread::spawn(move || {
        for key in ["background", "upscale", "denoise"] {
            assert!(!ai_glue::model_loaded(key), "{key} is not loaded here");
        }
        let mut s = Session::open(9, &bytes).unwrap();
        configure(&mut s, &page);
        let mut fx2 = Fx {
            s,
            origin: Point::new(0.0, 0.0),
            page,
        };
        let out = moves(&mut fx2.s);
        for key in ["background", "upscale", "denoise"] {
            assert!(!ai_glue::model_loaded(key), "the job loaded no model");
        }
        let checks = checks(&fx2.s);
        (out, checks)
    })
    .join()
    .unwrap();
    assert_eq!(other.0, burned, "byte-identical moves from the stored PNGs");
    assert!(
        !other.1.iter().any(|c| c.contains("out of date")),
        "the input hashes hold after reopen: {:?}",
        other.1
    );
}

// ---- criterion 48: Out of date ----------------------------------------------------------------

#[test]
fn c48_a_model_field_change_is_out_of_date_and_the_stored_result_still_burns() {
    let mut fx = setup();
    load_test_nets();
    assert!(fx.s.add_adjust_step("upscale"));
    run(&mut fx.s, 0);
    let ran = moves(&mut fx.s);
    assert_eq!(fx.s.edit_adjust_field(0, "factor", 4.0), Ok(true));
    let b = body(&fx.s, 0);
    assert_eq!(
        b["out_of_date"], "Out of date. The stored result still burns.",
        "{b}"
    );
    assert_eq!(
        moves(&mut fx.s),
        ran,
        "the dots do not change by themselves"
    );
    assert!(
        checks(&fx.s).contains(&"1 stored AI result of Photo 1 is out of date.".to_owned()),
        "{:?}",
        checks(&fx.s)
    );
    // Run refreshes it.
    run(&mut fx.s, 0);
    assert!(body(&fx.s, 0)["out_of_date"].is_null());
    // The checks read the last built job; build it (the app does on every change).
    let _ = moves(&mut fx.s);
    assert!(
        !checks(&fx.s).iter().any(|c| c.contains("out of date")),
        "the check follows the refreshed result: {:?}, body {}",
        checks(&fx.s),
        body(&fx.s, 0)
    );
    // Back to 2x: the 4x result is out of date again.
    assert_eq!(fx.s.edit_adjust_field(0, "factor", 2.0), Ok(true));
    assert!(!body(&fx.s, 0)["out_of_date"].is_null());
}

#[test]
fn c48_an_upstream_change_is_out_of_date_and_a_non_model_field_is_not() {
    let mut fx = setup();
    load_test_nets();
    assert!(fx.s.add_adjust_step("ai_denoise"));
    assert!(fx.s.add_adjust_step("remove_background"));
    run(&mut fx.s, 0);
    run(&mut fx.s, 1);
    let at50 = moves(&mut fx.s);
    // Strength, Threshold and Edge softness are applied to the stored PNG: no Out of date.
    assert_eq!(fx.s.edit_adjust_field(1, "threshold_pct", 0.0), Ok(true));
    assert_eq!(fx.s.edit_adjust_field(1, "softness_pct", 0.0), Ok(true));
    assert_eq!(fx.s.edit_adjust_field(0, "strength_pct", 0.0), Ok(true));
    assert!(body(&fx.s, 0)["out_of_date"].is_null());
    assert!(
        body(&fx.s, 1)["out_of_date"].is_null(),
        "{}",
        body(&fx.s, 1)
    );
    let tighter = moves(&mut fx.s);
    assert_ne!(tighter, at50, "Threshold changes the dots without a model");
    // A step above the mask changes its input: Out of date, it still burns.
    assert!(fx.s.add_adjust_step("levels"));
    let lv = row_of(&fx.s, "levels");
    let _ = fx.s.edit_adjust_field(lv, "black_pct", 20.0);
    let bg = row_of(&fx.s, "remove_background");
    let k = kinds(&fx.s);
    assert!(lv < bg, "{k:?}");
    // The mask reads the picture after AI denoise (criterion 3), so Levels after AI denoise does
    // not change its input; only a change at or above AI denoise does.
    assert!(
        body(&fx.s, bg)["out_of_date"].is_null(),
        "criterion 3: the mask is computed from the picture after AI denoise: {}",
        body(&fx.s, bg)
    );
    assert!(fx.s.add_adjust_step("resize"));
    let rs = row_of(&fx.s, "resize");
    let _ = fx.s.edit_adjust_field(rs, "width_px", 30.0);
    let dn = row_of(&fx.s, "ai_denoise");
    let bg = row_of(&fx.s, "remove_background");
    assert!(
        !body(&fx.s, dn)["out_of_date"].is_null(),
        "{}",
        body(&fx.s, dn)
    );
    assert!(
        !body(&fx.s, bg)["out_of_date"].is_null(),
        "{}",
        body(&fx.s, bg)
    );
    assert!(burns(&mut fx.s) > 0, "an Out of date result still burns");
    assert!(
        checks(&fx.s).contains(&"2 stored AI results of Photo 1 are out of date.".to_owned()),
        "{:?}",
        checks(&fx.s)
    );
}

// ---- criteria 59, 66: Remove stored result, pruning, budget ------------------------------------

#[test]
fn c66_remove_stored_result_is_one_step_and_the_picture_is_skipped_until_run() {
    let mut fx = setup();
    load_test_nets();
    assert!(fx.s.add_adjust_step("ai_denoise"));
    run(&mut fx.s, 0);
    let ran = moves(&mut fx.s);
    let line = view(&fx.s)["stored_results"]["line"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        line.starts_with("Stored results ") && line.ends_with(" MB of 512 MB"),
        "{line}"
    );
    assert_eq!(view(&fx.s)["stored_results"]["warns"], false);
    let n = ulen(&mut fx.s);
    assert!(fx.s.remove_ai_result(0));
    assert_eq!(ulen(&mut fx.s), n + 1);
    assert!(body(&fx.s, 0)["stored"].is_null());
    let skipped = gcode(&mut fx.s).is_none() || burns(&mut fx.s) == 0;
    assert!(skipped, "skipped with a warning until Run");
    assert!(!fx.s.remove_ai_result(0), "nothing left to remove");
    let _ = fx.s.undo();
    assert_eq!(moves(&mut fx.s), ran, "undo brings the stored result back");
}

/// Opens `saved` in a new session with the picture selected.
fn reopen(saved: &[u8], fx: &Fx, peer: u64) -> Fx {
    let mut s = Session::open(peer, saved).unwrap();
    configure(&mut s, &fx.page);
    let mut out = Fx {
        s,
        origin: fx.origin,
        page: fx.page.clone(),
    };
    select(&mut out);
    out
}

#[test]
fn c59_c66_results_no_step_and_no_undo_refers_to_are_dropped_at_save() {
    let mut fx = setup();
    load_test_nets();
    assert!(fx.s.add_adjust_step("ai_denoise"));
    run(&mut fx.s, 0);
    let saved = fx.s.pack("0.1.0").unwrap();
    assert_eq!(derived_members(&saved).len(), 1);
    let mut fx2 = reopen(&saved, &fx, 8);
    assert!(fx2.s.remove_ai_result(0));
    fx2.s.wipe_history();
    let pruned = fx2.s.pack("0.1.0").unwrap();
    let s3 = Session::open(10, &pruned).unwrap();
    let third = s3.pack("0.1.0").unwrap();
    assert_eq!(derived_members(&third).len(), 0, "dropped at save");
    eprintln!(
        "tester note: after Wipe history the same session saved {} derived member(s)",
        derived_members(&pruned).len()
    );
    // Restore original empties the stack, so its results go too (criterion 59).
    let mut fx3 = setup();
    let plain = moves(&mut fx3.s);
    assert!(fx3.s.add_adjust_step("remove_background"));
    run(&mut fx3.s, 0);
    assert!(fx3.s.restore_original_adjustments());
    assert_eq!(moves(&mut fx3.s), plain, "the dots of 0123 again");
}

/// Criteria 24 and 59: a save between Remove stored result and Undo keeps what the undo state
/// refers to, in a session that opened the file (did not run the model itself).
#[test]
fn c24_c59_undo_after_a_save_brings_the_removed_result_back_in_an_opened_file() {
    let mut fx = setup();
    load_test_nets();
    assert!(fx.s.add_adjust_step("ai_denoise"));
    run(&mut fx.s, 0);
    let ran = moves(&mut fx.s);
    let saved = fx.s.pack("0.1.0").unwrap();
    let mut fx2 = reopen(&saved, &fx, 8);
    assert_eq!(moves(&mut fx2.s), ran);
    assert!(fx2.s.remove_ai_result(0));
    let with_undo = fx2.s.pack("0.1.0").unwrap();
    let kept = derived_members(&with_undo).len();
    let _ = fx2.s.undo();
    assert!(
        !body(&fx2.s, 0)["stored"].is_null(),
        "undo restores the record"
    );
    let after = gcode(&mut fx2.s).map(|_| moves(&mut fx2.s));
    assert_eq!(
        after.as_ref(),
        Some(&ran),
        "undo after a save burns the stored result again (saved file kept {kept} member(s)); checks {:?}",
        checks(&fx2.s)
    );
    // The asset is not lost: the next save after the undo carries it again.
    let again = fx2.s.pack("0.1.0").unwrap();
    let mut fx3 = reopen(&again, &fx, 12);
    let reopened = gcode(&mut fx3.s).map(|_| moves(&mut fx3.s));
    assert_eq!(
        reopened.as_ref(),
        Some(&ran),
        "the file saved after the undo burns the result (it has {} derived member(s)); checks {:?}",
        derived_members(&again).len(),
        checks(&fx3.s)
    );
    assert_eq!(derived_members(&again).len(), 1, "saved again after undo");
    eprintln!(
        "tester note: the save while only the undo state referred to it kept {kept} member(s)"
    );
}

/// The same in the session that ran the model.
#[test]
fn c24_c59_undo_after_a_save_brings_the_removed_result_back_in_the_running_session() {
    let mut fx = setup();
    load_test_nets();
    assert!(fx.s.add_adjust_step("ai_denoise"));
    run(&mut fx.s, 0);
    let ran = moves(&mut fx.s);
    assert!(fx.s.remove_ai_result(0));
    let with_undo = fx.s.pack("0.1.0").unwrap();
    let _ = fx.s.undo();
    assert_eq!(moves(&mut fx.s), ran, "undo after a save burns it again");
    assert_eq!(
        derived_members(&with_undo).len(),
        1,
        "kept while undo refers to it"
    );
}

#[test]
fn c66_the_budget_line_warns_from_384_mib() {
    use curvyo_ui_core::{STORED_RESULTS_WARN_BYTES, budget_line, result_refusal};
    assert_eq!(STORED_RESULTS_WARN_BYTES, 384 * 1024 * 1024);
    assert_eq!(
        budget_line(120 * 1024 * 1024),
        ("Stored results 120 MB of 512 MB".to_owned(), false)
    );
    assert!(!budget_line(384 * 1024 * 1024 - 1).1);
    assert!(budget_line(384 * 1024 * 1024).1);
    let one = result_refusal(
        curvyo_document_core::AssetError::ResultTooLarge { bytes: 70 << 20 },
        70 << 20,
    );
    assert!(one.contains("64 MB"), "{one}");
    let all = result_refusal(
        curvyo_document_core::AssetError::ResultsFull { adds: 10 << 20 },
        10 << 20,
    );
    assert!(all.contains("512 MB"), "{all}");
}

// ---- criterion 47: Upscale limit, Cancel ------------------------------------------------------

#[test]
fn c47_upscale_over_the_working_limit_is_refused_before_running() {
    let mut fx = setup_with(2000, 1500);
    load_test_nets();
    assert!(fx.s.add_adjust_step("upscale"));
    let b = body(&fx.s, 0);
    assert_eq!(b["size_refused"], false, "2x is 4,000 x 3,000 px: {b}");
    assert_eq!(b["size_line"], "2,000 × 1,500 px to 4,000 × 3,000 px");
    assert_eq!(fx.s.edit_adjust_field(0, "factor", 4.0), Ok(true));
    let b = body(&fx.s, 0);
    assert_eq!(b["size_refused"], true, "{b}");
    assert!(
        b["size_line"].as_str().unwrap().contains("16 megapixels"),
        "{b}"
    );
    let n = ulen(&mut fx.s);
    let mut ran = false;
    let r = run_with(&mut fx.s, 0, &mut |_, _| {
        ran = true;
        true
    });
    assert!(r.is_err(), "{r:?}");
    assert!(!ran, "no tile ran");
    assert_eq!(ulen(&mut fx.s), n);
    assert!(body(&fx.s, 0)["progress"].is_null());
    assert!(body(&fx.s, 0)["stored"].is_null());
}

#[test]
fn c47_cancel_leaves_no_result_and_no_undo_step() {
    let mut fx = setup_with(600, 300);
    load_test_nets();
    assert!(fx.s.add_adjust_step("ai_denoise"));
    let n = ulen(&mut fx.s);
    let seen_progress;
    let r = {
        let input = fx.s.ai_run_input(0).unwrap();
        let pixels = input.image.pixels.clone();
        let mut calls = 0;
        let out = ai_glue::run_model(
            input.kind,
            pixels,
            (input.image.width, input.image.height),
            input.factor,
            &mut |done, total| {
                calls += 1;
                done < 1 || total == 0
            },
        );
        fx.s.set_ai_progress(0, 1, 4);
        seen_progress = body(&fx.s, 0)["progress"].clone();
        fx.s.end_ai_run();
        out
    };
    assert!(r.is_err(), "cancelled");
    assert!(!seen_progress.is_null(), "a progress bar while running");
    assert_eq!(ulen(&mut fx.s), n);
    assert!(body(&fx.s, 0)["stored"].is_null());
    assert!(body(&fx.s, 0)["progress"].is_null());
    assert_eq!(body(&fx.s, 0)["run"], "Run");
}

// ---- criteria 63 to 65: model states, consent, placeholders ------------------------------------

#[test]
fn c63_c64_c65_the_model_state_lines() {
    let mut fx = setup();
    for kind in ["upscale", "ai_denoise", "remove_background"] {
        assert!(fx.s.add_adjust_step(kind));
    }
    let (up, dn, bg) = (0, 1, 2);
    assert_eq!(body(&fx.s, bg)["run"], "Run");
    assert_eq!(
        body(&fx.s, bg)["state"],
        "Model not downloaded yet (4.4 MB)."
    );
    assert_eq!(
        body(&fx.s, up)["state"],
        "Model not downloaded yet (2.4 MB)."
    );
    assert_eq!(
        body(&fx.s, dn)["state"],
        "Model not downloaded yet (3.3 MB)."
    );
    assert!(
        body(&fx.s, bg)["consent"].is_null(),
        "no consent before Run"
    );
    assert!(fx.s.set_ai_model_state("denoise", "asking", 0));
    assert_eq!(
        body(&fx.s, dn)["consent"],
        "Downloads the AI denoise model (ffdnet-color, 3.3 MB) once from models.curvyo.org. Your picture does not leave this computer."
    );
    assert!(fx.s.set_ai_model_state("denoise", "downloading", 1024 * 1024));
    let p = body(&fx.s, dn)["progress"].clone();
    assert_eq!(p["line"], "Downloading 1.0 of 3.3 MB", "{p}");
    let f = p["fraction"].as_f64().unwrap();
    assert!(f > 0.29 && f < 0.31, "{f}");
    assert!(fx.s.set_ai_model_state("denoise", "unreachable", 0));
    assert_eq!(body(&fx.s, dn)["run"], "Try again");
    assert_eq!(
        body(&fx.s, dn)["state"],
        "Model host not reachable. Everything else keeps working and stored results still burn."
    );
    assert!(fx.s.set_ai_model_state("denoise", "checksum", 0));
    let b = body(&fx.s, dn);
    assert_eq!(
        b["state"],
        "The downloaded model did not match its checksum and was removed."
    );
    assert_eq!(b["alert"], true);
    assert_eq!(b["run"], "Try again");
    assert!(fx.s.set_ai_model_state("denoise", "paused", 1024 * 1024));
    assert!(
        body(&fx.s, dn)["state"]
            .as_str()
            .unwrap()
            .contains("paused")
    );
    assert_eq!(
        body(&fx.s, dn)["run"],
        "Resume",
        "UX review: a paused download resumes"
    );
    assert!(fx.s.set_ai_model_state("denoise", "ready", 0));
    assert_eq!(body(&fx.s, dn)["model_ready"], true);
    assert_eq!(body(&fx.s, dn)["run"], "Run");
    // The other steps are unaffected by the denoise model's states.
    assert_eq!(
        body(&fx.s, bg)["state"],
        "Model not downloaded yet (4.4 MB)."
    );
    assert!(!fx.s.set_ai_model_state("denoise", "nonsense", 0));
}

#[test]
fn c63_c64_the_pins_refuse_every_other_downloaded_file() {
    let json: Value = serde_json::from_str(&ai_glue::models_json()).unwrap();
    let list = json
        .as_array()
        .unwrap_or_else(|| json["models"].as_array().unwrap());
    assert_eq!(list.len(), 3);
    for m in list {
        let url = m["url"].as_str().unwrap();
        assert!(url.starts_with("https://models.curvyo.org/"), "{url}");
        assert!(url.ends_with(".onnx") && !url.contains('?'), "{url}");
        // Pinned 2026-11-21 (lead): the uploaded files; every other file is still refused.
        assert_eq!(
            m["placeholder"], false,
            "this build pins the uploaded files"
        );
        assert_eq!(m["sha256"].as_str().unwrap().len(), 64);
        assert_ne!(m["sha256"], "0".repeat(64));
        let key = m["kind"].as_str().unwrap();
        let net = onnx::identity_net();
        assert!(!ai_glue::verify_model(key, &net), "{key}");
        assert!(!ai_glue::load_model(key, &net).is_empty(), "{key} refused");
        assert!(!ai_glue::model_loaded(key));
    }
}
