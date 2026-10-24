//! The three-way restore of live containers against the values a step left and the values it
//! started from (ADR 0014 §1: write the old value back only where the current value is still the
//! one the step wrote).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use loro::{Container, LoroMap, LoroMovableList, LoroResult, LoroText, ValueOrContainer};

use super::node_value::{Node, read_list};
use crate::text_chars::{TextChars, carried, read_chars, write_chars};

/// What a restore did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Tally {
    /// Fields, anchors and entries written back.
    pub written: usize,
    /// Fields, anchors and entries left alone because someone changed them after the step.
    pub kept: usize,
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
        Node::Text { string, chars } => {
            fill_text(&map.insert_container(key, LoroText::new())?, string, chars)
        }
    }
}

/// Fills a fresh text container with `string` and its per-character values (ADR 0021 §5).
fn fill_text(text: &LoroText, string: &str, chars: &TextChars) -> LoroResult<()> {
    text.insert(0, string)?;
    write_chars(text, chars);
    Ok(())
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
        Node::Text { string, chars } => fill_text(
            &list.insert_container(index, LoroText::new())?,
            string,
            chars,
        ),
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
            (
                Some(Node::Text {
                    string: from_string,
                    chars: from_chars,
                }),
                Some(Node::Text {
                    string: to_string,
                    chars: to_chars,
                }),
                Some(ValueOrContainer::Container(Container::Text(child))),
            ) => restore_text(
                child,
                (from_string, from_chars),
                (to_string, to_chars),
                tally,
            ),
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

/// Brings the live text `child` from `from` to `to`: the string as a register, back only where
/// nobody typed since, written as the difference so the container (and a peer's merge) stays;
/// then, where the live string is the restored one, each character's value goes back only where
/// it still holds the value the step wrote (ADR 0021 §5).
fn restore_text(
    child: &LoroText,
    (from_string, from_chars): (&String, &TextChars),
    (to_string, to_chars): (&String, &TextChars),
    tally: &mut Tally,
) {
    if from_string != to_string {
        if child.to_string() == *from_string
            && crate::text_codec::apply_string_diff(child, to_string)
        {
            tally.written += 1;
        } else {
            tally.kept += 1;
            return;
        }
    }
    let from_chars = carried(from_chars, from_string, to_string);
    if from_chars == *to_chars || child.to_string() != *to_string {
        return;
    }
    let live = read_chars(child);
    let len = to_string.chars().count();
    let (live, from, to) = (
        live.to_dense(len),
        from_chars.to_dense(len),
        to_chars.to_dense(len),
    );
    let mut target = live.clone();
    let (mut written, mut kept) = (false, false);
    for index in 0..len {
        if from[index] == to[index] {
            continue;
        }
        if live[index] == from[index] {
            target[index] = to[index];
            written = true;
        } else {
            kept = true;
        }
    }
    if written {
        write_chars(child, &TextChars::from_dense(&target));
        tally.written += 1;
    }
    if kept {
        tally.kept += 1;
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

fn restore_keyed_list(live: &LoroMovableList, from: &[Node], to: &[Node], tally: &mut Tally) {
    let from_by_id: HashMap<&str, &Node> = from
        .iter()
        .filter_map(|n| Some((n.item_id()?, n)))
        .collect();
    let to_ids: HashSet<&str> = to.iter().filter_map(Node::item_id).collect();
    // Items the step added: remove them, highest index first, where nobody changed them since.
    let mut removals = Vec::new();
    for (index, item) in read_list(live).iter().enumerate() {
        let Some(id) = item.item_id() else { continue };
        if to_ids.contains(id) {
            continue;
        }
        match from_by_id.get(id) {
            Some(added) if *added == item => removals.push(index),
            Some(_) => tally.kept += 1,
            None => {}
        }
    }
    for index in removals.into_iter().rev() {
        if live.delete(index, 1).is_ok() {
            tally.written += 1;
        } else {
            tally.kept += 1;
        }
    }
    // Items the step removed: put them back after their predecessor.
    let mut order: Vec<Option<String>> = read_list(live)
        .iter()
        .map(|n| n.item_id().map(str::to_owned))
        .collect();
    let find =
        |order: &[Option<String>], id: &str| order.iter().position(|o| o.as_deref() == Some(id));
    for (position, item) in to.iter().enumerate() {
        let Some(id) = item.item_id() else { continue };
        if from_by_id.contains_key(id) || find(&order, id).is_some() {
            continue;
        }
        let index = to[..position]
            .iter()
            .rev()
            .find_map(|earlier| earlier.item_id().and_then(|e| find(&order, e)))
            .map_or(0, |found| found + 1);
        if insert_item(live, index, item).is_ok() {
            order.insert(index, Some(id.to_owned()));
            tally.written += 1;
        } else {
            tally.kept += 1;
        }
    }
    // Items both states hold: merge their fields (no structural change, one index map).
    let index_of: HashMap<&str, usize> = order
        .iter()
        .enumerate()
        .filter_map(|(i, o)| Some((o.as_deref()?, i)))
        .collect();
    for item in to {
        let Some(id) = item.item_id() else { continue };
        let Some(from_item) = from_by_id.get(id) else {
            continue;
        };
        if *from_item == item {
            continue;
        }
        let Some(&index) = index_of.get(id) else {
            tally.kept += 1;
            continue;
        };
        if let (Some(f), Some(t), Some(ValueOrContainer::Container(Container::Map(map)))) =
            (from_item.as_map(), item.as_map(), live.get(index))
        {
            restore_map(&map, f, t, tally);
        }
    }
    restore_item_order(live, &order, from, to, tally);
}

/// If the step moved items and nobody reordered them since, puts the common items back in the
/// order they had before. `order` is the live order of item ids; it is kept in step with the moves.
fn restore_item_order(
    live: &LoroMovableList,
    order: &[Option<String>],
    from: &[Node],
    to: &[Node],
    tally: &mut Tally,
) {
    let ids = |nodes: &[Node]| -> Vec<String> {
        nodes
            .iter()
            .filter_map(|n| n.item_id().map(str::to_owned))
            .collect()
    };
    let mut order: Vec<String> = order.iter().flatten().cloned().collect();
    let only_common = |wanted: &HashSet<&str>, source: &[String]| -> Vec<String> {
        source
            .iter()
            .filter(|id| wanted.contains(id.as_str()))
            .cloned()
            .collect()
    };
    let (from_ids, to_ids) = (ids(from), ids(to));
    let live_set: HashSet<&str> = order.iter().map(String::as_str).collect();
    let common_to = only_common(&live_set, &to_ids);
    let common_set: HashSet<&str> = common_to.iter().map(String::as_str).collect();
    let common_from = only_common(&common_set, &from_ids);
    let common_now = only_common(&common_set, &order);
    if common_from == common_to {
        return;
    }
    if common_now != common_from {
        tally.kept += 1;
        return;
    }
    for (target, id) in common_to.iter().enumerate() {
        let slots: Vec<usize> = order
            .iter()
            .enumerate()
            .filter(|(_, other)| common_set.contains(other.as_str()))
            .map(|(index, _)| index)
            .collect();
        let (Some(&slot), Some(from_index)) =
            (slots.get(target), order.iter().position(|o| o == id))
        else {
            continue;
        };
        if from_index != slot && live.mov(from_index, slot).is_ok() {
            let moved = order.remove(from_index);
            order.insert(slot, moved);
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
    use crate::history::node_value::read_map;
    use loro::{LoroDoc, LoroValue};

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
