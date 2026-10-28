//! What the Trace tool draws in each of its views (`specs/0025-raster-trace` criteria 2, 8, 51,
//! 56 and 68): the wash, the hollow outline of Overlay, the filled result of Trace, nothing of
//! its own in Image, from meshes kept across frames.

use std::collections::HashSet;

use curvyo_document_core::{
    AnchorId, AnchorKind, AnchorSnapshot, Angle, Color, Fill, ImageSnapshot, NodeId,
    ObjectSnapshot, Opacity, PathSnapshot, Point, Stroke, Style, SubpathSnapshot, Vec2,
    ViewTransform,
};
use curvyo_render_core::{DrawList, PreviewAnchor};

use super::trace_mesh::MeshFrame;
use super::{Session, Tool};

/// The wash over a traced image under the preview (criteria 2 and 68 b, UX notes section 4): the
/// image dimmed to 35 % by white at 65 % on the light theme (the only theme so far).
const TRACE_WASH: f64 = 0.65;

/// What the Trace tool shows of the traced image (criterion 51, UX notes section 13).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TraceViewMode {
    /// The image dimmed under the hollow preview outline.
    #[default]
    Overlay,
    /// The result filled as Create paths would make it, the image hidden.
    Trace,
    /// The original at full strength, no outline.
    Image,
}

impl TraceViewMode {
    /// The next view of Shift+`\`: Overlay, Trace, Image, Overlay.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Overlay => Self::Trace,
            Self::Trace => Self::Image,
            Self::Image => Self::Overlay,
        }
    }
}

impl Session {
    /// The chosen view of the Trace tool (criterion 51); the peek does not change it.
    #[must_use]
    pub fn trace_view_mode(&self) -> TraceViewMode {
        self.trace.view_mode
    }

    /// The view on the canvas: Image while `\` is held, else the chosen one; the View pill
    /// shows this one, so it always tells the truth (UX notes section 13).
    #[must_use]
    pub fn shown_trace_view_mode(&self) -> TraceViewMode {
        if self.trace.peek {
            TraceViewMode::Image
        } else {
            self.trace.view_mode
        }
    }

    /// Chooses the view (a segment of the View pill).
    pub fn set_trace_view_mode(&mut self, mode: TraceViewMode) {
        self.trace.view_mode = mode;
    }

    /// Shift+`\`: the next view after the chosen one.
    pub fn cycle_trace_view_mode(&mut self) {
        self.trace.view_mode = self.trace.view_mode.next();
    }

    /// `\` pressed (`true`) or released, the window blurred or the tool left (`false`).
    pub fn set_trace_peek(&mut self, held: bool) {
        self.trace.peek = held && self.tool == Tool::Trace;
    }

    /// The view is moving (`true`) or has rested (`false`, the host's 150 ms without a change of
    /// the view): while it moves the kept meshes are drawn at any zoom, at rest they are made
    /// again for the zoom (criterion 56).
    pub fn set_trace_view_moving(&mut self, moving: bool) {
        self.trace.moving = moving;
    }

    /// The artwork of the Trace tool in the shown view (criterion 51): Overlay washes the traced
    /// images; Trace takes the shown image out and returns where the filled result goes
    /// ([`Session::trace_fill_mesh`]), washing the other selected images; Image leaves the
    /// artwork as it is.
    pub(super) fn trace_artwork(&self, objects: &mut Vec<ObjectSnapshot>) -> Option<usize> {
        match self.shown_trace_view_mode() {
            TraceViewMode::Overlay => {
                self.trace_wash(objects);
                None
            }
            TraceViewMode::Image => None,
            TraceViewMode::Trace => {
                self.trace_wash(objects);
                let shown = self.trace_view().shown?;
                let at = objects.iter().position(
                    |object| matches!(object, ObjectSnapshot::Image(image) if image.id == shown),
                )?;
                // The image and the wash directly above it, which carries its id.
                objects.remove(at);
                if objects.get(at).is_some_and(|object| object.id() == shown) {
                    objects.remove(at);
                }
                Some(at)
            }
        }
    }

    /// The traced images washed to 35 % (criteria 2 and 68 b): a white quad at 65 % directly
    /// above each, under everything stacked above the image and under the preview outline.
    pub(super) fn trace_wash(&self, objects: &mut Vec<ObjectSnapshot>) {
        let view = self.trace_view();
        let mut at = 0;
        while at < objects.len() {
            if let ObjectSnapshot::Image(image) = &objects[at]
                && view.images.contains(&image.id)
            {
                let wash = ObjectSnapshot::Path(wash_over(image));
                objects.insert(at + 1, wash);
                at += 1;
            }
            at += 1;
        }
    }

    /// The filled result of the shown image in the Trace view, black (half strength when stale),
    /// tessellated once per result and zoom (criteria 51, 56); empty without a result.
    pub(super) fn trace_fill_mesh(&self, view: ViewTransform) -> DrawList {
        let shown = self.trace_view().shown;
        let Some(preview) = self
            .trace
            .preview
            .as_ref()
            .filter(|p| Some(p.image) == shown)
        else {
            return DrawList::default();
        };
        let frame = MeshFrame {
            scale: view.scale(),
            stale: preview.stale,
            visible: None,
            moving: self.trace.moving,
        };
        preview.fill_mesh.draw(frame, || {
            let mut fill = preview.fill.clone();
            if preview.stale {
                fill.style.fill.opacity = Opacity::new(0.5).unwrap_or(Opacity::OPAQUE);
            }
            let list = curvyo_render_core::build_artwork_with_guides(
                &[ObjectSnapshot::Path(fill)],
                &HashSet::new(),
                view,
            );
            (list, None)
        })
    }

    /// The visible part of the canvas in document millimetres; `None` before the canvas has a
    /// size.
    fn visible_area(&self) -> Option<(Point, Point)> {
        let (width, height) = self.viewport.canvas_size();
        if width <= 0.0 || height <= 0.0 {
            return None;
        }
        let (a, b) = (
            self.screen_to_document(0.0, 0.0),
            self.screen_to_document(width, height),
        );
        Some((
            Point::new(a.x.min(b.x), a.y.min(b.y)),
            Point::new(a.x.max(b.x), a.y.max(b.y)),
        ))
    }

    /// Appends the hollow preview outline of the shown image in the Overlay view to the overlay
    /// of `list` (criteria 8, 56): the outlines that meet the visible part grown by its own size
    /// on every side, tessellated as one mesh and kept across frames.
    pub(super) fn trace_draw_into(&self, view: ViewTransform, list: &mut DrawList) {
        if self.tool != Tool::Trace || self.shown_trace_view_mode() != TraceViewMode::Overlay {
            return;
        }
        let Some(preview) = &self.trace.preview else {
            return;
        };
        if self.trace_view().shown != Some(preview.image) {
            return;
        }
        let visible = self.visible_area();
        let frame = MeshFrame {
            scale: view.scale(),
            stale: preview.stale,
            visible,
            moving: self.trace.moving,
        };
        let build = || {
            let area = visible.map(|(min, max)| {
                let (x, y) = (max.x - min.x, max.y - min.y);
                (
                    Point::new(min.x - x, min.y - y),
                    Point::new(max.x + x, max.y + y),
                )
            });
            let shown: Vec<Vec<PreviewAnchor>> = preview
                .outlines
                .iter()
                .zip(&preview.boxes)
                .filter(|(_, (lo, hi))| {
                    area.is_none_or(|(min, max)| {
                        lo.x <= max.x && hi.x >= min.x && lo.y <= max.y && hi.y >= min.y
                    })
                })
                .map(|(outline, _)| outline.clone())
                .collect();
            let strength = if preview.stale { 0.5 } else { 1.0 };
            (
                curvyo_render_core::build_preview_outline(&shown, view, strength),
                area,
            )
        };
        // The outline has no artwork layers and no images: its triangles are overlay as they are.
        preview.outline_mesh.with(frame, build, |mesh| {
            list.triangles.extend_from_slice(&mesh.triangles);
        });
    }
}

/// A white quad over `image`'s frame at [`TRACE_WASH`] of the image's own strength; it carries
/// the image's id and is never a document node.
fn wash_over(image: &ImageSnapshot) -> PathSnapshot {
    let anchors = image
        .corners()
        .into_iter()
        .zip(0_u64..)
        .map(|(point, n)| AnchorSnapshot {
            id: AnchorId::new(0, n),
            point,
            handle_in: Vec2::ZERO,
            handle_out: Vec2::ZERO,
            kind: AnchorKind::Corner,
        })
        .collect();
    let strength = image.shown_opacity.get() * TRACE_WASH;
    PathSnapshot {
        id: image.id,
        closed: true,
        style: Style {
            stroke: Stroke {
                enabled: false,
                ..Stroke::default()
            },
            fill: Fill {
                enabled: true,
                color: Color {
                    r: 255,
                    g: 255,
                    b: 255,
                },
                opacity: Opacity::new(strength).unwrap_or(Opacity::OPAQUE),
            },
        },
        anchors,
        extra_subpaths: Vec::new(),
        rotation: Angle::from_radians(0.0),
    }
}

/// The preview `outlines` of `image` as one black compound path without stroke, as Create paths
/// writes it (criterion 13), for the Trace view; it carries the image's id and is never a
/// document node.
pub(super) fn filled_result(image: NodeId, outlines: &[Vec<PreviewAnchor>]) -> PathSnapshot {
    let mut n = 0_u64;
    let mut subpaths = outlines.iter().map(|outline| {
        outline
            .iter()
            .map(|&(point, handle_in, handle_out)| {
                n += 1;
                AnchorSnapshot {
                    id: AnchorId::new(0, n),
                    point,
                    handle_in,
                    handle_out,
                    kind: AnchorKind::Corner,
                }
            })
            .collect::<Vec<_>>()
    });
    let anchors = subpaths.next().unwrap_or_default();
    let extra_subpaths = subpaths
        .map(|anchors| SubpathSnapshot {
            closed: true,
            anchors,
        })
        .collect();
    PathSnapshot {
        id: image,
        closed: true,
        style: Style {
            stroke: Stroke {
                enabled: false,
                ..Stroke::default()
            },
            fill: Fill {
                enabled: true,
                color: Color { r: 0, g: 0, b: 0 },
                opacity: Opacity::OPAQUE,
            },
        },
        anchors,
        extra_subpaths,
        rotation: Angle::from_radians(0.0),
    }
}

/// The anchors of one closed outline for the preview: each node with its two handles.
pub(super) fn preview_anchors(path: &kurbo::BezPath) -> Vec<PreviewAnchor> {
    use curvyo_document_core::Vec2;
    use kurbo::PathEl;
    let point = |p: kurbo::Point| Point::new(p.x, p.y);
    let mut anchors: Vec<PreviewAnchor> = Vec::new();
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => {
                anchors.push((point(p), Vec2::ZERO, Vec2::ZERO));
            }
            PathEl::CurveTo(c1, c2, p) => {
                if let Some(last) = anchors.last_mut() {
                    last.2 = Vec2::new(c1.x - last.0.x, c1.y - last.0.y);
                }
                anchors.push((point(p), Vec2::new(c2.x - p.x, c2.y - p.y), Vec2::ZERO));
            }
            PathEl::QuadTo(_, p) => anchors.push((point(p), Vec2::ZERO, Vec2::ZERO)),
            PathEl::ClosePath => {}
        }
    }
    // The last segment ends on the first anchor: one anchor, both handles.
    if anchors.len() > 2 {
        let (first, last) = (anchors[0], anchors[anchors.len() - 1]);
        if (first.0.x - last.0.x).abs() < 1e-9 && (first.0.y - last.0.y).abs() < 1e-9 {
            anchors[0].1 = last.1;
            anchors.pop();
        }
    }
    anchors
}
