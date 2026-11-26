//! `specs/0156-fused-lettering` criteria 10 and 25 with a real font: Source Sans 3 Regular (SIL
//! Open Font License, shipped in `frontend/public/fonts/source-sans-3/` with its licence), set at
//! 20 mm on a laser page and bridged through the session. Lettering: i, ü, ö and ä get 1, 2, 2 and
//! 2 bridges, each from a dot to its letter. Stencil: B, A, e and 8 get two bridges per island.

#![allow(clippy::unwrap_used, missing_docs)]

use std::path::PathBuf;

use curvyo_document_core::Point;
use curvyo_editor_wasm::{EscapeStep, FuseOutcome, Session, Tool};
use curvyo_ui_core::{FontBlock, FuseMode, FuseSettings, TextInput};
use serde_json::Value;

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn load(s: &mut Session) {
    for file in s.wanted_font_files() {
        assert!(s.add_font_file(&file, &std::fs::read(fonts_dir().join(&file)).unwrap()));
    }
}

/// A session with a laser page made active and one text `string` in Source Sans 3 at 20 mm on
/// it, selected in the Select tool.
fn lettering(string: &str) -> Session {
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
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let origin = s.active_page().origin;
    let tab: Value = serde_json::from_str(&s.fonts_tab_json()).unwrap();
    let face = tab["faces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["family"] == "Source Sans 3" && f["style"] == "Regular")
        .unwrap()["index"]
        .as_u64()
        .unwrap();
    s.pick_font(usize::try_from(face).unwrap());
    load(&mut s);
    s.set_tool(Tool::Text);
    let at = Point::new(origin.x + 20.0, origin.y + 60.0);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    s.text_input(TextInput::Insert(string.to_owned()));
    load(&mut s);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    assert_eq!(s.escape(), EscapeStep::LeftTool, "the text is selected");
    s.set_text_size(20.0);
    load(&mut s);
    s
}

fn bridges(string: &str, mode: FuseMode) -> FuseOutcome {
    let mut s = lettering(string);
    s.apply_fuse(FuseSettings {
        mode,
        width_mm: 1.0,
        reach_mm: 5.0,
        join_all: false,
        bridges_per_island: 2.0,
    })
}

/// Criterion 10: the dots of i, ü, ö and ä each get one bridge to their letter, none between
/// two dots.
#[test]
fn c10_dotted_letters_get_one_bridge_per_dot() {
    for (letter, count) in [("i", 1), ("ü", 2), ("ö", 2), ("ä", 2)] {
        assert_eq!(
            bridges(letter, FuseMode::Lettering),
            FuseOutcome::Applied { bridges: count },
            "{letter}"
        );
    }
}

/// Criterion 25: B, A, e and 8 get two bridges per island (2, 1, 1 and 2 islands).
#[test]
fn c25_stencil_letters_get_two_bridges_per_island() {
    for (letter, islands) in [("B", 2), ("A", 1), ("e", 1), ("8", 2)] {
        assert_eq!(
            bridges(letter, FuseMode::Stencil),
            FuseOutcome::Applied {
                bridges: 2 * islands
            },
            "{letter}"
        );
    }
}
