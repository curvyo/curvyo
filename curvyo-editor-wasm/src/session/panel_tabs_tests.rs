//! Tests of the Properties panel tab glue.

use curvyo_document_core::{Length, NodeId, Point, RectBounds};

use super::super::document::{DocumentSide, SizeOutcome};
use super::*;

fn rect(session: &Session, x: f64) -> NodeId {
    session.document.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn active(session: &mut Session) -> String {
    PanelTabsRecord::new(&session.panel_view()).active
}

/// The index of the tab called `name` in the record; tests find a tab by its name, never by
/// its position (`0051` criterion 45).
fn at(record: &PanelTabsRecord, name: &str) -> usize {
    record.tab_names.iter().position(|n| n == name).unwrap()
}

/// Criteria 5 and 6: a new session shows Document; a selection brings
/// Style, clearing it brings Document back.
#[test]
fn the_tab_follows_the_selection_edges() {
    let mut session = Session::new(1);
    assert_eq!(active(&mut session), "document");
    let id = rect(&session, 0.0);
    session.selection.select_single(id);
    assert_eq!(active(&mut session), "style");
    assert_eq!(session.panel_content(), PanelContent::Style);
    session.selection.clear();
    assert_eq!(active(&mut session), "document");
}

/// Criterion 8: a pressed Document stays through a change of selection and
/// gives way to Style at the next empty to non-empty edge.
#[test]
fn a_pressed_document_tab_is_sticky_until_the_selection_empties() {
    let mut session = Session::new(1);
    let (a, b) = (rect(&session, 0.0), rect(&session, 30.0));
    session.selection.select_single(a);
    assert_eq!(active(&mut session), "style");
    assert!(session.press_panel_tab("document"));
    assert_eq!(session.panel_content(), PanelContent::Document);
    session.selection.select_single(b);
    assert_eq!(active(&mut session), "document");
    session.selection.clear();
    assert_eq!(active(&mut session), "document");
    session.selection.select_single(a);
    assert_eq!(active(&mut session), "style");
}

/// Criterion 4: Style cannot be pressed with nothing selected.
#[test]
fn the_dimmed_style_tab_does_nothing() {
    let mut session = Session::new(1);
    assert!(!session.press_panel_tab("style"));
    assert!(!session.press_panel_tab("unknown"));
    let record = PanelTabsRecord::new(&session.panel_view());
    assert_eq!(
        record.tab_names,
        [
            "file", "document", "style", "layers", "history", "fonts", "snapping", "grid", "share"
        ]
    );
    assert_eq!(record.tab_enabled[at(&record, "style")], 0);
    assert!(
        ["file", "document", "history"]
            .iter()
            .all(|name| record.tab_enabled[at(&record, name)] == 1)
    );
    assert_eq!(
        record.tab_tooltips[at(&record, "style")],
        "Style: select an object first"
    );
    assert_eq!(record.active, "document");
}

/// Criterion 4: the dimmed Style tab says why, also when something is
/// selected but the tool cannot style it.
#[test]
fn the_dimmed_style_tooltip_names_the_reason() {
    let mut session = Session::new(1);
    let id = rect(&session, 0.0);
    session.selection.select_single(id);
    session.set_tool(Tool::Node);
    let tip = |s: &Session| {
        let view = s.panel_view();
        view.tabs
            .iter()
            .find(|t| t.name == "style")
            .unwrap()
            .tooltip
    };
    assert_eq!(tip(&session), "Style: the Node tool styles paths only");
    session.set_tool(Tool::Pen);
    assert_eq!(tip(&session), "Style: the Pen tool has nothing to style");
    session.set_tool(Tool::Select);
    assert_eq!(tip(&session), "Style (Shift+Ctrl+F)");
    session.selection.clear();
    assert_eq!(tip(&session), "Style: select an object first");
}

/// Criterion 14: the two shortcuts.
#[test]
fn the_shortcuts_pick_style_or_document() {
    let mut session = Session::new(1);
    session.panel_shortcut_style();
    assert_eq!(active(&mut session), "document");
    let id = rect(&session, 0.0);
    session.selection.select_single(id);
    assert!(session.press_panel_tab("document"));
    session.panel_shortcut_style();
    assert_eq!(active(&mut session), "style");
    session.panel_shortcut_document();
    assert_eq!(active(&mut session), "document");
}

/// The Document body and strip remain visible during an unfinished Pen path.
#[test]
fn an_unfinished_pen_path_keeps_the_document_and_strip() {
    let mut session = Session::new(1);
    session.set_tool(Tool::Pen);
    for point in [Point::new(10.0, 10.0), Point::new(50.0, 40.0)] {
        session.pointer_down(point, false);
        session.pointer_up(point, false, false);
    }
    let view = session.panel_view();
    assert_eq!(view.content, PanelContent::Document);
    for name in ["file", "document", "style", "history"] {
        assert!(view.tabs.iter().any(|tab| tab.name == name), "{name}");
    }
    assert!(session.press_panel_tab("document"));
    assert_eq!(session.panel_content(), PanelContent::Document);
    session.finish_pen();
    assert_eq!(session.panel_content(), PanelContent::Document);
}

/// Criterion 6 with the Pen (`adrs.md` decision 2): switching to the Pen
/// with a selection is an edge to empty, so Style hands over to Document.
#[test]
fn switching_to_the_pen_with_a_selection_shows_the_document() {
    let mut session = Session::new(1);
    let id = rect(&session, 0.0);
    session.selection.select_single(id);
    assert_eq!(active(&mut session), "style");
    session.set_tool(Tool::Pen);
    assert_eq!(session.panel_content(), PanelContent::Document);
}

/// Criterion 24: the document can be resized with an object selected, and
/// the selection stays.
#[test]
fn the_size_is_editable_while_an_object_stays_selected() {
    let mut session = Session::new(1);
    let id = rect(&session, 10.0);
    session.selection.select_single(id);
    assert!(session.press_panel_tab("document"));
    assert_eq!(session.panel_content(), PanelContent::Document);
    assert_eq!(
        session.set_document_side(DocumentSide::Width, "300"),
        SizeOutcome::Committed
    );
    assert_eq!(session.selection.ids(), [id]);
    assert_eq!(session.panel_content(), PanelContent::Document);
}

/// `0051` criterion 14: the File tab is sticky. A selection change neither makes another tab
/// active nor leaves File; a fresh session (New, Open) starts at Document.
#[test]
fn the_file_tab_stays_through_selection_changes_and_a_new_session_leaves_it() {
    let mut session = Session::new(1);
    let id = rect(&session, 0.0);
    assert!(session.press_panel_tab("file"));
    assert_eq!(session.panel_content(), PanelContent::File);
    session.selection.select_single(id);
    assert_eq!(active(&mut session), "file");
    session.selection.clear();
    assert_eq!(active(&mut session), "file");
    // The tab is pressable with an empty scope and has no tooltip but its name.
    let record = PanelTabsRecord::new(&session.panel_view());
    assert_eq!(record.tab_tooltips[at(&record, "file")], "File");
    let mut fresh = Session::new(2);
    assert_eq!(active(&mut fresh), "document");
}

/// Criterion 11: a new session starts over.
#[test]
fn a_new_session_resets_the_choice() {
    let mut session = Session::new(1);
    let id = rect(&session, 0.0);
    session.selection.select_single(id);
    assert!(session.press_panel_tab("document"));
    let mut fresh = Session::new(2);
    assert_eq!(active(&mut fresh), "document");
}
