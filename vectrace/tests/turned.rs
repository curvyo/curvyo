//! Upside-down and steep text (`specs/0091-trace-text-recognition` slice 3, criteria 30, 31 and
//! 33): the committed slice 3 fixtures read with the shipped weights give each line's string
//! (exact or within one letter; the font fit settles the letters, checked in
//! `curvyo-editor-wasm`)
//! along its true reading direction within 2 degrees: the detector's box of an upright script or
//! descender line is off by up to about 2 degrees already (slice 1); the font fit refines it, and
//! the text objects are checked within 1 degree in `curvyo-editor-wasm`'s
//! `trace_text_turned_tests`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use serde::Deserialize;
use vectrace::ocr::{OcrModel, TextLine, read_lines};

#[derive(Deserialize)]
struct Truth {
    images: Vec<Image>,
}

#[derive(Deserialize)]
struct Image {
    file: String,
    lines: Vec<String>,
    rotation_degrees: Vec<f32>,
    upright: Option<String>,
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/turned")
}

fn model() -> OcrModel {
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/models/paddleocr-latin");
    let read = |name: &str| std::fs::read(dir.join(name)).unwrap();
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

/// The characters of `a` and `b` that differ (Levenshtein distance).
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let next = (diagonal + usize::from(ca != *cb))
                .min(row[j] + 1)
                .min(row[j + 1] + 1);
            diagonal = row[j + 1];
            row[j + 1] = next;
        }
    }
    row[b.len()]
}

/// The angle of the line's top edge, degrees.
fn angle(line: &TextLine) -> f32 {
    let q = line.quad;
    (q[1][1] - q[0][1]).atan2(q[1][0] - q[0][0]).to_degrees()
}

/// The turn from `want` to the line's top edge, degrees in -180 to 180.
fn off(line: &TextLine, want: f32) -> f32 {
    (angle(line) - want + 540.0).rem_euclid(360.0) - 180.0
}

/// The line of `lines` nearest `want` and its distance.
fn nearest<'a>(lines: &'a [TextLine], want: &str) -> (&'a TextLine, usize) {
    lines
        .iter()
        .map(|l| (l, distance(&l.text, want)))
        .min_by_key(|(_, d)| *d)
        .unwrap()
}

#[test]
fn turned_lines_read_the_right_way_round() {
    let model = model();
    let truth: Truth =
        toml::from_str(&std::fs::read_to_string(fixtures().join("truth.toml")).unwrap()).unwrap();
    for image in &truth.images {
        let lines = read(&model, &image.file);
        let upright = image.upright.as_deref().map(|file| read(&model, file));
        let found: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        println!("{}: read {found:?}", image.file);
        let mut wrong = 0;
        for (want, rotation) in image.lines.iter().zip(&image.rotation_degrees) {
            let (line, letters) = nearest(&lines, want);
            wrong += usize::from(letters > 0);
            assert!(letters <= 1, "{}: {want:?} read as {found:?}", image.file);
            let rotation = if let Some(upright) = &upright {
                rotation + angle(nearest(upright, want).0)
            } else {
                *rotation
            };
            let off = off(line, rotation);
            println!("{}: {:?} off by {off:.2} degrees", image.file, line.text);
            assert!(off.abs() <= 2.0, "{}: {want:?} off by {off}", image.file);
        }
        println!("{}: {wrong} lines not exact", image.file);
    }
}
