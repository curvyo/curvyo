//! What Print and cut draws over a laser page (`specs/0121-print-and-cut` criterion 5, UX notes
//! §5): the marks of every placed sheet as faint crosses, the measured positions of the shown
//! sheet as rings, and the fitted frame as a dashed outline while the placement differs.
//! View only: never artwork, never hit, never in the job.

use curvyo_document_core::{Point, mark_lines};
use curvyo_laser_core::print_and_cut::{FitState, machine_mm};
use curvyo_render_core::{DrawList, GridStroke, RgbaColor};
use serde::Serialize;

use super::Session;

/// `--reg-mark`: the guide green at 60 % (design system "Print and cut").
const REG_MARK: RgbaColor = curvyo_render_core::GUIDE.with_alpha(153);
/// `--accent` of the rings and the fit outline.
const ACCENT: RgbaColor = RgbaColor::opaque(0x2F, 0x6F, 0xEE);
/// The smallest cross on the screen, in px (UX notes §5: at least 12 px).
const MIN_CROSS_PX: f64 = 12.0;
/// The ring's diameter in px.
const RING_PX: f64 = 10.0;
/// The ring's segments.
const RING_SEGMENTS: u32 = 16;
/// The dash and gap of the fit outline in px.
const DASH_PX: (f64, f64) = (6.0, 3.0);

/// One tag over the laser canvas, in screen CSS px (UX notes §5): a mark's number beside its
/// cross, a measured row's number beside its ring, "Fit" at the fitted frame's top-left.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PrintAndCutLabel {
    /// `"mark"`, `"ring"` or `"fit"`.
    pub kind: &'static str,
    /// The text.
    pub text: String,
    /// The anchor in screen CSS px.
    pub x: f64,
    /// The anchor in screen CSS px.
    pub y: f64,
}

fn stroke(start: Point, end: Point) -> GridStroke {
    GridStroke {
        start,
        end,
        emphasis: 1.0,
        alpha: 1.0,
    }
}

impl Session {
    /// The number tags of the crosses and rings and the "Fit" tag of the fitted frame, over the
    /// active laser page; the host draws them as text, never hit.
    #[must_use]
    pub fn print_and_cut_labels(&self) -> Vec<PrintAndCutLabel> {
        let Some(manufacturing) = self.active_manufacturing() else {
            return Vec::new();
        };
        let view = self.view();
        let label = |kind, text: String, p: Point| {
            let (x, y) = view.document_to_screen(p);
            PrintAndCutLabel { kind, text, x, y }
        };
        let chosen = self.print_and_cut_view_sheet();
        let mut out = Vec::new();
        for sheet in self.print_and_cut_sheets(manufacturing.id) {
            let Some(map) = sheet.placed.global_map() else {
                continue;
            };
            for mark in sheet.objects.iter().flat_map(|object| &object.marks) {
                out.push(label(
                    "mark",
                    mark.number.to_string(),
                    map.apply_point(mark.centre),
                ));
            }
            if chosen != Some(sheet.placed.id) {
                continue;
            }
            let (Some(reading), Some((frame, _))) =
                (self.read_sheet(&sheet), self.print_and_cut_frame())
            else {
                continue;
            };
            let origin = sheet.placed.output.origin;
            for row in &reading.rows {
                let Some([x, y]) = row.measured else {
                    continue;
                };
                #[allow(clippy::cast_precision_loss)]
                let local = machine_mm(frame, Point::new(x as f64 / 1000.0, y as f64 / 1000.0));
                let at = Point::new(origin.x + local.x, origin.y + local.y);
                out.push(label("ring", row.number.to_string(), at));
            }
            if let FitState::Fit(fit) = &reading.fit
                && reading.apply != curvyo_laser_core::print_and_cut::Apply::OnFit
                && let Some(corners) = (curvyo_document_core::PlacedPage {
                    matrix: fit.map,
                    ..sheet.placed.clone()
                })
                .frame_corners()
            {
                out.push(label("fit", "Fit".to_owned(), corners[0]));
            }
        }
        out
    }

    /// The Print and cut marks over the active laser page.
    pub(in crate::session) fn print_and_cut_overlay(&self) -> DrawList {
        let mut list = DrawList::default();
        let Some(manufacturing) = self.active_manufacturing() else {
            return list;
        };
        let view = self.view();
        let px = 1.0 / view.scale();
        let mut crosses = Vec::new();
        let mut accents = Vec::new();
        let chosen = self.print_and_cut_view_sheet();
        for sheet in self.print_and_cut_sheets(manufacturing.id) {
            let Some(map) = sheet.placed.global_map() else {
                continue;
            };
            sheet_crosses(&mut crosses, &sheet, map, px);
            if chosen != Some(sheet.placed.id) {
                continue;
            }
            self.sheet_accents(&mut accents, &sheet, px);
        }
        list.extend(curvyo_render_core::build_grid_lines(
            &crosses,
            REG_MARK,
            view,
            self.device_pixel_ratio,
        ));
        list.extend(curvyo_render_core::build_grid_lines(
            &accents,
            ACCENT,
            view,
            self.device_pixel_ratio,
        ));
        list
    }
}

impl Session {
    /// The rings at the measured positions of the shown `sheet` and its fitted frame, dashed,
    /// while the placement differs from the fit.
    fn sheet_accents(
        &self,
        accents: &mut Vec<GridStroke>,
        sheet: &super::print_and_cut::Sheet,
        px: f64,
    ) {
        let (Some(reading), Some((frame, _))) =
            (self.read_sheet(sheet), self.print_and_cut_frame())
        else {
            return;
        };
        let origin = sheet.placed.output.origin;
        for row in &reading.rows {
            let Some([x, y]) = row.measured else {
                continue;
            };
            #[allow(clippy::cast_precision_loss)]
            let local = machine_mm(frame, Point::new(x as f64 / 1000.0, y as f64 / 1000.0));
            ring(
                accents,
                Point::new(origin.x + local.x, origin.y + local.y),
                RING_PX / 2.0 * px,
            );
        }
        if let FitState::Fit(fit) = &reading.fit
            && reading.apply != curvyo_laser_core::print_and_cut::Apply::OnFit
        {
            let fitted = curvyo_document_core::PlacedPage {
                matrix: fit.map,
                ..sheet.placed.clone()
            };
            if let Some(corners) = fitted.frame_corners() {
                for i in 0..4 {
                    dashed(accents, corners[i], corners[(i + 1) % 4], px);
                }
            }
        }
    }
}

/// The faint cross of every mark of `sheet`, placed by `map` (global sheet to global laser mm).
fn sheet_crosses(
    crosses: &mut Vec<GridStroke>,
    sheet: &super::print_and_cut::Sheet,
    map: curvyo_document_core::Affine,
    px: f64,
) {
    for mark in sheet.objects.iter().flat_map(|object| &object.marks) {
        let lines = mark_lines(mark);
        let arm = (lines.radius.as_mm() + 1.0).max(MIN_CROSS_PX / 2.0 * px);
        let c = map.apply_point(mark.centre);
        crosses.push(stroke(
            Point::new(c.x - arm, c.y),
            Point::new(c.x + arm, c.y),
        ));
        crosses.push(stroke(
            Point::new(c.x, c.y - arm),
            Point::new(c.x, c.y + arm),
        ));
    }
}

/// A ring of `radius` mm about `centre` with a centre dot.
fn ring(out: &mut Vec<GridStroke>, centre: Point, radius: f64) {
    let at = |k: u32| {
        let a = f64::from(k) / f64::from(RING_SEGMENTS) * std::f64::consts::TAU;
        Point::new(centre.x + radius * a.cos(), centre.y + radius * a.sin())
    };
    for k in 0..RING_SEGMENTS {
        out.push(stroke(at(k), at(k + 1)));
    }
    let dot = radius / 5.0;
    out.push(stroke(
        Point::new(centre.x - dot, centre.y),
        Point::new(centre.x + dot, centre.y),
    ));
}

/// A dashed line from `a` to `b`, dashes in screen px of `px` mm.
fn dashed(out: &mut Vec<GridStroke>, a: Point, b: Point, px: f64) {
    let length = (b.x - a.x).hypot(b.y - a.y);
    let (dash, gap) = (DASH_PX.0 * px, DASH_PX.1 * px);
    if length <= 0.0 || dash <= 0.0 {
        return;
    }
    let mut t = 0.0;
    while t < length {
        let end = (t + dash).min(length);
        let p = |s: f64| {
            Point::new(
                a.x + (b.x - a.x) * s / length,
                a.y + (b.y - a.y) * s / length,
            )
        };
        out.push(stroke(p(t), p(end)));
        t = end + gap;
    }
}
