use super::*;
use crate::machine_profile::FeedRate;
use crate::material::{Interval, Passes, PickKind, PowerPercent, Settings};
use crate::material_record::Marks;
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
        marks: Marks::default(),
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
    m.judge(
        id("7K2QX"),
        CellIndex { column: 2, row: 1 },
        PickKind::Best,
        stamp(500),
    )
    .unwrap();
    assert_eq!(m.tests[0].marks.best, Some(CellIndex { column: 2, row: 1 }));
    let set = m.set_for(&diode().id, SetKind::Cut).unwrap();
    assert_eq!(set.numbers, numbers(600, 1950, None));
    assert_eq!(set.source, SetSource::Test(id("7K2QX")));
    assert_eq!(m.lasers[0].machine, diode());
    assert_eq!(m.history.len(), 1);
    assert_eq!(m.modified, 500);
    assert_eq!(
        m.judge(
            id("7K2QX"),
            CellIndex { column: 2, row: 1 },
            PickKind::Best,
            stamp(600)
        ),
        Err(RecordRefusal::Unchanged),
        "a tap on the best cell does nothing (G14)"
    );
    m.judge(
        id("7K2QX"),
        CellIndex { column: 0, row: 0 },
        PickKind::Best,
        stamp(700),
    )
    .unwrap();
    assert_eq!(
        m.set_for(&diode().id, SetKind::Cut).unwrap().numbers,
        numbers(200, 600, None),
        "another cell moves the mark and the set"
    );
    assert_eq!(
        m.judge(
            id("7K2QX"),
            CellIndex { column: 5, row: 0 },
            PickKind::Best,
            stamp(800)
        ),
        Err(RecordRefusal::NoCell)
    );
    assert_eq!(
        m.judge(
            id("AAAAA"),
            CellIndex { column: 0, row: 0 },
            PickKind::Best,
            stamp(800)
        ),
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
    m.judge(
        id("7K2QX"),
        CellIndex { column: 4, row: 4 },
        PickKind::Best,
        stamp(1),
    )
    .unwrap();
    let set = m.set_for(&diode().id, SetKind::Engrave).unwrap();
    assert_eq!(set.numbers, numbers(1000, 6000, Some(10)));
}

/// ADR 0069 §14: judging an Image test writes the laser's Image set and a `set` entry of kind
/// `image`, never the Engrave set.
#[test]
fn an_image_judge_writes_the_image_set_and_never_the_engrave_set() {
    let mut m = material();
    m.type_set(
        &diode(),
        SetKind::Engrave,
        numbers(300, 6000, Some(8)),
        stamp(1),
    )
    .unwrap();
    m.tests[0].recipe = recipe().with_kind(
        SetKind::Image,
        &RecipeDefaults {
            max_feed: FeedRate::new(6000).unwrap(),
            set: None,
        },
    );
    m.judge(
        id("7K2QX"),
        CellIndex { column: 4, row: 4 },
        PickKind::Best,
        stamp(2),
    )
    .unwrap();
    let image = m.set_for(&diode().id, SetKind::Image).unwrap();
    assert_eq!(image.numbers, numbers(1000, 6000, Some(10)));
    assert_eq!(image.source, SetSource::Test(id("7K2QX")));
    assert_eq!(
        m.set_for(&diode().id, SetKind::Engrave).unwrap().numbers,
        numbers(300, 6000, Some(8)),
        "the Engrave set stays"
    );
    assert_eq!(m.history.last().unwrap().kind, SetKind::Image);
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
    m.judge(
        id("7K2QX"),
        CellIndex { column: 2, row: 1 },
        PickKind::Best,
        stamp(3),
    )
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
    m.tests[0].marks.best = Some(CellIndex { column: 4, row: 0 });
    m.put_test(changed.clone(), 200).unwrap();
    assert_eq!(m.tests[0].recipe, changed.recipe);
    assert_eq!(
        m.tests[0].marks.best, None,
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

/// Criterion 25 (0112): deleting a judged test leaves the set's source naming the tombstoned id,
/// and the record still serialises and reloads (it is not damage).
#[test]
fn a_record_whose_judged_test_was_deleted_reloads_with_its_source() {
    let mut m = material();
    m.judge(
        id("7K2QX"),
        CellIndex { column: 2, row: 1 },
        PickKind::Best,
        stamp(500),
    )
    .unwrap();
    m.delete_test(id("7K2QX"), 600).unwrap();
    let text = crate::material_record_file::write_record(&m);
    let back = crate::material_record_file::read_record(&format!("{UUID}.json"), &text).unwrap();
    let set = back.set_for(&diode().id, SetKind::Cut).unwrap();
    assert_eq!(set.source, SetSource::Test(id("7K2QX")));
    assert_eq!(back, m);
}

fn at(column: u8, row: u8) -> CellIndex {
    CellIndex { column, row }
}

/// `specs/0146` criteria 2, 10: Second best, Fastest and Deepest write the laser's pick set, mark
/// one cell each (a tap elsewhere moves it) and add no history entry.
#[test]
fn a_pick_marks_one_cell_per_kind_and_writes_its_pick_set_without_history() {
    let mut m = material();
    let (cut, test) = (SetKind::Cut, id("7K2QX"));
    m.judge(test, at(2, 1), PickKind::Best, stamp(500)).unwrap();
    m.judge(test, at(3, 3), PickKind::Fastest, stamp(600))
        .unwrap();
    m.judge(test, at(3, 3), PickKind::Deepest, stamp(700))
        .unwrap();
    m.judge(test, at(1, 1), PickKind::SecondBest, stamp(800))
        .unwrap();
    assert_eq!(m.history.len(), 1, "only Best appends a set entry");
    let marks = m.tests[0].marks;
    assert_eq!(
        marks.kinds_of(at(3, 3)),
        [PickKind::Fastest, PickKind::Deepest]
    );
    assert_eq!(
        m.pick_set(&diode().id, cut, PickKind::Fastest)
            .unwrap()
            .numbers,
        numbers(800, 4650, None)
    );
    assert_eq!(
        m.set_for(&diode().id, cut).unwrap().numbers,
        numbers(600, 1950, None),
        "a pick never touches the Best set"
    );
    m.judge(test, at(0, 0), PickKind::Fastest, stamp(900))
        .unwrap();
    assert_eq!(m.tests[0].marks.fastest, Some(at(0, 0)));
    assert_eq!(m.tests[0].marks.deepest, Some(at(3, 3)), "other kinds stay");
    assert_eq!(
        m.pick_set(&diode().id, cut, PickKind::Fastest)
            .unwrap()
            .numbers,
        numbers(200, 600, None)
    );
    assert_eq!(
        m.judge(test, at(0, 0), PickKind::Fastest, stamp(1000)),
        Err(RecordRefusal::Unchanged)
    );
}

/// Criterion 3: an unmark removes the pick set with the mark when the set came from this test, and
/// turns Best's set to Typed with its numbers kept.
#[test]
fn unmarking_removes_a_pick_set_and_keeps_best_as_typed() {
    let mut m = material();
    let (cut, test) = (SetKind::Cut, id("7K2QX"));
    m.judge(test, at(2, 1), PickKind::Best, stamp(500)).unwrap();
    m.judge(test, at(3, 3), PickKind::Fastest, stamp(600))
        .unwrap();
    m.unmark(test, PickKind::Fastest, stamp(700)).unwrap();
    assert!(m.pick_set(&diode().id, cut, PickKind::Fastest).is_none());
    assert_eq!(m.tests[0].marks.fastest, None);
    m.unmark(test, PickKind::Best, stamp(800)).unwrap();
    let set = m.set_for(&diode().id, cut).unwrap();
    assert_eq!(
        (set.numbers, &set.source),
        (numbers(600, 1950, None), &SetSource::Typed)
    );
    assert_eq!(m.tests[0].marks.best, None);
    assert_eq!(
        m.unmark(test, PickKind::Deepest, stamp(900)),
        Err(RecordRefusal::Unchanged)
    );
}

/// Criterion 3 (S4): a set taken from another test of the same laser and kind stays untouched when
/// this test's mark goes.
#[test]
fn unmarking_leaves_a_set_that_came_from_another_test() {
    let mut m = material();
    m.tests.push(test("9XK2P"));
    m.judge(id("7K2QX"), at(3, 3), PickKind::Fastest, stamp(500))
        .unwrap();
    m.judge(id("9XK2P"), at(1, 1), PickKind::Fastest, stamp(600))
        .unwrap();
    m.unmark(id("7K2QX"), PickKind::Fastest, stamp(700))
        .unwrap();
    let kept = m
        .pick_set(&diode().id, SetKind::Cut, PickKind::Fastest)
        .unwrap();
    assert_eq!(kept.source, SetSource::Test(id("9XK2P")));
    assert_eq!(m.tests[0].marks.fastest, None);
    assert_eq!(m.tests[1].marks.fastest, Some(at(1, 1)));
}

/// Criterion 10: deleting the test leaves its pick sets, read back as "Test <id>".
#[test]
fn a_pick_set_survives_its_deleted_test_and_reads_back() {
    let mut m = material();
    m.judge(id("7K2QX"), at(3, 3), PickKind::Deepest, stamp(500))
        .unwrap();
    m.delete_test(id("7K2QX"), 600).unwrap();
    let text = crate::material_record_file::write_record(&m);
    let again = crate::material_record_file::read_record(&format!("{UUID}.json"), &text).unwrap();
    let kept = again
        .pick_set(&diode().id, SetKind::Cut, PickKind::Deepest)
        .unwrap();
    assert_eq!(kept.source, SetSource::Test(id("7K2QX")));
}

/// Criterion 5 and ADR 0090 decision 3: a recipe change drops the marks outside the new grid and
/// keeps the pick sets.
#[test]
fn a_smaller_recipe_drops_marks_outside_it() {
    let mut m = material();
    m.judge(id("7K2QX"), at(4, 4), PickKind::SecondBest, stamp(500))
        .unwrap();
    m.judge(id("7K2QX"), at(0, 0), PickKind::Deepest, stamp(500))
        .unwrap();
    let mut smaller = test("7K2QX");
    smaller.recipe = recipe()
        .with_field(
            RecipeField::Count(Side::Columns),
            "3",
            FeedRate::new(6000).unwrap(),
        )
        .unwrap();
    m.put_test(smaller, 600).unwrap();
    assert_eq!(m.tests[0].marks.second_best, None);
    assert_eq!(m.tests[0].marks.deepest, Some(at(0, 0)));
}
