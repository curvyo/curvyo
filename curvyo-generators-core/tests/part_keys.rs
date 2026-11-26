//! Part keys (`specs/0147-rounded-box` criterion 15, ADR 0082 §10): every part of a result has a
//! key, unique within the result and the same on every regeneration, so an operation can name it.

#![allow(clippy::unwrap_used)]

use curvyo_document_core::Length;
use curvyo_extension_core::{
    Edit, Extension, GenerateRequest, Generated, ParamValue, Seed, StoredParams,
};
use curvyo_generators_core::{BOX, GEAR, GENERATORS, ROUNDED_BOX};

fn generate(kind: &str, entries: &[(&str, ParamValue)]) -> Generated {
    let params: StoredParams = entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.to_stored()))
        .collect();
    GENERATORS
        .generate(&GenerateRequest {
            kind,
            params: &params,
            seed: Seed(1),
            edit: None::<Edit<'_>>,
        })
        .unwrap()
}

fn keys(generated: &Generated) -> Vec<String> {
    generated.parts.iter().map(|p| p.key.to_string()).collect()
}

fn choice(id: &str) -> ParamValue {
    ParamValue::Choice(id.to_owned())
}

#[test]
fn a_gear_has_one_key_and_a_box_one_per_plate() {
    assert_eq!(keys(&generate(GEAR, &[])), ["gear"]);
    let boxed = keys(&generate(BOX, &[]));
    assert_eq!(boxed, ["bottom", "front", "back", "left", "right"]);
}

#[test]
fn the_flat_rounded_box_names_its_parts_by_role() {
    let all = keys(&generate(ROUNDED_BOX, &[]));
    assert!(all.contains(&"strip".to_owned()), "{all:?}");
    assert!(all.contains(&"base".to_owned()), "{all:?}");
    assert!(all.len() >= 3, "{all:?}");
}

#[test]
fn the_curved_rounded_box_has_the_five_keys_of_adr_0082_section_11() {
    let all = keys(&generate(ROUNDED_BOX, &[("top_shape", choice("curved"))]));
    assert_eq!(all, ["strip", "left", "right", "front", "base"]);
}

#[test]
fn keys_are_unique_and_stable_across_parameter_changes() {
    let wider = [("inner_width", ParamValue::Length(Length::from_mm(150.0)))];
    for kind in [GEAR, BOX, ROUNDED_BOX] {
        let a = keys(&generate(kind, &[]));
        let b = keys(&generate(kind, if kind == GEAR { &[] } else { &wider }));
        let mut sorted = a.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), a.len(), "{kind}: duplicate key in {a:?}");
        assert_eq!(a, b, "{kind}: keys changed with a size");
    }
}
