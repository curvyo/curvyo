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

/// The frame's outer and inner corner radii and its mounting tab (depth, length) in mm (UX E2).
const OUTER_RADIUS_MM: f64 = 14.0;
const INNER_RADIUS_MM: f64 = 10.0;
const TAB_MM: (f64, f64) = (16.0, 40.0);
/// The points of each quarter corner of the frame.
const CORNER_STEPS: u32 = 8;

/// The outline of `rect` with corners of `radius`, clockwise from the top-left corner's left end,
/// [`CORNER_STEPS`] + 1 points per corner, so two such loops pair point for point.
fn rounded_loop((left, top, width, height): Rect, radius: f64) -> Vec<Point> {
    use std::f64::consts::{FRAC_PI_2, PI};
    let centres = [
        (Point::new(left + radius, top + radius), PI),
        (Point::new(left + width - radius, top + radius), 1.5 * PI),
        (
            Point::new(left + width - radius, top + height - radius),
            0.0,
        ),
        (Point::new(left + radius, top + height - radius), FRAC_PI_2),
    ];
    let mut out = Vec::new();
    for (centre, start) in centres {
        for k in 0..=CORNER_STEPS {
            let a = start + f64::from(k) * FRAC_PI_2 / f64::from(CORNER_STEPS);
            out.push(centre.translated(Vec2::new(radius * a.cos(), radius * a.sin())));
        }
    }
    out
}

fn push_loop(list: &mut DrawList, points: &[Point], width: f64) {
    for (i, &p) in points.iter().enumerate() {
        list.extend(thick_line(p, points[(i + 1) % points.len()], width, INK));
    }
}

/// The hoop `hoop` centred on `page`, first-wins order: template marks, field edge, frame
/// outlines, the rounded frame band and the mounting tab on the left (UX 3, E2). `px` is one
/// device pixel in mm.
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
    let inner_loop = rounded_loop(inner, INNER_RADIUS_MM);
    let outer_loop = rounded_loop(outer, OUTER_RADIUS_MM);
    let (depth, length) = TAB_MM;
    let tab = (outer.0 - depth, c.y - length / 2.0, depth, length);
    push_loop(&mut list, &inner_loop, 1.5 * px);
    push_loop(&mut list, &outer_loop, 1.5 * px);
    let tab_corners = corners(tab);
    let [top_right, top_left, bottom_left, bottom_right] = [1, 0, 3, 2].map(|i| tab_corners[i]);
    for (from, to) in [
        (top_right, top_left),
        (top_left, bottom_left),
        (bottom_left, bottom_right),
    ] {
        list.extend(thick_line(from, to, 1.5 * px, INK));
    }
    // The band as a strip between the two loops, which pair point for point.
    let count = outer_loop.len();
    for i in 0..count {
        let j = (i + 1) % count;
        for position in [
            outer_loop[i],
            outer_loop[j],
            inner_loop[j],
            outer_loop[i],
            inner_loop[j],
            inner_loop[i],
        ] {
            list.triangles.push(Vertex {
                position,
                color: FRAME,
            });
        }
    }
    push_rect(&mut list, tab, FRAME);
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
    fn the_hoop_frame_has_rounded_corners_and_a_mounting_tab_on_the_left() {
        let mut s = Session::new(98_401);
        s.add_page_of_kind(curvyo_embroidery_core::PROCESS_ID, None)
            .unwrap();
        let page = s.active_page().clone();
        let (w, h) = HoopSize::Field150x250.field_mm();
        let (w, h) = (f64::from(w), f64::from(h));
        let o = Point::new(
            page.origin.x + page.size.width.as_mm() / 2.0 - w / 2.0,
            page.origin.y + page.size.height.as_mm() / 2.0 - h / 2.0,
        );
        let list = hoop_list(HoopSize::Field150x250, &page, 0.1);
        let at = |x: f64, y: f64| {
            list.triangles.iter().any(|v| {
                (v.position.x - o.x - x).abs() < 1e-6 && (v.position.y - o.y - y).abs() < 1e-6
            })
        };
        // The outer edge lies 20 mm outside the field: x and y = -20 at the top-left.
        assert!(!at(-20.0, -20.0), "no square outer corner");
        assert!(
            at(-20.0, -20.0 + 14.0),
            "the 14 mm outer corner starts there"
        );
        assert!(at(-8.0, -8.0 + 10.0), "the 10 mm inner corner");
        let mid = h / 2.0;
        assert!(
            at(-20.0 - 16.0, mid - 20.0),
            "the 40 x 16 mm tab, centred on the left"
        );
        assert!(at(-20.0 - 16.0, mid + 20.0));
    }

    #[test]
    fn the_casing_is_whichever_of_white_and_ink_contrasts_more() {
        assert_eq!(casing(RgbaColor::opaque(0xFF, 0xFF, 0xFF)), INK);
        assert_eq!(casing(RgbaColor::opaque(0, 0, 0)), WHITE);
        assert_eq!(casing(RgbaColor::opaque(0x80, 0x80, 0x80)), WHITE);
    }
}
