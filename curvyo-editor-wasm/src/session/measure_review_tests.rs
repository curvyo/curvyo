//! The 0070 UI review follow-ups (`ux-notes.md` "UI review (2026-11-01)"): the Layers row text,
//! the controls a selected dimension offers and the anchor dot of the Shift preview.
use super::*;
use curvyo_document_core::{AnchorId, NewAnchor, NodeId, RectBounds};

fn session_with_line() -> (Session, NodeId) {
    let mut session = Session::new(1);
    let mut snapping = session.snapping();
    snapping.enabled = true;
    session.set_snapping(snapping);
    let path = session.document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(9, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(9, 2), Point::new(60.0, 10.0)),
        ],
        false,
    );
    session.set_tool(Tool::Measure);
    (session, path)
}

fn click(session: &mut Session, at: Point, shift: bool) {
    session.modifiers_changed(shift, false, false);
    session.pointer_hover(at, shift, false);
    session.pointer_down(at, shift);
    session.pointer_up(at, shift, false);
}

/// The path, a 50 mm dimension of it, the Select tool.
fn placed() -> (Session, NodeId, NodeId) {
    let (mut session, path) = session_with_line();
    session.selection.select_single(path);
    assert!(session.measure_enter());
    let dimension = session
        .document
        .read_objects()
        .objects
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Dimension(d) => Some(d.id),
            _ => None,
        })
        .unwrap();
    session.set_tool(Tool::Select);
    (session, path, dimension)
}

fn row(session: &mut Session, id: NodeId) -> serde_json::Value {
    let rows: serde_json::Value = serde_json::from_str(&session.layers_view_json(0, 20)).unwrap();
    rows["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == serde_json::to_value(id).unwrap())
        .unwrap()
        .clone()
}

/// Review item 1: the row shows the value without the word "Dimension" (the Ruler glyph names
/// the kind), "· not linked" when unlinked; the name keeps the full words.
#[test]
fn a_dimension_row_shows_its_value_and_keeps_the_full_name() {
    let (mut session, path, dimension) = placed();
    let shown = row(&mut session, dimension);
    assert_eq!(shown["text"], "50 mm");
    assert_eq!(shown["name"], "Dimension 50 mm");
    session.selection.select_single(path);
    session.delete_selected();
    let shown = row(&mut session, dimension);
    assert_eq!(shown["text"], "50 mm · not linked");
    assert_eq!(shown["name"], "Dimension 50 mm not linked");
    assert!(
        shown["label"]
            .as_str()
            .unwrap()
            .starts_with("Dimension 50 mm")
    );
}

/// Every other row shows its name.
#[test]
fn another_row_shows_its_name() {
    let (mut session, path, _) = placed();
    let shown = row(&mut session, path);
    assert_eq!(shown["text"], shown["name"]);
}

/// Review item 3: a selection of dimensions only offers no Stroke and Fill, no Linked clone, no
/// Stroke to path and no scale switches; a mixed selection keeps them.
#[test]
fn a_selection_of_dimensions_only_offers_no_style_or_clone_controls() {
    let (mut session, path, dimension) = placed();
    session.selection.select_single(dimension);
    assert!(!session.style_panel_view().sections_shown);
    assert!(!session.linked_clone_shown());
    assert!(!session.stroke_to_path_shown());
    let bar = session.select_bar_state();
    assert!(!bar.scale_switches_shown);
    assert!(bar.radius.is_none());
    session.selection.set(&[path, dimension]);
    assert!(session.style_panel_view().sections_shown);
    assert!(session.linked_clone_shown());
    assert!(session.stroke_to_path_shown());
    assert!(session.select_bar_state().scale_switches_shown);
}

/// Review item 3: a dimension does not count for the Radius field of a mixed selection.
#[test]
fn a_dimension_does_not_count_for_the_radius_field() {
    let (mut session, _, dimension) = placed();
    let rect = session.document.create_rect(RectBounds::from_corners(
        Point::new(100.0, 100.0),
        Point::new(140.0, 120.0),
    ));
    session.selection.select_single(rect);
    let alone = session.select_bar_state();
    session.selection.set(&[rect, dimension]);
    let mixed = session.select_bar_state();
    assert_eq!(mixed.radius, alone.radius);
    assert_eq!(mixed.radius_corners, alone.radius_corners);
    assert_eq!(mixed.remove_rounding_shown, alone.remove_rounding_shown);
}

/// Review item 6: with Shift held after the first pick the dimension preview keeps the anchor
/// dot of the first pick.
#[test]
fn the_shift_preview_keeps_the_first_anchor_dot() {
    let (mut session, _) = session_with_line();
    click(&mut session, Point::new(10.0, 10.0), false);
    session.modifiers_changed(true, false, false);
    session.pointer_hover(Point::new(40.0, 30.0), true, false);
    let objects = session.objects();
    let layer = session.dimension_layer(&objects);
    let dot = curvyo_render_core::build_measure_dots(&[Point::new(10.0, 10.0)], session.view());
    assert!(
        layer
            .triangles
            .windows(dot.triangles.len())
            .any(|w| w == dot.triangles.as_slice()),
        "the A dot is drawn"
    );
}
