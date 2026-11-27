//! The cost of a parameter edit of a QR code (`specs/0144-qr-and-barcodes` criterion 17, ADR 0091
//! §10): the session's edit (encode, style, the one commit and the frame's canvas view) of a code of
//! version 10 within 20 ms and of version 40 within 100 ms of thread CPU time, for Square Merged
//! and Dot Separate. Asserted in release builds, printed in debug builds, as the other budget tests
//! do: `cargo test --release -p curvyo-editor-wasm --test codes_cpu_time -- --nocapture`. The
//! size of the saved file with the code is printed too (ADR 0091 §10).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod cpu_time;

use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_document_core::{Document, pack};
use curvyo_editor_wasm::{Session, Tool};

const QR: &str = "curvyo.codes/qr";

fn session() -> Session {
    let mut session = Session::open(2, &pack(&Document::new(1), "0.1.0").unwrap()).unwrap();
    session.set_tool(Tool::Select);
    session.resize_viewport(1600.0, 900.0);
    session
}

fn set(session: &mut Session, key: &str, value: &str) {
    let answer = session.set_generated_value(key, value);
    assert!(!answer.contains("error"), "{key}: {answer}");
}

fn check(label: &str, took: Duration, budget: Duration) {
    println!("{label}: {took:?} (budget {budget:?})");
    if !cfg!(debug_assertions) {
        assert!(took <= budget, "{label}: {took:?} is over {budget:?}");
    }
}

/// A text of `bytes` mixed bytes (byte mode, so the version follows the length).
fn text(bytes: usize) -> String {
    (0..bytes)
        .map(|i| char::from(b'a' + u8::try_from((i * 7 + i / 5) % 26).unwrap()))
        .collect()
}

fn measure(version: usize, bytes: usize, budget: Duration) {
    for (name, dot) in [("Square Merged", false), ("Dot Separate", true)] {
        let mut s = session();
        s.set_generator_kind(QR);
        assert!(s.place_generated(200.0, 200.0));
        set(
            &mut s,
            "content",
            &serde_json::to_string(&text(bytes)).unwrap(),
        );
        if dot {
            set(&mut s, "module_shape", "\"dot\"");
        }
        let panel = s.generated_panel_json();
        assert!(!panel.contains("Too long"), "{panel}");
        // Two edits that change the module size (the generator, the one commit, the canvas view).
        let (_, took) = cpu_time_of(|| {
            set(&mut s, "module_size", "1.5");
            s.draw_list()
        });
        let size = s.pack("0.1.0").unwrap().len();
        println!("version {version}, {name}: the saved file is {size} bytes");
        check(&format!("QR version {version}, {name}"), took, budget);
    }
}

#[test]
fn a_version_10_code_edits_within_20_ms() {
    measure(10, 150, Duration::from_millis(20));
}

#[test]
fn a_version_40_code_edits_within_100_ms() {
    measure(40, 2300, Duration::from_millis(100));
}

const EAN13: &str = "curvyo.codes/ean-13";

fn fonts_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

/// A session with the included faces listed and the ones wanted loaded (the human-readable text).
fn session_with_fonts() -> Session {
    let index: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    let faces = index["faces"]
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
        .collect();
    let mut s = session();
    s.set_font_list(curvyo_ui_core::FontBlock::Included, faces, Vec::new());
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes), "{file} loads");
    }
    s
}

/// Criterion 17, slice 2: a 1D code with its text regenerates within 20 ms (the glyph table is
/// built once per face, not per edit).
#[test]
fn a_one_d_code_with_text_edits_within_20_ms() {
    let mut s = session_with_fonts();
    s.set_generator_kind(EAN13);
    assert!(s.place_generated(200.0, 200.0));
    set(&mut s, "text_size", "2.5");
    let (_, took) = cpu_time_of(|| {
        set(&mut s, "module_width", "0.4");
        s.draw_list()
    });
    check("EAN-13 with text", took, Duration::from_millis(20));
}

/// Criterion 12 and 17, slice 3: a Data Matrix of the largest size regenerates within 100 ms like a
/// version 40 QR code (Square Merged and Dot Separate).
#[test]
fn a_144_by_144_data_matrix_edits_within_100_ms() {
    let high: String = (0..1555)
        .map(|i| char::from(0xa1 + u8::try_from(i * 7 % 90).unwrap()))
        .collect();
    for (name, dot) in [("Square Merged", false), ("Dot Separate", true)] {
        let mut s = session();
        s.set_generator_kind("curvyo.codes/data-matrix");
        assert!(s.place_generated(200.0, 200.0));
        set(&mut s, "content", &serde_json::to_string(&high).unwrap());
        if dot {
            set(&mut s, "module_shape", "\"dot\"");
        }
        let (_, took) = cpu_time_of(|| {
            set(&mut s, "module_size", "1.5");
            s.draw_list()
        });
        let size = s.pack("0.1.0").unwrap().len();
        println!("144 x 144, {name}: the saved file is {size} bytes");
        check(
            &format!("Data Matrix 144, {name}"),
            took,
            Duration::from_millis(100),
        );
    }
}
