//! The test grid recipe of record format 2 with its rules, defaults, field checks and stored form
//! (ADR 0057 §1, ADR 0058 §2, `specs/0112` criteria 3 to 6, 24).

use std::collections::BTreeMap;

use curvyo_document_core::Length;

use crate::machine_profile::FeedRate;
use crate::material::{SetKind, SetNumbers};
use crate::test_id::TestId;
use crate::test_param::{Param, ParamValue, numbers_of};

/// The fewest cells along an axis.
pub const MIN_COUNT: u8 = 2;
/// The most cells along an axis.
pub const MAX_COUNT: u8 = 12;
/// The smallest cell side in mm.
pub const MIN_CELL: f64 = 3.0;
/// The largest cell side in mm.
pub const MAX_CELL: f64 = 50.0;
/// The cell side of a new grid in mm.
const DEFAULT_CELL: f64 = 10.0;
/// The count of a new axis.
const DEFAULT_COUNT: u8 = 5;

/// One of the two axes of a grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// The columns, left to right.
    Columns,
    /// The rows, top down.
    Rows,
}

/// One axis: a parameter run from Min to Max over Count cells (Min above Max runs downward).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Axis {
    pub(crate) param: Param,
    pub(crate) min: u32,
    pub(crate) max: u32,
    pub(crate) count: u8,
}

impl Axis {
    /// The parameter.
    #[must_use]
    pub const fn param(&self) -> Param {
        self.param
    }

    /// The value of the first cell.
    #[must_use]
    pub const fn min(&self) -> ParamValue {
        ParamValue::new(self.param, self.min)
    }

    /// The value of the last cell.
    #[must_use]
    pub const fn max(&self) -> ParamValue {
        ParamValue::new(self.param, self.max)
    }

    /// The number of cells, 2 to 12.
    #[must_use]
    pub const fn count(&self) -> u8 {
        self.count
    }

    /// The value of cell `index`, rounded to the parameter's step (0.1 %, whole mm/min, whole
    /// passes, 0.01 mm), half away from zero.
    #[must_use]
    pub fn value(&self, index: u8) -> ParamValue {
        let (min, max) = (i64::from(self.min), i64::from(self.max));
        let steps = i64::from(self.count.max(2) - 1);
        let numerator = (max - min) * i64::from(index.min(self.count - 1));
        let half = steps / 2;
        let offset = if numerator >= 0 {
            (numerator + half) / steps
        } else {
            (numerator - half) / steps
        };
        // invariant: a value between min and max, both within u32.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        ParamValue::new(self.param, (min + offset) as u32)
    }

    fn defaulted(param: Param, count: u8, max_feed: FeedRate) -> Self {
        let (min, max) = param.axis_default(max_feed);
        Self {
            param,
            min,
            max,
            count,
        }
    }
}

/// The defaults a new grid and a freed parameter take (criterion 6, G7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecipeDefaults {
    /// The page machine's maximum feed.
    pub max_feed: FeedRate,
    /// The picked material's set for the page's laser of the grid's kind, if any.
    pub set: Option<SetNumbers>,
}

/// The fields of the bar a maker types into (UX 4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeField {
    /// The Count of an axis.
    Count(Side),
    /// The Min of an axis.
    Min(Side),
    /// The Max of an axis.
    Max(Side),
    /// A shared value of row 1.
    Fixed(Param),
    /// Cell size.
    Cell,
}

/// The recipe of a test as its record keeps it: everything but the id and the material.
#[derive(Debug, Clone, PartialEq)]
pub struct TestRecipe {
    pub(crate) kind: SetKind,
    pub(crate) columns: Axis,
    pub(crate) rows: Axis,
    pub(crate) fixed: BTreeMap<Param, u32>,
    pub(crate) cell: Length,
    pub(crate) crosshatch: bool,
}

impl TestRecipe {
    /// A new Cut grid: columns power 20 to 100 %, rows speed 10 % to 100 % of the maximum feed,
    /// 5 x 5, the shared passes from the defaults, cell 10 mm.
    #[must_use]
    pub fn new(defaults: &RecipeDefaults) -> Self {
        let mut recipe = Self {
            kind: SetKind::Cut,
            columns: Axis::defaulted(Param::Power, DEFAULT_COUNT, defaults.max_feed),
            rows: Axis::defaulted(Param::Speed, DEFAULT_COUNT, defaults.max_feed),
            fixed: BTreeMap::new(),
            cell: Length::from_mm(DEFAULT_CELL),
            crosshatch: false,
        };
        recipe.fill_fixed(defaults);
        recipe
    }

    /// What the cells run as.
    #[must_use]
    pub const fn kind(&self) -> SetKind {
        self.kind
    }

    /// The axis of `side`.
    #[must_use]
    pub const fn axis(&self, side: Side) -> &Axis {
        match side {
            Side::Columns => &self.columns,
            Side::Rows => &self.rows,
        }
    }

    /// The shared value of `param`, `None` when it is on an axis or not offered.
    #[must_use]
    pub fn fixed(&self, param: Param) -> Option<ParamValue> {
        self.fixed.get(&param).map(|&s| ParamValue::new(param, s))
    }

    /// The shared values in bar order.
    #[must_use]
    pub fn fixed_values(&self) -> Vec<ParamValue> {
        self.fixed
            .iter()
            .map(|(&p, &s)| ParamValue::new(p, s))
            .collect()
    }

    /// The cell side.
    #[must_use]
    pub const fn cell(&self) -> Length {
        self.cell
    }

    /// Crosshatch of the cells (Engrave only).
    #[must_use]
    pub const fn crosshatch(&self) -> bool {
        self.crosshatch
    }

    /// The numbers of the cell at `column`, `row`: its two axis values over the shared values
    /// (criterion 24).
    #[must_use]
    pub fn cell_values(&self, column: u8, row: u8) -> SetNumbers {
        let (c, r) = (self.columns.value(column), self.rows.value(row));
        numbers_of(self.kind, |param| {
            if param == c.param {
                c.steps()
            } else if param == r.param {
                r.steps()
            } else {
                // invariant: every offered parameter is on an axis or shared.
                self.fixed.get(&param).copied().unwrap_or(1)
            }
        })
    }

    /// Whether `column`, `row` names a cell of the grid.
    #[must_use]
    pub const fn has_cell(&self, column: u8, row: u8) -> bool {
        column < self.columns.count && row < self.rows.count
    }

    /// The recipe with `field` set to the typed text, or the validation text of UX section 10;
    /// nothing is clamped (criterion 5). A shared value that is not shared changes nothing.
    ///
    /// # Errors
    /// The text, at most 70 characters.
    pub fn with_field(
        &self,
        field: RecipeField,
        typed: &str,
        max_feed: FeedRate,
    ) -> Result<Self, String> {
        let mut next = self.clone();
        match field {
            RecipeField::Count(side) => {
                let count = typed_whole(typed)?;
                let count = u8::try_from(count)
                    .ok()
                    .filter(|c| (MIN_COUNT..=MAX_COUNT).contains(c))
                    .ok_or_else(|| match side {
                        Side::Columns => "Columns are 2 to 12.".to_owned(),
                        Side::Rows => "Rows are 2 to 12.".to_owned(),
                    })?;
                next.axis_mut(side).count = count;
            }
            RecipeField::Min(side) | RecipeField::Max(side) => {
                let param = self.axis(side).param;
                let steps = typed_steps(param, typed, max_feed)?;
                let axis = next.axis_mut(side);
                if matches!(field, RecipeField::Min(_)) {
                    axis.min = steps;
                } else {
                    axis.max = steps;
                }
            }
            RecipeField::Fixed(param) => {
                let steps = typed_steps(param, typed, max_feed)?;
                if let Some(slot) = next.fixed.get_mut(&param) {
                    *slot = steps;
                }
            }
            RecipeField::Cell => {
                let mm = typed_number(typed)?;
                if !(MIN_CELL..=MAX_CELL).contains(&mm) {
                    return Err("Cell size is 3 to 50 mm.".to_owned());
                }
                next.cell = Length::from_mm(mm);
            }
        }
        Ok(next)
    }

    /// The recipe with `param` on the axis `side` (criterion 4, G6): when the other axis holds it
    /// the two axes swap; else the axis takes the parameter's default range with its count, and
    /// the parameter it held becomes shared with its default. A parameter the kind does not offer
    /// changes nothing.
    #[must_use]
    pub fn with_axis_param(&self, side: Side, param: Param, defaults: &RecipeDefaults) -> Self {
        let mut next = self.clone();
        if !param.offered(self.kind) || self.axis(side).param == param {
            return next;
        }
        let other = match side {
            Side::Columns => Side::Rows,
            Side::Rows => Side::Columns,
        };
        if self.axis(other).param == param {
            std::mem::swap(&mut next.columns, &mut next.rows);
            return next;
        }
        let count = self.axis(side).count;
        *next.axis_mut(side) = Axis::defaulted(param, count, defaults.max_feed);
        next.fill_fixed(defaults);
        next
    }

    /// The recipe as a grid of `kind`: an axis on a parameter the kind does not offer takes the
    /// first shared one instead; shared values follow the kind's parameters; Crosshatch only for
    /// Engrave.
    #[must_use]
    pub fn with_kind(&self, kind: SetKind, defaults: &RecipeDefaults) -> Self {
        let mut next = self.clone();
        next.kind = kind;
        for side in [Side::Columns, Side::Rows] {
            if !next.axis(side).param.offered(kind) {
                let taken = [next.columns.param, next.rows.param];
                let free = Param::of_kind(kind).find(|p| !taken.contains(p));
                if let Some(free) = free {
                    let count = next.axis(side).count;
                    *next.axis_mut(side) = Axis::defaulted(free, count, defaults.max_feed);
                }
            }
        }
        next.crosshatch &= kind == SetKind::Engrave;
        next.fill_fixed(defaults);
        next
    }

    /// The recipe with Crosshatch `on`; only an Engrave grid takes it.
    #[must_use]
    pub fn with_crosshatch(&self, on: bool) -> Self {
        let mut next = self.clone();
        next.crosshatch = on && self.kind == SetKind::Engrave;
        next
    }

    /// The recipe whose shared values are the defaults' (a material pick, criterion 22).
    #[must_use]
    pub fn with_shared_defaults(&self, defaults: &RecipeDefaults) -> Self {
        let mut next = self.clone();
        next.fixed.clear();
        next.fill_fixed(defaults);
        next
    }

    fn axis_mut(&mut self, side: Side) -> &mut Axis {
        match side {
            Side::Columns => &mut self.columns,
            Side::Rows => &mut self.rows,
        }
    }

    /// Keeps exactly the kind's off-axis parameters shared, a new one with its default.
    fn fill_fixed(&mut self, defaults: &RecipeDefaults) {
        let axes = [self.columns.param, self.rows.param];
        let kind = self.kind;
        self.fixed
            .retain(|p, _| p.offered(kind) && !axes.contains(p));
        for param in Param::of_kind(kind).filter(|p| !axes.contains(p)) {
            self.fixed
                .entry(param)
                .or_insert_with(|| param.fixed_default(defaults.max_feed, defaults.set.as_ref()));
        }
    }
}

/// The material a grid uses (ADR 0057 §1): its record id and its name when picked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialRef {
    /// The record id.
    pub id: String,
    /// Its name at the pick, for display only.
    pub name: String,
}

/// The whole recipe of a grid object, stored atomically on its group: the test recipe with the
/// test id and the material.
#[derive(Debug, Clone, PartialEq)]
pub struct Recipe {
    /// The test id.
    pub id: TestId,
    /// What the test varies.
    pub test: TestRecipe,
    /// The picked material, `None` until picked.
    pub material: Option<MaterialRef>,
}

fn typed_number(typed: &str) -> Result<f64, String> {
    typed
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .ok_or_else(|| "Enter a number.".to_owned())
}

fn typed_whole(typed: &str) -> Result<i64, String> {
    let number = typed
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|n| n.is_finite() && n.fract() == 0.0 && n.abs() < 1.0e12)
        .ok_or_else(|| "Enter a whole number.".to_owned())?;
    // invariant: checked whole and small above.
    #[allow(clippy::cast_possible_truncation)]
    Ok(number as i64)
}

/// The steps of a typed value of `param`: whole parameters refuse fractions, the others round
/// to their step; a value out of range is refused before rounding (`specs/0111` criterion 24).
fn typed_steps(param: Param, typed: &str, max_feed: FeedRate) -> Result<u32, String> {
    let (lo, hi) = param.steps_up_to(max_feed);
    let steps = if param.whole() {
        typed_whole(typed)?
    } else {
        let number = typed_number(typed)?;
        crate::steps::typed_steps(number, param.per_unit(), f64::from(lo), f64::from(hi))
            .ok_or_else(|| param.range_text(max_feed))?
    };
    u32::try_from(steps)
        .ok()
        .filter(|s| (lo..=hi).contains(s))
        .ok_or_else(|| param.range_text(max_feed))
}

#[cfg(test)]
#[path = "test_recipe_tests.rs"]
mod tests;
