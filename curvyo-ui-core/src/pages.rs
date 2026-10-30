//! Ephemeral page activation and spatial selection context.
use curvyo_document_core::{Page, Point};
use std::collections::HashSet;
/// Per-user page controls, excluded from document history and persistence.
#[derive(Debug, Default)]
pub struct PageViewState {
    /// Preferred active identity; reads fall back after undo or deletion.
    pub active: Option<String>,
    /// Page under the most recent canvas press, or pasteboard/unset.
    pub context: Option<String>,
    /// Frames whose next move leaves all objects in place.
    pub frame_only: HashSet<String>,
}
impl PageViewState {
    /// Active frame, with deterministic fallback after a document change.
    #[must_use]
    pub fn active<'a>(&self, pages: &'a [Page]) -> Option<&'a Page> {
        pages
            .iter()
            .find(|p| Some(&p.id) == self.active.as_ref())
            .or_else(|| pages.first())
    }
    /// Records a canvas press without inventing context for panel actions.
    pub fn press(&mut self, pages: &[Page], point: Point) {
        let page = self
            .active(pages)
            .filter(|p| p.contains(point))
            .or_else(|| pages.iter().rev().find(|p| p.contains(point)));
        self.context = page.map(|p| p.id.clone());
        if let Some(page) = page {
            self.active = Some(page.id.clone());
        }
    }
}
