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
    /// How many other pages the last size change, drop or delete moved (`specs/0109` ux-notes
    /// §7).
    pub moved: usize,
}
impl PageViewState {
    /// Records how many pages of `before` other than `except` are no longer where they were in
    /// `after`: the auto-layout of the command between them.
    pub fn note_moved(&mut self, before: &[Page], after: &[Page], except: &str) {
        self.moved = after
            .iter()
            .filter(|p| p.id != except)
            .filter(|p| {
                before.iter().any(|b| {
                    b.id == p.id
                        && ((b.origin.x - p.origin.x).abs() > 1e-9
                            || (b.origin.y - p.origin.y).abs() > 1e-9)
                })
            })
            .count();
    }

    /// The screen-reader text of the last auto-layout: "1 page moved.", "2 pages moved.", empty
    /// when it moved no page (`specs/0109` ux-notes §7).
    #[must_use]
    pub fn moved_text(&self) -> String {
        match self.moved {
            0 => String::new(),
            1 => "1 page moved.".to_owned(),
            n => format!("{n} pages moved."),
        }
    }
    /// Active frame, with deterministic fallback after a document change.
    #[must_use]
    pub fn active<'a>(&self, pages: &'a [Page]) -> Option<&'a Page> {
        pages
            .iter()
            .find(|p| Some(&p.id) == self.active.as_ref())
            .or_else(|| pages.first())
    }
    /// Records a canvas press without inventing context for panel actions. A press that `hit`
    /// an object keeps the active page (`specs/0106` criterion 2); a press on empty canvas inside
    /// a frame activates it.
    pub fn press(&mut self, pages: &[Page], point: Point, hit: bool) {
        let page = self
            .active(pages)
            .filter(|p| p.contains(point))
            .or_else(|| pages.iter().rev().find(|p| p.contains(point)));
        self.context = page.map(|p| p.id.clone());
        if hit {
            return;
        }
        if let Some(page) = page {
            self.active = Some(page.id.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{DocumentSize, PageGrid, PageKind};

    fn page(id: &str, x: f64) -> Page {
        Page {
            id: id.to_owned(),
            name: id.to_owned(),
            origin: Point::new(x, 0.0),
            size: DocumentSize::from_mm(10.0, 10.0),
            grid: PageGrid::DEFAULT,
            kind: PageKind::Drawing,
        }
    }

    #[test]
    fn the_moved_text_counts_the_other_pages_that_moved() {
        let mut state = PageViewState::default();
        let before = [page("a", 0.0), page("b", 20.0), page("c", 40.0)];
        state.note_moved(
            &before,
            &[page("a", -5.0), page("b", 25.0), page("c", 45.0)],
            "a",
        );
        assert_eq!(state.moved_text(), "2 pages moved.");
        state.note_moved(&before, &[page("a", 0.0), page("c", 20.0)], "b");
        assert_eq!(state.moved_text(), "1 page moved.");
        state.note_moved(&before, &before, "a");
        assert_eq!(state.moved_text(), "");
    }
}
