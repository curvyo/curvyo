//! The test grid recipe with its limits, defaults and field checks (`specs/0028` criteria 1, 2,
//! 7, 9; ADR 0054 §4), and its stored form as plain values.

use curvyo_document_core::Length;
use serde_json::{Map, Value, json};

use crate::machine_profile::FeedRate;
use crate::material::{CellIndex, Passes, PowerPercent, SetKind, Settings};

/// The fewest columns or rows of a grid.
pub const MIN_CELLS: u8 = 2;
/// The most columns or rows of a grid.
pub const MAX_CELLS: u8 = 12;
/// The smallest cell side in mm.
pub const MIN_CELL_MM: f64 = 3.0;
/// The largest cell side in mm.
pub const MAX_CELL_MM: f64 = 50.0;
/// The largest gap in mm (the smallest is 0).
pub const MAX_GAP_MM: f64 = 20.0;
/// The smallest label height in mm (Proposal: the spec names only the default 3 mm).
pub const MIN_LABEL_MM: f64 = 1.0;
/// The largest label height in mm (Proposal).
pub const MAX_LABEL_MM: f64 = 20.0;

/// The fields of a grid without its kind and labels: what a material's history keeps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridRange {
    /// Columns (power), 2 to 12.
    pub columns: u8,
    /// Rows (speed), 2 to 12.
    pub rows: u8,
    /// The power of the left column.
    pub power_from: PowerPercent,
    /// The power of the right column.
    pub power_to: PowerPercent,
    /// The speed of the top row.
    pub speed_from: FeedRate,
    /// The speed of the bottom row.
    pub speed_to: FeedRate,
    /// The cell side, 3 to 50 mm.
    pub cell: Length,
    /// The gap between cells, 0 to 20 mm.
    pub gap: Length,
}

/// The label settings of a grid (Frage 186: the maker's, as in `LightBurn`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridLabels {
    /// Whether labels are made.
    pub on: bool,
    /// Engrave or Score, never Cut.
    pub kind: SetKind,
    /// The text height.
    pub height: Length,
    /// The numbers the labels run with.
    pub settings: Settings,
}

/// The whole recipe of one test grid, stored atomically on its top group.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridRecipe {
    /// What the cells run as.
    pub kind: SetKind,
    /// The cells.
    pub range: GridRange,
    /// The labels.
    pub labels: GridLabels,
}

/// The fields of the Grid block a maker types into (UX 6.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridField {
    /// Columns.
    Columns,
    /// Rows.
    Rows,
    /// Power from.
    PowerFrom,
    /// Power to.
    PowerTo,
    /// Speed from.
    SpeedFrom,
    /// Speed to.
    SpeedTo,
    /// Cell size.
    Cell,
    /// Gap.
    Gap,
    /// Label height.
    LabelHeight,
    /// Label power.
    LabelPower,
    /// Label speed.
    LabelSpeed,
    /// Label passes.
    LabelPasses,
}

/// `share` percent of `max`, whole and at least 1.
fn share_of(max: FeedRate, share: u32) -> FeedRate {
    let value = (u64::from(max.get()) * u64::from(share) + 50) / 100;
    // invariant: a share of at most 100 % of a valid feed rate, at least 1, is a valid feed rate.
    #[allow(clippy::expect_used)]
    FeedRate::new(value.max(1)).expect("within the feed range")
}

impl GridRecipe {
    /// The defaults of criteria 1 and 7 for a machine of maximum feed `max_feed`: Cut, 5 x 5,
    /// 20 to 100 %, 10 % to 100 % of the maximum feed, cell 10 mm, gap 3 mm, labels on as
    /// Engrave, 3 mm, 20 %, 50 % of the maximum feed, 1 pass.
    ///
    /// # Panics
    /// Does not panic: every default lies inside its range.
    #[must_use]
    pub fn defaults(max_feed: FeedRate) -> Self {
        // invariant: 20 % and 100 % are valid powers.
        #[allow(clippy::expect_used)]
        let power = |tenths| PowerPercent::from_tenths(tenths).expect("a valid power");
        Self {
            kind: SetKind::Cut,
            range: GridRange {
                columns: 5,
                rows: 5,
                power_from: power(200),
                power_to: power(1000),
                speed_from: share_of(max_feed, 10),
                speed_to: max_feed,
                cell: Length::from_mm(10.0),
                gap: Length::from_mm(3.0),
            },
            labels: GridLabels {
                on: true,
                kind: SetKind::Engrave,
                height: Length::from_mm(3.0),
                settings: Settings {
                    power: power(200),
                    speed: share_of(max_feed, 50),
                    passes: Passes::ONE,
                },
            },
        }
    }

    /// The power of column `column`, rounded to 0.1 % (spec S3).
    ///
    /// # Panics
    /// Does not panic: the value lies between two valid powers.
    #[must_use]
    pub fn column_power(&self, column: u8) -> PowerPercent {
        let (from, to) = (self.range.power_from.tenths(), self.range.power_to.tenths());
        let steps = i32::from(self.range.columns.max(2) - 1);
        let tenths = i32::from(from)
            + div_round((i32::from(to) - i32::from(from)) * i32::from(column), steps);
        // invariant: a value between two valid powers is a valid power.
        #[allow(
            clippy::expect_used,
            clippy::cast_sign_loss,
            clippy::cast_possible_truncation
        )]
        PowerPercent::from_tenths(tenths as u16).expect("between from and to")
    }

    /// The speed of row `row`, a whole mm/min.
    ///
    /// # Panics
    /// Does not panic: the value lies between two valid feed rates.
    #[must_use]
    pub fn row_speed(&self, row: u8) -> FeedRate {
        let (from, to) = (
            i64::from(self.range.speed_from.get()),
            i64::from(self.range.speed_to.get()),
        );
        let steps = i64::from(self.range.rows.max(2) - 1);
        let value = from + div_round_i64((to - from) * i64::from(row), steps);
        // invariant: a value between two valid feed rates is a valid feed rate.
        #[allow(clippy::expect_used, clippy::cast_sign_loss)]
        FeedRate::new(value as u64).expect("between from and to")
    }

    /// The numbers of the cell `cell` (one pass).
    #[must_use]
    pub fn cell_settings(&self, cell: CellIndex) -> Settings {
        Settings {
            power: self.column_power(cell.column),
            speed: self.row_speed(cell.row),
            passes: Passes::ONE,
        }
    }

    /// The recipe with `field` set to the typed text, or the validation text of UX 6.3 (nothing
    /// is clamped, criterion 2). Speeds are checked against `max_feed`.
    ///
    /// # Errors
    /// The chip text, at most 70 characters.
    pub fn with_field(
        &self,
        field: GridField,
        typed: &str,
        max_feed: FeedRate,
    ) -> Result<Self, String> {
        let mut next = *self;
        let number = parse_field_number(field, typed)?;
        match field {
            GridField::Columns | GridField::Rows => next.set_count(field, number)?,
            GridField::PowerFrom | GridField::PowerTo | GridField::LabelPower => {
                next.set_power(field, number)?;
            }
            GridField::SpeedFrom | GridField::SpeedTo | GridField::LabelSpeed => {
                next.set_speed(field, number, max_feed)?;
            }
            GridField::Cell => {
                next.range.cell = mm_in(number, MIN_CELL_MM, MAX_CELL_MM)
                    .ok_or_else(|| "Cell size is 3 to 50 mm.".to_owned())?;
            }
            GridField::Gap => {
                next.range.gap = mm_in(number, 0.0, MAX_GAP_MM)
                    .ok_or_else(|| "Gap is 0 to 20 mm.".to_owned())?;
            }
            GridField::LabelHeight => {
                next.labels.height = mm_in(number, MIN_LABEL_MM, MAX_LABEL_MM)
                    .ok_or_else(|| "Label height is 1 to 20 mm.".to_owned())?;
            }
            GridField::LabelPasses => {
                next.labels.settings.passes = whole_in(number, 1, 20)
                    .and_then(Passes::new)
                    .ok_or_else(|| "Passes are 1 to 20.".to_owned())?;
            }
        }
        Ok(next)
    }

    fn set_count(&mut self, field: GridField, number: f64) -> Result<(), String> {
        let count = whole_in(number, MIN_CELLS.into(), MAX_CELLS.into()).ok_or_else(|| {
            if field == GridField::Columns {
                "Columns are 2 to 12.".to_owned()
            } else {
                "Rows are 2 to 12.".to_owned()
            }
        })?;
        // invariant: checked within 2 to 12.
        #[allow(clippy::cast_possible_truncation)]
        let count = count as u8;
        if field == GridField::Columns {
            self.range.columns = count;
        } else {
            self.range.rows = count;
        }
        Ok(())
    }

    fn set_power(&mut self, field: GridField, number: f64) -> Result<(), String> {
        let power = (1.0..=100.0)
            .contains(&number)
            .then(|| PowerPercent::rounded(number))
            .flatten()
            .ok_or_else(|| "Power is 1 to 100 %.".to_owned())?;
        match field {
            GridField::PowerFrom => self.range.power_from = power,
            GridField::PowerTo => self.range.power_to = power,
            _ => self.labels.settings.power = power,
        }
        Ok(())
    }

    fn set_speed(
        &mut self,
        field: GridField,
        number: f64,
        max_feed: FeedRate,
    ) -> Result<(), String> {
        let speed = whole_in(number, 1, max_feed.get().into())
            .and_then(FeedRate::new)
            .ok_or_else(|| format!("Speed is 1 to {} mm/min.", max_feed.get()))?;
        match field {
            GridField::SpeedFrom => self.range.speed_from = speed,
            GridField::SpeedTo => self.range.speed_to = speed,
            _ => self.labels.settings.speed = speed,
        }
        Ok(())
    }

    /// The recipe as the plain values of `x.curvyo.laser.test_grid` (ADR 0054 §4).
    #[must_use]
    pub fn to_value(&self) -> Value {
        let mut map = range_value(&self.range);
        map.insert("kind".to_owned(), json!(self.kind.as_str()));
        map.insert(
            "labels".to_owned(),
            json!({
                "on": self.labels.on,
                "kind": self.labels.kind.as_str(),
                "height_mm": self.labels.height.as_mm(),
                "power_pct": self.labels.settings.power.percent(),
                "speed_mm_min": self.labels.settings.speed.get(),
                "passes": self.labels.settings.passes.get(),
            }),
        );
        Value::Object(map)
    }

    /// The recipe of stored plain values; `None` when it does not read (an unknown key, a missing
    /// or out-of-range value, a Cut label kind): the group is then no grid and its tags are inert.
    #[must_use]
    pub fn from_value(value: &Value) -> Option<Self> {
        let map = value.as_object()?;
        let known = [
            "kind",
            "columns",
            "rows",
            "power_from_pct",
            "power_to_pct",
            "speed_from_mm_min",
            "speed_to_mm_min",
            "cell_mm",
            "gap_mm",
            "labels",
        ];
        if map.keys().any(|key| !known.contains(&key.as_str())) {
            return None;
        }
        let kind = SetKind::parse(map.get("kind")?.as_str()?)?;
        let range = range_of(map)?;
        let labels = map.get("labels")?.as_object()?;
        let label_keys = [
            "on",
            "kind",
            "height_mm",
            "power_pct",
            "speed_mm_min",
            "passes",
        ];
        if labels.len() != label_keys.len()
            || labels.keys().any(|k| !label_keys.contains(&k.as_str()))
        {
            return None;
        }
        let label_kind = SetKind::parse(labels.get("kind")?.as_str()?)?;
        if label_kind == SetKind::Cut {
            return None;
        }
        Some(Self {
            kind,
            range,
            labels: GridLabels {
                on: labels.get("on")?.as_bool()?,
                kind: label_kind,
                height: mm_in(
                    labels.get("height_mm")?.as_f64()?,
                    MIN_LABEL_MM,
                    MAX_LABEL_MM,
                )?,
                settings: Settings {
                    power: PowerPercent::from_percent(labels.get("power_pct")?.as_f64()?)?,
                    speed: FeedRate::new(labels.get("speed_mm_min")?.as_u64()?)?,
                    passes: Passes::new(labels.get("passes")?.as_u64()?)?,
                },
            },
        })
    }
}

/// The grid fields of `range` as plain values (also the `grid` of a material's history entry).
#[must_use]
pub fn range_value(range: &GridRange) -> Map<String, Value> {
    let value = json!({
        "columns": range.columns,
        "rows": range.rows,
        "power_from_pct": range.power_from.percent(),
        "power_to_pct": range.power_to.percent(),
        "speed_from_mm_min": range.speed_from.get(),
        "speed_to_mm_min": range.speed_to.get(),
        "cell_mm": range.cell.as_mm(),
        "gap_mm": range.gap.as_mm(),
    });
    match value {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

/// The grid fields of a map holding at least the keys of [`range_value`]; `None` when one is
/// missing or out of range. Speeds are not checked against a machine (the profile can change).
#[must_use]
pub fn range_of(map: &Map<String, Value>) -> Option<GridRange> {
    let count = |key: &str| {
        let n = map.get(key)?.as_u64()?;
        // invariant: checked within 2 to 12 before the cast.
        #[allow(clippy::cast_possible_truncation)]
        (u64::from(MIN_CELLS)..=u64::from(MAX_CELLS))
            .contains(&n)
            .then_some(n as u8)
    };
    let power = |key: &str| PowerPercent::from_percent(map.get(key)?.as_f64()?);
    let speed = |key: &str| FeedRate::new(map.get(key)?.as_u64()?);
    Some(GridRange {
        columns: count("columns")?,
        rows: count("rows")?,
        power_from: power("power_from_pct")?,
        power_to: power("power_to_pct")?,
        speed_from: speed("speed_from_mm_min")?,
        speed_to: speed("speed_to_mm_min")?,
        cell: mm_in(map.get("cell_mm")?.as_f64()?, MIN_CELL_MM, MAX_CELL_MM)?,
        gap: mm_in(map.get("gap_mm")?.as_f64()?, 0.0, MAX_GAP_MM)?,
    })
}

/// The Worked mark `{ column, row }` of `recipe`, `None` when it does not read or names no cell.
#[must_use]
pub fn read_mark(value: &Value, recipe: &GridRecipe) -> Option<CellIndex> {
    let map = value.as_object()?;
    if map.len() != 2 {
        return None;
    }
    let column = u8::try_from(map.get("column")?.as_u64()?).ok()?;
    let row = u8::try_from(map.get("row")?.as_u64()?).ok()?;
    (column < recipe.range.columns && row < recipe.range.rows).then_some(CellIndex { column, row })
}

/// The stored form of a Worked mark.
#[must_use]
pub fn mark_value(cell: CellIndex) -> Value {
    json!({ "column": cell.column, "row": cell.row })
}

/// The typed text as a finite number, a whole one for the count and speed fields.
fn parse_field_number(field: GridField, typed: &str) -> Result<f64, String> {
    let number = typed
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .ok_or_else(|| {
            if is_whole_field(field) {
                "Enter a whole number.".to_owned()
            } else {
                "Enter a number.".to_owned()
            }
        })?;
    if is_whole_field(field) && number.fract() != 0.0 {
        return Err("Enter a whole number.".to_owned());
    }
    Ok(number)
}

fn is_whole_field(field: GridField) -> bool {
    matches!(
        field,
        GridField::Columns
            | GridField::Rows
            | GridField::SpeedFrom
            | GridField::SpeedTo
            | GridField::LabelSpeed
            | GridField::LabelPasses
    )
}

fn whole_in(number: f64, min: u64, max: u64) -> Option<u64> {
    #[allow(clippy::cast_precision_loss)] // the bounds are far below 2^53
    let inside = number.fract() == 0.0 && number >= min as f64 && number <= max as f64;
    // invariant: a whole number within a u64 range.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    inside.then_some(number as u64)
}

fn mm_in(number: f64, min: f64, max: f64) -> Option<Length> {
    (number.is_finite() && number >= min && number <= max).then_some(Length::from_mm(number))
}

/// `numerator / denominator` rounded half away from zero; `denominator` is positive.
fn div_round(numerator: i32, denominator: i32) -> i32 {
    let half = denominator / 2;
    if numerator >= 0 {
        (numerator + half) / denominator
    } else {
        (numerator - half) / denominator
    }
}

fn div_round_i64(numerator: i64, denominator: i64) -> i64 {
    let half = denominator / 2;
    if numerator >= 0 {
        (numerator + half) / denominator
    } else {
        (numerator - half) / denominator
    }
}

#[cfg(test)]
#[path = "test_grid_tests.rs"]
mod tests;
