//! What the open tiling bar draws (`specs/0069-tiling` criteria 14, 34 and 36, UX notes §5 and
//! §12.4): the preview instances (ghost fill with their variance under the hollow outline;
//! outlines only above 2,000 drawn, boxes above 10,000; only those meeting the viewport), then
//! the circle guide with its four handles.

use curvyo_document_core::{
    AnchorId, NewAnchor, NodeId, ObjectSnapshot, Opacity, PathSnapshot, Point, Style, TilingCell,
    ViewTransform, hue_shifted,
};
use curvyo_render_core::{
    DrawList, KnobState, TilingCircleGuide, build_history_preview, build_live_edit_preview,
    build_tiling_circle,
};
use curvyo_ui_core::tiling::{CircleHandle, CountStatus, count_status};

use super::Session;

/// Above this many drawn instances the ghost fill is dropped (UX notes §5).
const GHOST_FILL_LIMIT: usize = 2_000;

/// A closed path through `corners`, for the preview only.
pub(super) fn box_path(id: NodeId, corners: &[Point]) -> PathSnapshot {
    PathSnapshot {
        id,
        closed: true,
        style: Style::default(),
        anchors: corners
            .iter()
            .enumerate()
            .map(|(i, p)| NewAnchor::corner(AnchorId::new(0, i as u64), *p))
            .collect(),
        extra_subpaths: Vec::new(),
        curve_kind: curvyo_document_core::CurveKind::Bezier,
        spiro_transform: None,
        width_points: Vec::new(),
        rotation: curvyo_document_core::Angle::from_radians(0.0),
    }
}

/// `path` as instance `cell` shows it: its style with the cell's hue shift, its fill and
/// stroke opacity times the opacity the instance gets (criteria 15 and 18).
fn varied(path: &PathSnapshot, leaf_opacity: f64, cell: &TilingCell, vary: bool) -> PathSnapshot {
    let mut mapped = path.mapped(cell.affine);
    if !vary {
        return mapped;
    }
    let style = &mut mapped.style;
    style.fill.color = hue_shifted(style.fill.color, cell.hue);
    style.stroke.color = hue_shifted(style.stroke.color, cell.hue);
    let factor = (leaf_opacity + cell.opacity.get() / 100.0).clamp(0.0, 1.0);
    let times = |o: Opacity| Opacity::new(o.get() * factor).unwrap_or(o);
    style.fill.opacity = times(style.fill.opacity);
    style.stroke.opacity = times(style.stroke.opacity);
    mapped
}

fn bounds_of(corners: &[Point]) -> (Point, Point) {
    corners.iter().fold(
        (
            Point::new(f64::MAX, f64::MAX),
            Point::new(f64::MIN, f64::MIN),
        ),
        |(min, max), p| {
            (
                Point::new(min.x.min(p.x), min.y.min(p.y)),
                Point::new(max.x.max(p.x), max.y.max(p.y)),
            )
        },
    )
}

impl Session {
    /// The document rectangle the canvas shows.
    fn visible_document_box(&self) -> (Point, Point) {
        let (width, height) = self.viewport.canvas_size();
        let a = self.screen_to_document(0.0, 0.0);
        let b = self.screen_to_document(width, height);
        bounds_of(&[a, b])
    }

    /// The preview objects of the open tiling: every cell meeting the viewport, the outlines of
    /// every source leaf mapped by its affine, or its mapped cell box above 10,000 instances.
    pub(super) fn tiling_preview_objects(&self) -> Vec<ObjectSnapshot> {
        let Some(mode) = self.tiling.as_ref() else {
            return Vec::new();
        };
        let simplified = matches!(count_status(&mode.recipe), CountStatus::Simplified(_));
        let vary = mode.recipe.variance.varies_style();
        let (view_min, view_max) = self.visible_document_box();
        let b = mode.cell_box;
        let box_corners = [
            b.min,
            Point::new(b.max.x, b.min.y),
            b.max,
            Point::new(b.min.x, b.max.y),
        ];
        let mut objects = Vec::new();
        for cell in &mode.cells {
            let mapped = box_corners.map(|p| cell.affine.apply_point(p));
            let (min, max) = bounds_of(&mapped);
            if max.x < view_min.x || min.x > view_max.x || max.y < view_min.y || min.y > view_max.y
            {
                continue;
            }
            if simplified {
                objects.push(ObjectSnapshot::Path(box_path(mode.sources[0], &mapped)));
                continue;
            }
            objects.extend(mode.outlines.iter().map(|(outline, opacity)| {
                ObjectSnapshot::Path(varied(outline, *opacity, cell, vary))
            }));
        }
        objects
    }

    /// The canvas layer of the open tiling bar (UX notes §5, draw order): the instances' ghost
    /// fills, their outlines, then the dashed circle path and its handles.
    pub(super) fn tiling_draw_list(&self, view: ViewTransform) -> DrawList {
        if self.tiling.is_none() {
            return DrawList::default();
        }
        let objects = self.tiling_preview_objects();
        let mut list = if objects.len() > GHOST_FILL_LIMIT {
            build_live_edit_preview(&objects, view)
        } else {
            build_history_preview(&objects, view)
        };
        if let Some(guide) = self.tiling_guide() {
            list.extend(build_tiling_circle(&guide, view));
        }
        list
    }

    /// The circle guide with the state of the start and sweep knobs: active while dragged or
    /// while their entry box is open, hovered under the pointer.
    fn tiling_guide(&self) -> Option<TilingCircleGuide> {
        let circle = self.tiling_circle()?;
        let mode = self.tiling.as_ref()?;
        let active = mode.drag.or(mode.entry.map(|entry| entry.handle));
        let hovered = match (active, self.pointer_position) {
            (None, Some(pointer)) => circle.pick(pointer, super::tiling_handles::HANDLE_HIT_PX),
            _ => None,
        };
        let state = |handle: CircleHandle| {
            if active == Some(handle) {
                KnobState::Active
            } else if hovered == Some(handle) {
                KnobState::Hovered
            } else {
                KnobState::Idle
            }
        };
        Some(TilingCircleGuide {
            centre: circle.centre,
            radius_mm: circle.radius_mm,
            start: circle.start.as_radians(),
            sweep: circle.sweep.as_radians(),
            radius_knob: circle.handle_point(CircleHandle::Radius),
            sweep_knob: circle.handle_point(CircleHandle::Sweep),
            start_state: state(CircleHandle::Start),
            sweep_state: state(CircleHandle::Sweep),
        })
    }
}
