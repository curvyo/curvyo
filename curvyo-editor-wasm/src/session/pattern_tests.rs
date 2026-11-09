//! The Pattern block in the session (`specs/0071-pattern-fill` criteria 1 to 3, 16, 18, 19, 22
//! to 24): the kind switch, previews and their single commit, Escape, the canvas drawing, the
//! state lines, the over-limit refusal, Convert to paths, undo and redo.

use curvyo_document_core::{
    FillPaint, Length, NewAnchor, NodeId, ObjectSnapshot, Point, RectBounds, StyleEdit,
};
use curvyo_ui_core::Grid;

use super::text_tests::labels;
use super::{Session, Tool};

fn select(session: &mut Session, ids: &[NodeId]) {
    session.set_tool(Tool::Select);
    session.selection.clear();
    for &id in ids {
        session.selection.add(id);
    }
}

fn rect(session: &Session, side: f64) -> NodeId {
    session.document.create_rect(RectBounds {
        origin: Point::new(0.0, 0.0),
        width: Length::from_mm(side),
        height: Length::from_mm(side),
    })
}

fn patterned(side: f64) -> (Session, NodeId) {
    let mut session = Session::new(1);
    let id = rect(&session, side);
    select(&mut session, &[id]);
    assert!(session.set_fill_kind("pattern"));
    (session, id)
}

fn fill_of(session: &Session, id: NodeId) -> curvyo_document_core::Fill {
    session.document.object(id).unwrap().style().fill.clone()
}

fn triangles(session: &Session) -> usize {
    session.draw_list().triangles.len()
}

/// Criteria 1, 3, 22: Pattern is a third kind; one step; undo and redo bring it back exactly.
#[test]
fn ac1_ac3_ac22_the_kind_switch_is_one_step_and_undoes() {
    let mut session = Session::new(1);
    let id = rect(&session, 50.0);
    select(&mut session, &[id]);
    let plain = session.pattern_panel();
    assert!(plain.offered && !plain.shown);
    let steps = labels(&session).len();
    let before = triangles(&session);
    assert!(session.set_fill_kind("pattern"));
    assert_eq!(labels(&session).len(), steps + 1);
    let panel = session.pattern_panel();
    assert!(panel.shown);
    assert_eq!(panel.generator, "hatch");
    assert!(triangles(&session) > before, "the pattern is drawn");
    let pattern = fill_of(&session, id);
    assert!(session.set_fill_kind("solid"));
    assert!(fill_of(&session, id).paints_solid());
    session.undo();
    assert_eq!(fill_of(&session, id), pattern);
    session.redo();
    assert!(fill_of(&session, id).paints_solid());
}

/// Criterion 2: a drag previews without writing, the release is one step, Escape restores.
#[test]
fn ac2_a_field_drag_previews_and_commits_once() {
    let (mut session, id) = patterned(50.0);
    let steps = labels(&session).len();
    let drawn = triangles(&session);
    session.preview_pattern_field("spacing", 0.1, Grid::Normal);
    session.preview_pattern_field("spacing", 0.2, Grid::Normal);
    assert_eq!(
        labels(&session).len(),
        steps,
        "nothing written while dragging"
    );
    assert_ne!(triangles(&session), drawn, "the canvas follows the drag");
    session.cancel_style_preview();
    session.commit_style_preview();
    assert_eq!(labels(&session).len(), steps, "Escape writes nothing");
    assert_eq!(triangles(&session), drawn);
    session.preview_pattern_field("spacing", 0.5, Grid::Normal);
    session.commit_style_preview();
    assert_eq!(labels(&session).len(), steps + 1);
    let spacing = fill_of(&session, id)
        .pattern
        .unwrap()
        .generator
        .get(curvyo_document_core::PatternParam::Spacing)
        .unwrap();
    assert!((spacing - 2.24).abs() < 0.01, "{spacing}");
}

/// Criteria 4, 16: the generator and the mode are each one step; Shapes shows the colour rows.
#[test]
fn ac4_ac16_generator_and_mode() {
    let (mut session, _) = patterned(50.0);
    for kind in curvyo_document_core::GeneratorKind::ALL {
        assert!(session.set_pattern_generator(kind.id()) || kind.id() == "hatch");
        assert_eq!(session.pattern_panel().generator, kind.id());
    }
    assert!(session.set_pattern_mode("shapes"));
    let panel = session.pattern_panel();
    assert_eq!(panel.mode, "shapes");
    assert!(panel.colour_rows);
    assert_eq!(
        session.set_pattern_field_text("lines", "1"),
        Err("range"),
        "typed values are checked"
    );
    assert_eq!(session.set_pattern_field_text("lines", "20"), Ok(true));
}

/// Criterion 18: over the hard limit a typed value is refused with the count; the state line of
/// a shape pushed over the limit by an edit is the red one.
#[test]
fn ac18_over_the_limit() {
    let (mut session, id) = patterned(400.0);
    assert!(session.set_pattern_generator("dots"));
    let steps = labels(&session).len();
    assert_eq!(session.set_pattern_field_text("spacing", "0.1"), Ok(false));
    assert_eq!(labels(&session).len(), steps);
    let refusal = session.take_pattern_refusal().unwrap();
    assert!(refusal.starts_with("Not generated: "), "{refusal}");
    assert!(refusal.ends_with("The limit is 500,000."), "{refusal}");
    // A shape edit that pushes it over: the edit is kept, the line is red.
    session.set_pattern_field_text("spacing", "0.6").unwrap();
    session
        .document
        .edit_style(
            &[id],
            &StyleEdit::PatternParam(curvyo_document_core::PatternParam::Spacing, 0.2),
        )
        .unwrap();
    let panel = session.pattern_panel();
    assert!(panel.alert, "{:?}", panel.state);
}

/// Criterion 23: an open shape takes the kind and shows its state line, nothing drawn.
#[test]
fn ac23_an_open_shape_says_so() {
    let mut session = Session::new(1);
    let mut minter = curvyo_ui_core::AnchorIdMinter::new(9);
    let id = session.document.create_path(
        &[
            NewAnchor::corner(minter.mint(), Point::new(0.0, 0.0)),
            NewAnchor::corner(minter.mint(), Point::new(30.0, 0.0)),
            NewAnchor::corner(minter.mint(), Point::new(30.0, 30.0)),
        ],
        false,
    );
    select(&mut session, &[id]);
    assert!(session.set_fill_kind("pattern"));
    let panel = session.pattern_panel();
    assert_eq!(panel.state, ["Open shapes show no pattern."]);
    assert!(!panel.convert);
}

/// Criterion 24: Convert is one step; the group is selected; undo restores the live pattern.
#[test]
fn ac24_convert_to_paths() {
    let (mut session, id) = patterned(30.0);
    assert!(session.pattern_panel().convert);
    let steps = labels(&session).len();
    assert!(session.convert_pattern_to_paths());
    assert_eq!(labels(&session).len(), steps + 1);
    assert_eq!(labels(&session).last().unwrap(), "convert_pattern");
    let selected = session.selection.ids().to_vec();
    let group = session.document.parent_group(selected[0]).unwrap();
    assert_eq!(
        selected,
        session.document.leaves_of(group),
        "the new group is selected"
    );
    assert!(!fill_of(&session, id).enabled);
    session.undo();
    assert_eq!(fill_of(&session, id).paint, FillPaint::Pattern);
    assert!(fill_of(&session, id).enabled);
    assert!(
        session
            .objects()
            .iter()
            .all(|o| !matches!(o, ObjectSnapshot::Path(_))),
        "the converted paths are gone"
    );
}

/// `adrs.md` decision 3 (lead 2026-11-02): Lines pieces are drawn with the stroke settings even
/// while the outline's stroke is off; no state line.
#[test]
fn lines_are_drawn_with_the_stroke_settings_when_the_stroke_is_off() {
    let (mut session, _) = patterned(30.0);
    session.set_stroke_paint(false);
    assert_eq!(session.pattern_panel().state, Vec::<String>::new());
    let with_pattern = triangles(&session);
    session.set_fill_kind("none");
    assert!(with_pattern > triangles(&session), "the pieces are drawn");
}

/// UX notes §7: a field dragged past the hard limit keeps showing the last valid pattern with the
/// red count; the release writes nothing.
#[test]
fn ac18_a_drag_past_the_limit_shows_the_last_valid_pattern() {
    let (mut session, _) = patterned(100.0);
    assert!(session.set_pattern_generator("dots"));
    let steps = labels(&session).len();
    let drawn = triangles(&session);
    session.preview_pattern_field("spacing", 0.0, Grid::Normal);
    assert_eq!(triangles(&session), drawn, "the last valid pattern stays");
    assert!(session.pattern_panel().alert);
    session.commit_style_preview();
    assert_eq!(labels(&session).len(), steps, "the release is refused");
    assert!(session.take_pattern_refusal().is_some());
    assert!(!session.pattern_panel().alert);
}

/// UI review 2026-11-02, item 4: the work refusal names no lever the generator may lack.
#[test]
fn ac18_too_detailed_names_no_lever() {
    let (mut session, id) = patterned(100.0);
    assert!(session.set_pattern_generator("interpolation_waves"));
    assert_eq!(session.set_pattern_field_text("lines", "500"), Ok(true));
    session
        .document
        .edit_style(
            &[id],
            &StyleEdit::PatternParam(curvyo_document_core::PatternParam::Lines, 100_000.0),
        )
        .unwrap();
    assert_eq!(
        session.pattern_panel().state,
        ["Not generated: too detailed for this shape. Make the pattern coarser."]
    );
}

/// UI review 2026-11-02, item 5: a Joint over the brick height names the brick height.
#[test]
fn ac5_joint_over_the_brick_names_the_brick_height() {
    let (mut session, _) = patterned(50.0);
    assert!(session.set_pattern_generator("brick_wall"));
    assert_eq!(session.set_pattern_field_text("height", "4"), Ok(true));
    assert_eq!(
        session.set_pattern_field_text("joint", "6"),
        Err("brick_height")
    );
    assert!(session.set_pattern_generator("honeycomb"));
    assert_eq!(
        session.set_pattern_field_text("wall", "40"),
        Err("cell_size")
    );
}

/// UI review 2026-11-02, item 2: after Convert on a drawn shape (in a layer) the new group stays
/// selected once the session re-reads its context.
#[test]
fn ac24_convert_keeps_the_new_group_selected_in_a_layer() {
    let mut session = Session::new(1);
    let (from, to) = (Point::new(10.0, 10.0), Point::new(60.0, 60.0));
    session.set_tool(Tool::Rectangle);
    session.pointer_hover(from, false, false);
    session.pointer_down(from, false);
    session.pointer_hover(to, false, false);
    session.pointer_up(to, false, false);
    session.set_tool(Tool::Select);
    assert_eq!(session.selection.ids().len(), 1);
    assert!(session.set_fill_kind("pattern"));
    assert!(session.convert_pattern_to_paths());
    let selected = session.selection.ids().to_vec();
    session.sync_context();
    assert_eq!(session.selection.ids(), selected.as_slice());
    let group = session.document.parent_group(selected[0]).unwrap();
    assert_eq!(selected, session.document.leaves_of(group));
    assert!(
        session.style_panel_view().subject.contains("Group"),
        "the panel names the group"
    );
}

/// Criterion 29: Whole pieces only shows for the generators of shapes, is one named step, draws
/// fewer pieces, and undoes.
#[test]
fn ac29_whole_pieces_only_is_one_step_for_the_shape_generators() {
    use curvyo_document_core::{Direction, GeneratorKind};
    let (mut session, id) = patterned(50.0);
    assert!(
        !session.pattern_panel().whole_offered,
        "Hatch has no whole pieces"
    );
    for kind in GeneratorKind::ALL {
        session.set_pattern_generator(kind.id());
        assert_eq!(
            session.pattern_panel().whole_offered,
            kind.has_whole_pieces()
        );
    }
    assert!(session.set_pattern_generator("honeycomb"));
    let panel = session.pattern_panel();
    assert_eq!(panel.whole_pieces, "off");
    let cut = triangles(&session);
    let steps = labels(&session).len();
    assert!(session.set_pattern_whole_pieces(true));
    assert_eq!(labels(&session).len(), steps + 1);
    assert_eq!(
        labels(&session).last().map(String::as_str),
        Some("pattern_whole_pieces")
    );
    assert_eq!(
        session.history_target_name(Direction::Undo).as_deref(),
        Some("Pattern: whole pieces")
    );
    assert_eq!(session.pattern_panel().whole_pieces, "on");
    assert!(fill_of(&session, id).pattern.unwrap().whole_pieces);
    assert!(triangles(&session) < cut, "cut cells are left out");
    session.set_pattern_whole_pieces(true);
    assert_eq!(
        labels(&session).len(),
        steps + 1,
        "unchanged writes no step"
    );
    session.undo();
    assert_eq!(session.pattern_panel().whole_pieces, "off");
    assert_eq!(triangles(&session), cut);
}
