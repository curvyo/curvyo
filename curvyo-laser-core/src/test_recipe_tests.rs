use serde_json::{Value, json};

use super::*;
use crate::material::{Interval, Passes, PowerPercent, Settings};

fn feed(n: u64) -> FeedRate {
    FeedRate::new(n).unwrap()
}

fn defaults() -> RecipeDefaults {
    RecipeDefaults {
        max_feed: feed(6000),
        set: None,
    }
}

fn texts(axis: &Axis) -> Vec<String> {
    (0..axis.count())
        .map(|i| axis.value(i).to_string())
        .collect()
}

#[test]
fn a_new_grid_follows_criterion_6_and_g7() {
    let recipe = TestRecipe::new(&defaults());
    assert_eq!(recipe.kind(), SetKind::Cut);
    let (c, r) = (recipe.axis(Side::Columns), recipe.axis(Side::Rows));
    assert_eq!((c.param(), r.param()), (Param::Power, Param::Speed));
    assert_eq!(texts(c), ["20", "40", "60", "80", "100"]);
    assert_eq!(texts(r), ["600", "1950", "3300", "4650", "6000"]);
    assert_eq!(recipe.cell().as_mm(), 10.0);
    assert!(!recipe.crosshatch());
    assert_eq!(
        recipe.fixed_values(),
        [ParamValue::new(Param::Passes, 1)],
        "the parameter on no axis is shared"
    );
    let slow = TestRecipe::new(&RecipeDefaults {
        max_feed: feed(5),
        set: None,
    });
    assert_eq!(slow.axis(Side::Rows).min().steps(), 1, "at least 1 mm/min");
}

#[test]
fn cell_values_take_the_axes_over_the_shared_values() {
    let recipe = TestRecipe::new(&defaults());
    let cell = recipe.cell_values(2, 1);
    assert_eq!(cell.settings.power.to_string(), "60");
    assert_eq!(cell.settings.speed.get(), 1950);
    assert_eq!(cell.settings.passes, Passes::ONE);
    assert_eq!(cell.interval, None, "Cut has no interval");
    let engrave = recipe.with_kind(SetKind::Engrave, &defaults());
    let cell = engrave.cell_values(0, 0);
    assert_eq!(cell.interval, Some(Interval::DEFAULT));
    assert!(recipe.has_cell(4, 4) && !recipe.has_cell(5, 0));
}

#[test]
fn values_round_to_their_step_and_min_above_max_runs_down() {
    let recipe = TestRecipe::new(&defaults());
    let recipe = recipe
        .with_field(RecipeField::Count(Side::Columns), "7", feed(6000))
        .unwrap()
        .with_field(RecipeField::Min(Side::Columns), "10", feed(6000))
        .unwrap()
        .with_field(RecipeField::Max(Side::Columns), "20", feed(6000))
        .unwrap();
    assert_eq!(recipe.axis(Side::Columns).value(1).to_string(), "11.7");
    let down = recipe
        .with_field(RecipeField::Min(Side::Columns), "100", feed(6000))
        .unwrap()
        .with_field(RecipeField::Max(Side::Columns), "10", feed(6000))
        .unwrap();
    assert_eq!(texts(down.axis(Side::Columns))[1], "85");
    assert_eq!(texts(down.axis(Side::Columns))[6], "10");
}

#[test]
fn typed_values_are_checked_with_the_ux_texts_and_never_clamped() {
    let recipe = TestRecipe::new(&defaults());
    let set = |field, typed| recipe.with_field(field, typed, feed(6000));
    assert_eq!(
        set(RecipeField::Count(Side::Columns), "13").unwrap_err(),
        "Columns are 2 to 12."
    );
    assert_eq!(
        set(RecipeField::Count(Side::Rows), "1").unwrap_err(),
        "Rows are 2 to 12."
    );
    assert_eq!(
        set(RecipeField::Count(Side::Rows), "2.5").unwrap_err(),
        "Enter a whole number."
    );
    assert_eq!(
        set(RecipeField::Min(Side::Columns), "0.5").unwrap_err(),
        "Power is 1 to 100 %."
    );
    assert_eq!(
        set(RecipeField::Max(Side::Columns), "x").unwrap_err(),
        "Enter a number."
    );
    assert_eq!(
        set(RecipeField::Max(Side::Rows), "6001").unwrap_err(),
        "Speed is 1 to 6000 mm/min."
    );
    assert_eq!(
        set(RecipeField::Fixed(Param::Passes), "21").unwrap_err(),
        "Passes are 1 to 20."
    );
    assert_eq!(
        set(RecipeField::Cell, "60").unwrap_err(),
        "Cell size is 3 to 50 mm."
    );
    assert_eq!(
        set(RecipeField::Min(Side::Columns), "60.25")
            .unwrap()
            .axis(Side::Columns)
            .min()
            .steps(),
        603,
        "power rounds to 0.1 %"
    );
    assert_eq!(
        set(RecipeField::Fixed(Param::Passes), "3")
            .unwrap()
            .fixed(Param::Passes)
            .unwrap()
            .steps(),
        3
    );
    let engrave = recipe.with_kind(SetKind::Engrave, &defaults());
    assert_eq!(
        engrave
            .with_field(RecipeField::Fixed(Param::Interval), "2.5", feed(6000))
            .unwrap_err(),
        "Interval is 0.01 to 2.00 mm."
    );
    for text in [
        "Columns are 2 to 12.",
        "Speed is 1 to 100000 mm/min.",
        "Interval is 0.01 to 2.00 mm.",
    ] {
        assert!(text.chars().count() <= 70);
    }
}

#[test]
fn choosing_the_other_axis_parameter_swaps_the_axes() {
    let recipe = TestRecipe::new(&defaults())
        .with_field(RecipeField::Count(Side::Rows), "3", feed(6000))
        .unwrap();
    let swapped = recipe.with_axis_param(Side::Columns, Param::Speed, &defaults());
    assert_eq!(swapped.axis(Side::Columns), recipe.axis(Side::Rows));
    assert_eq!(swapped.axis(Side::Rows), recipe.axis(Side::Columns));
    assert_eq!(swapped.fixed_values(), recipe.fixed_values());
}

#[test]
fn a_freed_parameter_moves_to_the_shared_values_with_its_default() {
    let recipe = TestRecipe::new(&defaults());
    let passes = recipe.with_axis_param(Side::Rows, Param::Passes, &defaults());
    let rows = passes.axis(Side::Rows);
    assert_eq!((rows.param(), rows.count()), (Param::Passes, 5));
    assert_eq!(texts(rows), ["1", "2", "3", "4", "5"]);
    assert_eq!(passes.fixed(Param::Passes), None);
    assert_eq!(passes.fixed(Param::Speed).unwrap().steps(), 3000);
    let with_set = RecipeDefaults {
        max_feed: feed(6000),
        set: Some(SetNumbers {
            settings: Settings {
                power: PowerPercent::from_tenths(450).unwrap(),
                speed: feed(1200),
                passes: Passes::new(2).unwrap(),
            },
            interval: None,
        }),
    };
    let from_set = recipe.with_axis_param(Side::Columns, Param::Passes, &with_set);
    assert_eq!(from_set.fixed(Param::Power).unwrap().to_string(), "45");
    assert_eq!(
        recipe.with_axis_param(Side::Rows, Param::Interval, &defaults()),
        recipe,
        "Cut offers no interval"
    );
}

#[test]
fn a_kind_change_keeps_the_axes_it_can_and_the_shared_values_follow() {
    let engrave = TestRecipe::new(&defaults())
        .with_kind(SetKind::Engrave, &defaults())
        .with_axis_param(Side::Rows, Param::Interval, &defaults())
        .with_crosshatch(true);
    assert!(engrave.crosshatch());
    assert_eq!(
        texts(engrave.axis(Side::Rows)),
        ["0.05", "0.10", "0.15", "0.20", "0.25"]
    );
    assert_eq!(
        engrave
            .fixed_values()
            .iter()
            .map(|v| v.param)
            .collect::<Vec<_>>(),
        [Param::Speed, Param::Passes]
    );
    let cut = engrave.with_kind(SetKind::Cut, &defaults());
    assert_eq!(cut.axis(Side::Rows).param(), Param::Speed);
    assert_eq!(cut.fixed_values(), [ParamValue::new(Param::Passes, 1)]);
    assert!(!cut.crosshatch());
    assert!(!cut.with_crosshatch(true).crosshatch());
}

fn stored() -> Value {
    json!({
        "id": "K7Q2M",
        "kind": "cut",
        "columns": { "param": "power", "min": 20.0, "max": 100.0, "count": 5 },
        "rows": { "param": "speed", "min": 600, "max": 6000, "count": 5 },
        "fixed": { "passes": 1 },
        "cell_mm": 10.0,
        "crosshatch": false,
        "material": { "id": "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c", "name": "Birch" }
    })
}

#[test]
fn the_stored_recipe_reads_and_writes_back_equal() {
    let recipe = Recipe::from_value(&stored()).unwrap();
    assert_eq!(recipe.id.as_str(), "K7Q2M");
    assert_eq!(recipe.test, TestRecipe::new(&defaults()));
    assert_eq!(recipe.material.as_ref().unwrap().name, "Birch");
    assert_eq!(recipe.to_value(), stored());
    let mut none = stored();
    none.as_object_mut().unwrap().remove("material");
    assert_eq!(Recipe::from_value(&none).unwrap().material, None);
}

#[test]
fn a_damaged_recipe_does_not_read() {
    let damage: [(&str, Value); 13] = [
        ("id", json!("k7q2m")),
        ("id", Value::Null),
        ("kind", json!("cut_out")),
        (
            "rows",
            json!({ "param": "power", "min": 20.0, "max": 100.0, "count": 5 }),
        ),
        (
            "rows",
            json!({ "param": "speed", "min": 600, "max": 6000, "count": 13 }),
        ),
        (
            "rows",
            json!({ "param": "interval", "min": 0.05, "max": 0.25, "count": 5 }),
        ),
        (
            "rows",
            json!({ "param": "speed", "min": 600.5, "max": 6000, "count": 5 }),
        ),
        (
            "columns",
            json!({ "param": "power", "min": 0.5, "max": 100.0, "count": 5 }),
        ),
        ("fixed", json!({})),
        ("fixed", json!({ "passes": 1, "interval_mm": 0.1 })),
        ("cell_mm", json!(51.0)),
        ("crosshatch", json!(true)),
        ("unknown", json!(1)),
    ];
    for (key, value) in damage {
        let mut bad = stored();
        bad.as_object_mut()
            .unwrap()
            .insert(key.to_owned(), value.clone());
        assert_eq!(Recipe::from_value(&bad), None, "{key}: {value}");
    }
    let mut no_id = stored();
    no_id.as_object_mut().unwrap().remove("id");
    assert_eq!(Recipe::from_value(&no_id), None);
    let mut bad_material = stored();
    bad_material["material"]["id"] = json!("not-a-uuid");
    assert_eq!(Recipe::from_value(&bad_material), None);
    assert_eq!(Recipe::from_value(&json!("text")), None);
}

#[test]
fn an_engrave_recipe_keeps_interval_and_crosshatch() {
    let mut value = stored();
    value["kind"] = json!("engrave");
    value["fixed"] = json!({ "passes": 1, "interval_mm": 0.08 });
    value["crosshatch"] = json!(true);
    let recipe = Recipe::from_value(&value).unwrap();
    assert_eq!(
        recipe.test.cell_values(0, 0).interval.unwrap().hundredths(),
        8
    );
    assert_eq!(recipe.to_value(), value);
}
