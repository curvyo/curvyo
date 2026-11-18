//! The gradient paint of an element as the reader gives it and the writer takes it
//! (`specs/0124-gradient-fill` criteria 13 and 14; ADR 0067 §6, §7): a linear or radial shape in
//! gradient space, the map from gradient space into the drawing and the stops.

use kurbo::{Affine, Point};

use crate::drawing::Rgb;

/// The shape of a gradient in its own space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SvgGradientShape {
    /// `t` runs from 0 at `start` to 1 at `end`; the lines across it have one colour.
    Linear {
        /// Where `t` is 0.
        start: Point,
        /// Where `t` is 1.
        end: Point,
    },
    /// `t` runs from 0 at `centre` to 1 on the circle of `radius` (an ellipse once
    /// [`SvgGradient::transform`] is not a similarity).
    Radial {
        /// The centre (and the focal point: a focal point elsewhere is read as the centre).
        centre: Point,
        /// The radius, above 0.
        radius: f64,
    },
}

/// One colour stop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvgStop {
    /// Where it sits on `t`, in `[0, 1]`, never below the stop before.
    pub offset: f64,
    /// The colour.
    pub rgb: Rgb,
    /// The opacity, 0 to 1 (`stop-opacity` times the colour's alpha and, on import, the
    /// element's `fill-opacity`).
    pub alpha: f64,
}

/// A gradient paint: a shape, a map and two or more stops. Pad is the only spread; the first
/// stop is at 0 and the last at 1.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgGradient {
    /// The shape in gradient space.
    pub shape: SvgGradientShape,
    /// From gradient space into the drawing. On import: into millimetres (the element's map,
    /// its box for `objectBoundingBox` and `gradientTransform` composed). On export: into the
    /// exported coordinates; the writer re-expresses the gradient in each element's own user
    /// space and writes a `gradientTransform` only where that is not a similarity (an elliptical
    /// radial).
    pub transform: Affine,
    /// The stops, at least two.
    pub stops: Vec<SvgStop>,
}

/// What the reader of a file could not keep of its gradients (criterion 27): the import
/// notice counts them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GradientLosses {
    /// Gradients with `spreadMethod` `reflect` or `repeat`, drawn as pad.
    pub repeats: usize,
    /// Radial gradients with a focal point away from the centre, read as the centre.
    pub focal_points: usize,
    /// Mesh gradients, read as no fill.
    pub meshes: usize,
}

impl GradientLosses {
    /// Whether nothing was lost.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.repeats == 0 && self.focal_points == 0 && self.meshes == 0
    }
}
