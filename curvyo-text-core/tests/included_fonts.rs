//! The licence record of the shipped fonts (ADR 0020 §4, ADR 0006 item 5): `cargo deny` and the
//! npm check do not see `frontend/public/fonts/`, this test does. Every family folder has a
//! `font.toml` with an allowed licence and a licence text that matches it; every font file is
//! listed there, parses, and carries the family and style names the record states. The allowed
//! licences are OFL 1.1, Apache 2.0 and the Hershey notice (`specs/0076-single-line-fonts`
//! criterion 25, customer 2026-10-20).
#![allow(clippy::unwrap_used)]
#![cfg(not(target_arch = "wasm32"))]

mod common;

use std::collections::BTreeSet;

use common::fonts_dir;
use curvyo_text_core::{FaceKind, FaceStore, listable_faces};
use serde::Deserialize;
use skrifa::{FontRef, MetadataProvider, string::StringId};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    family: String,
    licence: String,
    licence_file: String,
    source_url: String,
    /// The shipped built-in tags (`specs/0075-font-library` criterion 14).
    tags: Vec<String>,
    version: String,
    faces: Vec<FaceRecord>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FaceRecord {
    file: String,
    style: String,
}

fn name(font: &FontRef<'_>, preferred: StringId, fallback: StringId) -> String {
    let read = |id| {
        font.localized_strings(id)
            .english_or_first()
            .map(|s| s.to_string())
    };
    read(preferred)
        .or_else(|| read(fallback))
        .unwrap_or_default()
}

#[derive(Deserialize, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(deny_unknown_fields)]
struct IndexFace {
    family: String,
    style: String,
    file: String,
    licence: String,
    /// `"single_line"` for a single-line face (`0076` decision 7); absent for an outline face.
    #[serde(default)]
    kind: Option<String>,
}

/// A family's shipped tags and the piece count of its check (`specs/0075-font-library`
/// criteria 14 and 17; the count is checked against the check in `curvyo-ui-core`).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexFamily {
    family: String,
    tags: Vec<String>,
    #[allow(dead_code)] // checked by `curvyo-ui-core/tests/cuttable_check.rs`
    cuttable_pieces: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    faces: Vec<IndexFace>,
    families: Vec<IndexFamily>,
}

/// Whether `file` is an SVG font (`specs/0076-single-line-fonts`).
fn is_svg(file: &str) -> bool {
    std::path::Path::new(file)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("svg"))
}

/// Every face of `record` parses and carries the names the record states.
fn check_faces(folder: &std::path::Path, record: &Record) {
    for face in &record.faces {
        let bytes = std::fs::read(folder.join(&face.file)).unwrap();
        let mut store = FaceStore::new();
        let key = store.add(bytes.clone(), 0);
        assert!(key.is_ok(), "{} parses", face.file);
        if is_svg(&face.file) {
            assert_eq!(
                store.get(&key.unwrap()).unwrap().kind(),
                FaceKind::SingleLine
            );
            let listed = &listable_faces(&bytes)[0];
            assert_eq!(
                (listed.family.as_str(), listed.style.as_str()),
                (record.family.as_str(), face.style.as_str())
            );
            continue;
        }
        let font = FontRef::new(&bytes).unwrap();
        let family = name(
            &font,
            StringId::TYPOGRAPHIC_FAMILY_NAME,
            StringId::FAMILY_NAME,
        );
        let style = name(
            &font,
            StringId::TYPOGRAPHIC_SUBFAMILY_NAME,
            StringId::SUBFAMILY_NAME,
        );
        assert_eq!(
            (family.as_str(), style.as_str()),
            (record.family.as_str(), face.style.as_str())
        );
    }
}

#[test]
fn every_included_font_has_an_allowed_licence_record() {
    let mut indexed: Vec<IndexFace> = Vec::new();
    let mut families = BTreeSet::new();
    let mut shipped_tags = BTreeSet::new();
    for entry in std::fs::read_dir(fonts_dir()).unwrap() {
        let folder = entry.unwrap().path();
        if !folder.is_dir() {
            continue;
        }
        let record: Record =
            toml::from_str(&std::fs::read_to_string(folder.join("font.toml")).unwrap()).unwrap();
        let licence_text = std::fs::read_to_string(folder.join(&record.licence_file)).unwrap();
        // The notice is wrapped over lines: compare with single spaces.
        let licence_text = licence_text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let marker = match record.licence.as_str() {
            "OFL-1.1" => "SIL Open Font License, Version 1.1",
            "Apache-2.0" => "Apache License",
            "Hershey" => "originally created by Dr. A. V. Hershey",
            other => panic!("{}: licence {other} is not allowed", folder.display()),
        };
        assert!(
            licence_text.contains(marker),
            "{}: licence text",
            folder.display()
        );
        assert!(record.source_url.starts_with("https://") && !record.version.is_empty());
        let folder_name = folder.file_name().unwrap().to_string_lossy().into_owned();
        for face in &record.faces {
            let single_line = is_svg(&face.file);
            indexed.push(IndexFace {
                family: record.family.clone(),
                style: face.style.clone(),
                file: format!("{folder_name}/{}", face.file),
                licence: record.licence.clone(),
                kind: single_line.then(|| "single_line".to_owned()),
            });
        }
        let listed: BTreeSet<&str> = record.faces.iter().map(|f| f.file.as_str()).collect();
        for file in std::fs::read_dir(&folder).unwrap() {
            let file = file.unwrap().file_name().into_string().unwrap();
            let is_font = std::path::Path::new(&file)
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| {
                    ["ttf", "otf", "ttc", "svg"].contains(&e.to_ascii_lowercase().as_str())
                });
            assert!(
                !is_font || listed.contains(file.as_str()),
                "{file} has no record"
            );
        }
        check_faces(&folder, &record);
        shipped_tags.insert((record.family.clone(), record.tags.clone()));
        families.insert(record.family);
    }
    let expected: BTreeSet<String> = [
        "EMS Allure",
        "EMS Readability",
        "Great Vibes",
        "Hershey Sans 1-stroke",
        "Hershey Script 1-stroke",
        "Source Sans 3",
        "Source Serif 4",
    ]
    .map(String::from)
    .into();
    assert_eq!(families, expected);
    // The browser build lists the included faces from `index.json` (it cannot read a folder):
    // it must name exactly the recorded faces.
    let index: Index =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    // Every included family has its tags in the index, and every entry names one.
    let index_tags: BTreeSet<(String, Vec<String>)> = index
        .families
        .iter()
        .map(|f| (f.family.clone(), f.tags.clone()))
        .collect();
    assert_eq!(index_tags, shipped_tags);
    assert_eq!(index.families.len(), shipped_tags.len());
    let mut from_index = index.faces;
    from_index.sort();
    indexed.sort();
    assert_eq!(from_index, indexed);
}
