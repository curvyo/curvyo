//! The shipped weights' licence record (ADR 0027 §5, `specs/0091-trace-text-recognition`
//! criterion 15): every file of `frontend/public/models/paddleocr-latin/` but the record and the
//! licence text has an entry with an allowed licence, its source and the SHA-256 this crate
//! expects; the record names the training-data statement.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use curvyo_ocr_core::{ALPHABET_SHA256, DETECTOR_SHA256, RECOGNISER_SHA256};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
struct Record {
    name: String,
    licence: String,
    licence_file: String,
    source_url: String,
    upstream_url: String,
    training_data: String,
    files: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    file: String,
    role: String,
    source_url: String,
    sha256: String,
}

/// The licences ADR 0006 §2 allows for shipped data.
const ALLOWED: [&str; 3] = ["Apache-2.0", "MIT", "OFL-1.1"];

fn folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/models/paddleocr-latin")
}

fn sha256(bytes: &[u8]) -> String {
    use std::fmt::Write;
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

#[test]
fn every_weight_file_has_its_record_and_hash() {
    let record: Record =
        toml::from_str(&std::fs::read_to_string(folder().join("model.toml")).unwrap()).unwrap();
    assert_eq!(record.name, "PaddleOCR Latin (mobile)");
    assert!(
        ALLOWED.contains(&record.licence.as_str()),
        "{}",
        record.licence
    );
    assert!(record.source_url.starts_with("https://"));
    assert!(record.upstream_url.starts_with("https://"));
    assert!(record.training_data.contains("Frage 112 A"));
    let licence = std::fs::read_to_string(folder().join(&record.licence_file)).unwrap();
    assert!(licence.contains("Apache License") && licence.contains("Version 2.0"));
    let mut files: Vec<String> = std::fs::read_dir(folder())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name != "model.toml" && *name != record.licence_file)
        .collect();
    files.sort();
    let mut recorded: Vec<String> = record.files.iter().map(|e| e.file.clone()).collect();
    recorded.sort();
    assert_eq!(
        files, recorded,
        "every file needs its record and every record its file"
    );
    for entry in &record.files {
        let bytes = std::fs::read(folder().join(&entry.file)).unwrap();
        assert_eq!(
            sha256(&bytes),
            entry.sha256,
            "{} differs from its record",
            entry.file
        );
        assert!(entry.source_url.starts_with("https://"));
        let expected = match entry.role.as_str() {
            "detector" => DETECTOR_SHA256,
            "recogniser" => RECOGNISER_SHA256,
            "alphabet" => ALPHABET_SHA256,
            other => panic!("unknown role {other}"),
        };
        assert_eq!(
            entry.sha256, expected,
            "{} is not what the crate expects",
            entry.file
        );
    }
}
