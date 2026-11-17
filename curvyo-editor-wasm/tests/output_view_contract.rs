//! The contract of the output group's, the operation rows' run lines and the play bar's JSON
//! (`specs/0029-laser-job-preview-and-output` criteria 1 to 7, 10, 37, 38; UX notes §5, §6;
//! reworked 2026-11-17, M9 to M16): representative states are built as files, opened, and their
//! view JSON compared with the shared fixtures in `tests/fixtures/output_view/`, which the
//! frontend decoder reads too. A drift fails. The fixtures are written only when
//! `CURVYO_UPDATE_GOLDEN` is set.

#![allow(clippy::unwrap_used, clippy::many_single_char_names)]

use std::path::PathBuf;

use curvyo_document_core::{
    Affine, Document, DocumentSize, FixtureAdd, FixtureJobSwitch, FixtureParams, FixturePartKey,
    NewFixture, NewOperation, NewOutputPage, NodeId, OperationSwitch, Point, RectBounds,
    SettingsStep, SettingsWrite, Vec2, pack, unpack,
};
use curvyo_editor_wasm::Session;
use serde_json::{Value, json};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/output_view")
}

/// A laser page of 400 x 300 mm: the document, its page id, its manufacturing and its origin.
fn document() -> (Document, String, NodeId, Point) {
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
    (d, laser.id, m, laser.origin)
}

/// A square of `side` mm at `(x, y)` from the page's origin.
fn square(d: &Document, o: Point, (x, y): (f64, f64), side: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(o.x + x, o.y + y),
        Point::new(o.x + x + side, o.y + y + side),
    ))
}

fn add(d: &Document, m: NodeId, kind: &str, name: &str, members: &[Vec<NodeId>]) -> NodeId {
    let index = d.operations(m).unwrap().len();
    d.add_operation(m, &NewOperation { kind, name }, index, members)
        .unwrap()
}

fn settings(d: &Document, op: NodeId, value: &Value) {
    let Value::Object(map) = value.clone() else {
        panic!("an object")
    };
    d.write_operation_settings(
        SettingsStep::EditSettings,
        &[SettingsWrite {
            operation: op,
            value: Some(map),
        }],
    )
    .unwrap();
}

/// A Cut and an Engrave with own numbers, an item not assigned and a Score with Output off.
fn state_cut_and_engrave() -> (Document, String) {
    let (d, laser, m, o) = document();
    let a = square(&d, o, (10.0, 10.0), 20.0);
    let b = square(&d, o, (50.0, 10.0), 20.0);
    let c = square(&d, o, (90.0, 10.0), 20.0);
    let e = square(&d, o, (130.0, 10.0), 20.0);
    let engrave = add(&d, m, "engrave", "Engrave 1", &[vec![b]]);
    settings(
        &d,
        engrave,
        &json!({"main": {"power_pct": 35.0, "speed_mm_min": 6000, "passes": 1,
            "interval_mm": 0.5}}),
    );
    let score = add(&d, m, "score", "Score 1", &[vec![e]]);
    d.set_operation_switch(score, OperationSwitch::Output, false)
        .unwrap();
    let cut = add(&d, m, "cut", "Cut 1", &[vec![a]]);
    settings(
        &d,
        cut,
        &json!({"main": {"power_pct": 80.0, "speed_mm_min": 600, "passes": 2}}),
    );
    let _ = c;
    (d, laser)
}

/// A Cut with no settings, and a Cut 2 whose square lies half off the bed: two blocks.
fn state_blocked() -> (Document, String) {
    let (d, laser, m, o) = document();
    let a = square(&d, o, (10.0, 10.0), 20.0);
    let b = square(&d, o, (390.0, 10.0), 20.0);
    add(&d, m, "cut", "Cut 1", &[vec![a]]);
    let cut = add(&d, m, "cut", "Cut 2", &[vec![b]]);
    settings(
        &d,
        cut,
        &json!({"main": {"power_pct": 60.0, "speed_mm_min": 600, "passes": 1}}),
    );
    (d, laser)
}

/// A Cut that runs with Start `start`.
fn state_start(start: &str) -> (Document, String) {
    let (d, laser, m, o) = document();
    let a = square(&d, o, (10.0, 10.0), 20.0);
    let cut = add(&d, m, "cut", "Cut 1", &[vec![a]]);
    settings(
        &d,
        cut,
        &json!({"main": {"power_pct": 60.0, "speed_mm_min": 600, "passes": 1}}),
    );
    d.set_start(m, start).unwrap();
    (d, laser)
}

/// The format 48 golden whose second laser page stores `start` true (ADR 0065 §6): a damaged
/// Start, its check opening the Start pick (criterion 4 e).
fn state_start_damaged() -> (Document, String) {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures/laser_output_damaged_v48.curvyo"),
    )
    .unwrap();
    let d = unpack(1, &bytes).unwrap();
    let page = d
        .pages()
        .into_iter()
        .filter(|p| p.kind.manufacturing().is_some())
        .nth(1)
        .unwrap();
    (d, page.id)
}

/// A holder whose running Engrave + Outline job has no fixture material: the check opens the
/// Fixture jobs section's pick (criterion 2).
fn state_fixture_no_material() -> (Document, String) {
    let (d, laser, m, _) = document();
    let Value::Object(params) = json!({"width_mm": 120.0, "height_mm": 80.0}) else {
        panic!("an object")
    };
    let params: FixtureParams = params;
    let holder = d
        .add_fixture(
            m,
            &NewFixture {
                add: FixtureAdd::Holder,
                kind: "curvyo.laser/holder",
                label: "Holder",
                params: &params,
                matrix: Some(Affine::translation(Vec2::new(20.0, 30.0))),
            },
        )
        .unwrap();
    let job = d
        .add_fixture_job(holder, "engrave_outline", "Engrave + Outline 1")
        .unwrap();
    let key = FixturePartKey {
        node: holder,
        suffix: None,
    };
    assert!(d.set_fixture_job_targets(job, &[key], true).unwrap());
    d.set_fixture_job_switch(job, FixtureJobSwitch::Output, true)
        .unwrap();
    (d, laser)
}

/// The states: name, document, laser page id and the play position while the Toolpath view is
/// pressed.
fn states() -> Vec<(&'static str, Document, String, Option<f64>)> {
    let (empty, laser, _, _) = document();
    let (cut, cut_page) = state_cut_and_engrave();
    let (blocked, blocked_page) = state_blocked();
    let (head, head_page) = state_start("head_position");
    let (home, home_page) = state_start("home_then_origin");
    let (played, played_page) = state_start("machine_origin");
    let (damaged, damaged_page) = state_start_damaged();
    let (fixture, fixture_page) = state_fixture_no_material();
    vec![
        ("empty", empty, laser, None),
        ("cut_and_engrave", cut, cut_page, None),
        ("blocked", blocked, blocked_page, None),
        ("start_head_position", head, head_page, None),
        ("start_home_then_origin", home, home_page, None),
        ("toolpath_half", played, played_page, Some(0.5)),
        ("start_damaged", damaged, damaged_page, None),
        ("fixture_no_material", fixture, fixture_page, None),
    ]
}

fn view_of(d: &Document, laser: &str, play: Option<f64>) -> Value {
    let bytes = pack(d, "output view").unwrap();
    let mut s = Session::open(3, &bytes).unwrap();
    assert!(s.activate_page(laser));
    if let Some(fraction) = play {
        assert!(s.toggle_toolpath_view(laser));
        s.set_play_position(fraction);
    }
    let ops: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    let rows: Vec<Value> = ops["rows"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            json!({"name": row["name"], "run": row["run"], "numbers": row["numbers"],
                "accessible_name": row["accessible_name"]})
        })
        .collect();
    json!({
        "output": serde_json::from_str::<Value>(&s.output_group_json()).unwrap(),
        "rows": rows,
        "ring_labels": serde_json::from_str::<Value>(&s.toolpath_ring_labels_json()).unwrap(),
        "play": serde_json::to_value(s.play_view()).unwrap(),
    })
}

#[test]
fn every_state_matches_its_fixture() {
    std::fs::create_dir_all(fixture_dir()).unwrap();
    for (name, d, laser, play) in states() {
        let json = view_of(&d, &laser, play);
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
fn a_drawing_page_has_no_output_view_and_no_play_bar() {
    let d = Document::new(1);
    let bytes = pack(&d, "drawing").unwrap();
    let s = Session::open(3, &bytes).unwrap();
    assert_eq!(s.output_group_json(), "null");
    assert!(s.play_view().is_none());
}

#[test]
fn the_blocked_state_names_its_blocks_first_and_disables_export() {
    let (d, laser) = state_blocked();
    let json = view_of(&d, &laser, None);
    let checks = json["output"]["checks"].as_array().unwrap();
    assert_eq!(checks[0]["severity"], "block");
    assert_eq!(checks[0]["text"], "Cut 1 has no power and speed.");
    assert_eq!(json["output"]["export"]["enabled"], false);
}

#[test]
fn the_blocked_state_also_names_the_bed_and_head_position_has_no_warn() {
    let (d, laser) = state_blocked();
    let json = view_of(&d, &laser, None);
    let texts: Vec<&str> = json["output"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["text"].as_str().unwrap())
        .collect();
    assert!(
        texts.contains(&"1 item lies outside the 400 × 300 mm bed."),
        "{texts:?}"
    );
    let (d, laser) = state_start("head_position");
    let json = view_of(&d, &laser, None);
    assert_eq!(json["output"]["start"]["value"], "head_position");
    assert_eq!(json["output"]["export"]["enabled"], true);
    assert!(
        json["output"]["checks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| !c["text"].as_str().unwrap().starts_with("Starts where"))
    );
}

#[test]
fn a_damaged_start_has_no_value_and_its_check_opens_the_pick() {
    let (d, laser) = state_start_damaged();
    let json = view_of(&d, &laser, None);
    let output = &json["output"];
    assert!(output["start"]["value"].is_null(), "{output}");
    assert_eq!(output["start"]["damaged"], true);
    let check = output["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["target"]["kind"] == "start_pick")
        .unwrap();
    assert_eq!(check["text"], "Start is damaged. Pick one above.");
    assert_eq!(output["export"]["enabled"], false);
}

#[test]
fn a_fixture_job_without_material_opens_the_fixture_pick() {
    let (d, laser) = state_fixture_no_material();
    let json = view_of(&d, &laser, None);
    let checks = json["output"]["checks"].as_array().unwrap();
    assert!(
        checks
            .iter()
            .any(|c| c["severity"] == "block" && c["target"]["kind"] == "fixture_pick"),
        "{checks:?}"
    );
}
