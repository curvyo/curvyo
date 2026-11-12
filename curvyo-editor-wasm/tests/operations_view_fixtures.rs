//! The contract of the Operations tab's JSON (`specs/0101-laser-operations` criteria 1, 17;
//! `adrs.md` decision 6; `specs/0111-laser-operation-tools` criteria 1, 2, 10, 15, 17 to 21 and
//! architecture finding 3): representative states are built as files, opened, put in their
//! session state, and their view JSON compared with the fixtures in
//! `tests/fixtures/operations_view/` that the TypeScript decoder reads. A drift fails. The
//! fixtures are written only when `CURVYO_UPDATE_GOLDEN` is set, as the other goldens of this
//! crate.

#![allow(clippy::unwrap_used, clippy::many_single_char_names)]

mod legacy_key;

use std::path::PathBuf;

use curvyo_document_core::{
    Affine, Document, DocumentSize, NewOperation, NewOutputPage, NodeId, OperationSwitch, Point,
    RectBounds, SettingsStep, SettingsWrite, pack,
};
use curvyo_editor_wasm::Session;
use serde_json::{Value, json};

const LASER: &str = "curvyo.laser";

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/operations_view")
}

/// A drawing page with a laser page of 400 x 300 mm; returns the document, the laser page id and
/// the laser page's origin.
fn document() -> (Document, String, Point) {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let origin = laser.origin;
    (d, laser.id, origin)
}

/// A square of 10 mm at `(x, y)` from the laser page's origin.
fn square(d: &Document, origin: Point, x: f64, y: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(origin.x + x, origin.y + y),
        Point::new(origin.x + x + 10.0, origin.y + y + 10.0),
    ))
}

fn manufacturing(d: &Document, laser: &str) -> NodeId {
    d.pages()
        .into_iter()
        .find(|p| p.id == laser)
        .unwrap()
        .kind
        .manufacturing()
        .unwrap()
}

fn add(d: &Document, m: NodeId, kind: &str, name: &str, members: &[Vec<NodeId>]) -> NodeId {
    let index = d.operations(m).unwrap().len();
    d.add_operation(m, &NewOperation { kind, name }, index, members)
        .unwrap()
}

fn state_empty() -> Document {
    document().0
}

/// Five kinds in run order with Cuts last on; one Output off, one Show off, one margin, an
/// unassigned square.
fn state_five_kinds() -> Document {
    let (d, laser, o) = document();
    let m = manufacturing(&d, &laser);
    let (a, b, c, e, g, f, _h) = (
        square(&d, o, 10.0, 10.0),
        square(&d, o, 30.0, 10.0),
        square(&d, o, 50.0, 10.0),
        square(&d, o, 70.0, 10.0),
        square(&d, o, 110.0, 10.0),
        square(&d, o, 130.0, 10.0),
        square(&d, o, 150.0, 10.0),
    );
    add(&d, m, "score", "Score 1", &[vec![a]]);
    add(&d, m, "engrave", "Engrave 1", &[vec![b], vec![c]]);
    let outline = add(&d, m, "engrave_outline", "Engrave + Outline 1", &[vec![e]]);
    add(&d, m, "cut", "Cut 1", &[vec![g]]);
    let cut_out = add(&d, m, "cut_out", "Cut-out 1", &[vec![a]]);
    d.set_operation_switch(outline, OperationSwitch::Show, false)
        .unwrap();
    d.set_operation_margin(cut_out, curvyo_document_core::Length::from_mm(2.0))
        .unwrap();
    let cut2 = add(&d, m, "cut", "Cut 2", &[vec![f]]);
    d.set_operation_switch(cut2, OperationSwitch::Output, false)
        .unwrap();
    d
}

/// The same kinds with Cuts last off: the cut stands above the score.
fn state_cuts_last_off() -> Document {
    let (d, laser, o) = document();
    let m = manufacturing(&d, &laser);
    let a = square(&d, o, 10.0, 10.0);
    let b = square(&d, o, 30.0, 10.0);
    add(&d, m, "cut", "Cut 1", &[vec![a]]);
    add(&d, m, "score", "Score 1", &[vec![b]]);
    d.set_cuts_last(
        m,
        false,
        &d.operations(m)
            .unwrap()
            .iter()
            .map(|op| op.id)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    d
}

/// A group of two squares cloned, one member's legacy key (a file of format 39 to 43, Frage 212 A:
/// not counted, shown nowhere); an operation of an unknown kind holding one square.
fn state_unknown_and_clones() -> Document {
    let (d, laser, o) = document();
    let m = manufacturing(&d, &laser);
    let (a, b) = (square(&d, o, 10.0, 10.0), square(&d, o, 30.0, 10.0));
    let group = d.group_objects(&[a, b]).unwrap();
    let clone = d.linked_clone(&[group], &[]).unwrap()[0];
    let path = d.clone_member_path(clone, a).unwrap();
    add(&d, m, "cut", "Cut 1", &[vec![clone]]);
    add(&d, m, "plasma", "Plasma 1", &[vec![b]]);
    legacy_key::with_legacy_key(&d, "Cut 1", &path)
}

fn map(value: &Value) -> serde_json::Map<String, Value> {
    value.as_object().unwrap().clone()
}

fn settings(d: &Document, op: NodeId, value: &Value) {
    d.write_operation_settings(
        SettingsStep::EditSettings,
        &[SettingsWrite {
            operation: op,
            value: Some(map(value)),
        }],
    )
    .unwrap();
}

/// An Engrave with own numbers, an interval of 0.25 mm and every option set (`specs/0111`
/// criteria 19, 20); the machine gets a motorised Z axis and the row is unfolded.
fn state_options_engrave() -> Document {
    let (d, laser, o) = document();
    let m = manufacturing(&d, &laser);
    let a = square(&d, o, 10.0, 10.0);
    let engrave = add(&d, m, "engrave", "Engrave 1", &[vec![a]]);
    settings(
        &d,
        engrave,
        &json!({"main": {"power_pct": 40.0, "speed_mm_min": 3000, "passes": 1,
                         "interval_mm": 0.25}}),
    );
    d.write_operation_options(
        engrave,
        &map(
            &json!({"crosshatch": true, "bidirectional": false, "scan_angle_deg": 15.0,
                     "overscan_pct": 5, "z_offset_mm": 1.5}),
        ),
    )
    .unwrap();
    d
}

/// A Cut with a Z offset stored while the machine has no motorised Z axis (`specs/0111`
/// criterion 21): the value is kept, the field not shown; the row is unfolded.
fn state_options_cut_z_hidden() -> Document {
    let (d, laser, o) = document();
    let m = manufacturing(&d, &laser);
    let a = square(&d, o, 10.0, 10.0);
    let cut = add(&d, m, "cut", "Cut 1", &[vec![a]]);
    d.write_operation_options(cut, &map(&json!({"z_offset_mm": 1.5})))
        .unwrap();
    d
}

/// An Engrave whose `op_options` hold an unknown key: Damaged settings, no options (`specs/0111`
/// criterion 25).
fn state_options_damaged() -> Document {
    let (d, laser, o) = document();
    let m = manufacturing(&d, &laser);
    let a = square(&d, o, 10.0, 10.0);
    let engrave = add(&d, m, "engrave", "Engrave 1", &[vec![a]]);
    d.write_operation_options(engrave, &map(&json!({"spot_size": 1})))
        .unwrap();
    d
}

/// Group G of A and B, B in Engrave 1, G assigned to Cut 1 by the group rule (G and B); G is
/// selected: Cut 1 checked, Engrave 1 mixed with the group in line 2 (`specs/0111` criteria 9, 10).
fn state_group_tick_mixed() -> Document {
    let (d, laser, o) = document();
    let m = manufacturing(&d, &laser);
    let (a, b) = (square(&d, o, 10.0, 10.0), square(&d, o, 30.0, 10.0));
    let group = d.group_objects(&[a, b]).unwrap();
    add(&d, m, "engrave", "Engrave 1", &[vec![b]]);
    add(&d, m, "cut", "Cut 1", &[vec![group], vec![b]]);
    d
}

/// One square selected with Engrave 1 on the page: the kind buttons and what each press does
/// (`specs/0111` criteria 1, 2).
fn state_select_bar_kinds() -> Document {
    let (d, laser, o) = document();
    let m = manufacturing(&d, &laser);
    let a = square(&d, o, 10.0, 10.0);
    square(&d, o, 30.0, 10.0);
    add(&d, m, "engrave", "Engrave 1", &[vec![a]]);
    d
}

/// A drawing square placed on the laser page, assigned as a whole and by its inner row; a clone.
fn state_placed_and_clones() -> Document {
    let (d, laser, o) = document();
    let m = manufacturing(&d, &laser);
    let drawing_page = d.pages().into_iter().find(|p| p.id != laser).unwrap();
    let drawing = drawing_page.id.clone();
    let (dx, dy) = (drawing_page.origin.x, drawing_page.origin.y);
    let inner = d.create_rect(RectBounds::from_corners(
        Point::new(dx + 5.0, dy + 5.0),
        Point::new(dx + 25.0, dy + 15.0),
    ));
    let placement = d
        .place_page(
            &laser,
            &drawing,
            Affine::translation(curvyo_document_core::Vec2::new(o.x + 10.0, o.y + 10.0)),
        )
        .unwrap();
    let c = square(&d, o, 200.0, 200.0);
    add(&d, m, "cut", "Cut 1", &[vec![placement]]);
    add(&d, m, "engrave", "Engrave 1", &[vec![inner]]);
    add(&d, m, "score", "Score 1", &[vec![c]]);
    d
}

/// The session state a scenario is shown in, after the file is opened.
type Setup = fn(&mut Session);

fn no_setup(_: &mut Session) {}

/// Unfolds the first row.
fn unfold_first(s: &mut Session) {
    let id = s.operations_view().unwrap().rows[0].id;
    assert!(s.unfold_operation_row(id));
}

fn motorised_and_unfolded(s: &mut Session) {
    assert!(s.set_machine_motorised_z(true));
    unfold_first(s);
}

/// Selects the page's running items (the group's two leaves select the group).
fn select_running(s: &mut Session) {
    assert!(s.select_operation_items("running"));
}

fn select_unassigned(s: &mut Session) {
    assert!(s.select_operation_items("not_assigned"));
}

/// The states, by fixture name.
fn states() -> Vec<(&'static str, Document, Setup)> {
    vec![
        ("empty", state_empty(), no_setup),
        ("five_kinds_cuts_last", state_five_kinds(), no_setup),
        ("cuts_last_off", state_cuts_last_off(), no_setup),
        (
            "unknown_and_clone_members",
            state_unknown_and_clones(),
            no_setup,
        ),
        ("placed_and_clone_rows", state_placed_and_clones(), no_setup),
        (
            "options_engrave",
            state_options_engrave(),
            motorised_and_unfolded,
        ),
        (
            "options_cut_z_hidden",
            state_options_cut_z_hidden(),
            unfold_first,
        ),
        ("options_damaged", state_options_damaged(), unfold_first),
        ("group_tick_mixed", state_group_tick_mixed(), select_running),
        (
            "select_bar_kinds",
            state_select_bar_kinds(),
            select_unassigned,
        ),
    ]
}

fn view_of(d: &Document, setup: Setup) -> serde_json::Value {
    let bytes = pack(d, "operations view").unwrap();
    let laser = d
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap()
        .id;
    let mut s = Session::open(3, &bytes).unwrap();
    s.load_machine_profiles_json("[]");
    assert!(s.activate_page(&laser));
    setup(&mut s);
    serde_json::from_str(&s.operations_view_json()).unwrap()
}

#[test]
fn every_state_matches_its_fixture_and_nothing_is_missing() {
    std::fs::create_dir_all(fixture_dir()).unwrap();
    for (name, d, setup) in states() {
        let json = view_of(&d, setup);
        let text = format!("{}\n", serde_json::to_string_pretty(&json).unwrap());
        assert!(
            !text.contains("missing"),
            "{name}: Frage 169 A, no missing count or row"
        );
        let path = fixture_dir().join(format!("{name}.json"));
        if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
            std::fs::write(&path, &text).unwrap();
        }
        let want: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("{} exists (CURVYO_UPDATE_GOLDEN=1)", path.display())),
        )
        .unwrap();
        assert_eq!(json, want, "{name} drifted from {}", path.display());
    }
}

#[test]
fn the_summary_counts_follow_the_disjoint_rule() {
    let json = view_of(&state_five_kinds(), no_setup);
    let summary = &json["summary"];
    let count = |key: &str| summary[key].as_u64().unwrap();
    assert!(count("runs_while_hidden") <= count("running"), "{summary}");
    assert_eq!(count("not_assigned"), 1, "{summary}");
    assert_eq!(count("output_off"), 1, "{summary}");
    assert_eq!(count("operations"), 5, "{summary}");
}

#[test]
fn the_group_scenario_reads_checked_and_mixed_and_the_legacy_key_is_not_counted() {
    let row = |json: &Value, name: &str| {
        json["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == name)
            .unwrap()
            .clone()
    };
    let json = view_of(&state_group_tick_mixed(), select_running);
    assert_eq!(row(&json, "Cut 1")["tick"], "all");
    assert_eq!(row(&json, "Engrave 1")["tick"], "some");
    assert!(
        row(&json, "Engrave 1")["group_in_part"]
            .as_str()
            .unwrap()
            .starts_with("Group ")
    );
    let legacy = view_of(&state_unknown_and_clones(), no_setup);
    assert_eq!(
        row(&legacy, "Cut 1")["items"],
        1,
        "the clone only, not its member's legacy key"
    );
}
