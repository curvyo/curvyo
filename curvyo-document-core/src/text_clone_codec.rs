//! The stored shape of a text clone (ADR 0022 §4, `specs/0080-text-on-path/adrs.md` decision 12),
//! read, written and validated here.
//!
//! ```text
//! meta map of a text clone (format 23):
//!   shape          : "text"
//!   text_source    : "<peer>:<counter>"     the source text, one register
//!   text_placement : [a, b, c, d, e, f]     its own straight placement (Release, criterion 41)
//!   + the on-path keys of `text_path_codec`, its own
//!   own_style      : true                   absent = false; then the style keys are its own
//!   + `name`, `visible`, `locked`, `opacity`, `guide`, its own
//! ```
//!
//! A text clone stores no string, font, size, layout or kerns: it reads them from its source. A
//! style key on a clone without `own_style` is ignored residue (ADR 0016 §5).

use loro::{LoroMap, LoroText, LoroValue, TreeID, ValueOrContainer};
use serde::Serialize;

use crate::history::{decode_node, encode_node};
use crate::path_model::NodeId;
use crate::style_model::Style;
use crate::text_model::TextSnapshot;
use crate::text_path_model::TextOnPath;

/// The meta key of the source reference.
const KEY_SOURCE: &str = "text_source";

/// What a text clone follows (`TextSnapshot::source`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TextSource {
    /// The source text.
    pub node: NodeId,
    /// Whether its fill and stroke are its own (`own_style`, `0057` criterion 11).
    pub own_style: bool,
}

/// What a text clone node stores of its own.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextCloneParts {
    /// The node.
    pub(crate) id: NodeId,
    /// The source text.
    pub(crate) source: NodeId,
    /// Its own straight placement.
    pub(crate) placement: crate::affine::Affine,
    /// Its own on-path values, as stored.
    pub(crate) on_path: Option<TextOnPath>,
    /// Its own style when it has the marker.
    pub(crate) own_style: Option<Style>,
}

impl TextCloneParts {
    /// The clone as shown: `source` with this clone's id, placement, on-path values and, with
    /// own style, style. The carrier is resolved by the caller.
    pub(crate) fn fill(&self, source: &TextSnapshot) -> TextSnapshot {
        let mut text = source.clone();
        text.id = self.id;
        text.placement = self.placement;
        text.on_path.clone_from(&self.on_path);
        if let Some(style) = &self.own_style {
            text.style = style.clone();
        }
        text.outline = None;
        text.source = Some(TextSource {
            node: self.source,
            own_style: self.own_style.is_some(),
        });
        text
    }
}

/// The source a text node follows, or `None` for a source or a straight text.
pub(crate) fn read_source(meta: &LoroMap) -> Option<NodeId> {
    let Some(ValueOrContainer::Value(LoroValue::String(node))) = meta.get(KEY_SOURCE) else {
        return None;
    };
    decode_node(&node).map(|id| NodeId::from_parts(id.peer, id.counter))
}

/// Whether `meta` carries a source key, readable or not.
pub(crate) fn has_source_key(meta: &LoroMap) -> bool {
    meta.get(KEY_SOURCE).is_some()
}

/// Writes the source reference.
pub(crate) fn write_source(meta: &LoroMap, source: NodeId) {
    // invariant: inserting a plain value under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(
        KEY_SOURCE,
        LoroValue::from(encode_node(TreeID::new(source.peer, source.counter))),
    )
    .unwrap();
}

/// What the text clone node `id` stores, or `None` when its source or placement cannot be read.
pub(crate) fn read_parts(id: NodeId, meta: &LoroMap) -> Option<TextCloneParts> {
    Some(TextCloneParts {
        id,
        source: read_source(meta)?,
        placement: crate::text_codec::read_placement(meta)?,
        on_path: crate::text_path_codec::read_on_path(meta),
        own_style: crate::clone_codec::read_overrides(meta)
            .own_style
            .then(|| crate::style_codec::read_style(meta)),
    })
}

/// Turns the text clone `meta` into an independent text with the values of `shown` (the clone
/// as read): the string, kerns, face, size and layout written, the style written unless it is
/// its own already, the source reference and the marker removed. Uncommitted.
pub(crate) fn unlink_meta(meta: &LoroMap, shown: &TextSnapshot) {
    let own = crate::clone_codec::read_overrides(meta).own_style;
    // invariant: inserting a new container under a known key of an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    let string = meta
        .insert_container(crate::text_codec::KEY_STRING, LoroText::new())
        .unwrap();
    // invariant: inserting into a fresh attached text at position 0 cannot fail.
    #[allow(clippy::unwrap_used)]
    string.insert(0, &shown.string).unwrap();
    crate::text_chars::write_chars(&string, &shown.chars);
    crate::text_codec::write_font(meta, &shown.font);
    crate::text_codec::write_size(meta, shown.size);
    crate::text_codec::write_layout(meta, &shown.layout);
    if !own {
        let stored = crate::style_codec::read_style(meta);
        crate::style_codec::write_changes(meta, &stored, &shown.style);
    }
    crate::clone_codec::write_flag(meta, crate::clone_codec::KEY_OWN_STYLE, false);
    // invariant: deleting a key just confirmed present on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.delete(KEY_SOURCE).unwrap();
}

/// Open-file validation of a text clone: a readable source and placement, the style keys valid
/// as on a path when it has its own style.
pub(crate) fn validate_text_clone(meta: &LoroMap) -> bool {
    read_source(meta).is_some()
        && crate::text_codec::read_placement(meta).is_some()
        && (!crate::clone_codec::read_overrides(meta).own_style
            || crate::style_validation::style_is_valid(meta))
}

#[cfg(test)]
mod tests {
    use loro::LoroDoc;

    use super::*;
    use crate::affine::Affine;

    fn written() -> (LoroDoc, LoroMap) {
        let doc = LoroDoc::new();
        let meta = doc.get_map("meta");
        crate::shape_codec::write_shape_tag(&meta, crate::text_codec::SHAPE_TEXT);
        write_source(&meta, NodeId::from_parts(3, 4));
        crate::text_codec::write_placement(&meta, Affine::IDENTITY);
        (doc, meta)
    }

    #[test]
    fn a_written_clone_reads_back_and_validates() {
        let (_doc, meta) = written();
        assert!(validate_text_clone(&meta));
        let parts = read_parts(NodeId::from_parts(1, 2), &meta).unwrap();
        assert_eq!(parts.source, NodeId::from_parts(3, 4));
        assert_eq!(parts.own_style, None);
    }

    #[test]
    fn a_damaged_source_or_placement_is_refused() {
        let (_doc, meta) = written();
        meta.insert(KEY_SOURCE, LoroValue::from("not a node"))
            .unwrap();
        assert!(!validate_text_clone(&meta));
        let (_doc, meta) = written();
        meta.insert(KEY_SOURCE, LoroValue::Double(1.0)).unwrap();
        assert!(!validate_text_clone(&meta));
        let (_doc, meta) = written();
        meta.delete("text_placement").unwrap();
        assert!(!validate_text_clone(&meta));
    }
}
