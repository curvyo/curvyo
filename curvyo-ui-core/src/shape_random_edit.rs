//! The edit of a polygon's or star's Randomized value from the bar
//! (`specs/0145-shape-spiro-handles/` criteria 21 and 22): the shape a typed or stepped value
//! gives, with the seed it is drawn with. There is no canvas handle.

use curvyo_document_core::{NodeId, Randomized, Seed, Shape};

/// Two values closer than this are the same (nothing to write).
pub const RANDOMIZED_EQUAL_EPSILON: f64 = 1e-9;

/// Whether `shape` takes a Randomized value: a polygon or a star.
#[must_use]
pub const fn randomized_capable(shape: &Shape) -> bool {
    matches!(shape, Shape::Polygon { .. } | Shape::Star { .. })
}

/// `shape` with its Randomized value set to `value` limited to -10 to 10, the seed kept; a seed
/// still 0 becomes the fresh seed of `id` when the value leaves 0, as the document command does.
/// `None` when the limited value equals the stored one, when `value` is not finite and for a
/// shape that takes none.
#[must_use]
pub fn with_randomized(shape: &Shape, id: NodeId, value: f64) -> Option<Shape> {
    if !value.is_finite() {
        return None;
    }
    let (Shape::Polygon { randomized, .. } | Shape::Star { randomized, .. }) = *shape else {
        return None;
    };
    let limited = value.clamp(-Randomized::RANGE, Randomized::RANGE);
    if (limited - randomized.effective()).abs() <= RANDOMIZED_EQUAL_EPSILON {
        return None;
    }
    let seed = if limited != 0.0 && randomized.seed().get() == 0 {
        Seed::fresh(id)
    } else {
        randomized.seed()
    };
    let next = randomized.with_clamped_amount(limited).with_seed(seed);
    Some(match *shape {
        Shape::Polygon {
            frame,
            point_count,
            curve,
            rounded,
            ..
        } => Shape::Polygon {
            frame,
            point_count,
            curve,
            rounded,
            randomized: next,
        },
        Shape::Star {
            frame,
            point_count,
            inner_ratio,
            curve,
            rounded,
            ..
        } => Shape::Star {
            frame,
            point_count,
            inner_ratio,
            curve,
            rounded,
            randomized: next,
        },
        Shape::Rect { .. } | Shape::Ellipse { .. } => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{Angle, Length, Point, PointCount, Rounded, StarFrame};

    fn polygon(r: f64) -> Shape {
        Shape::Polygon {
            frame: StarFrame {
                center: Point::new(0.0, 0.0),
                radius: Length::from_mm(10.0),
                angle: Angle::from_radians(0.0),
            },
            point_count: PointCount::new(5).unwrap(),
            curve: curvyo_document_core::Curve::POLYGON_DEFAULT,
            rounded: Rounded::ZERO,
            randomized: Randomized::new(r, Seed::new(0)).unwrap(),
        }
    }

    /// Criterion 21: a value is limited to ±10 and the seed is drawn once.
    #[test]
    fn a_value_is_limited_and_draws_its_seed_once() {
        let document = curvyo_document_core::Document::new(1);
        let id = document.create_polygon(
            StarFrame {
                center: Point::new(0.0, 0.0),
                radius: Length::from_mm(10.0),
                angle: Angle::from_radians(0.0),
            },
            PointCount::new(5).unwrap(),
        );
        let Some(Shape::Polygon { randomized, .. }) = with_randomized(&polygon(0.0), id, 99.0)
        else {
            panic!("a polygon")
        };
        assert_eq!(randomized.amount(), 10.0);
        assert_eq!(randomized.seed(), Seed::fresh(id));
        assert!(with_randomized(&polygon(10.0), id, 99.0).is_none());
        assert!(with_randomized(&polygon(0.0), id, f64::NAN).is_none());
    }
}
