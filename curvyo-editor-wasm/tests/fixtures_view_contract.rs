//! The contract of the Fixtures block's JSON (`specs/0102-laser-fixtures` criteria 1, 15, 19, 20,
//! 32, 33; feature `adrs.md` §6a.7 and §7, architecture review finding 3): representative states are built
//! as files, opened, and their view JSON compared with the shared fixtures in
//! `tests/fixtures/fixtures_view/`, which the frontend decoder reads too. A drift fails. The
//! fixtures are written only when `CURVYO_UPDATE_GOLDEN` is set.

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use curvyo_document_core::{
    Affine, Angle, Document, DocumentSize, FixtureAdd, FixtureParams, NewFixture, NewOutputPage,
    NodeId, Point, Vec2, pack,
};
use curvyo_editor_wasm::Session;
use serde_json::{Value, json};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fixtures_view")
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

/// One fixture of each kind: a grid with the top tape off, a holder turned by 30 degrees with a
/// rectangle slot, a circle slot and one past the board, an inactive clamp, a hidden stop, an
/// unknown kind and a damaged value.
fn state_every_kind() -> Document {
    let (d, m) = document();
    let grid = json!({"spacing_mm": 5.0, "major_every": 4, "tapes": ["bottom", "left", "right"],
        "origin_x_mm": 2.0, "origin_y_mm": 0.0});
    add(
        &d,
        m,
        FixtureAdd::Grid,
        "curvyo.laser/engraved_grid",
        "Grid",
        grid,
    );
    let holder = add(
        &d,
        m,
        FixtureAdd::Holder,
        "curvyo.laser/holder",
        "Holder",
        json!({"width_mm": 120.0, "height_mm": 80.0}),
    );
    let turn = Affine::rotation_about(
        Point::new(80.0, 70.0),
        Angle::from_radians(30f64.to_radians()),
    );
    d.set_fixture_matrix(
        holder,
        turn.compose(Affine::translation(Vec2::new(20.0, 30.0))),
        curvyo_document_core::FixtureTurn::Rotate,
    )
    .unwrap();
    for slot in [
        json!({"shape": "rect", "cx_mm": 30.0, "cy_mm": 40.0, "w_mm": 40.0, "h_mm": 25.0}),
        json!({"shape": "circle", "cx_mm": 90.0, "cy_mm": 40.0, "d_mm": 30.0}),
        json!({"shape": "rect", "cx_mm": 118.0, "cy_mm": 40.0, "w_mm": 10.0, "h_mm": 10.0}),
    ] {
        d.add_slot(holder, "Slot", &params(slot)).unwrap();
    }
    let clamp = add(
        &d,
        m,
        FixtureAdd::Clamp,
        "curvyo.laser/clamp",
        "Clamp",
        json!({"width_mm": 40.0, "height_mm": 20.0}),
    );
    d.set_fixture_active(clamp, false).unwrap();
    let stop = add(
        &d,
        m,
        FixtureAdd::Stop,
        "curvyo.laser/stop",
        "Stop",
        json!({"leg_a_mm": 60.0, "leg_b_mm": "long", "thickness_mm": 5.0}),
    );
    d.set_fixture_visible(stop, false).unwrap();
    add(
        &d,
        m,
        FixtureAdd::Screw,
        "curvyo.laser/screw",
        "Screw",
        json!({"thread_diameter_mm": 6.0, "head_diameter_mm": 10.0}),
    );
    add(
        &d,
        m,
        FixtureAdd::Holder,
        "curvyo.laser/pocket",
        "Pocket",
        json!({"depth_mm": 3.0}),
    );
    d
}

/// The states: the name, the document and whether the holder is selected on the canvas (its
/// announcement; the Select bar shows nothing for a fixture, criterion 14).
fn states() -> Vec<(&'static str, Document, bool)> {
    vec![
        ("empty", document().0, false),
        ("every_kind", state_every_kind(), false),
        ("selected_holder", state_every_kind(), true),
    ]
}

fn view_of(d: &Document, select_holder: bool) -> Value {
    let bytes = pack(d, "fixtures view").unwrap();
    let laser = d
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap()
        .id;
    let mut s = Session::open(3, &bytes).unwrap();
    assert!(s.activate_page(&laser));
    if select_holder {
        let view: Value = serde_json::from_str(&s.fixtures_view_json()).unwrap();
        let holder = view["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["kind"] == "holder")
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let id = s.fixture_part_named(&holder).unwrap();
        assert!(s.select_fixture(id));
    }
    serde_json::from_str(&s.fixtures_view_json()).unwrap()
}

#[test]
fn every_state_matches_its_fixture() {
    std::fs::create_dir_all(fixture_dir()).unwrap();
    for (name, d, select_holder) in states() {
        let json = view_of(&d, select_holder);
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
fn the_every_kind_state_holds_what_the_decoder_must_read() {
    let json = view_of(&state_every_kind(), false);
    let kinds: Vec<&str> = json["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["kind"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds,
        ["grid", "holder", "clamp", "stop", "screw", "unknown"]
    );
    assert!(
        json["rows"][4]["grid"].is_null() && json["rows"][4].get("size").is_none(),
        "a screw's row has no field"
    );
    let slots = &json["rows"][1]["slots"];
    assert_eq!(slots[1]["shape"], "circle");
    assert_eq!(slots[2]["outside"], true);
    assert_eq!(json["rows"][2]["active"], false);
    assert_eq!(json["rows"][3]["visible"], false);
    assert_eq!(
        json["damage_notices"],
        json!(["1 value damaged on Stop 1."])
    );
    assert_eq!(json["can_add_grid"], false);
    assert!(json["selected"].is_null());
    assert!(json["selection_announcement"].is_null());
    assert!(
        json.get("selected_pose").is_none(),
        "no pose for a Select bar"
    );
}

#[test]
fn the_selected_holder_state_carries_its_announcement() {
    let json = view_of(&state_every_kind(), true);
    assert_eq!(json["selected"], json["rows"][1]["id"]);
    assert_eq!(
        json["selection_announcement"],
        "Holder 1 selected, 120.0 by 80.0 mm."
    );
    assert!(
        json["rows"][1].get("size").is_none(),
        "no size field in the row"
    );
}

#[test]
fn off_a_laser_page_the_view_is_null() {
    let s = Session::new(1);
    assert_eq!(s.fixtures_view_json(), "null");
}
