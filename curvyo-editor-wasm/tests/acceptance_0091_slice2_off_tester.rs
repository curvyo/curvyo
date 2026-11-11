//! Tester case for `specs/0091-trace-text-recognition` criterion 24 with 12: with Recognise text
//! off, the multicolour trace (palette, outlines, nodes) and the paths Create paths writes are
//! exactly those of `main` before slice 2. Uses only names `main` already had, so the record
//! `tests/fixtures/acceptance_0091_slice2_off_main.json` is written on `main` with
//! `CURVYO_BLESS_MAIN=1` and compared on the story branch. The record keeps each wire's length
//! and an FNV-1a hash of its numbers' bits, and the path count and a hash of the written paths
//! (the full record of `main` e77fb42d was 2 MB and matched number for number).

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::path::PathBuf;
use vectrace::job::{Job as TraceJob, Mode as TraceMode, run as run_trace_job};
use vectrace::{PixelMap as TracePixelMap, Raster as TraceRaster};

use curvyo_document_core::{ObjectSnapshot, unpack};
use curvyo_editor_wasm::{Session, Tool, trace_colour_settings};
use serde_json::Value;

/// The frontend's first-use Multicolour settings (`settingsJson`): Brightness, automatic count.
const MULTI: &str = r#"{"output":1,"colours":null,"fixed":[],"excluded":[],"method":0,"background":null,"swatches":[],"threshold":128}"#;
/// Multicolour with Remove background on the white paper.
const MULTI_RB: &str =
    r##"{"output":1,"colours":null,"method":1,"background":"#ffffff","threshold":128}"##;
/// Multicolour with a fixed count of 4.
const MULTI_4: &str = r#"{"output":1,"colours":4,"method":0,"threshold":128}"#;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn record() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/acceptance_0091_slice2_off_main.json")
}

/// The plain multicolour wire of `file` with `settings`, and the paths Create paths writes.
fn off(file: &str, settings: &str) -> Value {
    let bytes = std::fs::read(
        root()
            .join("tests/assets/vectrace-fixtures/tester")
            .join(file),
    )
    .unwrap();
    let mut s = Session::new(91);
    s.resize_viewport(1600.0, 1000.0);
    s.import_begin(None);
    s.import_add(&bytes, file);
    s.import_finish(false);
    s.set_tool(Tool::Trace);
    let image = s.trace_view().shown.expect("the image is shown");
    let source = s.trace_source(image, false).unwrap();
    let wire = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: source.width,
                height: source.height,
                rgba: &source.rgba,
            },
            map: TracePixelMap::new(source.map),
            mode: TraceMode::Colours(&trace_colour_settings(settings).unwrap()),
            lines: &[],
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    );
    assert!(s.set_trace_preview(image, &wire));
    assert!(s.set_trace_result(image, &wire));
    let _ = s.create_traced_paths();
    let d = unpack(77, &s.pack("0.1.0").unwrap()).unwrap();
    let paths: Vec<String> = d
        .object_ids()
        .into_iter()
        .filter_map(|id| d.object(id))
        .filter_map(|o| match o {
            ObjectSnapshot::Path(p) => Some(format!("{p:?}")),
            _ => None,
        })
        .collect();
    serde_json::json!({
        "wire_len": wire.len(),
        "wire_fnv": fnv(wire.iter().flat_map(|v| v.to_bits().to_le_bytes())),
        "paths": paths.len(),
        "paths_fnv": fnv(paths.iter().flat_map(|p| p.bytes().collect::<Vec<_>>())),
    })
}

/// FNV-1a, 64 bit.
fn fnv(bytes: impl Iterator<Item = u8>) -> u64 {
    bytes.fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[test]
fn s2_c24_c12_plain_multicolour_equals_main() {
    let mut now = serde_json::Map::new();
    for file in ["colour_poster.png", "poster.png"] {
        for (name, settings) in [("multi", MULTI), ("rb", MULTI_RB), ("four", MULTI_4)] {
            now.insert(format!("{file} {name}"), off(file, settings));
        }
    }
    if std::env::var_os("CURVYO_BLESS_MAIN").is_some() {
        std::fs::write(record(), serde_json::to_string(&now).unwrap()).unwrap();
        return;
    }
    let kept: Value = serde_json::from_str(&std::fs::read_to_string(record()).unwrap()).unwrap();
    assert_eq!(kept.as_object().unwrap().len(), now.len());
    for (key, value) in kept.as_object().unwrap() {
        assert_eq!(
            value["wire"], now[key]["wire"],
            "{key}: the wire as on main"
        );
        assert_eq!(
            value["paths"], now[key]["paths"],
            "{key}: the paths as on main"
        );
    }
}
