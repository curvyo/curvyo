//! The drag of a page row out of the page list onto the canvas (`specs/0099-sketch-pages`
//! criteria 8 to 10, `adrs.md` note 4): view state until the drop, which writes one step.

use curvyo_document_core::{Affine, NodeId, ObjectSnapshot, PageKind, Point, Vec2};
use curvyo_render_core::DrawList;
use curvyo_ui_core::{SnapProbe, object_bounds};
use serde::Serialize;

use super::Session;

/// What a page row drag carries. `0100` adds its drawing page rows here.
#[derive(Debug, Clone, PartialEq)]
pub enum RowDragSource {
    /// The sketch of a sketch page: a drop places a linked clone of it.
    Sketch {
        /// The sketch page whose row is dragged.
        page: String,
        /// Its owning sketch, `None` while it has nothing to draw.
        sketch: Option<NodeId>,
    },
}

/// The badge beside the pointer, keyed to front-end words (UX notes §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DropBadge {
    /// No badge: the pointer is off the canvas.
    None,
    /// A drop places a linked clone ("Linked clone").
    Clone,
    /// Over a sketch page: a drop places nothing ("Not on a sketch page").
    NotOnSketchPage,
    /// The sketch has nothing to draw ("Bracket has no sketch yet", with the page name).
    NoSketch,
}

/// The answer to one pointer move of a row drag.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RowDragAnswer {
    /// The badge to show.
    pub badge: DropBadge,
    /// The page under the pointer that a drop would activate, if any.
    pub target: Option<String>,
    /// Whether the ghost is drawn faint (over a sketch page).
    pub faint: bool,
}

/// A row drag in flight: nothing is written before the drop.
#[derive(Debug, Clone)]
pub(super) struct RowDrag {
    source: RowDragSource,
    /// The sketch's solved outline at the drag's begin and its box centre; never re-solved.
    ghost: Option<(ObjectSnapshot, Point)>,
    /// The clone's box centre for a drop now (snapped), with the pointer on the canvas.
    at: Option<Point>,
    answer: RowDragAnswer,
}

impl Session {
    /// Begins dragging the row of page `page`; `false` for a page whose row places nothing.
    pub fn row_drag_begin(&mut self, page: &str) -> bool {
        if self.document.page_kind(page) != Some(PageKind::Sketch) {
            return false;
        }
        let sketch = self
            .document
            .page_sketch(page)
            .filter(|&id| self.sketch_has_geometry(id));
        let ghost = sketch.and_then(|id| {
            let object = self.object_read().get(id).cloned()?;
            let (min, max) = object_bounds(&object);
            Some((
                object,
                Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y)),
            ))
        });
        if ghost.is_some() {
            self.begin_snapping_drag_for_shape();
        }
        self.row_drag = Some(RowDrag {
            source: RowDragSource::Sketch {
                page: page.to_owned(),
                sketch,
            },
            ghost,
            at: None,
            answer: RowDragAnswer {
                badge: DropBadge::None,
                target: None,
                faint: false,
            },
        });
        true
    }

    /// One pointer move of the row drag: `point` on the canvas in document space, `None` while
    /// the pointer is over the panels or the page list.
    pub fn row_drag_move(&mut self, point: Option<Point>) -> RowDragAnswer {
        let Some(drag) = self.row_drag.clone() else {
            return RowDragAnswer {
                badge: DropBadge::None,
                target: None,
                faint: false,
            };
        };
        let Some(point) = point else {
            return self.set_row_drag(None, None, DropBadge::None, false);
        };
        let pages = self.document.pages();
        let under = pages.iter().rev().find(|p| p.contains(point));
        let Some((object, centre)) = &drag.ghost else {
            return self.set_row_drag(None, None, DropBadge::NoSketch, false);
        };
        if under.is_some_and(|p| p.kind == PageKind::Sketch) {
            let at = point;
            return self.set_row_drag(Some(at), None, DropBadge::NotOnSketchPage, true);
        }
        // The box centre follows the pointer and snaps as a Select move does (`0060`).
        let delta = centre.vector_to(point);
        let (min, max) = object_bounds(object);
        let snap = self.resolve_snapping(&SnapProbe::box_edges(
            min.translated(delta),
            max.translated(delta),
        ));
        let at = point.translated(snap.offset);
        let target = under.map(|p| p.id.clone());
        self.set_row_drag(Some(at), target, DropBadge::Clone, false)
    }

    fn set_row_drag(
        &mut self,
        at: Option<Point>,
        target: Option<String>,
        badge: DropBadge,
        faint: bool,
    ) -> RowDragAnswer {
        let answer = RowDragAnswer {
            badge,
            target,
            faint,
        };
        if let Some(drag) = self.row_drag.as_mut() {
            drag.at = at;
            drag.answer = answer.clone();
        }
        answer
    }

    /// Ends the row drag: over a drawing page, a laser page or the pasteboard it places the
    /// linked clone in one step "Place sketch clone", activates the page under the pointer and
    /// selects the clone; anywhere else it writes nothing. Returns whether a clone was placed.
    pub fn row_drag_drop(&mut self) -> bool {
        let Some(drag) = self.row_drag.take() else {
            return false;
        };
        self.end_snapping_drag();
        let (RowDragSource::Sketch { sketch, .. }, Some((_, centre)), Some(at)) =
            (&drag.source, &drag.ghost, drag.at)
        else {
            return false;
        };
        if drag.answer.badge != DropBadge::Clone {
            return false;
        }
        let Some(sketch) = *sketch else {
            return false;
        };
        self.place_sketch_clone(sketch, centre.vector_to(at), drag.answer.target.as_deref())
    }

    /// Cancels the row drag (Escape, a release over a panel): nothing is written.
    pub fn row_drag_cancel(&mut self) {
        if self.row_drag.take().is_some() {
            self.end_snapping_drag();
        }
    }

    /// The page a row drag would drop onto now, for its frame cue.
    pub(super) fn row_drag_target(&self) -> Option<&str> {
        self.row_drag.as_ref()?.answer.target.as_deref()
    }

    /// The ghost of a row drag in flight: the sketch's solved outline with its box centre at the
    /// pointer, in the "Drag ghost" look; nothing off the canvas or for an empty sketch.
    pub(super) fn row_drag_ghost_list(&self) -> DrawList {
        let Some(drag) = &self.row_drag else {
            return DrawList::default();
        };
        let (Some((object, centre)), Some(at)) = (&drag.ghost, drag.at) else {
            return DrawList::default();
        };
        let moved = object.translated(centre.vector_to(at));
        curvyo_render_core::build_drag_ghost(std::slice::from_ref(&moved), self.view())
    }

    /// Places a linked clone of `sketch` moved by `translation` as one step, activates `target`
    /// (the pasteboard keeps the active page) and selects the clone. A place past the clone
    /// limit is a refused drop: nothing changes.
    pub(super) fn place_sketch_clone(
        &mut self,
        sketch: NodeId,
        translation: Vec2,
        target: Option<&str>,
    ) -> bool {
        if !Affine::translation(translation).is_within_limit() {
            return false;
        }
        // The clone goes on top of the active layer, outside any entered group; undo leaves an
        // empty selection (UX notes §2).
        self.leave_to_level(0);
        self.selection.clear();
        self.document.set_draw_parent(self.draw_parent(None));
        let placed =
            self.recording(|this| this.document.place_linked_clone(sketch, translation).ok());
        let Some(clone) = placed else {
            return false;
        };
        if let Some(page) = target {
            self.activate_page(page);
        }
        self.node.clear_selection();
        self.selection.set(&[clone]);
        true
    }
}

#[cfg(test)]
#[path = "row_drag_tests.rs"]
mod tests;
