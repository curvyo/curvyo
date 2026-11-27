//! How the tabs and path options look on the canvas (`specs/0139` criteria 7, 14, 15, 19,
//! `ux-notes.md` 5.1, 6.1): the tab handles of the selected items and the gap, lead, overcut and
//! start marks of the Operations view.

use curvyo_document_core::Point;
use curvyo_laser_core::job::PointUm;
use curvyo_render_core::{
    DrawList, OP_CUT, OP_INK, RgbaColor, TransformDecorationInput, TransformGlyphKind,
    TransformHandleGlyph, build_transform_draw_list, thick_line,
};

use super::super::Session;
use super::tab_outlines::TabOutline;
use super::tabs::Current;

/// A path shorter than this on screen shows no handle (C5).
const MIN_PATH_PX: f64 = 48.0;

impl Session {
    /// The handles drawn now: none in the Toolpath view and on paths under 48 px on screen.
    fn tab_handles(&self) -> Vec<(TabOutline, usize, Point)> {
        if self.toolpath_view_on() || self.select.drag_in_flight() {
            return Vec::new();
        }
        let scale = self.view().scale();
        let mut out = Vec::new();
        for outline in self.tab_outlines(None) {
            if outline.measured.length() / 1000.0 * scale < MIN_PATH_PX {
                continue;
            }
            for (index, span) in outline.spans.iter().enumerate() {
                let at = outline.to_document(span.centre);
                out.push((outline.clone(), index, at));
            }
        }
        out
    }

    pub(super) fn handle_at(&self, point: Point) -> Option<(TabOutline, usize)> {
        let radius = self.transform_handle_tolerances().param_hit.as_mm();
        self.tab_handles()
            .into_iter()
            .map(|(outline, index, at)| (outline, index, at.vector_to(point).length()))
            .filter(|(_, _, d)| *d <= radius)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(outline, index, _)| (outline, index))
    }

    /// The tab handles over the drawing: the parameter handle look, solid while current or held.
    pub(in crate::session) fn tab_handles_draw_list(&self) -> DrawList {
        let handles = self.tab_handles();
        if handles.is_empty() && self.operations.tabs.drag.is_none() {
            return DrawList::default();
        }
        let current = self.operations.tabs.current.as_ref();
        let hovered = self
            .pointer_position
            .filter(|_| self.operations.tabs.drag.is_none())
            .and_then(|point| self.handle_at(point));
        let mut glyphs: Vec<TransformHandleGlyph> = handles
            .iter()
            .map(|(outline, index, at)| {
                let is = |c: &Current| {
                    c.op == outline.op
                        && c.key == outline.key
                        && c.reference == outline.reference
                        && c.index == *index
                };
                TransformHandleGlyph {
                    position: *at,
                    kind: TransformGlyphKind::Parameter,
                    dragging: current.is_some_and(is),
                    hovered: hovered
                        .as_ref()
                        .is_some_and(|(o, i)| o.reference == outline.reference && i == index),
                }
            })
            .collect();
        if let Some(drag) = &self.operations.tabs.drag {
            glyphs.push(TransformHandleGlyph {
                position: drag.outline.to_document(drag.centre),
                kind: TransformGlyphKind::Parameter,
                dragging: true,
                hovered: false,
            });
        }
        build_transform_draw_list(
            self.view(),
            &TransformDecorationInput {
                handles: glyphs,
                device_pixel_ratio: self.device_pixel_ratio,
                ..TransformDecorationInput::default()
            },
        )
    }
}

/// A lead's line, 1 px, dashed 3 on 2 (UX 6.1).
const LEAD_PX: f64 = 1.0;
const DASH_PX: f64 = 3.0;
const GAP_PX: f64 = 2.0;
/// The chevron of a set start or fixed direction: 5 px arms (UX 6.1).
const CHEVRON_PX: f64 = 5.0;

/// `points` as a line of `width`, solid or in dashes of `(dash, gap)` along its length.
fn dashed(
    list: &mut DrawList,
    points: &[Point],
    width: f64,
    color: RgbaColor,
    dash: Option<(f64, f64)>,
) {
    let Some((on, off)) = dash else {
        for pair in points.windows(2) {
            list.extend(thick_line(pair[0], pair[1], width, color));
        }
        return;
    };
    let period = on + off;
    let mut walked = 0.0_f64;
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let length = a.vector_to(b).length();
        if length <= 0.0 {
            continue;
        }
        let at = |t: f64| {
            Point::new(
                a.x + (b.x - a.x) * t / length,
                a.y + (b.y - a.y) * t / length,
            )
        };
        let mut t = 0.0;
        while t < length {
            let phase = (walked + t) % period;
            if phase < on {
                let end = (t + on - phase).min(length);
                list.extend(thick_line(at(t), at(end), width, color));
                t = end;
            } else {
                t += period - phase;
            }
        }
        walked += length;
    }
}

/// An open ink chevron at `at` pointing along `direction`, with its white casing (UX 6.1).
fn chevron(list: &mut DrawList, at: Point, direction: (f64, f64), scale: f64) {
    let (dx, dy) = direction;
    let arm = CHEVRON_PX / scale;
    let back = |side: f64| {
        Point::new(
            at.x - (dx + side * -dy) * arm * std::f64::consts::FRAC_1_SQRT_2,
            at.y - (dy + side * dx) * arm * std::f64::consts::FRAC_1_SQRT_2,
        )
    };
    for (width, color) in [
        (CASING_PX / scale, RgbaColor::WHITE),
        (BAR_WIDTH_PX / scale, OP_INK),
    ] {
        list.extend(thick_line(back(1.0), at, width, color));
        list.extend(thick_line(back(-1.0), at, width, color));
    }
}

/// The bar across a gap in the Operations view: 8 px long (UX 6.1).
const BAR_PX: f64 = 8.0;
/// Its width, 1.5 px, and the white casing that breaks the red cut line, 3 px.
const BAR_WIDTH_PX: f64 = 1.5;
const CASING_PX: f64 = 3.0;

impl Session {
    /// The tab gaps of the page's job in the Operations view (criterion 7): the cut line broken
    /// at each gap by a white casing and an ink bar across it, from the job's marks (adrs L12).
    pub(in crate::session) fn tab_marks_overlay(
        &self,
        view: curvyo_document_core::ViewTransform,
    ) -> DrawList {
        let mut list = DrawList::default();
        let Some((job, _)) = self.settled_job() else {
            return list;
        };
        let Some(origin) = self
            .active_manufacturing()
            .and_then(|m| m.page.map(|page| page.origin))
        else {
            return list;
        };
        let scale = view.scale();
        if scale <= 0.0 {
            return list;
        }
        let doc = |p: PointUm| {
            let page = p.to_mm();
            Point::new(page.x + origin.x, page.y + origin.y)
        };
        for plan in job.built.plans.iter().flatten() {
            for mark in &plan.marks {
                let points = |list: &[PointUm]| list.iter().map(|&p| doc(p)).collect::<Vec<_>>();
                for lead in [&mark.lead_in, &mark.lead_out] {
                    let lead = points(lead);
                    dashed(&mut list, &lead, CASING_PX / scale, RgbaColor::WHITE, None);
                    let dash = Some((DASH_PX / scale, GAP_PX / scale));
                    dashed(&mut list, &lead, LEAD_PX / scale, OP_CUT, dash);
                }
                let overcut = points(&mark.overcut);
                let dash = Some((DASH_PX / scale, GAP_PX / scale));
                dashed(&mut list, &overcut, BAR_WIDTH_PX / scale, OP_INK, dash);
                if let Some((at, (dx, dy))) = mark.start {
                    chevron(
                        &mut list,
                        doc(at),
                        (f64::from(dx) / 1e6, f64::from(dy) / 1e6),
                        scale,
                    );
                }
            }
            for gap in plan.marks.iter().flat_map(|mark| &mark.gaps) {
                for pair in gap.points.windows(2) {
                    list.extend(thick_line(
                        doc(pair[0]),
                        doc(pair[1]),
                        CASING_PX / scale,
                        RgbaColor::WHITE,
                    ));
                }
                let (dx, dy) = (
                    f64::from(gap.direction.0) / 1e6,
                    f64::from(gap.direction.1) / 1e6,
                );
                let half = BAR_PX / 2.0 / scale;
                let centre = doc(gap.centre);
                let a = Point::new(centre.x - dy * half, centre.y + dx * half);
                let b = Point::new(centre.x + dy * half, centre.y - dx * half);
                list.extend(thick_line(a, b, BAR_WIDTH_PX / scale, OP_INK));
            }
        }
        list
    }
}
