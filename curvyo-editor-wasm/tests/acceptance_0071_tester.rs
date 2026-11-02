//! Independent tester tests for `specs/0071-pattern-fill` through `Session`, driven the way the
//! wasm host drives it: the Pattern block's JSON, previews and commits of a field, Escape, the
//! history names, the count limits (note, refusal, last valid pattern while dragging), live
//! regeneration after a node drag, undo and redo, open shapes, Convert to paths and its equality
//! with the export, the SVG export without `<pattern>`, Lines with the outline stroke off, and the
//! byte-equal export across sessions. Written from the specification, ADR 0029 and the UX notes;
//! only public names and the panel JSON field names were looked up.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(clippy::too_many_lines, missing_docs, clippy::doc_markdown)]

use curvyo_document_core::{
    AnchorId, Document, FillPaint, GeneratorKind, NewAnchor, NodeId, ObjectSnapshot, PatternMode,
    PatternParam, Point, StyleEdit, pack, unpack,
};
use curvyo_editor_wasm::{Session, SvgExportResult, Tool};
use curvyo_ui_core::{Grid, SvgExportOptions, object_pattern_pieces, piece_style};
use serde_json::Value;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// A closed 60 x 40 path at the origin; anchor ids (1, 1..=4).
fn square(d: &Document) -> NodeId {
    d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), pt(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), pt(60.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 3), pt(60.0, 40.0)),
            NewAnchor::corner(AnchorId::new(1, 4), pt(0.0, 40.0)),
        ],
        true,
    )
}

fn open(d: &Document) -> Session {
    let mut s = Session::open(2, &pack(d, "t").unwrap()).unwrap();
    let mut sn = s.snapping();
    sn.enabled = false;
    s.set_snapping(sn);
    s
}

fn click(s: &mut Session, p: Point) {
    s.modifiers_changed(false, false, false);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

fn drag(s: &mut Session, a: Point, b: Point) {
    s.modifiers_changed(false, false, false);
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

/// A session on one patterned square, selected.
fn patterned() -> (Session, NodeId) {
    let d = Document::new(1);
    let id = square(&d);
    let mut s = open(&d);
    s.set_tool(Tool::Select);
    click(&mut s, pt(0.0, 20.0));
    assert_eq!(s.selected_object_count(), 1, "the square is selected");
    assert!(s.set_fill_kind("pattern"));
    (s, id)
}

fn panel(s: &Session) -> Value {
    serde_json::to_value(s.pattern_panel()).unwrap()
}

fn doc(s: &Session) -> Document {
    unpack(99, &s.pack("t").unwrap()).unwrap()
}

fn obj(d: &Document, id: NodeId) -> ObjectSnapshot {
    d.read_objects()
        .objects
        .iter()
        .find(|o| o.id() == id)
        .cloned()
        .unwrap()
}

fn param(s: &Session, id: NodeId, p: PatternParam) -> Option<f64> {
    obj(&doc(s), id)
        .style()
        .fill
        .painted_pattern()?
        .generator
        .get(p)
}

fn history_names(s: &mut Session) -> Vec<String> {
    let rows: Value = serde_json::from_str(&s.history_rows_json(0, 200)).unwrap();
    rows.as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect()
}

fn last_name(s: &mut Session) -> String {
    // The History rows list the newest first or last; take the one that is new.
    let names = history_names(s);
    names.first().cloned().unwrap_or_default()
}

fn export(s: &Session) -> String {
    match s.export_svg(SvgExportOptions::default()) {
        SvgExportResult::Written { text, .. } => text,
        SvgExportResult::NothingSelected => panic!("nothing selected"),
    }
}

/// The artwork triangles (the overlay after them holds selection marks, which differ by session).
fn triangles(s: &Session) -> Vec<String> {
    let list = s.draw_list();
    let end = list.layers().last().copied().unwrap_or(0);
    list.triangles[..end]
        .iter()
        .map(|v| format!("{v:?}"))
        .collect()
}

fn piece_count(s: &Session, id: NodeId) -> usize {
    object_pattern_pieces(&obj(&doc(s), id)).map_or(0, |r| r.map_or(0, |v| v.len()))
}

// ---------- criterion 1: the fill kind and the block ----------

#[test]
fn c1_pattern_is_a_fill_kind_and_the_block_shows_generator_and_parameters() {
    let d = Document::new(1);
    square(&d);
    let mut s = open(&d);
    click(&mut s, pt(0.0, 20.0));
    let before = panel(&s);
    assert_eq!(before["offered"], true, "{before}");
    assert_eq!(before["shown"], false);
    assert!(s.set_fill_kind("pattern"));
    let p = panel(&s);
    assert_eq!(p["kind"], "pattern");
    assert_eq!(p["shown"], true);
    assert_eq!(p["generator"], "hatch", "a first pattern is Hatch");
    let names: Vec<&str> = p["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["spacing", "angle"]);
    assert!(p["margin"].is_object());
    assert_eq!(p["mode"], "lines");
    assert_eq!(p["seeded"], false, "Hatch has no seed row");
    assert!(s.set_pattern_generator("leaf_forest"));
    assert_eq!(panel(&s)["seeded"], true);
    assert!(s.set_fill_kind("solid"));
    assert_eq!(panel(&s)["kind"], "solid");
    assert!(s.set_fill_kind("none"));
    assert_eq!(panel(&s)["kind"], "none");
}

#[test]
fn c4_every_generator_tile_switches_and_an_unknown_one_does_nothing() {
    let (mut s, id) = patterned();
    for kind in GeneratorKind::ALL {
        assert!(s.set_pattern_generator(kind.id()), "{kind:?}");
        assert_eq!(panel(&s)["generator"], kind.id());
        assert!(piece_count(&s, id) > 0, "{kind:?} draws on the square");
    }
    let undo = s.undo_len();
    assert!(!s.set_pattern_generator("voronoi"));
    assert_eq!(s.undo_len(), undo);
}

// ---------- criterion 2: live preview, one step, Escape ----------

#[test]
fn c2_a_drag_previews_live_without_writing_and_release_is_one_step() {
    let (mut s, id) = patterned();
    let start = triangles(&s);
    let undo = s.undo_len();
    let spacing = param(&s, id, PatternParam::Spacing).unwrap();
    s.preview_pattern_field("spacing", 0.9, Grid::Normal);
    let preview = triangles(&s);
    assert_ne!(
        preview, start,
        "the canvas shows the new pattern while dragging"
    );
    assert_eq!(s.undo_len(), undo, "nothing written while dragging");
    assert_eq!(param(&s, id, PatternParam::Spacing), Some(spacing));
    s.preview_pattern_field("spacing", 0.7, Grid::Normal);
    s.commit_style_preview();
    assert_eq!(s.undo_len(), undo + 1, "release is one undo step");
    let committed = param(&s, id, PatternParam::Spacing).unwrap();
    assert_ne!(committed, spacing);
    assert_eq!(last_name(&mut s), "Pattern spacing");
    s.undo();
    assert_eq!(param(&s, id, PatternParam::Spacing), Some(spacing));
}

#[test]
fn c2_cancel_restores_the_previous_value_and_canvas() {
    let (mut s, id) = patterned();
    let start = triangles(&s);
    let undo = s.undo_len();
    s.preview_pattern_field("angle", 0.2, Grid::Normal);
    assert_ne!(triangles(&s), start);
    s.cancel_style_preview();
    assert_eq!(triangles(&s), start, "Escape restores the canvas");
    assert_eq!(s.undo_len(), undo);
    assert_eq!(param(&s, id, PatternParam::Angle), Some(45.0));
}

#[test]
fn c2_typed_value_is_one_step_and_bad_text_writes_nothing() {
    let (mut s, id) = patterned();
    let undo = s.undo_len();
    assert_eq!(s.set_pattern_field_text("spacing", "3.5"), Ok(true));
    assert_eq!(s.undo_len(), undo + 1);
    assert_eq!(param(&s, id, PatternParam::Spacing), Some(3.5));
    for bad in ["", "abc", "0", "-2", "NaN", "1e400"] {
        assert!(
            s.set_pattern_field_text("spacing", bad).is_err(),
            "{bad:?} refused"
        );
    }
    assert_eq!(s.undo_len(), undo + 1);
    assert_eq!(param(&s, id, PatternParam::Spacing), Some(3.5));
}

// ---------- criterion 3 and 22: kind switch, undo and redo, history names ----------

#[test]
fn c3_c22_none_hides_the_pattern_and_undo_redo_restore_it_exactly() {
    let (mut s, id) = patterned();
    assert!(s.set_pattern_generator("honeycomb"));
    assert_eq!(s.set_pattern_field_text("cell_size", "7"), Ok(true));
    assert!(s.set_pattern_mode("shapes"));
    assert_eq!(s.set_pattern_field_text("margin", "1.5"), Ok(true));
    let with = obj(&doc(&s), id).style().clone();
    let pieces = piece_count(&s, id);
    assert!(pieces > 0);
    assert!(s.set_fill_kind("none"));
    assert_eq!(piece_count(&s, id), 0, "the pattern disappears");
    s.undo();
    assert_eq!(
        obj(&doc(&s), id).style(),
        &with,
        "undo brings it back exactly"
    );
    assert_eq!(piece_count(&s, id), pieces);
    s.redo();
    assert_eq!(piece_count(&s, id), 0);
    s.undo();
    let names = history_names(&mut s);
    for want in [
        "Fill: Pattern",
        "Pattern: Honeycomb",
        "Pattern cell size",
        "Pattern: Shapes",
        "Pattern margin",
    ] {
        assert!(
            names.iter().any(|n| n == want),
            "history names {names:?} lack {want}"
        );
    }
}

#[test]
fn c17_reroll_and_typed_seed_are_steps_and_change_only_the_leaves() {
    let (mut s, id) = patterned();
    assert!(s.set_pattern_generator("leaf_forest"));
    let before = object_pattern_pieces(&obj(&doc(&s), id)).unwrap().unwrap();
    assert!(s.reroll_pattern_seed());
    let after = object_pattern_pieces(&obj(&doc(&s), id)).unwrap().unwrap();
    assert_ne!(before, after);
    assert!(
        history_names(&mut s)
            .iter()
            .any(|n| n == "Pattern: new seed")
    );
    assert_eq!(s.set_pattern_seed_text("4294967295"), Ok(true));
    assert_eq!(panel(&s)["seed"], "4294967295");
    for bad in ["4294967296", "-1", "1.5", "x"] {
        assert!(s.set_pattern_seed_text(bad).is_err(), "seed {bad:?}");
    }
}

// ---------- criterion 18: limits ----------

#[test]
fn c18_between_the_limits_the_section_shows_a_quiet_count() {
    let (mut s, id) = patterned();
    assert!(s.set_pattern_generator("dots"));
    assert_eq!(s.set_pattern_field_text("diameter", "0.05"), Ok(true));
    assert_eq!(s.set_pattern_field_text("spacing", "0.15"), Ok(true));
    let n = piece_count(&s, id);
    assert!(n > 50_000, "{n} pieces");
    let p = panel(&s);
    let state: Vec<String> = p["state"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(p["alert"], false, "a note, not an alert");
    assert!(
        state.iter().any(|l| l.contains(&group(n))),
        "the note shows the count {n}: {state:?}"
    );
}

fn group(n: usize) -> String {
    let d = n.to_string();
    let mut out = String::new();
    for (i, c) in d.chars().enumerate() {
        if i > 0 && (d.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A big square (500 mm) so dots at the minimum spacing pass the hard limit.
fn big() -> (Session, NodeId) {
    let d = Document::new(1);
    let id = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), pt(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), pt(500.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 3), pt(500.0, 500.0)),
            NewAnchor::corner(AnchorId::new(1, 4), pt(0.0, 500.0)),
        ],
        true,
    );
    let mut s = open(&d);
    click(&mut s, pt(0.0, 250.0));
    assert_eq!(s.selected_object_count(), 1);
    assert!(s.set_fill_kind("pattern"));
    assert!(s.set_pattern_generator("dots"));
    assert_eq!(s.set_pattern_field_text("diameter", "0.05"), Ok(true));
    (s, id)
}

#[test]
fn c18_over_the_hard_limit_the_shape_keeps_its_state_and_the_section_says_why() {
    let (mut s, id) = big();
    let before = obj(&doc(&s), id).style().clone();
    let undo = s.undo_len();
    let canvas = triangles(&s);
    let r = s.set_pattern_field_text("spacing", "0.05");
    let refusal = s.take_pattern_refusal();
    let p = panel(&s);
    assert_eq!(
        obj(&doc(&s), id).style(),
        &before,
        "previous state kept ({r:?})"
    );
    assert_eq!(s.undo_len(), undo);
    assert_eq!(triangles(&s), canvas);
    let reason = refusal.unwrap_or_default()
        + &p["state"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<String>();
    assert!(
        reason.contains("500,000"),
        "the reason names the limit: {reason}"
    );
    assert!(
        reason.chars().filter(char::is_ascii_digit).count() > 6,
        "and the count: {reason}"
    );
}

#[test]
fn c18_dragging_over_the_limit_keeps_the_last_valid_pattern_on_the_canvas() {
    let (mut s, id) = big();
    let valid = triangles(&s);
    let spacing = param(&s, id, PatternParam::Spacing).unwrap();
    // Drag to the bottom of the scale (the minimum spacing): far over the limit.
    s.preview_pattern_field("spacing", 0.0, Grid::Normal);
    let over = triangles(&s);
    assert_eq!(over, valid, "the last valid pattern stays while dragging");
    let p = panel(&s);
    assert_eq!(
        p["alert"], true,
        "the section shows the over-limit alert: {p}"
    );
    s.commit_style_preview();
    assert_eq!(
        param(&s, id, PatternParam::Spacing),
        Some(spacing),
        "releasing over the limit keeps the previous state"
    );
}

// ---------- criterion 19: live regeneration after a shape edit ----------

#[test]
fn c19_a_node_drag_regenerates_the_pattern_to_the_new_outline() {
    let (mut s, id) = patterned();
    let _ = triangles(&s); // warm the cache
    s.set_tool(Tool::Node);
    click(&mut s, pt(0.0, 20.0));
    drag(&mut s, pt(60.0, 40.0), pt(90.0, 70.0));
    let d = doc(&s);
    let o = obj(&d, id);
    let ObjectSnapshot::Path(p) = &o else {
        panic!()
    };
    assert!(
        p.anchors.iter().any(|a| (a.point.x - 90.0).abs() < 1e-6),
        "the node moved"
    );
    // The live canvas equals a fresh session's (no cache) for the edited document.
    let fresh = open(&d);
    assert_eq!(triangles(&s), triangles(&fresh), "regenerated, not stale");
    // And the pieces reach into the new corner.
    let pieces = object_pattern_pieces(&o).unwrap().unwrap();
    assert!(
        pieces
            .iter()
            .flat_map(|pc| pc.outlines.iter())
            .flat_map(|(a, _)| a.iter())
            .any(|t| t.0.x > 70.0 && t.0.y > 45.0),
        "the pattern fills the dragged corner"
    );
    s.undo();
    let back = doc(&s);
    assert_eq!(
        triangles(&s),
        triangles(&open(&back)),
        "undo regenerates too"
    );
}

#[test]
fn c19_moving_the_shape_keeps_the_canvas_equal_to_a_fresh_read() {
    let (mut s, _) = patterned();
    let _ = triangles(&s);
    drag(&mut s, pt(0.0, 20.0), pt(17.5, 31.25));
    assert_eq!(triangles(&s), triangles(&open(&doc(&s))));
}

// ---------- criterion 23: open shapes ----------

#[test]
fn c23_an_open_path_is_offered_pattern_but_nothing_is_generated() {
    let d = Document::new(1);
    let id = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), pt(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 2), pt(60.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, 3), pt(60.0, 40.0)),
        ],
        false,
    );
    let mut s = open(&d);
    click(&mut s, pt(30.0, 0.0));
    assert_eq!(s.selected_object_count(), 1);
    assert_eq!(panel(&s)["offered"], true);
    assert!(s.set_fill_kind("pattern"));
    assert_eq!(piece_count(&s, id), 0);
    assert!(s.take_pattern_refusal().is_none(), "no error");
    let svg = export(&s);
    assert_eq!(svg.matches("<path").count(), 1, "only the open path: {svg}");
    assert!(!s.convert_pattern_to_paths(), "nothing to convert");
}

// ---------- criteria 24, 25: Convert to paths ----------

#[test]
fn c24_c25_convert_makes_a_selected_group_equal_to_the_export_and_undo_restores() {
    let (mut s, id) = patterned();
    assert!(s.set_pattern_generator("dots"));
    assert_eq!(s.set_pattern_field_text("margin", "1"), Ok(true));
    let before_doc = doc(&s);
    let shape = obj(&before_doc, id);
    let pieces = object_pattern_pieces(&shape).unwrap().unwrap();
    let svg_before = export(&s);
    let undo = s.undo_len();
    assert_eq!(panel(&s)["convert"], true);
    assert!(s.convert_pattern_to_paths());
    assert_eq!(s.undo_len(), undo + 1, "one undo step");
    assert_eq!(last_name(&mut s), "Convert pattern to paths");
    let d = doc(&s);
    let read = d.read_objects();
    let shape_now = obj(&d, id);
    assert!(!shape_now.style().fill.enabled, "the shape's fill is None");
    assert!(shape_now.style().fill.painted_pattern().is_none());
    // The new paths: one per piece, geometry identical to the pieces.
    let paths: Vec<&ObjectSnapshot> = read
        .objects
        .iter()
        .filter(|o| o.id() != id && matches!(o, ObjectSnapshot::Path(_)))
        .collect();
    assert_eq!(paths.len(), pieces.len(), "one path per piece");
    for (o, piece) in paths.iter().zip(&pieces) {
        let ObjectSnapshot::Path(p) = o else {
            unreachable!()
        };
        let outlines: Vec<_> = p.subpaths().collect();
        assert_eq!(outlines.len(), piece.outlines.len());
        for (sub, (anchors, closed)) in outlines.iter().zip(&piece.outlines) {
            assert_eq!(sub.closed, *closed);
            assert_eq!(sub.anchors.len(), anchors.len());
            for (a, t) in sub.anchors.iter().zip(anchors) {
                assert!(
                    (a.point.x - t.0.x).abs() < 1e-9 && (a.point.y - t.0.y).abs() < 1e-9,
                    "converted geometry equals the live pieces"
                );
            }
        }
        assert_eq!(o.style(), &piece_style(shape.style(), PatternMode::Lines));
    }
    // Selected the way Group selects: the group's member objects, nothing else, and they are
    // exactly the converted pieces; the announcement names the group.
    assert_eq!(
        s.selected_object_count(),
        paths.len(),
        "the new group's members are selected, nothing else"
    );
    assert!(
        s.selection_announcement()
            .starts_with(&format!("Group of {} objects", paths.len())),
        "announced as the group: {:?}",
        s.selection_announcement()
    );
    // The export of the converted drawing has the same piece paths as before.
    let svg_after = export(&s);
    let ds = |t: &str| -> Vec<String> {
        t.split("<path")
            .skip(1)
            .filter_map(|p| {
                p.split(" d=\"")
                    .nth(1)
                    .map(|d| d.split('"').next().unwrap().to_owned())
            })
            .collect()
    };
    let mut a = ds(&svg_before);
    let mut b = ds(&svg_after);
    a.sort();
    b.sort();
    assert_eq!(
        a, b,
        "converted paths export exactly as the live pattern did"
    );
    s.undo();
    let back = doc(&s);
    assert_eq!(
        obj(&back, id).style(),
        shape.style(),
        "undo restores the live pattern"
    );
    assert_eq!(
        back.read_objects().objects.len(),
        before_doc.read_objects().objects.len()
    );
}

// ---------- criteria 16, 26, 17: output ----------

#[test]
fn c26_svg_export_has_paths_no_pattern_element_and_no_generator_reference() {
    let (mut s, id) = patterned();
    for kind in GeneratorKind::ALL {
        assert!(s.set_pattern_generator(kind.id()));
        let svg = export(&s);
        let n = piece_count(&s, id);
        assert!(!svg.contains("<pattern"), "{kind:?}");
        assert!(!svg.contains("url(#"), "{kind:?}: no paint server");
        for word in [kind.id(), "fill_pattern", "generator", "seed"] {
            assert!(!svg.contains(word), "{kind:?}: the export names {word}");
        }
        assert_eq!(
            svg.matches("<path").count(),
            n + 1,
            "{kind:?}: region + pieces"
        );
    }
}

#[test]
fn c16_lines_draw_with_the_stroke_settings_even_with_the_outline_stroke_off() {
    let d = Document::new(1);
    let id = square(&d);
    d.edit_style(&[id], &StyleEdit::StrokeEnabled(false))
        .unwrap();
    d.edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Pattern))
        .unwrap();
    let s = open(&d);
    let o = obj(&doc(&s), id);
    let lines = piece_style(o.style(), PatternMode::Lines);
    assert!(lines.stroke.enabled, "Lines pieces are stroked");
    assert!(!lines.fill.enabled, "and never filled");
    assert_eq!(lines.stroke.width, o.style().stroke.width);
    assert_eq!(lines.stroke.color, o.style().stroke.color);
    let shapes = piece_style(o.style(), PatternMode::Shapes);
    assert!(
        shapes.fill.enabled && !shapes.stroke.enabled,
        "Shapes are filled"
    );
    assert_eq!(shapes.fill.color, o.style().fill.color);
    let svg = export(&s);
    let pieces_with_stroke = svg
        .split("<path")
        .skip(1)
        .filter(|p| p.contains("stroke=\"#") || p.contains("stroke=\"rgb"))
        .count();
    assert!(
        pieces_with_stroke > 0 || svg.contains("<g stroke=\"#"),
        "pieces stroked: {svg}"
    );
}

#[test]
fn c17_the_export_is_byte_equal_across_sessions_and_peers() {
    let (mut s, _) = patterned();
    assert!(s.set_pattern_generator("leaf_forest"));
    let bytes = s.pack("t").unwrap();
    let a = export(&Session::open(3, &bytes).unwrap());
    let b = export(&Session::open(77, &bytes).unwrap());
    assert_eq!(a, b);
    assert_eq!(a, export(&s));
}

#[test]
fn c18_a_shape_edit_past_the_limit_is_kept_and_draws_no_pattern_until_back_under() {
    let (mut s, id) = patterned();
    assert!(s.set_pattern_generator("dots"));
    assert_eq!(s.set_pattern_field_text("diameter", "0.05"), Ok(true));
    assert_eq!(s.set_pattern_field_text("spacing", "0.2"), Ok(true));
    let small = triangles(&s);
    s.set_tool(Tool::Node);
    click(&mut s, pt(0.0, 20.0));
    drag(&mut s, pt(60.0, 40.0), pt(400.0, 400.0));
    let d = doc(&s);
    let ObjectSnapshot::Path(p) = obj(&d, id) else {
        panic!()
    };
    assert!(
        p.anchors.iter().any(|a| (a.point.x - 400.0).abs() < 1e-6),
        "the edit is kept"
    );
    assert!(matches!(
        object_pattern_pieces(&obj(&d, id)),
        Some(Err(curvyo_ui_core::PatternError::TooMany { .. }))
    ));
    assert_ne!(triangles(&s), small);
    assert_eq!(
        panel(&s)["alert"],
        true,
        "the red line stays as the shape's state"
    );
    s.undo();
    assert_eq!(panel(&s)["alert"], false, "back under the limit");
    assert_eq!(triangles(&s), small);
}
