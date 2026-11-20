//! The job of pictures that burn with their own dither (`specs/0135` criteria 22, 60, 67; ADR 0072
//! amendment A4, A7; ADR 0073 note of 2026-11-20 §6, §7): two pictures in one Image operation keep
//! their own dots, a picture in two operations burns one dither, a dither change re-dithers
//! without re-adjusting, and a gradient leaf keeps the operation's dither.

#![allow(clippy::unwrap_used, clippy::missing_panics_doc)]

mod job_fixtures;

use curvyo_document_core::{Color, GradientFill};
use curvyo_image_core::WorkingImage;
use curvyo_laser_core::adjust::{AdjustRegisters, DitherSettings, Preset};
use curvyo_laser_core::job::{
    Built, Cause, GateFacts, JobInput, LeafGeometry, MoveKind, PlanCache, Timeline, build_job, gate,
};
use curvyo_laser_core::operations::OperationKind::Image;
use curvyo_laser_core::settings::{Dither, ImageSettings, OperationOptions};
use job_fixtures::{Page, grey_image, own_engrave, picture, plain};

fn photo() -> WorkingImage {
    grey_image(80, 60, |x, y| {
        u8::try_from((x * 3 + y * 2 + (x * y) % 7) % 200 + 30).unwrap()
    })
}

fn mode(dither: Dither) -> DitherSettings {
    DitherSettings {
        mode: dither,
        ..DitherSettings::DEFAULT
    }
}

fn with(dither: Dither) -> OperationOptions {
    OperationOptions {
        image: ImageSettings {
            dither,
            ..ImageSettings::DEFAULT
        },
        ..plain()
    }
}

/// A picture at `x` with `preset` (Strength 50) and its own `dither`.
fn shot(x: f64, preset: Preset, dither: DitherSettings) -> LeafGeometry {
    let LeafGeometry::Image {
        frame,
        asset,
        pixels,
        ..
    } = picture(x, 0.0, 8.0, 6.0, photo())
    else {
        unreachable!()
    };
    LeafGeometry::Image {
        frame,
        asset,
        pixels,
        adjust: AdjustRegisters {
            preset,
            dither,
            ..AdjustRegisters::default()
        },
    }
}

fn gradient(x: f64) -> LeafGeometry {
    let outline = job_fixtures::rect(x, 0.0, 8.0, 6.0);
    let frame = curvyo_document_core::GradientBox::axis_aligned(
        curvyo_document_core::Point::new(x, 0.0),
        curvyo_document_core::Point::new(x + 8.0, 6.0),
    );
    LeafGeometry::Gradient {
        outlines: vec![outline],
        gradient: curvyo_document_core::PlacedGradient::new(
            &GradientFill::first_for(Color::BLACK, 255),
            &frame,
        ),
    }
}

/// One Image operation per entry of `operations`, each with its options and leaves as they are.
fn page(operations: Vec<(OperationOptions, Vec<LeafGeometry>)>) -> JobInput {
    let mut page = Page::new();
    for (n, (options, leaves)) in operations.into_iter().enumerate() {
        page.operation_as_is(
            Image,
            &format!("Image {}", n + 1),
            own_engrave(Image, 50.0, 6000, 0.1),
            options,
            leaves,
        );
    }
    page.input
}

fn build(input: &JobInput) -> Built {
    build_job(input, &[], &mut PlanCache::default())
}

/// The sweeps of block `block`, sorted (the dots each item burns, whatever the run order).
fn sweeps(built: &Built, block: usize) -> Vec<String> {
    let mut out: Vec<String> = built
        .job
        .moves
        .iter()
        .filter(|m| m.block == block)
        .filter_map(|m| match m.kind {
            MoveKind::Sweep { sweep, .. } => Some(format!("{:?}", built.job.sweep(sweep).unwrap())),
            _ => None,
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

fn all_sweeps(built: &Built) -> Vec<String> {
    let mut out: Vec<String> = (0..built.job.blocks.len())
        .flat_map(|b| sweeps(built, b))
        .collect();
    out.sort();
    out
}

/// Criterion 67: two pictures with two presets in one operation give each the dots it gives
/// alone; the operation's own Dither does not reach them.
#[test]
fn c67_two_pictures_in_one_operation_keep_their_own_dots() {
    let a = shot(0.0, Preset::Photo, mode(Dither::Jarvis));
    let b = shot(
        20.0,
        Preset::DotRaster,
        DitherSettings {
            mode: Dither::Halftone,
            screen_size_um: 800,
            ..DitherSettings::DEFAULT
        },
    );
    let together = build(&page(vec![(
        with(Dither::Stucki),
        vec![a.clone(), b.clone()],
    )]));
    let alone = build(&page(vec![
        (with(Dither::Ordered), vec![a]),
        (with(Dither::Grey), vec![b]),
    ]));
    assert_ne!(sweeps(&together, 0), Vec::<String>::new());
    assert_eq!(sweeps(&together, 0), all_sweeps(&alone));
}

/// Amendment A7: a picture in two operations at one Interval burns its one dither in both.
#[test]
fn a7_a_picture_in_two_operations_burns_one_dither() {
    let picture = shot(0.0, Preset::Expert, mode(Dither::Atkinson));
    let built = build(&page(vec![
        (with(Dither::Threshold), vec![picture.clone()]),
        (with(Dither::Grey), vec![picture]),
    ]));
    assert_ne!(sweeps(&built, 0), Vec::<String>::new());
    assert_eq!(sweeps(&built, 0), sweeps(&built, 1));
}

/// ADR 0073 note §7: a dither change re-dithers without a new adjusted image.
#[test]
fn a_dither_change_redithers_without_readjusting() {
    let mut cache = PlanCache::default();
    let jarvis = page(vec![(
        plain(),
        vec![shot(0.0, Preset::Photo, mode(Dither::Jarvis))],
    )]);
    let first = build_job(&jarvis, &[], &mut cache);
    assert_eq!(cache.pictures().len(), 1, "one adjusted image");
    let stucki = page(vec![(
        plain(),
        vec![shot(0.0, Preset::Photo, mode(Dither::Stucki))],
    )]);
    let second = build_job(&stucki, &[], &mut cache);
    assert_ne!(all_sweeps(&first), all_sweeps(&second), "other dots");
    assert_eq!(cache.pictures().len(), 1, "the same adjusted image");
}

/// Amendment A4: a gradient leaf burns with the operation's Dither, a picture never does.
#[test]
fn a4_a_gradient_leaf_keeps_the_operations_dither() {
    let gradient_jarvis = build(&page(vec![(with(Dither::Jarvis), vec![gradient(0.0)])]));
    let gradient_floyd = build(&page(vec![(
        with(Dither::FloydSteinberg),
        vec![gradient(0.0)],
    )]));
    assert_ne!(all_sweeps(&gradient_jarvis), all_sweeps(&gradient_floyd));
    let picture = || vec![shot(0.0, Preset::Expert, DitherSettings::DEFAULT)];
    assert_eq!(
        all_sweeps(&build(&page(vec![(with(Dither::Jarvis), picture())]))),
        all_sweeps(&build(&page(vec![(
            with(Dither::FloydSteinberg),
            picture()
        )]))),
        "the operation's Dither does not reach a picture"
    );
}

/// Frage 284 B: a picture in Grey raises the operation's Grey lines (Grey on M3, Lowest power
/// above Power) and they target the operation.
#[test]
fn a_picture_in_grey_raises_the_operations_grey_checks() {
    let mut input = page(vec![(
        OperationOptions {
            image: ImageSettings {
                lowest_power_tenths: 900,
                ..ImageSettings::DEFAULT
            },
            ..plain()
        },
        vec![shot(0.0, Preset::Expert, mode(Dither::Grey))],
    )]);
    input.machine.values.laser_mode = false;
    let built = build(&input);
    let facts = gate(
        &input,
        &built,
        &Timeline::of(&built.job),
        &GateFacts::default(),
    );
    let op = input.blocks[0].id;
    for cause in [
        Cause::GreyNeedsM4 { op },
        Cause::LowestPowerAbovePower { op },
    ] {
        assert!(
            facts.issues.iter().any(|i| i.cause == cause),
            "{cause:?} in {:?}",
            facts.issues
        );
    }
    let floyd = page(vec![(
        with(Dither::Grey),
        vec![shot(0.0, Preset::Expert, DitherSettings::DEFAULT)],
    )]);
    let mut floyd = floyd;
    floyd.machine.values.laser_mode = false;
    let built = build(&floyd);
    let facts = gate(
        &floyd,
        &built,
        &Timeline::of(&built.job),
        &GateFacts::default(),
    );
    assert!(
        !facts
            .issues
            .iter()
            .any(|i| matches!(i.cause, Cause::GreyNeedsM4 { .. })),
        "the operation's Grey does not reach a picture"
    );
}

/// An unreadable Grey picture burns nothing, so it raises no Grey check and the Settings view
/// and the numbers line agree (`burns_grey`).
#[test]
fn an_unreadable_grey_picture_raises_no_grey_check() {
    let LeafGeometry::Image {
        frame,
        asset,
        adjust,
        ..
    } = shot(0.0, Preset::Expert, mode(Dither::Grey))
    else {
        unreachable!()
    };
    let unreadable = LeafGeometry::Image {
        frame,
        asset,
        pixels: None,
        adjust,
    };
    let mut input = page(vec![(plain(), vec![unreadable])]);
    input.machine.values.laser_mode = false;
    assert!(!curvyo_laser_core::job::burns_grey(
        &ImageSettings::DEFAULT,
        &input.blocks[0].leaves
    ));
    let built = build(&input);
    let facts = gate(
        &input,
        &built,
        &Timeline::of(&built.job),
        &GateFacts::default(),
    );
    assert!(
        !facts
            .issues
            .iter()
            .any(|i| matches!(i.cause, Cause::GreyNeedsM4 { .. })),
        "{:?}",
        facts.issues
    );
}

/// ADR 0069 note §13, UX P31: the row's numbers line names the one dither the items burn with,
/// none when they differ.
#[test]
fn the_numbers_line_names_the_dither_only_when_every_item_shares_it() {
    let line = |leaves: Vec<LeafGeometry>| {
        let input = page(vec![(with(Dither::Stucki), leaves)]);
        let built = build(&input);
        curvyo_laser_core::job::numbers_lines(&built.numbers[0], &input.blocks[0].leaves)[0]
            .dither
            .map(|(dither, _)| dither)
    };
    let jarvis = |x| shot(x, Preset::Expert, mode(Dither::Jarvis));
    assert_eq!(line(vec![jarvis(0.0), jarvis(20.0)]), Some(Dither::Jarvis));
    assert_eq!(
        line(vec![
            jarvis(0.0),
            shot(20.0, Preset::Expert, mode(Dither::Atkinson))
        ]),
        None
    );
    assert_eq!(
        line(vec![jarvis(0.0), gradient(20.0)]),
        None,
        "the gradient burns Stucki"
    );
    assert_eq!(line(vec![gradient(0.0)]), Some(Dither::Stucki));
    assert_eq!(
        line(Vec::new()),
        Some(Dither::Stucki),
        "no item: the operation's"
    );
}
