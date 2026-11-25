//! The overlay of a path box: strokes, discs and rings in page mm that show how the outline was
//! read (`specs/0149-path-box` UX notes §3). Editor decoration, never artwork.

use curvyo_document_core::Point;

use crate::color::RgbaColor;
use crate::glyphs::{DrawList, circle, ring, thick_line};
use crate::theme::{ACCENT, DRAG_GHOST, REFUSAL_OUTLINE};

/// The look of one piece of the overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkTone {
    /// The accent: corner crosses and badge discs.
    Accent,
    /// The drag-ghost colour: the stripes of a bent span.
    Stripe,
    /// The warning colour: a very sharp corner, a bend too tight for the wood.
    Warning,
    /// White, on a badge disc: its digits.
    OnDisc,
}

fn colour(tone: MarkTone) -> RgbaColor {
    match tone {
        MarkTone::Accent => ACCENT,
        MarkTone::Stripe => DRAG_GHOST,
        MarkTone::Warning => REFUSAL_OUTLINE,
        MarkTone::OnDisc => RgbaColor::WHITE,
    }
}

/// One stroke of `width_mm` between two points.
pub type MarkLine = (Point, Point, MarkTone);
/// A disc or a ring: centre, diameter in mm and tone.
pub type MarkRound = (Point, f64, MarkTone);

/// Draws `lines` `width_mm` wide, `discs` filled and `rings` 1.5 `pixel` thick, in that order.
#[must_use]
pub fn build_marks_overlay(
    lines: &[MarkLine],
    discs: &[MarkRound],
    rings: &[MarkRound],
    pixel: f64,
) -> DrawList {
    let mut list = DrawList::default();
    for &(at, diameter, tone) in discs {
        list.extend(circle(at, diameter, colour(tone)));
    }
    for &(a, b, tone) in lines {
        let width = if tone == MarkTone::OnDisc { 1.2 } else { 1.5 } * pixel;
        list.extend(thick_line(a, b, width, colour(tone)));
    }
    for &(at, diameter, tone) in rings {
        list.extend(ring(at, diameter, 1.5 * pixel, colour(tone)));
    }
    list
}
