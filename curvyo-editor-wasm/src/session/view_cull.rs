//! The artwork objects the canvas can show: an object whose outline box, grown by its stroke,
//! misses the visible rectangle is left out of the frame's tessellation
//! (`specs/0126-performance-pass` phase 4).

use std::borrow::Cow;
use std::collections::HashSet;

use curvyo_document_core::{NodeId, ObjectSnapshot, Point};

use super::Session;

/// How far a stroke reaches past its path, in stroke widths: a miter (limit 4, so 2 widths), a
/// square cap (0.71) and an arrow marker (2 widths past the end) all stay inside 3.
const STROKE_REACH: f64 = 3.0;

/// Screen pixels added on every side: the 1 px minimum drawn width times the reach, the
/// tessellation tolerance and the anti-aliasing fringe.
const MARGIN_PX: f64 = 4.0;

/// The document rectangle the canvas shows, at the view's scale (pixels per millimetre).
#[derive(Debug, Clone, Copy)]
pub(super) struct VisibleRect {
    min: Point,
    max: Point,
    scale: f64,
}

impl VisibleRect {
    /// Whether `object` may draw a pixel inside the rectangle. Only paths, shapes, clones of
    /// a path, sketches and laid-out texts without a pattern or gradient fill are ever left
    /// out; images keep their layer, pictures and patterns draw pieces of their own, and the
    /// frame's gradient spans and stop table carry every shaded leaf (`specs/0124` criterion
    /// 10 reads them).
    fn may_show(&self, object: &ObjectSnapshot) -> bool {
        let style = object.style();
        let cullable = object.picture().is_none()
            && style.fill.painted_pattern().is_none()
            && style.fill.painted_gradient().is_none()
            && match object {
                ObjectSnapshot::Path(_)
                | ObjectSnapshot::Primitive(_)
                | ObjectSnapshot::Clone(_)
                | ObjectSnapshot::Sketch(_) => true,
                ObjectSnapshot::Text(text) => text.outline.is_some(),
                ObjectSnapshot::Image(_) | ObjectSnapshot::Dimension(_) => false,
            };
        if !cullable {
            return true;
        }
        let width = style.stroke.width.as_mm();
        let stroke = if style.stroke.enabled && width.is_finite() {
            width.clamp(0.0, MAX_REACH_MM)
        } else {
            0.0
        };
        let margin = STROKE_REACH * stroke + MARGIN_PX / self.scale;
        let (low, high) = curvyo_ui_core::object_outline_bounds(object);
        !(high.x + margin < self.min.x
            || low.x - margin > self.max.x
            || high.y + margin < self.min.y
            || low.y - margin > self.max.y)
    }
}

/// The widest stroke reach considered, in millimetres: the widest stroke the artwork draws.
const MAX_REACH_MM: f64 = 100_000.0;

impl Session {
    /// The rectangle the canvas shows, `None` while the canvas has no size or the view no
    /// usable scale (nothing is left out then).
    pub(super) fn visible_rect(&self) -> Option<VisibleRect> {
        // The whole canvas element: a side pane over its right part (`cover_right`) shrinks
        // the viewport, but the GPU still draws under the pane.
        let (_, height) = self.viewport.canvas_size();
        let width = self.canvas_css_width;
        let view = self.view();
        let scale = view.scale();
        if !(width > 0.0 && height > 0.0 && scale.is_finite() && scale > 0.0) {
            return None;
        }
        let a = view.screen_to_document(0.0, 0.0);
        let b = view.screen_to_document(width, height);
        Some(VisibleRect {
            min: Point::new(a.x.min(b.x), a.y.min(b.y)),
            max: Point::new(a.x.max(b.x), a.y.max(b.y)),
            scale,
        })
    }
}

/// `objects` without those `visible` cannot show; guides are always kept (their marking style
/// is drawn in screen pixels). Borrowed when nothing is left out.
// One hasher is used (`CLAUDE.md` §5: no generic parameter for a single type).
#[allow(clippy::implicit_hasher)]
pub(super) fn in_view<'o>(
    objects: &'o [ObjectSnapshot],
    guides: &HashSet<NodeId>,
    visible: Option<VisibleRect>,
) -> Cow<'o, [ObjectSnapshot]> {
    let Some(visible) = visible else {
        return Cow::Borrowed(objects);
    };
    let keep = |object: &ObjectSnapshot| guides.contains(&object.id()) || visible.may_show(object);
    if objects.iter().all(keep) {
        return Cow::Borrowed(objects);
    }
    Cow::Owned(objects.iter().filter(|o| keep(o)).cloned().collect())
}

#[cfg(test)]
#[path = "view_cull_tests.rs"]
mod tests;
