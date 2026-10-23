//! Independent tester tests for `specs/0046-document-shapes`, stage 1, at the editor session:
//! the Guide button state and press, selection and hit rules, Fit to content, the Template
//! load (errors, limits, placement, selection, undo), the marking style in the draw list, the
//! history preview, Boolean/Combine/Split on guides, copy and determinism. Written from the
//! specification; only public names were looked up.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(clippy::too_many_lines, clippy::cast_precision_loss)]
#![allow(missing_docs, clippy::doc_markdown, clippy::needless_pass_by_value)]
#![allow(
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::assert_is_empty
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::items_after_statements
)]

mod cpu_time;

use cpu_time::cpu_time_of;
use curvyo_document_core::{
    AnchorId, Direction, Document, EllipseFrame, Length, NewAnchor, NodeId, ObjectSnapshot, Point,
    RectBounds, pack, unpack,
};
use curvyo_editor_wasm::{
    BooleanOutcome, CombineOutcome, FitOutcome, KeyInput, KeyOutcome, Session, Tool,
};
use curvyo_ui_core::BooleanOp;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn rect(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds {
        origin: pt(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

fn open(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.resize_viewport(1600.0, 1000.0);
    s
}

fn doc_of(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn click(s: &mut Session, p: Point, shift: bool) {
    s.pointer_hover(p, shift, false);
    s.pointer_down(p, shift);
    s.pointer_up(p, shift, false);
}

fn marquee(s: &mut Session, a: Point, b: Point) {
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
}

fn guides(s: &Session) -> Vec<NodeId> {
    doc_of(s)
        .read_nodes()
        .iter()
        .filter(|n| n.guide)
        .map(|n| n.id)
        .collect()
}

fn undo_name(s: &mut Session) -> Option<String> {
    s.history_target_name(Direction::Undo)
}

fn key(s: &mut Session, k: &str, ctrl: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        ..KeyInput::default()
    })
}

/// A valid .curvyo with the given rectangles (all visible, no guides).
fn source_bytes(rects: &[(f64, f64, f64, f64)]) -> Vec<u8> {
    let d = Document::new(7);
    for &(x, y, w, h) in rects {
        rect(&d, x, y, w, h);
    }
    pack(&d, "t").unwrap()
}

fn json(s: &Session) -> String {
    String::from_utf8(doc_of(s).export_json().unwrap()).unwrap()
}

// ---- Guide button (9, 10) ----------------------------------------------------------------------

#[test]
fn c9_button_is_disabled_with_nothing_selected_and_with_a_non_select_tool() {
    let d = Document::new(1);
    rect(&d, 10.0, 10.0, 40.0, 40.0);
    let mut s = open(&d);
    assert!(!s.guide_marking().enabled(), "nothing selected");
    assert!(!s.toggle_guide());
    click(&mut s, pt(10.0, 30.0), false);
    assert!(s.guide_marking().enabled());
    s.set_tool(Tool::Node);
    assert!(!s.guide_marking().enabled(), "greyed in other tools");
}

#[test]
fn c9_press_marks_in_one_step_named_guide_selection_stays_and_second_press_unmarks() {
    let d = Document::new(1);
    rect(&d, 10.0, 10.0, 40.0, 40.0);
    rect(&d, 100.0, 10.0, 40.0, 40.0);
    let mut s = open(&d);
    marquee(&mut s, pt(-5.0, -5.0), pt(200.0, 100.0));
    assert_eq!(s.selected_object_count(), 2);
    let undo_before = s.undo_len();
    assert!(s.toggle_guide());
    assert_eq!(s.undo_len(), undo_before + 1, "one undo step");
    assert_eq!(undo_name(&mut s).as_deref(), Some("Guide"));
    assert_eq!(s.selected_object_count(), 2, "the selection stays");
    assert_eq!(guides(&s).len(), 2);
    assert!(s.guide_marking().all_guides(), "pressed");
    // second press turns them back, again one step
    assert!(s.toggle_guide());
    assert!(guides(&s).is_empty());
    assert_eq!(s.undo_len(), undo_before + 2);
    assert!(!json(&s).contains("\"guide\""), "unmarking leaves no key");
    // undo / redo
    s.undo();
    assert_eq!(guides(&s).len(), 2);
    s.redo();
    assert!(guides(&s).is_empty());
}

#[test]
fn c10_mixed_selection_is_not_pressed_and_a_press_marks_all() {
    let d = Document::new(1);
    rect(&d, 10.0, 10.0, 40.0, 40.0);
    rect(&d, 100.0, 10.0, 40.0, 40.0);
    let mut s = open(&d);
    click(&mut s, pt(10.0, 30.0), false);
    s.toggle_guide();
    marquee(&mut s, pt(-5.0, -5.0), pt(200.0, 100.0));
    let m = s.guide_marking();
    assert!(m.enabled() && !m.all_guides());
    assert!(s.toggle_guide());
    assert_eq!(guides(&s).len(), 2);
}

#[test]
fn c9_group_selection_marks_its_paths() {
    let d = Document::new(1);
    let a = rect(&d, 10.0, 10.0, 40.0, 40.0);
    let b = rect(&d, 100.0, 10.0, 40.0, 40.0);
    d.group_objects(&[a, b]).unwrap();
    let mut s = open(&d);
    click(&mut s, pt(10.0, 30.0), false);
    assert!(s.guide_marking().enabled());
    assert!(s.toggle_guide());
    assert_eq!(guides(&s).len(), 2);
}

// ---- hit rules (7) -------------------------------------------------------------------------------

#[test]
fn c7_click_on_the_line_selects_a_guide_click_inside_does_not() {
    let d = Document::new(1);
    let r = rect(&d, 50.0, 50.0, 100.0, 100.0);
    d.edit_style(&[r], &curvyo_document_core::StyleEdit::FillEnabled(true))
        .unwrap();
    let mut s = open(&d);
    click(&mut s, pt(100.0, 100.0), false);
    assert_eq!(
        s.selected_object_count(),
        1,
        "filled rectangle: inside selects"
    );
    s.toggle_guide();
    click(&mut s, pt(400.0, 400.0), false);
    assert_eq!(s.selected_object_count(), 0);
    click(&mut s, pt(100.0, 100.0), false);
    assert_eq!(
        s.selected_object_count(),
        0,
        "inside a closed guide does not select"
    );
    click(&mut s, pt(50.0, 100.0), false);
    assert_eq!(s.selected_object_count(), 1, "the line is the hit area");
}

#[test]
fn c7_marquee_and_select_all_include_guides_and_delete_and_move_work() {
    let d = Document::new(1);
    rect(&d, 10.0, 10.0, 40.0, 40.0);
    rect(&d, 100.0, 10.0, 40.0, 40.0);
    let mut s = open(&d);
    click(&mut s, pt(10.0, 30.0), false);
    s.toggle_guide();
    click(&mut s, pt(400.0, 400.0), false);
    let _ = key(&mut s, "a", true);
    assert_eq!(
        s.selected_object_count(),
        2,
        "Select all includes the guide"
    );
    // a guide moves with a drag on its line and is deleted with Delete
    click(&mut s, pt(400.0, 400.0), false);
    click(&mut s, pt(10.0, 30.0), false);
    s.pointer_hover(pt(10.0, 30.0), false, false);
    s.pointer_down(pt(10.0, 30.0), false);
    s.pointer_hover(pt(30.0, 30.0), false, false);
    s.pointer_up(pt(30.0, 30.0), false, false);
    let moved = doc_of(&s);
    let g = moved.read_nodes().into_iter().find(|n| n.guide).unwrap();
    assert!(
        format!("{:?}", g.object().unwrap()).contains("x: 30.0"),
        "moved by 20 mm"
    );
    assert_eq!(key(&mut s, "Delete", false), KeyOutcome::Deleted);
    assert!(guides(&s).is_empty());
}

// ---- Fit to content (13) -----------------------------------------------------------------------

#[test]
fn c13_fit_ignores_guides_and_with_guides_only_is_empty() {
    let d = Document::new(1);
    rect(&d, 10.0, 10.0, 40.0, 40.0);
    let mut s = open(&d);
    s.load_template(&source_bytes(&[(-500.0, -500.0, 2000.0, 2000.0)]))
        .unwrap();
    let size_before = s.size_text();
    assert_eq!(s.fit_document(), FitOutcome::Fitted);
    assert_ne!(s.size_text(), size_before);
    assert_eq!(
        s.size_text(),
        "40.0 × 40.0 mm",
        "fit measures the artwork only"
    );
    // guides only
    let mut g = open(&Document::new(1));
    g.load_template(&source_bytes(&[(1.0, 1.0, 5.0, 5.0)]))
        .unwrap();
    let before = json(&g);
    assert_eq!(g.fit_document(), FitOutcome::Empty);
    assert_eq!(json(&g), before, "nothing written");
}

// ---- Template load (19-23, 33) -----------------------------------------------------------------

#[test]
fn c22_garbage_empty_and_oversized_files_change_nothing() {
    let mut s = open(&Document::new(1));
    let before = json(&s);
    let steps = s.undo_len();
    assert!(s.load_template(b"").is_err());
    assert!(s.load_template(b"not a project at all").is_err());
    let mut zipish = vec![0x50, 0x4b, 0x03, 0x04];
    zipish.extend_from_slice(&[0u8; 64]);
    assert!(s.load_template(&zipish).is_err());
    let big = vec![0u8; 64 * 1024 * 1024 + 1];
    let err = s.load_template(&big).unwrap_err();
    assert!(err.message().contains("64 MiB"));
    assert_eq!(json(&s), before);
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn c22_a_future_format_is_refused_with_a_plain_message() {
    let mut s = open(&Document::new(1));
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures/future_format_version.curvyo"),
    )
    .unwrap();
    let e = s.load_template(&bytes).unwrap_err();
    assert!(e.message().contains("newer"));
}

#[test]
fn c23_load_is_one_step_template_guides_selected_top_left_kept() {
    let mut s = open(&Document::new(1));
    let steps = s.undo_len();
    let n = s
        .load_template(&source_bytes(&[
            (10.0, 20.0, 30.0, 40.0),
            (100.0, 100.0, 5.0, 5.0),
        ]))
        .unwrap();
    assert_eq!(n, 2);
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_name(&mut s).as_deref(), Some("Template"));
    assert_eq!(s.selected_object_count(), 2, "selected afterwards");
    let d = doc_of(&s);
    let nodes: Vec<_> = d.read_nodes().into_iter().filter(|n| n.guide).collect();
    assert_eq!(nodes.len(), 2);
    let ObjectSnapshot::Primitive(p) = nodes[0].object().unwrap().clone() else {
        panic!()
    };
    assert!(
        format!("{:?}", p.shape).contains("x: 10.0, y: 20.0"),
        "coordinates, not recentred"
    );
    // loading again adds again
    s.load_template(&source_bytes(&[(0.0, 0.0, 1.0, 1.0)]))
        .unwrap();
    assert_eq!(guides(&s).len(), 3);
    // undo removes the second load only
    s.undo();
    assert_eq!(guides(&s).len(), 2);
    s.undo();
    assert!(guides(&s).is_empty());
    s.redo();
    s.redo();
    assert_eq!(guides(&s).len(), 3);
}

#[test]
fn c20_an_empty_source_loads_zero_and_adds_no_step() {
    let mut s = open(&Document::new(1));
    let steps = s.undo_len();
    assert_eq!(s.load_template(&source_bytes(&[])).unwrap(), 0);
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn c20_source_with_guides_and_ordinary_paths_only_guides_come_in() {
    let src = Document::new(7);
    let a = rect(&src, 0.0, 0.0, 10.0, 10.0);
    rect(&src, 50.0, 0.0, 10.0, 10.0);
    src.set_guide(&[a], true).unwrap();
    let mut s = open(&Document::new(1));
    assert_eq!(s.load_template(&pack(&src, "t").unwrap()).unwrap(), 1);
}

#[test]
fn c20_source_is_the_open_documents_own_saved_copy() {
    let d = Document::new(1);
    rect(&d, 10.0, 10.0, 40.0, 40.0);
    let mut s = open(&d);
    let saved = s.pack("0.1.0").unwrap();
    assert_eq!(s.load_template(&saved).unwrap(), 1);
    assert_eq!(doc_of(&s).object_ids().len(), 2);
}

#[test]
fn c23_hidden_source_nodes_and_images_are_left_out_when_no_guides() {
    let src = Document::new(7);
    let a = rect(&src, 0.0, 0.0, 10.0, 10.0);
    let h = rect(&src, 50.0, 0.0, 10.0, 10.0);
    let _ = a;
    src.set_node_visible(&[h], false).unwrap();
    let mut s = open(&Document::new(1));
    assert_eq!(s.load_template(&pack(&src, "t").unwrap()).unwrap(), 1);
}

#[test]
fn c33_five_thousand_nodes_and_a_100_000_mm_template_draw_without_nan() {
    let rects: Vec<_> = (0..5000)
        .map(|k| {
            (
                f64::from(k % 100) * 12.0,
                f64::from(k / 100) * 12.0,
                10.0,
                10.0,
            )
        })
        .collect();
    let mut s = open(&Document::new(1));
    // Thread CPU time, not wall time: under a parallel test run the load took 10 to 14 s of
    // wall time for 2.3 s of work.
    let (loaded, load) = cpu_time_of(|| s.load_template(&source_bytes(&rects)));
    assert_eq!(loaded.unwrap(), 5000);
    let list = s.frame_draw_list();
    assert!(
        list.triangles
            .iter()
            .all(|v| v.position.x.is_finite() && v.position.y.is_finite())
    );
    assert!(list.triangle_count() > 0);
    eprintln!("5000-node template: load {load:?} (CPU time)");
    assert!(load.as_secs_f64() < 10.0);
    let mut w = open(&Document::new(1));
    w.load_template(&source_bytes(&[(0.0, 0.0, 100_000.0, 10.0)]))
        .unwrap();
    let list = w.frame_draw_list();
    assert!(
        list.triangles
            .iter()
            .all(|v| v.position.x.is_finite() && v.position.y.is_finite())
    );
    assert!(list.triangle_count() > 0, "not empty");
}

// ---- marking style (5, 8, 33) ---------------------------------------------------------------------

#[test]
fn c5_a_guide_draws_a_stroke_only_in_one_colour_not_its_fill() {
    let d = Document::new(1);
    let r = rect(&d, 50.0, 50.0, 100.0, 100.0);
    d.edit_style(&[r], &curvyo_document_core::StyleEdit::FillEnabled(true))
        .unwrap();
    d.edit_style(
        &[r],
        &curvyo_document_core::StyleEdit::FillColor(curvyo_document_core::Color {
            r: 255,
            g: 0,
            b: 0,
        }),
    )
    .unwrap();
    let mut s = open(&d);
    let plain = s.draw_list();
    let plain_colours: std::collections::HashSet<_> = plain
        .triangles
        .iter()
        .map(|v| format!("{:?}", v.color))
        .collect();
    click(&mut s, pt(50.0, 100.0), false);
    s.toggle_guide();
    click(&mut s, pt(900.0, 900.0), false);
    let g = s.draw_list();
    let colours: std::collections::HashSet<_> = g
        .triangles
        .iter()
        .map(|v| format!("{:?}", v.color))
        .collect();
    assert_eq!(colours.len(), 1, "one marking colour: {colours:?}");
    assert!(
        plain_colours.len() >= 2,
        "fill and stroke before: {plain_colours:?}"
    );
    assert_ne!(colours, plain_colours);
    // fill is not drawn: nothing inside the outline's inner area
    let inner = g
        .triangles
        .iter()
        .filter(|v| {
            v.position.x > 52.0
                && v.position.x < 148.0
                && v.position.y > 52.0
                && v.position.y < 148.0
        })
        .count();
    assert_eq!(inner, 0, "no fill triangles inside");
    assert_eq!(g.layers().len(), 1, "one stroke run per guide");
}

#[test]
fn c33_stroke_runs_equal_guide_count_and_a_hidden_guide_is_not_drawn() {
    let d = Document::new(1);
    for k in 0..7 {
        rect(&d, 20.0 * f64::from(k), 0.0, 10.0, 10.0);
    }
    let mut s = open(&d);
    marquee(&mut s, pt(-5.0, -5.0), pt(500.0, 100.0));
    let n_plain_layers = s.draw_list().layers().len();
    s.toggle_guide();
    click(&mut s, pt(900.0, 900.0), false);
    let layers = s.draw_list().layers().len();
    assert!(
        layers <= n_plain_layers.max(7),
        "at most one run per guide ({layers})"
    );
    assert_eq!(layers, 7);
}

#[test]
fn c8_line_is_one_screen_pixel_wide_at_every_zoom() {
    let d = Document::new(1);
    let l = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), pt(100.0, 100.0)),
            NewAnchor::corner(AnchorId::new(1, 2), pt(100.0, 300.0)),
        ],
        false,
    );
    let _ = l;
    let mut s = open(&d);
    click(&mut s, pt(100.0, 200.0), false);
    s.toggle_guide();
    click(&mut s, pt(900.0, 900.0), false);
    for ctrl_steps in [-30_i32, -10, 0, 10, 25] {
        for _ in 0..ctrl_steps.abs() {
            s.wheel(
                0.0,
                if ctrl_steps < 0 { 120.0 } else { -120.0 },
                800.0,
                500.0,
                false,
                true,
            );
        }
        let scale = s.view().scale();
        let list = s.draw_list();
        if list.triangles.is_empty() {
            continue;
        }
        let xs: Vec<f64> = list.triangles.iter().map(|v| v.position.x).collect();
        let width = xs.iter().copied().fold(f64::MIN, f64::max)
            - xs.iter().copied().fold(f64::MAX, f64::min);
        assert!(
            (width * scale - 1.0).abs() < 0.05,
            "width {} px at scale {scale}",
            width * scale
        );
    }
}

// ---- history preview (29) -------------------------------------------------------------------------

#[test]
fn c29_preview_of_an_older_version_draws_guides_of_that_version() {
    let d = Document::new(1);
    rect(&d, 50.0, 50.0, 100.0, 100.0);
    let mut s = open(&d);
    click(&mut s, pt(50.0, 100.0), false);
    s.toggle_guide();
    click(&mut s, pt(900.0, 900.0), false);
    click(&mut s, pt(50.0, 100.0), false);
    s.toggle_guide(); // unmarked again: the live document has no guide
    let live = s.frame_draw_list();
    let colours = |l: &curvyo_render_core::DrawList| -> std::collections::HashSet<String> {
        l.triangles
            .iter()
            .map(|v| format!("{:?}", v.color))
            .collect()
    };
    let live_colours = colours(&live);
    let rows: serde_json::Value = serde_json::from_str(&s.history_rows_json(0, 20)).unwrap();
    let n = rows.as_array().unwrap().len();
    assert!(n >= 3, "rows: {rows}");
    // Find the row that is the "Guide" step number 1 (older of the two): preview each row and see
    // that some version draws a colour the live document does not.
    let mut saw_marking = false;
    for i in 0..n {
        if s.preview_history_item(false, i) == curvyo_editor_wasm::HistoryRowPress::Previewed {
            let c = colours(&s.frame_draw_list());
            if !c.is_subset(&live_colours) {
                saw_marking = true;
            }
        }
    }
    assert!(saw_marking, "an older version is drawn with its guide");
}

// ---- Boolean / Combine / Split (34) ---------------------------------------------------------------

fn two_squares(flags: [bool; 2]) -> Session {
    two_squares_at(flags, 10.0)
}

fn two_squares_at(flags: [bool; 2], offset: f64) -> Session {
    let d = Document::new(1);
    let a = rect(&d, 0.0, 0.0, 20.0, 20.0);
    let b = rect(&d, offset, offset, 20.0, 20.0);
    if flags[0] {
        d.set_guide(&[a], true).unwrap();
    }
    if flags[1] {
        d.set_guide(&[b], true).unwrap();
    }
    let mut s = open(&d);
    marquee(&mut s, pt(-5.0, -5.0), pt(60.0, 60.0));
    assert_eq!(s.selected_object_count(), 2);
    s
}

#[test]
fn c34_boolean_result_is_a_guide_only_when_all_inputs_were() {
    for (flags, expect) in [
        ([true, true], true),
        ([true, false], false),
        ([false, true], false),
        ([false, false], false),
    ] {
        for op in [
            BooleanOp::Union,
            BooleanOp::Intersection,
            BooleanOp::Difference,
        ] {
            let mut s = two_squares(flags);
            let out = s.apply_boolean(op);
            assert!(matches!(out, BooleanOutcome::Applied { .. }), "{out:?}");
            let d = doc_of(&s);
            let leaves: Vec<_> = d
                .read_nodes()
                .into_iter()
                .filter(|n| n.object().is_some())
                .collect();
            assert_eq!(leaves.len(), 1, "{flags:?} {op:?}");
            assert_eq!(leaves[0].guide, expect, "{flags:?} {op:?}");
        }
    }
}

#[test]
fn c34_combine_result_is_a_guide_only_when_all_inputs_were() {
    for (flags, expect) in [
        ([true, true], true),
        ([true, false], false),
        ([false, true], false),
    ] {
        let mut s = two_squares_at(flags, 40.0);
        let out = s.apply_combine();
        assert!(matches!(out, CombineOutcome::Applied { .. }), "{out:?}");
        let d = doc_of(&s);
        let leaves: Vec<_> = d
            .read_nodes()
            .into_iter()
            .filter(|n| n.object().is_some())
            .collect();
        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0].guide, expect, "{flags:?}");
    }
}

#[test]
fn c34_split_and_break_apart_of_a_guide_give_guides() {
    let mut s = two_squares_at([true, true], 40.0);
    assert!(matches!(s.apply_combine(), CombineOutcome::Applied { .. }));
    assert_eq!(guides(&s).len(), 1);
    let out = s.apply_break_apart();
    assert!(
        matches!(out, curvyo_editor_wasm::BreakApartOutcome::Applied { .. }),
        "{out:?}"
    );
    let d = doc_of(&s);
    let leaves: Vec<_> = d
        .read_nodes()
        .into_iter()
        .filter(|n| n.object().is_some())
        .collect();
    assert_eq!(leaves.len(), 2);
    assert!(
        leaves.iter().all(|n| n.guide),
        "every piece of a guide is a guide"
    );
    // and a normal compound path gives normal pieces
    let mut t = two_squares_at([false, false], 40.0);
    t.apply_combine();
    t.apply_break_apart();
    assert!(guides(&t).is_empty());
}

// ---- copy (34) ------------------------------------------------------------------------------------

// ---- determinism (30) -----------------------------------------------------------------------------

#[test]
fn c30_session_save_is_deterministic_with_guides() {
    let d = Document::new(1);
    rect(&d, 10.0, 10.0, 40.0, 40.0);
    let _ = d.create_ellipse(EllipseFrame {
        center: pt(100.0, 100.0),
        rx: Length::from_mm(10.0),
        ry: Length::from_mm(10.0),
    });
    let mut s = open(&d);
    marquee(&mut s, pt(-5.0, -5.0), pt(300.0, 300.0));
    s.toggle_guide();
    let a = s.pack("0.1.0").unwrap();
    let b = s.pack("0.1.0").unwrap();
    assert_eq!(a, b);
    let mut s2 = Session::open(5, &a).unwrap();
    s2.set_tool(Tool::Select);
    assert_eq!(json(&s2), json(&s), "the model is the same after reopening");
    // A file that was only opened (no session history) saves byte-identically.
    let again = Session::open(6, &a).unwrap();
    let fresh = Session::open(7, &a).unwrap();
    assert_eq!(again.pack("0.1.0").unwrap(), fresh.pack("0.1.0").unwrap());
}

fn ctrl_drag_copy(s: &mut Session, from: Point, to: Point) -> usize {
    s.modifiers_changed(false, true, false);
    s.pointer_hover(from, false, true);
    s.pointer_down(from, false);
    s.pointer_hover(to, false, true);
    s.pointer_up(to, false, true);
    s.modifiers_changed(false, false, false);
    doc_of(s)
        .read_nodes()
        .into_iter()
        .filter(|n| n.object().is_some())
        .count()
}

#[test]
fn c34_ctrl_drag_copy_keeps_the_flag() {
    let d = Document::new(1);
    let r = rect(&d, 50.0, 50.0, 100.0, 100.0);
    d.set_guide(&[r], true).unwrap();
    let mut s = open(&d);
    let n = ctrl_drag_copy(&mut s, pt(50.0, 100.0), pt(250.0, 100.0));
    assert_eq!(n, 2, "a guide is copied like any node");
    assert!(
        doc_of(&s)
            .read_nodes()
            .iter()
            .filter(|n| n.object().is_some())
            .all(|n| n.guide)
    );
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures")
            .join(name),
    )
    .unwrap()
}

#[test]
fn c24_golden_templates_load_with_the_counts_their_names_say() {
    let mut counts = Vec::new();
    for name in [
        "template_key_ring.curvyo",
        "template_two_circles.curvyo",
        "template_four_lines.curvyo",
        "template_mixed.curvyo",
    ] {
        let src = unpack(5, &fixture(name)).unwrap();
        let mut s = open(&Document::new(1));
        let n = s.load_template(&fixture(name)).unwrap();
        assert_eq!(n, src.template_nodes().len(), "{name}");
        assert_eq!(guides(&s).len(), n, "{name}: all arrive as guides");
        assert_eq!(s.selected_object_count(), n, "{name}: selected");
        counts.push((name, n));
    }
    assert_eq!(counts[0].1, 1, "a key ring is one path with a hole");
    assert_eq!(counts[1].1, 2);
    assert_eq!(counts[2].1, 4);
    let mixed = unpack(5, &fixture("template_mixed.curvyo")).unwrap();
    let nodes = mixed.read_nodes();
    let leaves = nodes.iter().filter(|n| n.object().is_some()).count();
    let g = nodes.iter().filter(|n| n.guide).count();
    assert!(g > 0 && leaves > g, "mixed has guides and ordinary paths");
    assert_eq!(counts[3].1, g, "only the guides come in");
}
