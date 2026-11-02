//! The stored shape of a tiling (ADR 0028 §1, §2 and §10): the four registers read, written and
//! validated here.
//!
//! ```text
//! meta map of a tiling group (format 33), besides `group: true` and `name = "Tiling"`:
//!   tiling         : ONE map value, written whole:
//!     layout       : "grid" | "circle"
//!     grid         : { columns, rows, spacing_x mm, spacing_y mm, spacing: "gap" | "pitch",
//!                      mirror_rows, mirror_columns, offset: "none" | "rows" | "columns",
//!                      shift % }
//!     circle       : { count, centre_x mm, centre_y mm (from the cell box centre), radius mm,
//!                      start rad, sweep rad, face_centre }
//!     variance     : { rotation, scale_x, scale_y, offset_x, offset_y, opacity, hue:
//!                      [random, per row, per column] in rad, %, mm or percentage points,
//!                      absent = 0; uniform_scale }
//!     seed         : 0 ..= 4294967295
//!     result       : "clones" | "copies"
//!     keep_source  : bool
//!   tiling_sources : ["<peer>:<counter>", ...]   the selected source nodes, one list value
//! meta map of a generated instance (a direct child of the tiling group):
//!   tiling_cell    : [row, column]               a circle uses [0, index]
//! meta map of a member of a multi-source instance group:
//!   tiling_member  : "<peer>:<counter>"          its source leaf
//! ```
//!
//! Damage (refused at open): `tiling` on a node that is not a group; an unknown `layout`,
//! `result`, `spacing` or `offset`; a count of 0 or over the limit; a number that is not finite;
//! a malformed `tiling`, `tiling_sources`, `tiling_cell` or `tiling_member`. Tolerated residue:
//! dangling source ids, duplicate and missing keys, keyed children of a group without a recipe,
//! unkeyed children of a tiling.

use std::collections::HashMap;

use loro::{LoroMap, LoroValue, TreeID, ValueOrContainer};

use crate::history::{decode_node, encode_node};
use crate::path_codec::as_f64;
use crate::path_model::NodeId;
use crate::tiling_model::{
    CellKey, Channel, CircleOptions, GridOptions, OffsetLines, Percent, SpacingMode, TilingLayout,
    TilingRecipe, TilingResult, Variance,
};
use crate::units::{Angle, Length, Vec2};

/// The recipe register.
pub(crate) const KEY_TILING: &str = "tiling";
/// The source list register.
pub(crate) const KEY_SOURCES: &str = "tiling_sources";
/// The cell key of an instance.
pub(crate) const KEY_CELL: &str = "tiling_cell";
/// The source leaf of a member.
pub(crate) const KEY_MEMBER: &str = "tiling_member";

/// Every tiling register, for the writers that strip them (Release, Ungroup, copy out).
pub(crate) const ALL_TILING_KEYS: [&str; 4] = [KEY_TILING, KEY_SOURCES, KEY_CELL, KEY_MEMBER];

fn insert(meta: &LoroMap, key: &str, value: impl Into<LoroValue>) {
    // invariant: inserting a plain value under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(key, value).unwrap();
}

/// Removes `key` from `meta` when present. Uncommitted.
pub(crate) fn remove(meta: &LoroMap, key: &str) {
    if meta.get(key).is_some() {
        // invariant: deleting a key just confirmed present on an attached map cannot fail.
        #[allow(clippy::unwrap_used)]
        meta.delete(key).unwrap();
    }
}

/// Removes every tiling register from `meta`. Uncommitted.
pub(crate) fn strip(meta: &LoroMap) {
    for key in ALL_TILING_KEYS {
        remove(meta, key);
    }
}

fn map(entries: Vec<(&str, LoroValue)>) -> LoroValue {
    let map: HashMap<String, LoroValue> = entries
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect();
    LoroValue::Map(map.into())
}

fn triple(values: [f64; 3]) -> LoroValue {
    LoroValue::List(
        values
            .iter()
            .map(|v| LoroValue::Double(*v))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn recipe_value(recipe: &TilingRecipe) -> LoroValue {
    let g = &recipe.grid;
    let c = &recipe.circle;
    let v = &recipe.variance;
    let grid = map(vec![
        ("columns", i64::from(g.columns).into()),
        ("rows", i64::from(g.rows).into()),
        ("spacing_x", g.spacing_x.as_mm().into()),
        ("spacing_y", g.spacing_y.as_mm().into()),
        (
            "spacing",
            match g.spacing_mode {
                SpacingMode::Gap => "gap",
                SpacingMode::Pitch => "pitch",
            }
            .into(),
        ),
        ("mirror_rows", g.mirror_rows.into()),
        ("mirror_columns", g.mirror_columns.into()),
        (
            "offset",
            match g.offset {
                OffsetLines::None => "none",
                OffsetLines::Rows => "rows",
                OffsetLines::Columns => "columns",
            }
            .into(),
        ),
        ("shift", g.shift.get().into()),
    ]);
    let circle = map(vec![
        ("count", i64::from(c.count).into()),
        ("centre_x", c.centre.x.into()),
        ("centre_y", c.centre.y.into()),
        ("radius", c.radius.as_mm().into()),
        ("start", c.start.as_radians().into()),
        ("sweep", c.sweep.as_radians().into()),
        ("face_centre", c.face_centre.into()),
    ]);
    let angles =
        |ch: Channel<Angle>| triple([ch.random, ch.per_row, ch.per_column].map(Angle::as_radians));
    let percents =
        |ch: Channel<Percent>| triple([ch.random, ch.per_row, ch.per_column].map(Percent::get));
    let lengths =
        |ch: Channel<Length>| triple([ch.random, ch.per_row, ch.per_column].map(Length::as_mm));
    let variance = map(vec![
        ("rotation", angles(v.rotation)),
        ("scale_x", percents(v.scale_x)),
        ("scale_y", percents(v.scale_y)),
        ("offset_x", lengths(v.offset_x)),
        ("offset_y", lengths(v.offset_y)),
        ("opacity", percents(v.opacity)),
        ("hue", angles(v.hue)),
        ("uniform_scale", v.uniform_scale.into()),
    ]);
    map(vec![
        (
            "layout",
            match recipe.layout {
                TilingLayout::Grid => "grid",
                TilingLayout::Circle => "circle",
            }
            .into(),
        ),
        ("grid", grid),
        ("circle", circle),
        ("variance", variance),
        ("seed", i64::from(recipe.seed).into()),
        (
            "result",
            match recipe.result {
                TilingResult::Clones => "clones",
                TilingResult::Copies => "copies",
            }
            .into(),
        ),
        ("keep_source", recipe.keep_source.into()),
    ])
}

/// Writes the recipe register, only when it differs from the stored one. Uncommitted.
pub(crate) fn write_recipe(meta: &LoroMap, recipe: &TilingRecipe) {
    if read_recipe(meta) != Some(Ok(*recipe)) {
        insert(meta, KEY_TILING, recipe_value(recipe));
    }
}

/// Writes the source list register, only when it differs. Uncommitted.
pub(crate) fn write_sources(meta: &LoroMap, sources: &[NodeId]) {
    if read_sources(meta).as_deref() == Some(sources) {
        return;
    }
    let list: Vec<LoroValue> = sources
        .iter()
        .map(|id| encode_node(TreeID::new(id.peer, id.counter)).into())
        .collect();
    insert(meta, KEY_SOURCES, LoroValue::List(list.into()));
}

/// Writes an instance's cell key. Uncommitted.
pub(crate) fn write_cell(meta: &LoroMap, key: CellKey) {
    insert(
        meta,
        KEY_CELL,
        LoroValue::List(vec![i64::from(key.row).into(), i64::from(key.column).into()].into()),
    );
}

/// Writes a member's source leaf. Uncommitted.
pub(crate) fn write_member(meta: &LoroMap, leaf: NodeId) {
    insert(
        meta,
        KEY_MEMBER,
        encode_node(TreeID::new(leaf.peer, leaf.counter)),
    );
}

fn value_of(meta: &LoroMap, key: &str) -> Option<LoroValue> {
    match meta.get(key)? {
        ValueOrContainer::Value(value) => Some(value),
        ValueOrContainer::Container(_) => Some(LoroValue::Null),
    }
}

/// The recipe register: `None` when absent, `Some(Err(()))` when malformed (damage).
pub(crate) fn read_recipe(meta: &LoroMap) -> Option<Result<TilingRecipe, ()>> {
    let value = value_of(meta, KEY_TILING)?;
    Some(recipe_from_value(&value).ok_or(()))
}

/// Whether `meta` carries a recipe register at all.
pub(crate) fn has_recipe(meta: &LoroMap) -> bool {
    meta.get(KEY_TILING).is_some()
}

fn field<'a>(map: &'a loro::LoroMapValue, key: &str) -> Option<&'a LoroValue> {
    map.get(key)
}

fn number(map: &loro::LoroMapValue, key: &str) -> Option<f64> {
    as_f64(field(map, key)?)
}

fn count(map: &loro::LoroMapValue, key: &str) -> Option<u32> {
    match field(map, key)? {
        LoroValue::I64(n) => u32::try_from(*n).ok(),
        _ => None,
    }
}

fn flag(map: &loro::LoroMapValue, key: &str) -> Option<bool> {
    match field(map, key)? {
        LoroValue::Bool(b) => Some(*b),
        _ => None,
    }
}

fn text<'a>(map: &'a loro::LoroMapValue, key: &str) -> Option<&'a str> {
    match field(map, key)? {
        LoroValue::String(s) => Some(s.as_str()),
        _ => None,
    }
}

fn sub_map<'a>(map: &'a loro::LoroMapValue, key: &str) -> Option<&'a loro::LoroMapValue> {
    match field(map, key)? {
        LoroValue::Map(m) => Some(m),
        _ => None,
    }
}

/// An absent channel is zero; a present one is a list of three numbers.
fn channel(map: &loro::LoroMapValue, key: &str) -> Option<[f64; 3]> {
    match map.get(key) {
        None => Some([0.0; 3]),
        Some(LoroValue::List(list)) if list.len() == 3 => {
            let values: Option<Vec<f64>> = list.iter().map(as_f64).collect();
            let values = values?;
            Some([values[0], values[1], values[2]])
        }
        Some(_) => None,
    }
}

fn grid_from(g: &loro::LoroMapValue) -> Option<GridOptions> {
    Some(GridOptions {
        columns: count(g, "columns")?,
        rows: count(g, "rows")?,
        spacing_x: Length::from_mm(number(g, "spacing_x")?),
        spacing_y: Length::from_mm(number(g, "spacing_y")?),
        spacing_mode: match text(g, "spacing")? {
            "gap" => SpacingMode::Gap,
            "pitch" => SpacingMode::Pitch,
            _ => return None,
        },
        mirror_rows: flag(g, "mirror_rows")?,
        mirror_columns: flag(g, "mirror_columns")?,
        offset: match text(g, "offset")? {
            "none" => OffsetLines::None,
            "rows" => OffsetLines::Rows,
            "columns" => OffsetLines::Columns,
            _ => return None,
        },
        shift: Percent::new(number(g, "shift")?),
    })
}

fn circle_from(c: &loro::LoroMapValue) -> Option<CircleOptions> {
    Some(CircleOptions {
        count: count(c, "count")?,
        centre: Vec2::new(number(c, "centre_x")?, number(c, "centre_y")?),
        radius: Length::from_mm(number(c, "radius")?),
        start: Angle::from_radians(number(c, "start")?),
        sweep: Angle::from_radians(number(c, "sweep")?),
        face_centre: flag(c, "face_centre")?,
    })
}

fn variance_from(v: &loro::LoroMapValue) -> Option<Variance> {
    let angles = |key| {
        channel(v, key).map(|[r, row, col]| Channel {
            random: Angle::from_radians(r),
            per_row: Angle::from_radians(row),
            per_column: Angle::from_radians(col),
        })
    };
    let percents = |key| {
        channel(v, key).map(|[r, row, col]| Channel {
            random: Percent::new(r),
            per_row: Percent::new(row),
            per_column: Percent::new(col),
        })
    };
    let lengths = |key| {
        channel(v, key).map(|[r, row, col]| Channel {
            random: Length::from_mm(r),
            per_row: Length::from_mm(row),
            per_column: Length::from_mm(col),
        })
    };
    Some(Variance {
        rotation: angles("rotation")?,
        scale_x: percents("scale_x")?,
        scale_y: percents("scale_y")?,
        offset_x: lengths("offset_x")?,
        offset_y: lengths("offset_y")?,
        opacity: percents("opacity")?,
        hue: angles("hue")?,
        uniform_scale: match v.get("uniform_scale") {
            None => true,
            Some(LoroValue::Bool(b)) => *b,
            Some(_) => return None,
        },
    })
}

fn recipe_from_value(value: &LoroValue) -> Option<TilingRecipe> {
    let LoroValue::Map(root) = value else {
        return None;
    };
    let recipe = TilingRecipe {
        layout: match text(root, "layout")? {
            "grid" => TilingLayout::Grid,
            "circle" => TilingLayout::Circle,
            _ => return None,
        },
        grid: grid_from(sub_map(root, "grid")?)?,
        circle: circle_from(sub_map(root, "circle")?)?,
        variance: variance_from(sub_map(root, "variance")?)?,
        seed: count(root, "seed")?,
        result: match text(root, "result")? {
            "clones" => TilingResult::Clones,
            "copies" => TilingResult::Copies,
            _ => return None,
        },
        keep_source: flag(root, "keep_source")?,
    };
    recipe.check_stored().ok()?;
    Some(recipe)
}

/// The source list: `None` when absent or malformed.
pub(crate) fn read_sources(meta: &LoroMap) -> Option<Vec<NodeId>> {
    let LoroValue::List(list) = value_of(meta, KEY_SOURCES)? else {
        return None;
    };
    list.iter()
        .map(|item| match item {
            LoroValue::String(s) => {
                decode_node(s).map(|id| NodeId::from_parts(id.peer, id.counter))
            }
            _ => None,
        })
        .collect()
}

/// An instance's cell key: `None` when absent or malformed.
pub(crate) fn read_cell(meta: &LoroMap) -> Option<CellKey> {
    let LoroValue::List(list) = value_of(meta, KEY_CELL)? else {
        return None;
    };
    if list.len() != 2 {
        return None;
    }
    let part = |value: &LoroValue| match value {
        LoroValue::I64(n) => u32::try_from(*n).ok(),
        _ => None,
    };
    Some(CellKey {
        row: part(&list[0])?,
        column: part(&list[1])?,
    })
}

/// A member's source leaf: `None` when absent or malformed.
pub(crate) fn read_member(meta: &LoroMap) -> Option<NodeId> {
    match value_of(meta, KEY_MEMBER)? {
        LoroValue::String(s) => decode_node(&s).map(|id| NodeId::from_parts(id.peer, id.counter)),
        _ => None,
    }
}

/// The open-file check of one node's tiling registers (ADR 0028 §10): a recipe only on a group
/// and well formed; a source list, cell key and member key well formed wherever they are.
pub(crate) fn registers_are_valid(meta: &LoroMap, is_group: bool) -> bool {
    let recipe_ok = match read_recipe(meta) {
        None => true,
        Some(Ok(_)) => is_group,
        Some(Err(())) => false,
    };
    recipe_ok
        && (meta.get(KEY_SOURCES).is_none() || read_sources(meta).is_some())
        && (meta.get(KEY_CELL).is_none() || read_cell(meta).is_some())
        && (meta.get(KEY_MEMBER).is_none() || read_member(meta).is_some())
}
