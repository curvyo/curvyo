//! The Toolpath view of the active laser page (`specs/0029` criteria 8, 9, 11, 12, 36 f, 37; ADR
//! 0064 §10; UX notes §3, §4): the per-page toggle, exclusive with the Operations view, the
//! job's document-space look built once per job, and its draw list cached by view and head.

use std::cell::RefCell;
use std::sync::Arc;

use curvyo_document_core::{Point, Vec2, ViewTransform};
use curvyo_laser_core::job::{Job, MoveKind, PointUm, Ring};
use curvyo_laser_core::operations::OperationKind;
use curvyo_laser_core::settings::Role;
use curvyo_render_core::{
    DrawList, ToolpathArrow, ToolpathHead, ToolpathLook, ToolpathSegment, ToolpathStroke,
    build_toolpath,
};
use serde::Serialize;

use super::super::Session;
use super::job_cache::PageJob;

/// The toolpath look of the last job and the draw list of the last frame, with their keys; the
/// raster items' dots are drawn as pictures beside it (`raster_view.rs`).
#[derive(Debug, Default)]
pub(super) struct DrawCache {
    look: RefCell<Option<HeldLook>>,
    list: RefCell<Option<(DrawKey, DrawList)>>,
    /// How often the look was built, for the tests.
    #[cfg(test)]
    pub(super) looks: std::cell::Cell<usize>,
    /// How often the draw list was built, for the tests.
    #[cfg(test)]
    pub(super) lists: std::cell::Cell<usize>,
}

/// The look of one job generation with the ring of each ring.
type HeldLook = (u64, Arc<ToolpathLook>, Vec<Ring>);

/// What one frame's draw list is built from.
#[derive(Debug, Clone, Copy, PartialEq)]
struct DrawKey {
    generation: u64,
    view: ViewTransform,
    fraction: f64,
}

/// One ring's label and where the frontend puts it, in CSS px of the canvas (UX P26).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RingLabel {
    /// The operation row's digit; `None` for the fixture jobs of one kind (the fixture glyph).
    pub digit: Option<u32>,
    /// The ring's centre, x.
    pub x: f64,
    /// The ring's centre, y.
    pub y: f64,
}

/// The stroke of a burn of `kind` with the set `role`.
const fn burn_stroke(kind: OperationKind, role: Role) -> ToolpathStroke {
    match (kind, role) {
        (OperationKind::Cut, _) => ToolpathStroke::Cut,
        (OperationKind::CutOut, _) => ToolpathStroke::CutOut,
        (OperationKind::Score, _) | (OperationKind::EngraveOutline, Role::Outline) => {
            ToolpathStroke::Score
        }
        (OperationKind::Engrave | OperationKind::Image, _)
        | (OperationKind::EngraveOutline, Role::Main) => ToolpathStroke::Engrave,
    }
}

/// A builder of the look of one job, the moves walked once in order.
struct LookBuilder<'a> {
    job: &'a Job,
    origin: Vec2,
    look: ToolpathLook,
    runs: Vec<Ring>,
    /// The open burn path: its arrow's index in `look.arrows`.
    open_arrow: Option<usize>,
    ringed: Vec<bool>,
    lines: Vec<u32>,
    areas: Vec<(usize, curvyo_document_core::NodeId)>,
}

impl LookBuilder<'_> {
    fn point(&self, p: PointUm) -> Point {
        p.to_mm().translated(self.origin)
    }

    /// The burn kind of move `index`, `None` when it is no burn.
    fn burn(&self, index: usize) -> Option<(ToolpathStroke, usize, curvyo_document_core::NodeId)> {
        let m = self.job.moves.get(index)?;
        let (MoveKind::Burn { role, node } | MoveKind::Sweep { role, node, .. }) = m.kind else {
            return None;
        };
        let block = self.job.blocks.get(m.block)?;
        Some((burn_stroke(block.kind, role), m.block, node))
    }

    /// Whether the travel `index` lies inside one Engrave area: between two scan lines of the
    /// same leaf (UX P5, not drawn).
    fn inside_area(&self, index: usize) -> bool {
        let before = (0..index)
            .rev()
            .find(|&i| !matches!(self.job.moves[i].kind, MoveKind::RunOut(_)));
        let after = (index + 1..self.job.moves.len())
            .find(|&i| !matches!(self.job.moves[i].kind, MoveKind::RunOut(_)));
        match (
            before.and_then(|i| self.burn(i)),
            after.and_then(|i| self.burn(i)),
        ) {
            (Some((ToolpathStroke::Engrave, b0, n0)), Some((ToolpathStroke::Engrave, b1, n1))) => {
                b0 == b1 && n0 == n1
            }
            _ => false,
        }
    }

    fn add(&mut self, index: usize, from: Point, to: Point) {
        let m = self.job.moves[index];
        let stroke = match m.kind {
            MoveKind::RunOut(_) | MoveKind::Z(_) => {
                self.open_arrow = None;
                return;
            }
            MoveKind::Travel => {
                self.open_arrow = None;
                if self.inside_area(index) {
                    return;
                }
                ToolpathStroke::Travel
            }
            MoveKind::Burn { .. } | MoveKind::Sweep { .. } => {
                // invariant: a burn move's block is in the job.
                let Some((stroke, block, node)) = self.burn(index) else {
                    return;
                };
                self.burn_start(index, from, to, (stroke, block, node));
                // A sweep is drawn as its dots (the item's dot picture, `raster_view.rs`), never
                // as a line over its gaps (`specs/0123` criterion 20); its ring and its one
                // arrowhead stay.
                if matches!(m.kind, MoveKind::Sweep { .. }) {
                    return;
                }
                stroke
            }
        };
        let (line, spacing_mm) = if stroke == ToolpathStroke::Engrave {
            let block = &self.job.blocks[m.block];
            let line = self.lines[m.block];
            self.lines[m.block] += 1;
            (line, f64::from(block.numbers.interval.hundredths()) / 100.0)
        } else {
            (0, 0.0)
        };
        self.look.segments.push(ToolpathSegment {
            index,
            from,
            to,
            stroke,
            line,
            spacing_mm,
        });
    }

    /// Opens a burn path at move `index` when none is open: its ring (the run's first), its
    /// arrowhead (one per Engrave area); grows the open path's length.
    fn burn_start(
        &mut self,
        index: usize,
        from: Point,
        to: Point,
        (stroke, block, node): (ToolpathStroke, usize, curvyo_document_core::NodeId),
    ) {
        let length = from.vector_to(to).length();
        if let Some(open) = self.open_arrow {
            self.look.arrows[open].length_mm += length;
            return;
        }
        if !self.ringed[block] {
            self.ringed[block] = true;
            self.look.rings.push(from);
            self.runs.push(self.job.blocks[block].ring);
        }
        if stroke == ToolpathStroke::Engrave {
            if self.areas.contains(&(block, node)) {
                return;
            }
            self.areas.push((block, node));
        }
        if length <= 0.0 {
            return;
        }
        self.look.arrows.push(ToolpathArrow {
            index,
            at: from,
            direction: from.vector_to(to).scaled(1.0 / length),
            length_mm: length,
            stroke,
        });
        self.open_arrow = Some(self.look.arrows.len() - 1);
    }
}

/// The look of `job` in document space (the page's corner at `origin`) and the ring of each
/// ring.
fn look_of(job: &Job, origin: Point) -> (ToolpathLook, Vec<Ring>) {
    let mut builder = LookBuilder {
        job,
        origin: Vec2::new(origin.x, origin.y),
        look: ToolpathLook::default(),
        runs: Vec::new(),
        open_arrow: None,
        ringed: vec![false; job.blocks.len()],
        lines: vec![0; job.blocks.len()],
        areas: Vec::new(),
    };
    let mut from = builder.point(job.home);
    for index in 0..job.moves.len() {
        let to = builder.point(job.moves[index].to);
        builder.add(index, from, to);
        from = to;
    }
    (builder.look, builder.runs)
}

impl Session {
    /// Whether laser page `manufacturing`'s Toolpath view is pressed.
    pub(in crate::session) fn toolpath_view_pressed(
        &self,
        manufacturing: curvyo_document_core::NodeId,
    ) -> bool {
        self.output.toolpath.contains(&manufacturing)
    }

    /// Whether the active laser page's Toolpath view is pressed.
    pub(in crate::session) fn toolpath_view_on(&self) -> bool {
        match self.workbench() {
            super::super::Workbench::Manufacturing(id) => self.toolpath_view_pressed(id),
            _ => false,
        }
    }

    /// Presses or releases the Toolpath view toggle in the label of laser page `page`
    /// (criterion 8, UX P2): pressed, the Operations view reads unpressed; released, it reads as
    /// before. Session state, no step. `false` for a page that is not a laser page.
    pub fn toggle_toolpath_view(&mut self, page: &str) -> bool {
        let Some(manufacturing) = self
            .document
            .page_kind(page)
            .and_then(|kind| kind.manufacturing())
            .filter(|&m| self.manufacturing_available(m))
        else {
            return false;
        };
        if !self.output.toolpath.remove(&manufacturing) {
            self.output.toolpath.insert(manufacturing);
            self.output.play = super::play::PlayState::default();
        }
        true
    }

    /// Releases the Toolpath view of `manufacturing`; whether it was pressed (a press on the
    /// Operations view toggle while it is pressed, UX P2).
    pub(in crate::session) fn release_toolpath_view(
        &mut self,
        manufacturing: curvyo_document_core::NodeId,
    ) -> bool {
        self.output.toolpath.remove(&manufacturing)
    }

    /// The look of `job`, from the cache while it is the same job.
    pub(super) fn toolpath_look(&self, job: &PageJob) -> (Arc<ToolpathLook>, Vec<Ring>) {
        let cache = &self.output.draw;
        if let Some((generation, look, runs)) = cache.look.borrow().as_ref()
            && *generation == job.generation
        {
            return (Arc::clone(look), runs.clone());
        }
        #[cfg(test)]
        cache.looks.set(cache.looks.get() + 1);
        let origin = self
            .document
            .manufacturing(job.manufacturing)
            .and_then(|m| m.page)
            .map_or(Point::new(0.0, 0.0), |page| page.origin);
        let (look, runs) = look_of(&job.built.job, origin);
        let look = Arc::new(look);
        *cache.look.borrow_mut() = Some((job.generation, Arc::clone(&look), runs.clone()));
        (look, runs)
    }

    /// The Toolpath view's draw list for this frame, `None` while it is not pressed: the job's
    /// look cut at the play position, rebuilt only when the job, the view or the position
    /// changed (criteria 12, 36 f).
    pub(in crate::session) fn toolpath_overlay(&self) -> Option<DrawList> {
        if !self.toolpath_view_on() {
            return None;
        }
        let job = self.laser_job()?;
        let view = self.view();
        let fraction = self.play_fraction();
        let key = DrawKey {
            generation: job.generation,
            view,
            fraction,
        };
        let cache = &self.output.draw;
        if let Some((held, list)) = cache.list.borrow().as_ref()
            && *held == key
        {
            return Some(list.clone());
        }
        #[cfg(test)]
        cache.lists.set(cache.lists.get() + 1);
        let (look, _) = self.toolpath_look(&job);
        let head = (fraction < 1.0).then(|| {
            let head = job.timeline.head_at(&job.built.job, fraction);
            let origin = look_origin(self, &job);
            ToolpathHead {
                index: head.done,
                at: head.at.translated(origin),
            }
        });
        let list = build_toolpath(&look, head, view);
        *cache.list.borrow_mut() = Some((key, list.clone()));
        Some(list)
    }

    /// The rings' labels in CSS px of the canvas, for the frontend to write in them (UX notes
    /// §4, P26); empty while the Toolpath view is not pressed.
    #[must_use]
    pub fn toolpath_ring_labels(&self) -> Vec<RingLabel> {
        if !self.toolpath_view_on() {
            return Vec::new();
        }
        let Some(job) = self.laser_job() else {
            return Vec::new();
        };
        let (look, runs) = self.toolpath_look(&job);
        let view = self.view();
        let offset = curvyo_render_core::RING_OFFSET_PX;
        look.rings
            .iter()
            .zip(runs)
            .map(|(&at, ring)| {
                let (x, y) = view.document_to_screen(at);
                RingLabel {
                    digit: match ring {
                        Ring::Operation(digit) => Some(digit),
                        Ring::Fixture => None,
                    },
                    x: x - offset,
                    y: y - offset,
                }
            })
            .collect()
    }

    /// The rings' labels as JSON (`[{"digit", "x", "y"}]`, `digit` `null` for the fixture
    /// glyph).
    #[must_use]
    pub fn toolpath_ring_labels_json(&self) -> String {
        serde_json::to_string(&self.toolpath_ring_labels()).unwrap_or_else(|_| "[]".to_owned())
    }
}

/// The page corner of `job`'s laser page as a vector.
fn look_origin(session: &Session, job: &PageJob) -> Vec2 {
    session
        .document
        .manufacturing(job.manufacturing)
        .and_then(|m| m.page)
        .map_or(Vec2::ZERO, |page| Vec2::new(page.origin.x, page.origin.y))
}
