//! The tests of `operation_image_options.rs`.

use super::*;
use crate::operation_kind::OperationKind;
use crate::operation_options::read_options;

#[test]
fn missing_keys_read_their_defaults() {
    let options = read_options(None, OperationKind::Image).unwrap();
    assert_eq!(options.image, ImageSettings::DEFAULT);
    assert_eq!(options.image.dither, Dither::FloydSteinberg);
    assert_eq!(options.image.threshold_pct, 50);
    assert_eq!(options.image.gamma_hundredths, 100);
}

#[test]
fn every_key_reads_and_writes_back_equal() {
    let value = json!({
        "bidirectional": true, "scan_angle_deg": 0.0,
        "overscan_pct": 2, "z_offset_mm": 0.0,
        "dither": "grey", "threshold_pct": 100, "brightness_pct": -100,
        "contrast_pct": 100, "gamma": 4.0, "invert": true, "lowest_power_pct": 12.5,
    });
    let options = read_options(Some(&value), OperationKind::Image).unwrap();
    let image = options.image;
    assert_eq!(image.dither, Dither::Grey);
    assert_eq!(image.threshold_pct, 100);
    assert_eq!(image.brightness_pct, -100);
    assert_eq!(image.contrast_pct, 100);
    assert_eq!(image.gamma_hundredths, 400);
    assert!(image.invert);
    assert_eq!(image.lowest_power_tenths, 125);
    assert_eq!(options.to_value(OperationKind::Image), value);
    for mode in Dither::ALL {
        assert_eq!(Dither::from_key(mode.key()), Some(mode));
    }
}

#[test]
fn default_settings_write_no_image_key() {
    let options = read_options(None, OperationKind::Image).unwrap();
    let value = options.to_value(OperationKind::Image);
    for key in IMAGE_KEYS {
        assert!(value.get(key).is_none(), "{key}");
    }
}

#[test]
fn typed_values_round_to_their_step_and_name_their_range() {
    use ImageField as F;
    assert_eq!(F::Threshold.typed(50.4), Ok(ImageEdit::Threshold(50)));
    assert_eq!(F::Threshold.typed(99.5), Ok(ImageEdit::Threshold(100)));
    assert_eq!(F::Threshold.typed(100.4), Err("Threshold is 0 to 100 %."));
    assert_eq!(F::Threshold.typed(-0.4), Err("Threshold is 0 to 100 %."));
    assert_eq!(F::Brightness.typed(-100.0), Ok(ImageEdit::Brightness(-100)));
    assert_eq!(
        F::Brightness.typed(101.0),
        Err("Brightness is -100 to 100 %.")
    );
    assert_eq!(F::Contrast.typed(12.6), Ok(ImageEdit::Contrast(13)));
    assert_eq!(F::Contrast.typed(-101.0), Err("Contrast is -100 to 100 %."));
    assert_eq!(F::Gamma.typed(1.234), Ok(ImageEdit::Gamma(123)));
    assert_eq!(F::Gamma.typed(0.10), Ok(ImageEdit::Gamma(10)));
    assert_eq!(F::Gamma.typed(4.0), Ok(ImageEdit::Gamma(400)));
    assert_eq!(F::Gamma.typed(4.01), Err("Gamma is 0.10 to 4.00."));
    assert_eq!(F::Gamma.typed(0.09), Err("Gamma is 0.10 to 4.00."));
    assert_eq!(F::LowestPower.typed(12.34), Ok(ImageEdit::LowestPower(123)));
    assert_eq!(
        F::LowestPower.typed(100.0),
        Ok(ImageEdit::LowestPower(1000))
    );
    assert_eq!(
        F::LowestPower.typed(100.1),
        Err("Lowest power is 0 to 100 %.")
    );
    for field in [
        F::Threshold,
        F::Brightness,
        F::Contrast,
        F::Gamma,
        F::LowestPower,
    ] {
        assert_eq!(field.typed(f64::NAN), Err("Enter a number."));
        assert!(field.range_text().chars().count() <= 70);
    }
}

#[test]
fn an_image_edit_changes_one_setting_and_reads_back_from_the_stored_map() {
    use crate::operation_options::{OperationOptions, OptionEdit};
    let kind = OperationKind::Image;
    let options = OperationOptions::default();
    let edits = [
        ImageEdit::Dither(Dither::Grey),
        ImageEdit::Threshold(70),
        ImageEdit::Brightness(-20),
        ImageEdit::Contrast(35),
        ImageEdit::Gamma(180),
        ImageEdit::Invert(true),
        ImageEdit::LowestPower(125),
    ];
    let mut now = options;
    for edit in edits {
        let next = now.with(kind, OptionEdit::Image(edit)).unwrap();
        assert_ne!(next.image, now.image, "{edit:?}");
        assert_eq!(
            (next.crosshatch, next.scan_angle, next.overscan),
            (now.crosshatch, now.scan_angle, now.overscan)
        );
        let stored = next.to_value(kind);
        assert_eq!(read_options(Some(&stored), kind), Ok(next), "{edit:?}");
        now = next;
    }
    assert_eq!(
        now.image,
        ImageSettings {
            dither: Dither::Grey,
            threshold_pct: 70,
            brightness_pct: -20,
            contrast_pct: 35,
            gamma_hundredths: 180,
            invert: true,
            lowest_power_tenths: 125,
            ..ImageSettings::DEFAULT
        }
    );
    // The same value writes nothing; a Cut takes no Image setting.
    let same = OptionEdit::Image(ImageEdit::Dither(Dither::FloydSteinberg));
    assert_eq!(options.with(kind, same), None);
    let invert = OptionEdit::Image(ImageEdit::Invert(true));
    for other in [
        OperationKind::Cut,
        OperationKind::Score,
        OperationKind::CutOut,
    ] {
        assert_eq!(options.with(other, invert), None, "{other:?}");
    }
    // Back to every default: the map is `0111`'s again.
    let back = now
        .with(
            kind,
            OptionEdit::Image(ImageEdit::Dither(Dither::FloydSteinberg)),
        )
        .and_then(|o| o.with(kind, OptionEdit::Image(ImageEdit::Threshold(50))))
        .and_then(|o| o.with(kind, OptionEdit::Image(ImageEdit::Brightness(0))))
        .and_then(|o| o.with(kind, OptionEdit::Image(ImageEdit::Contrast(0))))
        .and_then(|o| o.with(kind, OptionEdit::Image(ImageEdit::Gamma(100))))
        .and_then(|o| o.with(kind, OptionEdit::Image(ImageEdit::Invert(false))))
        .and_then(|o| o.with(kind, OptionEdit::Image(ImageEdit::LowestPower(0))))
        .unwrap();
    assert_eq!(back.to_value(kind), options.to_value(kind));
}

#[test]
fn every_damage_reads_as_damage() {
    let cases = [
        (json!({"dither": "halftone_x"}), OptionsDamage::BadValue),
        (json!({"screen_size_mm": 0.19}), OptionsDamage::BadValue),
        (json!({"screen_size_mm": 0.805}), OptionsDamage::BadValue),
        (json!({"screen_angle_deg": 90}), OptionsDamage::BadValue),
        (json!({"screen_angle_deg": 4.5}), OptionsDamage::WrongType),
        (json!({"dither": 1}), OptionsDamage::WrongType),
        (json!({"threshold_pct": 101}), OptionsDamage::BadValue),
        (json!({"threshold_pct": 50.5}), OptionsDamage::WrongType),
        (json!({"brightness_pct": 101}), OptionsDamage::BadValue),
        (json!({"contrast_pct": -101}), OptionsDamage::BadValue),
        (json!({"contrast_pct": 1.5}), OptionsDamage::WrongType),
        (json!({"gamma": 4.01}), OptionsDamage::BadValue),
        (json!({"gamma": 0.09}), OptionsDamage::BadValue),
        (json!({"gamma": 1.005}), OptionsDamage::BadValue),
        (json!({"gamma": "1"}), OptionsDamage::WrongType),
        (json!({"invert": 0}), OptionsDamage::WrongType),
        (json!({"lowest_power_pct": 100.1}), OptionsDamage::BadValue),
        (json!({"lowest_power_pct": 10.25}), OptionsDamage::BadValue),
    ];
    for (value, damage) in cases {
        assert_eq!(
            read_options(Some(&value), OperationKind::Image),
            Err(damage),
            "{value}"
        );
    }
    for kind in [
        OperationKind::Cut,
        OperationKind::Score,
        OperationKind::CutOut,
        OperationKind::Engrave,
        OperationKind::EngraveOutline,
    ] {
        for key in IMAGE_KEYS {
            let mut map = Map::new();
            map.insert(key.to_owned(), json!(null));
            assert_eq!(
                read_options(Some(&Value::Object(map)), kind),
                Err(OptionsDamage::KeyOnKind),
                "{key} on {kind:?}"
            );
        }
    }
}
