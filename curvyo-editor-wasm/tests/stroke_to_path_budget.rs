//! Criterion 29 (b) and (c) of `specs/0085-stroke-to-path`: lettering stroked to paths through
//! the session. (b) 500 characters of Hershey Sans 1-stroke at 10 mm, width 0.5 mm, Round cap
//! and join, within 2 s of CPU time; (c) "Annabelle" in Great Vibes at 20 mm with a 1 mm stroke within
//! 500 ms of CPU time. Each matches its golden summary in `tests/fixtures/stroke_to_path/` (objects, closed
//! outlines, nodes, a hash of the grid coordinates). Budgets are asserted in release builds (the
//! `performance-budgets` job); a debug build prints the times. Regenerate the golden files with
//! `CURVYO_UPDATE_GOLDEN=1`.

#![allow(clippy::unwrap_used, clippy::cast_possible_truncation, missing_docs)]

mod cpu_time;

use std::path::{Path, PathBuf};
use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_document_core::{Document, LineCap, LineJoin, ObjectSnapshot, Point, unpack};
use curvyo_editor_wasm::{EscapeStep, Session, StrokeToPathOutcome, Tool};
use curvyo_ui_core::{FontBlock, StyleField, TextInput};
use serde_json::Value;

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn load(s: &mut Session) {
    for file in s.wanted_font_files() {
        assert!(s.add_font_file(&file, &std::fs::read(fonts_dir().join(&file)).unwrap()));
    }
}

/// A session with one text `string` in `family` at `size_mm`, selected in the Select tool.
fn lettering(family: &str, string: &str, size_mm: f64) -> Session {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            let text = |key: &str| f[key].as_str().unwrap().to_owned();
            (text("file"), text("family"), text("style"), 0)
        })
        .collect();
    let mut s = Session::new(1);
    s.resize_viewport(1200.0, 800.0);
    s.set_font_list(FontBlock::Included, faces, Vec::new());
    load(&mut s);
    let tab: Value = serde_json::from_str(&s.fonts_tab_json()).unwrap();
    let face = tab["faces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["family"] == family)
        .unwrap()["index"]
        .as_u64()
        .unwrap();
    s.pick_font(usize::try_from(face).unwrap());
    load(&mut s);
    s.set_tool(Tool::Text);
    s.pointer_down(Point::new(10.0, 40.0), false);
    s.pointer_up(Point::new(10.0, 40.0), false, false);
    s.text_input(TextInput::Insert(string.to_owned()));
    load(&mut s);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    assert_eq!(s.escape(), EscapeStep::LeftTool, "the text is selected");
    s.set_text_size(size_mm);
    load(&mut s);
    s
}

/// The command's thread CPU time (`tests/cpu_time/`), so a busy machine does not fail the budget.
fn timed(s: &mut Session) -> Duration {
    let (outcome, elapsed) = cpu_time_of(|| s.apply_stroke_to_path());
    assert!(
        matches!(outcome, StrokeToPathOutcome::Applied { .. }),
        "{outcome:?}"
    );
    elapsed
}

/// Objects, closed outlines, nodes and a hash of the grid coordinates of the saved document.
fn summary(s: &Session) -> String {
    let document: Document = unpack(5, &s.pack("0.1.0").unwrap()).unwrap();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let (mut objects, mut outlines, mut nodes) = (0, 0, 0);
    for id in document.object_ids() {
        let Some(ObjectSnapshot::Path(path)) = document.object(id) else {
            continue;
        };
        objects += 1;
        for subpath in path.subpaths() {
            outlines += usize::from(subpath.closed);
            for anchor in subpath.anchors {
                nodes += 1;
                for value in [anchor.point.x, anchor.point.y] {
                    hash ^= ((value * 1000.0).round() as i64).cast_unsigned();
                    hash = hash.wrapping_mul(0x0100_0000_01b3);
                }
            }
        }
    }
    format!("objects {objects} closed {outlines} nodes {nodes} hash {hash:016x}\n")
}

fn check_golden(name: &str, got: &str) {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-geometry-core/tests/fixtures/stroke_to_path")
        .join(name);
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden, got).unwrap();
    }
    assert_eq!(
        got,
        std::fs::read_to_string(&golden)
            .unwrap()
            .replace("\r\n", "\n")
    );
}

/// Criterion 29 (b).
#[test]
fn ac29b_five_hundred_hershey_characters_within_two_seconds() {
    let string: String = "Laser cut 42 ".chars().cycle().take(500).collect();
    let mut s = lettering("Hershey Sans 1-stroke", &string, 10.0);
    s.set_style_text(StyleField::StrokeWidth, "0.5").unwrap();
    s.set_stroke_cap(LineCap::Round);
    s.set_stroke_join(LineJoin::Round);
    let elapsed = timed(&mut s);
    println!("500 Hershey characters, 10 mm: {elapsed:?}");
    check_golden("hershey500.golden", &summary(&s));
    if !cfg!(debug_assertions) {
        assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
    }
}

/// Criterion 29 (c).
#[test]
fn ac29c_annabelle_with_a_one_millimetre_stroke_within_500_ms() {
    let mut s = lettering("Great Vibes", "Annabelle", 20.0);
    s.set_style_text(StyleField::StrokeWidth, "1").unwrap();
    let elapsed = timed(&mut s);
    println!("Annabelle, 20 mm, 1 mm stroke: {elapsed:?}");
    check_golden("annabelle.golden", &summary(&s));
    if !cfg!(debug_assertions) {
        assert!(elapsed < Duration::from_millis(500), "{elapsed:?}");
    }
}
