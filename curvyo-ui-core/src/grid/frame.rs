//! The map between document and grid coordinates of one page.

use curvyo_document_core::{Page, Point, Vec2};

/// The grid's global origin and its axes. `u` runs along the rotated x axis, `v` along the
/// rotated y axis; a positive angle turns clockwise on screen (y grows downward). Each axis
/// points into the page from the page's origin corner (`specs/0090-grid` criterion 17), so the
/// decades of a logarithmic grid start at that corner and grow into the page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GridFrame {
    /// The global origin: the page's origin corner.
    pub origin: Point,
    u: Vec2,
    v: Vec2,
}

impl GridFrame {
    /// The frame of `page`'s grid.
    pub fn of(page: &Page) -> Self {
        let angle = page.grid.angle.as_radians();
        let (cos, sin) = (angle.cos(), angle.sin());
        let (sx, sy) = page.grid.origin.signs();
        Self {
            origin: page.zero_point(),
            u: Vec2::new(cos, sin).scaled(sx),
            v: Vec2::new(-sin, cos).scaled(sy),
        }
    }

    /// The unit vector of the `u` axis in document coordinates.
    pub const fn u_axis(self) -> Vec2 {
        self.u
    }

    /// The unit vector of the `v` axis in document coordinates.
    pub const fn v_axis(self) -> Vec2 {
        self.v
    }

    /// A document point in grid coordinates `(u, v)`.
    pub fn to_grid(self, point: Point) -> (f64, f64) {
        let d = self.origin.vector_to(point);
        (
            d.x * self.u.x + d.y * self.u.y,
            d.x * self.v.x + d.y * self.v.y,
        )
    }

    /// The document point at grid coordinates `(u, v)`.
    pub fn to_document(self, u: f64, v: f64) -> Point {
        Point::new(
            self.origin.x + u * self.u.x + v * self.v.x,
            self.origin.y + u * self.u.y + v * self.v.y,
        )
    }
}
