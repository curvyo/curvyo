//! The job of a picture with AI steps (`specs/0135` slice 3, criteria 3, 23, 30, 47, 48, 66; ADR
//! 0077 §3, §4): the job reads only the stored results (no model), a masked-out area burns
//! nothing, a step without its stored result skips the picture with the unreadable-image line,
//! and an out-of-date result still burns with its own check line.

#![allow(clippy::unwrap_used, clippy::missing_panics_doc)]

mod job_fixtures;

use std::sync::Arc;

use curvyo_document_core::{AssetHash, StepResult};
use curvyo_image_core::{StoredPixels, WorkingImage, encode_png};
use curvyo_laser_core::adjust::{
    AdjustRegisters, ManualStep, Preset, ResultRef, StepKind, StoredStep,
};
use curvyo_laser_core::job::{
    Built, Cause, GateFacts, JobInput, LeafGeometry, Names, PlanCache, Timeline, build_job,
    check_text, gate,
};
use curvyo_laser_core::operations::OperationKind::Image;
use job_fixtures::{Page, grey_image, own_engrave, picture, plain};

fn photo() -> WorkingImage {
    grey_image(40, 30, |x, y| {
        u8::try_from((x * 5 + y * 3) % 160 + 20).unwrap()
    })
}

/// A stored mask: the left half is background (0), the right half the subject (255).
fn half_mask() -> WorkingImage {
    grey_image(20, 20, |x, _| if x < 10 { 0 } else { 255 })
}

fn result(pixels: &WorkingImage, model: &str) -> (AssetHash, ResultRef) {
    let png = encode_png(pixels).unwrap();
    let asset = AssetHash::of(&png);
    let record = StepResult {
        asset: asset.clone(),
        model: model.to_owned(),
        model_version: "1".to_owned(),
        input_hash: AssetHash::of(b"made from"),
        width_px: pixels.width,
        height_px: pixels.height,
    };
    (asset, ResultRef::of(&record).unwrap())
}

fn on(step: ManualStep) -> StoredStep {
    StoredStep {
        enabled: true,
        step: Some(step),
    }
}

fn registers(steps: Vec<ManualStep>) -> AdjustRegisters {
    AdjustRegisters {
        steps: steps.into_iter().map(on).collect(),
        ..AdjustRegisters::default()
    }
}

fn mask(result: Option<ResultRef>) -> ManualStep {
    ManualStep::RemoveBackground {
        softness_pct: 0,
        threshold_pct: 50,
        result,
    }
}

fn page(registers: &AdjustRegisters) -> Page {
    let mut page = Page::new();
    let leaf = match picture(0.0, 0.0, 4.0, 3.0, photo()) {
        LeafGeometry::Image {
            frame,
            asset,
            pixels,
            ..
        } => LeafGeometry::Image {
            frame,
            asset,
            pixels,
            adjust: registers.clone(),
        },
        other => other,
    };
    page.operation(
        Image,
        "Image 1",
        own_engrave(Image, 50.0, 6000, 0.1),
        plain(),
        vec![leaf],
    );
    page
}

fn build(input: &JobInput, held: &[(AssetHash, WorkingImage)]) -> Built {
    let mut cache = PlanCache::default();
    for (asset, pixels) in held {
        cache
            .pictures()
            .hold_result(asset, Arc::new(pixels.clone()));
    }
    build_job(input, &[], &mut cache)
}

fn lines(input: &JobInput, built: &Built) -> Vec<String> {
    gate(
        input,
        built,
        &Timeline::of(&built.job),
        &GateFacts::default(),
    )
    .issues
    .iter()
    .map(|i| check_text(i, &Names::of([])).whole())
    .collect()
}

fn dots(built: &Built) -> String {
    format!("{:?}", built.job.sweeps)
}

/// The burned dots left of `x_um` and from it on.
fn burned_split(built: &Built, x_um: i64) -> (usize, usize) {
    let (mut left, mut right) = (0, 0);
    for sweep in &built.job.sweeps {
        for i in 0..sweep.levels.len() {
            if sweep.levels.get(i) == 0 {
                continue;
            }
            let (a, b) = (sweep.point(i).x, sweep.point(i + 1).x);
            if a.min(b) < x_um {
                left += 1;
            } else {
                right += 1;
            }
        }
    }
    (left, right)
}

/// Criteria 47, 48: the mask burns nothing where it is background; the job reads only the
/// stored PNG (no model is in this crate's tree) and is the same on every build.
#[test]
fn c47_c48_a_stored_mask_burns_only_the_subject_and_needs_no_model() {
    let (asset, stored) = result(&half_mask(), "u2netp");
    let with_mask = page(&registers(vec![mask(Some(stored))]));
    let plain_page = page(&AdjustRegisters::default());
    let masked = build(&with_mask.input, &[(asset.clone(), half_mask())]);
    let plain_job = build(&plain_page.input, &[]);
    assert_ne!(dots(&masked), dots(&plain_job), "the mask changes the dots");
    let again = build(&with_mask.input, &[(asset, half_mask())]);
    assert_eq!(
        dots(&masked),
        dots(&again),
        "byte-identical from the stored result"
    );
    // The background half (x < 2 mm of the 4 mm frame) burns nothing; the subject burns as before.
    let (left, right) = burned_split(&masked, 2_000);
    let (plain_left, plain_right) = burned_split(&plain_job, 2_000);
    assert_eq!(left, 0, "masked out burns nothing");
    assert!(plain_left > 0 && right > 0 && right <= plain_right + plain_right / 10);
}

/// ADR 0077 §4 (criterion 66): an AI step without a stored result does not burn the picture;
/// the check names it as an image that cannot be read.
#[test]
fn c66_a_step_without_a_result_skips_the_picture() {
    let pending = page(&registers(vec![mask(None)]));
    let built = build(&pending.input, &[]);
    assert!(built.job.sweeps.is_empty(), "no raster dots");
    let text = lines(&pending.input, &built);
    assert!(
        text.iter()
            .any(|l| l.contains("cannot be read and is skipped")),
        "{text:?}"
    );
    let off = AdjustRegisters {
        steps: vec![StoredStep {
            enabled: false,
            step: Some(mask(None)),
        }],
        ..AdjustRegisters::default()
    };
    assert_eq!(
        off.missing_results(),
        0,
        "a step that is off needs no result"
    );
}

/// Criterion 48: an out-of-date result still burns; the check says so, once per picture.
#[test]
fn c48_an_out_of_date_result_still_burns_with_a_line() {
    let (asset, stored) = result(&half_mask(), "u2netp");
    let mut stale = registers(vec![mask(Some(stored))]);
    let fresh = build(&page(&stale).input, &[(asset.clone(), half_mask())]);
    stale.stale_results = 1;
    let input = page(&stale).input;
    let built = build(&input, &[(asset, half_mask())]);
    assert_eq!(
        dots(&built),
        dots(&fresh),
        "the dots do not change by themselves"
    );
    let text = lines(&input, &built);
    assert!(
        text.contains(&"1 stored AI result of An item is out of date.".to_owned()),
        "{text:?}"
    );
    assert!(matches!(
        Cause::StaleResults {
            item: built_item(&input)
        }
        .rank(),
        27
    ));
}

fn built_item(input: &JobInput) -> curvyo_document_core::NodeId {
    input.blocks[0].leaves[0].node
}

/// Criterion 48: the input hash follows the upstream steps; a changed Crop marks the stored
/// result Out of date, the same stack does not.
#[test]
fn c48_a_changed_upstream_step_marks_the_result_out_of_date() {
    let image = photo();
    let crop = ManualStep::Crop {
        pct: [0, 0, 50, 100],
    };
    let first = registers(vec![crop, mask(None)]);
    let input = first
        .ai_input(1, &image, &StoredPixels::default(), ("u2netp", "1"))
        .unwrap();
    assert_eq!(
        (input.image.width, input.image.height),
        (20, 30),
        "after the Crop"
    );
    let png = encode_png(&half_mask()).unwrap();
    let record = StepResult {
        asset: AssetHash::of(&png),
        model: "u2netp".into(),
        model_version: "1".into(),
        input_hash: AssetHash::from_bytes(&input.hash),
        width_px: 20,
        height_px: 20,
    };
    let stored = ResultRef::of(&record).unwrap();
    let current = registers(vec![crop, mask(Some(stored))]);
    assert_eq!(current.count_stale(&image, &StoredPixels::default()), 0);
    let wider = ManualStep::Crop {
        pct: [0, 0, 60, 100],
    };
    let changed = registers(vec![wider, mask(Some(stored))]);
    assert_eq!(changed.count_stale(&image, &StoredPixels::default()), 1);
}

/// Criteria 3, 30: the AI steps run in their bands (Upscale and AI denoise after Crop and
/// Resize, Remove background last) and always run with a preset.
#[test]
fn c3_c30_ai_steps_keep_their_bands_with_a_preset() {
    let up = StepKind::Upscale.default_step(100);
    let ai = StepKind::AiDenoise.default_step(100);
    let rb = StepKind::RemoveBackground.default_step(100);
    let levels = StepKind::Levels.default_step(100);
    let crop = StepKind::Crop.default_step(100);
    let stack = registers(vec![rb, levels, ai, up, crop]);
    assert_eq!(
        stack.effective_steps(100, |_| unreachable!()),
        vec![crop, up, ai, levels, rb]
    );
    let preset = AdjustRegisters {
        preset: Preset::Graphics,
        ..stack
    };
    let steps = preset.effective_steps(100, |_| curvyo_image_core::Analysis::of(&photo()));
    assert_eq!(&steps[..3], &[crop, up, ai]);
    assert_eq!(steps.last(), Some(&rb), "the mask last");
    assert!(!steps.contains(&levels), "the manual tone step is replaced");
}

/// Criterion 47: the fields of the AI steps and their ranges.
#[test]
fn c47_ai_fields_read_with_their_ranges() {
    let field = |pairs: &'static [(&'static str, i64)]| {
        move |key: &str| pairs.iter().find(|(k, _)| *k == key).map(|p| p.1)
    };
    assert!(ManualStep::read("upscale", field(&[("factor", 2)])).is_ok());
    assert!(ManualStep::read("upscale", field(&[("factor", 4)])).is_ok());
    assert!(ManualStep::read("upscale", field(&[("factor", 3)])).is_err());
    assert!(ManualStep::read("ai_denoise", field(&[("strength_pct", 101)])).is_err());
    assert!(
        ManualStep::read(
            "remove_background",
            field(&[("softness_pct", 21), ("threshold_pct", 50)])
        )
        .is_err()
    );
    assert_eq!(
        StepKind::RemoveBackground.default_step(100),
        mask(None)
            .with_result(None)
            .with_result(None)
            .pipe_softness(5)
    );
}

trait Pipe {
    fn pipe_softness(self, softness: u8) -> Self;
}

impl Pipe for ManualStep {
    fn pipe_softness(self, softness: u8) -> Self {
        match self {
            Self::RemoveBackground {
                threshold_pct,
                result,
                ..
            } => Self::RemoveBackground {
                softness_pct: softness,
                threshold_pct,
                result,
            },
            other => other,
        }
    }
}
