//! Format 47 (`specs/0113-fixture-sets-and-pins` criteria 38 to 40, ADR 0059 §1, §2, §4, §5, §7,
//! §9): the goldens `fixture_sets_v47.curvyo`, `fixture_sets_damaged_v47.curvyo` and
//! `fixtures_pre_sets_v46.curvyo` read back every value and save back unchanged; an old file's
//! fixtures read as job fixtures and nothing is written. `CURVYO_WRITE_FIXTURES=1` rewrites the
//! goldens.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, AlignCommit, Angle, CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureAdd,
    FixtureJobSwitch, FixtureParams, FixturePartKey, JobRole, ManufacturingSnapshot, NewFixture,
    NewOutputPage, NodeId, Point, Vec2, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};

const PIN: &str = "curvyo.laser/pin";
const SCREW: &str = "curvyo.laser/screw";
const HOLDER: &str = "curvyo.laser/holder";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn params(value: serde_json::Value) -> FixtureParams {
    serde_json::from_value(value).unwrap()
}

fn laser(d: &Document) -> NodeId {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: None,
        work_area: DocumentSize::from_mm(600.0, 400.0),
    })
    .unwrap()
    .kind
    .manufacturing()
    .unwrap()
}

fn new<'a>(kind: &'a str, label: &'a str, values: &'a FixtureParams, x: f64) -> NewFixture<'a> {
    NewFixture {
        add: FixtureAdd::Holder,
        kind,
        label,
        params: values,
        matrix: (kind != "curvyo.laser/engraved_grid")
            .then(|| Affine::translation(Vec2::new(x, 60.0))),
    }
}

fn slot(d: &Document, holder: NodeId, cx: f64) -> NodeId {
    let values = params(serde_json::json!({
        "shape": "rect", "cx_mm": cx, "cy_mm": 30.0, "w_mm": 20.0, "h_mm": 15.0, "d_mm": 12.0
    }));
    d.add_slot(holder, "Slot", &values).unwrap()
}

fn output_on(d: &Document, fixture: NodeId, role: JobRole) {
    let job = d
        .fixture(fixture)
        .unwrap()
        .jobs
        .into_iter()
        .find(|j| j.role == Some(role))
        .unwrap();
    d.set_fixture_job_switch(job.id, FixtureJobSwitch::Output, true)
        .unwrap();
}

/// Criterion 38: two sets (the second off for this page); in them a grid, a holder with two slots
/// and a job with a hand-set tick, a 4 x 1 screw grid with Centre mark on and a 3 mm Size, and a
/// clamp with its own check off; a locked 2 x 2 pin grid with Diameters on and a single pin as job
/// fixtures.
/// The profile ids of one golden, the same on every build of it.
fn uids() -> impl FnMut() -> String {
    let mut n = 0_u32;
    move || {
        n += 1;
        format!("00000000-0000-4000-8000-{n:012x}")
    }
}

/// The record `modified` the golden's mirror was written with.
const GOLDEN_MODIFIED: i64 = 1_763_300_000_000;

fn golden_document() -> Document {
    let d = Document::new(1);
    let mfg = laser(&d);
    let mut uid = uids();
    let bench = d.add_fixture_set(mfg, &uid()).unwrap();
    d.rename_fixture_set(bench, "Bench").unwrap();
    let grid = params(serde_json::json!({
        "spacing_mm": 10.0, "major_every": 5, "tapes": ["bottom"],
        "origin_x_mm": 0.0, "origin_y_mm": 0.0
    }));
    d.add_set_fixture(
        bench,
        &new("curvyo.laser/engraved_grid", "Grid", &grid, 0.0),
    )
    .unwrap();
    let board = params(serde_json::json!({"width_mm": 100.0, "height_mm": 60.0}));
    let holder = d
        .add_set_fixture(bench, &new(HOLDER, "Holder", &board, 100.0))
        .unwrap();
    slot(&d, holder, 30.0);
    let second = slot(&d, holder, 70.0);
    let job = d.add_fixture_job(holder, "cut", "Cut 1").unwrap();
    let tick = FixturePartKey {
        node: second,
        suffix: None,
    };
    d.set_fixture_job_targets(job, &[tick], true).unwrap();
    let jig = d.add_fixture_set(mfg, &uid()).unwrap();
    d.rename_fixture_set(jig, "Coaster jig").unwrap();
    let screws = params(serde_json::json!({
        "thread_diameter_mm": 6.0, "head_diameter_mm": 10.0, "columns": 4, "rows": 1,
        "column_spacing_mm": 30.0, "row_spacing_mm": 20.0, "mark_mm": 3.0
    }));
    let screw = d
        .add_set_fixture(jig, &new(SCREW, "Screw", &screws, 300.0))
        .unwrap();
    output_on(&d, screw, JobRole::CentreMark);
    let clamp_values = params(serde_json::json!({"width_mm": 40.0, "height_mm": 20.0}));
    let clamp = d
        .add_set_fixture(
            jig,
            &new("curvyo.laser/clamp", "Clamp", &clamp_values, 450.0),
        )
        .unwrap();
    d.set_fixture_active(clamp, false).unwrap();
    d.set_fixture_set_on_page(jig, false).unwrap();
    let grid_pins = params(serde_json::json!({
        "diameter_mm": 6.0, "columns": 2, "rows": 2,
        "column_spacing_mm": 25.0, "row_spacing_mm": 25.0, "mark_mm": 4.0
    }));
    let pins = d
        .add_job_fixture(mfg, &new(PIN, "Pin", &grid_pins, 200.0))
        .unwrap();
    output_on(&d, pins, JobRole::Diameters);
    d.set_fixture_locked(pins, true).unwrap();
    let single = params(serde_json::json!({"diameter_mm": 8.0}));
    d.add_job_fixture(mfg, &new(PIN, "Pin", &single, 500.0))
        .unwrap();
    // 2026-11-16 (ADR 0059 §9 amended): the mirror's profile ids, a locked set, a locked set
    // fixture and the record time it was written with.
    d.fill_fixture_set_uids(mfg, &mut uid, "fill_uids").unwrap();
    d.set_fixture_set_locked(jig, true).unwrap();
    d.set_fixture_locked(holder, true).unwrap();
    let records = d.manufacturing(mfg).unwrap().set_records();
    d.align_fixture_sets(mfg, &records, GOLDEN_MODIFIED, AlignCommit::Step("written"))
        .unwrap();
    d
}

fn read_or_write(name: &str, build: impl Fn() -> Vec<u8>) -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(name), build()).unwrap();
    }
    std::fs::read(fixture_path(name)).unwrap()
}

fn members(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut content = Vec::new();
            file.read_to_end(&mut content).unwrap();
            (file.name().unwrap().to_string(), content)
        })
        .collect()
}

fn rezip(files: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in files {
        writer
            .start_file(name.as_str(), zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// `bytes` with its document replaced by its state after `edit`, at the format `version`.
fn edited(bytes: &[u8], version: u32, edit: impl Fn(&LoroDoc)) -> Vec<u8> {
    let loro = LoroDoc::new();
    loro.import(&members(bytes)["document.loro"]).unwrap();
    edit(&loro);
    loro.commit();
    let mut files = members(bytes);
    files.insert(
        "document.loro".to_owned(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    );
    let mut manifest: serde_json::Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["format_version"] = serde_json::json!(version);
    files.insert(
        "manifest.json".to_owned(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    rezip(&files)
}

fn format_of(bytes: &[u8]) -> u64 {
    let json: serde_json::Value = serde_json::from_slice(&members(bytes)["manifest.json"]).unwrap();
    json["format_version"].as_u64().unwrap()
}

type Registers = BTreeMap<String, LoroValue>;

/// The registers of every node below each manufacturing, depth first, as stored.
fn raw_nodes(bytes: &[u8]) -> Vec<Registers> {
    fn walk(tree: &loro::LoroTree, node: loro::TreeID, out: &mut Vec<Registers>) {
        let mut registers = Registers::new();
        tree.get_meta(node).unwrap().for_each(|key, value| {
            registers.insert(key.to_owned(), value.into_value().unwrap());
        });
        out.push(registers);
        for child in tree.children(TreeParentId::Node(node)).unwrap_or_default() {
            walk(tree, child, out);
        }
    }
    let loro = LoroDoc::new();
    loro.import(&members(bytes)["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    let mut found = Vec::new();
    for top in tree.children(TreeParentId::Root).unwrap() {
        walk(&tree, top, &mut found);
    }
    found
}

/// The meta map of every node whose register `key` is `value`, in tree order.
fn nodes_with(loro: &LoroDoc, key: &str, value: &str) -> Vec<(loro::TreeID, loro::LoroMap)> {
    let tree = loro.get_tree("paths");
    let mut found = Vec::new();
    let mut stack: Vec<loro::TreeID> = tree.children(TreeParentId::Root).unwrap();
    stack.reverse();
    while let Some(node) = stack.pop() {
        let meta = tree.get_meta(node).unwrap();
        if meta.get(key).and_then(|v| v.into_value().ok()) == Some(LoroValue::from(value)) {
            found.push((node, meta));
        }
        let mut children = tree.children(TreeParentId::Node(node)).unwrap_or_default();
        children.reverse();
        stack.extend(children);
    }
    found
}

/// Opens `bytes` and checks that the open wrote nothing and a save keeps every register.
fn open_unchanged(bytes: &[u8]) -> Document {
    let opened = unpack(2, bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0, "opening writes no step");
    let saved = pack(&opened, "0.1.0").unwrap();
    assert_eq!(raw_nodes(&saved), raw_nodes(bytes), "saved back unchanged");
    opened
}

fn page(d: &Document) -> ManufacturingSnapshot {
    d.manufacturings().remove(0)
}

#[test]
fn the_round_trip_golden_keeps_every_set_fixture_grid_size_lock_output_and_tick() {
    let bytes = read_or_write("fixture_sets_v47.curvyo", || {
        pack(&golden_document(), "0.1.0").unwrap()
    });
    assert_eq!(format_of(&bytes), 47);
    // Format 49 since `specs/0124-gradient-fill` (ADR 0066 §8).
    // Format 50 since `specs/0123-raster-engraving` (ADR 0069 §10).
    // Format 51 since `specs/0135-image-optimisation` slice 1 (ADR 0072 §8); Superseded by 0135 slice 1 (2026-11-19).
    assert_eq!(CURRENT_FORMAT_VERSION, 55);
    let opened = open_unchanged(&bytes);
    let read = page(&opened);
    assert_eq!(read, page(&golden_document()));
    let sets: Vec<(&str, bool)> = read
        .sets
        .iter()
        .map(|s| (s.name.as_str(), s.on_page))
        .collect();
    assert_eq!(sets, [("Bench", true), ("Coaster jig", false)]);
    let listed: Vec<(&str, Option<usize>, bool)> = read
        .fixtures
        .iter()
        .map(|f| {
            let set = f
                .set
                .map(|id| read.sets.iter().position(|s| s.id == id).unwrap());
            (f.kind.as_str(), set, f.active)
        })
        .collect();
    assert_eq!(
        listed,
        [
            ("curvyo.laser/engraved_grid", Some(0), true),
            (HOLDER, Some(0), true),
            (SCREW, Some(1), true),
            ("curvyo.laser/clamp", Some(1), false),
            (PIN, None, true),
            (PIN, None, true),
        ]
    );
    let holder = &read.fixtures[1];
    assert_eq!(holder.slots.len(), 2);
    assert_eq!(holder.jobs[0].targets[0].target.slot, Some(1), "the tick");
    let screw = &read.fixtures[2];
    assert_eq!(screw.params["columns"], 4);
    assert_eq!(screw.params["mark_mm"], 3.0);
    assert_eq!(screw.jobs[0].role, Some(JobRole::CentreMark));
    assert!(screw.jobs[0].output && !screw.jobs[1].output);
    let pins = &read.fixtures[4];
    assert!(pins.locked);
    assert_eq!(
        (pins.params["columns"].clone(), pins.params["rows"].clone()),
        (2.into(), 2.into())
    );
    assert_eq!(pins.jobs[1].role, Some(JobRole::Diameters));
    assert!(pins.jobs[1].output && !pins.jobs[0].output);
    assert!(!read.fixtures[5].locked);
    assert_eq!(read.fixtures[5].params["diameter_mm"], 8.0);
    assert!(read.sets[1].locked && !read.sets[0].locked, "a locked set");
    assert!(read.fixtures[1].locked, "a locked set fixture");
    assert!(read.sets.iter().all(|s| s.uid.is_some()));
    assert!(
        read.fixtures
            .iter()
            .filter(|f| f.set.is_some())
            .all(|f| f.uid.is_some() && f.jobs.iter().all(|j| j.uid.is_some()))
    );
    assert!(
        read.job_fixtures().all(|f| f.uid.is_none()),
        "job fixtures have none"
    );
    assert_eq!(read.sets_modified, GOLDEN_MODIFIED);
    assert!(read.sets.iter().all(|s| s.damaged == 0));
    assert!(read.fixtures.iter().all(|f| f.damaged == 0));
}

/// Criterion 40: a missing set name, an On this page that is text, text in a pin's diameter, a NaN
/// spacing, a pin without a diameter, a count of 0, a product above 400 (20 x 21: 401 has no two
/// factors up to 50), a third role job, a role that does not read, a Lock that is a number and a
/// turned pin.
fn damaged_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let mfg = laser(&d);
    let mut uid = uids();
    let first = d.add_fixture_set(mfg, &uid()).unwrap();
    let second = d.add_fixture_set(mfg, &uid()).unwrap();
    // 2026-11-16: a set without its profile id, and two sets of one id (the second is not read).
    let bare = d.add_fixture_set(mfg, "").unwrap();
    d.rename_fixture_set(bare, "No id").unwrap();
    let twin_id = uid();
    let twin_a = d.add_fixture_set(mfg, &twin_id).unwrap();
    d.rename_fixture_set(twin_a, "Twin").unwrap();
    let twin_b = d.add_fixture_set(mfg, &twin_id).unwrap();
    d.rename_fixture_set(twin_b, "Twin copy").unwrap();
    let values = |columns: u32| {
        params(serde_json::json!({"diameter_mm": 6.0, "columns": columns, "rows": 1}))
    };
    let pin = |set: NodeId, name: &str, x: f64| {
        let id = d
            .add_set_fixture(set, &new(PIN, "Pin", &values(1), x))
            .unwrap();
        d.rename_fixture(id, name).unwrap();
    };
    pin(first, "text", 10.0);
    pin(first, "nan", 20.0);
    pin(second, "bare", 30.0);
    pin(second, "zero", 40.0);
    let job_pin = |name: &str, x: f64| {
        let id = d
            .add_job_fixture(mfg, &new(PIN, "Pin", &values(1), x))
            .unwrap();
        d.rename_fixture(id, name).unwrap();
    };
    job_pin("product", 50.0);
    job_pin("roles", 60.0);
    job_pin("turned", 70.0);
    job_pin("lock", 80.0);
    edited(&pack(&d, "0.1.0").unwrap(), 47, |loro| {
        let tree = loro.get_tree("paths");
        let named = |name: &str| nodes_with(loro, "name", name).remove(0);
        let put = |name: &str, key: &str, value: LoroValue| {
            let (_, meta) = named(name);
            let LoroValue::Map(map) = meta.get("fixture_params").unwrap().into_value().unwrap()
            else {
                panic!("a params map")
            };
            let mut map: HashMap<String, LoroValue> = map.as_ref().clone().into_iter().collect();
            if matches!(value, LoroValue::Null) {
                map.remove(key);
            } else {
                map.insert(key.to_owned(), value);
            }
            meta.insert("fixture_params", LoroValue::from(map)).unwrap();
        };
        let (_, set_1) = named("Set 1");
        set_1.delete("name").unwrap();
        let (_, set_2) = named("Set 2");
        set_2.insert("set_on_page", "yes").unwrap();
        put("text", "diameter_mm", LoroValue::from("wide"));
        put("nan", "column_spacing_mm", LoroValue::Double(f64::NAN));
        put("bare", "diameter_mm", LoroValue::Null);
        put("zero", "columns", LoroValue::I64(0));
        put("product", "columns", LoroValue::I64(20));
        put("product", "rows", LoroValue::I64(21));
        let (roles, _) = named("roles");
        let third = tree.create(TreeParentId::Node(roles)).unwrap();
        let meta = tree.get_meta(third).unwrap();
        meta.insert("shape", "fixture_job").unwrap();
        meta.insert("op_kind", "score").unwrap();
        meta.insert("name", "Centre mark 2").unwrap();
        meta.insert("job_role", "centre_mark").unwrap();
        let marks = nodes_with(loro, "name", "Diameters")
            .into_iter()
            .find(|(node, _)| tree.parent(*node) == Some(TreeParentId::Node(roles)))
            .unwrap()
            .1;
        marks.insert("job_role", "spin").unwrap();
        let turned = Affine::translation(Vec2::new(70.0, 60.0)).compose(Affine::rotation_about(
            Point::new(0.0, 0.0),
            Angle::from_radians(0.5),
        ));
        let entries: Vec<LoroValue> = turned
            .to_array()
            .into_iter()
            .map(LoroValue::Double)
            .collect();
        named("turned")
            .1
            .insert("fixture_matrix", LoroValue::from(entries))
            .unwrap();
        named("lock").1.insert("fixture_locked", 3_i64).unwrap();
    })
}

#[test]
fn damaged_set_values_read_as_their_defaults_are_counted_per_set_and_nothing_is_rewritten() {
    let bytes = read_or_write("fixture_sets_damaged_v47.curvyo", damaged_bytes);
    assert_eq!(format_of(&bytes), 47);
    let read = page(&open_unchanged(&bytes));
    assert_eq!(read.sets[0].name, "Set 1", "a missing name reads its place");
    assert_eq!(read.sets[0].damaged, 1);
    assert!(read.sets[1].on_page, "text reads on");
    assert_eq!(read.sets[1].damaged, 1);
    let damage = |name: &str| {
        read.fixtures
            .iter()
            .find(|f| f.name == name)
            .unwrap()
            .damaged
    };
    // Document-level damage; the kind's values (text, no diameter, zero, product, turned) are the
    // laser extension's to count (`curvyo-laser-core/tests/fixtures_stored.rs`).
    assert_eq!(damage("nan"), 1, "a NaN spacing is left out");
    assert_eq!(damage("text"), 0);
    assert_eq!(
        damage("roles"),
        2,
        "a role that does not read and a third role job"
    );
    assert_eq!(damage("lock"), 1);
    let lock = read.fixtures.iter().find(|f| f.name == "lock").unwrap();
    assert!(!lock.locked, "a damaged Lock reads off");
    let roles = read.fixtures.iter().find(|f| f.name == "roles").unwrap();
    let read_roles: Vec<Option<JobRole>> = roles.jobs.iter().map(|j| j.role).collect();
    assert_eq!(read_roles, [Some(JobRole::CentreMark), None, None]);
    assert_eq!(roles.jobs.len(), 3, "kept and read as ordinary jobs");
    let first_set: usize = read.set_fixtures(read.sets[0].id).map(|f| f.damaged).sum();
    assert_eq!(read.sets[0].damaged + first_set, 2, "the notice of Set 1");
    let names: Vec<&str> = read.sets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        names,
        ["Set 1", "Set 2", "No id", "Twin"],
        "the twin is not read"
    );
    assert_eq!(
        read.sets[2].damaged, 1,
        "a set without its profile id counts one"
    );
    assert_eq!(read.sets[3].damaged, 0);
}

/// A format-46 page as `0102` and `0103` wrote it: a holder with a slot and a job ticking it, a
/// screw with its own `0103` job (no role), flat under the manufacturing.
fn pre_sets_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let mfg = laser(&d);
    let board = params(serde_json::json!({"width_mm": 100.0, "height_mm": 60.0}));
    let holder = d
        .add_fixture(
            mfg,
            &NewFixture {
                add: FixtureAdd::Holder,
                kind: HOLDER,
                label: "Holder",
                params: &board,
                matrix: Some(Affine::translation(Vec2::new(100.0, 60.0))),
            },
        )
        .unwrap();
    let pocket = slot(&d, holder, 40.0);
    let job = d.add_fixture_job(holder, "cut", "Cut 1").unwrap();
    let tick = FixturePartKey {
        node: pocket,
        suffix: None,
    };
    d.set_fixture_job_targets(job, &[tick], true).unwrap();
    let screw_values =
        params(serde_json::json!({"thread_diameter_mm": 6.0, "head_diameter_mm": 10.0}));
    d.add_fixture(
        mfg,
        &NewFixture {
            add: FixtureAdd::Screw,
            kind: SCREW,
            label: "Screw",
            params: &screw_values,
            matrix: Some(Affine::translation(Vec2::new(300.0, 60.0))),
        },
    )
    .unwrap();
    edited(&pack(&d, "0.1.0").unwrap(), 46, |loro| {
        let tree = loro.get_tree("paths");
        let (screw, _) = nodes_with(loro, "name", "Screw 1").remove(0);
        let job = tree.create(TreeParentId::Node(screw)).unwrap();
        let meta = tree.get_meta(job).unwrap();
        meta.insert("shape", "fixture_job").unwrap();
        meta.insert("op_kind", "engrave_outline").unwrap();
        meta.insert("name", "Engrave 1").unwrap();
        meta.insert("op_show", true).unwrap();
        meta.insert("op_output", true).unwrap();
        let key = format!("target.{}:{}.head", screw.peer, screw.counter);
        meta.insert(&key, true).unwrap();
    })
}

#[test]
fn a_format_46_page_opens_unchanged_with_its_fixtures_as_job_fixtures_and_no_role_jobs() {
    let bytes = read_or_write("fixtures_pre_sets_v46.curvyo", pre_sets_bytes);
    assert_eq!(format_of(&bytes), 46);
    let read = page(&open_unchanged(&bytes));
    assert!(read.sets.is_empty(), "no set is made");
    assert_eq!(read.job_fixtures().count(), 2);
    assert!(read.fixtures.iter().all(|f| f.set.is_none() && !f.locked));
    let screw = &read.fixtures[1];
    assert_eq!(screw.kind, SCREW);
    assert_eq!(
        screw.jobs.len(),
        1,
        "its 0103 job stays, no role job is added"
    );
    assert_eq!(screw.jobs[0].role, None);
    assert!(screw.jobs[0].output, "it still runs");
    assert_eq!(screw.jobs[0].targets[0].key.suffix.as_deref(), Some("head"));
    assert_eq!(read.fixtures[0].jobs[0].targets[0].target.slot, Some(0));
    assert!(read.fixtures.iter().all(|f| f.damaged == 0));
}

#[test]
fn a_child_of_a_set_that_is_no_fixture_is_kept_and_never_read() {
    let bytes = read_or_write("fixture_sets_v47.curvyo", || {
        pack(&golden_document(), "0.1.0").unwrap()
    });
    let with_orphan = edited(&bytes, 47, |loro| {
        let tree = loro.get_tree("paths");
        let (set, _) = nodes_with(loro, "name", "Bench").remove(0);
        let orphan = tree.create(TreeParentId::Node(set)).unwrap();
        let meta = tree.get_meta(orphan).unwrap();
        meta.insert("shape", "fixture_slot").unwrap();
        meta.insert("name", "Stray").unwrap();
    });
    let read = page(&open_unchanged(&with_orphan));
    assert_eq!(read.fixtures.len(), 6, "the orphan is not read");
    assert_eq!(read.sets[0].damaged, 0);
}
