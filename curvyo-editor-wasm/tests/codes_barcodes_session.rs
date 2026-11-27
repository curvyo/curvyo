//! The 1D codes at the session (`specs/0144` slice 2, ADR 0092): the Code segment as one family
//! entry, "Loading font..." as a precondition, the Symbology switch in one step, a pick in the
//! Fonts tab for a code and the free resize with its handles. The CPU-time budget of a code with
//! text is in `codes_cpu_time.rs`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]

use curvyo_document_core::{Document, pack};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_ui_core::FontBlock;
use serde_json::Value;
use std::path::PathBuf;

const QR: &str = "curvyo.codes/qr";
const EAN13: &str = "curvyo.codes/ean-13";
const CODE128: &str = "curvyo.codes/code-128";

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn included_faces() -> Vec<(String, String, String, u32)> {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
            )
        })
        .collect()
}

fn load(s: &mut Session) {
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes), "{file} loads");
    }
}

fn session() -> Session {
    let mut session = Session::open(2, &pack(&Document::new(1), "0.1.0").unwrap()).unwrap();
    session.set_tool(Tool::Select);
    session.resize_viewport(1600.0, 900.0);
    session.set_font_list(FontBlock::Included, included_faces(), Vec::new());
    session
}

fn panel(s: &Session) -> Value {
    serde_json::from_str(&s.generated_panel_json()).unwrap()
}

fn row_value(panel: &Value, key: &str) -> Value {
    panel["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["param"]["key"] == key)
        .map_or(Value::Null, |row| row["value"]["value"].clone())
}

#[test]
fn the_code_segment_is_one_family_entry_that_places_the_last_symbology() {
    let mut s = session();
    let kinds: Value = serde_json::from_str(&s.generator_kinds_json()).unwrap();
    let codes: Vec<&Value> = kinds
        .as_array()
        .unwrap()
        .iter()
        .filter(|k| k["segment"] == "Code")
        .collect();
    assert_eq!(codes.len(), 1, "one segment for the family");
    assert_eq!(codes[0]["id"], QR);
    assert_eq!(codes[0]["icon"], "QrCode");
    assert_eq!(
        kinds
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k["segment"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["Box", "Box from path", "Code", "Gear", "Rounded box"]
    );
    // Pressing it, then switching a placed code to EAN-13, makes the segment place EAN-13.
    s.set_generator_kind(QR);
    assert!(s.place_generated(100.0, 100.0));
    assert!(
        !s.switch_generated_kind(EAN13),
        "the included sans is not loaded yet"
    );
    load(&mut s);
    assert!(
        s.switch_generated_kind(EAN13),
        "the included sans is loaded"
    );
    let kinds: Value = serde_json::from_str(&s.generator_kinds_json()).unwrap();
    let entry = kinds
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["segment"] == "Code")
        .unwrap();
    assert_eq!(entry["id"], EAN13);
    assert_eq!(entry["icon"], "Barcode");
}

#[test]
fn a_kind_with_text_waits_for_the_included_sans_and_writes_nothing_meanwhile() {
    let mut s = session();
    s.set_generator_kind(EAN13);
    let kinds: Value = serde_json::from_str(&s.generator_kinds_json()).unwrap();
    let entry = kinds
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["segment"] == "Code")
        .unwrap();
    assert_eq!(entry["loading"], true);
    assert!(!s.place_generated(100.0, 100.0), "nothing is written");
    assert_eq!(s.selected_object_count(), 0);
    assert!(
        !s.wanted_font_files().is_empty(),
        "the session asks for the face"
    );
    load(&mut s);
    assert!(s.place_generated(100.0, 100.0));
    let kinds: Value = serde_json::from_str(&s.generator_kinds_json()).unwrap();
    assert_eq!(
        kinds
            .as_array()
            .unwrap()
            .iter()
            .find(|k| k["segment"] == "Code")
            .unwrap()["loading"],
        false
    );
}

#[test]
fn a_switch_is_one_step_with_the_family_in_the_panel_and_undo_restores_the_qr_code() {
    let mut s = session();
    load(&mut s);
    s.set_generator_kind(QR);
    assert!(s.place_generated(100.0, 100.0));
    let before = panel(&s);
    assert_eq!(before["family"]["label"], "Symbology");
    assert_eq!(before["family"]["current"], QR);
    assert_eq!(before["family"]["kinds"].as_array().unwrap().len(), 7);
    load(&mut s);
    assert!(s.switch_generated_kind(EAN13));
    let after = panel(&s);
    assert_eq!(after["label"], "EAN-13");
    assert_eq!(after["family"]["current"], EAN13);
    assert_eq!(row_value(&after, "content"), "2001234567893");
    assert_eq!(row_value(&after, "text"), true);
    assert_eq!(after["loading"], false);
    s.undo();
    assert_eq!(panel(&s)["label"], "QR code");
    assert!(
        !s.switch_generated_kind("curvyo.generators/gear"),
        "another family"
    );
}

#[test]
fn a_one_d_code_resizes_freely_so_its_box_has_edge_handles_and_a_qr_code_does_not() {
    let mut s = session();
    load(&mut s);
    s.set_generator_kind(CODE128);
    assert!(s.place_generated(100.0, 100.0));
    assert!(s.selection_resizes_freely());
    s.set_generator_kind(QR);
    assert!(s.place_generated(200.0, 100.0));
    assert!(!s.selection_resizes_freely());
}

const DATA_MATRIX: &str = "curvyo.codes/data-matrix";

#[test]
fn a_qr_code_switches_to_data_matrix_with_its_content_and_quiet_zone_default() {
    let mut s = session();
    load(&mut s);
    s.set_generator_kind(QR);
    assert!(s.place_generated(100.0, 100.0));
    assert!(s.switch_generated_kind(DATA_MATRIX));
    let after = panel(&s);
    assert_eq!(after["label"], "Data Matrix");
    assert_eq!(after["family"]["current"], DATA_MATRIX);
    assert_eq!(row_value(&after, "content"), "https://example.com");
    assert_eq!(
        row_value(&after, "quiet_zone"),
        1,
        "an untouched default follows"
    );
    for gone in ["finder_outer", "ec", "logo"] {
        assert_eq!(row_value(&after, gone), Value::Null, "{gone}");
    }
    s.undo();
    assert_eq!(panel(&s)["label"], "QR code");
}
