//! The stored form of a test grid recipe, shared by the grid register and the test record
//! (ADR 0057 §1, §6; ADR 0058 §2).

use std::collections::BTreeMap;

use curvyo_document_core::Length;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

use crate::material::SetKind;
use crate::test_id::TestId;
use crate::test_param::{Param, ParamValue};
use crate::test_recipe::{
    Axis, MAX_CELL, MAX_COUNT, MIN_CELL, MIN_COUNT, MaterialRef, Recipe, TestRecipe,
};

impl Recipe {
    /// The recipe as the plain values of `x.curvyo.laser.test_grid`.
    #[must_use]
    pub fn to_value(&self) -> Value {
        let mut json = recipe_json(&self.test);
        json.id = Some(self.id.to_string());
        json.material = self.material.as_ref().map(|m| MaterialRefJson {
            id: m.id.clone(),
            name: m.name.clone(),
        });
        serde_json::to_value(json).unwrap_or(Value::Null)
    }

    /// The recipe of stored plain values; `None` when it does not read (ADR 0057 §6: not a map,
    /// a missing or bad id, an unknown key, a value out of range, both axes on one parameter).
    #[must_use]
    pub fn from_value(value: &Value) -> Option<Self> {
        let json: RecipeJson = serde_json::from_value(value.clone()).ok()?;
        let id = TestId::parse(json.id.as_deref()?)?;
        let material = match &json.material {
            None => None,
            Some(m) if crate::record_envelope::is_record_id(&m.id) => Some(MaterialRef {
                id: m.id.clone(),
                name: m.name.clone(),
            }),
            Some(_) => return None,
        };
        Some(Self {
            id,
            test: read_recipe(&json)?,
            material,
        })
    }
}

/// The stored form of a recipe, shared by the grid register and the test record (one golden
/// shape).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecipeJson {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) id: Option<String>,
    kind: String,
    columns: AxisJson,
    rows: AxisJson,
    fixed: FixedJson,
    cell_mm: f64,
    crosshatch: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) material: Option<MaterialRefJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MaterialRefJson {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AxisJson {
    param: String,
    min: Number,
    max: Number,
    count: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixedJson {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    power_pct: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    speed_mm_min: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    passes: Option<Number>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    interval_mm: Option<Number>,
}

impl FixedJson {
    fn slot(&mut self, param: Param) -> &mut Option<Number> {
        match param {
            Param::Power => &mut self.power_pct,
            Param::Speed => &mut self.speed_mm_min,
            Param::Passes => &mut self.passes,
            Param::Interval => &mut self.interval_mm,
        }
    }
}

/// A value as written: a float for power and interval (`20.0`), an integer otherwise.
fn number_json(value: ParamValue) -> Number {
    if value.param.whole() {
        Number::from(value.steps())
    } else {
        Number::from_f64(value.number()).unwrap_or_else(|| Number::from(0))
    }
}

/// The steps of a stored `number` of `param`, exact and within its stored range.
fn number_steps(param: Param, number: &Number) -> Option<u32> {
    let steps = if param.whole() {
        i64::try_from(number.as_u64()?).ok()?
    } else {
        crate::steps::exact_steps(number.as_f64()?, param.per_unit())?
    };
    let (lo, hi) = param.stored_steps();
    u32::try_from(steps).ok().filter(|s| (lo..=hi).contains(s))
}

pub(crate) fn recipe_json(recipe: &TestRecipe) -> RecipeJson {
    let axis = |a: &Axis| AxisJson {
        param: a.param.as_str().to_owned(),
        min: number_json(a.min()),
        max: number_json(a.max()),
        count: u64::from(a.count),
    };
    let mut fixed = FixedJson::default();
    for value in recipe.fixed_values() {
        *fixed.slot(value.param) = Some(number_json(value));
    }
    RecipeJson {
        id: None,
        kind: recipe.kind.as_str().to_owned(),
        columns: axis(&recipe.columns),
        rows: axis(&recipe.rows),
        fixed,
        cell_mm: recipe.cell.as_mm(),
        crosshatch: recipe.crosshatch,
        material: None,
    }
}

/// The test recipe of `json`, `None` on any rule of ADR 0057 §1 broken (id and material are
/// not looked at).
pub(crate) fn read_recipe(json: &RecipeJson) -> Option<TestRecipe> {
    let kind = SetKind::parse(&json.kind)?;
    let axis = |a: &AxisJson| {
        let param = Param::parse(&a.param).filter(|p| p.offered(kind))?;
        let count = u8::try_from(a.count)
            .ok()
            .filter(|c| (MIN_COUNT..=MAX_COUNT).contains(c))?;
        Some(Axis {
            param,
            min: number_steps(param, &a.min)?,
            max: number_steps(param, &a.max)?,
            count,
        })
    };
    let (columns, rows) = (axis(&json.columns)?, axis(&json.rows)?);
    if columns.param == rows.param {
        return None;
    }
    let mut stored = json.fixed.clone();
    let mut fixed = BTreeMap::new();
    for param in Param::ALL {
        let slot = stored.slot(param).take();
        let shared = param.offered(kind) && param != columns.param && param != rows.param;
        match (shared, slot) {
            (true, Some(number)) => {
                fixed.insert(param, number_steps(param, &number)?);
            }
            (false, None) => {}
            _ => return None,
        }
    }
    let cell = json.cell_mm;
    if !(MIN_CELL..=MAX_CELL).contains(&cell) || (json.crosshatch && kind != SetKind::Engrave) {
        return None;
    }
    Some(TestRecipe {
        kind,
        columns,
        rows,
        fixed,
        cell: Length::from_mm(cell),
        crosshatch: json.crosshatch,
    })
}
