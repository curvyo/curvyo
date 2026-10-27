//! Golden files of the SVG import (`specs/0024-svg-import-export` criteria 72, 74 and 75): each
//! fixture `tests/fixtures/svg/<name>.svg` opened as a project gives the document in
//! `<name>.document.json` (page, display unit, background, the tree with leaves numbered in
//! document order; clones by their original's number and matrix; texts with their values, images
//! with their asset number and frame; the assets by hash), compared within 1e-9 mm. Texts use
//! the included faces (slice 2, criterion 107).
//!
//! `CURVYO_UPDATE_GOLDENS=1 cargo test -p curvyo-ui-core --test svg_import_golden` rewrites the
//! goldens; a change to one is a change of the spec's behaviour and is explained in the PR.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

mod svg_fonts;

use curvyo_ui_core::SvgPlacement;
use serde_json::{Value, json};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/svg")
}

/// The fixtures that import (not the hostile ones, not Inkscape's `name.paths.svg` conversions
/// of criterion 86), sorted.
fn names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(fixtures())
        .unwrap()
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;
            let stem = name.strip_suffix(".svg")?;
            let conversion = stem.strip_suffix(".paths").is_some();
            (!stem.starts_with("hostile_") && !conversion).then(|| stem.to_owned())
        })
        .collect();
    names.sort();
    names
}

/// The imported document of a fixture as JSON.
fn document_json(bytes: &[u8]) -> Value {
    thread_local! {
        static FONTS: svg_fonts::Fonts = svg_fonts::Fonts::included();
    }
    let import = FONTS.with(|fonts| fonts.import(bytes, SvgPlacement::File));
    let background = import.page.background.map(|b| {
        json!({
            "paint": b.paint.name(),
            "color": [b.color.r, b.color.g, b.color.b],
            "opacity": b.opacity.get(),
        })
    });
    json!({
        "page": [import.page.size.width.as_mm(), import.page.size.height.as_mm()],
        "display_unit": import.page.display_unit.map(curvyo_document_core::DisplayUnit::symbol),
        "background": background,
        "leaves": import.leaves,
        "nodes": serde_json::to_value(&import.nodes).unwrap(),
        "assets": import
            .assets
            .iter()
            .map(|bytes| curvyo_document_core::AssetHash::of(bytes).as_str().to_owned())
            .collect::<Vec<_>>(),
    })
}

/// Whether two JSON values are equal, numbers within `1e-9` (relative for large ones).
fn same(a: &Value, b: &Value, path: &str) -> Result<(), String> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            if (x - y).abs() <= 1e-9 * x.abs().max(1.0) {
                Ok(())
            } else {
                Err(format!("{path}: {x} != {y}"))
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Err(format!("{path}: {} items != {}", x.len(), y.len()));
            }
            for (i, (x, y)) in x.iter().zip(y).enumerate() {
                same(x, y, &format!("{path}[{i}]"))?;
            }
            Ok(())
        }
        (Value::Object(x), Value::Object(y)) => {
            if x.len() != y.len() || x.keys().any(|k| !y.contains_key(k)) {
                return Err(format!("{path}: keys differ"));
            }
            for (k, v) in x {
                same(v, &y[k], &format!("{path}.{k}"))?;
            }
            Ok(())
        }
        _ if a == b => Ok(()),
        _ => Err(format!("{path}: {a} != {b}")),
    }
}

#[test]
fn ac74_every_fixture_matches_its_golden_document() {
    let update = std::env::var_os("CURVYO_UPDATE_GOLDENS").is_some();
    let mut failures = Vec::new();
    for name in names() {
        let bytes = std::fs::read(fixtures().join(format!("{name}.svg"))).unwrap();
        let actual = document_json(&bytes);
        let golden = fixtures().join(format!("{name}.document.json"));
        if update {
            let text = serde_json::to_string_pretty(&actual).unwrap() + "\n";
            std::fs::write(&golden, text).unwrap();
            continue;
        }
        let expected: Value = serde_json::from_str(
            &std::fs::read_to_string(&golden).unwrap_or_else(|_| panic!("{name}: no golden")),
        )
        .unwrap();
        if let Err(diff) = same(&actual, &expected, &name) {
            failures.push(diff);
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn ac72_the_same_bytes_give_the_same_document() {
    for name in names() {
        let bytes = std::fs::read(fixtures().join(format!("{name}.svg"))).unwrap();
        assert_eq!(document_json(&bytes), document_json(&bytes), "{name}");
    }
}
