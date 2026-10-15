//! The stored shape of clone overrides (`specs/0057-clone-overrides` criteria 37 to 39, ADR 0016
//! §1, §5 and §6): save and open to the last bit, the golden file, damaged files, residue, older
//! files, and the `document.json` entry.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    AnchorId, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, CloneSnapshot, Color,
    Document, Length, NewAnchor, NodeId, ObjectSnapshot, Opacity, OpenError, Point, Style,
    StyleEdit, Vec2, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeID};

/// The version this slice takes. A rebase onto another bump renumbers this constant, the
/// fixtures and their names (`specs/README.md`, "`format_version` plan").
const OVERRIDES_FORMAT_VERSION: u32 = 14;
const GOLDEN: &str = "clone_overrides_v14.curvyo";
const DAMAGED: &str = "clone_overrides_flag_damaged_v14.curvyo";

const RED: Color = Color { r: 255, g: 0, b: 0 };
const GREEN: Color = Color { r: 0, g: 255, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const ORANGE: Color = Color {
    r: 255,
    g: 128,
    b: 1,
};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn loro_of(document: &Document) -> LoroDoc {
    let loro = LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    loro
}

fn container_from_loro(loro_bytes: &[u8]) -> Vec<u8> {
    let manifest = serde_json::json!({
        "format_version": CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "tester",
    });
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in [
        ("manifest.json", serde_json::to_vec(&manifest).unwrap()),
        ("document.loro", loro_bytes.to_vec()),
        ("document.json", b"{}".to_vec()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn file_from_loro(loro: &LoroDoc) -> Vec<u8> {
    loro.commit();
    container_from_loro(&loro.export(loro::ExportMode::Snapshot).unwrap())
}

fn read_member(bytes: &[u8], name: &str) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Vec::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_end(&mut out)
        .unwrap();
    out
}

fn manifest_version(bytes: &[u8]) -> u32 {
    let manifest: serde_json::Value =
        serde_json::from_slice(&read_member(bytes, "manifest.json")).unwrap();
    u32::try_from(manifest["format_version"].as_u64().unwrap()).unwrap()
}

fn tree_id(id: NodeId) -> TreeID {
    let json = serde_json::to_value(id).unwrap();
    TreeID::new(
        json["peer"].as_str().unwrap().parse().unwrap(),
        i32::try_from(json["counter"].as_i64().unwrap()).unwrap(),
    )
}

fn square(document: &Document, n: u64, at: Point) -> NodeId {
    let corner = |k: u64, dx: f64, dy: f64| {
        NewAnchor::corner(
            AnchorId::new(1, n * 10 + k),
            Point::new(at.x + dx, at.y + dy),
        )
    };
    document.create_path(
        &[
            corner(0, 0.0, 0.0),
            corner(1, 10.0, 0.0),
            corner(2, 10.0, 10.0),
            corner(3, 0.0, 10.0),
        ],
        true,
    )
}

fn clone_at(d: &Document, original: NodeId, x: f64, y: f64) -> NodeId {
    let clone = d.linked_clone(&[original], &[]).unwrap()[0];
    d.translate_objects(&[clone], Vec2::new(x, y)).unwrap();
    clone
}

fn clone_of(d: &Document, id: NodeId) -> CloneSnapshot {
    match d.object(id) {
        Some(ObjectSnapshot::Clone(clone)) => clone,
        other => panic!("not a clone: {other:?}"),
    }
}

fn clones(document: &Document) -> Vec<CloneSnapshot> {
    document
        .read_objects()
        .objects
        .iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Clone(clone) => Some(clone.clone()),
            _ => None,
        })
        .collect()
}

fn opacity_of(d: &Document, id: NodeId) -> f64 {
    d.read_nodes()
        .into_iter()
        .find(|n| n.id == id)
        .unwrap()
        .opacity
        .get()
}

/// What the golden holds, by role.
struct Golden {
    document: Document,
    placed: NodeId,
    styled: NodeId,
    both: NodeId,
    middle: NodeId,
    behind: NodeId,
}

/// A root with a red 0.5 mm dashed stroke, a blue fill at 40 % and opacity 80 %; a clone with own
/// placement, one with own style, one with both; then the root turns green at 50 %. A second root
/// with a chain B → A → O2 whose middle clone A has own style; then O2 turns orange.
fn golden() -> Golden {
    let d = Document::new(1);
    let root = square(&d, 1, Point::new(0.0, 0.0));
    // Never two adjacent steps with one label: an older file shows those as one row.
    for edit in [
        StyleEdit::StrokeColor(RED),
        StyleEdit::StrokeWidth(Length::from_mm(0.5)),
        StyleEdit::FillEnabled(true),
        StyleEdit::FillColor(BLUE),
        StyleEdit::FillOpacity(Opacity::new(0.4).unwrap()),
    ] {
        d.edit_style(&[root], &edit).unwrap();
        d.translate_objects(&[root], Vec2::new(1.0, 0.0)).unwrap();
    }
    d.set_node_opacity(&[root], Opacity::new(0.8).unwrap())
        .unwrap();
    let placed = clone_at(&d, root, 30.0, 0.0);
    let styled = clone_at(&d, root, 60.0, 0.0);
    let both = clone_at(&d, root, 90.0, 0.0);
    d.set_own_placement(&[placed, both], true).unwrap();
    d.set_own_style(&[styled, both], true).unwrap();
    d.edit_style(&[root], &StyleEdit::StrokeColor(GREEN))
        .unwrap();
    d.set_node_opacity(&[root], Opacity::new(0.5).unwrap())
        .unwrap();
    let chain_root = square(&d, 2, Point::new(0.0, 40.0));
    let middle = clone_at(&d, chain_root, 30.0, 0.0);
    let behind = clone_at(&d, middle, 30.0, 0.0);
    d.set_own_style(&[middle], true).unwrap();
    d.edit_style(&[chain_root], &StyleEdit::StrokeColor(ORANGE))
        .unwrap();
    Golden {
        document: d,
        placed,
        styled,
        both,
        middle,
        behind,
    }
}

fn red_stroke(style: &Style) -> bool {
    style.stroke.color == RED
        && style.stroke.width.as_mm() == 0.5
        && style.fill.enabled
        && style.fill.color == BLUE
        && style.fill.opacity.get() == 0.4
}

/// A clone whose `own_style` register holds a number, as no writer of ours stores it.
fn damaged_file() -> Vec<u8> {
    let d = Document::new(1);
    let root = square(&d, 1, Point::new(0.0, 0.0));
    let clone = d.linked_clone(&[root], &[]).unwrap()[0];
    let loro = loro_of(&d);
    loro.set_peer_id(7).unwrap();
    let meta = loro.get_tree("paths").get_meta(tree_id(clone)).unwrap();
    meta.insert("own_style", 1_i64).unwrap();
    file_from_loro(&loro)
}

fn write_fixtures_on_request() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(
            fixture_path(GOLDEN),
            pack(&golden().document, "0.1.0").unwrap(),
        )
        .unwrap();
        std::fs::write(fixture_path(DAMAGED), damaged_file()).unwrap();
    }
}

/// A file with one clone of a square whose meta map gets `edit`, as another build or a merge
/// could leave it.
fn file_with_clone(edit: impl FnOnce(&loro::LoroMap)) -> (Vec<u8>, NodeId, NodeId) {
    let d = Document::new(1);
    let root = square(&d, 1, Point::new(0.0, 0.0));
    let clone = d.linked_clone(&[root], &[]).unwrap()[0];
    let loro = loro_of(&d);
    loro.set_peer_id(7).unwrap();
    edit(&loro.get_tree("paths").get_meta(tree_id(clone)).unwrap());
    (file_from_loro(&loro), root, clone)
}

#[test]
fn ac37_this_slice_takes_its_format_version() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= OVERRIDES_FORMAT_VERSION);
    let bytes = pack(&Document::new(1), "t").unwrap();
    assert_eq!(manifest_version(&bytes), CURRENT_FORMAT_VERSION);
}

#[test]
fn ac37_save_and_open_keep_every_state_style_opacity_and_matrix_to_the_last_bit() {
    let g = golden();
    let reopened = unpack(2, &pack(&g.document, "t").unwrap()).unwrap();
    assert_eq!(reopened.read_objects(), g.document.read_objects());
    assert_eq!(reopened.read_nodes(), g.document.read_nodes());
    assert_eq!(
        reopened.export_json().unwrap(),
        g.document.export_json().unwrap()
    );
}

#[test]
fn ac37_the_golden_reads_back_and_opening_it_writes_nothing() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&bytes), OVERRIDES_FORMAT_VERSION);
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.read_objects().orphans, Vec::<NodeId>::new());
    // The ids of the golden are those `golden()` mints: the build is deterministic.
    let g = golden();
    let (placed, styled, both) = (
        clone_of(&d, g.placed),
        clone_of(&d, g.styled),
        clone_of(&d, g.both),
    );
    assert!(placed.chain.own_placement && !placed.chain.own_style);
    assert!(!styled.chain.own_placement && styled.chain.own_style);
    assert!(both.chain.own_placement && both.chain.own_style);
    assert_eq!(
        placed.resolved.style.stroke.color, GREEN,
        "follows the root"
    );
    assert!(
        red_stroke(&styled.resolved.style),
        "{:?}",
        styled.resolved.style
    );
    assert!(red_stroke(&both.resolved.style));
    assert_eq!(opacity_of(&d, g.placed), 0.5);
    assert_eq!(opacity_of(&d, g.styled), 0.8);
    assert_eq!(opacity_of(&d, g.both), 0.8);
    let (middle, behind) = (clone_of(&d, g.middle), clone_of(&d, g.behind));
    assert_eq!(middle.resolved.style.stroke.color, Color::BLACK);
    assert_eq!(behind.resolved.style, middle.resolved.style);
    assert_eq!(behind.chain.style_source, g.middle);
    assert_eq!(behind.chain.depth, 2);
    assert_eq!(
        styled.chain.matrix.to_array().map(f64::to_bits),
        [1.0, 0.0, 0.0, 1.0, 60.0, 0.0].map(f64::to_bits)
    );
    let stored = LoroDoc::new();
    stored
        .import(&read_member(&bytes, "document.loro"))
        .unwrap();
    assert_eq!(loro_of(&d).oplog_vv(), stored.oplog_vv(), "nothing written");
}

#[test]
fn ac38_a_damaged_flag_is_refused_and_does_not_crash() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(DAMAGED)).unwrap();
    assert_eq!(manifest_version(&bytes), OVERRIDES_FORMAT_VERSION);
    assert!(matches!(unpack(2, &bytes), Err(OpenError::Damaged)));
    let cases: [(&str, &str, LoroValue); 5] = [
        ("placement a number", "own_placement", LoroValue::I64(1)),
        (
            "placement a string",
            "own_placement",
            LoroValue::from("true"),
        ),
        ("style a string", "own_style", LoroValue::from("yes")),
        ("style a list", "own_style", LoroValue::from(vec![true])),
        ("opacity out of range", "opacity", LoroValue::Double(1.5)),
    ];
    for (name, key, value) in cases {
        let (bytes, ..) = file_with_clone(|meta| {
            meta.insert(key, value.clone()).unwrap();
        });
        assert!(
            matches!(unpack(2, &bytes), Err(OpenError::Damaged)),
            "{name}"
        );
    }
}

#[test]
fn ac38_a_damaged_own_style_is_refused_and_the_same_keys_without_it_are_residue() {
    for (key, value) in [
        ("stroke_width", LoroValue::Double(-1.0)),
        ("stroke", LoroValue::from(vec![300_i64, 0, 0])),
        ("fill_opacity", LoroValue::Double(2.0)),
        ("stroke_join", LoroValue::from("wobbly")),
    ] {
        let (residue, root, clone) = file_with_clone(|meta| {
            meta.insert(key, value.clone()).unwrap();
        });
        let d = unpack(2, &residue).unwrap_or_else(|e| panic!("{key}: {e:?}"));
        let root_style = match d.object(root).unwrap() {
            ObjectSnapshot::Path(path) => path.style,
            other => panic!("{other:?}"),
        };
        assert_eq!(clone_of(&d, clone).resolved.style, root_style, "{key}");
        let (own, ..) = file_with_clone(|meta| {
            meta.insert(key, value.clone()).unwrap();
            meta.insert("own_style", true).unwrap();
        });
        assert!(
            matches!(unpack(2, &own), Err(OpenError::Damaged)),
            "{key} with own style"
        );
    }
}

#[test]
fn ac38_flags_on_a_path_are_residue_whatever_their_type() {
    let d = Document::new(1);
    let p = square(&d, 1, Point::new(0.0, 0.0));
    let loro = loro_of(&d);
    loro.set_peer_id(7).unwrap();
    let meta = loro.get_tree("paths").get_meta(tree_id(p)).unwrap();
    meta.insert("own_placement", true).unwrap();
    meta.insert("own_style", 3_i64).unwrap();
    let opened = unpack(2, &file_from_loro(&loro)).unwrap();
    assert!(matches!(opened.object(p), Some(ObjectSnapshot::Path(_))));
    let json: serde_json::Value = serde_json::from_slice(&opened.export_json().unwrap()).unwrap();
    assert!(json["objects"][0].get("own_style").is_none());
}

#[test]
fn ac38_an_own_style_with_missing_keys_reads_their_defaults() {
    let (bytes, _, clone) = file_with_clone(|meta| {
        meta.insert("own_style", true).unwrap();
        meta.insert("stroke", vec![255_i64, 0, 0]).unwrap();
    });
    let d = unpack(2, &bytes).unwrap();
    let mut expected = Style::default();
    expected.stroke.color = RED;
    assert_eq!(clone_of(&d, clone).resolved.style, expected);
    assert_eq!(opacity_of(&d, clone), 1.0, "absent opacity is 1");
}

#[test]
fn ac38_a_stored_false_is_the_default_and_opacity_without_own_style_never_counts() {
    let (bytes, root, clone) = file_with_clone(|meta| {
        meta.insert("own_style", false).unwrap();
        meta.insert("own_placement", false).unwrap();
        meta.insert("fill_enabled", true).unwrap();
        meta.insert("opacity", 0.25).unwrap();
    });
    let d = unpack(2, &bytes).unwrap();
    let chain = clone_of(&d, clone).chain;
    assert!(!chain.own_placement && !chain.own_style);
    assert_eq!(chain.style_source, root);
    assert_eq!(opacity_of(&d, clone), 1.0);
    let json: serde_json::Value = serde_json::from_slice(&d.export_json().unwrap()).unwrap();
    let keys: Vec<_> = json["objects"][1].as_object().unwrap().keys().collect();
    assert_eq!(
        keys,
        ["id", "matrix", "origin", "shape"],
        "residue is not listed"
    );
}

#[test]
fn ac39_document_json_lists_the_states_and_the_own_style_with_its_opacity() {
    let g = golden();
    let json: serde_json::Value =
        serde_json::from_slice(&g.document.export_json().unwrap()).unwrap();
    let entry = |id: NodeId| {
        json["objects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["id"] == serde_json::to_value(id).unwrap())
            .unwrap()
            .clone()
    };
    let placed = entry(g.placed);
    assert_eq!(placed["own_placement"], true);
    assert!(placed.get("own_style").is_none() && placed.get("style").is_none());
    assert!(placed.get("opacity").is_none());
    let both = entry(g.both);
    assert_eq!(both["own_placement"], true);
    assert_eq!(both["own_style"], true);
    assert_eq!(both["opacity"], 0.8);
    assert_eq!(
        both["style"]["stroke"]["color"],
        serde_json::json!({"r": 255, "g": 0, "b": 0})
    );
    assert_eq!(
        both["matrix"],
        serde_json::json!([1.0, 0.0, 0.0, 1.0, 90.0, 0.0])
    );
    let styled = entry(g.styled);
    assert!(styled.get("own_placement").is_none());
    assert_eq!(styled["own_style"], true);
    assert_eq!(entry(g.behind).get("style"), None);
}

#[test]
fn ac37_every_older_fixture_opens_unchanged_with_no_override() {
    for name in [
        "format_version_1.curvyo",
        "paths_v2.curvyo",
        "compound_v8.curvyo",
        "markers_v9.curvyo",
        "groups_v11.curvyo",
        "layers_v12.curvyo",
        "clone_v13.curvyo",
    ] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        let d = unpack(2, &bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        for clone in clones(&d) {
            assert!(
                !clone.chain.own_placement && !clone.chain.own_style,
                "{name}"
            );
            assert_eq!(clone.chain.style_source, clone.chain.root, "{name}");
        }
        let stored = LoroDoc::new();
        stored
            .import(&read_member(&bytes, "document.loro"))
            .unwrap();
        assert_eq!(
            loro_of(&d).oplog_vv(),
            stored.oplog_vv(),
            "{name}: not rewritten"
        );
    }
}

/// Review F1: the switches back leave exactly what the clone had before the switches (ADR 0016 §1).
#[test]
fn ac14_switching_both_back_leaves_only_the_tag_the_link_and_the_registers() {
    let d = Document::new(1);
    let root = square(&d, 1, Point::new(0.0, 0.0));
    d.set_node_opacity(&[root], Opacity::new(0.6).unwrap())
        .unwrap();
    let clone = clone_at(&d, root, 30.0, 0.0);
    d.set_node_locked(&[clone], true).unwrap();
    let keys = |d: &Document| {
        let loro = loro_of(d);
        let meta = loro.get_tree("paths").get_meta(tree_id(clone)).unwrap();
        let mut keys: Vec<String> = meta.keys().map(|k| k.to_string()).collect();
        keys.sort();
        keys
    };
    let before = keys(&d);
    assert_eq!(before, ["clone_link", "locked", "shape"]);
    d.set_own_placement(&[clone], true).unwrap();
    d.set_own_style(&[clone], true).unwrap();
    assert!(keys(&d).contains(&"opacity".to_owned()));
    assert!(keys(&d).contains(&"stroke_width".to_owned()));
    d.set_own_style(&[clone], false).unwrap();
    d.set_own_placement(&[clone], false).unwrap();
    assert_eq!(keys(&d), before);
}

#[test]
fn ac36_an_orphan_with_overrides_is_removed_like_any_orphan() {
    let (bytes, _, clone) = file_with_clone(|meta| {
        meta.insert("own_style", true).unwrap();
        meta.insert("own_placement", true).unwrap();
        meta.insert(
            "clone_link",
            LoroValue::from(std::collections::HashMap::from([
                ("origin".to_owned(), LoroValue::from("999:999")),
                (
                    "matrix".to_owned(),
                    LoroValue::from(vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
                ),
            ])),
        )
        .unwrap();
    });
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.read_objects().orphans, vec![clone]);
    assert_eq!(d.remove_orphan_clones(), 1);
    assert!(d.object(clone).is_none());
}
