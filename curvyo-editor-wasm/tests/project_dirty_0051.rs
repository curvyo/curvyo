//! The dirty flag of the open project (`specs/0051-sidebar-and-no-menu/` criteria 22, 42 to 44):
//! the document differs from what the last Save, Open or New left, and an undo back to the saved
//! content does not clean it.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]

use std::io::Read;
use std::path::PathBuf;

use curvyo_document_core::{DisplayUnit, Point};
use curvyo_editor_wasm::{APP_VERSION, DocumentSide, Session, SizeOutcome, Tool, WipeOutcome};

/// The orphan file of `0049-linked-clone`, whose Open repairs it by design (criterion 35).
const ORPHAN_FIXTURE: &str = "clone_orphan_and_ring_v13.curvyo";

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fresh() -> Session {
    let mut session = Session::new(2);
    session.resize_viewport(1200.0, 800.0);
    session
}

/// Draws a 20 mm square at `x` with the Rectangle tool (one commit) and goes back to Select.
fn draw_square(session: &mut Session, x: f64) {
    session.set_tool(Tool::Rectangle);
    session.pointer_hover(pt(x, 10.0), false, false);
    session.pointer_down(pt(x, 10.0), false);
    session.pointer_hover(pt(x + 20.0, 30.0), false, false);
    session.pointer_up(pt(x + 20.0, 30.0), false, false);
    session.set_tool(Tool::Select);
}

/// A save whose write succeeded.
fn save(session: &mut Session) {
    session.pack_for_save().unwrap();
    session.mark_saved();
}

#[test]
fn a_new_project_is_clean() {
    assert!(!fresh().is_dirty());
}

#[test]
fn a_change_makes_the_project_dirty_and_a_save_makes_it_clean() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    assert!(session.is_dirty());
    save(&mut session);
    assert!(!session.is_dirty());
    draw_square(&mut session, 50.0);
    assert!(session.is_dirty());
}

/// Selection, tool, pan, zoom and a held preview are not the document (criterion 22).
#[test]
fn selection_tool_pan_and_zoom_do_not_make_the_project_dirty() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    save(&mut session);
    session.set_tool(Tool::Node);
    session.set_tool(Tool::Select);
    session.pointer_hover(pt(20.0, 20.0), false, false);
    session.pointer_down(pt(20.0, 20.0), false);
    session.pointer_up(pt(20.0, 20.0), false, false);
    session.wheel(0.0, 40.0, 100.0, 100.0, false, true);
    session.wheel(10.0, 0.0, 100.0, 100.0, false, false);
    session.begin_pan(10.0, 10.0);
    session.pan_to(40.0, 60.0);
    session.end_pan();
    assert!(!session.is_dirty());
}

/// One kind of change, named.
type Change = (&'static str, fn(&mut Session));

/// Every kind of step marks the project: a drawing, the document size, the display unit, the
/// background and the history wipe.
#[test]
fn every_kind_of_change_makes_it_dirty() {
    let steps: [Change; 5] = [
        ("draw", |s| draw_square(s, 10.0)),
        ("size", |s| {
            assert_eq!(
                s.set_document_side(DocumentSide::Width, "300"),
                SizeOutcome::Committed
            );
        }),
        ("unit", |s| {
            assert!(s.set_display_unit(DisplayUnit::In));
        }),
        ("background", |s| {
            assert!(s.set_background_hex("336699").unwrap());
        }),
        ("wipe", |s| {
            draw_square(s, 10.0);
            save_and_forget(s);
            assert!(matches!(s.wipe_history(), WipeOutcome::Wiped(_)));
        }),
    ];
    for (name, step) in steps {
        let mut session = fresh();
        step(&mut session);
        assert!(session.is_dirty(), "{name}");
    }
}

/// The `wipe` row above saves first: the wipe itself must be what dirties the project.
fn save_and_forget(session: &mut Session) {
    save(session);
    assert!(!session.is_dirty());
}

/// Criterion 22: a save, one step and Ctrl+Z leave the project marked unsaved.
#[test]
fn undo_back_to_the_saved_content_does_not_clean_the_project() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    save(&mut session);
    draw_square(&mut session, 50.0);
    assert!(session.is_dirty());
    session.undo();
    assert!(session.is_dirty());
    session.redo();
    assert!(session.is_dirty());
}

/// A history wipe is a commit, so it makes the project dirty (`adrs.md` Depends on, ADR 0014).
#[test]
fn a_history_wipe_makes_the_project_dirty() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    save(&mut session);
    assert!(matches!(session.wipe_history(), WipeOutcome::Wiped(_)));
    assert!(session.is_dirty());
}

/// Criterion 22: a change made while a save is being written is still unsaved after the write.
#[test]
fn a_change_during_a_slow_write_stays_dirty() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    session.pack_for_save().unwrap();
    draw_square(&mut session, 50.0);
    session.mark_saved();
    assert!(session.is_dirty());
    // The next save, with nothing in between, cleans it.
    save(&mut session);
    assert!(!session.is_dirty());
}

/// A failed write reports nothing: the project is as dirty as before, whatever happens next.
#[test]
fn a_failed_write_changes_nothing() {
    let mut session = fresh();
    draw_square(&mut session, 10.0);
    session.pack_for_save().unwrap();
    assert!(session.is_dirty());
    draw_square(&mut session, 50.0);
    assert!(session.is_dirty());
    save(&mut session);
    assert!(!session.is_dirty());
}

/// Criterion 43, New: the same.
#[test]
fn open_and_new_are_clean_after_an_idle_callback() {
    let mut source = fresh();
    draw_square(&mut source, 10.0);
    let bytes = source.pack_for_save().unwrap();
    let mut opened = Session::open(7, &bytes).unwrap();
    assert!(!opened.is_dirty());
    opened.warm_history();
    assert!(!opened.is_dirty());
    let mut new = Session::new(8);
    new.warm_history();
    assert!(!new.is_dirty());
}

/// Criterion 43: every project file in the fixtures opens clean and stays clean after the idle
/// callback that reads its history. Files that are refused (damaged, too new) are skipped.
#[test]
fn every_fixture_opens_clean() {
    let directories = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../curvyo-document-core/tests/fixtures"),
    ];
    let mut opened = 0;
    for directory in directories {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            // The orphan file of `0049` is marked changed by the orphan repair of its Open
            // (`specs/0049-linked-clone` criterion 35).
            if path.extension().and_then(|e| e.to_str()) != Some("curvyo")
                || path.file_name().is_some_and(|name| name == ORPHAN_FIXTURE)
            {
                continue;
            }
            let bytes = std::fs::read(&path).unwrap();
            let Ok(mut session) = Session::open(5, &bytes) else {
                continue;
            };
            opened += 1;
            assert!(!session.is_dirty(), "{} right after open", path.display());
            session.warm_history();
            assert!(
                !session.is_dirty(),
                "{} after the idle callback",
                path.display()
            );
        }
    }
    assert!(opened >= 10, "only {opened} fixtures opened");
}

/// Criterion 42: the version a save records is the crate version, the one the File tab shows.
#[test]
fn a_saved_file_records_the_crate_version() {
    assert_eq!(APP_VERSION, env!("CARGO_PKG_VERSION"));
    let mut session = fresh();
    let bytes = session.pack_for_save().unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut manifest = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(manifest["app_version"], APP_VERSION);
}

// ---- the reference model --------------------------------------------------------------------

/// A small deterministic generator (xorshift), so a failure names its seed.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n
    }
}

/// What the maker has done to the content, counted: `revision` moves on every step that changes
/// the document, `saved` is the revision that went into the last successful write, `packed` the
/// revision of a write that has not been reported yet. The project is dirty exactly when the two
/// differ.
#[derive(Default)]
struct Model {
    revision: u64,
    saved: u64,
    packed: Option<u64>,
}

impl Model {
    fn dirty(&self) -> bool {
        self.revision != self.saved
    }
}

/// Edit, save, an edit made while a save is pending, undo, redo, history wipe, open and New, in a
/// generated order: after every step the session's flag equals the model's (criterion 44).
#[test]
fn the_dirty_flag_follows_a_reference_model_over_generated_sequences() {
    for seed in 1..=60_u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let mut session = fresh();
        let mut model = Model::default();
        let mut x = 0.0;
        for step in 0..60 {
            let what = rng.below(12);
            let ctx = format!("seed {seed} step {step} op {what}");
            match what {
                0..=2 => {
                    x += 25.0;
                    draw_square(&mut session, x);
                    model.revision += 1;
                }
                3 => {
                    if session.undo_len() > 0 {
                        session.undo();
                        model.revision += 1;
                    }
                }
                4 => {
                    if session.redo_len() > 0 {
                        session.redo();
                        model.revision += 1;
                    }
                }
                5 => {
                    if matches!(session.wipe_history(), WipeOutcome::Wiped(_)) {
                        model.revision += 1;
                    }
                }
                6 => {
                    // A save whose write succeeded, with an edit made while it ran.
                    session.pack_for_save().unwrap();
                    model.packed = Some(model.revision);
                    if rng.below(2) == 0 {
                        x += 25.0;
                        draw_square(&mut session, x);
                        model.revision += 1;
                    }
                    session.mark_saved();
                    if let Some(packed) = model.packed.take() {
                        model.saved = packed;
                    }
                }
                7 => {
                    // A save whose write failed: nothing is reported.
                    session.pack_for_save().unwrap();
                    model.packed = Some(model.revision);
                }
                8 => {
                    let bytes = session.pack_for_save().unwrap();
                    session = Session::open(rng.below(1000) + 10, &bytes).unwrap();
                    session.resize_viewport(1200.0, 800.0);
                    model = Model::default();
                }
                9 => {
                    session = fresh();
                    model = Model::default();
                }
                10 => {
                    // Selection and the view are not the document.
                    session.set_tool(Tool::Node);
                    session.set_tool(Tool::Select);
                    session.wheel(0.0, 30.0, 10.0, 10.0, false, true);
                }
                _ => {
                    let unit = if rng.below(2) == 0 {
                        DisplayUnit::In
                    } else {
                        DisplayUnit::Mm
                    };
                    if session.set_display_unit(unit) {
                        model.revision += 1;
                    }
                }
            }
            assert_eq!(session.is_dirty(), model.dirty(), "{ctx}");
        }
    }
}
