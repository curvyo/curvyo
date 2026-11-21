//! The contract of the Adjustments group's and the Before and after pane's JSON (`specs/0135`
//! criteria 1, 8, 9, 11, 12, 23, 25, 28, 59): representative states are built in a session and
//! their JSON compared with the shared fixtures in `tests/fixtures/adjustments_view/` and
//! `tests/fixtures/before_after_view/`, which the frontend decoders read too. A drift fails. The
//! fixtures are written only when `CURVYO_UPDATE_GOLDEN` is set.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    AdjustEntry, AdjustStep, AdjustmentsWrite, Affine, NewImage, NodeId, Vec2,
};
use curvyo_image_core::{WorkingImage, encode_png};
use serde_json::Value;

use crate::session::Session;

fn ramp_png() -> Vec<u8> {
    let (width, height) = (40, 20);
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let g = u8::try_from((x * 200 / width + y * 3) % 230 + 20).unwrap();
            pixels.extend_from_slice(&[g, g, g, 255]);
        }
    }
    encode_png(&WorkingImage {
        width,
        height,
        pixels,
        original_size: (width, height),
        truncated: false,
        animated: false,
    })
    .unwrap()
}

/// A laser page with one picture of 8 x 4 mm in `operations` Image operations at 0.10, 0.20 mm.
fn engraved(operations: usize) -> (Session, NodeId) {
    let mut s = Session::new(290_137);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let origin = s.active_manufacturing().unwrap().page.unwrap().origin;
    let image = s
        .document
        .add_images(&[NewImage {
            bytes: Arc::from(ramp_png().as_slice()),
            placement: Affine::new(8.0, 0.0, 0.0, 4.0, origin.x + 10.0, origin.y + 10.0),
            name: "Photo 1".to_owned(),
        }])
        .unwrap()[0];
    for (i, interval) in [0.1, 0.2].iter().take(operations).enumerate() {
        s.selection.set(&[image]);
        let op = s.press_operation_kind("image", i > 0).unwrap();
        s.edit_operation_setting(op, "main", "power", 50.0).unwrap();
        s.edit_operation_setting(op, "main", "speed", 6000.0)
            .unwrap();
        let _ = s.edit_operation_option(op, "interval", *interval);
    }
    s.selection.set(&[image]);
    (s, image)
}

fn step(kind: &str, enabled: bool, fields: &[(&str, i64)]) -> AdjustEntry {
    AdjustEntry::Step(AdjustStep {
        kind: kind.to_owned(),
        enabled,
        fields: fields
            .iter()
            .map(|(k, v)| ((*k).to_owned(), *v))
            .collect::<BTreeMap<_, _>>(),
    })
}

fn write_steps(s: &Session, image: NodeId, steps: Vec<AdjustEntry>) {
    assert!(
        s.document
            .edit_image_adjustments(
                image,
                &AdjustmentsWrite {
                    steps: Some(steps),
                    ..AdjustmentsWrite::default()
                }
            )
            .unwrap()
    );
}

/// A clone of the selected picture, 20 mm to the right, selected.
fn cloned(s: &mut Session) -> NodeId {
    s.linked_clone();
    let clone = s.selection.ids()[0];
    s.document
        .translate_objects(&[clone], Vec2::new(20.0, 0.0))
        .unwrap();
    s.selection.set(&[clone]);
    clone
}

fn adjustments_states() -> Vec<(&'static str, Session)> {
    let (mut none, _) = engraved(1);
    none.selection.set(&[]);
    let (mut photo, _) = engraved(1);
    assert!(photo.pick_adjust_preset("photo"));
    let (mut expert, _) = engraved(1);
    assert!(expert.add_adjust_step("levels"));
    assert!(expert.add_adjust_step("sharpen"));
    assert!(expert.set_adjust_step_enabled(1, false));
    let (mut original, _) = engraved(1);
    assert!(original.pick_adjust_preset("graphics"));
    cloned(&mut original);
    let (mut own, _) = engraved(1);
    assert!(own.pick_adjust_preset("graphics"));
    cloned(&mut own);
    assert!(own.set_adjust_strength(80.0).unwrap());
    let (damaged, image) = engraved(1);
    write_steps(
        &damaged,
        image,
        vec![
            step(
                "levels",
                true,
                &[("black_pct", 150), ("white_pct", 100), ("gamma_centi", 100)],
            ),
            step(
                "sharpen",
                true,
                &[
                    ("radius_um", 100),
                    ("amount_pct", 100),
                    ("threshold_pct", 0),
                ],
            ),
        ],
    );
    let (unknown, image) = engraved(1);
    write_steps(
        &unknown,
        image,
        vec![
            step(
                "levels",
                true,
                &[("black_pct", 5), ("white_pct", 95), ("gamma_centi", 100)],
            ),
            step("curves_v9", true, &[("points", 3)]),
        ],
    );
    // Criterion 56: Levels does not fit a run of 5 x the picture's 800 pixels.
    let (mut short, _) = engraved(1);
    assert!(short.add_adjust_step("levels"));
    assert!(short.add_adjust_step("sharpen"));
    short.adjust_pictures().set_run_bytes(5 * 800);
    let _ = short.laser_job();
    // Slice 2: one row of each new kind with its body, and Paste enabled after a Copy.
    let (mut slice2, _) = engraved(1);
    for kind in ["crop", "resize", "channel_mix", "curves"] {
        assert!(slice2.add_adjust_step(kind));
    }
    assert!(slice2.add_adjust_curve_point(3, 40, 25));
    assert!(slice2.set_adjust_channel_preset(2, 1));
    assert!(slice2.copy_adjustments());
    vec![
        ("slice2_steps", slice2),
        ("no_image", none),
        ("memory_short", short),
        ("preset_photo", photo),
        ("expert_steps", expert),
        ("clone_original", original),
        ("clone_own", own),
        ("damaged_step", damaged),
        ("unknown_step", unknown),
    ]
}

fn before_after_states() -> Vec<(&'static str, Session)> {
    let (mut empty, _) = engraved(1);
    assert!(empty.toggle_before_after());
    empty.selection.set(&[]);
    let (mut two, _) = engraved(2);
    assert!(two.pick_adjust_preset("photo"));
    assert!(two.toggle_before_after());
    let _ = two.laser_job();
    vec![("no_image", empty), ("open_two_operations", two)]
}

fn check(folder: &str, name: &str, json: &str) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(folder);
    let value: Value = serde_json::from_str(json).unwrap();
    let path = dir.join(format!("{name}.json"));
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(&dir).unwrap();
        let text = format!("{}\n", serde_json::to_string_pretty(&value).unwrap());
        std::fs::write(&path, text).unwrap();
    }
    let want: Value = serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("{} exists (CURVYO_UPDATE_GOLDEN=1)", path.display())),
    )
    .unwrap();
    assert_eq!(value, want, "{name} drifted from {}", path.display());
}

#[test]
fn every_adjustments_state_matches_its_fixture() {
    for (name, s) in adjustments_states() {
        check("adjustments_view", name, &s.adjustments_view_json());
    }
}

#[test]
fn every_before_after_state_matches_its_fixture() {
    for (name, s) in before_after_states() {
        check("before_after_view", name, &s.before_after_view_json());
    }
}

/// The states show what their names say.
#[test]
fn the_states_are_what_they_are_named() {
    let states = adjustments_states();
    let view = |name: &str| {
        let s = &states.iter().find(|(n, _)| *n == name).unwrap().1;
        serde_json::from_str::<Value>(&s.adjustments_view_json()).unwrap()
    };
    assert!(view("no_image").is_null());
    assert!(view("preset_photo")["derived"].is_object());
    assert_eq!(view("expert_steps")["steps"].as_array().unwrap().len(), 2);
    assert_eq!(view("slice2_steps")["steps"].as_array().unwrap().len(), 4);
    assert_eq!(view("slice2_steps")["paste_enabled"], true);
    assert_eq!(view("clone_original")["clone_row"], "original");
    assert_eq!(view("clone_own")["clone_row"], "own");
    assert!(
        view("damaged_step")["steps"][0]["unknown"]
            .as_bool()
            .unwrap()
    );
    assert_eq!(view("unknown_step")["steps"][1]["kind"], "unknown");
    let panes = before_after_states();
    let pane = serde_json::from_str::<Value>(&panes[1].1.before_after_view_json()).unwrap();
    assert_eq!(pane["operations"].as_array().unwrap().len(), 2);
    assert_eq!(pane["after"]["reduced"], false);
}
