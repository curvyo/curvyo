//! Session tests of `specs/0160-embroidery` slice 1: Add page, profile, hoops, the three kinds
//! with their numbers and threads, the hoop check, the narrow satin, the Toolpath view and play,
//! Export and the library sync of embroidery records.

use curvyo_document_core::{Direction, DocumentSize, NodeId, Point, RectBounds};
use curvyo_embroidery_core::{HoopSize, W6_EU7, read_dst, write_record};
use serde_json::Value;

use super::super::Session;

const EMBROIDERY: &str = "curvyo.embroidery";

fn embroidery() -> Session {
    let mut s = Session::new(160_001);
    s.resize_viewport(1200.0, 800.0);
    s.add_page_of_kind(EMBROIDERY, None).unwrap();
    s
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.embroidery_view_json()).unwrap()
}

/// A rectangle `w` x `h` mm centred `dx`, `dy` mm from the hoop centre.
fn rect(s: &mut Session, dx: f64, dy: f64, w: f64, h: f64) -> NodeId {
    let c = s.active_embroidery().unwrap().centre();
    s.document.create_rect(RectBounds::from_corners(
        Point::new(c.x + dx - w / 2.0, c.y + dy - h / 2.0),
        Point::new(c.x + dx + w / 2.0, c.y + dy + h / 2.0),
    ))
}

fn assign(s: &mut Session, kind: &str, items: &[NodeId]) -> NodeId {
    s.selection.set(items);
    let id = s.press_embroidery_kind(kind, false).unwrap();
    s.selection.set(&[]);
    id
}

/// The top of the drawn hoop (its frame's outer edge, 20 mm above the sewing field).
fn hoop_field_top(s: &Session) -> f64 {
    let page = s.active_embroidery().unwrap();
    let mut grids = Vec::new();
    s.add_hoop_lists(std::slice::from_ref(&page.page), &mut grids);
    grids[0]
        .triangles
        .iter()
        .map(|v| v.position.y)
        .fold(f64::MAX, f64::min)
}

fn undo_name(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}

#[test]
fn criterion_1_2_add_page_is_the_150_x_250_field_with_the_w6_eu7() {
    let mut s = Session::new(160_002);
    let steps = s.undo_len();
    s.add_page_of_kind(EMBROIDERY, None).unwrap();
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_name(&mut s), "Add embroidery page");
    let page = s.active_embroidery().unwrap();
    // ADR 0098 decision 1: the page is the active hoop's sewing field.
    assert_eq!(page.page.size, DocumentSize::from_mm(150.0, 250.0));
    let v = view(&s);
    assert_eq!(v["profile"]["name"], "W6 EU7");
    assert_eq!(v["profile"]["output"], "DST");
    assert_eq!(v["profile"]["speed"], 600);
    assert_eq!(v["profile"]["maxStitch"], "12.1 mm");
    let sizes: Vec<&str> = v["hoop"]["sizes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["label"].as_str().unwrap())
        .collect();
    assert_eq!(sizes, ["150 × 250 mm", "100 × 100 mm"]);
    assert!(s.active_manufacturing().is_none(), "no laser tools");
    let typed = s.set_document_side(crate::session::DocumentSide::Width, "200");
    assert_eq!(
        typed,
        crate::session::SizeOutcome::Unchanged,
        "not editable"
    );
    assert_eq!(
        s.active_embroidery().unwrap().page.size,
        DocumentSize::from_mm(150.0, 250.0)
    );
}

#[test]
fn criterion_2_save_as_profile_writes_an_embroidery_record_that_syncs() {
    let mut s = embroidery();
    assert_eq!(
        s.set_embroidery_speed("2000"),
        Err("Speed is 100 to 1,500 stitches/min.".to_owned())
    );
    assert_eq!(s.set_embroidery_speed("800"), Ok(true));
    assert_eq!(undo_name(&mut s), "Edit speed");
    let id = s.save_embroidery_profile([3; 16], 1_000).unwrap();
    let writes: Value = serde_json::from_str(&s.take_machine_writes_json()).unwrap();
    let text = writes[0]["text"].as_str().unwrap();
    assert!(text.contains("\"kind\": \"embroidery\"") && text.contains("\"speed_spm\": 800"));
    let items = s.library_items();
    assert!(
        items
            .iter()
            .any(|(_, i, t)| *i == id && super::super::library_sync::is_embroidery_record(t))
    );
    let mut other = W6_EU7.profile();
    other.id = "0b5f6c2e-9d4a-4c1b-8e7f-1a2b3c4d5e6f".to_owned();
    other.values.name = "From the other device".to_owned();
    s.load_machine_profiles(&[(format!("{}.json", other.id), write_record(&other))]);
    assert!(s.manufacturing.embroidery.find(&other.id).is_some());
    assert!(
        s.manufacturing.machine.profiles.find(&other.id).is_none(),
        "never a laser profile"
    );
    assert!(s.pick_embroidery_profile(&other.id));
    assert_eq!(view(&s)["profile"]["name"], "From the other device");
}

#[test]
fn criterion_3_set_hoop_switches_the_field_and_resizes_the_page_about_its_centre() {
    let mut s = embroidery();
    s.show_default_view();
    assert_eq!(s.embroidery_hoop(), Some(HoopSize::Field150x250));
    let a = rect(&mut s, 0.0, 0.0, 20.0, 20.0);
    assign(&mut s, "running", &[a]);
    let large = s.embroidery_export().unwrap().bytes;
    let before = hoop_field_top(&s);
    let centre = s.active_embroidery().unwrap().centre();
    assert!(s.set_embroidery_hoop("100x100"));
    assert_eq!(undo_name(&mut s), "Set hoop");
    assert_eq!(s.embroidery_hoop(), Some(HoopSize::Field100x100));
    assert_eq!(view(&s)["hoop"]["key"], "100x100");
    // ADR 0098 decision 1: the page is the new field, resized about the hoop centre.
    let page = s.active_embroidery().unwrap();
    assert_eq!(page.page.size, DocumentSize::from_mm(100.0, 100.0));
    let moved = page.centre();
    assert!((moved.x - centre.x).abs() < 1e-6 && (moved.y - centre.y).abs() < 1e-6);
    let after = hoop_field_top(&s);
    assert!(
        (after - before - 75.0).abs() < 1e-6,
        "the drawn hoop's top moves 75 mm down: {before} {after}"
    );
    assert_eq!(
        s.embroidery_export().unwrap().bytes,
        large,
        "the hoop is never written to the file"
    );
    assert!(!s.set_embroidery_hoop("100x100"), "no change");
    s.undo();
    assert_eq!(s.embroidery_hoop(), Some(HoopSize::Field150x250));
}

#[test]
fn criterion_4_a_fill_1_mm_past_the_100_x_100_field_blocks_export() {
    let mut s = embroidery();
    assert!(s.set_embroidery_hoop("100x100"));
    // A 20 mm square whose right edge lies 51 mm right of the centre.
    let a = rect(&mut s, 41.0, 0.0, 20.0, 20.0);
    assign(&mut s, "tatami", &[a]);
    let v = view(&s);
    assert_eq!(v["checks"][0]["severity"], "block", "{v}");
    assert_eq!(
        v["checks"][0]["text"],
        "Tatami fill 1: 1 item lies outside the hoop."
    );
    assert_eq!(v["exportAllowed"], false);
    assert!(s.embroidery_export().is_none());
    assert!(s.set_embroidery_hoop("150x250"));
    assert!(s.embroidery_export().is_some(), "inside the large field");
}

#[test]
fn criterion_5_6_three_kinds_each_with_its_numbers_and_thread() {
    let mut s = embroidery();
    let a = rect(&mut s, 0.0, -30.0, 30.0, 20.0);
    let b = rect(&mut s, 0.0, 30.0, 30.0, 20.0);
    let running = assign(&mut s, "running", &[a]);
    assert_eq!(undo_name(&mut s), "Assign to Running stitch 1");
    let tatami = assign(&mut s, "tatami", &[b]);
    let v = view(&s);
    let kinds: Vec<&str> = v["kinds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k["label"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["Running stitch", "Tatami fill", "Satin"]);
    let rows = view(&s)["operations"].clone();
    assert_eq!(rows[0]["line3"], "2.5 mm, 1 ×");
    assert_eq!(rows[1]["line3"], "0°, 0.25 mm, 3 mm");
    assert_eq!(rows[0]["thread"], "#000000", "the rectangle's black stroke");
    assert_eq!(
        s.edit_embroidery_setting(running, "stitch_length_mm", "12"),
        Err("Stitch length is 1.0 to 10.0 mm.".to_owned())
    );
    assert_eq!(s.edit_embroidery_setting(running, "repeats", "3"), Ok(true));
    assert_eq!(undo_name(&mut s), "Edit settings");
    assert_eq!(
        s.edit_embroidery_setting(tatami, "angle_deg", "45"),
        Ok(true)
    );
    assert!(s.set_embroidery_thread(tatami, "#c8102e"));
    assert_eq!(undo_name(&mut s), "Set thread colour");
    let rows = view(&s)["operations"].clone();
    assert_eq!(rows[0]["line3"], "2.5 mm, 3 ×");
    assert_eq!(rows[1]["line3"], "45°, 0.25 mm, 3 mm");
    assert_eq!(rows[1]["thread"], "#C8102E");
    assert_eq!(rows[1]["startsColour"], true);
}

#[test]
fn criterion_9_a_narrow_satin_warns_and_still_exports() {
    let mut s = embroidery();
    let a = rect(&mut s, 0.0, 0.0, 30.0, 20.0);
    let satin = assign(&mut s, "satin", &[a]);
    let v = view(&s);
    // The rectangle's default stroke is narrower than 0.5 mm.
    assert_eq!(v["checks"][0]["severity"], "warn");
    assert_eq!(
        v["checks"][0]["text"],
        "Satin 1: 1 item is too narrow and runs as running stitch."
    );
    assert_eq!(v["exportAllowed"], true);
    assert!(
        s.edit_embroidery_setting(satin, "zigzag_spacing_mm", "0.5")
            .unwrap()
    );
}

#[test]
fn criterion_10_two_colours_give_one_colour_change() {
    let mut s = embroidery();
    let a = rect(&mut s, 0.0, -30.0, 30.0, 20.0);
    let b = rect(&mut s, 0.0, 30.0, 30.0, 20.0);
    assign(&mut s, "running", &[a]);
    let fill = assign(&mut s, "tatami", &[b]);
    assert!(s.set_embroidery_thread(fill, "#C8102E"));
    let job = s.embroidery_job(&s.active_embroidery().unwrap());
    let changes = job
        .plan
        .stitches
        .iter()
        .filter(|st| st.kind == curvyo_embroidery_core::StitchKind::ColourChange)
        .count();
    assert_eq!(changes, 1);
    let total = view(&s)["total"]["stitches"].as_str().unwrap().to_owned();
    assert!(total.contains("2 colours"), "{total}");
}

#[test]
fn criterion_12_13_the_toolpath_view_plays_the_written_stitches() {
    let mut s = embroidery();
    s.show_default_view();
    let a = rect(&mut s, 0.0, 0.0, 30.0, 20.0);
    assign(&mut s, "running", &[a]);
    let page = s.active_page().id;
    assert!(s.embroidery_toolpath(s.view()).is_none(), "not pressed");
    assert!(s.toggle_toolpath_view(&page));
    let all = s.embroidery_toolpath(s.view()).unwrap().triangles.len();
    let readout: Value = serde_json::from_str(&s.set_embroidery_play(10)).unwrap();
    assert_eq!(readout["position"], 10);
    assert_eq!(readout["colourText"], "1 of 1");
    let part = s.embroidery_toolpath(s.view()).unwrap().triangles.len();
    assert!(part < all, "nothing after the head: {part} {all}");
    let steps = s.undo_len();
    s.set_embroidery_play(0);
    assert_eq!(s.undo_len(), steps, "session state, no step");
    assert_eq!(view(&s)["toolpath"], true);
}

#[test]
fn criterion_12_14_18_export_writes_the_dst_under_the_laser_name_rule() {
    let mut s = embroidery();
    let a = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    assign(&mut s, "running", &[a]);
    let steps = s.undo_len();
    let job = s.export_job("Badge.curvyo").unwrap();
    assert_eq!(job.name, "Badge - Embroidery 1.dst");
    assert_eq!(job.file_type, "Tajima DST");
    let read = read_dst(&job.bytes).unwrap();
    assert_eq!(read.label, "Embroidery 1");
    assert_eq!(
        (read.extents.plus_x, read.extents.minus_x),
        (50, 50),
        "centred square"
    );
    assert_eq!((read.extents.plus_y, read.extents.minus_y), (50, 50));
    let stitches = view(&s)["total"]["stitches"].as_str().unwrap().to_owned();
    let summary = curvyo_embroidery_core::summarise(
        &read.stitches,
        curvyo_embroidery_core::StitchesPerMin::new(600).unwrap(),
    );
    assert!(
        stitches.starts_with(&format!("{} stitches", summary.stitches)),
        "{stitches}"
    );
    assert!(s.export_job_done(job.token, &job.name, false, 0).is_some());
    assert_eq!(s.undo_len(), steps, "no step");
    assert_eq!(
        s.export_job("Badge.curvyo").unwrap().bytes,
        job.bytes,
        "criterion 18"
    );
}

#[test]
fn criterion_20_an_embroidery_page_survives_save_and_reopen() {
    let mut s = embroidery();
    assert!(s.set_embroidery_hoop("100x100"));
    let a = rect(&mut s, 0.0, 0.0, 20.0, 20.0);
    let op = assign(&mut s, "tatami", &[a]);
    assert!(s.set_embroidery_thread(op, "#1F4E9C"));
    let before = view(&s);
    let mut r = Session::open(160_003, &s.pack("0.1.0").unwrap()).unwrap();
    let page = r
        .document
        .manufacturings()
        .into_iter()
        .find(|m| m.process == EMBROIDERY)
        .and_then(|m| m.page)
        .unwrap();
    assert!(r.activate_page(&page.id));
    let after = view(&r);
    assert_eq!(after["hoop"], before["hoop"]);
    assert_eq!(after["operations"], before["operations"]);
    assert_eq!(after["profile"], before["profile"]);
    assert_eq!(after["notice"], Value::Null);
}
