//! A container tree read as plain data, with the container-versus-value distinction kept, and the
//! three-way restore of live containers against it (ADR 0014 §1: for each field, write the old
//! value back only where the current value is still the one the step wrote).
//!
//! A [`Node`] is what the document schema stores: plain registers, maps (an object's meta map, an
//! anchor) and movable lists (anchors, further outlines). `LoroValue::get_deep_value` flattens
//! containers into plain maps and lists, which would lose that distinction, so the values are read
//! container by container.

use std::collections::{BTreeMap, BTreeSet};

use loro::{Container, LoroMap, LoroMovableList, LoroResult, LoroValue, ValueOrContainer};

/// The key that names an item of a list of maps (an anchor's id).
const KEY_ITEM_ID: &str = "id";

/// A stored value with its container structure.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Node {
    /// A plain register value (a number, a string, a point as a list).
    Value(LoroValue),
    /// A map container.
    Map(BTreeMap<String, Node>),
    /// A movable list container.
    List(Vec<Node>),
}

/// What a restore did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Tally {
    /// Fields, anchors and entries written back.
    pub written: usize,
    /// Fields, anchors and entries left alone because someone changed them after the step.
    pub kept: usize,
}

impl Node {
    /// Reads any stored child.
    pub(crate) fn read(child: &ValueOrContainer) -> Self {
        match child {
            ValueOrContainer::Value(value) => Self::Value(value.clone()),
            ValueOrContainer::Container(Container::Map(map)) => Self::Map(read_map(map)),
            ValueOrContainer::Container(Container::MovableList(list)) => {
                Self::List(read_list(list))
            }
            // No other container kind is stored; its flattened value keeps the data.
            ValueOrContainer::Container(_) => Self::Value(child.get_deep_value()),
        }
    }

    fn as_map(&self) -> Option<&BTreeMap<String, Self>> {
        match self {
            Self::Map(map) => Some(map),
            _ => None,
        }
    }

    /// The item id of a map that has one.
    fn item_id(&self) -> Option<&str> {
        match self.as_map()?.get(KEY_ITEM_ID)? {
            Self::Value(LoroValue::String(id)) => Some(id.as_str()),
            _ => None,
        }
    }
}

/// Reads a map container.
pub(crate) fn read_map(map: &LoroMap) -> BTreeMap<String, Node> {
    map.keys()
        .filter_map(|key| {
            let child = map.get(&key)?;
            Some((key.to_string(), Node::read(&child)))
        })
        .collect()
}

/// Reads a movable list container.
pub(crate) fn read_list(list: &LoroMovableList) -> Vec<Node> {
    (0..list.len())
        .filter_map(|index| list.get(index))
        .map(|child| Node::read(&child))
        .collect()
}

/// Writes `node` under `key` of `map`, creating containers where it holds them.
fn write_entry(map: &LoroMap, key: &str, node: &Node) -> LoroResult<()> {
    match node {
        Node::Value(value) => map.insert(key, value.clone()),
        Node::Map(entries) => {
            let child = map.insert_container(key, LoroMap::new())?;
            fill_map(&child, entries)
        }
        Node::List(items) => {
            let child = map.insert_container(key, LoroMovableList::new())?;
            fill_list(&child, items)
        }
    }
}

fn fill_map(map: &LoroMap, entries: &BTreeMap<String, Node>) -> LoroResult<()> {
    entries
        .iter()
        .try_for_each(|(key, node)| write_entry(map, key, node))
}

fn fill_list(list: &LoroMovableList, items: &[Node]) -> LoroResult<()> {
    items
        .iter()
        .try_for_each(|item| insert_item(list, list.len(), item))
}

/// Inserts `item` at `index` of `list`.
fn insert_item(list: &LoroMovableList, index: usize, item: &Node) -> LoroResult<()> {
    match item {
        Node::Value(value) => list.insert(index, value.clone()),
        Node::Map(entries) => {
            let child = list.insert_container(index, LoroMap::new())?;
            fill_map(&child, entries)
        }
        Node::List(items) => {
            let child = list.insert_container(index, LoroMovableList::new())?;
            fill_list(&child, items)
        }
    }
}

/// Brings the live `map` from the state `from` (what a step left) to the state `to` (what it
/// started from, for an undo): a key goes to its `to` value only where the live value is still its
/// `from` value. A nested map or list is merged recursively.
pub(crate) fn restore_map(
    live: &LoroMap,
    from: &BTreeMap<String, Node>,
    to: &BTreeMap<String, Node>,
    tally: &mut Tally,
) {
    let keys: BTreeSet<&String> = from.keys().chain(to.keys()).collect();
    for key in keys {
        let (from_node, to_node) = (from.get(key), to.get(key));
        if from_node == to_node {
            continue;
        }
        let current = live.get(key);
        match (from_node, to_node, &current) {
            (
                Some(Node::Map(from_inner)),
                Some(Node::Map(to_inner)),
                Some(ValueOrContainer::Container(Container::Map(child))),
            ) => restore_map(child, from_inner, to_inner, tally),
            (
                Some(Node::List(from_items)),
                Some(Node::List(to_items)),
                Some(ValueOrContainer::Container(Container::MovableList(child))),
            ) => restore_list(child, from_items, to_items, tally),
            _ => {
                let now = current.as_ref().map(Node::read);
                if now.as_ref() == from_node && write_back(live, key, to_node) {
                    tally.written += 1;
                } else {
                    tally.kept += 1;
                }
            }
        }
    }
}

/// Sets `key` to `node`, or removes it for `None`. False when the CRDT refuses the write.
fn write_back(map: &LoroMap, key: &str, node: Option<&Node>) -> bool {
    match node {
        Some(node) => write_entry(map, key, node).is_ok(),
        None => map.delete(key).is_ok(),
    }
}

/// Whether every item of both lists is a map with an id, so that items can be matched by id.
fn is_keyed(from: &[Node], to: &[Node]) -> bool {
    from.iter().chain(to).all(|item| item.item_id().is_some())
}

/// Brings the live `list` from `from` to `to`, matching items by id when they have one and by
/// position otherwise.
pub(crate) fn restore_list(live: &LoroMovableList, from: &[Node], to: &[Node], tally: &mut Tally) {
    if from == to {
        return;
    }
    if is_keyed(from, to) {
        restore_keyed_list(live, from, to, tally);
    } else {
        restore_positional_list(live, from, to, tally);
    }
}

/// The index of the live item with `id`.
fn position_of(live: &[Node], id: &str) -> Option<usize> {
    live.iter().position(|item| item.item_id() == Some(id))
}

fn restore_keyed_list(live: &LoroMovableList, from: &[Node], to: &[Node], tally: &mut Tally) {
    let from_ids: Vec<&str> = from.iter().filter_map(Node::item_id).collect();
    let to_ids: Vec<&str> = to.iter().filter_map(Node::item_id).collect();
    // Items the step added: remove them where nobody changed them since.
    for item in from
        .iter()
        .filter(|i| i.item_id().is_some_and(|id| !to_ids.contains(&id)))
    {
        let Some(id) = item.item_id() else { continue };
        let now = read_list(live);
        match position_of(&now, id) {
            Some(index) if now[index] == *item && live.delete(index, 1).is_ok() => {
                tally.written += 1;
            }
            Some(_) => tally.kept += 1,
            None => {}
        }
    }
    // Items the step removed: put them back where they were, after their predecessor.
    for (position, item) in to.iter().enumerate() {
        let Some(id) = item.item_id() else { continue };
        if from_ids.contains(&id) {
            continue;
        }
        let now = read_list(live);
        if position_of(&now, id).is_some() {
            continue;
        }
        let index = to[..position]
            .iter()
            .rev()
            .find_map(|earlier| earlier.item_id().and_then(|e| position_of(&now, e)))
            .map_or(0, |found| found + 1);
        if insert_item(live, index, item).is_ok() {
            tally.written += 1;
        } else {
            tally.kept += 1;
        }
    }
    // Items both states hold: merge their fields.
    for item in to
        .iter()
        .filter(|i| i.item_id().is_some_and(|id| from_ids.contains(&id)))
    {
        let Some(id) = item.item_id() else { continue };
        let Some(from_item) = from.iter().find(|f| f.item_id() == Some(id)) else {
            continue;
        };
        if from_item == item {
            continue;
        }
        let now = read_list(live);
        let Some(index) = position_of(&now, id) else {
            tally.kept += 1;
            continue;
        };
        if let (
            Some(from_map),
            Some(to_map),
            Some(ValueOrContainer::Container(Container::Map(map))),
        ) = (from_item.as_map(), item.as_map(), live.get(index))
        {
            restore_map(&map, from_map, to_map, tally);
        }
    }
    restore_item_order(live, from, to, tally);
}

/// If the step moved items and nobody reordered them since, puts the common items back in the
/// order they had before.
fn restore_item_order(live: &LoroMovableList, from: &[Node], to: &[Node], tally: &mut Tally) {
    let ids = |nodes: &[Node]| -> Vec<String> {
        nodes
            .iter()
            .filter_map(|n| n.item_id().map(str::to_owned))
            .collect()
    };
    let now = read_list(live);
    let now_ids = ids(&now);
    let only_common = |wanted: &[String], source: &[String]| -> Vec<String> {
        source
            .iter()
            .filter(|id| wanted.contains(id))
            .cloned()
            .collect()
    };
    let (from_ids, to_ids) = (ids(from), ids(to));
    let common_to = only_common(&now_ids, &to_ids);
    let common_from = only_common(&common_to, &from_ids);
    let common_now = only_common(&common_to, &now_ids);
    if common_from == common_to {
        return;
    }
    if common_now != common_from {
        tally.kept += 1;
        return;
    }
    for (target, id) in common_to.iter().enumerate() {
        let current = read_list(live);
        let order = ids(&current);
        let slots: Vec<usize> = order
            .iter()
            .enumerate()
            .filter(|(_, other)| common_to.contains(other))
            .map(|(index, _)| index)
            .collect();
        let (Some(&slot), Some(from_index)) =
            (slots.get(target), order.iter().position(|o| o == id))
        else {
            continue;
        };
        if from_index != slot && live.mov(from_index, slot).is_ok() {
            tally.written += 1;
        }
    }
}

fn restore_positional_list(live: &LoroMovableList, from: &[Node], to: &[Node], tally: &mut Tally) {
    let now = read_list(live);
    if from.len() == to.len() && now.len() == from.len() {
        for (index, (from_item, to_item)) in from.iter().zip(to).enumerate() {
            if from_item == to_item {
                continue;
            }
            match (from_item, to_item, live.get(index)) {
                (
                    Node::Map(f),
                    Node::Map(t),
                    Some(ValueOrContainer::Container(Container::Map(map))),
                ) => restore_map(&map, f, t, tally),
                _ if now[index] == *from_item => {
                    if live.delete(index, 1).is_ok() && insert_item(live, index, to_item).is_ok() {
                        tally.written += 1;
                    } else {
                        tally.kept += 1;
                    }
                }
                _ => tally.kept += 1,
            }
        }
    } else if now == from {
        let cleared = live.clear().is_ok() && fill_list(live, to).is_ok();
        if cleared {
            tally.written += 1;
        } else {
            tally.kept += 1;
        }
    } else {
        tally.kept += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loro::LoroDoc;

    fn anchor(list: &LoroMovableList, id: &str, x: f64) {
        let map = list.push_container(LoroMap::new()).unwrap();
        map.insert("id", id).unwrap();
        map.insert("point", vec![x, 0.0_f64]).unwrap();
    }

    fn object() -> (LoroDoc, LoroMap, LoroMovableList) {
        let doc = LoroDoc::new();
        let meta = doc.get_map("meta");
        meta.insert("width", 1.0).unwrap();
        meta.insert("fill", "red").unwrap();
        let anchors = meta
            .insert_container("anchors", LoroMovableList::new())
            .unwrap();
        anchor(&anchors, "a", 1.0);
        anchor(&anchors, "b", 2.0);
        anchor(&anchors, "c", 3.0);
        (doc, meta, anchors)
    }

    fn state(meta: &LoroMap) -> BTreeMap<String, Node> {
        read_map(meta)
    }

    fn point_of(list: &LoroMovableList, index: usize) -> f64 {
        match list.get(index) {
            Some(ValueOrContainer::Container(Container::Map(map))) => {
                match map.get("point").unwrap().get_deep_value() {
                    LoroValue::List(point) => *point[0].as_double().unwrap(),
                    _ => panic!("no point"),
                }
            }
            _ => panic!("no anchor"),
        }
    }

    fn ids(list: &LoroMovableList) -> Vec<String> {
        read_list(list)
            .iter()
            .map(|n| n.item_id().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn a_field_goes_back_only_where_it_still_holds_the_steps_value() {
        let (_doc, meta, _) = object();
        let before = state(&meta);
        meta.insert("width", 2.0).unwrap();
        meta.insert("fill", "blue").unwrap();
        let after = state(&meta);
        // A peer changes the fill afterwards.
        meta.insert("fill", "green").unwrap();
        let mut tally = Tally::default();
        restore_map(&meta, &after, &before, &mut tally);
        assert_eq!(meta.get("width").unwrap().get_deep_value(), 1.0.into());
        assert_eq!(meta.get("fill").unwrap().get_deep_value(), "green".into());
        assert_eq!(
            tally,
            Tally {
                written: 1,
                kept: 1
            }
        );
    }

    #[test]
    fn a_key_the_step_added_is_removed_and_a_key_it_removed_comes_back() {
        let (_doc, meta, _) = object();
        let before = state(&meta);
        meta.delete("fill").unwrap();
        meta.insert("dash", "1 2").unwrap();
        let after = state(&meta);
        let mut tally = Tally::default();
        restore_map(&meta, &after, &before, &mut tally);
        assert_eq!(state(&meta), before);
        // Redo: the other direction.
        restore_map(&meta, &before, &after, &mut tally);
        assert_eq!(state(&meta), after);
    }

    #[test]
    fn anchors_match_by_id_so_a_peers_edit_of_another_anchor_survives() {
        let (_doc, meta, anchors) = object();
        let before = state(&meta);
        // The step moves anchor a; a peer moves anchor c.
        let first = anchors
            .get(0)
            .unwrap()
            .into_container()
            .unwrap()
            .into_map()
            .unwrap();
        first.insert("point", vec![10.0_f64, 0.0]).unwrap();
        let after = state(&meta);
        let third = anchors
            .get(2)
            .unwrap()
            .into_container()
            .unwrap()
            .into_map()
            .unwrap();
        third.insert("point", vec![30.0_f64, 0.0]).unwrap();
        let mut tally = Tally::default();
        restore_map(&meta, &after, &before, &mut tally);
        assert!((point_of(&anchors, 0) - 1.0).abs() < f64::EPSILON);
        assert!((point_of(&anchors, 2) - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn an_anchor_the_step_inserted_is_removed_unless_a_peer_changed_it() {
        let (_doc, meta, anchors) = object();
        let before = state(&meta);
        let inserted = anchors.insert_container(1, LoroMap::new()).unwrap();
        inserted.insert("id", "x").unwrap();
        inserted.insert("point", vec![9.0_f64, 0.0]).unwrap();
        let after = state(&meta);

        let mut tally = Tally::default();
        restore_map(&meta, &after, &before, &mut tally);
        assert_eq!(ids(&anchors), ["a", "b", "c"]);

        // Redo puts it back at its place; a peer then edits it; undo keeps it.
        restore_map(&meta, &before, &after, &mut tally);
        assert_eq!(ids(&anchors), ["a", "x", "b", "c"]);
        let live = anchors
            .get(1)
            .unwrap()
            .into_container()
            .unwrap()
            .into_map()
            .unwrap();
        live.insert("point", vec![99.0_f64, 0.0]).unwrap();
        let mut kept = Tally::default();
        restore_map(&meta, &after, &before, &mut kept);
        assert_eq!(ids(&anchors), ["a", "x", "b", "c"]);
        assert_eq!(kept.kept, 1);
    }

    #[test]
    fn an_anchor_the_step_deleted_comes_back_after_its_predecessor() {
        let (_doc, meta, anchors) = object();
        let before = state(&meta);
        anchors.delete(1, 1).unwrap();
        let after = state(&meta);
        let mut tally = Tally::default();
        restore_map(&meta, &after, &before, &mut tally);
        assert_eq!(ids(&anchors), ["a", "b", "c"]);
        assert!((point_of(&anchors, 1) - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn a_reordering_is_undone_only_while_nobody_reordered_after_it() {
        let (_doc, meta, anchors) = object();
        let before = state(&meta);
        anchors.mov(0, 2).unwrap();
        let after = state(&meta);
        assert_eq!(ids(&anchors), ["b", "c", "a"]);
        let mut tally = Tally::default();
        restore_map(&meta, &after, &before, &mut tally);
        assert_eq!(ids(&anchors), ["a", "b", "c"]);

        // A peer reorders after the step: the order stays.
        restore_map(&meta, &before, &after, &mut tally);
        anchors.mov(0, 1).unwrap();
        let peer_order = ids(&anchors);
        restore_map(&meta, &after, &before, &mut tally);
        assert_eq!(ids(&anchors), peer_order);
    }

    #[test]
    fn a_container_a_step_created_goes_away_and_comes_back_whole() {
        let (_doc, meta, _anchors) = object();
        let before = state(&meta);
        let extra = meta
            .insert_container("extra", LoroMovableList::new())
            .unwrap();
        let outline = extra.push_container(LoroMap::new()).unwrap();
        outline.insert("closed", true).unwrap();
        let inner = outline
            .insert_container("anchors", LoroMovableList::new())
            .unwrap();
        anchor(&inner, "p", 5.0);
        let after = state(&meta);
        let mut tally = Tally::default();
        restore_map(&meta, &after, &before, &mut tally);
        assert!(meta.get("extra").is_none());
        restore_map(&meta, &before, &after, &mut tally);
        assert_eq!(
            state(&meta),
            after,
            "containers are recreated as containers"
        );
    }

    #[test]
    fn positional_lists_merge_item_fields_when_the_length_is_unchanged() {
        let doc = LoroDoc::new();
        let meta = doc.get_map("meta");
        let extra = meta
            .insert_container("extra", LoroMovableList::new())
            .unwrap();
        for closed in [true, false] {
            let outline = extra.push_container(LoroMap::new()).unwrap();
            outline.insert("closed", closed).unwrap();
        }
        let before = state(&meta);
        let second = extra
            .get(1)
            .unwrap()
            .into_container()
            .unwrap()
            .into_map()
            .unwrap();
        second.insert("closed", true).unwrap();
        let after = state(&meta);
        let mut tally = Tally::default();
        restore_map(&meta, &after, &before, &mut tally);
        assert_eq!(state(&meta), before);
    }
}
