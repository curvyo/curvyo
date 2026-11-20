//! The stored form of a test grid recipe, shared by the grid register and the test record
//! (ADR 0057 §1, §6; ADR 0058 §2; ADR 0069 §8).

use std::collections::BTreeMap;

use curvyo_document_core::Length;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

use crate::material::SetKind;
use crate::operation_image_options::Dither;
use crate::test_id::TestId;
use crate::test_param::{Param, ParamValue};
use crate::test_recipe::{
    Axis, MAX_CELL, MAX_COUNT, MIN_CELL, MIN_COUNT, MaterialRef, PictureSource, Recipe, TestRecipe,
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
    dither: Option<String>,
    /// The picture an Image grid engraves (`specs/0135` criterion 51): absent the Ramp,
    /// `"selected"` the selected picture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    picture: Option<String>,
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
    /// The slot of a shared `param`; Strength is never shared.
    fn slot(&mut self, param: Param) -> Option<&mut Option<Number>> {
        match param {
            Param::Power => Some(&mut self.power_pct),
            Param::Speed => Some(&mut self.speed_mm_min),
            Param::Passes => Some(&mut self.passes),
            Param::Interval => Some(&mut self.interval_mm),
            Param::Strength => None,
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
        if let Some(slot) = fixed.slot(value.param) {
            *slot = Some(number_json(value));
        }
    }
    RecipeJson {
        id: None,
        kind: recipe.kind.as_str().to_owned(),
        columns: axis(&recipe.columns),
        rows: axis(&recipe.rows),
        fixed,
        cell_mm: recipe.cell.as_mm(),
        crosshatch: recipe.crosshatch,
        // Only an Image grid writes its dither, so the other kinds keep the stored form of `0112`.
        dither: recipe.dither().map(|d| d.key().to_owned()),
        // The Ramp is the absent key, so a Ramp grid keeps the stored form of `0123`.
        picture: (recipe.picture == PictureSource::Selected).then(|| PICTURE_SELECTED.to_owned()),
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
    let picture = read_picture(json, kind)?;
    if picture == PictureSource::Ramp && [columns.param, rows.param].contains(&Param::Strength) {
        return None;
    }
    let mut stored = json.fixed.clone();
    let mut fixed = BTreeMap::new();
    for param in Param::ALL {
        let slot = stored.slot(param).and_then(Option::take);
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
    let dither = read_dither(json, kind)?;
    Some(TestRecipe {
        kind,
        columns,
        rows,
        fixed,
        cell: Length::from_mm(cell),
        crosshatch: json.crosshatch,
        dither,
        picture,
    })
}

/// The stored text of the selected picture.
const PICTURE_SELECTED: &str = "selected";
/// The stored text of the Ramp, read as the absent key (a Ramp grid never writes it).
const PICTURE_RAMP: &str = "ramp";

/// The picture of `json` (ADR 0072 §6): absent or `"ramp"` the Ramp, `"selected"` on an Image
/// grid the selected picture; any other value, or `"selected"` on another kind, is `None`.
fn read_picture(json: &RecipeJson, kind: SetKind) -> Option<PictureSource> {
    match json.picture.as_deref() {
        None | Some(PICTURE_RAMP) => Some(PictureSource::Ramp),
        Some(PICTURE_SELECTED) if kind == SetKind::Image => Some(PictureSource::Selected),
        Some(_) => None,
    }
}

/// The dither of `json` (ADR 0069 §13): on an Image grid a binary mode, absent reading
/// Floyd-Steinberg; a dither on another kind, an unknown value or Grey is `None`.
fn read_dither(json: &RecipeJson, kind: SetKind) -> Option<Dither> {
    match (json.dither.as_deref(), kind) {
        (None, _) => Some(Dither::FloydSteinberg),
        (Some(key), SetKind::Image) => Dither::from_key(key).filter(|d| d.in_test_grid()),
        (Some(_), _) => None,
    }
}
