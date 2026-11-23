//! The artwork objects the canvas can show: an object whose outline box, grown by its stroke,
//! misses the visible rectangle is left out of the frame's tessellation
//! (`specs/0126-performance-pass` phase 4).

use std::borrow::Cow;
use std::collections::HashSet;

use curvyo_document_core::{NodeId, ObjectSnapshot, Point};

use super::Session;

/// How far a stroke reaches past its path, in stroke widths: a miter (limit 2, so 1 width), a
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

/// What the cull reads of one object, whatever the view: nothing (the object is always drawn)
/// or its outline box and the width its stroke reaches by.
#[derive(Debug, Clone, Copy)]
pub(super) enum CullBox {
    /// Never left out: an image, a picture, a pattern or gradient fill, a dimension, a text
    /// not laid out, or bounds that cannot be computed.
    Always,
    /// The outline box and the clamped stroke width, in millimetres.
    Outline {
        low: Point,
        high: Point,
        stroke: f64,
    },
}

/// The cull box of `object`. Only paths, shapes, clones of a path, sketches and laid-out texts
/// without a pattern or gradient fill are ever left out; images keep their layer, pictures and
/// patterns draw pieces of their own, and the frame's gradient spans and stop table carry every
/// shaded leaf (`specs/0124` criterion 10 reads them).
pub(super) fn cull_box(object: &ObjectSnapshot) -> CullBox {
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
            ObjectSnapshot::Image(_)
            | ObjectSnapshot::Dimension(_)
            | ObjectSnapshot::RegistrationMark(_) => false,
        };
    if !cullable {
        return CullBox::Always;
    }
    let width = style.stroke.width.as_mm();
    let stroke = if style.stroke.enabled && width.is_finite() {
        width.clamp(0.0, MAX_REACH_MM)
    } else {
        0.0
    };
    // Bounds that cannot be computed (a compound Spiro path) never cull: the object stays.
    match curvyo_ui_core::try_object_outline_bounds(object) {
        Some((low, high)) => CullBox::Outline { low, high, stroke },
        None => CullBox::Always,
    }
}

impl VisibleRect {
    /// Whether an object of cull box `cull` may draw a pixel inside the rectangle.
    fn shows(&self, cull: CullBox) -> bool {
        let CullBox::Outline { low, high, stroke } = cull else {
            return true;
        };
        let margin = STROKE_REACH * stroke + MARGIN_PX / self.scale;
        !(high.x + margin < self.min.x
            || low.x - margin > self.max.x
            || high.y + margin < self.min.y
            || low.y - margin > self.max.y)
    }

    /// Whether `object` may draw a pixel inside the rectangle.
    fn may_show(&self, object: &ObjectSnapshot) -> bool {
        self.shows(cull_box(object))
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

/// The cull boxes of every object of one read, measured once: the cull of a frame that draws the
/// same read again (a pan, a zoom) is then a test of boxes, with the answer [`in_view`] gives.
#[derive(Debug)]
pub(super) struct CullBoxes {
    boxes: Vec<CullBox>,
    /// Whether any object paints a pattern (its pieces depend on more than the read).
    patterned: bool,
}

impl CullBoxes {
    /// The boxes of `objects`, in order.
    pub(super) fn of(objects: &[ObjectSnapshot]) -> Self {
        Self {
            boxes: objects.iter().map(cull_box).collect(),
            patterned: curvyo_ui_core::PatternCache::any_pattern(objects),
        }
    }

    /// Whether any object paints a pattern.
    pub(super) const fn patterned(&self) -> bool {
        self.patterned
    }

    /// The indices of the objects [`in_view`] keeps of the read these boxes were measured
    /// over, ascending; `objects` is that read (for the ids of the guides).
    #[allow(clippy::implicit_hasher)]
    pub(super) fn kept(
        &self,
        objects: &[ObjectSnapshot],
        guides: &HashSet<NodeId>,
        visible: Option<VisibleRect>,
    ) -> Vec<usize> {
        debug_assert_eq!(objects.len(), self.boxes.len(), "boxes of another read");
        let Some(visible) = visible else {
            return (0..objects.len()).collect();
        };
        self.boxes
            .iter()
            .zip(objects)
            .enumerate()
            .filter(|(_, (cull, object))| {
                visible.shows(**cull) || (!guides.is_empty() && guides.contains(&object.id()))
            })
            .map(|(index, _)| index)
            .collect()
    }
}

#[cfg(test)]
#[path = "view_cull_tests.rs"]
mod tests;
