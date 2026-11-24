//! Tester acceptance cases for the "Dither on the picture" rework of
//! `specs/0135-image-optimisation` (Frage 283 B, 284 B, 300 A; criteria 4, 8, 9, 22, 23, 27, 59,
//! 67; ADR 0072 amendment A1 to A10), written from the specification through the public
//! `Session` before the rework's code was read: each picture of one Image operation burns with
//! its own dither, a preset pick writes the Preset and the picture's dither only (screens at k x
//! the Interval of the first Image operation, 0.10 mm with none), the operation's Dither reaches
//! only its gradient-filled leaves, Lowest power stays the operation's, Restore original removes
//! the dither, linked clones follow, own, Follow original and Unlink the dither with the other
//! registers, and a damaged dither burns as Floyd-Steinberg and counts in the check line.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::map_unwrap_or,
    dead_code,
    missing_docs
)]

#[path = "../../curvyo-laser-core/tests/tester_raster_0123/mod.rs"]
mod reference;

use std::sync::Arc;

use curvyo_document_core::{
    AdjustEntry, AdjustStep, AdjustmentsWrite, Affine, Direction, Document, DocumentSize, NewImage,
    NewOperation, NewOutputPage, NodeId, Point, RectBounds, StoredDither, Timestamp, pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_image_core::{WorkingImage, encode_png};
use reference::{Gcode, burned_power, read};
use serde_json::Value;

const LASER: &str = "curvyo.laser";
const BED_H: f64 = 300.0;
/// Every picture and the gradient rectangle sit at page y 20.
const TOP: f64 = 20.0;

// ---- fixture ----------------------------------------------------------------------------------

fn grey_png(w: u32, h: u32, mut g: impl FnMut(u32, u32) -> u8) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let v = g(x, y);
            pixels.extend_from_slice(&[v, v, v, 255]);
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

/// A 16 x 16 photo-like picture: a gradient, a disc, grain.
fn photo16() -> Vec<u8> {
    let mut seed = 11u32;
    grey_png(16, 16, move |x, y| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let grain = (seed >> 24) as i32 % 9 - 4;
        let dx = x as i32 - 8;
        let dy = y as i32 - 8;
        let disc = if dx * dx + dy * dy < 20 { 70 } else { 0 };
        (60 + x as i32 * 6 + disc + grain).clamp(0, 255) as u8
    })
}

/// A laser page and its manufacturing node and origin.
fn laser_doc() -> (Document, NodeId, Point) {
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
    (d, m, o)
}

/// An 8 x 8 mm picture at page (`x`, 20).
fn picture(d: &Document, o: Point, x: f64, name: &str) -> NodeId {
    d.add_images(&[NewImage {
        bytes: Arc::from(photo16().as_slice()),
        placement: Affine::new(8.0, 0.0, 0.0, 8.0, o.x + x, o.y + TOP),
        name: name.to_owned(),
    }])
    .unwrap()[0]
}

/// A 10 x 10 mm rectangle with a Gradient fill at page (`x`, 20).
fn gradient_rect(d: &Document, o: Point, x: f64) -> NodeId {
    let r = d.create_rect(RectBounds::from_corners(
        Point::new(o.x + x, o.y + TOP),
        Point::new(o.x + x + 10.0, o.y + TOP + 10.0),
    ));
    d.edit_style(
        &[r],
        &curvyo_document_core::StyleEdit::FillPaint(curvyo_document_core::FillPaint::Gradient),
    )
    .unwrap();
    r
}

fn image_op(d: &Document, m: NodeId, name: &str, members: &[NodeId]) -> NodeId {
    let index = d.operations(m).unwrap().len();
    let groups: Vec<Vec<NodeId>> = members.iter().map(|&n| vec![n]).collect();
    d.add_operation(
        m,
        &NewOperation {
            kind: "image",
            name,
            settings: None,
        },
        index,
        &groups,
    )
    .unwrap()
}

/// Opens `d` on its laser page; each `(op, interval)` gets 60 %, 3,000 mm/min, no overscan.
fn open(d: &Document, ops: &[(NodeId, f64)]) -> Session {
    let page = d
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap();
    let mut s = Session::open(7, &pack(d, "0135-dither-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    s.set_clock(|| Timestamp::from_secs(1_762_948_800));
    assert!(s.activate_page(&page.id));
    s.load_materials(&[], None);
    let _ = s.save_machine_profile([13; 16], 1_000);
    let _ = s.set_machine_name("Diode 10 W");
    for &(op, i) in ops {
        assert!(s.edit_operation_setting(op, "main", "power", 60.0).is_ok());
        assert!(
            s.edit_operation_setting(op, "main", "speed", 3000.0)
                .is_ok()
        );
        assert_eq!(s.edit_operation_option(op, "interval", i), Ok(true));
        let _ = s.edit_operation_option(op, "overscan", 0.0);
    }
    s.wipe_history();
    s
}

fn select(s: &mut Session, id: NodeId) {
    assert!(s.select_output_items(&[id]));
    assert!(view(s).is_object(), "the Adjustments group shows");
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.adjustments_view_json()).unwrap()
}

fn doc(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn regs(s: &Session, id: NodeId) -> Value {
    serde_json::to_value(doc(s).shown_adjustments(id).unwrap()).unwrap()
}

fn dither_of(s: &Session, id: NodeId) -> Value {
    regs(s, id)["adjustments"]["dither"].clone()
}

fn checks(s: &Session) -> Vec<String> {
    let out: Value = serde_json::from_str(&s.output_group_json()).unwrap();
    out["checks"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|c| format!("{}: {}", c["severity"], c["text"]))
        .collect()
}

fn broken_lines(s: &Session) -> Vec<String> {
    checks(s)
        .into_iter()
        .filter(|l| l.contains("in the Adjustments of"))
        .collect()
}

fn gcode(s: &mut Session) -> Gcode {
    let bytes = s
        .export_job("Untitled.curvyo")
        .map(|e| e.bytes)
        .unwrap_or_else(|| panic!("export blocked: {:?}", checks(s)));
    read(&bytes).unwrap()
}

/// The power map of the `n` x `n` dots at page (`x`, 20), Interval `i`.
fn dots(g: &Gcode, x: f64, i: f64, n: usize) -> Vec<Vec<Option<u32>>> {
    g.dot_map(x, BED_H - TOP - i / 2.0, i, n, n)
}

fn ulen(s: &mut Session) -> usize {
    let _ = s.history_target_name(Direction::Undo);
    s.undo_len()
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

fn image_group(s: &mut Session, op: NodeId) -> Value {
    let rows = |s: &Session| -> Value {
        let v: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
        let full = serde_json::to_value(op).unwrap();
        v["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == full)
            .cloned()
            .unwrap_or(Value::Null)
    };
    if rows(s)["unfolded"] != true {
        assert!(s.unfold_operation_row(op));
    }
    rows(s)["settings"]["options"]["image"].clone()
}

/// The dots of one picture at page x 20 alone in one Image operation at `i`, after `edit` on it.
fn alone(i: f64, edit: impl FnOnce(&mut Session, NodeId)) -> Vec<Vec<Option<u32>>> {
    let (d, m, o) = laser_doc();
    let p = picture(&d, o, 20.0, "Photo 1");
    let op = image_op(&d, m, "Image 1", &[p]);
    let mut s = open(&d, &[(op, i)]);
    edit(&mut s, p);
    let n = (8.0 / i).round() as usize;
    dots(&gcode(&mut s), 20.0, i, n)
}

fn stored(mode: &str, fields: &[(&str, i64)]) -> StoredDither {
    StoredDither {
        mode: mode.to_owned(),
        fields: fields.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect(),
    }
}

// ---- criteria 4, 67: each picture burns with its own dither in one operation -----------------

#[test]
fn c67_two_pictures_with_two_presets_in_one_operation_each_burn_their_own_dots() {
    // A at page x 20, B at page x 60; one Image operation holds both.
    let (d, m, o) = laser_doc();
    let a = picture(&d, o, 20.0, "Photo A");
    let b = picture(&d, o, 60.0, "Photo B");
    let op = image_op(&d, m, "Image 1", &[a, b]);
    let mut s = open(&d, &[(op, 0.5)]);
    select(&mut s, a);
    assert!(s.pick_adjust_preset("photo"));
    select(&mut s, b);
    assert!(s.pick_adjust_preset("black_white"));
    assert_eq!(dither_of(&s, a)["mode"], "jarvis", "A keeps its own dither");
    assert_eq!(dither_of(&s, b)["mode"], "threshold");
    let g = gcode(&mut s);
    let dots_a = dots(&g, 20.0, 0.5, 16);
    let dots_b = dots(&g, 60.0, 0.5, 16);
    assert!(burned_power(&dots_a) > 0 && burned_power(&dots_b) > 0);

    // Each equals the dots of that picture alone in its own operation (B moved to x 20 there).
    let lone_a = alone(0.5, |s, p| {
        select(s, p);
        assert!(s.pick_adjust_preset("photo"));
    });
    let lone_b = alone(0.5, |s, p| {
        select(s, p);
        assert!(s.pick_adjust_preset("black_white"));
    });
    assert_eq!(dots_a, lone_a, "A burns as it does alone");
    assert_eq!(dots_b, lone_b, "B burns as it does alone");
    assert_ne!(dots_a, dots_b, "two dithers, two dot maps");
}

#[test]
fn c67_the_dither_strip_of_one_picture_leaves_the_other_and_the_operation_alone() {
    let (d, m, o) = laser_doc();
    let a = picture(&d, o, 20.0, "Photo A");
    let b = picture(&d, o, 60.0, "Photo B");
    let op = image_op(&d, m, "Image 1", &[a, b]);
    let mut s = open(&d, &[(op, 0.5)]);
    let before_b = dots(&gcode(&mut s), 60.0, 0.5, 16);
    let before_op = options_of(&s, op);
    select(&mut s, a);
    let n = ulen(&mut s);
    assert!(s.set_picture_dither("ordered"));
    assert_eq!(ulen(&mut s), n + 1, "one undo step");
    assert_eq!(
        s.history_target_name(Direction::Undo).as_deref(),
        Some("Edit adjustments")
    );
    assert_eq!(dither_of(&s, a)["mode"], "ordered");
    assert!(dither_of(&s, b).is_null(), "B keeps no stored dither");
    assert_eq!(
        options_of(&s, op),
        before_op,
        "the operation is not written"
    );
    let g = gcode(&mut s);
    assert_eq!(dots(&g, 60.0, 0.5, 16), before_b, "B's dots unchanged");
    let lone = alone(0.5, |s, p| {
        select(s, p);
        assert!(s.set_picture_dither("ordered"));
    });
    assert_eq!(dots(&g, 20.0, 0.5, 16), lone);
}

#[test]
fn c67_a_picture_without_a_stored_dither_burns_floyd_steinberg_whatever_the_operation_holds() {
    let plain = alone(0.5, |_, _| {});
    let fs = alone(0.5, |s, p| {
        select(s, p);
        assert!(s.set_picture_dither("floyd_steinberg"));
    });
    assert_eq!(plain, fs, "absent reads as Floyd-Steinberg at the defaults");
    let op_jarvis = alone(0.5, |s, _| {
        let op = s_first_op(s);
        let _ = s.set_operation_dither(op, "jarvis");
    });
    assert_eq!(
        plain, op_jarvis,
        "the operation's Dither never reaches a picture"
    );
}

fn s_first_op(s: &Session) -> NodeId {
    let v: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    serde_json::from_value(v["rows"][0]["id"].clone()).unwrap()
}

// ---- criterion 27: a preset pick writes the Preset and the picture's dither only -------------

#[test]
fn c27_screen_presets_resolve_k_times_the_first_operations_interval() {
    // (intervals of the operations in list order, preset, expected screen_size_um)
    for (intervals, preset, want) in [
        (vec![0.25, 0.5], "dot_raster", 2_000), // 8 x 0.25, the first one
        (vec![0.5, 0.25], "dot_raster", 4_000),
        (vec![0.25, 0.5], "newspaper", 3_000), // 12 x 0.25
        (vec![0.07], "dot_raster", 560),       // the Interval has two decimals
        (vec![0.07], "newspaper", 840),
        (vec![0.02], "dot_raster", 200), // 160 kept within 0.20 mm
        (vec![1.0], "newspaper", 5_000), // 12,000 kept within 5.00 mm
        (vec![], "dot_raster", 800),     // none: 8 x 0.10
        (vec![], "newspaper", 1_200),    // none: 12 x 0.10
    ] {
        let (d, m, o) = laser_doc();
        let p = picture(&d, o, 20.0, "Photo 1");
        let ops: Vec<(NodeId, f64)> = intervals
            .iter()
            .enumerate()
            .map(|(k, &i)| (image_op(&d, m, &format!("Image {}", k + 1), &[p]), i))
            .collect();
        let mut s = open(&d, &ops);
        select(&mut s, p);
        let before: Vec<_> = ops.iter().map(|&(op, _)| options_of(&s, op)).collect();
        let n = ulen(&mut s);
        assert!(s.pick_adjust_preset(preset), "{preset} {intervals:?}");
        assert_eq!(ulen(&mut s), n + 1, "one step");
        assert_eq!(
            s.history_target_name(Direction::Undo).as_deref(),
            Some("Edit adjustments")
        );
        let dth = dither_of(&s, p);
        let mode = if preset == "dot_raster" {
            "halftone"
        } else {
            "line_screen"
        };
        assert_eq!(dth["mode"], mode, "{preset} {intervals:?}: {dth}");
        assert_eq!(dth["screen_size_um"], want, "{preset} {intervals:?}: {dth}");
        assert_eq!(dth["screen_angle_deg"], 45);
        let after: Vec<_> = ops.iter().map(|&(op, _)| options_of(&s, op)).collect();
        assert_eq!(after, before, "{preset}: no operation is written");
        assert_eq!(s.take_adjust_notice(), None, "no notice");
    }
}

#[test]
fn c27_a_pick_keeps_the_fields_its_preset_does_not_name_and_expert_writes_only_the_preset() {
    let (d, m, o) = laser_doc();
    let p = picture(&d, o, 20.0, "Photo 1");
    let op = image_op(&d, m, "Image 1", &[p]);
    let mut s = open(&d, &[(op, 0.5)]);
    let _ = s.edit_operation_option(op, "gamma", 1.4);
    let _ = s.edit_operation_option(op, "lowest_power", 12.0);
    select(&mut s, p);
    assert!(s.set_adjust_strength(64.0).is_ok());
    assert!(s.set_picture_dither("threshold"));
    assert_eq!(s.edit_picture_dither_field("threshold", 30.0), Ok(true));
    assert!(s.set_picture_dither("halftone"));
    assert_eq!(s.edit_picture_dither_field("screen_size", 1.5), Ok(true));
    assert_eq!(s.edit_picture_dither_field("screen_angle", 20.0), Ok(true));
    let op_before = options_of(&s, op);

    // Photo names only the mode (Jarvis): Threshold, size and angle are kept.
    assert!(s.pick_adjust_preset("photo"));
    let dth = dither_of(&s, p);
    assert_eq!(dth["mode"], "jarvis");
    assert_eq!(dth["threshold_pct"], 30, "{dth}");
    assert_eq!(dth["screen_size_um"], 1_500, "{dth}");
    assert_eq!(dth["screen_angle_deg"], 20, "{dth}");
    // Black and white names Threshold 50 %: the screen fields are kept.
    assert!(s.pick_adjust_preset("black_white"));
    let dth = dither_of(&s, p);
    assert_eq!(dth["mode"], "threshold");
    assert_eq!(dth["threshold_pct"], 50);
    assert_eq!(dth["screen_size_um"], 1_500);
    assert_eq!(dth["screen_angle_deg"], 20);
    // Dot raster names Cell size and Angle: the Threshold is kept.
    assert!(s.pick_adjust_preset("dot_raster"));
    let dth = dither_of(&s, p);
    assert_eq!(dth["threshold_pct"], 50);
    assert_eq!(dth["screen_size_um"], 4_000);
    assert_eq!(dth["screen_angle_deg"], 45);
    // Expert writes the Preset only.
    assert!(s.pick_adjust_preset("expert"));
    assert_eq!(regs(&s, p)["adjustments"]["preset"], "expert");
    assert_eq!(dither_of(&s, p), dth, "Expert keeps the dither");
    // Strength and every operation field are as they were.
    assert_eq!(regs(&s, p)["adjustments"]["strength"], 64);
    assert_eq!(options_of(&s, op), op_before, "no operation field written");
}

#[test]
fn c27_c67_a_dither_strip_change_keeps_preset_and_strength_and_undo_restores_it() {
    let (d, m, o) = laser_doc();
    let p = picture(&d, o, 20.0, "Photo 1");
    let op = image_op(&d, m, "Image 1", &[p]);
    let mut s = open(&d, &[(op, 0.5)]);
    select(&mut s, p);
    assert!(s.pick_adjust_preset("graphics"));
    assert!(s.set_adjust_strength(70.0).is_ok());
    let before = regs(&s, p);
    assert!(s.set_picture_dither("atkinson"));
    let after = regs(&s, p);
    assert_eq!(after["adjustments"]["preset"], "graphics");
    assert_eq!(after["adjustments"]["strength"], 70);
    assert_eq!(
        after["adjustments"]["steps"],
        before["adjustments"]["steps"]
    );
    assert_eq!(after["adjustments"]["dither"]["mode"], "atkinson");
    let _ = s.undo();
    assert_eq!(regs(&s, p), before, "undo restores the register");
    // A field out of range is refused with the range and writes nothing.
    assert!(s.set_picture_dither("halftone"));
    let n = ulen(&mut s);
    let msg = s.edit_picture_dither_field("screen_size", 5.5).unwrap_err();
    assert!(msg.contains("0.20") && msg.contains("5.00"), "{msg}");
    let msg = s
        .edit_picture_dither_field("screen_angle", 90.0)
        .unwrap_err();
    assert!(msg.contains("89"), "{msg}");
    assert_eq!(ulen(&mut s), n);
}

// ---- criterion 8: the register's shape --------------------------------------------------------

#[test]
fn c8_image_dither_writes_all_four_fields_and_holds_no_lowest_power() {
    let (d, m, o) = laser_doc();
    let p = picture(&d, o, 20.0, "Photo 1");
    let op = image_op(&d, m, "Image 1", &[p]);
    let mut s = open(&d, &[(op, 0.5)]);
    select(&mut s, p);
    assert!(s.set_picture_dither("grey"));
    let dth = dither_of(&s, p);
    let mut keys: Vec<&str> = dth
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "mode",
            "screen_angle_deg",
            "screen_size_um",
            "threshold_pct"
        ],
        "all four, no Lowest power (Frage 284 B)"
    );
    assert_eq!(dth["threshold_pct"], 50);
    assert_eq!(dth["screen_size_um"], 800);
    assert_eq!(dth["screen_angle_deg"], 45);
    assert!(
        !s.adjustments_view_json().contains("lowest_power"),
        "no machine setting in the Adjustments group"
    );
    // Survives save and reopen.
    let again = Session::open(5, &s.pack("0.1.0").unwrap()).unwrap();
    assert_eq!(dither_of(&again, p), dth);
}

// ---- criteria 22, 67, A4: the operation's Dither is its gradient leaves' ----------------------

#[test]
fn c67_the_operations_dither_reaches_only_its_gradient_leaf() {
    let (d, m, o) = laser_doc();
    let p = picture(&d, o, 20.0, "Photo 1");
    let r = gradient_rect(&d, o, 100.0);
    let op = image_op(&d, m, "Image 1", &[p, r]);
    let mut s = open(&d, &[(op, 0.5)]);
    assert!(
        !image_group(&mut s, op)["dither"].is_null(),
        "the row shows its Dither while it holds a gradient leaf"
    );
    let _ = s.set_operation_dither(op, "threshold");
    let g1 = gcode(&mut s);
    assert!(s.set_operation_dither(op, "ordered"));
    let g2 = gcode(&mut s);
    assert_eq!(
        dots(&g1, 20.0, 0.5, 16),
        dots(&g2, 20.0, 0.5, 16),
        "the picture ignores the operation's Dither"
    );
    assert_ne!(
        dots(&g1, 100.0, 0.5, 20),
        dots(&g2, 100.0, 0.5, 20),
        "the gradient leaf burns with the operation's Dither"
    );
    // The picture's Dither strip moves the picture and leaves the gradient alone.
    select(&mut s, p);
    assert!(s.set_picture_dither("threshold"));
    let g3 = gcode(&mut s);
    assert_ne!(dots(&g2, 20.0, 0.5, 16), dots(&g3, 20.0, 0.5, 16));
    assert_eq!(dots(&g2, 100.0, 0.5, 20), dots(&g3, 100.0, 0.5, 20));
}

#[test]
fn c22_a_picture_only_row_shows_no_dither_and_shows_lowest_power_while_a_picture_burns_grey() {
    let (d, m, o) = laser_doc();
    let p = picture(&d, o, 20.0, "Photo 1");
    let op = image_op(&d, m, "Image 1", &[p]);
    let mut s = open(&d, &[(op, 0.5)]);
    let g = image_group(&mut s, op);
    assert!(g["dither"].is_null(), "{g}");
    assert!(g["lowest_power"].is_null(), "nothing burns Grey: {g}");
    select(&mut s, p);
    assert!(s.set_picture_dither("grey"));
    let g = image_group(&mut s, op);
    assert!(g["dither"].is_null(), "{g}");
    assert_eq!(g["lowest_power"]["label"], "Lowest power", "{g}");
}

// ---- criteria 22, 27, A10: Lowest power stays the operation's -------------------------------

fn burn_powers(g: &Gcode) -> Vec<u32> {
    let mut v: Vec<u32> = g.moves.iter().filter(|m| m.burn).map(|m| m.s).collect();
    v.sort_unstable();
    v
}

fn grey_in(lowest: &[f64]) -> (Session, NodeId, Vec<NodeId>) {
    let (d, m, o) = laser_doc();
    let p = picture(&d, o, 20.0, "Photo 1");
    let ops: Vec<NodeId> = (0..lowest.len())
        .map(|k| image_op(&d, m, &format!("Image {}", k + 1), &[p]))
        .collect();
    let with: Vec<(NodeId, f64)> = ops.iter().map(|&op| (op, 0.5)).collect();
    let mut s = open(&d, &with);
    for (&op, &l) in ops.iter().zip(lowest) {
        assert_eq!(s.edit_operation_option(op, "lowest_power", l), Ok(true));
    }
    select(&mut s, p);
    assert!(s.set_picture_dither("grey"));
    (s, p, ops)
}

#[test]
fn c22_a_grey_picture_burns_with_the_lowest_power_of_each_operation() {
    let (mut low, _, _) = grey_in(&[10.0]);
    let (mut high, _, _) = grey_in(&[30.0]);
    let (mut both, p, ops) = grey_in(&[10.0, 30.0]);
    let (gl, gh) = (gcode(&mut low), gcode(&mut high));
    let (pl, ph) = (burn_powers(&gl), burn_powers(&gh));
    assert!(!pl.is_empty() && !ph.is_empty());
    assert!(
        pl[0] < ph[0],
        "a higher Lowest power raises the palest dot: {} vs {}",
        pl[0],
        ph[0]
    );
    // The picture in both operations burns once with each one's Lowest power.
    let mut want = [pl, ph].concat();
    want.sort_unstable();
    assert_eq!(burn_powers(&gcode(&mut both)), want);
    // The picture's register holds no Lowest power and a preset pick writes none.
    let before: Vec<_> = ops.iter().map(|&op| options_of(&both, op)).collect();
    for preset in ["photo", "dot_raster", "expert"] {
        assert!(both.pick_adjust_preset(preset) || preset == "expert");
    }
    let after: Vec<_> = ops.iter().map(|&op| options_of(&both, op)).collect();
    assert_eq!(after, before, "Lowest power untouched by preset picks");
    assert!(dither_of(&both, p).get("lowest_power_pct").is_none());
}

// ---- criterion 59: Restore original removes the dither ----------------------------------------

#[test]
fn c59_a_stored_dither_alone_enables_restore_which_removes_it_and_gives_the_0123_dots() {
    let plain = alone(0.5, |_, _| {});
    let (d, m, o) = laser_doc();
    let p = picture(&d, o, 20.0, "Photo 1");
    let op = image_op(&d, m, "Image 1", &[p]);
    let mut s = open(&d, &[(op, 0.5)]);
    select(&mut s, p);
    assert_eq!(view(&s)["restore_enabled"], false, "nothing to restore");
    assert!(s.set_picture_dither("halftone"));
    assert_eq!(
        view(&s)["restore_enabled"],
        true,
        "a stored dither is something to restore"
    );
    assert_ne!(dots(&gcode(&mut s), 20.0, 0.5, 16), plain);
    let op_before = options_of(&s, op);
    let n = ulen(&mut s);
    assert!(s.restore_original_adjustments());
    assert_eq!(ulen(&mut s), n + 1);
    assert_eq!(
        s.history_target_name(Direction::Undo).as_deref(),
        Some("Restore original")
    );
    assert!(dither_of(&s, p).is_null(), "image_dither removed");
    assert_eq!(view(&s)["restore_enabled"], false);
    assert_eq!(options_of(&s, op), op_before, "operation untouched");
    assert_eq!(
        dots(&gcode(&mut s), 20.0, 0.5, 16),
        plain,
        "the 0123 dots again"
    );
    let _ = s.undo();
    assert_eq!(dither_of(&s, p)["mode"], "halftone", "undo brings it back");
}

// ---- criterion 9, A2: linked clones carry image_dither ----------------------------------------

/// The original in no operation, a linked clone of it alone in one Image operation at 0.5.
fn clone_fx() -> (Session, NodeId, NodeId) {
    let (d, m, o) = laser_doc();
    let p = picture(&d, o, 20.0, "Photo 1");
    let c = d.linked_clone(&[p], &[]).unwrap()[0];
    let op = image_op(&d, m, "Image 1", &[c]);
    let s = open(&d, &[(op, 0.5)]);
    (s, p, c)
}

#[test]
fn c9_c67_a_following_clone_burns_its_originals_dither() {
    let (mut s, p, c) = clone_fx();
    select(&mut s, p);
    assert!(s.set_picture_dither("stucki"));
    assert_eq!(regs(&s, c)["owner"], "CloneFollowing");
    assert_eq!(dither_of(&s, c)["mode"], "stucki", "shows the original's");
    let lone = alone(0.5, |s, q| {
        select(s, q);
        assert!(s.set_picture_dither("stucki"));
    });
    assert_eq!(dots(&gcode(&mut s), 20.0, 0.5, 16), lone);
    // A change of the original reaches the following clone.
    select(&mut s, p);
    assert!(s.set_picture_dither("ordered"));
    let lone = alone(0.5, |s, q| {
        select(s, q);
        assert!(s.set_picture_dither("ordered"));
    });
    assert_eq!(dots(&gcode(&mut s), 20.0, 0.5, 16), lone);
}

#[test]
fn c9_c67_the_clones_dither_strip_is_its_first_edit_and_copies_all_four_registers() {
    let (mut s, p, c) = clone_fx();
    select(&mut s, p);
    assert!(s.pick_adjust_preset("graphics"));
    assert!(s.set_adjust_strength(30.0).is_ok());
    assert!(s.add_adjust_step("levels"));
    let original = regs(&s, p);
    select(&mut s, c);
    let n = ulen(&mut s);
    assert!(s.set_picture_dither("atkinson"));
    assert_eq!(ulen(&mut s), n + 1, "one step");
    let own = regs(&s, c);
    assert_eq!(own["owner"], "CloneOwn");
    assert_eq!(own["adjustments"]["preset"], "graphics");
    assert_eq!(own["adjustments"]["strength"], 30);
    assert_eq!(
        own["adjustments"]["steps"],
        original["adjustments"]["steps"]
    );
    assert_eq!(own["adjustments"]["dither"]["mode"], "atkinson");
    assert_eq!(regs(&s, p), original, "the original is unchanged");
    // From now on independent: the original's dither change does not reach the clone.
    let clone_dots = dots(&gcode(&mut s), 20.0, 0.5, 16);
    select(&mut s, p);
    assert!(s.set_picture_dither("jarvis"));
    assert_eq!(dither_of(&s, c)["mode"], "atkinson");
    assert_eq!(dots(&gcode(&mut s), 20.0, 0.5, 16), clone_dots);
    // Restore original on the clone: own, empty, no image_dither; the original keeps its own.
    select(&mut s, c);
    assert!(s.restore_original_adjustments());
    let r = regs(&s, c);
    assert_eq!(r["owner"], "CloneOwn");
    assert!(r["adjustments"]["dither"].is_null(), "{r}");
    assert_eq!(dither_of(&s, p)["mode"], "jarvis");
    assert_eq!(
        dots(&gcode(&mut s), 20.0, 0.5, 16),
        alone(0.5, |_, _| {}),
        "the clone burns the picture as imported"
    );
    // Follow original: back to the original's dither.
    assert!(s.follow_original_adjustments());
    assert_eq!(regs(&s, c)["owner"], "CloneFollowing");
    assert_eq!(dither_of(&s, c)["mode"], "jarvis");
}

#[test]
fn c9_unlink_and_duplicate_carry_the_dither() {
    let (mut s, p, c) = clone_fx();
    select(&mut s, p);
    assert!(s.set_picture_dither("halftone"));
    assert_eq!(s.edit_picture_dither_field("screen_size", 1.2), Ok(true));
    let shown = dither_of(&s, c);
    assert_eq!(shown["screen_size_um"], 1_200);
    select(&mut s, c);
    let _ = s.unlink_clones();
    let d = doc(&s);
    let images: Vec<NodeId> = d
        .read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            curvyo_document_core::ObjectSnapshot::Image(i) => Some(i.id),
            _ => None,
        })
        .collect();
    assert_eq!(images.len(), 2);
    let new = images.into_iter().find(|&i| i != p).unwrap();
    let r = serde_json::to_value(d.shown_adjustments(new).unwrap()).unwrap();
    assert_eq!(r["owner"], "Image");
    assert_eq!(r["adjustments"]["dither"], shown, "Unlink keeps the dither");

    // Duplicate (Ctrl+D) of a picture copies its dither.
    select(&mut s, p);
    let _ = s.duplicate_layer_target();
    let d = doc(&s);
    let copy = d
        .read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            curvyo_document_core::ObjectSnapshot::Image(i) => Some(i.id),
            _ => None,
        })
        .find(|&i| i != p && i != new)
        .expect("duplicated");
    assert_eq!(dither_of(&s, copy), dither_of(&s, p));
}

// ---- criterion 23, A5: a damaged dither -------------------------------------------------------

#[test]
fn c23_a_damaged_dither_burns_floyd_steinberg_is_kept_and_counts_one_step() {
    let plain = alone(0.5, |_, _| {});
    let ok = [
        ("threshold_pct", 50),
        ("screen_size_um", 800),
        ("screen_angle_deg", 45),
    ];
    let with = |k: &str, v: i64| -> Vec<(&str, i64)> {
        ok.iter()
            .map(|&(kk, vv)| if kk == k { (kk, v) } else { (kk, vv) })
            .collect()
    };
    let cases: Vec<(&str, StoredDither)> = vec![
        ("unknown mode", stored("halftone_x", &ok)),
        ("missing field", stored("jarvis", &ok[..2])),
        (
            "threshold 101",
            stored("threshold", &with("threshold_pct", 101)),
        ),
        (
            "threshold -1",
            stored("threshold", &with("threshold_pct", -1)),
        ),
        (
            "size 6000",
            stored("halftone", &with("screen_size_um", 6_000)),
        ),
        ("size 190", stored("halftone", &with("screen_size_um", 190))),
        ("size 805", stored("halftone", &with("screen_size_um", 805))),
        (
            "angle 90",
            stored("line_screen", &with("screen_angle_deg", 90)),
        ),
    ];
    for (what, bad) in cases {
        let (d, m, o) = laser_doc();
        let p = picture(&d, o, 20.0, "Photo 1");
        let op = image_op(&d, m, "Image 1", &[p]);
        d.edit_image_adjustments(
            p,
            &AdjustmentsWrite {
                dither: Some(bad.clone()),
                ..AdjustmentsWrite::default()
            },
        )
        .unwrap();
        let mut s = open(&d, &[(op, 0.5)]);
        assert_eq!(
            broken_lines(&s),
            ["\"warn\": \"1 step in the Adjustments of Photo 1 could not run.\""],
            "{what}: {:?}",
            checks(&s)
        );
        assert_eq!(
            dots(&gcode(&mut s), 20.0, 0.5, 16),
            plain,
            "{what}: burns as Floyd-Steinberg at the defaults"
        );
        // Kept and saved back unchanged.
        let again = doc(&s);
        let back = serde_json::to_value(again.shown_adjustments(p).unwrap()).unwrap();
        assert_eq!(
            back["adjustments"]["dither"],
            serde_json::to_value(&bad).unwrap(),
            "{what}: kept"
        );
    }
}

#[test]
fn c23_a_damaged_dither_and_a_broken_step_count_two() {
    let (d, m, o) = laser_doc();
    let p = picture(&d, o, 20.0, "Photo 1");
    let op = image_op(&d, m, "Image 1", &[p]);
    d.edit_image_adjustments(
        p,
        &AdjustmentsWrite {
            steps: Some(vec![AdjustEntry::Step(AdjustStep {
                kind: "vignette".to_owned(),
                enabled: true,
                fields: [("amount_pct".to_owned(), 5)].into_iter().collect(),
            })]),
            dither: Some(stored("stochastic", &[("threshold_pct", 50)])),
            ..AdjustmentsWrite::default()
        },
    )
    .unwrap();
    let s = open(&d, &[(op, 0.5)]);
    assert_eq!(
        broken_lines(&s),
        ["\"warn\": \"2 steps in the Adjustments of Photo 1 could not run.\""],
        "{:?}",
        checks(&s)
    );
}
