//! Tester acceptance cases for `specs/0135-image-optimisation` slice 2 in `curvyo-laser-core`,
//! written from the specification (criteria 3, 5, 7, 14, 30, 39) and ADR 0073 §2, §3, §6: the
//! stored fields and ranges of Crop, Resize, Channel mix and Curves, the zone order of a stored
//! list, the geometry band kept under a preset, and the pixel pitch after Crop and Resize that
//! the later mm fields use.
#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::cast_possible_wrap,
    clippy::assert_is_empty,
    clippy::too_many_lines,
    missing_docs
)]

use std::collections::HashMap;

use curvyo_document_core::Point;
use curvyo_image_core::{Adjusted, Analysis, WorkingImage, run_steps};
use curvyo_laser_core::adjust::{
    AdjustRegisters, Band, CHANNEL_PRESETS, LUMINANCE_MIX, ManualStep, StepDamage, StepKind,
    StoredStep, adjust_picture,
};

fn photo(w: u32, h: u32) -> WorkingImage {
    let mut seed = 99u32;
    let mut pixels = Vec::new();
    for y in 0..h {
        for x in 0..w {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let n = (seed >> 24) as i32 % 31 - 15;
            let g =
                ((x * 255 / w.max(1)) as i32 + n + if (x / 4 + y / 4) % 2 == 0 { 30 } else { -30 })
                    .clamp(0, 255) as u8;
            pixels.extend_from_slice(&[g, g.saturating_sub(20), g / 2, 255]);
        }
    }
    WorkingImage {
        width: w,
        height: h,
        pixels,
        original_size: (w, h),
        truncated: false,
        animated: false,
    }
}

/// The frame of a `w_mm` x `h_mm` picture at the origin.
fn frame(w_mm: f64, h_mm: f64) -> [Point; 4] {
    [
        Point::new(0.0, 0.0),
        Point::new(w_mm, 0.0),
        Point::new(w_mm, h_mm),
        Point::new(0.0, h_mm),
    ]
}

fn on(step: ManualStep) -> StoredStep {
    StoredStep {
        enabled: true,
        step: Some(step),
    }
}

fn regs(steps: Vec<ManualStep>) -> AdjustRegisters {
    AdjustRegisters {
        steps: steps.into_iter().map(on).collect(),
        ..AdjustRegisters::default()
    }
}

fn read(kind: &str, fields: &[(&str, i64)]) -> Result<ManualStep, StepDamage> {
    let m: HashMap<String, i64> = fields.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect();
    ManualStep::read(kind, |k| m.get(k).copied())
}

const SHARPEN: ManualStep = ManualStep::Sharpen {
    radius_um: 400,
    amount_pct: 150,
    threshold_pct: 0,
};

// ---- criterion 3: pipeline order --------------------------------------------------------------

#[test]
fn c3_the_kinds_and_their_bands_are_in_the_spec_pipeline_order() {
    let keys: Vec<&str> = [
        "crop",
        "resize",
        "channel_mix",
        "denoise",
        "levels",
        "curves",
        "local_contrast",
        "contours",
        "sharpen",
    ]
    .to_vec();
    for k in &keys {
        assert!(StepKind::from_key(k).is_some(), "{k} known");
    }
    assert_eq!(StepKind::from_key("crop").unwrap().band(), Band::Geometry);
    assert_eq!(StepKind::from_key("resize").unwrap().band(), Band::Geometry);
    for k in &keys[2..] {
        assert_eq!(StepKind::from_key(k).unwrap().band(), Band::Tone, "{k}");
    }
    assert_eq!(StepKind::Crop.name(), "Crop");
    assert_eq!(StepKind::ChannelMix.name(), "Channel mix");
}

#[test]
fn c3_a_stored_list_out_of_zone_order_runs_in_zone_order_stable_within_the_tone_zone() {
    let img = photo(40, 30);
    let crop = ManualStep::Crop {
        pct: [10, 10, 80, 80],
    };
    let resize = ManualStep::Resize { width_px: 20 };
    let curves = ManualStep::curve(&[(0, 10), (50, 60), (100, 90)]).unwrap();
    let mix = ManualStep::ChannelMix {
        weights: [100, 0, 0],
    };
    // Stored out of zone order: Sharpen, Curves, Resize, Channel mix, Crop.
    let r = regs(vec![SHARPEN, curves, resize, mix, crop]);
    let (ran, out) = adjust_picture(&img, &frame(4.0, 3.0), &r, 100);
    let kinds: Vec<StepKind> = ran.iter().map(ManualStep::kind).collect();
    assert_eq!(
        kinds,
        [
            StepKind::Crop,
            StepKind::Resize,
            StepKind::Sharpen,
            StepKind::Curves,
            StepKind::ChannelMix
        ],
        "geometry first in fixed order, tone in stored order"
    );
    let (_, again) = adjust_picture(&img, &frame(4.0, 3.0), &regs(ran.clone()), 100);
    assert_eq!(out, again, "the run is the steps it reports");
}

// ---- criterion 5: fields and ranges -----------------------------------------------------------

#[test]
fn c5_defaults_of_the_new_steps() {
    assert_eq!(
        StepKind::Crop.default_step(100),
        ManualStep::Crop {
            pct: [0, 0, 100, 100]
        },
        "the whole picture"
    );
    assert_eq!(
        StepKind::ChannelMix.default_step(100),
        ManualStep::ChannelMix {
            weights: [21, 72, 7]
        },
        "the luminance shown as 21, 72, 7"
    );
    assert_eq!(LUMINANCE_MIX, [21, 72, 7]);
    let presets: Vec<[i16; 3]> = CHANNEL_PRESETS.iter().map(|p| p.1).collect::<Vec<_>>();
    assert_eq!(
        presets,
        [[21, 72, 7], [100, 0, 0], [0, 100, 0], [0, 0, 100]],
        "Luminance, Red filter, Green filter, Blue filter"
    );
    let names: Vec<&str> = CHANNEL_PRESETS.iter().map(|p| p.0).collect();
    assert_eq!(names.len(), 4);
    assert_eq!(
        StepKind::Curves.default_step(100),
        ManualStep::curve(&[(0, 0), (100, 100)]).unwrap(),
        "a straight curve"
    );
}

#[test]
fn c5_crop_fields_are_whole_percent_with_width_and_height_at_least_one() {
    let ok = |l, t, w, h| {
        read(
            "crop",
            &[
                ("left_pct", l),
                ("top_pct", t),
                ("width_pct", w),
                ("height_pct", h),
            ],
        )
    };
    assert_eq!(
        ok(20, 10, 60, 80).unwrap(),
        ManualStep::Crop {
            pct: [20, 10, 60, 80]
        }
    );
    assert!(ok(0, 0, 100, 100).is_ok());
    assert!(ok(99, 99, 1, 1).is_ok());
    assert!(ok(0, 0, 0, 50).is_err(), "width at least 1 %");
    assert!(ok(0, 0, 50, 0).is_err(), "height at least 1 %");
    assert!(ok(-1, 0, 50, 50).is_err());
    assert!(ok(0, 0, 101, 50).is_err());
    assert!(ok(60, 0, 50, 50).is_err(), "past the right edge");
    assert!(ok(0, 60, 50, 50).is_err(), "past the bottom edge");
    assert!(matches!(
        read(
            "crop",
            &[("left_pct", 0), ("top_pct", 0), ("width_pct", 10)]
        ),
        Err(StepDamage::Missing(_))
    ));
}

#[test]
fn c5_c14_resize_width_is_at_least_16_pixels() {
    assert!(read("resize", &[("width_px", 16)]).is_ok());
    assert!(read("resize", &[("width_px", 15)]).is_err());
    assert!(read("resize", &[("width_px", 0)]).is_err());
    assert!(read("resize", &[("width_px", -5)]).is_err());
}

#[test]
fn c5_channel_mix_weights_are_minus_200_to_200() {
    let mix = |r, g, b| {
        read(
            "channel_mix",
            &[("red_pct", r), ("green_pct", g), ("blue_pct", b)],
        )
    };
    assert_eq!(
        mix(-200, 200, 0).unwrap(),
        ManualStep::ChannelMix {
            weights: [-200, 200, 0]
        }
    );
    assert!(mix(150, 150, 150).is_ok(), "the sum may differ from 100 %");
    assert!(mix(201, 0, 0).is_err());
    assert!(mix(0, -201, 0).is_err());
}

#[test]
fn c5_curves_hold_two_to_eight_points_with_the_ends_at_0_and_100() {
    assert!(ManualStep::curve(&[(0, 0), (100, 100)]).is_some());
    let eight: Vec<(u8, u8)> = (0..8)
        .map(|i| ((i * 100 / 7) as u8, (i * 10) as u8))
        .collect();
    assert_eq!(eight.last().unwrap().0, 100);
    assert!(ManualStep::curve(&eight).is_some(), "8 points");
    let nine: Vec<(u8, u8)> = (0..9).map(|i| ((i * 100 / 8) as u8, 50)).collect();
    assert!(ManualStep::curve(&nine).is_none(), "at most 8");
    assert!(ManualStep::curve(&[(0, 0)]).is_none(), "one point");
    assert!(
        ManualStep::curve(&[(5, 0), (100, 100)]).is_none(),
        "first end at 0"
    );
    assert!(
        ManualStep::curve(&[(0, 0), (95, 100)]).is_none(),
        "last end at 100"
    );
    assert!(
        ManualStep::curve(&[(0, 0), (50, 10), (50, 20), (100, 100)]).is_none(),
        "Input strictly increasing"
    );
    assert!(ManualStep::curve(&[(0, 0), (60, 30), (40, 20), (100, 100)]).is_none());
    assert!(
        ManualStep::curve(&[(0, 0), (100, 101)]).is_none(),
        "Output at most 100"
    );
    // Stored as in_k/out_k pairs.
    let c = ManualStep::curve(&[(0, 5), (40, 60), (100, 95)]).unwrap();
    let f = c.fields();
    let back = read(
        "curves",
        &f.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(back, c, "round trip of the stored fields {f:?}");
    for (k, _) in &f {
        assert!(
            k.starts_with("in_") || k.starts_with("out_"),
            "curve keys are in_k / out_k: {k}"
        );
    }
}

// ---- criterion 30, 39: the geometry band under a preset ---------------------------------------

#[test]
fn c30_crop_and_resize_run_under_a_preset_and_the_manual_tone_steps_do_not() {
    use curvyo_laser_core::adjust::Preset;
    let img = photo(64, 48);
    let crop = ManualStep::Crop {
        pct: [25, 0, 50, 100],
    };
    let resize = ManualStep::Resize { width_px: 16 };
    let mix = ManualStep::ChannelMix {
        weights: [0, 0, 100],
    };
    let r = AdjustRegisters {
        preset: Preset::Photo,
        strength: 50,
        steps: vec![on(crop), on(mix), on(resize)],
        ..AdjustRegisters::default()
    };
    // The frame is the visible (cropped) part on the page (ADR 0072 §3).
    let (ran, out) = adjust_picture(&img, &frame(3.2, 4.8), &r, 100);
    assert_eq!(ran[0], crop);
    assert_eq!(ran[1], resize);
    assert!(
        !ran.contains(&mix),
        "manual tone step replaced by the preset"
    );
    assert!(ran.len() > 2, "the preset's tone steps run");
    match &out {
        Adjusted::Grey(p) => assert_eq!((p.width, p.height), (16, 24)),
        Adjusted::Rgba(i) => assert_eq!((i.width, i.height), (16, 24)),
        Adjusted::Unchanged => panic!("something ran"),
    }
    // The analysis the preset read is of the picture after Crop and Resize (criterion 39).
    let geo = run_steps(&img, &[crop.pixel_step(0.1), resize.pixel_step(0.1)]);
    let a = match &geo {
        Adjusted::Rgba(i) => Analysis::of(i),
        Adjusted::Grey(p) => Analysis::of_plane(p),
        Adjusted::Unchanged => Analysis::of(&img),
    };
    let derived = Preset::Photo.derive(50, &a, 100);
    assert_eq!(
        &ran[2..],
        derived.as_slice(),
        "derived from the cropped, resized analysis"
    );
}

// ---- criterion 5, 14: the pitch after Crop and Resize -----------------------------------------

#[test]
fn c5_later_mm_fields_use_the_pixel_size_after_resize() {
    // 64 px over 6.4 mm: 0.1 mm per pixel; after Resize to 32 px, 0.2 mm per pixel.
    let img = photo(64, 64);
    let f = frame(6.4, 6.4);
    let resize = ManualStep::Resize { width_px: 32 };
    let (_, got) = adjust_picture(&img, &f, &regs(vec![resize, SHARPEN]), 100);
    let at = |pitch| run_steps(&img, &[resize.pixel_step(0.1), SHARPEN.pixel_step(pitch)]);
    assert_eq!(
        got,
        at(0.2),
        "Sharpen's 0.4 mm are 2 pixels after the Resize"
    );
    assert_ne!(got, at(0.1), "not 4 pixels of the original pitch");
}

#[test]
fn c5_a_crop_keeps_the_pixel_size_for_later_mm_fields() {
    let img = photo(64, 64);
    // The visible frame of a 6.4 mm picture cropped to its left half (ADR 0072 §3).
    let f = frame(3.2, 6.4);
    let crop = ManualStep::Crop {
        pct: [0, 0, 50, 100],
    };
    let (_, got) = adjust_picture(&img, &f, &regs(vec![crop, SHARPEN]), 100);
    let at = |pitch| run_steps(&img, &[crop.pixel_step(0.1), SHARPEN.pixel_step(pitch)]);
    assert_eq!(
        got,
        at(0.1),
        "a pixel of the cropped picture is still 0.1 mm"
    );
}

#[test]
fn c2_a_switched_off_crop_is_as_if_absent() {
    let img = photo(32, 32);
    let r = AdjustRegisters {
        steps: vec![StoredStep {
            enabled: false,
            step: Some(ManualStep::Crop {
                pct: [10, 10, 10, 10],
            }),
        }],
        ..AdjustRegisters::default()
    };
    let (ran, out) = adjust_picture(&img, &frame(3.2, 3.2), &r, 100);
    assert!(ran.is_empty());
    assert_eq!(out, Adjusted::Unchanged);
    assert!(r.is_adjusted(), "Restore original stays enabled");
}
