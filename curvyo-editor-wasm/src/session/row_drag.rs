//! The drag of a page row out of the page list onto the canvas (`specs/0099-sketch-pages`
//! criteria 8 to 10, `adrs.md` note 4): view state until the drop, which writes one step.

use curvyo_document_core::{Affine, NodeId, ObjectSnapshot, PageKind, Point, Vec2};
use curvyo_render_core::DrawList;
use curvyo_ui_core::{SnapProbe, object_bounds};
use serde::Serialize;

use super::Session;

/// Past this many member leaves a page row's ghost is its frame only (UX notes §3).
const GHOST_LEAVES: usize = 2_000;

/// The id the frame ghost carries: no node's (counters are never negative).
fn ghost_id() -> NodeId {
    // invariant: the literal is a valid `NodeId` wire value.
    #[allow(clippy::unwrap_used)]
    serde_json::from_value(serde_json::json!({ "peer": "0", "counter": -1 })).unwrap()
}

/// The frame of `page` as a closed path, the ghost of its row drag.
fn frame_path(page: &curvyo_document_core::Page) -> ObjectSnapshot {
    let (x, y) = (page.origin.x, page.origin.y);
    let (w, h) = (page.size.width.as_mm(), page.size.height.as_mm());
    let corner = |k: u64, px: f64, py: f64| {
        curvyo_document_core::NewAnchor::corner(
            curvyo_document_core::AnchorId::new(0, k),
            Point::new(px, py),
        )
    };
    ObjectSnapshot::Path(curvyo_document_core::PathSnapshot {
        id: ghost_id(),
        closed: true,
        style: curvyo_document_core::Style::default(),
        anchors: vec![
            corner(0, x, y),
            corner(1, x + w, y),
            corner(2, x + w, y + h),
            corner(3, x, y + h),
        ],
        extra_subpaths: Vec::new(),
        rotation: curvyo_document_core::Angle::from_radians(0.0),
    })
}

/// What a page row drag carries.
#[derive(Debug, Clone, PartialEq)]
pub enum RowDragSource {
    /// A drawing page: a drop on a laser page places it (`specs/0100` criterion 13).
    DrawingPage {
        /// The drawing page whose row is dragged.
        page: String,
    },
    /// The sketch of a sketch page: a drop places a linked clone of it.
    Sketch {
        /// The sketch page whose row is dragged.
        page: String,
        /// Its owning sketch, `None` while it has nothing to draw.
        sketch: Option<NodeId>,
    },
}

/// The badge beside the pointer, keyed to front-end words (UX notes §2): one enum for the row
/// drag and the cloning drop (`specs/0100-bring-content-in` `adrs.md` note 4); `Clone` reads
/// "Linked clone" for both.
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
    /// An Alt release over a laser page moves the objects ("Move", `0100` criterion 2).
    Move,
    /// A Ctrl release over a laser page copies them ("Copy", `0100` criterion 2).
    Copy,
    /// A drawing page row over a laser page: a drop places the page ("Placed page", `0100`
    /// criterion 13).
    PlacedPage,
    /// A drawing page row anywhere else: a drop places nothing ("Only on a laser page",
    /// `0100` criterion 14).
    OnlyOnLaserPage,
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
    /// For a drawing page: its frame as a path and the frame's centre.
    ghost: Option<(ObjectSnapshot, Point)>,
    /// A drawing page's members as drawn at the drag's begin, only translated (`0100`).
    content: Vec<ObjectSnapshot>,
    /// The clone's box centre for a drop now (snapped), with the pointer on the canvas.
    at: Option<Point>,
    answer: RowDragAnswer,
}

impl Session {
    /// Begins dragging the row of page `page`; `false` for a page whose row places nothing.
    pub fn row_drag_begin(&mut self, page: &str) -> bool {
        if self.document.page_kind(page) == Some(PageKind::Drawing) {
            return self.drawing_row_drag_begin(page);
        }
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
            content: Vec::new(),
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
        if matches!(drag.source, RowDragSource::DrawingPage { .. }) {
            let laser = under.filter(|p| match &p.kind {
                PageKind::Output { process, .. } => self.process_enabled(process),
                _ => false,
            });
            let Some(laser) = laser else {
                return self.set_row_drag(Some(point), None, DropBadge::OnlyOnLaserPage, true);
            };
            let target = Some(laser.id.clone());
            let Some((object, centre)) = &drag.ghost else {
                return self.set_row_drag(Some(point), target, DropBadge::PlacedPage, false);
            };
            let delta = centre.vector_to(point);
            let (min, max) = object_bounds(object);
            let snap = self.resolve_snapping(&SnapProbe::box_edges(
                min.translated(delta),
                max.translated(delta),
            ));
            return self.set_row_drag(
                Some(point.translated(snap.offset)),
                target,
                DropBadge::PlacedPage,
                false,
            );
        }
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
        if let RowDragSource::DrawingPage { page } = &drag.source {
            if drag.answer.badge != DropBadge::PlacedPage {
                return false;
            }
            let (Some(target), Some(at)) = (drag.answer.target.as_deref(), drag.at) else {
                return false;
            };
            return self.place_page_at(page, target, at);
        }
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
        let mut ghost = vec![moved];
        if let RowDragSource::DrawingPage { .. } = drag.source {
            ghost.extend(
                drag.content
                    .iter()
                    .map(|o| o.translated(centre.vector_to(at))),
            );
        }
        curvyo_render_core::build_drag_ghost(&ghost, self.view())
    }

    /// Begins the row drag of drawing page `page`: its ghost is the page's frame and the
    /// outlines of its members, read once here and only translated (`adrs.md` note 2).
    fn drawing_row_drag_begin(&mut self, page: &str) -> bool {
        let Some(source) = self.document.pages().into_iter().find(|p| p.id == page) else {
            return false;
        };
        let read = self.document_read();
        let membership = self.page_membership();
        let mut content: Vec<ObjectSnapshot> =
            curvyo_ui_core::placement_members(&read.drawn, &source, &read.guides, &membership)
                .into_iter()
                .map(|at| read.drawn[at].clone())
                .collect();
        // Past 2,000 leaves the ghost is the frame only (UX notes §3).
        if content.len() > GHOST_LEAVES {
            content.clear();
        }
        let frame = frame_path(&source);
        let centre = Point::new(
            source.origin.x + source.size.width.as_mm() / 2.0,
            source.origin.y + source.size.height.as_mm() / 2.0,
        );
        self.begin_snapping_drag_for_shape();
        self.row_drag = Some(RowDrag {
            source: RowDragSource::DrawingPage {
                page: page.to_owned(),
            },
            ghost: Some((frame, centre)),
            content,
            at: None,
            answer: RowDragAnswer {
                badge: DropBadge::None,
                target: None,
                faint: false,
            },
        });
        true
    }

    /// Places drawing page `source` on laser page `laser` with its frame's centre at `at`, as
    /// one step "Place page", activates the laser page and selects the placed page. Refused
    /// (nothing written) when it would not overlap the laser page.
    pub(super) fn place_page_at(&mut self, source: &str, laser: &str, at: Point) -> bool {
        let pages = self.document.pages();
        let (Some(src), Some(out)) = (
            pages.iter().find(|p| p.id == source),
            pages.iter().find(|p| p.id == laser),
        ) else {
            return false;
        };
        let matrix = Affine::translation(Vec2::new(
            at.x - out.origin.x - src.size.width.as_mm() / 2.0,
            at.y - out.origin.y - src.size.height.as_mm() / 2.0,
        ));
        let placed =
            self.recording_unguarded(|this| this.document.place_page(laser, source, matrix).ok());
        let Some(id) = placed else {
            return false;
        };
        self.activate_page(laser);
        self.node.clear_selection();
        self.selection.set(&[id]);
        true
    }

    /// Place page (`specs/0100` criterion 15): places drawing page `source` at the centre of
    /// the active laser page. `false` when the active page is no laser page.
    pub fn place_page_on_active(&mut self, source: &str) -> bool {
        let laser = self.active_page();
        if !matches!(laser.kind, PageKind::Output { .. }) {
            return false;
        }
        let centre = Point::new(
            laser.origin.x + laser.size.width.as_mm() / 2.0,
            laser.origin.y + laser.size.height.as_mm() / 2.0,
        );
        self.place_page_at(source, &laser.id, centre)
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
        let placed = self
            .recording_unguarded(|this| this.document.place_linked_clone(sketch, translation).ok());
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
