//! The map from a glyph's em outline into text space: scale to the size, place at its origin, map
//! by the writing mode's frame, taper in the glyph's own frame and turn about its cluster's centre
//! (`specs/0079-text-layout-controls` criteria 11, 17, 19, 22, 23).

use kurbo::{Affine, BezPath, Point};

use crate::layout::PlacedGlyph;
use crate::taper::Taper;

/// The map of one glyph's em outline into text space, in three parts: em to the glyph frame (mm,
/// y down, its baseline at y = 0), the taper in that frame, and the frame to text space.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GlyphMap {
    /// Em units to the glyph frame: the scale.
    inner: Affine,
    /// The taper in the glyph frame; `None` for none.
    taper: Option<Taper>,
    /// The glyph frame to text space: origin, writing frame, turn.
    outer: Affine,
}

impl GlyphMap {
    /// `outline` (em) in text space, curves within `accuracy` mm of the exact taper.
    pub fn path(&self, outline: &BezPath, accuracy: f64) -> BezPath {
        let framed = self.inner * outline;
        match self.taper {
            Some(taper) => self.outer * &taper.path(&framed, accuracy),
            None => self.outer * &framed,
        }
    }
}

/// The map of `placed`'s outline (em units, y down, origin on the baseline) into text space:
/// scaled by `mm_per_em`, tapered by `taper` (a fraction, criterion 17) about the middle of its
/// advance in its own frame, moved to its origin in line space and mapped by `frame` (a glyph
/// that stands upright in a column only has its origin mapped, so it is not turned), then turned
/// by `rotation_rad` about its turning point (`centre_x`, `centre_y`): the taper comes first
/// (criterion 19). Text space is y down, so a positive angle turns clockwise on screen, the sign
/// of the Select tool's rotate entry.
pub(crate) fn glyph_to_text(
    placed: &PlacedGlyph,
    frame: Affine,
    mm_per_em: f64,
    (rotation_rad, taper, size): (f64, f64, f64),
) -> GlyphMap {
    let pivot = frame * Point::new(placed.centre_x.0, placed.centre_y.0);
    let (origin, centre_x) = if placed.upright {
        let origin = frame * placed.origin;
        (Affine::translate(origin.to_vec2()), pivot.x - origin.x)
    } else {
        (
            frame * Affine::translate(placed.origin.to_vec2()),
            placed.centre_x.0 - placed.origin.x,
        )
    };
    let turn = if rotation_rad == 0.0 {
        Affine::IDENTITY
    } else {
        Affine::rotate_about(rotation_rad, pivot)
    };
    GlyphMap {
        inner: Affine::scale(mm_per_em),
        taper: (taper != 0.0).then_some(Taper {
            centre_x,
            amount: taper,
            size,
        }),
        outer: turn * origin,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Mm;

    #[test]
    fn a_quarter_turn_moves_the_top_to_the_right_of_the_pivot() {
        let placed = PlacedGlyph {
            glyph: 1,
            origin: Point::new(0.0, 0.0),
            cluster: 0,
            centre_x: Mm(5.0),
            centre_y: Mm(-5.0),
            baseline: Mm(0.0),
            upright: false,
            rotation_rad: 0.0,
        };
        let mut dot = BezPath::new();
        dot.move_to((0.5, -1.0));
        let at = |rotation: f64| {
            let map = glyph_to_text(&placed, Affine::IDENTITY, 10.0, (rotation, 0.0, 10.0));
            match map.path(&dot, 0.01).elements()[0] {
                kurbo::PathEl::MoveTo(p) => p,
                _ => Point::ZERO,
            }
        };
        // The em point (0.5, -1.0) is (5, -10) in text space, 5 mm above the pivot (5, -5).
        let moved = at(std::f64::consts::FRAC_PI_2);
        assert!((moved.x - 10.0).abs() < 1e-9 && (moved.y + 5.0).abs() < 1e-9);
        let still = at(0.0);
        assert!((still.x - 5.0).abs() < 1e-9 && (still.y + 10.0).abs() < 1e-9);
    }
}
