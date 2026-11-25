//! Acceptance tests of `specs/0147-rounded-box/specification.md`, slice 2 (criteria 11, 21 and the
//! panel side of 20): the Top shape choice hides and keeps rows, one undo step, the clamp notice.
//! Black-box through the instance mechanism (`curvyo_ui_core::instance`), as `generated_instance.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{Document, Length, NodeId, Point};
use curvyo_extension_core::ParamValue;
use curvyo_ui_core::ExtensionSettings;
use curvyo_ui_core::instance::{self};

const ROUNDED: &str = "curvyo.generators/rounded-box";

fn placed() -> (Document, NodeId, ExtensionSettings) {
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let id = instance::place(&document, &settings, ROUNDED, Point::new(50.0, 40.0)).unwrap();
    (document, id, settings)
}

fn mm(value: f64) -> ParamValue {
    ParamValue::Length(Length::from_mm(value))
}

fn labels(document: &Document, settings: &ExtensionSettings, id: NodeId) -> Vec<String> {
    instance::view(settings, &document.generated(id).unwrap())
        .rows
        .iter()
        .map(|row| row.param.label.to_owned())
        .collect()
}

fn stored(document: &Document, id: NodeId, key: &str) -> ParamValue {
    let snapshot = document.generated(id).unwrap();
    ParamValue::from_stored(&snapshot.record.params[key]).unwrap()
}

const CURVED_HIDDEN: [&str; 8] = [
    "Lid",
    "Lid seat",
    "Ring thickness",
    "Lid thickness",
    "Wall slits",
    "Base cover",
    "Cover thickness",
    "Base inset",
];

const CURVED_SHOWN: [&str; 8] = [
    "Top shape",
    "Corner radius",
    "Wall thickness",
    "End plate thickness",
    "Front thickness",
    "Inner base thickness",
    "Finger hollow",
    "Slit pattern",
];

#[test]
fn criterion_11_top_shape_change_is_one_undo_step_named_rounded_box_top_shape() {
    let (document, id, settings) = placed();
    let before = document.own_steps().len();
    instance::change_value(
        &document,
        &settings,
        id,
        "top_shape",
        &ParamValue::Choice("curved".into()),
    )
    .unwrap();
    assert_eq!(document.own_steps().len(), before + 1, "one undo step");
    // The commit label is the machine id, as for every generator step; the History row reads
    // "Rounded box: Top shape" from the step's kind and parameter fields (the editor-wasm test
    // `the_top_shape_step_of_a_rounded_box_is_named_after_the_kind_and_the_field`).
    assert_eq!(
        document.own_steps().last().unwrap().label,
        "change_parameter"
    );
}

#[test]
fn criterion_21_curved_top_hides_its_rows_and_shows_the_joint_rows() {
    let (document, id, settings) = placed();
    instance::change_value(
        &document,
        &settings,
        id,
        "top_shape",
        &ParamValue::Choice("curved".into()),
    )
    .unwrap();
    let rows = labels(&document, &settings, id);
    for hidden in CURVED_HIDDEN {
        assert!(
            !rows.iter().any(|l| l == hidden),
            "{hidden} is not offered in Curved"
        );
    }
    for shown in CURVED_SHOWN {
        assert!(
            rows.iter().any(|l| l == shown),
            "{shown} is offered in Curved"
        );
    }
}

#[test]
fn criterion_21_rows_hidden_in_curved_keep_their_values_on_flat() {
    let (document, id, settings) = placed();
    instance::change_value(&document, &settings, id, "lid", &ParamValue::Switch(false)).unwrap();
    instance::change_value(&document, &settings, id, "base_inset", &mm(4.0)).unwrap();
    instance::change_value(
        &document,
        &settings,
        id,
        "top_shape",
        &ParamValue::Choice("curved".into()),
    )
    .unwrap();
    assert_eq!(stored(&document, id, "lid"), ParamValue::Switch(false));
    instance::change_value(
        &document,
        &settings,
        id,
        "top_shape",
        &ParamValue::Choice("flat".into()),
    )
    .unwrap();
    assert_eq!(
        stored(&document, id, "lid"),
        ParamValue::Switch(false),
        "kept through Curved"
    );
    assert_eq!(
        stored(&document, id, "base_inset"),
        mm(4.0),
        "kept through Curved"
    );
    let rows = labels(&document, &settings, id);
    assert!(
        rows.iter().any(|l| l == "Lid"),
        "Lid is offered again on Flat"
    );
    assert!(
        !rows.iter().any(|l| l == "End plate thickness"),
        "the curved-only rows hide on Flat"
    );
}

#[test]
fn criterion_20_corner_radius_clamp_names_the_field_and_the_most_the_box_allows() {
    let (document, id, settings) = placed();
    for (key, value) in [
        ("inner_width", mm(120.0)),
        ("inner_depth", mm(80.0)),
        ("inner_height", mm(50.0)),
        ("thickness", mm(3.0)),
    ] {
        instance::change_value(&document, &settings, id, key, &value).unwrap();
    }
    instance::change_value(
        &document,
        &settings,
        id,
        "top_shape",
        &ParamValue::Choice("curved".into()),
    )
    .unwrap();
    let changed =
        instance::change_value(&document, &settings, id, "corner_radius", &mm(40.0)).unwrap();
    assert!(
        changed
            .notices
            .iter()
            // spec revision 9: "this rounded box", as slice 1
            .any(|n| n == "Corner radius set to 32 mm, the most this rounded box allows."),
        "notices: {:?}",
        changed.notices
    );
}
