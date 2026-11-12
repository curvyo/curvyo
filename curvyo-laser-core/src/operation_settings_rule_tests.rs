use std::collections::BTreeMap;

use super::*;
use crate::material::{Interval, MachineTie, MaterialSet, SetSource, Thickness};
use crate::operation_settings::SetOverride;

const PROFILE: &str = "generic-laser";

fn set(power: f64, speed: u64, passes: u64) -> Settings {
    Settings {
        power: PowerPercent::from_percent(power).unwrap(),
        speed: FeedRate::new(speed).unwrap(),
        passes: Passes::new(passes).unwrap(),
    }
}

fn material(id: &str, modified: u64, sets: &[(SetKind, Settings)]) -> Material {
    Material {
        id: id.to_owned(),
        modified,
        name: "Birch 3 mm".to_owned(),
        thickness: Thickness::from_mm(3.0).unwrap(),
        machine: MachineTie {
            id: PROFILE.to_owned(),
            name: "Generic laser".to_owned(),
        },
        sets: sets
            .iter()
            .map(|&(kind, settings)| {
                (
                    kind,
                    MaterialSet {
                        settings,
                        source: SetSource::Edited,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>(),
        history: Vec::new(),
    }
}

fn linked(main: Settings, modified: u64, overrides: Overrides) -> SettingsRead {
    SettingsRead::Stored(StoredSettings {
        main,
        interval: None,
        outline: None,
        material: Some(MaterialLink {
            id: "m1".to_owned(),
            name: "Birch".to_owned(),
            outline_from: None,
            modified,
        }),
        overrides,
    })
}

fn power_override(power: f64) -> Overrides {
    Overrides {
        main: SetOverride {
            power: PowerPercent::from_percent(power),
            ..SetOverride::default()
        },
        ..Overrides::default()
    }
}

#[test]
fn the_sets_follow_the_kind_and_the_outline_falls_back_to_engrave() {
    assert_eq!(main_set(OperationKind::CutOut), SetKind::Cut);
    assert_eq!(main_set(OperationKind::EngraveOutline), SetKind::Engrave);
    let both = material(
        "m1",
        7,
        &[
            (SetKind::Engrave, set(30.0, 6000, 1)),
            (SetKind::Score, set(15.0, 9000, 1)),
        ],
    );
    let stored = use_material(&both, OperationKind::EngraveOutline, &SettingsRead::None).unwrap();
    assert_eq!(stored.outline, Some(set(15.0, 9000, 1)));
    let link = stored.material.unwrap();
    assert_eq!(
        (link.outline_from, link.modified),
        (Some(SetKind::Score), 7)
    );
    assert!(stored.overrides.is_empty(), "a pick clears typed numbers");
    let engrave_only = material("m2", 1, &[(SetKind::Engrave, set(30.0, 6000, 1))]);
    let stored = use_material(
        &engrave_only,
        OperationKind::EngraveOutline,
        &SettingsRead::None,
    )
    .unwrap();
    assert_eq!(stored.outline, Some(set(30.0, 6000, 1)));
    assert_eq!(
        stored.material.unwrap().outline_from,
        Some(SetKind::Engrave)
    );
    assert_eq!(
        use_material(&engrave_only, OperationKind::Cut, &SettingsRead::None),
        Err(SetKind::Cut),
        "skipped and named"
    );
    let cut = use_material(
        &material("m3", 1, &[(SetKind::Cut, set(60.0, 300, 2))]),
        OperationKind::CutOut,
        &SettingsRead::None,
    )
    .unwrap();
    assert_eq!((cut.main, cut.outline), (set(60.0, 300, 2), None));
}

#[test]
fn a_live_link_runs_the_material_with_the_typed_numbers_on_top() {
    let library = [MaterialRead::Loaded(material(
        "m1",
        9,
        &[(SetKind::Cut, set(65.0, 3000, 1))],
    ))];
    let plain = linked(set(60.0, 3000, 1), 5, Overrides::default());
    let state = resolve(&plain, OperationKind::Cut, &library, PROFILE);
    let SettingsState::Live {
        name,
        run,
        material,
        ..
    } = &state
    else {
        panic!("live: {state:?}");
    };
    assert_eq!(name, "Birch 3 mm", "the material's current name");
    assert_eq!(
        (run.main, material.main),
        (set(65.0, 3000, 1), set(65.0, 3000, 1))
    );
    assert_eq!(state.differs_text(), None);

    let typed = linked(set(60.0, 3000, 1), 5, power_override(70.0));
    let state = resolve(&typed, OperationKind::Cut, &library, PROFILE);
    assert_eq!(state.run().unwrap().main, set(70.0, 3000, 1));
    assert_eq!(state.material().unwrap().main, set(65.0, 3000, 1));
    assert_eq!(
        state.differs_text().as_deref(),
        Some("Differs from Birch 3 mm")
    );
    let SettingsState::Live { overridden, .. } = &state else {
        panic!("live");
    };
    assert_eq!(overridden, &[(Role::Main, Field::Power)]);
}

#[test]
fn a_link_runs_its_stored_numbers_with_the_reason_and_the_typed_numbers() {
    let library = [MaterialRead::Loaded(material(
        "m1",
        9,
        &[(SetKind::Cut, set(65.0, 3000, 1))],
    ))];
    let read = linked(set(60.0, 3000, 1), 5, power_override(70.0));
    let reason = |library: &[MaterialRead], profile: &str, kind| match resolve(
        &read, kind, library, profile,
    ) {
        SettingsState::Stored {
            reason,
            run,
            material,
            ..
        } => {
            assert_eq!(
                material.main,
                set(60.0, 3000, 1),
                "never changed or guessed"
            );
            assert_eq!(run.main, set(70.0, 3000, 1), "the typed power on top");
            Some(reason)
        }
        _ => None,
    };
    assert_eq!(
        reason(&[], PROFILE, OperationKind::Cut),
        Some(StoredReason::Missing)
    );
    let damaged = [MaterialRead::Damaged {
        id: "m1".to_owned(),
        reason: "x".to_owned(),
    }];
    assert_eq!(
        reason(&damaged, PROFILE, OperationKind::Cut),
        Some(StoredReason::Damaged)
    );
    let newer = [MaterialRead::Newer {
        id: "m1".to_owned(),
    }];
    assert_eq!(
        reason(&newer, PROFILE, OperationKind::Cut),
        Some(StoredReason::Newer)
    );
    assert_eq!(
        reason(&library, "other", OperationKind::Cut),
        Some(StoredReason::OtherMachine)
    );
    assert_eq!(
        reason(&library, PROFILE, OperationKind::Score),
        Some(StoredReason::NoSet(SetKind::Score))
    );
    assert_eq!(
        StoredReason::NoSet(SetKind::Score).text(),
        "Material has no Score set, using stored numbers"
    );
    assert_eq!(
        StoredReason::Missing.text(),
        "Material not found, using stored numbers"
    );
}

#[test]
fn own_none_and_damaged_states_and_the_feed_check() {
    let own = SettingsRead::Stored(StoredSettings::own(
        OperationKind::EngraveOutline,
        set(70.0, 7000, 1),
        None,
    ));
    let state = resolve(&own, OperationKind::EngraveOutline, &[], "p");
    let run = *state.run().unwrap();
    assert_eq!(run.outline, Some(set(70.0, 7000, 1)), "outline from main");
    assert!(
        run.over_feed(FeedRate::new(6000).unwrap()),
        "named, not damage"
    );
    assert!(!run.over_feed(FeedRate::new(7000).unwrap()));
    assert_eq!(
        resolve(&SettingsRead::None, OperationKind::Cut, &[], "p"),
        SettingsState::NoSettings
    );
    let damaged = SettingsRead::Damaged(SettingsDamage::BadSet);
    assert!(
        resolve(&damaged, OperationKind::Cut, &[], "p")
            .run()
            .is_none()
    );
}

#[test]
fn refresh_writes_only_a_live_link_that_differs_and_keeps_the_typed_numbers() {
    let library = [MaterialRead::Loaded(material(
        "m1",
        9,
        &[(SetKind::Cut, set(65.0, 3000, 1))],
    ))];
    let behind = linked(set(60.0, 3000, 1), 5, power_override(70.0));
    let fresh = refresh(&behind, OperationKind::Cut, &library, PROFILE, false).unwrap();
    assert_eq!(fresh.main, set(65.0, 3000, 1));
    let link = fresh.material.as_ref().unwrap();
    assert_eq!((link.name.as_str(), link.modified), ("Birch 3 mm", 9));
    assert_eq!(fresh.overrides, power_override(70.0), "typed numbers kept");
    let current = SettingsRead::Stored(fresh);
    assert_eq!(
        refresh(&current, OperationKind::Cut, &library, PROFILE, false),
        None,
        "nothing differs"
    );
    assert!(refresh(&behind, OperationKind::Cut, &library, PROFILE, true).is_some());
    let synced_late = linked(set(60.0, 3000, 1), 9, Overrides::default());
    assert_eq!(
        refresh(&synced_late, OperationKind::Cut, &library, PROFILE, true),
        None,
        "the guard: the record is not newer than the link"
    );
    assert!(
        refresh(&synced_late, OperationKind::Cut, &library, PROFILE, false).is_some(),
        "a material edit in this project writes without the guard"
    );
    assert_eq!(
        refresh(&behind, OperationKind::Cut, &[], PROFILE, false),
        None,
        "stored: never written"
    );
    let own = SettingsRead::Stored(StoredSettings::own(
        OperationKind::Cut,
        set(1.0, 1, 1),
        None,
    ));
    assert_eq!(
        refresh(&own, OperationKind::Cut, &library, PROFILE, false),
        None
    );
}

#[test]
fn a_typed_number_changes_own_numbers_or_becomes_an_override_of_a_link() {
    let cut = OperationKind::Cut;
    let own = SettingsRead::Stored(StoredSettings::own(cut, set(60.0, 3000, 1), None));
    let state = resolve(&own, cut, &[], PROFILE);
    let power = |p: f64| TypedNumber::Power(PowerPercent::from_percent(p).unwrap());
    let next = edit_number(&own, &state, cut, Role::Main, power(65.0)).unwrap();
    assert_eq!(next.main, set(65.0, 3000, 1));
    assert_eq!(
        edit_number(&own, &state, cut, Role::Main, power(60.0)),
        None
    );
    assert_eq!(
        edit_number(&own, &state, cut, Role::Outline, power(60.0)),
        None
    );

    let library = [MaterialRead::Loaded(material(
        "m1",
        5,
        &[(SetKind::Cut, set(65.0, 3000, 1))],
    ))];
    let plain = linked(set(65.0, 3000, 1), 5, Overrides::default());
    let state = resolve(&plain, cut, &library, PROFILE);
    let typed = edit_number(&plain, &state, cut, Role::Main, power(70.0)).unwrap();
    assert!(typed.material.is_some(), "the link is kept");
    assert_eq!(
        typed.main,
        set(65.0, 3000, 1),
        "the stored numbers stay the material's"
    );
    assert_eq!(typed.overrides, power_override(70.0));
    let typed_read = SettingsRead::Stored(typed);
    let typed_state = resolve(&typed_read, cut, &library, PROFILE);
    let back = edit_number(&typed_read, &typed_state, cut, Role::Main, power(65.0)).unwrap();
    assert!(
        back.overrides.is_empty(),
        "equal to the material: the override goes"
    );
    let passes = edit_number(
        &typed_read,
        &typed_state,
        cut,
        Role::Main,
        TypedNumber::Passes(Passes::new(2).unwrap()),
    )
    .unwrap();
    assert_eq!(
        passes.overrides.main.fields(),
        [Field::Power, Field::Passes]
    );
    assert_eq!(
        edit_number(
            &SettingsRead::None,
            &SettingsState::NoSettings,
            cut,
            Role::Main,
            power(1.0)
        ),
        None
    );
}

#[test]
fn reset_and_own_numbers_end_the_typed_numbers_or_the_link() {
    let cut = OperationKind::Cut;
    let library = [MaterialRead::Loaded(material(
        "m1",
        5,
        &[(SetKind::Cut, set(65.0, 3000, 1))],
    ))];
    let typed = linked(set(65.0, 3000, 1), 5, power_override(70.0));
    let reset = reset_to_material(&typed).unwrap();
    assert!(reset.overrides.is_empty() && reset.material.is_some());
    assert_eq!(reset_to_material(&SettingsRead::Stored(reset)), None);
    let state = resolve(&typed, cut, &library, PROFILE);
    let own = use_own_numbers(&typed, &state, cut).unwrap();
    assert_eq!(
        own,
        StoredSettings::own(cut, set(70.0, 3000, 1), None),
        "what ran"
    );
    let own_read = SettingsRead::Stored(own);
    let own_state = resolve(&own_read, cut, &library, PROFILE);
    assert_eq!(use_own_numbers(&own_read, &own_state, cut), None);
}

fn with_interval(read: SettingsRead, hundredths: i64) -> SettingsRead {
    let SettingsRead::Stored(mut stored) = read else {
        panic!("stored");
    };
    stored.interval = Interval::from_hundredths(hundredths);
    SettingsRead::Stored(stored)
}

#[test]
fn material_paths_keep_the_stored_interval_and_a_typed_interval_writes_main() {
    // `specs/0111` criteria 19 and 23: no material holds an interval in this slice.
    let kind = OperationKind::Engrave;
    let birch = material("m1", 9, &[(SetKind::Engrave, set(35.0, 6000, 1))]);
    let library = [MaterialRead::Loaded(birch.clone())];
    let read = with_interval(linked(set(30.0, 6000, 1), 5, power_override(40.0)), 8);
    let state = resolve(&read, kind, &library, PROFILE);
    let eight = Interval::from_hundredths(8);
    assert_eq!(state.run().unwrap().interval, eight);
    assert_eq!(state.material().unwrap().interval, eight);

    let picked = use_material(&birch, kind, &read).unwrap();
    assert_eq!(picked.interval, eight, "a pick keeps the interval");
    let refreshed = refresh(&read, kind, &library, PROFILE, false).unwrap();
    assert_eq!(refreshed.interval, eight, "a refresh keeps it");
    let reset = reset_to_material(&read).unwrap();
    assert_eq!(reset.interval, eight, "Reset to material keeps it");
    let own = use_own_numbers(&read, &state, kind).unwrap();
    assert_eq!(own.interval, eight, "own numbers keep it");

    let typed = Interval::from_hundredths(15).unwrap();
    let edited = edit_interval(&read, kind, typed).unwrap();
    assert_eq!(edited.interval, Some(typed), "into main, not an override");
    assert_eq!(edited.overrides, read_overrides_of(&read));
    assert!(edited.material.is_some(), "the link stays");
    assert_eq!(
        edit_interval(&SettingsRead::Stored(edited.clone()), kind, typed),
        None,
        "unchanged"
    );
    assert_eq!(
        edit_interval(&read, OperationKind::Cut, typed),
        None,
        "a Cut has no interval"
    );
    assert_eq!(edit_interval(&SettingsRead::None, kind, typed), None);

    let cut = linked(set(60.0, 300, 1), 5, Overrides::default());
    let cut_state = resolve(&cut, OperationKind::Cut, &[], PROFILE);
    assert_eq!(cut_state.run().unwrap().interval, None);
    let absent = linked(set(30.0, 6000, 1), 5, Overrides::default());
    assert_eq!(
        resolve(&absent, kind, &[], PROFILE).run().unwrap().interval,
        Some(Interval::DEFAULT)
    );
}

#[test]
fn an_interval_typed_over_the_link_in_a_file_runs_and_a_typed_interval_replaces_it() {
    let kind = OperationKind::EngraveOutline;
    let overrides = Overrides {
        main: SetOverride {
            interval: Interval::from_hundredths(20),
            ..SetOverride::default()
        },
        ..Overrides::default()
    };
    let read = with_interval(linked(set(30.0, 6000, 1), 5, overrides), 8);
    let state = resolve(&read, kind, &[], PROFILE);
    assert_eq!(state.run().unwrap().interval, Interval::from_hundredths(20));
    assert_eq!(
        state.material().unwrap().interval,
        Interval::from_hundredths(8)
    );
    let edited = edit_interval(&read, kind, Interval::from_hundredths(9).unwrap()).unwrap();
    assert_eq!(edited.overrides, Overrides::default());
    assert_eq!(edited.interval, Interval::from_hundredths(9));
}

fn read_overrides_of(read: &SettingsRead) -> Overrides {
    let SettingsRead::Stored(stored) = read else {
        panic!("stored");
    };
    stored.overrides
}
