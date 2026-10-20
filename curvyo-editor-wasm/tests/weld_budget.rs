//! Fixtures (a) and (b) of `specs/0068-laserable-lettering` criterion 27: script lettering in
//! Great Vibes welded through the session. (a) "Annabelle" at 20 mm within 300 ms, matching
//! `tests/fixtures/weld/annabelle.golden` (outline counts, node count and a hash of the grid
//! coordinates); (b) 500 characters at 10 mm within 2 s. Budgets are asserted in release builds
//! (the `performance-budgets` job); a debug build prints the times. Regenerate the golden file
//! with `CURVYO_UPDATE_GOLDEN=1`.

#![allow(clippy::unwrap_used, clippy::cast_possible_truncation, missing_docs)]

/// An anchor as the kernel reads it: point, handle in, handle out.
type OutlineTriple = (Point, Vec2, Vec2);

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use curvyo_document_core::{Document, ObjectSnapshot, Point, Vec2, unpack};
use curvyo_editor_wasm::{EscapeStep, Session, Tool, WeldOutcome};
use curvyo_ui_core::{FontBlock, TextInput};
use serde_json::Value;

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn load(s: &mut Session) {
    for file in s.wanted_font_files() {
        assert!(s.add_font_file(&file, &std::fs::read(fonts_dir().join(&file)).unwrap()));
    }
}

/// A session with one text `string` in Great Vibes at `size_mm`, selected in the Select tool.
fn lettering(string: &str, size_mm: f64) -> Session {
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
    let vibes = tab["faces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["family"] == "Great Vibes")
        .unwrap()["index"]
        .as_u64()
        .unwrap();
    s.pick_font(usize::try_from(vibes).unwrap());
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

/// The welded path's outlines with their closed flag, read back from the saved document.
fn result_outlines(s: &Session) -> Vec<(Vec<OutlineTriple>, bool)> {
    let document: Document = unpack(5, &s.pack("0.1.0").unwrap()).unwrap();
    let ids = document.object_ids();
    assert_eq!(ids.len(), 1, "one object");
    let Some(ObjectSnapshot::Path(path)) = document.object(ids[0]) else {
        panic!("a path");
    };
    path.subpaths()
        .map(|subpath| {
            let anchors = subpath
                .anchors
                .iter()
                .map(|a| (a.point, a.handle_in, a.handle_out))
                .collect();
            (anchors, subpath.closed)
        })
        .collect()
}

fn weld_timed(s: &mut Session) -> Duration {
    let started = Instant::now();
    let outcome = s.apply_weld();
    let elapsed = started.elapsed();
    assert!(
        matches!(outcome, WeldOutcome::Applied { .. }),
        "{outcome:?}"
    );
    elapsed
}

fn summary(outlines: &[(Vec<OutlineTriple>, bool)]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut nodes = 0;
    for (outline, closed) in outlines {
        hash = (hash ^ u64::from(*closed)).wrapping_mul(0x0100_0000_01b3);
        for (point, _, _) in outline {
            nodes += 1;
            for value in [point.x, point.y] {
                hash ^= ((value * 1000.0).round() as i64).cast_unsigned();
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
        }
    }
    let closed = outlines.iter().filter(|o| o.1).count();
    format!(
        "closed {closed} open {} nodes {nodes} hash {hash:016x}\n",
        outlines.len() - closed
    )
}

/// Criterion 27 (a) and 25.
#[test]
fn ac27a_annabelle_welds_within_300_ms_and_matches_its_golden_file() {
    let mut s = lettering("Annabelle", 20.0);
    let elapsed = weld_timed(&mut s);
    println!("Annabelle, 20 mm: {elapsed:?}");
    let outlines = result_outlines(&s);
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/weld/annabelle.golden");
    let got = summary(&outlines);
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden, &got).unwrap();
    }
    assert_eq!(
        got,
        std::fs::read_to_string(&golden)
            .unwrap()
            .replace("\r\n", "\n")
    );
    if !cfg!(debug_assertions) {
        assert!(elapsed < Duration::from_millis(300), "{elapsed:?}");
    }
}

/// Criterion 27 (b).
#[test]
fn ac27b_five_hundred_characters_weld_within_two_seconds() {
    let string: String = "Annabelle ".chars().cycle().take(500).collect();
    let mut s = lettering(&string, 10.0);
    let elapsed = weld_timed(&mut s);
    println!("500 characters, 10 mm: {elapsed:?}");
    if !cfg!(debug_assertions) {
        assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
    }
}
