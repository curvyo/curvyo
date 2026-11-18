//! The Angle and Radius of a gradient, measured in millimetres in the shape's box frame
//! (`specs/0124-gradient-fill` criteria 3 and 4).
//!
//! Angle 0 points right and grows counter-clockwise on the page (Y grows downward), as the Scan
//! angle of `0111`. These are panel values: a typed Angle or Radius becomes new box fractions for
//! each shape's own box, written by `Document::set_gradient_geometries`.

use crate::gradient_eval::GradientBox;
use crate::gradient_model::{BoxPoint, GradientGeometry};
use crate::units::{Angle, Length, Point};

/// The box fractions of frame point `p`; `None` when the box has no width or no height.
fn fractions(frame: &GradientBox, p: Point) -> Option<BoxPoint> {
    let (w, h) = frame.size();
    if w.is_nan() || h.is_nan() || w <= 0.0 || h <= 0.0 {
        return None;
    }
    BoxPoint::new((p.x - frame.min.x) / w, (p.y - frame.min.y) / h).ok()
}

fn frame_point(frame: &GradientBox, p: BoxPoint) -> Point {
    frame.frame_point(p.x.get(), p.y.get())
}

impl GradientGeometry {
    /// The direction of a Linear gradient line in `frame`, wrapped to `(-180°, 180°]`; `None`
    /// for a Radial or a line of no length.
    #[must_use]
    pub fn angle_in(&self, frame: &GradientBox) -> Option<Angle> {
        let Self::Linear { start, end } = *self else {
            return None;
        };
        let (s, e) = (frame_point(frame, start), frame_point(frame, end));
        let (dx, dy) = (e.x - s.x, e.y - s.y);
        (dx != 0.0 || dy != 0.0).then(|| Angle::from_radians((-dy).atan2(dx)).normalized())
    }

    /// The Linear line turned to `angle` about its midpoint, its length kept (criterion 4);
    /// unchanged for a Radial, a line of no length or a box of no width or height.
    #[must_use]
    pub fn with_angle(self, frame: &GradientBox, angle: Angle) -> Self {
        let Self::Linear { start, end } = self else {
            return self;
        };
        let (s, e) = (frame_point(frame, start), frame_point(frame, end));
        let half = ((e.x - s.x).hypot(e.y - s.y)) / 2.0;
        if half.is_nan() || half <= 0.0 || !angle.as_radians().is_finite() {
            return self;
        }
        let (sin, cos) = angle.as_radians().sin_cos();
        let (mx, my) = (f64::midpoint(s.x, e.x), f64::midpoint(s.y, e.y));
        let (ux, uy) = (cos * half, -sin * half);
        match (
            fractions(frame, Point::new(mx - ux, my - uy)),
            fractions(frame, Point::new(mx + ux, my + uy)),
        ) {
            (Some(start), Some(end)) => Self::Linear { start, end },
            _ => self,
        }
    }

    /// The distance from the Radial centre to its radius handle in `frame`; `None` for a Linear.
    #[must_use]
    pub fn radius_in(&self, frame: &GradientBox) -> Option<Length> {
        let Self::Radial { centre, radius, .. } = *self else {
            return None;
        };
        let (c, r) = (frame_point(frame, centre), frame_point(frame, radius));
        Some(Length::from_mm((r.x - c.x).hypot(r.y - c.y)))
    }

    /// The Radial radius handle moved along its direction to `radius` from the centre (to the
    /// right when it sits on the centre); unchanged for a Linear, a radius that is not above 0 or
    /// a box of no width or height.
    #[must_use]
    pub fn with_radius(self, frame: &GradientBox, radius: Length) -> Self {
        let Self::Radial {
            centre,
            radius: handle,
            ratio,
        } = self
        else {
            return self;
        };
        let r = radius.as_mm();
        if !r.is_finite() || r <= 0.0 {
            return self;
        }
        let (c, h) = (frame_point(frame, centre), frame_point(frame, handle));
        let (dx, dy) = (h.x - c.x, h.y - c.y);
        let length = dx.hypot(dy);
        let (ux, uy) = if length > 0.0 {
            (dx / length, dy / length)
        } else {
            (1.0, 0.0)
        };
        fractions(frame, Point::new(c.x + ux * r, c.y + uy * r)).map_or(self, |radius| {
            Self::Radial {
                centre,
                radius,
                ratio,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> GradientBox {
        GradientBox::axis_aligned(Point::new(10.0, 10.0), Point::new(110.0, 50.0))
    }

    fn degrees(angle: Angle) -> f64 {
        angle.as_radians().to_degrees()
    }

    #[test]
    fn the_default_line_points_right() {
        let angle = GradientGeometry::DEFAULT_LINEAR.angle_in(&frame()).unwrap();
        assert!(degrees(angle).abs() < 1e-12);
        assert_eq!(GradientGeometry::DEFAULT_RADIAL.angle_in(&frame()), None);
    }

    #[test]
    fn an_angle_turns_the_line_about_its_midpoint_counter_clockwise() {
        let turned = GradientGeometry::DEFAULT_LINEAR
            .with_angle(&frame(), Angle::from_radians(std::f64::consts::FRAC_PI_2));
        let GradientGeometry::Linear { start, end } = turned else {
            panic!("linear")
        };
        // Length 100 mm about the centre (60, 30): from (60, 80) up to (60, -20).
        let (s, e) = (frame_point(&frame(), start), frame_point(&frame(), end));
        assert!(
            (s.x - 60.0).abs() < 1e-9 && (s.y - 80.0).abs() < 1e-9,
            "{s:?}"
        );
        assert!(
            (e.x - 60.0).abs() < 1e-9 && (e.y + 20.0).abs() < 1e-9,
            "{e:?}"
        );
        assert!((degrees(turned.angle_in(&frame()).unwrap()) - 90.0).abs() < 1e-9);
    }

    #[test]
    fn a_radius_moves_the_handle_along_its_direction() {
        let geometry = GradientGeometry::DEFAULT_RADIAL;
        assert!((geometry.radius_in(&frame()).unwrap().as_mm() - 50.0).abs() < 1e-12);
        let wider = geometry.with_radius(&frame(), Length::from_mm(20.0));
        assert!((wider.radius_in(&frame()).unwrap().as_mm() - 20.0).abs() < 1e-12);
        assert_eq!(
            geometry.with_radius(&frame(), Length::from_mm(0.0)),
            geometry
        );
        assert_eq!(GradientGeometry::DEFAULT_LINEAR.radius_in(&frame()), None);
    }

    #[test]
    fn a_flat_box_leaves_the_geometry_alone() {
        let flat = GradientBox::axis_aligned(Point::new(0.0, 5.0), Point::new(40.0, 5.0));
        let geometry = GradientGeometry::DEFAULT_LINEAR;
        assert_eq!(
            geometry.with_angle(&flat, Angle::from_radians(1.0)),
            geometry
        );
    }
}
