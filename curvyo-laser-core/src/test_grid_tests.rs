use super::*;

fn feed(n: u64) -> FeedRate {
    FeedRate::new(n).unwrap()
}

#[test]
fn the_defaults_follow_criteria_1_and_7_for_the_machine() {
    let recipe = GridRecipe::defaults(feed(6000));
    assert_eq!(recipe.kind, SetKind::Cut);
    assert_eq!((recipe.range.columns, recipe.range.rows), (5, 5));
    assert_eq!(recipe.range.power_from.to_string(), "20");
    assert_eq!(recipe.range.power_to.to_string(), "100");
    assert_eq!(recipe.range.speed_from.get(), 600);
    assert_eq!(recipe.range.speed_to.get(), 6000);
    assert_eq!(recipe.range.cell.as_mm(), 10.0);
    assert_eq!(recipe.range.gap.as_mm(), 3.0);
    assert!(recipe.labels.on);
    assert_eq!(recipe.labels.kind, SetKind::Engrave);
    assert_eq!(recipe.labels.height.as_mm(), 3.0);
    assert_eq!(recipe.labels.settings.power.to_string(), "20");
    assert_eq!(
        recipe.labels.settings.speed.get(),
        3000,
        "50 % of the max feed"
    );
    assert_eq!(recipe.labels.settings.passes, Passes::ONE);
    let slow = GridRecipe::defaults(feed(5));
    assert_eq!(slow.range.speed_from.get(), 1, "at least 1 mm/min");
}

#[test]
fn cell_values_round_to_a_tenth_and_whole_mm_per_min() {
    let mut recipe = GridRecipe::defaults(feed(6000));
    assert_eq!(
        (0..5)
            .map(|c| recipe.column_power(c).to_string())
            .collect::<Vec<_>>(),
        ["20", "40", "60", "80", "100"]
    );
    assert_eq!(recipe.row_speed(0).get(), 600);
    assert_eq!(recipe.row_speed(4).get(), 6000);
    assert_eq!(recipe.row_speed(1).get(), 1950);
    recipe.range.columns = 7;
    recipe.range.power_from = PowerPercent::from_tenths(100).unwrap();
    recipe.range.power_to = PowerPercent::from_tenths(200).unwrap();
    assert_eq!(
        recipe.column_power(1).to_string(),
        "11.7",
        "10 + 10/6 rounded"
    );
    recipe.range.power_from = PowerPercent::from_tenths(1000).unwrap();
    recipe.range.power_to = PowerPercent::from_tenths(100).unwrap();
    assert_eq!(
        recipe.column_power(6).to_string(),
        "10",
        "From above To runs down"
    );
    assert_eq!(recipe.column_power(1).to_string(), "85");
    let cell = recipe.cell_settings(CellIndex { column: 0, row: 4 });
    assert_eq!((cell.speed.get(), cell.passes.get()), (6000, 1));
}

#[test]
fn a_typed_field_is_checked_with_the_ux_texts_and_never_clamped() {
    let recipe = GridRecipe::defaults(feed(6000));
    let max = feed(6000);
    let set = |field, typed| recipe.with_field(field, typed, max);
    assert_eq!(set(GridField::Columns, "6").unwrap().range.columns, 6);
    assert_eq!(
        set(GridField::Columns, "13").unwrap_err(),
        "Columns are 2 to 12."
    );
    assert_eq!(set(GridField::Rows, "1").unwrap_err(), "Rows are 2 to 12.");
    assert_eq!(
        set(GridField::Rows, "2.5").unwrap_err(),
        "Enter a whole number."
    );
    assert_eq!(
        set(GridField::Rows, "x").unwrap_err(),
        "Enter a whole number."
    );
    assert_eq!(set(GridField::Cell, "x").unwrap_err(), "Enter a number.");
    assert_eq!(
        set(GridField::PowerFrom, "0.5").unwrap_err(),
        "Power is 1 to 100 %."
    );
    assert_eq!(
        set(GridField::PowerTo, "100.5").unwrap_err(),
        "Power is 1 to 100 %."
    );
    assert_eq!(
        set(GridField::PowerFrom, "33.33")
            .unwrap()
            .range
            .power_from
            .to_string(),
        "33.3"
    );
    assert_eq!(
        set(GridField::SpeedTo, "6001").unwrap_err(),
        "Speed is 1 to 6000 mm/min."
    );
    assert_eq!(
        set(GridField::SpeedFrom, "0").unwrap_err(),
        "Speed is 1 to 6000 mm/min."
    );
    assert_eq!(
        set(GridField::Cell, "2.9").unwrap_err(),
        "Cell size is 3 to 50 mm."
    );
    assert_eq!(
        set(GridField::Cell, "12.5").unwrap().range.cell.as_mm(),
        12.5
    );
    assert_eq!(set(GridField::Gap, "0").unwrap().range.gap.as_mm(), 0.0);
    assert_eq!(set(GridField::Gap, "21").unwrap_err(), "Gap is 0 to 20 mm.");
    assert_eq!(
        set(GridField::LabelPasses, "21").unwrap_err(),
        "Passes are 1 to 20."
    );
    assert_eq!(
        set(GridField::LabelSpeed, "1200")
            .unwrap()
            .labels
            .settings
            .speed
            .get(),
        1200
    );
    for text in ["Columns are 2 to 12.", "Speed is 1 to 100000 mm/min."] {
        assert!(text.chars().count() <= 70);
    }
}

#[test]
fn the_recipe_round_trips_its_plain_values_and_refuses_damage() {
    let recipe = GridRecipe::defaults(feed(6000));
    let value = recipe.to_value();
    assert_eq!(value["kind"], "cut");
    assert_eq!(value["power_from_pct"], 20.0);
    assert_eq!(value["speed_to_mm_min"], 6000);
    assert_eq!(value["labels"]["kind"], "engrave");
    assert_eq!(GridRecipe::from_value(&value), Some(recipe));
    let damaged = |edit: &dyn Fn(&mut Value)| {
        let mut copy = value.clone();
        edit(&mut copy);
        GridRecipe::from_value(&copy)
    };
    assert_eq!(damaged(&|v| v["columns"] = json!(13)), None);
    assert_eq!(damaged(&|v| v["extra"] = json!(1)), None, "an unknown key");
    assert_eq!(damaged(&|v| v["labels"]["kind"] = json!("cut")), None);
    assert_eq!(damaged(&|v| v["power_to_pct"] = json!(100.5)), None);
    assert_eq!(damaged(&|v| v["kind"] = json!("cut_out")), None);
    assert_eq!(GridRecipe::from_value(&json!("grid")), None);
    assert!(damaged(&|v| v["speed_to_mm_min"] = json!(90_000)).is_some());
}

#[test]
fn a_mark_reads_only_inside_the_grid() {
    let recipe = GridRecipe::defaults(feed(6000));
    let cell = CellIndex { column: 2, row: 4 };
    assert_eq!(read_mark(&mark_value(cell), &recipe), Some(cell));
    assert_eq!(read_mark(&json!({"column": 5, "row": 0}), &recipe), None);
    assert_eq!(read_mark(&json!({"column": 1}), &recipe), None);
    assert_eq!(read_mark(&json!([1, 2]), &recipe), None);
}
