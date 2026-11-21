//! The laser extension reads the format 51 goldens of `curvyo-document-core/tests/fixtures/`
//! (`specs/0135-image-optimisation` slice 1, criteria 8, 9, 23, 30, 59 to 61; ADR 0072 §1, §2, §5,
//! §6): the preset picture, the manual stack, the clone with own adjustments and its follower, the
//! picture in two Image operations with their screen keys, each picture's Dither (ADR 0072
//! amendment A1) and one count per damage reason. The
//! goldens `clone_own_adjustments` (the original's registers unchanged by the clone) and
//! `restore_original` (a restored picture runs nothing, so its dots are the `0123` dots) are
//! asserted on the registers the job reads.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use curvyo_document_core::{Document, NodeId, ObjectSnapshot, unpack};
use curvyo_image_core::{Analysis, WorkingImage};
use curvyo_laser_core::adjust::{AdjustRegisters, DitherSettings, ManualStep, Preset, StepKind};
use curvyo_laser_core::operations::{KindRead, OperationKind};
use curvyo_laser_core::settings::{Dither, read_options};

fn golden(name: &str) -> Document {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name);
    unpack(5, &std::fs::read(path).unwrap()).unwrap()
}

fn by_name(d: &Document, name: &str) -> NodeId {
    d.read_objects()
        .objects
        .iter()
        .map(ObjectSnapshot::id)
        .find(|id| d.node_registers(*id).unwrap().name.as_deref() == Some(name))
        .unwrap()
}

/// The registers the job reads for the picture `id` draws (criterion 61).
fn registers(d: &Document, id: NodeId) -> AdjustRegisters {
    let stored = match d.object(id).unwrap() {
        ObjectSnapshot::Image(image) => image.adjustments,
        ObjectSnapshot::Clone(clone) => match clone.chain.picture.as_deref() {
            Some([ObjectSnapshot::Image(image)]) => image.adjustments.clone(),
            other => panic!("a clone of an image, got {other:?}"),
        },
        other => panic!("an image, got {other:?}"),
    };
    AdjustRegisters::read(&stored)
}

fn clones(d: &Document) -> Vec<NodeId> {
    d.read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Clone(c) => Some(c.id()),
            _ => None,
        })
        .collect()
}

fn ramp() -> WorkingImage {
    WorkingImage {
        width: 32,
        height: 32,
        pixels: (0..1024_u32)
            .flat_map(|i| {
                let g = u8::try_from(i % 32 * 8).unwrap();
                [g, g, g, 255]
            })
            .collect(),
        original_size: (32, 32),
        truncated: false,
        animated: false,
    }
}

#[test]
fn the_golden_reads_every_register_of_every_picture() {
    let d = golden("image_adjust_v51.curvyo");
    let photo = registers(&d, by_name(&d, "Photo"));
    assert_eq!((photo.preset, photo.strength), (Preset::Photo, 65));
    let manual = registers(&d, by_name(&d, "Manual"));
    assert_eq!(manual.preset, Preset::Expert);
    let kinds: Vec<StepKind> = manual
        .steps
        .iter()
        .map(|s| s.step.unwrap().kind())
        .collect();
    assert_eq!(
        kinds,
        [
            StepKind::Denoise,
            StepKind::Levels,
            StepKind::LocalContrast,
            StepKind::Contours,
            StepKind::Sharpen
        ],
        "every slice-1 kind, in pipeline order"
    );
    assert!(!manual.steps[4].enabled, "Sharpen is switched off");
    assert_eq!(manual.broken_steps(), 0);
    let [own, follower] = clones(&d)[..] else {
        panic!("two clones")
    };
    let own = registers(&d, own);
    assert_eq!((own.preset, own.strength), (Preset::DotRaster, 65));
    assert_eq!(
        registers(&d, follower),
        own,
        "the chain reads the nearest own"
    );
    let screen = |mode, size, angle| DitherSettings {
        mode,
        threshold_pct: 50,
        screen_size_um: size,
        screen_angle_deg: angle,
    };
    assert_eq!(photo.dither, screen(Dither::Halftone, 1_600, 30));
    assert_eq!(manual.dither, screen(Dither::Jarvis, 800, 45));
    assert_eq!(own.dither, screen(Dither::LineScreen, 1_200, 15));
}

/// Criterion 60: one picture in two operations at Interval 0.1 and 0.2; the derived steps differ
/// exactly in the mm fields. Superseded by the Dither on the picture (2026-11-20): the picture
/// holds the Halftone dots it burns in both operations (amendment A7); the coarse operation keeps
/// its keys for its gradient leaf only.
#[test]
fn a_picture_in_two_operations_derives_its_steps_at_each_interval() {
    let d = golden("image_adjust_v51.curvyo");
    let ops = d.operations(d.manufacturings()[0].id).unwrap();
    let kind = |op: &curvyo_document_core::OperationSnapshot| {
        KindRead::of(op.kind.as_deref()).known().unwrap()
    };
    assert!(ops.iter().all(|op| kind(op) == OperationKind::Image));
    let coarse = read_options(ops[1].options.as_ref(), kind(&ops[1])).unwrap();
    assert_eq!(coarse.image.dither, Dither::Halftone);
    assert_eq!(
        (
            coarse.image.screen_size_hundredths,
            coarse.image.screen_angle_deg
        ),
        (160, 30)
    );
    let photo = registers(&d, by_name(&d, "Photo"));
    assert_eq!(
        coarse.image.with_dither(photo.dither),
        coarse.image,
        "the picture holds the dither the operation held"
    );
    let analysis = Analysis::of(&ramp());
    let fine = photo.effective_steps(100, |_| analysis.clone());
    let wide = photo.effective_steps(200, |_| analysis.clone());
    assert_eq!(fine.len(), wide.len());
    for (a, b) in fine.iter().zip(&wide) {
        assert_eq!(a.kind(), b.kind());
        for ((key, x), (_, y)) in a.fields().iter().zip(b.fields()) {
            if key.ends_with("_um") {
                assert!(*x <= y, "{key}: the mm grow with the Interval");
            } else {
                assert_eq!(*x, y, "{key}: not an mm field");
            }
        }
    }
    assert_ne!(fine, wide);
}

/// Criterion 23, ADR 0072 §5: the check counts one step per damage reason; nothing is dropped.
#[test]
fn the_damaged_golden_counts_each_reason() {
    let d = golden("image_adjust_damaged_v51.curvyo");
    let unknown = registers(&d, by_name(&d, "Unknown preset"));
    assert_eq!(
        (unknown.preset, unknown.broken_steps()),
        (Preset::Expert, 1)
    );
    let entries = registers(&d, by_name(&d, "Damaged entries"));
    assert_eq!(entries.steps.len(), 7);
    assert_eq!(entries.broken_steps(), 6);
    assert_eq!(
        entries.effective_steps(100, |_| unreachable!()),
        vec![ManualStep::Levels {
            black_pct: 3,
            white_pct: 97,
            gamma_centi: 110
        }],
        "only the entry that reads runs"
    );
    for name in [
        "Dither not a map",
        "Dither unknown mode",
        "Dither missing field",
        "Dither too large",
        "Dither off the step",
    ] {
        let read = registers(&d, by_name(&d, name));
        assert_eq!(
            (read.dither, read.broken_steps()),
            (DitherSettings::DEFAULT, 1),
            "{name}: Floyd-Steinberg, one step that could not run"
        );
    }
    let typed = registers(&d, by_name(&d, "Wrong types"));
    assert_eq!(typed.broken_steps(), 3);
    assert_eq!(
        typed,
        AdjustRegisters {
            damaged_registers: 3,
            ..AdjustRegisters::default()
        }
    );
}

/// `clone_own_adjustments` and `restore_original` (criteria 9, 54, 59): the clone's own set
/// leaves the original's registers as they were; Restore original runs nothing, so the adjusted
/// image is the working image and the dots are those of `0123`.
#[test]
fn clone_own_adjustments_and_restore_original() {
    let d = golden("image_adjust_v51.curvyo");
    let photo = by_name(&d, "Photo");
    let before = registers(&d, photo);
    let own = clones(&d)[0];
    d.edit_image_adjustments(
        own,
        &curvyo_document_core::AdjustmentsWrite {
            strength: Some(0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(registers(&d, photo), before, "clone_own_adjustments");
    assert!(d.restore_original(photo).unwrap());
    let restored = registers(&d, photo);
    assert!(!restored.is_adjusted());
    let frame = [
        curvyo_document_core::Point::new(0.0, 0.0),
        curvyo_document_core::Point::new(3.2, 0.0),
        curvyo_document_core::Point::new(3.2, 3.2),
        curvyo_document_core::Point::new(0.0, 3.2),
    ];
    let (steps, adjusted) =
        curvyo_laser_core::adjust::adjust_picture(&ramp(), &frame, &restored, 100);
    assert_eq!(steps, []);
    assert_eq!(
        adjusted,
        curvyo_image_core::Adjusted::Unchanged,
        "restore_original"
    );
}
