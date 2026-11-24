//! The generated-object arm of the instance mechanism (`specs/0141-parametric-generators`,
//! criteria 1 to 7, ADR 0080 §6): place, change by number with clamps and their notices, a
//! resize that writes parameters, Convert to paths, an unknown kind, the stored outline equal to
//! a regeneration from the stored numbers.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    Affine, Document, GeneratedRecord, Length, NodeId, ObjectSnapshot, Point, pack, unpack,
};
use curvyo_extension_core::{Degrees, ParamValue};
use curvyo_ui_core::instance::{self, InstanceError};
use curvyo_ui_core::{AnchorIdMinter, ExtensionSettings};

const GEAR: &str = "curvyo.generators/gear";
const BOX: &str = "curvyo.generators/box";

fn mm(value: f64) -> ParamValue {
    ParamValue::Length(Length::from_mm(value))
}

fn placed(kind: &str) -> (Document, NodeId, ExtensionSettings) {
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let id = instance::place(&document, &settings, kind, Point::new(50.0, 40.0)).unwrap();
    (document, id, settings)
}

fn value(record: &GeneratedRecord, key: &str) -> ParamValue {
    ParamValue::from_stored(&record.params[key]).unwrap()
}

#[test]
fn placing_writes_one_step_with_the_defaults_and_a_seed_from_the_node_id() {
    let (document, id, _) = placed(GEAR);
    let snapshot = document.generated(id).unwrap();
    assert_eq!(document.own_step_count(), 1);
    assert_eq!(snapshot.record.kind, GEAR);
    assert_eq!(value(&snapshot.record, "teeth"), ParamValue::Count(24));
    assert_eq!(snapshot.frame, Affine::new(1.0, 0.0, 0.0, 1.0, 50.0, 40.0));
    assert_eq!(snapshot.record.parts.len(), 1);
    assert!(
        instance::place(
            &document,
            &ExtensionSettings::default(),
            "future/spiral",
            Point::new(0.0, 0.0)
        )
        .is_err()
    );
}

#[test]
fn a_parameter_change_rebuilds_the_outline_in_one_step_and_an_equal_value_writes_nothing() {
    let (document, id, settings) = placed(GEAR);
    let before = document.generated(id).unwrap();
    let changed =
        instance::change_value(&document, &settings, id, "teeth", &ParamValue::Count(40)).unwrap();
    assert!(changed.written && changed.notices.is_empty());
    let after = document.generated(id).unwrap();
    assert_eq!(value(&after.record, "teeth"), ParamValue::Count(40));
    assert_eq!(value(&after.record, "pitch_diameter"), mm(80.0));
    assert_ne!(after.record.parts, before.record.parts);
    assert_eq!(after.frame, before.frame, "the object stays where it is");
    assert_eq!(document.own_step_count(), 2);
    // Criterion 7: the stored outline is what the generator builds from the stored numbers.
    let again =
        instance::change_value(&document, &settings, id, "teeth", &ParamValue::Count(40)).unwrap();
    assert!(!again.written);
    assert_eq!(document.own_step_count(), 2);
}

#[test]
fn typing_the_pitch_diameter_changes_the_module() {
    let (document, id, settings) = placed(GEAR);
    instance::change_value(&document, &settings, id, "pitch_diameter", &mm(50.0)).unwrap();
    let record = document.generated(id).unwrap().record;
    assert_eq!(value(&record, "pitch_diameter"), mm(50.0));
    let ParamValue::Length(module) = value(&record, "module") else {
        panic!("a length");
    };
    assert!((module.as_mm() - 50.0 / 24.0).abs() < 1e-9);
}

#[test]
fn a_value_out_of_bounds_is_applied_clamped_with_a_notice_that_names_field_and_limit() {
    let (document, id, settings) = placed(GEAR);
    let changed =
        instance::change_value(&document, &settings, id, "teeth", &ParamValue::Count(3)).unwrap();
    // The knock-on clamp of the 6 mm bore, which the smaller gear no longer holds, is named too.
    assert_eq!(
        changed.notices,
        vec![
            "Teeth set to 6, the minimum.".to_owned(),
            "Bore set to 5 mm, the most this gear allows.".to_owned()
        ]
    );
    assert_eq!(
        value(&document.generated(id).unwrap().record, "teeth"),
        ParamValue::Count(6)
    );
    // A bore larger than the root circle allows (module 2, 6 teeth: root radius 3.5).
    let changed = instance::change_value(&document, &settings, id, "bore", &mm(100.0)).unwrap();
    assert_eq!(
        changed.notices,
        vec!["Bore set to 5 mm, the most this gear allows.".to_owned()]
    );
    assert!(!changed.written, "it was 5 mm already");
    let (document, id, settings) = placed(BOX);
    let changed = instance::change_value(&document, &settings, id, "thickness", &mm(0.0)).unwrap();
    assert_eq!(
        changed.notices,
        vec!["Thickness set to 0.1 mm, the minimum.".to_owned()]
    );
    // A wrong kind of value is refused.
    assert_eq!(
        instance::change_value(&document, &settings, id, "thickness", &ParamValue::Count(2)),
        Err(InstanceError::WrongValue)
    );
    let _ = Degrees(0.0);
}

#[test]
fn the_block_hides_rows_that_do_not_apply_and_keeps_their_values() {
    let (document, id, settings) = placed(GEAR);
    let labels = |document: &Document| -> Vec<String> {
        instance::view(&settings, &document.generated(id).unwrap())
            .rows
            .iter()
            .map(|row| row.param.label.to_owned())
            .collect()
    };
    let solid = labels(&document);
    assert!(solid.contains(&"Keyway".to_owned()));
    assert!(!solid.contains(&"Keyway width".to_owned()));
    assert!(!solid.contains(&"Spokes".to_owned()));
    instance::change_value(
        &document,
        &settings,
        id,
        "keyway",
        &ParamValue::Switch(true),
    )
    .unwrap();
    instance::change_value(
        &document,
        &settings,
        id,
        "web",
        &ParamValue::Choice("spokes".into()),
    )
    .unwrap();
    let open = labels(&document);
    for shown in [
        "Keyway width",
        "Keyway depth",
        "Spokes",
        "Rim width",
        "Hub diameter",
    ] {
        assert!(open.contains(&shown.to_owned()), "{shown}");
    }
    instance::change_value(&document, &settings, id, "bore", &mm(0.0)).unwrap();
    assert!(!labels(&document).contains(&"Keyway".to_owned()));
}

#[test]
fn a_resize_writes_parameters_not_a_hidden_transform() {
    let (document, id, settings) = placed(GEAR);
    let ObjectSnapshot::Path(path) = document.object(id).unwrap() else {
        panic!("path");
    };
    let result = ObjectSnapshot::Path(path.mapped(Affine::new(2.0, 0.0, 0.0, 2.0, -50.0, -40.0)));
    instance::resize(&document, &settings, &result, "transform_resize").unwrap();
    let after = document.generated(id).unwrap();
    // 0141 revision 2: a gear resize writes Teeth (m (z + 2) = 52 doubles to 104), nothing else.
    assert_eq!(value(&after.record, "module"), mm(2.0));
    assert_eq!(value(&after.record, "teeth"), ParamValue::Count(50));
    assert_eq!(value(&after.record, "bore"), mm(6.0));
    // The object's centre followed the map: (50, 40) about the origin of the scale.
    assert!((after.frame.e - 50.0).abs() < 1e-9 && (after.frame.f - 40.0).abs() < 1e-9);
    assert!(
        (after.frame.a - 1.0).abs() < 1e-12,
        "a frame never holds a scale"
    );
    // The box: near, not exactly, the dragged size (the thickness stays).
    let (document, id, settings) = placed(BOX);
    let ObjectSnapshot::Path(path) = document.object(id).unwrap() else {
        panic!("path");
    };
    let result = ObjectSnapshot::Path(path.mapped(Affine::scale(0.5, 0.5)));
    instance::resize(&document, &settings, &result, "transform_resize").unwrap();
    let record = document.generated(id).unwrap().record;
    assert_eq!(value(&record, "inner_width"), mm(50.0));
    assert_eq!(value(&record, "thickness"), mm(3.0));
}

#[test]
fn convert_gives_one_compound_path_for_a_gear_and_a_group_of_plates_for_a_box() {
    let (document, id, _) = placed(GEAR);
    let mut minter = AnchorIdMinter::new(1);
    instance::convert(&document, &mut minter, id).unwrap();
    assert!(document.generated(id).is_none());
    let ObjectSnapshot::Path(path) = document.object(id).unwrap() else {
        panic!("a path");
    };
    assert!(path.is_compound(), "outline and bore");
    let (document, id, _) = placed(BOX);
    let mut minter = AnchorIdMinter::new(1);
    instance::convert(&document, &mut minter, id).unwrap();
    assert!(document.generated(id).is_none());
    assert_eq!(document.read_objects().objects.len(), 5, "five plates");
}

#[test]
fn an_unknown_or_disabled_kind_is_kept_drawn_and_converted_but_not_changed_or_resized() {
    let (document, id, settings) = placed(GEAR);
    // Disabled: the generators extension is switched off.
    let mut off = ExtensionSettings::default();
    off.set_enabled("curvyo.generators", false);
    assert_eq!(
        instance::change_value(&document, &off, id, "teeth", &ParamValue::Count(30)),
        Err(InstanceError::UnknownKind)
    );
    let view = instance::view(&off, &document.generated(id).unwrap());
    assert!(!view.known && view.rows.is_empty());
    let ObjectSnapshot::Path(path) = document.object(id).unwrap() else {
        panic!("a path");
    };
    let result = ObjectSnapshot::Path(path.mapped(Affine::scale(2.0, 2.0)));
    assert_eq!(
        instance::resize(&document, &off, &result, "transform_resize"),
        Err(InstanceError::UnknownKind)
    );
    // The record survives a save and reopen as stored, and converts.
    let reopened = unpack(2, &pack(&document, "0.1.0").unwrap()).unwrap();
    assert_eq!(
        reopened.generated(id).unwrap().record,
        document.generated(id).unwrap().record
    );
    let mut minter = AnchorIdMinter::new(2);
    instance::convert(&reopened, &mut minter, id).unwrap();
    assert!(reopened.generated(id).is_none());
    let _ = settings;
}

/// Criterion 7: the file opens as stored, and regenerating from the stored numbers gives the same
/// outline: setting any stored value again rebuilds the record equal to the stored one (nothing is
/// written), for both generators, after a save and a reopen.
#[test]
fn regenerating_from_the_stored_numbers_gives_the_stored_outline_after_reopen() {
    for kind in [GEAR, BOX] {
        let (document, _, settings) = placed(kind);
        let reopened = unpack(2, &pack(&document, "0.1.0").unwrap()).unwrap();
        let id = reopened.read_objects().objects.first().unwrap().id();
        let stored = reopened.generated(id).unwrap();
        let steps = reopened.own_step_count();
        assert!(!stored.record.params.is_empty());
        for (key, raw) in &stored.record.params {
            let same = ParamValue::from_stored(raw).unwrap();
            let changed = instance::change_value(&reopened, &settings, id, key, &same).unwrap();
            assert!(
                !changed.written,
                "{kind}: {key} rebuilds to the stored outline"
            );
        }
        assert_eq!(
            reopened.own_step_count(),
            steps,
            "{kind}: nothing was written"
        );
        assert_eq!(reopened.generated(id).unwrap(), stored, "{kind}");
    }
}

#[test]
fn a_converted_box_keeps_the_operation_assigned_to_it() {
    use curvyo_document_core::{DocumentSize, NewOperation, NewOutputPage};
    let (document, id, _) = placed(BOX);
    let page = document
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let manufacturing = page.kind.manufacturing().unwrap();
    let operation = document
        .add_operation(
            manufacturing,
            &NewOperation {
                kind: "cut",
                name: "Cut 1",
            },
            0,
            &[vec![id]],
        )
        .unwrap();
    let members = |document: &Document| {
        document
            .operations(manufacturing)
            .unwrap()
            .into_iter()
            .find(|o| o.id == operation)
            .unwrap()
            .members
    };
    assert_eq!(members(&document), vec![vec![id]]);
    let mut minter = AnchorIdMinter::new(1);
    instance::convert(&document, &mut minter, id).unwrap();
    assert_eq!(
        members(&document),
        vec![vec![id]],
        "the group of plates is the member the box was"
    );
    assert_eq!(
        document.leaves_of(id).len(),
        5,
        "the five plates sit under it"
    );
}

const ROUNDED: &str = "curvyo.generators/rounded-box";

#[test]
fn a_rounded_box_block_shows_the_values_the_generator_used_for_automatic_rows() {
    let (document, id, settings) = placed(ROUNDED);
    let view = instance::view(&settings, &document.generated(id).unwrap());
    let row = |key: &str| view.rows.iter().find(|row| row.param.key == key).unwrap();
    // Stored 0 is automatic: the row keeps 0 and carries what the generator used.
    assert_eq!(row("slit_width").value, mm(0.0));
    assert_eq!(row("slit_width").used, Some(mm(0.3)));
    assert_eq!(
        row("lid_thickness").used,
        Some(mm(3.0)),
        "follows the wall thickness"
    );
    assert_eq!(row("thickness").used, None, "a plain row has none");
    // Typing a value makes it the maker's own: nothing is reported for it.
    instance::change_value(&document, &settings, id, "slit_width", &mm(0.5)).unwrap();
    let view = instance::view(&settings, &document.generated(id).unwrap());
    let own = view
        .rows
        .iter()
        .find(|row| row.param.key == "slit_width")
        .unwrap();
    assert_eq!(own.value, mm(0.5));
    assert_eq!(own.used, None);
    // The box places, resizes with the radius kept and converts to its five plates.
    assert_eq!(document.generated(id).unwrap().record.parts.len(), 5);
}

#[test]
fn ring_thickness_and_the_hollow_rows_need_the_lid_and_its_seat() {
    let (document, id, settings) = placed(ROUNDED);
    let labels = |document: &Document| -> Vec<String> {
        instance::view(&settings, &document.generated(id).unwrap())
            .rows
            .iter()
            .map(|row| row.param.label.to_owned())
            .collect()
    };
    for shown in [
        "Ring thickness",
        "Finger hollow",
        "Hollow width",
        "Hollow depth",
    ] {
        assert!(labels(&document).contains(&shown.to_owned()), "{shown}");
    }
    instance::change_value(&document, &settings, id, "lid", &ParamValue::Switch(false)).unwrap();
    let after = labels(&document);
    for hidden in [
        "Ring thickness",
        "Finger hollow",
        "Hollow width",
        "Hollow depth",
        "Lid seat",
    ] {
        assert!(
            !after.contains(&hidden.to_owned()),
            "{hidden} stays hidden without a lid"
        );
    }
}

#[test]
fn the_wave_pattern_names_itself_when_it_raises_the_slit_spacing() {
    let (document, id, settings) = placed(ROUNDED);
    let choice = |id: &str| ParamValue::Choice(id.to_owned());
    instance::change_value(&document, &settings, id, "slit_pattern", &choice("wave")).unwrap();
    let changed =
        instance::change_value(&document, &settings, id, "slit_spacing", &mm(1.3)).unwrap();
    assert_eq!(
        changed.notices,
        ["Slit spacing set to 1.48 mm, the least the Wave pattern allows."]
    );
    // Without the Wave pattern the plain sentence stays.
    instance::change_value(
        &document,
        &settings,
        id,
        "slit_pattern",
        &choice("straight"),
    )
    .unwrap();
    let changed =
        instance::change_value(&document, &settings, id, "slit_spacing", &mm(0.6)).unwrap();
    assert_eq!(
        changed.notices,
        ["Slit spacing set to 1.3 mm, the least this rounded box allows."]
    );
}

#[test]
fn a_full_round_corner_radius_reports_its_state_for_the_field() {
    let (document, id, settings) = placed(ROUNDED);
    let state = |document: &Document| {
        instance::view(&settings, &document.generated(id).unwrap())
            .rows
            .iter()
            .find(|row| row.param.key == "corner_radius")
            .unwrap()
            .state
    };
    assert_eq!(state(&document), None);
    instance::change_value(&document, &settings, id, "corner_radius", &mm(40.0)).unwrap();
    assert_eq!(state(&document), Some("full round"));
}

#[test]
fn a_gear_resize_previews_whole_teeth_and_reads_teeth_and_outer_diameter() {
    let (document, id, settings) = placed(GEAR);
    let ObjectSnapshot::Path(path) = document.object(id).unwrap() else {
        panic!("path");
    };
    let before = path.anchors.len();
    // The pointer asks for 1.5 times the size: m (z + 2) = 52 becomes 78, so z = 37.
    let result = ObjectSnapshot::Path(path.mapped(Affine::new(1.5, 0.0, 0.0, 1.5, 0.0, 0.0)));
    let view = instance::resize_view(&document, &settings, &result).expect("a gear steps by teeth");
    assert_eq!(view.readout, "37 teeth · ⌀ 78 mm");
    let ObjectSnapshot::Path(preview) = view.preview else {
        panic!("a path");
    };
    assert_eq!(preview.id, id, "the preview is the object's own");
    assert!(preview.anchors.len() > before, "more teeth, more anchors");
    // The bounds of the whole-tooth gear, not the pointer's exact size.
    let tiny = ObjectSnapshot::Path(path.mapped(Affine::new(0.01, 0.0, 0.0, 0.01, 0.0, 0.0)));
    assert_eq!(
        instance::resize_view(&document, &settings, &tiny)
            .unwrap()
            .readout,
        "6 teeth min · ⌀ 16 mm"
    );
    let huge = ObjectSnapshot::Path(path.mapped(Affine::new(50.0, 0.0, 0.0, 50.0, 0.0, 0.0)));
    assert_eq!(
        instance::resize_view(&document, &settings, &huge)
            .unwrap()
            .readout,
        "200 teeth max · ⌀ 404 mm"
    );
    // Other generators keep the plain size readout.
    let (document, id, settings) = placed(BOX);
    let ObjectSnapshot::Path(path) = document.object(id).unwrap() else {
        panic!("path");
    };
    let result = ObjectSnapshot::Path(path.mapped(Affine::scale(2.0, 2.0)));
    assert!(instance::resize_view(&document, &settings, &result).is_none());
}

#[test]
fn a_row_reports_the_live_limit_of_its_dependent_bound() {
    let (document, id, settings) = placed(ROUNDED);
    instance::change_value(&document, &settings, id, "corner_radius", &mm(40.0)).unwrap();
    let view = instance::view(&settings, &document.generated(id).unwrap());
    let limit = |key: &str| {
        view.rows
            .iter()
            .find(|row| row.param.key == key)
            .unwrap()
            .limit
            .clone()
    };
    assert_eq!(
        limit("hollow_width"),
        Some(mm(40.0)),
        "the front straight at r 40"
    );
    assert_eq!(limit("corner_radius"), Some(mm(40.0)));
    assert_eq!(limit("thickness"), None);
}
