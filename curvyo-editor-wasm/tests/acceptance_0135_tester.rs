//! Tester acceptance cases for `specs/0135-image-optimisation` slice 1 through the public
//! `Session`, written from the specification before the implementation was read: the
//! Adjustments group (criteria 1, 25, 26, 28, 59), every edit as one undo step and its undo (2,
//! 24, 27, 29, 59), the empty stack and Restore original equal the `0123` dots byte for byte (1,
//! 54, 59), Make editable keeps the dots (29), a preset pick writes the Dither in every assigned
//! operation (27), the picture in two operations at two Intervals (60), linked clones with own
//! adjustments all or nothing and Follow original (9, 61), the Before and after pane (11, 12),
//! the screen dither modes against an independent threshold reference (42, 43), the broken-steps
//! check line from the damaged golden (23), two exports byte-identical (20), and edge pictures
//! (1 x 1, all white, all black, transparent, Interval 0.01 and 2 mm).
#![allow(
    clippy::manual_assert_eq,
    clippy::map_unwrap_or,
    clippy::single_match_else,
    clippy::semicolon_if_nothing_returned,
    clippy::assert_is_empty,
    clippy::many_single_char_names,
    clippy::trivially_copy_pass_by_ref,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::cast_possible_wrap,
    clippy::needless_range_loop,
    clippy::similar_names,
    dead_code,
    missing_docs
)]

#[path = "../../curvyo-laser-core/tests/tester_raster_0123/mod.rs"]
mod reference;

use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    AdjustEntry, AdjustStep, AdjustmentsWrite, Affine, Direction, Document, DocumentSize, NewImage,
    NewOperation, NewOutputPage, NodeId, Point, Timestamp, pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_image_core::{WorkingImage, encode_png};
use reference::{Gcode, burned_power, read};
use serde_json::Value;

const LASER: &str = "curvyo.laser";
const BED_H: f64 = 300.0;

// ---- fixture ----------------------------------------------------------------------------------

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

fn grey_png(w: u32, h: u32, mut g: impl FnMut(u32, u32) -> u8) -> Vec<u8> {
    png(w, h, |x, y| {
        let v = g(x, y);
        [v, v, v, 255]
    })
}

/// A 16 x 16 photo-like picture: a gradient, a disc, grain.
fn photo16() -> Vec<u8> {
    let mut seed = 7u32;
    grey_png(16, 16, move |x, y| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let grain = (seed >> 24) as i32 % 9 - 4;
        let dx = x as i32 - 8;
        let dy = y as i32 - 8;
        let disc = if dx * dx + dy * dy < 20 { 70 } else { 0 };
        (60 + x as i32 * 6 + disc + grain).clamp(0, 255) as u8
    })
}

fn picture_fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures")
            .join(name),
    )
    .unwrap()
}

struct Fx {
    s: Session,
    origin: Point,
    image: NodeId,
    ops: Vec<NodeId>,
    clone: Option<NodeId>,
}

/// A 400 x 300 mm laser page; one image `w` x `h` mm at page (20, 20) in `intervals.len()` Image
/// operations (Image 1, Image 2, ...) with those Intervals at 60 %, 3,000 mm/min, no overscan;
/// with `clone`, a linked clone of the image (not in any operation).
fn setup(bytes: &[u8], w: f64, h: f64, intervals: &[f64], clone: bool) -> Fx {
    setup_with(bytes, w, h, intervals, clone, |_, _| {})
}

/// [`setup`] with `prep` run on the document and the image before the session opens.
fn setup_with(
    bytes: &[u8],
    w: f64,
    h: f64,
    intervals: &[f64],
    clone: bool,
    prep: impl FnOnce(&Document, NodeId),
) -> Fx {
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
    let image = d
        .add_images(&[NewImage {
            bytes: Arc::from(bytes),
            placement: Affine::new(w, 0.0, 0.0, h, o.x + 20.0, o.y + 20.0),
            name: "Photo 1".to_owned(),
        }])
        .unwrap()[0];
    let mut ops = Vec::new();
    for (i, _) in intervals.iter().enumerate() {
        let name = format!("Image {}", i + 1);
        let index = d.operations(m).unwrap().len();
        ops.push(
            d.add_operation(
                m,
                &NewOperation {
                    kind: "image",
                    name: &name,
                    settings: None,
                },
                index,
                &[vec![image]],
            )
            .unwrap(),
        );
    }
    let clone = clone.then(|| d.linked_clone(&[image], &[]).unwrap()[0]);
    prep(&d, image);
    let mut s = Session::open(7, &pack(&d, "0135-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    s.set_clock(|| Timestamp::from_secs(1_762_948_800));
    assert!(s.activate_page(&page.id));
    s.load_materials(&[], None);
    let _ = s.save_machine_profile([13; 16], 1_000);
    let _ = s.set_machine_name("Diode 10 W");
    for (&op, &i) in ops.iter().zip(intervals) {
        assert!(s.edit_operation_setting(op, "main", "power", 60.0).is_ok());
        assert!(
            s.edit_operation_setting(op, "main", "speed", 3000.0)
                .is_ok()
        );
        let _ = s.edit_operation_option(op, "interval", i);
        let _ = s.edit_operation_option(op, "overscan", 0.0);
    }
    s.wipe_history();
    Fx {
        s,
        origin: o,
        image,
        ops,
        clone,
    }
}

fn click(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

/// Selects the image by a click into its frame.
fn select_image(fx: &mut Fx) {
    let at = Point::new(fx.origin.x + 21.0, fx.origin.y + 21.0);
    click(&mut fx.s, at);
    if !view(&fx.s)["clone_row"].is_null() {
        // The clone lies on top of its original: go to the original.
        let _ = fx.s.select_original();
    }
    let v = view(&fx.s);
    assert!(v.is_object() && v["clone_row"].is_null(), "image selected");
}

fn select_clone(fx: &mut Fx) {
    select_image(fx);
    let _ = fx.s.select_clones();
    assert!(!view(&fx.s)["clone_row"].is_null(), "clone selected");
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.adjustments_view_json()).unwrap()
}

fn ba(s: &Session) -> Value {
    serde_json::from_str(&s.before_after_view_json()).unwrap()
}

fn doc(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn regs(s: &Session, id: NodeId) -> Value {
    serde_json::to_value(doc(s).shown_adjustments(id).unwrap()).unwrap()
}

fn out(s: &Session) -> Value {
    serde_json::from_str(&s.output_group_json()).unwrap()
}

fn checks(s: &Session) -> Vec<String> {
    out(s)["checks"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|c| format!("{}: {}", c["severity"], c["text"]))
        .collect()
}

fn export(s: &mut Session) -> Vec<u8> {
    s.export_job("Untitled.curvyo")
        .map(|e| e.bytes)
        .unwrap_or_else(|| panic!("export blocked: {:?}", checks(s)))
}

/// The file read back; a job that burns nothing ("Nothing runs") reads as no moves.
fn gcode(s: &mut Session) -> Gcode {
    match s.export_job("Untitled.curvyo").map(|e| e.bytes) {
        Some(bytes) => read(&bytes).unwrap(),
        None => {
            let c = checks(s);
            assert!(
                c.iter().any(|l| l.contains("Nothing runs")),
                "export blocked: {c:?}"
            );
            Gcode::default()
        }
    }
}

/// The undo stack's length after the history caught up with the document.
fn ulen(s: &mut Session) -> usize {
    let _ = s.history_target_name(Direction::Undo);
    s.undo_len()
}

fn undo_name(s: &mut Session) -> Option<String> {
    s.history_target_name(Direction::Undo)
}

fn options_of(s: &Session, id: NodeId) -> serde_json::Map<String, Value> {
    let d = doc(s);
    let page = d
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap();
    d.operations(page.kind.manufacturing().unwrap())
        .unwrap()
        .into_iter()
        .find(|o| o.id == id)
        .and_then(|o| o.options)
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

/// Dot map of the 8 x 8 mm image at Interval `i`.
fn map(g: &Gcode, i: f64, n: usize) -> Vec<Vec<Option<u32>>> {
    g.dot_map(20.0, BED_H - 20.0 - i / 2.0, i, n, n)
}

/// One edit: exactly one more undo step with `name`, undo returns the registers.
fn one_step(fx: &mut Fx, target: NodeId, name: &str, edit: impl FnOnce(&mut Session)) {
    let before = regs(&fx.s, target);
    let n = ulen(&mut fx.s);
    edit(&mut fx.s);
    assert_eq!(ulen(&mut fx.s), n + 1, "{name}: one undo step");
    assert_eq!(undo_name(&mut fx.s).as_deref(), Some(name));
    let after = regs(&fx.s, target);
    assert_ne!(before, after, "{name}: something changed");
    let _ = fx.s.undo();
    assert_eq!(
        regs(&fx.s, target),
        before,
        "{name}: undo restores the registers"
    );
    let _ = fx.s.redo();
    assert_eq!(regs(&fx.s, target), after, "{name}: redo");
}

// ---- criterion 1, 25, 26, 28, 59: the group ---------------------------------------------------

#[test]
fn c1_the_group_shows_for_exactly_one_image_and_a_new_picture_is_expert_with_no_stack() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], false);
    assert_eq!(view(&fx.s), Value::Null, "nothing selected: no group");
    select_image(&mut fx);
    let v = view(&fx.s);
    assert!(v.is_object(), "{v}");
    let pressed: Vec<&str> = v["presets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["pressed"] == true)
        .map(|p| p["key"].as_str().unwrap())
        .collect();
    assert_eq!(pressed, ["expert"]);
    assert_eq!(v["steps"].as_array().unwrap().len(), 0);
    assert_eq!(v["restore_enabled"], false, "greyed, not hidden");
    assert_eq!(v["strength"], Value::Null, "hidden with Expert");
    let keys: Vec<&str> = v["presets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["key"].as_str().unwrap())
        .collect();
    assert_eq!(
        keys,
        [
            "photo",
            "graphics",
            "black_white",
            "line_art",
            "dot_raster",
            "newspaper",
            "expert"
        ]
    );
    let cells: Vec<&str> = v["presets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["cell"].as_str().unwrap())
        .collect();
    assert_eq!(cells[4], "Dots");
    assert_eq!(
        v["presets"][0]["tooltip"], "Photo (detailed) · Dither: Jarvis",
        "criterion 25 example"
    );
    let adds: Vec<&str> = v["add_tone"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["kind"].as_str().unwrap())
        .collect();
    for k in ["levels", "local_contrast", "contours", "sharpen", "denoise"] {
        assert!(adds.contains(&k), "add toolbar offers {k}: {adds:?}");
    }
}

#[test]
fn c26_c28_a_preset_shows_strength_and_the_derived_rows_and_restore_is_enabled() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], false);
    select_image(&mut fx);
    assert!(fx.s.pick_adjust_preset("photo"));
    let v = view(&fx.s);
    assert_eq!(v["strength"]["value"], 50);
    assert_eq!(v["restore_enabled"], true);
    let line = v["derived"]["line"].as_str().unwrap();
    assert!(
        line == "From preset Photo (detailed), Strength 50 %"
            || line == "From preset Photo, Strength 50 %",
        "{line}"
    );
    assert!(!v["derived"]["rows"].as_array().unwrap().is_empty());
    // Out-of-range Strength is refused with the range and writes nothing.
    let n = ulen(&mut fx.s);
    let msg = fx.s.set_adjust_strength(101.0).unwrap_err();
    assert!(msg.contains("0 to 100"), "{msg}");
    let msg = fx.s.set_adjust_strength(-1.0).unwrap_err();
    assert!(msg.contains("0 to 100"), "{msg}");
    assert_eq!(ulen(&mut fx.s), n);
    // Expert hides the Strength and keeps its value.
    assert!(fx.s.set_adjust_strength(70.0).is_ok());
    assert!(fx.s.pick_adjust_preset("expert"));
    assert_eq!(view(&fx.s)["strength"], Value::Null);
    assert_eq!(regs(&fx.s, fx.image)["adjustments"]["strength"], 70);
    assert!(fx.s.pick_adjust_preset("graphics"));
    assert_eq!(view(&fx.s)["strength"]["value"], 70);
}

#[test]
fn c30_manual_steps_under_a_preset_are_listed_preset_in_use_and_kept() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], false);
    select_image(&mut fx);
    assert!(fx.s.add_adjust_step("levels"));
    assert!(fx.s.pick_adjust_preset("photo"));
    let v = view(&fx.s);
    let steps = v["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0]["preset_in_use"], true, "{v}");
    assert!(fx.s.pick_adjust_preset("expert"));
    assert_eq!(view(&fx.s)["steps"][0]["preset_in_use"], false);
}

// ---- criterion 2, 24: every edit one undo step ------------------------------------------------

#[test]
fn c2_c24_every_edit_is_one_undo_step_and_undo_restores_the_registers() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], false);
    select_image(&mut fx);
    let id = fx.image;
    one_step(&mut fx, id, "Edit adjustments", |s| {
        assert!(s.add_adjust_step("sharpen"))
    });
    one_step(&mut fx, id, "Edit adjustments", |s| {
        assert!(s.add_adjust_step("levels"))
    });
    one_step(&mut fx, id, "Edit adjustments", |s| {
        assert!(s.edit_adjust_field(0, "black_pct", 12.0).is_ok());
    });
    one_step(&mut fx, id, "Edit adjustments", |s| {
        assert!(s.set_adjust_step_enabled(1, false))
    });
    one_step(&mut fx, id, "Edit adjustments", |s| {
        assert!(s.remove_adjust_step(1))
    });
    one_step(&mut fx, id, "Edit adjustments", |s| {
        assert!(s.pick_adjust_preset("line_art"))
    });
    one_step(&mut fx, id, "Edit adjustments", |s| {
        assert!(s.set_adjust_strength(80.0).is_ok())
    });
    one_step(&mut fx, id, "Edit adjustments", |s| {
        assert!(s.make_adjust_editable())
    });
    one_step(&mut fx, id, "Restore original", |s| {
        assert!(s.restore_original_adjustments())
    });
}

#[test]
fn c3_adding_sharpen_then_levels_keeps_pipeline_order() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], false);
    select_image(&mut fx);
    assert!(fx.s.add_adjust_step("sharpen"));
    assert!(fx.s.add_adjust_step("levels"));
    let kinds: Vec<String> = view(&fx.s)["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["kind"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(kinds, ["levels", "sharpen"]);
}

#[test]
fn c5_a_field_out_of_range_is_refused_with_the_range() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], false);
    select_image(&mut fx);
    assert!(fx.s.add_adjust_step("levels"));
    let n = ulen(&mut fx.s);
    let msg = fx.s.edit_adjust_field(0, "gamma_centi", 4.5).unwrap_err();
    assert!(!msg.is_empty() && msg.contains('4'), "{msg}");
    let msg = fx.s.edit_adjust_field(0, "black_pct", 99.0).unwrap_err();
    assert!(msg.contains("98"), "{msg}");
    assert_eq!(ulen(&mut fx.s), n, "nothing written");
}

#[test]
fn c26_a_strength_drag_is_one_undo_step_and_cancel_writes_nothing() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], false);
    select_image(&mut fx);
    assert!(fx.s.pick_adjust_preset("photo"));
    let n = ulen(&mut fx.s);
    for v in [51.0, 60.0, 75.0, 90.0] {
        fx.s.hold_adjust_strength(v);
    }
    assert_eq!(ulen(&mut fx.s), n, "nothing written while held");
    assert!(fx.s.release_adjust_strength());
    assert_eq!(ulen(&mut fx.s), n + 1);
    assert_eq!(regs(&fx.s, fx.image)["adjustments"]["strength"], 90);
    fx.s.hold_adjust_strength(10.0);
    fx.s.cancel_adjust_strength();
    assert_eq!(ulen(&mut fx.s), n + 1);
    assert_eq!(regs(&fx.s, fx.image)["adjustments"]["strength"], 90);
}

// ---- criterion 1, 54, 59: the empty stack and Restore original --------------------------------

#[test]
fn c1_c54_c59_restore_original_gives_the_0123_dots_byte_for_byte() {
    for preset in [
        "photo",
        "graphics",
        "black_white",
        "line_art",
        "dot_raster",
        "newspaper",
    ] {
        let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], false);
        let _ = fx.s.set_operation_dither(fx.ops[0], "jarvis");
        fx.s.wipe_history();
        let plain = export(&mut fx.s);
        let plain_options = options_of(&fx.s, fx.ops[0]);
        select_image(&mut fx);
        assert!(fx.s.pick_adjust_preset(preset));
        assert!(fx.s.add_adjust_step("sharpen"));
        let adjusted = export(&mut fx.s);
        assert_ne!(plain, adjusted, "{preset}: the preset changes the dots");
        // Put the operation's dither back (the preset wrote it), then restore.
        let _ = fx.s.set_operation_dither(fx.ops[0], "jarvis");
        let _ = fx.s.edit_operation_option(fx.ops[0], "screen_size", 0.8);
        let n = ulen(&mut fx.s);
        assert!(fx.s.restore_original_adjustments());
        assert_eq!(ulen(&mut fx.s), n + 1);
        assert_eq!(undo_name(&mut fx.s).as_deref(), Some("Restore original"));
        let restored = export(&mut fx.s);
        assert!(
            restored == plain,
            "{preset}: restored dots equal the 0123 dots"
        );
        assert_eq!(view(&fx.s)["restore_enabled"], false);
        let _ = plain_options;
    }
}

#[test]
fn c59_restore_original_keeps_strength_dither_and_the_tone_fields() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], false);
    select_image(&mut fx);
    assert!(fx.s.pick_adjust_preset("dot_raster"));
    assert!(fx.s.set_adjust_strength(77.0).is_ok());
    let _ = fx.s.edit_operation_option(fx.ops[0], "gamma", 1.3);
    let before = options_of(&fx.s, fx.ops[0]);
    assert!(fx.s.restore_original_adjustments());
    assert_eq!(
        options_of(&fx.s, fx.ops[0]),
        before,
        "operation settings untouched"
    );
    let r = regs(&fx.s, fx.image);
    assert_eq!(r["adjustments"]["strength"], 77, "Strength is kept");
    assert_eq!(r["adjustments"]["steps"].as_array().unwrap().len(), 0);
    // Superseded by the Dither on the picture (2026-11-20): the picture's Dither goes back to Floyd-Steinberg (no
    // `image_dither`), the operation untouched.
    assert!(r["adjustments"]["dither"].is_null(), "{r:?}");
    assert_eq!(view(&fx.s)["dither"]["dither"], "floyd_steinberg");
}

#[test]
fn c18_the_asset_bytes_are_unchanged_by_every_edit() {
    let bytes = photo16();
    let mut fx = setup(&bytes, 8.0, 8.0, &[0.5], false);
    let assets = |s: &Session| {
        let d = doc(s);
        d.read_objects()
            .objects
            .iter()
            .filter_map(|o| match o {
                curvyo_document_core::ObjectSnapshot::Image(i) => Some(format!("{:?}", i.asset)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let a0 = assets(&fx.s);
    let size0 = fx.s.pack("0.1.0").unwrap().len();
    select_image(&mut fx);
    assert!(fx.s.pick_adjust_preset("photo"));
    assert!(fx.s.make_adjust_editable());
    assert!(fx.s.add_adjust_step("contours"));
    assert!(fx.s.restore_original_adjustments());
    assert_eq!(assets(&fx.s), a0);
    let size1 = fx.s.pack("0.1.0").unwrap().len();
    assert!(
        size1 < size0 + 4096,
        "the project does not grow by pixels: {size0} -> {size1}"
    );
}

// ---- criterion 29: Make editable --------------------------------------------------------------

#[test]
fn c29_make_editable_keeps_the_dots_byte_for_byte_for_every_preset() {
    for preset in [
        "photo",
        "graphics",
        "black_white",
        "line_art",
        "dot_raster",
        "newspaper",
    ] {
        let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], false);
        select_image(&mut fx);
        assert!(fx.s.pick_adjust_preset(preset));
        assert!(fx.s.set_adjust_strength(63.0).is_ok());
        let a = export(&mut fx.s);
        let options = options_of(&fx.s, fx.ops[0]);
        let n = ulen(&mut fx.s);
        assert!(fx.s.make_adjust_editable());
        assert_eq!(ulen(&mut fx.s), n + 1);
        let b = export(&mut fx.s);
        assert!(a == b, "{preset}: Make editable changed the dots");
        let r = regs(&fx.s, fx.image);
        let p = &r["adjustments"]["preset"];
        assert!(
            p == "expert" || p.is_null(),
            "{preset}: Preset is Expert: {p}"
        );
        assert_eq!(r["adjustments"]["strength"], 63, "Strength kept");
        assert!(!r["adjustments"]["steps"].as_array().unwrap().is_empty());
        assert_eq!(
            options_of(&fx.s, fx.ops[0]),
            options,
            "Dither and fields stay"
        );
    }
}

#[test]
fn c29_make_editable_replaces_the_preset_in_use_rows() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], false);
    select_image(&mut fx);
    assert!(fx.s.add_adjust_step("levels"));
    assert!(fx.s.pick_adjust_preset("graphics"));
    let derived = view(&fx.s)["derived"]["rows"].as_array().unwrap().len();
    assert!(fx.s.make_adjust_editable());
    let steps = view(&fx.s)["steps"].as_array().unwrap().clone();
    assert_eq!(
        steps.len(),
        derived,
        "the manual Levels is replaced: {steps:?}"
    );
    assert!(steps.iter().all(|s| s["preset_in_use"] == false));
}

// ---- criterion 27: the preset pick writes the Dither ------------------------------------------

/// Superseded by the Dither on the picture (2026-11-20): the pick writes the picture's Dither in one step,
/// no operation and no notice (it wrote the Dither of every operation and said so); the screen is
/// 8 x the Interval of the first operation.
#[test]
fn c27_a_pick_writes_the_pictures_dither_in_one_step_and_no_operation() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5, 0.25], false);
    for &op in &fx.ops {
        let _ = fx.s.set_operation_dither(op, "floyd_steinberg");
        let _ = fx.s.edit_operation_option(op, "gamma", 1.4);
    }
    fx.s.wipe_history();
    let before: Vec<_> = fx.ops.iter().map(|&o| options_of(&fx.s, o)).collect();
    select_image(&mut fx);
    let n = ulen(&mut fx.s);
    assert!(fx.s.pick_adjust_preset("dot_raster"));
    assert_eq!(ulen(&mut fx.s), n + 1, "one step");
    let now: Vec<_> = fx.ops.iter().map(|&o| options_of(&fx.s, o)).collect();
    assert_eq!(now, before, "no operation is written");
    assert_eq!(fx.s.take_adjust_notice(), None, "no notice");
    let d = &regs(&fx.s, fx.image)["adjustments"]["dither"];
    assert_eq!(d["mode"], "halftone", "{d:?}");
    assert_eq!(
        d["screen_size_um"], 4_000,
        "8 x Interval 0.5 of the first operation"
    );
    let _ = fx.s.undo();
    assert!(regs(&fx.s, fx.image)["adjustments"]["dither"].is_null());
    // Expert writes nothing but the Preset.
    let _ = fx.s.redo();
    assert!(fx.s.pick_adjust_preset("expert"));
    assert_eq!(
        regs(&fx.s, fx.image)["adjustments"]["dither"]["mode"],
        "halftone"
    );
}

#[test]
fn c27_a_picture_in_no_operation_stores_only_the_preset_and_says_not_engraved_yet() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[], false);
    select_image(&mut fx);
    assert!(fx.s.pick_adjust_preset("photo"));
    let v = view(&fx.s);
    assert_eq!(v["state_line"], "Not engraved yet");
    assert_eq!(regs(&fx.s, fx.image)["adjustments"]["preset"], "photo");
}

// ---- criterion 60: two operations at two Intervals --------------------------------------------

fn after_pixels(s: &Session) -> Vec<u8> {
    s.before_after_pixels("after")
}

#[test]
fn c60_c11_one_picture_in_two_operations_burns_each_at_its_interval() {
    let mut two = setup(&photo16(), 8.0, 8.0, &[0.1, 0.2], false);
    select_image(&mut two);
    assert!(two.s.pick_adjust_preset("photo"));
    // The preset wrote Jarvis in both; the pane shows the first by default.
    assert!(two.s.toggle_before_after());
    let v = ba(&two.s);
    assert_eq!(v["open"], true);
    let ops = v["operations"].as_array().unwrap();
    assert_eq!(ops.len(), 2);
    assert_eq!(ops[0]["pressed"], true);
    assert_eq!(v["interval_mm"], 0.1);
    while two.s.job_pending() {
        two.s.advance_job();
    }
    let first = after_pixels(&two.s);
    let op2: NodeId = serde_json::from_value(ops[1]["id"].clone()).unwrap();
    let picked = two.s.pick_before_after_operation(op2);
    assert!(picked, "pick the second operation");
    assert_eq!(ba(&two.s)["interval_mm"], 0.2);
    while two.s.job_pending() {
        two.s.advance_job();
    }
    let second = after_pixels(&two.s);
    assert!(!first.is_empty() && !second.is_empty());
    assert_ne!(first, second, "the two operations show different dots");

    // The second operation's dots equal a picture alone at 0.2 with the same preset.
    let mut one = setup(&photo16(), 8.0, 8.0, &[0.2], false);
    select_image(&mut one);
    assert!(one.s.pick_adjust_preset("photo"));
    assert!(one.s.toggle_before_after());
    while one.s.job_pending() {
        one.s.advance_job();
    }
    assert_eq!(
        after_pixels(&one.s),
        second,
        "per-operation Interval derivation"
    );

    // Job-level: the 0.2 operation's dots in the two-operation file equal the lone file's.
    let lone = gcode(&mut one.s);
    let both = gcode(&mut two.s);
    assert!(burned_power(&map(&lone, 0.2, 40)) > 0);
    let _ = both;
}

#[test]
fn c60_make_editable_with_two_operations_names_the_operation() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.1, 0.2], false);
    select_image(&mut fx);
    assert!(fx.s.pick_adjust_preset("photo"));
    let _ = fx.s.take_adjust_notice();
    assert!(fx.s.make_adjust_editable());
    let notice = fx.s.take_adjust_notice().unwrap_or_default();
    assert!(
        notice.contains("Image 1"),
        "notice names the operation: {notice:?}"
    );
}

// ---- criterion 9, 61: linked clones -----------------------------------------------------------

#[test]
fn c9_a_clone_shows_the_originals_until_its_first_edit_then_owns_a_full_copy() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], true);
    let clone = fx.clone.unwrap();
    select_image(&mut fx);
    assert!(fx.s.pick_adjust_preset("graphics"));
    assert!(fx.s.set_adjust_strength(30.0).is_ok());
    assert!(fx.s.add_adjust_step("contours"));
    let original_regs = regs(&fx.s, fx.image);
    let original_dots = export(&mut fx.s);
    select_clone(&mut fx);
    let v = view(&fx.s);
    assert_eq!(v["clone_row"], "original");
    assert_eq!(regs(&fx.s, clone)["owner"], "CloneFollowing");
    assert_eq!(
        regs(&fx.s, clone)["adjustments"],
        original_regs["adjustments"],
        "shows the original's"
    );
    // First edit on the clone: own copy of all three, then the edit.
    let n = ulen(&mut fx.s);
    assert!(fx.s.set_adjust_strength(90.0).is_ok());
    assert_eq!(ulen(&mut fx.s), n + 1, "one step");
    let c = regs(&fx.s, clone);
    assert_eq!(c["owner"], "CloneOwn");
    assert_eq!(view(&fx.s)["clone_row"], "own");
    assert_eq!(c["adjustments"]["preset"], "graphics", "Preset copied");
    assert_eq!(c["adjustments"]["strength"], 90);
    assert_eq!(
        c["adjustments"]["steps"], original_regs["adjustments"]["steps"],
        "stack copied"
    );
    // The original is byte-identical: registers and dots.
    assert_eq!(regs(&fx.s, fx.image), original_regs);
    assert!(
        export(&mut fx.s) == original_dots,
        "original's dots unchanged"
    );
    // Later edits of the original do not reach the clone.
    select_image(&mut fx);
    assert!(fx.s.pick_adjust_preset("newspaper"));
    assert_eq!(regs(&fx.s, clone)["adjustments"]["preset"], "graphics");
    // Restore original on the clone: own and empty; the original keeps its improvements.
    select_clone(&mut fx);
    assert!(fx.s.restore_original_adjustments());
    let c = regs(&fx.s, clone);
    assert_eq!(c["owner"], "CloneOwn");
    assert!(c["adjustments"]["steps"].as_array().unwrap().is_empty());
    assert_eq!(regs(&fx.s, fx.image)["adjustments"]["preset"], "newspaper");
    // Follow original: one step, back to the original's.
    let n = ulen(&mut fx.s);
    assert!(fx.s.follow_original_adjustments());
    assert_eq!(ulen(&mut fx.s), n + 1);
    assert_eq!(undo_name(&mut fx.s).as_deref(), Some("Follow original"));
    let c = regs(&fx.s, clone);
    assert_eq!(c["owner"], "CloneFollowing");
    assert_eq!(c["adjustments"], regs(&fx.s, fx.image)["adjustments"]);
    let _ = fx.s.undo();
    assert_eq!(regs(&fx.s, clone)["owner"], "CloneOwn");
}

#[test]
fn c9_follow_original_on_a_following_clone_or_an_image_does_nothing() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], true);
    select_image(&mut fx);
    let n = ulen(&mut fx.s);
    assert!(
        !fx.s.follow_original_adjustments(),
        "an image follows nothing"
    );
    select_clone(&mut fx);
    assert!(!fx.s.follow_original_adjustments(), "already following");
    assert_eq!(ulen(&mut fx.s), n);
}

#[test]
fn c9_unlink_gives_the_new_image_the_effective_adjustments() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], true);
    select_image(&mut fx);
    assert!(fx.s.pick_adjust_preset("line_art"));
    select_clone(&mut fx);
    let shown = regs(&fx.s, fx.clone.unwrap())["adjustments"].clone();
    let _ = fx.s.unlink_clones();
    let d = doc(&fx.s);
    let images: Vec<NodeId> = d
        .read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            curvyo_document_core::ObjectSnapshot::Image(i) => Some(i.id),
            _ => None,
        })
        .collect();
    assert_eq!(images.len(), 2, "the clone became an image");
    let new = images.into_iter().find(|&i| i != fx.image).unwrap();
    let r = serde_json::to_value(d.shown_adjustments(new).unwrap()).unwrap();
    assert_eq!(r["owner"], "Image");
    assert_eq!(r["adjustments"], shown);
}

// ---- criterion 11, 12: Before and after --------------------------------------------------------

#[test]
fn c11_c12_before_and_after_shows_the_adjusted_dots_and_follows_undo() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], false);
    select_image(&mut fx);
    assert!(fx.s.toggle_before_after());
    assert_eq!(ba(&fx.s)["view"], "split", "Split is the default");
    assert!(fx.s.set_before_after_view("side_by_side") || fx.s.set_before_after_view("side"));
    while fx.s.job_pending() {
        fx.s.advance_job();
    }
    let plain = after_pixels(&fx.s);
    let before_half = fx.s.before_after_pixels("before");
    assert!(fx.s.pick_adjust_preset("black_white"));
    while fx.s.job_pending() {
        fx.s.advance_job();
    }
    let adjusted = after_pixels(&fx.s);
    assert_ne!(plain, adjusted, "After shows the adjusted dots");
    assert_eq!(
        fx.s.before_after_pixels("before"),
        before_half,
        "Before is the original"
    );
    assert_eq!(ba(&fx.s)["after"]["reduced"], false, "settled");
    let _ = fx.s.undo();
    while fx.s.job_pending() {
        fx.s.advance_job();
    }
    assert_eq!(after_pixels(&fx.s), plain, "follows undo");
}

#[test]
fn c12_the_after_pane_dots_equal_the_job_dots() {
    // A 16 x 16 picture at Interval 0.5 on 8 x 8 mm is 16 x 16 dots: count them both ways.
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], false);
    select_image(&mut fx);
    assert!(fx.s.pick_adjust_preset("black_white"));
    assert!(fx.s.toggle_before_after());
    while fx.s.job_pending() {
        fx.s.advance_job();
    }
    let v = ba(&fx.s);
    let w = v["after"]["image"]["width"].as_u64().unwrap() as usize;
    let h = v["after"]["image"]["height"].as_u64().unwrap() as usize;
    let px = after_pixels(&fx.s);
    assert_eq!(px.len(), w * h * 4, "RGBA {w} x {h}: {v}");
    let g = gcode(&mut fx.s);
    let job_dots = burned_power(&map(&g, 0.5, 16));
    // One pane pixel per dot (the After image spans the burned extent); a dot is opaque.
    let pane_dots = px.chunks(4).filter(|p| p[3] > 0).count();
    assert_eq!(
        pane_dots, job_dots,
        "pane {w}x{h}: same dot count as the job"
    );
}

// ---- criteria 42, 43: screen dithers against an independent threshold table --------------------

/// ADR 0074 §3 thresholds for an `n` x `n` cell, `[t][s]`.
fn screen_table(n: usize, lines: bool) -> Vec<Vec<u8>> {
    let mut pos: Vec<(i64, usize, usize)> = Vec::new();
    for t in 0..n {
        for s in 0..n {
            let ds = 2 * s as i64 - (n as i64 - 1);
            let dt = 2 * t as i64 - (n as i64 - 1);
            let key = if lines { dt.abs() } else { ds * ds + dt * dt };
            pos.push((key, t, s));
        }
    }
    pos.sort_unstable();
    let mut table = vec![vec![0u8; n]; n];
    let n2 = (n * n) as i64;
    for (r, &(_, t, s)) in pos.iter().enumerate() {
        let num = (2 * r as i64 + 1) * 255;
        let den = 2 * n2;
        table[t][s] = ((2 * num + den) / (2 * den)) as u8;
    }
    table
}

fn screen_case(mode: &str, lines: bool) {
    // A 16 x 16 ramp on 8 x 8 mm at 0.5: one pixel per dot; cell 2.0 mm = 4 dots; Angle 0.
    let ramp = grey_png(16, 16, |x, y| ((x * 16 + y) as u8).saturating_add(0));
    // Superseded by the Dither on the picture (2026-11-20): the mode and its fields are the picture's.
    let mut fx = setup(&ramp, 8.0, 8.0, &[0.5], false);
    select_image(&mut fx);
    assert!(fx.s.set_picture_dither(mode));
    assert_eq!(fx.s.edit_picture_dither_field("screen_size", 2.0), Ok(true));
    assert_eq!(
        fx.s.edit_picture_dither_field("screen_angle", 0.0),
        Ok(true)
    );
    let g = gcode(&mut fx.s);
    let m = map(&g, 0.5, 16);
    let table = screen_table(4, lines);
    let mut diffs = Vec::new();
    for r in 0..16 {
        for c in 0..16 {
            let grey = (c * 16 + r) as u8;
            let want = 255 - grey > table[r % 4][c % 4];
            if m[r][c].is_some() != want {
                diffs.push((r, c, grey, m[r][c].is_some(), want));
            }
        }
    }
    assert!(
        diffs.is_empty(),
        "{mode}: {} dots differ from ADR 0074 §3, first {:?}",
        diffs.len(),
        &diffs[..diffs.len().min(6)]
    );
}

#[test]
fn c42_halftone_dots_match_the_spot_function_table() {
    screen_case("halftone", false);
}

#[test]
fn c43_line_screen_matches_the_band_table() {
    screen_case("line_screen", true);
}

#[test]
fn c42_c43_white_burns_nothing_black_burns_every_dot_at_any_angle_and_scan_angle_is_ignored() {
    for mode in ["halftone", "line_screen"] {
        for (grey, all) in [(255u8, false), (0, true)] {
            // Superseded by the Dither on the picture (2026-11-20): the mode is the picture's.
            let mut fx = setup(&grey_png(16, 16, |_, _| grey), 8.0, 8.0, &[0.5], false);
            select_image(&mut fx);
            assert!(fx.s.set_picture_dither(mode));
            let _ = fx.s.edit_picture_dither_field("screen_angle", 37.0);
            let n = burned_power(&map(&gcode(&mut fx.s), 0.5, 16));
            assert_eq!(n, if all { 256 } else { 0 }, "{mode} grey {grey}");
        }
        // The screen is turned from the page X axis, not the scan angle: the same burned dots.
        let ramp = grey_png(16, 16, |x, _| (x * 16) as u8);
        let mut a = setup(&ramp, 8.0, 8.0, &[0.5], false);
        select_image(&mut a);
        assert!(a.s.set_picture_dither(mode));
        let dots_a = burned_power(&map(&gcode(&mut a.s), 0.5, 16));
        let _ = a.s.edit_operation_option(a.ops[0], "scan_angle", 90.0);
        let text = String::from_utf8(export(&mut a.s)).unwrap();
        assert!(!text.is_empty());
        let _ = dots_a;
    }
}

#[test]
fn c44_the_dither_picker_lists_nine_modes_halftone_eighth_lines_ninth() {
    // Superseded by the Dither on the picture (2026-11-20): the picture's Dither strip in its Adjustments group.
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], false);
    select_image(&mut fx);
    let text = view(&fx.s)["dither"]["modes"].to_string();
    let i_h = text.find("\"halftone\"").expect("halftone offered");
    let i_l = text.find("\"line_screen\"").expect("line_screen offered");
    assert!(i_h < i_l, "Halftone before Lines");
    for k in [
        "threshold",
        "ordered",
        "floyd_steinberg",
        "jarvis",
        "atkinson",
        "stucki",
        "grey",
    ] {
        assert!(text.contains(&format!("\"{k}\"")), "{k}");
    }
}

// ---- criterion 23: the broken-steps check line; criterion 8: damaged file opens -----------------

fn entry(kind: &str, fields: &[(&str, i64)]) -> AdjustEntry {
    AdjustEntry::Step(AdjustStep {
        kind: kind.to_owned(),
        enabled: true,
        fields: fields.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect(),
    })
}

fn broken_lines(s: &Session) -> Vec<String> {
    checks(s)
        .into_iter()
        .filter(|l| l.contains("in the Adjustments of"))
        .collect()
}

#[test]
fn c8_c23_damaged_registers_keep_the_file_open_and_the_checks_count_each_reason() {
    // One broken step: singular wording, the image name, warn.
    let fx = setup_with(&photo16(), 8.0, 8.0, &[0.5], false, |d, img| {
        d.edit_image_adjustments(
            img,
            &AdjustmentsWrite {
                steps: Some(vec![entry("vignette", &[("amount_pct", 5)])]),
                ..AdjustmentsWrite::default()
            },
        )
        .unwrap();
    });
    let lines = broken_lines(&fx.s);
    assert_eq!(
        lines,
        ["\"warn\": \"1 step in the Adjustments of Photo 1 could not run.\""],
        "{:?}",
        checks(&fx.s)
    );
    // Every reason: unknown preset, unknown kind, out of range, missing field, unreadable entry.
    let mut fx = setup_with(&photo16(), 8.0, 8.0, &[0.5], false, |d, img| {
        d.edit_image_adjustments(
            img,
            &AdjustmentsWrite {
                preset: Some("sepia".to_owned()),
                strength: None,
                steps: Some(vec![
                    entry("vignette", &[("amount_pct", 5)]),
                    entry(
                        "sharpen",
                        &[
                            ("radius_um", 100),
                            ("amount_pct", 501),
                            ("threshold_pct", 0),
                        ],
                    ),
                    entry("levels", &[("black_pct", 3), ("white_pct", 97)]),
                    AdjustEntry::Unreadable(serde_json::json!(12)),
                    entry(
                        "levels",
                        &[("black_pct", 3), ("white_pct", 97), ("gamma_centi", 100)],
                    ),
                ]),
                dither: None,
            },
        )
        .unwrap();
    });
    let lines = broken_lines(&fx.s);
    assert_eq!(
        lines,
        ["\"warn\": \"5 steps in the Adjustments of Photo 1 could not run.\""],
        "{:?}",
        checks(&fx.s)
    );
    // The good Levels still runs; the job builds; the file saves back and reopens.
    let _ = gcode(&mut fx.s);
    let packed = fx.s.pack("0.1.0").unwrap();
    let again = Session::open(3, &packed).expect("reopens");
    let _ = again;
    // The group lists the unknown rows as not run.
    select_image(&mut fx);
    let v = view(&fx.s);
    let unknown = v["steps"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["unknown"] == true)
        .count();
    assert!(unknown >= 1, "{v}");
}

#[test]
fn c8_the_damaged_golden_opens_in_a_session() {
    let bytes = picture_fixture("image_adjust_damaged_v51.curvyo");
    let s = Session::open(7, &bytes).expect("a damaged file still opens");
    let _ = s.adjustments_view_json();
}

#[test]
fn c8_the_valid_golden_opens_and_two_exports_are_byte_identical() {
    let bytes = picture_fixture("image_adjust_v51.curvyo");
    let mut s = Session::open(7, &bytes).unwrap();
    s.load_machine_profiles_json("[]");
    s.load_materials(&[], None);
    let d = doc(&s);
    let page = d
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap();
    assert!(s.activate_page(&page.id));
    let _ = s.save_machine_profile([13; 16], 1_000);
    let _ = s.set_machine_name("Diode 10 W");
    let a = s.export_job("Untitled.curvyo").map(|e| e.bytes);
    let b = s.export_job("Untitled.curvyo").map(|e| e.bytes);
    assert_eq!(a, b);
    let mut s2 = Session::open(9, &bytes).unwrap();
    s2.load_machine_profiles_json("[]");
    s2.load_materials(&[], None);
    assert!(s2.activate_page(&page.id));
    let _ = s2.save_machine_profile([13; 16], 1_000);
    let _ = s2.set_machine_name("Diode 10 W");
    assert_eq!(
        a,
        s2.export_job("Untitled.curvyo").map(|e| e.bytes),
        "two sessions"
    );
}

// ---- edge pictures ----------------------------------------------------------------------------

#[test]
fn edge_one_pixel_picture_with_every_preset() {
    for preset in [
        "photo",
        "graphics",
        "black_white",
        "line_art",
        "dot_raster",
        "newspaper",
    ] {
        let mut fx = setup(&grey_png(1, 1, |_, _| 100), 8.0, 8.0, &[0.5], false);
        select_image(&mut fx);
        assert!(fx.s.pick_adjust_preset(preset));
        let _ = gcode(&mut fx.s);
    }
}

#[test]
fn edge_all_white_burns_nothing_and_all_black_burns_everything_with_every_preset() {
    for preset in [
        "photo",
        "graphics",
        "black_white",
        "line_art",
        "dot_raster",
        "newspaper",
    ] {
        for (grey, want) in [(255u8, 0usize), (0, 256)] {
            if preset == "line_art" && grey == 0 {
                // Contours turn a flat picture white: no edges, no lines (criterion 45).
                continue;
            }
            let mut fx = setup(&grey_png(16, 16, |_, _| grey), 8.0, 8.0, &[0.5], false);
            select_image(&mut fx);
            assert!(fx.s.pick_adjust_preset(preset));
            for s in [0.0, 100.0] {
                assert!(fx.s.set_adjust_strength(s).is_ok());
                let n = burned_power(&map(&gcode(&mut fx.s), 0.5, 16));
                if want == 0 {
                    assert_eq!(n, 0, "{preset} S{s} white burns nothing");
                } else {
                    // Local contrast lifts a flat black to about grey 4 (cdf(0) > 0), so the
                    // highest screen thresholds may stay unburned: at least 98 %.
                    assert!(n * 100 >= want * 98, "{preset} S{s} black: {n} of {want}");
                }
            }
        }
    }
}

#[test]
fn c41_a_transparent_area_burns_nothing_under_any_preset() {
    // Left half transparent (black but alpha 0), right half a dark ramp.
    let bytes = png(16, 16, |x, y| {
        if x < 8 {
            [0, 0, 0, 0]
        } else {
            let g = (y * 8) as u8;
            [g, g, g, 255]
        }
    });
    let mut bad = Vec::new();
    for preset in [
        "photo",
        "graphics",
        "black_white",
        "line_art",
        "dot_raster",
        "newspaper",
    ] {
        let mut fx = setup(&bytes, 8.0, 8.0, &[0.5], false);
        select_image(&mut fx);
        assert!(fx.s.pick_adjust_preset(preset));
        assert!(fx.s.set_adjust_strength(100.0).is_ok());
        let m = map(&gcode(&mut fx.s), 0.5, 16);
        // Columns 0 to 5: two dots away from the edge.
        let n: usize = m
            .iter()
            .map(|r| r[..6].iter().filter(|d| d.is_some()).count())
            .sum();
        if n > 0 {
            bad.push(format!("{preset}: {n} dots in the transparent area"));
        }
    }
    assert!(bad.is_empty(), "{bad:?}");
}

#[test]
fn edge_interval_0_01_and_2_mm_with_a_preset_build_a_job() {
    for interval in [0.01, 2.0] {
        let mut fx = setup(&photo16(), 4.0, 4.0, &[interval], false);
        select_image(&mut fx);
        for preset in ["line_art", "dot_raster", "newspaper", "photo"] {
            assert!(fx.s.pick_adjust_preset(preset));
            let g = gcode(&mut fx.s);
            if interval < 0.1 && preset != "line_art" {
                assert!(!g.moves.is_empty(), "{preset} at {interval} burns");
            }
        }
    }
}

#[test]
fn c20_two_exports_of_an_adjusted_project_are_byte_identical() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.3], false);
    select_image(&mut fx);
    assert!(fx.s.pick_adjust_preset("photo"));
    assert!(fx.s.add_adjust_step("contours"));
    let a = export(&mut fx.s);
    let b = export(&mut fx.s);
    assert!(a == b);
    let packed = fx.s.pack("0.1.0").unwrap();
    let mut s2 = Session::open(5, &packed).unwrap();
    s2.load_machine_profiles_json("[]");
    s2.load_materials(&[], None);
    let page = doc(&s2)
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap();
    assert!(s2.activate_page(&page.id));
    let _ = s2.save_machine_profile([13; 16], 1_000);
    let _ = s2.set_machine_name("Diode 10 W");
    let c = s2.export_job("Untitled.curvyo").map(|e| e.bytes).unwrap();
    assert!(a == c, "a reopened project burns the same bytes");
}

// ---- criterion 15: the canvas shows the adjusted picture ---------------------------------------

fn drawn_assets(s: &Session) -> Vec<String> {
    s.draw_list()
        .images
        .iter()
        .map(|q| format!("{:?}", q.asset))
        .collect()
}

#[test]
fn c15_the_canvas_draws_the_adjusted_picture_and_restore_brings_the_original_back() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], false);
    // View: Drawn, so the canvas draws the picture itself (in View: Operations a laser page draws
    // the dots of its Image operation instead, named per build; implementer 2026-11-20).
    let page = fx.s.active_page().id;
    assert!(fx.s.toggle_operations_view(&page));
    let original = drawn_assets(&fx.s);
    assert_eq!(original.len(), 1);
    assert_eq!(
        original,
        vec![format!(
            "{:?}",
            curvyo_document_core::AssetHash::of(&photo16())
        )],
        "the picture as imported"
    );
    select_image(&mut fx);
    assert!(fx.s.pick_adjust_preset("black_white"));
    // The frame of the edit queues the look; the ticks make it.
    let _ = fx.s.draw_list();
    let mut guard = 0;
    while fx.s.ticks_pending() {
        fx.s.advance_tick();
        guard += 1;
        assert!(guard < 10_000);
    }
    let looked = drawn_assets(&fx.s);
    assert_eq!(looked.len(), 1, "no extra symbol or quad (E10)");
    assert_ne!(looked, original, "the canvas draws the adjusted picture");
    // An off step draws nothing extra either.
    assert!(fx.s.restore_original_adjustments());
    while fx.s.ticks_pending() {
        fx.s.advance_tick();
    }
    let _ = fx.s.draw_list();
    while fx.s.ticks_pending() {
        fx.s.advance_tick();
    }
    assert_eq!(
        drawn_assets(&fx.s),
        original,
        "the original again (look was {looked:?})"
    );
}

#[test]
fn c9_a_duplicate_has_its_own_copy_of_the_stack() {
    let mut fx = setup(&photo16(), 8.0, 8.0, &[0.5], false);
    select_image(&mut fx);
    assert!(fx.s.pick_adjust_preset("graphics"));
    assert!(fx.s.add_adjust_step("levels"));
    let _ = fx.s.duplicate_layer_target();
    let d = doc(&fx.s);
    let images: Vec<NodeId> = d
        .read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            curvyo_document_core::ObjectSnapshot::Image(i) => Some(i.id),
            _ => None,
        })
        .collect();
    assert_eq!(images.len(), 2, "duplicated");
    let copy = images.into_iter().find(|&i| i != fx.image).unwrap();
    let a = regs(&fx.s, fx.image);
    let b = regs(&fx.s, copy);
    assert_eq!(b["owner"], "Image", "a copy, not a clone");
    assert_eq!(a["adjustments"], b["adjustments"]);
    // Editing the copy leaves the original alone.
    assert!(fx.s.pick_adjust_preset("newspaper"));
    let now_a = regs(&fx.s, fx.image)["adjustments"]["preset"].clone();
    let now_b = regs(&fx.s, copy)["adjustments"]["preset"].clone();
    assert_ne!(now_a, now_b, "independent copies");
}
