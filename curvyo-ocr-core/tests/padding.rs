//! The recogniser's padding and batching must not change what is read (`specs/0091` round 2,
//! lead 2026-11-02): the strings of the tester poster, the reference poster and the customer's
//! screenshot are those of the read with every line padded to 1024 px, kept in
//! `tests/fixtures/reads_1024.txt` (`CURVYO_BLESS=1` rewrote it with that read). A mean confidence
//! that crosses a filter threshold (0.6 accepted, 0.8 the garbage filter) is reported.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use std::path::{Path, PathBuf};

use curvyo_ocr_core::{OcrModel, read_lines};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn model() -> OcrModel {
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/models/paddleocr-latin");
    let read = |name: &str| std::fs::read(dir.join(name)).unwrap();
    let alphabet = String::from_utf8(read("dict.txt")).unwrap();
    OcrModel::load(&read("det.onnx"), &read("rec.onnx"), &alphabet).unwrap()
}

const FILES: [&str; 3] = [
    "tester/poster.png",
    "poster.png",
    "customer/font-manager-screenshot.png",
];

/// One line per read line: file, string and mean confidence, tab separated.
fn reads(model: &OcrModel) -> Vec<(String, String, f32)> {
    let mut out = Vec::new();
    for file in FILES {
        let image =
            curvyo_image_core::decode_working(&std::fs::read(fixtures().join(file)).unwrap())
                .unwrap();
        let lines = read_lines(model, image.width, image.height, &image.pixels, &mut |_| {
            std::ops::ControlFlow::Continue(())
        })
        .unwrap();
        for line in lines {
            let mean = line.confidence.iter().sum::<f32>() / line.confidence.len().max(1) as f32;
            out.push((file.to_owned(), line.text, mean));
        }
    }
    out
}

#[test]
fn padding_keeps_every_read_string() {
    let now = reads(&model());
    let path = fixtures().join("reads_1024.txt");
    if std::env::var_os("CURVYO_BLESS").is_some() {
        let mut text = String::new();
        for (f, t, m) in &now {
            use std::fmt::Write;
            let _ = writeln!(text, "{f}\t{t}\t{m:.4}");
        }
        std::fs::write(&path, text).unwrap();
    }
    let kept: Vec<(String, String, f32)> = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .map(|l| {
            let p: Vec<&str> = l.split('\t').collect();
            (p[0].to_owned(), p[1].to_owned(), p[2].parse().unwrap())
        })
        .collect();
    for ((file, text, mean), (_, was, kept_mean)) in now.iter().zip(&kept) {
        for cut in [0.6_f32, 0.8] {
            if (*mean >= cut) != (*kept_mean >= cut) {
                println!("{file}: {text:?} mean {kept_mean:.3} -> {mean:.3} crosses {cut}");
            }
        }
        assert_eq!(text, was, "{file}");
    }
    assert_eq!(now.len(), kept.len(), "the same lines");
}
