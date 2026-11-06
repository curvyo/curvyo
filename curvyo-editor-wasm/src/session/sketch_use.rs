//! How a sketch page's sketch is used elsewhere (`specs/0099-sketch-pages` criteria 11 to 14):
//! its clones counted by page, Place clone on, Select clones, and the way back from a clone.

use std::collections::BTreeSet;

use curvyo_document_core::{NodeId, ObjectSnapshot, Page, PageKind, Point};
use curvyo_sketch_core::SolveStatus;
use curvyo_ui_core::{Reach, object_bounds};
use serde::Serialize;

use super::Session;
use super::clones::CloneResult;

/// How many linked clones follow a sketch and on how many pages they lie (criterion 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct SketchUsage {
    /// Every clone whose chain's root original is the sketch, pasteboard ones included.
    pub clones: usize,
    /// The pages those clones are members of.
    pub pages: usize,
}

/// The state of a sketch page's sketch for its page row (criterion 14).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "dof")]
pub enum SketchRowState {
    /// No geometry and no parameter (`0098` criterion 3).
    Empty,
    /// Fully constrained.
    Full,
    /// This many degrees of freedom.
    Free(usize),
    /// Conflicting constraints.
    Conflict,
    /// The solver failed.
    Failed,
}

impl Session {
    /// Whether the solved sketch `id` has something to draw (`adrs.md` note 5): its outline has
    /// at least one anchor, so a clone, a ghost and a conversion draw something.
    pub(super) fn sketch_has_geometry(&self, id: NodeId) -> bool {
        self.document
            .sketch(id)
            .is_some_and(|s| s.resolved.all_anchors().next().is_some())
    }

    /// The clones whose chain's root is `sketch`, in paint order.
    fn clones_of_sketch(&self, sketch: NodeId) -> Vec<ObjectSnapshot> {
        self.document_read()
            .leaves
            .iter()
            .filter(|o| matches!(o, ObjectSnapshot::Clone(c) if c.chain.root == sketch))
            .cloned()
            .collect()
    }

    /// The usage of `sketch` (criterion 12; `adrs.md` note 6).
    #[must_use]
    pub fn sketch_usage(&self, sketch: NodeId) -> SketchUsage {
        let clones = self.clones_of_sketch(sketch);
        if clones.is_empty() {
            return SketchUsage::default();
        }
        let pages = self.document.pages();
        let membership = self.page_membership();
        let mut on: BTreeSet<&str> = BTreeSet::new();
        for clone in &clones {
            for page in
                membership.pages_of(&pages, clone.id(), object_bounds(clone), Reach::Touches)
            {
                on.insert(page.id.as_str());
            }
        }
        SketchUsage {
            clones: clones.len(),
            pages: on.len(),
        }
    }

    /// The state the row of sketch page `page` shows (criterion 14).
    pub(super) fn sketch_row_state(&self, page: &Page) -> Option<SketchRowState> {
        if page.kind != PageKind::Sketch {
            return None;
        }
        let Some(snapshot) = self
            .document
            .page_sketch(&page.id)
            .and_then(|id| self.document.sketch(id))
        else {
            return Some(SketchRowState::Empty);
        };
        let graph = &snapshot.graph;
        if graph.entities.keys().all(|id| *id == graph.origin) && graph.parameters.0.is_empty() {
            return Some(SketchRowState::Empty);
        }
        Some(match snapshot.status {
            Some(SolveStatus::FullyConstrained) => SketchRowState::Full,
            Some(SolveStatus::UnderConstrained(n)) => SketchRowState::Free(n),
            Some(SolveStatus::Conflicting) => SketchRowState::Conflict,
            Some(SolveStatus::Preview) => SketchRowState::Free(0),
            Some(SolveStatus::SolverFailure) | None => SketchRowState::Failed,
        })
    }

    /// The owning sketch of the active page while it is a sketch page.
    fn active_page_sketch(&self) -> Option<NodeId> {
        self.document.page_sketch(&self.active_page().id)
    }

    /// Place clone on (criterion 11): a linked clone of the active sketch page's sketch with its
    /// box centre at the centre of drawing or laser page `page`, one step; the page becomes active
    /// and the clone selected.
    pub fn place_sketch_clone_on(&mut self, page: &str) -> bool {
        let Some(sketch) = self
            .active_page_sketch()
            .filter(|&id| self.sketch_has_geometry(id))
        else {
            return false;
        };
        let Some(target) = self
            .document
            .pages()
            .into_iter()
            .find(|p| p.id == page && p.kind != PageKind::Sketch)
        else {
            return false;
        };
        let Some(object) = self.object_read().get(sketch).cloned() else {
            return false;
        };
        let (min, max) = object_bounds(&object);
        let centre = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
        let goal = Point::new(
            target.origin.x + target.size.width.as_mm() / 2.0,
            target.origin.y + target.size.height.as_mm() / 2.0,
        );
        self.place_sketch_clone(sketch, centre.vector_to(goal), Some(page))
    }

    /// Select clones in the Sketches tab (criterion 12): the clones of the active sketch on the
    /// first page in list order that has one, which becomes active; with every clone on the
    /// pasteboard the last active drawing or laser page becomes active and those are selected.
    /// The `0049` refusals for hidden or locked clones apply and then no page changes.
    pub fn select_sketch_clones(&mut self) -> CloneResult {
        let Some(sketch) = self.active_page_sketch() else {
            return CloneResult::ignored();
        };
        let clones = self.clones_of_sketch(sketch);
        if clones.is_empty() {
            return CloneResult::ignored();
        }
        let pages = self.document.pages();
        let membership = self.page_membership();
        let first = pages.iter().find_map(|page| {
            let on: Vec<NodeId> = clones
                .iter()
                .filter(|c| membership.is_member(page, c.id(), object_bounds(c), Reach::Touches))
                .map(ObjectSnapshot::id)
                .collect();
            (!on.is_empty()).then(|| (page.id.clone(), on))
        });
        let (page, targets) = first.unwrap_or_else(|| {
            let page = self
                .last_work_page
                .clone()
                .filter(|id| pages.iter().any(|p| p.id == *id))
                .or_else(|| {
                    pages
                        .iter()
                        .find(|p| p.kind != PageKind::Sketch)
                        .map(|p| p.id.clone())
                })
                .unwrap_or_else(|| self.active_page().id);
            (page, clones.iter().map(ObjectSnapshot::id).collect())
        });
        self.open_page_with(&page, &targets)
    }

    /// Activates `page` and selects `targets` through the `0049` rules; a refusal changes no page.
    fn open_page_with(&mut self, page: &str, targets: &[NodeId]) -> CloneResult {
        let inert = std::sync::Arc::clone(&self.document_read().inert);
        if targets.iter().all(|id| inert.contains(id)) {
            return self.select_targets(targets, "clone");
        }
        self.activate_page(page);
        self.select_targets(targets, "clone")
    }

    /// Select original or a double-click on clones of fixed sketches (criterion 13): the first
    /// of their sketch pages in list order becomes active with nothing selected. `false` when no
    /// selected clone follows a fixed sketch.
    pub(super) fn open_sketch_page_of(&mut self, clones: &[NodeId]) -> bool {
        let read = self.object_read();
        let sketches: Vec<NodeId> = clones
            .iter()
            .filter_map(|&id| match read.get(id) {
                Some(ObjectSnapshot::Clone(clone)) => Some(clone.chain.root),
                _ => None,
            })
            .filter(|&root| self.document.is_fixed_sketch(root))
            .collect();
        if sketches.is_empty() {
            return false;
        }
        let pages = self.document.pages();
        let Some(page) = pages.iter().find(|p| {
            p.kind == PageKind::Sketch
                && self
                    .document
                    .page_sketch(&p.id)
                    .is_some_and(|s| sketches.contains(&s))
        }) else {
            return false;
        };
        let id = page.id.clone();
        self.selection.clear();
        self.activate_page(&id);
        self.selection.clear();
        true
    }

    /// The selection without fixed sketches: outside the Sketch workbench no command takes one
    /// (ADR 0043 §2).
    pub(super) fn drop_fixed_sketches_from_selection(&mut self) {
        let ids = self.selection.ids();
        if ids.iter().any(|&id| self.document.is_fixed_sketch(id)) {
            let kept: Vec<NodeId> = ids
                .iter()
                .copied()
                .filter(|&id| !self.document.is_fixed_sketch(id))
                .collect();
            self.selection.set(&kept);
        }
    }

    /// A Layers row press on a fixed sketch (criterion 4): its sketch page becomes active, as by
    /// Select original, and nothing is selected. `false` for any other row.
    pub(super) fn press_fixed_sketch_row(&mut self, id: NodeId) -> bool {
        let Some(page) = self
            .document
            .sketch_page(id)
            .filter(|_| self.document.is_fixed_sketch(id))
        else {
            return false;
        };
        self.selection.clear();
        self.activate_page(&page);
        self.selection.clear();
        true
    }
}

#[cfg(test)]
#[path = "sketch_use_tests.rs"]
pub(in crate::session) mod tests;
