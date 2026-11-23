//! The pinned model list (ADR 0075 §4, ADR 0076 §1, §2; `specs/0135` criteria 50, 64): the
//! compiled-in `MODELS` agree with `models.toml`, every licence is permissive, every file is under
//! its size target, and every URL is the one fixed scheme on the one host.
//!
//! The release gate: `CURVYO_RELEASE_MODELS=1 cargo test -p curvyo-imageai-core --test
//! model_record` fails while a placeholder hash remains (`docs/runbooks/ai-models.md`); CI runs it
//! as a step marked pending until the customer has uploaded the converted files.

#![allow(clippy::unwrap_used, clippy::missing_panics_doc, missing_docs)]

use curvyo_imageai_core::{MODEL_HOST, MODELS, ModelKind};

/// The licences criterion 50 allows for weights.
const ALLOWED: [&str; 5] = [
    "Apache-2.0",
    "MIT",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "CC0-1.0",
];

fn record() -> Vec<toml::Table> {
    let text = include_str!("../models.toml");
    let table: toml::Table = text.parse().unwrap();
    table["model"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m.as_table().unwrap().clone())
        .collect()
}

fn hex(bytes: &[u8; 32]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

#[test]
fn the_compiled_list_is_models_toml() {
    let toml = record();
    assert_eq!(toml.len(), MODELS.len());
    for (m, t) in MODELS.iter().zip(&toml) {
        let s = |k: &str| t[k].as_str().unwrap().to_owned();
        assert_eq!(m.kind.key(), s("kind"));
        assert_eq!(m.name, s("name"));
        assert_eq!(m.version, s("version"));
        assert_eq!(hex(&m.sha256), s("sha256"), "{}", m.name);
        assert_eq!(
            m.bytes,
            u64::try_from(t["bytes"].as_integer().unwrap()).unwrap()
        );
        assert_eq!(m.placeholder, t["placeholder"].as_bool().unwrap());
        assert_eq!(m.licence, s("licence"));
        assert_eq!(m.copyright, s("copyright"));
        assert_eq!(m.upstream, s("upstream"));
        assert_eq!(m.training_data, s("training_data"));
    }
}

/// Criterion 50: permissive licences; Remove background under 50 MB, the other two under 100 MB.
#[test]
fn licences_are_permissive_and_sizes_under_their_targets() {
    for m in MODELS {
        assert!(ALLOWED.contains(&m.licence), "{}: {}", m.name, m.licence);
        let target = match m.kind {
            ModelKind::Background => 50_000_000,
            ModelKind::Upscale | ModelKind::Denoise => 100_000_000,
        };
        assert!(m.bytes < target, "{}: {} bytes", m.name, m.bytes);
        assert!(!m.copyright.is_empty() && !m.training_data.is_empty());
    }
    let kinds: Vec<ModelKind> = MODELS.iter().map(|m| m.kind).collect();
    assert_eq!(
        kinds,
        [
            ModelKind::Background,
            ModelKind::Upscale,
            ModelKind::Denoise
        ]
    );
}

/// ADR 0076 §2: one fixed URL per model on the one host, no query.
#[test]
fn each_model_has_one_url_on_the_one_host() {
    assert_eq!(MODEL_HOST, "https://models.curvyo.org");
    for m in MODELS {
        let url = m.url();
        assert_eq!(
            url,
            format!(
                "{MODEL_HOST}/{}/{}/{}.onnx",
                m.name,
                m.version,
                hex(&m.sha256)
            )
        );
        assert!(!url.contains('?') && !url.contains('#'));
        assert_eq!(
            m.licence_url(),
            format!("{MODEL_HOST}/{}/{}/LICENSE.txt", m.name, m.version)
        );
    }
}

/// A placeholder is marked as one: its hash is 64 zeros, so it can never load.
#[test]
fn placeholders_are_marked() {
    for m in MODELS {
        assert_eq!(m.placeholder, m.sha256 == [0; 32], "{}", m.name);
    }
}

/// The release gate (`CURVYO_RELEASE_MODELS=1`): no placeholder may ship.
#[test]
fn release_gate_no_placeholder_remains() {
    if std::env::var_os("CURVYO_RELEASE_MODELS").is_none() {
        return;
    }
    let left: Vec<&str> = MODELS
        .iter()
        .filter(|m| m.placeholder)
        .map(|m| m.name)
        .collect();
    assert!(
        left.is_empty(),
        "placeholder model hashes remain: {left:?} (docs/runbooks/ai-models.md)"
    );
}
