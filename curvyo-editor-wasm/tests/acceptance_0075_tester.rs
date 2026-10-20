//! Independent tester cases for `specs/0075-font-library/specification.md` at the `Session`
//! level: criterion 26 (after Remove, a text falls back to the same family and style from
//! another block without a notice, else it is Missing; Undo does not bring the file back) and
//! criterion 25's count of texts using a Curvyo family.
//!
//! Written from the specification and the public names before the implementation was read.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{Point, Timestamp};
use curvyo_editor_wasm::{EscapeStep, Session, Tool};
use curvyo_ui_core::{FontBlock, TextInput};

static NOW: AtomicU64 = AtomicU64::new(1_950_000_000);

fn clock() -> Timestamp {
    Timestamp::from_secs(NOW.fetch_add(3, Ordering::SeqCst))
}

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

/// A Curvyo-folder copy of an included file with other bytes (a trailing pad), so its face
/// hash differs from the included face and matching must go by family and style.
const CURVYO_SANS: &str = "curvyo:SourceSans3-Copy.otf";
const CURVYO_SCRIPT: &str = "curvyo:GreatVibes-Copy.ttf";

fn bytes_of(host_id: &str) -> Vec<u8> {
    let (file, pad) = match host_id {
        CURVYO_SANS => ("source-sans-3/SourceSans3-Regular.otf", true),
        CURVYO_SCRIPT => ("great-vibes/GreatVibes-Regular.ttf", true),
        other => (other, false),
    };
    let mut b = std::fs::read(fonts_dir().join(file)).unwrap();
    if pad {
        b.extend_from_slice(&[0u8; 16]);
    }
    b
}

fn load(s: &mut Session) {
    for id in s.wanted_font_files() {
        assert!(s.add_font_file(&id, &bytes_of(&id)), "{id} loads");
    }
}

fn included_without_great_vibes() -> Vec<(String, String, String, u32)> {
    vec![
        (
            "source-sans-3/SourceSans3-Regular.otf".into(),
            "Source Sans 3".into(),
            "Regular".into(),
            0,
        ),
        (
            "source-serif-4/SourceSerif4-Regular.ttf".into(),
            "Source Serif 4".into(),
            "Regular".into(),
            0,
        ),
    ]
}

fn curvyo_block() -> Vec<(String, String, String, u32)> {
    vec![
        (
            CURVYO_SANS.into(),
            "Source Sans 3".into(),
            "Regular".into(),
            0,
        ),
        (
            CURVYO_SCRIPT.into(),
            "Great Vibes".into(),
            "Regular".into(),
            0,
        ),
    ]
}

fn session() -> Session {
    let mut s = Session::new(75);
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_font_list(FontBlock::Included, included_without_great_vibes(), vec![]);
    s.set_font_list(FontBlock::Curvyo, curvyo_block(), vec![]);
    load(&mut s);
    s
}

fn click(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

/// Types a text at `at` in the Curvyo face `host_id`; the text stays selected.
fn text_in(s: &mut Session, at: Point, host_id: &str) {
    s.set_tool(Tool::Text);
    click(s, at);
    assert!(s.text_editing());
    s.text_input(TextInput::Insert("Name".into()));
    load(s);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    s.pick_font_face(host_id, 0);
    load(s);
}

#[test]
fn ac26_removed_face_falls_back_to_same_family_else_missing() {
    let mut s = session();
    text_in(&mut s, Point::new(100.0, 100.0), CURVYO_SANS);
    assert_eq!(s.text_bar().font, "Source Sans 3 Regular");
    assert!(!s.text_bar().missing);
    assert_eq!(s.texts_using_family("Source Sans 3"), 1);
    assert_eq!(s.texts_using_family("source  sans 3 "), 1, "by family key");

    // Remove: the Curvyo block no longer lists the face.
    s.set_font_list(FontBlock::Curvyo, vec![curvyo_block()[1].clone()], vec![]);
    load(&mut s);
    let bar = s.text_bar();
    assert!(!bar.missing, "the included Source Sans 3 Regular is used");
    assert_eq!(bar.font, "Source Sans 3 Regular");
    assert!(s.missing_font_marks().is_empty(), "no notice, no mark");
    assert_eq!(s.texts_using_family("Source Sans 3"), 0);
}

#[test]
fn ac26_family_only_in_curvyo_becomes_missing_and_undo_does_not_restore() {
    let mut s = session();
    text_in(&mut s, Point::new(100.0, 100.0), CURVYO_SCRIPT);
    assert_eq!(s.text_bar().font, "Great Vibes Regular");
    assert!(!s.text_bar().missing);
    assert_eq!(s.texts_using_family("Great Vibes"), 1);
    let doc_before = s.pack("0.1.0").unwrap();

    s.set_font_list(FontBlock::Curvyo, vec![curvyo_block()[0].clone()], vec![]);
    load(&mut s);
    assert!(s.text_bar().missing, "no Great Vibes elsewhere: Missing");
    assert_eq!(s.missing_font_marks().len(), 1);
    // Removing a file is not a document change: the project is untouched.
    assert_eq!(s.pack("0.1.0").unwrap(), doc_before);

    // Undo steps back the font pick (a document step), never the listing.
    let _ = s.undo();
    load(&mut s);
    let _ = s.redo();
    load(&mut s);
    assert!(
        s.text_bar().missing,
        "undo and redo do not bring the removed file back"
    );
    // Add font brings it back.
    s.set_font_list(FontBlock::Curvyo, curvyo_block(), vec![]);
    load(&mut s);
    assert!(!s.text_bar().missing);
}
