//! The symbol chips of the output page (`specs/0101-laser-operations` ux-notes section 13.3): one
//! 22 px chip per kind an object is in, top-right of the object, View: Operations only. A chip is
//! drawn only when its item is on screen, it does not collide with a chip already drawn, and at
//! most `CHIP_CAP` chips are drawn per frame (the summary counts the rest).

use curvyo_document_core::{Point, ViewTransform};

use crate::glyphs::{DrawList, filled_rect, thick_line};
use crate::theme::{OP_CHIP_GROUND, OP_CUT, OP_INK, OP_LANE_LINE};

/// The chip's side, screen pixels.
pub const CHIP_PX: f64 = 22.0;
/// The gap between two chips of one item, screen pixels.
pub const CHIP_GAP_PX: f64 = 4.0;
/// The chip row starts this far right of the object's top-right corner, screen pixels.
pub const CHIP_OFFSET_X_PX: f64 = 10.0;
/// The chip row's bottom edge sits this far above the object's top edge, screen pixels.
pub const CHIP_OFFSET_Y_PX: f64 = 6.0;
/// The most chips of one frame.
pub const CHIP_CAP: usize = 500;

/// The symbol of one kind (ux-notes section 3). Cut and Cut-out are red; the rest ink.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    /// A solid square outline, 1.5 px.
    Cut,
    /// A dashed square outline, 1.5 px, dash 4 on 2.
    CutOut,
    /// A dotted line through the middle, round dots every 3 px.
    Score,
    /// A square with three 45 degree hatch lines inside, 1 px.
    Engrave,
    /// The Engrave hatch with a 1.5 px square outline.
    EngraveOutline,
}

/// One object to mark: its box in document millimetres and the symbols of its kinds, in kind order.
#[derive(Debug, Clone, PartialEq)]
pub struct ChipItem {
    /// The object's box, `(min, max)`.
    pub bounds: (Point, Point),
    /// One symbol per kind the object is in; a repeated kind is given once.
    pub glyphs: Vec<Glyph>,
}

/// The chips of `items`, in order, for a canvas of `canvas` screen pixels. Items that are off the
/// canvas get none; a chip that would collide with one already placed is skipped.
#[must_use]
pub fn chips_list(items: &[ChipItem], canvas: (f64, f64), view: ViewTransform) -> DrawList {
    let mut list = DrawList::default();
    let mut placed: Vec<(f64, f64, f64, f64)> = Vec::new();
    let origin = view.document_to_screen(Point::new(0.0, 0.0));
    let scale = view.scale();
    if scale <= 0.0 {
        return list;
    }
    let to_doc = |x: f64, y: f64| Point::new((x - origin.0) / scale, (y - origin.1) / scale);
    for item in items {
        if item.glyphs.is_empty() {
            continue;
        }
        let (min, max) = item.bounds;
        let top_right = view.document_to_screen(Point::new(max.x, min.y));
        let top_left = view.document_to_screen(Point::new(min.x, min.y));
        let count = item.glyphs.len();
        // At most five symbols per object: the counts are small.
        #[allow(clippy::cast_precision_loss)]
        let width = count as f64 * CHIP_PX + (count - 1) as f64 * CHIP_GAP_PX;
        let top = top_right.1 - CHIP_OFFSET_Y_PX - CHIP_PX;
        // Right of the corner, or the top-left corner when the row would leave the canvas.
        let mut left = top_right.0 + CHIP_OFFSET_X_PX;
        if left + width > canvas.0 {
            left = top_left.0 - CHIP_OFFSET_X_PX - width;
        }
        // Off the canvas: the whole item is outside the viewport.
        let (sx0, sx1) = (
            view.document_to_screen(min).0,
            view.document_to_screen(max).0,
        );
        let (sy0, sy1) = (
            view.document_to_screen(min).1,
            view.document_to_screen(max).1,
        );
        if sx1 < 0.0 || sy1 < 0.0 || sx0 > canvas.0 || sy0 > canvas.1 {
            continue;
        }
        for (index, glyph) in item.glyphs.iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let x = left + index as f64 * (CHIP_PX + CHIP_GAP_PX);
            let rect = (x, top, x + CHIP_PX, top + CHIP_PX);
            if placed.len() >= CHIP_CAP {
                return list;
            }
            if placed.iter().any(|other| overlaps(*other, rect)) {
                continue;
            }
            placed.push(rect);
            let a = to_doc(rect.0, rect.1);
            let b = to_doc(rect.2, rect.3);
            list.extend(chip(a, b, *glyph, scale));
        }
    }
    list
}

fn overlaps(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}

/// One chip: ground, 1 px border, and the symbol at a 2 px inset.
fn chip(a: Point, b: Point, glyph: Glyph, scale: f64) -> DrawList {
    let mut list = filled_rect(a, b, OP_CHIP_GROUND);
    let edge = 1.0 / scale;
    list.extend(filled_rect(a, Point::new(b.x, a.y + edge), OP_LANE_LINE));
    list.extend(filled_rect(Point::new(a.x, b.y - edge), b, OP_LANE_LINE));
    list.extend(filled_rect(a, Point::new(a.x + edge, b.y), OP_LANE_LINE));
    list.extend(filled_rect(Point::new(b.x - edge, a.y), b, OP_LANE_LINE));
    let inset = 2.0;
    let at = |x: f64, y: f64| Point::new(a.x + (inset + x) / scale, a.y + (inset + y) / scale);
    list.extend(symbol(glyph, &at, scale));
    list
}

/// The symbol inside its 16 px box; `at` maps box pixels to document millimetres.
fn symbol(glyph: Glyph, at: &dyn Fn(f64, f64) -> Point, scale: f64) -> DrawList {
    let mut list = DrawList::default();
    let red = matches!(glyph, Glyph::Cut | Glyph::CutOut);
    let colour = if red { OP_CUT } else { OP_INK };
    let box_width = 1.5 / scale;
    match glyph {
        Glyph::Cut => list.extend(square_outline(at, 16.0, box_width, colour, None)),
        Glyph::CutOut => list.extend(square_outline(
            at,
            16.0,
            box_width,
            colour,
            Some((4.0, 2.0)),
        )),
        Glyph::Score => {
            let mut x = 0.0;
            while x < 16.0 {
                let p = at(x, 7.25);
                let q = at(x + 1.5, 8.75);
                list.extend(filled_rect(p, q, colour));
                x += 3.0;
            }
        }
        Glyph::Engrave => list.extend(hatch_box(at, 1.0 / scale, OP_INK)),
        Glyph::EngraveOutline => {
            list.extend(hatch_box(at, 1.0 / scale, OP_INK));
            list.extend(square_outline(at, 16.0, box_width, OP_INK, None));
        }
    }
    list
}

/// A square outline of `size` box pixels; `dash` = (on, off) pixels, or a solid outline.
fn square_outline(
    at: &dyn Fn(f64, f64) -> Point,
    size: f64,
    width_mm: f64,
    colour: crate::color::RgbaColor,
    dash: Option<(f64, f64)>,
) -> DrawList {
    let mut list = DrawList::default();
    let corners = [
        (0.0, 0.0),
        (size, 0.0),
        (size, size),
        (0.0, size),
        (0.0, 0.0),
    ];
    for pair in corners.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let length = (to.0 - from.0).abs() + (to.1 - from.1).abs();
        match dash {
            None => list.extend(thick_line(
                at(from.0, from.1),
                at(to.0, to.1),
                width_mm,
                colour,
            )),
            Some((on, off)) => {
                let mut t = 0.0;
                while t < length {
                    let end = (t + on).min(length);
                    let along =
                        |u: f64| (from.0 + (to.0 - from.0) * u, from.1 + (to.1 - from.1) * u);
                    let p = along(t / length);
                    let q = along(end / length);
                    list.extend(thick_line(at(p.0, p.1), at(q.0, q.1), width_mm, colour));
                    t += on + off;
                }
            }
        }
    }
    list
}

/// Three 45 degree lines inside the 16 px box.
fn hatch_box(
    at: &dyn Fn(f64, f64) -> Point,
    width_mm: f64,
    colour: crate::color::RgbaColor,
) -> DrawList {
    let mut list = DrawList::default();
    for c in [-5.0_f64, 0.0, 5.0] {
        // The line y = x - c inside [0, 16]^2.
        let x0 = c.max(0.0);
        let x1 = (16.0 + c).min(16.0);
        list.extend(thick_line(at(x0, x0 - c), at(x1, x1 - c), width_mm, colour));
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(x: f64, glyphs: Vec<Glyph>) -> ChipItem {
        ChipItem {
            bounds: (Point::new(x, 10.0), Point::new(x + 20.0, 30.0)),
            glyphs,
        }
    }

    #[test]
    fn an_item_gets_one_chip_per_kind_and_none_off_the_canvas() {
        let view = ViewTransform::new(2.0, Point::new(0.0, 0.0));
        let one = chips_list(
            &[item(10.0, vec![Glyph::Cut, Glyph::Score])],
            (800.0, 600.0),
            view,
        );
        let none = chips_list(&[item(10_000.0, vec![Glyph::Cut])], (800.0, 600.0), view);
        assert!(one.triangle_count() > 0);
        assert_eq!(none.triangle_count(), 0);
    }

    #[test]
    fn colliding_chips_are_skipped() {
        let view = ViewTransform::new(2.0, Point::new(0.0, 0.0));
        let twice = chips_list(
            &[item(10.0, vec![Glyph::Cut]), item(10.0, vec![Glyph::Cut])],
            (800.0, 600.0),
            view,
        );
        let once = chips_list(&[item(10.0, vec![Glyph::Cut])], (800.0, 600.0), view);
        assert_eq!(twice.triangle_count(), once.triangle_count());
    }

    #[test]
    fn the_cap_holds_the_chips_of_one_frame() {
        let view = ViewTransform::new(1.0, Point::new(0.0, 0.0));
        let items: Vec<ChipItem> = (0..700)
            .map(|i| ChipItem {
                bounds: (
                    Point::new(f64::from(i) * 40.0, 10.0),
                    Point::new(f64::from(i) * 40.0 + 20.0, 30.0),
                ),
                glyphs: vec![Glyph::Cut],
            })
            .collect();
        let capped = chips_list(&items, (100_000.0, 100_000.0), view);
        let expected = chips_list(&items[..CHIP_CAP], (100_000.0, 100_000.0), view);
        assert_eq!(capped.triangle_count(), expected.triangle_count());
    }
}
