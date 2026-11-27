//! Session tests of `specs/0139` slice 1: the Tabs group's edits (criterion 1), the job's gaps
//! from the item's record (criteria 4, 5, 8), the Tabs tool and handle steps (5, 6, 12) and the
//! Tabs pill of the item part.

use curvyo_document_core::{NodeId, Point};

use super::super::Session;
use super::tools_tests::{Fx, fixture, op_named, undo_label};

fn cut_with_a(fx: &mut Fx) -> NodeId {
    let manufacturing = fx.s.active_manufacturing().unwrap().id;
    let a = fx.a;
    let op = fx.s.recording_unguarded(|this| {
        this.document
            .add_operation(
                manufacturing,
                &curvyo_document_core::NewOperation {
                    kind: "cut",
                    name: "Cut 1",
                    settings: None,
                },
                0,
                &[vec![a]],
            )
            .unwrap()
    });
    fx.s.selection.set(&[a]);
    op
}

fn gaps(s: &Session) -> usize {
    let job = s.laser_job().unwrap();
    job.built
        .plans
        .iter()
        .flatten()
        .flat_map(|plan| &plan.marks)
        .map(|mark| mark.gaps.len())
        .sum()
}

fn record_tabs(s: &Session, op: NodeId) -> usize {
    s.document
        .operation_tabs(op)
        .unwrap()
        .values()
        .filter_map(|v| curvyo_laser_core::tabs::read_own_tabs(v).ok())
        .map(|own| own.count())
        .sum()
}

fn at(s: &Session, op: NodeId, arc_mm: f64) -> Point {
    let outline = s.tab_outlines(Some(op)).remove(0);
    let (x, y) = outline.measured.point_at(arc_mm * 1000.0);
    Point::new(x / 1000.0 + outline.origin.x, y / 1000.0 + outline.origin.y)
}

#[test]
fn the_tabs_group_edits_count_width_and_skip_as_edit_settings_and_refuses_out_of_range() {
    let mut fx = fixture(false);
    let op = cut_with_a(&mut fx);
    assert_eq!(fx.s.edit_operation_option(op, "tab_count", 4.0), Ok(true));
    assert_eq!(undo_label(&mut fx.s), "Edit settings");
    assert_eq!(
        fx.s.edit_operation_option(op, "tab_count", 21.0),
        Err("Count is 0 to 20.".to_owned())
    );
    assert_eq!(
        fx.s.edit_operation_option(op, "tab_width", 0.09),
        Err("Width is 0.10 to 20.00 mm.".to_owned())
    );
    assert_eq!(
        fx.s.edit_operation_option(op, "tab_skip_inner", 1.0),
        Ok(true)
    );
    let options = op_named(&fx.s, "Cut 1").options.unwrap();
    assert_eq!(options["tab_count"], 4);
    assert_eq!(gaps(&fx.s), 4, "a 40 mm square gets its four tabs");
    let view = super::tools_tests::row(&fx.s, "Cut 1");
    let tabs = view.settings.unwrap().options.unwrap().tabs.unwrap();
    assert_eq!((tabs.count, tabs.skip_inner, tabs.placed), (4, true, None));
}

#[test]
fn the_tabs_tool_adds_moves_deletes_and_widens_a_tab_each_one_step() {
    let mut fx = fixture(false);
    let op = cut_with_a(&mut fx);
    fx.s.edit_operation_option(op, "tab_count", 4.0).unwrap();
    assert!(fx.s.tabs_tool_shown());
    assert!(fx.s.set_tabs_tool(true));
    let before = fx.s.laser_job().unwrap().built.job.clone();
    // Between two automatic tabs (centres at 5, 15, 25 and 35 mm of the 40 mm outline).
    let point = at(&fx.s, op, 10.0);
    assert!(fx.s.tab_press(point));
    assert_eq!(undo_label(&mut fx.s), "Add tab");
    assert_eq!(
        record_tabs(&fx.s, op),
        5,
        "own tabs: the four shown and the new one"
    );
    assert_eq!(gaps(&fx.s), 5);
    assert_eq!(
        fx.s.set_current_tab_width(Some(0.05)),
        Err("Width is 0.10 to 20.00 mm.".to_owned())
    );
    assert_eq!(
        fx.s.set_current_tab_width(Some(9.0)),
        Err("No room for this width.".to_owned())
    );
    assert_eq!(fx.s.set_current_tab_width(Some(1.0)), Ok(true));
    assert_eq!(undo_label(&mut fx.s), "Tab width");
    assert!(fx.s.current_tab_chip().unwrap().own_width);
    // A drag towards its neighbour stops at 1 mm of burnt piece.
    let from = fx.s.current_tab_chip().unwrap().at;
    assert!(fx.s.tab_press(from));
    assert!(fx.s.tab_drag_end(at(&fx.s, op, 14.0)));
    assert_eq!(undo_label(&mut fx.s), "Move tab");
    assert!(fx.s.delete_current_tab());
    assert_eq!(undo_label(&mut fx.s), "Delete tab");
    assert_eq!(record_tabs(&fx.s, op), 4);
    let view = fx.s.item_tabs_view(op).unwrap();
    assert_eq!(
        (view.own, view.readout.as_str()),
        (Some(true), "Own tabs (4)")
    );
    // Back to the operation's: the job is the operation's again, byte for byte.
    assert!(fx.s.set_selection_tabs_own(op, false));
    assert_eq!(undo_label(&mut fx.s), "Back to the operation's");
    assert_eq!(fx.s.laser_job().unwrap().built.job, before);
    assert!(fx.s.set_selection_tabs_own(op, true));
    assert_eq!(undo_label(&mut fx.s), "Own tabs");
    assert_eq!(
        fx.s.laser_job().unwrap().built.job,
        before,
        "Own copies what it shows"
    );
    assert!(
        fx.s.tabs_escape(),
        "Escape clears the current handle or the tool"
    );
}

/// A point `px` screen pixels from `at` along x.
fn off(s: &Session, at: Point, px: f64) -> Point {
    Point::new(at.x + px / s.view().scale(), at.y)
}

#[test]
fn a_click_on_a_handle_off_its_centre_selects_it_and_changes_nothing() {
    let mut fx = fixture(false);
    let op = cut_with_a(&mut fx);
    fx.s.edit_operation_option(op, "tab_count", 4.0).unwrap();
    // The first automatic tab of the top side sits at 5 mm along the 40 mm outline.
    let centre = at(&fx.s, op, 5.0);
    let press = off(&fx.s, centre, 6.0);
    assert!(fx.s.tab_press(press));
    assert!(
        fx.s.tab_drag_move(off(&fx.s, press, 2.0)),
        "within the 3 px dead zone"
    );
    assert!(fx.s.tab_drag_end(off(&fx.s, press, 2.0)));
    assert_eq!(undo_label(&mut fx.s), "Edit settings", "no Move tab step");
    assert_eq!(record_tabs(&fx.s, op), 0, "the tabs stay the operation's");
    assert!(fx.s.current_tab_chip().is_some(), "the handle is current");
}

#[test]
fn escape_during_a_tab_drag_cancels_it() {
    let mut fx = fixture(false);
    let op = cut_with_a(&mut fx);
    fx.s.edit_operation_option(op, "tab_count", 4.0).unwrap();
    let centre = at(&fx.s, op, 5.0);
    assert!(fx.s.tab_press(centre));
    let away = at(&fx.s, op, 7.0);
    assert!(fx.s.tab_drag_move(away));
    let _ = fx.s.escape();
    assert!(!fx.s.tab_drag_end(away), "no drag is held after Escape");
    assert_eq!(undo_label(&mut fx.s), "Edit settings", "nothing committed");
    assert_eq!(record_tabs(&fx.s, op), 0);
}

#[test]
fn the_tabs_tool_shows_the_crosshair_over_a_selected_cut_path() {
    let mut fx = fixture(false);
    let op = cut_with_a(&mut fx);
    fx.s.edit_operation_option(op, "tab_count", 4.0).unwrap();
    let between = at(&fx.s, op, 10.0);
    fx.s.pointer_hover(between, false, false);
    assert_ne!(
        fx.s.cursor_hint(),
        "crosshair",
        "only with the Tabs tool on"
    );
    assert!(fx.s.set_tabs_tool(true));
    fx.s.pointer_hover(between, false, false);
    assert_eq!(fx.s.cursor_hint(), "crosshair");
    fx.s.pointer_hover(at(&fx.s, op, 5.0), false, false);
    assert_eq!(fx.s.cursor_hint(), "pointer", "a handle");
}
