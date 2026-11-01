//! Clears room for a dimension's value text: its own lines break around it, other texts slide away.
//!
//! The text is a DOM label placed by Rust; its box is estimated from the character count
//! (UX notes revision 2, section 7: "the line does not run under the digits").
use super::{DimensionGeometry, DimensionItem, DimensionMarks, at, linear_frame};
use curvyo_document_core::Point;

/// Text box estimate: width per character plus padding, and the box height (12px text).
const TEXT_CHAR_PX: f64 = 7.0;
const TEXT_PAD_PX: f64 = 8.0;
const TEXT_HEIGHT_PX: f64 = 16.0;
/// Clearance between the text box and the dimension line that breaks around it.
const TEXT_GAP_PX: f64 = 4.0;
/// Pieces of a broken line shorter than this, in millimetres, are dropped.
const MIN_PIECE_MM: f64 = 1e-9;

/// Half width and half height of a text box of `chars` characters, in screen pixels.
pub(super) fn text_half_px(chars: usize) -> (f64, f64) {
    #[allow(clippy::cast_precision_loss)] // a short text's character count
    let width = TEXT_CHAR_PX * chars as f64 + TEXT_PAD_PX;
    (width / 2.0, TEXT_HEIGHT_PX / 2.0)
}

/// Cuts every line where it would run under the text box plus its gap, so the value is never
/// struck through (UX notes revision 2, section 7: "the line does not run under the digits").
pub(super) fn break_around_text(marks: &mut DimensionMarks, chars: usize, px: f64) {
    let (w, h) = text_half_px(chars);
    let (w, h) = ((w + TEXT_GAP_PX) * px, (h + TEXT_GAP_PX) * px);
    let c = marks.text;
    marks.lines = marks
        .lines
        .iter()
        .flat_map(|&(p, q)| {
            let Some((t0, t1)) = box_interval(p, q, (c.x - w, c.x + w), (c.y - h, c.y + h)) else {
                return vec![(p, q)];
            };
            let point = |t: f64| Point::new(p.x + (q.x - p.x) * t, p.y + (q.y - p.y) * t);
            let length = p.vector_to(q).length();
            let mut pieces = Vec::new();
            if t0 * length > MIN_PIECE_MM {
                pieces.push((p, point(t0)));
            }
            if (1.0 - t1) * length > MIN_PIECE_MM {
                pieces.push((point(t1), q));
            }
            pieces
        })
        .collect();
}

/// The parameter range of segment `p`-`q` inside an axis-aligned box (Liang-Barsky), or `None`
/// when the segment misses the box or only touches it.
fn box_interval(p: Point, q: Point, xs: (f64, f64), ys: (f64, f64)) -> Option<(f64, f64)> {
    let (dx, dy) = (q.x - p.x, q.y - p.y);
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for (step, room) in [
        (-dx, p.x - xs.0),
        (dx, xs.1 - p.x),
        (-dy, p.y - ys.0),
        (dy, ys.1 - p.y),
    ] {
        if step.abs() <= f64::EPSILON {
            if room <= 0.0 {
                return None;
            }
        } else if step < 0.0 {
            t0 = t0.max(room / step);
        } else {
            t1 = t1.min(room / step);
        }
    }
    (t1 - t0 > f64::EPSILON).then_some((t0, t1))
}

/// A text centre for `marks` that clears every earlier text box, sliding along the dimension
/// line by up to a third of its length each way; `None` when no slide clears.
pub(super) fn free_text(
    item: &DimensionItem,
    marks: &DimensionMarks,
    earlier: &[DimensionMarks],
    items: &[DimensionItem],
    px: f64,
) -> Option<Point> {
    let half = |chars: usize| {
        let (w, h) = text_half_px(chars);
        (w * px, h * px)
    };
    let (w, h) = half(item.text_chars);
    let clear = |p: Point| {
        earlier.iter().zip(items).all(|(other, other_item)| {
            let (ow, oh) = half(other_item.text_chars);
            (p.x - other.text.x).abs() >= w + ow || (p.y - other.text.y).abs() >= h + oh
        })
    };
    if clear(marks.text) {
        return Some(marks.text);
    }
    let (along, reach) = match item.geometry {
        DimensionGeometry::Linear { from, to } => {
            let (along, _, length) = linear_frame(from, to);
            (along, length / 3.0)
        }
        DimensionGeometry::Projected { from, to, along } => {
            let along = along.normalized_to(1.0);
            let span = from.vector_to(to);
            (along, (span.x * along.x + span.y * along.y).abs() / 3.0)
        }
        _ => return None,
    };
    (1..=6)
        .flat_map(|step| [f64::from(step), -f64::from(step)])
        .map(|step| at(marks.text, along, reach * step / 6.0))
        .find(|p| clear(*p))
}
