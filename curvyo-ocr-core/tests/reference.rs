//! The reference set (`specs/0091-trace-text-recognition` criteria 1, 2, 8, 13 and 20): the
//! committed synthetic posters read with the shipped weights; every line exact, the umlauts kept,
//! the rotated line within 1 degree, the same lines twice. Prints the numbers and the time.

#![allow(clippy::unwrap_used, clippy::expect_used)]

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
