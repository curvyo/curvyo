//! Tester acceptance cases for `specs/0028-material-test-library` in the document model,
//! written from the specification: a peer merge of two concurrent settings writes keeps one
//! whole value, never a mix (ADR 0054 §1, criteria 24 to 26), an unknown operation keeps its
//! settings untouched (criterion 28, R-MFG-010), a format 41 file opens with no settings and no
//! grid and saves back unchanged (28), and the settings survive a pack round trip (28).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, DocumentSize, NewOperation, NewOutputPage, NodeId,
    SettingsStep, SettingsWrite, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};
use serde_json::{Map, Value, json};

fn laser(kinds: &[&str]) -> (Document, NodeId, Vec<NodeId>) {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let m = made.kind.manufacturing().unwrap();
    let ids = kinds
        .iter()
        .enumerate()
        .map(|(i, kind)| {
            let name = format!("op {i}");
            d.add_operation(m, &NewOperation { kind, name: &name }, i, &[])
                .unwrap()
        })
        .collect();
    (d, m, ids)
}

fn map(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        _ => unreachable!(),
    }
}

fn settings_of(d: &Document, m: NodeId, op: NodeId) -> Option<Value> {
    d.operations(m)
        .unwrap()
        .into_iter()
        .find(|o| o.id == op)
        .unwrap()
        .settings
}

fn write(d: &Document, step: SettingsStep, op: NodeId, v: Value) {
    d.write_operation_settings(
        step,
        &[SettingsWrite {
            operation: op,
            value: Some(map(v)),
        }],
    )
    .unwrap();
}

fn merge(a: &Document, b: &Document) {
    let aa = a.export_updates(&b.version()).unwrap();
    let bb = b.export_updates(&a.version()).unwrap();
    a.import_updates(&bb).unwrap();
    b.import_updates(&aa).unwrap();
}

fn birch() -> Value {
    json!({
        "main": {"power_pct": 60.0, "speed_mm_min": 300, "passes": 2},
        "material": {"id": "22222222-2222-4222-8222-222222222222", "name": "Birch", "modified": 5}
    })
}

fn card() -> Value {
    json!({
        "main": {"power_pct": 40.0, "speed_mm_min": 900, "passes": 1},
        "material": {"id": "11111111-1111-4111-8111-111111111111", "name": "Card", "modified": 7}
    })
}

#[test]
fn merge_of_two_concurrent_picks_keeps_one_whole_value() {
    let (a, m, ids) = laser(&["cut"]);
    let b = unpack(2, &pack(&a, "t").unwrap()).unwrap();
    write(&a, SettingsStep::UseMaterial, ids[0], birch());
    write(&b, SettingsStep::UseMaterial, ids[0], card());
    merge(&a, &b);
    let sa = settings_of(&a, m, ids[0]).unwrap();
    let sb = settings_of(&b, m, ids[0]).unwrap();
    assert_eq!(sa, sb, "both peers agree");
    assert!(sa == birch() || sa == card(), "never a mix: {sa}");
}

#[test]
fn merge_of_an_override_and_a_concurrent_pick_keeps_one_whole_value() {
    let (a, m, ids) = laser(&["cut"]);
    write(&a, SettingsStep::UseMaterial, ids[0], birch());
    let b = unpack(2, &pack(&a, "t").unwrap()).unwrap();
    let mut typed = birch();
    typed["overrides"] = json!({"main": {"power_pct": 65.0}});
    write(&a, SettingsStep::EditSettings, ids[0], typed.clone());
    write(&b, SettingsStep::UseMaterial, ids[0], card());
    merge(&a, &b);
    let s = settings_of(&a, m, ids[0]).unwrap();
    assert_eq!(Some(&s), settings_of(&b, m, ids[0]).as_ref());
    assert!(
        s == typed || s == card(),
        "no override typed for Birch over Card: {s}"
    );
}

#[test]
fn settings_survive_a_pack_round_trip_and_reopen_writes_no_step() {
    let (d, m, ids) = laser(&["cut", "engrave_outline", "score"]);
    let mut eo = json!({
        "main": {"power_pct": 35.0, "speed_mm_min": 6000, "passes": 1},
        "outline": {"power_pct": 12.5, "speed_mm_min": 3000, "passes": 1},
        "material": {"id": "22222222-2222-4222-8222-222222222222", "name": "Birch",
                     "outline_from": "score", "modified": 5}
    });
    eo["overrides"] = json!({"outline": {"passes": 2}});
    write(&d, SettingsStep::UseMaterial, ids[0], birch());
    write(&d, SettingsStep::UseMaterial, ids[1], eo.clone());
    write(
        &d,
        SettingsStep::EditSettings,
        ids[2],
        json!({"main": {"power_pct": 22.0, "speed_mm_min": 2200, "passes": 1}}),
    );
    let bytes = pack(&d, "t").unwrap();
    let r = unpack(3, &bytes).unwrap();
    assert_eq!(r.own_step_count(), 0);
    assert_eq!(settings_of(&r, m, ids[0]), Some(birch()));
    assert_eq!(settings_of(&r, m, ids[1]), Some(eo));
    assert_eq!(
        settings_of(&r, m, ids[2]).unwrap()["main"]["power_pct"],
        json!(22.0)
    );
    assert_eq!(format_of(&bytes), u64::from(CURRENT_FORMAT_VERSION));
    assert_eq!(CURRENT_FORMAT_VERSION, 45);
}

// ---- format 41 ----------------------------------------------------------------------------------

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
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

fn format_of(bytes: &[u8]) -> u64 {
    let json: Value = serde_json::from_slice(&members(bytes)["manifest.json"]).unwrap();
    json["format_version"].as_u64().unwrap()
}

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

#[test]
fn c28_a_format_41_file_opens_with_no_settings_no_grid_and_saves_back_unchanged() {
    let bytes = fixture("fixture_jobs_v41.curvyo");
    assert_eq!(format_of(&bytes), 41);
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0);
    for mf in d.manufacturings() {
        for op in d.operations(mf.id).unwrap() {
            assert_eq!(op.settings, None);
            assert!(!op.grid);
        }
    }
    let saved = pack(&d, "t").unwrap();
    assert_eq!(raw_nodes(&saved), raw_nodes(&bytes), "saved back unchanged");
    assert!(
        !raw_nodes(&saved).iter().any(|r| r
            .keys()
            .any(|k| k == "op_settings" || k.contains("test_grid"))),
        "no settings or grid key is added"
    );
}

#[test]
fn c28_an_unknown_operation_keeps_its_settings_untouched() {
    let (d, m, ids) = laser(&["cut"]);
    write(&d, SettingsStep::UseMaterial, ids[0], birch());
    // Make the kind unknown to this build by rewriting the stored register.
    let bytes = pack(&d, "t").unwrap();
    let loro = LoroDoc::new();
    let mut files = members(&bytes);
    loro.import(&files["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    let mut stack: Vec<loro::TreeID> = tree.children(TreeParentId::Root).unwrap();
    while let Some(node) = stack.pop() {
        let meta = tree.get_meta(node).unwrap();
        if meta.get("op_kind").is_some() {
            meta.insert("op_kind", "weld_2030").unwrap();
        }
        stack.extend(tree.children(TreeParentId::Node(node)).unwrap_or_default());
    }
    loro.commit();
    files.insert(
        "document.loro".to_owned(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    );
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in &files {
        writer
            .start_file(name.as_str(), zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut writer, content).unwrap();
    }
    let edited = writer.finish().unwrap().into_inner();
    let r = unpack(3, &edited).unwrap();
    let op = r
        .operations(m)
        .unwrap()
        .into_iter()
        .find(|o| o.id == ids[0])
        .unwrap();
    assert_eq!(op.kind.as_deref(), Some("weld_2030"));
    assert_eq!(op.settings, Some(birch()));
    let saved = pack(&r, "t").unwrap();
    assert_eq!(raw_nodes(&saved), raw_nodes(&edited));
}
