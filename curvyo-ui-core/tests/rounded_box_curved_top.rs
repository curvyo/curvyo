//! The curved top in the shell (`specs/0147-rounded-box` slice 2, ADR 0082 §11): the rows the
//! Generator block shows per Top shape and the overlay marks of a curved box.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{Affine, GeneratedSnapshot, NodeId};
use curvyo_extension_core::Mark;
use curvyo_ui_core::ExtensionSettings;
use curvyo_ui_core::instance::{draws_marks, find_kind, marks_of, new_record, view};
use serde_json::json;

fn snapshot(top_shape: &str, extra: &[(&str, serde_json::Value)]) -> GeneratedSnapshot {
    let found = find_kind(
        &ExtensionSettings::default(),
        "curvyo.generators/rounded-box",
    )
    .unwrap();
    let (mut record, _) = new_record(&found).unwrap();
    record
        .params
        .insert("top_shape".into(), json!({ "choice": top_shape }));
    for (key, value) in extra {
        record.params.insert((*key).into(), value.clone());
    }
    GeneratedSnapshot {
        id: NodeId::NONE,
        record,
        frame: Affine::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
    }
}

fn rows(snapshot: &GeneratedSnapshot) -> Vec<&'static str> {
    view(&ExtensionSettings::default(), snapshot)
        .rows
        .iter()
        .map(|row| row.param.key)
        .collect()
}

#[test]
fn each_top_shape_shows_its_own_rows_and_the_path_box_keeps_all_of_its_own() {
    let flat = rows(&snapshot("flat", &[]));
    let curved = rows(&snapshot("curved", &[]));
    for key in [
        "lid",
        "lid_seat",
        "ring_thickness",
        "wall_slits",
        "base_cover",
        "base_inset",
    ] {
        assert!(flat.contains(&key) && !curved.contains(&key), "{key}");
    }
    for key in ["end_plate_thickness", "front_thickness"] {
        assert!(!flat.contains(&key) && curved.contains(&key), "{key}");
    }
    assert!(flat.contains(&"top_shape") && curved.contains(&"top_shape"));
    // Top shape sits right after Corner radius (ADR 0082 §11 key order).
    let at = |rows: &[&str], key: &str| rows.iter().position(|k| *k == key).unwrap();
    assert_eq!(at(&curved, "top_shape"), at(&curved, "corner_radius") + 1);
    // A curved box with the hollow off hides its two sizes.
    let off = rows(&snapshot(
        "curved",
        &[("finger_hollow", json!({ "on": false }))],
    ));
    assert!(off.contains(&"finger_hollow") && !off.contains(&"hollow_width"));
    // The path box still shows every row it did.
    let found = find_kind(&ExtensionSettings::default(), "curvyo.generators/path-box").unwrap();
    let (record, _) = new_record(&found).unwrap();
    let path_box = GeneratedSnapshot {
        id: NodeId::NONE,
        record,
        frame: Affine::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
    };
    let shown = rows(&path_box);
    for key in [
        "lid",
        "lid_seat",
        "ring_thickness",
        "base_cover",
        "cover_thickness",
        "finger_hollow",
    ] {
        assert!(shown.contains(&key), "the path box shows {key}");
    }
}

#[test]
fn a_curved_box_has_bend_bands_and_part_badges_and_a_flat_one_has_none() {
    let settings = ExtensionSettings::default();
    assert!(!draws_marks(&settings, &snapshot("flat", &[])));
    assert!(draws_marks(&settings, &snapshot("curved", &[])));
    let flat = marks_of(&settings, &snapshot("flat", &[])).unwrap();
    assert!(flat.on_footprint.is_empty() && flat.on_parts.is_empty());
    let curved = marks_of(&settings, &snapshot("curved", &[])).unwrap();
    let bends = curved
        .on_footprint
        .iter()
        .filter(|m| matches!(m, Mark::Bend { tight: false, .. }))
        .count();
    assert_eq!(bends, 2);
    assert_eq!(curved.on_parts.len(), 5);
}
