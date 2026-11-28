//! Session tests of `specs/0161-cnc-output` slice 1: Add page, profile, stock, tools, Profile
//! outside and inside, the checks, the Toolpath view and its time, Export and Send.
#![allow(clippy::many_single_char_names)]

use curvyo_cnc_core::{read_record, write_record};
use curvyo_connector_core::virtual_grbl::plan_time;
use curvyo_document_core::{Direction, DocumentSize, NodeId, Point, RectBounds};
use serde_json::Value;

use super::super::Session;
use super::job::grbl_settings;

const CNC: &str = "curvyo.cnc";

fn cnc() -> Session {
    let mut s = Session::new(161_001);
    s.resize_viewport(1200.0, 800.0);
    s.add_page_of_kind(CNC, None).unwrap();
    s
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.cnc_view_json()).unwrap()
}

/// A rectangle from `(x, y)` page mm, `w` x `h` mm.
fn rect(s: &mut Session, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    let o = s.active_cnc().unwrap().page.origin;
    s.document.create_rect(RectBounds::from_corners(
        Point::new(o.x + x, o.y + y),
        Point::new(o.x + x + w, o.y + y + h),
    ))
}

fn assign(s: &mut Session, side: &str, items: &[NodeId]) -> NodeId {
    s.selection.set(items);
    let id = s.press_cnc_profile(side, false).unwrap();
    s.selection.set(&[]);
    id
}

fn undo_name(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}

fn checks(s: &Session) -> Vec<String> {
    view(s)["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            format!(
                "{}: {}",
                c["severity"].as_str().unwrap(),
                c["text"].as_str().unwrap()
            )
        })
        .collect()
}

#[test]
fn criterion_1_2_add_page_is_the_work_area_with_the_generic_grbl_router() {
    let mut s = Session::new(161_002);
    let steps = s.undo_len();
    s.add_page_of_kind(CNC, None).unwrap();
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_name(&mut s), "Add CNC page");
    let page = s.active_cnc().unwrap();
    assert_eq!(page.page.size, DocumentSize::from_mm(300.0, 300.0));
    let v = view(&s);
    assert_eq!(v["profile"]["name"], "Generic GRBL router");
    assert_eq!(v["profile"]["output"], "GRBL");
    assert_eq!(v["profile"]["safeHeight"], 5.0);
    assert!(s.active_manufacturing().is_none(), "no laser tools");
    let before = s.active_cnc().unwrap().page.size;
    let _ = s.set_document_side(crate::session::DocumentSide::Width, "200");
    assert_eq!(s.active_cnc().unwrap().page.size, before, "not editable");
}

#[test]
fn criterion_2_save_as_profile_writes_a_cnc_record_with_the_connector() {
    let mut s = cnc();
    let id = s.save_cnc_profile([7; 16], 1_700_000_000_000).unwrap();
    // ADR 0098: the connector address is a Machines-tab field of the record.
    assert_eq!(
        s.set_machine_value(&id, "connector", "127.0.0.1:7447", 1_700_000_000_001),
        Ok(None)
    );
    let record = s.manufacturing.cnc.find(&id).unwrap().clone();
    let text = write_record(&record);
    assert!(text.contains("\"kind\": \"cnc\""));
    let read = read_record(&curvyo_laser_core::record_file_name(&id), &text).unwrap();
    assert_eq!(read.values.connection.connector_address, "127.0.0.1:7447");
    assert_eq!(view(&s)["profile"]["connector"], "127.0.0.1:7447");
}

#[test]
fn criterion_3_the_stock_defaults_refuses_beyond_the_machine_and_edits_in_one_step() {
    let mut s = cnc();
    let v = view(&s);
    assert_eq!(v["stock"]["width"], 200.0);
    assert_eq!(v["stock"]["thickness"], 10.0);
    assert_eq!(v["stock"]["material"], "Plywood");
    assert_eq!(
        s.edit_cnc_stock("width_mm", "400"),
        Err("Width is 1 to 300 mm.".to_owned())
    );
    let steps = s.undo_len();
    assert_eq!(s.edit_cnc_stock("thickness_mm", "6"), Ok(true));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_name(&mut s), "Edit stock");
    assert_eq!(view(&s)["stock"]["thickness"], 6.0);
}

#[test]
fn criterion_4_one_default_tool_ranges_and_one_tool_per_job() {
    let mut s = cnc();
    let v = view(&s);
    assert_eq!(v["tools"][0]["name"], "Flat 3.175 mm");
    assert_eq!(
        v["tools"][0]["line2"],
        "2 flutes · 800 mm/min · 12,000 rpm · 1 mm down"
    );
    let first = v["tools"][0]["id"].as_str().unwrap().to_owned();
    let second = s.add_cnc_tool([3; 16]).unwrap();
    assert_eq!(undo_name(&mut s), "Add tool");
    assert!(s.edit_cnc_tool(&second, "diameter_mm", "30").is_err());
    assert_eq!(s.edit_cnc_tool(&second, "diameter_mm", "6"), Ok(true));
    let a = rect(&mut s, 20.0, 20.0, 40.0, 30.0);
    let b = rect(&mut s, 100.0, 20.0, 40.0, 30.0);
    assign(&mut s, "outside", &[a]);
    s.selection.set(&[b]);
    let op2 = s.press_cnc_profile("outside", true).unwrap();
    assert_eq!(s.edit_cnc_setting(op2, "tool", &second), Ok(true));
    assert!(
        checks(&s)
            .contains(&"block: Profile 2 uses another tool. One tool per job for now.".to_owned())
    );
    assert!(s.cnc_export().is_none());
    // Deleting the used tool moves Profile 2 to the first tool.
    assert!(s.remove_cnc_tool(&second));
    assert_eq!(undo_name(&mut s), "Remove tool");
    let v = view(&s);
    assert_eq!(v["operations"][1]["numbers"]["tool"], first.as_str());
    assert!(!s.remove_cnc_tool(&first), "the last tool stays");
}

#[test]
fn criterion_5_profile_has_side_depth_tool_direction_and_the_tools_feeds() {
    let mut s = cnc();
    let a = rect(&mut s, 20.0, 20.0, 50.0, 30.0);
    let op = assign(&mut s, "outside", &[a]);
    assert_eq!(undo_name(&mut s), "Assign to Profile 1");
    let v = view(&s);
    let n = &v["operations"][0]["numbers"];
    assert_eq!(n["side"], "outside");
    assert_eq!(n["direction"], "conventional");
    assert_eq!(n["depth_mm"], 10.0);
    assert_eq!(n["feed_mm_min"], 800.0);
    assert_eq!(
        v["operations"][0]["line3"],
        "Ø 3.175 mm, 10 mm deep, 10 passes, 800 mm/min"
    );
    assert_eq!(
        s.edit_cnc_setting(op, "depth_mm", "11"),
        Err("Depth is 0.1 to 10.0 mm.".to_owned())
    );
    assert_eq!(s.edit_cnc_setting(op, "feed_mm_min", "600"), Ok(true));
    assert_eq!(undo_name(&mut s), "Edit settings");
    let v = view(&s);
    assert!(
        v["operations"][0]["line3"]
            .as_str()
            .unwrap()
            .ends_with("600 mm/min typed")
    );
}

#[test]
fn criteria_7_10_too_narrow_and_outside_the_stock_block_export() {
    let mut s = cnc();
    let slot = rect(&mut s, 20.0, 20.0, 30.0, 2.0);
    assign(&mut s, "inside", &[slot]);
    assert!(
        checks(&s).contains(&"block: Profile 1: 1 item is too narrow for this tool.".to_owned())
    );
    assert!(s.cnc_export().is_none());
    let mut s = cnc();
    let off = rect(&mut s, 180.0, 20.0, 50.0, 30.0);
    assign(&mut s, "outside", &[off]);
    assert!(checks(&s).contains(&"block: 1 item lies outside the stock.".to_owned()));
    assert!(s.cnc_export().is_none());
}

#[test]
fn criteria_11_12_14_18_19_the_view_the_file_and_the_send_agree() {
    let mut s = cnc();
    let a = rect(&mut s, 20.0, 20.0, 50.0, 30.0);
    assign(&mut s, "outside", &[a]);
    assert_eq!(s.edit_cnc_stock("thickness_mm", "3"), Ok(true));
    let steps = s.undo_len();
    let job = s.export_job("Parts.curvyo").unwrap();
    assert_eq!(job.name, "Parts - CNC 1.nc");
    assert_eq!(job.extension, "nc");
    assert_eq!(s.undo_len(), steps, "no step");
    assert_eq!(
        s.export_job("Parts.curvyo").unwrap().bytes,
        job.bytes,
        "byte-identical"
    );
    let text = String::from_utf8(job.bytes.clone()).unwrap();
    assert!(text.contains("G21 G90 G17 G94\nM3 S12000\nG0 Z5\n"));
    let values = s.active_cnc().unwrap().copy.values;
    let planned = plan_time(&text, &grbl_settings(&values)).total.get();
    let v = view(&s);
    assert_eq!(v["play"]["length"].as_f64().unwrap(), planned);
    assert!(v["total"]["time"].as_str().unwrap().starts_with("About "));
    // The Toolpath view draws the moves.
    let page_id = s.active_cnc().unwrap().page.id;
    assert!(s.toggle_toolpath_view(&page_id));
    assert!(
        s.cnc_toolpath(s.view())
            .is_some_and(|l| !l.triangles.is_empty())
    );
    let readout: Value = serde_json::from_str(&s.set_cnc_play(planned / 2.0)).unwrap();
    assert!(readout["text"].as_str().unwrap().contains(" · Z "));
    // Send needs a saved profile with a connector address; it sends Export's bytes.
    assert!(s.cnc_send_job().is_none());
    let id = s.save_cnc_profile([9; 16], 1).unwrap();
    s.set_machine_value(&id, "connector", "127.0.0.1:7447", 2)
        .unwrap();
    let sent = s.cnc_send_job().unwrap();
    assert_eq!(sent.bytes, job.bytes);
    assert_eq!(sent.address, "127.0.0.1:7447");
}

#[test]
fn criterion_6_inner_holes_run_before_their_outline_in_the_file() {
    let mut s = cnc();
    let outer = rect(&mut s, 20.0, 20.0, 60.0, 60.0);
    let inner = rect(&mut s, 40.0, 40.0, 20.0, 20.0);
    assign(&mut s, "outside", &[outer]);
    assign(&mut s, "inside", &[inner]);
    let page = s.active_cnc().unwrap();
    let job = s.cnc_job(&page);
    assert_eq!(job.job.reports.len(), 2);
    assert!(job.job.reports.iter().all(|r| r.loops == 1));
}

/// UX unify 2.2 (U15): the stock carries its label in its origin corner while it is 80 device px
/// wide, as a holder board does.
#[test]
fn the_stock_shows_its_label_while_it_is_wide_enough() {
    let mut s = cnc();
    s.show_default_view();
    let labels = s.fixture_labels();
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].text, "Plywood · 10 mm");
    let (x, y) = s
        .view()
        .document_to_screen(s.active_cnc().unwrap().page.origin);
    assert_eq!((labels[0].x, labels[0].y), (x, y));
    assert_eq!(s.edit_cnc_stock("width_mm", "10"), Ok(true));
    assert!(s.fixture_labels().is_empty(), "too narrow");
}
