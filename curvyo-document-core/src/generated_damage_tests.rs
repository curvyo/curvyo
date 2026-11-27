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
        part_paints: std::collections::BTreeMap::new(),
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
        part_paints: std::collections::BTreeMap::new(),
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
            part_paints: std::collections::BTreeMap::new(),
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

const PAINT_DAMAGED: &str = "generated_part_paint_damaged_v70.curvyo";

fn painted() -> GeneratedRecord {
    let mut part_paints = std::collections::BTreeMap::new();
    part_paints.insert(
        "lid".to_owned(),
        crate::PartPaint {
            fill: crate::Color { r: 1, g: 2, b: 3 },
            job: false,
        },
    );
    GeneratedRecord {
        kind: "curvyo.codes/qr".into(),
        seed: 3,
        params: serde_json::Map::new(),
        parts: vec![vec![square()], vec![square()]],
        part_keys: vec!["strip".into(), "lid".into()],
        part_paints,
    }
}

/// A record that paints a key no part has: refused at open as damaged (ADR 0091 P2).
fn paint_damaged_bytes() -> Vec<u8> {
    let document = Document::new(1);
    let id = document
        .create_generated(&painted(), Affine::IDENTITY)
        .unwrap();
    let tree = document.loro().get_tree(OBJECTS_TREE);
    let meta = tree.get_meta(tree_id_of(id)).unwrap();
    let mut broken = painted();
    let paint = broken.part_paints.remove("lid").unwrap();
    broken.part_paints.insert("ghost".to_owned(), paint);
    crate::generated_codec::write_record(&meta, &broken);
    document.loro().commit();
    pack(&document, "0.1.0").unwrap()
}

#[test]
fn a_record_that_paints_a_key_with_no_part_is_refused_as_damaged() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture(PAINT_DAMAGED), paint_damaged_bytes()).unwrap();
    }
    let bytes = std::fs::read(fixture(PAINT_DAMAGED)).unwrap();
    assert!(matches!(unpack(2, &bytes), Err(OpenError::Damaged)));
}

#[test]
fn a_part_paints_register_of_the_wrong_shape_is_damage_and_a_good_one_reads_back() {
    use loro::LoroValue as V;
    let entry = |fill: V, job: Option<V>| {
        let mut map = std::collections::HashMap::new();
        map.insert("fill".to_owned(), fill);
        if let Some(job) = job {
            map.insert("job".to_owned(), job);
        }
        V::Map(map.into())
    };
    let rgb = |r: i64, g: i64, b: i64| V::List(vec![V::I64(r), V::I64(g), V::I64(b)].into());
    let paints = |name: &str, value: V| {
        let mut map = std::collections::HashMap::new();
        map.insert(name.to_owned(), value);
        V::Map(map.into())
    };
    let cases = [
        (V::I64(3), false),
        (paints("lid", V::I64(3)), false),
        (
            paints("lid", entry(V::List(vec![V::I64(1)].into()), None)),
            false,
        ),
        (paints("lid", entry(rgb(0, 0, 256), None)), false),
        (paints("lid", entry(rgb(0, -1, 5), None)), false),
        (paints("lid", entry(rgb(0, 0, 5), Some(V::I64(0)))), false),
        (paints("ghost", entry(rgb(0, 0, 5), None)), false),
        (paints("lid", entry(rgb(0, 0, 5), None)), true),
        (
            paints("lid", entry(rgb(0, 0, 5), Some(V::Bool(false)))),
            true,
        ),
    ];
    for (value, good) in cases {
        let doc = loro::LoroDoc::new();
        let meta = doc.get_map("meta");
        let record = GeneratedRecord {
            part_paints: std::collections::BTreeMap::new(),
            ..painted()
        };
        crate::generated_codec::write_generated(&meta, &record, Affine::IDENTITY);
        let mut map = match meta.get("generated").unwrap().get_deep_value() {
            V::Map(map) => (*map).clone(),
            _ => panic!("a map"),
        };
        map.insert("part_paints".to_owned(), value.clone());
        meta.insert("generated", V::Map(map.into())).unwrap();
        assert_eq!(
            crate::generated_codec::validate_generated_node(&meta),
            good,
            "{value:?}"
        );
    }
    // Paints without part keys are damage: there is nothing to key them by.
    let doc = loro::LoroDoc::new();
    let meta = doc.get_map("meta");
    let keyless = GeneratedRecord {
        part_keys: Vec::new(),
        ..painted()
    };
    crate::generated_codec::write_generated(&meta, &keyless, Affine::IDENTITY);
    assert!(!crate::generated_codec::validate_generated_node(&meta));
    // A good record round-trips, `job` written only when false.
    let doc = loro::LoroDoc::new();
    let meta = doc.get_map("meta");
    crate::generated_codec::write_generated(&meta, &painted(), Affine::IDENTITY);
    let read = crate::generated_codec::read_record(&meta).unwrap();
    assert_eq!(read, painted());
}
