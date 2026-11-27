//! What the cells of a test grid are drawn with (`specs/0123` criteria 44, 45; ADR 0069 §8,
//! §13; `specs/0146` criterion 18, ADR 0093 decision 10): an outline for Cut and Score, a filled
//! circle for Engrave, a black-to-white Gradient rectangle for Image.

use curvyo_document_core::{
    Color, CornerRadii, Curve, EllipseFrame, FillPaint, GradientFill, Length, Point, RectBounds,
    Rounded, Shape, Style,
};

use crate::material::SetKind;
use crate::test_recipe::TestRecipe;

/// The gradient of an Image cell (ADR 0066 default linear, `#000000FF` to `#FFFFFFFF`, so the
/// burn ramps from full at the left to none at the right).
#[must_use]
pub fn cell_gradient() -> GradientFill {
    GradientFill::first_for(Color::BLACK, 255)
}

/// The shape of the cell of `test` whose square has its top-left corner at `min` and the side
/// `side`: a circle of that diameter (the ellipse primitive with equal radii, centred on the
/// square) for Engrave, the square for every other kind.
#[must_use]
pub fn cell_shape(test: &TestRecipe, min: Point, side: Length) -> Shape {
    if test.kind() == SetKind::Engrave {
        let radius = Length::from_mm(side.as_mm() / 2.0);
        return Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(min.x + radius.as_mm(), min.y + radius.as_mm()),
                rx: radius,
                ry: radius,
            },
            arc: None,
            curve: Curve::ELLIPSE_DEFAULT,
            rounded: Rounded::ZERO,
        };
    }
    Shape::Rect {
        bounds: RectBounds {
            origin: min,
            width: side,
            height: side,
        },
        corner_radii: CornerRadii::uniform(Length::from_mm(0.0)),
    }
}

/// The style of a cell of `test`: an outline for Cut and Score, a filled circle for Engrave, a
/// square painted with [`cell_gradient`] for Image.
#[must_use]
pub fn cell_style(test: &TestRecipe) -> Style {
    let kind = test.kind();
    let filled = matches!(kind, SetKind::Engrave | SetKind::Image);
    let mut style = Style::default();
    style.fill.enabled = filled;
    style.stroke.enabled = !filled;
    if kind == SetKind::Image {
        style.fill.paint = FillPaint::Gradient;
        style.fill.gradient = Some(cell_gradient());
    }
    style
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine_profile::FeedRate;
    use crate::test_recipe::RecipeDefaults;

    fn defaults() -> RecipeDefaults {
        RecipeDefaults {
            max_feed: FeedRate::new(6000).unwrap(),
            set: None,
        }
    }

    fn circle(side_mm: f64) -> (Shape, Point) {
        let recipe = TestRecipe::new(&defaults()).with_kind(SetKind::Engrave, &defaults());
        let min = Point::new(5.0, 7.0);
        (cell_shape(&recipe, min, Length::from_mm(side_mm)), min)
    }

    /// Criterion 20: the circle is the ellipse primitive with equal radii; every point of its
    /// outline is within 0.03 % of the radius, the area within 0.1 %, and the bounding box is the
    /// square, at Cell sizes 3, 10 and 50 mm.
    #[test]
    #[allow(clippy::many_single_char_names)]
    fn the_engrave_circle_is_the_ellipse_primitive_within_tolerance() {
        for side in [3.0_f64, 10.0, 50.0] {
            let (shape, min) = circle(side);
            let Shape::Ellipse {
                frame, arc: None, ..
            } = shape
            else {
                panic!("an ellipse at {side}");
            };
            let r = side / 2.0;
            assert!((frame.rx.as_mm() - r).abs() < 1e-9 && (frame.ry.as_mm() - r).abs() < 1e-9);
            let (cx, cy) = (min.x + r, min.y + r);
            assert!((frame.center.x - cx).abs() < 1e-9 && (frame.center.y - cy).abs() < 1e-9);
            let anchors = curvyo_document_core::ellipse_outline(frame);
            let (mut worst, mut area) = (0.0_f64, 0.0_f64);
            let (mut lo, mut hi) = ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN));
            let steps = 400;
            let mut previous: Option<(f64, f64)> = None;
            let first = anchors[0].point;
            let mut points = Vec::new();
            for i in 0..anchors.len() {
                let (a, b) = (anchors[i], anchors[(i + 1) % anchors.len()]);
                let (p0, p1) = (a.point, a.point.translated(a.handle_out));
                let (p2, p3) = (b.point.translated(b.handle_in), b.point);
                for k in 0..steps {
                    let t = f64::from(k) / f64::from(steps);
                    let m = 1.0 - t;
                    let x = m.powi(3) * p0.x
                        + 3.0 * m * m * t * p1.x
                        + 3.0 * m * t * t * p2.x
                        + t.powi(3) * p3.x;
                    let y = m.powi(3) * p0.y
                        + 3.0 * m * m * t * p1.y
                        + 3.0 * m * t * t * p2.y
                        + t.powi(3) * p3.y;
                    points.push((x, y));
                }
            }
            points.push((first.x, first.y));
            for &(x, y) in &points {
                worst = worst.max(((x - cx).hypot(y - cy) - r).abs());
                lo = (lo.0.min(x), lo.1.min(y));
                hi = (hi.0.max(x), hi.1.max(y));
                if let Some((px, py)) = previous {
                    area += px * y - x * py;
                }
                previous = Some((x, y));
            }
            let exact = std::f64::consts::PI * r * r;
            assert!(worst <= 0.0003 * r, "side {side}: {worst} off the circle");
            assert!(
                ((area.abs() / 2.0) - exact).abs() <= 0.001 * exact,
                "side {side}: area"
            );
            for (got, want) in [
                (lo.0, min.x),
                (lo.1, min.y),
                (hi.0, min.x + side),
                (hi.1, min.y + side),
            ] {
                assert!((got - want).abs() < 1e-9, "side {side}: bounding box");
            }
        }
    }

    /// Criterion 18: Cut, Score and Image cells stay squares.
    #[test]
    fn cut_score_and_image_cells_stay_squares() {
        for kind in [SetKind::Cut, SetKind::Score, SetKind::Image] {
            let recipe = TestRecipe::new(&defaults()).with_kind(kind, &defaults());
            let shape = cell_shape(&recipe, Point::new(0.0, 0.0), Length::from_mm(10.0));
            assert!(matches!(shape, Shape::Rect { .. }), "{kind:?}");
        }
    }

    #[test]
    fn an_image_cell_paints_black_to_white_and_an_engrave_cell_its_colour() {
        let recipe = TestRecipe::new(&defaults());
        let solid = cell_style(&recipe.with_kind(SetKind::Engrave, &defaults()));
        assert!(solid.fill.enabled && !solid.stroke.enabled);
        assert_eq!(solid.fill.painted_gradient(), None);
        let image = cell_style(&recipe.with_kind(SetKind::Image, &defaults()));
        assert!(image.fill.enabled && !image.stroke.enabled);
        let painted = image.fill.painted_gradient().unwrap();
        assert_eq!(painted, &cell_gradient());
        let stops = painted.stops.as_slice();
        assert_eq!((stops[0].colour, stops[0].alpha), (Color::BLACK, 255));
        assert_eq!((stops[1].colour, stops[1].alpha), (Color::WHITE, 255));
        let cut = cell_style(&recipe);
        assert!(!cut.fill.enabled && cut.stroke.enabled);
    }
}
