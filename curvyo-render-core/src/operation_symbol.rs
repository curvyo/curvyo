//! The symbol pills of the output page (`docs/ui-elements.md` §13, `specs/0101-laser-operations`
//! ux-notes 13.3, `specs/0164-ui-consistency` slice S3): one rounded pill per object, View:
//! Operations only, holding one 16 px symbol per kind the object is in.
//!
//! The pill sits centred 6 px above the object (Frage 459 A); while the object shows its transform
//! handles it takes the corner anchor right of the top-right corner, mirrored to the top-left at
//! the viewport edge. A pill never lies over a transform handle: it moves past the handles it
//! would touch, `HANDLE_GAP_PX` clear of their glyphs, and is not drawn when no place is left. A
//! pill is drawn only when its item is on screen and it does not collide with a pill already drawn;
//! at most `CHIP_CAP` pills are drawn per frame (the summary counts the rest).

use curvyo_document_core::{Angle, Point, ViewTransform};

use crate::glyphs::{DrawList, filled_rect, rounded_rect, thick_line};
use crate::select_decoration::TransformHandleGlyph;
use crate::theme::{OP_CHIP_GROUND, OP_CUT, OP_INK, OP_LANE_LINE, OP_MUTED_INK};

/// The pill's height, screen pixels.
pub const CHIP_PX: f64 = 22.0;
/// The padding between the pill's edge and its symbol cells, screen pixels.
pub const PILL_PADDING_PX: f64 = 3.0;
/// The side of one symbol cell, screen pixels.
pub const CELL_PX: f64 = 16.0;
/// The gap between two symbol cells of one pill, screen pixels.
pub const CHIP_GAP_PX: f64 = 4.0;
/// The corner anchor starts this far right of the object's top-right corner, screen pixels.
pub const CHIP_OFFSET_X_PX: f64 = 10.0;
/// The pill's bottom edge sits this far above the object's top edge, screen pixels.
pub const CHIP_OFFSET_Y_PX: f64 = 6.0;
/// The most pills of one frame.
pub const CHIP_CAP: usize = 500;
/// The least distance between a pill and a transform handle glyph, screen pixels.
pub const HANDLE_GAP_PX: f64 = 4.0;
/// How many handles in a row a pill moves past before it gives up its place.
const HANDLE_STEPS: usize = 16;
/// The width of the symbol outlines (Cut, Cut-out, Engrave + Outline), screen pixels; the outline
/// is drawn inside its cell, half its width in from the cell edge.
const OUTLINE_PX: f64 = 1.5;

/// The width of a pill of `count` symbols, screen pixels: `6 + 16n + 4(n - 1)`.
#[must_use]
pub fn pill_width(count: usize) -> f64 {
    // At most five symbols per object: the count is small.
    #[allow(clippy::cast_precision_loss)]
    let n = count.max(1) as f64;
    2.0 * PILL_PADDING_PX + CELL_PX * n + CHIP_GAP_PX * (n - 1.0)
}

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
    /// Image (`specs/0123` UX notes 2.1): a dot ramp of 2 x 2 squares on a 4 px pitch, four
    /// columns of 4, 3, 2 and 1 dots.
    Image,
}

/// The dots of the Image glyph by column: the rows (0 to 3) that hold a dot.
const IMAGE_RAMP: [&[u8]; 4] = [&[0, 1, 2, 3], &[0, 1, 3], &[0, 2], &[1]];

/// One object to mark: its box in document millimetres and the symbols of its kinds, in kind order.
#[derive(Debug, Clone, PartialEq)]
pub struct ChipItem {
    /// The object's box, `(min, max)`.
    pub bounds: (Point, Point),
    /// One symbol per kind the object is in; a repeated kind is given once.
    pub glyphs: Vec<Glyph>,
    /// The symbols of `glyphs` drawn in `--panel-muted-fg`: kinds that hold the object but do
    /// not run it (Output off, `0101` UX notes 13.2; a vector shape in Image, `specs/0123`
    /// criterion 48).
    pub muted: Vec<Glyph>,
    /// The object shows its own transform handles (it is the one selected object): the pill takes
    /// the corner anchor instead of the centred one.
    pub handles: bool,
}

/// The pills of `items`, in order, for a canvas of `canvas` screen pixels, with no transform handle
/// on screen. Items that are off the canvas get none; a pill that would collide with one already
/// placed is skipped.
#[must_use]
pub fn chips_list(items: &[ChipItem], canvas: (f64, f64), view: ViewTransform) -> DrawList {
    chips_list_clear_of(items, canvas, view, &[])
}

/// [`chips_list`] with the transform handles `handles` on screen: no pill lies over one.
#[must_use]
pub fn chips_list_clear_of(
    items: &[ChipItem],
    canvas: (f64, f64),
    view: ViewTransform,
    handles: &[TransformHandleGlyph],
) -> DrawList {
    let blocked: Vec<(f64, f64, f64, f64)> = handles
        .iter()
        .map(|handle| {
            let (x, y) = view.document_to_screen(handle.position);
            let (w, h) = handle.kind.footprint_px();
            let (hw, hh) = (w / 2.0 + HANDLE_GAP_PX, h / 2.0 + HANDLE_GAP_PX);
            (x - hw, y - hh, x + hw, y + hh)
        })
        .collect();
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
        let (sx0, sy0) = view.document_to_screen(min);
        let (sx1, sy1) = view.document_to_screen(max);
        // Off the canvas: the whole item is outside the viewport.
        if sx1 < 0.0 || sy1 < 0.0 || sx0 > canvas.0 || sy0 > canvas.1 {
            continue;
        }
        if placed.len() >= CHIP_CAP {
            return list;
        }
        let Some(rect) = pill_rect(item, (sx0, sy0, sx1), canvas.0, &blocked) else {
            continue;
        };
        if placed.iter().any(|other| overlaps(*other, rect)) {
            continue;
        }
        placed.push(rect);
        list.extend(pill(item, rect, &to_doc, scale));
    }
    list
}

/// The screen rectangle of the pill of `item` whose box spans `x0..x1` with its top edge at `top`
/// (`(x0, top, x1)`), on a canvas `width` wide, clear of the handle boxes `blocked`; `None` when
/// no place is left.
fn pill_rect(
    item: &ChipItem,
    (x0, top, x1): (f64, f64, f64),
    width: f64,
    blocked: &[(f64, f64, f64, f64)],
) -> Option<(f64, f64, f64, f64)> {
    let w = pill_width(item.glyphs.len());
    let y = top - CHIP_OFFSET_Y_PX - CHIP_PX;
    let at = |left: f64| (left, y, left + w, y + CHIP_PX);
    let fits = |left: f64| left >= 0.0 && left + w <= width;
    if item.handles {
        // Right of the corner past the handles there; else left of the top-left corner.
        let right = clear(at(x1 + CHIP_OFFSET_X_PX), blocked, 1.0);
        if fits(right.0) {
            return Some(right);
        }
        let left = clear(at(x0 - CHIP_OFFSET_X_PX - w), blocked, -1.0);
        return fits(left.0).then_some(left);
    }
    // Centred over the object, moved inside the canvas when it would leave it (Proposal); past a
    // handle it would touch, to the right, else to the left.
    let centred = at((f64::midpoint(x0, x1) - w / 2.0).min(width - w).max(0.0));
    [1.0, -1.0]
        .into_iter()
        .map(|direction| clear(centred, blocked, direction))
        .find(|rect| fits(rect.0))
}

/// `rect` moved along x in `direction` (1 right, -1 left) past every box of `blocked` it touches.
/// After `HANDLE_STEPS` moves it is returned off the canvas, so the caller drops it.
fn clear(
    mut rect: (f64, f64, f64, f64),
    blocked: &[(f64, f64, f64, f64)],
    direction: f64,
) -> (f64, f64, f64, f64) {
    for _ in 0..HANDLE_STEPS {
        let Some(hit) = blocked.iter().find(|b| overlaps(**b, rect)) else {
            return rect;
        };
        let w = rect.2 - rect.0;
        let left = if direction > 0.0 { hit.2 } else { hit.0 - w };
        rect = (left, rect.1, left + w, rect.3);
    }
    (f64::NEG_INFINITY, rect.1, f64::NEG_INFINITY, rect.3)
}

fn overlaps(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}

/// One pill over the screen rectangle `rect`: a 1 px `--lane-line` edge, the `--toolbar-bg`
/// ground, both fully rounded, and the symbols in their cells, in muted ink when muted.
fn pill(
    item: &ChipItem,
    rect: (f64, f64, f64, f64),
    to_doc: &dyn Fn(f64, f64) -> Point,
    scale: f64,
) -> DrawList {
    let (w, h) = (rect.2 - rect.0, rect.3 - rect.1);
    let centre = to_doc(rect.0 + w / 2.0, rect.1 + h / 2.0);
    let flat = Angle::from_radians(0.0);
    let mut list = rounded_rect(
        centre,
        (w / scale, h / scale),
        h / 2.0 / scale,
        flat,
        OP_LANE_LINE,
    );
    list.extend(rounded_rect(
        centre,
        ((w - 2.0) / scale, (h - 2.0) / scale),
        (h - 2.0) / 2.0 / scale,
        flat,
        OP_CHIP_GROUND,
    ));
    for (index, glyph) in (0_u32..).zip(&item.glyphs) {
        let cell_x = rect.0 + PILL_PADDING_PX + f64::from(index) * (CELL_PX + CHIP_GAP_PX);
        let cell_y = rect.1 + PILL_PADDING_PX;
        let at = |x: f64, y: f64| to_doc(cell_x + x, cell_y + y);
        list.extend(symbol(*glyph, &at, scale, item.muted.contains(glyph)));
    }
    list
}

/// The symbol inside its 16 px box; `at` maps box pixels to document millimetres. A muted symbol
/// is all `--panel-muted-fg`.
fn symbol(glyph: Glyph, at: &dyn Fn(f64, f64) -> Point, scale: f64, muted: bool) -> DrawList {
    let mut list = DrawList::default();
    let red = matches!(glyph, Glyph::Cut | Glyph::CutOut);
    let ink = if muted { OP_MUTED_INK } else { OP_INK };
    let colour = if red && !muted { OP_CUT } else { ink };
    let box_width = OUTLINE_PX / scale;
    match glyph {
        Glyph::Cut => list.extend(square_outline(at, box_width, colour, None)),
        Glyph::CutOut => list.extend(square_outline(at, box_width, colour, Some((4.0, 2.0)))),
        Glyph::Score => {
            // Five 1.5 px dots every 3 px, centred in the cell: 1.25 to 14.75.
            for dot in 0_u8..5 {
                let x = 1.25 + f64::from(dot) * 3.0;
                list.extend(filled_rect(at(x, 7.25), at(x + 1.5, 8.75), colour));
            }
        }
        Glyph::Engrave => list.extend(hatch_box(at, 1.0 / scale, ink)),
        Glyph::EngraveOutline => {
            list.extend(hatch_box(at, 1.0 / scale, ink));
            list.extend(square_outline(at, box_width, ink, None));
        }
        Glyph::Image => {
            for (column, rows) in (0_u8..).zip(IMAGE_RAMP) {
                for &row in rows {
                    let (x, y) = (f64::from(column) * 4.0 + 1.0, f64::from(row) * 4.0 + 1.0);
                    list.extend(filled_rect(at(x, y), at(x + 2.0, y + 2.0), ink));
                }
            }
        }
    }
    list
}

/// The outline of the 16 px cell, drawn inside it (its centre line half the outline width in from
/// the cell edge, so the stroke ends on the edge and the symbol stays centred); `dash` = (on, off)
/// pixels, or a solid outline.
fn square_outline(
    at: &dyn Fn(f64, f64) -> Point,
    width_mm: f64,
    colour: crate::color::RgbaColor,
    dash: Option<(f64, f64)>,
) -> DrawList {
    let mut list = DrawList::default();
    let (near, far) = (OUTLINE_PX / 2.0, CELL_PX - OUTLINE_PX / 2.0);
    let corners = [
        (near, near),
        (far, near),
        (far, far),
        (near, far),
        (near, near),
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
#[path = "operation_symbol_tests.rs"]
mod tests;
