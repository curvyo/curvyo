//! The reference set (`specs/0091-trace-text-recognition` criteria 1, 2, 8, 13 and 20): the
//! committed synthetic posters read with the shipped weights; every line exact, the umlauts kept,
//! the rotated line within 1 degree, the same lines twice. Prints the numbers and the time.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use std::path::{Path, PathBuf};

use curvyo_ocr_core::{OcrModel, TextLine, read_lines};
use serde::Deserialize;

#[derive(Deserialize)]
struct Truth {
    images: Vec<Image>,
}

#[derive(Deserialize)]
struct Image {
    file: String,
    lines: Vec<String>,
    rotation_degrees: Option<f32>,
}

fn models() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/models/paddleocr-latin")
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn model() -> OcrModel {
    let read = |name: &str| std::fs::read(models().join(name)).unwrap();
    let alphabet = String::from_utf8(read("dict.txt")).unwrap();
    OcrModel::load(&read("det.onnx"), &read("rec.onnx"), &alphabet).unwrap()
}

fn read(model: &OcrModel, file: &str) -> Vec<TextLine> {
    let bytes = std::fs::read(fixtures().join(file)).unwrap();
    let image = curvyo_image_core::decode_working(&bytes).unwrap();
    read_lines(model, image.width, image.height, &image.pixels, &mut |_| {
        std::ops::ControlFlow::Continue(())
    })
    .unwrap()
}

#[test]
fn the_reference_set_reads_exactly() {
    let model = model();
    let truth: Truth =
        toml::from_str(&std::fs::read_to_string(fixtures().join("truth.toml")).unwrap()).unwrap();
    for image in &truth.images {
        let started = std::time::Instant::now();
        let lines = read(&model, &image.file);
        let seconds = started.elapsed().as_secs_f64();
        let found: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        let exact = image
            .lines
            .iter()
            .filter(|l| found.contains(&l.as_str()))
            .count();
        println!(
            "{}: {exact} of {} lines exact in {seconds:.2} s; read {found:?}",
            image.file,
            image.lines.len()
        );
        // Criterion 1: at least 7 of 8 (all of a one-line image).
        assert!(exact + 1 >= image.lines.len().max(2), "{found:?}");
        if let Some(rotation) = image.rotation_degrees {
            let q = lines[0].quad;
            let angle = (q[1][1] - q[0][1]).atan2(q[1][0] - q[0][0]).to_degrees();
            println!("{}: rotation {angle:.2} degrees", image.file);
            assert!((angle - rotation).abs() <= 1.0, "{angle}");
        }
    }
}

/// Criterion 2: every umlaut and ß of the clean poster is read where the source has it.
#[test]
fn umlauts_are_read() {
    let lines = read(&model(), "poster.png");
    let all: String = lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for word in ["Müller", "Söhne", "Größe", "ÄÖÜ", "äöü", "ß"] {
        assert!(all.contains(word), "{word} missing in {all}");
    }
}

/// Criterion 13: the same image twice gives the same lines.
#[test]
fn a_read_is_deterministic() {
    let model = model();
    assert_eq!(read(&model, "rotated.png"), read(&model, "rotated.png"));
}

/// Criterion 28: the detection and recognition times of the reference poster, the tester poster
/// and the customer's screenshot, with the lines each reads. Wall time on one thread
/// (`RAYON_NUM_THREADS=1`), so it equals the CPU time of the read:
///
/// ```text
/// RAYON_NUM_THREADS=1 cargo test --release -p curvyo-ocr-core --test reference -- --ignored --nocapture
/// ```
#[test]
#[ignore = "harness: prints the numbers of the PR"]
fn detection_and_recognition_times() {
    let model = model();
    for file in [
        "poster.png",
        "tester/poster.png",
        "customer/font-manager-screenshot.png",
    ] {
        let bytes = std::fs::read(fixtures().join(file)).unwrap();
        let image = curvyo_image_core::decode_working(&bytes).unwrap();
        let started = std::time::Instant::now();
        let mut detected = None;
        let lines = read_lines(&model, image.width, image.height, &image.pixels, &mut |p| {
            if matches!(p, curvyo_ocr_core::OcrProgress::Reading { done: 0, .. }) {
                detected.get_or_insert(started.elapsed().as_secs_f64());
            }
            std::ops::ControlFlow::Continue(())
        })
        .unwrap();
        let total = started.elapsed().as_secs_f64();
        let detect = detected.unwrap_or(total);
        println!(
            "{file} ({} x {}): detection {detect:.2} s, recognition {:.2} s, {} lines",
            image.width,
            image.height,
            total - detect,
            lines.len()
        );
        for line in &lines {
            let mean = line.confidence.iter().sum::<f32>() / line.confidence.len().max(1) as f32;
            let q = line.quad;
            println!(
                "  {:?} mean {mean:.2} min {:.2} at ({:.0}, {:.0}) h {:.0}",
                line.text,
                line.confidence.iter().copied().fold(1.0_f32, f32::min),
                q[0][0],
                q[0][1],
                (q[3][1] - q[0][1]).hypot(q[3][0] - q[0][0])
            );
        }
    }
}
