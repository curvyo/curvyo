//! Glyph outlines as cubic Bézier paths: TrueType quadratics are degree-elevated exactly, CFF
//! cubics are kept (spec criterion 23).

use kurbo::{BezPath, PathEl, Point, Shape};
use skrifa::{
    FontRef, GlyphId, MetadataProvider,
    instance::{LocationRef, Size},
    outline::{DrawSettings, OutlinePen},
};

use crate::limits::MAX_GLYPH_POINTS;

/// One glyph's outline in em units, y down, origin at the glyph origin on the baseline. Only
/// `MoveTo`, `LineTo`, `CurveTo` and `ClosePath` elements; may be empty (a space, `.notdef`
/// without contours, or a glyph that could not be drawn).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GlyphOutline {
    /// The path.
    pub path: BezPath,
}

impl GlyphOutline {
    /// The distance from the baseline up to the outline's highest point, in em; `None` when empty.
    #[must_use]
    pub fn top(&self) -> Option<f64> {
        if self.path.elements().is_empty() {
            return None;
        }
        Some(-self.path.bounding_box().y0)
    }
}

/// Draws glyph `glyph` of `font` unhinted at its default instance, in em units (font units
/// divided by `units_per_em`). `None` when the glyph is absent, cannot be drawn, or exceeds
/// [`MAX_GLYPH_POINTS`].
pub(crate) fn draw_glyph(
    font: &FontRef<'_>,
    glyph: u32,
    units_per_em: f64,
) -> Option<GlyphOutline> {
    let outline = font.outline_glyphs().get(GlyphId::new(glyph))?;
    let mut pen = CubicPen {
        path: BezPath::new(),
        scale: 1.0 / units_per_em,
        current: Point::ZERO,
        points: 0,
        too_large: false,
    };
    let settings = DrawSettings::unhinted(Size::unscaled(), LocationRef::default());
    outline.draw(settings, &mut pen).ok()?;
    if pen.too_large || !pen.is_finite() {
        return None;
    }
    Some(GlyphOutline { path: pen.path })
}

/// Collects a glyph as cubics in em units, y flipped to point down.
struct CubicPen {
    path: BezPath,
    scale: f64,
    current: Point,
    points: usize,
    /// Set once the glyph passes [`MAX_GLYPH_POINTS`]; nothing is appended after that.
    too_large: bool,
}

impl CubicPen {
    fn point(&self, x: f32, y: f32) -> Point {
        Point::new(f64::from(x) * self.scale, -f64::from(y) * self.scale)
    }

    /// Counts `added` points; false (and nothing more is appended) past the limit.
    fn take(&mut self, added: usize) -> bool {
        self.points += added;
        if self.points > MAX_GLYPH_POINTS {
            self.too_large = true;
        }
        !self.too_large
    }

    fn is_finite(&self) -> bool {
        self.path.elements().iter().all(|element| {
            let points: &[Point] = match element {
                PathEl::MoveTo(p) | PathEl::LineTo(p) => std::slice::from_ref(p),
                PathEl::QuadTo(a, b) => &[*a, *b],
                PathEl::CurveTo(a, b, c) => &[*a, *b, *c],
                PathEl::ClosePath => &[],
            };
            points.iter().all(|p| p.x.is_finite() && p.y.is_finite())
        })
    }
}

impl OutlinePen for CubicPen {
    fn move_to(&mut self, x: f32, y: f32) {
        if !self.take(1) {
            return;
        }
        self.current = self.point(x, y);
        self.path.move_to(self.current);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        if !self.take(1) {
            return;
        }
        self.current = self.point(x, y);
        self.path.line_to(self.current);
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        if !self.take(2) {
            return;
        }
        let control = self.point(cx0, cy0);
        let end = self.point(x, y);
        // Exact degree elevation: the cubic with these handles is the same curve.
        let first = self.current + (control - self.current) * (2.0 / 3.0);
        let second = end + (control - end) * (2.0 / 3.0);
        self.path.curve_to(first, second, end);
        self.current = end;
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        if !self.take(3) {
            return;
        }
        let end = self.point(x, y);
        self.path
            .curve_to(self.point(cx0, cy0), self.point(cx1, cy1), end);
        self.current = end;
    }

    fn close(&mut self) {
        if self.too_large {
            return;
        }
        self.path.close_path();
    }
}
