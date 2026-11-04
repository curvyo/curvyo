//! Independent acceptance tests for `specs/0027-manufacturing-roles` at the document model and
//! file format (ADR 0041, format 36): the generic `x.<process>.*` register command (one step, no
//! step for an unchanged value, refusals; criteria 4, 5, ADR 0041 §4), the drawing never written
//! (4), per-layer merge of concurrent peers (5), no role written by Add manufacturing or a new
//! layer (3), stale and unknown entries kept and never rewritten (17, 18), the container check on
//! open (18), the golden round trip and the files from before the slice (19), and delete of a
//! manufacturing with undo (20).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::assert_is_empty,
    clippy::many_single_char_names
)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, DocumentSize, DocumentVersion, ManufacturingError,
    NewManufacturing, NodeId, NodeKind, OpenError, Point, RectBounds, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeID, TreeParentId};
use serde_json::{Value, json};

const LABEL: &str = "set_role";
const ROLE_PREFIX: &str = "x.curvyo.laser.role.";

// ---- helpers ------------------------------------------------------------------------------------

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn key(layer: NodeId) -> String {
    format!("{ROLE_PREFIX}{}", layer.to_register_text())
}

fn laser(d: &Document) -> NodeId {
    let first = d.pages()[0].id.clone();
    d.add_manufacturing(NewManufacturing {
        process: "curvyo.laser",
        label: "Laser",
        profile: None,
        work_area: DocumentSize::from_mm(400.0, 300.0),
        place: Some(&first),
    })
    .unwrap()
}

/// A new layer called `name` with one 10 x 10 rectangle at x.
fn layer(d: &Document, name: &str, x: f64) -> NodeId {
    let l = d.new_layer(None);
    d.rename_node(l, name).unwrap();
    d.set_draw_parent(Some(l));
    let _ = d.create_rect(RectBounds::from_corners(
        Point::new(x, 10.0),
        Point::new(x + 10.0, 20.0),
    ));
    d.set_draw_parent(None);
    l
}

fn set(d: &Document, m: NodeId, k: &str, v: Option<&str>) -> bool {
    d.set_process_register(m, k, v.map(Value::from).as_ref(), LABEL)
        .unwrap()
}

fn roles(d: &Document, m: NodeId) -> BTreeMap<String, Value> {
    d.manufacturing(m)
        .unwrap()
        .process_values
        .into_iter()
        .filter(|(k, _)| k.starts_with(ROLE_PREFIX))
        .collect()
}

fn member(bytes: &[u8], name: &str) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Vec::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_end(&mut out)
        .unwrap();
    out
}

fn stored_loro(bytes: &[u8]) -> LoroDoc {
    let l = LoroDoc::new();
    l.import(&member(bytes, "document.loro")).unwrap();
    l
}

fn rezip(loro: &LoroDoc, version: u32) -> Vec<u8> {
    let manifest = json!({
        "format_version": version,
        "loro_snapshot_version": curvyo_document_core::CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "tester",
    });
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let o = zip::write::SimpleFileOptions::default();
    w.start_file("manifest.json", o).unwrap();
    w.write_all(manifest.to_string().as_bytes()).unwrap();
    w.start_file("document.loro", o).unwrap();
    w.write_all(&loro.export(loro::ExportMode::Snapshot).unwrap())
        .unwrap();
    w.start_file("document.json", o).unwrap();
    w.write_all(b"{}").unwrap();
    w.finish().unwrap().into_inner()
}

fn all_nodes(tree: &loro::LoroTree) -> Vec<TreeID> {
    let mut out = Vec::new();
    let mut stack: Vec<TreeID> = tree.children(TreeParentId::Root).unwrap_or_default();
    while let Some(n) = stack.pop() {
        out.push(n);
        stack.extend(tree.children(TreeParentId::Node(n)).unwrap_or_default());
    }
    out
}

fn is_manufacturing(tree: &loro::LoroTree, n: TreeID) -> bool {
    tree.get_meta(n).unwrap().get("manufacturing").is_some()
}

fn manufacturing_node(loro: &LoroDoc) -> TreeID {
    let tree = loro.get_tree("paths");
    tree.children(TreeParentId::Root)
        .unwrap()
        .into_iter()
        .find(|&n| is_manufacturing(&tree, n))
        .expect("a manufacturing node")
}

/// Every register of every drawing node (all nodes outside the manufacturings) plus every root
/// container other than the node tree, in the saved file: what criterion 4 calls "the saved
/// registers of the drawing's nodes (layers, objects, pages, styles)".
fn drawing_registers(d: &Document) -> String {
    let loro = stored_loro(&pack(d, "0.1.0").unwrap());
    let tree = loro.get_tree("paths");
    let mut manufacturing_subtrees = Vec::new();
    for n in tree.children(TreeParentId::Root).unwrap() {
        if is_manufacturing(&tree, n) {
            manufacturing_subtrees.push(n);
        }
    }
    let mut skip = std::collections::HashSet::new();
    let mut stack = manufacturing_subtrees.clone();
    while let Some(n) = stack.pop() {
        skip.insert(n);
        stack.extend(tree.children(TreeParentId::Node(n)).unwrap_or_default());
    }
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    for n in all_nodes(&tree) {
        if skip.contains(&n) {
            continue;
        }
        let meta = tree.get_meta(n).unwrap().get_deep_value();
        out.insert(format!("{n:?}"), format!("{meta:?}"));
    }
    let mut deep = loro.get_deep_value();
    if let LoroValue::Map(m) = &mut deep {
        let mut m = (**m).clone();
        m.remove("paths");
        // The output pages belong to the manufacturings (`0093`), not to the drawing.
        if let Some(LoroValue::Map(root)) = m.get("root").cloned() {
            let mut root = (*root).clone();
            root.retain(|k, _| !k.starts_with("page.mfg-"));
            m.insert("root".into(), LoroValue::Map(root.into()));
        }
        out.insert("roots".into(), format!("{m:?}"));
    }
    format!("{out:?} {:?}", d.pages())
}

fn undo_last(d: &Document) {
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
}

fn synced(a: &Document, b: &Document) {
    let from_a = a.export_updates(&DocumentVersion::never()).unwrap();
    let from_b = b.export_updates(&DocumentVersion::never()).unwrap();
    a.import_updates(&from_b).unwrap();
    b.import_updates(&from_a).unwrap();
}

// ---- criterion 3: nothing assigned for the maker --------------------------------------------

#[test]
fn ac3_add_manufacturing_and_new_layers_write_no_role() {
    let d = Document::new(1);
    let a = layer(&d, "A", 10.0);
    let m = laser(&d);
    assert!(roles(&d, m).is_empty(), "Add manufacturing writes no role");
    let b = layer(&d, "B", 30.0);
    assert!(roles(&d, m).is_empty(), "a later layer writes no role");
    let bytes = pack(&d, "0.1.0").unwrap();
    let loro = stored_loro(&bytes);
    let tree = loro.get_tree("paths");
    for n in all_nodes(&tree) {
        let meta = tree.get_meta(n).unwrap().get_deep_value();
        assert!(
            !format!("{meta:?}").contains("x.curvyo.laser.role"),
            "no role register anywhere: {meta:?}"
        );
    }
    let _ = (a, b);
}

// ---- criterion 4: one step, drawing untouched --------------------------------------------------

#[test]
fn ac4_set_a_in_m1_only_one_step_undo_and_the_drawing_is_byte_identical() {
    let d = Document::new(1);
    let a = layer(&d, "A", 10.0);
    let b = layer(&d, "B", 30.0);
    let m1 = laser(&d);
    let m2 = laser(&d);
    let drawing = drawing_registers(&d);
    let steps = d.own_step_count();

    assert!(set(&d, m1, &key(a), Some("cut")));
    assert_eq!(d.own_step_count(), steps + 1, "one undo step");
    assert_eq!(d.own_steps().last().unwrap().label, LABEL);
    assert_eq!(roles(&d, m1).get(&key(a)), Some(&Value::from("cut")));
    assert!(roles(&d, m2).is_empty(), "M2 still shows A as No role");
    assert_eq!(
        drawing_registers(&d),
        drawing,
        "drawing unchanged after set"
    );

    assert!(
        !set(&d, m1, &key(a), Some("cut")),
        "pressing the same state again"
    );
    assert_eq!(d.own_step_count(), steps + 1, "no second step");

    assert!(set(&d, m1, &key(b), Some("engrave")));
    assert!(set(&d, m1, &key(a), Some("ignore")));
    assert_eq!(roles(&d, m1).get(&key(a)), Some(&Value::from("ignore")));
    assert_eq!(drawing_registers(&d), drawing);

    let own = d.own_steps();
    let last3: Vec<_> = own[own.len() - 3..].iter().map(|s| s.step).collect();
    d.undo_step(last3[2]).unwrap();
    assert_eq!(roles(&d, m1).get(&key(a)), Some(&Value::from("cut")));
    d.undo_step(last3[1]).unwrap();
    d.undo_step(last3[0]).unwrap();
    assert!(roles(&d, m1).is_empty(), "undo makes A No role again");
    assert_eq!(
        drawing_registers(&d),
        drawing,
        "drawing unchanged after undo"
    );
}

#[test]
fn ac2_clearing_deletes_the_register_in_one_step_and_clearing_nothing_is_no_step() {
    let d = Document::new(1);
    let a = layer(&d, "A", 10.0);
    let m = laser(&d);
    let steps = d.own_step_count();
    assert!(!set(&d, m, &key(a), None), "clearing an absent role");
    assert_eq!(d.own_step_count(), steps);
    set(&d, m, &key(a), Some("engrave"));
    assert!(set(&d, m, &key(a), None));
    assert_eq!(d.own_step_count(), steps + 2);
    assert!(roles(&d, m).is_empty());
    let loro = stored_loro(&pack(&d, "0.1.0").unwrap());
    let meta = loro
        .get_tree("paths")
        .get_meta(manufacturing_node(&loro))
        .unwrap();
    assert!(meta.get(&key(a)).is_none(), "the register is deleted");
    undo_last(&d);
    assert_eq!(roles(&d, m).get(&key(a)), Some(&Value::from("engrave")));
}

// ---- criterion 5: storage -------------------------------------------------------------------

#[test]
fn ac5_the_register_sits_on_the_manufacturing_as_a_plain_string_never_on_the_layer() {
    let d = Document::new(3);
    let a = layer(&d, "A", 10.0);
    let m = laser(&d);
    set(&d, m, &key(a), Some("cut"));
    set(&d, m, &format!("{ROLE_PREFIX}outside"), Some("engrave"));
    let loro = stored_loro(&pack(&d, "0.1.0").unwrap());
    let tree = loro.get_tree("paths");
    let mn = manufacturing_node(&loro);
    let meta = tree.get_meta(mn).unwrap();
    assert!(key(a).starts_with("x.curvyo.laser.role.3:"), "{}", key(a));
    assert_eq!(
        meta.get(&key(a)).unwrap().get_deep_value(),
        LoroValue::from("cut")
    );
    assert_eq!(
        meta.get("x.curvyo.laser.role.outside")
            .unwrap()
            .get_deep_value(),
        LoroValue::from("engrave")
    );
    for n in all_nodes(&tree) {
        if n == mn {
            continue;
        }
        let v = format!("{:?}", tree.get_meta(n).unwrap().get_deep_value());
        assert!(!v.contains("x.curvyo"), "a role on a drawing node: {v}");
    }
}

#[test]
fn ac5_two_peers_on_different_layers_both_survive_the_merge() {
    let d = Document::new(1);
    let a = layer(&d, "A", 10.0);
    let b = layer(&d, "B", 30.0);
    let m = laser(&d);
    let bytes = pack(&d, "0.1.0").unwrap();
    let one = unpack(11, &bytes).unwrap();
    let two = unpack(12, &bytes).unwrap();
    set(&one, m, &key(a), Some("cut"));
    set(&two, m, &key(b), Some("engrave"));
    synced(&one, &two);
    for peer in [&one, &two] {
        let r = roles(peer, m);
        assert_eq!(r.get(&key(a)), Some(&Value::from("cut")), "{r:?}");
        assert_eq!(r.get(&key(b)), Some(&Value::from("engrave")), "{r:?}");
    }
    // And through a save and reopen of the merge.
    let back = unpack(13, &pack(&one, "0.1.0").unwrap()).unwrap();
    assert_eq!(roles(&back, m), roles(&two, m));
}

#[test]
fn ac5_two_peers_set_first_roles_concurrently_in_a_fresh_manufacturing() {
    // The case option B of ADR 0041 loses: both peers write the very first roles at once.
    let d = Document::new(1);
    let a = layer(&d, "A", 10.0);
    let b = layer(&d, "B", 30.0);
    let c = layer(&d, "C", 50.0);
    let m = laser(&d);
    let bytes = pack(&d, "0.1.0").unwrap();
    let one = unpack(21, &bytes).unwrap();
    let two = unpack(22, &bytes).unwrap();
    set(&one, m, &key(a), Some("cut"));
    set(&one, m, &format!("{ROLE_PREFIX}outside"), Some("ignore"));
    set(&two, m, &key(b), Some("engrave"));
    set(&two, m, &key(c), Some("ignore"));
    synced(&one, &two);
    assert_eq!(roles(&one, m).len(), 4, "{:?}", roles(&one, m));
    assert_eq!(roles(&one, m), roles(&two, m));
}

#[test]
fn ac5_same_layer_by_both_peers_converges_to_one_of_the_two() {
    let d = Document::new(1);
    let a = layer(&d, "A", 10.0);
    let m = laser(&d);
    let bytes = pack(&d, "0.1.0").unwrap();
    let one = unpack(31, &bytes).unwrap();
    let two = unpack(32, &bytes).unwrap();
    set(&one, m, &key(a), Some("cut"));
    set(&two, m, &key(a), Some("engrave"));
    synced(&one, &two);
    let r1 = roles(&one, m);
    assert_eq!(r1, roles(&two, m), "both peers converge");
    let v = r1.get(&key(a)).unwrap();
    assert!(v == "cut" || v == "engrave", "{v}");
}

#[test]
fn ac5_one_peer_clears_while_the_other_sets_another_layer() {
    let d = Document::new(1);
    let a = layer(&d, "A", 10.0);
    let b = layer(&d, "B", 30.0);
    let m = laser(&d);
    set(&d, m, &key(a), Some("cut"));
    let bytes = pack(&d, "0.1.0").unwrap();
    let one = unpack(41, &bytes).unwrap();
    let two = unpack(42, &bytes).unwrap();
    set(&one, m, &key(a), None);
    set(&two, m, &key(b), Some("engrave"));
    synced(&one, &two);
    let r = roles(&one, m);
    assert_eq!(r.get(&key(a)), None, "the clear survives: {r:?}");
    assert_eq!(r.get(&key(b)), Some(&Value::from("engrave")));
    assert_eq!(r, roles(&two, m));
}

// ---- ADR 0041 §4: refusals of the generic command ---------------------------------------------

#[test]
fn adr4_keys_outside_the_own_namespace_and_unknown_ids_are_refused_without_a_step() {
    let d = Document::new(1);
    let a = layer(&d, "A", 10.0);
    let m = laser(&d);
    let steps = d.own_step_count();
    let before = d.manufacturing(m).unwrap();
    for bad in [
        "x.curvyo.laser.",
        "x.curvyo.laser",
        "x.curvyo.lase.role.1:1",
        "x.curvyo.plotter.role.1:1",
        "x.curvyo.laserx.role",
        "mfg_profile",
        "manufacturing",
        "label",
        "x.",
        "",
        "X.curvyo.laser.role.1:1",
    ] {
        let r = d.set_process_register(m, bad, Some(&Value::from("cut")), LABEL);
        assert!(
            matches!(r, Err(ManufacturingError::OutsideProcess)),
            "{bad:?}: {r:?}"
        );
        let r = d.set_process_register(m, bad, None, LABEL);
        assert!(r.is_err(), "{bad:?} delete: {r:?}");
    }
    assert!(matches!(
        d.set_process_register(a, &key(a), Some(&Value::from("cut")), LABEL),
        Err(ManufacturingError::NotFound)
    ));
    let gone = d.new_layer(None);
    d.delete_layer(gone).unwrap();
    let steps_now = d.own_step_count();
    assert!(
        d.set_process_register(gone, &key(a), Some(&Value::from("cut")), LABEL)
            .is_err()
    );
    assert_eq!(d.own_step_count(), steps_now);
    assert_eq!(steps + 2, steps_now, "only the layer add and delete wrote");
    assert_eq!(
        d.manufacturing(m).unwrap().process_values,
        before.process_values
    );
}

#[test]
fn adr4_whatever_value_the_command_accepts_reopens() {
    // No build may write a value the open check refuses (ADR 0041 §3 "No build writes one").
    let d = Document::new(1);
    let a = layer(&d, "A", 10.0);
    let m = laser(&d);
    for v in [
        json!("score"),
        json!(1),
        json!(1.5),
        json!(true),
        json!(null),
        json!([1, 2]),
        json!({"a": 1}),
        json!(""),
    ] {
        if d.set_process_register(m, &key(a), Some(&v), LABEL).is_ok() {
            let bytes = pack(&d, "0.1.0").unwrap();
            let back = unpack(2, &bytes);
            assert!(
                back.is_ok(),
                "{v} was accepted but does not reopen: {:?}",
                back.err()
            );
        }
    }
}

// ---- criterion 17: deleted layers -----------------------------------------------------------

#[test]
fn ac17_deleting_a_layer_keeps_its_role_and_undo_brings_both_back() {
    let d = Document::new(1);
    let a = layer(&d, "A", 10.0);
    let m = laser(&d);
    set(&d, m, &key(a), Some("cut"));
    d.delete_layer(a).unwrap();
    assert!(!d.contains_node(a));
    assert_eq!(
        roles(&d, m).get(&key(a)),
        Some(&Value::from("cut")),
        "the drawing command does not write the manufacturing"
    );
    undo_last(&d);
    assert!(d.contains_node(a), "the same node id comes back");
    assert_eq!(roles(&d, m).get(&key(a)), Some(&Value::from("cut")));
}

#[test]
fn ac17_a_stale_entry_opens_is_kept_and_saved_back_byte_for_byte() {
    let d = Document::new(1);
    let _a = layer(&d, "A", 10.0);
    let m = laser(&d);
    let stale = "x.curvyo.laser.role.99:12345";
    set(&d, m, stale, Some("engrave"));
    let first = pack(&d, "0.1.0").unwrap();
    let back = unpack(2, &first).unwrap();
    assert_eq!(roles(&back, m).get(stale), Some(&Value::from("engrave")));
    let second = pack(&back, "0.1.0").unwrap();
    let l1 = stored_loro(&first);
    let l2 = stored_loro(&second);
    let v1 = l1
        .get_tree("paths")
        .get_meta(manufacturing_node(&l1))
        .unwrap()
        .get_deep_value();
    let v2 = l2
        .get_tree("paths")
        .get_meta(manufacturing_node(&l2))
        .unwrap()
        .get_deep_value();
    assert_eq!(v1, v2, "nothing rewritten or removed on read");
    assert_eq!(
        l1.oplog_frontiers(),
        l2.oplog_frontiers(),
        "open and save wrote no change"
    );
}

// ---- criterion 18: unknown values and containers ----------------------------------------------

#[test]
fn ac18_unknown_and_non_string_values_survive_open_and_save_unchanged() {
    let d = Document::new(1);
    let a = layer(&d, "A", 10.0);
    let b = layer(&d, "B", 30.0);
    let c = layer(&d, "C", 50.0);
    let m = laser(&d);
    let bytes = pack(&d, "0.1.0").unwrap();
    let loro = stored_loro(&bytes);
    let tree = loro.get_tree("paths");
    let meta = tree.get_meta(manufacturing_node(&loro)).unwrap();
    meta.insert(&key(a), "score").unwrap();
    meta.insert(&key(b), 7_i64).unwrap();
    meta.insert(&key(c), true).unwrap();
    loro.commit();
    let crafted = rezip(&loro, CURRENT_FORMAT_VERSION);
    let back = unpack(5, &crafted).expect("a file with unknown roles opens");
    let r = roles(&back, m);
    assert_eq!(r.get(&key(a)), Some(&Value::from("score")));
    assert_eq!(r.get(&key(b)), Some(&Value::from(7)));
    assert_eq!(r.get(&key(c)), Some(&Value::from(true)));
    let again = stored_loro(&pack(&back, "0.1.0").unwrap());
    assert_eq!(
        loro.oplog_frontiers(),
        again.oplog_frontiers(),
        "never rewritten on read"
    );
    // A role press replaces it in one step.
    let steps = back.own_step_count();
    assert!(set(&back, m, &key(a), Some("cut")));
    assert_eq!(back.own_step_count(), steps + 1);
    assert_eq!(roles(&back, m).get(&key(a)), Some(&Value::from("cut")));
}

fn with_container(k: &str, list: bool) -> Vec<u8> {
    let d = Document::new(1);
    let _a = layer(&d, "A", 10.0);
    let _m = laser(&d);
    let loro = stored_loro(&pack(&d, "0.1.0").unwrap());
    let meta = loro
        .get_tree("paths")
        .get_meta(manufacturing_node(&loro))
        .unwrap();
    if list {
        let l = meta.insert_container(k, loro::LoroList::new()).unwrap();
        l.push("cut").unwrap();
    } else {
        let m = meta.insert_container(k, loro::LoroMap::new()).unwrap();
        m.insert("1:2", "cut").unwrap();
    }
    loro.commit();
    rezip(&loro, CURRENT_FORMAT_VERSION)
}

#[test]
fn ac18_a_container_under_a_role_key_is_refused_as_damaged() {
    for list in [false, true] {
        let bytes = with_container("x.curvyo.laser.role.1:2", list);
        let r = unpack(3, &bytes);
        assert!(
            matches!(r, Err(OpenError::Damaged)),
            "list={list}: {:?}",
            r.map(|_| ())
        );
    }
}

#[test]
fn ac18_a_container_under_any_x_key_is_refused_also_for_a_family_or_another_process() {
    for k in [
        "x.curvyo.laser.roles",
        "x.curvyo.laser.role.outside",
        "x.acme.cnc.anything",
    ] {
        let r = unpack(3, &with_container(k, false));
        assert!(
            matches!(r, Err(OpenError::Damaged)),
            "{k}: {:?}",
            r.map(|_| ())
        );
    }
}

// ---- criterion 19: golden round trip ----------------------------------------------------------

fn by_name(d: &Document) -> BTreeMap<String, NodeId> {
    d.read_nodes()
        .into_iter()
        .filter(|n| matches!(n.kind, NodeKind::Layer))
        .map(|n| (n.registers.name.clone().unwrap_or_default(), n.id))
        .collect()
}

#[test]
fn ac19_the_golden_holds_a_to_d_the_stale_entry_and_score_and_round_trips() {
    let bytes = std::fs::read(fixture("laser_roles_v36.curvyo")).expect("golden present");
    let manifest: Value = serde_json::from_slice(&member(&bytes, "manifest.json")).unwrap();
    assert_eq!(manifest["format_version"], 36);
    assert_eq!(CURRENT_FORMAT_VERSION, 36, "the slice does not bump");
    let d = unpack(7, &bytes).unwrap();
    let ms = d.manufacturings();
    assert_eq!(ms.len(), 1);
    let m = ms[0].id;
    let layers = by_name(&d);
    let r = roles(&d, m);
    let want = [
        ("A", Some("cut")),
        ("B", Some("engrave")),
        ("C", Some("ignore")),
        ("D", None),
    ];
    for (name, role) in want {
        let id = layers
            .get(name)
            .unwrap_or_else(|| panic!("layer {name}: {layers:?}"));
        assert_eq!(r.get(&key(*id)).and_then(Value::as_str), role, "{name}");
    }
    let unknown: Vec<_> = r.values().filter(|v| *v == "score").collect();
    assert_eq!(unknown.len(), 1, "one unknown role score: {r:?}");
    let live: Vec<String> = layers.values().map(|id| key(*id)).collect();
    let stale: Vec<_> = r.keys().filter(|k| !live.contains(k)).collect();
    assert_eq!(
        stale.len(),
        1,
        "exactly one entry for a deleted layer: {r:?}"
    );
    assert!(
        NodeId::from_register_text(stale[0].trim_start_matches(ROLE_PREFIX)).is_some(),
        "{stale:?} is a node id"
    );
    let again = unpack(8, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(
        roles(&again, m),
        r,
        "every role identical after save and reopen"
    );
    assert_eq!(
        stored_loro(&bytes).oplog_frontiers(),
        stored_loro(&pack(&d, "0.1.0").unwrap()).oplog_frontiers(),
        "open and save write nothing"
    );
}

#[test]
fn ac19_files_from_before_the_slice_open_with_no_role_and_write_nothing() {
    for name in ["laser_workbench_v36.curvyo", "machine_profile_v36.curvyo"] {
        let bytes = std::fs::read(fixture(name)).unwrap();
        let d = unpack(4, &bytes).unwrap();
        for m in d.manufacturings() {
            assert!(
                !m.process_values.keys().any(|k| k.starts_with(ROLE_PREFIX)),
                "{name}: {:?}",
                m.process_values
            );
        }
        assert_eq!(
            stored_loro(&bytes).oplog_frontiers(),
            stored_loro(&pack(&d, "0.1.0").unwrap()).oplog_frontiers(),
            "{name}: open writes nothing"
        );
    }
}

// ---- criterion 20: deleting the manufacturing ---------------------------------------------------

#[test]
fn ac20_deleting_the_manufacturing_takes_its_roles_and_undo_restores_them() {
    let d = Document::new(1);
    let a = layer(&d, "A", 10.0);
    let m = laser(&d);
    set(&d, m, &key(a), Some("cut"));
    set(&d, m, &format!("{ROLE_PREFIX}outside"), Some("ignore"));
    let drawing = drawing_registers(&d);
    let before = roles(&d, m);
    d.delete_manufacturing(m).unwrap();
    assert!(d.manufacturing(m).is_none());
    assert_eq!(drawing_registers(&d), drawing, "the drawing is untouched");
    let loro = stored_loro(&pack(&d, "0.1.0").unwrap());
    assert!(
        !format!("{:?}", loro.get_deep_value()).contains("x.curvyo.laser.role"),
        "the roles went with it"
    );
    undo_last(&d);
    assert_eq!(roles(&d, m), before);
}
