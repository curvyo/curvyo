use super::*;
use crate::machine_profile::FeedRate;
use crate::material::{MachineTie, Passes, PowerPercent, SetKind, SetNumbers, Settings, Thickness};
use crate::material_record_edits::Stamp;

const ONE: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";
const TWO: &str = "11111111-1111-4111-8111-111111111111";

fn golden(name: &str) -> String {
    match name {
        "format2_one_laser.json" => {
            include_str!("../tests/fixtures/materials/format2_one_laser.json").to_owned()
        }
        "format2_full.json" => {
            include_str!("../tests/fixtures/materials/format2_full.json").to_owned()
        }
        "full.json" => include_str!("../tests/fixtures/materials/full.json").to_owned(),
        other => panic!("no golden {other}"),
    }
}

fn material(id: &str, name: &str) -> Material {
    Material {
        id: id.to_owned(),
        modified: 1,
        name: name.to_owned(),
        thickness: Thickness::from_mm(3.0).unwrap(),
        lasers: Vec::new(),
        tests: Vec::new(),
        history: Vec::new(),
    }
}

fn laser() -> MachineTie {
    MachineTie {
        id: "generic-laser".to_owned(),
        name: "Generic laser".to_owned(),
    }
}

fn numbers() -> SetNumbers {
    SetNumbers {
        settings: Settings {
            power: PowerPercent::from_tenths(600).unwrap(),
            speed: FeedRate::new(300).unwrap(),
            passes: Passes::ONE,
        },
        interval: None,
    }
}

#[test]
fn load_reads_format_two_and_names_the_others_once() {
    let files = vec![
        (format!("{ONE}.json"), golden("format2_one_laser.json")),
        (format!("{TWO}.json"), "{".to_owned()),
        ("notes.txt".to_owned(), "x".to_owned()),
    ];
    let library = Library::load(&files, None);
    assert_eq!(library.reads().len(), 2);
    assert_eq!(library.find(ONE).unwrap().name, "Acrylic 3 mm");
    assert!(library.find(TWO).is_none());
    assert_eq!(library.load_notices().len(), 1);
}

#[test]
fn a_format_one_record_is_skipped_with_its_notice() {
    let files = vec![(format!("{ONE}.json"), golden("full.json"))];
    let library = Library::load(&files, None);
    assert!(matches!(library.reads()[0], MaterialRead::Older { .. }));
    assert!(library.load_notices()[0].ends_with("has an older format."));
}

#[test]
fn listed_takes_the_order_file_then_names() {
    let mut library = Library::default();
    library.add(material(ONE, "Walnut")).unwrap();
    library.add(material(TWO, "acrylic")).unwrap();
    let names: Vec<&str> = library.listed().iter().map(|r| r.id()).collect();
    assert_eq!(names, [TWO, ONE]);
    let text = library.move_row(ONE, 0).unwrap();
    assert!(text.contains(ONE));
    let names: Vec<&str> = library.listed().iter().map(|r| r.id()).collect();
    assert_eq!(names, [ONE, TWO]);
}

#[test]
fn an_edit_writes_the_canonical_text_and_a_refusal_changes_nothing() {
    let mut library = Library::default();
    library.add(material(ONE, "Birch")).unwrap();
    let before = library.text(ONE).unwrap().to_owned();
    let stamp = Stamp {
        entry_id: "aaaaaaaa-0000-4000-8000-000000000001",
        now: 5,
    };
    let write = library
        .edit(ONE, |m| {
            m.type_set(&laser(), SetKind::Cut, numbers(), stamp)
        })
        .unwrap();
    let LibraryWrite::Write { text, .. } = write else {
        panic!("a write");
    };
    assert_eq!(library.text(ONE), Some(text.as_str()));
    assert_ne!(text, before);
    let refused = library.edit(ONE, |m| m.rename("Birch", 6));
    assert_eq!(refused, Err(StoreRefusal::Record(RecordRefusal::Unchanged)));
    assert_eq!(library.text(ONE), Some(text.as_str()));
}

#[test]
fn a_record_over_512_kib_is_refused() {
    let mut library = Library::default();
    library.add(material(ONE, "Birch")).unwrap();
    let before = library.text(ONE).unwrap().to_owned();
    let refused = library.edit(ONE, |m| {
        for i in 0..5_000 {
            m.history.push(crate::material_record::HistoryEntry {
                id: format!("{i:0>36}"),
                date: 1,
                machine: "generic-laser".to_owned(),
                kind: SetKind::Cut,
                event: crate::material_record::HistoryEvent::Removed,
            });
        }
        Ok(())
    });
    assert_eq!(
        refused,
        Err(StoreRefusal::TooLarge {
            name: "Birch".to_owned()
        })
    );
    assert_eq!(
        refused.unwrap_err().to_string(),
        "Could not save Birch: it is over 512 KiB."
    );
    assert_eq!(library.text(ONE), Some(before.as_str()));
}

#[test]
fn delete_returns_the_material_and_revert_brings_it_back() {
    let mut library = Library::default();
    library.add(material(ONE, "Birch")).unwrap();
    let text = library.text(ONE).unwrap().to_owned();
    let (write, gone) = library.delete(ONE).unwrap();
    assert_eq!(write, LibraryWrite::Remove { id: ONE.to_owned() });
    assert_eq!(gone.unwrap().name, "Birch");
    assert!(library.find(ONE).is_none());
    let back = library.revert(ONE, None, Some(&text), 10).unwrap();
    assert!(matches!(back.write, Some(LibraryWrite::Write { .. })));
    assert!(library.find(ONE).unwrap().modified >= 10);
    assert_eq!(
        library.revert(ONE, Some("other"), None, 11),
        Err(RevertRefused::Changed)
    );
}

#[test]
fn test_ids_and_owner_come_from_the_loaded_records() {
    let files = vec![(format!("{ONE}.json"), golden("format2_full.json"))];
    let library = Library::load(&files, None);
    let ids: Vec<String> = library.test_ids().map(|id| id.to_string()).collect();
    assert!(ids.contains(&"7K2QX".to_owned()));
    let id = TestId::parse("7K2QX").unwrap();
    assert_eq!(library.test_owner(id).unwrap().id, ONE);
}
