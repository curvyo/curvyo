//! The map from a glyph's em outline into text space: scale to the size, place at its origin, map
//! by the writing mode's frame, turn about its cluster's centre (`specs/0079-text-layout-controls`
//! criteria 11, 22, 23).

use kurbo::{Affine, Point};

use crate::layout::PlacedGlyph;

/// The map of `placed`'s outline (em units, y down, origin on the baseline) into text space:
/// scaled by `mm_per_em`, moved to its origin in line space and mapped by `frame` (a glyph that
/// stands upright in a column only has its origin mapped, so it is not turned), then turned by
/// `rotation_rad` about its turning point (`centre_x`, `centre_y`). Text space is y down, so a
/// positive angle turns clockwise on screen, the sign of the Select tool's rotate entry.
pub(crate) fn glyph_to_text(
    placed: &PlacedGlyph,
    frame: Affine,
    mm_per_em: f64,
    rotation_rad: f64,
) -> Affine {
    let scale = Affine::scale(mm_per_em);
    let placed_map = if placed.upright {
        Affine::translate((frame * placed.origin).to_vec2()) * scale
    } else {
        frame * Affine::translate(placed.origin.to_vec2()) * scale
    };
    if rotation_rad == 0.0 {
        return placed_map;
    }
    let pivot = frame * Point::new(placed.centre_x.0, placed.centre_y.0);
    Affine::rotate_about(rotation_rad, pivot) * placed_map
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
        let map = glyph_to_text(&placed, Affine::IDENTITY, 10.0, std::f64::consts::FRAC_PI_2);
        // The em point (0.5, -1.0) is (5, -10) in text space, 5 mm above the pivot (5, -5).
        let moved = map * Point::new(0.5, -1.0);
        assert!((moved.x - 10.0).abs() < 1e-9 && (moved.y + 5.0).abs() < 1e-9);
        let still = glyph_to_text(&placed, Affine::IDENTITY, 10.0, 0.0) * Point::new(0.5, -1.0);
        assert!((still.x - 5.0).abs() < 1e-9 && (still.y + 10.0).abs() < 1e-9);
    }
}
