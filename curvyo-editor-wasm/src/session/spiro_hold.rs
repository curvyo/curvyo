//! Keeps a moving Spiro point where its curve has a solution near the one shown (customer
//! 2026-11-11): the Pen's live point and the points of a Node drag stop at their last such place.

use curvyo_document_core::{AnchorKind, NewAnchor, Point};

use super::Session;

/// Halvings of the way back from a refused place toward the last taken one.
const HOLD_STEPS: usize = 5;
/// After its first halving the search stops once the way left to halve is shorter than this
/// (mm).
const HOLD_RESOLUTION_MM: f64 = 0.5;

/// The farthest fraction of the way (`length` mm) from the last taken place (0) to the pointer
/// (1) that `accepts`, trying the whole way first; 0 when no step of the search was taken. Only
/// the try at the pointer itself (`amount == 1.0`) needs a fresh solve; the steps follow the
/// shown shape.
fn farthest(length: f64, mut accepts: impl FnMut(f64) -> bool) -> f64 {
    if accepts(1.0) {
        return 1.0;
    }
    let (mut taken, mut refused) = (0.0, 1.0);
    for step in 0..HOLD_STEPS {
        if step > 0 && (refused - taken) * length < HOLD_RESOLUTION_MM {
            break;
        }
        let middle = f64::midpoint(taken, refused);
        if accepts(middle) {
            taken = middle;
        } else {
            refused = middle;
        }
    }
    taken
}

/// How much farther than the pointer moved (mm) a held point may come toward it per event.
const CATCH_UP_MM: f64 = 1.0;

/// The place a point at `from` heads for when the pointer moved from `last` to `target`:
/// `target`, or, for a point left behind by a hold, the place toward it no farther than the
/// pointer moved plus [`CATCH_UP_MM`].
fn approach(from: Point, target: Point, last: Option<Point>) -> Point {
    let gap = (target.x - from.x).hypot(target.y - from.y);
    let allowed = last.map_or(f64::INFINITY, |last| {
        (target.x - last.x).hypot(target.y - last.y) + CATCH_UP_MM
    });
    if gap <= allowed {
        target
    } else {
        between(from, target, allowed / gap)
    }
}

fn between(from: Point, to: Point, amount: f64) -> Point {
    Point::new(
        from.x + amount * (to.x - from.x),
        from.y + amount * (to.y - from.y),
    )
}

impl Session {
    /// Where the Pen's next Spiro point may go when the pointer puts it at `target`: `target`
    /// itself, or the farthest place toward it from the last one shown at which the curve has a
    /// solution near the shown one; `None` when there is none. The preview follows.
    pub(super) fn spiro_pen_hold(&mut self, target: Point) -> Option<Point> {
        let nodes = self.pen.in_progress_nodes()?.to_vec();
        let last = nodes.last()?.point;
        let id = self.minter.peek();
        let shown = self
            .spiro_cache
            .borrow()
            .preview_controls()
            .and_then(|controls| controls.last().filter(|control| control.id == id).copied())
            .map(|control| control.point);
        let from = shown.unwrap_or(last);
        let pointer = target;
        let target = approach(from, target, self.spiro_hold_pointer.replace(pointer));
        let with = |point: Point| {
            let mut controls = nodes.clone();
            let mut next = NewAnchor::corner(id, point);
            next.kind = AnchorKind::Symmetric;
            controls.push(next);
            controls
        };
        let cache = &self.spiro_cache;
        let amount = farthest(from.vector_to(target).length(), |amount| {
            let point = between(from, target, amount);
            point != last
                && cache
                    .borrow_mut()
                    .try_preview(&with(point), false, amount >= 1.0)
        });
        if amount < 1.0 || target != pointer {
            // The frame draws the pointer's controls: they keep the place found here.
            cache.borrow_mut().refuse_preview(&with(pointer), false);
        }
        if amount > 0.0 {
            Some(between(from, target, amount))
        } else {
            shown
        }
    }

    /// Where a Pen click at `target` places the next Spiro point: at `target`, with the curve
    /// near the one shown or else solved from nothing, since every click places a control (lead
    /// decision 2026-11-11, Proposal); at the held place only when `target` has no curve at all.
    pub(super) fn spiro_pen_place(&mut self, target: Point) -> Point {
        let Some(mut controls) = self.pen.in_progress_nodes().map(<[_]>::to_vec) else {
            return target;
        };
        if controls.last().is_some_and(|last| last.point == target) {
            return target;
        }
        let mut next = NewAnchor::corner(self.minter.peek(), target);
        next.kind = AnchorKind::Symmetric;
        controls.push(next);
        if self
            .spiro_cache
            .borrow_mut()
            .place_preview(&controls, false)
        {
            target
        } else {
            self.spiro_pen_hold(target).unwrap_or(target)
        }
    }

    /// Whether the Pen's Spiro in progress may close: the closed curve has a solution, near the
    /// open one shown or else from nothing (a click always takes effect).
    pub(super) fn spiro_pen_may_close(&mut self) -> bool {
        let Some(nodes) = self.pen.in_progress_nodes().map(<[_]>::to_vec) else {
            return false;
        };
        self.spiro_cache.borrow_mut().place_preview(&nodes, true)
    }

    /// Whether every Spiro of the Node drag in flight has, at the current pointer, a solution
    /// near its last shown shape; without `fresh` following that shape only.
    fn node_spiro_accepts(&mut self, fresh: bool) -> bool {
        let mut live = self.node_spiro_sources.clone();
        self.apply_live_node_drag(&mut live);
        let mut cache = self.spiro_cache.borrow_mut();
        live.iter()
            .all(|path| cache.try_node(&self.document, path, fresh))
    }

    /// Moves the Node drag in flight to `point`, or as far toward it as its Spiros keep a
    /// solution near their shown shapes, and remembers the place.
    pub(super) fn node_spiro_hold(
        &mut self,
        paths: &[curvyo_document_core::PathSnapshot],
        point: Point,
        shift: bool,
        threshold: curvyo_document_core::Tolerance,
    ) {
        if self.node_spiro_sources.is_empty() || !self.node.drag_in_flight() {
            return;
        }
        let from = self.node_spiro_pointer.unwrap_or(point);
        let point = approach(from, point, self.spiro_hold_pointer.replace(point));
        let move_to = |this: &mut Self, at: Point| {
            this.pointer_position = Some(at);
            this.node.pointer_moved(paths, at, shift, threshold);
            this.resolve_node_snapping(at);
        };
        let amount = farthest(from.vector_to(point).length(), |amount| {
            move_to(self, between(from, point, amount));
            self.node_spiro_accepts(amount >= 1.0)
        });
        let held = between(from, point, amount);
        if amount < 1.0 {
            move_to(self, held);
        }
        self.node_spiro_pointer = Some(held);
    }
}
