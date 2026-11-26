//! The damaged golden of format 54 (`generated_damaged_v54.curvyo`): a generated leaf whose
//! `generated` register breaks a stored-shape rule is refused at open as damaged (ADR 0080 §4).
//! `CURVYO_WRITE_FIXTURES=1` rewrites the golden.

use std::path::PathBuf;

use crate::affine::Affine;
use crate::container::{pack, unpack};
use crate::document::{Document, OBJECTS_TREE};
use crate::error::OpenError;
use crate::generated_codec::GeneratedRecord;
use crate::path_codec::leaf_exists;
use crate::paths::tree_id_of;

const DAMAGED: &str = "generated_damaged_v54.curvyo";

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn square() -> Vec<f64> {
    vec![
        0.0, 0.0, 0.0, 0.0, 4.0, 0.0, 4.0, 0.0, 4.0, 0.0, 4.0, 4.0, 4.0, 4.0, 4.0, 4.0, 0.0, 4.0,
        0.0, 4.0, 0.0, 4.0, 0.0, 0.0, 0.0, 0.0,
    ]
}

fn damaged_bytes() -> Vec<u8> {
    let document = Document::new(1);
    let record = GeneratedRecord {
        kind: "curvyo.generators/gear".into(),
        seed: 3,
        params: serde_json::Map::new(),
        parts: vec![vec![square()]],
        part_keys: Vec::new(),
    };
    let id = document
        .create_generated(&record, Affine::IDENTITY)
        .unwrap();
    let tree = document.loro().get_tree(OBJECTS_TREE);
    let tree_id = tree_id_of(id);
    assert!(leaf_exists(&tree, tree_id));
    let meta = tree.get_meta(tree_id).unwrap();
    // A subpath of seven numbers is not `2 + 6n`.
    let broken = GeneratedRecord {
        parts: vec![vec![vec![0.0; 7]]],
        ..record
    };
    crate::generated_codec::write_record(&meta, &broken);
    document.loro().commit();
    pack(&document, "0.1.0").unwrap()
}

#[test]
fn a_generated_leaf_that_breaks_the_stored_shape_is_refused_as_damaged() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture(DAMAGED), damaged_bytes()).unwrap();
    }
    let bytes = std::fs::read(fixture(DAMAGED)).unwrap();
    assert!(matches!(unpack(2, &bytes), Err(OpenError::Damaged)));
}

const KEYS_DAMAGED: &str = "generated_part_keys_damaged_v65.curvyo";

/// A record whose `part_keys` repeat a key: refused at open as damaged (ADR 0082 §10).
fn keys_damaged_bytes() -> Vec<u8> {
    let document = Document::new(1);
    let record = GeneratedRecord {
        kind: "curvyo.generators/rounded-box".into(),
        seed: 3,
        params: serde_json::Map::new(),
        parts: vec![vec![square()], vec![square()]],
        part_keys: vec!["strip".into(), "lid".into()],
    };
    let id = document
        .create_generated(&record, Affine::IDENTITY)
        .unwrap();
    let tree = document.loro().get_tree(OBJECTS_TREE);
    let meta = tree.get_meta(tree_id_of(id)).unwrap();
    let broken = GeneratedRecord {
        part_keys: vec!["lid".into(), "lid".into()],
        ..record
    };
    crate::generated_codec::write_record(&meta, &broken);
    document.loro().commit();
    pack(&document, "0.1.0").unwrap()
}

#[test]
fn a_record_whose_part_keys_repeat_a_key_is_refused_as_damaged() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture(KEYS_DAMAGED), keys_damaged_bytes()).unwrap();
    }
    let bytes = std::fs::read(fixture(KEYS_DAMAGED)).unwrap();
    assert!(matches!(unpack(2, &bytes), Err(OpenError::Damaged)));
}

#[test]
fn part_keys_of_the_wrong_length_or_type_are_damage_too() {
    for keys in [
        loro::LoroValue::List(vec![loro::LoroValue::String("strip".into())].into()),
        loro::LoroValue::List(
            vec![
                loro::LoroValue::String("strip".into()),
                loro::LoroValue::I64(1),
            ]
            .into(),
        ),
        loro::LoroValue::List(Vec::<loro::LoroValue>::new().into()),
        loro::LoroValue::I64(3),
    ] {
        let doc = loro::LoroDoc::new();
        let meta = doc.get_map("meta");
        let record = GeneratedRecord {
            kind: "curvyo.generators/rounded-box".into(),
            seed: 3,
            params: serde_json::Map::new(),
            parts: vec![vec![square()], vec![square()]],
            part_keys: vec!["strip".into(), "lid".into()],
        };
        crate::generated_codec::write_generated(&meta, &record, Affine::IDENTITY);
        assert!(crate::generated_codec::validate_generated_node(&meta));
        let mut map = match meta.get("generated").unwrap().get_deep_value() {
            loro::LoroValue::Map(map) => (*map).clone(),
            _ => panic!("a map"),
        };
        map.insert("part_keys".to_owned(), keys);
        meta.insert("generated", loro::LoroValue::Map(map.into()))
            .unwrap();
        assert!(!crate::generated_codec::validate_generated_node(&meta));
    }
}
