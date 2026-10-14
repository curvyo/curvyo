//! The stored shape of a linked clone (`specs/0049-linked-clone/adrs.md` decision 1): the
//! `shape = "clone"` tag and the one `clone_link` register, read, written and validated here.
//!
//! ```text
//! meta map of a clone:
//!   shape      : "clone"
//!   clone_link : { "origin": "<peer>:<counter>", "matrix": [a, b, c, d, e, f] }
//!                ONE LWW register holding a map *value*, written whole with `insert`,
//!                never as a container, so a merge keeps an origin with its own matrix.
//! ```
//!
//! A clone carries no anchors, style, `opacity` or `rotation`; such keys on a clone, and a
//! `clone_link` on any other node, are ignored residue (ADR 0015 §4).

use std::collections::HashMap;

use loro::{LoroMap, LoroValue, TreeID, ValueOrContainer};

use crate::affine::Affine;
use crate::history::{decode_node, encode_node};
use crate::path_codec::as_f64;
use crate::path_model::NodeId;

/// The `shape` tag of a clone node.
pub(crate) const SHAPE_CLONE: &str = "clone";
/// The meta key of the link register.
pub(crate) const KEY_CLONE_LINK: &str = "clone_link";
const KEY_ORIGIN: &str = "origin";
const KEY_MATRIX: &str = "matrix";

/// What one clone node stores: the object it follows and its own matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CloneLink {
    /// The direct original.
    pub(crate) origin: NodeId,
    /// The clone matrix M.
    pub(crate) matrix: Affine,
}

/// Whether this meta map is tagged as a clone.
pub(crate) fn is_clone_meta(meta: &LoroMap) -> bool {
    matches!(
        meta.get(crate::shape_codec::KEY_SHAPE),
        Some(ValueOrContainer::Value(LoroValue::String(tag))) if *tag == SHAPE_CLONE
    )
}

/// Writes `shape = "clone"` and the whole link register.
pub(crate) fn write_clone(meta: &LoroMap, link: CloneLink) {
    crate::shape_codec::write_shape_tag(meta, SHAPE_CLONE);
    write_clone_link(meta, link);
}

/// Writes the link register as one value.
pub(crate) fn write_clone_link(meta: &LoroMap, link: CloneLink) {
    let matrix: Vec<LoroValue> = link
        .matrix
        .to_array()
        .into_iter()
        .map(LoroValue::Double)
        .collect();
    let value: HashMap<String, LoroValue> = HashMap::from([
        (
            KEY_ORIGIN.to_owned(),
            LoroValue::from(encode_node(TreeID::new(
                link.origin.peer,
                link.origin.counter,
            ))),
        ),
        (KEY_MATRIX.to_owned(), LoroValue::from(matrix)),
    ]);
    // invariant: inserting a plain map value under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY_CLONE_LINK, LoroValue::from(value)).unwrap();
}

/// The link register, or `None` when it is missing or not of the stored shape. Entries beyond
/// ±1e7 are returned: that limit refuses files and edits, never a read.
pub(crate) fn read_clone_link(meta: &LoroMap) -> Option<CloneLink> {
    let Some(ValueOrContainer::Value(value)) = meta.get(KEY_CLONE_LINK) else {
        return None;
    };
    link_from_value(&value)
}

/// The link a stored register value holds, or `None` when it is not of the stored shape.
pub(crate) fn link_from_value(value: &LoroValue) -> Option<CloneLink> {
    let LoroValue::Map(map) = value else {
        return None;
    };
    let LoroValue::String(origin) = map.get(KEY_ORIGIN)? else {
        return None;
    };
    let origin = decode_node(origin)?;
    let LoroValue::List(matrix) = map.get(KEY_MATRIX)? else {
        return None;
    };
    if matrix.len() != 6 {
        return None;
    }
    let mut entries = [0.0; 6];
    for (entry, value) in entries.iter_mut().zip(matrix.iter()) {
        *entry = as_f64(value)?;
    }
    Some(CloneLink {
        origin: NodeId::from_parts(origin.peer, origin.counter),
        matrix: Affine::from_array(entries),
    })
}

/// Open-file validation of a clone node (`0049` criterion 38): a link that is present has an id
/// string as origin and six finite numbers within ±1e7 as matrix. What the origin names is not
/// checked here: a missing or wrong-kind original makes an orphan, not a damaged file. No link at
/// all is merge residue and an orphan too (`adrs.md` decision 1, amended 2026-10-14): the link is
/// written whole, so only a malformed one is damage.
pub(crate) fn validate_clone_node(meta: &LoroMap) -> bool {
    meta.get(KEY_CLONE_LINK).is_none()
        || read_clone_link(meta).is_some_and(|link| link.matrix.is_within_limit())
}

#[cfg(test)]
mod tests {
    use loro::LoroDoc;

    use super::*;

    fn meta() -> (LoroDoc, LoroMap) {
        let doc = LoroDoc::new();
        let tree = doc.get_tree("t");
        let node = tree.create(None).unwrap();
        let meta = tree.get_meta(node).unwrap();
        (doc, meta)
    }

    #[test]
    fn a_written_link_reads_back_to_the_last_bit() {
        let (_doc, meta) = meta();
        let link = CloneLink {
            origin: NodeId::from_parts(u64::MAX, 17),
            matrix: Affine::new(0.1, 0.2, -0.3, 1.0 / 3.0, 1e7, -12.5),
        };
        write_clone(&meta, link);
        assert!(is_clone_meta(&meta));
        assert_eq!(read_clone_link(&meta), Some(link));
        assert!(validate_clone_node(&meta));
    }

    #[test]
    fn the_link_is_one_plain_value_not_a_container() {
        let (_doc, meta) = meta();
        write_clone_link(
            &meta,
            CloneLink {
                origin: NodeId::from_parts(1, 2),
                matrix: Affine::IDENTITY,
            },
        );
        assert!(matches!(
            meta.get(KEY_CLONE_LINK),
            Some(ValueOrContainer::Value(LoroValue::Map(_)))
        ));
    }

    fn with_link(value: LoroValue) -> (LoroDoc, LoroMap) {
        let (doc, meta) = meta();
        crate::shape_codec::write_shape_tag(&meta, SHAPE_CLONE);
        meta.insert(KEY_CLONE_LINK, value).unwrap();
        (doc, meta)
    }

    fn link(origin: LoroValue, matrix: LoroValue) -> LoroValue {
        LoroValue::from(HashMap::from([
            (KEY_ORIGIN.to_owned(), origin),
            (KEY_MATRIX.to_owned(), matrix),
        ]))
    }

    fn numbers(values: &[f64]) -> LoroValue {
        LoroValue::from(values.to_vec())
    }

    #[test]
    fn every_damaged_link_is_refused() {
        let id = LoroValue::from("1:2");
        let six = numbers(&[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        let damaged = [
            ("not a map", Some(LoroValue::from("1:2"))),
            (
                "no origin",
                Some(LoroValue::from(HashMap::from([(
                    KEY_MATRIX.to_owned(),
                    six.clone(),
                )]))),
            ),
            (
                "origin not an id",
                Some(link(LoroValue::from("x"), six.clone())),
            ),
            (
                "origin a number",
                Some(link(LoroValue::I64(3), six.clone())),
            ),
            (
                "no matrix",
                Some(LoroValue::from(HashMap::from([(
                    KEY_ORIGIN.to_owned(),
                    id.clone(),
                )]))),
            ),
            (
                "five entries",
                Some(link(id.clone(), numbers(&[1.0, 0.0, 0.0, 1.0, 0.0]))),
            ),
            (
                "seven entries",
                Some(link(
                    id.clone(),
                    numbers(&[1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0]),
                )),
            ),
            (
                "a string entry",
                Some(link(
                    id.clone(),
                    LoroValue::from(vec![LoroValue::from("1"); 6]),
                )),
            ),
            (
                "NaN",
                Some(link(
                    id.clone(),
                    numbers(&[f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0]),
                )),
            ),
            (
                "infinite",
                Some(link(
                    id.clone(),
                    numbers(&[1.0, 0.0, 0.0, 1.0, f64::INFINITY, 0.0]),
                )),
            ),
            (
                "beyond 1e7",
                Some(link(id.clone(), numbers(&[1.0, 0.0, 0.0, 1.0, 2e7, 0.0]))),
            ),
        ];
        for (name, value) in damaged {
            let (_doc, meta) = match value {
                Some(value) => with_link(value),
                None => meta(),
            };
            assert!(!validate_clone_node(&meta), "{name}");
        }
        let (_doc, meta) = with_link(link(id, numbers(&[1.0, 0.0, 0.0, 1.0, 1e7, -1e7])));
        assert!(validate_clone_node(&meta), "the limit itself is valid");
    }

    /// A clone with no `clone_link` key at all is merge residue, an orphan (criterion 38 as
    /// amended 2026-10-14), not damage.
    #[test]
    fn a_clone_with_no_link_at_all_is_not_damaged() {
        let (_doc, meta) = meta();
        crate::shape_codec::write_shape_tag(&meta, SHAPE_CLONE);
        assert!(validate_clone_node(&meta));
    }

    #[test]
    fn an_integer_entry_reads_as_a_number() {
        let (_doc, meta) = with_link(link(
            LoroValue::from("1:2"),
            LoroValue::from(vec![
                LoroValue::I64(1),
                LoroValue::I64(0),
                LoroValue::I64(0),
                LoroValue::I64(1),
                LoroValue::I64(5),
                LoroValue::I64(0),
            ]),
        ));
        assert_eq!(
            read_clone_link(&meta).map(|link| link.matrix),
            Some(Affine::translation(crate::units::Vec2::new(5.0, 0.0)))
        );
    }
}
