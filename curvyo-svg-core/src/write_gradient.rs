//! One gradient definition in `<defs>` (`specs/0124-gradient-fill` criterion 13, ADR 0067 §6):
//! `linearGradient` or `radialGradient` in the element's user space with its stops directly
//! inside, as Inkscape writes and opens it; `gradientTransform` only for an elliptical radial.

use kurbo::{Affine, Point, Vec2};

use crate::gradient_paint::{SvgGradient, SvgGradientShape, SvgStop};
use crate::write::{Writer, hex};

/// A stop opacity: three decimals (enough to recover the byte), trailing zeros trimmed.
fn opacity_text(alpha: f64) -> String {
    let mut text = format!("{:.3}", alpha.clamp(0.0, 1.0));
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

/// Two radial axes whose lengths and right angle agree within this (relative) are a circle.
const CIRCLE_TOLERANCE: f64 = 1e-9;

/// The map from the unit gradient (linear: `t = u`; radial: `t = |(u, v)|`) into gradient space.
fn unit_map(shape: SvgGradientShape) -> Affine {
    match shape {
        SvgGradientShape::Linear { start, end } => {
            let d = end - start;
            Affine::new([d.x, d.y, -d.y, d.x, start.x, start.y])
        }
        SvgGradientShape::Radial { centre, radius } => {
            Affine::translate(centre.to_vec2()) * Affine::scale(radius)
        }
    }
}

/// `gradient` in the user space of an element whose own transform is `element` (from its user
/// space into the exported coordinates): a linear gradient as plain numbers under any map, a
/// radial as a circle under a similarity, otherwise as the circle of its first axis about its
/// centre and the matrix that makes the ellipse (it fixes the centre). `None` for a gradient of
/// no area.
#[allow(clippy::many_single_char_names)] // the matrix entries
pub(crate) fn in_element_space(
    gradient: &SvgGradient,
    element: Affine,
) -> Option<(SvgGradientShape, Affine)> {
    let full = element.inverse() * gradient.transform * unit_map(gradient.shape);
    let det = full.determinant();
    if !det.is_finite() || det.abs() < 1e-18 {
        return None;
    }
    let [a, b, c, d, e, f] = full.as_coeffs();
    let centre = Point::new(e, f);
    Some(match gradient.shape {
        SvgGradientShape::Linear { .. } => {
            // `t = u`: the line runs along the gradient of `u` in the element's space.
            let [ga, _, gc, _, _, _] = full.inverse().as_coeffs();
            let g2 = ga * ga + gc * gc;
            let end = Point::new(e + ga / g2, f + gc / g2);
            (
                SvgGradientShape::Linear { start: centre, end },
                Affine::IDENTITY,
            )
        }
        SvgGradientShape::Radial { .. } => {
            let (u, v) = (Vec2::new(a, b), Vec2::new(c, d));
            let radius = u.hypot();
            let circle = (u.hypot() - v.hypot()).abs() <= CIRCLE_TOLERANCE * radius
                && u.dot(v).abs() <= CIRCLE_TOLERANCE * radius * radius;
            let transform = if circle {
                Affine::IDENTITY
            } else {
                full * Affine::scale(1.0 / radius) * Affine::translate(-centre.to_vec2())
            };
            (SvgGradientShape::Radial { centre, radius }, transform)
        }
    })
}

impl Writer<'_> {
    /// Writes `gradient` as the definition `id`, in the user space of an element with the
    /// transform `element`.
    pub(crate) fn gradient(&mut self, id: &str, gradient: &SvgGradient, element: Affine) {
        let Some((shape, transform)) = in_element_space(gradient, element) else {
            return;
        };
        self.indent(2);
        let tag = match shape {
            SvgGradientShape::Linear { start, end } => {
                self.out.push_str("<linearGradient");
                self.attr("id", id);
                self.attr("gradientUnits", "userSpaceOnUse");
                self.attr_coord("x1", start.x);
                self.attr_coord("y1", start.y);
                self.attr_coord("x2", end.x);
                self.attr_coord("y2", end.y);
                "linearGradient"
            }
            SvgGradientShape::Radial { centre, radius } => {
                self.out.push_str("<radialGradient");
                self.attr("id", id);
                self.attr("gradientUnits", "userSpaceOnUse");
                self.attr_coord("cx", centre.x);
                self.attr_coord("cy", centre.y);
                self.attr_coord("r", radius);
                self.attr_coord("fx", centre.x);
                self.attr_coord("fy", centre.y);
                "radialGradient"
            }
        };
        if transform != Affine::IDENTITY {
            self.matrix_attr("gradientTransform", transform);
        }
        self.out.push_str(">\n");
        for stop in &gradient.stops {
            self.stop(stop);
        }
        self.indent(2);
        self.out.push_str("</");
        self.out.push_str(tag);
        self.out.push_str(">\n");
    }

    fn stop(&mut self, stop: &SvgStop) {
        self.indent(3);
        self.out.push_str("<stop");
        self.attr_exact("offset", stop.offset);
        self.attr("stop-color", &hex(stop.rgb));
        self.attr("stop-opacity", &opacity_text(stop.alpha));
        self.out.push_str("/>\n");
    }
}

#[cfg(test)]
mod tests {
    use super::opacity_text;

    #[test]
    fn opacities_have_at_most_three_decimals() {
        assert_eq!(opacity_text(1.0), "1");
        assert_eq!(opacity_text(0.0), "0");
        assert_eq!(opacity_text(128.0 / 255.0), "0.502");
        assert_eq!(opacity_text(0.5), "0.5");
    }
}
