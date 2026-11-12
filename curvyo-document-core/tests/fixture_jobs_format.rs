//! Format 41 (`specs/0103-fixture-jobs` criteria 20 to 22, ADR 0052 §3, §5, §6): the goldens
//! `fixture_jobs_v41.curvyo`, `fixture_jobs_unknown_kind_v41.curvyo` and
//! `fixture_jobs_damaged_v41.curvyo` read back every job, switch and tick and save back
//! unchanged; the format 40 goldens open with no job; a format 42 file is refused.
//! `CURVYO_WRITE_FIXTURES=1` rewrites the goldens.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureAdd, FixtureJobSwitch,
    FixtureParams, FixturePartKey, FixtureSnapshot, NewFixture, NewOutputPage, NodeId, OpenError,
    Vec2, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
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

fn add(
    d: &Document,
    mfg: NodeId,
    add: FixtureAdd,
    kind: &str,
    values: serde_json::Value,
) -> NodeId {
    let params: FixtureParams = serde_json::from_value(values).unwrap();
    d.add_fixture(
        mfg,
        &NewFixture {
            add,
            kind,
            label: kind.rsplit('/').next().unwrap(),
            params: &params,
            matrix: (add != FixtureAdd::Grid).then(|| Affine::translation(Vec2::new(20.0, 20.0))),
        },
    )
    .unwrap()
}

fn holder(d: &Document, mfg: NodeId, slots: usize) -> (NodeId, Vec<NodeId>) {
    let values = serde_json::json!({"width_mm": 100.0, "height_mm": 60.0});
    let id = add(d, mfg, FixtureAdd::Holder, "curvyo.laser/holder", values);
    let slot: FixtureParams = serde_json::from_value(serde_json::json!({
        "shape": "circle", "cx_mm": 30.0, "cy_mm": 30.0, "w_mm": 0.0, "h_mm": 0.0, "d_mm": 12.0
    }))
    .unwrap();
    // Each slot gets a name of its own, so no two adjacent steps share a label (the
    // legacy-history oracle of `0020`).
    let slots = (0..slots)
        .map(|n| {
            let id = d.add_slot(id, "Coaster", &slot).unwrap();
            assert!(d.set_slot(id, &format!("Pocket {}", n + 1), &slot).unwrap());
            id
        })
        .collect();
    (id, slots)
}

fn grid(d: &Document, mfg: NodeId) -> NodeId {
    let values = serde_json::json!({"spacing_mm": 10.0, "major_every": 5, "tapes": ["bottom"]});
    add(
        d,
        mfg,
        FixtureAdd::Grid,
        "curvyo.laser/engraved_grid",
        values,
    )
}

fn part(node: NodeId, suffix: Option<&str>) -> FixturePartKey {
    FixturePartKey {
        node,
        suffix: suffix.map(str::to_owned),
    }
}

/// Criterion 20: a holder with an Engrave + Outline job on its board and two slots (Output on),
/// a Cut job on a grid's lines (Output off), a grid with no job, an inactive holder with one job.
fn golden_document() -> Document {
    let d = Document::new(1);
    let mfg = laser(&d);
    let (board, slots) = holder(&d, mfg, 3);
    let engrave = d
        .add_fixture_job(board, "engrave_outline", "Engrave + Outline 1")
        .unwrap();
    let ticks = [
        part(board, None),
        part(slots[0], None),
        part(slots[2], None),
    ];
    d.set_fixture_job_targets(engrave, &ticks, true).unwrap();
    d.set_fixture_job_switch(engrave, FixtureJobSwitch::Output, true)
        .unwrap();
    let lines = grid(&d, mfg);
    let cut = d.add_fixture_job(lines, "cut", "Cut 1").unwrap();
    d.set_fixture_job_targets(cut, &[part(lines, Some("lines"))], true)
        .unwrap();
    grid(&d, mfg);
    let (inactive, _) = holder(&d, mfg, 0);
    d.set_fixture_active(inactive, false).unwrap();
    let job = d.add_fixture_job(inactive, "cut_out", "Cut-out 1").unwrap();
    d.set_fixture_job_targets(job, &[part(inactive, None)], true)
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

/// `bytes` with its document edited by `edit` on every job node's meta map.
fn edited_jobs(bytes: &[u8], edit: impl Fn(usize, &loro::LoroMap)) -> Vec<u8> {
    let loro = LoroDoc::new();
    loro.import(&members(bytes)["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    let mut stack: Vec<loro::TreeID> = tree.children(TreeParentId::Root).unwrap();
    let mut jobs = Vec::new();
    while let Some(node) = stack.pop() {
        let meta = tree.get_meta(node).unwrap();
        if meta.get("shape").and_then(|v| v.into_value().ok())
            == Some(LoroValue::from("fixture_job"))
        {
            jobs.push((node, meta));
        }
        stack.extend(tree.children(TreeParentId::Node(node)).unwrap_or_default());
    }
    jobs.sort_by_key(|(node, _)| (node.peer, node.counter));
    for (at, (_, meta)) in jobs.iter().enumerate() {
        edit(at, meta);
    }
    loro.commit();
    let mut files = members(bytes);
    files.insert(
        "document.loro".to_owned(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    );
    rezip(&files)
}

fn with_format(bytes: &[u8], version: u32) -> Vec<u8> {
    let mut files = members(bytes);
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

/// The registers of every node of the document, sorted, as stored.
fn raw_nodes(bytes: &[u8]) -> Vec<BTreeMap<String, LoroValue>> {
    let loro = LoroDoc::new();
    loro.import(&members(bytes)["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    let mut out = Vec::new();
    let mut stack: Vec<loro::TreeID> = tree.children(TreeParentId::Root).unwrap();
    while let Some(node) = stack.pop() {
        let mut registers = BTreeMap::new();
        tree.get_meta(node).unwrap().for_each(|key, value| {
            registers.insert(key.to_owned(), value.into_value().unwrap());
        });
        out.push(registers);
        stack.extend(tree.children(TreeParentId::Node(node)).unwrap_or_default());
    }
    out.sort_by_key(|r| format!("{r:?}"));
    out
}

fn open_unchanged(bytes: &[u8]) -> Vec<FixtureSnapshot> {
    let opened = unpack(2, bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0, "opening writes no step");
    let saved = pack(&opened, "0.1.0").unwrap();
    assert_eq!(raw_nodes(&saved), raw_nodes(bytes), "saved back unchanged");
    opened.manufacturings()[0].fixtures.clone()
}

fn golden_bytes() -> Vec<u8> {
    edited_jobs(&pack(&golden_document(), "0.1.0").unwrap(), |at, meta| {
        if at == 0 {
            // A tick of a node that is gone (a merge leftover): kept, not damage, no target.
            meta.insert("target.999:999", true).unwrap();
        }
    })
}

#[test]
fn the_round_trip_golden_reads_back_every_job_switch_and_tick() {
    let bytes = read_or_write("fixture_jobs_v41.curvyo", golden_bytes);
    assert_eq!(format_of(&bytes), 41);
    // Format 42 since `specs/0110-pen-modes-variable-stroke` (ADR 0053); the v41 golden stays.
    // Format 44 since `specs/0111-laser-operation-tools` (ADR 0056 §8).
    assert_eq!(CURRENT_FORMAT_VERSION, 45);
    let read = open_unchanged(&bytes);
    assert_eq!(read.len(), 4);
    let engrave = &read[0].jobs[0];
    assert_eq!(engrave.name, "Engrave + Outline 1");
    assert_eq!(engrave.kind.as_deref(), Some("engrave_outline"));
    assert!(engrave.show && engrave.output);
    let places: Vec<Option<usize>> = engrave.targets.iter().map(|t| t.target.slot).collect();
    assert_eq!(
        places,
        [None, Some(0), Some(2)],
        "the board, then the slots"
    );
    assert!(engrave.targets.iter().all(|t| t.ticked));
    assert_eq!(read[0].slots.len(), 3, "a job is no slot");
    assert_eq!(read[0].damaged, 0, "the dead key is no damage");
    let cut = &read[1].jobs[0];
    assert!(cut.show && !cut.output, "a new job's Output is off");
    assert_eq!(cut.targets[0].target.suffix.as_deref(), Some("lines"));
    assert!(read[2].jobs.is_empty(), "the grid without a job");
    assert!(!read[3].active && read[3].jobs.len() == 1);
    assert!(read.iter().all(|f| f.damaged == 0));
    let raw = raw_nodes(&bytes);
    assert!(raw.iter().any(|r| r.contains_key("target.999:999")), "kept");
}

fn unknown_kind_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let mfg = laser(&d);
    let (board, _) = holder(&d, mfg, 1);
    let weld = d.add_fixture_job(board, "weld", "Weld 1").unwrap();
    d.set_fixture_job_targets(weld, &[part(board, None)], true)
        .unwrap();
    d.add_fixture_job(board, "cut", "Cut 1").unwrap();
    edited_jobs(&pack(&d, "0.1.0").unwrap(), |at, meta| {
        if at == 1 {
            meta.insert("op_kind", 7.0).unwrap();
        }
    })
}

#[test]
fn an_unknown_kind_is_kept_with_its_ticks_and_saved_back_unchanged() {
    let bytes = read_or_write("fixture_jobs_unknown_kind_v41.curvyo", unknown_kind_bytes);
    assert_eq!(format_of(&bytes), 41);
    let read = open_unchanged(&bytes);
    let jobs = &read[0].jobs;
    assert_eq!(jobs[0].kind.as_deref(), Some("weld"), "opaque here");
    assert_eq!(jobs[0].targets.len(), 1);
    assert_eq!(jobs[1].kind, None, "a kind that is not a text");
    assert_eq!(jobs[1].damaged, 1);
    assert_eq!(read[0].damaged, 1);
    assert!(jobs[1].record().is_none(), "no kind, no record");
}

/// Criterion 22: a text Show, a text Output, a tick of a slot whose value is `false`, and a
/// `.tapes` suffix on a holder.
fn damaged_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let mfg = laser(&d);
    let (board, slots) = holder(&d, mfg, 1);
    let job = d.add_fixture_job(board, "engrave", "Engrave 1").unwrap();
    let ticks = [part(slots[0], None), part(board, Some("tapes"))];
    d.set_fixture_job_targets(job, &ticks, true).unwrap();
    edited_jobs(&pack(&d, "0.1.0").unwrap(), |_, meta| {
        meta.insert("op_show", "yes").unwrap();
        meta.insert("op_output", "on").unwrap();
        let mut slot_key = None;
        meta.for_each(|key, _| {
            if key
                .strip_prefix("target.")
                .is_some_and(|rest| !rest.contains('.'))
            {
                slot_key = Some(key.to_owned());
            }
        });
        meta.insert(&slot_key.unwrap(), false).unwrap();
    })
}

#[test]
fn damaged_values_read_safe_are_counted_and_nothing_is_rewritten() {
    let bytes = read_or_write("fixture_jobs_damaged_v41.curvyo", damaged_bytes);
    assert_eq!(format_of(&bytes), 41);
    let read = open_unchanged(&bytes);
    let job = &read[0].jobs[0];
    assert!(job.show, "a text Show reads on");
    assert!(!job.output, "a text Output reads off, never on");
    let false_key = job.targets.iter().find(|t| !t.ticked).unwrap();
    assert_eq!(false_key.target.slot, Some(0), "kept, no target");
    assert!(
        job.targets
            .iter()
            .any(|t| t.ticked && t.target.suffix.as_deref() == Some("tapes")),
        "the extension judges the suffix"
    );
    assert_eq!(job.damaged, 3, "Show, Output and the false value");
    assert_eq!(read[0].damaged, 3);
}

#[test]
fn the_format_40_goldens_open_with_no_job() {
    for name in [
        "fixtures_v40.curvyo",
        "fixtures_unknown_kind_v40.curvyo",
        "fixtures_damaged_v40.curvyo",
        "fixtures_slot_orphan_v40.curvyo",
        "fixtures_grid_v39.curvyo",
    ] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        let opened = unpack(2, &bytes).unwrap();
        assert_eq!(opened.own_step_count(), 0, "{name}");
        let fixtures = &opened.manufacturings()[0].fixtures;
        assert!(fixtures.iter().all(|f| f.jobs.is_empty()), "{name}");
    }
}

#[test]
fn a_file_of_format_42_is_refused() {
    let bytes = with_format(
        &read_or_write("fixture_jobs_v41.curvyo", golden_bytes),
        CURRENT_FORMAT_VERSION + 1,
    );
    assert!(matches!(
        unpack(2, &bytes),
        // Superseded by `0110` (format 42) and `0028` (format 43): the next number is 44.
        Err(OpenError::FormatTooNew {
            found: 46,
            supported: 45
        })
    ));
}
