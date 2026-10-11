//! The two optional keys 0041 adds to an object (`specs/0041-object-history` criteria 37 to 39):
//! `clone_of`, which names the object a copy was made from and the document version at the copy,
//! and `history_floor`, the version before which the object's history was wiped. Both are plain
//! text registers an older build ignores and a value that cannot be read means "none".

use loro::{Frontiers, LoroDoc, TreeID};

use super::wipe::encode_frontiers;
use crate::document::OBJECTS_TREE;

/// The meta key of the clone origin: `<peer>:<counter>@<version>`.
pub(crate) const KEY_CLONE_OF: &str = "clone_of";
/// The meta key of the floor: the version of the wipe, as [`encode_frontiers`] writes it.
pub(crate) const KEY_HISTORY_FLOOR: &str = "history_floor";

/// The text of a node id inside a header or a register: `<peer>:<counter>`.
pub(crate) fn encode_node(id: TreeID) -> String {
    format!("{}:{}", id.peer, id.counter)
}

/// The inverse of [`encode_node`].
pub(crate) fn decode_node(text: &str) -> Option<TreeID> {
    let (peer, counter) = text.split_once(':')?;
    Some(TreeID::new(peer.parse().ok()?, counter.parse().ok()?))
}

/// The `clone_of` value for a copy of `source` made at `version`.
pub(crate) fn encode_clone_of(source: TreeID, version: &Frontiers) -> String {
    format!("{}@{}", encode_node(source), encode_frontiers(version))
}

/// The inverse of [`encode_frontiers`]; `None` for an empty or malformed text.
pub(crate) fn decode_frontiers(text: &str) -> Option<Frontiers> {
    let ids: Option<Vec<loro::ID>> = text
        .split(',')
        .map(|pair| {
            let (peer, counter) = pair.split_once('@')?;
            Some(loro::ID::new(peer.parse().ok()?, counter.parse().ok()?))
        })
        .collect();
    ids.filter(|ids| !ids.is_empty())
        .map(|ids| ids.into_iter().collect())
}

/// The inverse of [`encode_clone_of`].
pub(crate) fn decode_clone_of(text: &str) -> Option<(TreeID, Frontiers)> {
    let (source, version) = text.split_once('@')?;
    Some((decode_node(source)?, decode_frontiers(version)?))
}

/// The origin of a copy, read from its meta map: the source object and the version at the copy.
pub(crate) fn clone_origin(doc: &LoroDoc, id: TreeID) -> Option<(TreeID, Frontiers)> {
    let tree = doc.get_tree(OBJECTS_TREE);
    let meta = tree.get_meta(id).ok()?;
    match meta.get(KEY_CLONE_OF)?.get_deep_value() {
        loro::LoroValue::String(text) => decode_clone_of(&text),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clone_origin_round_trips() {
        let version: Frontiers = [loro::ID::new(9, 4), loro::ID::new(3, 12)]
            .into_iter()
            .collect();
        let text = encode_clone_of(TreeID::new(7, 21), &version);
        assert_eq!(text, "7:21@3@12,9@4");
        assert_eq!(decode_clone_of(&text), Some((TreeID::new(7, 21), version)));
    }

    #[test]
    fn an_unreadable_value_means_no_origin() {
        for text in ["", "7:21", "7:21@", "x:1@2@3", "7:21@2@x", "@2@3", "7:21@2"] {
            assert_eq!(decode_clone_of(text), None, "{text}");
        }
    }
}
