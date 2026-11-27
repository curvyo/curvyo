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
