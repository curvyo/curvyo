//! The stored shape of manufacturings (ADR 0033 §1, §2, §7): the registers of the manufacturing,
//! placement and fixture nodes, how they are read, and the open-file check.

use std::collections::HashMap;

use loro::{LoroDoc, LoroMap, LoroTree, LoroValue, TreeID, ValueOrContainer};

use crate::affine::{Affine, MATRIX_ENTRY_LIMIT};
use crate::document::{OBJECTS_TREE, ROOT_MAP};
use crate::history::lineage::{decode_node, encode_node};
use crate::path_codec::{as_f64, node_exists};
use crate::shape_codec::KEY_SHAPE;

/// The marker register of a manufacturing node.
pub(crate) const KEY_MANUFACTURING: &str = "manufacturing";
/// The process id (`curvyo.laser`).
pub(crate) const KEY_PROCESS: &str = "mfg_process";
/// The owned page's id.
pub(crate) const KEY_PAGE: &str = "mfg_page";
/// The format-35 placeholder machine name; read for the migration, removed by the first profile
/// write (ADR 0039 §6).
pub(crate) const KEY_MACHINE_NAME: &str = "mfg_machine_name";
/// The atomic map of the machine profile copy (ADR 0039 §1), opaque here.
pub(crate) const KEY_PROFILE: &str = "mfg_profile";
/// The `shape` tag of a placement node.
pub(crate) const SHAPE_PLACEMENT: &str = "placement";
/// The `shape` tag of a fixture node.
pub(crate) const SHAPE_FIXTURE: &str = "fixture";
/// The atomic `{page, matrix}` register of a placement.
pub(crate) const KEY_PLACEMENT_LINK: &str = "placement_link";
/// The `<process id>/<kind>` register of a fixture.
pub(crate) const KEY_FIXTURE_KIND: &str = "fixture_kind";
/// The atomic map of a fixture's parameter values.
pub(crate) const KEY_FIXTURE_PARAMS: &str = "fixture_params";
/// The page record field naming its owning manufacturing (`page.<id>.owner`).
pub(crate) const PAGE_OWNER_FIELD: &str = "owner";

const LINK_PAGE: &str = "page";
const LINK_MATRIX: &str = "matrix";

/// Whether `meta` carries the manufacturing marker.
pub(crate) fn meta_is_manufacturing(meta: &LoroMap) -> bool {
    matches!(
        meta.get(KEY_MANUFACTURING),
        Some(ValueOrContainer::Value(LoroValue::Bool(true)))
    )
}

/// The one skip predicate of ADR 0032 (`adrs.md` decision 2): whether the root-level node `id`
/// is a manufacturing, so every walk of the drawing leaves its subtree out.
pub(crate) fn is_manufacturing_node(tree: &LoroTree, id: TreeID) -> bool {
    tree.get_meta(id)
        .is_ok_and(|meta| meta_is_manufacturing(&meta))
}

/// Whether `id` is a manufacturing or one of its placements or fixtures (they are its direct
/// children): no drawing command reads or writes such a node.
pub(crate) fn in_manufacturing_subtree(tree: &LoroTree, id: TreeID) -> bool {
    is_manufacturing_node(tree, id)
        || matches!(tree.parent(id), Some(loro::TreeParentId::Node(parent)) if is_manufacturing_node(tree, parent))
}

/// The live manufacturing nodes at the root of the tree, in creation (tree) order.
pub(crate) fn manufacturing_nodes(tree: &LoroTree) -> Vec<TreeID> {
    tree.children(loro::TreeParentId::Root)
        .unwrap_or_default()
        .into_iter()
        .filter(|&id| is_manufacturing_node(tree, id))
        .collect()
}

/// A string register, `None` when absent or not a string.
pub(crate) fn string(meta: &LoroMap, key: &str) -> Option<String> {
    match meta.get(key) {
        Some(ValueOrContainer::Value(LoroValue::String(text))) => Some(text.to_string()),
        _ => None,
    }
}

/// The stored placement link: the drawing page it shows and its matrix (page-local mm).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlacementLink {
    pub(crate) page: String,
    pub(crate) matrix: Affine,
}

pub(crate) fn write_placement_link(meta: &LoroMap, link: &PlacementLink) {
    let matrix: Vec<LoroValue> = link
        .matrix
        .to_array()
        .into_iter()
        .map(LoroValue::Double)
        .collect();
    let value: HashMap<String, LoroValue> = HashMap::from([
        (LINK_PAGE.to_owned(), LoroValue::from(link.page.as_str())),
        (LINK_MATRIX.to_owned(), LoroValue::from(matrix)),
    ]);
    // invariant: inserting a plain map value under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY_PLACEMENT_LINK, LoroValue::from(value))
        .unwrap();
}

/// The placement link, `None` when missing or not `{page string, six finite numbers within
/// ±1e7}` (ADR 0033 §7 damage rule).
pub(crate) fn read_placement_link(meta: &LoroMap) -> Option<PlacementLink> {
    let Some(ValueOrContainer::Value(LoroValue::Map(map))) = meta.get(KEY_PLACEMENT_LINK) else {
        return None;
    };
    let LoroValue::String(page) = map.get(LINK_PAGE)? else {
        return None;
    };
    let LoroValue::List(entries) = map.get(LINK_MATRIX)? else {
        return None;
    };
    if entries.len() != 6 {
        return None;
    }
    let mut matrix = [0.0; 6];
    for (slot, value) in matrix.iter_mut().zip(entries.iter()) {
        *slot = as_f64(value).filter(|n| n.is_finite() && n.abs() <= MATRIX_ENTRY_LIMIT)?;
    }
    Some(PlacementLink {
        page: page.to_string(),
        matrix: Affine::from_array(matrix),
    })
}

/// The fixture's parameter map as plain JSON values, `None` when it is not a map of plain values.
pub(crate) fn read_fixture_params(
    meta: &LoroMap,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    read_plain_map(meta, KEY_FIXTURE_PARAMS)
}

pub(crate) fn write_fixture_params(
    meta: &LoroMap,
    params: &serde_json::Map<String, serde_json::Value>,
) {
    write_plain_map(meta, KEY_FIXTURE_PARAMS, params);
}

/// The register `key` as a map of plain JSON values, `None` when absent or not such a map.
pub(crate) fn read_plain_map(
    meta: &LoroMap,
    key: &str,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    let Some(ValueOrContainer::Value(LoroValue::Map(map))) = meta.get(key) else {
        return None;
    };
    map.iter()
        .map(|(key, value)| Some((key.clone(), json_of(value)?)))
        .collect()
}

/// The machine profile copy `mfg_profile` as plain JSON values, `None` when absent or not a map of
/// plain values. A number that is not finite reads as `null`, so the extension reads that one
/// field as its default and the file still opens (ADR 0039 §3).
pub(crate) fn read_profile_map(
    meta: &LoroMap,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    let Some(ValueOrContainer::Value(LoroValue::Map(map))) = meta.get(KEY_PROFILE) else {
        return None;
    };
    map.iter()
        .map(|(key, value)| {
            let json = match value {
                LoroValue::Double(number) if !number.is_finite() => serde_json::Value::Null,
                other => json_of(other)?,
            };
            Some((key.clone(), json))
        })
        .collect()
}

/// Writes `params` whole as the plain map register `key`.
pub(crate) fn write_plain_map(
    meta: &LoroMap,
    key: &str,
    params: &serde_json::Map<String, serde_json::Value>,
) {
    let value: HashMap<String, LoroValue> = params
        .iter()
        .map(|(key, value)| (key.clone(), loro_of(value)))
        .collect();
    // invariant: inserting a plain map value under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(key, LoroValue::from(value)).unwrap();
}

/// A plain Loro value as JSON; `None` for a container, binary or non-finite number.
pub(crate) fn json_of(value: &LoroValue) -> Option<serde_json::Value> {
    Some(match value {
        LoroValue::Null => serde_json::Value::Null,
        LoroValue::Bool(flag) => serde_json::Value::Bool(*flag),
        LoroValue::Double(number) => {
            serde_json::Value::from(serde_json::Number::from_f64(*number)?)
        }
        LoroValue::I64(number) => serde_json::Value::from(*number),
        LoroValue::String(text) => serde_json::Value::String(text.to_string()),
        LoroValue::List(items) => {
            serde_json::Value::Array(items.iter().map(json_of).collect::<Option<_>>()?)
        }
        LoroValue::Map(map) => serde_json::Value::Object(
            map.iter()
                .map(|(key, value)| Some((key.clone(), json_of(value)?)))
                .collect::<Option<_>>()?,
        ),
        LoroValue::Binary(_) | LoroValue::Container(_) => return None,
    })
}

/// A JSON value as a plain Loro value; an integer is stored as `I64` (ADR 0039 §1), every other
/// number as a double.
fn loro_of(value: &serde_json::Value) -> LoroValue {
    match value {
        serde_json::Value::Null => LoroValue::Null,
        serde_json::Value::Bool(flag) => LoroValue::Bool(*flag),
        serde_json::Value::Number(number) => number.as_i64().map_or_else(
            || LoroValue::Double(number.as_f64().unwrap_or_default()),
            LoroValue::I64,
        ),
        serde_json::Value::String(text) => LoroValue::from(text.as_str()),
        serde_json::Value::Array(items) => {
            LoroValue::from(items.iter().map(loro_of).collect::<Vec<_>>())
        }
        serde_json::Value::Object(map) => LoroValue::from(
            map.iter()
                .map(|(key, value)| (key.clone(), loro_of(value)))
                .collect::<HashMap<_, _>>(),
        ),
    }
}

/// The process registers `x.<process id>.<key>` of a manufacturing, raw (ADR 0033 §3, §6).
pub(crate) fn process_values(meta: &LoroMap) -> serde_json::Map<String, serde_json::Value> {
    let mut values = serde_json::Map::new();
    meta.for_each(|key, value| {
        if let (true, ValueOrContainer::Value(value)) = (key.starts_with("x."), value)
            && let Some(json) = json_of(&value)
        {
            values.insert(key.to_owned(), json);
        }
    });
    values
}

/// The text a node id is stored as in `page.<id>.owner`.
pub(crate) fn owner_text(id: TreeID) -> String {
    encode_node(id)
}

/// The open-file check of ADR 0033 §7: every manufacturing names a process and a page, holds only
/// placement and fixture leaves with readable registers, and every page owner of a live page
/// names a live manufacturing (or a removed node, whose page is not shown).
pub(crate) fn validate(loro: &LoroDoc) -> bool {
    let tree = loro.get_tree(OBJECTS_TREE);
    let root = loro.get_map(ROOT_MAP);
    let nodes_ok = tree
        .children(loro::TreeParentId::Root)
        .unwrap_or_default()
        .into_iter()
        .all(|id| {
            let Ok(meta) = tree.get_meta(id) else {
                return false;
            };
            match meta.get(KEY_MANUFACTURING) {
                None | Some(ValueOrContainer::Value(LoroValue::Bool(false))) => true,
                Some(ValueOrContainer::Value(LoroValue::Bool(true))) => {
                    manufacturing_is_valid(&tree, id, &meta)
                }
                Some(_) => false,
            }
        });
    nodes_ok && owners_are_valid(&tree, &root)
}

fn manufacturing_is_valid(tree: &LoroTree, id: TreeID, meta: &LoroMap) -> bool {
    let strings_ok = [KEY_PROCESS, KEY_PAGE]
        .iter()
        .all(|key| string(meta, key).is_some())
        && matches!(
            meta.get(KEY_MACHINE_NAME),
            None | Some(ValueOrContainer::Value(LoroValue::String(_)))
        )
        && (meta.get(KEY_PROFILE).is_none() || read_profile_map(meta).is_some())
        && crate::node_registers::registers_are_valid(meta, false)
        && meta.get(crate::group_tree::KEY_GROUP).is_none();
    strings_ok
        && tree
            .children(loro::TreeParentId::Node(id))
            .unwrap_or_default()
            .into_iter()
            .all(|child| child_is_valid(tree, child))
}

fn child_is_valid(tree: &LoroTree, id: TreeID) -> bool {
    let Ok(meta) = tree.get_meta(id) else {
        return false;
    };
    if tree.children_num(id).unwrap_or(0) != 0
        || !crate::node_registers::registers_are_valid(&meta, false)
    {
        return false;
    }
    match string(&meta, KEY_SHAPE).as_deref() {
        Some(SHAPE_PLACEMENT) => read_placement_link(&meta).is_some(),
        Some(SHAPE_FIXTURE) => {
            string(&meta, KEY_FIXTURE_KIND).is_some() && read_fixture_params(&meta).is_some()
        }
        _ => false,
    }
}

fn owners_are_valid(tree: &LoroTree, root: &LoroMap) -> bool {
    let mut ok = true;
    root.for_each(|name, value| {
        let Some(page) = name
            .strip_prefix("page.")
            .and_then(|rest| rest.strip_suffix(".owner"))
        else {
            return;
        };
        let ValueOrContainer::Value(LoroValue::String(text)) = value else {
            ok = false;
            return;
        };
        let Some(owner) = decode_node(&text) else {
            ok = false;
            return;
        };
        let deleted = matches!(
            root.get(&crate::pages::key(page, "deleted")),
            Some(ValueOrContainer::Value(LoroValue::Bool(true)))
        );
        if !deleted && node_exists(tree, owner) && !is_manufacturing_node(tree, owner) {
            ok = false;
        }
    });
    ok
}
