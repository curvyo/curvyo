//! The automatic analysis of `specs/0089-trace-quality-and-colour` slice F against the reference
//! set (criteria 31, 32 and 34). Every test here decodes or traces the whole set and is
//! `#[ignore]`d, meant for a release build:
//!
//! ```text
//! cargo test --release -p curvyo-vectorize-core --test analysis -- --ignored --nocapture
//! ```
//!
//! `auto_is_at_least_the_default_per_group` traces every synthetic fixture with the analysis'
//! proposal and with the preset default (Logo / Line art, Brightness at the automatic Threshold)
//! and fails when a group's true-shape area error or lost topology gets worse;
//! `criterion_32_type_accuracy_on_the_test_part` checks the type on the held-out part of the real
//! images; `analysis_table` prints every image's properties and proposal.

// The harness measures thread CPU time, which needs Unix.
#![cfg(unix)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::type_complexity,
    clippy::needless_pass_by_value
)]

mod common;

use std::collections::BTreeMap;

use common::Picture;
use common::quality::real::{customer_images, real_images};
use common::quality::reference::{Group, SCALES};
use common::quality::report::Row;
use common::quality::run::{Case, Truth, measure_with, params_for};
use common::quality::{reference_set, selection_set};
use curvyo_vectorize_core::{
    AnalysisReport, Capture, ImageType, PRESETS, PropertyKind, Selection, TraceParams, analyse,
};

/// The synthetic fixtures at their three resolutions, with their truth.
fn synthetic_cases() -> Vec<Case> {
    let mut out = Vec::new();
    let colour = selection_set().into_iter().map(|s| s.reference);
    for reference in reference_set().into_iter().chain(colour) {
        for index in 0..SCALES.len() {
            let v = reference.variant(index);
            out.push(Case {
                selection: Selection::Brightness,
                key: v.key,
                group: Some(reference.group),
                picture: v.picture,
                mm_per_px: v.mm_per_px,
                truth: v.truth_4x.map(|raster_4x| Truth {
                    raster_4x,
                    paths: v.truth_paths,
                    corners: v.corners,
                    edges: v.edges,
                }),
            });
        }
    }
    out
}

/// The preset default and the analysis' proposal over it.
fn default_and_auto(picture: &Picture) -> (TraceParams, TraceParams, AnalysisReport) {
    let report = analyse(picture.raster()).unwrap();
    let base = params_for(&PRESETS[0], picture);
    let auto = TraceParams {
        selection: report.proposal.selection,
        threshold: report.proposal.threshold,
        ..base
    };
    (base, auto, report)
}

fn get(row: &Row, key: &str) -> f64 {
    row.get(key).copied().unwrap_or(0.0)
}

fn lost(row: &Row) -> f64 {
    get(row, "components_lost") + get(row, "holes_lost") + get(row, "connections_lost")
}

/// The source asks for no hidden systematic worsening of one group: per group of the synthetic
/// set (all three resolutions), the summed true-shape area error and lost topology of the
/// proposal are at most the preset default's; every changed fixture is printed.
#[test]
#[ignore = "traces the reference set: run in release"]
fn auto_is_at_least_the_default_per_group() {
    let mut groups: BTreeMap<&str, [f64; 4]> = BTreeMap::new();
    for case in synthetic_cases() {
        let (base, auto, report) = default_and_auto(&case.picture);
        let specks = PRESETS[0].specks_mm;
        let a = measure_with(&case, &base, specks);
        let b = if auto == base {
            a.clone()
        } else {
            measure_with(&case, &auto, specks)
        };
        let sums = groups.entry(case.group.unwrap().key()).or_default();
        sums[0] += get(&a, "area_error_true_pct");
        sums[1] += get(&b, "area_error_true_pct");
        sums[2] += lost(&a);
        sums[3] += lost(&b);
        if auto != base {
            println!(
                "{:<30} area {:>6.1} -> {:>6.1} %, lost {} -> {}, {:?}",
                case.key,
                get(&a, "area_error_true_pct"),
                get(&b, "area_error_true_pct"),
                lost(&a),
                lost(&b),
                report.reasons
            );
        }
    }
    let mut worse = Vec::new();
    for (group, [area_a, area_b, lost_a, lost_b]) in &groups {
        println!(
            "{group:<14} area sum {area_a:>8.1} -> {area_b:>8.1} %, lost {lost_a} -> {lost_b}"
        );
        if *area_b > area_a + 1e-6 || lost_b > lost_a {
            worse.push(*group);
        }
    }
    assert!(worse.is_empty(), "worse than the default: {worse:?}");
}

/// Criterion 34: a 2000 x 2000 px picture (a photographed sheet: grain, a shadow, strokes) is
/// analysed in at most 0.5 s thread CPU, the same report twice; the real set's slowest is
/// printed.
#[test]
#[ignore = "budget: run in release"]
fn criterion_34_analysis_budget_and_determinism() {
    let side = 2000_u32;
    let mut seed = 0x2545_f491_u32;
    let mut rgba = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        for x in 0..side {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let grain = (seed >> 28) as u8;
            let paper = if x + y > 2600 { 150 } else { 225 };
            let ink = (x % 97 < 3) || (y % 131 < 2);
            let v = if ink { 30 } else { paper } + grain;
            rgba.extend([v, v, v.saturating_sub(4), 255]);
        }
    }
    let picture = Picture {
        width: side,
        height: side,
        rgba,
    };
    let t0 = common::thread_cpu_seconds();
    let first = analyse(picture.raster()).unwrap();
    let cpu = common::thread_cpu_seconds() - t0;
    println!("2000 x 2000 px: {:.0} ms, {:?}", cpu * 1e3, first.reasons);
    assert_eq!(analyse(picture.raster()).unwrap(), first);
    assert!(cpu <= 0.5, "{cpu} s");
    let mut slowest = (0.0, String::new());
    for image in real_images().into_iter().chain(customer_images()) {
        let t0 = common::thread_cpu_seconds();
        analyse(image.picture.raster()).unwrap();
        let ms = (common::thread_cpu_seconds() - t0) * 1e3;
        if ms > slowest.0 {
            slowest = (ms, image.id);
        }
    }
    println!("slowest real image: {} {:.0} ms", slowest.1, slowest.0);
}

/// Every image with its group, the analysis' type, capture, confidence, main properties and
/// proposal (the table of `plan.md`).
#[test]
#[ignore = "prints the analysis of the reference set"]
fn analysis_table() {
    let mut images: Vec<(String, String, Picture)> = synthetic_cases()
        .into_iter()
        .filter(|c| c.key.ends_with("@1x"))
        .map(|c| (c.key, c.group.unwrap().key().to_string(), c.picture))
        .collect();
    for image in real_images().into_iter().chain(customer_images()) {
        images.push((
            image.id,
            image.group.unwrap().key().to_string(),
            image.picture,
        ));
    }
    println!(
        "image                          group         type        capture    conf  noise  bg    \
         mid   pale  stroke txt | method threshold reasons"
    );
    for (key, group, picture) in images {
        let (base, auto, r) = default_and_auto(&picture);
        let v = |k| r.value(k).unwrap_or(f64::NAN);
        println!(
            "{key:<30} {group:<13} {:<11} {:<10} {:.2}  {:>5.2} {:.2} {:.2} {:.3} {:>6.1} {:>3} | {} {}{} {:?}",
            format!("{:?}", r.image_type),
            format!("{:?}", r.capture),
            r.confidence,
            v(PropertyKind::Noise),
            v(PropertyKind::BackgroundUniformity),
            v(PropertyKind::Midtones),
            v(PropertyKind::PaleContent),
            v(PropertyKind::StrokeWidth),
            v(PropertyKind::TextRows),
            auto.selection.index(),
            auto.threshold,
            if auto.threshold == base.threshold {
                ""
            } else {
                " (lower)"
            },
            r.reasons
        );
    }
}

/// What a real image is, for criterion 32: its type (digital pictures, from the sidecar group:
/// graphics and diagram are Graphic, logos Logo, line drawings and AI black and white drawings
/// Line drawing, sketches Sketch), its capture (paper scans and photos), or unclear (a photo).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Label {
    Type(ImageType),
    Capture(Capture),
    Unclear,
}

/// The test part of the split, fixed before the rules were written: whole image families (the
/// fox, the bird head and the ladybug with their scans and photos) plus single images per group.
const TEST_PART: &[&str] = &[
    "clipart-goatee",
    "customer-eagle-mountains",
    "customer-ladybug-colour",
    "logo-ymc",
    "customer-precision-logo",
    "line-richmond-bridge",
    "customer-turtle-shell",
    "customer-elephant",
    "customer-fox-outline",
    "customer-bird-head",
    "customer-ladybug-lines",
    "customer-horse-sketch",
    "sketch-portrait",
    "ai-pocket-theft",
    "scan-handbag",
    "customer-scan-fox",
    "customer-scan-bird-head",
    "photo-still-life",
    "customer-photo-fox-1",
    "customer-photo-fox-2",
    "customer-photo-fox-3",
    "customer-photo-bird-head",
];

fn label_of(group: Group) -> Label {
    use Group as G;
    match group {
        G::Graphics | G::Diagram => Label::Type(ImageType::Graphic),
        G::Logos => Label::Type(ImageType::Logo),
        G::LineDrawings | G::AiDrawings => Label::Type(ImageType::LineDrawing),
        G::Sketches => Label::Type(ImageType::Sketch),
        G::PaperScan => Label::Capture(Capture::Scan),
        G::PaperPhoto => Label::Capture(Capture::PaperPhoto),
        G::Photo => Label::Unclear,
    }
}

fn correct(label: Label, r: &AnalysisReport) -> bool {
    match label {
        Label::Type(t) => !r.low_confidence() && r.image_type == t,
        Label::Capture(c) => r.capture == c,
        Label::Unclear => r.low_confidence(),
    }
}

/// Criterion 32: on the test part of the real images the type (the capture for paper scans and
/// photos, low confidence for a photo) is right on at least 80 %; the tuning part is reported.
#[test]
#[ignore = "decodes the real images: run in release"]
fn criterion_32_type_accuracy_on_the_test_part() {
    let (mut test, mut test_right, mut tune, mut tune_right) = (0, 0, 0, 0);
    let mut misses = Vec::new();
    for image in real_images().into_iter().chain(customer_images()) {
        let label = label_of(image.group.unwrap());
        let r = analyse(image.picture.raster()).unwrap();
        let ok = correct(label, &r);
        let in_test = TEST_PART.contains(&image.id.as_str());
        if in_test {
            test += 1;
            test_right += usize::from(ok);
        } else {
            tune += 1;
            tune_right += usize::from(ok);
        }
        if !ok {
            misses.push(format!(
                "{} {}: want {label:?}, got {:?} {:?} {:.2}",
                if in_test { "test" } else { "tune" },
                image.id,
                r.image_type,
                r.capture,
                r.confidence
            ));
        }
    }
    println!("tuning {tune_right}/{tune}, test {test_right}/{test}");
    for m in &misses {
        println!("{m}");
    }
    assert_eq!(test, TEST_PART.len());
    assert!(test_right * 5 >= test * 4, "{test_right}/{test}");
}
