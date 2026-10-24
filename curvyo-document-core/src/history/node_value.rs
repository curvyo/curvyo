//! A container tree read as plain data, with the container-versus-value distinction kept.
//!
//! A [`Node`] is what the document schema stores: plain registers, maps (an object's meta map, an
//! anchor) and movable lists (anchors, further outlines). `LoroValue::get_deep_value` flattens
//! containers into plain maps and lists, which would lose that distinction, so the values are read
//! container by container.

use std::collections::BTreeMap;

use loro::{Container, LoroMap, LoroMovableList, LoroValue, ValueOrContainer};

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
    /// A text container (a text object's string, `specs/0074-text-objects`) with its
    /// per-character values (ADR 0021 §5).
    Text {
        /// The string.
        string: String,
        /// Its characters' values.
        chars: crate::text_chars::TextChars,
    },
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
            ValueOrContainer::Container(Container::Text(text)) => Self::Text {
                string: text.to_string(),
                chars: crate::text_chars::read_chars(text),
            },
            // No other container kind is stored; its flattened value keeps the data.
            ValueOrContainer::Container(_) => Self::Value(child.get_deep_value()),
        }
    }

    pub(super) fn as_map(&self) -> Option<&BTreeMap<String, Self>> {
        match self {
            Self::Map(map) => Some(map),
            _ => None,
        }
    }

    /// The item id of a map that has one.
    pub(super) fn item_id(&self) -> Option<&str> {
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
