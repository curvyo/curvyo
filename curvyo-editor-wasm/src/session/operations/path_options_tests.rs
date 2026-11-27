//! Session tests of `specs/0139` slice 2: the Path options group's edits (criterion 13), the item
//! part's Path options pill, own fields, Start and Direction (criteria 19 to 21, 23), the start
//! markers (criterion 19, Frage 415 C) and the job's marks (adrs L12).

use curvyo_document_core::{AnchorId, NewAnchor, NodeId, Point};

use super::super::Session;
use super::tools_tests::{Fx, fixture, undo_label};

fn cut_with(fx: &mut Fx, members: &[NodeId]) -> NodeId {
    let manufacturing = fx.s.active_manufacturing().unwrap().id;
    let keys: Vec<Vec<NodeId>> = members.iter().map(|&m| vec![m]).collect();
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
                &keys,
            )
            .unwrap()
    });
    fx.s.selection.set(members);
    op
}

fn marks(s: &Session) -> Vec<curvyo_laser_core::job::CutMark> {
    let job = s.laser_job().unwrap();
    job.built
        .plans
        .iter()
        .flatten()
        .flat_map(|plan| plan.marks.clone())
        .collect()
}

fn at(s: &Session, op: NodeId, arc_mm: f64) -> Point {
    let outline = s.tab_outlines(Some(op)).remove(0);
    let (x, y) = outline.measured.point_at(arc_mm * 1000.0);
    Point::new(x / 1000.0 + outline.origin.x, y / 1000.0 + outline.origin.y)
}

#[test]
fn the_path_options_group_edits_each_field_as_edit_settings_and_refuses_out_of_range() {
    let mut fx = fixture(false);
    let a = fx.a;
    let op = cut_with(&mut fx, &[a]);
    assert_eq!(fx.s.edit_operation_option(op, "lead_in", 2.0), Ok(true));
    assert_eq!(undo_label(&mut fx.s), "Edit settings");
    assert_eq!(
        fx.s.edit_operation_option(op, "lead_in_mm", 0.05),
        Err("Lead-in is 0.10 to 20.00 mm.".to_owned())
    );
    assert_eq!(
        fx.s.edit_operation_option(op, "lead_out_mm", 21.0),
        Err("Lead-out is 0.10 to 20.00 mm.".to_owned())
    );
    assert_eq!(
        fx.s.edit_operation_option(op, "overcut", 10.5),
        Err("Overcut is 0.00 to 10.00 mm.".to_owned())
    );
    assert_eq!(fx.s.edit_operation_option(op, "overcut", 1.0), Ok(true));
    let view = super::tools_tests::row(&fx.s, "Cut 1");
    let path = view.settings.unwrap().options.unwrap().path.unwrap();
    assert_eq!((path.lead_in, path.overcut_mm), ("arc", 1.0));
    let marks = marks(&fx.s);
    assert_eq!(marks.len(), 1);
    assert_ne!(marks[0].lead_in.len(), 0, "the Operations view's lead");
    assert_ne!(marks[0].overcut.len(), 0);
    assert_eq!(marks[0].start, None, "Automatic draws no chevron");
}

#[test]
fn the_path_options_pill_copies_edits_and_returns_each_one_step() {
    let mut fx = fixture(false);
    let a = fx.a;
    let op = cut_with(&mut fx, &[a]);
    fx.s.edit_operation_option(op, "lead_out", 1.0).unwrap();
    let before = fx.s.laser_job().unwrap().built.job.clone();
    let view = fx.s.item_path_view(op).unwrap();
    let options = view.options.unwrap();
    assert_eq!(options.own, Some(false));
    assert_eq!(
        options.summary.as_deref(),
        Some("Lead-in None · Lead-out Line 1 mm · Overcut 0 mm")
    );
    assert_eq!(view.direction, None, "no open path selected");
    assert!(fx.s.set_selection_path_own(op, true));
    assert_eq!(undo_label(&mut fx.s), "Own path options");
    assert_eq!(fx.s.laser_job().unwrap().built.job, before, "Own copies");
    assert_eq!(
        fx.s.edit_selection_path_option(op, "overcut", 2.0),
        Ok(true)
    );
    assert_eq!(undo_label(&mut fx.s), "Edit settings");
    let own = fx.s.item_path_view(op).unwrap().options.unwrap();
    assert_eq!((own.own, own.values.unwrap().overcut_mm), (Some(true), 2.0));
    assert_ne!(fx.s.laser_job().unwrap().built.job, before);
    assert!(fx.s.set_selection_path_own(op, false));
    assert_eq!(undo_label(&mut fx.s), "Back to the operation's");
    assert_eq!(fx.s.laser_job().unwrap().built.job, before);
}

#[test]
fn a_start_marker_shows_moves_and_returns_to_automatic_each_one_step() {
    let mut fx = fixture(false);
    let a = fx.a;
    let op = cut_with(&mut fx, &[a]);
    assert_eq!(
        fx.s.start_markers().len(),
        0,
        "Automatic shows only with the Tabs tool"
    );
    assert!(fx.s.set_tabs_tool(true));
    let markers = fx.s.start_markers();
    assert_eq!(markers.len(), 1);
    assert!(!markers[0].2, "an Automatic marker");
    let from = markers[0].0.to_document(markers[0].1);
    assert!(fx.s.tab_press(from));
    let to = at(&fx.s, op, 15.0);
    assert!(fx.s.tab_drag_move(to));
    assert!(fx.s.tab_drag_end(to));
    assert_eq!(undo_label(&mut fx.s), "Move start point");
    let start = fx.s.item_path_view(op).unwrap().start.unwrap();
    assert_eq!((start.readout.as_str(), start.set), ("Set", true));
    let marks = marks(&fx.s);
    let (entered, _) = marks[0].start.unwrap();
    let expected = fx.s.tab_outlines(Some(op))[0].measured.point_um(15_000.0);
    assert!((entered.x - expected.x).abs() < 5 && (entered.y - expected.y).abs() < 5);
    assert!(fx.s.set_tabs_tool(false));
    assert_eq!(
        fx.s.start_markers().len(),
        1,
        "a set marker shows without the tool"
    );
    // Delete on the current set marker is Automatic start (C8).
    let marker = fx.s.start_markers().remove(0);
    assert!(fx.s.tab_press(marker.0.to_document(marker.1)));
    assert!(fx.s.tab_drag_end(marker.0.to_document(marker.1)));
    assert!(fx.s.delete_current_start());
    assert_eq!(undo_label(&mut fx.s), "Automatic start");
    assert!(!fx.s.item_path_view(op).unwrap().start.unwrap().set);
}

#[test]
fn a_start_marker_stops_at_a_tab_edge_and_the_automatic_button_clears_it() {
    let mut fx = fixture(false);
    let a = fx.a;
    let op = cut_with(&mut fx, &[a]);
    fx.s.edit_operation_option(op, "tab_count", 4.0).unwrap();
    fx.s.edit_operation_option(op, "tab_width", 2.0).unwrap();
    assert!(fx.s.set_tabs_tool(true));
    let marker = fx.s.start_markers().remove(0);
    let from = marker.0.to_document(marker.1);
    assert!(fx.s.tab_press(from));
    // Tabs are centred at 5, 15, 25 and 35 mm: 15.3 mm lies in the second, nearer its end.
    let into = at(&fx.s, op, 15.3);
    assert!(fx.s.tab_drag_end(into));
    let outline = fx.s.tab_outlines(Some(op)).remove(0);
    let Some(curvyo_laser_core::tabs::OutlineEntry::Start(f)) = outline.entry else {
        panic!("a set start");
    };
    assert!(
        (outline.measured.arc_of(f) - 16_000.0).abs() < 1.0,
        "at the tab's end"
    );
    assert!(fx.s.set_selection_start_automatic(op));
    assert_eq!(undo_label(&mut fx.s), "Automatic start");
    assert!(
        !fx.s.set_selection_start_automatic(op),
        "nothing left to clear"
    );
}

#[test]
fn the_direction_pick_fixes_an_open_path_in_one_step() {
    let mut fx = fixture(false);
    let origin = fx.origin;
    let line = fx.s.recording_unguarded(|this| {
        this.document.create_path(
            &[
                NewAnchor::corner(
                    AnchorId::new(9, 1),
                    Point::new(origin.x + 10.0, origin.y + 50.0),
                ),
                NewAnchor::corner(
                    AnchorId::new(9, 2),
                    Point::new(origin.x + 60.0, origin.y + 50.0),
                ),
            ],
            false,
        )
    });
    let op = cut_with(&mut fx, &[line]);
    let view = fx.s.item_path_view(op).unwrap();
    assert_eq!(view.direction.unwrap().value, Some("automatic"));
    assert_eq!(view.options, None, "no closed outline selected");
    assert!(fx.s.set_selection_direction(op, "reversed"));
    assert_eq!(undo_label(&mut fx.s), "Direction");
    assert_eq!(
        fx.s.item_path_view(op).unwrap().direction.unwrap().value,
        Some("reversed")
    );
    let job = fx.s.laser_job().unwrap();
    let plan = job.built.plans.iter().flatten().next().unwrap();
    let path = &plan.runs[0].paths[0];
    assert!(path.points[0].x > path.points[1].x, "from its last point");
    assert!(fx.s.set_selection_direction(op, "automatic"));
    assert_eq!(
        fx.s.document
            .operation_records(op, curvyo_document_core::ItemRecord::Entry)
            .unwrap()
            .len(),
        0,
        "Automatic deletes the register"
    );
}
