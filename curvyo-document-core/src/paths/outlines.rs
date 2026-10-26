//! Finds the outline of a path that holds an anchor or a segment
//! (`specs/0055-compound-path-node-editing/adrs.md`, "one outline resolver").

use loro::{LoroMap, LoroMovableList};

use crate::document::Document;
use crate::path_model::{AnchorId, NodeId, PathEditError};
use crate::subpath_codec::anchor_positions;

impl Document {
    /// The map that holds `closed` and `anchors` of the outline of `path` that contains `anchor`
    /// (the meta map for the first outline, the `extra_subpaths` element for a further one), and
    /// that outline's anchor list (`specs/0055-compound-path-node-editing/adrs.md`, "one outline
    /// resolver"). Every command that edits one anchor or one outline reads and writes through it,
    /// so it acts on whichever outline holds the anchor.
    ///
    /// # Errors
    /// [`PathEditError::NoSuchPath`] as [`Document::path_parts`];
    /// [`PathEditError::NoSuchAnchor`] if no outline of `path` holds `anchor`.
    pub(crate) fn outline_parts(
        &self,
        path: NodeId,
        anchor: AnchorId,
    ) -> Result<(LoroMap, LoroMovableList), PathEditError> {
        let (meta, _) = self.path_parts(path)?;
        let at = anchor_positions(&meta)
            .at(anchor)
            .ok_or(PathEditError::NoSuchAnchor)?;
        Ok((at.map, at.list))
    }

    /// [`Document::outline_parts`] for a segment: both anchors must be on the same outline.
    ///
    /// # Errors
    /// As [`Document::outline_parts`] for either anchor;
    /// [`PathEditError::NotAnAdjacentSegment`] if the two are on different outlines.
    pub(crate) fn segment_outline(
        &self,
        path: NodeId,
        start: AnchorId,
        end: AnchorId,
    ) -> Result<(LoroMap, LoroMovableList), PathEditError> {
        let (meta, _) = self.path_parts(path)?;
        let positions = anchor_positions(&meta);
        let (Some(start), Some(end)) = (positions.at(start), positions.at(end)) else {
            return Err(PathEditError::NoSuchAnchor);
        };
        if start.outline != end.outline {
            return Err(PathEditError::NotAnAdjacentSegment);
        }
        Ok((start.map, start.list))
    }
}
