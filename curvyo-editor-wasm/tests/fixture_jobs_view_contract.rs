//! The contract of the Fixture jobs section's JSON (`specs/0103-fixture-jobs` criteria 1 to 5, 8,
//! 15, 17, 21; feature `adrs.md` §6): representative states are built as files, opened, and their
//! view JSON compared with the shared fixtures in `tests/fixtures/fixture_jobs_view/`, which the
//! frontend decoder reads too. A drift fails. The fixtures are written only when
//! `CURVYO_UPDATE_GOLDEN` is set.

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use curvyo_document_core::{
    Affine, Document, DocumentSize, FixtureAdd, FixtureJobSwitch, FixtureParams, FixturePartKey,
    NewFixture, NewOutputPage, NodeId, Vec2, pack,
};
use curvyo_editor_wasm::Session;
use serde_json::{Value, json};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fixture_jobs_view")
}

fn params(value: Value) -> FixtureParams {
    let Value::Object(map) = value else {
        panic!("an object")
    };
    map
}

/// A laser page of 400 x 300 mm; the document and its manufacturing.
fn document() -> (Document, NodeId) {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let m = laser.kind.manufacturing().unwrap();
    (d, m)
}

fn add(d: &Document, m: NodeId, add: FixtureAdd, kind: &str, word: &str, p: Value) -> NodeId {
    let matrix = (add != FixtureAdd::Grid).then(|| Affine::translation(Vec2::new(20.0, 30.0)));
    d.add_fixture(
        m,
        &NewFixture {
            add,
            kind,
            label: word,
            params: &params(p),
            matrix,
        },
    )
    .unwrap()
}

fn holder(d: &Document, m: NodeId) -> (NodeId, NodeId) {
    let holder = add(
        d,
        m,
        FixtureAdd::Holder,
        "curvyo.laser/holder",
        "Holder",
        json!({"width_mm": 120.0, "height_mm": 80.0}),
    );
    let slot = json!({"shape": "rect", "cx_mm": 30.0, "cy_mm": 40.0, "w_mm": 40.0, "h_mm": 25.0});
    let slot = d.add_slot(holder, "Coaster", &params(slot)).unwrap();
    (holder, slot)
}

fn tick(d: &Document, job: NodeId, node: NodeId, suffix: Option<&str>) {
    let key = FixturePartKey {
        node,
        suffix: suffix.map(str::to_owned),
    };
    assert!(d.set_fixture_job_targets(job, &[key], true).unwrap());
}

/// A holder with a running Engrave + Outline job on its board and slot, a grid with a Cut job
/// with Output off, and an inactive holder with one job.
fn state_two_fixtures() -> Document {
    let (d, m) = document();
    let grid = json!({"spacing_mm": 10.0, "major_every": 5, "tapes": ["bottom"],
        "origin_x_mm": 0.0, "origin_y_mm": 0.0});
    let grid = add(
        &d,
        m,
        FixtureAdd::Grid,
        "curvyo.laser/engraved_grid",
        "Grid",
        grid,
    );
    let (board, slot) = holder(&d, m);
    let engrave = d
        .add_fixture_job(board, "engrave_outline", "Engrave + Outline 1")
        .unwrap();
    tick(&d, engrave, board, None);
    tick(&d, engrave, slot, None);
    d.set_fixture_job_switch(engrave, FixtureJobSwitch::Output, true)
        .unwrap();
    let cut = d.add_fixture_job(grid, "cut", "Cut 1").unwrap();
    tick(&d, cut, grid, Some("lines"));
    let (off_bed, _) = holder(&d, m);
    d.set_fixture_active(off_bed, false).unwrap();
    let job = d.add_fixture_job(off_bed, "score", "Score 1").unwrap();
    tick(&d, job, off_bed, None);
    d.set_fixture_job_switch(job, FixtureJobSwitch::Output, true)
        .unwrap();
    d
}

/// A holder with an Unknown job and a Show-off Cut job on its slot, the Cut job selected.
fn state_unknown_and_show_off() -> Document {
    let (d, m) = document();
    let (board, slot) = holder(&d, m);
    d.add_fixture_job(board, "laser_burn", "Burn 1").unwrap();
    let cut = d.add_fixture_job(board, "cut", "Cut 1").unwrap();
    tick(&d, cut, slot, None);
    d.set_fixture_job_switch(cut, FixtureJobSwitch::Show, false)
        .unwrap();
    d
}

/// The states: the name, the document and the name of the job to select, if any.
fn states() -> Vec<(&'static str, Document, Option<&'static str>)> {
    vec![
        ("empty", document().0, None),
        (
            "two_fixtures",
            state_two_fixtures(),
            Some("Engrave + Outline 1"),
        ),
        (
            "unknown_and_show_off",
            state_unknown_and_show_off(),
            Some("Cut 1"),
        ),
    ]
}

fn view_of(d: &Document, select: Option<&str>) -> Value {
    let bytes = pack(d, "fixture jobs view").unwrap();
    let laser = d
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap()
        .id;
    let mut s = Session::open(3, &bytes).unwrap();
    assert!(s.activate_page(&laser));
    if let Some(name) = select {
        let view: Value = serde_json::from_str(&s.fixture_jobs_view_json()).unwrap();
        let id = view["groups"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|g| g["jobs"].as_array().unwrap().iter())
            .find(|j| j["name"] == name)
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let job = s.fixture_job_named(&id).unwrap();
        assert!(s.select_fixture_job(job));
    }
    serde_json::from_str(&s.fixture_jobs_view_json()).unwrap()
}

#[test]
fn every_state_matches_its_fixture() {
    std::fs::create_dir_all(fixture_dir()).unwrap();
    for (name, d, select) in states() {
        let json = view_of(&d, select);
        let text = format!("{}\n", serde_json::to_string_pretty(&json).unwrap());
        let path = fixture_dir().join(format!("{name}.json"));
        if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
            std::fs::write(&path, &text).unwrap();
        }
        let want: Value = serde_json::from_str(
            &std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("{} exists (CURVYO_UPDATE_GOLDEN=1)", path.display())),
        )
        .unwrap();
        assert_eq!(json, want, "{name} drifted from {}", path.display());
    }
}

#[test]
fn the_empty_state_has_no_group_and_hides_the_add_buttons() {
    let json = view_of(&document().0, None);
    assert_eq!(json["count"], 0);
    assert_eq!(json["groups"], json!([]));
    assert!(json["add_to"].is_null() && json["parts"].is_null());
    assert_eq!(json["running"], 0);
    assert_eq!(json["damaged"], 0);
    assert_eq!(json["damage_notices"], json!([]));
}

#[test]
fn the_two_fixtures_state_holds_what_the_decoder_must_read() {
    let json = view_of(&state_two_fixtures(), Some("Engrave + Outline 1"));
    assert_eq!(json["count"], 3);
    let kinds: Vec<&str> = json["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["grid", "holder", "holder"], "creation order");
    assert_eq!(
        json["running"], 1,
        "the Output-off and the off-bed jobs do not run"
    );
    assert_eq!(json["damaged"], 0, "a clean state has no damaged job value");
    let engrave = &json["groups"][1]["jobs"][0];
    assert_eq!(
        engrave["kind"],
        json!({"text": "engrave_outline", "word": "Engrave + Outline"})
    );
    assert_eq!(engrave["runs"], true);
    assert_eq!(
        (
            engrave["parts_ticked"].clone(),
            engrave["parts_total"].clone()
        ),
        (json!(2), json!(2))
    );
    assert_eq!(json["groups"][2]["jobs"][0]["fixture_active"], false);
    let rows = json["parts"]["rows"].as_array().unwrap();
    assert_eq!(rows[0]["token"], "board");
    assert_eq!(rows[1]["name"], "Coaster 1");
    assert_eq!(rows[1]["state"], "running");
    assert_eq!(rows[1]["kind"], "engrave_outline");
}

#[test]
fn the_unknown_state_lists_the_unknown_job_and_a_show_off_part() {
    let json = view_of(&state_unknown_and_show_off(), Some("Cut 1"));
    let jobs = json["groups"][0]["jobs"].as_array().unwrap();
    assert!(jobs[0]["kind"].is_null(), "Unknown");
    assert_eq!(jobs[0]["runs"], false);
    assert_eq!(json["parts"]["rows"][1]["state"], "show_off");
    assert!(json["parts"]["rows"][1]["kind"].is_null());
}

#[test]
fn off_a_laser_page_the_view_is_null() {
    let s = Session::new(1);
    assert_eq!(s.fixture_jobs_view_json(), "null");
}
