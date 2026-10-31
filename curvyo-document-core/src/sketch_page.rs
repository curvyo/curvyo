//! Page sketch claims elect one editable original without changing graph intent.
use crate::document::ROOT_MAP;
use crate::{Affine, Document, NodeId, Point, SketchEditError, Style};
use curvyo_sketch_core::Sketch;

pub(crate) const PAGE_KEY: &str = "sketch_page";
pub(crate) const OWNERSHIP_CONFLICT: &str = "This object is not the page’s owning sketch; convert or remove it to resolve the competing creation";
const OWNER_FIELD: &str = "sketch_owner";

impl Document {
    /// The live owning original of a page; reading never repairs dangling claims.
    #[must_use]
    pub fn page_sketch(&self, page: &str) -> Option<NodeId> {
        if !self.pages().iter().any(|p| p.id == page) {
            return None;
        }
        let root = self.loro().get_map(ROOT_MAP);
        let id = crate::sketch_codec::atomic(&root, &crate::pages::key(page, OWNER_FIELD)).ok()?;
        let meta = self.sketch_meta(id).ok()?;
        let source: String = crate::sketch_codec::atomic(&meta, PAGE_KEY).ok()?;
        (source == page).then_some(id)
    }

    /// Original assignment provenance; this field never elects the owner itself.
    #[must_use]
    pub fn sketch_page(&self, id: NodeId) -> Option<String> {
        crate::sketch_codec::atomic(&self.sketch_meta(id).ok()?, PAGE_KEY).ok()
    }

    /// Creates the page's original with geometry and claim in one history step.
    /// The local zero starts at the global page centre; subsequent resize does not recenter it.
    /// # Errors
    /// Refuses missing/occupied pages or invalid graph geometry without writing.
    pub fn create_page_sketch(
        &self,
        page: &str,
        graph: &Sketch,
        style: &Style,
    ) -> Result<NodeId, SketchEditError> {
        let frame = self
            .pages()
            .into_iter()
            .find(|p| p.id == page)
            .ok_or_else(|| SketchEditError("Page not found".into()))?;
        if self.page_sketch(page).is_some() {
            return Err(SketchEditError("This page already has a sketch".into()));
        }
        let center = Point::new(
            frame.origin.x + frame.size.width.as_mm() / 2.0,
            frame.origin.y + frame.size.height.as_mm() / 2.0,
        );
        self.batch("add_sketch", |doc| {
            let id = doc.create_sketch(
                graph,
                Affine::translation(crate::Vec2::new(center.x, center.y)),
                style,
            )?;
            let meta = doc.sketch_meta(id)?;
            meta.insert(crate::node_registers::KEY_NAME, "Sketch")
                .map_err(|error| SketchEditError(error.to_string()))?;
            crate::sketch_codec::write(&meta, PAGE_KEY, &page).map_err(SketchEditError)?;
            crate::sketch_codec::write(
                &doc.loro().get_map(ROOT_MAP),
                &crate::pages::key(page, OWNER_FIELD),
                &id,
            )
            .map_err(SketchEditError)?;
            Ok(id)
        })
    }

    /// Whether assigned graph intent lost its page claim or its page was removed.
    #[must_use]
    pub fn sketch_ownership_conflict(&self, id: NodeId) -> bool {
        self.sketch_page(id)
            .is_some_and(|page| self.page_sketch(&page) != Some(id))
    }

    pub(crate) fn clear_sketch_claim(&self, id: NodeId) {
        let Some(page) = self.sketch_page(id) else {
            return;
        };
        let root = self.loro().get_map(ROOT_MAP);
        let key = crate::pages::key(&page, OWNER_FIELD);
        if crate::sketch_codec::atomic::<NodeId>(&root, &key).ok() == Some(id) {
            // invariant: attached root map and a register known to exist.
            #[allow(clippy::unwrap_used)]
            root.delete(&key).unwrap();
        }
    }
}

/// Reject malformed present claims while permitting repairable missing references.
pub(crate) fn valid_claims(root: &loro::LoroMap) -> bool {
    let mut valid = true;
    root.for_each(|key, _| {
        if key.starts_with("page.") && key.ends_with(".sketch_owner") {
            valid &= crate::sketch_codec::atomic::<NodeId>(root, key).is_ok();
        }
    });
    valid
}
