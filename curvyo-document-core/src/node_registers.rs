//! The registers every node of the object tree may carry, `name`, `visible`, `locked`,
//! `opacity` and `guide`, plus the `layer` marker of a group: their keys, their defaults, the
//! cleaning of a name and their validation on open (`specs/0050-layers-and-objects-panel/adrs.md`
//! decision 1, ADR 0015 §2, ADR 0019 §2).
//!
//! An absent key reads as its default (no name, visible, unlocked, opacity 1, not a guide, not a
//! layer), so a file from an earlier build opens unchanged.

use loro::{LoroMap, LoroValue, ValueOrContainer};

use crate::style_model::Opacity;

/// The register that holds a node's name (a string).
pub(crate) const KEY_NAME: &str = "name";
/// The register that holds whether a node is shown (a bool; absent is `true`).
pub(crate) const KEY_VISIBLE: &str = "visible";
/// The register that holds whether a node is locked (a bool; absent is `false`).
pub(crate) const KEY_LOCKED: &str = "locked";
/// The register that holds a node's opacity (a number in 0..=1; absent is 1).
pub(crate) const KEY_OPACITY: &str = "opacity";
/// The register that marks a path or shape as a guide (a bool; absent is `false`, ADR 0019 §2).
pub(crate) const KEY_GUIDE: &str = "guide";
/// The register that marks a top-level group as a layer (a bool).
pub(crate) const KEY_LAYER: &str = "layer";

/// The most characters a name keeps (`specs/0050-layers-and-objects-panel` criterion 16).
pub const MAX_NAME_CHARS: usize = 128;

/// The registers of one node as stored, read with their defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeRegisters {
    /// The node's name, cleaned for display; `None` when it has none (the panel shows a default).
    pub name: Option<String>,
    /// Whether the node itself is shown (its own flag, not the effective one).
    pub visible: bool,
    /// Whether the node itself is locked (its own flag, not the effective one).
    pub locked: bool,
    /// The node's own opacity.
    pub opacity: Opacity,
    /// Whether the node itself is marked as a guide (`specs/0046-document-shapes`).
    pub guide: bool,
}

impl Default for NodeRegisters {
    fn default() -> Self {
        Self {
            name: None,
            visible: true,
            locked: false,
            opacity: Opacity::OPAQUE,
            guide: false,
        }
    }
}

/// Whether `c` is a bidirectional control character (U+202A to U+202E, U+2066 to U+2069), which
/// a name never keeps (criterion 17).
const fn is_bidi_control(c: char) -> bool {
    matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// A name as it is stored and shown: control and bidirectional control characters dropped,
/// leading and trailing white space trimmed, at most [`MAX_NAME_CHARS`] characters. `None` when
/// nothing is left, which means "no name" (criteria 16 and 17).
#[must_use]
pub fn clean_name(raw: &str) -> Option<String> {
    let kept: String = raw
        .chars()
        .filter(|&c| !c.is_control() && !is_bidi_control(c))
        .collect();
    let cut: String = kept.trim().chars().take(MAX_NAME_CHARS).collect();
    let cut = cut.trim_end();
    (!cut.is_empty()).then(|| cut.to_owned())
}

fn bool_register(meta: &LoroMap, key: &str) -> Option<bool> {
    match meta.get(key) {
        Some(ValueOrContainer::Value(LoroValue::Bool(value))) => Some(value),
        _ => None,
    }
}

/// The number stored in a numeric register, an integer included.
#[allow(clippy::cast_precision_loss)] // only 0 and 1 are valid integers; both are exact
fn number_register(value: &ValueOrContainer) -> Option<f64> {
    match value {
        ValueOrContainer::Value(LoroValue::Double(number)) => Some(*number),
        ValueOrContainer::Value(LoroValue::I64(number)) => Some(*number as f64),
        _ => None,
    }
}

/// Reads the registers of one node, each absent or unreadable one at its default. A file that
/// opened has passed [`registers_are_valid`], so the defaults only matter for merge residue.
pub(crate) fn read_registers(meta: &LoroMap) -> NodeRegisters {
    let name = match meta.get(KEY_NAME) {
        Some(ValueOrContainer::Value(LoroValue::String(text))) => clean_name(&text),
        _ => None,
    };
    let opacity = meta
        .get(KEY_OPACITY)
        .as_ref()
        .and_then(number_register)
        .and_then(|value| Opacity::new(value).ok())
        .unwrap_or(Opacity::OPAQUE);
    NodeRegisters {
        name,
        visible: bool_register(meta, KEY_VISIBLE).unwrap_or(true),
        locked: bool_register(meta, KEY_LOCKED).unwrap_or(false),
        opacity,
        guide: bool_register(meta, KEY_GUIDE).unwrap_or(false),
    }
}

/// Whether `meta` holds the `layer` marker. Only a group at the top level of the tree is a layer;
/// the caller checks both (a marker deeper down is merge residue and reads as a plain group).
pub(crate) fn meta_has_layer_marker(meta: &LoroMap) -> bool {
    bool_register(meta, KEY_LAYER) == Some(true)
}

/// Whether the registers of one node are well formed (criterion 47): `name` a string, `visible`,
/// `locked`, `guide` and `layer` booleans, `opacity` a finite number in 0..=1, and `layer: true` only on a
/// group. Absent keys are valid.
pub(crate) fn registers_are_valid(meta: &LoroMap, is_group: bool) -> bool {
    let name_ok = match meta.get(KEY_NAME) {
        None | Some(ValueOrContainer::Value(LoroValue::String(_))) => true,
        Some(_) => false,
    };
    let flag_ok = |key: &str| {
        matches!(
            meta.get(key),
            None | Some(ValueOrContainer::Value(LoroValue::Bool(_)))
        )
    };
    let opacity_ok = meta.get(KEY_OPACITY).is_none_or(|value| {
        number_register(&value).is_some_and(|number| Opacity::new(number).is_ok())
    });
    let layer_ok = !meta_has_layer_marker(meta) || is_group;
    name_ok
        && flag_ok(KEY_VISIBLE)
        && flag_ok(KEY_LOCKED)
        && flag_ok(KEY_GUIDE)
        && flag_ok(KEY_LAYER)
        && opacity_ok
        && layer_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_trimmed_cut_and_loses_control_and_bidi_characters() {
        assert_eq!(clean_name("  Cut  "), Some("Cut".to_owned()));
        assert_eq!(
            clean_name("a\u{202E}b\u{2066}c\u{7}d\ne"),
            Some("abcde".to_owned())
        );
        assert_eq!(clean_name(" \t\n "), None);
        assert_eq!(clean_name(""), None);
        let long = "x".repeat(200);
        assert_eq!(clean_name(&long).map(|n| n.chars().count()), Some(128));
        assert_eq!(
            clean_name("<img src=x onerror=alert(1)>"),
            Some("<img src=x onerror=alert(1)>".to_owned()),
            "markup is text, kept as it is"
        );
    }

    #[test]
    fn a_name_cut_at_128_does_not_end_in_white_space() {
        let name = format!("{} tail", "y".repeat(127));
        assert_eq!(clean_name(&name), Some("y".repeat(127)));
    }

    #[test]
    fn absent_registers_read_as_their_defaults() {
        let doc = loro::LoroDoc::new();
        let meta = doc.get_map("m");
        assert_eq!(read_registers(&meta), NodeRegisters::default());
        assert!(registers_are_valid(&meta, false));
    }

    #[test]
    fn mistyped_or_out_of_range_registers_are_invalid() {
        let cases: [(&str, LoroValue); 9] = [
            (KEY_GUIDE, LoroValue::I64(1)),
            (KEY_GUIDE, LoroValue::from("true")),
            (KEY_NAME, LoroValue::I64(3)),
            (KEY_VISIBLE, LoroValue::from("yes")),
            (KEY_LOCKED, LoroValue::Double(1.0)),
            (KEY_OPACITY, LoroValue::Double(1.5)),
            (KEY_OPACITY, LoroValue::Double(f64::NAN)),
            (KEY_OPACITY, LoroValue::from("0.5")),
            (KEY_LAYER, LoroValue::I64(1)),
        ];
        for (key, value) in cases {
            let doc = loro::LoroDoc::new();
            let meta = doc.get_map("m");
            meta.insert(key, value.clone()).unwrap();
            assert!(!registers_are_valid(&meta, true), "{key} = {value:?}");
        }
    }

    #[test]
    fn a_stored_guide_flag_reads_as_it_is() {
        let doc = loro::LoroDoc::new();
        let meta = doc.get_map("m");
        meta.insert(KEY_GUIDE, true).unwrap();
        assert!(read_registers(&meta).guide);
        assert!(registers_are_valid(&meta, false));
        meta.insert(KEY_GUIDE, false).unwrap();
        assert!(!read_registers(&meta).guide);
    }

    #[test]
    fn a_layer_marker_on_a_leaf_is_invalid_and_on_a_group_valid() {
        let doc = loro::LoroDoc::new();
        let meta = doc.get_map("m");
        meta.insert(KEY_LAYER, true).unwrap();
        assert!(!registers_are_valid(&meta, false));
        assert!(registers_are_valid(&meta, true));
    }

    #[test]
    fn an_integer_opacity_of_zero_or_one_is_a_number() {
        let doc = loro::LoroDoc::new();
        let meta = doc.get_map("m");
        meta.insert(KEY_OPACITY, 0_i64).unwrap();
        assert!(registers_are_valid(&meta, false));
        assert_eq!(read_registers(&meta).opacity.get(), 0.0);
    }
}
