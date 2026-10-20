//! Fixtures (a) and (b) of `specs/0068-laserable-lettering` criterion 31 and criterion 32: script
//! lettering in Great Vibes welded through the session. (a) "Annabelle" at 20 mm, g = 0.1 mm,
//! within 300 ms, matching `tests/fixtures/weld/annabelle.golden` (its piece count, node count and
//! a hash of the grid coordinates), with no edge inside the result; (b) 500 characters at 10 mm
//! within 2 s. Budgets are asserted in release builds (the `performance-budgets` job); a debug
//! build prints the times. Regenerate the golden file with `CURVYO_UPDATE_GOLDEN=1`.

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

/// The welded path's outlines, read back from the saved document.
fn result_outlines(s: &Session) -> Vec<Vec<OutlineTriple>> {
    let document: Document = unpack(5, &s.pack("0.1.0").unwrap()).unwrap();
    let ids = document.object_ids();
    assert_eq!(ids.len(), 1, "one object");
    let Some(ObjectSnapshot::Path(path)) = document.object(ids[0]) else {
        panic!("a path");
    };
    path.subpaths()
        .map(|subpath| {
            subpath
                .anchors
                .iter()
                .map(|a| (a.point, a.handle_in, a.handle_out))
                .collect()
        })
        .collect()
}

fn weld_timed(s: &mut Session) -> (Duration, usize) {
    let started = Instant::now();
    let outcome = s.apply_weld(0.1);
    let elapsed = started.elapsed();
    let WeldOutcome::Applied { pieces, .. } = outcome else {
        panic!("{outcome:?}");
    };
    (elapsed, pieces)
}

fn summary(outlines: &[Vec<OutlineTriple>], pieces: usize) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut nodes = 0;
    for outline in outlines {
        for (point, _, _) in outline {
            nodes += 1;
            for value in [point.x, point.y] {
                hash ^= ((value * 1000.0).round() as i64).cast_unsigned();
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
        }
    }
    format!(
        "pieces {pieces} outlines {} nodes {nodes} hash {hash:016x}\n",
        outlines.len()
    )
}

/// Criteria 31 (a), 29 and 32.
#[test]
fn ac31a_annabelle_welds_within_300_ms_and_matches_its_golden_file() {
    let mut s = lettering("Annabelle", 20.0);
    let (elapsed, pieces) = weld_timed(&mut s);
    println!("Annabelle, 20 mm, g = 0.1 mm: {elapsed:?}, {pieces} pieces");
    let outlines = result_outlines(&s);
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/weld/annabelle.golden");
    let got = summary(&outlines, pieces);
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&golden, &got).unwrap();
    }
    assert_eq!(
        got,
        std::fs::read_to_string(&golden)
            .unwrap()
            .replace("\r\n", "\n")
    );
    // Criterion 32: no edge lies inside the result. Welded again alone at g = 0 (its own union),
    // it stays the same node for node.
    let WeldOutcome::Applied { pieces: again, .. } = s.apply_weld(0.0) else {
        panic!("the result welds again");
    };
    assert_eq!(again, pieces);
    let points = |o: &[Vec<OutlineTriple>]| -> Vec<Vec<Point>> {
        o.iter().map(|l| l.iter().map(|a| a.0).collect()).collect()
    };
    assert_eq!(points(&result_outlines(&s)), points(&outlines));
    if !cfg!(debug_assertions) {
        assert!(elapsed < Duration::from_millis(300), "{elapsed:?}");
    }
}

/// Criterion 31 (b).
#[test]
fn ac31b_five_hundred_characters_weld_within_two_seconds() {
    let string: String = "Annabelle ".chars().cycle().take(500).collect();
    let mut s = lettering(&string, 10.0);
    let (elapsed, pieces) = weld_timed(&mut s);
    println!("500 characters, 10 mm, g = 0.1 mm: {elapsed:?}, {pieces} pieces");
    if !cfg!(debug_assertions) {
        assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
    }
}
