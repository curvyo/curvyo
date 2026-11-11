//! Previews one drag of a width point on the centre line until it is committed: along the line
//! it moves the point, across it sets the width (`specs/0110-pen-modes-variable-stroke`
//! criterion 4).

use curvyo_document_core::{
    AnchorId, Document, Length, NodeId, Point, WidthPoint, WidthPointError,
};
use curvyo_geometry_core::{PathMeasure, WidthPoint as GeometryWidthPoint, WidthProfile};

use crate::{width_handle, width_position_at};

/// What a drag of a width point changes, chosen by its first movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidthGesturePart {
    /// Move the point along the centre line.
    Position,
    /// Change its width across the centre line.
    Width,
}

/// The width point whose diamond on the centre line lies nearest `pointer`, within `tolerance`;
/// ties go to the smaller id.
#[must_use]
pub fn width_hit(
    path: &PathMeasure,
    points: &[WidthPoint],
    pointer: Point,
    tolerance: Length,
) -> Option<AnchorId> {
    let tolerance = tolerance.as_mm();
    if !tolerance.is_finite() || tolerance < 0.0 || !pointer.x.is_finite() || !pointer.y.is_finite()
    {
        return None;
    }
    points
        .iter()
        .filter_map(|point| {
            let at = width_handle(path, point.position, Length::from_mm(0.0), 0.0)?;
            let distance = at.vector_to(pointer).length();
            (distance <= tolerance).then_some((point.id, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.as_u128().cmp(&b.0.as_u128())))
        .map(|(id, _)| id)
}

/// One provisional width point; the document changes only on commit.
#[derive(Debug, Clone, Copy)]
pub struct WidthGesture {
    point: WidthPoint,
    press: Point,
    axis: Option<WidthGesturePart>,
    /// The width multiplier at the press: a sideways drag changes it relatively.
    start_multiplier: f64,
    /// 1 when the first sideways movement went along the centre line's left normal, else -1.
    outward: f64,
}

impl WidthGesture {
    /// Starts a new point at the centre-line position nearest `pointer`, with the width the
    /// profile already has there.
    #[must_use]
    pub fn add(
        path: &PathMeasure,
        points: &[WidthPoint],
        id: AnchorId,
        pointer: Point,
    ) -> Option<Self> {
        let position = width_position_at(path, pointer)?;
        let controls = points
            .iter()
            .map(|p| {
                GeometryWidthPoint::new(p.id.as_u128(), p.position, p.multiplier, path.is_closed())
            })
            .collect::<Option<Vec<_>>>()?;
        let multiplier = WidthProfile::new(controls, path.is_closed())?.sample(position)?;
        Some(Self {
            point: WidthPoint {
                id,
                position,
                multiplier,
            },
            press: pointer,
            axis: None,
            start_multiplier: multiplier,
            outward: 1.0,
        })
    }

    /// Starts a drag of an existing point pressed at `press`.
    #[must_use]
    pub const fn existing(point: WidthPoint, press: Point) -> Self {
        Self {
            point,
            press,
            axis: None,
            start_multiplier: point.multiplier,
            outward: 1.0,
        }
    }

    /// The current provisional point.
    #[must_use]
    pub const fn preview(&self) -> WidthPoint {
        self.point
    }

    /// What the drag changes, once its first movement chose it.
    #[must_use]
    pub const fn axis(&self) -> Option<WidthGesturePart> {
        self.axis
    }

    /// Follows the pointer. Until it leaves `threshold` of the press nothing changes; then the
    /// larger of its movement along and across the centre line decides, for the whole drag,
    /// whether the point moves along the line or its width changes. The width changes relative to
    /// the press (UX review of 2026-11-10): the full width at the press plus twice how far the
    /// pointer moved outward, toward the side of its first sideways movement; moving back
    /// shrinks it, never below zero. Unmeasurable input leaves the preview unchanged.
    pub fn drag(
        &mut self,
        path: &PathMeasure,
        base: Length,
        pointer: Point,
        threshold: Length,
    ) -> Option<()> {
        if self.axis.is_none() {
            let moved = self.press.vector_to(pointer);
            if moved.length() <= threshold.as_mm() {
                return Some(());
            }
            let across = self.normal(path)?;
            let normal = moved.x.mul_add(across.x, moved.y * across.y);
            self.outward = if normal < 0.0 { -1.0 } else { 1.0 };
            let sideways = normal.abs();
            let along = moved.x.mul_add(across.y, -(moved.y * across.x)).abs();
            self.axis = Some(if sideways > along {
                WidthGesturePart::Width
            } else {
                WidthGesturePart::Position
            });
        }
        match self.axis? {
            WidthGesturePart::Position => self.point.position = width_position_at(path, pointer)?,
            WidthGesturePart::Width => {
                self.point.multiplier = self.dragged_width(path, base, pointer)?;
            }
        }
        Some(())
    }

    /// The unit normal of the centre line at the point's position.
    fn normal(&self, path: &PathMeasure) -> Option<curvyo_document_core::Vec2> {
        let on_line = width_handle(path, self.point.position, Length::from_mm(0.0), 0.0)?;
        let off = width_handle(path, self.point.position, Length::from_mm(2.0), 1.0)?;
        Some(on_line.vector_to(off))
    }

    /// The width multiplier with the pointer at `pointer`, relative to the press.
    fn dragged_width(&self, path: &PathMeasure, base: Length, pointer: Point) -> Option<f64> {
        let base = base.as_mm();
        if !base.is_finite() || base <= 0.0 {
            return None;
        }
        let normal = self.normal(path)?;
        let offset = |at: Point| {
            let from = self.press.vector_to(at);
            from.x.mul_add(normal.x, from.y * normal.y)
        };
        // Outward is the side of the drag's first sideways movement.
        let width = 2.0f64
            .mul_add(self.outward * offset(pointer), base * self.start_multiplier)
            .max(0.0);
        let multiplier = width / base;
        multiplier.is_finite().then_some(multiplier)
    }
    /// Sets the width to `multiplier` (the numeric field); a negative or nonfinite value is
    /// refused.
    pub fn set_multiplier(&mut self, multiplier: f64) -> Option<()> {
        (multiplier.is_finite() && multiplier >= 0.0).then(|| self.point.multiplier = multiplier)
    }

    /// Commits the preview as one undoable document change.
    ///
    /// # Errors
    /// Returns the document's error if the target or point is invalid or shared.
    pub fn commit(self, document: &Document, path: NodeId) -> Result<(), WidthPointError> {
        document.set_width_point(
            path,
            self.point.id,
            self.point.position,
            self.point.multiplier,
        )
    }
}
