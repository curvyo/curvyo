//! The stored fixtures of a manufacturing (ADR 0049 §2, §3, §6): the fixture and slot nodes as
//! read field by field, with a count of the values that did not read.

use loro::{LoroMap, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer};
use serde::Serialize;

use crate::affine::{Affine, MATRIX_ENTRY_LIMIT};
use crate::group_tree::node_id;
use crate::manufacturing::FixtureParams;
use crate::manufacturing_codec::{
    KEY_FIXTURE_ACTIVE, KEY_FIXTURE_KIND, KEY_FIXTURE_MATRIX, KEY_FIXTURE_PARAMS, KEY_SLOT_PARAMS,
    SHAPE_FIXTURE, SHAPE_FIXTURE_SLOT, json_of, string, write_plain_map,
};
use crate::node_registers::{KEY_NAME, KEY_VISIBLE, read_registers};
use crate::path_codec::as_f64;
use crate::path_model::NodeId;
use crate::shape_codec::KEY_SHAPE;

/// The most characters a fixture or slot name keeps (`specs/0102-laser-fixtures` criteria 3, 19).
pub const MAX_FIXTURE_NAME_CHARS: usize = 40;
/// The most slots a holder takes from Add slot (criterion 19); a file may hold more, all read.
pub const MAX_SLOTS_PER_FIXTURE: usize = 50;

/// An installed fixture as read (ADR 0049 §2). The kind and the values are opaque here and typed
/// by the process's extension (ADR 0033 §3).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FixtureSnapshot {
    /// Its node.
    pub id: NodeId,
    /// Its name ("Holder 1").
    pub name: String,
    /// The stored kind, `curvyo.laser/<kind>` for the laser's fixtures.
    pub kind: String,
    /// The parameter values that read; a value that does not is left out and counted.
    pub params: FixtureParams,
    /// The map from the kind's local frame into the page; `None` when absent or damaged.
    pub matrix: Option<Affine>,
    /// Whether the fixture is on the bed (absent reads as on).
    pub active: bool,
    /// The eye: whether it is shown.
    pub visible: bool,
    /// The slots that read, in tree order.
    pub slots: Vec<SlotSnapshot>,
    /// How many stored values did not read (each read as its default, a slot skipped).
    pub damaged: usize,
}

impl FixtureSnapshot {
    /// The values of the fixture without its node ids and its eye: what a profile set holds
    /// (ADR 0050 §1).
    #[must_use]
    pub fn record(&self) -> FixtureRecord {
        FixtureRecord {
            kind: self.kind.clone(),
            name: self.name.clone(),
            active: self.active,
            params: self.params.clone(),
            matrix: self.matrix,
            slots: self
                .slots
                .iter()
                .map(|slot| SlotRecord {
                    name: slot.name.clone(),
                    params: slot.params.clone(),
                })
                .collect(),
        }
    }
}

/// A slot of a holder as read (ADR 0049 §3).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlotSnapshot {
    /// Its node.
    pub id: NodeId,
    /// Its name ("Slot 1").
    pub name: String,
    /// The atomic `slot_params` map that read (`shape`, `cx_mm`, `cy_mm`, `w_mm`, `h_mm`, `d_mm`).
    pub params: FixtureParams,
}

/// A fixture's values without a node: what Pick profile and Reset from profile write as a new node
/// (ADR 0050 §4), and what a profile's set stores.
#[derive(Debug, Clone, PartialEq)]
pub struct FixtureRecord {
    /// The stored kind.
    pub kind: String,
    /// The name.
    pub name: String,
    /// The Active state.
    pub active: bool,
    /// The parameter values.
    pub params: FixtureParams,
    /// The matrix; `None` writes none (the grid).
    pub matrix: Option<Affine>,
    /// The slots, in order.
    pub slots: Vec<SlotRecord>,
}

/// A slot's values without a node.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotRecord {
    /// The name.
    pub name: String,
    /// The `slot_params` values.
    pub params: FixtureParams,
}

/// The fixture nodes among `children`, read in tree order.
pub(crate) fn read_fixtures(tree: &LoroTree, children: &[TreeID]) -> Vec<FixtureSnapshot> {
    children
        .iter()
        .filter_map(|&child| {
            let meta = tree.get_meta(child).ok()?;
            (string(&meta, KEY_SHAPE).as_deref() == Some(SHAPE_FIXTURE))
                .then(|| read_fixture(tree, child, &meta))
        })
        .collect()
}

fn read_fixture(tree: &LoroTree, node: TreeID, meta: &LoroMap) -> FixtureSnapshot {
    let registers = read_registers(meta);
    let (mut params, mut damaged) = read_counted_map(meta, KEY_FIXTURE_PARAMS);
    damaged += screw_diameters_read(&mut params);
    let (matrix, matrix_damage) = read_matrix(meta);
    let (active, active_damage) = match meta.get(KEY_FIXTURE_ACTIVE) {
        None => (true, 0),
        Some(ValueOrContainer::Value(LoroValue::Bool(flag))) => (flag, 0),
        Some(_) => (true, 1),
    };
    damaged += matrix_damage + active_damage;
    let mut slots = Vec::new();
    for child in tree.children(TreeParentId::Node(node)).unwrap_or_default() {
        match tree
            .get_meta(child)
            .ok()
            .and_then(|slot| read_slot(child, &slot))
        {
            Some((slot, slot_damage)) => {
                damaged += slot_damage;
                slots.push(slot);
            }
            None => damaged += 1,
        }
    }
    FixtureSnapshot {
        id: node_id(node),
        name: registers.name.unwrap_or_default(),
        kind: string(meta, KEY_FIXTURE_KIND).unwrap_or_default(),
        params,
        matrix,
        active,
        visible: registers.visible,
        slots,
        damaged,
    }
}

/// The keys of a screw's two diameters and their range in mm (ADR 0049 §2, `0102` criterion 37).
const SCREW_THREAD_KEY: &str = "thread_diameter_mm";
const SCREW_HEAD_KEY: &str = "head_diameter_mm";
const SCREW_DIAMETER_MM: std::ops::RangeInclusive<f64> = 0.5..=1000.0;

/// Reads a screw's diameters field by field (ADR 0049 §6): a value that is not a number within
/// 0.5 to 1,000 mm is left out, so it reads as its default, and counts as one damaged value; a
/// head below its thread reads as the thread and counts as one. Returns the damaged count.
fn screw_diameters_read(params: &mut FixtureParams) -> usize {
    let mut damaged = 0;
    for key in [SCREW_THREAD_KEY, SCREW_HEAD_KEY] {
        let readable = params
            .get(key)
            .map(|v| v.as_f64().is_some_and(|mm| SCREW_DIAMETER_MM.contains(&mm)));
        if readable == Some(false) {
            params.remove(key);
            damaged += 1;
        }
    }
    let number =
        |key: &str, params: &FixtureParams| params.get(key).and_then(serde_json::Value::as_f64);
    if let (Some(thread), Some(head)) = (
        number(SCREW_THREAD_KEY, params),
        number(SCREW_HEAD_KEY, params),
    ) && head < thread
    {
        params.insert(SCREW_HEAD_KEY.to_owned(), serde_json::Value::from(thread));
        damaged += 1;
    }
    damaged
}

/// A slot and its damaged values; `None` when its `slot_params` is missing or not a map.
fn read_slot(node: TreeID, meta: &LoroMap) -> Option<(SlotSnapshot, usize)> {
    if string(meta, KEY_SHAPE).as_deref() != Some(SHAPE_FIXTURE_SLOT) {
        return None;
    }
    let Some(ValueOrContainer::Value(LoroValue::Map(_))) = meta.get(KEY_SLOT_PARAMS) else {
        return None;
    };
    let (params, damaged) = read_counted_map(meta, KEY_SLOT_PARAMS);
    let slot = SlotSnapshot {
        id: node_id(node),
        name: read_registers(meta).name.unwrap_or_default(),
        params,
    };
    Some((slot, damaged))
}

/// The register `key` as plain values, with the number of entries that are not (a non-finite
/// number, a container); a register that is present and not a map reads empty and counts one.
fn read_counted_map(meta: &LoroMap, key: &str) -> (FixtureParams, usize) {
    match meta.get(key) {
        None => (FixtureParams::new(), 0),
        Some(ValueOrContainer::Value(LoroValue::Map(map))) => {
            let mut params = FixtureParams::new();
            let mut damaged = 0;
            for (name, value) in map.iter() {
                match json_of(value) {
                    Some(json) => {
                        params.insert(name.clone(), json);
                    }
                    None => damaged += 1,
                }
            }
            (params, damaged)
        }
        Some(_) => (FixtureParams::new(), 1),
    }
}

/// How far from a rotation a stored fixture matrix may be (ADR 0049 §6).
const RIGID_TOLERANCE: f64 = 1e-6;

/// The `fixture_matrix` register: `None` with no damage when absent, `None` and one damaged value
/// when it is not six finite numbers within ±1e7 or not rigid (a scale, a shear or a mirror).
fn read_matrix(meta: &LoroMap) -> (Option<Affine>, usize) {
    let entries = match meta.get(KEY_FIXTURE_MATRIX) {
        None => return (None, 0),
        Some(ValueOrContainer::Value(LoroValue::List(entries))) if entries.len() == 6 => entries,
        Some(_) => return (None, 1),
    };
    let mut matrix = [0.0; 6];
    for (slot, value) in matrix.iter_mut().zip(entries.iter()) {
        match as_f64(value).filter(|n| n.is_finite() && n.abs() <= MATRIX_ENTRY_LIMIT) {
            Some(number) => *slot = number,
            None => return (None, 1),
        }
    }
    let matrix = Affine::from_array(matrix);
    if matrix.is_rigid_within(RIGID_TOLERANCE) {
        (Some(matrix), 0)
    } else {
        (None, 1)
    }
}

/// Writes `matrix` as the six-number `fixture_matrix` register.
pub(crate) fn write_matrix(meta: &LoroMap, matrix: Affine) {
    let entries: Vec<LoroValue> = matrix
        .to_array()
        .into_iter()
        .map(LoroValue::Double)
        .collect();
    // invariant: inserting a plain list under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY_FIXTURE_MATRIX, LoroValue::from(entries))
        .unwrap();
}

/// Writes a new fixture node's registers from `record`, without its slots.
pub(crate) fn write_fixture_registers(meta: &LoroMap, record: &FixtureRecord) {
    // invariant: plain values under known keys on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    {
        meta.insert(KEY_SHAPE, SHAPE_FIXTURE).unwrap();
        meta.insert(KEY_FIXTURE_KIND, record.kind.as_str()).unwrap();
        meta.insert(KEY_NAME, record.name.as_str()).unwrap();
        meta.insert(KEY_FIXTURE_ACTIVE, record.active).unwrap();
    }
    write_plain_map(meta, KEY_FIXTURE_PARAMS, &record.params);
    if let Some(matrix) = record.matrix {
        write_matrix(meta, matrix);
    }
}

/// Writes a new slot node's registers from `record`.
pub(crate) fn write_slot_registers(meta: &LoroMap, record: &SlotRecord) {
    // invariant: plain values under known keys on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    {
        meta.insert(KEY_SHAPE, SHAPE_FIXTURE_SLOT).unwrap();
        meta.insert(KEY_NAME, record.name.as_str()).unwrap();
    }
    write_plain_map(meta, KEY_SLOT_PARAMS, &record.params);
}

/// Whether the eye register of `meta` differs from `visible`; writes it when it does.
pub(crate) fn write_visible(meta: &LoroMap, visible: bool) -> bool {
    if read_registers(meta).visible == visible {
        return false;
    }
    // invariant: a plain value under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY_VISIBLE, visible).unwrap();
    true
}

/// The stored `active` register of `meta`, as read (absent or damaged reads as on).
pub(crate) fn read_active(meta: &LoroMap) -> bool {
    !matches!(
        meta.get(KEY_FIXTURE_ACTIVE),
        Some(ValueOrContainer::Value(LoroValue::Bool(false)))
    )
}

/// The register `key` of `meta` as plain values, `None` unless it is a map that reads whole.
pub(crate) fn read_whole_map(meta: &LoroMap, key: &str) -> Option<FixtureParams> {
    match read_counted_map(meta, key) {
        (params, 0) if meta.get(key).is_some() => Some(params),
        _ => None,
    }
}

/// A fixture or slot name as stored: cleaned of control characters, trimmed and cut to
/// [`MAX_FIXTURE_NAME_CHARS`]; `None` when nothing is left.
pub(crate) fn fixture_name(raw: &str) -> Option<String> {
    let clean = crate::node_registers::clean_name(raw)?;
    let cut: String = clean.chars().take(MAX_FIXTURE_NAME_CHARS).collect();
    let trimmed = cut.trim_end();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(value: serde_json::Value) -> FixtureParams {
        serde_json::from_value(value).unwrap_or_default()
    }

    #[test]
    fn a_text_or_out_of_range_screw_diameter_reads_as_its_default_and_counts() {
        let mut text =
            params(serde_json::json!({"thread_diameter_mm": "M6", "head_diameter_mm": 10.0}));
        assert_eq!(screw_diameters_read(&mut text), 1);
        assert!(
            !text.contains_key("thread_diameter_mm"),
            "the default reads"
        );
        let mut huge =
            params(serde_json::json!({"thread_diameter_mm": 6.0, "head_diameter_mm": 5000.0}));
        assert_eq!(screw_diameters_read(&mut huge), 1);
        let mut small_head =
            params(serde_json::json!({"thread_diameter_mm": 8.0, "head_diameter_mm": 6.0}));
        assert_eq!(screw_diameters_read(&mut small_head), 1);
        assert_eq!(small_head["head_diameter_mm"], 8.0);
        let mut fine =
            params(serde_json::json!({"thread_diameter_mm": 6.0, "head_diameter_mm": 10.0}));
        assert_eq!(screw_diameters_read(&mut fine), 0);
    }

    #[test]
    fn a_matrix_with_scale_shear_or_a_mirror_is_not_rigid() {
        let tolerance = RIGID_TOLERANCE;
        let turn =
            Affine::rotation_about(crate::Point::new(3.0, 4.0), crate::Angle::from_radians(0.5));
        assert!(turn.is_rigid_within(tolerance));
        assert!(!Affine::scale(2.0, 2.0).is_rigid_within(tolerance), "scale");
        assert!(
            !Affine::scale(-1.0, 1.0).is_rigid_within(tolerance),
            "mirror"
        );
        assert!(
            !Affine::new(1.0, 0.0, 0.5, 1.0, 0.0, 0.0).is_rigid_within(tolerance),
            "shear"
        );
    }
}
