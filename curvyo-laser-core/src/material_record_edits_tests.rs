use super::*;
use crate::machine_profile::FeedRate;
use crate::material::{Interval, Passes, PowerPercent, Settings};
use crate::test_recipe::{RecipeDefaults, RecipeField, Side, TestRecipe};

const UUID: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";
const ENTRY: &str = "5d1e7a2b-0c3f-4e6d-8a9b-7c6d5e4f3a2b";

fn diode() -> MachineTie {
    MachineTie {
        id: "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f".to_owned(),
        name: "Diode 20 W".to_owned(),
    }
}

fn generic() -> MachineTie {
    MachineTie {
        id: "generic-laser".to_owned(),
        name: "Generic laser".to_owned(),
    }
}

fn stamp(now: u64) -> Stamp<'static> {
    Stamp {
        entry_id: ENTRY,
        now,
    }
}

fn id(text: &str) -> TestId {
    TestId::parse(text).unwrap()
}

fn recipe() -> TestRecipe {
    TestRecipe::new(&RecipeDefaults {
        max_feed: FeedRate::new(6000).unwrap(),
        set: None,
    })
}

fn test(text: &str) -> TestRecord {
    TestRecord {
        id: id(text),
        created: 10,
        machine: diode(),
        recipe: recipe(),
        lasered: None,
        best: None,
        note: String::new(),
        photo: None,
    }
}

fn material() -> Material {
    Material {
        id: UUID.to_owned(),
        modified: 100,
        name: "Birch plywood".to_owned(),
        thickness: Thickness::from_mm(3.0).unwrap(),
        lasers: Vec::new(),
        tests: vec![test("7K2QX")],
        history: Vec::new(),
    }
}

fn numbers(power: u16, speed: u64, interval: Option<u16>) -> SetNumbers {
    SetNumbers {
        settings: Settings {
            power: PowerPercent::from_tenths(power).unwrap(),
            speed: FeedRate::new(speed).unwrap(),
            passes: Passes::ONE,
        },
        interval: interval.and_then(|h| Interval::from_hundredths(i64::from(h))),
    }
}

#[test]
fn judging_writes_the_best_cell_and_the_laser_set_with_a_history_entry() {
    let mut m = material();
    m.judge(id("7K2QX"), CellIndex { column: 2, row: 1 }, stamp(500))
        .unwrap();
    assert_eq!(m.tests[0].best, Some(CellIndex { column: 2, row: 1 }));
    let set = m.set_for(&diode().id, SetKind::Cut).unwrap();
    assert_eq!(set.numbers, numbers(600, 1950, None));
    assert_eq!(set.source, SetSource::Test(id("7K2QX")));
    assert_eq!(m.lasers[0].machine, diode());
    assert_eq!(m.history.len(), 1);
    assert_eq!(m.modified, 500);
    assert_eq!(
        m.judge(id("7K2QX"), CellIndex { column: 2, row: 1 }, stamp(600)),
        Err(RecordRefusal::Unchanged),
        "a tap on the best cell does nothing (G14)"
    );
    m.judge(id("7K2QX"), CellIndex { column: 0, row: 0 }, stamp(700))
        .unwrap();
    assert_eq!(
        m.set_for(&diode().id, SetKind::Cut).unwrap().numbers,
        numbers(200, 600, None),
        "another cell moves the mark and the set"
    );
    assert_eq!(
        m.judge(id("7K2QX"), CellIndex { column: 5, row: 0 }, stamp(800)),
        Err(RecordRefusal::NoCell)
    );
    assert_eq!(
        m.judge(id("AAAAA"), CellIndex { column: 0, row: 0 }, stamp(800)),
        Err(RecordRefusal::UnknownTest)
    );
}

#[test]
fn an_engrave_judge_takes_the_interval_from_the_shared_values() {
    let mut m = material();
    m.tests[0].recipe = recipe().with_kind(
        SetKind::Engrave,
        &RecipeDefaults {
            max_feed: FeedRate::new(6000).unwrap(),
            set: None,
        },
    );
    m.judge(id("7K2QX"), CellIndex { column: 4, row: 4 }, stamp(1))
        .unwrap();
    let set = m.set_for(&diode().id, SetKind::Engrave).unwrap();
    assert_eq!(set.numbers, numbers(1000, 6000, Some(10)));
}

#[test]
fn a_typed_set_adds_its_laser_and_a_hand_edit_of_a_tested_set_becomes_typed() {
    let mut m = material();
    m.type_set(
        &generic(),
        SetKind::Engrave,
        numbers(300, 6000, Some(8)),
        stamp(1),
    )
    .unwrap();
    assert_eq!(m.lasers.len(), 1);
    assert_eq!(
        m.set_for("generic-laser", SetKind::Engrave).unwrap().source,
        SetSource::Typed
    );
    assert_eq!(
        m.type_set(
            &generic(),
            SetKind::Cut,
            numbers(300, 600, Some(8)),
            stamp(2)
        ),
        Err(RecordRefusal::WrongNumbers),
        "an interval only on Engrave"
    );
    assert_eq!(
        m.type_set(
            &generic(),
            SetKind::Engrave,
            numbers(300, 600, None),
            stamp(2)
        ),
        Err(RecordRefusal::WrongNumbers)
    );
    m.judge(id("7K2QX"), CellIndex { column: 2, row: 1 }, stamp(3))
        .unwrap();
    m.type_set(&diode(), SetKind::Cut, numbers(610, 1950, None), stamp(4))
        .unwrap();
    assert_eq!(
        m.set_for(&diode().id, SetKind::Cut).unwrap().source,
        SetSource::Typed
    );
    assert_eq!(m.lasers.len(), 2, "the order of first sets");
    m.delete_set(&diode().id, SetKind::Cut, stamp(5)).unwrap();
    assert_eq!(m.set_for(&diode().id, SetKind::Cut), None);
    assert_eq!(m.history.last().unwrap().event, HistoryEvent::Removed);
    assert_eq!(
        m.delete_set(&diode().id, SetKind::Cut, stamp(6)),
        Err(RecordRefusal::NoSet { kind: "Cut" })
    );
}

#[test]
fn limits_of_lasers_tests_and_history_refuse_the_edit() {
    let mut m = material();
    for n in 0..16_u8 {
        let machine = MachineTie {
            id: crate::record_envelope::new_record_id([n; 16]),
            name: format!("Laser {n}"),
        };
        m.type_set(&machine, SetKind::Cut, numbers(500, 600, None), stamp(1))
            .unwrap();
    }
    assert_eq!(
        m.type_set(&generic(), SetKind::Cut, numbers(500, 600, None), stamp(1)),
        Err(RecordRefusal::TooManyLasers)
    );
    let mut full = material();
    full.history = vec![m.history[0].clone(); 500];
    assert!(matches!(
        full.type_set(&diode(), SetKind::Cut, numbers(500, 600, None), stamp(1)),
        Err(RecordRefusal::FullHistory { .. })
    ));
    let mut many = material();
    many.tests = (0..200).map(|n| test(&format!("A{n:04}"))).collect();
    assert_eq!(
        many.put_test(test("ZZZZZ"), 1),
        Err(RecordRefusal::TooManyTests)
    );
}

#[test]
fn a_test_follows_its_grid_until_lasered_and_moves_without_retiring() {
    let mut m = material();
    let mut changed = test("7K2QX");
    changed.recipe = recipe()
        .with_field(
            RecipeField::Count(Side::Columns),
            "3",
            FeedRate::new(6000).unwrap(),
        )
        .unwrap();
    m.tests[0].best = Some(CellIndex { column: 4, row: 0 });
    m.put_test(changed.clone(), 200).unwrap();
    assert_eq!(m.tests[0].recipe, changed.recipe);
    assert_eq!(
        m.tests[0].best, None,
        "a best cell outside the new grid goes"
    );
    assert_eq!(
        m.put_test(changed.clone(), 300),
        Err(RecordRefusal::Unchanged)
    );
    m.tests[0].lasered = Some(250);
    let mut again = changed.clone();
    again.recipe = recipe();
    assert_eq!(m.put_test(again, 300), Err(RecordRefusal::Lasered));
    m.put_test(test("8PL4D"), 400).unwrap();
    assert_eq!(m.tests.len(), 2, "newest last");
    let moved = m.take_test(id("8PL4D"), 500).unwrap();
    assert_eq!(moved.id, id("8PL4D"));
    let gone = m.delete_test(id("7K2QX"), 600).unwrap();
    assert_eq!(gone.id, id("7K2QX"));
    assert_eq!(m.tests.len(), 0);
}

#[test]
fn note_photo_name_and_thickness_edits_raise_modified() {
    let mut m = material();
    m.set_note(id("7K2QX"), "Clean edge at 60 %.", 50).unwrap();
    assert_eq!(m.modified, 101, "a clock behind the record");
    assert_eq!(
        m.set_note(id("7K2QX"), &"x".repeat(501), 200),
        Err(RecordRefusal::LongNote)
    );
    m.set_note(id("7K2QX"), &"ä".repeat(500), 200).unwrap();
    m.set_photo(id("7K2QX"), Some(PhotoKind::Jpg), 300).unwrap();
    assert_eq!(
        m.set_photo(id("7K2QX"), Some(PhotoKind::Jpg), 300),
        Err(RecordRefusal::Unchanged)
    );
    m.rename(" Birch 3 mm ", 400).unwrap();
    assert_eq!(m.name, "Birch 3 mm");
    assert_eq!(m.rename("", 500), Err(RecordRefusal::InvalidName));
    m.set_thickness(Thickness::from_mm(4.0).unwrap(), 600)
        .unwrap();
    assert_eq!(m.modified, 600);
}
