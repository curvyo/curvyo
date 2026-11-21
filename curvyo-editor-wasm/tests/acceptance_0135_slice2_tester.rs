//! Tester acceptance cases for `specs/0135-image-optimisation` slice 2 through the editor
//! session, written from the specification (criteria 2, 3, 5, 9, 10, 14, 18, 29, 51, 52, 59)
//! before the implementation was read: insertion at the pipeline place and moves within a band,
//! the field ranges of the four new steps, Curves points (at most 8, ends fixed in Input, Input
//! strictly increasing), the Channel preset strip, Crop narrowing the frame with the asset and
//! the placement unchanged, Make editable keeping Crop and Resize with byte-identical dots,
//! Copy and Paste adjustments as one undo step "Paste adjustments" (also onto a clone, which
//! then owns its adjustments while the original stays unchanged), and the Image grid's Picture
//! pick with the Strength axis (0, 100, 5).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names,
    clippy::map_unwrap_or,
    dead_code,
    missing_docs
)]

#[path = "../../curvyo-laser-core/tests/tester_raster_0123/mod.rs"]
mod reference;

use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    Affine, Direction, Document, DocumentSize, NewImage, NewOperation, NewOutputPage, NodeId,
    ObjectSnapshot, Point, Timestamp, pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_image_core::{WorkingImage, encode_png};
use curvyo_ui_core::FontBlock;
use reference::{Gcode, read};
use serde_json::Value;

const LASER: &str = "curvyo.laser";

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

/// A 32 x 16 colour picture: a gradient with grain and a red disc.
fn photo() -> Vec<u8> {
    let mut seed = 3u32;
    png(32, 16, move |x, y| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let n = (seed >> 24) as i32 % 9 - 4;
        let g = (40 + x as i32 * 6 + n).clamp(0, 255) as u8;
        let (dx, dy) = (x as i32 - 10, y as i32 - 8);
        if dx * dx + dy * dy < 16 {
            [220, 30, 40, 255]
        } else {
            [g, g, g.saturating_sub(10), 255]
        }
    })
}

struct Fx {
    s: Session,
    origin: Point,
    image: NodeId,
    other: NodeId,
    clone: NodeId,
    op: NodeId,
}

/// A laser page; picture "Photo 1" 16 x 8 mm at page (20, 20) in one Image operation at
/// Interval 0.5; a second picture "Photo 2" at page (60, 20) in the same operation; a linked
/// clone of Photo 1 on top of it (not in any operation).
fn setup() -> Fx {
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
    let bytes = photo();
    let ids = d
        .add_images(&[
            NewImage {
                bytes: Arc::from(bytes.as_slice()),
                placement: Affine::new(16.0, 0.0, 0.0, 8.0, o.x + 20.0, o.y + 20.0),
                name: "Photo 1".to_owned(),
            },
            NewImage {
                bytes: Arc::from(bytes.as_slice()),
                placement: Affine::new(16.0, 0.0, 0.0, 8.0, o.x + 60.0, o.y + 20.0),
                name: "Photo 2".to_owned(),
            },
        ])
        .unwrap();
    let (image, other) = (ids[0], ids[1]);
    let op = d
        .add_operation(
            m,
            &NewOperation {
                kind: "image",
                name: "Image 1",
            },
            0,
            &[vec![image], vec![other]],
        )
        .unwrap();
    let clone = d.linked_clone(&[image], &[]).unwrap()[0];

    let mut s = Session::open(7, &pack(&d, "0135-s2-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    s.set_clock(|| Timestamp::from_secs(1_762_948_800));
    assert!(s.activate_page(&page.id));
    s.load_materials(&[], None);
    let _ = s.save_machine_profile([13; 16], 1_000);
    let _ = s.set_machine_name("Diode 10 W");
    assert!(s.edit_operation_setting(op, "main", "power", 60.0).is_ok());
    assert!(
        s.edit_operation_setting(op, "main", "speed", 3000.0)
            .is_ok()
    );
    let _ = s.edit_operation_option(op, "interval", 0.5);
    let _ = s.edit_operation_option(op, "overscan", 0.0);
    s.wipe_history();
    Fx {
        s,
        origin: o,
        image,
        other,
        clone,
        op,
    }
}

fn click(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

fn select_at(fx: &mut Fx, x: f64) {
    let at = Point::new(fx.origin.x + x + 1.0, fx.origin.y + 21.0);
    click(&mut fx.s, at);
    assert!(view(&fx.s).is_object(), "a picture is selected at {x}");
}

fn select_image(fx: &mut Fx) {
    select_at(fx, 20.0);
    if !view(&fx.s)["clone_row"].is_null() {
        // The clone lies on top of its original: go to the original.
        let _ = fx.s.select_original();
    }
    assert!(view(&fx.s)["clone_row"].is_null());
}

fn select_other(fx: &mut Fx) {
    select_at(fx, 60.0);
}

fn select_clone(fx: &mut Fx) {
    select_image(fx);
    let _ = fx.s.select_clones();
    assert!(!view(&fx.s)["clone_row"].is_null(), "the clone is selected");
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

fn kinds(s: &Session) -> Vec<String> {
    view(s)["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["kind"].as_str().unwrap().to_owned())
        .collect()
}

fn ulen(s: &mut Session) -> usize {
    let _ = s.history_target_name(Direction::Undo);
    s.undo_len()
}

fn undo_name(s: &mut Session) -> Option<String> {
    s.history_target_name(Direction::Undo)
}

fn image_snap(s: &Session, id: NodeId) -> curvyo_document_core::ImageSnapshot {
    doc(s)
        .read_objects()
        .objects
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Image(i) if i.id == id => Some(i.clone()),
            _ => None,
        })
        .unwrap()
}

fn gcode(s: &mut Session) -> Gcode {
    let bytes = s
        .export_job("Untitled.curvyo")
        .map(|e| e.bytes)
        .unwrap_or_else(|| panic!("export blocked: {}", s.output_group_json()));
    read(&bytes).unwrap()
}

fn field_value(s: &Session, row: usize, key: &str) -> f64 {
    view(s)["steps"][row]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["key"] == key)
        .unwrap_or_else(|| panic!("field {key} in row {row}"))["value"]
        .as_f64()
        .unwrap()
}

fn curve_points(s: &Session, row: usize) -> Vec<(u64, u64)> {
    view(s)["steps"][row]["body"]["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p[0].as_u64().unwrap(), p[1].as_u64().unwrap()))
        .collect()
}

// ---- criterion 3: insertion and moves ---------------------------------------------------------

#[test]
fn c3_new_steps_are_inserted_at_their_pipeline_place() {
    let mut fx = setup();
    select_image(&mut fx);
    for k in [
        "sharpen",
        "curves",
        "crop",
        "channel_mix",
        "levels",
        "resize",
        "denoise",
    ] {
        assert!(fx.s.add_adjust_step(k), "add {k}");
    }
    assert_eq!(
        kinds(&fx.s),
        [
            "crop",
            "resize",
            "channel_mix",
            "denoise",
            "levels",
            "curves",
            "sharpen"
        ]
    );
    // A second step of a kind is allowed.
    assert!(fx.s.add_adjust_step("curves"));
    assert_eq!(kinds(&fx.s).iter().filter(|k| *k == "curves").count(), 2);
    let geo: Vec<String> = view(&fx.s)["add_geometry"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["kind"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(geo, ["crop", "resize"]);
}

#[test]
fn c3_geometry_rows_stay_first_and_tone_rows_reorder_among_themselves() {
    let mut fx = setup();
    select_image(&mut fx);
    for k in ["crop", "resize", "channel_mix", "curves"] {
        assert!(fx.s.add_adjust_step(k));
    }
    // Crop below Resize, or Resize into the tone band, is not offered.
    assert!(
        !fx.s.move_adjust_step(0, false),
        "Crop cannot go below Resize"
    );
    assert!(
        !fx.s.move_adjust_step(1, false),
        "Resize cannot go into the tone band"
    );
    assert!(
        !fx.s.move_adjust_step(2, true),
        "Channel mix cannot go above Resize"
    );
    assert_eq!(kinds(&fx.s), ["crop", "resize", "channel_mix", "curves"]);
    // Tone rows may change their order.
    assert!(fx.s.move_adjust_step(3, true));
    assert_eq!(kinds(&fx.s), ["crop", "resize", "curves", "channel_mix"]);
    let v = view(&fx.s);
    assert_eq!(v["steps"][0]["down"], false);
    assert_eq!(v["steps"][1]["up"], false);
    assert_eq!(v["steps"][2]["up"], false);
}

#[test]
fn c5_resize_above_the_working_width_is_refused_with_the_range() {
    let mut fx = setup();
    select_image(&mut fx);
    assert!(fx.s.add_adjust_step("resize"));
    let e = fx.s.edit_adjust_field(0, "width_px", 64.0).unwrap_err();
    assert!(
        e.contains("16 to 32"),
        "the range names the working width: {e}"
    );
}

// ---- criterion 5: fields and ranges -----------------------------------------------------------

#[test]
fn c5_crop_fields_refuse_out_of_range_with_the_range() {
    let mut fx = setup();
    select_image(&mut fx);
    assert!(fx.s.add_adjust_step("crop"));
    let n = ulen(&mut fx.s);
    let e = fx.s.edit_adjust_field(0, "width_pct", 0.0).unwrap_err();
    assert!(e.contains("1 to 100"), "{e}");
    let e = fx.s.edit_adjust_field(0, "left_pct", 100.0).unwrap_err();
    // UX review UX-2 (2026-11-21): the range as it stands, Left 0 to 100 minus Width (was "0 to 99").
    assert!(e.contains("0 to 0"), "{e}");
    assert_eq!(ulen(&mut fx.s), n, "a refused value writes nothing");
    // Rounded to the field's step.
    assert!(fx.s.edit_adjust_field(0, "width_pct", 40.4).is_ok());
    assert_eq!(field_value(&fx.s, 0, "width_pct"), 40.0);
    assert_eq!(undo_name(&mut fx.s).as_deref(), Some("Edit adjustments"));
}

#[test]
fn c5_c14_resize_is_16_to_the_working_width_and_only_reduces() {
    let mut fx = setup();
    select_image(&mut fx);
    assert!(fx.s.add_adjust_step("resize"));
    assert_eq!(
        field_value(&fx.s, 0, "width_px"),
        32.0,
        "a new Resize keeps the width"
    );
    let e = fx.s.edit_adjust_field(0, "width_px", 15.0).unwrap_err();
    assert!(e.contains("16"), "{e}");
    assert!(fx.s.edit_adjust_field(0, "width_px", 16.0).is_ok());
    assert_eq!(
        view(&fx.s)["steps"][0]["body"]["height_px"],
        8,
        "the height follows"
    );
    let line = view(&fx.s)["steps"][0]["body"]["line"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        line.contains("32") && line.contains("16"),
        "the working-image line: {line}"
    );
    // Above the working image's width there is never an enlargement: the dots are those of the
    // whole width.
    assert!(fx.s.edit_adjust_field(0, "width_px", 32.0).is_ok());
    let whole = gcode(&mut fx.s);
    if fx.s.edit_adjust_field(0, "width_px", 64.0).is_ok() {
        let over = gcode(&mut fx.s);
        assert_eq!(
            format!("{:?}", over.moves),
            format!("{:?}", whole.moves),
            "no enlargement"
        );
    }
}

#[test]
fn c5_channel_mix_presets_set_the_three_fields_in_one_step() {
    let mut fx = setup();
    select_image(&mut fx);
    assert!(fx.s.add_adjust_step("channel_mix"));
    let v = view(&fx.s);
    let names: Vec<&str> = v["steps"][0]["body"]["presets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        ["Luminance", "Red filter", "Green filter", "Blue filter"]
    );
    assert_eq!(
        v["steps"][0]["body"]["presets"][0]["pressed"], true,
        "the default is Luminance"
    );
    assert_eq!(
        (
            field_value(&fx.s, 0, "red_pct"),
            field_value(&fx.s, 0, "green_pct"),
            field_value(&fx.s, 0, "blue_pct")
        ),
        (21.0, 72.0, 7.0)
    );
    for (i, w) in [
        (1usize, (100.0, 0.0, 0.0)),
        (2, (0.0, 100.0, 0.0)),
        (3, (0.0, 0.0, 100.0)),
    ] {
        let n = ulen(&mut fx.s);
        assert!(fx.s.set_adjust_channel_preset(0, i));
        assert_eq!(ulen(&mut fx.s), n + 1, "one undo step");
        assert_eq!(
            (
                field_value(&fx.s, 0, "red_pct"),
                field_value(&fx.s, 0, "green_pct"),
                field_value(&fx.s, 0, "blue_pct")
            ),
            w
        );
        assert_eq!(
            view(&fx.s)["steps"][0]["body"]["presets"][i]["pressed"],
            true
        );
    }
    let e = fx.s.edit_adjust_field(0, "red_pct", 201.0).unwrap_err();
    assert!(e.contains("-200 to 200"), "{e}");
    assert!(fx.s.edit_adjust_field(0, "red_pct", -200.0).is_ok());
    assert!(!fx.s.set_adjust_channel_preset(0, 4), "no fifth preset");
}

#[test]
fn c5_curves_points_ends_fixed_in_input_at_most_eight_input_increasing() {
    let mut fx = setup();
    select_image(&mut fx);
    assert!(fx.s.add_adjust_step("curves"));
    assert_eq!(
        curve_points(&fx.s, 0),
        [(0, 0), (100, 100)],
        "a straight curve"
    );
    // The end points keep their Input; only their Output moves.
    assert!(fx.s.move_adjust_curve_point(0, 0, 0, 10));
    let _ = fx.s.move_adjust_curve_point(0, 0, 30, 20);
    let _ = fx.s.move_adjust_curve_point(0, 1, 70, 90);
    let p = curve_points(&fx.s, 0);
    assert_eq!(p.first().unwrap().0, 0, "{p:?}");
    assert_eq!(p.last().unwrap().0, 100, "{p:?}");
    // The end points cannot be removed.
    assert!(!fx.s.remove_adjust_curve_point(0, 0));
    assert!(!fx.s.remove_adjust_curve_point(0, p.len() - 1));
    // Add up to eight points.
    for x in [10u8, 25, 40, 55, 70, 85] {
        assert!(fx.s.add_adjust_curve_point(0, x, x), "add at {x}");
    }
    assert_eq!(curve_points(&fx.s, 0).len(), 8);
    assert!(!fx.s.add_adjust_curve_point(0, 50, 50), "at most 8 points");
    // Input stays strictly increasing: a point does not pass its neighbours.
    let _ = fx.s.move_adjust_curve_point(0, 2, 95, 50);
    let p = curve_points(&fx.s, 0);
    assert!(
        p.windows(2).all(|w| w[0].0 < w[1].0),
        "Input strictly increasing: {p:?}"
    );
    assert!(p.iter().all(|q| q.1 <= 100));
    // A point at an Input that exists is refused.
    assert!(fx.s.remove_adjust_curve_point(0, 3));
    let p = curve_points(&fx.s, 0);
    assert!(
        !fx.s.add_adjust_curve_point(0, p[2].0 as u8, 10),
        "same Input twice"
    );
    // Straight curve resets.
    assert!(fx.s.straighten_adjust_curve(0));
    assert_eq!(curve_points(&fx.s, 0).len(), 2);
}

// ---- criterion 5, 18, 59: Crop on the object --------------------------------------------------

#[test]
fn c5_c18_c59_a_crop_narrows_the_frame_keeps_the_asset_and_restore_gives_it_back() {
    let mut fx = setup();
    let whole = image_snap(&fx.s, fx.image);
    select_image(&mut fx);
    assert!(fx.s.add_adjust_step("crop"));
    assert_eq!(
        view(&fx.s)["restore_enabled"],
        true,
        "a crop is something to restore"
    );
    assert!(fx.s.open_crop_bar(0));
    let n = ulen(&mut fx.s);
    assert!(fx.s.set_crop_rect(25, 0, 50, 50));
    assert_eq!(ulen(&mut fx.s), n + 1, "a crop release is one step");
    assert_eq!(undo_name(&mut fx.s).as_deref(), Some("Edit adjustments"));
    fx.s.close_crop_bar();
    let s = image_snap(&fx.s, fx.image);
    assert_eq!(s.asset, whole.asset, "asset unchanged");
    assert_eq!(s.placement, whole.placement, "image_placement kept");
    let c = s.corners();
    let o = fx.origin;
    let want = [
        Point::new(o.x + 24.0, o.y + 20.0),
        Point::new(o.x + 32.0, o.y + 20.0),
        Point::new(o.x + 32.0, o.y + 24.0),
        Point::new(o.x + 24.0, o.y + 24.0),
    ];
    for (g, w) in c.iter().zip(want) {
        assert!(
            (g.x - w.x).abs() < 1e-9 && (g.y - w.y).abs() < 1e-9,
            "{c:?}"
        );
    }
    // The job burns only inside the visible part.
    let g = gcode(&mut fx.s);
    let burned: Vec<f64> = g
        .moves
        .iter()
        .filter(|m| m.burn && m.s > 0)
        .flat_map(|m| [m.from.0, m.to.0])
        .collect();
    assert!(!burned.is_empty(), "something burns");
    let (lo, hi) = (24.0 - 0.5, 32.0 + 0.5);
    for x in &burned {
        if *x < 50.0 {
            assert!(
                *x >= lo && *x <= hi,
                "Photo 1 burns only in its cropped part: x {x}"
            );
        }
    }
    // Restore original: the whole frame again, the asset untouched.
    assert!(fx.s.restore_original_adjustments());
    let r = image_snap(&fx.s, fx.image);
    assert_eq!(r.corners(), whole.corners());
    assert_eq!(r.asset, whole.asset);
}

// ---- criterion 29: Make editable keeps the geometry -------------------------------------------

#[test]
fn c29_make_editable_keeps_crop_and_resize_and_the_dots() {
    let mut fx = setup();
    select_image(&mut fx);
    assert!(fx.s.add_adjust_step("crop"));
    assert!(fx.s.edit_adjust_field(0, "width_pct", 75.0).is_ok());
    assert!(fx.s.add_adjust_step("resize"));
    assert!(fx.s.edit_adjust_field(1, "width_px", 16.0).is_ok());
    assert!(fx.s.add_adjust_step("curves"));
    assert!(fx.s.pick_adjust_preset("photo"));
    let v = view(&fx.s);
    let rows = v["steps"].as_array().unwrap();
    assert_eq!(rows[0]["preset_in_use"], false, "Crop always runs");
    assert_eq!(rows[1]["preset_in_use"], false, "Resize always runs");
    assert_eq!(rows[2]["preset_in_use"], true, "Curves under a preset");
    let before = gcode(&mut fx.s);
    assert!(fx.s.make_adjust_editable());
    let after = gcode(&mut fx.s);
    assert_eq!(
        format!("{:?}", before.moves),
        format!("{:?}", after.moves),
        "the dots are byte-identical"
    );
    let k = kinds(&fx.s);
    assert_eq!(&k[..2], ["crop", "resize"], "{k:?}");
    assert!(
        !k.contains(&"curves".to_owned()),
        "Preset in use rows are replaced: {k:?}"
    );
    assert_eq!(field_value(&fx.s, 0, "width_pct"), 75.0);
    assert_eq!(field_value(&fx.s, 1, "width_px"), 16.0);
    let p = regs(&fx.s, fx.image)["adjustments"]["preset"].clone();
    assert!(p.is_null() || p == "expert", "Preset becomes Expert: {p}");
}

// ---- criterion 10: Copy and Paste adjustments -------------------------------------------------

#[test]
fn c10_paste_adjustments_replaces_preset_strength_stack_and_dither_in_one_step() {
    let mut fx = setup();
    select_image(&mut fx);
    assert!(view(&fx.s)["paste_enabled"] == false, "nothing copied yet");
    assert!(fx.s.add_adjust_step("crop"));
    assert!(fx.s.add_adjust_step("curves"));
    assert!(fx.s.pick_adjust_preset("newspaper"));
    assert!(fx.s.set_adjust_strength(80.0).is_ok());
    let n = ulen(&mut fx.s);
    assert!(fx.s.copy_adjustments());
    assert_eq!(ulen(&mut fx.s), n, "Copy writes nothing");
    let source = regs(&fx.s, fx.image)["adjustments"].clone();

    select_other(&mut fx);
    assert!(fx.s.add_adjust_step("sharpen"));
    assert_eq!(view(&fx.s)["paste_enabled"], true);
    let target_before = regs(&fx.s, fx.other);
    let n = ulen(&mut fx.s);
    assert!(fx.s.paste_adjustments());
    assert_eq!(ulen(&mut fx.s), n + 1, "one undo step");
    assert_eq!(undo_name(&mut fx.s).as_deref(), Some("Paste adjustments"));
    let pasted = regs(&fx.s, fx.other)["adjustments"].clone();
    for k in ["preset", "strength", "steps", "dither"] {
        assert_eq!(pasted[k], source[k], "{k} travels");
    }
    assert_eq!(
        regs(&fx.s, fx.image)["adjustments"],
        source,
        "the source is unchanged"
    );
    // Both now burn the same dots for the same picture bytes (each in its own place).
    let _ = fx.s.undo();
    assert_eq!(
        regs(&fx.s, fx.other),
        target_before,
        "undo restores the target"
    );
}

#[test]
fn c9_c10_pasting_onto_a_following_clone_gives_it_own_adjustments_and_leaves_the_original() {
    let mut fx = setup();
    select_other(&mut fx);
    assert!(fx.s.add_adjust_step("resize"));
    assert!(fx.s.pick_adjust_preset("graphics"));
    assert!(fx.s.copy_adjustments());
    let copied = regs(&fx.s, fx.other)["adjustments"].clone();
    let original_before = regs(&fx.s, fx.image);

    select_clone(&mut fx);
    let clone_owner_before = regs(&fx.s, fx.clone)["owner"].clone();
    let n = ulen(&mut fx.s);
    assert!(fx.s.paste_adjustments());
    assert_eq!(ulen(&mut fx.s), n + 1, "one undo step");
    assert_eq!(undo_name(&mut fx.s).as_deref(), Some("Paste adjustments"));
    let c = regs(&fx.s, fx.clone);
    for k in ["preset", "strength", "steps", "dither"] {
        assert_eq!(c["adjustments"][k], copied[k], "{k} on the clone");
    }
    assert_ne!(
        c["owner"], clone_owner_before,
        "the clone now owns its adjustments: {c}"
    );
    assert_eq!(
        regs(&fx.s, fx.image),
        original_before,
        "the original is unchanged"
    );
    let row = view(&fx.s)["clone_row"].clone();
    assert!(
        row.to_string().to_lowercase().contains("own"),
        "the override row shows Own: {row}"
    );
}

// ---- criterion 51, 52: the Image grid's Picture pick and Strength axis -------------------------

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn load_fonts(s: &mut Session) {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
            )
        })
        .collect();
    s.set_font_list(FontBlock::Included, faces, Vec::new());
    s.set_tool(Tool::TestGrid);
    let _ = s.test_grid_press(None);
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes));
    }
    s.set_tool(Tool::Select);
}

fn grid_view(s: &Session) -> Value {
    serde_json::from_str(&s.test_grid_view_json()).unwrap()
}

#[test]
fn c51_c52_the_picture_pick_and_the_strength_axis_0_100_5() {
    let mut fx = setup();
    load_fonts(&mut fx.s);
    select_image(&mut fx);
    assert!(fx.s.pick_adjust_preset("photo"));
    let source_before = regs(&fx.s, fx.image);
    fx.s.set_tool(Tool::TestGrid);
    fx.s.set_test_grid_kind("image").unwrap();
    let g = grid_view(&fx.s);
    assert_eq!(g["picture"]["value"], "ramp", "Ramp is the default");
    assert_eq!(
        g["picture"]["selected_offered"], true,
        "a picture is selected"
    );
    assert_eq!(
        g["picture"]["strength_offered"], false,
        "no Strength with the Ramp"
    );
    assert!(!g["dither"].is_null(), "the Ramp has Dither segments");
    assert_eq!(fx.s.set_test_grid_picture("selected"), Ok(true));
    let g = grid_view(&fx.s);
    assert!(
        g["dither"].is_null(),
        "no Dither segments with Selected picture: {g}"
    );
    assert_eq!(g["picture"]["strength_offered"], true);
    assert!(
        g["columns"]["offered"]
            .as_array()
            .unwrap()
            .contains(&"strength".into()),
        "{g}"
    );
    assert_eq!(fx.s.set_test_grid_axis("columns", "strength"), Ok(true));
    let g = grid_view(&fx.s);
    assert_eq!(g["columns"]["param"], "strength");
    assert_eq!(
        (
            g["columns"]["min"].as_f64(),
            g["columns"]["max"].as_f64(),
            g["columns"]["count"].as_u64()
        ),
        (Some(0.0), Some(100.0), Some(5)),
        "the Strength axis defaults to 0, 100, 5: {}",
        g["columns"]
    );
    let o = fx.origin;
    let group =
        fx.s.test_grid_press(Some(Point::new(o.x + 150.0, o.y + 100.0)))
            .unwrap();
    fx.s.set_tool(Tool::Select);
    let d = doc(&fx.s);
    let cells: Vec<NodeId> = d
        .leaves_of(group)
        .into_iter()
        .filter(|leaf| {
            d.read_objects()
                .objects
                .iter()
                .any(|o| matches!(o, ObjectSnapshot::Image(i) if i.id == *leaf))
        })
        .collect();
    let rows = grid_view(&fx.s)["rows"]["count"].as_u64().unwrap_or(1) as usize;
    assert_eq!(cells.len(), 5 * rows, "one picture per cell");
    let mut strengths: Vec<u64> = cells
        .iter()
        .map(|c| d.shown_adjustments(*c).unwrap())
        .map(|a| {
            serde_json::to_value(a).unwrap()["adjustments"]["strength"]
                .as_u64()
                .unwrap()
        })
        .collect();
    strengths.sort_unstable();
    strengths.dedup();
    assert_eq!(strengths, [0, 25, 50, 75, 100], "each cell at its Strength");
    for c in &cells {
        let a = serde_json::to_value(d.shown_adjustments(*c).unwrap()).unwrap();
        assert_eq!(
            a["adjustments"]["preset"],
            source_before["adjustments"]["preset"]
        );
        assert_eq!(
            a["adjustments"]["dither"],
            source_before["adjustments"]["dither"]
        );
    }
    assert_eq!(
        regs(&fx.s, fx.image),
        source_before,
        "the picture itself is not changed"
    );
    // Expert: no Strength axis.
    select_image(&mut fx);
    assert!(fx.s.pick_adjust_preset("expert"));
    fx.s.set_tool(Tool::TestGrid);
    fx.s.set_test_grid_kind("image").unwrap();
    let _ = fx.s.set_test_grid_picture("selected");
    assert_eq!(
        grid_view(&fx.s)["picture"]["strength_offered"],
        false,
        "Expert: no Strength"
    );
}
