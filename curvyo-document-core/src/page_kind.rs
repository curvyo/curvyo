//! The fixed kind of a page (ADR 0042 §1, §2), read from its `kind` register or its owner link.

use loro::{LoroTree, LoroValue};
use serde::{Serialize, Serializer};

use crate::manufacturing_codec::{KEY_PROCESS, is_manufacturing_node, string};
use crate::path_codec::node_exists;
use crate::path_model::NodeId;

/// The page record field holding the kind (`page.<id>.kind`).
pub(crate) const KIND_FIELD: &str = "kind";
/// The stored kind of a drawing page.
pub(crate) const DRAWING: &str = "drawing";
/// The stored kind of a sketch page.
pub(crate) const SKETCH: &str = "sketch";

/// What a page is; fixed from its creation (criterion 5).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PageKind {
    /// A drawing page: the vector tools.
    #[default]
    Drawing,
    /// A sketch page: it may own one sketch (ADR 0042 §8).
    Sketch,
    /// An output page: its machine data is the manufacturing `manufacturing` of `process`.
    Output {
        /// The manufacturing node that holds the page's machine data.
        manufacturing: NodeId,
        /// The process id (`curvyo.laser`).
        process: String,
    },
    /// A kind this build does not know, kept as stored (R-MFG-010).
    Unknown(String),
}

impl PageKind {
    /// Whether the page's size comes from elsewhere and no size command may write it
    /// (ADR 0042 §3).
    #[must_use]
    pub const fn fixed_size(&self) -> bool {
        matches!(self, Self::Output { .. } | Self::Unknown(_))
    }

    /// Whether this is a drawing page.
    #[must_use]
    pub const fn is_drawing(&self) -> bool {
        matches!(self, Self::Drawing)
    }

    /// The manufacturing of an output page.
    #[must_use]
    pub const fn manufacturing(&self) -> Option<NodeId> {
        match self {
            Self::Output { manufacturing, .. } => Some(*manufacturing),
            _ => None,
        }
    }

    /// The stored text of a page kind a command may create, `None` for the others.
    pub(crate) const fn stored(&self) -> Option<&'static str> {
        match self {
            Self::Drawing => Some(DRAWING),
            Self::Sketch => Some(SKETCH),
            _ => None,
        }
    }
}

impl Serialize for PageKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Drawing => serializer.serialize_str(DRAWING),
            Self::Sketch => serializer.serialize_str(SKETCH),
            Self::Output { process, .. } => serializer.serialize_str(process),
            Self::Unknown(text) => serializer.serialize_str(text),
        }
    }
}

/// The kind of a page from its `kind` and `owner` registers (ADR 0042 §2): an owner naming a live
/// manufacturing makes an output page and the `kind` register is ignored; else no `kind` or
/// `"drawing"` is Drawing, `"sketch"` Sketch, another text Unknown, and a value that is not a text
/// reads as Drawing (never rewritten).
pub(crate) fn read_kind(
    kind: Option<&LoroValue>,
    owner: Option<&LoroValue>,
    tree: &LoroTree,
) -> PageKind {
    if let Some(LoroValue::String(text)) = owner
        && let Some(node) = crate::history::lineage::decode_node(text)
        && node_exists(tree, node)
        && is_manufacturing_node(tree, node)
        && let Ok(meta) = tree.get_meta(node)
    {
        return PageKind::Output {
            manufacturing: NodeId::from_parts(node.peer, node.counter),
            process: string(&meta, KEY_PROCESS).unwrap_or_default(),
        };
    }
    match kind {
        Some(LoroValue::String(text)) => match text.as_str() {
            DRAWING => PageKind::Drawing,
            SKETCH => PageKind::Sketch,
            other => PageKind::Unknown(other.to_owned()),
        },
        _ => PageKind::Drawing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(kind: Option<&LoroValue>) -> PageKind {
        let doc = loro::LoroDoc::new();
        read_kind(kind, None, &doc.get_tree("paths"))
    }

    #[test]
    fn a_kind_register_reads_by_the_rule_of_adr_0042() {
        assert_eq!(read(None), PageKind::Drawing);
        assert_eq!(read(Some(&"drawing".into())), PageKind::Drawing);
        assert_eq!(read(Some(&"sketch".into())), PageKind::Sketch);
        assert_eq!(
            read(Some(&"template".into())),
            PageKind::Unknown("template".into())
        );
        assert_eq!(read(Some(&LoroValue::Double(3.0))), PageKind::Drawing);
    }

    #[test]
    fn an_owner_that_names_no_live_manufacturing_is_no_output_page() {
        let doc = loro::LoroDoc::new();
        let owner = LoroValue::from("7:3");
        let kind = read_kind(Some(&"sketch".into()), Some(&owner), &doc.get_tree("paths"));
        assert_eq!(kind, PageKind::Sketch);
    }

    #[test]
    fn only_drawing_and_sketch_are_created_and_only_output_and_unknown_are_fixed() {
        assert_eq!(PageKind::Drawing.stored(), Some("drawing"));
        assert_eq!(PageKind::Sketch.stored(), Some("sketch"));
        assert_eq!(PageKind::Unknown("x".into()).stored(), None);
        assert!(PageKind::Unknown("x".into()).fixed_size());
        assert!(!PageKind::Sketch.fixed_size());
    }
}
