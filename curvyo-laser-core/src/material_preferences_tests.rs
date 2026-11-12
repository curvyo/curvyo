use super::*;

const ID: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";
const MACHINE: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

/// The golden files, compiled in (this crate reads no file, ADR 0034 §1).
fn fixture(name: &str) -> String {
    match name {
        "order.json" => include_str!("../tests/fixtures/materials/order.json"),
        "order_unknown_id.json" => {
            include_str!("../tests/fixtures/materials/order_unknown_id.json")
        }
        "test_grid_choice.json" => {
            include_str!("../tests/fixtures/materials/test_grid_choice.json")
        }
        _ => panic!("no fixture {name}"),
    }
    .to_owned()
}

#[test]
fn the_order_file_keeps_unknown_ids_and_reads_damage_as_empty() {
    let text = fixture("order.json");
    let ids = read_order(&text);
    assert_eq!(ids[0], ID);
    assert_eq!(write_order(&ids), text);
    let unknown = read_order(&fixture("order_unknown_id.json"));
    assert_eq!(unknown, ["11111111-2222-4333-8444-555555555555", ID]);
    for bad in [
        "{",
        "{\"format_version\": 2, \"order\": []}",
        "{\"order\": []}",
    ] {
        assert!(read_order(bad).is_empty(), "{bad}");
    }
}

#[test]
fn the_grid_choice_round_trips_and_damage_reads_empty() {
    let text = fixture("test_grid_choice.json");
    let grids = read_grid_choice(&text);
    assert_eq!(grids.len(), 2);
    let generic = &grids["generic-laser"];
    assert_eq!(generic.range.speed_to.get(), 6000);
    assert!(generic.labels.on);
    let k40 = &grids[MACHINE];
    assert_eq!((k40.range.columns, k40.range.rows), (6, 4));
    assert_eq!(k40.labels.kind, SetKind::Score);
    assert_eq!(write_grid_choice(&grids), text);
    for (from, to, why) in [
        ("\"kind\": \"score\"", "\"kind\": \"cut\"", "a Cut label"),
        ("\"columns\": 6", "\"columns\": 1", "one column"),
        ("\"format_version\": 1", "\"format_version\": 2", "newer"),
        ("\"passes\": 1", "\"passes\": 1, \"x\": 1", "an unknown key"),
        ("\"generic-laser\"", "\"K40\"", "a bad machine id"),
    ] {
        let damaged = text.replacen(from, to, 1);
        assert_ne!(damaged, text, "{why}");
        assert!(read_grid_choice(&damaged).is_empty(), "{why}");
    }
}

#[test]
fn at_most_100_machines_are_remembered() {
    let grid = read_grid_choice(&fixture("test_grid_choice.json"))["generic-laser"];
    let mut grids = BTreeMap::new();
    for seed in 0..=100u8 {
        remember_grid(
            &mut grids,
            &crate::record_envelope::new_record_id([seed; 16]),
            grid,
        );
    }
    assert_eq!(grids.len(), MAX_GRID_CHOICE_MACHINES);
    let newest = crate::record_envelope::new_record_id([100; 16]);
    assert!(grids.contains_key(&newest));
    assert!(!grids.contains_key(&crate::record_envelope::new_record_id([0; 16])));
    remember_grid(&mut grids, &newest, grid);
    assert_eq!(
        grids.len(),
        MAX_GRID_CHOICE_MACHINES,
        "a known machine replaces"
    );
    let text = write_grid_choice(&grids);
    assert_eq!(read_grid_choice(&text).len(), MAX_GRID_CHOICE_MACHINES);
    let mut over = grids.clone();
    over.insert("generic-laser".to_owned(), grid);
    assert!(
        read_grid_choice(&write_grid_choice(&over)).is_empty(),
        "101 machines"
    );
}
