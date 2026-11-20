//! The job of an adjusted picture (`specs/0135` criteria 19, 23, 29, 60; ADR 0073 §6 to §8):
//! the job reads the adjusted picture at each operation's Interval, built in steps, with the
//! check line of steps that cannot run.

#![allow(clippy::unwrap_used, clippy::missing_panics_doc)]

mod job_fixtures;

use curvyo_image_core::WorkingImage;
use curvyo_laser_core::adjust::{AdjustRegisters, Preset, StepKind, StoredStep, adjust_picture};
use curvyo_laser_core::job::{
    ADJUST_DOTS, Built, Cause, GateFacts, JobInput, LeafGeometry, Names, PlanCache, Timeline,
    build_job, check_text, gate, prepare_job,
};
use curvyo_laser_core::operations::OperationKind::Image;
use curvyo_laser_core::settings::{Dither, ImageSettings, OperationOptions};
use job_fixtures::{Page, grey_image, own_engrave, picture, plain};

fn build(input: &JobInput) -> Built {
    build_job(input, &[], &mut PlanCache::default())
}

fn jarvis() -> OperationOptions {
    OperationOptions {
        image: ImageSettings {
            dither: Dither::Jarvis,
            ..ImageSettings::DEFAULT
        },
        ..plain()
    }
}

fn photo() -> WorkingImage {
    grey_image(80, 60, |x, y| {
        u8::try_from((x * 3 + y * 2 + (x * y) % 7) % 200 + 30).unwrap()
    })
}

fn adjusted(leaf: LeafGeometry, registers: &AdjustRegisters) -> LeafGeometry {
    match leaf {
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
    }
}

/// One picture in one Image operation per Interval of `intervals`.
fn page(registers: &AdjustRegisters, intervals: &[f64]) -> Page {
    let mut page = Page::new();
    for (n, &interval) in intervals.iter().enumerate() {
        page.operation(
            Image,
            &format!("Image {}", n + 1),
            own_engrave(Image, 50.0, 6000, interval),
            jarvis(),
            vec![adjusted(picture(0.0, 0.0, 8.0, 6.0, photo()), registers)],
        );
    }
    page
}

fn preset(preset: Preset, strength: u8) -> AdjustRegisters {
    AdjustRegisters {
        preset,
        strength,
        ..AdjustRegisters::default()
    }
}

fn expert(steps: &[curvyo_laser_core::adjust::ManualStep]) -> AdjustRegisters {
    AdjustRegisters {
        steps: steps
            .iter()
            .map(|&step| StoredStep {
                enabled: true,
                step: Some(step),
            })
            .collect(),
        ..AdjustRegisters::default()
    }
}

/// Criteria 19, 29: the job burns the adjusted picture; the preset's derived steps written out
/// as a manual stack (Make editable) burn the same dots byte for byte.
#[test]
fn c29_the_job_of_a_preset_equals_the_job_of_its_resolved_stack() {
    let registers = preset(Preset::Photo, 70);
    let frame = job_fixtures::frame(0.0, 0.0, 8.0, 6.0);
    let (steps, _) = adjust_picture(&photo(), &frame, &registers, 100);
    assert_ne!(steps, Vec::new());
    let with_preset = build(&page(&registers, &[0.1]).input);
    let editable = build(&page(&expert(&steps), &[0.1]).input);
    let plain = build(&page(&AdjustRegisters::default(), &[0.1]).input);
    assert_eq!(with_preset.job, editable.job);
    assert_ne!(
        with_preset.job, plain.job,
        "the adjustment changes the dots"
    );
}

/// Criterion 60: a picture in two operations derives its preset at each operation's Interval.
#[test]
fn c60_two_operations_derive_the_preset_at_their_own_interval() {
    let registers = preset(Preset::Graphics, 50);
    let frame = job_fixtures::frame(0.0, 0.0, 8.0, 6.0);
    let (at_1, _) = adjust_picture(&photo(), &frame, &registers, 100);
    let (at_2, _) = adjust_picture(&photo(), &frame, &registers, 200);
    assert_ne!(at_1, at_2, "the mm fields differ");
    let both = build(&page(&registers, &[0.1, 0.2]).input);
    let first = build(&page(&expert(&at_1), &[0.1, 0.2]).input);
    let second = build(&page(&expert(&at_2), &[0.1, 0.2]).input);
    assert_eq!(both.job.blocks[0], first.job.blocks[0]);
    assert_eq!(both.job.blocks[1], second.job.blocks[1]);
}

/// Criterion 13 with the `0123` ticks: a step with fewer than [`ADJUST_DOTS`] dots does not
/// adjust; a tick adjusts one picture and ends; the stepped job equals the job built at once.
#[test]
fn c13_an_adjusted_picture_is_prepared_in_ticks() {
    let input = page(&preset(Preset::Photo, 50), &[0.1, 0.2]).input;
    let at_once = build(&input);
    let mut cache = PlanCache::default();
    for _ in 0..5 {
        assert!(!prepare_job(&input, &[], &mut cache, ADJUST_DOTS - 1));
    }
    assert!(
        cache.pictures().is_empty(),
        "nothing adjusted in a light step"
    );
    let mut ticks = 0;
    while !prepare_job(&input, &[], &mut cache, ADJUST_DOTS) {
        ticks += 1;
        assert!(ticks < 1_000);
    }
    assert!(ticks >= 2, "one adjustment per tick: {ticks}");
    assert_eq!(cache.pictures().len(), 2, "two Intervals, two pictures");
    assert_eq!(build_job(&input, &[], &mut cache), at_once);
}

/// Criterion 23: the check line names the picture and counts its steps that cannot run.
#[test]
fn c23_steps_that_cannot_run_are_one_warn_line_per_picture() {
    let mut registers = expert(&[StepKind::Levels.default_step(100)]);
    registers.steps.push(StoredStep {
        enabled: true,
        step: None,
    });
    registers.damaged_registers = 1;
    let page = page(&registers, &[0.1, 0.2]);
    let built = build(&page.input);
    let g = gate(
        &page.input,
        &built,
        &Timeline::of(&built.job),
        &GateFacts::default(),
    );
    let lines: Vec<String> = g
        .issues
        .iter()
        .filter(|i| matches!(i.cause, Cause::BrokenAdjustments { .. }))
        .map(|i| check_text(i, &Names::of([])).whole())
        .collect();
    assert_eq!(
        lines,
        [
            "2 steps in the Adjustments of An item could not run.",
            "2 steps in the Adjustments of An item could not run."
        ],
        "one line per picture node (two nodes here)"
    );
    let one = expert(&[]);
    let mut one = one;
    one.steps.push(StoredStep {
        enabled: false,
        step: None,
    });
    let page = self::page(&one, &[0.1]);
    let built = build(&page.input);
    let g = gate(
        &page.input,
        &built,
        &Timeline::of(&built.job),
        &GateFacts::default(),
    );
    let text: Vec<String> = g
        .issues
        .iter()
        .map(|i| check_text(i, &Names::of([])).whole())
        .collect();
    assert!(
        text.contains(&"1 step in the Adjustments of An item could not run.".to_owned()),
        "{text:?}"
    );
}
