//! Independent tester cases for `specs/0051-sidebar-and-no-menu/specification.md` at the `Session`
//! level: the dirty flag state machine (criteria 17 to 22, 43, 44), the file chord gate (32, 33,
//! 35), the saved version (42), the File tab in the panel state (2, 14), and hostile inputs.
//!
//! Written from the specification and the public names before the implementation was read (the
//! public signatures of `is_dirty`, `pack_for_save`, `mark_saved` and `Session::file_chord` were
//! listed to compile against).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::similar_names,
    clippy::many_single_char_names,
    missing_docs,
    clippy::collapsible_if,
    clippy::bool_assert_comparison,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::needless_pass_by_value
)]

use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{Point, Timestamp};
use curvyo_editor_wasm::{
    APP_VERSION, DocumentSide, KeyInput, KeyOutcome, Session, SizeOutcome, Tool, WipeOutcome,
};
use curvyo_ui_core::{FileCommand, Platform};

static NOW: AtomicU64 = AtomicU64::new(1_800_000_000);

fn clock() -> Timestamp {
    Timestamp::from_secs(NOW.fetch_add(7, Ordering::SeqCst))
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fresh() -> Session {
    let mut s = Session::new(11);
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_tool(Tool::Select);
    s
}

fn drag(s: &mut Session, tool: Tool, a: Point, b: Point) {
    s.set_tool(tool);
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(
        pt(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)),
        false,
        false,
    );
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
}

/// One more rectangle: a committed change.
fn edit(s: &mut Session) {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = (N.fetch_add(1, Ordering::SeqCst) % 20) as f64;
    drag(
        s,
        Tool::Rectangle,
        pt(10.0 + n, 10.0 + n),
        pt(60.0 + n, 50.0 + n),
    );
    s.set_tool(Tool::Select);
}

fn save(s: &mut Session) -> Vec<u8> {
    let bytes = s.pack_for_save().expect("pack");
    s.mark_saved();
    bytes
}

fn key(s: &mut Session, k: &str, ctrl: bool, shift: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        shift,
        ..KeyInput::default()
    })
}

fn undo(s: &mut Session) {
    if key(s, "z", true, false) == KeyOutcome::UndoBusy {
        s.undo();
    }
}

fn open(bytes: &[u8]) -> Session {
    match Session::open(5, bytes) {
        Ok(s) => s,
        Err(e) => panic!("open failed: {e:?}"),
    }
}

fn input(k: &str, shift: bool) -> KeyInput<'_> {
    KeyInput {
        key: k,
        ctrl: true,
        shift,
        ..KeyInput::default()
    }
}

// ---------------------------------------------------------------- dirty state machine

#[test]
fn c17_new_session_is_clean_and_empty_history() {
    let s = fresh();
    assert!(!s.is_dirty());
    assert_eq!(s.undo_len(), 0);
    assert_eq!(s.redo_len(), 0);
}

#[test]
fn c22_change_makes_dirty_save_makes_clean() {
    let mut s = fresh();
    edit(&mut s);
    assert!(s.is_dirty());
    save(&mut s);
    assert!(!s.is_dirty());
    edit(&mut s);
    assert!(s.is_dirty());
}

#[test]
fn c22_undo_back_to_saved_state_stays_dirty_and_redo_too() {
    let mut s = fresh();
    edit(&mut s);
    save(&mut s);
    assert!(!s.is_dirty());
    edit(&mut s);
    assert!(s.is_dirty());
    undo(&mut s);
    assert!(s.is_dirty(), "undo is a forward step: still unsaved");
    key(&mut s, "Z", true, true);
    assert!(s.is_dirty());
    undo(&mut s);
    assert!(s.is_dirty());
}

#[test]
fn c22_undo_on_empty_stack_and_redo_on_empty_stack_do_not_dirty() {
    let mut s = fresh();
    undo(&mut s);
    key(&mut s, "Z", true, true);
    key(&mut s, "y", true, false);
    assert!(!s.is_dirty());
    edit(&mut s);
    save(&mut s);
    // nothing to redo
    key(&mut s, "Z", true, true);
    assert!(
        !s.is_dirty(),
        "redo with an empty redo stack changes nothing"
    );
}

#[test]
fn c22_non_document_state_never_dirties() {
    let mut s = fresh();
    edit(&mut s);
    edit(&mut s);
    save(&mut s);
    // tool, pan, zoom, selection, viewport, active tab
    for t in [
        Tool::Pen,
        Tool::Node,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::PolygonStar,
        Tool::Select,
    ] {
        s.set_tool(t);
    }
    s.resize_viewport(900.0, 600.0);
    s.wheel(0.0, 40.0, 100.0, 100.0, false, false);
    s.wheel(0.0, -120.0, 100.0, 100.0, false, true);
    s.begin_pan(10.0, 10.0);
    s.pan_to(80.0, 90.0);
    s.end_pan();
    key(&mut s, "a", true, false);
    s.escape();
    s.panel_shortcut_history();
    s.panel_shortcut_document();
    s.panel_shortcut_style();
    s.press_panel_tab("file");
    s.press_panel_tab("history");
    assert!(
        !s.is_dirty(),
        "view/selection/tool/tab changes are not document changes"
    );
}

#[test]
fn c22_every_document_step_kind_dirties() {
    // rectangle, ellipse, polygon, delete, document size, wipe
    for tool in [Tool::Rectangle, Tool::Ellipse, Tool::PolygonStar] {
        let mut s = fresh();
        edit(&mut s);
        save(&mut s);
        drag(&mut s, tool, pt(20.0, 20.0), pt(90.0, 70.0));
        assert!(s.is_dirty(), "{tool:?}");
    }
    let mut s = fresh();
    edit(&mut s);
    key(&mut s, "a", true, false);
    save(&mut s);
    key(&mut s, "Delete", false, false);
    assert!(s.is_dirty(), "delete");

    let mut s = fresh();
    save(&mut s);
    assert_eq!(
        s.set_document_side(DocumentSide::Width, "300"),
        SizeOutcome::Committed
    );
    assert!(s.is_dirty(), "document size");

    let mut s = fresh();
    edit(&mut s);
    save(&mut s);
    assert!(matches!(s.wipe_history(), WipeOutcome::Wiped(_)));
    assert!(
        s.is_dirty(),
        "history wipe is a document step (criterion 22)"
    );
}

#[test]
fn c22_rejected_input_does_not_dirty() {
    let mut s = fresh();
    edit(&mut s);
    save(&mut s);
    for text in [
        "",
        " ",
        "NaN",
        "nan",
        "inf",
        "-inf",
        "-5",
        "0",
        "1e999",
        "abc",
        "１２３",
        "0x10",
        "1,5,5",
    ] {
        let outcome = s.set_document_side(DocumentSide::Width, text);
        if outcome != SizeOutcome::Committed {
            assert!(!s.is_dirty(), "{text:?} was {outcome:?} but dirtied");
        }
        if s.is_dirty() {
            // text was accepted as a size; clean up for the next iteration
            save(&mut s);
        }
    }
    let unchanged = s.set_document_side(DocumentSide::Width, "300");
    save(&mut s);
    assert_eq!(unchanged, SizeOutcome::Committed);
    assert_eq!(
        s.set_document_side(DocumentSide::Width, "300"),
        SizeOutcome::Unchanged
    );
    assert!(
        !s.is_dirty(),
        "typing the size the document already has is no change"
    );
}

#[test]
fn c19_failed_write_changes_nothing_and_stays_dirty() {
    let mut s = fresh();
    edit(&mut s);
    let _ = s.pack_for_save().unwrap(); // the write fails: no mark_saved
    assert!(s.is_dirty());
    // a later successful save works
    save(&mut s);
    assert!(!s.is_dirty());
}

#[test]
fn c22_failed_write_after_clean_edit_leaves_dirty_then_next_save_cleans() {
    let mut s = fresh();
    edit(&mut s);
    save(&mut s);
    edit(&mut s);
    let _ = s.pack_for_save().unwrap(); // failed write
    assert!(s.is_dirty());
    edit(&mut s);
    save(&mut s);
    assert!(!s.is_dirty());
}

#[test]
fn c22_change_while_save_is_being_written_is_dirty_after_write() {
    let mut s = fresh();
    edit(&mut s);
    let bytes = s.pack_for_save().unwrap();
    edit(&mut s); // the maker keeps working during the write
    s.mark_saved();
    assert!(s.is_dirty(), "the file does not hold the second edit");
    // and the file really lacks it
    let reopened = open(&bytes);
    assert!(!reopened.is_dirty());
}

#[test]
fn c22_mark_saved_without_pack_is_a_no_op() {
    let mut s = fresh();
    edit(&mut s);
    s.mark_saved();
    assert!(s.is_dirty(), "nothing was written: never shown clean");
    s.mark_saved();
    assert!(s.is_dirty());
}

#[test]
fn c22_mark_saved_twice_after_one_pack_stays_correct() {
    let mut s = fresh();
    edit(&mut s);
    let _ = s.pack_for_save().unwrap();
    s.mark_saved();
    assert!(!s.is_dirty());
    edit(&mut s);
    s.mark_saved(); // late duplicate completion callback
    assert!(
        s.is_dirty(),
        "a duplicate completion must not clean a later change"
    );
}

#[test]
fn c22_two_saves_in_flight_the_older_completing_last_is_safe() {
    let mut s = fresh();
    edit(&mut s);
    let _first = s.pack_for_save().unwrap();
    edit(&mut s);
    let _second = s.pack_for_save().unwrap(); // second save starts before first completes
    s.mark_saved(); // the completion of one of them
    // The file on disk holds at most the second pack's content; whatever was marked, an
    // edit after both packs must be dirty.
    edit(&mut s);
    s.mark_saved();
    assert!(s.is_dirty());
}

#[test]
fn c18_open_replaces_and_is_clean_even_from_a_dirty_session_bytes() {
    let mut a = fresh();
    edit(&mut a);
    edit(&mut a);
    let bytes = save(&mut a);
    edit(&mut a);
    assert!(a.is_dirty());
    let b = open(&bytes);
    assert!(!b.is_dirty());
    assert_eq!(
        b.undo_len(),
        0,
        "opened project has no undo steps of the previous session"
    );
    assert_eq!(b.redo_len(), 0);
}

#[test]
fn c18_damaged_open_is_refused_and_does_not_panic() {
    let mut a = fresh();
    edit(&mut a);
    let mut bytes = a.pack_for_save().unwrap();
    a.mark_saved();
    // Hostile inputs
    for bad in [
        Vec::new(),
        vec![0u8],
        b"not a zip at all".to_vec(),
        b"PK\x03\x04".to_vec(),
        b"PK\x05\x06\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0".to_vec(),
        bytes[..bytes.len() / 2].to_vec(),
        bytes[..bytes.len() - 1].to_vec(),
    ] {
        assert!(Session::open(5, &bad).is_err(), "{} bytes", bad.len());
    }
    // flip bytes in the middle
    let mid = bytes.len() / 2;
    for b in &mut bytes[mid..mid + 16] {
        *b ^= 0xFF;
    }
    let _ = Session::open(5, &bytes); // may fail or not; must not panic
    // the open session is untouched by anything above
    assert!(!a.is_dirty());
}

#[test]
fn c30_title_sequence_edit_save_edit_undo_reopen() {
    // The flag sequence behind "*Curvyo", "Curvyo", "*Curvyo", "*Curvyo", "Curvyo".
    let mut s = fresh();
    edit(&mut s);
    assert!(s.is_dirty());
    let bytes = save(&mut s);
    assert!(!s.is_dirty());
    edit(&mut s);
    assert!(s.is_dirty());
    undo(&mut s);
    assert!(s.is_dirty());
    let s = open(&bytes);
    assert!(!s.is_dirty());
}

#[test]
fn c31_first_change_after_save_is_visible_immediately_after_commit() {
    // No deferred commit: the flag is true right after the gesture returns, with no further call.
    let mut s = fresh();
    save(&mut s);
    s.set_tool(Tool::Rectangle);
    s.pointer_hover(pt(10.0, 10.0), false, false);
    s.pointer_down(pt(10.0, 10.0), false);
    s.pointer_hover(pt(50.0, 50.0), false, false);
    s.pointer_up(pt(50.0, 50.0), false, false);
    assert!(s.is_dirty());
}

// ---------------------------------------------------------------- fixtures open clean (43)

fn fixture_files() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut out = Vec::new();
    for dir in [
        "curvyo-editor-wasm/tests/fixtures",
        "curvyo-document-core/tests/fixtures",
    ] {
        let Ok(rd) = fs::read_dir(root.join(dir)) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "curvyo") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn c43_every_fixture_that_opens_is_clean_after_one_idle_callback() {
    let files = fixture_files();
    assert!(files.len() >= 20, "found {} fixtures", files.len());
    let mut opened = 0;
    let mut refused = 0;
    for p in files {
        let bytes = fs::read(&p).unwrap();
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        match Session::open(5, &bytes) {
            Ok(mut s) => {
                opened += 1;
                assert!(!s.is_dirty(), "{name}: dirty right after open");
                // one idle callback: reads the host does between events
                s.resize_viewport(1024.0, 768.0);
                let _ = s.draw_list();
                let _ = s.panel_view();
                let _ = s.panel_content();
                let _ = s.take_history_notice();
                let _ = s.zoom_percent();
                let _ = s.undo_len();
                let _ = s.redo_len();
                assert!(!s.is_dirty(), "{name}: dirty after one idle callback");
                // saving what was opened and reopening is also clean
                let out = s.pack_for_save().unwrap();
                s.mark_saved();
                assert!(!s.is_dirty(), "{name}: dirty after a save");
                let again = open(&out);
                assert!(!again.is_dirty(), "{name}: reopened save is dirty");
            }
            Err(_) => refused += 1,
        }
    }
    assert!(opened >= 10, "opened {opened}, refused {refused}");
    assert!(refused >= 1, "no damaged fixture was refused");
}

#[test]
fn c43_open_of_current_save_with_history_wipe_is_clean_and_new_is_clean() {
    let mut s = fresh();
    edit(&mut s);
    s.wipe_history();
    let bytes = save(&mut s);
    assert!(!open(&bytes).is_dirty());
    assert!(!Session::new(1).is_dirty());
}

// ---------------------------------------------------------------- version (42)

fn manifest_text(bytes: &[u8]) -> String {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut all = String::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        if f.name().ends_with(".json") {
            let mut t = String::new();
            let _ = f.read_to_string(&mut t);
            all.push_str(&t);
        }
    }
    all
}

#[test]
fn c42_saved_file_records_the_crate_version() {
    let mut s = fresh();
    edit(&mut s);
    let bytes = s.pack_for_save().unwrap();
    let text = manifest_text(&bytes);
    assert!(text.contains(APP_VERSION), "manifest: {text}");
    assert!(!text.contains("web-demo"));
    let workspace =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml"))
            .unwrap();
    assert!(
        workspace.contains(&format!("version = \"{APP_VERSION}\"")),
        "APP_VERSION {APP_VERSION} is not the workspace version"
    );
}

// ---------------------------------------------------------------- chord gate (32, 33)

#[test]
fn c33_gate_blocks_during_drag_chip_repeat_and_blocked_but_not_pen_path() {
    let s = fresh();
    let platform = Platform::Linux;
    assert_eq!(
        s.file_chord(&input("s", false), platform),
        Some(FileCommand::Save)
    );

    // key repeat
    let mut rep = input("s", false);
    rep.repeat = true;
    assert_eq!(s.file_chord(&rep, platform), None);
    // dom blocked flag (IME / long operation reported by the page)
    let mut blocked = input("s", false);
    blocked.dom_blocked = true;
    assert_eq!(s.file_chord(&blocked, platform), None);

    // a pointer drag in each tool
    for tool in [
        Tool::Select,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::PolygonStar,
        Tool::Node,
        Tool::Pen,
    ] {
        let mut s = fresh();
        edit(&mut s);
        s.set_tool(tool);
        s.pointer_hover(pt(200.0, 200.0), false, false);
        s.pointer_down(pt(200.0, 200.0), false);
        s.pointer_hover(pt(240.0, 240.0), false, false);
        assert_eq!(
            s.file_chord(&input("s", false), platform),
            None,
            "drag in {tool:?}"
        );
        assert_eq!(
            s.file_chord(&input("o", false), platform),
            None,
            "drag in {tool:?}"
        );
        s.pointer_up(pt(240.0, 240.0), false, false);
        assert_eq!(
            s.file_chord(&input("s", false), platform),
            Some(FileCommand::Save),
            "after drag in {tool:?}"
        );
    }

    // a typed-entry chip is open
    let mut s = fresh();
    edit(&mut s);
    key(&mut s, "a", true, false);
    assert_eq!(key(&mut s, "r", false, false), KeyOutcome::EntryOpened);
    for (k, shift) in [
        ("s", false),
        ("S", true),
        ("o", false),
        ("n", false),
        ("w", false),
        ("q", false),
    ] {
        assert_eq!(
            s.file_chord(&input(k, shift), platform),
            None,
            "chip open: {k}"
        );
    }
    s.escape();
    assert_eq!(
        s.file_chord(&input("s", false), platform),
        Some(FileCommand::Save)
    );

    // an unfinished Pen path does not block (the path is not part of the document)
    let mut s = fresh();
    s.set_tool(Tool::Pen);
    for p in [pt(20.0, 20.0), pt(80.0, 20.0), pt(80.0, 60.0)] {
        s.pointer_hover(p, false, false);
        s.pointer_down(p, false);
        s.pointer_up(p, false, false);
    }
    assert_eq!(
        s.file_chord(&input("s", false), platform),
        Some(FileCommand::Save)
    );
    assert_eq!(
        s.file_chord(&input("S", true), platform),
        Some(FileCommand::SaveAs)
    );
}

#[test]
fn c33_file_chords_leave_the_editors_own_key_table_alone() {
    let mut s = fresh();
    edit(&mut s);
    let before = s.undo_len();
    for (k, shift) in [
        ("s", false),
        ("S", true),
        ("n", false),
        ("o", false),
        ("w", false),
        ("q", false),
    ] {
        let out = key(&mut s, k, true, shift);
        assert_eq!(out, KeyOutcome::Ignored, "editor key table took Ctrl+{k}");
    }
    assert_eq!(s.undo_len(), before);
}

#[test]
fn c33_gate_on_a_state_never_mutates_the_document() {
    let mut s = fresh();
    edit(&mut s);
    save(&mut s);
    for _ in 0..50 {
        for k in ["s", "o", "n", "w", "q"] {
            let _ = s.file_chord(&input(k, false), Platform::Linux);
        }
    }
    assert!(!s.is_dirty());
}

#[test]
fn c35_undo_redo_keys_do_not_map_to_file_commands() {
    let s = fresh();
    for p in [
        Platform::Linux,
        Platform::Windows,
        Platform::MacOs,
        Platform::Browser,
    ] {
        for (k, shift) in [
            ("z", false),
            ("Z", true),
            ("y", false),
            ("u", false),
            ("U", true),
            ("a", false),
        ] {
            assert_eq!(s.file_chord(&input(k, shift), p), None, "{p:?} {k}");
        }
    }
}

// ---------------------------------------------------------------- panel tabs (2, 5, 14)

fn names(s: &Session) -> Vec<&'static str> {
    s.panel_view().tabs.iter().map(|t| t.name).collect()
}

fn active(s: &Session) -> Vec<&'static str> {
    s.panel_view()
        .tabs
        .iter()
        .filter(|t| t.selected)
        .map(|t| t.name)
        .collect()
}

#[test]
fn c2_tab_order_is_file_document_style_history_and_labels_and_tooltips() {
    let s = fresh();
    let view = s.panel_view();
    assert_eq!(
        names(&s),
        ["file", "document", "style", "layers", "history"]
    );
    let labels: Vec<_> = view.tabs.iter().map(|t| t.label).collect();
    assert_eq!(labels, ["File", "Document", "Style", "Layers", "History"]);
    let tip = |n: &str| view.tabs.iter().find(|t| t.name == n).unwrap().tooltip;
    assert_eq!(tip("file"), "File", "the File tab names no key");
    assert_eq!(tip("document"), "Document (Shift+Ctrl+D)");
    assert_eq!(tip("history"), "History (Shift+Ctrl+H)");
    assert!(tip("style").starts_with("Style"));
    // exactly one active tab at any time
    assert_eq!(active(&s).len(), 1);
}

#[test]
fn c14_a_fresh_or_opened_project_starts_on_document() {
    let mut s = fresh();
    assert_eq!(active(&s), ["document"]);
    edit(&mut s);
    let bytes = save(&mut s);
    let o = open(&bytes);
    assert_eq!(
        active(&o),
        ["document"],
        "Open hands the maker to the drawing"
    );
}

#[test]
fn c14_file_tab_is_sticky_and_content_is_file() {
    let mut s = fresh();
    assert!(s.press_panel_tab("file"));
    assert_eq!(active(&s), ["file"]);
    assert_eq!(s.panel_content(), curvyo_ui_core::PanelContent::File);
    // selection appears, changes, clears, tools change, drawing happens: still File
    edit(&mut s);
    key(&mut s, "a", true, false);
    assert_eq!(active(&s), ["file"], "a selection must not leave File");
    drag(&mut s, Tool::Rectangle, pt(100.0, 100.0), pt(150.0, 150.0));
    assert_eq!(active(&s), ["file"]);
    s.escape();
    assert_eq!(
        active(&s),
        ["file"],
        "clearing the selection must not leave File"
    );
    s.set_tool(Tool::Pen);
    assert_eq!(active(&s), ["file"]);
    // an unfinished Pen path must not empty the File body
    for p in [pt(20.0, 20.0), pt(80.0, 20.0)] {
        s.pointer_hover(p, false, false);
        s.pointer_down(p, false);
        s.pointer_up(p, false, false);
    }
    assert_eq!(s.panel_content(), curvyo_ui_core::PanelContent::File);
}

#[test]
fn c14_shortcuts_leave_file_and_style_stays_dimmed_without_selection() {
    let mut s = fresh();
    s.press_panel_tab("file");
    s.panel_shortcut_document();
    assert_eq!(active(&s), ["document"]);
    s.press_panel_tab("file");
    s.panel_shortcut_history();
    assert_eq!(active(&s), ["history"]);
    s.press_panel_tab("file");
    s.panel_shortcut_style();
    assert_eq!(
        active(&s),
        ["document"],
        "0043: Style shortcut with nothing in scope shows Document"
    );
    // dimmed Style: pressing it does nothing
    s.press_panel_tab("file");
    assert!(!s.press_panel_tab("style"));
    assert_eq!(active(&s), ["file"]);
    let view = s.panel_view();
    let style = view.tabs.iter().find(|t| t.name == "style").unwrap();
    assert!(!style.enabled);
}

#[test]
fn c14_file_tab_does_not_change_the_dirty_flag() {
    let mut s = fresh();
    edit(&mut s);
    save(&mut s);
    s.press_panel_tab("file");
    s.press_panel_tab("document");
    s.press_panel_tab("history");
    assert!(!s.is_dirty());
}

#[test]
fn c2_hostile_tab_names_are_refused() {
    let mut s = fresh();
    for n in [
        "",
        "File",
        "FILE",
        " file",
        "file\0",
        "Layers",
        "../file",
        "fil\u{e9}",
    ] {
        assert!(!s.press_panel_tab(n), "{n:?}");
    }
    assert_eq!(active(&s), ["document"]);
}

// ---------------------------------------------------------------- reference model (44)

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

#[test]
fn c44_generated_sequences_match_the_reference_model() {
    for seed in 1..=60u64 {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let mut s = fresh();
        let mut model_dirty = false;
        let mut last_saved: Option<Vec<u8>> = None;
        let mut log: Vec<String> = Vec::new();
        for step in 0..120 {
            let op = rng.below(12);
            match op {
                0 | 1 => {
                    edit(&mut s);
                    model_dirty = true;
                    log.push("edit".into());
                }
                2 => {
                    last_saved = Some(save(&mut s));
                    model_dirty = false;
                    log.push("save".into());
                }
                3 => {
                    // an edit made while a save is pending
                    let bytes = s.pack_for_save().unwrap();
                    edit(&mut s);
                    s.mark_saved();
                    last_saved = Some(bytes);
                    model_dirty = true;
                    log.push("save+edit-during".into());
                }
                4 => {
                    // failed write: pack, then nothing
                    let _ = s.pack_for_save().unwrap();
                    log.push("failed-write".into());
                }
                5 => {
                    if s.undo_len() > 0 {
                        undo(&mut s);
                        model_dirty = true;
                    }
                    log.push("undo".into());
                }
                6 => {
                    if s.redo_len() > 0 {
                        key(&mut s, "Z", true, true);
                        model_dirty = true;
                    }
                    log.push("redo".into());
                }
                7 => {
                    if matches!(s.wipe_history(), WipeOutcome::Wiped(_)) {
                        model_dirty = true;
                    }
                    log.push("wipe".into());
                }
                8 => {
                    if let Some(bytes) = &last_saved {
                        s = open(bytes);
                        s.set_clock(clock);
                        s.resize_viewport(1200.0, 800.0);
                        model_dirty = false;
                        log.push("open".into());
                    }
                }
                9 => {
                    s = fresh();
                    model_dirty = false;
                    last_saved = None;
                    log.push("new".into());
                }
                10 => {
                    s.set_tool(Tool::Rectangle);
                    s.set_tool(Tool::Select);
                    s.wheel(0.0, 33.0, 50.0, 50.0, false, false);
                    key(&mut s, "a", true, false);
                    s.escape();
                    log.push("view".into());
                }
                _ => {
                    let w = format!("{}", 100 + rng.below(400));
                    if s.set_document_side(DocumentSide::Width, &w) == SizeOutcome::Committed {
                        model_dirty = true;
                    }
                    log.push("size".into());
                }
            }
            assert_eq!(
                s.is_dirty(),
                model_dirty,
                "seed {seed} step {step}: after {:?}",
                log.iter().rev().take(6).collect::<Vec<_>>()
            );
        }
    }
}

#[test]
fn c33_any_key_that_opens_an_entry_chip_closes_the_chord_gate() {
    let mut opened = 0;
    for tool in [
        Tool::Select,
        Tool::Node,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::PolygonStar,
    ] {
        for c in ('a'..='z').chain('0'..='9') {
            let mut s = fresh();
            edit(&mut s);
            key(&mut s, "a", true, false);
            s.set_tool(tool);
            let k = c.to_string();
            let outcome = key(&mut s, &k, false, false);
            let chip = outcome == KeyOutcome::EntryOpened
                || s.move_entry().is_some()
                || s.transform_entry().is_some();
            if chip {
                opened += 1;
                for (fk, shift) in [
                    ("s", false),
                    ("S", true),
                    ("o", false),
                    ("n", false),
                    ("w", false),
                    ("q", false),
                ] {
                    assert_eq!(
                        s.file_chord(&input(fk, shift), Platform::Linux),
                        None,
                        "chip opened by {k:?} in {tool:?}, Ctrl+{fk} must be ignored"
                    );
                }
            }
        }
    }
    assert!(opened > 0, "no key opened a chip: the test is vacuous");
}

// ---------------------------------------------------------------- the 280 px jump (8)

fn doc_at(s: &Session, x: f64, y: f64) -> Point {
    s.screen_to_document(x, y)
}

#[test]
fn c8_hiding_and_showing_the_body_keeps_the_document_where_it_is_on_screen() {
    for touched in [false, true] {
        let mut s = fresh();
        if touched {
            s.wheel(30.0, 50.0, 400.0, 300.0, false, false);
            s.wheel(0.0, -200.0, 400.0, 300.0, false, true);
        }
        let probes = [(10.0, 10.0), (300.0, 200.0), (700.0, 500.0)];
        let before: Vec<_> = probes.iter().map(|p| doc_at(&s, p.0, p.1)).collect();
        // hide: canvas 280 px wider
        s.keep_view_origin_for_width_change(280.0);
        s.resize_viewport(1200.0 + 280.0, 800.0);
        for (p, b) in probes.iter().zip(&before) {
            let a = doc_at(&s, p.0, p.1);
            assert!(
                (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6,
                "touched={touched} hide {p:?}: {a:?} vs {b:?}"
            );
        }
        // show again
        s.keep_view_origin_for_width_change(-280.0);
        s.resize_viewport(1200.0, 800.0);
        for (p, b) in probes.iter().zip(&before) {
            let a = doc_at(&s, p.0, p.1);
            assert!(
                (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6,
                "touched={touched} show {p:?}: {a:?} vs {b:?}"
            );
        }
        assert!(!s.is_dirty(), "a view change is not a document change");
    }
}

#[test]
fn c8_hostile_width_deltas_are_ignored() {
    let mut s = fresh();
    let before = doc_at(&s, 100.0, 100.0);
    for d in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        s.keep_view_origin_for_width_change(d);
    }
    s.resize_viewport(1200.0, 800.0);
    let after = doc_at(&s, 100.0, 100.0);
    assert!((after.x - before.x).abs() < 1e-6 && (after.y - before.y).abs() < 1e-6);
    // zero, huge and NaN sizes must not panic
    for (w, h) in [
        (0.0, 0.0),
        (-5.0, 10.0),
        (f64::NAN, 100.0),
        (1e12, 1e12),
        (1.0, 1.0),
    ] {
        s.resize_viewport(w, h);
    }
    assert!(!s.is_dirty());
}

// ---------------------------------------------------------------- no-op operations stay clean (22)

#[test]
fn c22_operations_that_change_nothing_do_not_dirty() {
    let mut s = fresh();
    edit(&mut s);
    save(&mut s);
    let check = |s: &Session, what: &str| assert!(!s.is_dirty(), "{what} dirtied a clean project");

    s.commit_style_preview();
    check(&s, "commit_style_preview without a preview");
    s.commit_background_preview();
    check(&s, "commit_background_preview without a preview");
    s.finish_pen();
    check(&s, "finish_pen without a path");
    s.escape();
    check(&s, "escape");
    key(&mut s, "Delete", false, false);
    check(&s, "Delete with nothing selected");
    key(&mut s, "Backspace", false, false);
    check(&s, "Backspace with nothing selected");
    key(&mut s, "g", true, false);
    check(&s, "group with nothing selected");
    key(&mut s, "G", true, true);
    check(&s, "ungroup with nothing selected");
    key(&mut s, "ArrowLeft", false, false);
    check(&s, "nudge with nothing selected");
    key(&mut s, "a", true, false);
    check(&s, "select all");
    assert_eq!(s.selected_object_count(), 1);
    match s.fit_document() {
        curvyo_editor_wasm::FitOutcome::Fitted => {
            assert!(
                s.is_dirty(),
                "Fitted rewrote the document size: it is a change"
            );
        }
        _ => check(&s, "a fit that wrote nothing"),
    }
}

#[test]
fn c22_fit_document_on_an_empty_document_is_not_a_change() {
    let mut s = fresh();
    assert!(matches!(
        s.fit_document(),
        curvyo_editor_wasm::FitOutcome::Empty
    ));
    assert!(!s.is_dirty());
}

#[test]
fn c22_loading_the_formats_file_is_not_a_document_change() {
    let mut s = fresh();
    edit(&mut s);
    save(&mut s);
    let _ = s.load_formats("");
    s.mark_formats_broken("unreadable");
    assert!(
        !s.is_dirty(),
        "the maker's formats library is not part of the project"
    );
    let mut o = open(&save(&mut s));
    let _ = o.load_formats("");
    assert!(!o.is_dirty());
}

#[test]
fn c22_repeated_identical_entries_do_not_dirty() {
    let mut s = fresh();
    assert_eq!(
        s.set_document_side(DocumentSide::Width, "250"),
        SizeOutcome::Committed
    );
    save(&mut s);
    for _ in 0..5 {
        assert_eq!(
            s.set_document_side(DocumentSide::Width, "250"),
            SizeOutcome::Unchanged
        );
    }
    assert!(!s.is_dirty());
}
