//! What an embroidery page draws (`specs/0160-embroidery` criteria 3, 12, 13, `ux-notes.md` 3,
//! 6.1): the hoop frame and its sewing field under the page's items, and the stitch list as the
//! Toolpath view up to the play position, in thread colours over a casing.

use curvyo_document_core::{Page, Point, Vec2, ViewTransform};
use curvyo_embroidery_core::{HoopSize, Stitch, StitchKind, ThreadColour};
use curvyo_render_core::{
    DrawList, RgbaColor, ToolpathLook, ToolpathSegment, ToolpathStroke, Vertex, build_toolpath,
    thick_line,
};

use super::super::Session;
use super::embroidery_page;

/// `--toolbar-icon`, the ink.
const INK: RgbaColor = RgbaColor::opaque(0x3A, 0x3A, 0x3F);
/// `--selection-casing`, white.
const WHITE: RgbaColor = RgbaColor::opaque(0xFF, 0xFF, 0xFF);
/// `--toolbar-bg`, the frame's ground.
const FRAME: RgbaColor = RgbaColor::opaque(0xDC, 0xDC, 0xE0);
/// The frame band's gap outside the sewing field and its width in mm (UX E2).
const FRAME_GAP_MM: f64 = 8.0;
const FRAME_BAND_MM: f64 = 12.0;
/// The template marks: the side ticks and the centre cross in mm.
const TICK_MM: f64 = 6.0;
const CROSS_MM: f64 = 10.0;
/// The trim mark's and the colour change's disc radius in px.
const MARK_PX: f64 = 7.0;

/// A rectangle `(left, top, width, height)` in mm.
type Rect = (f64, f64, f64, f64);

fn corners((left, top, width, height): Rect) -> [Point; 4] {
    [
        Point::new(left, top),
        Point::new(left + width, top),
        Point::new(left + width, top + height),
        Point::new(left, top + height),
    ]
}

fn push_rect(list: &mut DrawList, rect: Rect, color: RgbaColor) {
    let p = corners(rect);
    for i in [0, 1, 2, 0, 2, 3] {
        list.triangles.push(Vertex {
            position: p[i],
            color,
        });
    }
}

fn push_outline(list: &mut DrawList, rect: Rect, width: f64, color: RgbaColor) {
    let c = corners(rect);
    for i in 0..4 {
        list.extend(thick_line(c[i], c[(i + 1) % 4], width, color));
    }
}

fn push_disc(list: &mut DrawList, centre: Point, radius: f64, color: RgbaColor) {
    const STEPS: u32 = 12;
    for k in 0..STEPS {
        let a = |k: u32| f64::from(k) * std::f64::consts::TAU / f64::from(STEPS);
        let p = |a: f64| centre.translated(Vec2::new(radius * a.cos(), radius * a.sin()));
        for position in [centre, p(a(k)), p(a(k + 1))] {
            list.triangles.push(Vertex { position, color });
        }
    }
}

/// The hoop template's marks: the centre cross at `c` and a tick inward at each side's middle
/// of `field`.
fn push_marks(list: &mut DrawList, c: Point, field: Rect, px: f64) {
    let (w, h) = (field.2, field.3);
    let half = CROSS_MM / 2.0;
    list.extend(thick_line(
        c.translated(Vec2::new(-half, 0.0)),
        c.translated(Vec2::new(half, 0.0)),
        px,
        INK,
    ));
    list.extend(thick_line(
        c.translated(Vec2::new(0.0, -half)),
        c.translated(Vec2::new(0.0, half)),
        px,
        INK,
    ));
    for (from, to) in [
        (Point::new(c.x, field.1), Point::new(c.x, field.1 + TICK_MM)),
        (
            Point::new(c.x, field.1 + h),
            Point::new(c.x, field.1 + h - TICK_MM),
        ),
        (Point::new(field.0, c.y), Point::new(field.0 + TICK_MM, c.y)),
        (
            Point::new(field.0 + w, c.y),
            Point::new(field.0 + w - TICK_MM, c.y),
        ),
    ] {
        list.extend(thick_line(from, to, px, INK));
    }
}

/// The hoop `hoop` centred on `page`, first-wins order: template marks, field edge, frame
/// outlines, frame band (UX 3, E2; corners square in slice 1). `px` is one device pixel in mm.
fn hoop_list(hoop: HoopSize, page: &Page, px: f64) -> DrawList {
    let mut list = DrawList::default();
    let (w, h) = hoop.field_mm();
    let (w, h) = (f64::from(w), f64::from(h));
    let c = Point::new(
        page.origin.x + page.size.width.as_mm() / 2.0,
        page.origin.y + page.size.height.as_mm() / 2.0,
    );
    let field = (c.x - w / 2.0, c.y - h / 2.0, w, h);
    push_marks(&mut list, c, field, px);
    push_outline(&mut list, field, px, INK);
    let inner = (
        field.0 - FRAME_GAP_MM,
        field.1 - FRAME_GAP_MM,
        w + 2.0 * FRAME_GAP_MM,
        h + 2.0 * FRAME_GAP_MM,
    );
    let band = FRAME_BAND_MM;
    let outer = (
        inner.0 - band,
        inner.1 - band,
        inner.2 + 2.0 * band,
        inner.3 + 2.0 * band,
    );
    push_outline(&mut list, inner, 1.5 * px, INK);
    push_outline(&mut list, outer, 1.5 * px, INK);
    push_rect(&mut list, (outer.0, outer.1, outer.2, band), FRAME);
    push_rect(
        &mut list,
        (outer.0, inner.1 + inner.3, outer.2, band),
        FRAME,
    );
    push_rect(&mut list, (outer.0, inner.1, band, inner.3), FRAME);
    push_rect(
        &mut list,
        (inner.0 + inner.2, inner.1, band, inner.3),
        FRAME,
    );
    list
}

/// The relative luminance of an sRGB colour (WCAG).
fn luminance(c: RgbaColor) -> f64 {
    let lin = |v: u8| {
        let v = f64::from(v) / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
}

/// The thread casing: white or ink, whichever contrasts more with `thread` (UX E1).
fn casing(thread: RgbaColor) -> RgbaColor {
    let l = luminance(thread);
    let on_white = 1.05 / (l + 0.05);
    let on_ink = (l + 0.05) / (luminance(INK) + 0.05);
    if on_white >= on_ink { WHITE } else { INK }
}

fn rgba(thread: ThreadColour) -> RgbaColor {
    let (r, g, b) = thread.channels();
    RgbaColor::opaque(r, g, b)
}

impl Session {
    /// Adds the hoop of every embroidery page under that page's grid (`grids[i]` is `pages[i]`'s).
    pub(in crate::session) fn add_hoop_lists(&self, pages: &[Page], grids: &mut Vec<DrawList>) {
        let px = 1.0 / (self.view().scale() * self.device_pixel_ratio.max(f64::MIN_POSITIVE));
        for (index, page) in pages.iter().enumerate() {
            let Some(manufacturing) = page
                .kind
                .manufacturing()
                .and_then(|id| self.document.manufacturing(id))
                .filter(|m| self.process_enabled(&m.process))
            else {
                continue;
            };
            let Some(embroidery) = embroidery_page(&manufacturing) else {
                continue;
            };
            if grids.len() < pages.len() {
                grids.resize_with(pages.len(), DrawList::default);
            }
            let mut list = hoop_list(embroidery.hoop.size, page, px);
            list.triangles.append(&mut grids[index].triangles);
            grids[index] = list;
        }
    }

    /// The Toolpath view of the active embroidery page (criteria 12, 13): the stitches up to the
    /// play position in their thread colours over the casing, jumps dashed, trims as an ink mark
    /// on a white disc and colour changes as a disc of the new thread; nothing after the head
    /// (UX E9). `None` while it is not pressed.
    pub(in crate::session) fn embroidery_toolpath(&self, view: ViewTransform) -> Option<DrawList> {
        let page = self.active_embroidery()?;
        if !self.toolpath_view_pressed(page.id) {
            return None;
        }
        let plan = self.embroidery_job(&page).plan;
        let centre = page.centre();
        let head = self
            .manufacturing
            .embroidery
            .play
            .unwrap_or(plan.stitches.len())
            .min(plan.stitches.len());
        let mut threads = vec![None; plan.stitches.len()];
        for block in &plan.blocks {
            for slot in &mut threads[block.stitches.clone()] {
                *slot = Some(rgba(block.thread));
            }
        }
        let mut painter = Painter {
            px: 1.0 / view.scale(),
            from: centre,
            thread: INK,
            casings: DrawList::default(),
            lines: DrawList::default(),
            marks: DrawList::default(),
            travel: ToolpathLook::default(),
        };
        for (index, stitch) in plan.stitches[..head].iter().enumerate() {
            let to = Point::new(centre.x + stitch.at.x.mm(), centre.y - stitch.at.y.mm());
            let next = (stitch.kind == StitchKind::ColourChange)
                .then(|| threads[index..].iter().flatten().next().copied())
                .flatten();
            painter.paint(index, stitch, to, threads[index], next);
        }
        let mut list = build_toolpath(&painter.travel, None, view);
        list.extend(painter.casings);
        list.extend(painter.lines);
        list.extend(painter.marks);
        Some(list)
    }
}

/// Draws a stitch list stitch by stitch: casings under thread lines, marks over both.
struct Painter {
    px: f64,
    from: Point,
    thread: RgbaColor,
    casings: DrawList,
    lines: DrawList,
    marks: DrawList,
    travel: ToolpathLook,
}

impl Painter {
    /// Draws `stitch` ending at `to`, in `thread` when it belongs to a block; `next` is the next
    /// block's thread, for a colour change.
    fn paint(
        &mut self,
        index: usize,
        stitch: &Stitch,
        to: Point,
        thread: Option<RgbaColor>,
        next: Option<RgbaColor>,
    ) {
        let (from, px) = (self.from, self.px);
        match stitch.kind {
            StitchKind::Normal => {
                self.thread = thread.unwrap_or(self.thread);
                let casing = casing(self.thread);
                self.casings.extend(thick_line(from, to, 3.0 * px, casing));
                self.lines.extend(thick_line(from, to, px, self.thread));
            }
            StitchKind::Jump => self.travel.segments.push(ToolpathSegment {
                index,
                from,
                to,
                stroke: ToolpathStroke::Travel,
                line: 0,
                spacing_mm: 0.0,
            }),
            StitchKind::Trim => {
                let r = MARK_PX * px;
                push_disc(&mut self.marks, to, r, WHITE);
                let d = 0.5 * r;
                for (a, b) in [(-d, -d), (-d, d)] {
                    let (p, q) = (Vec2::new(a, b), Vec2::new(-a, -b));
                    self.marks.extend(thick_line(
                        to.translated(p),
                        to.translated(q),
                        1.5 * px,
                        INK,
                    ));
                }
            }
            StitchKind::ColourChange => {
                let next = next.unwrap_or(INK);
                push_disc(&mut self.marks, to, MARK_PX * px, casing(next));
                push_disc(&mut self.marks, to, (MARK_PX - 1.5) * px, next);
            }
            StitchKind::End => {}
        }
        self.from = to;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_casing_is_whichever_of_white_and_ink_contrasts_more() {
        assert_eq!(casing(RgbaColor::opaque(0xFF, 0xFF, 0xFF)), INK);
        assert_eq!(casing(RgbaColor::opaque(0, 0, 0)), WHITE);
        assert_eq!(casing(RgbaColor::opaque(0x80, 0x80, 0x80)), WHITE);
    }
}
