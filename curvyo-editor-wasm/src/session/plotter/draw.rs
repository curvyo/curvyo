//! What a plotter page draws (`specs/0159-cutting-plotter` criteria 3 to 5, 10, `ux-notes.md`
//! 5.2, 5.3, 8): the mat under the page's items as one first-wins layer, its printed numbers as
//! canvas labels, and the job's moves as the Toolpath view.

use curvyo_cutter_core::{
    Grip, MatRead, MoveKind, OpKind, PrintText, Rect, SHEET_RADIUS_MM, TEXT_MM, mat_print,
};
use curvyo_document_core::{Page, Point, Vec2, ViewTransform};
use curvyo_render_core::{
    DrawList, RgbaColor, ToolpathArrow, ToolpathLook, ToolpathSegment, ToolpathStroke, Vertex,
    build_toolpath, thick_line,
};
use serde::Serialize;

use super::super::Session;
use super::{PlotterPage, plotter_page};

/// `--mat-print`, white.
const PRINT: RgbaColor = RgbaColor::opaque(0xFF, 0xFF, 0xFF);
/// The ink of the cut-area edge and the sheet outline (`--toolbar-icon`).
const INK: RgbaColor = RgbaColor::opaque(0x3A, 0x3A, 0x3F);
/// The veil over the band outside the cut area: white at 45 % over the grip colour.
const VEIL_PERCENT: u32 = 45;
/// Numbers and the grip word are not drawn below this digit height in device px (UX 5.2).
const MIN_TEXT_PX: f64 = 6.0;
/// Grid lines go below this spacing in device px (UX 5.2).
const MIN_GRID_PX: f64 = 4.0;

/// A printed text of the mat over the canvas, placed by the host in CSS px.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MatLabel {
    /// The text.
    pub text: String,
    /// Its centre, screen x.
    pub x: f64,
    /// Its centre, screen y.
    pub y: f64,
    /// Its height in CSS px.
    pub size: f64,
    /// Turned 180 degrees.
    pub turned: bool,
}

/// The grip's colour.
fn grip_colour(grip: Grip) -> RgbaColor {
    let c = grip.colour();
    let byte = |shift: u32| u8::try_from((c >> shift) & 0xFF).unwrap_or(u8::MAX);
    RgbaColor::opaque(byte(16), byte(8), byte(0))
}

/// `colour` with white laid over it at `percent`.
fn veiled(colour: RgbaColor, percent: u32) -> RgbaColor {
    let mix = |c: u8| {
        let v = (u32::from(c) * (100 - percent) + 255 * percent + 50) / 100;
        u8::try_from(v).unwrap_or(u8::MAX)
    };
    RgbaColor::opaque(mix(colour.r), mix(colour.g), mix(colour.b))
}

fn push_triangle(list: &mut DrawList, a: Point, b: Point, c: Point, color: RgbaColor) {
    for position in [a, b, c] {
        list.triangles.push(Vertex { position, color });
    }
}

fn push_rect(list: &mut DrawList, rect: Rect, color: RgbaColor) {
    let top_left = Point::new(rect.x, rect.y);
    let top_right = Point::new(rect.x + rect.w, rect.y);
    let bottom_right = Point::new(rect.x + rect.w, rect.y + rect.h);
    let bottom_left = Point::new(rect.x, rect.y + rect.h);
    push_triangle(list, top_left, top_right, bottom_right, color);
    push_triangle(list, top_left, bottom_right, bottom_left, color);
}

/// The triangles of each corner fan of the sheet.
const STEPS: u32 = 6;

/// The sheet as a rounded rectangle: the core and four corner fans.
fn push_rounded(list: &mut DrawList, r: Rect, radius: f64, color: RgbaColor) {
    push_rect(
        list,
        Rect::new(r.x + radius, r.y, r.w - 2.0 * radius, r.h),
        color,
    );
    push_rect(
        list,
        Rect::new(r.x, r.y + radius, radius, r.h - 2.0 * radius),
        color,
    );
    push_rect(
        list,
        Rect::new(r.x + r.w - radius, r.y + radius, radius, r.h - 2.0 * radius),
        color,
    );
    let corners = [
        (Point::new(r.x + radius, r.y + radius), std::f64::consts::PI),
        (
            Point::new(r.x + r.w - radius, r.y + radius),
            1.5 * std::f64::consts::PI,
        ),
        (Point::new(r.x + r.w - radius, r.y + r.h - radius), 0.0),
        (
            Point::new(r.x + radius, r.y + r.h - radius),
            0.5 * std::f64::consts::PI,
        ),
    ];
    for (centre, start) in corners {
        for k in 0..STEPS {
            let angle =
                |k: u32| start + f64::from(k) * std::f64::consts::FRAC_PI_2 / f64::from(STEPS);
            let p = |a: f64| centre.translated(Vec2::new(radius * a.cos(), radius * a.sin()));
            push_triangle(list, centre, p(angle(k)), p(angle(k + 1)), color);
        }
    }
}

/// The outline of `r` as four `width` lines.
fn push_outline(list: &mut DrawList, r: Rect, width: f64, color: RgbaColor) {
    let c = [
        Point::new(r.x, r.y),
        Point::new(r.x + r.w, r.y),
        Point::new(r.x + r.w, r.y + r.h),
        Point::new(r.x, r.y + r.h),
    ];
    for i in 0..4 {
        list.extend(thick_line(c[i], c[(i + 1) % 4], width, color));
    }
}

/// The mat of `mat` in document space at `origin`, first-wins order: notch hole, printing,
/// cut-area edge, veil band, sheet (UX 5.3). `px` is one device pixel in mm.
fn mat_list(mat: MatRead, origin: Point, px: f64, pasteboard: RgbaColor) -> DrawList {
    let mut list = DrawList::default();
    let size = mat.size;
    let notch = size.notch();
    push_triangle(&mut list, notch[0], notch[1], notch[2], pasteboard);
    if mat.grid && 25.4 / px >= MIN_GRID_PX {
        for line in mat_print(size, mat.grip).lines {
            list.extend(thick_line(line.a, line.b, line.width_mm.max(px), PRINT));
        }
    }
    let cut = size.cut_area();
    push_outline(&mut list, cut, px, INK);
    if !mat.grid {
        push_outline(&mut list, size.field(), px, INK);
    }
    let colour = grip_colour(mat.grip);
    let field = size.field();
    let veil = veiled(colour, VEIL_PERCENT);
    push_rect(
        &mut list,
        Rect::new(field.x, field.y, field.w, cut.y - field.y),
        veil,
    );
    let bottom = cut.y + cut.h;
    push_rect(
        &mut list,
        Rect::new(field.x, bottom, field.w, field.h - bottom),
        veil,
    );
    push_rect(
        &mut list,
        Rect::new(field.x, cut.y, cut.x - field.x, cut.h),
        veil,
    );
    let right = cut.x + cut.w;
    push_rect(
        &mut list,
        Rect::new(right, cut.y, field.w - right, cut.h),
        veil,
    );
    push_outline(&mut list, size.sheet(), px, INK);
    push_rounded(&mut list, size.sheet(), SHEET_RADIUS_MM, colour);
    for vertex in &mut list.triangles {
        vertex.position = vertex.position.translated(Vec2::new(origin.x, origin.y));
    }
    list
}

impl Session {
    /// Adds the mat of every plotter page under that page's grid (`grids[i]` is `pages[i]`'s).
    pub(in crate::session) fn add_mat_lists(&self, pages: &[Page], grids: &mut Vec<DrawList>) {
        let px = 1.0 / (self.view().scale() * self.device_pixel_ratio.max(f64::MIN_POSITIVE));
        for (index, page) in pages.iter().enumerate() {
            let Some(id) = page.kind.manufacturing() else {
                continue;
            };
            let Some(manufacturing) = self.document.manufacturing(id) else {
                continue;
            };
            if !self.process_enabled(&manufacturing.process) {
                continue;
            }
            let Some(plotter) = plotter_page(&manufacturing) else {
                continue;
            };
            let Some(mat) = plotter.active_mat() else {
                continue;
            };
            if grids.len() < pages.len() {
                grids.resize_with(pages.len(), DrawList::default);
            }
            let mut list = mat_list(mat, page.origin, px, curvyo_render_core::PASTEBOARD_BG);
            list.triangles.append(&mut grids[index].triangles);
            grids[index] = list;
        }
    }

    /// The printed numbers and grip words of the active plotter page's mat, in CSS px; empty
    /// with Mat grid off or below the smallest readable size (UX 5.2).
    #[must_use]
    pub fn plotter_mat_labels(&self) -> Vec<MatLabel> {
        let Some(page) = self.active_plotter() else {
            return Vec::new();
        };
        let Some(mat) = page.active_mat().filter(|m| m.grid) else {
            return Vec::new();
        };
        let view = self.view();
        let size = TEXT_MM * view.scale();
        if size * self.device_pixel_ratio.max(f64::MIN_POSITIVE) < MIN_TEXT_PX {
            return Vec::new();
        }
        let origin = page.page.origin;
        mat_print(mat.size, mat.grip)
            .texts
            .into_iter()
            .map(|PrintText { at, text, turned }| {
                let (x, y) = view.document_to_screen(Point::new(origin.x + at.x, origin.y + at.y));
                MatLabel {
                    text,
                    x,
                    y,
                    size,
                    turned,
                }
            })
            .collect()
    }

    /// The Toolpath view of the active plotter page (criterion 10): Cut moves red, Draw moves
    /// ink, travel dashed, an arrowhead at each path start; `None` while it is not pressed.
    pub(in crate::session) fn plotter_toolpath(&self, view: ViewTransform) -> Option<DrawList> {
        let page = self.active_plotter()?;
        if !self.toolpath_view_pressed(page.id) {
            return None;
        }
        let look = self.plotter_look(&page);
        Some(build_toolpath(&look, None, view))
    }

    pub(in crate::session) fn plotter_look(&self, page: &PlotterPage) -> ToolpathLook {
        let built = self.plotter_job(page);
        let origin = page.page.origin;
        let at = |p: curvyo_geometry_core::PointUm| {
            let mm = p.to_mm();
            Point::new(origin.x + mm.x, origin.y + mm.y)
        };
        let mut look = ToolpathLook::default();
        let mut last_op = None;
        let mut path_open = false;
        for (index, m) in built.job.moves().into_iter().enumerate() {
            let stroke = match m.kind {
                MoveKind::Travel => {
                    path_open = false;
                    ToolpathStroke::Travel
                }
                MoveKind::Work { op } => match built.job.operations[op].kind {
                    OpKind::Cut => ToolpathStroke::Cut,
                    OpKind::Draw => ToolpathStroke::Score,
                },
            };
            let (from, to) = (at(m.from), at(m.to));
            if let MoveKind::Work { op } = m.kind {
                if last_op != Some(op) {
                    look.rings.push(from);
                    last_op = Some(op);
                    path_open = false;
                }
                if !path_open {
                    let d = from.vector_to(to);
                    if d.length() > f64::EPSILON {
                        look.arrows.push(ToolpathArrow {
                            index,
                            at: from,
                            direction: d.normalized_to(1.0),
                            length_mm: d.length(),
                            stroke,
                        });
                    }
                    path_open = true;
                }
            }
            look.segments.push(ToolpathSegment {
                index,
                from,
                to,
                stroke,
                line: 0,
                spacing_mm: 0.0,
            });
        }
        look
    }
}

#[cfg(test)]
mod tests {
    use curvyo_cutter_core::MatSize;

    use super::*;

    #[test]
    fn the_veil_lightens_the_grip_colour() {
        let standard = grip_colour(Grip::Standard);
        assert_eq!((standard.r, standard.g, standard.b), (0x9C, 0xB8, 0x7D));
        let v = veiled(standard, VEIL_PERCENT);
        assert!(v.r > standard.r && v.g > standard.g && v.b > standard.b);
    }

    #[test]
    fn grid_off_draws_fewer_triangles_than_grid_on() {
        let on = MatRead {
            size: MatSize::In12x12,
            grip: Grip::Standard,
            grid: true,
            damaged: 0,
        };
        let off = MatRead { grid: false, ..on };
        let pb = curvyo_render_core::PASTEBOARD_BG;
        let a = mat_list(on, Point::new(0.0, 0.0), 0.1, pb).triangles.len();
        let b = mat_list(off, Point::new(0.0, 0.0), 0.1, pb).triangles.len();
        assert!(a > b, "{a} {b}");
    }
}
