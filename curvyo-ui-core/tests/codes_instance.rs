//! The QR code through the generated-object arm of the instance mechanism (`specs/0144`, ADR
//! 0091 §3): place, the refusal that keeps the last geometry and stores the typed text, the level
//! edits, the clamp sentences, a resize that writes the module size, Convert to paths.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{Affine, Document, Length, NodeId, ObjectSnapshot, Point};
use curvyo_extension_core::ParamValue;
use curvyo_ui_core::instance::{self, InstanceView};
use curvyo_ui_core::{AnchorIdMinter, ExtensionSettings};

const QR: &str = "curvyo.codes/qr";

fn placed() -> (Document, NodeId, ExtensionSettings) {
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let id = instance::place(&document, &settings, QR, Point::new(50.0, 40.0)).unwrap();
    (document, id, settings)
}

fn text(content: &str) -> ParamValue {
    ParamValue::Text(content.to_owned())
}

fn choice(id: &str) -> ParamValue {
    ParamValue::Choice(id.to_owned())
}

fn view(document: &Document, settings: &ExtensionSettings, id: NodeId) -> InstanceView {
    instance::view(settings, &document.generated(id).unwrap())
}

fn problem(document: &Document, settings: &ExtensionSettings, id: NodeId) -> Option<String> {
    view(document, settings, id)
        .rows
        .into_iter()
        .find_map(|row| row.problem)
}

#[test]
fn placing_writes_one_step_with_the_default_content_and_one_part() {
    let (document, id, settings) = placed();
    let snapshot = document.generated(id).unwrap();
    assert_eq!(document.own_step_count(), 1);
    assert_eq!(snapshot.record.part_keys, ["code"]);
    assert_eq!(
        snapshot.record.params["content"]["text"],
        "https://example.com"
    );
    assert_eq!(snapshot.frame, Affine::new(1.0, 0.0, 0.0, 1.0, 50.0, 40.0));
    assert_eq!(problem(&document, &settings, id), None);
    let json = serde_json::to_value(view(&document, &settings, id)).unwrap();
    assert_eq!(json["label"], "QR code");
}

#[test]
fn a_text_that_cannot_be_drawn_is_stored_and_the_last_geometry_is_kept() {
    let (document, id, settings) = placed();
    let before = document.generated(id).unwrap();
    let changed = instance::change_value(&document, &settings, id, "content", &text("")).unwrap();
    assert!(changed.written, "the commit is stored");
    assert!(
        changed.notices.is_empty(),
        "the reason is a line, not a notice"
    );
    let after = document.generated(id).unwrap();
    assert_eq!(after.record.parts, before.record.parts, "geometry kept");
    assert_eq!(after.record.part_keys, before.record.part_keys);
    assert_eq!(after.record.params["content"]["text"], "");
    assert_eq!(
        problem(&document, &settings, id).as_deref(),
        Some("Enter a text to encode.")
    );
    let long = "a".repeat(2332);
    instance::change_value(&document, &settings, id, "content", &text(&long)).unwrap();
    assert_eq!(
        problem(&document, &settings, id).as_deref(),
        Some("Too long for a QR code at error correction M: 2332 bytes, at most 2331.")
    );
    assert_eq!(
        document.generated(id).unwrap().record.parts,
        before.record.parts
    );
    // Lowering the level makes the same text fit without typing it again.
    instance::change_value(&document, &settings, id, "ec", &choice("l")).unwrap();
    let fitted = document.generated(id).unwrap();
    assert_ne!(fitted.record.parts, before.record.parts);
    assert_eq!(fitted.record.params["content"]["text"], long.as_str());
    assert_eq!(problem(&document, &settings, id), None);
    // A text back that fits restores a real code.
    instance::change_value(&document, &settings, id, "content", &text("Hello")).unwrap();
    assert_eq!(problem(&document, &settings, id), None);
}

#[test]
fn a_level_the_content_does_not_fit_is_lowered_with_the_house_sentence() {
    let (document, id, settings) = placed();
    instance::change_value(&document, &settings, id, "ec", &choice("l")).unwrap();
    instance::change_value(
        &document,
        &settings,
        id,
        "content",
        &text(&"a".repeat(2500)),
    )
    .unwrap();
    let changed = instance::change_value(&document, &settings, id, "ec", &choice("h")).unwrap();
    assert_eq!(
        changed.notices,
        ["Error correction set to L, the most this content allows."]
    );
    assert_eq!(
        document.generated(id).unwrap().record.params["ec"]["choice"],
        "l"
    );
}

#[test]
fn the_clear_area_raises_the_level_and_a_lower_level_limits_it_with_the_house_sentence() {
    let (document, id, settings) = placed();
    instance::change_value(
        &document,
        &settings,
        id,
        "logo_size",
        &ParamValue::Percent(35),
    )
    .unwrap();
    instance::change_value(&document, &settings, id, "logo", &ParamValue::Switch(true)).unwrap();
    let record = document.generated(id).unwrap().record;
    assert_eq!(
        record.params["ec"]["choice"], "h",
        "raised in the same step"
    );
    let changed = instance::change_value(&document, &settings, id, "ec", &choice("l")).unwrap();
    let sentence = changed
        .notices
        .iter()
        .find(|n| n.starts_with("Logo size set to"))
        .expect("a knock-on sentence");
    assert!(
        sentence.ends_with(" %, the most error correction L allows."),
        "{sentence}"
    );
}

#[test]
fn a_resize_writes_the_module_size_and_the_width_follows() {
    let (document, id, settings) = placed();
    let ObjectSnapshot::Path(path) = document.object(id).unwrap() else {
        panic!("a path");
    };
    let result = ObjectSnapshot::Path(path.mapped(Affine::scale(2.0, 2.0)));
    instance::resize(&document, &settings, &result, "transform_resize").unwrap();
    let after = document.generated(id).unwrap();
    assert_eq!(after.record.params["module_size"]["mm"], 2.0);
    assert_eq!(
        after.record.params["width"]["mm"], 66.0,
        "25 modules + 8 quiet, 2 mm each"
    );
    assert!(
        (after.frame.a - 1.0).abs() < 1e-12,
        "a frame never holds a scale"
    );
}

#[test]
fn a_resize_of_a_code_that_cannot_be_drawn_is_refused() {
    let (document, id, settings) = placed();
    instance::change_value(&document, &settings, id, "content", &text("")).unwrap();
    let ObjectSnapshot::Path(path) = document.object(id).unwrap() else {
        panic!("a path");
    };
    let result = ObjectSnapshot::Path(path.mapped(Affine::scale(2.0, 2.0)));
    assert!(instance::resize(&document, &settings, &result, "transform_resize").is_err());
}

#[test]
fn convert_to_paths_gives_one_compound_path_for_merged_and_separate_alike() {
    for separate in [false, true] {
        let (document, id, settings) = placed();
        if separate {
            instance::change_value(&document, &settings, id, "module_shape", &choice("dot"))
                .unwrap();
        }
        let mut minter = AnchorIdMinter::new(1);
        instance::convert(&document, &mut minter, id).unwrap();
        assert!(document.generated(id).is_none());
        let ObjectSnapshot::Path(path) = document.object(id).unwrap() else {
            panic!("a path");
        };
        assert!(path.is_compound(), "separate = {separate}");
        assert_eq!(document.read_objects().objects.len(), 1);
    }
}

#[test]
fn a_text_and_a_percent_come_from_json_and_numbers() {
    let (document, id, settings) = placed();
    let rows = view(&document, &settings, id).rows;
    let find = |key: &str| {
        rows.iter()
            .find(|row| row.param.key == key)
            .unwrap()
            .param
            .clone()
    };
    assert_eq!(
        instance::value_from_json(&find("content"), &serde_json::json!("two\nlines")),
        Some(text("two\nlines"))
    );
    assert_eq!(
        instance::value_from_number(&find("module_fill"), 74.6),
        Some(ParamValue::Percent(75))
    );
    assert_eq!(instance::value_from_number(&find("content"), 1.0), None);
    let _ = Length::from_mm(0.0);
}

#[test]
fn the_output_row_is_hidden_below_a_full_module_fill() {
    let (document, id, settings) = placed();
    let keys = |view: InstanceView| {
        view.rows
            .into_iter()
            .map(|row| row.param.key)
            .collect::<Vec<_>>()
    };
    assert!(keys(view(&document, &settings, id)).contains(&"output"));
    instance::change_value(
        &document,
        &settings,
        id,
        "module_fill",
        &ParamValue::Percent(80),
    )
    .unwrap();
    assert!(!keys(view(&document, &settings, id)).contains(&"output"));
}

#[test]
fn the_kind_seed_and_params_are_readable_without_the_outline() {
    let (document, id, _) = placed();
    let found = document.generated_params(id).unwrap();
    let (kind, seed, params) = (found.kind, found.seed, found.params);
    let record = document.generated(id).unwrap().record;
    assert_eq!((kind.as_str(), seed), (QR, record.seed));
    assert_eq!(params, record.params);
    let mut minter = AnchorIdMinter::new(1);
    instance::convert(&document, &mut minter, id).unwrap();
    assert!(
        document.generated_params(id).is_none(),
        "a converted path is no generated object"
    );
}
