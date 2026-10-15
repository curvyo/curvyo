//! The stored shape of a linked clone (`specs/0049-linked-clone/adrs.md` decision 1): the
//! `shape = "clone"` tag, the one `clone_link` register and the two override flags of ADR 0016
//! §1, read, written and validated here.
//!
//! ```text
//! meta map of a clone:
//!   shape         : "clone"
//!   clone_link    : { "origin": "<peer>:<counter>", "matrix": [a, b, c, d, e, f] }
//!                   ONE LWW register holding a map *value*, written whole with `insert`,
//!                   never as a container, so a merge keeps an origin with its own matrix.
//!   own_placement : true        absent = false; held back when its original is transformed
//!   own_style     : true        absent = false; then the ordinary style keys (`style_codec`,
//!                               absent key = default) and `opacity` (absent = 1) are its own
//! ```
//!
//! A clone carries no anchors or `rotation`, and without `own_style` no style and no `opacity`;
//! such keys on a clone, and a `clone_link` or override flag on any other node, are ignored
//! residue (ADR 0015 §4, ADR 0016 §5).

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
/// The meta key of the own-placement flag (ADR 0016 §1).
pub(crate) const KEY_OWN_PLACEMENT: &str = "own_placement";
/// The meta key of the own-style marker (ADR 0016 §1).
pub(crate) const KEY_OWN_STYLE: &str = "own_style";
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

/// The two override flags of one clone node as stored, absent or mistyped ones `false` (a file
/// that opened has passed [`validate_clone_node`], so only merge residue reads a default).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct CloneOverrides {
    /// Held back when its original is transformed (`0057` criterion 3).
    pub(crate) own_placement: bool,
    /// Its style keys and `opacity` are its own (`0057` criterion 11).
    pub(crate) own_style: bool,
}

/// Reads the override flags of a clone node.
pub(crate) fn read_overrides(meta: &LoroMap) -> CloneOverrides {
    let flag = |key| {
        matches!(
            meta.get(key),
            Some(ValueOrContainer::Value(LoroValue::Bool(true)))
        )
    };
    CloneOverrides {
        own_placement: flag(KEY_OWN_PLACEMENT),
        own_style: flag(KEY_OWN_STYLE),
    }
}

/// Sets or removes one override flag: `true` is stored, `false` is the absent key.
pub(crate) fn write_flag(meta: &LoroMap, key: &str, on: bool) {
    if on {
        // invariant: inserting a bool under a known key on an attached map cannot fail.
        #[allow(clippy::unwrap_used)]
        meta.insert(key, true).unwrap();
    } else if meta.get(key).is_some() {
        // invariant: deleting a key just confirmed present on an attached map cannot fail.
        #[allow(clippy::unwrap_used)]
        meta.delete(key).unwrap();
    }
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
/// written whole, so only a malformed one is damage. The override flags are booleans when present;
/// with `own_style` true the style keys must be valid as on a path (`0057` criterion 38); without
/// it they are residue and not checked.
pub(crate) fn validate_clone_node(meta: &LoroMap) -> bool {
    let link_ok = meta.get(KEY_CLONE_LINK).is_none()
        || read_clone_link(meta).is_some_and(|link| link.matrix.is_within_limit());
    let flag_ok = |key| {
        matches!(
            meta.get(key),
            None | Some(ValueOrContainer::Value(LoroValue::Bool(_)))
        )
    };
    link_ok
        && flag_ok(KEY_OWN_PLACEMENT)
        && flag_ok(KEY_OWN_STYLE)
        && (!read_overrides(meta).own_style || crate::style_validation::style_is_valid(meta))
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
    fn override_flags_read_write_and_validate() {
        let (_doc, meta) = with_link(link(
            LoroValue::from("1:2"),
            numbers(&[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
        ));
        assert_eq!(read_overrides(&meta), CloneOverrides::default());
        write_flag(&meta, KEY_OWN_PLACEMENT, true);
        write_flag(&meta, KEY_OWN_STYLE, true);
        assert_eq!(
            read_overrides(&meta),
            CloneOverrides {
                own_placement: true,
                own_style: true
            }
        );
        assert!(validate_clone_node(&meta));
        write_flag(&meta, KEY_OWN_PLACEMENT, false);
        assert!(
            meta.get(KEY_OWN_PLACEMENT).is_none(),
            "false is the absent key"
        );
        write_flag(&meta, KEY_OWN_PLACEMENT, false);
        assert!(meta.get(KEY_OWN_PLACEMENT).is_none());
    }

    #[test]
    fn a_mistyped_flag_or_a_damaged_own_style_is_refused() {
        let good = || {
            link(
                LoroValue::from("1:2"),
                numbers(&[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
            )
        };
        for (key, value) in [
            (KEY_OWN_PLACEMENT, LoroValue::I64(1)),
            (KEY_OWN_STYLE, LoroValue::from("true")),
            (KEY_OWN_STYLE, LoroValue::Double(1.0)),
        ] {
            let (_doc, meta) = with_link(good());
            meta.insert(key, value.clone()).unwrap();
            assert!(!validate_clone_node(&meta), "{key} = {value:?}");
        }
        let (_doc, meta) = with_link(good());
        meta.insert(crate::style_codec::KEY_STROKE_WIDTH, -1.0)
            .unwrap();
        assert!(validate_clone_node(&meta), "residue without own style");
        meta.insert(KEY_OWN_STYLE, true).unwrap();
        assert!(!validate_clone_node(&meta), "a damaged own style");
        meta.insert(KEY_OWN_STYLE, false).unwrap();
        assert!(validate_clone_node(&meta), "a stored false is not damage");
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
