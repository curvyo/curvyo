//! The derived operations a recipe resolves into (ADR 0057 §2, `specs/0112` criteria 10, 12).

use serde_json::{Value, json};

use crate::machine_profile::FeedRate;
use crate::material::{Passes, PowerPercent, SetKind, SetNumbers, Settings};
use crate::operation_kind::OperationKind;
use crate::operation_options::OperationOptions;
use crate::operation_settings::{MaterialLink, Overrides, StoredSettings};
use crate::test_param::{Param, ParamValue};
use crate::test_recipe::{Recipe, Side};

/// Which part of a grid a derived operation runs (`op_grid_part`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GridPart {
    /// Every burned label.
    Labels,
    /// One cell.
    Cell {
        /// Its column, from 0.
        column: u8,
        /// Its row, from 0.
        row: u8,
    },
}

impl GridPart {
    /// The plain values of `op_grid_part` for the grid group whose encoded id is `grid`.
    #[must_use]
    pub fn to_value(self, grid: &str) -> Value {
        match self {
            Self::Labels => json!({ "grid": grid, "part": "labels" }),
            Self::Cell { column, row } => {
                json!({ "grid": grid, "part": "cell", "column": column, "row": row })
            }
        }
    }

    /// The grid id and part of a stored `op_grid_part`; `None` when it is not a readable map
    /// (the row is then ordinary, ADR 0057 §6).
    #[must_use]
    pub fn read(value: &Value) -> Option<(String, Self)> {
        let map = value.as_object()?;
        let grid = map.get("grid")?.as_str()?.to_owned();
        let index = |key: &str| u8::try_from(map.get(key)?.as_u64()?).ok();
        match map.get("part")?.as_str()? {
            "labels" if map.len() == 2 => Some((grid, Self::Labels)),
            "cell" if map.len() == 4 => Some((
                grid,
                Self::Cell {
                    column: index("column")?,
                    row: index("row")?,
                },
            )),
            _ => None,
        }
    }
}

/// The Engrave set of the picked material for the page's laser, which the labels operation
/// links to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelSet {
    /// The link to write.
    pub link: MaterialLink,
    /// The set's numbers.
    pub numbers: SetNumbers,
}

/// One operation a grid resolves into, as regeneration writes it.
#[derive(Debug, Clone, PartialEq)]
pub struct DerivedOperation {
    /// What it runs.
    pub part: GridPart,
    /// Its kind: Engrave for the labels, the grid's kind for a cell.
    pub kind: OperationKind,
    /// Its name ("60 % · 1950 · K7Q2M", "K7Q2M labels").
    pub name: String,
    /// Its `op_settings`.
    pub settings: StoredSettings,
    /// Its `op_options`, `None` for no register (only Engrave cells carry Crosshatch).
    pub options: Option<OperationOptions>,
}

/// The label numbers without a material set (Proposal P12): 20 %, 3000 mm/min, 1 pass.
fn own_label_numbers() -> Settings {
    // invariant: 20 % and 3000 mm/min are valid values.
    #[allow(clippy::expect_used)]
    Settings {
        power: PowerPercent::from_tenths(200).expect("valid power"),
        speed: FeedRate::new(3000).expect("valid speed"),
        passes: Passes::ONE,
    }
}

const fn kind_of(set: SetKind) -> OperationKind {
    match set {
        SetKind::Cut => OperationKind::Cut,
        SetKind::Score => OperationKind::Score,
        SetKind::Engrave => OperationKind::Engrave,
    }
}

/// A value in a row name: units only on power.
fn name_text(value: ParamValue) -> String {
    if value.param == Param::Power {
        format!("{value} %")
    } else {
        value.to_string()
    }
}

/// The derived operations of `recipe` in list order: the labels, then one per cell, top-left
/// first, row by row (UX 5). The labels link to `label_set` when given, else run own numbers.
#[must_use]
pub fn derived_operations(recipe: &Recipe, label_set: Option<&LabelSet>) -> Vec<DerivedOperation> {
    let test = &recipe.test;
    let (columns, rows) = (test.axis(Side::Columns), test.axis(Side::Rows));
    let mut out = Vec::with_capacity(1 + usize::from(columns.count()) * usize::from(rows.count()));
    let labels = match label_set {
        Some(set) => StoredSettings {
            main: set.numbers.settings,
            interval: set.numbers.interval,
            outline: None,
            material: Some(set.link.clone()),
            overrides: Overrides::default(),
        },
        None => StoredSettings::own(OperationKind::Engrave, own_label_numbers(), None),
    };
    out.push(DerivedOperation {
        part: GridPart::Labels,
        kind: OperationKind::Engrave,
        name: format!("{} labels", recipe.id),
        settings: labels,
        options: None,
    });
    let kind = kind_of(test.kind());
    let options = (test.kind() == SetKind::Engrave).then(|| OperationOptions {
        crosshatch: test.crosshatch(),
        ..OperationOptions::default()
    });
    for row in 0..rows.count() {
        for column in 0..columns.count() {
            let numbers = test.cell_values(column, row);
            let mut settings = StoredSettings::own(kind, numbers.settings, None);
            settings.interval = numbers.interval;
            out.push(DerivedOperation {
                part: GridPart::Cell { column, row },
                kind,
                name: format!(
                    "{} · {} · {}",
                    name_text(columns.value(column)),
                    name_text(rows.value(row)),
                    recipe.id
                ),
                settings,
                options,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::material::Interval;
    use crate::operation_settings::{SettingsRead, read_settings};

    fn recipe(kind: &str, count: u8) -> Recipe {
        let fixed = if kind == "engrave" {
            json!({ "passes": 1, "interval_mm": 0.08 })
        } else {
            json!({ "passes": 1 })
        };
        Recipe::from_value(&json!({
            "id": "K7Q2M",
            "kind": kind,
            "columns": { "param": "power", "min": 20.0, "max": 100.0, "count": count },
            "rows": { "param": "speed", "min": 600, "max": 6000, "count": count },
            "fixed": fixed,
            "cell_mm": 10.0,
            "crosshatch": kind == "engrave"
        }))
        .unwrap()
    }

    #[test]
    fn a_five_by_five_grid_gives_26_operations_with_their_names_and_numbers() {
        let ops = derived_operations(&recipe("cut", 5), None);
        assert_eq!(ops.len(), 26);
        assert_eq!(ops[0].name, "K7Q2M labels");
        assert_eq!(
            (ops[0].part, ops[0].kind),
            (GridPart::Labels, OperationKind::Engrave)
        );
        assert_eq!(ops[0].settings.main.power.to_string(), "20");
        assert_eq!(ops[0].settings.main.speed.get(), 3000);
        assert_eq!(ops[0].settings.material, None);
        assert_eq!(ops[1].name, "20 % · 600 · K7Q2M");
        assert_eq!(ops[1].part, GridPart::Cell { column: 0, row: 0 });
        let cell = &ops[1 + 5 + 2]; // row 1, column 2
        assert_eq!(cell.name, "60 % · 1950 · K7Q2M");
        assert_eq!(cell.kind, OperationKind::Cut);
        assert_eq!(cell.settings.main.power.to_string(), "60");
        assert_eq!(cell.settings.main.speed.get(), 1950);
        assert_eq!((cell.settings.interval, cell.options), (None, None));
        assert!(matches!(
            read_settings(Some(&cell.settings.to_value()), OperationKind::Cut),
            SettingsRead::Stored(_)
        ));
    }

    #[test]
    fn a_twelve_by_twelve_grid_gives_145_operations() {
        assert_eq!(derived_operations(&recipe("cut", 12), None).len(), 145);
    }

    #[test]
    fn engrave_cells_carry_interval_and_crosshatch_and_the_labels_link_the_set() {
        let set = LabelSet {
            link: MaterialLink {
                id: "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c".to_owned(),
                name: "Birch".to_owned(),
                outline_from: None,
                modified: 7,
            },
            numbers: SetNumbers {
                settings: own_label_numbers(),
                interval: Interval::from_hundredths(9),
            },
        };
        let ops = derived_operations(&recipe("engrave", 2), Some(&set));
        assert_eq!(ops[0].settings.material.as_ref().unwrap().name, "Birch");
        assert_eq!(ops[0].settings.interval, Interval::from_hundredths(9));
        assert_eq!(ops[1].kind, OperationKind::Engrave);
        assert_eq!(ops[1].settings.interval, Interval::from_hundredths(8));
        assert!(ops[1].options.unwrap().crosshatch);
    }

    #[test]
    fn the_part_reads_back_and_anything_else_is_ordinary() {
        for part in [GridPart::Labels, GridPart::Cell { column: 3, row: 11 }] {
            let value = part.to_value("12@4");
            assert_eq!(GridPart::read(&value), Some(("12@4".to_owned(), part)));
        }
        for bad in [
            json!("labels"),
            json!({ "grid": "12@4", "part": "cell", "column": 1 }),
            json!({ "grid": "12@4", "part": "labels", "x": 1 }),
            json!({ "grid": 4, "part": "labels" }),
            json!({ "grid": "12@4", "part": "cells" }),
        ] {
            assert_eq!(GridPart::read(&bad), None, "{bad}");
        }
    }
}
