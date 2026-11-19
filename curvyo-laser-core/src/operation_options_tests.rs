//! The tests of `operation_options.rs`.

use super::*;

fn read(value: &Value, kind: OperationKind) -> Result<OperationOptions, OptionsDamage> {
    read_options(Some(value), kind)
}

#[test]
fn absent_options_read_every_default() {
    let options = read_options(None, OperationKind::Engrave).unwrap();
    assert!(!options.crosshatch);
    assert!(options.bidirectional);
    assert_eq!(options.scan_angle.degrees(), 0.0);
    assert_eq!(options.overscan.percent(), 2);
    assert_eq!(options.z_offset.mm(), 0.0);
    assert_eq!(read(&json!({}), OperationKind::Cut), Ok(options));
}

#[test]
fn a_full_engrave_map_reads_and_writes_back_the_same_plain_values() {
    let value = json!({
        "crosshatch": true, "bidirectional": false, "scan_angle_deg": 15.0,
        "overscan_pct": 50, "z_offset_mm": -1.5,
    });
    let options = read(&value, OperationKind::EngraveOutline).unwrap();
    assert!(options.crosshatch);
    assert!(!options.bidirectional);
    assert_eq!(options.scan_angle.tenths(), 150);
    assert_eq!(options.overscan.percent(), 50);
    assert_eq!(options.z_offset.tenths(), -15);
    assert_eq!(options.to_value(OperationKind::EngraveOutline), value);
}

#[test]
fn a_cut_takes_the_z_offset_only() {
    let value = json!({"z_offset_mm": 20.0});
    let options = read(&value, OperationKind::Cut).unwrap();
    assert_eq!(options.z_offset.tenths(), 200);
    assert_eq!(options.to_value(OperationKind::Cut), value);
    for hatch in [
        KEY_CROSSHATCH,
        KEY_BIDIRECTIONAL,
        KEY_SCAN_ANGLE,
        KEY_OVERSCAN,
    ] {
        let mut map = Map::new();
        map.insert(hatch.to_owned(), json!(false));
        for kind in [
            OperationKind::Cut,
            OperationKind::Score,
            OperationKind::CutOut,
        ] {
            assert_eq!(
                read(&Value::Object(map.clone()), kind),
                Err(OptionsDamage::KeyOnKind),
                "{hatch} on {kind:?}"
            );
        }
    }
}

#[test]
fn every_damage_reads_as_damage() {
    let engrave = OperationKind::Engrave;
    let cases = [
        (json!([1]), OptionsDamage::NotAMap),
        (json!(null), OptionsDamage::NotAMap),
        (json!({"halftone": true}), OptionsDamage::UnknownKey),
        (json!({"crosshatch": 1}), OptionsDamage::WrongType),
        (json!({"scan_angle_deg": "15"}), OptionsDamage::WrongType),
        (json!({"overscan_pct": 2.5}), OptionsDamage::WrongType),
        (json!({"overscan_pct": -1}), OptionsDamage::WrongType),
        (json!({"scan_angle_deg": 90.1}), OptionsDamage::BadValue),
        (json!({"scan_angle_deg": -90.1}), OptionsDamage::BadValue),
        (json!({"scan_angle_deg": 12.25}), OptionsDamage::BadValue),
        (json!({"overscan_pct": 51}), OptionsDamage::BadValue),
        (json!({"z_offset_mm": 20.1}), OptionsDamage::BadValue),
        (json!({"z_offset_mm": 0.05}), OptionsDamage::BadValue),
    ];
    for (value, damage) in cases {
        assert_eq!(read(&value, engrave), Err(damage), "{value}");
    }
    assert_eq!(
        read(
            &json!({"scan_angle_deg": -90.0, "z_offset_mm": -20.0}),
            engrave
        )
        .unwrap()
        .scan_angle
        .tenths(),
        -900
    );
}

#[test]
fn typed_values_round_to_their_decimals_and_name_their_range() {
    assert_eq!(ScanAngle::typed(15.04).unwrap().tenths(), 150);
    assert_eq!(ScanAngle::typed(90.1), Err("Scan angle is -90.0 to 90.0°."));
    assert_eq!(ScanAngle::typed(f64::NAN), Err("Enter a number."));
    assert_eq!(Overscan::typed(50.0).unwrap().percent(), 50);
    assert_eq!(Overscan::typed(51.0), Err("Overscanning is 0 to 50 %."));
    assert_eq!(Overscan::typed(-1.0), Err("Overscanning is 0 to 50 %."));
    assert_eq!(Overscan::typed(2.5), Err("Enter a whole number."));
    assert_eq!(ZOffset::typed(1.54).unwrap().tenths(), 15);
    assert_eq!(ZOffset::typed(-20.5), Err("Z offset is -20.0 to 20.0 mm."));
    // Out of range before rounding is refused, never rounded into the range (criterion 24).
    assert_eq!(
        ScanAngle::typed(90.04),
        Err("Scan angle is -90.0 to 90.0°.")
    );
    assert_eq!(ScanAngle::typed(-90.0).unwrap().tenths(), -900);
    assert_eq!(ZOffset::typed(-20.04), Err("Z offset is -20.0 to 20.0 mm."));
    assert_eq!(ZOffset::typed(20.0).unwrap().tenths(), 200);
    for text in [
        ScanAngle::RANGE_TEXT,
        Overscan::RANGE_TEXT,
        ZOffset::RANGE_TEXT,
    ] {
        assert!(text.chars().count() <= 70);
    }
}

#[test]
fn an_edit_changes_one_value_and_a_hatch_edit_on_a_cut_is_refused() {
    let options = OperationOptions::default();
    let on = options
        .with(OperationKind::Engrave, OptionEdit::Crosshatch(true))
        .unwrap();
    assert!(on.crosshatch);
    assert_eq!(
        options.with(OperationKind::Engrave, OptionEdit::Bidirectional(true)),
        None,
        "unchanged"
    );
    assert_eq!(
        options.with(OperationKind::Cut, OptionEdit::Crosshatch(true)),
        None
    );
    let z = ZOffset::typed(1.5).unwrap();
    let cut = options
        .with(OperationKind::Cut, OptionEdit::ZOffset(z))
        .unwrap();
    assert_eq!(
        cut.to_value(OperationKind::Cut),
        json!({"z_offset_mm": 1.5})
    );
    assert_eq!(cut.running_z_offset(false), None, "kept, runs nothing");
    assert_eq!(cut.running_z_offset(true), Some(z));
}

#[test]
fn each_key_is_valid_on_its_kinds_only() {
    let image = OperationKind::Image;
    let scans = json!({"bidirectional": false, "scan_angle_deg": 30.0, "overscan_pct": 5,
        "z_offset_mm": 0.0});
    let options = read(&scans, image).unwrap();
    assert!(!options.bidirectional);
    assert_eq!(options.scan_angle.tenths(), 300);
    assert_eq!(options.to_value(image), scans, "no crosshatch key on Image");
    let damaged = [
        (json!({"crosshatch": false}), image),
        (json!({"dither": "ordered"}), OperationKind::Engrave),
        (json!({"threshold_pct": 40}), OperationKind::EngraveOutline),
        (json!({"gamma": 1.0}), OperationKind::Cut),
    ];
    for (value, kind) in damaged {
        assert_eq!(
            read(&value, kind),
            Err(OptionsDamage::KeyOnKind),
            "{value} on {kind:?}"
        );
    }
    let defaults = OperationOptions::default();
    assert_eq!(defaults.with(image, OptionEdit::Crosshatch(true)), None);
    assert!(
        defaults
            .with(image, OptionEdit::Bidirectional(false))
            .is_some()
    );
    let grey = OptionEdit::Image(ImageEdit::Dither(
        crate::operation_image_options::Dither::Grey,
    ));
    assert!(defaults.with(image, grey).is_some());
    assert_eq!(defaults.with(OperationKind::Engrave, grey), None);
}
