//! What the Trace tool draws (`specs/0025-raster-trace` criteria 2, 8, 56 and 68): the wash and
//! the hollow preview outline, from a mesh kept across frames.

use curvyo_document_core::{
    AnchorId, AnchorKind, AnchorSnapshot, Angle, Color, Fill, ImageSnapshot, ObjectSnapshot,
    Opacity, PathSnapshot, Point, Stroke, Style, Vec2, ViewTransform,
};
use curvyo_render_core::{DrawList, PreviewAnchor};

use super::trace_mesh::MeshFrame;
use super::{Session, Tool};

/// The wash over a traced image under the preview (criteria 2 and 68 b, UX notes section 4): the
/// image dimmed to 35 % by white at 65 % on the light theme (the only theme so far).
const TRACE_WASH: f64 = 0.65;

impl Session {
    /// The view is moving (`true`) or has rested (`false`, the host's 150 ms without a change of
    /// the view): while it moves the kept meshes are drawn at any zoom, at rest they are made
    /// again for the zoom (criterion 56).
    pub fn set_trace_view_moving(&mut self, moving: bool) {
        self.trace.moving = moving;
    }

    /// The artwork of the Trace tool (criteria 2, 68 b): the traced images washed, and the
    /// region the trace is limited to.
    pub(super) fn trace_artwork(&self, objects: &mut Vec<ObjectSnapshot>) {
        self.trace_wash(objects);
        self.trace_region_wash(objects);
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
        if self.tool != Tool::Trace {
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
            let seen: Vec<Vec<PreviewAnchor>> = preview
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
                curvyo_render_core::build_preview_outline(&seen, view, strength),
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
                ..Fill::default()
            },
        },
        anchors,
        extra_subpaths: Vec::new(),
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
