use std::collections::BTreeMap;

use super::*;
use crate::material::{
    Interval, MachineTie, Passes, PickKind, PowerPercent, SetNumbers, Thickness,
};
use crate::material_record::{LaserSets, Set, SetSource};
use crate::operation_settings::SetOverride;
use crate::operation_settings_edit::{TypedNumber, edit_interval, edit_number};

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
        lasers: vec![LaserSets {
            machine: MachineTie {
                id: PROFILE.to_owned(),
                name: "Generic laser".to_owned(),
            },
            sets: sets
                .iter()
                .map(|&(kind, settings)| {
                    (
                        kind,
                        Set {
                            numbers: SetNumbers {
                                settings,
                                interval: None,
                            },
                            source: SetSource::Typed,
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>(),
            picks: BTreeMap::new(),
        }],
        tests: Vec::new(),
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
            pick: PickKind::Best,
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
    let stored = use_material(&both, PROFILE, OperationKind::EngraveOutline, &absent()).unwrap();
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
        PROFILE,
        OperationKind::EngraveOutline,
        &absent(),
    )
    .unwrap();
    assert_eq!(stored.outline, Some(set(30.0, 6000, 1)));
    assert_eq!(
        stored.material.unwrap().outline_from,
        Some(SetKind::Engrave)
    );
    assert_eq!(
        use_material(&engrave_only, PROFILE, OperationKind::Cut, &absent()),
        Err(SetKind::Cut),
        "skipped and named"
    );
    let cut = use_material(
        &material("m3", 1, &[(SetKind::Cut, set(60.0, 300, 2))]),
        PROFILE,
        OperationKind::CutOut,
        &absent(),
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
        Some(StoredReason::NoSet(SetKind::Cut, PickKind::Best)),
        "a laser without sets in a shared material (ADR 0058 §7)"
    );
    let older = [MaterialRead::Older {
        id: "m1".to_owned(),
    }];
    assert_eq!(
        reason(&older, PROFILE, OperationKind::Cut),
        Some(StoredReason::Older)
    );
    assert_eq!(
        reason(&library, PROFILE, OperationKind::Score),
        Some(StoredReason::NoSet(SetKind::Score, PickKind::Best))
    );
    assert_eq!(
        StoredReason::NoSet(SetKind::Score, PickKind::Best).text(),
        "Material has no Score set for this laser, using stored numbers"
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
        resolve(&absent(), OperationKind::Cut, &[], "p"),
        SettingsState::Own(RunSettings {
            main: crate::DefaultNumbers::BUILT_IN.settings(),
            outline: None,
            interval: None,
        }),
        "an absent register runs the copy's pair (specs/0150 criterion 15)"
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
            &SettingsRead::Damaged(SettingsDamage::BadSet),
            &SettingsState::Damaged(SettingsDamage::BadSet),
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

    let picked = use_material(&birch, PROFILE, kind, &read).unwrap();
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
    assert_eq!(
        edit_interval(&SettingsRead::Damaged(SettingsDamage::BadSet), kind, typed),
        None
    );

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

#[test]
fn a_linked_engrave_without_an_interval_takes_the_one_of_its_laser_set() {
    let mut m = material("m1", 7, &[(SetKind::Engrave, set(30.0, 6000, 1))]);
    let interval = Interval::from_mm(0.08).unwrap();
    m.lasers[0]
        .sets
        .get_mut(&SetKind::Engrave)
        .unwrap()
        .numbers
        .interval = Some(interval);
    let stored = use_material(&m, PROFILE, OperationKind::Engrave, &absent()).unwrap();
    assert_eq!(stored.interval, Some(interval));
    let own = SettingsRead::Stored(StoredSettings {
        interval: Interval::from_mm(0.3),
        ..stored.clone()
    });
    let kept = use_material(&m, PROFILE, OperationKind::Engrave, &own).unwrap();
    assert_eq!(
        kept.interval,
        Interval::from_mm(0.3),
        "an own interval stays (0111 c23)"
    );
    let other = use_material(&m, "other", OperationKind::Engrave, &absent());
    assert_eq!(
        other,
        Err(SetKind::Engrave),
        "the set of another laser never runs"
    );
}

/// A material with an Engrave set at 30 % and, if `image`, an Image set at 45 % and 0.08 mm.
fn engrave_and_image(modified: u64, image: bool) -> Material {
    let mut sets = vec![(SetKind::Engrave, set(30.0, 6000, 1))];
    if image {
        sets.push((SetKind::Image, set(45.0, 9000, 2)));
    }
    let mut m = material("m1", modified, &sets);
    for (kind, mm) in [(SetKind::Engrave, 0.1), (SetKind::Image, 0.08)] {
        if let Some(set) = m.lasers[0].sets.get_mut(&kind) {
            set.numbers.interval = Interval::from_mm(mm);
        }
    }
    m
}

#[test]
fn an_image_operation_reads_the_image_set_and_never_the_engrave_set() {
    assert_eq!(main_set(OperationKind::Image), SetKind::Image);
    let m = engrave_and_image(7, true);
    let stored = use_material(&m, PROFILE, OperationKind::Image, &absent()).unwrap();
    assert_eq!(stored.main, set(45.0, 9000, 2));
    assert_eq!(
        stored.interval,
        Interval::from_mm(0.08),
        "the Image set's interval"
    );
    let library = [MaterialRead::Loaded(m.clone())];
    let read = SettingsRead::Stored(stored.clone());
    let SettingsState::Live { material, .. } =
        resolve(&read, OperationKind::Image, &library, PROFILE)
    else {
        panic!("live")
    };
    assert_eq!(material.main, set(45.0, 9000, 2));
    let engrave = use_material(&m, PROFILE, OperationKind::Engrave, &absent()).unwrap();
    assert_eq!(
        engrave.main,
        set(30.0, 6000, 1),
        "Engrave keeps the Engrave set"
    );
    let without = engrave_and_image(7, false);
    assert_eq!(
        use_material(&without, PROFILE, OperationKind::Image, &absent()),
        Err(SetKind::Image),
        "no fallback to the Engrave set: Use material skips"
    );
    let library = [MaterialRead::Loaded(without)];
    let state = resolve(&read, OperationKind::Image, &library, PROFILE);
    let SettingsState::Stored { reason, .. } = state else {
        panic!("stored: {state:?}")
    };
    assert_eq!(reason, StoredReason::NoSet(SetKind::Image, PickKind::Best));
    assert_eq!(
        reason.text(),
        "Material has no Image set for this laser, using stored numbers"
    );
    let own = SettingsRead::Stored(StoredSettings::own(
        OperationKind::Image,
        set(20.0, 3000, 1),
        None,
    ));
    assert!(matches!(
        resolve(&own, OperationKind::Image, &library, PROFILE),
        SettingsState::Own(_)
    ));
}

#[test]
fn refresh_of_an_image_operation_follows_the_image_set() {
    let old = engrave_and_image(5, true);
    let stored = use_material(&old, PROFILE, OperationKind::Image, &absent()).unwrap();
    let read = SettingsRead::Stored(stored);
    let mut edited = engrave_and_image(9, true);
    edited.lasers[0]
        .sets
        .get_mut(&SetKind::Image)
        .unwrap()
        .numbers
        .settings = set(55.0, 9000, 2);
    let library = [MaterialRead::Loaded(edited)];
    for guard in [false, true] {
        let fresh = refresh(&read, OperationKind::Image, &library, PROFILE, guard).unwrap();
        assert_eq!(fresh.main, set(55.0, 9000, 2), "guard {guard}");
    }
    let only_engrave = [MaterialRead::Loaded(engrave_and_image(9, false))];
    assert_eq!(
        refresh(&read, OperationKind::Image, &only_engrave, PROFILE, false),
        None,
        "no Image set: the stored numbers stay"
    );
}

/// What an operation without `op_settings` reads (`specs/0150` criterion 15).
fn absent() -> SettingsRead {
    crate::operation_settings::read_settings(
        None,
        OperationKind::Cut,
        crate::DefaultNumbers::BUILT_IN,
    )
}

/// `material()` plus a laser pick set of `kind` for `pick`.
fn with_pick(mut m: Material, kind: SetKind, pick: PickKind, settings: Settings) -> Material {
    m.lasers[0].picks.insert(
        (kind, pick),
        Set {
            numbers: SetNumbers {
                settings,
                interval: kind
                    .has_interval()
                    .then(|| Interval::from_hundredths(10).unwrap()),
            },
            source: SetSource::Test(crate::test_id::TestId::parse("7K2QX").unwrap()),
        },
    );
    m
}

/// `specs/0146` criteria 13, 15: Use pick links the pick's numbers; a link without a pick follows
/// Best, and a pick the material lacks reads the stored state with the kind's word.
#[test]
fn use_pick_takes_the_pick_set_and_a_missing_pick_names_its_word() {
    let m = with_pick(
        material("m1", 7, &[(SetKind::Cut, set(60.0, 1950, 1))]),
        SetKind::Cut,
        PickKind::Fastest,
        set(80.0, 4650, 1),
    );
    let fast = use_pick(
        &m,
        PROFILE,
        OperationKind::Cut,
        PickKind::Fastest,
        &absent(),
    )
    .unwrap();
    assert_eq!(fast.main, set(80.0, 4650, 1));
    assert_eq!(fast.material.as_ref().unwrap().pick, PickKind::Fastest);
    let best = use_material(&m, PROFILE, OperationKind::Cut, &absent()).unwrap();
    assert_eq!(best.main, set(60.0, 1950, 1));
    assert_eq!(best.material.unwrap().pick, PickKind::Best);
    assert_eq!(
        use_pick(
            &m,
            PROFILE,
            OperationKind::Cut,
            PickKind::Deepest,
            &absent()
        ),
        Err(SetKind::Cut)
    );
    let library = [MaterialRead::Loaded(m)];
    let mut gone = fast.clone();
    gone.material.as_mut().unwrap().pick = PickKind::Deepest;
    let state = resolve(
        &SettingsRead::Stored(gone),
        OperationKind::Cut,
        &library,
        PROFILE,
    );
    let SettingsState::Stored { reason, .. } = state else {
        panic!("stored");
    };
    assert_eq!(reason, StoredReason::NoSet(SetKind::Cut, PickKind::Deepest));
    assert_eq!(
        reason.text(),
        "Material has no Deepest set for this laser, using stored numbers."
    );
}

/// Criterion 14: a linked pick follows the material when it is re-judged, and typed numbers stay.
#[test]
fn a_linked_pick_follows_the_material() {
    let m = with_pick(
        material("m1", 7, &[(SetKind::Cut, set(60.0, 1950, 1))]),
        SetKind::Cut,
        PickKind::Fastest,
        set(80.0, 4650, 1),
    );
    let fast = use_pick(
        &m,
        PROFILE,
        OperationKind::Cut,
        PickKind::Fastest,
        &absent(),
    )
    .unwrap();
    let read = SettingsRead::Stored(fast);
    let mut moved = with_pick(
        material("m1", 9, &[(SetKind::Cut, set(60.0, 1950, 1))]),
        SetKind::Cut,
        PickKind::Fastest,
        set(90.0, 5000, 1),
    );
    moved.name = "Birch".to_owned();
    let library = [MaterialRead::Loaded(moved)];
    let fresh = refresh(&read, OperationKind::Cut, &library, PROFILE, true).unwrap();
    assert_eq!(fresh.main, set(90.0, 5000, 1));
    assert_eq!(fresh.material.unwrap().pick, PickKind::Fastest);
}

/// Criterion 13: Engrave + Outline takes the Score set of the pick for the outline, else the
/// Engrave set of that pick.
#[test]
fn engrave_outline_follows_the_pick_for_both_parts() {
    let engrave = with_pick(
        material("m1", 7, &[(SetKind::Engrave, set(30.0, 6000, 1))]),
        SetKind::Engrave,
        PickKind::Fastest,
        set(40.0, 9000, 1),
    );
    let engrave_only = use_pick(
        &engrave,
        PROFILE,
        OperationKind::EngraveOutline,
        PickKind::Fastest,
        &absent(),
    )
    .unwrap();
    assert_eq!(engrave_only.outline, Some(set(40.0, 9000, 1)));
    assert_eq!(
        engrave_only.material.unwrap().outline_from,
        Some(SetKind::Engrave)
    );
    let with_score = with_pick(
        engrave,
        SetKind::Score,
        PickKind::Fastest,
        set(20.0, 12000, 1),
    );
    let both = use_pick(
        &with_score,
        PROFILE,
        OperationKind::EngraveOutline,
        PickKind::Fastest,
        &absent(),
    )
    .unwrap();
    assert_eq!(both.outline, Some(set(20.0, 12000, 1)));
    assert_eq!(both.main, set(40.0, 9000, 1));
}
