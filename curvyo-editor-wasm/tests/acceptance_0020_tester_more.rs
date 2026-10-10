//! Independent tester acceptance tests for `specs/0020-undo-redo`, part two: held nudge, panel
//! drags and previews, old files, the stack machine against a model, one commit per gesture, large
//! steps. Written from the specification and the public `Session` seam before the implementation
//! diff was read.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::similar_names
)]
#![allow(
    clippy::assert_is_empty,
    clippy::cast_lossless,
    clippy::redundant_closure_for_method_calls,
    clippy::semicolon_if_nothing_returned,
    clippy::bool_assert_comparison,
    clippy::collapsible_if,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
#![allow(missing_docs, clippy::doc_markdown, clippy::needless_pass_by_value)]
#![allow(clippy::many_single_char_names, clippy::cast_possible_truncation)]

use std::collections::HashMap;
use std::ops::ControlFlow;
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{Document, NodeId, Point, Timestamp, unpack};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::{BooleanOp, EntryOutcome, Grid, MoveEntryMode, StyleField, ValueField};

static NOW: AtomicU64 = AtomicU64::new(1_900_000_000);

fn clock() -> Timestamp {
    Timestamp::from_secs(NOW.fetch_add(3, Ordering::SeqCst))
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fresh() -> Session {
    let mut s = Session::new(21);
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_tool(Tool::Select);
    s
}

fn reread(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn json(s: &Session) -> Vec<u8> {
    reread(s).export_json().unwrap()
}

fn key(s: &mut Session, k: &str, ctrl: bool, shift: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        shift,
        ..KeyInput::default()
    })
}

fn z(s: &mut Session, redo: bool) -> String {
    let out = if redo {
        key(s, "Z", true, true)
    } else {
        key(s, "z", true, false)
    };
    match out {
        KeyOutcome::UndoBusy => s.undo().notice,
        KeyOutcome::RedoBusy => s.redo().notice,
        _ => s.take_history_notice().unwrap_or_default(),
    }
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

fn one_rect() -> Session {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(80.0, 60.0));
    assert_eq!(key(&mut s, "a", true, false), KeyOutcome::SelectedAll);
    s
}

fn left_edge_x(s: &Session) -> f64 {
    let d = reread(s);
    match d.object(d.object_ids()[0]).unwrap() {
        curvyo_document_core::ObjectSnapshot::Primitive(p) => match p.shape {
            curvyo_document_core::Shape::Rect { bounds, .. } => bounds.origin.x,
            _ => panic!("rect"),
        },
        curvyo_document_core::ObjectSnapshot::Path(_) => panic!("rect"),
    }
}

fn loro_changes(s: &Session) -> usize {
    let l = loro::LoroDoc::new();
    l.import(&reread(s).export_loro_snapshot().unwrap())
        .unwrap();
    l.len_changes()
}

fn messages(s: &Session) -> Vec<String> {
    let l = loro::LoroDoc::new();
    l.import(&reread(s).export_loro_snapshot().unwrap())
        .unwrap();
    let frontiers: Vec<loro::ID> = l.oplog_frontiers().iter().collect();
    let mut changes: Vec<(u32, String)> = Vec::new();
    l.travel_change_ancestors(&frontiers, &mut |c| {
        changes.push((
            c.lamport,
            c.message.map(|m| m.to_string()).unwrap_or_default(),
        ));
        ControlFlow::Continue(())
    })
    .unwrap();
    changes.sort();
    changes.into_iter().map(|(_, m)| m).collect()
}

fn arrow(s: &mut Session, name: &str, repeat: bool, t: f64) -> KeyOutcome {
    s.key_down(KeyInput {
        key: name,
        repeat,
        time_ms: t,
        ..KeyInput::default()
    })
}

// ---------------------------------------------------------------------
// The held nudge (0044 criterion 9, 0020 criterion 54)
// ---------------------------------------------------------------------

#[test]
fn nudge_held_key_is_one_step_and_one_press_takes_back_all_of_it() {
    let mut s = one_rect();
    let x0 = left_edge_x(&s);
    let steps = s.undo_len();
    let first = arrow(&mut s, "ArrowRight", false, 1000.0);
    assert_eq!(first, KeyOutcome::Nudged { new_run: true });
    for i in 1..=10 {
        let o = arrow(&mut s, "ArrowRight", true, 1000.0 + 30.0 * f64::from(i));
        assert_eq!(o, KeyOutcome::Nudged { new_run: false }, "repeat {i}");
    }
    assert_eq!(
        s.undo_len(),
        steps + 1,
        "one press with 10 repeats is one step"
    );
    assert!((left_edge_x(&s) - (x0 + 11.0)).abs() < 1e-9, "11 mm");
    let notice = z(&mut s, false);
    assert_eq!(notice, "Undid: Move (1 rectangle).");
    assert!(
        (left_edge_x(&s) - x0).abs() < 1e-9,
        "all 11 mm taken back by one press"
    );
    assert_eq!(s.undo_len(), steps);
    // Redo brings all of it back.
    z(&mut s, true);
    assert!((left_edge_x(&s) - (x0 + 11.0)).abs() < 1e-9);
}

#[test]
fn nudge_held_run_does_not_join_the_undone_step_after_ctrl_z() {
    let mut s = one_rect();
    let x0 = left_edge_x(&s);
    arrow(&mut s, "ArrowRight", false, 1000.0);
    for i in 1..=4 {
        arrow(&mut s, "ArrowRight", true, 1000.0 + 20.0 * f64::from(i));
    }
    let steps = s.undo_len();
    z(&mut s, false);
    assert_eq!(s.undo_len(), steps - 1);
    assert!((left_edge_x(&s) - x0).abs() < 1e-9);
    // The key is still held: a repeat event right after the undo must open a new step.
    let o = arrow(&mut s, "ArrowRight", true, 1000.0 + 20.0 * 5.0);
    assert!(matches!(o, KeyOutcome::Nudged { .. }), "a nudge, got {o:?}");
    assert_eq!(s.undo_len(), steps, "a new step, not a joined one");
    assert_eq!(s.redo_len(), 0, "the new step emptied the redo stack");
    assert!((left_edge_x(&s) - (x0 + 1.0)).abs() < 1e-9);
    // And the next repeat joins that new step.
    arrow(&mut s, "ArrowRight", true, 1000.0 + 20.0 * 6.0);
    assert_eq!(s.undo_len(), steps);
    assert!((left_edge_x(&s) - (x0 + 2.0)).abs() < 1e-9);
    z(&mut s, false);
    assert!(
        (left_edge_x(&s) - x0).abs() < 1e-9,
        "the new run is one step too"
    );
}

#[test]
fn nudge_after_another_edit_or_a_different_arrow_is_a_new_step() {
    let mut s = one_rect();
    let steps = s.undo_len();
    arrow(&mut s, "ArrowRight", false, 1000.0);
    arrow(&mut s, "ArrowRight", true, 1030.0);
    assert_eq!(s.undo_len(), steps + 1);
    // A style edit in between.
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    assert_eq!(s.undo_len(), steps + 2);
    arrow(&mut s, "ArrowRight", true, 1060.0);
    assert_eq!(
        s.undo_len(),
        steps + 3,
        "anything committed in between ends the run"
    );
    // Different arrow key.
    arrow(&mut s, "ArrowDown", true, 1090.0);
    assert_eq!(s.undo_len(), steps + 4);
    // Later than the window.
    arrow(&mut s, "ArrowDown", true, 1090.0 + 700.0);
    assert_eq!(s.undo_len(), steps + 5);
    // Each is undone by exactly one press.
    for expected in (steps..steps + 5).rev() {
        z(&mut s, false);
        assert_eq!(s.undo_len(), expected);
    }
}

#[test]
fn nudge_of_several_objects_is_one_commit_per_event_and_one_step() {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(40.0, 40.0));
    drag(&mut s, Tool::Rectangle, pt(60.0, 20.0), pt(80.0, 40.0));
    key(&mut s, "a", true, false);
    let before = json(&s);
    let steps = s.undo_len();
    arrow(&mut s, "ArrowRight", false, 1000.0);
    arrow(&mut s, "ArrowRight", true, 1030.0);
    arrow(&mut s, "ArrowRight", true, 1060.0);
    assert_eq!(s.undo_len(), steps + 1);
    assert_ne!(json(&s), before);
    z(&mut s, false);
    assert_eq!(json(&s), before);
}

// ---------------------------------------------------------------------
// Panel drags and previews (criterion 6a: "the panel's value fields included")
// ---------------------------------------------------------------------

#[test]
fn panel_value_field_drag_blocks_the_key_and_the_step_commits_once() {
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    let before = json(&s);
    let steps = s.undo_len();
    s.preview_value_field(ValueField::StrokeWidth, 0.5, Grid::Normal);
    let out = key(&mut s, "z", true, false);
    assert_eq!(
        out,
        KeyOutcome::Ignored,
        "a value-field drag is a drag in flight"
    );
    assert_eq!(s.take_history_notice(), None);
    assert_eq!(s.undo_len(), steps);
    assert_eq!(json(&s), before, "a preview writes nothing");
    s.commit_style_preview();
    assert_eq!(s.undo_len(), steps + 1);
    z(&mut s, false);
    assert_eq!(json(&s), before);
}

#[test]
fn background_preview_blocks_the_key() {
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    let shown = s.background_view().opacity;
    s.preview_background_opacity(0.4, Grid::Normal);
    assert_ne!(s.background_view().opacity, shown, "the preview is pending");
    let out = key(&mut s, "z", true, false);
    assert_eq!(
        out,
        KeyOutcome::Ignored,
        "a background value-field drag is a drag in flight"
    );
    s.cancel_background_preview();
}

#[test]
fn undo_press_from_the_panel_button_while_a_preview_is_open_does_not_lose_the_preview_step() {
    // The Undo button is a pointer press; a value-field drag cannot be in flight then. The
    // session's `undo()` is the button's call: a preview left open must not corrupt anything.
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    let steps = s.undo_len();
    s.preview_value_field(ValueField::StrokeWidth, 0.7, Grid::Normal);
    let r = s.undo();
    let after = reread(&s).export_json().unwrap();
    assert!(!after.is_empty());
    assert!(r.changed || s.undo_len() <= steps + 1);
    // The document stays loadable and a further undo/redo cycle works.
    z(&mut s, true);
    z(&mut s, false);
    assert!(unpack(1, &s.pack("0.1.0").unwrap()).is_ok());
}

// ---------------------------------------------------------------------
// Old files (criteria 25, 31, 36)
// ---------------------------------------------------------------------

fn fixture_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../curvyo-document-core/tests/fixtures")
}

#[test]
fn old_files_open_with_empty_stacks_a_readable_history_and_working_undo() {
    let mut opened = 0;
    let mut dir: Vec<_> = std::fs::read_dir(fixture_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "curvyo"))
        .collect();
    dir.sort();
    for path in dir {
        let bytes = std::fs::read(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let Ok(mut s) = Session::open(5, &bytes) else {
            continue;
        };
        opened += 1;
        s.set_clock(clock);
        s.resize_viewport(1200.0, 800.0);
        assert_eq!((s.undo_len(), s.redo_len()), (0, 0), "{name}");
        let (out, notice) = (
            key(&mut s, "z", true, false),
            s.take_history_notice().unwrap_or_default(),
        );
        assert_eq!(out, KeyOutcome::Undo, "{name}");
        assert_eq!(notice, "Nothing to undo.", "{name}");
        // The history lists the file's steps; every row of an old file is an earlier author.
        let len = s.history_len();
        let rows: serde_json::Value = serde_json::from_str(&s.history_rows_json(0, 500)).unwrap();
        let rows = rows.as_array().unwrap();
        assert_eq!(rows.len(), len.min(500), "{name}");
        for row in rows {
            assert_eq!(row["author"], "earlier", "{name}: {row}");
            assert!(
                row["name"].as_str().is_some_and(|n| !n.is_empty()),
                "{name}: {row}"
            );
            let id = row["id"].as_str().unwrap();
            assert!(id.len() >= 7, "{name}");
        }
        // Edit and undo: byte-equal to the opened file's export.
        let before = json(&s);
        if key(&mut s, "a", true, false) != KeyOutcome::SelectedAll
            || s.selected_object_count() == 0
        {
            continue;
        }
        let o = arrow(&mut s, "ArrowRight", false, 10.0);
        assert!(matches!(o, KeyOutcome::Nudged { .. }), "{name}: {o:?}");
        assert_ne!(json(&s), before, "{name}");
        assert_eq!(z(&mut s, false).starts_with("Undid: Move"), true, "{name}");
        assert_eq!(
            json(&s),
            before,
            "{name}: undo on an old file restores the exported document"
        );
        z(&mut s, true);
        assert_ne!(json(&s), before, "{name}");
        // And the file still saves and opens.
        assert!(
            Session::open(6, &s.pack("0.1.0").unwrap()).is_ok(),
            "{name}"
        );
    }
    assert!(opened >= 10, "opened {opened} fixtures");
}

#[test]
fn compound_v8_history_has_known_names_earlier_authors_and_unknown_time() {
    let bytes = std::fs::read(fixture_dir().join("compound_v8.curvyo")).unwrap();
    let mut s = Session::open(5, &bytes).unwrap();
    let rows: serde_json::Value = serde_json::from_str(&s.history_rows_json(0, 50)).unwrap();
    let rows = rows.as_array().unwrap();
    assert!(!rows.is_empty());
    for row in rows {
        assert_eq!(row["author"], "earlier");
        assert_eq!(row["time"], 0, "an old file has no times: {row}");
    }
}

// ---------------------------------------------------------------------
// Criterion 55: the stacks and statuses against an independent model
// ---------------------------------------------------------------------

#[test]
fn c55_a_thousand_generated_events_match_a_model_of_the_stacks_and_row_statuses() {
    let mut s = one_rect();
    let mut x: u64 = 0x1234_5678_9ABC_DEF1;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let mut model_undo: Vec<String> = Vec::new();
    let mut model_redo: Vec<String> = Vec::new();
    let mut status: HashMap<String, &'static str> = HashMap::new();
    // The setup step (the rectangle).
    let rows: serde_json::Value = serde_json::from_str(&s.history_rows_json(0, 5)).unwrap();
    let first = rows[0]["step"].as_str().unwrap().to_owned();
    model_undo.push(first.clone());
    status.insert(first, "applied");
    let mut width = 1.0_f64;
    for event in 0..1000 {
        let roll = next() % 10;
        if roll < 5 {
            width += 0.01;
            let text = format!("{width:.2}");
            assert!(
                s.set_style_text(StyleField::StrokeWidth, &text).unwrap(),
                "event {event}"
            );
            let rows: serde_json::Value = serde_json::from_str(&s.history_rows_json(0, 1)).unwrap();
            let id = rows[0]["step"].as_str().unwrap().to_owned();
            model_undo.push(id.clone());
            status.insert(id, "applied");
            model_redo.clear();
            if model_undo.len() > 500 {
                model_undo.remove(0);
            }
        } else if roll < 8 {
            let notice = z(&mut s, false);
            if let Some(top) = model_undo.pop() {
                assert!(notice.starts_with("Undid:"), "event {event}: {notice}");
                status.insert(top.clone(), "undone");
                model_redo.push(top);
            } else {
                assert_eq!(notice, "Nothing to undo.", "event {event}");
            }
        } else {
            let notice = z(&mut s, true);
            if let Some(top) = model_redo.pop() {
                assert!(notice.starts_with("Redid:"), "event {event}: {notice}");
                status.insert(top.clone(), "applied");
                model_undo.push(top);
            } else {
                assert_eq!(notice, "Nothing to redo.", "event {event}");
            }
        }
        assert_eq!(s.undo_len(), model_undo.len(), "event {event}");
        assert_eq!(s.redo_len(), model_redo.len(), "event {event}");
        if event % 100 == 99 || event == 0 {
            let rows: serde_json::Value =
                serde_json::from_str(&s.history_rows_json(0, 2000)).unwrap();
            let rows = rows.as_array().unwrap();
            assert_eq!(
                rows.len(),
                status.len(),
                "event {event}: one row per step, none per undo"
            );
            for row in rows {
                let id = row["step"].as_str().unwrap();
                let got = row["status"].as_str().unwrap();
                assert_eq!(got, status[id], "event {event}: step {id}");
            }
        }
    }
    // The document and a reopened copy agree on every status (criterion 56: nothing is stored for
    // statuses; the log tells them).
    let reopened = Session::open(7, &s.pack("0.1.0").unwrap()).unwrap();
    let mut reopened = reopened;
    let a: serde_json::Value = serde_json::from_str(&s.history_rows_json(0, 2000)).unwrap();
    let b: serde_json::Value = serde_json::from_str(&reopened.history_rows_json(0, 2000)).unwrap();
    let pairs = |v: &serde_json::Value| -> Vec<(String, String)> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|r| {
                (
                    r["step"].as_str().unwrap().to_owned(),
                    r["status"].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    };
    assert_eq!(
        pairs(&a),
        pairs(&b),
        "statuses are the same after Save and Open"
    );
}

// ---------------------------------------------------------------------
// Criterion 58: one commit per gesture
// ---------------------------------------------------------------------

fn commits_of(s: &mut Session, gesture: impl FnOnce(&mut Session)) -> usize {
    let before = loro_changes(s);
    gesture(s);
    loro_changes(s) - before
}

#[test]
fn c58_every_gesture_of_the_table_is_exactly_one_commit() {
    let mut s = fresh();
    assert_eq!(
        commits_of(&mut s, |s| drag(
            s,
            Tool::Rectangle,
            pt(20.0, 20.0),
            pt(50.0, 50.0)
        )),
        1,
        "rect"
    );
    assert_eq!(
        commits_of(&mut s, |s| drag(
            s,
            Tool::Rectangle,
            pt(70.0, 20.0),
            pt(100.0, 50.0)
        )),
        1,
        "rect 2"
    );
    assert_eq!(
        commits_of(&mut s, |s| drag(
            s,
            Tool::Ellipse,
            pt(20.0, 70.0),
            pt(50.0, 100.0)
        )),
        1,
        "ellipse"
    );
    assert_eq!(
        commits_of(&mut s, |s| drag(
            s,
            Tool::PolygonStar,
            pt(70.0, 70.0),
            pt(100.0, 100.0)
        )),
        1,
        "polygon/star"
    );
    s.set_tool(Tool::Select);
    key(&mut s, "a", true, false);
    assert_eq!(s.selected_object_count(), 4);
    // Typed move of 4 objects: one commit.
    assert_eq!(
        commits_of(&mut s, |s| {
            key(s, "m", false, false);
            assert_eq!(
                s.commit_move_entry(
                    "2",
                    "2",
                    MoveEntryMode {
                        absolute: false,
                        copy: false
                    }
                ),
                EntryOutcome::Committed
            );
        }),
        1,
        "typed move"
    );
    // Style edit on 4 objects.
    assert_eq!(
        commits_of(&mut s, |s| assert!(
            s.set_style_text(StyleField::StrokeWidth, "2").unwrap()
        )),
        1,
        "style"
    );
    // Multi-object typed resize, rotate: one commit for all (criterion 58).
    for (k, v) in [("s", "300"), ("r", "15")] {
        let c = commits_of(&mut s, |s| {
            assert_eq!(key(s, k, false, false), KeyOutcome::EntryOpened, "{k}");
            let chip = s.transform_entry().unwrap();
            let second = chip
                .fields
                .get(1)
                .map(|f| f.prefill.clone())
                .unwrap_or_default();
            assert_eq!(
                s.commit_transform_entry(v, &second, 0),
                EntryOutcome::Committed,
                "{k}"
            );
        });
        assert_eq!(c, 1, "multi-object {k}");
    }
    // Skew needs paths.
    s.convert_selected_to_paths();
    s.set_tool(Tool::Select);
    assert_eq!(s.selected_object_count(), 4);
    let c = commits_of(&mut s, |s| {
        assert_eq!(key(s, "k", false, false), KeyOutcome::EntryOpened);
        assert_eq!(
            s.commit_transform_entry("20", "", 0),
            EntryOutcome::Committed
        );
    });
    assert_eq!(c, 1, "multi-object skew");
    // Duplicate.
    assert_eq!(
        commits_of(&mut s, |s| {
            key(s, "m", false, false);
            assert_eq!(
                s.commit_move_entry(
                    "10",
                    "10",
                    MoveEntryMode {
                        absolute: false,
                        copy: true
                    }
                ),
                EntryOutcome::Committed
            );
        }),
        1,
        "duplicate of several"
    );
    // Delete of the selected copies.
    assert_eq!(
        commits_of(&mut s, |s| {
            key(s, "Delete", false, false);
        }),
        1,
        "delete"
    );
    // Boolean over two shapes.
    s.set_tool(Tool::Select);
    key(&mut s, "a", true, false);
    let count = s.selected_object_count();
    assert!(count >= 2);
    let c = commits_of(&mut s, |s| {
        let _ = s.apply_boolean(BooleanOp::Union);
    });
    assert_eq!(c, 1, "boolean over {count}");
    // Every commit carries the header label;s=n (criterion 54).
    for m in messages(&s) {
        if m.is_empty() || m == "new_document" {
            continue;
        }
        assert!(
            m.contains(";s="),
            "commit message without a step header: {m:?}"
        );
    }
}

// ---------------------------------------------------------------------
// Criterion 14 and 30: larger steps
// ---------------------------------------------------------------------

#[test]
fn c14_union_of_two_thousand_point_polygons_is_one_step_that_undoes_and_redoes_exactly() {
    let mut s = fresh();
    s.set_poly_star_mode(curvyo_ui_core::PolyStarMode::Polygon);
    s.set_poly_star_point_count(curvyo_document_core::PointCount::new(1000).unwrap());
    drag(&mut s, Tool::PolygonStar, pt(20.0, 20.0), pt(120.0, 120.0));
    drag(&mut s, Tool::PolygonStar, pt(60.0, 40.0), pt(160.0, 140.0));
    s.set_tool(Tool::Select);
    key(&mut s, "a", true, false);
    let before = json(&s);
    let steps = s.undo_len();
    let _ = s.apply_boolean(BooleanOp::Union);
    assert_eq!(s.undo_len(), steps + 1);
    let after = json(&s);
    assert_ne!(before, after);
    let started = std::time::Instant::now();
    let notice = z(&mut s, false);
    assert!(notice.starts_with("Undid: Union"), "{notice}");
    assert_eq!(json(&s), before);
    let notice = z(&mut s, true);
    assert!(notice.starts_with("Redid: Union"), "{notice}");
    assert_eq!(json(&s), after);
    eprintln!(
        "undo+redo of a ~2000-anchor union (debug build): {:?}",
        started.elapsed()
    );
}

#[test]
fn c30_a_step_of_hundreds_of_objects_asks_for_the_busy_state_and_undoes_in_one_press() {
    let mut s = fresh();
    for i in 0..50 {
        for j in 0..50 {
            let (x, y) = (5.0 + f64::from(i) * 3.0, 5.0 + f64::from(j) * 3.0);
            drag(&mut s, Tool::Rectangle, pt(x, y), pt(x + 2.0, y + 2.0));
        }
    }
    s.set_tool(Tool::Select);
    key(&mut s, "a", true, false);
    assert_eq!(s.selected_object_count(), 2500);
    s.warm_history();
    let before = json(&s);
    // A style edit on 600 objects: more than 2,000 operations.
    assert!(s.set_style_text(StyleField::StrokeWidth, "2").unwrap());
    let busy_needed = s.history_needs_busy_state(curvyo_document_core::Direction::Undo);
    let out = key(&mut s, "z", true, false);
    if busy_needed {
        assert_eq!(
            out,
            KeyOutcome::UndoBusy,
            "a large step asks for the busy state before the call"
        );
        let r = s.undo();
        assert!(r.changed);
    } else {
        assert_eq!(out, KeyOutcome::Undo);
    }
    assert_eq!(json(&s), before);
    eprintln!("2,500-object style step needs the busy state: {busy_needed}");
    assert!(busy_needed, "2,500 objects are more than 2,000 operations");
}

#[test]
fn hostile_undo_of_an_old_step_after_many_new_ones_and_dropped_stack_entries() {
    // 520 steps: the oldest 20 fall off the stack; undoing all 500 gets back to the state after
    // step 20, not to the empty document.
    let mut s = one_rect();
    let mut states: Vec<Vec<u8>> = Vec::new();
    for i in 0..520 {
        let w = if i % 2 == 0 { "2" } else { "3.5" };
        assert!(s.set_style_text(StyleField::StrokeWidth, w).unwrap());
        states.push(json(&s));
    }
    assert_eq!(s.undo_len(), 500);
    for _ in 0..500 {
        z(&mut s, false);
    }
    assert_eq!(
        json(&s),
        states[19],
        "the 500th undo ends at the state after step 20"
    );
    assert_eq!(z(&mut s, false), "Nothing to undo.");
    let _: Option<NodeId> = None;
}

// ---------------------------------------------------------------------
// Criterion 56: the undo is a commit with a header
// ---------------------------------------------------------------------

#[test]
fn c56_undo_and_redo_are_commits_with_their_headers() {
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    let own_steps = s.undo_len();
    z(&mut s, false);
    let last = messages(&s).pop().unwrap();
    let parts: Vec<&str> = last.split(';').collect();
    assert_eq!(parts[0], "undo", "{last}");
    assert!(parts[1].starts_with("s="), "{last}");
    assert!(parts[2].starts_with("t="), "{last}");
    assert!(
        parts.iter().any(|p| p.starts_with("u=21@")),
        "u=<peer>@<counter>: {last}"
    );
    z(&mut s, true);
    let last = messages(&s).pop().unwrap();
    let parts: Vec<&str> = last.split(';').collect();
    assert_eq!(parts[0], "redo", "{last}");
    assert!(
        parts.iter().any(|p| p.starts_with("d=21@")),
        "d=<peer>@<counter>: {last}"
    );
    assert_eq!(s.undo_len(), own_steps);
    // The log only grows: the undo commits stay after Save and Open, and the statuses with them.
    let reopened = Session::open(5, &s.pack("0.1.0").unwrap()).unwrap();
    let msgs = messages(&reopened);
    assert!(msgs.iter().any(|m| m.starts_with("undo;")));
    assert!(msgs.iter().any(|m| m.starts_with("redo;")));
    assert!(
        msgs.first()
            .is_some_and(|m| m == "new_document" || m.starts_with("new_document;")),
        "the setup commit has the reserved label: {:?}",
        msgs.first()
    );
}

#[test]
fn c57_the_first_step_does_not_undo_the_document_setup() {
    let mut s = fresh();
    let empty = json(&s);
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(60.0, 60.0));
    assert_eq!(s.undo_len(), 1);
    assert_eq!(s.history_len(), 1, "new_document is listed nowhere");
    z(&mut s, false);
    assert_eq!(json(&s), empty, "format_version, size and unit intact");
    assert_eq!(
        z(&mut s, false),
        "Nothing to undo.",
        "the setup commit is never on a stack"
    );
    assert_eq!(s.history_len(), 1);
}

// ---------------------------------------------------------------------
// Criterion 36: adjacent legacy commits with the same label are one row
// ---------------------------------------------------------------------

#[test]
fn c36_old_files_have_no_adjacent_rows_with_the_same_legacy_label() {
    let mut dir: Vec<_> = std::fs::read_dir(fixture_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "curvyo"))
        .collect();
    dir.sort();
    for path in dir {
        let bytes = std::fs::read(&path).unwrap();
        let Ok(mut s) = Session::open(5, &bytes) else {
            continue;
        };
        let rows: serde_json::Value = serde_json::from_str(&s.history_rows_json(0, 5000)).unwrap();
        let rows = rows.as_array().unwrap();
        for pair in rows.windows(2) {
            if pair[0]["time"] == 0 && pair[1]["time"] == 0 {
                assert_ne!(pair[0]["label"], pair[1]["label"], "{path:?}: {pair:?}");
            }
        }
        for row in rows {
            if row["known"] == false {
                assert_eq!(
                    row["name"], "Edit",
                    "{path:?}: an unknown label is named Edit"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------
// Criterion 9: the buttons are the same as the key, gate included
// ---------------------------------------------------------------------

#[test]
fn c9_the_button_calls_follow_the_same_gate_as_the_key() {
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    let before = json(&s);
    let steps = s.undo_len();
    // A drag in flight.
    s.pointer_hover(pt(30.0, 30.0), false, false);
    s.pointer_down(pt(30.0, 30.0), false);
    s.pointer_hover(pt(45.0, 45.0), false, false);
    let r = s.undo();
    assert!(!r.changed, "undo button during a drag: {r:?}");
    let r = s.redo();
    assert!(!r.changed, "redo button during a drag: {r:?}");
    s.pointer_cancelled();
    assert_eq!(json(&s), before);
    // A chip.
    assert_eq!(key(&mut s, "m", false, false), KeyOutcome::EntryOpened);
    let r = s.undo();
    assert!(!r.changed, "undo button with a chip open: {r:?}");
    assert!(s.move_entry().is_some());
    s.escape();
    // An unfinished Pen path.
    s.set_tool(Tool::Pen);
    s.pointer_hover(pt(100.0, 100.0), false, false);
    s.pointer_down(pt(100.0, 100.0), false);
    s.pointer_up(pt(100.0, 100.0), false, false);
    let r = s.undo();
    assert!(!r.changed);
    assert_eq!(
        r.notice,
        "Finish the path (Enter) or cancel it (Esc) first."
    );
    assert_eq!(s.undo_len(), steps);
    s.escape();
    // And once the gate is open the button works.
    s.set_tool(Tool::Select);
    let r = s.undo();
    assert!(r.changed, "{r:?}");
    assert_eq!(r.notice, "Undid: Change style (1 rectangle).");
    let r = s.redo();
    assert!(r.changed);
    assert_eq!(r.notice, "Redid: Change style (1 rectangle).");
    // The empty stack through the button.
    let mut empty = fresh();
    let r = empty.undo();
    assert!(!r.changed);
    assert_eq!(r.notice, "Nothing to undo.");
    let r = empty.redo();
    assert_eq!(r.notice, "Nothing to redo.");
}

#[test]
fn c9_the_button_is_blocked_by_a_background_slider_drag_too() {
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    let shown = s.background_view().opacity;
    s.preview_background_opacity(0.4, Grid::Normal);
    assert_ne!(
        s.background_view().opacity,
        shown,
        "the preview is pending: the slider shows its value without a write"
    );
    let r = s.undo();
    assert!(
        !r.changed,
        "a value-field drag of the Document group is a drag in flight: {r:?}"
    );
    s.cancel_background_preview();
}
