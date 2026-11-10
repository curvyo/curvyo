//! The write of a page layout inside the triggering command's step: each moved frame with the
//! objects its page owns (ADR 0051 §4 to §6).

use std::collections::BTreeMap;

use crate::document::ROOT_MAP;
use crate::page_frame::{Frame, write_frame};
use crate::page_layout::Layout;
use crate::pages::{Page, write};
use crate::{Document, NodeId, ObjectEditError, Vec2};

/// The objects each page owns, by page id: the leaves of every top-level object whose centre the
/// page holds, read by the caller from the state before the command (ADR 0051 §4). A page
/// without an entry carries nothing; placements and fixtures are never listed, they follow
/// their frame (ADR 0042 §4).
pub type PageContents = BTreeMap<String, Vec<NodeId>>;

impl Document {
    /// Refuses a translation of `objects` by `delta` that would leave a coordinate out of range;
    /// nothing is written.
    pub(crate) fn check_page_translation(
        &self,
        objects: &[NodeId],
        delta: Vec2,
    ) -> Result<(), ObjectEditError> {
        let moved = objects
            .iter()
            .map(|id| {
                self.object(*id)
                    .map(|o| o.translated(delta))
                    .ok_or(ObjectEditError::NoSuchObject)
            })
            .collect::<Result<Vec<_>, _>>()?;
        crate::transform_objects::check_numbers(&moved, false)
    }

    /// Refuses `layout` when one of its pages could not carry its objects; nothing is written.
    pub(crate) fn check_layout(
        &self,
        layout: &Layout,
        contents: &PageContents,
    ) -> Result<(), ObjectEditError> {
        for (id, by) in &layout.moves {
            if let Some(objects) = contents.get(id) {
                self.check_page_translation(objects, *by)?;
            }
        }
        Ok(())
    }

    /// Writes every move of `layout` over `pages` (the frames before the step): the frame and
    /// the page's owned objects from `contents`, every downstream clone held back (ADR 0016 §2).
    /// Uncommitted; the caller's batch holds it.
    pub(crate) fn write_layout(
        &self,
        pages: &[Page],
        layout: &Layout,
        contents: &PageContents,
    ) -> Result<(), ObjectEditError> {
        let root = self.loro().get_map(ROOT_MAP);
        for (id, by) in &layout.moves {
            let Some(page) = pages.iter().find(|p| p.id == *id) else {
                continue;
            };
            if let Some(objects) = contents.get(id).filter(|o| !o.is_empty()) {
                self.translate_page_objects(objects, *by)?;
            }
            write(&root, id, "deleted", false);
            write_frame(
                &root,
                id,
                Frame {
                    origin: page.origin.translated(*by),
                    size: page.size,
                },
            );
        }
        Ok(())
    }
}
