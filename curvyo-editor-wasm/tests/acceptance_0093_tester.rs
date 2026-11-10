//! Independent acceptance tests for `specs/0093-laser-workbench-shell` through the public session:
//! fixtures (criteria 9, 10), the extension switch (11b) and save and reopen with an unknown
//! process (12).
//!
//! Updated for `specs/0098-page-kinds` by its "Changes to accepted specs" table: the registry and
//! Add manufacturing (`0093` 1, 2), the work area edit (3), the Laser workbench showing only the
//! output page (4), the default placement (5), the read-only output page (7) and Delete
//! manufacturing (13) are superseded by `0098` criteria 1, 4, 7, 8, 10 to 13 and 16, so their
//! tests are gone; the live placement (6) and overhang (8) need a placement, which only older
//! files hold now, and are covered with a format-36 file by the session tests of `0098`. A laser
//! page is added with Add page (Kind Laser) and reached by activating it.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::assert_is_empty
)]

use std::io::{Cursor, Read};

use curvyo_document_core::{
    Color, Document, DocumentSize, Length, NewOutputPage, Point, RectBounds, StyleEdit, pack,
    unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use serde_json::Value;

const LASER: &str = "curvyo.laser";

// ---- helpers ------------------------------------------------------------------------------------

fn red_rect(d: &Document, a: (f64, f64), b: (f64, f64)) {
    let id = d.create_rect(RectBounds::from_corners(
        Point::new(a.0, a.1),
        Point::new(b.0, b.1),
    ));
    d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    d.edit_style(&[id], &StyleEdit::FillColor(Color { r: 255, g: 0, b: 0 }))
        .unwrap();
}

/// One page at the origin with a red 40 x 20 rectangle at (10, 20).
fn one_page() -> Session {
    let d = Document::new(1);
    red_rect(&d, (10.0, 20.0), (50.0, 40.0));
    Session::open(9201, &pack(&d, "0.1.0").unwrap()).unwrap()
}

fn workbenches(s: &Session) -> Value {
    serde_json::from_str(&s.workbenches_json()).unwrap()
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.manufacturing_view_json()).unwrap()
}

fn active(s: &Session) -> String {
    workbenches(s)["active"].as_str().unwrap().to_owned()
}

fn key(s: &mut Session, k: &str, ctrl: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        ..KeyInput::default()
    })
}

fn drag(s: &mut Session, a: Point, b: Point) {
    s.pointer_down(a, false);
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
}

fn opened(s: &Session) -> Document {
    unpack(9299, &s.pack("0.1.0").unwrap()).unwrap()
}

fn mm(v: f64) -> Length {
    Length::from_mm(v)
}

/// Add page with Kind Laser; the new page's id.
fn add_laser(s: &mut Session) -> String {
    s.add_page_of_kind(LASER, None).expect("Laser can be added")
}

fn first_page(s: &mut Session) {
    assert!(s.activate_page("first"));
}

fn fixture_ids(s: &Session) -> Vec<String> {
    view(s)["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["id"].as_str().unwrap().to_owned())
        .collect()
}

// ---- criteria 9, 10 ---------------------------------------------------------------------------

#[test]
fn ac9_a_new_setup_has_no_fixture_and_the_grid_is_added_with_its_defaults() {
    let mut s = one_page();
    add_laser(&mut s);
    assert!(view(&s)["fixtures"].as_array().unwrap().is_empty());
    let steps = s.undo_len();
    assert!(s.add_fixture("engraved_grid").is_some());
    assert_eq!(s.undo_len(), steps + 1);
    let f = &view(&s)["fixtures"][0];
    // Named "<Kind> N" since `specs/0102-laser-fixtures` criterion 2; the row's grid fields are
    // those of `0102` criterion 15 (the line width is no longer shown, criterion 16).
    assert_eq!(f["name"], "Grid 1");
    assert_eq!(f["visible"], true);
    assert_eq!(f["grid"]["spacing_mm"], 10.0);
    assert_eq!(f["grid"]["origin_x_mm"], 0.0);
    assert_eq!(f["grid"]["origin_y_mm"], 0.0);
    assert!(s.add_fixture("pocket").is_none(), "only the declared kind");
    // One grid per page since `0102` criterion 2.
    assert!(s.add_fixture("engraved_grid").is_none());
}

#[test]
fn ac9_a_spacing_edit_redraws_and_is_one_step_and_clamped() {
    let mut s = one_page();
    add_laser(&mut s);
    s.add_fixture("engraved_grid").unwrap();
    let id = s.fixture_named(&fixture_ids(&s)[0]).unwrap();
    let n10 = s.frame_draw_list().triangles.len();
    let steps = s.undo_len();
    assert!(s.set_fixture_length(id, "spacing", mm(50.0)));
    assert_eq!(s.undo_len(), steps + 1);
    assert!(s.frame_draw_list().triangles.len() < n10);
    assert!(s.set_fixture_length(id, "spacing", mm(0.0)));
    // `0102` criterion 15: the spacing is 1 to 100 mm.
    assert_eq!(
        view(&s)["fixtures"][0]["grid"]["spacing_mm"],
        1.0,
        "clamped to its minimum"
    );
    assert!(!s.set_fixture_length(id, "nonsense", mm(3.0)));
}

#[test]
fn ac10_fixtures_are_never_selectable_and_hide_is_view_state_of_one_step() {
    let mut s = one_page();
    add_laser(&mut s);
    let o = s.active_page().origin;
    s.add_fixture("engraved_grid").unwrap();
    let id = s.fixture_named(&fixture_ids(&s)[0]).unwrap();
    // `0102` criterion 2: a new fixture is selected; its box is not the grid's drawing.
    s.escape();
    let shown = s.frame_draw_list().triangles.len();

    // Press on a grid line of the laser page, marquee over the page, Select All.
    s.set_tool(Tool::Select);
    s.pointer_down(Point::new(o.x + 200.0, o.y + 200.0), false);
    s.pointer_up(Point::new(o.x + 200.0, o.y + 200.0), false, false);
    drag(
        &mut s,
        Point::new(o.x + 5.0, o.y + 5.0),
        Point::new(o.x + 395.0, o.y + 295.0),
    );
    assert_eq!(s.selected_object_count(), 0);

    let steps = s.undo_len();
    assert!(s.set_fixture_visible(id, false));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(view(&s)["fixtures"][0]["visible"], false);
    let hidden = s.frame_draw_list().triangles.len();
    assert!(hidden < shown, "hidden fixture is not drawn");
    s.undo();
    assert_eq!(s.frame_draw_list().triangles.len(), shown);
}

#[test]
fn ac10_select_all_marquee_and_click_reach_only_the_drawing() {
    let mut s = one_page();
    add_laser(&mut s);
    s.add_fixture("engraved_grid").unwrap();
    first_page(&mut s);
    s.set_tool(Tool::Select);
    s.pointer_down(Point::new(100.0, 100.0), false);
    s.pointer_up(Point::new(100.0, 100.0), false, false);
    assert_eq!(s.selected_object_count(), 0);
    key(&mut s, "a", true);
    assert_eq!(s.selected_object_count(), 1, "only the drawing object");
    s.escape();
    drag(&mut s, Point::new(-50.0, -50.0), Point::new(205.0, 290.0));
    assert_eq!(s.selected_object_count(), 1);
    // Delete everything selectable: the manufacturing stays.
    s.delete_selected();
    let d = opened(&s);
    assert!(d.object_ids().is_empty());
    assert_eq!(d.manufacturings().len(), 1);
    assert_eq!(d.manufacturings()[0].fixtures.len(), 1);
}

#[test]
fn ac10_layers_show_no_manufacturing_fixture_or_placement() {
    let mut s = one_page();
    let layers = s.layers_view_json(0, 100);
    add_laser(&mut s);
    s.add_fixture("engraved_grid").unwrap();
    first_page(&mut s);
    assert_eq!(s.layers_view_json(0, 100), layers);
}

// ---- criterion 11b ------------------------------------------------------------------------------

#[test]
fn ac11b_switched_off_the_laser_kind_goes_and_the_page_stays_with_its_data() {
    let mut s = one_page();
    let page = add_laser(&mut s);
    s.add_fixture("engraved_grid").unwrap();
    let ext = workbenches(&s)["extensions"].clone();
    let laser = ext
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == LASER)
        .cloned()
        .unwrap();
    assert_eq!(laser["name"], "Laser workbench");
    assert!(!laser["version"].as_str().unwrap().is_empty());
    assert_eq!(laser["enabled"], true);

    let steps = s.undo_len();
    let text = s.set_extension_enabled(LASER, false);
    assert!(text.contains(LASER));
    assert_eq!(s.undo_len(), steps, "not an undo step");
    assert_eq!(active(&s), "vector", "the page shows the vector tools");
    let w = workbenches(&s);
    assert!(
        w["kinds"]
            .as_array()
            .unwrap()
            .iter()
            .all(|k| k["id"] != LASER),
        "Laser leaves the Kind switch"
    );
    assert!(w["manifests"].as_array().unwrap().is_empty());
    assert_eq!(w["page"]["word"], "Laser (off)");
    assert!(s.add_page_of_kind(LASER, None).is_none());

    let saved = s.pack("0.1.0").unwrap();
    let back = unpack(5, &saved).unwrap();
    assert_eq!(back.manufacturings(), opened(&s).manufacturings());
    assert_eq!(
        back.manufacturings()[0].fixtures.len(),
        1,
        "data kept in the file"
    );

    s.set_extension_enabled(LASER, true);
    assert!(s.activate_page(&page));
    assert_eq!(active(&s), "manufacturing");
}

#[test]
fn ac11b_the_settings_file_switch_is_read_at_load() {
    let mut s = one_page();
    add_laser(&mut s);
    s.load_extension_settings("disabled = [\"curvyo.laser\"]\n");
    assert_eq!(active(&s), "vector");
    assert_eq!(workbenches(&s)["kinds"].as_array().unwrap().len(), 2);
    s.load_extension_settings("");
    assert_eq!(active(&s), "manufacturing");
}

#[test]
fn ac11b_a_disabled_laser_page_can_still_be_deleted_and_undone() {
    let mut s = one_page();
    let page = add_laser(&mut s);
    s.set_extension_enabled(LASER, false);
    assert!(s.delete_page(&page));
    assert!(opened(&s).manufacturings().is_empty());
    s.undo();
    assert_eq!(opened(&s).manufacturings().len(), 1);
    assert!(s.activate_page(&page));
    assert_eq!(active(&s), "vector", "still disabled");
}

// ---- criterion 12 -------------------------------------------------------------------------------

#[test]
fn ac12_save_and_reopen_keep_the_laser_page_and_its_fixtures() {
    let mut s = one_page();
    let page = add_laser(&mut s);
    s.add_fixture("engraved_grid").unwrap();
    // One grid per page since `0102` criterion 2: the second fixture is a holder.
    s.add_fixture("holder").unwrap();
    let hidden = s.fixture_named(&fixture_ids(&s)[1]).unwrap();
    assert!(s.set_fixture_visible(hidden, false));
    let before = opened(&s).manufacturings();
    let frame = s.frame_draw_list();

    let mut r = Session::open(9205, &s.pack("0.1.0").unwrap()).unwrap();
    let after = opened(&r).manufacturings();
    assert_eq!(after, before);
    assert!(!after[0].fixtures[1].visible);
    assert!(r.activate_page(&page));
    assert_eq!(active(&r), "manufacturing");
    r.show_default_view();
    s.show_default_view();
    assert_eq!(
        r.frame_draw_list().triangles.len(),
        s.frame_draw_list().triangles.len()
    );
    assert!(!frame.triangles.is_empty());
}

fn stored_vv(bytes: &[u8]) -> loro::VersionVector {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut stored = Vec::new();
    archive
        .by_name("document.loro")
        .unwrap()
        .read_to_end(&mut stored)
        .unwrap();
    let loro = loro::LoroDoc::new();
    loro.import(&stored).unwrap();
    loro.oplog_vv()
}

#[test]
fn ac12_an_unknown_process_opens_with_vector_tools_and_saves_back_unchanged() {
    let d = Document::new(1);
    red_rect(&d, (10.0, 20.0), (50.0, 40.0));
    let page = d
        .add_output_page(NewOutputPage {
            process: "acme.cnc",
            label: "CNC",
            profile: None,
            work_area: DocumentSize::from_mm(800.0, 600.0),
        })
        .unwrap();
    let id = page.kind.manufacturing().unwrap();
    let params = serde_json::from_value(serde_json::json!({"depth": {"mm": 3.0}})).unwrap();
    d.add_fixture(
        id,
        &curvyo_document_core::NewFixture {
            add: curvyo_document_core::FixtureAdd::Grid,
            kind: "acme.cnc/clamp",
            label: "Clamp",
            params: &params,
            matrix: None,
        },
    )
    .unwrap();
    let bytes = pack(&d, "0.1.0").unwrap();

    let mut s = Session::open(9206, &bytes).unwrap();
    assert!(s.activate_page(&page.id));
    assert_eq!(
        active(&s),
        "vector",
        "an unknown process shows the vector tools"
    );
    assert_eq!(workbenches(&s)["page"]["word"], "Unknown kind");
    s.set_tool(Tool::Select);
    key(&mut s, "a", true);
    // The rectangle lies on the drawing page: Select all on the unknown page, a drawing page for
    // the context rule, leaves it out (`specs/0106` criteria 1 and 3).
    assert_eq!(s.selected_object_count(), 0);
    s.escape();
    let saved = s.pack("0.1.0").unwrap();
    assert_eq!(stored_vv(&saved), stored_vv(&bytes), "saved back unchanged");
    assert_eq!(
        unpack(3, &saved).unwrap().manufacturings(),
        d.manufacturings()
    );
}
