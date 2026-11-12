use super::*;

fn set(power: f64, speed: u64, passes: u64) -> Settings {
    Settings {
        power: PowerPercent::from_percent(power).unwrap(),
        speed: FeedRate::new(speed).unwrap(),
        passes: Passes::new(passes).unwrap(),
    }
}

fn damage(value: &Value, kind: OperationKind) -> Option<SettingsDamage> {
    match read_settings(Some(value), kind) {
        SettingsRead::Damaged(damage) => Some(damage),
        _ => None,
    }
}

#[test]
fn a_linked_value_with_overrides_reads_back_and_writes_the_same_plain_values() {
    let value = json!({
        "main": {"power_pct": 60.0, "speed_mm_min": 3000, "passes": 1},
        "outline": {"power_pct": 25.0, "speed_mm_min": 6000, "passes": 1},
        "material": {"id": "m1", "name": "Birch 3 mm", "outline_from": "score",
                     "modified": 1_762_862_400_000_u64},
        "overrides": {"main": {"power_pct": 65.0}, "outline": {"passes": 2}},
    });
    let SettingsRead::Stored(stored) = read_settings(Some(&value), OperationKind::EngraveOutline)
    else {
        panic!("reads");
    };
    assert_eq!(stored.main, set(60.0, 3000, 1));
    let link = stored.material.as_ref().unwrap();
    assert_eq!(link.outline_from, Some(SetKind::Score));
    assert_eq!(link.modified, 1_762_862_400_000);
    assert_eq!(
        stored.overrides.main.power,
        PowerPercent::from_percent(65.0)
    );
    assert_eq!(stored.overrides.outline.passes, Passes::new(2));
    assert_eq!(stored.overrides.main.fields(), [Field::Power]);
    assert_eq!(
        stored.to_value(),
        value,
        "writes back the same plain values"
    );
    assert_eq!(read_settings(None, OperationKind::Cut), SettingsRead::None);
}

#[test]
fn own_numbers_carry_the_outline_only_for_engrave_outline() {
    let own = StoredSettings::own(OperationKind::EngraveOutline, set(30.0, 6000, 1), None);
    assert_eq!(own.outline, Some(set(30.0, 6000, 1)));
    let cut = StoredSettings::own(OperationKind::Cut, set(60.0, 300, 2), None);
    assert_eq!(cut.outline, None);
    assert_eq!(cut.stored_set(OperationKind::Cut, Role::Outline), None);
    assert_eq!(
        cut.to_value(),
        json!({"main": {"power_pct": 60.0, "speed_mm_min": 300, "passes": 2}})
    );
}

#[test]
fn every_damage_of_adr_0054_is_named_and_read_as_damaged() {
    let main = json!({"power_pct": 60.0, "speed_mm_min": 3000, "passes": 1});
    let link = json!({"id": "m1", "name": "Birch", "modified": 5});
    let cut = OperationKind::Cut;
    assert_eq!(damage(&json!(7), cut), Some(SettingsDamage::NotAMap));
    assert_eq!(damage(&Value::Null, cut), Some(SettingsDamage::NotAMap));
    assert_eq!(
        damage(&json!({"main": main, "kerf": 1}), cut),
        Some(SettingsDamage::UnknownKey)
    );
    for bad in [
        json!({"power_pct": 0.0, "speed_mm_min": 3000, "passes": 1}),
        json!({"power_pct": 60.0, "speed_mm_min": 0, "passes": 1}),
        json!({"power_pct": 60.0, "speed_mm_min": 3000, "passes": 21}),
        json!({"power_pct": 60.25, "speed_mm_min": 3000, "passes": 1}),
        json!({"power_pct": 60.0, "speed_mm_min": 3000}),
    ] {
        assert_eq!(
            damage(&json!({ "main": bad }), cut),
            Some(SettingsDamage::BadSet)
        );
    }
    assert_eq!(damage(&json!({}), cut), Some(SettingsDamage::BadSet));
    assert_eq!(
        damage(&json!({"main": main, "outline": main}), cut),
        Some(SettingsDamage::OutlineOnKind)
    );
    assert_eq!(
        damage(
            &json!({"main": main, "material": {"name": "x", "modified": 1}}),
            cut
        ),
        Some(SettingsDamage::BadLink),
        "a link without an id"
    );
    assert_eq!(
        damage(
            &json!({"main": main, "material": {"id": "m1", "name": "x"}}),
            cut
        ),
        Some(SettingsDamage::BadLink),
        "a link without modified"
    );
    for (overrides, why) in [
        (json!({}), "empty overrides"),
        (json!({"main": {}}), "an empty role map"),
        (json!({"main": {"kerf": 1}}), "an unknown key"),
        (json!({"main": {"power_pct": 0.5}}), "a bad number"),
        (json!({"outline": {"passes": 2}}), "an outline on a Cut"),
        (json!({"hatch": {"passes": 2}}), "an unknown role"),
    ] {
        assert_eq!(
            damage(
                &json!({"main": main, "material": link, "overrides": overrides}),
                cut
            ),
            Some(SettingsDamage::BadOverride),
            "{why}"
        );
    }
    assert_eq!(
        damage(
            &json!({"main": main, "overrides": {"main": {"passes": 2}}}),
            cut
        ),
        Some(SettingsDamage::BadOverride),
        "overrides without a link"
    );
}

#[test]
fn an_engrave_interval_reads_back_and_absent_reads_the_default() {
    let value = json!({
        "main": {"power_pct": 30.0, "speed_mm_min": 6000, "passes": 1, "interval_mm": 0.08},
        "material": {"id": "m1", "name": "Birch", "modified": 5},
        "overrides": {"main": {"interval_mm": 0.12}},
    });
    for kind in [OperationKind::Engrave, OperationKind::EngraveOutline] {
        let SettingsRead::Stored(stored) = read_settings(Some(&value), kind) else {
            panic!("reads for {kind:?}");
        };
        assert_eq!(stored.interval.unwrap().hundredths(), 8);
        assert_eq!(stored.overrides.main.interval.unwrap().hundredths(), 12);
        assert_eq!(stored.overrides.main.fields(), []);
        assert_eq!(stored.to_value(), value);
    }
    let own = StoredSettings::own(OperationKind::Engrave, set(30.0, 6000, 1), None);
    assert_eq!(own.interval, None);
    assert_eq!(
        own.stored_interval(OperationKind::Engrave),
        Some(Interval::DEFAULT)
    );
    assert_eq!(own.stored_interval(OperationKind::Score), None);
}

#[test]
fn an_interval_out_of_place_or_range_is_damage() {
    let main = |interval: Value| {
        json!({"main": {"power_pct": 30.0, "speed_mm_min": 6000, "passes": 1,
                        "interval_mm": interval}})
    };
    for kind in [
        OperationKind::Cut,
        OperationKind::Score,
        OperationKind::CutOut,
    ] {
        assert_eq!(
            damage(&main(json!(0.1)), kind),
            Some(SettingsDamage::BadInterval),
            "{kind:?}"
        );
    }
    for bad in [json!(0.005), json!(2.01), json!(0.125), json!("0.1")] {
        assert_eq!(
            damage(&main(bad.clone()), OperationKind::Engrave),
            Some(SettingsDamage::BadInterval),
            "{bad}"
        );
    }
    let outline = json!({
        "main": {"power_pct": 30.0, "speed_mm_min": 6000, "passes": 1},
        "outline": {"power_pct": 30.0, "speed_mm_min": 6000, "passes": 1, "interval_mm": 0.1},
    });
    assert_eq!(
        damage(&outline, OperationKind::EngraveOutline),
        Some(SettingsDamage::BadInterval)
    );
    let outline_override = json!({
        "main": {"power_pct": 30.0, "speed_mm_min": 6000, "passes": 1},
        "material": {"id": "m1", "name": "Birch", "modified": 5},
        "overrides": {"outline": {"interval_mm": 0.1}},
    });
    assert_eq!(
        damage(&outline_override, OperationKind::EngraveOutline),
        Some(SettingsDamage::BadInterval)
    );
}
