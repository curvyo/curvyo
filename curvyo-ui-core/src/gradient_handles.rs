//! Where a gradient's handles sit on the canvas and which one a pointer hits
//! (`specs/0124-gradient-fill` criteria 6 and 7, UX notes §6): the end or centre and radius
//! handles, a dot per inner stop, the line between them, the screen-size rule that hides them,
//! and the shape handles they cover.

use curvyo_document_core::{BoxPoint, GradientBox, GradientFill, GradientGeometry, Point, Vec2};

use crate::transform_handle_layout::EditHandle;
use crate::yielded_handles::YieldedHandles;

/// The hit radius of an end, Start, centre or radius handle, screen pixels (24 px target).
pub const GRADIENT_END_HIT_PX: f64 = 12.0;
/// The hit radius of an inner stop dot, screen pixels (20 px target).
pub const GRADIENT_STOP_HIT_PX: f64 = 10.0;
/// How far from the gradient line a double-click still adds a stop, screen pixels.
pub const GRADIENT_LINE_HIT_PX: f64 = 4.0;
/// A shape handle this close to a drawn gradient handle is neither drawn nor hit, screen pixels.
pub(crate) const GRADIENT_YIELD_PX: f64 = 8.0;
/// The shortest Linear line on screen that shows handles, screen pixels.
pub const GRADIENT_MIN_LINE_PX: f64 = 24.0;
/// The smallest Radial radius on screen that shows handles, screen pixels.
pub const GRADIENT_MIN_RADIUS_PX: f64 = 12.0;

/// One handle of a gradient on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientHandle {
    /// The Linear Start (offset 0), drawn as a square.
    Start,
    /// The Linear End (offset 1).
    End,
    /// The Radial centre (offset 0), drawn as a square; a drag moves the whole circle.
    Centre,
    /// The Radial radius handle (offset 1).
    Radius,
    /// The inner stop at this index of the stop list.
    Stop(usize),
}

impl GradientHandle {
    /// Whether it is one of the two ends (not an inner stop).
    #[must_use]
    pub const fn is_end(self) -> bool {
        !matches!(self, Self::Stop(_))
    }

    /// The index of the stop it stands for.
    #[must_use]
    pub const fn stop(self, stop_count: usize) -> usize {
        match self {
            Self::Start | Self::Centre => 0,
            Self::End | Self::Radius => stop_count.saturating_sub(1),
            Self::Stop(index) => index,
        }
    }
}

/// A gradient's handles placed on the document: the line from the first to the last handle and
/// every handle's point, ends first.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientLayout {
    /// Start (Linear) or centre (Radial), document millimetres.
    pub from: Point,
    /// End (Linear) or radius handle (Radial), document millimetres.
    pub to: Point,
    /// The second axis of a Radial ellipse (the first is `to - from`); `None` for a Linear.
    pub second_axis: Option<Vec2>,
    /// Every handle and its point: the two ends, then the inner stops in order.
    pub handles: Vec<(GradientHandle, Point)>,
}

/// The document point at box fractions `p` of `frame`.
#[must_use]
pub(crate) fn document_point(frame: &GradientBox, p: BoxPoint) -> Point {
    frame
        .to_document
        .apply_point(frame.frame_point(p.x.get(), p.y.get()))
}

/// The box fractions of document point `p` in `frame`; `None` for a box of no width or height or
/// a frame that cannot be inverted.
#[must_use]
pub(crate) fn box_fractions(frame: &GradientBox, p: Point) -> Option<BoxPoint> {
    let q = frame.to_document.inverse()?.apply_point(p);
    let (w, h) = frame.size();
    if w.is_nan() || h.is_nan() || w <= 0.0 || h <= 0.0 {
        return None;
    }
    BoxPoint::new((q.x - frame.min.x) / w, (q.y - frame.min.y) / h).ok()
}

fn lerp(a: Point, b: Point, t: f64) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

/// The handles of `fill` placed in `frame`.
#[must_use]
pub fn gradient_layout(fill: &GradientFill, frame: &GradientBox) -> GradientLayout {
    let (from, to, ends, second_axis) = match fill.geometry {
        GradientGeometry::Linear { start, end } => (
            document_point(frame, start),
            document_point(frame, end),
            (GradientHandle::Start, GradientHandle::End),
            None,
        ),
        GradientGeometry::Radial {
            centre,
            radius,
            ratio,
        } => {
            let (c, r) = (document_point(frame, centre), document_point(frame, radius));
            // The second axis in the frame, a quarter turn of the first scaled by the ratio,
            // carried into the document.
            let (fc, fr) = (
                frame.frame_point(centre.x.get(), centre.y.get()),
                frame.frame_point(radius.x.get(), radius.y.get()),
            );
            let (ax, ay) = (fr.x - fc.x, fr.y - fc.y);
            let second = frame
                .to_document
                .apply_vector(Vec2::new(-ay * ratio.get(), ax * ratio.get()));
            (
                c,
                r,
                (GradientHandle::Centre, GradientHandle::Radius),
                Some(second),
            )
        }
    };
    let stops = fill.stops.as_slice();
    let mut handles = vec![(ends.0, from), (ends.1, to)];
    handles.extend(
        stops
            .iter()
            .enumerate()
            .take(stops.len().saturating_sub(1))
            .skip(1)
            .map(|(index, stop)| {
                (
                    GradientHandle::Stop(index),
                    lerp(from, to, stop.offset.get()),
                )
            }),
    );
    GradientLayout {
        from,
        to,
        second_axis,
        handles,
    }
}

impl GradientLayout {
    /// Whether the handles are drawn at `px_per_mm`: the line at least 24 px long (Linear) or the
    /// radius at least 12 px (Radial) on screen (UX notes §6.1, P12).
    #[must_use]
    pub fn shown_at(&self, px_per_mm: f64) -> bool {
        let length = (self.to.x - self.from.x).hypot(self.to.y - self.from.y) * px_per_mm;
        let least = if self.second_axis.is_some() {
            GRADIENT_MIN_RADIUS_PX
        } else {
            GRADIENT_MIN_LINE_PX
        };
        length >= least
    }

    /// The handle under `point` at `px_per_mm`: the nearest end within 12 px, else the nearest
    /// inner stop within 10 px. Ends win over a stop dot that lies on them.
    #[must_use]
    pub fn hit(&self, point: Point, px_per_mm: f64) -> Option<GradientHandle> {
        let distance = |p: Point| (p.x - point.x).hypot(p.y - point.y) * px_per_mm;
        let nearest = |ends: bool, radius: f64| {
            self.handles
                .iter()
                .filter(|(handle, _)| handle.is_end() == ends)
                .map(|(handle, at)| (*handle, distance(*at)))
                .filter(|(_, d)| *d <= radius)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(handle, _)| handle)
        };
        nearest(true, GRADIENT_END_HIT_PX).or_else(|| nearest(false, GRADIENT_STOP_HIT_PX))
    }

    /// Where along the line (`0..=1`) a double-click at `point` adds a stop: within 4 px of the
    /// segment from the first to the last handle, not on a handle; `None` elsewhere.
    #[must_use]
    pub fn line_offset_at(&self, point: Point, px_per_mm: f64) -> Option<f64> {
        if self.hit(point, px_per_mm).is_some() {
            return None;
        }
        let t = self.offset_along(point)?;
        let on = lerp(self.from, self.to, t);
        let gap = (on.x - point.x).hypot(on.y - point.y) * px_per_mm;
        (gap <= GRADIENT_LINE_HIT_PX && t > 0.0 && t < 1.0).then_some(t)
    }

    /// The projection of `point` on the line as an offset, clamped to `0..=1`; `None` for a line
    /// of no length.
    #[must_use]
    pub fn offset_along(&self, point: Point) -> Option<f64> {
        let (dx, dy) = (self.to.x - self.from.x, self.to.y - self.from.y);
        let len2 = dx.mul_add(dx, dy * dy);
        if len2.is_nan() || len2 <= 0.0 {
            return None;
        }
        let t = ((point.x - self.from.x) * dx + (point.y - self.from.y) * dy) / len2;
        Some(t.clamp(0.0, 1.0))
    }

    /// The shape handles of `shape_handles` within 8 px of a gradient handle at `px_per_mm`: they
    /// yield (criterion 6, UX notes §6.3).
    #[must_use]
    pub fn yielded(&self, shape_handles: &[(EditHandle, Point)], px_per_mm: f64) -> YieldedHandles {
        shape_handles
            .iter()
            .filter(|(_, at)| {
                self.handles
                    .iter()
                    .any(|(_, g)| (g.x - at.x).hypot(g.y - at.y) * px_per_mm <= GRADIENT_YIELD_PX)
            })
            .fold(YieldedHandles::NONE, |set, (handle, _)| set.with(*handle))
    }
}
