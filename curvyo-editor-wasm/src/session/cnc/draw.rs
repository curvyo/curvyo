//! What a CNC page draws (`specs/0161-cnc-output` criteria 3, 11, `ux-notes.md` 3, 7): the stock
//! from the page origin under the page's items, and the Toolpath view of the job's moves with
//! the tool band, plunge dots and the head at the play position.

use curvyo_cnc_core::{MoveKind, Stock, WorkPoint, work_to_page};
use curvyo_document_core::{Page, Point, Vec2, ViewTransform};
use curvyo_render_core::{
    DrawList, RgbaColor, ToolpathHead, ToolpathLook, ToolpathSegment, ToolpathStroke, Vertex,
    build_toolpath, thick_line,
};

use super::super::Session;
use super::cnc_page;
use super::job::CncJobView;

/// `--toolbar-icon`, the ink.
const INK: RgbaColor = RgbaColor::opaque(0x3A, 0x3A, 0x3F);
/// `--toolbar-bg`, the stock's ground (the holder board, UX N1).
const STOCK: RgbaColor = RgbaColor::opaque(0xDC, 0xDC, 0xE0);
/// `--tool-band`: `--cut` at 20 % over the canvas (UX 2).
const TOOL_BAND: RgbaColor = RgbaColor::opaque(0xE2, 0xBD, 0xC5);
/// The plunge dot's radius in px (UX 2).
const DOT_PX: f64 = 2.5;

fn push_rect(list: &mut DrawList, p: [Point; 4], color: RgbaColor) {
    for i in [0, 1, 2, 0, 2, 3] {
        list.triangles.push(Vertex {
            position: p[i],
            color,
        });
    }
}

fn push_disc(list: &mut DrawList, centre: Point, radius: f64, color: RgbaColor) {
    const STEPS: u32 = 12;
    let a = |k: u32| f64::from(k) * std::f64::consts::TAU / f64::from(STEPS);
    let p = |a: f64| centre.translated(Vec2::new(radius * a.cos(), radius * a.sin()));
    for k in 0..STEPS {
        for position in [centre, p(a(k)), p(a(k + 1))] {
            list.triangles.push(Vertex { position, color });
        }
    }
}

/// The stock board from `page`'s origin, first-wins order: outline, then ground.
fn stock_list(stock: &Stock, page: &Page, px: f64) -> DrawList {
    let o = page.origin;
    let (w, h) = (stock.width.as_mm(), stock.height.as_mm());
    let c = [
        o,
        Point::new(o.x + w, o.y),
        Point::new(o.x + w, o.y + h),
        Point::new(o.x, o.y + h),
    ];
    let mut list = DrawList::default();
    for i in 0..4 {
        list.extend(thick_line(c[i], c[(i + 1) % 4], 1.5 * px, INK));
    }
    push_rect(&mut list, c, STOCK);
    list
}

/// The head at `t` seconds of `job`: the first move not done and where along it the head is.
pub(in crate::session) fn head_at(job: &CncJobView, t: f64) -> Option<(usize, WorkPoint)> {
    let moves = &job.job.moves;
    let index = job.move_end.iter().position(|&end| end > t)?;
    let start = if index == 0 {
        0.0
    } else {
        job.move_end[index - 1]
    };
    let end = job.move_end[index];
    let from = if index == 0 {
        WorkPoint::mm(0.0, 0.0, 0.0)
    } else {
        moves[index - 1].to
    };
    let to = moves[index].to;
    let f = if end > start {
        ((t - start) / (end - start)).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let lerp = |a: f64, b: f64| a + (b - a) * f;
    Some((
        index,
        WorkPoint::mm(
            lerp(from.x(), to.x()),
            lerp(from.y(), to.y()),
            lerp(from.z(), to.z()),
        ),
    ))
}

impl Session {
    /// Adds the stock of every CNC page under that page's grid (`grids[i]` is `pages[i]`'s).
    pub(in crate::session) fn add_stock_lists(&self, pages: &[Page], grids: &mut Vec<DrawList>) {
        let px = 1.0 / (self.view().scale() * self.device_pixel_ratio.max(f64::MIN_POSITIVE));
        for (index, page) in pages.iter().enumerate() {
            let Some(cnc) = page
                .kind
                .manufacturing()
                .and_then(|id| self.document.manufacturing(id))
                .filter(|m| self.process_enabled(&m.process))
                .and_then(|m| cnc_page(&m))
            else {
                continue;
            };
            if grids.len() < pages.len() {
                grids.resize_with(pages.len(), DrawList::default);
            }
            let mut list = stock_list(&cnc.stock, page, px);
            list.triangles.append(&mut grids[index].triangles);
            grids[index] = list;
        }
    }

    /// The Toolpath view of the active CNC page (criterion 11): the tool centre path in the cut
    /// look over the tool band (first pass), rapids dashed, a plunge dot at each pass start, a
    /// ring at each operation's first cut, the head at the play position with everything after it
    /// pale. `None` while it is not pressed.
    pub(in crate::session) fn cnc_toolpath(&self, view: ViewTransform) -> Option<DrawList> {
        let page = self.active_cnc()?;
        if !self.toolpath_view_pressed(page.id) {
            return None;
        }
        let job = self.cnc_job(&page);
        let height = page.stock.height.as_mm();
        let origin = page.page.origin;
        let at = |w: WorkPoint| {
            let p = work_to_page(w, height);
            Point::new(origin.x + p.x, origin.y + p.y)
        };
        let px = 1.0 / view.scale();
        let diameter = job
            .list_index
            .first()
            .and_then(|&l| page.operations.get(l))
            .map_or(page.tools[0].diameter.get(), |op| {
                page.tool(&page.settings_of(op).0.tool).diameter.get()
            });
        let mut band = DrawList::default();
        let mut dots = DrawList::default();
        let mut look = ToolpathLook::default();
        let mut from = at(WorkPoint::mm(0.0, 0.0, 0.0));
        let mut ring_op = None;
        for (index, m) in job.job.moves.iter().enumerate() {
            let to = at(m.to);
            let stroke = match m.kind {
                MoveKind::Rapid => ToolpathStroke::Travel,
                MoveKind::Plunge => {
                    push_disc(&mut dots, to, DOT_PX * px, INK);
                    if ring_op != Some(m.operation) {
                        look.rings.push(to);
                        ring_op = Some(m.operation);
                    }
                    ToolpathStroke::Cut
                }
                MoveKind::Cut => {
                    if m.pass == 1 {
                        band.extend(thick_line(from, to, diameter, TOOL_BAND));
                    }
                    ToolpathStroke::Cut
                }
            };
            if from.vector_to(to).length() > 1e-9 {
                look.segments.push(ToolpathSegment {
                    index,
                    from,
                    to,
                    stroke,
                    line: 0,
                    spacing_mm: 0.0,
                });
            }
            from = to;
        }
        let head = self
            .manufacturing
            .cnc
            .play
            .and_then(|t| head_at(&job, t))
            .map(|(index, w)| ToolpathHead { index, at: at(w) });
        let mut list = band;
        list.extend(build_toolpath(&look, head, view));
        list.extend(dots);
        if let Some(head) = head {
            let r = diameter / 2.0;
            if r / px >= 8.0 {
                const STEPS: u32 = 32;
                for k in 0..STEPS {
                    let a = |k: u32| f64::from(k) * std::f64::consts::TAU / f64::from(STEPS);
                    let p = |a: f64| head.at.translated(Vec2::new(r * a.cos(), r * a.sin()));
                    list.extend(thick_line(p(a(k)), p(a(k + 1)), px, INK));
                }
            }
        }
        Some(list)
    }
}

impl Session {
    /// The stock's name label "Plywood · 10 mm" in its origin corner, the `0102` board label,
    /// while the stock is at least 80 device px wide (UX unify 2.2, U15).
    pub(in crate::session) fn cnc_stock_label(
        &self,
    ) -> Option<super::super::fixtures::FixtureLabel> {
        let page = self.active_cnc()?;
        let view = self.view();
        let device = view.scale() * self.device_pixel_ratio.max(f64::MIN_POSITIVE);
        let stock = &page.stock;
        if stock.width.as_mm() * device < 80.0 {
            return None;
        }
        let (x, y) = view.document_to_screen(page.page.origin);
        Some(super::super::fixtures::FixtureLabel {
            kind: "fixture",
            text: format!(
                "{} · {} mm",
                stock.material,
                curvyo_cnc_core::number(stock.thickness.as_mm())
            ),
            x,
            y,
            edge: None,
        })
    }
}
