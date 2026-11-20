//! The goldens of `specs/0135-image-optimisation` criteria 42, 43 and 54 on the laser side:
//! the two screen dithers on the ramp (`tests/fixtures/raster/halftone_dots_ramp_*.txt`,
//! `line_screen_ramp_*.txt`), a manual stack on the ramp (`ramp_adjusted_stack.txt`), and per
//! preset the derived stack as JSON and its dot map at Strength 0, 50 and 100 on its golden
//! picture (`tests/fixtures/adjust/preset_<name>_s<n>.{json,txt}`; the pictures are the
//! image-core goldens `photo.png`, `graphic.png`, `pen.png`). `CURVYO_UPDATE_GOLDEN=1` rewrites
//! the files natively; review the diff. The goldens are embedded with `include_bytes!` so the
//! same comparison runs natively and in Node on `wasm32-unknown-unknown` (ADR 0073 §1):
//!
//! ```text
//! cargo test --target wasm32-unknown-unknown -p curvyo-laser-core --test adjust_goldens
//! ```
//!
//! A new golden needs an empty file and a row in [`GOLDENS`] first.

#![allow(clippy::unwrap_used, clippy::missing_panics_doc, missing_docs)]

mod job_fixtures;

use curvyo_image_core::{WorkingImage, decode_working};
use curvyo_laser_core::adjust::{
    AdjustRegisters, ManualStep, Preset, StepKind, StoredStep, adjust_picture,
};
use curvyo_laser_core::job::LeafGeometry;
use curvyo_laser_core::settings::{Dither, ImageSettings, Interval, OperationOptions};
use job_fixtures::{grey_image, picture, plain};
use serde_json::{Value, json};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

/// Superseded by the Dither on the picture (2026-11-20): the picture leaf holds the dither the
/// operation's `options` name (ADR 0072 amendment A4), so the dot maps keep their bytes.
fn raster_dot_map(leaf: &LeafGeometry, options: &OperationOptions, interval: Interval) -> String {
    curvyo_laser_core::job::raster_dot_map(
        &job_fixtures::with_dither(leaf.clone(), job_fixtures::dither_of(&options.image)),
        options,
        interval,
    )
}

/// Superseded by the Dither on the picture (2026-11-20): as [`raster_dot_map`].
fn raster_dot_map_adjusted(
    leaf: &LeafGeometry,
    options: &OperationOptions,
    interval: Interval,
    adjusted: &curvyo_image_core::Adjusted,
) -> String {
    curvyo_laser_core::job::raster_dot_map_adjusted(
        &job_fixtures::with_dither(leaf.clone(), job_fixtures::dither_of(&options.image)),
        options,
        interval,
        adjusted,
    )
}

macro_rules! goldens {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_bytes!(concat!("fixtures/", $name)))),*]
    };
}

/// Every stored golden under `tests/fixtures/`, embedded at compile time.
const GOLDENS: &[(&str, &[u8])] = goldens![
    "raster/halftone_dots_ramp_0.txt",
    "raster/halftone_dots_ramp_45.txt",
    "raster/line_screen_ramp_0.txt",
    "raster/line_screen_ramp_45.txt",
    "raster/ramp_adjusted_stack.txt",
    "adjust/preset_photo_s0.json",
    "adjust/preset_photo_s0.txt",
    "adjust/preset_photo_s50.json",
    "adjust/preset_photo_s50.txt",
    "adjust/preset_photo_s100.json",
    "adjust/preset_photo_s100.txt",
    "adjust/preset_graphics_s0.json",
    "adjust/preset_graphics_s0.txt",
    "adjust/preset_graphics_s50.json",
    "adjust/preset_graphics_s50.txt",
    "adjust/preset_graphics_s100.json",
    "adjust/preset_graphics_s100.txt",
    "adjust/preset_black_white_s0.json",
    "adjust/preset_black_white_s0.txt",
    "adjust/preset_black_white_s50.json",
    "adjust/preset_black_white_s50.txt",
    "adjust/preset_black_white_s100.json",
    "adjust/preset_black_white_s100.txt",
    "adjust/preset_line_art_s0.json",
    "adjust/preset_line_art_s0.txt",
    "adjust/preset_line_art_s50.json",
    "adjust/preset_line_art_s50.txt",
    "adjust/preset_line_art_s100.json",
    "adjust/preset_line_art_s100.txt",
    "adjust/preset_dot_raster_s0.json",
    "adjust/preset_dot_raster_s0.txt",
    "adjust/preset_dot_raster_s50.json",
    "adjust/preset_dot_raster_s50.txt",
    "adjust/preset_dot_raster_s100.json",
    "adjust/preset_dot_raster_s100.txt",
    "adjust/preset_newspaper_s0.json",
    "adjust/preset_newspaper_s0.txt",
    "adjust/preset_newspaper_s50.json",
    "adjust/preset_newspaper_s50.txt",
    "adjust/preset_newspaper_s100.json",
    "adjust/preset_newspaper_s100.txt",
];

/// The image-core golden pictures.
const PICTURES: &[(&str, &[u8])] = &[
    (
        "photo",
        include_bytes!("../../curvyo-image-core/tests/fixtures/adjust/photo.png"),
    ),
    (
        "graphic",
        include_bytes!("../../curvyo-image-core/tests/fixtures/adjust/graphic.png"),
    ),
    (
        "pen",
        include_bytes!("../../curvyo-image-core/tests/fixtures/adjust/pen.png"),
    ),
];

/// Writes `bytes` as the golden `name` when `CURVYO_UPDATE_GOLDEN` is set (native only).
#[cfg(not(target_arch = "wasm32"))]
fn update(name: &str, bytes: &[u8]) -> bool {
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_none() {
        return false;
    }
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
    true
}

#[cfg(target_arch = "wasm32")]
fn update(_: &str, _: &[u8]) -> bool {
    false
}

/// `bytes` against the stored golden `name` (written instead when updating).
fn check(name: &str, bytes: &[u8]) {
    if update(name, bytes) {
        return;
    }
    let stored = GOLDENS
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap_or_else(|| panic!("missing golden {name}"))
        .1;
    assert!(stored == bytes, "{name} differs from its golden");
}

fn tenth() -> Interval {
    Interval::from_hundredths(10).unwrap()
}

fn options(image: ImageSettings) -> OperationOptions {
    OperationOptions { image, ..plain() }
}

/// The 16-step ramp of the `0123` goldens.
fn ramp() -> LeafGeometry {
    picture(
        0.0,
        0.0,
        3.2,
        3.2,
        grey_image(32, 32, |x, _| u8::try_from(x * 255 / 31).unwrap()),
    )
}

#[test]
fn screen_dot_maps_of_the_ramp() {
    for (dither, name) in [
        (Dither::Halftone, "halftone_dots_ramp"),
        (Dither::LineScreen, "line_screen_ramp"),
    ] {
        for angle in [0_u8, 45] {
            let image = ImageSettings {
                dither,
                screen_size_hundredths: 80,
                screen_angle_deg: angle,
                ..ImageSettings::DEFAULT
            };
            let text = raster_dot_map(&ramp(), &options(image), tenth());
            check(&format!("raster/{name}_{angle}.txt"), text.as_bytes());
            let lines: Vec<&str> = text.lines().collect();
            let burned = |column: usize| {
                lines
                    .iter()
                    .filter(|l| l.as_bytes().get(column) == Some(&b'#'))
                    .count()
            };
            assert_eq!(burned(0), lines.len(), "{name} {angle}: black burns");
            assert_eq!(burned(31), 0, "{name} {angle}: white burns nothing");
            assert!(burned(4) >= burned(26), "{name} {angle}: darker burns more");
        }
    }
}

#[test]
fn the_scan_angle_does_not_move_a_screen() {
    let image = ImageSettings {
        dither: Dither::Halftone,
        ..ImageSettings::DEFAULT
    };
    let mut turned = options(image);
    turned.scan_angle = curvyo_laser_core::settings::ScanAngle::from_degrees(90.0).unwrap();
    let burned = |o: &OperationOptions| {
        raster_dot_map(&ramp(), o, tenth())
            .bytes()
            .filter(|&b| b == b'#')
            .count()
    };
    assert_eq!(burned(&options(image)), burned(&turned));
}

fn stored(steps: &[ManualStep]) -> Vec<StoredStep> {
    steps
        .iter()
        .map(|&s| StoredStep {
            enabled: true,
            step: Some(s),
        })
        .collect()
}

fn pixels(leaf: &LeafGeometry) -> (&WorkingImage, &[curvyo_document_core::Point; 4]) {
    match leaf {
        LeafGeometry::Image {
            pixels: Some(p),
            frame,
            ..
        } => (p, frame),
        _ => unreachable!(),
    }
}

#[test]
fn an_empty_stack_is_the_0123_dot_map_and_a_stack_changes_it() {
    let leaf = ramp();
    let (image, frame) = pixels(&leaf);
    let o = options(ImageSettings {
        dither: Dither::Jarvis,
        ..ImageSettings::DEFAULT
    });
    let (_, empty) = adjust_picture(image, frame, &AdjustRegisters::default(), 100);
    assert_eq!(
        raster_dot_map_adjusted(&leaf, &o, tenth(), &empty),
        raster_dot_map(&leaf, &o, tenth()),
        "criterion 1: byte for byte"
    );
    let registers = AdjustRegisters {
        steps: stored(&[
            ManualStep::Levels {
                black_pct: 20,
                white_pct: 80,
                gamma_centi: 120,
            },
            StepKind::Sharpen.default_step(100),
        ]),
        ..AdjustRegisters::default()
    };
    let (_, adjusted) = adjust_picture(image, frame, &registers, 100);
    let text = raster_dot_map_adjusted(&leaf, &o, tenth(), &adjusted);
    check("raster/ramp_adjusted_stack.txt", text.as_bytes());
    assert_ne!(text, raster_dot_map(&leaf, &o, tenth()));
}

/// The image-core golden picture `name`, 6.4 mm square.
fn golden_picture(name: &str) -> LeafGeometry {
    let bytes = PICTURES.iter().find(|(n, _)| *n == name).unwrap().1;
    let image = decode_working(bytes).unwrap();
    picture(0.0, 0.0, 6.4, 6.4, image)
}

fn steps_json(steps: &[ManualStep]) -> Value {
    Value::Array(
        steps
            .iter()
            .map(|s| {
                let fields: serde_json::Map<String, Value> = s
                    .fields()
                    .into_iter()
                    .map(|(k, v)| (k.to_owned(), json!(v)))
                    .collect();
                json!({"kind": s.kind().key(), "enabled": true, "fields": fields})
            })
            .collect(),
    )
}

#[test]
fn every_preset_on_its_picture_at_three_strengths() {
    for (preset, input) in [
        (Preset::Photo, "photo"),
        (Preset::Graphics, "graphic"),
        (Preset::BlackWhite, "graphic"),
        (Preset::LineArt, "pen"),
        (Preset::DotRaster, "photo"),
        (Preset::Newspaper, "photo"),
    ] {
        let leaf = golden_picture(input);
        let (image, frame) = pixels(&leaf);
        let mut settings = ImageSettings::DEFAULT;
        for edit in preset.dither_edits(100) {
            settings = settings.with(edit);
        }
        let o = options(settings);
        for strength in [0_u8, 50, 100] {
            let registers = AdjustRegisters {
                preset,
                strength,
                steps: Vec::new(),
                ..AdjustRegisters::default()
            };
            let (steps, adjusted) = adjust_picture(image, frame, &registers, 100);
            let name = format!("preset_{}_s{strength}", preset.key());
            let mut text = serde_json::to_string_pretty(&steps_json(&steps)).unwrap();
            text.push('\n');
            check(&format!("adjust/{name}.json"), text.as_bytes());
            let map = raster_dot_map_adjusted(&leaf, &o, tenth(), &adjusted);
            check(&format!("adjust/{name}.txt"), map.as_bytes());
            // Make editable (criterion 29): the same steps as a manual stack burn the same dots.
            let editable = AdjustRegisters {
                preset: Preset::Expert,
                strength,
                steps: stored(&steps),
                ..AdjustRegisters::default()
            };
            let (_, again) = adjust_picture(image, frame, &editable, 100);
            assert_eq!(
                raster_dot_map_adjusted(&leaf, &o, tenth(), &again),
                map,
                "{name}"
            );
        }
    }
}
